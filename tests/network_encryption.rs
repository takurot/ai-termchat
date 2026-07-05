use std::net::TcpListener;
use std::time::{Duration, Instant};

use triadchat::application::Application;
use triadchat::config::Config;
use triadchat::message::{NetMessage, PeerInfo, SignaturePayload};
use triadchat::state::State;

mod common;

fn test_config(user_name: &str, discovery_port: u16) -> Config {
    Config {
        user_name: user_name.to_string(),
        discovery_addr: format!("239.255.0.1:{discovery_port}").parse().unwrap(),
        terminal_bell: false,
        ..Config::default()
    }
}

fn pump_until<F>(
    left: &mut Application<'_>,
    right: &mut Application<'_>,
    timeout: Duration,
    mut predicate: F,
) where
    F: FnMut(&Application<'_>, &Application<'_>) -> bool,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate(left, right) {
            return;
        }
        let _ = left.process_next_event_with_timeout_for_test(Duration::from_millis(50));
        let _ = right.process_next_event_with_timeout_for_test(Duration::from_millis(50));
    }
    if predicate(left, right) {
        return;
    }
    panic!(
        "timed out: left peers={:?}, right peers={:?}",
        left.state().peer_names(),
        right.state().peer_names()
    );
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

#[test]
fn full_key_exchange_establishes_secure_session_between_two_nodes() {
    let discovery_port = 20000 + (rand::random::<u16>() % 10000);
    let alice_config = test_config("alice", discovery_port);
    let bob_config = test_config("bob", discovery_port + 1);
    let mut alice = Application::new_for_test(&alice_config).unwrap();
    let mut bob = Application::new_for_test(&bob_config).unwrap();

    alice.start_network_for_test().unwrap();
    bob.start_network_for_test().unwrap();
    alice.connect_peer_for_test(bob.local_server_port_for_test().unwrap()).unwrap();

    pump_until(&mut alice, &mut bob, Duration::from_secs(5), |left, right| {
        left.state().peer_is_ready("bob") && right.state().peer_is_ready("alice")
    });

    let alice_endpoint = bob.state().peer_endpoint_by_name("alice").unwrap();
    let bob_endpoint = alice.state().peer_endpoint_by_name("bob").unwrap();

    // After PeerIdentity verification, "alice" < "bob", so alice initiates key exchange.
    // Give it time for the key exchange messages to propagate.
    pump_until(&mut alice, &mut bob, Duration::from_secs(5), |left, right| {
        left.has_secure_session(bob_endpoint) && right.has_secure_session(alice_endpoint)
    });

    assert!(alice.has_secure_session(bob_endpoint), "alice should have secure session with bob");
    assert!(bob.has_secure_session(alice_endpoint), "bob should have secure session with alice");
}

#[test]
fn encrypted_chat_messages_exchanged_after_key_exchange() {
    let discovery_port = 20000 + (rand::random::<u16>() % 10000);
    let alice_config = test_config("alice", discovery_port);
    let bob_config = test_config("bob", discovery_port + 1);
    let mut alice = Application::new_for_test(&alice_config).unwrap();
    let mut bob = Application::new_for_test(&bob_config).unwrap();

    alice.start_network_for_test().unwrap();
    bob.start_network_for_test().unwrap();
    alice.connect_peer_for_test(bob.local_server_port_for_test().unwrap()).unwrap();

    pump_until(&mut alice, &mut bob, Duration::from_secs(5), |left, right| {
        left.state().peer_is_ready("bob") && right.state().peer_is_ready("alice")
    });

    let alice_endpoint = bob.state().peer_endpoint_by_name("alice").unwrap();
    let bob_endpoint = alice.state().peer_endpoint_by_name("bob").unwrap();

    pump_until(&mut alice, &mut bob, Duration::from_secs(5), |left, right| {
        left.has_secure_session(bob_endpoint) && right.has_secure_session(alice_endpoint)
    });

    // Alice sends a chat message
    alice.handle_input_line_for_test("hello bob (encrypted)").unwrap();

    pump_until(&mut alice, &mut bob, Duration::from_secs(5), |_left, right| {
        rendered_messages(right.state()).contains("hello bob (encrypted)")
    });

    let bob_rendered = rendered_messages(bob.state());
    assert_contains(&bob_rendered, "hello bob (encrypted)");
}

