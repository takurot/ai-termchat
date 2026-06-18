use super::{AvatarPlugin, AvatarSize, AvatarState, colors_to_lines};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

// ─── human_default ────────────────────────────────────────────────────────────

struct HumanDefault;

impl AvatarPlugin for HumanDefault {
    fn preset_name(&self) -> &str {
        "human_default"
    }

    fn render(&self, state: AvatarState, size: AvatarSize) -> Vec<Line<'static>> {
        let color = Color::Cyan;
        match size {
            AvatarSize::Compact => {
                let text = match state {
                    AvatarState::Online | AvatarState::Idle => "[H]●",
                    AvatarState::Busy | AvatarState::Acting => "[H]◉",
                    AvatarState::Away => "[H]◌",
                    AvatarState::Offline | AvatarState::Disabled | AvatarState::Failed => "[H]○",
                    _ => "[H]·",
                };
                vec![Line::from(Span::styled(text, Style::default().fg(color)))]
            }
            AvatarSize::Normal => {
                let art: &'static str = match state {
                    AvatarState::Online | AvatarState::Idle => " (^_^)\n  |H|\n  / \\",
                    AvatarState::Busy | AvatarState::Acting => " (>_<)\n  |H|\n  / \\",
                    AvatarState::Away => " (-_-)\n  |H|\n  / \\",
                    AvatarState::Offline | AvatarState::Disabled | AvatarState::Failed => {
                        " (x_x)\n  |H|\n  / \\"
                    }
                    _ => " (o_o)\n  |H|\n  / \\",
                };
                art.lines()
                    .map(|l| Line::from(Span::styled(l, Style::default().fg(color))))
                    .collect()
            }
            AvatarSize::Expressive => {
                let art: &'static str = match state {
                    AvatarState::Online | AvatarState::Idle => {
                        "  .-\"\"-.\n ( ^_^ )\n  \\|H|/\n  / | \\\n /  |  \\"
                    }
                    AvatarState::Busy | AvatarState::Acting => {
                        "  .-\"\"-.\n ( >_< )\n  \\|H|/\n  / | \\\n /  |  \\"
                    }
                    AvatarState::Away => "  .-\"\"-.\n ( -_- )\n  \\|H|/\n  / | \\\n /  |  \\",
                    AvatarState::Offline | AvatarState::Disabled | AvatarState::Failed => {
                        "  .-\"\"-.\n ( x_x )\n  \\|H|/\n  / | \\\n /  |  \\"
                    }
                    _ => "  .-\"\"-.\n ( o_o )\n  \\|H|/\n  / | \\\n /  |  \\",
                };
                art.lines()
                    .map(|l| Line::from(Span::styled(l, Style::default().fg(color))))
                    .collect()
            }
        }
    }
}

/// Returns a `Box<dyn AvatarPlugin>` for the `human_default` preset.
pub fn human_default() -> Box<dyn AvatarPlugin> {
    Box::new(HumanDefault)
}

// ─── ai_default ───────────────────────────────────────────────────────────────

struct AiDefault;

impl AiDefault {
    fn get_grid(&self, state: AvatarState) -> Vec<Vec<Color>> {
        let (head, socket, pupil) = match state {
            AvatarState::Thinking => (Color::Yellow, Color::Rgb(255, 165, 0), Color::White),
            AvatarState::Acting => (Color::Magenta, Color::Blue, Color::Cyan),
            _ => (Color::Green, Color::DarkGray, Color::White),
        };
        let x = Color::Reset;
        let h = head;
        let s = socket;
        let p = pupil;

        vec![
            vec![x, x, h, h, h, h, x, x],
            vec![x, h, h, h, h, h, h, x],
            vec![h, h, h, h, h, h, h, h],
            vec![h, s, s, h, h, s, s, h],
            vec![h, p, p, h, h, p, p, h],
            vec![h, h, h, h, h, h, h, h],
            vec![x, h, h, h, h, h, h, x],
            vec![x, x, h, h, h, h, x, x],
        ]
    }
}

impl AvatarPlugin for AiDefault {
    fn preset_name(&self) -> &str {
        "ai_default"
    }

