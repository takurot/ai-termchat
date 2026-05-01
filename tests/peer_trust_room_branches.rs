use std::fs;
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;

use triadchat::application::Application;
use triadchat::config::Config;
use triadchat::message::{NetMessage, PeerInfo};
use triadchat::state::{peer_fingerprint, State};

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

fn peer_info(user_name: &str) -> PeerInfo {
    PeerInfo {
        user_name: user_name.into(),
        server_port: 9000,
        node_version: "1.0.0".into(),
        avatar: "human_default".into(),
    }
}

fn rendered_messages(state: &State) -> String {
    state
        .messages()
        .iter()
        .map(|m| m.rendered_text())
        .collect::<Vec<_>>()
        .join("\n")
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

#[test]
fn peer_info_untrusted_emits_ready_message() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let peer = peer_info("tanaka");
    let fp_short = &peer_fingerprint(&peer)[..12];
    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, "tanaka connected");
    assert_contains(&rendered, &format!("peer ready: tanaka [untrusted] fp={}", fp_short));
}

#[test]
fn peer_info_trusted_emits_ready_message() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let peer = peer_info("tanaka");
    let fp_short = &peer_fingerprint(&peer)[..12];

    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer.clone()));
    app.handle_input_line_for_test("/trust add tanaka").unwrap();
    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, &format!("peer ready: tanaka [trusted] fp={}", fp_short));
}

#[test]
fn repeated_peer_info_does_not_duplicate() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let peer = peer_info("tanaka");

    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer.clone()));
    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));

    let rendered = rendered_messages(app.state());
    let ready_count = rendered.matches("peer ready: tanaka").count();
    assert_eq!(ready_count, 2, "two PeerInfo messages should produce two ready lines");
}

#[test]
fn room_create_includes_local_joins() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let room_id = "room-1".to_string();
    let member_ids = vec!["tester".to_string(), "tanaka".to_string()];
    app.inject_network_message_for_test(
        endpoint,
        NetMessage::RoomCreate(room_id.clone(), member_ids),
    );

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, &format!("joined room {}", room_id));
    assert_eq!(app.state().active_room_id(), Some(room_id.as_str()));
}

#[test]
fn room_create_excludes_local_ignored() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let room_id = "room-2".to_string();
    let member_ids = vec!["tanaka".to_string(), "sato".to_string()];
    app.inject_network_message_for_test(
        endpoint,
        NetMessage::RoomCreate(room_id.clone(), member_ids),
    );

    let rendered = rendered_messages(app.state());
    assert_not_contains(&rendered, "joined room");
    assert_eq!(app.state().active_room_id(), None);
}

#[test]
fn room_join_switches_and_emits_ready() {
    let dir = TempDir::new().unwrap();
    let config = make_test_config(&dir);
    let (mut app, listener) = make_app(&config);

    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();

    let room_id = "room-3".to_string();
    let member_ids = vec!["tester".to_string(), "tanaka".to_string()];
    app.inject_network_message_for_test(
        endpoint,
        NetMessage::RoomCreate(room_id.clone(), member_ids),
    );

    app.inject_network_message_for_test(endpoint, NetMessage::RoomJoin(room_id.clone()));

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, &format!("room {} is ready", room_id));
    assert_eq!(app.state().active_room_id(), Some(room_id.as_str()));
}

#[test]
fn trust_persistence_failure_does_not_crash() {
    let dir = TempDir::new().unwrap();
    let script = dir.path().join("mock-claude.sh");
    fs::write(&script, "#!/bin/sh\nprintf 'ok'\n").unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let workspace = TempDir::new().unwrap();
    let test_config_dir = workspace.path().join(".triadchat-test-config");
    fs::create_dir_all(&test_config_dir).unwrap();

    let config_toml = test_config_dir.join("config.toml");
    fs::write(&config_toml, "[security]").unwrap();

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    config.user_name = "tester".into();
    config.terminal_bell = false;

    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();
    app.start_network_for_test().unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = app.connect_raw_for_test(listener.local_addr().unwrap()).unwrap();
    let peer = peer_info("tanaka");
    app.inject_network_message_for_test(endpoint, NetMessage::PeerInfo(peer));

    let mut raw_perms = fs::metadata(&config_toml).unwrap().permissions();
    raw_perms.set_readonly(true);
    fs::set_permissions(&config_toml, raw_perms).unwrap();

    app.handle_input_line_for_test("/trust add tanaka").unwrap();

    let rendered = rendered_messages(app.state());
    assert_contains(&rendered, "trusted peer tanaka");

    let mut perms = fs::metadata(&config_toml).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&config_toml, perms).unwrap();
}
