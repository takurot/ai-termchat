# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] — 2026

### Added
- File transfer offer/accept/reject/cancel flow (`NetMessage::TransferOffer`/`Accept`/`Reject`/`Cancel`) with a 100 MB cap (`MAX_TRANSFER_SIZE`) and inbound size controls.
- Transport security: x25519 key exchange + ChaCha20Poly1305 framing (`src/secure.rs`, `NetMessage::Secure`) with replay protection.
- Peer identity verification via ed25519 signatures (`NetMessage::PeerIdentity`); authenticated endpoints reject plaintext downgrade.
- `AiProvider` switching (`/ai provider claude|codex|gemini|custom`).
- Art dictionary commands (`/art list`, `/art reload`).
- Multi-provider sidecar (`claude -p`, `codex exec`, `gemini -p`).

### Changed
- Phase 2 (avatar plugin + 3-pane TUI) is implemented.
- `bincode` upgraded from 1.x to 2.0.0-rc.3 (`legacy()` config preserves wire format).
- TUI migrated from `tui` 0.14 to `ratatui` 0.26.

## [0.1.1] — 2026

### Added
- Phase 1: LAN 2-player rooms with AI participant (`RoomCreateV2`, `RoomEngine`).
- Skill registry (`.claude/skills/` frontmatter scan) and async executor with confirmation UI.
- `/send` file transfer (termchat port), `/skills`, `/skill`, `/run`, `/cancel`.
- Transcript JSONL persistence (`~/.local/share/triadchat/transcripts/`).
- Trusted-peer model and per-endpoint identity binding (`Issue #90`).

## [0.1.0] — 2026

### Added
- Fork from termchat v1.3.1; rename to `triadchat`, command prefix `?` → `/`.
- Phase 0: solo + AI. `/summary`, `/todos`, `/decisions`, `/context`.
- Claude Code sidecar adapter with 30s timeout.
- `AiMediator` + intervention triggers; AI modes (clerk/listener/moderator/operator).
- `LanguageConfig` (AI output ja/en/zh/ko; UI ja/en) with `$LANG` auto-detection.
- Input history navigation (Up/Down).

[Unreleased]: https://github.com/takurot/ai-termchat/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/takurot/ai-termchat/releases/tag/v0.1.2
[0.1.1]: https://github.com/takurot/ai-termchat/releases/tag/v0.1.1
[0.1.0]: https://github.com/takurot/ai-termchat/releases/tag/v0.1.0
