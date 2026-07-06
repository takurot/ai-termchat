//! Documentation consistency checks.
//!
//! These tests guard against drift between the docs and the implementation:
//!   - CLAUDE.md module table covers every `pub mod` declared in `src/lib.rs` (#155)
//!   - README/CLAUDE/GEMINI reference `ratatui` (not the abandoned `tui-rs`) (#121)
//!   - README command table mirrors the in-app `/help` surface (#83)
//!   - SPEC does not reintroduce invented config/provider values (#121)
//!   - `Cargo.toml` declares an MSRV (#122)
//!   - SPEC documents the bincode 1 -> 2 wire-format contract (#121)
//!
//! Keep assertions mechanical and literal so a future edit that drifts a doc
//! fails here loudly rather than silently shipping stale documentation.

static LIB_RS: &str = include_str!("../src/lib.rs");
static CARGO_TOML: &str = include_str!("../Cargo.toml");
static README: &str = include_str!("../README.md");
static CLAUDE_MD: &str = include_str!("../CLAUDE.md");
static GEMINI_MD: &str = include_str!("../GEMINI.md");
static SPEC_MD: &str = include_str!("../docs/SPEC.md");

/// Extract module names declared as `pub mod <name>;` in `src/lib.rs`.
fn pub_modules() -> Vec<&'static str> {
    LIB_RS
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("pub mod ")?;
            let name = rest.strip_suffix(';')?;
            let name = name.trim();
            if name.is_empty() || name.contains(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                return None;
            }
            Some(name)
        })
        .filter(|name| *name != "message") // re-exported as `message` but table lists `src/message.rs`
        .collect()
}

#[test]
fn claude_md_module_table_covers_every_pub_mod() {
    let modules = pub_modules();
    assert!(!modules.is_empty(), "failed to parse pub mods from src/lib.rs");
    let mut missing = Vec::new();
    for name in &modules {
        // accept either `src/<name>` or ` <name> ` references in the table
        if !CLAUDE_MD.contains(&format!("src/{name}")) && !CLAUDE_MD.contains(&format!("/{name}")) {
            missing.push(*name);
        }
    }
    assert!(
        missing.is_empty(),
        "CLAUDE.md module table is missing pub mods declared in src/lib.rs: {missing:?}. \
         Add a row for each (see src/lib.rs)."
    );
}

#[test]
fn docs_reference_ratatui_not_abandoned_tui_crate() {
    for (name, body) in [("README.md", README), ("CLAUDE.md", CLAUDE_MD), ("GEMINI.md", GEMINI_MD)]
    {
        assert!(body.contains("ratatui"), "{name} must reference `ratatui` (the active TUI crate)");
        assert!(
            !body.contains("tui-rs") && !body.contains("tui 0.14"),
            "{name} still references the abandoned `tui-rs`/`tui 0.14` crate; update to ratatui"
        );
    }
}

/// Command tokens that the in-app `/help` (`src/application/mod.rs::help_text`)
/// advertises. README must document every one of them.
const EXPECTED_README_COMMANDS: &[&str] = &[
    "/summary",
    "/todos",
    "/decisions",
    "/context",
    "/ai mode",
    "/ai quiet",
    "/ai freq",
    "/ai provider",
    "/room create",
    "/room list",
    "/room switch",
    "/peers",
    "/peer connect",
    "/trust list",
    "/trust add",
    "/trust remove",
    "/skills",
    "/skill",
    "/run",
    "/cancel",
    "/avatar set",
    "/avatar list",
    "/avatar preview",
    "/avatar mode",
    "/art list",
    "/art reload",
    "/send",
];

#[test]
fn readme_command_table_mirrors_in_app_help() {
    let mut missing = Vec::new();
    for cmd in EXPECTED_README_COMMANDS {
        if !README.contains(cmd) {
            missing.push(*cmd);
        }
    }
    assert!(
        missing.is_empty(),
        "README command table is missing commands that `/help` advertises: {missing:?}. \
         The in-app `help_text()` is the source of truth (see src/application/mod.rs)."
    );
}

#[test]
fn cargo_toml_declares_msrv() {
    assert!(
        CARGO_TOML.contains("rust-version"),
        "Cargo.toml must declare `rust-version` (MSRV) in [package]"
    );
}

#[test]
fn spec_documents_bincode_wire_contract() {
    // bincode 1 -> 2 migration is load-bearing for wire compatibility.
    // SPEC §5 must call it out instead of silently pinning an old version.
    assert!(
        SPEC_MD.to_lowercase().contains("bincode"),
        "SPEC must document the bincode wire-format contract"
    );
    assert!(
        SPEC_MD.contains("legacy"),
        "SPEC must reference bincode's legacy() config that preserves wire format"
    );
}

#[test]
fn spec_has_no_invented_provider_disabled_value() {
    // `AiProvider::Disabled` does not exist; the real fallback is
    // `ai.enabled = false` -> `AiState::Disabled`. SPEC must not claim otherwise.
    assert!(
        !SPEC_MD.contains("provider = \"disabled\""),
        "SPEC must not document the invented `provider = \"disabled\"` value; \
         the real fallback is `ai.enabled = false` -> `AiState::Disabled`"
    );
}

#[test]
fn spec_uses_state_not_appstate_for_central_state() {
    // The central mutable state type is `State`, not `AppState`. The only
    // legitimate occurrence of `AppState` is the explanatory rename note.
    let appstate_mentions = SPEC_MD
        .lines()
        .filter(|l| l.contains("AppState"))
        // the explanatory note in §7 (and equivalent) is allowed to name the old type
        .filter(|l| {
            let lower = l.to_lowercase();
            !(lower.contains("旧称") || lower.contains("改名") || lower.contains("renamed"))
        })
        .collect::<Vec<_>>();
    assert!(
        appstate_mentions.is_empty(),
        "SPEC still references `AppState` in prose (should be `State`): {appstate_mentions:?}"
    );
}
