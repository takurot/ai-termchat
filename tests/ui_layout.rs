use triadchat::avatar::AvatarSize;
use triadchat::ui::layout::{should_show_side_panels, three_pane_constraints};
use tui::layout::Constraint;

#[test]
fn side_panel_visibility_and_avatar_size_change_at_same_boundary() {
    for (cols, should_show_panels, avatar_size) in [
        (0, false, AvatarSize::Compact),
        (79, false, AvatarSize::Compact),
        (80, true, AvatarSize::Normal),
        (120, true, AvatarSize::Normal),
    ] {
        assert_eq!(should_show_side_panels(cols), should_show_panels);
        assert_eq!(AvatarSize::for_width(cols), avatar_size);
    }
}

#[test]
fn wide_layout_uses_spec_three_pane_constraints() {
    assert_eq!(
        three_pane_constraints(),
        vec![Constraint::Length(18), Constraint::Min(0), Constraint::Length(22)]
    );
}

#[test]
fn wide_draw_path_uses_three_pane_constraints() {
    let ui_source =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui.rs")).unwrap();

    assert!(ui_source.contains("layout::three_pane_constraints()"));
}
