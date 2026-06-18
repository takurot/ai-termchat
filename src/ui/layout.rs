use ratatui::layout::Constraint;

/// Phase 2 wide split: [peers(18), chat(min), status(22)].
pub fn three_pane_constraints() -> Vec<Constraint> {
    vec![Constraint::Length(18), Constraint::Min(0), Constraint::Length(22)]
}

/// Legacy horizontal split: [peers(18), right(min)].
pub fn left_right_constraints() -> Vec<Constraint> {
    vec![Constraint::Length(18), Constraint::Min(0)]
}

/// Vertical split for the right column: [chat(min), status(8)].
/// ops-ai panel sits below chat at full right-column width.
pub fn right_column_constraints() -> Vec<Constraint> {
    vec![Constraint::Min(0), Constraint::Length(8)]
}

/// Returns `true` when the terminal is wide enough to show the peers side panel.
/// Below 80 columns the layout collapses to chat-above / ops-ai-below only.
pub fn should_show_side_panels(cols: u16) -> bool {
    cols >= 80
}

/// Returns vertical constraints for the left column: [peers(min), rooms(height)].
/// The rooms panel height is at least 8 (clamped to available height), scaling to 40% above 30.
pub fn left_column_constraints(left_height: u16) -> Vec<Constraint> {
    let rooms_height = if left_height > 30 { left_height * 2 / 5 } else { left_height.min(8) };
    vec![Constraint::Min(0), Constraint::Length(rooms_height)]
}

/// Truncates a string to `max_chars` Unicode scalar values.
pub fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
