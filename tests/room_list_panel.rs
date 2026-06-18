use triadchat::room::{Member, Room};
use triadchat::state::{AiMode, ScrollMovement, State};
use triadchat::ui::room_list_panel::build_room_lines;

// ─── helpers ─────────────────────────────────────────────────────────────────

fn make_room(id: &str, members: &[&str], ai_mode: Option<AiMode>) -> Room {
    let mut m: Vec<Member> = members.iter().map(|&n| Member::human(n)).collect();
    if let Some(ref mode) = ai_mode {
        m.push(Member::ai("ops-ai", mode.clone()));
    }
    Room { id: id.to_string(), members: m, ai_mode }
}

fn line_text(lines: &[String], idx: usize) -> &str {
    lines.get(idx).map(String::as_str).unwrap_or("")
}

// ─── build_room_lines ─────────────────────────────────────────────────────────

#[test]
fn empty_rooms_shows_placeholder() {
    let lines = build_room_lines(&[], None, 0);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("no rooms"), "got: {:?}", lines[0]);
}

#[test]
fn active_room_has_asterisk_marker() {
    let rooms = vec![make_room("room-1", &["alice"], Some(AiMode::Clerk))];
    let lines = build_room_lines(&rooms, Some("room-1"), 0);
    assert!(line_text(&lines, 0).starts_with("* "), "got: {:?}", lines[0]);
}

#[test]
fn inactive_room_has_space_marker() {
    let rooms = vec![make_room("room-1", &["alice"], None)];
    let lines = build_room_lines(&rooms, Some("room-99"), 0);
    assert!(line_text(&lines, 0).starts_with("  "), "got: {:?}", lines[0]);
}

#[test]
fn room_id_truncated_to_8_chars() {
    let rooms = vec![make_room("room-abcdefghij", &["alice"], None)];
    let lines = build_room_lines(&rooms, None, 0);
    // line 0: "  room-abc" (marker 2 + id 8 = 10, then optional mode)
    let trimmed = line_text(&lines, 0).trim();
    assert!(trimmed.starts_with("room-abc"), "got: {:?}", lines[0]);
    assert!(!trimmed.contains("room-abcdefghij"), "id should be truncated");
}

#[test]
fn ai_mode_shown_on_first_line() {
    let rooms = vec![make_room("room-1", &["alice"], Some(AiMode::Clerk))];
    let lines = build_room_lines(&rooms, None, 0);
    assert!(line_text(&lines, 0).contains("clerk"), "got: {:?}", lines[0]);
}

#[test]
fn members_shown_on_second_line() {
    let rooms = vec![make_room("room-1", &["alice", "bob"], None)];
    let lines = build_room_lines(&rooms, None, 0);
    let member_line = line_text(&lines, 1);
    assert!(member_line.contains("alice"), "got: {:?}", member_line);
    assert!(member_line.contains("bob"), "got: {:?}", member_line);
}

#[test]
fn member_name_truncated_to_6_chars() {
    let rooms = vec![make_room("room-1", &["verylongname"], None)];
    let lines = build_room_lines(&rooms, None, 0);
    let member_line = line_text(&lines, 1);
    assert!(member_line.contains("verylo"), "got: {:?}", member_line);
    assert!(!member_line.contains("verylongname"), "name should be truncated");
}

#[test]
fn ops_ai_excluded_from_member_line() {
    let rooms = vec![make_room("room-1", &["alice"], Some(AiMode::Clerk))];
    let lines = build_room_lines(&rooms, None, 0);
    let member_line = line_text(&lines, 1);
    assert!(!member_line.contains("ops-ai"), "ops-ai should not appear in member line");
    assert!(member_line.contains("alice"), "got: {:?}", member_line);
}

#[test]
fn multiple_rooms_render_two_lines_each() {
    let rooms = vec![
        make_room("room-1", &["alice"], Some(AiMode::Clerk)),
        make_room("room-2", &["bob"], None),
    ];
    let lines = build_room_lines(&rooms, Some("room-1"), 0);
    // 2 rooms × 2 lines = 4
    assert_eq!(lines.len(), 4, "got {} lines: {:?}", lines.len(), lines);
}

#[test]
fn scroll_offset_skips_lines() {
    let rooms = vec![make_room("room-1", &["alice"], None), make_room("room-2", &["bob"], None)];
    let all = build_room_lines(&rooms, None, 0);
    let scrolled = build_room_lines(&rooms, None, 2);
    assert_ne!(all[0], scrolled[0], "scroll should shift content");
    assert_eq!(all[2], scrolled[0], "first line with offset=2 should be room-2 line");
}

#[test]
fn member_line_total_truncated_to_14_chars() {
    // 3 members with 6-char names + spaces = 6+1+6+1+6 = 20 → truncate to 14
    let rooms = vec![make_room("room-1", &["aaaaaa", "bbbbbb", "cccccc"], None)];
    let lines = build_room_lines(&rooms, None, 0);
    let member_line = line_text(&lines, 1).trim_start(); // trim the "  " prefix
    assert!(member_line.len() <= 14, "member line too long: {:?}", member_line);
}

// ─── State scroll ─────────────────────────────────────────────────────────────

#[test]
fn scroll_down_increments() {
    let mut s = State::default();
    s.scroll_room_list(ScrollMovement::Down);
    assert_eq!(s.room_list_scroll(), 1);
}

#[test]
fn scroll_up_clamps_at_zero() {
    let mut s = State::default();
    s.scroll_room_list(ScrollMovement::Up);
    assert_eq!(s.room_list_scroll(), 0);
}

#[test]
fn reset_room_list_scroll_returns_to_zero() {
    let mut s = State::default();
    s.scroll_room_list(ScrollMovement::Down);
    s.scroll_room_list(ScrollMovement::Down);
    s.reset_room_list_scroll();
    assert_eq!(s.room_list_scroll(), 0);
}

// ─── layout constraints ───────────────────────────────────────────────────────

#[test]
fn left_column_constraints_has_two_entries() {
    use triadchat::ui::layout::left_column_constraints;
    assert_eq!(left_column_constraints(20).len(), 2);
}

#[test]
fn left_column_rooms_floor_is_8_for_short_terminals() {
    use ratatui::layout::Constraint;
    use triadchat::ui::layout::left_column_constraints;
    let c = left_column_constraints(20);
    assert_eq!(c[1], Constraint::Length(8));
}

#[test]
fn left_column_rooms_scales_for_tall_terminals() {
    use ratatui::layout::Constraint;
    use triadchat::ui::layout::left_column_constraints;
    let c = left_column_constraints(50);
    assert_eq!(c[1], Constraint::Length(50 * 2 / 5));
}

#[test]
fn left_column_rooms_clamped_for_very_short_terminals() {
    use ratatui::layout::Constraint;
    use triadchat::ui::layout::left_column_constraints;
    // height=4 is less than the 8-row floor — must not request more than available
    let c = left_column_constraints(4);
    assert_eq!(c[1], Constraint::Length(4));
}

#[test]
fn scroll_past_content_shows_placeholder() {
    let rooms = vec![make_room("room-1", &["alice"], None)];
    // 1 room × 2 lines; scroll=99 is far past the end
    let lines = build_room_lines(&rooms, None, 99);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("no rooms"), "got: {:?}", lines[0]);
}
