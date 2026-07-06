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
- **Language config** — AI output: ja/en/zh/ko · UI: ja/en (zh/ko fall back to `en` for UI), set in `config.toml`
- **Avatar plugin** — swap ASCII avatars via preset names or FFI plugin (optional)

---

## Status

| Phase | Scope | Status |
|-------|-------|--------|
| **Phase 0** | Solo + AI. `/summary` `/todos` `/decisions` `/skill` | ✅ Implemented |
| **Phase 1** | LAN 2-player. 2-person + AI 3-party room. `/skill` execution + transport security (ChaCha20Poly1305) | ✅ Implemented |
| **Phase 2** | ASCII avatar plugin + 3-pane TUI | ✅ Implemented |

---

## Quick Start

### Prerequisites

- Rust 1.82+ (`rustup update stable`)
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
# Flat keys (overridable by CLI flags)
discovery_addr   = "238.255.0.1:5877"
tcp_server_port  = 0
user_name        = "your-name"
terminal_bell    = true

[language]
ai_output = "en"   # ja | en | zh | ko
ui        = "en"   # ja | en (zh/ko fall back to en)

[ai]
enabled      = true
provider     = "claude"   # claude | codex | gemini | custom
# command    = "/path/to/claude"   # override if claude is not in PATH
timeout_secs = 30

[security]
default_permission = "confirm-required"
trusted_peers      = []

[user]
avatar    = "human_default"
ai_avatar = "ai_default"

# [theme] is auto-generated; edit only if you want custom colours.
```

---

## In-App Commands

> The in-app `/help` is the canonical, grouped reference. This table mirrors it.

### AI

| Command | Description |
|---------|-------------|
| `/ai mode <mode>` | Change AI behaviour mode: `clerk` `listener` `moderator` `operator` `companion` |
| `/ai quiet <on\|off>` | Mute/unmute AI responses |
| `/ai freq <low\|normal\|high>` | Adjust AI intervention frequency |
| `/ai provider <provider>` | Switch AI engine: `claude` `codex` `gemini` `custom` |

### Summary

| Command | Description |
|---------|-------------|
| `/summary` | AI summary of the conversation |
| `/todos` | Extract TODO items |
| `/decisions` | Extract decision items |
| `/context` | Summarise conversation context |

### Rooms

| Command | Description |
|---------|-------------|
| `/room create @user [--ai <mode>]` | Create a room with peers (AI mode optional) |
| `/room list` | List all rooms |
| `/room switch <id\|name>` | Switch active room |

### Peers

| Command | Description |
|---------|-------------|
| `/peers` | Show connected peers |
| `/peer connect <host:port>` | Connect to a peer directly |
| `/trust list` | List trusted peer fingerprints |
| `/trust add <peer\|fp>` | Trust a peer explicitly |
| `/trust remove <peer\|fp>` | Remove stored peer trust |

### Skills

| Command | Description |
|---------|-------------|
| `/skills` | List available skills |
| `/skill <name> [args]` | Run a Claude Code skill |
| `/run <id>` | Accept a skill proposal from AI |
| `/cancel` | Cancel current AI task or skill |

### Avatar

| Command | Description |
|---------|-------------|
| `/avatar set <target> <preset>` | Set avatar (target: `self`, `@ops-ai`) |
| `/avatar list` | List available avatar presets |
| `/avatar preview` | Preview your current avatar |
| `/avatar mode <size>` | Set size: `compact` `normal` `expressive` |

### Art

| Command | Description |
|---------|-------------|
| `/art list` | List configured art shortcodes |
| `/art reload` | Reload `art.yaml` |

### Files

| Command | Description |
|---------|-------------|
| `/send <file>` | Send a file to peers in the room |

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

- **Language:** Rust (edition 2021, MSRV 1.82)
- **Networking:** [message-io](https://github.com/lemunozm/message-io) (UDP multicast + TCP)
- **TUI:** [ratatui](https://github.com/ratatui/ratatui) 0.26 + crossterm
- **Crypto:** ed25519-dalek (peer identity) + x25519-dalek + ChaCha20Poly1305 (transport)
- **AI:** Claude Code sidecar (`claude -p`)
- **Config:** TOML

---

## Documentation

| File | Contents |
|------|----------|
| [CLAUDE.md](CLAUDE.md) | Claude Code guidance — build commands, architecture |
| [GEMINI.md](GEMINI.md) | Gemini CLI guidance — build commands, architecture |
| [docs/IDEA.md](docs/IDEA.md) | Product idea and concept (historical) |
| [docs/SPEC.md](docs/SPEC.md) | Feature specification |
| [docs/PLAN.md](docs/PLAN.md) | PR-level implementation plan (milestones) |
| [docs/PROMPT.md](docs/PROMPT.md) | Execution contract for issue/PLAN-driven implementation |
| [docs/EVAL.md](docs/EVAL.md) | Evaluation methodology for comprehensive review |
| [scripts/README.md](scripts/README.md) | Multi-agent dev pipeline — automated research → TDD → PR workflow |

---

## Based On

termchat v1.3.1 — [github.com/lemunozm/termchat](https://github.com/lemunozm/termchat)

---

## License

Apache-2.0
