use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

use triadchat::application::Application;
use triadchat::config::Config;
use triadchat::state::{AiMode, MessageType};

fn write_script(dir: &TempDir, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

#[test]
fn summary_commands_and_auto_intervention_work_end_to_end() {
    let dir = TempDir::new().unwrap();
    let script = write_script(
        &dir,
        "mock-claude.sh",
        "#!/bin/sh\ncase \"$2\" in\n  *TASK:summary*) printf 'INTENT: Summary\nTEXT: auth を service に切り出す。takuro が設計し、tanaka が回帰確認する。\nSTRUCTURED: {\"todos\":[{\"text\":\"auth の設計\",\"assignee\":\"takuro\"},{\"text\":\"回帰確認\",\"assignee\":\"tanaka\"}],\"decisions\":[\"auth は service に切り出す\"],\"skill_suggestions\":[]}\n' ;;\n  *TASK:todos*) printf 'INTENT: Todo\nTEXT: TODO を抽出しました\nSTRUCTURED: {\"todos\":[{\"text\":\"auth の設計\",\"assignee\":\"takuro\"}],\"decisions\":[],\"skill_suggestions\":[]}\n' ;;\n  *TASK:intervene*) printf 'INTENT: Summary\nTEXT: 決定事項を整理します\nSTRUCTURED: {\"todos\":[],\"decisions\":[\"auth は service に切り出す\"],\"skill_suggestions\":[]}\n' ;;\n  *) printf 'INTENT: Clarify\nTEXT: fallback\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\n' ;;\nesac\n",
    );

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("auth serviceに切り出したい").unwrap();
    app.handle_input_line_for_test("既存IFは維持したい").unwrap();
    app.handle_input_line_for_test("takuro が設計を書く").unwrap();

    assert!(app.state().ai_mode == AiMode::Clerk);
    assert!(!app.state().ai_thinking);

    app.handle_input_line_for_test("/summary").unwrap();
    app.handle_input_line_for_test("/todos").unwrap();

    let rendered = app
        .state()
        .messages()
        .iter()
        .filter_map(|message| match &message.message_type {
            MessageType::Text(text) | MessageType::AiText(text) => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("auth を service に切り出す"));
    assert!(rendered.contains("takuro"));
}

#[test]
fn clerk_mode_intervenes_before_human_streak_limit_when_task_marker_exists() {
    let dir = TempDir::new().unwrap();
    let script = write_script(
        &dir,
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'INTENT: Summary\nTEXT: 自動介入しました\nSTRUCTURED: {\"todos\":[{\"text\":\"auth の設計\",\"assignee\":\"takuro\"}],\"decisions\":[],\"skill_suggestions\":[]}\n'",
    );

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("takuro が auth の設計を書く").unwrap();

    let rendered = app
        .state()
        .messages()
        .iter()
        .filter_map(|message| match &message.message_type {
            MessageType::AiText(text) => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("自動介入しました"));
}

#[test]
fn moderator_mode_does_not_intervene_for_plain_task_language() {
    let dir = TempDir::new().unwrap();
    let script = write_script(
        &dir,
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'INTENT: Summary\nTEXT: moderator intervened\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\n'",
    );

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/ai mode moderator").unwrap();
    app.handle_input_line_for_test("takuro が auth の設計を書く").unwrap();
    app.handle_input_line_for_test("tanaka が回帰確認を書く").unwrap();
    app.handle_input_line_for_test("sato が README を直す").unwrap();

    let ai_messages = app
        .state()
        .messages()
        .iter()
        .filter(|message| matches!(message.message_type, MessageType::AiText(_)))
        .count();

    assert_eq!(ai_messages, 0);
}

#[test]
fn slash_commands_are_not_included_in_transcript() {
    let dir = TempDir::new().unwrap();
    let script = write_script(
        &dir,
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'INTENT: Summary\nTEXT: noop\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\n'",
    );

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("auth serviceに切り出したい").unwrap();
    app.handle_input_line_for_test("/summary").unwrap();

    let transcript = app.state().transcript(10);
    assert!(transcript.contains("auth serviceに切り出したい"));
    assert!(!transcript.contains("/summary"));
}

#[test]
fn help_command_groups_commands_with_descriptions() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/help").unwrap();

    let rendered = app
        .state()
        .messages()
        .iter()
        .map(|message| message.rendered_text())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("AI"));
    assert!(rendered.contains("Summary"));
    assert!(rendered.contains("Rooms"));
    assert!(rendered.contains("Skills"));
    assert!(rendered.contains("Avatar"));
    assert!(rendered.contains("Files"));
    assert!(rendered.contains("Change AI behaviour mode"));
    assert!(rendered.contains("Summarise the conversation"));
    assert!(rendered.contains("Create a room with peers"));
    assert!(rendered.contains("Set avatar preset"));
    assert!(rendered.contains("Send a file to peers"));
    assert!(rendered.contains("/ai mode <clerk|listener|moderator|operator|companion>"));
    assert!(rendered.contains("/room switch <room_id>"));
    assert!(rendered.contains("Switch active room"));
    assert!(rendered.contains("/skill <name> [args]"));
    assert!(rendered.contains("Run a skill"));
    assert!(rendered.contains("/send <file>"));
    assert!(rendered.contains('\n'));
}
