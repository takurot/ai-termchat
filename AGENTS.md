# AGENTS.md

This file provides guidance to OpenAI Codex and other agent runtimes when working with code in this repository.

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
cargo build --features avatar-ffi   # FFI avatar plugin loading
cargo test --features ui-test       # UI test helpers
```

Config file is auto-created at `~/.config/triadchat/config.toml` on first run.

## Architecture

**triadchat** is a terminal LAN chat application with an embedded AI clerk. Rust binary using:
- `message-io` for networking (UDP multicast discovery + TCP)
- `tui-rs` + crossterm for the TUI
- Claude Code CLI (`claude -p`) as the AI sidecar

### Module layout

| Module | Purpose |
|--------|---------|
| `src/application/` | Event loop; wires network, TUI, commands, AI |
| `src/state.rs` | All runtime state: messages, input, history, rooms, AI state |
| `src/ai/` | `AiMediator` → `SidecarAdapter` → `claude -p` subprocess |
| `src/room/` | `RoomEngine` multi-room state machine + transcript writer |
| `src/commands/` | `/`-prefixed command parser |
| `src/skill/` | Skill registry and executor |
| `src/ui/` | TUI layout panels |
| `src/avatar/` | Avatar rendering + optional FFI plugin |
| `src/config.rs` | TOML config |
| `src/message.rs` | Wire types: `AiPayload`, `PeerInfo`, `StructuredOutput` |
| `tests/` | Integration tests |

### Important design rules

1. **Single mutable state** — `State` in `src/state.rs` is the only mutable runtime struct. All mutations go through its public methods. Do not access fields directly from outside the module.

2. **AI task pipeline** — adding a new AI capability requires:
   - A variant in `AiTask` (`src/ai/mod.rs`)
   - A prompt builder in `src/ai/prompt.rs`
   - A match arm in `AiMediator::request`

3. **Commands** — each `/`-command family lives in `src/commands/<family>_cmd.rs`. Register new commands in `application/mod.rs` via `CommandManager::with(...)`.

4. **No `unwrap()` in production** — use `?` with `anyhow::Context` for error propagation. Reserve `unwrap()` / `expect()` for tests and provably-unreachable states.

5. **Input history** — `State::input_history` (Vec<String>) is appended on each non-whitespace submit. Navigation state is `history_cursor: Option<usize>` and `history_draft: String`.

### Data flow summary

```
KeyEvent
  └─► application::handle_input
        ├─► State::input_write / input_history_prev / input_history_next
        └─► CommandManager::find_command
              ├─► ParsedCommand::Action  → network / file I/O
              └─► ParsedCommand::App    → AppCommand handler
                    └─► AiMediator::request (async, spawned task)
                          └─► SidecarAdapter: claude -p <prompt>
                                └─► parse_ai_payload → State::add_ai_message
```

## Coding conventions

- `cargo fmt` before every commit.
- `cargo clippy -- -D warnings` — all warnings must be resolved.
- Use `anyhow` for application errors, `thiserror` for typed library errors.
- Prefer `&str` / `&[T]` parameters over owned types unless ownership is needed.
- Module visibility: default private, `pub(crate)` for internal sharing, `pub` only for the public API.
