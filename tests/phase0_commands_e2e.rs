use tempfile::TempDir;

mod common;

use common::{config_with_ai_script, rendered_messages, write_executable_script};
use triadchat::application::Application;
use triadchat::config::{AiProvider, Config};
use triadchat::state::{AiMode, AiState, MessageType};

#[test]
fn summary_commands_and_auto_intervention_work_end_to_end() {
    let dir = TempDir::new().unwrap();
    let script = write_executable_script(
        dir.path(),
        "mock-claude.sh",
        "#!/bin/sh\ncase \"$2\" in\n  *TASK:summary*) printf 'INTENT: Summary\nTEXT: auth を service に切り出す。takuro が設計し、tanaka が回帰確認する。\nSTRUCTURED: {\"todos\":[{\"text\":\"auth の設計\",\"assignee\":\"takuro\"},{\"text\":\"回帰確認\",\"assignee\":\"tanaka\"}],\"decisions\":[\"auth は service に切り出す\"],\"skill_suggestions\":[]}\n' ;;\n  *TASK:todos*) printf 'INTENT: Todo\nTEXT: TODO を抽出しました\nSTRUCTURED: {\"todos\":[{\"text\":\"auth の設計\",\"assignee\":\"takuro\"}],\"decisions\":[],\"skill_suggestions\":[]}\n' ;;\n  *TASK:intervene*) printf 'INTENT: Summary\nTEXT: 決定事項を整理します\nSTRUCTURED: {\"todos\":[],\"decisions\":[\"auth は service に切り出す\"],\"skill_suggestions\":[]}\n' ;;\n  *) printf 'INTENT: Clarify\nTEXT: fallback\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\n' ;;\nesac\n",
    );

    let config = config_with_ai_script(&script, "takuro");
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("auth serviceに切り出したい").unwrap();
    app.handle_input_line_for_test("既存IFは維持したい").unwrap();
    app.handle_input_line_for_test("takuro が設計を書く").unwrap();

    assert!(app.state().ai_mode == AiMode::Clerk);
    assert!(!app.state().ai_thinking);

    app.handle_input_line_for_test("/summary").unwrap();
    app.handle_input_line_for_test("/todos").unwrap();

    let rendered = rendered_messages(&app);

    assert!(rendered.contains("auth を service に切り出す"));
    assert!(rendered.contains("takuro"));
}

#[test]
fn clerk_mode_intervenes_before_human_streak_limit_when_task_marker_exists() {
    let dir = TempDir::new().unwrap();
    let script = write_executable_script(
        dir.path(),
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'INTENT: Summary\nTEXT: 自動介入しました\nSTRUCTURED: {\"todos\":[{\"text\":\"auth の設計\",\"assignee\":\"takuro\"}],\"decisions\":[],\"skill_suggestions\":[]}\n'",
    );

    let config = config_with_ai_script(&script, "takuro");
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("takuro が auth の設計を書く").unwrap();

    let rendered = rendered_messages(&app);

    assert!(rendered.contains("自動介入しました"));
}

