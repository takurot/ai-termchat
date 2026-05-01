use std::fs;

use tempfile::TempDir;

mod common;

use common::{rendered_messages, write_executable_script};
use triadchat::application::Application;
use triadchat::config::Config;

fn create_confirm_skill_workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    let skills_dir = dir.path().join(".claude/skills/deploy-prod");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        r#"---
name: deploy-prod
invoke: confirm
risk: high
allowed-tools: [Bash]
description: Deploy to production
---
"#,
    )
    .unwrap();
    dir
}

/// Regression test for issue #4: confirmation prompt must be in English.
#[test]
fn skill_confirmation_prompt_is_in_english() {
    let workspace = create_confirm_skill_workspace();
    let mut config = Config::default();
    config.ai.command = Some("true".into());
    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();

    app.handle_input_line_for_test("/skill deploy-prod").unwrap();

    let rendered = rendered_messages(&app);

    assert!(
        rendered.contains("[deploy-prod] Execute this skill? [y/n]"),
        "Confirmation prompt should be in English with skill name, got:\n{rendered}"
    );
    assert!(
        !rendered.contains("実行しますか"),
        "Confirmation prompt must not contain Japanese, got:\n{rendered}"
    );
}

/// Regression test for issue #4: running message must be in English.
#[test]
fn skill_running_message_is_in_english() {
    let workspace = create_confirm_skill_workspace();
    let script = write_executable_script(
        workspace.path(),
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'deploy-prod finished'\n",
    );

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();

    app.handle_input_line_for_test("/skill deploy-prod").unwrap();
    app.handle_confirmation_input_for_test('y').unwrap();

    let rendered = rendered_messages(&app);

    assert!(
        rendered.contains("[ops-ai: running /deploy-prod...]"),
        "Running message should be in English, got:\n{rendered}"
    );
    assert!(
        !rendered.contains("実行中"),
        "Running message must not contain Japanese, got:\n{rendered}"
    );
}
