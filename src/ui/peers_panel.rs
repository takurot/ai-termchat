use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph};
use tui::Frame;

use std::io::Write;

use crate::avatar::builtin::render_human;
use crate::avatar::{AvatarSize, AvatarState};
use crate::state::State;
use crate::ui::layout::truncate;

/// Draws the left peers panel (18-column side panel).
///
/// Shows each connected peer with a compact avatar and presence indicator.
/// When the terminal is too narrow this function should not be called — the
/// caller is responsible for the visibility decision (see `layout::should_show_side_panels`).
pub fn draw_peers_panel(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    state: &State,
    chunk: Rect,
) {
    let mut lines: Vec<Spans> = vec![Spans::from(Span::styled(
        "Peers",
        Style::default().add_modifier(Modifier::BOLD),
    ))];

    for peer_name in state.peer_names() {
        // Compact avatar + name line
        let av = render_human(AvatarState::Online, AvatarSize::Compact);
        lines.push(Spans::from(vec![
            Span::styled(av, Style::default().fg(Color::Green)),
            Span::raw(" "),
            Span::styled(
                truncate(&peer_name, 10),
                Style::default().fg(Color::LightGreen),
            ),
        ]));
    }

    if lines.len() == 1 {
        lines.push(Spans::from(Span::styled(
            "(no peers)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let panel = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Peers", Style::default().add_modifier(Modifier::BOLD))),
    );
    frame.render_widget(panel, chunk);
}