#[test]
fn moderator_mode_does_not_intervene_for_plain_task_language() {
    let dir = TempDir::new().unwrap();
    let script = write_executable_script(
        dir.path(),
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'INTENT: Summary\nTEXT: moderator intervened\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\n'",
    );

    let config = config_with_ai_script(&script, "takuro");
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
    let script = write_executable_script(
        dir.path(),
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'INTENT: Summary\nTEXT: noop\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\n'",
    );

    let config = config_with_ai_script(&script, "takuro");
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

    let rendered = rendered_messages(&app);

    assert!(rendered.contains("【 AI 】"));
    assert!(rendered.contains("【 Summary 】"));
    assert!(rendered.contains("【 Rooms 】"));
    assert!(rendered.contains("【 Peers 】"));
    assert!(rendered.contains("【 Skills 】"));
    assert!(rendered.contains("【 Avatar 】"));
    assert!(rendered.contains("【 Files 】"));
    assert!(rendered.contains("Change AI behaviour mode:"));
    assert!(rendered.contains("Summarise the conversation"));
    assert!(rendered.contains("Create a room with peers"));
    assert!(rendered.contains("Set avatar (target: self, @ops-ai)"));
    assert!(rendered.contains("Send a file to peers in the room"));
    assert!(rendered.contains("/ai mode <mode>"));
    assert!(rendered.contains("/ai provider <provider>"));
    assert!(rendered.contains("/room switch <id|name>"));
    assert!(rendered.contains("Switch active room"));
    assert!(rendered.contains("/peer connect <host:port>"));
    assert!(rendered.contains("/trust add <peer|fp>"));
    assert!(rendered.contains("/skill <name> [args]"));
    assert!(rendered.contains("Run a skill manually"));
    assert!(rendered.contains("/send <file>"));
    assert!(rendered.contains('\n'));
}

#[test]
fn ai_commands_report_human_readable_feedback() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/ai mode moderator").unwrap();
    app.handle_input_line_for_test("/ai freq low").unwrap();

    let rendered = rendered_messages(&app);

    assert!(rendered.contains("AI mode set to moderator"));
    assert!(rendered.contains("AI frequency set to low"));
}

#[test]
fn ai_provider_command_switches_active_provider() {
    let dir = TempDir::new().unwrap();
    let script =
        write_executable_script(dir.path(), "mock-claude.sh", "#!/bin/sh\nprintf 'noop\n'");
    let config = config_with_ai_script(&script, "takuro");
    let mut app = Application::new_for_test(&config).unwrap();

    assert_eq!(app.state().ai_provider, AiProvider::Claude);

    app.handle_input_line_for_test("/ai provider gemini").unwrap();

    assert_eq!(app.state().ai_provider, AiProvider::Gemini);
    assert_eq!(app.state().ai_state, AiState::Idle);
    assert!(!app.state().ai_thinking);
    let rendered = rendered_messages(&app);
    assert!(rendered.contains("AI provider set to gemini"));
}

#[test]
fn ai_provider_command_refused_while_ai_thinking() {
    let dir = TempDir::new().unwrap();
    let script =
        write_executable_script(dir.path(), "mock-claude.sh", "#!/bin/sh\nprintf 'noop\n'");
    let config = config_with_ai_script(&script, "takuro");
    let mut app = Application::new_for_test(&config).unwrap();

    app.set_ai_thinking_for_test(true);

    app.handle_input_line_for_test("/ai provider gemini").unwrap();

    // State must be untouched.
    assert_eq!(app.state().ai_provider, AiProvider::Claude);
    assert!(app.state().ai_thinking);
    let rendered = rendered_messages(&app);
    assert!(rendered.contains("Cannot switch AI provider while a request is in flight"));
    assert!(!rendered.contains("AI provider set to gemini"));
}

#[test]
fn ai_provider_command_refused_when_ai_disabled_in_config() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    assert_eq!(app.state().ai_provider, AiProvider::Claude);

    app.handle_input_line_for_test("/ai provider gemini").unwrap();

    assert_eq!(app.state().ai_provider, AiProvider::Claude);
    let rendered = rendered_messages(&app);
    assert!(rendered.contains("AI is disabled in config; cannot switch provider"));
    assert!(!rendered.contains("AI provider set to gemini"));
}

