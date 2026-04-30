use tui::backend::Backend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph};
use tui::Frame;

use crate::avatar::loader::AvatarManager;
use crate::avatar::AvatarState;
use crate::config::AiProvider;
use crate::state::{AiMode, AiState, SkillProposal, State};
use crate::ui::layout::truncate;

/// Draws the ops-ai status panel below chat.
pub fn draw_status_panel(
    frame: &mut Frame<impl Backend>,
    state: &State,
    chunk: Rect,
    avatar_manager: &AvatarManager,
) {
    let inner_width = chunk.width.saturating_sub(2);
    let avatar_state = ai_state_to_avatar_state(&state.ai_state);
    let av_art = avatar_manager.render(&state.ai_avatar, avatar_state, state.avatar_size);

    let mut lines: Vec<Spans> = Vec::new();

    // AI avatar
    lines.extend(av_art);

    lines.push(Spans::from(vec![
        Span::styled("Mode: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!(
                "{} [{}]",
                format_ai_mode(&state.ai_mode),
                format_ai_provider(&state.ai_provider)
            ),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("State: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_ai_state_with_width(&state.ai_state, inner_width),
            ai_state_color(&state.ai_state),
        ),
    ]));

    let proposals = state.skill_proposals();
    if !proposals.is_empty() {
        lines.push(Spans::from(Span::styled(
            "Proposals:",
            Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
        )));
        for proposal in proposals.iter().take(3) {
            lines.push(proposal_span(proposal, inner_width));
        }
        if let Some(overflow) = overflow_line(proposals.len().saturating_sub(3)) {
            lines.push(Spans::from(Span::styled(overflow, Style::default().fg(Color::DarkGray))));
        }
        lines.push(Spans::from(Span::styled(
            "✓ trusted  ? unverified",
            Style::default().fg(Color::DarkGray),
        )));
    }

    if let Some(structured) = &state.last_structured_output {
        if !structured.todos.is_empty() {
            lines.push(Spans::from(Span::styled(
                "TODOs:",
                Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
            )));
            for todo in structured.todos.iter().take(5) {
                let assignee =
                    todo.assignee.as_deref().map(|a| format!("[{}] ", a)).unwrap_or_default();
                let text =
                    format!("• {}{}", assignee, truncate(&todo.text, todo_text_limit(inner_width)));
                lines
                    .push(Spans::from(Span::styled(text, Style::default().fg(Color::LightYellow))));
            }
            if let Some(overflow) = overflow_line(structured.todos.len().saturating_sub(5)) {
                lines.push(Spans::from(Span::styled(
                    overflow,
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    let panel = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("ops-ai", Style::default().add_modifier(Modifier::BOLD))),
    );
    frame.render_widget(panel, chunk);
}

fn ai_state_to_avatar_state(ai_state: &AiState) -> AvatarState {
    match ai_state {
        AiState::Idle => AvatarState::Idle,
        AiState::Thinking => AvatarState::Thinking,
        AiState::Acting => AvatarState::Acting,
        AiState::Disabled => AvatarState::Disabled,
        AiState::Failed(_) => AvatarState::Failed,
    }
}

fn format_ai_mode(mode: &AiMode) -> &'static str {
    match mode {
        AiMode::Clerk => "clerk",
        AiMode::Listener => "listener",
        AiMode::Moderator => "moderator",
        AiMode::Operator => "operator",
        AiMode::Companion => "companion 🗣",
    }
}

fn format_ai_provider(provider: &AiProvider) -> &'static str {
    provider.label()
}

fn format_ai_state_with_width(state: &AiState, width: u16) -> String {
    match state {
        AiState::Idle => "idle".into(),
        AiState::Thinking => "thinking…".into(),
        AiState::Acting => "acting".into(),
        AiState::Disabled => "disabled".into(),
        AiState::Failed(reason) => {
            format!("failed: {}", truncate(reason, failure_reason_limit(width)))
        }
    }
}

fn ai_state_color(state: &AiState) -> Style {
    match state {
        AiState::Idle => Style::default().fg(Color::Green),
        AiState::Thinking => Style::default().fg(Color::Yellow),
        AiState::Acting => Style::default().fg(Color::LightMagenta),
        AiState::Disabled => Style::default().fg(Color::DarkGray),
        AiState::Failed(_) => Style::default().fg(Color::Red),
    }
}

fn proposal_span(proposal: &SkillProposal, width: u16) -> Spans<'static> {
    let trusted_marker = if proposal.trusted { "✓" } else { "?" };
    let text = format!(
        "[{}] {} {}",
        proposal.id,
        trusted_marker,
        truncate(&proposal.skill_name, proposal_name_limit(width))
    );
    Spans::from(Span::styled(text, Style::default().fg(Color::LightBlue)))
}

fn todo_text_limit(width: u16) -> usize {
    width.saturating_sub(20).clamp(16, 40) as usize
}

fn failure_reason_limit(width: u16) -> usize {
    width.saturating_sub(30).clamp(10, 30) as usize
}

fn proposal_name_limit(width: u16) -> usize {
    width.saturating_sub(40).clamp(12, 20) as usize
}

fn overflow_line(hidden_count: usize) -> Option<String> {
    (hidden_count > 0).then(|| format!("  … +{hidden_count} more"))
}

#[cfg(test)]
mod tests {
    use tui::backend::TestBackend;
    use tui::Terminal;

    use super::*;
    use crate::message::{StructuredOutput, TodoItem};
    use crate::state::State;

    #[test]
    fn wide_panel_uses_roomier_text_limits() {
        assert_eq!(todo_text_limit(60), 40);
        assert_eq!(failure_reason_limit(60), 30);
        assert_eq!(proposal_name_limit(60), 20);
    }

    #[test]
    fn narrow_panel_keeps_minimum_text_limits() {
        assert!(todo_text_limit(20) >= 16);
        assert!(failure_reason_limit(20) >= 10);
        assert!(proposal_name_limit(20) >= 12);
    }

    #[test]
    fn failed_state_uses_width_aware_truncation() {
        let state = AiState::Failed("Connection refused by sidecar".into());
        let rendered = format_ai_state_with_width(&state, 60);

        assert!(rendered.contains("Connection refused by sidecar"));
    }

    #[test]
    fn overflow_line_is_hidden_when_nothing_overflows() {
        assert_eq!(overflow_line(0), None);
    }

    #[test]
    fn overflow_line_reports_hidden_count() {
        assert_eq!(overflow_line(3).as_deref(), Some("  … +3 more"));
    }

    #[test]
    fn rendered_panel_includes_provider_and_wider_text() {
        let mut state = State::default();
        state.avatar_size = crate::avatar::AvatarSize::Compact;
        state.ai_mode = AiMode::Clerk;
        state.ai_provider = AiProvider::Gemini;
        state.ai_state = AiState::Failed("Connection refused by upstream sidecar".into());
        state.set_skill_proposals(&["email_processor".into()], Some("alice".into()), true);
        state.last_structured_output = Some(StructuredOutput {
            todos: vec![TodoItem {
                text: "Implement auth module with provider switching".into(),
                assignee: Some("takuro".into()),
            }],
            decisions: Vec::new(),
            skill_suggestions: Vec::new(),
            raw_text: None,
        });

        let avatar_dir = std::path::PathBuf::from("/tmp/triadchat-test-avatars");
        let avatar_manager = AvatarManager::new(avatar_dir);

        let backend = TestBackend::new(72, 11);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.size();
                draw_status_panel(frame, &state, area, &avatar_manager);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = (0..11)
            .map(|y| {
                (0..72)
                    .map(|x| buffer.get(x, y).symbol.clone())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Mode: clerk [gemini]"));
        assert!(rendered.contains("Connection refused"));
        assert!(rendered.contains("email_processor"));
        assert!(rendered.contains("Implement auth module"));
        assert!(rendered.contains("✓ trusted"));
    }

    #[test]
    fn rendered_panel_reports_proposal_overflow() {
        let mut state = State::default();
        state.avatar_size = crate::avatar::AvatarSize::Compact;
        state.ai_mode = AiMode::Operator;
        state.ai_provider = AiProvider::Claude;
        state.ai_state = AiState::Idle;
        state.set_skill_proposals(
            &["first".into(), "second".into(), "third".into(), "fourth".into()],
            None,
            true,
        );

        let avatar_dir = std::path::PathBuf::from("/tmp/triadchat-test-avatars");
        let avatar_manager = AvatarManager::new(avatar_dir);

        let backend = TestBackend::new(72, 11);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.size();
                draw_status_panel(frame, &state, area, &avatar_manager);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = (0..11)
            .map(|y| {
                (0..72)
                    .map(|x| buffer.get(x, y).symbol.clone())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("… +1 more"));
    }
}
