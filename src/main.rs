use clap::{Arg, Command};
use std::net::SocketAddrV4;
use triadchat::application::Application;
use triadchat::config::Config;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_target(false)
        .try_init()
        .ok();

    let matches = Command::new("triadchat")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("discovery")
                .long("discovery")
                .short('d')
                .value_name("IP:PORT")
                .value_parser(clap::builder::ValueParser::new(|addr: &str| {
                    addr.parse::<SocketAddrV4>()
                        .map(|_| addr.to_string())
                        .map_err(|_| "The value must have syntax ipv4:port".to_string())
                }))
                .help("Multicast address used to find other triadchat applications"),
        )
        .arg(
            Arg::new("tcp_server_port")
                .long("tcp-server-port")
                .short('t')
                .value_name("PORT")
                .value_parser(clap::value_parser!(String))
                .help("TCP server port used when communicating with other triadchat instances"),
        )
        .arg(
            Arg::new("username")
                .long("username")
                .short('u')
                .value_name("NAME")
                .help("Name used as the local user"),
        )
        .arg(
            Arg::new("quiet-mode")
                .long("quiet-mode")
                .short('q')
                .action(clap::ArgAction::SetTrue)
                .help("Disable the terminal bell sound"),
        )
        .arg(
            Arg::new("theme")
                .long("theme")
                .value_name("THEME")
                .value_parser(["dark", "light"])
                .help("Choose a theme: dark or light"),
        )
        .get_matches();

    let config = Config::from_matches(&matches);
    let result = Application::new(&config).and_then(|mut app| app.run(std::io::stdout()));

    if let Err(error) = result {
        eprintln!("triadchat exited with error: {error}");
    }
}