#[test]
fn ai_provider_command_failure_keeps_previous_provider() {
    // Initial config has a real script, so AiMediator builds on construction.
    let script_dir = TempDir::new().unwrap();
    let script =
        write_executable_script(script_dir.path(), "mock-claude.sh", "#!/bin/sh\nprintf 'noop\n'");
    let mut config = Config::default();
    config.ai.enabled = true;
    config.ai.command = Some(script.display().to_string());
    let mut app = Application::new_for_test(&config).unwrap();
    assert_eq!(app.state().ai_provider, AiProvider::Claude);

    // Drop the script dir so the command path no longer resolves. The handler
    // clones self.config.ai and re-checks command existence inside
    // SidecarAdapter::new, so the switch deterministically fails here without
    // depending on what provider binaries happen to be on PATH.
    drop(script_dir);

    app.handle_input_line_for_test("/ai provider gemini").unwrap();

    assert_eq!(app.state().ai_provider, AiProvider::Claude);
    let rendered = rendered_messages(&app);
    assert!(rendered.contains("Failed to set AI provider"));
    assert!(!rendered.contains("AI provider set to gemini"));
}

#[test]
fn ai_provider_command_defaults_to_claude_when_omitted() {
    let dir = TempDir::new().unwrap();
    let script =
        write_executable_script(dir.path(), "mock-claude.sh", "#!/bin/sh\nprintf 'noop\n'");
    let config = config_with_ai_script(&script, "takuro");
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/ai provider gemini").unwrap();
    assert_eq!(app.state().ai_provider, AiProvider::Gemini);

    app.handle_input_line_for_test("/ai provider").unwrap();

    assert_eq!(app.state().ai_provider, AiProvider::Claude);
    let rendered = rendered_messages(&app);
    assert!(rendered.contains("AI provider set to claude"));
}

#[test]
fn unknown_command_points_users_to_help() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/does-not-exist").unwrap();

    let rendered = app.state().messages().last().expect("unknown command message").rendered_text();

    assert_eq!(rendered, "Unknown command '/does-not-exist'. Type /help for available commands.");
}

#[test]
fn room_create_unknown_peer_points_to_peers_command() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/room create @carol").unwrap();

    let rendered = app.state().messages().last().expect("unknown peer message").rendered_text();

    assert_eq!(rendered, "unknown peer 'carol'. Use /peers to see connected peers.");
}

// ─── /room list edge ──────────────────────────────────────────────────────────

#[test]
fn room_list_when_no_rooms_emits_no_rooms() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/room list").unwrap();

    let rendered = rendered_messages(&app);
    assert!(rendered.contains("no rooms"));
}

// ─── /peers edge ──────────────────────────────────────────────────────────────

#[test]
fn peers_when_no_discovered_peers_emits_no_peers_discovered() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/peers").unwrap();

    let rendered = rendered_messages(&app);
    assert!(rendered.contains("no peers discovered"));
}

// ─── /run <id> edge ───────────────────────────────────────────────────────────

#[test]
fn run_with_unknown_proposal_id_shows_unknown_proposal() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/run 1").unwrap();

    let rendered = rendered_messages(&app);
    assert!(rendered.contains("unknown proposal id: 1"));
}

// ─── /cancel edge ─────────────────────────────────────────────────────────────

#[test]
fn cancel_with_no_active_task_emits_no_active_task() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/cancel").unwrap();

    let rendered = rendered_messages(&app);
    assert!(rendered.contains("no active task"));
}

// ─── /avatar set edge ─────────────────────────────────────────────────────────

#[test]
fn avatar_set_with_unknown_preset_warns_unknown_avatar_preset() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/avatar set self nonexistent_avatar").unwrap();

    let rendered = rendered_messages(&app);
    assert!(rendered.contains("Unknown avatar preset 'nonexistent_avatar'"));
}

#[test]
fn avatar_set_with_unknown_target_warns_unknown_target() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/avatar set nope_user robot_guardian").unwrap();

    let rendered = rendered_messages(&app);
    assert!(rendered.contains("Unknown target 'nope_user'"));
}

// ─── /avatar mode edge ────────────────────────────────────────────────────────

#[test]
fn avatar_mode_with_unknown_mode_warns_unknown_avatar_mode() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    app.handle_input_line_for_test("/avatar mode bogus").unwrap();

    let rendered = rendered_messages(&app);
    assert!(rendered.contains("Unknown avatar mode 'bogus'"));
}
