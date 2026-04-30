use std::time::{Duration, Instant};

use triadchat::application::Application;
use triadchat::commands::send_file::SendFile;
use triadchat::config::Config;

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
    // On most platforms the OS error mentions 'such file' or 'file'.
    assert!(
        text.to_lowercase().contains("file"),
        "expected error to mention 'file', got: '{}'",
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

    let received_path = std::env::temp_dir().join("triadchat").join("1").join("test");
    let expected_len = data.len() as u64;
    pump_until(&mut sender, &mut receiver, Duration::from_secs(20), |_, _| {
        std::fs::metadata(&received_path).map(|meta| meta.len() == expected_len).unwrap_or(false)
    });

    let send_data = std::fs::read(received_path).unwrap();
    assert_eq!(data.len(), send_data.len());
    assert_eq!(data, send_data);
}
