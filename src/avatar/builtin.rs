use super::{AvatarPlugin, AvatarSize, AvatarState};

// ─── human_default ────────────────────────────────────────────────────────────

struct HumanDefault;

impl AvatarPlugin for HumanDefault {
    fn preset_name(&self) -> &str {
        "human_default"
    }

    fn render(&self, state: AvatarState, size: AvatarSize) -> String {
        match size {
            AvatarSize::Compact => match state {
                AvatarState::Online | AvatarState::Idle => "[H]●".into(),
                AvatarState::Busy | AvatarState::Acting => "[H]◉".into(),
                AvatarState::Away => "[H]◌".into(),
                AvatarState::Offline | AvatarState::Disabled | AvatarState::Failed => "[H]○".into(),
                _ => "[H]·".into(),
            },
            AvatarSize::Normal => match state {
                AvatarState::Online | AvatarState::Idle => {
                    " (^_^)\n  |H|\n  / \\".into()
                }
                AvatarState::Busy | AvatarState::Acting => {
                    " (>_<)\n  |H|\n  / \\".into()
                }
                AvatarState::Away => {
                    " (-_-)\n  |H|\n  / \\".into()
                }
                AvatarState::Offline | AvatarState::Disabled | AvatarState::Failed => {
                    " (x_x)\n  |H|\n  / \\".into()
                }
                _ => " (o_o)\n  |H|\n  / \\".into(),
            },
            AvatarSize::Expressive => match state {
                AvatarState::Online | AvatarState::Idle => {
                    "  .-\"\"-.\n ( ^_^ )\n  \\|H|/\n  / | \\\n /  |  \\".into()
                }
                AvatarState::Busy | AvatarState::Acting => {
                    "  .-\"\"-.\n ( >_< )\n  \\|H|/\n  / | \\\n /  |  \\".into()
                }
                AvatarState::Away => {
                    "  .-\"\"-.\n ( -_- )\n  \\|H|/\n  / | \\\n /  |  \\".into()
                }
                AvatarState::Offline | AvatarState::Disabled | AvatarState::Failed => {
                    "  .-\"\"-.\n ( x_x )\n  \\|H|/\n  / | \\\n /  |  \\".into()
                }
                _ => {
                    "  .-\"\"-.\n ( o_o )\n  \\|H|/\n  / | \\\n /  |  \\".into()
                }
            },
        }
    }
}

/// Returns a `Box<dyn AvatarPlugin>` for the `human_default` preset.
pub fn human_default() -> Box<dyn AvatarPlugin> {
    Box::new(HumanDefault)
}

// ─── ai_default ───────────────────────────────────────────────────────────────

struct AiDefault;

impl AvatarPlugin for AiDefault {
    fn preset_name(&self) -> &str {
        "ai_default"
    }

    fn render(&self, state: AvatarState, size: AvatarSize) -> String {
        match size {
            AvatarSize::Compact => match state {
                AvatarState::Idle | AvatarState::Online => "[AI]◆".into(),
                AvatarState::Thinking => "[AI]…".into(),
                AvatarState::Acting => "[AI]▶".into(),
                AvatarState::Disabled => "[AI]□".into(),
                AvatarState::Failed => "[AI]✗".into(),
                _ => "[AI]·".into(),
            },
            AvatarSize::Normal => match state {
                AvatarState::Idle | AvatarState::Online => {
                    " [*_*]\n  |AI|\n  / \\".into()
                }
                AvatarState::Thinking => {
                    " [._.]  \n  |AI| ~\n  / \\".into()
                }
                AvatarState::Acting => {
                    " [>_>]\n  |AI|\n  >>\\".into()
                }
                AvatarState::Disabled => {
                    " [- -]\n  |AI|\n  / \\".into()
                }
                AvatarState::Failed => {
                    " [!_!]\n  |AI|\n  / \\".into()
                }
                _ => " [o_o]\n  |AI|\n  / \\".into(),
            },
            AvatarSize::Expressive => match state {
                AvatarState::Idle | AvatarState::Online => {
                    "  .---.\n |*_ *|\n  |AI|\n  / | \\\n /  |  \\".into()
                }
                AvatarState::Thinking => {
                    "  .---.\n |._. |  ~\n  |AI|\n  / | \\\n /  |  \\".into()
                }
                AvatarState::Acting => {
                    "  .---.\n |>_> |\n  |AI|\n  >> | \\\n />> |  \\".into()
                }
                AvatarState::Disabled => {
                    "  .---.\n |- - |\n  |AI|\n  / | \\\n /  |  \\".into()
                }
                AvatarState::Failed => {
                    "  .---.\n |!_! |\n  |AI|\n  / | \\\n /  |  \\".into()
                }
                _ => {
                    "  .---.\n |o_o |\n  |AI|\n  / | \\\n /  |  \\".into()
                }
            },
        }
    }
}

