# ai-termchat (triadchat)

> Terminal chat with a built-in AI clerk — no server, no GUI, just your LAN and a terminal.

**ai-termchat** is a fork of [termchat](https://github.com/lemunozm/termchat), extended into an "human + AI" or "human A · human B · AI" three-party operations terminal.

Chat in your terminal and the AI clerk automatically structures TODOs and decisions, and can trigger Claude Code skills on demand.

---

## Core Concept

```
10:01 takuro: this function has too many responsibilities
10:02 tanaka:  want to extract the auth layer
10:03 ops-ai ✦ Decision: separate auth into service layer (keep existing IF)
               TODO: takuro — design auth extraction
                     tanaka — regression check
               Proposal: [1] /skill review-auth
```

Just talk — the AI clerk structures the conversation for you. No commands to memorise.

---

## Features

- **No server required** — LAN multicast peer discovery, direct TCP connections
- **TUI** — runs entirely in the terminal
- **AI clerk** — `/summary` `/todos` `/decisions` instantly structure the conversation
- **Claude Code skill integration** — `/skill <name>` executes skills from `.claude/skills/`
- **AI modes** — Clerk / Companion / Listener / Moderator / Operator
- **`@ops-ai` mention** — direct replies from the AI inside any conversation
- **Input history** — Up/Down arrow keys navigate previously sent messages
- **Language config** — set AI output and UI language in `config.toml` (ja/en/zh/ko)
- **Avatar plugin** — swap ASCII avatars via preset names or FFI plugin (optional)

---

## Status

| Phase | Scope | Status |
|-------|-------|--------|
| **Phase 0** | Solo + AI. `/summary` `/todos` `/decisions` `/skill` | ✅ Implemented |
| **Phase 1** | LAN 2-player. 2-person + AI 3-party room. `/skill` execution | 🚧 In progress |
| **Phase 2** | ASCII avatar plugin + 3-pane TUI | 📋 Planned |

---

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustup update stable`)
- [Claude Code CLI](https://claude.ai/code) installed and authenticated (`claude` in PATH)

### Build & Run

```bash
git clone https://github.com/takurot/ai-termchat
cd ai-termchat
cargo run -- --username <your-name>
```

### Options

| Flag | Short | Description |
|------|-------|-------------|
| `--username <NAME>` | `-u` | Display name (default: system username) |
| `--discovery <IP:PORT>` | `-d` | Multicast address (default: `238.255.0.1:5877`) |
| `--tcp-server-port <PORT>` | `-t` | TCP listen port (default: random) |
| `--quiet-mode` | `-q` | Disable terminal bell |
| `--theme <dark\|light>` | | Color theme (default: dark) |

---

## Configuration

Config is auto-created at `~/.config/triadchat/config.toml` on first run.

```toml
[ai]
enabled = true
timeout_secs = 30
# command = "/path/to/claude"   # override if claude is not in PATH

[language]
ai_output = "en"   # ja | en | zh | ko
ui = "en"

[user]
avatar = "human_default"
ai_avatar = "ai_default"

[security]
default_permission = "confirm-required"
trusted_peers = []
```

---

## In-App Commands

| Command | Description |
|---------|-------------|
| `/summary` | AI summary of the conversation |
| `/todos` | Extract TODO items |
| `/decisions` | Extract decision items |
| `/skill <name> [args]` | Run a Claude Code skill |
| `/skills` | List available skills |
| `/room create [peers] [--ai <mode>]` | Create a room |
| `/room list` | List rooms |
| `/room switch <id>` | Switch active room |
| `/peers` | Show connected peers |
| `/avatar set <target> <preset>` | Change avatar preset |
| `/avatar list` | List available presets |
| `/ai mode <mode>` | Change AI mode |
| `/help` | Show help |

### AI Modes

| Mode | Behaviour |
|------|-----------|
| `clerk` | Intervenes after several human messages to summarise |
| `companion` | Responds conversationally to every message |
| `listener` | Silent — only responds to explicit `/summary` etc. |
| `moderator` | Summarises periodically at higher frequency |
| `operator` | Proposes skill executions proactively |

---

## Key Bindings

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Up` / `Down` | Navigate input history |
| `←` / `→` | Move cursor |
| `Ctrl+A` / `Ctrl+E` | Jump to start / end of input |
| `Ctrl+C` | Quit |
| `Page Up` / `Page Down` | Scroll message history |

---

## Tech Stack

- **Language:** Rust (edition 2021, MSRV 1.75)
- **Networking:** [message-io](https://github.com/lemunozm/message-io) (UDP multicast + TCP)
- **TUI:** [tui-rs](https://github.com/fdehau/tui-rs) + crossterm
- **AI:** Claude Code sidecar (`claude -p`)
- **Config:** TOML

---

## Documentation

| File | Contents |
|------|----------|
| [CLAUDE.md](CLAUDE.md) | Claude Code guidance — build commands, architecture |
| [docs/IDEA.md](docs/IDEA.md) | Product idea and concept |
| [docs/SPEC.md](docs/SPEC.md) | Feature specification (v0.3) |
| [docs/PLAN.md](docs/PLAN.md) | PR-level implementation plan |

---

## Based On

termchat v1.3.1 — [github.com/lemunozm/termchat](https://github.com/lemunozm/termchat)

---

## License

Apache-2.0
