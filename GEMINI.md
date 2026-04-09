# GEMINI.md

This file provides guidance to Gemini CLI when working with code in this repository.

## Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run -- --username <name>

# Test
cargo test                          # All tests
cargo test <pattern>                # Tests matching pattern
cargo test --lib                    # Unit tests only
cargo test --test <file>            # Specific integration test (tests/<file>.rs)
cargo test -- --nocapture           # Show println output

# Lint & format
cargo fmt
cargo clippy -- -D warnings

# Feature flags
cargo build --features avatar-ffi   # Enable dynamic avatar plugin loading via FFI
cargo test --features ui-test       # Enable UI test helpers
```

Config file is auto-created at `~/.config/triadchat/config.toml` on first run.

## Architecture

**triadchat** is a terminal LAN chat application with an embedded AI clerk. It is a Rust binary built on:
- `message-io` — UDP multicast peer discovery + TCP data transport
- `tui-rs` + crossterm — terminal UI
- Claude Code (`claude -p`) — AI sidecar subprocess

### Module layout

| Module | Purpose |
|--------|---------|
| `src/application/` | Top-level event loop; wires together network, TUI, commands, and AI |
| `src/state.rs` | Central mutable state: messages, input buffer, history, rooms, AI state |
| `src/ai/` | AI integration: `AiMediator` dispatches `AiTask` variants → `SidecarAdapter` → `claude -p` |
| `src/room/` | `RoomEngine` (multi-room state machine) + `transcript.rs` (YAML/JSON log) |
| `src/commands/` | `/`-prefixed command parser; each submodule handles one command family |
| `src/skill/` | Skill registry and executor; runs `.claude/skills/<name>` via sidecar |
| `src/ui/` | TUI layout panels: messages, peers, room list, status |
| `src/avatar/` | Avatar rendering; optional FFI plugin loading (`avatar-ffi` feature) |
| `src/config.rs` | TOML config struct; auto-created at `~/.config/triadchat/config.toml` |
| `src/message.rs` | Shared wire types (serde): `AiPayload`, `PeerInfo`, `StructuredOutput`, IDs |
| `tests/` | Integration tests (each `.rs` file is a separate test binary) |

### Data flow

```
Terminal events
      │
      ▼
application::mod  ──► CommandManager ──► AppCommand / Action
      │
      ├──► State (input, messages, rooms, AI state)
      │
      ├──► AiMediator.request(AiTask, transcript, last_messages)
      │         └──► SidecarAdapter ("claude -p" subprocess)
      │                   └──► parse_ai_payload → AiPayload
      │
      └──► TUI renderer (ui/ panels)
```

### Key types

- **`State`** (`src/state.rs`) — single mutable struct holding all runtime state. Input history (`input_history`, `history_cursor`, `history_draft`) lives here. All mutations go through methods on `State`.
- **`AiTask`** — enum of AI task kinds (Summary, Todos, Decisions, Intervene, Companion, Mention). Each maps to a prompt builder in `src/ai/prompt.rs`.
- **`AiMode`** — Clerk / Listener / Moderator / Operator / Companion. Controls when the AI intervenes automatically.
- **`AppCommand`** — parsed result of `/`-commands; handled in `application/mod.rs`.

### AI sidecar

The AI runs as a subprocess (`claude -p <prompt>`). `SidecarAdapter` manages the process, timeout (default 30 s), and stdout capture. Prompts are truncated to 50 000 characters. `parse_ai_payload` in `src/ai/parser.rs` converts raw output into `AiPayload` (text + optional `StructuredOutput`).

### Networking

Peer discovery via UDP multicast (`238.255.0.1:5877`); data transport over TCP (random port by default). Phase 1 adds explicit Room negotiation messages (`src/net_message_phase1.rs`).

### Input history

`State::input_history` is a `Vec<String>` appended on each non-whitespace submit. Up/Down arrow keys call `input_history_prev` / `input_history_next`. A `history_draft` field saves unsent text before entering history mode and is restored when the user navigates past the last entry.

## Coding conventions

- Run `cargo fmt` before committing.
- Run `cargo clippy -- -D warnings` and fix all warnings.
- Use `anyhow` for application-level errors; `thiserror` for library-level typed errors.
- Never use `unwrap()` in production paths — use `?` or `.with_context(...)`.
- All state mutations go through methods on `State`; do not access fields directly from outside the module.
- New AI task types require: a variant in `AiTask`, a prompt builder in `src/ai/prompt.rs`, and a match arm in `AiMediator::request`.