    fn render(&self, state: AvatarState, size: AvatarSize) -> Vec<Line<'static>> {
        match size {
            AvatarSize::Compact => {
                let (text, color) = match state {
                    AvatarState::Idle | AvatarState::Online => ("[AI]◆", Color::Green),
                    AvatarState::Thinking => ("[AI]…", Color::Yellow),
                    AvatarState::Acting => ("[AI]▶", Color::Magenta),
                    AvatarState::Disabled => ("[AI]□", Color::Gray),
                    AvatarState::Failed => ("[AI]✗", Color::Red),
                    _ => ("[AI]·", Color::Gray),
                };
                vec![Line::from(Span::styled(text, Style::default().fg(color)))]
            }
            AvatarSize::Normal | AvatarSize::Expressive => colors_to_lines(self.get_grid(state)),
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

    fn render(&self, state: AvatarState, size: AvatarSize) -> Vec<Line<'static>> {
        let color = Color::Yellow;
        match size {
            AvatarSize::Compact => {
                let text = match state {
                    AvatarState::Idle | AvatarState::Online => "[RG]■",
                    AvatarState::Thinking => "[RG]⠿",
                    AvatarState::Acting => "[RG]⚡",
                    AvatarState::Disabled => "[RG]░",
                    AvatarState::Failed => "[RG]✗",
                    AvatarState::Busy => "[RG]◈",
                    AvatarState::Away => "[RG]◇",
                    AvatarState::Offline => "[RG]□",
                };
                vec![Line::from(Span::styled(text, Style::default().fg(color)))]
            }
            AvatarSize::Normal => {
                let art: &'static str = match state {
                    AvatarState::Idle | AvatarState::Online => " <|=|>\n [RG]\n  /|\\",
                    AvatarState::Thinking => " <|?|>\n [RG] ~\n  /|\\",
                    AvatarState::Acting => " <|!|>\n [RG]\n  >>\\",
                    AvatarState::Disabled => " <|-|>\n [RG]\n  /|\\",
                    AvatarState::Failed => " <|X|>\n [RG]\n  /|\\",
                    AvatarState::Busy => " <|*|>\n [RG]\n  /|\\",
                    AvatarState::Away => " <|.|>\n [RG]\n  /|\\",
                    AvatarState::Offline => " <| |>\n [RG]\n  /|\\",
                };
                art.lines()
                    .map(|l| Line::from(Span::styled(l, Style::default().fg(color))))
                    .collect()
            }
            AvatarSize::Expressive => {
                let art: &'static str = match state {
                    AvatarState::Idle | AvatarState::Online => {
                        " ┌─────┐\n │ |=| │\n │[RG] │\n └──┬──┘\n   /|\\"
                    }
                    AvatarState::Thinking => " ┌─────┐\n │ |?| │~\n │[RG] │\n └──┬──┘\n   /|\\",
                    AvatarState::Acting => " ┌─────┐\n │ |!| │\n │[RG] │\n └──┬──┘\n  >>|\\",
                    AvatarState::Disabled => " ┌─────┐\n │ |-| │\n │[RG] │\n └──┬──┘\n   /|\\",
                    AvatarState::Failed => " ┌─────┐\n │ |X| │\n │[RG] │\n └──┬──┘\n   /|\\",
                    AvatarState::Busy => " ┌─────┐\n │ |*| │\n │[RG] │\n └──┬──┘\n   /|\\",
                    AvatarState::Away => " ┌─────┐\n │ |.| │\n │[RG] │\n └──┬──┘\n   /|\\",
                    AvatarState::Offline => " ┌─────┐\n │ | | │\n │[RG] │\n └──┬──┘\n   /|\\",
                };
                art.lines()
                    .map(|l| Line::from(Span::styled(l, Style::default().fg(color))))
                    .collect()
            }
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

