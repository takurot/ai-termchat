use std::net::SocketAddrV4;
use std::path::PathBuf;

use clap::ArgMatches;
use serde::{Deserialize, Serialize};
use tui::style::Color;

use crate::util::Result;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub discovery_addr: SocketAddrV4,
    pub tcp_server_port: u16,
    pub user_name: String,
    pub terminal_bell: bool,
    pub language: LanguageConfig,
    pub ai: AiConfig,
    pub security: SecurityConfig,
    pub theme: Theme,
    #[serde(default)]
    pub user: UserConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            discovery_addr: "238.255.0.1:5877".parse().expect("valid default multicast address"),
            tcp_server_port: 0,
            user_name: whoami::username(),
            terminal_bell: true,
            language: LanguageConfig::default(),
            ai: AiConfig::default(),
            security: SecurityConfig::default(),
            theme: Theme::default(),
            user: UserConfig::default(),
        }
    }
}

impl Config {
    pub fn config_dir_path_with_base(base: impl AsRef<std::path::Path>) -> PathBuf {
        base.as_ref().join("triadchat")
    }

    pub fn config_file_path_with_base(base: impl AsRef<std::path::Path>) -> PathBuf {
        Self::config_dir_path_with_base(base).join("config.toml")
    }

    pub fn config_dir_path() -> Option<PathBuf> {
        Some(Self::config_dir_path_with_base(dirs_next::config_dir()?))
    }

    pub fn config_file_path() -> Option<PathBuf> {
        Some(Self::config_file_path_with_base(dirs_next::config_dir()?))
    }

    fn from_config_file() -> Option<Self> {
        let config_dir_path = Self::config_dir_path()?;
        if let Err(error) = std::fs::create_dir_all(&config_dir_path) {
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return None;
            }
        }

        let config_file_path = Self::config_file_path()?;
        let create_config = |path: &PathBuf| -> Result<Config> {
            let config = Config::default();
            std::fs::write(path, toml::to_string(&config)?)?;
            Ok(config)
        };

        match std::fs::read_to_string(&config_file_path) {
            Ok(config) => toml::from_str(&config).ok(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                create_config(&config_file_path).ok()
            }
            Err(_) => None,
        }
    }

    pub fn from_matches(matches: &ArgMatches) -> Self {
        let mut config = Config::from_config_file().unwrap_or_default();

        if let Some(discovery_addr) = matches.get_one::<String>("discovery") {
            if let Ok(addr) = discovery_addr.parse() {
                config.discovery_addr = addr;
            }
        }
        if let Some(tcp_server_port) = matches.get_one::<String>("tcp_server_port") {
            if let Ok(port) = tcp_server_port.parse() {
                config.tcp_server_port = port;
            }
        }
        if let Some(user_name) = matches.get_one::<String>("username") {
            config.user_name = user_name.to_owned();
        }
        if matches.get_flag("quiet-mode") {
            config.terminal_bell = false;
        }
        if let Some(theme) = matches.get_one::<String>("theme") {
            config.theme = if theme == "dark" { Theme::dark_theme() } else { Theme::light_theme() };
        }

        config
    }
}

/// Per-user display preferences (serialised to `config.toml` under `[user]`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserConfig {
    /// Avatar preset name for the local user (default: `"human_default"`).
    pub avatar: String,
    /// Avatar preset name for the AI (default: `"ai_default"`).
    pub ai_avatar: String,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self { avatar: "human_default".into(), ai_avatar: "ai_default".into() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    #[default]
    Claude,
    Codex,
    Gemini,
    Custom,
}

impl AiProvider {
    pub fn invocation(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Claude => ("claude", &["-p"]),
            Self::Codex => ("codex", &["exec"]),
            Self::Gemini => ("gemini", &["-p"]),
            Self::Custom => ("", &[]),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    #[serde(default)]
    pub provider: AiProvider,
    pub command: Option<String>,
    pub timeout_secs: u64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self { enabled: true, provider: AiProvider::Claude, command: None, timeout_secs: 30 }
    }
}

#[cfg(test)]
mod tests {
    use super::{AiConfig, AiProvider};

