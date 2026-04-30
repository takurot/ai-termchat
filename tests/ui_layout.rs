use triadchat::ui::layout::{left_right_constraints, right_column_constraints, should_show_side_panels};

// ─── Left/right horizontal constraints ───────────────────────────────────────

#[test]
fn left_right_constraints_have_two_panes() {
    let constraints = left_right_constraints();
    assert_eq!(constraints.len(), 2);
}

#[test]
fn peers_panel_width_is_18() {
    use tui::layout::Constraint;
    let constraints = left_right_constraints();
    assert_eq!(constraints[0], Constraint::Length(18));
}

#[test]
fn right_column_uses_min_constraint() {
    use tui::layout::Constraint;
    let constraints = left_right_constraints();
    assert_eq!(constraints[1], Constraint::Min(0));
}

// ─── Right column vertical constraints ───────────────────────────────────────

#[test]
fn right_column_constraints_have_two_panes() {
    let constraints = right_column_constraints();
    assert_eq!(constraints.len(), 2);
}

#[test]
fn chat_uses_min_constraint() {
    use tui::layout::Constraint;
    let constraints = right_column_constraints();
    assert_eq!(constraints[0], Constraint::Min(0));
}

#[test]
fn status_panel_height_is_8() {
    use tui::layout::Constraint;
    let constraints = right_column_constraints();
    assert_eq!(constraints[1], Constraint::Length(8));
}

// ─── Side panel visibility ────────────────────────────────────────────────────

#[test]
fn side_panels_hidden_for_narrow_terminal() {
    assert!(!should_show_side_panels(79));
    assert!(!should_show_side_panels(0));
}

#[test]
fn side_panels_shown_for_wide_terminal() {
    assert!(should_show_side_panels(80));
    assert!(should_show_side_panels(120));
    assert!(should_show_side_panels(200));
}

#[test]
fn boundary_at_exactly_80_columns_shows_panels() {
    assert!(should_show_side_panels(80));
}
