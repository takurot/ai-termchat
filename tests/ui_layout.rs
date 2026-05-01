use triadchat::avatar::AvatarSize;
use triadchat::ui::layout::should_show_side_panels;

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
