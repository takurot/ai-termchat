use std::fs;
use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;

use triadchat::application::{Application, Signal};
use triadchat::config::Config;
use triadchat::message::{AiIntent, AiPayload, StructuredOutput};

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
args_hint: <ticket-id>
---
"#,
    )
    .unwrap();
    dir
}

#[test]
fn phase1_skill_commands_execute_without_polluting_transcript() {
    let workspace = create_skill_workspace();
    let script = write_script(
        &workspace,
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'review-auth finished successfully'\n",
    );

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();
    let node = app.node_handler();

    app.handle_input_line_for_test("/skills").unwrap();

    node.signals().send(Signal::AiResponse(
        AiPayload {
            text: "review-auth を実行".into(),
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
    app.handle_confirmation_input_for_test('y').unwrap();

    let rendered = app
        .state()
        .messages()
        .iter()
        .map(|message| message.rendered_text())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("name"));
    assert!(rendered.contains("risk"));
    assert!(rendered.contains("mode"));
    assert!(rendered.contains("description"));
    assert!(rendered.contains("review-auth"));
    assert!(rendered.contains("medium"));
    assert!(rendered.contains("confirm"));
    assert!(rendered.contains("args: <ticket-id>"));
    assert!(rendered.contains("review-auth finished successfully"));

    let transcript = app.state().transcript(20);
    assert!(!transcript.contains("/skills"));
    assert!(!transcript.contains("/run 1"));
    assert!(transcript.contains("review-auth"));
}
