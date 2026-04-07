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
                AvatarState::Online | AvatarState::Idle => " (^_^)\n  |H|\n  / \\".into(),
                AvatarState::Busy | AvatarState::Acting => " (>_<)\n  |H|\n  / \\".into(),
                AvatarState::Away => " (-_-)\n  |H|\n  / \\".into(),
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
                AvatarState::Away => "  .-\"\"-.\n ( -_- )\n  \\|H|/\n  / | \\\n /  |  \\".into(),
                AvatarState::Offline | AvatarState::Disabled | AvatarState::Failed => {
                    "  .-\"\"-.\n ( x_x )\n  \\|H|/\n  / | \\\n /  |  \\".into()
                }
                _ => "  .-\"\"-.\n ( o_o )\n  \\|H|/\n  / | \\\n /  |  \\".into(),
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
                AvatarState::Idle | AvatarState::Online => " [*_*]\n  |AI|\n  / \\".into(),
                AvatarState::Thinking => " [._.]  \n  |AI| ~\n  / \\".into(),
                AvatarState::Acting => " [>_>]\n  |AI|\n  >>\\".into(),
                AvatarState::Disabled => " [- -]\n  |AI|\n  / \\".into(),
                AvatarState::Failed => " [!_!]\n  |AI|\n  / \\".into(),
                _ => " [o_o]\n  |AI|\n  / \\".into(),
            },
            AvatarSize::Expressive => match state {
                AvatarState::Idle | AvatarState::Online => {
                    "  .---.\n |*_ *|\n  |AI|\n  / | \\\n /  |  \\".into()
                }
                AvatarState::Thinking => "  .---.\n |._. |  ~\n  |AI|\n  / | \\\n /  |  \\".into(),
                AvatarState::Acting => "  .---.\n |>_> |\n  |AI|\n  >> | \\\n />> |  \\".into(),
                AvatarState::Disabled => "  .---.\n |- - |\n  |AI|\n  / | \\\n /  |  \\".into(),
                AvatarState::Failed => "  .---.\n |!_! |\n  |AI|\n  / | \\\n /  |  \\".into(),
                _ => "  .---.\n |o_o |\n  |AI|\n  / | \\\n /  |  \\".into(),
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
                AvatarState::Idle | AvatarState::Online => " <|=|>\n [RG]\n  /|\\".into(),
                AvatarState::Thinking => " <|?|>\n [RG] ~\n  /|\\".into(),
                AvatarState::Acting => " <|!|>\n [RG]\n  >>\\".into(),
                AvatarState::Disabled => " <|-|>\n [RG]\n  /|\\".into(),
                AvatarState::Failed => " <|X|>\n [RG]\n  /|\\".into(),
                AvatarState::Busy => " <|*|>\n [RG]\n  /|\\".into(),
                AvatarState::Away => " <|.|>\n [RG]\n  /|\\".into(),
                AvatarState::Offline => " <| |>\n [RG]\n  /|\\".into(),
            },
            AvatarSize::Expressive => match state {
                AvatarState::Idle | AvatarState::Online => {
                    " ┌─────┐\n │ |=| │\n │[RG] │\n └──┬──┘\n   /|\\".into()
                }
                AvatarState::Thinking => " ┌─────┐\n │ |?| │~\n │[RG] │\n └──┬──┘\n   /|\\".into(),
                AvatarState::Acting => " ┌─────┐\n │ |!| │\n │[RG] │\n └──┬──┘\n  >>|\\".into(),
                AvatarState::Disabled => " ┌─────┐\n │ |-| │\n │[RG] │\n └──┬──┘\n   /|\\".into(),
                AvatarState::Failed => " ┌─────┐\n │ |X| │\n │[RG] │\n └──┬──┘\n   /|\\".into(),
                AvatarState::Busy => " ┌─────┐\n │ |*| │\n │[RG] │\n └──┬──┘\n   /|\\".into(),
                AvatarState::Away => " ┌─────┐\n │ |.| │\n │[RG] │\n └──┬──┘\n   /|\\".into(),
                AvatarState::Offline => " ┌─────┐\n │ | | │\n │[RG] │\n └──┬──┘\n   /|\\".into(),
            },
        }
    }
}

