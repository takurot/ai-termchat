# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development automation

`scripts/dev-workflow.sh` runs a 13-step multi-agent pipeline (research → TDD → verification → PR → CI → review) for implementing a planned task autonomously.

```bash
# Basic
scripts/dev-workflow.sh docs/PLAN.md "Phase 2.1: task description"

# With GitHub issue for background context
scripts/dev-workflow.sh docs/PLAN.md "Phase 2.1: task description" 42

# Dry run — prints commands without invoking agents
DEV_DRY_RUN=1 scripts/dev-workflow.sh docs/PLAN.md "Phase 2.1: task description"
```

See `scripts/README.md` for the full pipeline reference, environment variables, and skill file requirements.

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

Config file is written automatically on first run to `~/.config/triadchat/config.toml`.

## Architecture

**triadchat** is a terminal LAN chat app with an embedded AI clerk. It is a Rust binary built on `message-io` (networking), `ratatui` + crossterm (TUI), and Claude Code (`claude -p`) as a sidecar AI process.

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
| `src/secure.rs` | Transport security: X25519 key exchange + ChaCha20Poly1305 framing for `NetMessage::Secure` |
| `src/config.rs` | TOML config struct; auto-created at `~/.config/triadchat/config.toml` |
| `src/message.rs` | Shared wire types (serde): `NetMessage`, `AiPayload`, `PeerInfo`, `StructuredOutput`, IDs |
| `src/encoder.rs` | bincode encode/decode for wire framing (termchat-inherited) |
| `src/renderer.rs` | `Renderer` draw orchestration (termchat-inherited) |
| `src/action.rs` | `Action` trait for termchat-style actions (termchat-inherited) |
| `src/terminal_events.rs` | crossterm input event polling (termchat-inherited) |
| `src/util.rs` | Shared utilities and error type (termchat-inherited) |
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

- **`State`** (`src/state.rs`) — single mutable struct holding all runtime state. Input history (`input_history`, `history_cursor`, `history_draft`) lives here. Mutations go through methods on `State`; no field is mutated directly from outside.
- **`AiTask`** — enum of AI task kinds (Summary, Todos, Decisions, Intervene, Companion, Mention). Each maps to a prompt builder in `src/ai/prompt.rs`.
- **`AiMode`** — Clerk / Listener / Moderator / Operator / Companion. Controls when the AI intervenes automatically.
- **`AppCommand`** — parsed result of `/`-commands; handled in `application/mod.rs`.

### AI sidecar

The AI runs as a subprocess (`claude -p <prompt>`). `SidecarAdapter` manages the process, timeout, and stdout capture. `parse_ai_payload` in `src/ai/parser.rs` converts the raw output into `AiPayload` (text + optional `StructuredOutput`).

### Networking

Peer discovery via UDP multicast (`238.255.0.1:5877`); data transport over TCP (random port by default). `message-io` handles both. Room negotiation and Phase 1 wire types are exercised in `tests/net_message_phase1.rs`. After peer identity verification, TCP payloads upgrade to `NetMessage::Secure` (ChaCha20Poly1305) via `src/secure.rs`.

### Input history

`State::input_history` is a `Vec<String>` appended on each non-whitespace submit. Up/Down arrow keys call `input_history_prev` / `input_history_next`. A `history_draft` field saves unsent text before entering history mode; it is restored when the user presses Down past the last entry. All history logic is unit-tested inside `src/state.rs`.
