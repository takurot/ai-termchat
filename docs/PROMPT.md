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

When the user specifies an Issue number, that Issue is the required scope. Do not replace it with another Issue based on the general priority rules. Read the Issue body, comments, labels, linked PRs, and dependencies before planning.

Prefer GitHub Issues when:
- Issues include security, bug, or specific Phase 1/2 implementation gaps.
- The Issue has clear acceptance criteria (e.g., "Verify /summary works on 2 nodes").

Use `docs/PLAN.md` directly when:
- Bootstrapping a new Phase or PR (e.g., moving from Phase 0 PoC to Phase 1 MVP).
- Sequencing tasks based on the dependency graph (PR-01 -> PR-02 -> ...).

## Specified Issue End-to-End Workflow

For a specified GitHub Issue `#N`, execute the following workflow autonomously from planning through merge. Do not stop after producing a plan, implementing locally, opening a PR, or posting review findings. Continue until the PR is merged unless a documented external blocker makes further progress impossible.

**Autonomous Pipeline Overview (7 phases, loop until merged):**

```
1. PLAN        — Inspect Issue, write implementation plan
2. PLAN-REVIEW — Sub-agents review plan from multiple perspectives → refine until solid
3. IMPLEMENT   — TDD: red-green-refactor; test at the appropriate level
4. PUBLISH     — Tests + E2E green → commit, push, open PR with evidence
5. PR-REVIEW   — Sub-agents review PR diff from multiple perspectives → post findings
6. FIX         — Address findings → commit, push → loop to 5 if fix changes behavior/boundaries
7. MERGE       — Tests + E2E + CI green → merge
```

