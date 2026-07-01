use std::fs;
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;

use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use sha2::Digest;

use triadchat::application::Application;
use triadchat::config::Config;
use triadchat::message::{NetMessage, PeerInfo, SignaturePayload};
use triadchat::state::State;

fn make_test_config(dir: &TempDir) -> Config {
    let script = dir.path().join("mock-claude.sh");
    fs::write(&script, "#!/bin/sh\nprintf 'ok'\n").unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let discovery_port = 50000 + (rand::random::<u16>() % 10000);
    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    config.user_name = "tester".into();
    config.terminal_bell = false;
    config.discovery_addr = format!("239.255.0.1:{discovery_port}").parse().unwrap();
    config
}

fn make_app(config: &Config) -> (Application<'_>, TcpListener) {
    let mut app = Application::new_for_test(config).unwrap();
    app.start_network_for_test().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    (app, listener)
}

fn peer_info_identity_ready(user_name: &str) -> PeerInfo {
    PeerInfo {
        user_name: user_name.into(),
        server_port: 9000,
        node_version: "0.1.2".into(),
        avatar: "human_default".into(),
    }
}

fn rendered_messages(state: &State) -> String {
    state.messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n")
}

fn assert_contains(rendered: &str, expected: &str) {
    assert!(
        rendered.contains(expected),
        "expected messages to contain '{}', got:\n{}",
        expected,
        rendered
    );
}

fn assert_not_contains(rendered: &str, unexpected: &str) {
    assert!(
        !rendered.contains(unexpected),
        "expected messages to NOT contain '{}', got:\n{}",
        unexpected,
        rendered
    );
}

fn build_peer_identity(signing_key: &SigningKey, peer: &PeerInfo, timestamp: u64) -> NetMessage {
    let payload = SignaturePayload {
        user_name: peer.user_name.clone(),
        node_version: peer.node_version.clone(),
        server_port: peer.server_port,
        timestamp,
    };
    let serialized = bincode::serde::encode_to_vec(&payload, bincode::config::legacy()).unwrap();
    let signature = signing_key.sign(&serialized);
    NetMessage::PeerIdentity {
        public_key: signing_key.verifying_key().to_bytes().to_vec(),
        signature: signature.to_bytes().to_vec(),
        timestamp,
    }
}

#[test]
fn valid_signature_verification_succeeds() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let signing_key = SigningKey::generate(&mut OsRng);
    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let peer = peer_info_identity_ready("tanaka");

    let timestamp = chrono::Utc::now().timestamp() as u64;
    let identity = build_peer_identity(&signing_key, &peer, timestamp);

    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));
    app.inject_network_message_for_test(endpoint, identity);

    let public_key_bytes = signing_key.verifying_key().to_bytes();
    let mut hasher = sha2::Sha256::new();
    hasher.update(&public_key_bytes);
    let fp = format!("{:x}", hasher.finalize());
    let fp_short = &fp[..12];

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, "tanaka connected");
    assert_contains(&rendered, &format!("peer ready: tanaka [untrusted] fp={}", fp_short));
}

#[test]
fn signature_timestamp_drift_is_rejected() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let signing_key = SigningKey::generate(&mut OsRng);
    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let peer = peer_info_identity_ready("tanaka");

    let timestamp = chrono::Utc::now().timestamp() as u64 - 61;
    let identity = build_peer_identity(&signing_key, &peer, timestamp);

    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));
    app.inject_network_message_for_test(endpoint, identity);

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, "Security warning: signature timestamp drift too high");
    assert_contains(&rendered, "Disconnecting.");
    assert_not_contains(&rendered, "peer ready: tanaka");
}

#[test]
fn replayed_signature_is_rejected() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let signing_key = SigningKey::generate(&mut OsRng);
    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let peer = peer_info_identity_ready("tanaka");

    let timestamp = chrono::Utc::now().timestamp() as u64;
    let identity = build_peer_identity(&signing_key, &peer, timestamp);

    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer.clone()));
    app.inject_network_message_for_test(endpoint, identity.clone());

    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));
    app.inject_network_message_for_test(endpoint, identity);

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, "peer ready: tanaka");
    assert_contains(
        &rendered,
        "Security warning: replayed signature detected from tanaka. Disconnecting.",
    );
}

#[test]
fn invalid_signature_is_rejected() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let signing_key = SigningKey::generate(&mut OsRng);
    let different_key = SigningKey::generate(&mut OsRng);
    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let peer = peer_info_identity_ready("tanaka");

    let timestamp = chrono::Utc::now().timestamp() as u64;
    let payload = SignaturePayload {
        user_name: peer.user_name.clone(),
        node_version: peer.node_version.clone(),
        server_port: peer.server_port,
        timestamp,
    };
    let serialized = bincode::serde::encode_to_vec(&payload, bincode::config::legacy()).unwrap();
    let wrong_signature = different_key.sign(&serialized);
    let identity = NetMessage::PeerIdentity {
        public_key: signing_key.verifying_key().to_bytes().to_vec(),
        signature: wrong_signature.to_bytes().to_vec(),
        timestamp,
    };

    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));
    app.inject_network_message_for_test(endpoint, identity);

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, "Security warning: signature verification failed for tanaka");
    assert_contains(&rendered, "Disconnecting.");
    assert_not_contains(&rendered, "peer ready: tanaka");
}

#[test]
fn malformed_public_key_is_rejected() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let signing_key = SigningKey::generate(&mut OsRng);
    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let peer = peer_info_identity_ready("tanaka");

    let timestamp = chrono::Utc::now().timestamp() as u64;
    let payload = SignaturePayload {
        user_name: peer.user_name.clone(),
        node_version: peer.node_version.clone(),
        server_port: peer.server_port,
        timestamp,
    };
    let serialized = bincode::serde::encode_to_vec(&payload, bincode::config::legacy()).unwrap();
    let signature = signing_key.sign(&serialized);
    let identity = NetMessage::PeerIdentity {
        public_key: vec![0xFFu8; 32],
        signature: signature.to_bytes().to_vec(),
        timestamp,
    };

    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));
    app.inject_network_message_for_test(endpoint, identity);

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, "signature verification failed for tanaka");
    assert_contains(&rendered, "Disconnecting.");
    assert_not_contains(&rendered, "peer ready: tanaka");
}

#[test]
fn authenticated_peer_messages_accepted_after_verification() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let signing_key = SigningKey::generate(&mut OsRng);
    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let peer = peer_info_identity_ready("tanaka");

    let timestamp = chrono::Utc::now().timestamp() as u64;
    let identity = build_peer_identity(&signing_key, &peer, timestamp);

    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));
    app.inject_network_message_for_test(endpoint, identity);

    app.inject_network_message_for_test(
        endpoint,
        NetMessage::UserMessage("hello from verified peer".to_string()),
    );

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, "peer ready: tanaka");
    assert_contains(&rendered, "hello from verified peer");
}
