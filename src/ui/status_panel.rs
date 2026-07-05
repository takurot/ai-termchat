use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::avatar::loader::AvatarManager;
use crate::avatar::AvatarState;
use crate::config::AiProvider;
use crate::state::{ActiveTransferView, AiMode, AiState, SkillProposal, State};
use crate::ui::layout::truncate;

/// Draws the ops-ai status panel below chat.
pub fn draw_status_panel(
    frame: &mut Frame,
    state: &State,
    chunk: Rect,
    avatar_manager: &AvatarManager,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled("ops-ai", Style::default().add_modifier(Modifier::BOLD)));
    let inner_area = block.inner(chunk);
    frame.render_widget(block, chunk);

    let left_width = (inner_area.width as f32 * 0.45).clamp(25.0, 35.0) as u16;
    let side_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(left_width), Constraint::Min(0)].as_ref())
        .split(inner_area);

    let left_chunk = side_chunks[0];
    let right_chunk = side_chunks[1];

    // Left column: Avatar and Mode/State
    let avatar_state = ai_state_to_avatar_state(&state.ai_state);
    let av_art = avatar_manager.render(&state.ai_avatar, avatar_state, state.avatar_size);
    let mut left_lines = Vec::new();

    // AI avatar
    left_lines.extend(av_art);

    left_lines.push(Line::from(vec![
        Span::styled("Mode: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_ai_mode(&state.ai_mode),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    ]));
    left_lines.push(Line::from(vec![Span::styled(
        format!("[{}]", format_ai_provider(&state.ai_provider)),
        Style::default().fg(Color::DarkGray),
    )]));
    left_lines.push(Line::from(vec![
        Span::styled("State: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_ai_state_with_width(&state.ai_state, left_chunk.width),
            ai_state_color(&state.ai_state),
        ),
    ]));

    frame.render_widget(Paragraph::new(left_lines), left_chunk);

    // Right column: Receiving transfers, Proposals, TODOs
    let mut right_lines = Vec::new();

    let transfers = state.active_transfers_view();
    if !transfers.is_empty() {
        right_lines.push(Line::from(vec![Span::styled(
            "Receiving",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]));

        let total_h = right_chunk.height as usize;
        let available = total_h.saturating_sub(right_lines.len());
        let show = available.saturating_sub(1).min(2);

        for t in transfers.iter().take(show) {
            right_lines.push(receiving_line(t, right_chunk.width));
        }
        if let Some(overflow) = overflow_line(transfers.len().saturating_sub(show)) {
            right_lines
                .push(Line::from(Span::styled(overflow, Style::default().fg(Color::DarkGray))));
        }
    }

    let proposals = state.skill_proposals();
    if !proposals.is_empty() {
        right_lines.push(Line::from(vec![
            Span::styled(
                "Proposals ",
                Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
            ),
            Span::styled("(✓ trusted ? verify)", Style::default().fg(Color::DarkGray)),
        ]));
        for proposal in proposals.iter().take(2) {
            right_lines.push(proposal_line(proposal, right_chunk.width));
        }
        if let Some(overflow) = overflow_line(proposals.len().saturating_sub(2)) {
            right_lines
                .push(Line::from(Span::styled(overflow, Style::default().fg(Color::DarkGray))));
        }
    }

    if let Some(structured) = &state.last_structured_output {
        if !structured.todos.is_empty() {
            if !right_lines.is_empty() {
                // Add separator if we have proposals above
                right_lines.push(Line::from(""));
            }
            right_lines.push(Line::from(Span::styled(
                "TODOs:",
                Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
            )));

            // Calculate remaining lines for TODOs
            let used = right_lines.len();
            let total_h = right_chunk.height as usize;
            let available = total_h.saturating_sub(used);
            let todo_take = available.saturating_sub(1).min(3); // leave 1 for overflow, max 3

            if todo_take > 0 {
                for todo in structured.todos.iter().take(todo_take) {
                    let assignee =
                        todo.assignee.as_deref().map(|a| format!("[{}] ", a)).unwrap_or_default();
                    let text = format!(
                        "• {}{}",
                        assignee,
                        truncate(&todo.text, todo_text_limit(right_chunk.width))
                    );
                    right_lines.push(Line::from(Span::styled(
                        text,
                        Style::default().fg(Color::LightYellow),
                    )));
                }
                if let Some(overflow) =
                    overflow_line(structured.todos.len().saturating_sub(todo_take))
                {
                    right_lines.push(Line::from(Span::styled(
                        overflow,
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            } else if !structured.todos.is_empty() {
                right_lines.push(Line::from(Span::styled(
                    format!("  … {} tasks", structured.todos.len()),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    frame.render_widget(Paragraph::new(right_lines), right_chunk);
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

fn proposal_line(proposal: &SkillProposal, width: u16) -> Line<'static> {
    let trusted_marker = if proposal.trusted { "✓" } else { "?" };
    let text = format!(
        "[{}] {} {}",
        proposal.id,
        trusted_marker,
        truncate(&proposal.skill_name, proposal_name_limit(width))
    );
    Line::from(Span::styled(text, Style::default().fg(Color::LightBlue)))
}

fn todo_text_limit(width: u16) -> usize {
    width.saturating_sub(10).clamp(16, 60) as usize
}

fn failure_reason_limit(width: u16) -> usize {
    width.saturating_sub(5).clamp(10, 40) as usize
}

fn proposal_name_limit(width: u16) -> usize {
    width.saturating_sub(10).clamp(12, 30) as usize
}

fn overflow_line(hidden_count: usize) -> Option<String> {
    (hidden_count > 0).then(|| format!("  … +{hidden_count} more"))
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_idx = 0;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 || value >= 10.0 {
        format!("{:.0} {}", value, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", value, UNITS[unit_idx])
    }
}

fn receiving_line(t: &ActiveTransferView, width: u16) -> Line<'static> {
    let text = format!(
        "↓ {}  {}: {}",
        format_bytes(t.bytes_received),
        t.user,
        truncate(&t.filename, receiving_filename_limit(width))
    );
    Line::from(Span::styled(text, Style::default().fg(Color::LightCyan)))
}

fn receiving_filename_limit(width: u16) -> usize {
    width.saturating_sub(24).clamp(8, 40) as usize
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;
    use crate::message::{StructuredOutput, TodoItem};
    use crate::state::State;

    #[test]
    fn wide_panel_uses_roomier_text_limits() {
        assert_eq!(todo_text_limit(60), 50);
        assert_eq!(failure_reason_limit(60), 40);
        assert_eq!(proposal_name_limit(60), 30);
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

        let backend = TestBackend::new(80, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.size();
                draw_status_panel(frame, &state, area, &avatar_manager);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = (0..8)
            .map(|y| {
                (0..80)
                    .map(|x| buffer.get(x, y).symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Mode: clerk"));
        assert!(rendered.contains("[gemini]"));
        assert!(rendered.contains("Connection refused"));
        assert!(rendered.contains("email_processor"));
        assert!(rendered.contains("Implement auth module"));
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

        let backend = TestBackend::new(80, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.size();
                draw_status_panel(frame, &state, area, &avatar_manager);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = (0..8)
            .map(|y| {
                (0..80)
                    .map(|x| buffer.get(x, y).symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("… +2 more"));
    }

    // ── format_bytes ──────────────────────────────────────────────────

    #[test]
    fn format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn format_bytes_bytes() {
        assert_eq!(format_bytes(512), "512 B");
    }

    #[test]
    fn format_bytes_kib_decimal() {
        assert_eq!(format_bytes(2048), "2.0 KB");
    }

    #[test]
    fn format_bytes_mib_fractional() {
        assert_eq!(format_bytes(1_500_000), "1.4 MB");
    }

    #[test]
    fn format_bytes_gib_integer() {
        let gib = 10 * 1024 * 1024 * 1024_u64;
        assert_eq!(format_bytes(gib), "10 GB");
    }

    #[test]
    fn format_bytes_large_enough_for_integer_unit() {
        assert_eq!(format_bytes(15 * 1024), "15 KB");
    }

    // ── receiving section rendering ───────────────────────────────────

    fn state_with_one_transfer() -> State {
        let mut s = State::default();
        s.avatar_size = crate::avatar::AvatarSize::Compact;
        s.start_transfer(
            "alice".into(),
            "notes.txt".into(),
            std::path::PathBuf::from("/tmp/test-transfer"),
        );
        s.record_transfer_bytes("alice", "notes.txt", 1_500_000);
        s
    }

    #[test]
    fn rendered_panel_shows_active_transfer() {
        let state = state_with_one_transfer();

        let avatar_dir = std::path::PathBuf::from("/tmp/triadchat-test-avatars");
        let avatar_manager = AvatarManager::new(avatar_dir);

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.size();
                draw_status_panel(frame, &state, area, &avatar_manager);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = (0..10)
            .map(|y| {
                (0..80)
                    .map(|x| buffer.get(x, y).symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Receiving"));
        assert!(rendered.contains("1.4 MB"));
        assert!(rendered.contains("alice: notes.txt"));
    }

    #[test]
    fn rendered_panel_omits_receiving_section_when_idle() {
        let mut state = State::default();
        state.avatar_size = crate::avatar::AvatarSize::Compact;

        let avatar_dir = std::path::PathBuf::from("/tmp/triadchat-test-avatars");
        let avatar_manager = AvatarManager::new(avatar_dir);

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.size();
                draw_status_panel(frame, &state, area, &avatar_manager);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = (0..10)
            .map(|y| {
                (0..80)
                    .map(|x| buffer.get(x, y).symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(!rendered.contains("Receiving"));
    }

    #[test]
    fn rendered_panel_shows_transfer_overflow() {
        let mut state = State::default();
        state.avatar_size = crate::avatar::AvatarSize::Compact;
        state.start_transfer("a".into(), "x.txt".into(), std::path::PathBuf::from("/tmp/t1"));
        state.start_transfer("b".into(), "y.txt".into(), std::path::PathBuf::from("/tmp/t2"));
        state.start_transfer("c".into(), "z.txt".into(), std::path::PathBuf::from("/tmp/t3"));

        let avatar_dir = std::path::PathBuf::from("/tmp/triadchat-test-avatars");
        let avatar_manager = AvatarManager::new(avatar_dir);

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.size();
                draw_status_panel(frame, &state, area, &avatar_manager);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = (0..10)
            .map(|y| {
                (0..80)
                    .map(|x| buffer.get(x, y).symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("… +1 more"));
    }

    #[test]
    fn rendered_panel_budgets_receiving_proposals_and_todos() {
        let mut state = State::default();
        state.avatar_size = crate::avatar::AvatarSize::Compact;
        state.ai_mode = AiMode::Clerk;
        state.ai_provider = AiProvider::Claude;
        state.ai_state = AiState::Idle;
        state.start_transfer(
            "alice".into(),
            "notes.txt".into(),
            std::path::PathBuf::from("/tmp/t1"),
        );
        state.record_transfer_bytes("alice", "notes.txt", 1024);
        state.set_skill_proposals(&["email_processor".into()], Some("alice".into()), true);
        state.last_structured_output = Some(StructuredOutput {
            todos: vec![TodoItem {
                text: "Implement auth module".into(),
                assignee: Some("takuro".into()),
            }],
            decisions: Vec::new(),
            skill_suggestions: Vec::new(),
            raw_text: None,
        });

        let avatar_dir = std::path::PathBuf::from("/tmp/triadchat-test-avatars");
        let avatar_manager = AvatarManager::new(avatar_dir);

        let backend = TestBackend::new(80, 14);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.size();
                draw_status_panel(frame, &state, area, &avatar_manager);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = (0..14)
            .map(|y| {
                (0..80)
                    .map(|x| buffer.get(x, y).symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Receiving"));
        assert!(rendered.contains("1.0 KB"));
        assert!(rendered.contains("Proposals"));
        assert!(rendered.contains("Implement auth module"));
    }
}