    #[test]
    fn provider_invocation_matches_supported_clis() {
        assert_eq!(AiProvider::Claude.invocation(), ("claude", &["-p"][..]));
        assert_eq!(AiProvider::Codex.invocation(), ("codex", &["exec"][..]));
        assert_eq!(AiProvider::Gemini.invocation(), ("gemini", &["-p"][..]));
        assert_eq!(AiProvider::Custom.invocation(), ("", &[][..]));
    }

    #[test]
    fn ai_config_defaults_to_claude_provider() {
        assert_eq!(AiConfig::default().provider, AiProvider::Claude);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefaultPermission {
    ConfirmRequired,
    TrustedAutoSafe,
    DenyRemoteExec,
}

impl DefaultPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConfirmRequired => "confirm-required",
            Self::TrustedAutoSafe => "trusted-auto-safe",
            Self::DenyRemoteExec => "deny-remote-exec",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub default_permission: String,
    pub trusted_peers: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            default_permission: DefaultPermission::ConfirmRequired.as_str().into(),
            trusted_peers: Vec::new(),
        }
    }
}

impl SecurityConfig {
    pub fn default_permission_policy(&self) -> DefaultPermission {
        match self.default_permission.as_str() {
            "trusted-auto-safe" => DefaultPermission::TrustedAutoSafe,
            "deny-remote-exec" => DefaultPermission::DenyRemoteExec,
            _ => DefaultPermission::ConfirmRequired,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageConfig {
    pub ai_output: String,
    pub ui: String,
}

impl LanguageConfig {
    pub fn from_lang_env_value(lang: Option<&str>) -> Self {
        let normalized = lang
            .unwrap_or_default()
            .split('.')
            .next()
            .unwrap_or_default()
            .split('_')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();

        match normalized.as_str() {
            "en" => Self { ai_output: "en".into(), ui: "en".into() },
            "zh" => Self { ai_output: "zh".into(), ui: "en".into() },
            "ko" => Self { ai_output: "ko".into(), ui: "en".into() },
            _ => Self { ai_output: "ja".into(), ui: "ja".into() },
        }
    }

    pub fn from_lang_env(lang: Option<&str>) -> Self {
        Self::from_lang_env_value(lang)
    }
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self::from_lang_env_value(std::env::var("LANG").ok().as_deref())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Theme {
    pub message_colors: Vec<Color>,
    pub my_user_color: Color,
    pub date_color: Color,
    pub system_info_color: (Color, Color),
    pub system_warning_color: (Color, Color),
    pub system_error_color: (Color, Color),
    pub chat_panel_color: Color,
    pub progress_bar_color: Color,
    pub command_color: Color,
    pub input_panel_color: Color,
    pub mention_me_color: Color,
    pub mention_other_color: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark_theme()
    }
}

impl Theme {
    fn dark_theme() -> Self {
        Self {
            message_colors: vec![Color::Blue, Color::Yellow, Color::Cyan, Color::Magenta],
            my_user_color: Color::Green,
            date_color: Color::DarkGray,
            system_info_color: (Color::Cyan, Color::LightCyan),
            system_warning_color: (Color::Yellow, Color::LightYellow),
            system_error_color: (Color::Red, Color::LightRed),
            chat_panel_color: Color::White,
            progress_bar_color: Color::LightGreen,
            command_color: Color::LightYellow,
            input_panel_color: Color::White,
            mention_me_color: Color::Yellow,
            mention_other_color: Color::Cyan,
        }
    }

    fn light_theme() -> Self {
        Self {
            message_colors: vec![Color::Blue, Color::Yellow, Color::Cyan, Color::Magenta],
            my_user_color: Color::Green,
            date_color: Color::DarkGray,
            system_info_color: (Color::Cyan, Color::LightCyan),
            system_warning_color: (Color::Yellow, Color::LightYellow),
            system_error_color: (Color::Red, Color::LightRed),
            chat_panel_color: Color::Black,
            progress_bar_color: Color::LightGreen,
            command_color: Color::LightYellow,
            input_panel_color: Color::Black,
            mention_me_color: Color::Rgb(255, 215, 0), // Gold
            mention_other_color: Color::Blue,
        }
    }
}
