use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph};
use tui::Frame;

use std::io::Write;

use crate::avatar::builtin::render_ai;
use crate::avatar::{AvatarSize, AvatarState};
use crate::state::{AiMode, AiState, SkillProposal, State};
use crate::ui::layout::truncate;

/// Draws the right status panel (22-column side panel).
///
/// Shows AI avatar (normal size), current mode/state, last 5 TODO items, and
/// pending skill proposals.
pub fn draw_status_panel(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    state: &State,
    chunk: Rect,
) {
    let avatar_state = ai_state_to_avatar_state(&state.ai_state);
    let av_art = render_ai(avatar_state, AvatarSize::Normal);

    let mut lines: Vec<Spans> = Vec::new();

    // AI avatar
    for line in av_art.lines() {
        lines.push(Spans::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::LightCyan),
        )));
    }

    lines.push(Spans::from(Span::raw("")));

    // Mode line
    lines.push(Spans::from(vec![
        Span::styled("Mode: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_ai_mode(&state.ai_mode),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    ]));

    // State line
    lines.push(Spans::from(vec![
        Span::styled("State: ", Style::default().fg(Color::Gray)),
        Span::styled(format_ai_state(&state.ai_state), ai_state_color(&state.ai_state)),
    ]));

    lines.push(Spans::from(Span::raw("")));

    // Skill proposals
    let proposals = state.skill_proposals();
    if !proposals.is_empty() {
        lines.push(Spans::from(Span::styled(
            "Proposals:",
            Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
        )));
        for proposal in proposals.iter().take(3) {
            lines.push(proposal_span(proposal));
        }
        lines.push(Spans::from(Span::raw("")));
    }

    // Last 5 TODOs from structured output
    if let Some(structured) = &state.last_structured_output {
        if !structured.todos.is_empty() {
            lines.push(Spans::from(Span::styled(
                "TODOs:",
                Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
            )));
            for todo in structured.todos.iter().take(5) {
                let assignee =
                    todo.assignee.as_deref().map(|a| format!("[{}] ", a)).unwrap_or_default();
                let text = format!("• {}{}", assignee, truncate(&todo.text, 16));
                lines
                    .push(Spans::from(Span::styled(text, Style::default().fg(Color::LightYellow))));
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

// ─── helpers ─────────────────────────────────────────────────────────────────

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
    }
}

fn format_ai_state(state: &AiState) -> String {
    match state {
        AiState::Idle => "idle".into(),
        AiState::Thinking => "thinking…".into(),
        AiState::Acting => "acting".into(),
        AiState::Disabled => "disabled".into(),
        AiState::Failed(reason) => format!("failed: {}", truncate(reason, 10)),
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

fn proposal_span(proposal: &SkillProposal) -> Spans<'static> {
    let trusted_marker = if proposal.trusted { "✓" } else { "?" };
    let text =
        format!("[{}] {} {}", proposal.id, trusted_marker, truncate(&proposal.skill_name, 12));
    Spans::from(Span::styled(text, Style::default().fg(Color::LightBlue)))
}
