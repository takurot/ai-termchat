use std::path::Path;

use triadchat::commands::CommandManager;
use triadchat::config::{Config, LanguageConfig};

#[test]
fn command_prefix_is_slash() {
    assert_eq!(CommandManager::COMMAND_PREFIX, "/");
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
    assert_eq!(fallback.ai_output, "ja");
    assert_eq!(fallback.ui, "ja");
}
