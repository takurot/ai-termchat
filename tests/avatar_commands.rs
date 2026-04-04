use triadchat::commands::{AppCommand, AvatarCommandKind, CommandManager, ParsedCommand};
use triadchat::commands::avatar_cmd::AvatarCommand;

fn manager() -> CommandManager {
    CommandManager::default().with(AvatarCommand)
}

// ─── /avatar list ─────────────────────────────────────────────────────────────

#[test]
fn avatar_list_parses() {
    let result = manager().find_command("/avatar list");
    assert!(result.is_some(), "command must be recognised");
    let cmd = result.unwrap().expect("must parse without error");
    assert!(matches!(
        cmd,
        ParsedCommand::App(AppCommand::Avatar(AvatarCommandKind::List))
    ));
}

// ─── /avatar preview ─────────────────────────────────────────────────────────

#[test]
fn avatar_preview_parses() {
    let cmd = manager().find_command("/avatar preview").unwrap().unwrap();
    assert!(matches!(cmd, ParsedCommand::App(AppCommand::Avatar(AvatarCommandKind::Preview))));
}

// ─── /avatar set ─────────────────────────────────────────────────────────────

#[test]
fn avatar_set_parses_target_and_preset() {
    let cmd = manager()
        .find_command("/avatar set @ops-ai robot_guardian")
        .unwrap()
        .unwrap();
    match cmd {
        ParsedCommand::App(AppCommand::Avatar(AvatarCommandKind::Set { target, preset })) => {
            assert_eq!(target, "@ops-ai");
            assert_eq!(preset, "robot_guardian");
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn avatar_set_missing_preset_returns_error() {
    let result = manager().find_command("/avatar set self").unwrap();
    assert!(result.is_err(), "missing preset argument must return an error");
}

#[test]
fn avatar_set_missing_target_returns_error() {
    let result = manager().find_command("/avatar set").unwrap();
    assert!(result.is_err(), "missing target must return an error");
}

// ─── /avatar mode ─────────────────────────────────────────────────────────────

#[test]
fn avatar_mode_compact_parses() {
    let cmd = manager().find_command("/avatar mode compact").unwrap().unwrap();
    match cmd {
        ParsedCommand::App(AppCommand::Avatar(AvatarCommandKind::Mode(m))) => {
            assert_eq!(m, "compact");
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn avatar_mode_expressive_parses() {
    let cmd = manager().find_command("/avatar mode expressive").unwrap().unwrap();
    match cmd {
        ParsedCommand::App(AppCommand::Avatar(AvatarCommandKind::Mode(m))) => {
            assert_eq!(m, "expressive");
        }
        _ => panic!("unexpected command variant"),
    }
}

// ─── unknown subcommand ───────────────────────────────────────────────────────

#[test]
fn avatar_unknown_subcommand_returns_error() {
    let result = manager().find_command("/avatar unknown_cmd").unwrap();
    assert!(result.is_err(), "unknown subcommand must return an error");
}
