use tui::backend::Backend;
use tui::layout::Rect;
use tui::style::{Modifier, Style};
use tui::text::Span;
use tui::widgets::{Block, Borders, Paragraph};
use tui::Frame;

use crate::room::Room;
use crate::ui::layout::truncate;

const ID_MAX: usize = 8;
const MEMBER_NAME_MAX: usize = 6;
const MEMBER_LINE_MAX: usize = 14;
const MODE_MAX: usize = 6;

/// Builds the display lines for the room list panel (pure function, easily testable).
///
/// Returns 2 lines per room (after applying scroll offset):
/// - Line 1: `[marker][id:8] [mode:6]`
/// - Line 2: `  [members, comma-separated, total ≤14 chars]`
///
/// `scroll` skips that many lines from the top of the full rendered list.
pub fn build_room_lines(rooms: &[Room], active_id: Option<&str>, scroll: usize) -> Vec<String> {
    if rooms.is_empty() {
        return vec!["  (no rooms)".to_string()];
    }

    let mut lines: Vec<String> = Vec::with_capacity(rooms.len() * 2);

    for room in rooms {
        let marker = if active_id == Some(room.id.as_str()) { "* " } else { "  " };
        let id = truncate(&room.id, ID_MAX);
        let mode = room
            .ai_mode
            .as_ref()
            .map(|m| format!(" {:.*}", MODE_MAX, format!("{:?}", m).to_lowercase()))
            .unwrap_or_default();
        lines.push(format!("{}{}{}", marker, id, mode));

        // Member line: exclude ops-ai, truncate each name, cap total length
        use crate::room::MemberKind;
        let mut member_str = String::new();
        for member in &room.members {
            if member.kind == MemberKind::Ai && member.id == "ops-ai" {
                continue;
            }
            let name = truncate(&member.id, MEMBER_NAME_MAX);
            if member_str.is_empty() {
                member_str.push_str(&name);
            } else {
                let candidate = format!("{} {}", member_str, name);
                if candidate.len() <= MEMBER_LINE_MAX {
                    member_str = candidate;
                } else {
                    break;
                }
            }
        }
        lines.push(format!("  {}", member_str));
    }

    let scrolled: Vec<String> = lines.into_iter().skip(scroll).collect();
    if scrolled.is_empty() {
        vec!["  (no rooms)".to_string()]
    } else {
        scrolled
    }
}

use crate::state::State;

pub fn draw_room_list_panel(frame: &mut Frame<impl Backend>, state: &State, chunk: Rect) {
    let lines: Vec<_> =
        build_room_lines(state.rooms(), state.active_room_id(), state.room_list_scroll())
            .into_iter()
            .map(|l| tui::text::Spans::from(Span::raw(l)))
            .collect();

    let panel = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Rooms", Style::default().add_modifier(Modifier::BOLD))),
    );

    frame.render_widget(panel, chunk);
}
