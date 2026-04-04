use std::time::{Duration, Instant};

use triadchat::application::Application;
use triadchat::config::Config;

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
