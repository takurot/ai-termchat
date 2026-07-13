use std::time::{Duration, Instant};

use triadchat::application::Application;
use triadchat::commands::send_file::SendFile;
use triadchat::config::Config;
use triadchat::message::Chunk;
use triadchat::state::{MessageType, SystemMessageType};

fn test_config(user_name: &str, discovery_port: u16) -> Config {
    Config {
        user_name: user_name.to_string(),
        discovery_addr: format!("239.255.0.1:{discovery_port}").parse().unwrap(),
        terminal_bell: false,
        ..Config::default()
    }
}

// ── `/send` command error-path tests ────────────────────────────────

#[test]
fn send_no_args_emits_error() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app =
        Application::new_for_test(&config).expect("application should build for no-args test");
    app.handle_input_line_for_test("/send").expect("/send with no args should not panic");
    let messages = app.state().messages();
    let last = messages.last().expect("a system error should be emitted for /send with no args");
    assert!(
        last.rendered_text().contains("No file specified"),
        "expected message to contain 'No file specified', got: '{}'",
        last.rendered_text(),
    );
}

#[test]
fn send_nonexistent_file_emits_error() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config)
        .expect("application should build for nonexistent-file test");
    app.handle_input_line_for_test("/send /nonexistent/file/path.txt")
        .expect("/send with nonexistent path should not panic");
    let messages = app.state().messages();
    let last = messages.last().expect("a system error should be emitted for nonexistent file");
    let text = last.rendered_text();
    assert!(!text.is_empty(), "expected a non-empty error message for nonexistent file, got empty",);

    assert!(
        matches!(last.message_type, MessageType::System(_, SystemMessageType::Error)),
        "expected last message to be a system error, got: {:?}",
        last.message_type,
    );

    assert!(
        text.contains("/nonexistent/file/path.txt")
            || text.contains("No such file")
            || text.contains("not found")
            || text.to_lowercase().contains("unable to"),
        "expected error to reference the file, got: '{}'",
        text,
    );
}

#[test]
fn send_trailing_space_emits_error() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config)
        .expect("application should build for trailing-space test");
    app.handle_input_line_for_test("/send ").expect("/send with trailing space should not panic");
    let messages = app.state().messages();
    let last =
        messages.last().expect("a system error should be emitted for /send with trailing space");
    assert!(
        last.rendered_text().contains("No file specified"),
        "expected 'No file specified' for /send with trailing space, got: '{}'",
        last.rendered_text(),
    );
}

// ── SendFile::new() error-path unit tests ───────────────────────────

#[test]
fn send_file_new_root_path_fails() {
    let result = SendFile::new("/");
    assert!(result.is_err(), "SendFile::new('/') should fail (no file name)");
    let err = result.err().expect("just asserted is_err");
    assert!(
        err.to_string().contains("Unable to read file name"),
        "expected 'Unable to read file name', got: '{err}'",
    );
}

#[test]
fn send_file_new_nonexistent_fails() {
    let result = SendFile::new("/nonexistent/file/path.txt");
    assert!(
        result.is_err(),
        "SendFile::new('/nonexistent/file/path.txt') should fail (file not found)",
    );
    let err = result.err().expect("just asserted is_err");
    let text = err.to_string();
    assert!(!text.is_empty(), "expected a non-empty error for nonexistent file",);
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

    panic!("timed out waiting for send_file integration condition");
}

#[test]
fn send_file() {
    let triadchat_dir = std::env::temp_dir().join("triadchat");
    let test_path = triadchat_dir.join("test");
    let _ = std::fs::remove_dir_all(&triadchat_dir);
    std::fs::create_dir_all(&triadchat_dir).unwrap();

    let data = vec![rand::random(); 10usize.pow(6)];
    std::fs::write(&test_path, &data).unwrap();

    let discovery_port = 39000 + (rand::random::<u16>() % 1000);
    let config1 = test_config("1", discovery_port);
    let config2 = test_config("2", discovery_port + 1);
    let mut sender = Application::new_for_test(&config1).unwrap();
    let mut receiver = Application::new_for_test(&config2).unwrap();

    sender.start_network_for_test().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    receiver.start_network_for_test().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    sender.connect_peer_for_test(receiver.local_server_port_for_test().unwrap()).unwrap();

    pump_until(&mut sender, &mut receiver, Duration::from_secs(3), |left, right| {
        left.state().peers().len() == 1 && right.state().peers().len() == 1
    });

    sender.handle_input_line_for_test(&format!("/send {}", test_path.display())).unwrap();

    pump_until(&mut sender, &mut receiver, Duration::from_secs(5), |_, right| {
        right.state().pending_transfer_offer().is_some()
    });

    receiver.handle_input_line_for_test("/accept test").unwrap();

    for _ in 0..20 {
        let _ = sender.process_next_event_with_timeout_for_test(Duration::from_millis(50));
        let _ = receiver.process_next_event_with_timeout_for_test(Duration::from_millis(50));
    }

    let received_path = std::env::temp_dir().join("triadchat/downloads").join("1").join("test");
    let expected_len = data.len() as u64;
    pump_until(&mut sender, &mut receiver, Duration::from_secs(20), |_, _| {
        std::fs::metadata(&received_path).map(|meta| meta.len() == expected_len).unwrap_or(false)
    });

    let send_data = std::fs::read(received_path).unwrap();
    assert_eq!(data.len(), send_data.len());
    assert_eq!(data, send_data);
}