    fn render(&self, state: AvatarState, size: AvatarSize) -> Vec<Line<'static>> {
        let color = Color::LightMagenta;
        match size {
            AvatarSize::Compact => {
                let text = match state {
                    AvatarState::Idle | AvatarState::Online => "◈(・ω・)",
                    AvatarState::Thinking => "◉(・・・)",
                    AvatarState::Acting => "▶(☆ω☆)",
                    AvatarState::Failed => "✕(×_×)",
                    AvatarState::Disabled => "○(・ー・)",
                    AvatarState::Busy => "◉(>ω<)",
                    AvatarState::Away => "◈(-ω-)",
                    AvatarState::Offline => "○(._.)",
                };
                vec![Line::from(Span::styled(text, Style::default().fg(color)))]
            }
            AvatarSize::Normal => {
                let art: &'static str = match state {
                    AvatarState::Idle | AvatarState::Online => " (・ω・)\n╰[claude]╯\n  /   \\",
                    AvatarState::Thinking => " (・・・)\n╰[claude]╯ ~\n  /   \\",
                    AvatarState::Acting => " (☆ω☆)\n╰[claude]╯>>\n  >>  \\",
                    AvatarState::Failed => " (×_×)\n╰[claude]╯\n  /   \\",
                    AvatarState::Disabled => " (・ー・)\n╰[claude]╯\n  /   \\",
                    AvatarState::Busy => " (>ω<)\n╰[claude]╯\n  / ! \\",
                    AvatarState::Away => " (-ω-)\n╰[claude]╯\n  /   \\",
                    AvatarState::Offline => " (._. )\n╰[claude]╯\n  /   \\",
                };
                art.lines()
                    .map(|l| Line::from(Span::styled(l, Style::default().fg(color))))
                    .collect()
            }
            AvatarSize::Expressive => {
                let art: &'static str = match state {
                    AvatarState::Idle | AvatarState::Online => {
                        "╭──────╮\n│ ◕ω◕  │\n│claude│\n╰──┬───╯\n __|__\n/     \\"
                    }
                    AvatarState::Thinking => {
                        "╭──────╮\n│ ◉ ◉  │~\n│claude│\n╰──┬───╯\n __|__\n/     \\"
                    }
                    AvatarState::Acting => {
                        "╭──────╮\n│ ☆ω☆  │\n│claude│\n╰──┬───╯\n >>|__\n/>>   \\"
                    }
                    AvatarState::Failed => {
                        "╭──────╮\n│ ×_×  │\n│claude│\n╰──┬───╯\n __|__\n/     \\"
                    }
                    AvatarState::Disabled => {
                        "╭──────╮\n│ ・ー・ │\n│claude│\n╰──┬───╯\n __|__\n/     \\"
                    }
                    AvatarState::Busy => "╭──────╮\n│ >ω<  │\n│claude│\n╰──┬───╯\n __|__\n/ ! \\",
                    AvatarState::Away => "╭──────╮\n│ -ω-  │\n│claude│\n╰──┬───╯\n __|__\nz/    \\",
                    AvatarState::Offline => {
                        "╭──────╮\n│ ._.  │\n│claude│\n╰──┬───╯\n __|__\n/     \\"
                    }
                };
                art.lines()
                    .map(|l| Line::from(Span::styled(l, Style::default().fg(color))))
                    .collect()
            }
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

    fn render(&self, state: AvatarState, size: AvatarSize) -> Vec<Line<'static>> {
        let color = Color::LightRed;
        match size {
            AvatarSize::Compact => {
                let text = match state {
                    AvatarState::Online | AvatarState::Idle => "=^・ω・^=",
                    AvatarState::Away => "=^-ω-^=",
                    AvatarState::Offline => "=^x_x^=",
                    AvatarState::Busy | AvatarState::Acting => "=^>ω<^=",
                    AvatarState::Thinking => "=^・・・^=",
                    AvatarState::Disabled => "=^・ー・^=",
                    AvatarState::Failed => "=^×_×^=",
                };
                vec![Line::from(Span::styled(text, Style::default().fg(color)))]
            }
            AvatarSize::Normal => {
                let art: &'static str = match state {
                    AvatarState::Online | AvatarState::Idle => " /\\_/\\\n( ^ω^ )\n > 🐾 <",
                    AvatarState::Away => " /\\_/\\\n( -ω- )\n > zzz",
                    AvatarState::Offline => " /\\_/\\\n( x_x )\n >    <",
                    AvatarState::Busy | AvatarState::Acting => " /\\_/\\\n( >ω< )\n > !! <",
                    AvatarState::Thinking => " /\\_/\\\n( ・・・)\n > ... <",
                    AvatarState::Disabled => " /\\_/\\\n( ・ー・)\n >    <",
                    AvatarState::Failed => " /\\_/\\\n( ×_× )\n > !! <",
                };
                art.lines()
                    .map(|l| Line::from(Span::styled(l, Style::default().fg(color))))
                    .collect()
            }
            AvatarSize::Expressive => {
                let art: &'static str = match state {
                    AvatarState::Online | AvatarState::Idle => {
                        " /\\_____/\\\n/  ^   ^  \\\n\\ ( ◕ω◕ ) /\n \\  =^=  /\n  \\/   \\/\n  neko!"
                    }
                    AvatarState::Away => {
                        " /\\_____/\\\n/  -   -  \\\n\\ ( -ω- ) /\n \\  =^=  /\n  \\/   \\/\n  zzzz"
                    }
                    AvatarState::Offline => {
                        " /\\_____/\\\n/  x   x  \\\n\\ ( x_x ) /\n \\  =^=  /\n  \\/   \\/\n  gone"
                    }
                    AvatarState::Busy | AvatarState::Acting => {
                        " /\\_____/\\\n/  >   <  \\\n\\ ( >ω< ) /\n \\  =^=  /\n  \\/   \\/\n  busy!"
                    }
                    AvatarState::Thinking => {
                        " /\\_____/\\\n/  .   .  \\\n\\ (・・・) /\n \\  =^=  /\n  \\/   \\/\n  hmm..."
                    }
                    AvatarState::Disabled => {
                        " /\\_____/\\\n/  -   -  \\\n\\ (・ー・) /\n \\  =^=  /\n  \\/   \\/\n  ..."
                    }
                    AvatarState::Failed => {
                        " /\\_____/\\\n/  ×   ×  \\\n\\ ( ×_× ) /\n \\  =^=  /\n  \\/   \\/\n  oh no"
                    }
                };
                art.lines()
                    .map(|l| Line::from(Span::styled(l, Style::default().fg(color))))
                    .collect()
            }
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
pub fn render_human(state: AvatarState, size: AvatarSize) -> Vec<Line<'static>> {
    HumanDefault.render(state, size)
}

/// Render the `ai_default` avatar without a heap allocation.
pub fn render_ai(state: AvatarState, size: AvatarSize) -> Vec<Line<'static>> {
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
                    rendered.len() <= 1,
                    "Compact render for '{}' {:?} must be single-line (got {} lines)",
                    plugin.preset_name(),
                    state,
                    rendered.len()
                );
            }
        }
    }
}
