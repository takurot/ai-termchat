use tui::layout::Constraint;

/// Returns the 3-pane horizontal constraints: [peers(18), chat(min), status(22)].
pub fn three_pane_constraints() -> Vec<Constraint> {
    vec![Constraint::Length(18), Constraint::Min(0), Constraint::Length(22)]
}

/// Returns `true` when the terminal is wide enough to show side panels.
/// Below 80 columns the layout collapses to a single chat pane.
pub fn should_show_side_panels(cols: u16) -> bool {
    cols >= 80
}

/// Returns vertical constraints for the left column: [peers(min), rooms(height)].
/// The rooms panel height is at least 8, and scales to 40% of terminal height above 30.
pub fn left_column_constraints(left_height: u16) -> Vec<Constraint> {
    let rooms_height = if left_height > 30 { left_height * 2 / 5 } else { 8 };
    vec![Constraint::Min(0), Constraint::Length(rooms_height)]
}

/// Truncates a string to `max_chars` Unicode scalar values.
pub fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
