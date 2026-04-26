# Execution Prompt: Issue- and PLAN-Driven Parallel Implementation (triadchat)

Do not begin coding blindly. First, treat this document as the execution contract for implementing `triadchat` from both `docs/PLAN.md` tasks and GitHub Issues.

The executor must be able to work in two modes:

- PLAN mode: execute planned tasks from `docs/PLAN.md` (Phase 0, 1, 2).
- Issue mode: select the highest-value GitHub Issues, infer dependencies, and execute them as implementation tasks.

When GitHub Issues exist, prefer Issue mode for ongoing development because Issues reflect the current state of the codebase, discovered gaps, security findings, and post-plan work.

## Primary Objectives

- Deliver the highest-priority unblocked work from GitHub Issues or `docs/PLAN.md`.
- Use TDD (Test-Driven Development) as the default implementation method.
- Use git branches, worktrees, and sub-agents to parallelize independent tasks when doing so is safe.
- Verify every task (Networking, AI logic, UI rendering) before integration.
- Open PRs for meaningful task or milestone boundaries (e.g., PR-01 to PR-16 in PLAN.md).
- Review, fix, verify, and merge PRs only after CI (cargo test, clippy) is green.
- Repeat until no high-priority ready tasks remain or the requested scope is complete.

## Source of Truth

- GitHub Issues: current implementation backlog, security findings, priority labels.
- `docs/SPEC.md`: product behavior, architecture decisions (AiMediator, SidecarAdapter, RoomEngine), API contracts, data model, and acceptance criteria.
- `docs/PLAN.md`: task decomposition by PRs, dependency graph, Phase 0/1/2 milestones, and verification strategy.
- Repository code: Rust implementation patterns, existing tests, and TUI behavior.

If these sources conflict, prefer this order:

1. Existing tested behavior (Rust unit and integration tests).
2. Security Issues and explicit bug Issues.
3. `docs/SPEC.md`.
4. GitHub Issue acceptance criteria.
5. `docs/PLAN.md`.
6. This prompt.

When a real implementation constraint (e.g., `message-io` thread limits, `tokio` runtime conflicts) requires changing the plan, update the relevant document and explain the reason.

## Initial Setup

Before selecting work:

- Inspect the repository state with `git status` and `cargo check`.
- Identify uncommitted user changes and do not overwrite them.
- Fetch remote branches and Issues before scheduling.
- Confirm CI status (github actions) for the base branch when possible.

Recommended initial commands:

```sh
git status --short --branch
git fetch --all --prune
gh issue list --state open --limit 50
cargo check
cargo test --lib
```

## Work Source Selection

At each scheduling point, build a candidate list from open GitHub Issues and incomplete `docs/PLAN.md` tasks.

Prefer GitHub Issues when:
- Issues include security, bug, or specific Phase 1/2 implementation gaps.
- The Issue has clear acceptance criteria (e.g., "Verify /summary works on 2 nodes").

Use `docs/PLAN.md` directly when:
- Bootstrapping a new Phase or PR (e.g., moving from Phase 0 PoC to Phase 1 MVP).
- Sequencing tasks based on the dependency graph (PR-01 -> PR-02 -> ...).

## Priority Rules

Rank ready Issues by impact and risk:

1. **Networking Stability:** Peer discovery (UDP multicast), TCP transport reliability, and `NetMessage` backward compatibility.
2. **AI Precision:** Prompt accuracy in `src/ai/prompt.rs`, payload parsing in `src/ai/parser.rs`, and sidecar timeout handling.
3. **Skill Execution:** Non-blocking `tokio` spawn, approval UI workflows, and skill timeout management.
4. **Data Integrity:** `Transcript` JSONL persistence and room state consistency.
5. **UI/UX:** TUI responsiveness, 3-pane layout correctness, and ASCII avatar rendering.
6. **Maintenance:** Dependency updates, linting (clippy), and formatting (rustfmt).

## Parallel Execution Policy

Parallelize only when tasks are genuinely independent.

Safe parallel candidates:
- AI prompt refinement (`src/ai/prompt.rs`) and UI layout tweaks (`src/ui/layout.rs`).
- Command implementation (`src/commands/`) and documentation updates.
- Independent Phase 1 tasks (e.g., PR-11 and PR-12) once their common upstream (PR-10) is merged.

Do not parallelize when:
- Multiple tasks modify the central `AppState` in `src/state.rs`.
- Changing the `NetMessage` enum (affects all network-related PRs).
- One task redesigns the `tokio` runtime while another depends on it.

## Worktree Policy

Use separate worktrees for independent tasks.

Naming convention:
- Branch: `feat/<pr-number>-short-name` or `issue/<number>-short-name`
- Worktree path: `../triadchat-feat-<name>`

Before creating a worktree:
- Confirm dependencies are satisfied (e.g., don't start PR-05 if PR-04 isn't stable).
- Check for file ownership conflicts (especially `src/state.rs` or `src/application/mod.rs`).

## TDD and Implementation Rules

For every task:
- **Test First:** Write unit tests in `src/` or integration tests in `tests/` before implementation.
- **No Unwraps:** Never use `unwrap()` or `expect()` in production paths. Use `anyhow` for app-level errors and `thiserror` for library-level errors.
- **State Mutation:** All state changes must go through methods on `AppState` in `src/state.rs`.
- **Async Safety:** AI and Skill tasks must be spawned on the custom `tokio` runtime; never block the `message-io` event loop.
- **Claude Code Integration:** Use `SidecarAdapter` for all `claude -p` interactions with a 30s timeout.

## Verification Policy

Run verification at three levels.

**1. Task-level verification:**
- `cargo test <module>` or `cargo test --test <integration_file>`.
- `cargo clippy -- -D warnings` and `cargo fmt --check`.

**2. Milestone-level (Phase) verification:**
- Phase 0: Manual verification of `/summary` and `/todos` in single-node mode.
- Phase 1: Two-node network integration test (`tests/network_integration.rs`) and skill execution flow.
- Phase 2: Avatar rendering checks and 3-pane layout verification.

**3. Release-level verification:**
- Full `cargo test` suite.
- Manual E2E check: 2+ nodes, AI intervention, skill approval, and file transfer.

## CI and Merge Loop

Merge only when:
- `cargo clippy` and `cargo test` pass locally.
- Networking backward compatibility is verified (if applicable).
- Documentation (SPEC.md/PLAN.md) is updated.

## Completion Criteria

A task or Phase is complete only when:
- Acceptance criteria in `SPEC.md` or the Issue are met.
- Unit and integration tests pass.
- AI behavior is verified via golden tests (`tests/prompt_quality.rs`).
- The PR is merged and the branch/worktree is cleaned up.
