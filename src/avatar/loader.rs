use std::path::PathBuf;
use tui::text::Spans;

use super::builtin::{ai_default, all_builtins};
use super::{AvatarPlugin, AvatarSize, AvatarState};
#[cfg(feature = "avatar-ffi")]
use super::{AvatarPluginVTable, VTABLE_VERSION};
#[cfg(feature = "avatar-ffi")]
use tracing::warn;

/// Manages avatar plugins: builtin presets + optional external `.so`/`.dylib`
/// files loaded from a directory.
///
/// External plugins are loaded lazily on construction. If the plugin directory
/// doesn't exist or is empty, only builtins are available.
pub struct AvatarManager {
    plugins: Vec<Box<dyn AvatarPlugin>>,
}

impl AvatarManager {
    /// Create a new `AvatarManager`.
    ///
    /// `plugin_dir` is scanned for `*.so` / `*.dylib` files. Each valid file
    /// is loaded as an external plugin. Files with incompatible vtable versions
    /// are skipped with a warning.
    ///
    /// Builtins are always registered first.
    pub fn new(plugin_dir: PathBuf) -> Self {
        let plugins: Vec<Box<dyn AvatarPlugin>> = all_builtins();
        #[cfg(feature = "avatar-ffi")]
        let mut plugins = plugins;

        #[cfg(feature = "avatar-ffi")]
        {
            if let Ok(entries) = std::fs::read_dir(&plugin_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if is_dylib(&path) {
                        match load_external_plugin(&path) {
                            Ok(plugin) => plugins.push(plugin),
                            Err(e) => warn!("Skipping avatar plugin {:?}: {}", path, e),
                        }
                    }
                }
            }
        }
        #[cfg(not(feature = "avatar-ffi"))]
        {
            // External plugin loading is disabled unless the `avatar-ffi` feature is enabled.
            let _ = plugin_dir;
        }

        // Deduplicate: external plugins override builtins with the same name.
        // We keep the *last* occurrence (external wins).
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let deduplicated: Vec<Box<dyn AvatarPlugin>> = plugins
            .into_iter()
            .rev()
            .filter(|p| seen.insert(p.preset_name().to_owned()))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        Self { plugins: deduplicated }
    }

    /// Render an avatar for the given preset name, state, and size.
    ///
    /// Falls back to `ai_default` if the preset is not found.
    pub fn render(
        &self,
        preset: &str,
        state: AvatarState,
        size: AvatarSize,
    ) -> Vec<Spans<'static>> {
        self.find_plugin(preset)
            .map(|p| p.render(state.clone(), size))
            .unwrap_or_else(|| ai_default().render(state, size))
    }

    /// Returns the names of all available presets (builtin + external), deduplicated.
    pub fn list_all_presets(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.preset_name().to_owned()).collect()
    }

    fn find_plugin(&self, preset: &str) -> Option<&dyn AvatarPlugin> {
        self.plugins.iter().find(|p| p.preset_name() == preset).map(|p| p.as_ref())
    }
}

// ─── External plugin loading (requires avatar-ffi feature) ──────────────────

#[cfg(feature = "avatar-ffi")]
fn is_dylib(path: &std::path::Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some("so") | Some("dylib") | Some("dll") => true,
        _ => false,
    }
}

/// Wraps a loaded `libloading::Library` as an `AvatarPlugin`.
///
/// The library must export an `AVATAR_VTABLE` symbol of type `AvatarPluginVTable`.
#[cfg(feature = "avatar-ffi")]
struct ExternalPlugin {
    _library: libloading::Library,
    preset: String,
    render_fn: unsafe extern "C" fn(state: u32, size: u32) -> *mut std::os::raw::c_char,
}

#[cfg(feature = "avatar-ffi")]
impl AvatarPlugin for ExternalPlugin {
    fn preset_name(&self) -> &str {
        &self.preset
    }

    fn render(&self, state: AvatarState, size: AvatarSize) -> Vec<Spans<'static>> {
        let state_code = avatar_state_to_u32(&state);
        let size_code = avatar_size_to_u32(size);

        // SAFETY: `render_fn` comes from a loaded library that passed vtable
        // version validation.  The returned pointer is a valid, null-terminated
        // C string that we must free via `libc::free`.
        let result_str = unsafe {
            let raw = (self.render_fn)(state_code, size_code);
            if raw.is_null() {
                String::new()
            } else {
                let cstr = std::ffi::CStr::from_ptr(raw);
                let result = cstr.to_string_lossy().into_owned();
                libc::free(raw as *mut libc::c_void);
                result
            }
        };

        parse_ansi(&result_str)
    }
}

