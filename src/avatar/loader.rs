use std::path::PathBuf;

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
    pub fn render(&self, preset: &str, state: AvatarState, size: AvatarSize) -> String {
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

    fn render(&self, state: AvatarState, size: AvatarSize) -> String {
        let state_code = avatar_state_to_u32(&state);
        let size_code = avatar_size_to_u32(size);

        // SAFETY: `render_fn` comes from a loaded library that passed vtable
        // version validation.  The returned pointer is a valid, null-terminated
        // C string that we must free via `libc::free`.
        unsafe {
            let raw = (self.render_fn)(state_code, size_code);
            if raw.is_null() {
                return String::new();
            }
            let cstr = std::ffi::CStr::from_ptr(raw);
            let result = cstr.to_string_lossy().into_owned();
            libc::free(raw as *mut libc::c_void);
            result
        }
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
