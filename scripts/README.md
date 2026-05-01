# scripts/dev-workflow.sh

Multi-agent development automation script. Runs a 13-step pipeline from research through PR creation, CI monitoring, and code review using pluggable AI agent backends (claude, codex, opencode).

## Usage

```bash
scripts/dev-workflow.sh <PLAN.md> <task> [issue-number]
```

| Argument | Required | Description |
|---|---|---|
| `PLAN.md` | yes | Implementation plan file to pass as context to all agents |
| `task` | yes | Task description string (used in commit messages, prompts, and PR titles) |
| `issue-number` | no | GitHub issue number; fetches title/body and injects into research notes |

### Examples

```bash
# Basic
scripts/dev-workflow.sh docs/PLAN.md "Phase 2.1: Dockerfile multi-stage build"

# With GitHub issue for context
scripts/dev-workflow.sh docs/PLAN.md "Phase 2.1: Dockerfile multi-stage build" 42

# Dry run (no agents invoked, no gh calls)
DEV_DRY_RUN=1 scripts/dev-workflow.sh docs/PLAN.md "Phase 2.1: ..." 42
```

## Pipeline steps

| Step | Label | Description |
|---|---|---|
| 0 | Research | Codebase survey; finds reusable patterns and notes pitfalls |
| 1 | Eval definition | Defines capability/regression evals and breaks task into units |
| 2 | TDD implementation | Red-green-refactor cycle; ≥80% coverage required |
| 3 | Cleanup | Removes test slop, debug prints, commented-out code |
| 4 | Verification | Build, lint, types, tests — fixes deterministic local failures |
| 5 | E2E tests | Runs `cargo test` integration tests (Playwright if configured); on failure attempts one fix-and-retry; writes `.dev-e2e-report.md` with root cause analysis and exits if still failing |
| 6 | Security review | OWASP Top 10 checklist; fixes CRITICAL/HIGH before continuing |
| 7 | Eval verification | Re-runs evals from step 1; exits 1 if any fail |
| 8 | Commit → PR | Conventional commit, push, `gh pr create` |
| 9 | CI monitor ① | Waits for green; auto-fixes failures up to `CI_MAX_ATTEMPTS` |
| 10 | Code review | Findings-first review; outputs APPROVE / REQUEST_CHANGES / BLOCK |
| 11 | Post review | Posts review as PR comment |
| 12 | Review remediation | Addresses CRITICAL/HIGH findings, pushes fix commit, CI monitor ② |
| post | Learning | Extracts 1–2 instincts via continuous-learning-v2 skill |

## Environment variables

### Agent family selection

Each role can be set to `claude`, `codex`, or `opencode`.

| Variable | Default | Role |
|---|---|---|
| `DEV_PLAN_AGENT_FAMILY` | `codex` | Steps 0, 1, 7 (research + eval) |
| `DEV_IMPL_AGENT_FAMILY` | `opencode` | Steps 2, 3, 12 (implementation) |
| `DEV_VERIFY_AGENT_FAMILY` | `opencode` | Step 4 (verification) |
| `DEV_REVIEW_AGENT_FAMILY` | `codex` | Steps 6, 10, 11 (review) |
| `DEV_E2E_AGENT_FAMILY` | `codex` | Step 5 (E2E tests) |
| `DEV_RELEASE_AGENT_FAMILY` | `opencode` | Steps 8, 11, post (release + learning) |
| `DEV_CI_AGENT_FAMILY` | `codex` | CI fix loop inside step 9 |

### Model overrides

Each role's model can be overridden independently.

| Variable | Default source |
|---|---|
| `MODEL_PLAN` | `default_model($DEV_PLAN_AGENT_FAMILY)` |
| `MODEL_IMPL` | `default_model($DEV_IMPL_AGENT_FAMILY)` |
| `MODEL_VERIFY` | `default_model($DEV_VERIFY_AGENT_FAMILY)` |
| `MODEL_CLEANUP` | `default_cleanup_model($DEV_IMPL_AGENT_FAMILY)` |
| `REVIEW_MODEL` | `default_model($DEV_REVIEW_AGENT_FAMILY)` |
| `MODEL_E2E` | `default_model($DEV_E2E_AGENT_FAMILY)` |
| `MODEL_RELEASE` | `default_model($DEV_RELEASE_AGENT_FAMILY)` |
| `MODEL_CI` | `default_model($DEV_CI_AGENT_FAMILY)` |

Default models per family: `claude` → `sonnet`, `codex` → `gpt-5.4`, `opencode` → `deepseek/deepseek-chat`.

### Other variables

| Variable | Default | Description |
|---|---|---|
| `DEV_DRY_RUN` | `0` | Set to `1` to print commands without invoking agents or `gh` |
| `DEV_SKILL_DIR` | auto-detected | Override skill file root directory |
| `DEV_SCRIPT_NAME` | `script/dev.sh` | Script name shown in usage output |
| `CI_MAX_ATTEMPTS` | `10` | Maximum CI fix attempts per monitor loop |
| `CI_PUSH_SETTLE_DELAY` | `30` | Seconds to wait after pushing a CI fix before re-checking |

## Required skill files

The script requires three skill files at startup (exits if missing):

- `<SKILL_DIR>/tdd-workflow/SKILL.md`
- `<SKILL_DIR>/verification-loop/SKILL.md`
- `<SKILL_DIR>/e2e-testing/SKILL.md`

Optional skills (warning emitted if missing, step continues):

- `eval-harness/SKILL.md`
- `security-review/SKILL.md`
- `review/SKILL.md`
- `ship-release/SKILL.md`
- `continuous-learning-v2/SKILL.md`

Skill directory resolution order: `.agents/skills/` → `.agent/skills/` → `~/.codex/skills/`.

## Checkpoint and resume

The script writes `.dev-task-checkpoint` after each completed step. If interrupted, re-running the same command resumes from the next step automatically. Delete `.dev-task-checkpoint` to force a full restart.

## Temporary files

| File | Purpose | Lifetime |
|---|---|---|
| `.dev-task-notes.md` | Shared scratchpad passed to all agents | Deleted on successful completion |
| `.dev-task-checkpoint` | Last completed step number | Deleted on successful completion |
| `.dev-e2e-report.md` | E2E test report with root cause analysis (if failed) | Deleted on success; persists on failure for diagnosis |
| `review-<PR>.md` | Raw code review output | Persists after run for reference |