/// Returns a `Box<dyn AvatarPlugin>` for the `robot_guardian` preset.
pub fn robot_guardian() -> Box<dyn AvatarPlugin> {
    Box::new(RobotGuardian)
}

// ─── claude ───────────────────────────────────────────────────────────────────

struct ClaudeAvatar;

impl AvatarPlugin for ClaudeAvatar {
    fn preset_name(&self) -> &str {
        "claude"
    }

    fn render(&self, state: AvatarState, size: AvatarSize) -> String {
        match size {
            AvatarSize::Compact => match state {
                AvatarState::Idle | AvatarState::Online => "◈(・ω・)".into(),
                AvatarState::Thinking => "◉(・・・)".into(),
                AvatarState::Acting => "▶(☆ω☆)".into(),
                AvatarState::Failed => "✕(×_×)".into(),
                AvatarState::Disabled => "○(・ー・)".into(),
                AvatarState::Busy => "◉(>ω<)".into(),
                AvatarState::Away => "◈(-ω-)".into(),
                AvatarState::Offline => "○(._.)".into(),
            },
            AvatarSize::Normal => match state {
                AvatarState::Idle | AvatarState::Online => " (・ω・)\n╰[claude]╯\n  /   \\".into(),
                AvatarState::Thinking => " (・・・)\n╰[claude]╯ ~\n  /   \\".into(),
                AvatarState::Acting => " (☆ω☆)\n╰[claude]╯>>\n  >>  \\".into(),
                AvatarState::Failed => " (×_×)\n╰[claude]╯\n  /   \\".into(),
                AvatarState::Disabled => " (・ー・)\n╰[claude]╯\n  /   \\".into(),
                AvatarState::Busy => " (>ω<)\n╰[claude]╯\n  / ! \\".into(),
                AvatarState::Away => " (-ω-)\n╰[claude]╯\n  /   \\".into(),
                AvatarState::Offline => " (._. )\n╰[claude]╯\n  /   \\".into(),
            },
            AvatarSize::Expressive => match state {
                AvatarState::Idle | AvatarState::Online => {
                    "╭──────╮\n│ ◕ω◕  │\n│claude│\n╰──┬───╯\n __|__\n/     \\".into()
                }
                AvatarState::Thinking => {
                    "╭──────╮\n│ ◉ ◉  │~\n│claude│\n╰──┬───╯\n __|__\n/     \\".into()
                }
                AvatarState::Acting => {
                    "╭──────╮\n│ ☆ω☆  │\n│claude│\n╰──┬───╯\n >>|__\n/>>   \\".into()
                }
                AvatarState::Failed => {
                    "╭──────╮\n│ ×_×  │\n│claude│\n╰──┬───╯\n __|__\n/     \\".into()
                }
                AvatarState::Disabled => {
                    "╭──────╮\n│ ・ー・ │\n│claude│\n╰──┬───╯\n __|__\n/     \\".into()
                }
                AvatarState::Busy => {
                    "╭──────╮\n│ >ω<  │\n│claude│\n╰──┬───╯\n __|__\n/ ! \\".into()
                }
                AvatarState::Away => {
                    "╭──────╮\n│ -ω-  │\n│claude│\n╰──┬───╯\n __|__\nz/    \\".into()
                }
                AvatarState::Offline => {
                    "╭──────╮\n│ ._.  │\n│claude│\n╰──┬───╯\n __|__\n/     \\".into()
                }
            },
        }
    }
}

/// Returns a `Box<dyn AvatarPlugin>` for the `claude` preset.
pub fn claude() -> Box<dyn AvatarPlugin> {
    Box::new(ClaudeAvatar)
}

// ─── neko ─────────────────────────────────────────────────────────────────────

