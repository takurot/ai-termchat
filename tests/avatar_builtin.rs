use triadchat::avatar::{AvatarSize, AvatarState};
use triadchat::avatar::builtin::{ai_default, claude, human_default, neko, robot_guardian};

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
            assert!(!rendered.is_empty(), "human_default rendered empty for {state:?} {size:?}");
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
            assert!(!rendered.is_empty(), "ai_default rendered empty for {state:?} {size:?}");
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
            assert!(!rendered.is_empty(), "robot_guardian rendered empty for {state:?} {size:?}");
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

// ─── claude ───────────────────────────────────────────────────────────────────

#[test]
fn claude_preset_name_is_claude() {
    assert_eq!(claude().preset_name(), "claude");
}

#[test]
fn claude_returns_non_empty_for_all_states_and_sizes() {
    let plugin = claude();
    for state in all_states() {
        for size in all_sizes() {
            let rendered = plugin.render(state.clone(), size);
            assert!(!rendered.is_empty(), "claude rendered empty for {state:?} {size:?}");
        }
    }
}

#[test]
fn claude_compact_is_single_line_for_all_states() {
    let plugin = claude();
    for state in all_states() {
        let rendered = plugin.render(state.clone(), AvatarSize::Compact);
        assert!(!rendered.contains('\n'), "claude compact must be single-line for {state:?}");
    }
}

#[test]
fn claude_thinking_differs_from_idle() {
    let plugin = claude();
    let idle = plugin.render(AvatarState::Idle, AvatarSize::Normal);
    let thinking = plugin.render(AvatarState::Thinking, AvatarSize::Normal);
    assert_ne!(idle, thinking);
}

#[test]
fn claude_acting_differs_from_idle() {
    let plugin = claude();
    let idle = plugin.render(AvatarState::Idle, AvatarSize::Normal);
    let acting = plugin.render(AvatarState::Acting, AvatarSize::Normal);
    assert_ne!(idle, acting);
}

// ─── neko ─────────────────────────────────────────────────────────────────────

#[test]
fn neko_preset_name_is_neko() {
    assert_eq!(neko().preset_name(), "neko");
}

#[test]
fn neko_returns_non_empty_for_all_states_and_sizes() {
    let plugin = neko();
    for state in all_states() {
        for size in all_sizes() {
            let rendered = plugin.render(state.clone(), size);
            assert!(!rendered.is_empty(), "neko rendered empty for {state:?} {size:?}");
        }
    }
}

#[test]
fn neko_compact_is_single_line_for_all_states() {
    let plugin = neko();
    for state in all_states() {
        let rendered = plugin.render(state.clone(), AvatarSize::Compact);
        assert!(!rendered.contains('\n'), "neko compact must be single-line for {state:?}");
    }
}

#[test]
fn neko_online_differs_from_offline() {
    let plugin = neko();
    let online = plugin.render(AvatarState::Online, AvatarSize::Normal);
    let offline = plugin.render(AvatarState::Offline, AvatarSize::Normal);
    assert_ne!(online, offline);
}

#[test]
fn neko_busy_differs_from_online() {
    let plugin = neko();
    let online = plugin.render(AvatarState::Online, AvatarSize::Normal);
    let busy = plugin.render(AvatarState::Busy, AvatarSize::Normal);
    assert_ne!(online, busy);
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
