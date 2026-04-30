use tui::backend::Backend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph};
use tui::Frame;

use crate::avatar::loader::AvatarManager;
use crate::avatar::{AvatarSize, AvatarState};
use crate::state::State;
use crate::ui::layout::truncate;

/// Draws the left peers panel (18-column side panel).
///
/// Shows each connected peer with a compact avatar and presence indicator.
/// When the terminal is too narrow this function should not be called — the
/// caller is responsible for the visibility decision (see `layout::should_show_side_panels`).
pub fn draw_peers_panel(
    frame: &mut Frame<impl Backend>,
    state: &State,
    chunk: Rect,
    avatar_manager: &AvatarManager,
) {
    let mut lines: Vec<Spans> =
        vec![Spans::from(Span::styled("Peers", Style::default().add_modifier(Modifier::BOLD)))];

    // Local user
    let local_av_lines =
        avatar_manager.render(&state.user_avatar, AvatarState::Online, AvatarSize::Compact);
    let local_av = local_av_lines.first().cloned().unwrap_or_default();

    let mut local_user_spans = local_av.0;
    local_user_spans.push(Span::raw(" "));
    local_user_spans.push(Span::styled(
        truncate(state.local_user_name(), 10),
        Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD),
    ));
    lines.push(Spans::from(local_user_spans));

    let remote_peers = state.peer_info_list();
    let remote_peer_count = remote_peers.len();

    for (peer_name, avatar_preset) in remote_peers {
        let preset = if avatar_preset.is_empty() { "human_default" } else { &avatar_preset };
        let av_lines = avatar_manager.render(preset, AvatarState::Online, AvatarSize::Compact);
        let av = av_lines.first().cloned().unwrap_or_default();

        let mut peer_spans = av.0;
        peer_spans.push(Span::raw(" "));
        peer_spans
            .push(Span::styled(truncate(&peer_name, 10), Style::default().fg(Color::LightGreen)));
        lines.push(Spans::from(peer_spans));
    }

    if remote_peer_count == 0 {
        lines.push(Spans::from(Span::styled(
            "(no other peers)",
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
