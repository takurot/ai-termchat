use std::path::Path;

use triadchat::commands::{AppCommand, CommandManager, ParsedCommand};
use triadchat::config::{Config, LanguageConfig};

#[test]
fn slash_prefixed_help_parses_but_plain_text_does_not() {
    let manager = CommandManager::default();

    let parsed = manager.find_command("/help").expect("/help should be recognized").unwrap();
    assert!(matches!(parsed, ParsedCommand::App(AppCommand::Help)));
    assert!(manager.find_command("help").is_none());
    assert!(manager.find_command("hello /help").is_none());
}

#[test]
fn config_path_uses_triadchat_directory_and_toml_name() {
    let base = Path::new("/tmp/example-config");
    assert_eq!(Config::config_dir_path_with_base(base), base.join("triadchat"));
    assert_eq!(
        Config::config_file_path_with_base(base),
        base.join("triadchat").join("config.toml")
    );
}

#[test]
fn language_defaults_follow_lang_environment() {
    let ja = LanguageConfig::from_lang_env_value(Some("ja_JP.UTF-8"));
    assert_eq!(ja.ai_output, "ja");
    assert_eq!(ja.ui, "ja");

    let en = LanguageConfig::from_lang_env_value(Some("en_US.UTF-8"));
    assert_eq!(en.ai_output, "en");
    assert_eq!(en.ui, "en");

    let zh = LanguageConfig::from_lang_env_value(Some("zh_CN.UTF-8"));
    assert_eq!(zh.ai_output, "zh");
    assert_eq!(zh.ui, "en");

    let fallback = LanguageConfig::from_lang_env_value(Some("unknown"));
    assert_eq!(fallback.ai_output, "en");
    assert_eq!(fallback.ui, "en");
}