#[test]
fn receive_chunk_error_reports_error_message() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.inject_receive_chunk_for_test("test.txt", triadchat::message::Chunk::Error, "sender");

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");
    assert!(rendered.contains("had an error while sending"));
    assert!(rendered.contains("test.txt"));
    assert!(rendered.contains("sender"));
}

#[test]
fn receive_chunk_end_reports_success_message() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.inject_receive_chunk_for_test("test.txt", triadchat::message::Chunk::End, "sender");

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");
    assert!(rendered.contains("Successfully received file"));
    assert!(rendered.contains("test.txt"));
    assert!(rendered.contains("sender"));
}

fn tx_bytes(app: &Application, user: &str, filename: &str) -> Option<u64> {
    app.state()
        .active_transfers_view()
        .iter()
        .find(|v| v.user == user && v.filename == filename)
        .map(|v| v.bytes_received)
}

// ── receive-progress (byte counter) integration tests ────────────────

#[test]
fn chunk_data_accumulates_byte_counter() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.inject_receive_chunk_for_test("test.txt", Chunk::Data(vec![42u8; 1024]), "sender");
    assert_eq!(tx_bytes(&app, "sender", "test.txt"), Some(1024));

    app.inject_receive_chunk_for_test("test.txt", Chunk::Data(vec![7u8; 512]), "sender");
    assert_eq!(tx_bytes(&app, "sender", "test.txt"), Some(1536));
}

#[test]
fn chunk_end_clears_byte_counter() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.inject_receive_chunk_for_test("test.txt", Chunk::Data(vec![42u8; 1024]), "sender");
    assert_eq!(tx_bytes(&app, "sender", "test.txt"), Some(1024));

    app.inject_receive_chunk_for_test("test.txt", Chunk::End, "sender");

    assert_eq!(tx_bytes(&app, "sender", "test.txt"), None);
}

#[test]
fn chunk_error_clears_byte_counter() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.inject_receive_chunk_for_test("test.txt", Chunk::Data(vec![42u8; 1024]), "sender");
    assert_eq!(tx_bytes(&app, "sender", "test.txt"), Some(1024));

    app.inject_receive_chunk_for_test("test.txt", Chunk::Error, "sender");

    assert_eq!(tx_bytes(&app, "sender", "test.txt"), None);
}

#[test]
fn disconnected_user_clears_active_transfers() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.inject_authenticated_peer_for_test("sender", "fp:sender:test");
    let endpoint = app.state().peer_endpoint_by_name("sender").expect("peer endpoint should exist");

    app.inject_receive_chunk_for_test("test.txt", Chunk::Data(vec![42u8; 1024]), "sender");
    assert_eq!(tx_bytes(&app, "sender", "test.txt"), Some(1024));

    app.disconnect_peer_for_test(endpoint);

    assert_eq!(tx_bytes(&app, "sender", "test.txt"), None);
}

#[test]
fn transfer_reject_sends_rejection() {
    let triadchat_dir = std::env::temp_dir().join("triadchat-reject");
    let test_path = triadchat_dir.join("reject_test.bin");
    let _ = std::fs::remove_dir_all(&triadchat_dir);
    std::fs::create_dir_all(&triadchat_dir).unwrap();
    let data = vec![88u8; 512];
    std::fs::write(&test_path, &data).unwrap();

    let discovery_port = 42000 + (rand::random::<u16>() % 1000);
    let config1 = test_config("sender", discovery_port);
    let config2 = test_config("receiver", discovery_port + 1);
    let mut sender = Application::new_for_test(&config1).unwrap();
    let mut receiver = Application::new_for_test(&config2).unwrap();

    sender.start_network_for_test().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    receiver.start_network_for_test().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    sender.connect_peer_for_test(receiver.local_server_port_for_test().unwrap()).unwrap();

    pump_until(&mut sender, &mut receiver, Duration::from_secs(3), |left, right| {
        left.state().peers().len() == 1 && right.state().peers().len() == 1
    });

    sender.handle_input_line_for_test(&format!("/send {}", test_path.display())).unwrap();

    pump_until(&mut sender, &mut receiver, Duration::from_secs(5), |_, right| {
        right.state().pending_transfer_offer().is_some()
    });

    receiver.handle_input_line_for_test("/reject reject_test.bin").unwrap();

    assert!(receiver.state().pending_transfer_offer().is_none());
    let rendered = receiver
        .state()
        .messages()
        .iter()
        .map(|m| m.rendered_text())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("rejected transfer"), "got: {}", rendered);
}

