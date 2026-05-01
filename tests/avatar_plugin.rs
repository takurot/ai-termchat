use std::path::PathBuf;

use triadchat::avatar::builtin::ai_default;
use triadchat::avatar::loader::AvatarManager;
use triadchat::avatar::{AvatarSize, AvatarState};

// ─── Builtin fallback (no external dylibs) ───────────────────────────────────

#[test]
fn avatar_manager_registers_all_builtin_fallback_presets() {
    let manager = manager_without_plugin_dir();
    let presets = manager.list_all_presets();

    for expected in ["human_default", "ai_default", "robot_guardian", "claude", "neko"] {
        assert!(presets.iter().any(|preset| preset == expected), "missing builtin {expected}");
    }
}

// ─── render() falls back to builtins ─────────────────────────────────────────

#[test]
fn render_known_builtin_matches_direct_builtin_render() {
    let manager = manager_without_plugin_dir();
    let rendered = manager.render("ai_default", AvatarState::Idle, AvatarSize::Normal);
    let direct = ai_default().render(AvatarState::Idle, AvatarSize::Normal);

    assert_eq!(rendered, direct);
}

#[test]
fn render_unknown_preset_falls_back_to_ai_default() {
    let manager = manager_without_plugin_dir();

    for (state, size) in [
        (AvatarState::Thinking, AvatarSize::Compact),
        (AvatarState::Acting, AvatarSize::Normal),
        (AvatarState::Idle, AvatarSize::Expressive),
    ] {
        let rendered = manager.render("no_such_preset", state.clone(), size);
        let fallback = ai_default().render(state, size);

        assert_eq!(rendered, fallback, "unknown preset must exactly match ai_default fallback");
    }
}

// ─── empty dir is tolerated (no panic) ──────────────────────────────────────

#[test]
fn avatar_manager_with_empty_plugin_dir_does_not_panic() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = AvatarManager::new(tmp.path().to_path_buf());
    assert!(!manager.list_all_presets().is_empty());
}

fn manager_without_plugin_dir() -> AvatarManager {
    AvatarManager::new(PathBuf::from("/tmp/triadchat-test-no-such-avatars-dir-xyz"))
}