#[test]
fn plaintext_works_without_key_exchange_for_backward_compat() {
    let dir = tempfile::TempDir::new().unwrap();
    let script = dir.path().join("mock-claude.sh");
    std::fs::write(&script, "#!/bin/sh\nprintf 'ok'\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();
    }

    let discovery_port = 40000 + (rand::random::<u16>() % 10000);
    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    config.user_name = "tester".into();
    config.terminal_bell = false;
    config.discovery_addr = format!("239.255.0.1:{discovery_port}").parse().unwrap();

    let mut app = Application::new_for_test(&config).unwrap();
    app.start_network_for_test().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();

    let peer_signing_key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();

    let peer = PeerInfo {
        user_name: "tanaka".into(),
        server_port: 9000,
        node_version: "0.1.2".into(),
        avatar: "human_default".into(),
    };

    let timestamp = chrono::Utc::now().timestamp() as u64;
    let payload = SignaturePayload {
        user_name: peer.user_name.clone(),
        node_version: peer.node_version.clone(),
        server_port: peer.server_port,
        timestamp,
    };
    let serialized = bincode::serde::encode_to_vec(&payload, bincode::config::legacy()).unwrap();
    use ed25519_dalek::Signer;
    let identity = NetMessage::PeerIdentity {
        public_key: peer_signing_key.verifying_key().to_bytes().to_vec(),
        signature: peer_signing_key.sign(&serialized).to_bytes().to_vec(),
        timestamp,
    };

    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));
    app.inject_network_message_for_test(endpoint, identity);

    // No key exchange — plaintext message should still be accepted
    app.inject_network_message_for_test(
        endpoint,
        NetMessage::UserMessage("plaintext hello".to_string()),
    );

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, "plaintext hello");
    assert_contains(&rendered, "peer ready: tanaka");
}

#[test]
fn replayed_secure_frame_is_rejected() {
    let discovery_port = 30000 + (rand::random::<u16>() % 10000);
    let alice_config = test_config("alice", discovery_port);
    let bob_config = test_config("bob", discovery_port + 1);
    let mut alice = Application::new_for_test(&alice_config).unwrap();
    let mut bob = Application::new_for_test(&bob_config).unwrap();

    alice.start_network_for_test().unwrap();
    bob.start_network_for_test().unwrap();
    alice.connect_peer_for_test(bob.local_server_port_for_test().unwrap()).unwrap();

    pump_until(&mut alice, &mut bob, Duration::from_secs(5), |left, right| {
        left.state().peer_is_ready("bob") && right.state().peer_is_ready("alice")
    });

    let alice_endpoint = bob.state().peer_endpoint_by_name("alice").unwrap();
    let bob_endpoint = alice.state().peer_endpoint_by_name("bob").unwrap();

    pump_until(&mut alice, &mut bob, Duration::from_secs(5), |left, right| {
        left.has_secure_session(bob_endpoint) && right.has_secure_session(alice_endpoint)
    });

    let inner = triadchat::message::NetMessage::UserMessage("replay test".to_string());
    let secure_frame =
        alice.build_secure_frame_for_test(bob_endpoint, inner).expect("should build secure frame");

    bob.inject_network_message_for_test(alice_endpoint, secure_frame.clone());
    let after_first = rendered_messages(bob.state());
    assert_contains(&after_first, "replay test");

    let first_count = after_first.matches("replay test").count();

    bob.inject_network_message_for_test(alice_endpoint, secure_frame);
    let after_replay = rendered_messages(bob.state());
    assert_eq!(
        after_replay.matches("replay test").count(),
        first_count,
        "replayed secure frame must not produce a duplicate message"
    );
}
