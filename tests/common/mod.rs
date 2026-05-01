#![allow(dead_code)]

use std::path::{Path, PathBuf};

use triadchat::application::Application;
use triadchat::config::{AiConfig, Config};

pub fn write_executable_script(dir: impl AsRef<Path>, name: &str, contents: &str) -> PathBuf {
    let path = dir.as_ref().join(name);
    std::fs::write(&path, contents).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o755);
    }
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

pub fn rendered_messages(app: &Application<'_>) -> String {
    app.state()
        .messages()
        .iter()
        .map(|message| message.rendered_text())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn config_with_ai_script(script: impl AsRef<Path>, user_name: &str) -> Config {
    Config {
        user_name: user_name.to_string(),
        ai: AiConfig { command: Some(script.as_ref().display().to_string()), ..Default::default() },
        ..Default::default()
    }
}