/// Returns a `Box<dyn AvatarPlugin>` for the `ai_default` preset.
pub fn ai_default() -> Box<dyn AvatarPlugin> {
    Box::new(AiDefault)
}

// ─── robot_guardian ───────────────────────────────────────────────────────────

struct RobotGuardian;

impl AvatarPlugin for RobotGuardian {
    fn preset_name(&self) -> &str {
        "robot_guardian"
    }

    fn render(&self, state: AvatarState, size: AvatarSize) -> String {
        match size {
            AvatarSize::Compact => match state {
                AvatarState::Idle | AvatarState::Online => "[RG]■".into(),
                AvatarState::Thinking => "[RG]⠿".into(),
                AvatarState::Acting => "[RG]⚡".into(),
                AvatarState::Disabled => "[RG]░".into(),
                AvatarState::Failed => "[RG]✗".into(),
                AvatarState::Busy => "[RG]◈".into(),
                AvatarState::Away => "[RG]◇".into(),
                AvatarState::Offline => "[RG]□".into(),
            },
            AvatarSize::Normal => match state {
                AvatarState::Idle | AvatarState::Online => {
                    " <|=|>\n [RG]\n  /|\\".into()
                }
                AvatarState::Thinking => {
                    " <|?|>\n [RG] ~\n  /|\\".into()
                }
                AvatarState::Acting => {
                    " <|!|>\n [RG]\n  >>\\".into()
                }
                AvatarState::Disabled => {
                    " <|-|>\n [RG]\n  /|\\".into()
                }
                AvatarState::Failed => {
                    " <|X|>\n [RG]\n  /|\\".into()
                }
                AvatarState::Busy => {
                    " <|*|>\n [RG]\n  /|\\".into()
                }
                AvatarState::Away => {
                    " <|.|>\n [RG]\n  /|\\".into()
                }
                AvatarState::Offline => {
                    " <| |>\n [RG]\n  /|\\".into()
                }
            },
            AvatarSize::Expressive => match state {
                AvatarState::Idle | AvatarState::Online => {
                    " ┌─────┐\n │ |=| │\n │[RG] │\n └──┬──┘\n   /|\\".into()
                }
                AvatarState::Thinking => {
                    " ┌─────┐\n │ |?| │~\n │[RG] │\n └──┬──┘\n   /|\\".into()
                }
                AvatarState::Acting => {
                    " ┌─────┐\n │ |!| │\n │[RG] │\n └──┬──┘\n  >>|\\".into()
                }
                AvatarState::Disabled => {
                    " ┌─────┐\n │ |-| │\n │[RG] │\n └──┬──┘\n   /|\\".into()
                }
                AvatarState::Failed => {
                    " ┌─────┐\n │ |X| │\n │[RG] │\n └──┬──┘\n   /|\\".into()
                }
                AvatarState::Busy => {
                    " ┌─────┐\n │ |*| │\n │[RG] │\n └──┬──┘\n   /|\\".into()
                }
                AvatarState::Away => {
                    " ┌─────┐\n │ |.| │\n │[RG] │\n └──┬──┘\n   /|\\".into()
                }
                AvatarState::Offline => {
                    " ┌─────┐\n │ | | │\n │[RG] │\n └──┬──┘\n   /|\\".into()
                }
            },
        }
    }
}

/// Returns a `Box<dyn AvatarPlugin>` for the `robot_guardian` preset.
pub fn robot_guardian() -> Box<dyn AvatarPlugin> {
    Box::new(RobotGuardian)
}

// ─── All builtin presets ─────────────────────────────────────────────────────

/// Returns all builtin preset plugins.
pub fn all_builtins() -> Vec<Box<dyn AvatarPlugin>> {
    vec![human_default(), ai_default(), robot_guardian()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_have_unique_preset_names() {
        let builtins = all_builtins();
        let mut names: Vec<String> = builtins.iter().map(|p| p.preset_name().to_owned()).collect();
        names.sort_unstable();
        let original_len = names.len();
        names.dedup();
        assert_eq!(names.len(), original_len, "Duplicate preset names found");
    }

    #[test]
    fn compact_is_single_line() {
        for plugin in all_builtins() {
            for state in [
                AvatarState::Idle,
                AvatarState::Thinking,
                AvatarState::Acting,
                AvatarState::Disabled,
                AvatarState::Failed,
                AvatarState::Online,
                AvatarState::Offline,
                AvatarState::Busy,
                AvatarState::Away,
            ] {
                let rendered = plugin.render(state.clone(), AvatarSize::Compact);
                assert!(
                    !rendered.contains('\n'),
                    "Compact render for '{}' {:?} must be single-line",
                    plugin.preset_name(),
                    state
                );
            }
        }
    }
}
