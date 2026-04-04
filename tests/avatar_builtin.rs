use triadchat::avatar::{AvatarSize, AvatarState};
use triadchat::avatar::builtin::{ai_default, human_default, robot_guardian};

// ─── AvatarSize auto-selection ────────────────────────────────────────────────

#[test]
fn compact_size_selected_for_narrow_terminals() {
    assert_eq!(AvatarSize::for_width(79), AvatarSize::Compact);
    assert_eq!(AvatarSize::for_width(0), AvatarSize::Compact);
}

#[test]
fn normal_size_selected_for_standard_width() {
    assert_eq!(AvatarSize::for_width(80), AvatarSize::Normal);
    assert_eq!(AvatarSize::for_width(120), AvatarSize::Normal);
}

// ─── human_default ────────────────────────────────────────────────────────────

#[test]
fn human_default_returns_non_empty_for_all_states_and_sizes() {
    let plugin = human_default();
    for state in all_states() {
        for size in all_sizes() {
            let rendered = plugin.render(state.clone(), size);
            assert!(
                !rendered.is_empty(),
                "human_default rendered empty for {state:?} {size:?}"
            );
        }
    }
}

#[test]
fn human_default_compact_is_shorter_than_normal() {
    let plugin = human_default();
    let compact = plugin.render(AvatarState::Online, AvatarSize::Compact);
    let normal = plugin.render(AvatarState::Online, AvatarSize::Normal);
    assert!(
        compact.len() <= normal.len(),
        "Compact ({}) should be <= Normal ({})",
        compact.len(),
        normal.len()
    );
}

#[test]
fn human_default_preset_name_is_human_default() {
    assert_eq!(human_default().preset_name(), "human_default");
}

// ─── ai_default ───────────────────────────────────────────────────────────────

#[test]
fn ai_default_returns_non_empty_for_all_states_and_sizes() {
    let plugin = ai_default();
    for state in all_states() {
        for size in all_sizes() {
            let rendered = plugin.render(state.clone(), size);
            assert!(
                !rendered.is_empty(),
                "ai_default rendered empty for {state:?} {size:?}"
            );
        }
    }
}

#[test]
fn ai_default_thinking_differs_from_idle() {
    let plugin = ai_default();
    let idle = plugin.render(AvatarState::Idle, AvatarSize::Normal);
    let thinking = plugin.render(AvatarState::Thinking, AvatarSize::Normal);
    assert_ne!(idle, thinking, "idle and thinking should produce different renders");
}

#[test]
fn ai_default_acting_differs_from_idle() {
    let plugin = ai_default();
    let idle = plugin.render(AvatarState::Idle, AvatarSize::Normal);
    let acting = plugin.render(AvatarState::Acting, AvatarSize::Normal);
    assert_ne!(idle, acting, "idle and acting should produce different renders");
}

#[test]
fn ai_default_preset_name_is_ai_default() {
    assert_eq!(ai_default().preset_name(), "ai_default");
}

// ─── robot_guardian ───────────────────────────────────────────────────────────

#[test]
fn robot_guardian_returns_non_empty_for_all_states_and_sizes() {
    let plugin = robot_guardian();
    for state in all_states() {
        for size in all_sizes() {
            let rendered = plugin.render(state.clone(), size);
            assert!(
                !rendered.is_empty(),
                "robot_guardian rendered empty for {state:?} {size:?}"
            );
        }
    }
}

#[test]
fn robot_guardian_state_changes_are_reflected() {
    let plugin = robot_guardian();
    let idle = plugin.render(AvatarState::Idle, AvatarSize::Normal);
    let acting = plugin.render(AvatarState::Acting, AvatarSize::Normal);
    assert_ne!(idle, acting, "robot_guardian idle vs acting should differ");
}

#[test]
fn robot_guardian_preset_name_is_robot_guardian() {
    assert_eq!(robot_guardian().preset_name(), "robot_guardian");
}

// ─── AvatarPlugin trait object safety ────────────────────────────────────────

#[test]
fn all_builtins_can_be_used_as_trait_objects() {
    let plugins: Vec<Box<dyn triadchat::avatar::AvatarPlugin>> =
        vec![human_default(), ai_default(), robot_guardian()];
    for plugin in &plugins {
        let output = plugin.render(AvatarState::Idle, AvatarSize::Normal);
        assert!(!output.is_empty(), "trait object render for '{}' was empty", plugin.preset_name());
    }
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn all_states() -> Vec<AvatarState> {
    vec![
        AvatarState::Idle,
        AvatarState::Thinking,
        AvatarState::Acting,
        AvatarState::Disabled,
        AvatarState::Failed,
        AvatarState::Online,
        AvatarState::Offline,
        AvatarState::Busy,
        AvatarState::Away,
    ]
}

fn all_sizes() -> Vec<AvatarSize> {
    vec![AvatarSize::Compact, AvatarSize::Normal, AvatarSize::Expressive]
}
