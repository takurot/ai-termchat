pub mod builtin;
pub mod loader;

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
        if cols < 80 { AvatarSize::Compact } else { AvatarSize::Normal }
    }
}

/// A plugin that renders ASCII avatar art.
///
/// This trait is object-safe and can be used as `Box<dyn AvatarPlugin>`.
pub trait AvatarPlugin: Send + Sync {
    /// The unique preset name (e.g. `"human_default"`, `"robot_guardian"`).
    fn preset_name(&self) -> &str;
    /// Render the avatar for the given state and size.
    fn render(&self, state: AvatarState, size: AvatarSize) -> String;
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
    /// Renders the avatar; caller must free the returned CString.
    pub render: unsafe extern "C" fn(state: u32, size: u32) -> *mut std::os::raw::c_char,
}

/// Current vtable ABI version. Bump this on any incompatible change.
pub const VTABLE_VERSION: u32 = 1;
