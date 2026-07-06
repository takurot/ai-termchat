# Contributing to triadchat

Thanks for your interest in contributing! triadchat is a Rust terminal chat
app with an embedded AI clerk, forked from [termchat](https://github.com/lemunozm/termchat).

## Getting started

- **Rust toolchain:** stable, MSRV **1.82** (declared in `Cargo.toml`).
- **Build & run:**
  ```bash
  cargo run -- --username <your-name>
  ```
- **AI runtime (optional for chat, required for AI features):** the
  [Claude Code CLI](https://claude.ai/code) (`claude`) must be on `PATH`.

## Development workflow

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test`

Each file under `tests/` is a separate integration test binary; run a single
one with `cargo test --test <name>`.

### LLM-assisted development

This repo is designed to be worked on alongside an AI agent. The agent-facing
contracts live in:

- `CLAUDE.md` / `GEMINI.md` — build commands and architecture for Claude Code / Gemini CLI
- `docs/PROMPT.md` — execution contract for issue/PLAN-driven implementation
- `AGENTS.md` — behavioural guidelines for AI agents
- `scripts/dev-workflow.sh` — automated research → TDD → PR pipeline (see `scripts/README.md`)

If you are a human contributor, you can follow the same workflow manually;
`docs/PROMPT.md` describes the acceptance-criteria-first approach used for
issues.

## Code style & conventions

- Match the style of surrounding code; do not reformat unrelated code.
- Keep changes surgical — every changed line should trace to the task.
- New wire types go in `src/message.rs` with a `validate()` and, for
  `NetMessage` variants, **append at the end only** (bincode ordering).
- Document user-facing behaviour in `docs/SPEC.md`; the command surface in
  `README.md` must mirror `help_text()` in `src/application/mod.rs`.
- Add a regression test for bug fixes.

## Pull requests

- Use the [pull request template](.github/PULL_REQUEST_TEMPLATE.md).
- Reference the GitHub issue(s) being addressed (e.g. `Closes #123`).
- Ensure `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` pass.
- For security-sensitive changes, note the trust boundary that is affected.

## Reporting issues

Open a GitHub issue. For security issues, see [SECURITY.md](SECURITY.md)
(private reporting — do not use public issues).