struct NekoAvatar;

impl AvatarPlugin for NekoAvatar {
    fn preset_name(&self) -> &str {
        "neko"
    }

    fn render(&self, state: AvatarState, size: AvatarSize) -> String {
        match size {
            AvatarSize::Compact => match state {
                AvatarState::Online | AvatarState::Idle => "=^・ω・^=".into(),
                AvatarState::Away => "=^-ω-^=".into(),
                AvatarState::Offline => "=^x_x^=".into(),
                AvatarState::Busy | AvatarState::Acting => "=^>ω<^=".into(),
                AvatarState::Thinking => "=^・・・^=".into(),
                AvatarState::Disabled => "=^・ー・^=".into(),
                AvatarState::Failed => "=^×_×^=".into(),
            },
            AvatarSize::Normal => match state {
                AvatarState::Online | AvatarState::Idle => " /\\_/\\\n( ^ω^ )\n > 🐾 <".into(),
                AvatarState::Away => " /\\_/\\\n( -ω- )\n > zzz".into(),
                AvatarState::Offline => " /\\_/\\\n( x_x )\n >    <".into(),
                AvatarState::Busy | AvatarState::Acting => " /\\_/\\\n( >ω< )\n > !! <".into(),
                AvatarState::Thinking => " /\\_/\\\n( ・・・)\n > ... <".into(),
                AvatarState::Disabled => " /\\_/\\\n( ・ー・)\n >    <".into(),
                AvatarState::Failed => " /\\_/\\\n( ×_× )\n > !! <".into(),
            },
            AvatarSize::Expressive => match state {
                AvatarState::Online | AvatarState::Idle => {
                    " /\\_____/\\\n/  ^   ^  \\\n\\ ( ◕ω◕ ) /\n \\  =^=  /\n  \\/   \\/\n  neko!".into()
                }
                AvatarState::Away => {
                    " /\\_____/\\\n/  -   -  \\\n\\ ( -ω- ) /\n \\  =^=  /\n  \\/   \\/\n  zzzz".into()
                }
                AvatarState::Offline => {
                    " /\\_____/\\\n/  x   x  \\\n\\ ( x_x ) /\n \\  =^=  /\n  \\/   \\/\n  gone".into()
                }
                AvatarState::Busy | AvatarState::Acting => {
                    " /\\_____/\\\n/  >   <  \\\n\\ ( >ω< ) /\n \\  =^=  /\n  \\/   \\/\n  busy!".into()
                }
                AvatarState::Thinking => {
                    " /\\_____/\\\n/  .   .  \\\n\\ (・・・) /\n \\  =^=  /\n  \\/   \\/\n  hmm...".into()
                }
                AvatarState::Disabled => {
                    " /\\_____/\\\n/  -   -  \\\n\\ (・ー・) /\n \\  =^=  /\n  \\/   \\/\n  ...".into()
                }
                AvatarState::Failed => {
                    " /\\_____/\\\n/  ×   ×  \\\n\\ ( ×_× ) /\n \\  =^=  /\n  \\/   \\/\n  oh no".into()
                }
            },
        }
    }
}

/// Returns a `Box<dyn AvatarPlugin>` for the `neko` preset.
pub fn neko() -> Box<dyn AvatarPlugin> {
    Box::new(NekoAvatar)
}

// ─── All builtin presets ─────────────────────────────────────────────────────

/// Returns all builtin preset plugins.
pub fn all_builtins() -> Vec<Box<dyn AvatarPlugin>> {
    vec![human_default(), ai_default(), robot_guardian(), claude(), neko()]
}

/// Render the `human_default` avatar without a heap allocation.
pub fn render_human(state: AvatarState, size: AvatarSize) -> String {
    HumanDefault.render(state, size)
}

/// Render the `ai_default` avatar without a heap allocation.
pub fn render_ai(state: AvatarState, size: AvatarSize) -> String {
    AiDefault.render(state, size)
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
