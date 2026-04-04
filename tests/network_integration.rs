use std::time::{Duration, Instant};

use triadchat::application::Application;
use triadchat::config::Config;

fn test_config(user_name: &str, discovery_port: u16) -> Config {
    let mut config = Config::default();
    config.user_name = user_name.to_string();
    config.discovery_addr = format!("239.255.0.1:{discovery_port}").parse().unwrap();
    config.terminal_bell = false;
    config
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

    panic!("timed out waiting for integration condition");
}

#[test]
fn peers_exchange_info_and_room_create_propagates() {
    let discovery_port = 38000 + (rand::random::<u16>() % 1000);
    let takuro_config = test_config("takuro", discovery_port);
    let tanaka_config = test_config("tanaka", discovery_port);
    let mut takuro = Application::new_for_test(&takuro_config).unwrap();
    let mut tanaka = Application::new_for_test(&tanaka_config).unwrap();

    takuro.start_network_for_test().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    tanaka.start_network_for_test().unwrap();
    std::thread::sleep(Duration::from_millis(100));

    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(3), |left, right| {
        left.state().peers().len() == 1 && right.state().peers().len() == 1
    });

    takuro
        .handle_input_line_for_test("/room create @tanaka --ai clerk")
        .unwrap();

    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(3), |left, right| {
        left.state().rooms().len() == 1
            && right.state().rooms().len() == 1
            && left.state().active_room_id().is_some()
            && left.state().active_room_id() == right.state().active_room_id()
    });

    let takuro_room = &takuro.state().rooms()[0];
    assert!(takuro_room.members.iter().any(|member| member.id == "ops-ai"));
    assert_eq!(
        tanaka
            .state()
            .peers()
            .values()
            .next()
            .map(|peer| peer.user_name.as_str()),
        Some("takuro")
    );
}