Select and load the appropriate skill at each phase (see [Skill Selection Guide](#skill-selection-guide) below). Use sub-agents for parallel independent reviews. If sub-agent spawning is unavailable, perform reviews sequentially in the primary agent, loading each skill in turn.

### 1. Inspect and Plan

- Fetch Issue `#N` and its discussion with `gh`; confirm that it is open, not superseded, and not blocked by an unmet dependency.
- Inspect the relevant code, tests, `docs/SPEC.md`, and `docs/PLAN.md`. Treat the current tested behavior as the baseline.
- Create an Issue-specific branch and isolated worktree when the current checkout is dirty or parallel work could conflict.
- Write an implementation plan that identifies scope, assumptions, acceptance criteria, affected files and interfaces, dependencies, risks, test cases, E2E coverage, and required documentation updates.
- Resolve ambiguities from repository evidence where possible. Ask the user only when a product decision cannot be inferred safely or the requested action would be destructive.

### 2. Review and Refine the Plan

- Load `plan-architecture` skill first to structure the planning approach.
- Send the draft plan to independent read-only sub-agents before implementation.
- Instruct each sub-agent to load a specific skill for its review perspective before analyzing the plan:
  - `plan-architecture` — architecture, dependencies, compatibility, unnecessary complexity
  - `review` — correctness, edge cases, regression risk, Issue acceptance criteria
  - `security-review` — security, validation, concurrency, data integrity, operational risk
  - `tdd-workflow` — test strategy, E2E coverage gaps, verification steps
- Cover at least these perspectives, combining roles only for a genuinely small change:
  - architecture, dependencies, compatibility, and unnecessary complexity;
  - correctness, edge cases, regression risk, and Issue acceptance criteria;
  - security, validation, concurrency, data integrity, and operational risk where applicable;
  - test strategy, E2E coverage gaps, and verification completeness.
- Require reviewers to identify concrete omissions, contradictions, and unverifiable steps rather than merely approve the plan.
- Reconcile conflicting feedback, record any rejected recommendation with a reason, and update the plan. The primary agent owns the final plan and integration decisions.
- Begin implementation only after the refined plan has explicit verification steps for every acceptance criterion.

### 3. Implement with TDD

- Load `tdd-workflow` skill for test-first methodology guidance.
- Follow red-green-refactor for each behavioral unit: add a failing test, make the smallest implementation pass, then clean up without changing behavior.
- **Always add tests for new features.** Add regression tests for bugs and integration/E2E tests for user-visible or cross-component flows.
- **Choose test level by bug type:**
  - Isolated logic, parsing, or runtime bugs → unit test in `src/`
  - Cross-component or user-visible flow bugs → integration or E2E test in `tests/`
  - Networking, multi-node, or end-to-end acceptance bugs → E2E with `Application::new_for_test()`
- **E2E test infrastructure (this project):** Use `Application::new_for_test()` for single-node flows, multiple `Application` instances with distinct `discovery_port` for multi-node flows, and mock the AI sidecar via `write_executable_script()` + `config_with_ai_script()`. Run isolated: `cargo test --test <file> <test_name>`. Place tests in `tests/` alongside the relevant feature file.
- When a bug arises, write a failing test at the appropriate level before fixing.
- Keep changes within the Issue scope. Update `docs/SPEC.md`, `docs/PLAN.md`, and user-facing documentation when behavior or contracts change.
- Run focused tests during development and the full local verification suite before publishing.
- Prefer `scripts/dev-workflow.sh <reviewed-plan> "<task>" N` when it fits the task, but do not skip any mandatory step in this contract when automation is unavailable or incomplete.

### 4. Publish the Pull Request

- Run formatting (`cargo fmt`) and verify with `cargo fmt --check`, and run linting (`cargo clippy`) locally to ensure zero diffs on style. Specifically verify that formatting rules (such as putting small expressions on a single line like `validate` logic) are fully applied to avoid CI `Format` job failures.
- Review the final diff for scope, generated files, debug artifacts, secrets, and unrelated changes.
- Create a conventional commit, push the Issue branch, and open a PR that links the Issue with `Closes #N`.
- Include the refined plan, acceptance-criteria checklist, verification evidence, E2E results, risks, and any intentional limitations in the PR description.

### 5. Perform Multi-Perspective PR Review

- After the PR exists, assign independent read-only sub-agents to review the actual PR diff, not the intended plan alone. Load `review` skill in the primary agent before coordinating reviews.
- Instruct each sub-agent to load a specific skill before reviewing:
  - `review` — correctness, regression risk, code quality, and unintended side effects
  - `plan-architecture` — architecture fit, maintainability, interface consistency
  - `tdd-workflow` — test/E2E coverage, missing test cases, verification gaps
  - `security-review` — security, data safety, input validation, concurrency risks
  - `verification-loop` — build, lint, type, and test completeness
  - `coding-standards` — naming, structure, validation, and error-handling consistency
  - `dmux-workflows` — coordinate parallel sub-agent review sessions
  - If the change touches APIs: `api-design`
  - If the change touches backend/data flows: `backend-patterns`
- Require findings-first output with severity, rationale, concrete file and line references, and a proposed remediation or missing test.
- Reviewers must identify concrete omissions, contradictions, and unverifiable steps rather than merely approve.
- Deduplicate and validate findings against the source before posting them. Post all valid actionable findings to the PR; if no issues are found, post an explicit approval summary with the checks performed.

### 6. Remediate and Re-Review

- Address every valid actionable finding. Add or strengthen regression tests before fixing behavioral defects.
- For rejected or deferred findings, post a concrete technical rationale and confirm that deferral does not violate the Issue acceptance criteria.
- Commit and push remediation changes, then re-run affected reviewers when the fix changes behavior, security boundaries, shared interfaces, or test strategy.
- Repeat review and remediation until there are no unresolved blocking findings.

### 7. Verify CI and Merge

- Run formatting (`cargo fmt`), linting (`cargo clippy`), unit tests, integration tests, security checks, and relevant E2E scenarios locally. Load `verification-loop` skill to ensure full coverage. Always ensure formatting rules are perfectly applied locally before checking CI.
- Monitor all required CI checks. For deterministic failures, diagnose, fix, commit, push, and monitor CI again rather than stopping at the failure report.
- Before merging, confirm the PR is mergeable, required approvals are present, the branch is current enough for repository policy, all required checks are green, and no blocking review thread remains unresolved.
- Merge using the repository's standard strategy, verify the merged state, and close the Issue if GitHub did not close it automatically.
- Remove the merged worktree and branch when safe. Preserve unrelated user changes.

### Allowed Stop Conditions

Stop and report the blocker only when progress requires unavailable credentials or infrastructure (e.g., GitHub `gh` CLI authentication errors), an unresolved external dependency, a destructive decision requiring authorization, or a product decision that cannot be inferred safely. Record the exact blocker and completed evidence in the Issue or PR so execution can resume without repeating work.

## Skill Selection Guide

Load the appropriate skill with the `skill` tool at the start of each phase. Do not proceed without the relevant skill loaded when one exists for the task at hand.

| Phase | Skill | Purpose |
|-------|-------|---------|
| 1. PLAN (inspect) | `plan-architecture` | Structure the planning approach, identify affected interfaces |
| 1. PLAN (research) | `deep-research` | Multi-source investigation for unfamiliar domains |
| 1. PLAN (research) | `documentation-lookup` | Primary docs lookup for library API and version-specific behavior |
| 2. PLAN-REVIEW | `plan-architecture` | Architecture, dependencies, compatibility review |
| 2. PLAN-REVIEW | `review` | Correctness, edge cases, regression risk |
| 2. PLAN-REVIEW | `security-review` | Security, validation, concurrency, data integrity |
| 2. PLAN-REVIEW | `tdd-workflow` | Test strategy, E2E coverage gaps |
| 3. IMPLEMENT | `tdd-workflow` | Red-green-refactor discipline, test-first methodology |
| 3. IMPLEMENT (debug) | `investigate` | Root-cause analysis for bugs, regressions, and stack traces |
| 4. PUBLISH | `verification-loop` | Build, lint, type, test — full quality gate |
| 4. PUBLISH | `ship-release` | Release prep, documentation updates, PR readiness |
| 5. PR-REVIEW | `review` | Correctness, regression, code quality |
| 5. PR-REVIEW | `plan-architecture` | Architecture fit, maintainability |
| 5. PR-REVIEW | `security-review` | Security, data safety, concurrency |
| 5. PR-REVIEW | `tdd-workflow` | Test/E2E coverage, missing cases |
| 5. PR-REVIEW | `verification-loop` | Build/lint/type/test completeness |
| 5. PR-REVIEW | `coding-standards` | Naming, structure, error handling |
| 5. PR-REVIEW | `dmux-workflows` | Coordinate parallel sub-agent review sessions |
| 5. PR-REVIEW | `api-design` | API validation (if APIs changed) |
| 5. PR-REVIEW | `backend-patterns` | Backend patterns (if backend/data flows changed) |
| 6. FIX | `tdd-workflow` | Test-first remediation of defects |
| 6. FIX | `investigate` | Root-cause analysis (if finding needs investigation) |
| 7. MERGE | `verification-loop` | Final quality gate before merge |
| 7. MERGE | `health-check` | Build, lint, type, test, security pass/fail summary |
| All phases | `guard` | Combined safety: destructive warnings + edit boundaries |
| All phases | `dmux-workflows` | Parallel task orchestration across sub-agents and worktrees |

Use `find-skills` to discover additional skills when needed. When a skill is loaded, follow its workflow instructions precisely.

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
- **Choose Test Level by Context:**
  - Isolated logic, parsing, or runtime → unit test in `src/`
  - Cross-component or user-visible flow → integration or E2E test in `tests/`
  - Networking, multi-node, or end-to-end acceptance → E2E with `Application::new_for_test()`
- **New Features Require Tests:** Every new feature must include corresponding tests — unit tests for logic, integration/E2E tests for user-visible or cross-component behavior.
- **Surgical Changes:** Follow the guidelines in AGENTS.md. Touch only what you must. Do not "improve" or refactor adjacent code, comments, or formatting unless requested.
- **No Unwraps:** Never use `unwrap()` or `expect()` in production paths. Use `anyhow` for app-level errors and `thiserror` for library-level errors.
- **State Mutation:** All state changes must go through methods on `AppState` in `src/state.rs`.
- **Async Safety:** AI and Skill tasks must be spawned on the custom `tokio` runtime; never block the `message-io` event loop.
- **Claude Code Integration:** Use `SidecarAdapter` for all `claude -p` interactions with a 30s timeout.
- **Python Dependency Management:** If a task requires managing Python packages, always use `pybun` (preferring the system PATH, and falling back to `/Users/takurot/Library/Python/3.14/bin/pybun` if not globally available) instead of standard `pip` or `venv`.
- **Format Consistency:** Always run `cargo fmt` before staging files. Ensure complex expressions or helper functions adhere to the cargo format style (e.g. avoiding unnecessary multi-line breaks for short logic statements) to prevent CI Format check failures.

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