#[test]
fn unsolicited_chunk_rejected_without_offer() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();
    app.inject_authenticated_peer_for_test("attacker", "fp:attacker:test");

    let endpoint = app.state().peer_endpoint_by_name("attacker").unwrap();
    app.inject_network_message_for_test(
        endpoint,
        triadchat::message::NetMessage::UserData("evil.bin".into(), Chunk::Data(vec![0u8; 64])),
    );

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");
    assert!(
        rendered.contains("rejected unsolicited file chunk"),
        "expected rejection warning, got: {}",
        rendered
    );
}

#[test]
fn oversized_transfer_rejected() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();
    app.inject_authenticated_peer_for_test("sender", "fp:sender:oversized");

    let endpoint = app.state().peer_endpoint_by_name("sender").unwrap();
    app.inject_network_message_for_test(
        endpoint,
        triadchat::message::NetMessage::TransferOffer {
            file_name: "huge.bin".into(),
            file_size: 200 * 1024 * 1024, // 200 MB, exceeds 100 MB limit
            sender: "sender".into(),
        },
    );

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");
    assert!(
        rendered.contains("rejected: 209715200"),
        "expected oversize rejection, got: {}",
        rendered
    );
    assert!(
        app.state().pending_transfer_offer().is_none(),
        "oversized offer should not be stored as pending"
    );
}

// ── #110: file transfer data plane over the secure session ───────────

#[test]
fn encrypted_file_transfer_completes_after_secure_session() {
    // Isolated downloads dir so this test cannot collide with the `send_file`
    // test's `remove_dir_all(triadchat_dir)` cleanup.
    let downloads_root = tempfile::TempDir::new().unwrap();
    let downloads_base = downloads_root.path().to_path_buf();

    // 96 KiB source file = 3 data chunks (32 KiB CHUNK_SIZE) + 1 Chunk::End.
    let src_dir = tempfile::TempDir::new().unwrap();
    let src_path = src_dir.path().join("encrypted_96k.bin");
    let data: Vec<u8> = (0..(96 * 1024)).map(|i| (i & 0xFF) as u8).collect();
    std::fs::write(&src_path, &data).unwrap();

    let discovery_port = 60000 + (rand::random::<u16>() % 5000);
    let config_sender = test_config("alice", discovery_port);
    let config_receiver = test_config("bob", discovery_port + 1);
    let mut sender = Application::new_for_test(&config_sender).unwrap();
    let mut receiver = Application::new_for_test(&config_receiver).unwrap();
    receiver.set_downloads_base_dir_for_test(downloads_base.clone());

    sender.start_network_for_test().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    receiver.start_network_for_test().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    sender.connect_peer_for_test(receiver.local_server_port_for_test().unwrap()).unwrap();

    // Wait for both peers to be authenticated and a secure session to exist in
    // both directions before initiating the transfer.
    pump_until(&mut sender, &mut receiver, Duration::from_secs(5), |left, right| {
        left.state().peer_is_ready("bob") && right.state().peer_is_ready("alice")
    });
    let receiver_endpoint = sender.state().peer_endpoint_by_name("bob").unwrap();
    let sender_endpoint = receiver.state().peer_endpoint_by_name("alice").unwrap();
    pump_until(&mut sender, &mut receiver, Duration::from_secs(5), |left, right| {
        left.has_secure_session(receiver_endpoint) && right.has_secure_session(sender_endpoint)
    });
    assert!(sender.has_secure_session(receiver_endpoint), "session must be established pre-send");

    sender.handle_input_line_for_test(&format!("/send {}", src_path.display())).unwrap();

    pump_until(&mut sender, &mut receiver, Duration::from_secs(5), |_, right| {
        right.state().pending_transfer_offer().is_some()
    });

    receiver.handle_input_line_for_test("/accept encrypted_96k.bin").unwrap();

    let received_path =
        downloads_base.join("triadchat/downloads").join("alice").join("encrypted_96k.bin");
    pump_until(&mut sender, &mut receiver, Duration::from_secs(10), |_, _| {
        std::fs::metadata(&received_path).map(|m| m.len() == data.len() as u64).unwrap_or(false)
    });

    let received = std::fs::read(&received_path).unwrap();
    assert_eq!(received, data, "decrypted transfer must reproduce the source bytes exactly");
}