/// A very basic ANSI parser that converts a string into Spans.
/// Supports standard SGR color codes (30-37, 40-47, 90-97, 100-107).
#[cfg(feature = "avatar-ffi")]
fn parse_ansi(input: &str) -> Vec<Spans<'static>> {
    use tui::style::{Color, Style};
    use tui::text::Span;

    input
        .lines()
        .map(|line| {
            let mut spans = Vec::new();
            let mut pos = 0;
            let mut current_style = Style::default();

            while let Some(esc_start) = line[pos..].find('\x1b') {
                let esc_start = pos + esc_start;
                if esc_start > pos {
                    spans.push(Span::styled(line[pos..esc_start].to_string(), current_style));
                }

                if line[esc_start..].starts_with("\x1b[") {
                    if let Some(m_pos) = line[esc_start..].find('m') {
                        let m_pos = esc_start + m_pos;
                        let params = &line[esc_start + 2..m_pos];
                        for param in params.split(';') {
                            if let Ok(code) = param.parse::<u32>() {
                                match code {
                                    0 => current_style = Style::default(),
                                    1 => {
                                        current_style =
                                            current_style.add_modifier(tui::style::Modifier::BOLD)
                                    }
                                    30 => current_style = current_style.fg(Color::Black),
                                    31 => current_style = current_style.fg(Color::Red),
                                    32 => current_style = current_style.fg(Color::Green),
                                    33 => current_style = current_style.fg(Color::Yellow),
                                    34 => current_style = current_style.fg(Color::Blue),
                                    35 => current_style = current_style.fg(Color::Magenta),
                                    36 => current_style = current_style.fg(Color::Cyan),
                                    37 => current_style = current_style.fg(Color::Gray),
                                    39 => current_style.fg = None,
                                    40 => current_style = current_style.bg(Color::Black),
                                    41 => current_style = current_style.bg(Color::Red),
                                    42 => current_style = current_style.bg(Color::Green),
                                    43 => current_style = current_style.bg(Color::Yellow),
                                    44 => current_style = current_style.bg(Color::Blue),
                                    45 => current_style = current_style.bg(Color::Magenta),
                                    46 => current_style = current_style.bg(Color::Cyan),
                                    47 => current_style = current_style.bg(Color::Gray),
                                    49 => current_style.bg = None,
                                    90 => current_style = current_style.fg(Color::DarkGray),
                                    91 => current_style = current_style.fg(Color::LightRed),
                                    92 => current_style = current_style.fg(Color::LightGreen),
                                    93 => current_style = current_style.fg(Color::LightYellow),
                                    94 => current_style = current_style.fg(Color::LightBlue),
                                    95 => current_style = current_style.fg(Color::LightMagenta),
                                    96 => current_style = current_style.fg(Color::LightCyan),
                                    97 => current_style = current_style.fg(Color::White),
                                    100 => current_style = current_style.bg(Color::DarkGray),
                                    101 => current_style = current_style.bg(Color::LightRed),
                                    102 => current_style = current_style.bg(Color::LightGreen),
                                    103 => current_style = current_style.bg(Color::LightYellow),
                                    104 => current_style = current_style.bg(Color::LightBlue),
                                    105 => current_style = current_style.bg(Color::LightMagenta),
                                    106 => current_style = current_style.bg(Color::LightCyan),
                                    107 => current_style = current_style.bg(Color::White),
                                    _ => {}
                                }
                            }
                        }
                        pos = m_pos + 1;
                    } else {
                        pos = esc_start + 2;
                    }
                } else {
                    pos = esc_start + 1;
                }
            }
            if pos < line.len() {
                spans.push(Span::styled(line[pos..].to_string(), current_style));
            }
            Spans::from(spans)
        })
        .collect()
}

#[cfg(all(test, feature = "avatar-ffi"))]
mod tests {
    use super::*;
    use tui::style::Color;

    #[test]
    fn test_parse_ansi_colors() {
        let input = "\x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m";
        let rendered = parse_ansi(input);
        assert_eq!(rendered.len(), 1);
        let spans = &rendered[0].0;
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "Red");
        assert_eq!(spans[0].style.fg, Some(Color::Red));
        assert_eq!(spans[1].content, " ");
        assert_eq!(spans[1].style.fg, None);
        assert_eq!(spans[2].content, "Green");
        assert_eq!(spans[2].style.fg, Some(Color::Green));
    }
}

#[cfg(feature = "avatar-ffi")]
fn load_external_plugin(path: &std::path::Path) -> anyhow::Result<Box<dyn AvatarPlugin>> {
    // SAFETY: loading untrusted dynamic libraries is inherently unsafe.
    let lib = unsafe { libloading::Library::new(path) }
        .map_err(|e| anyhow::anyhow!("Failed to open {path:?}: {e}"))?;

    let vtable: &AvatarPluginVTable = unsafe {
        let sym = lib
            .get::<*const AvatarPluginVTable>(b"AVATAR_VTABLE\0")
            .map_err(|e| anyhow::anyhow!("Missing AVATAR_VTABLE: {e}"))?;
        let ptr: *const AvatarPluginVTable = *sym;
        ptr.as_ref().ok_or_else(|| anyhow::anyhow!("AVATAR_VTABLE is null"))?
    };

    if vtable.version != VTABLE_VERSION {
        return Err(anyhow::anyhow!(
            "vtable version mismatch: expected {VTABLE_VERSION}, got {}",
            vtable.version
        ));
    }

    let preset = unsafe {
        let raw = (vtable.preset_name)();
        if raw.is_null() {
            return Err(anyhow::anyhow!("preset_name() returned null"));
        }
        std::ffi::CStr::from_ptr(raw).to_string_lossy().into_owned()
    };

    let render_fn = vtable.render;

    Ok(Box::new(ExternalPlugin { _library: lib, preset, render_fn }))
}

#[cfg(feature = "avatar-ffi")]
fn avatar_state_to_u32(state: &AvatarState) -> u32 {
    match state {
        AvatarState::Idle => 0,
        AvatarState::Thinking => 1,
        AvatarState::Acting => 2,
        AvatarState::Disabled => 3,
        AvatarState::Failed => 4,
        AvatarState::Online => 5,
        AvatarState::Offline => 6,
        AvatarState::Busy => 7,
        AvatarState::Away => 8,
    }
}

#[cfg(feature = "avatar-ffi")]
fn avatar_size_to_u32(size: AvatarSize) -> u32 {
    match size {
        AvatarSize::Compact => 0,
        AvatarSize::Normal => 1,
        AvatarSize::Expressive => 2,
    }
}
