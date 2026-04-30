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
# AGENTS.md

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.
