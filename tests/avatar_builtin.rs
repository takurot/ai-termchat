use triadchat::avatar::builtin::all_builtins;
use triadchat::avatar::{AvatarSize, AvatarState};

// ─── Builtin render invariants ────────────────────────────────────────────────

#[test]
fn compact_render_is_single_visible_line_for_every_builtin_state() {
    for plugin in all_builtins() {
        for state in all_states() {
            let rendered = plugin.render(state.clone(), AvatarSize::Compact);

            assert_eq!(
                rendered.len(),
                1,
                "{} compact render must fit the single-line peers panel for {state:?}",
                plugin.preset_name()
            );
            assert!(
                !render_text(&rendered).trim().is_empty(),
                "{} compact render must remain visible for {state:?}",
                plugin.preset_name()
            );
        }
    }
}

#[test]
fn active_ai_states_change_every_builtin_avatar() {
    for plugin in all_builtins() {
        let idle = plugin.render(AvatarState::Idle, AvatarSize::Normal);
        let thinking = plugin.render(AvatarState::Thinking, AvatarSize::Normal);
        let acting = plugin.render(AvatarState::Acting, AvatarSize::Normal);

        assert_ne!(
            idle,
            thinking,
            "{} should visually change while thinking",
            plugin.preset_name()
        );
        assert_ne!(idle, acting, "{} should visually change while acting", plugin.preset_name());
    }
}

#[test]
fn presence_states_change_human_facing_builtin_avatars() {
    for plugin in all_builtins() {
        let online = plugin.render(AvatarState::Online, AvatarSize::Compact);
        let away = plugin.render(AvatarState::Away, AvatarSize::Compact);
        let offline = plugin.render(AvatarState::Offline, AvatarSize::Compact);

        assert_ne!(
            render_text(&online),
            render_text(&away),
            "{} compact render should distinguish away from online",
            plugin.preset_name()
        );
        assert_ne!(
            render_text(&online),
            render_text(&offline),
            "{} compact render should distinguish offline from online",
            plugin.preset_name()
        );
    }
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn render_text(rendered: &[tui::text::Spans<'static>]) -> String {
    rendered
        .iter()
        .map(|line| line.0.iter().map(|span| span.content.as_ref()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

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
