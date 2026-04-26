use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use tempfile::TempDir;

use triadchat::application::{Application, Signal};
use triadchat::config::Config;
use triadchat::message::{AiIntent, AiPayload, StructuredOutput};
use triadchat::skill::registry::{InvokeMode, RiskLevel};
use triadchat::state::AiState;

fn write_script(dir: &TempDir, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

fn create_skill_workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    let skills_dir = dir.path().join(".claude/skills/review-auth");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        r#"---
name: review-auth
invoke: confirm
risk: medium
allowed-tools: [Read, Grep]
description: 認証ロジックをレビューする
---
"#,
    )
    .unwrap();
    dir
}

fn create_suggest_skill_workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    let skills_dir = dir.path().join(".claude/skills/summarise");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        r#"---
name: summarise
invoke: suggest
risk: low
description: Summarise text
---
"#,
    )
    .unwrap();
    dir
}

#[test]
fn skill_command_requires_confirmation_then_posts_result() {
    let workspace = create_skill_workspace();
    let script = write_script(
        &workspace,
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'review-auth finished successfully'\n",
    );

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();

    app.handle_input_line_for_test("/skill review-auth").unwrap();

    let pending = app.state().pending_confirmation().expect("confirmation should be pending");
    assert_eq!(pending.meta.name, "review-auth");
    assert_eq!(pending.meta.invoke_mode, InvokeMode::Confirm);
    assert_eq!(pending.meta.risk, RiskLevel::Medium);

    app.handle_confirmation_input_for_test('y').unwrap();

    let rendered = app
        .state()
        .messages()
        .iter()
        .map(|message| message.rendered_text())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("review-auth finished successfully"));
    assert_eq!(app.state().ai_state, AiState::Idle);
}

#[test]
fn cancel_stops_running_skill_task() {
    let workspace = create_skill_workspace();
    let script =
        write_script(&workspace, "slow-claude.sh", "#!/bin/sh\nsleep 2\nprintf 'late result'\n");

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    let mut app = Application::new_in_workspace(&config, workspace.path()).unwrap();

    app.handle_input_line_for_test("/skill review-auth").unwrap();
    app.handle_confirmation_input_for_test('y').unwrap();
    std::thread::sleep(Duration::from_millis(50));
    app.handle_input_line_for_test("/cancel").unwrap();

    assert_eq!(app.state().ai_state, AiState::Idle);
}

#[test]
fn run_uses_pending_skill_proposals_from_ai_response() {
    let workspace = create_skill_workspace();
    let script =
        write_script(&workspace, "mock-claude.sh", "#!/bin/sh\nprintf 'proposal executed'\n");

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();
    let node = app.node_handler();

    node.signals().send(Signal::AiResponse(
        AiPayload {
            text: "Using review-auth skill".into(),
            intent: AiIntent::SkillSuggest,
            structured: Some(StructuredOutput {
                todos: Vec::new(),
                decisions: Vec::new(),
                skill_suggestions: vec!["review-auth".into()],
                raw_text: None,
            }),
        },
        false,
    ));
    app.process_next_event_for_test().unwrap();

    app.handle_input_line_for_test("/run 1").unwrap();
    let pending = app.state().pending_confirmation().expect("proposal should require confirmation");
    assert_eq!(pending.meta.name, "review-auth");

    app.handle_confirmation_input_for_test('y').unwrap();

    let rendered = app
        .state()
        .messages()
        .iter()
        .map(|message| message.rendered_text())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("proposal executed"));
}

#[test]
fn suggest_only_skill_explains_how_to_run_it() {
    let workspace = create_suggest_skill_workspace();
    let mut config = Config::default();
    config.ai.command = Some("true".into());
    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();

    app.handle_input_line_for_test("/skill summarise").unwrap();

    let rendered = app
        .state()
        .messages()
        .iter()
        .map(|message| message.rendered_text())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("Skill 'summarise' is propose-only"));
    assert!(rendered.contains("/run <id>"));
}
