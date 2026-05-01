use std::fs;
use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;

use triadchat::application::{Application, Signal};
use triadchat::config::{AiConfig, Config};
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

// ─── confirmation flow: /run + pending + 'n' ─────────────────────────────────

#[test]
fn confirmation_flow_cancels_with_n() {
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
    app.handle_confirmation_input_for_test('n').unwrap();

    assert!(app.state().pending_confirmation().is_none(), "pending confirmation should be cleared");

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");
    assert!(rendered.contains("skill execution cancelled"));
    assert!(
        !rendered.contains("review-auth finished successfully"),
        "skill should not have executed"
    );
}

// ─── /cancel with pending confirmation, no running task ──────────────────────

#[test]
fn cancel_with_pending_confirmation_but_no_running_task_cancels_confirmation() {
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
    assert!(
        app.state().pending_confirmation().is_some(),
        "should have pending confirmation after /run"
    );
    app.handle_input_line_for_test("/cancel").unwrap();
    assert!(
        app.state().pending_confirmation().is_none(),
        "pending confirmation should be cleared after /cancel"
    );

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");
    assert!(rendered.contains("skill execution cancelled"));
}

// ─── /run <id> for untrusted remote proposal ─────────────────────────────────

#[test]
fn run_proposal_from_untrusted_remote_peer_is_permission_denied() {
    let workspace = create_skill_workspace();
    let script = write_script(&workspace, "mock-claude.sh", "#!/bin/sh\nprintf 'noop'\n");

    let config = Config {
        user_name: "takuro".into(),
        ai: AiConfig { command: Some(script.display().to_string()), ..Default::default() },
        ..Default::default()
    };

    let mut takuro = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();

    let payload = AiPayload {
        text: "review-auth suggested".into(),
        intent: AiIntent::SkillSuggest,
        structured: Some(StructuredOutput {
            todos: Vec::new(),
            decisions: Vec::new(),
            skill_suggestions: vec!["review-auth".into()],
            raw_text: None,
        }),
    };
    takuro.inject_remote_ai_response_for_test("tanaka", payload);

    takuro.handle_input_line_for_test("/run 1").unwrap();

    assert!(
        takuro.state().pending_confirmation().is_none(),
        "permission denied should not leave pending confirmation"
    );

    let rendered =
        takuro.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");
    assert!(rendered.contains("permission denied"));
    assert!(rendered.contains("untrusted peer tanaka"));
    assert!(
        !rendered.contains("noop"),
        "mock script output should not appear when permission denied"
    );
}
