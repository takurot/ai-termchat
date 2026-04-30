pub mod builtin;
pub mod loader;

use tui::style::{Color, Style};
use tui::text::{Span, Spans};

/// Logical state of an avatar subject (AI or human peer).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AvatarState {
    // AI states
    Idle,
    Thinking,
    Acting,
    Disabled,
    Failed,
    // Peer presence states
    Online,
    Offline,
    Busy,
    Away,
}

/// Rendering size hint for avatar ASCII art.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AvatarSize {
    /// Single-line or very compact (used when terminal width < 80).
    Compact,
    /// Default multi-line rendering.
    Normal,
    /// Larger, more detailed rendering.
    Expressive,
}

impl AvatarSize {
    /// Choose a size based on terminal column width.
    /// Columns < 80 → Compact, otherwise Normal.
    pub fn for_width(cols: u16) -> Self {
        if cols < 80 {
            AvatarSize::Compact
        } else {
            AvatarSize::Normal
        }
    }
}

/// A plugin that renders pixel-art or ASCII avatar art.
///
/// This trait is object-safe and can be used as `Box<dyn AvatarPlugin>`.
pub trait AvatarPlugin: Send + Sync {
    /// The unique preset name (e.g. `"human_default"`, `"robot_guardian"`).
    fn preset_name(&self) -> &str;
    /// Render the avatar for the given state and size.
    fn render(&self, state: AvatarState, size: AvatarSize) -> Vec<Spans<'static>>;
}

/// Helper to convert a 2D color grid into multi-line Spans using the "half-block" technique.
/// Each character represents two vertical pixels.
pub fn colors_to_spans(colors: Vec<Vec<Color>>) -> Vec<Spans<'static>> {
    let mut spans_vec = Vec::new();
    let height = colors.len();
    if height == 0 {
        return spans_vec;
    }
    let width = colors[0].len();

    for y in (0..height).step_by(2) {
        let mut line = Vec::new();
        for (x, &top) in colors[y].iter().enumerate().take(width) {
            let bottom = if y + 1 < height { colors[y + 1][x] } else { Color::Reset };

            match (top, bottom) {
                (Color::Reset, Color::Reset) => line.push(Span::raw(" ")),
                (t, Color::Reset) => line.push(Span::styled("▀", Style::default().fg(t))),
                (Color::Reset, b) => line.push(Span::styled("▄", Style::default().fg(b))),
                (t, b) => line.push(Span::styled("▀", Style::default().fg(t).bg(b))),
            }
        }
        spans_vec.push(Spans::from(line));
    }
    spans_vec
}

/// C-compatible vtable for FFI plugin loading (`libloading`).
///
/// External shared libraries must export a symbol named `AVATAR_VTABLE` of
/// this type. The `version` field must match `VTABLE_VERSION` to be loaded.
#[repr(C)]
pub struct AvatarPluginVTable {
    /// Must equal `VTABLE_VERSION`. Mismatches are rejected at load time.
    pub version: u32,
    /// Returns the null-terminated preset name.
    pub preset_name: unsafe extern "C" fn() -> *const std::os::raw::c_char,
    /// Renders the avatar as an ANSI-encoded string; caller must free the returned CString.
    pub render: unsafe extern "C" fn(state: u32, size: u32) -> *mut std::os::raw::c_char,
}

/// Current vtable ABI version. Bump this on any incompatible change.
pub const VTABLE_VERSION: u32 = 2;
