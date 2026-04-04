use std::path::PathBuf;

use triadchat::avatar::loader::AvatarManager;
use triadchat::avatar::{AvatarSize, AvatarState};

// ─── Builtin fallback (no external dylibs) ───────────────────────────────────

#[test]
fn avatar_manager_loads_builtins_when_no_plugin_dir() {
    let non_existent = PathBuf::from("/tmp/triadchat-test-no-such-avatars-dir-xyz");
    let manager = AvatarManager::new(non_existent);
    let presets = manager.list_all_presets();
    assert!(!presets.is_empty(), "builtin presets must always be available");
}

#[test]
fn avatar_manager_includes_builtin_human_default() {
    let manager = AvatarManager::new(PathBuf::from("/tmp/triadchat-test-no-such-avatars-dir-xyz"));
    let presets = manager.list_all_presets();
    assert!(
        presets.iter().any(|p| p == "human_default"),
        "human_default must be a builtin preset; got: {presets:?}"
    );
}

#[test]
fn avatar_manager_includes_builtin_ai_default() {
    let manager = AvatarManager::new(PathBuf::from("/tmp/triadchat-test-no-such-avatars-dir-xyz"));
    let presets = manager.list_all_presets();
    assert!(presets.iter().any(|p| p == "ai_default"));
}

#[test]
fn avatar_manager_includes_builtin_robot_guardian() {
    let manager = AvatarManager::new(PathBuf::from("/tmp/triadchat-test-no-such-avatars-dir-xyz"));
    let presets = manager.list_all_presets();
    assert!(presets.iter().any(|p| p == "robot_guardian"));
}

// ─── render() falls back to builtins ─────────────────────────────────────────

#[test]
fn render_known_preset_returns_non_empty() {
    let manager = AvatarManager::new(PathBuf::from("/tmp/triadchat-test-no-such-avatars-dir-xyz"));
    let rendered = manager.render("ai_default", AvatarState::Idle, AvatarSize::Normal);
    assert!(!rendered.is_empty());
}

#[test]
fn render_unknown_preset_falls_back_to_ai_default() {
    let manager = AvatarManager::new(PathBuf::from("/tmp/triadchat-test-no-such-avatars-dir-xyz"));
    let rendered = manager.render("no_such_preset", AvatarState::Idle, AvatarSize::Normal);
    assert!(!rendered.is_empty(), "fallback must always produce output");
}

// ─── list_all_presets deduplicates ───────────────────────────────────────────

#[test]
fn list_all_presets_has_no_duplicates() {
    let manager = AvatarManager::new(PathBuf::from("/tmp/triadchat-test-no-such-avatars-dir-xyz"));
    let mut presets = manager.list_all_presets();
    presets.sort_unstable();
    let original_len = presets.len();
    presets.dedup();
    assert_eq!(presets.len(), original_len, "preset list must not contain duplicates");
}

// ─── empty dir is tolerated (no panic) ──────────────────────────────────────

#[test]
fn avatar_manager_with_empty_plugin_dir_does_not_panic() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = AvatarManager::new(tmp.path().to_path_buf());
    assert!(!manager.list_all_presets().is_empty());
}
