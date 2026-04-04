#![cfg(feature = "ui-test")]

use triadchat::application::{Application, Signal};
use triadchat::config::Config;

use message_io::node::{NodeHandler};

#[test]
#[ignore = "Phase 1 file transfer flow is out of Phase 0 scope"]
fn send_file() {
    let triadchat_dir = std::env::temp_dir().join("triadchat");
    let test_path = triadchat_dir.join("test");
    let _ = std::fs::remove_dir_all(&triadchat_dir);
    std::fs::create_dir_all(&triadchat_dir).unwrap();

    let data = vec![rand::random(); 10usize.pow(6)];
    std::fs::write(&test_path, &data).unwrap();

    // spawn users
    let config1: Config = Config { user_name: 1.to_string(), ..Config::default() };
    let config2: Config = Config { user_name: 2.to_string(), ..Config::default() };
    let (mut s1, t1) = test_user(config1);
    // wait a bit or triadchat will create two communication channels at the same time
    std::thread::sleep(std::time::Duration::from_millis(100));
    let (s2, t2) = test_user(config2);

    // wait for users to connect
    std::thread::sleep(std::time::Duration::from_millis(100));
    // send file
    input(&mut s1, &format!("/send {}", test_path.display()));
    // wait for the file to finish sending
    let received_path = std::env::temp_dir().join("triadchat").join("1").join("test");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !received_path.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // finish
    s1.signals().send(Signal::Close(None));
    s2.signals().send(Signal::Close(None));
    t1.join().unwrap();
    t2.join().unwrap();

    // assert eq
    let send_data = std::fs::read(received_path).unwrap();
    assert_eq!(data.len(), send_data.len());
    assert_eq!(data, send_data);
}

fn test_user(config: Config) -> (NodeHandler<Signal>, std::thread::JoinHandle<()>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let t = std::thread::spawn(move || {
        let mut app = Application::new_for_test(&config).unwrap();
        tx.send(app.node_handler()).unwrap();
        app.run(std::io::sink()).unwrap();
    });
    (rx.recv().unwrap(), t)
}

fn input(handler: &mut NodeHandler<Signal>, s: &str) {
    for c in s.chars() {
        handler.signals().send(Signal::Terminal(crossterm::event::Event::Key(
            crossterm::event::KeyEvent {
                code: crossterm::event::KeyCode::Char(c),
                modifiers: crossterm::event::KeyModifiers::NONE,
            },
        )));
    }
    handler.signals().send(Signal::Terminal(crossterm::event::Event::Key(
        crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Enter,
            modifiers: crossterm::event::KeyModifiers::NONE,
        },
    )));
}
