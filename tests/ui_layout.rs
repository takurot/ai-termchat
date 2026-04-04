use triadchat::ui::layout::{three_pane_constraints, should_show_side_panels};

// ─── Three-pane constraint generation ────────────────────────────────────────

#[test]
fn three_pane_constraints_have_correct_widths() {
    let constraints = three_pane_constraints();
    assert_eq!(constraints.len(), 3, "must have exactly 3 constraints");
}

#[test]
fn peers_panel_width_is_18() {
    use tui::layout::Constraint;
    let constraints = three_pane_constraints();
    assert_eq!(constraints[0], Constraint::Length(18));
}

#[test]
fn status_panel_width_is_22() {
    use tui::layout::Constraint;
    let constraints = three_pane_constraints();
    assert_eq!(constraints[2], Constraint::Length(22));
}

#[test]
fn chat_panel_uses_min_constraint() {
    use tui::layout::Constraint;
    let constraints = three_pane_constraints();
    assert_eq!(constraints[1], Constraint::Min(0));
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
