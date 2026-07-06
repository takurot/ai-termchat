# Execution Prompt: Comprehensive Repository Evaluation and GitHub Issue Filing (triadchat)

Treat this document as the execution contract for a comprehensive evaluation of the current `triadchat` repository. Evaluate the product as it exists at the selected commit, identify evidence-backed bugs, improvements, test gaps, documentation gaps, and new feature opportunities, then publish the refined findings as GitHub Issues.

This is an evaluation workflow, not an implementation workflow. Do not fix product code during the evaluation because doing so changes the baseline being assessed. Temporary test fixtures, isolated configs, logs, screenshots, and evaluation scripts are allowed, but do not commit them unless the user explicitly requests it.

## Required Outcome

The evaluation is complete only when all of the following are true:

- The documented and implemented feature surface has been inventoried.
- A traceable evaluation plan has been written and reviewed by independent sub-agents.
- Existing automated tests, CI-equivalent checks, and relevant optional-feature checks have been executed.
- Critical user journeys have been evaluated end to end, including real terminal behavior and multi-node networking.
- Quality, security, reliability, usability, performance, compatibility, and documentation have been assessed.
- Every finding has reproducible evidence or is explicitly marked as a hypothesis requiring validation.
- The findings list has been reviewed from multiple independent perspectives and refined.
- Existing Issues and PRs have been checked for duplicates across open and closed states.
- Each validated, non-duplicate, actionable finding has been posted as a GitHub Issue and the posted Issue has been verified.
- A final evaluation summary records what passed, failed, was blocked, was not tested, and which Issues were created or matched.

Do not stop after planning, running tests, producing a findings list, or drafting Issue bodies. Continue through verified Issue creation unless GitHub access is unavailable.

## Sources of Truth

Read these sources before planning:

- `README.md`: advertised behavior, setup, CLI options, commands, and current status claims.
- `docs/SPEC.md`: architecture, failure modes, command contracts, non-functional requirements, and acceptance criteria.
- `docs/PLAN.md`: intended implementation milestones and completion claims.
- `docs/PROMPT.md`: repository execution and quality expectations.
- `Cargo.toml` and `Cargo.lock`: default and optional features, dependencies, and MSRV claims.
- `.github/workflows/ci.yml`: current CI gates.
- `src/`: actual runtime behavior and trust boundaries.
- `tests/`: existing deterministic, integration, and E2E coverage.
- `scripts/README.md`, `scripts/dev-workflow.sh`, and `scripts/test-dev-workflow.sh`: development automation behavior.
- GitHub Issues, PRs, releases, and recent commits: already-known defects, accepted constraints, and recent changes.

When sources disagree, do not silently choose one. Record the mismatch as evaluation evidence. For expected runtime behavior, prefer this order:

1. Explicit security and correctness requirements.
2. Current tested behavior that is intentional and user-visible.
3. `docs/SPEC.md` acceptance criteria and contracts.
4. README promises and CLI help text.
5. `docs/PLAN.md` completion claims.

## Non-Negotiable Rules

- Preserve unrelated user changes. Start with `git status --short --branch` and do not clean, reset, stash, or overwrite them.
- Record the exact commit SHA, branch, OS, architecture, Rust version, terminal, and important tool versions.
- Use isolated temporary directories for `HOME`, `XDG_CONFIG_HOME`, workspaces, received files, transcripts, skills, and avatar plugins. Do not alter the user's real config or credentials.
- Prefer deterministic code-based graders. Use model-based or human grading only for behavior that cannot be reduced to deterministic assertions.
- Distinguish `PASS`, `FAIL`, `BLOCKED`, and `NOT TESTED`. A blocked or unavailable check is never a pass.
- Record exact commands, exit codes, relevant logs, screenshots or terminal captures, and reproduction steps.
- Reproduce deterministic bugs at least twice. Run release-critical or historically flaky paths for at least three consecutive successful trials (`pass^3`) before calling them stable.
- Do not expose secrets, private conversation content, dangerous payloads, or weaponized security details in public Issues.
- Do not create speculative Issues. Unverified ideas may remain in the final report as hypotheses, but only validated and actionable items should be filed.
- Continue unaffected evaluation lanes when one external dependency is blocked.

## Phase 1: Establish the Baseline

Before drafting the evaluation plan:

1. Inspect repository and environment state.
2. Fetch current remote metadata when network access is available.
3. Capture the complete existing Issue and PR inventory for duplicate detection.
4. Read the implementation and tests sufficiently to identify actual feature boundaries.
5. Record known environmental constraints before executing tests.

Suggested baseline commands:

```sh
git status --short --branch
git rev-parse HEAD
git log -1 --oneline
git remote -v
rustc --version
cargo --version
gh issue list --state all --limit 200 --json number,title,state,labels,url,body
gh pr list --state all --limit 100 --json number,title,state,url,body
```

Build a feature-to-evidence inventory before testing. At minimum, inventory:

- Installation, build, startup, CLI flags, shutdown, and first-run behavior.
- Config loading, defaults, validation, persistence, themes, terminal bell, and language fallback.
- TUI rendering, input editing, cursor movement, history, scrolling, resize handling, and status panels.
- Chat messages, Unicode, mentions, ASCII art shortcodes, `/art list`, and `/art reload`.
- AI sidecar lifecycle, prompt construction, payload parsing, timeout, cancellation, and error reporting.
- `/summary`, `/todos`, `/decisions`, `/context`, and direct `@ops-ai` interaction.
- AI modes, frequency controls, automatic intervention, and non-blocking behavior.
- Peer discovery, direct `/peer connect`, peer metadata, fingerprints, and trust persistence.
- Room creation, listing, switching, membership, message routing, and AI mode propagation.
- Skill discovery, frontmatter parsing, invocation modes, approval, denial, cancellation, timeout, and output.
- `/send` file transfer, integrity, storage paths, interruption, and error handling.
- Transcript JSONL persistence, ordering, separation by room, privacy, and storage failures.
- Built-in avatars, avatar commands, terminal-size variants, and optional `avatar-ffi` loading.
- Development automation, checkpoint/resume behavior, edit scoping, and CI parity.
- README, SPEC, PLAN, CLI help, and actual implementation consistency.

## Phase 2: Draft the Evaluation Plan

Create a traceability matrix with one row for every feature, acceptance criterion, important failure mode, and non-functional requirement. Use at least these columns:

| ID | Area | Requirement or risk | Source | Existing test | Additional deterministic check | E2E/manual scenario | Environment | Evidence | Status | Finding IDs |
|---|---|---|---|---|---|---|---|---|---|---|

The draft plan must include:

- Scope and explicitly excluded areas.
- Assumptions and environmental prerequisites.
- Feature and acceptance-criteria inventory.
- Test and E2E scenarios, including negative and recovery paths.
- Fixtures and isolation strategy.
- Evidence storage and naming convention.
- Reliability repetitions and quality metrics.
- Security boundaries and tests that must not be destructive.
- Planned sub-agent responsibilities with no overlapping write ownership.
- Stop conditions and fallback paths for unavailable external services.
- Exit criteria for evaluation, finding validation, and Issue publication.

Do not begin the full evaluation while acceptance criteria remain unmapped or major product areas have no planned evidence source.

## Phase 3: Review and Refine the Plan with Sub-Agents

Send the draft plan to independent read-only sub-agents. Use at least these perspectives; run them sequentially if parallel capacity is unavailable:

1. **Architecture and specification reviewer**
   - Check feature inventory completeness, architecture boundaries, protocol compatibility, and conflicts among README, SPEC, PLAN, tests, and code.

2. **Test and E2E reviewer**
   - Check happy paths, negative paths, recovery, determinism, flakiness controls, platform coverage, and whether tests exercise the real runtime path rather than only helpers.

3. **Security and reliability reviewer**
   - Check network trust, untrusted decoding, file paths, process execution, skill permissions, config persistence, denial of service, resource bounds, and failure recovery.

4. **Product, UX, and accessibility reviewer**
   - Check terminal usability, discoverability, feedback, localization, small-terminal behavior, keyboard-only operation, error messages, and user-visible consistency.

5. **Maintainer and operations reviewer**
   - Check CI parity, dependency health, diagnostics, documentation, portability, upgrade behavior, issue-worthiness, and long-term maintenance risks.

Require every reviewer to return:

- Missing areas or scenarios.
- Invalid assumptions.
- Tests that do not prove the claimed behavior.
- Risky or destructive evaluation steps.
- Better deterministic graders.
- Suggested acceptance criteria and evidence.
- A verdict: `READY`, `REVISE`, or `BLOCKED`.

The primary evaluator owns the integrated plan. Reconcile conflicting advice, record material rejected recommendations with a reason, update the traceability matrix, and proceed only when all critical gaps are resolved.

## Phase 4: Execute Existing Verification

Run the repository's exact CI-equivalent checks first, then broader checks. Preserve complete output and timing.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --tests
cargo build --release
```

Then run broader applicable checks:

```sh
cargo test --all-targets
cargo test --all-features --all-targets
cargo build --release --all-features
cargo test --doc
scripts/test-dev-workflow.sh
```

Also evaluate:

- Default features separately from `--all-features` so optional-feature failures are attributable.
- Debug and release behavior where timing, overflow, or optimization could matter.
- The documented Rust 1.75 MSRV if that toolchain is already available; otherwise record it as unverified.
- Linux CI results and the current local platform. Do not claim cross-platform support from a single platform run.
- Test coverage and untested production paths. Use `cargo llvm-cov` if already available; otherwise perform source-to-test mapping and record the tooling gap.
- Dependency duplication with `cargo tree -d`.
- Dependency advisories with `cargo audit` if available. If unavailable, report the missing security check instead of silently skipping it.
- Secret-like material and unsafe external process or filesystem usage through focused source inspection. Treat examples and placeholders separately from real secrets.

For every failure, determine whether it is reproducible, environment-specific, flaky, already known, or caused by unrelated local changes. Do not fix it during this workflow.

## Phase 5: Execute Comprehensive E2E Evaluation

Automated integration tests are necessary but not sufficient. Exercise the compiled application through PTYs or real terminal sessions and use at least two isolated local processes for networking. Use two physical hosts as an additional check when available, but do not block local two-node evaluation on missing hardware.

### Environment Isolation

- Use unique usernames, config directories, transcript directories, discovery ports, and TCP ports for every node.
- Use an isolated workspace containing controlled `.claude/skills/` fixtures.
- Use a deterministic mock sidecar for repeatable functional checks.
- Run live Claude Code checks only when authenticated credentials are already available and the test content contains no sensitive data.
- Capture terminal dimensions, locale, config, commands, timestamps, and relevant logs for every scenario.

### A. Installation, Startup, and CLI

- Clean build and release startup.
- Every documented CLI flag, invalid flag, invalid address, occupied port, and missing dependency path.
- First-run config creation, existing config loading, malformed or partial config, read-only config, and safe shutdown.
- AI-enabled and AI-disabled startup.
- Missing, non-executable, failing, and slow `claude` command behavior.

### B. Single-Node Chat and TUI

- Send plain, empty, long, multi-line, Unicode, CJK, emoji, and mention-containing messages.
- Input history, cursor movement, insertion/deletion, start/end shortcuts, scrolling, and resize during activity.
- Narrow, short, large, and unusual terminal dimensions without panic, overlap, or hidden critical status.
- Light and dark themes, terminal bell behavior, and keyboard-only operation.
- `/help`, unknown commands, malformed arguments, and recovery after errors.
- `/art list`, valid shortcode replacement, unknown shortcode behavior, live reload, malformed YAML, Unicode art, and oversized art.

### C. AI Functional and Quality Evaluation

With a deterministic mock sidecar, verify:

- `/summary`, `/todos`, `/decisions`, and `/context` success and error paths.
- Valid, multi-line, partially malformed, invalid JSON, missing field, oversized, and non-UTF-8-safe output handling.
- Thinking, success, failure, timeout, cancellation, and subsequent recovery states.
- No blocking of terminal input, network processing, or rendering while AI work is running.
- Listener, clerk, moderator, operator, and any implemented companion behavior.
- Frequency controls, explicit `@ops-ai` mentions, automatic intervention thresholds, and false-positive intervention cases.
- Slash commands are excluded from conversational transcripts where required.

When live Claude Code is available, use a fixed, non-sensitive conversation corpus covering development review, incident response, planning, ambiguity, conflicting statements, missing assignees, and multilingual conversations. Grade:

- Factual faithfulness: no invented decisions, tasks, owners, or status.
- Completeness: material decisions and TODOs are not omitted.
- Assignment accuracy and uncertainty handling.
- Separation of decisions, TODOs, summaries, and skill suggestions.
- Language compliance for `ja`, `en`, `zh`, and `ko` AI output settings.
- Prompt injection resistance from chat content and skill names.
- Latency, timeout rate, malformed-output rate, and recovery.

Use a documented rubric. Run each critical corpus case up to three times and report `pass@1`, `pass@3`, and stability where meaningful. Do not hide nondeterminism behind retries.

### D. Peer Discovery, Direct Connect, Trust, and Rooms

- Start two nodes in both startup orders and verify multicast discovery where the environment permits it.
- Verify `/peer connect <host:port>` as a deterministic fallback.
- Confirm symmetric peer readiness, usernames, versions, avatars, fingerprints, and no duplicate peer state.
- Test trust list/add/remove by peer and fingerprint, restart persistence, malformed entries, and persistence failure.
- Verify an untrusted peer cannot trigger privileged skill execution.
- Create, list, switch, and use rooms; verify membership and AI mode propagation on both nodes.
- Exchange interleaved messages in both directions and verify ordering, room isolation, mention display, and transcript consistency.
- Test unknown peers, duplicate usernames, repeated connection attempts, disconnect, reconnect, one-sided shutdown, stale room state, port conflicts, and discovery loss.
- Exercise backward/unknown protocol variants only with bounded, non-destructive fixtures.

Critical two-node paths must pass three consecutive runs before being considered stable.

### E. Skill Discovery and Execution

- Missing skills directory, empty directory, valid skills, duplicate names, malformed YAML, missing fields, unsupported values, and unreadable files.
- `/skills` completeness and consistency with discovered metadata.
- Manual, confirm-required, and any automatic invocation policy.
- Direct `/skill`, AI proposal plus `/run`, accept, reject, `/cancel`, timeout, non-zero exit, large output, and recovery.
- Argument quoting, spaces, Unicode, shell metacharacters, and attempts to escape the workspace or allowed tools.
- Local proposals, trusted remote proposals, and untrusted remote proposals.
- UI responsiveness and network responsiveness during long-running skills.
- No stale pending confirmation or running state after success, failure, cancellation, or disconnect.

### F. File Transfer

- No argument, nonexistent path, directory path, unreadable file, empty file, small text, binary, Unicode filename, spaces, and a representative large file.
- Send in both directions and verify content using a cryptographic hash, not only file length.
- Concurrent chat and AI activity during transfer.
- Interrupted sender or receiver, duplicate filename, repeated transfer, write failure, and cleanup of partial files.
- Network-sourced filenames containing traversal, absolute paths, separators, control characters, or extreme lengths. Use safe temporary targets and do not write outside the isolated root.
- Clear sender and receiver status for start, progress where supported, success, and failure.

### G. Transcript and Persistence

- Valid JSONL after single-node and multi-room conversations.
- Correct timestamp, sender, room, message type, Unicode, and ordering.
- Commands and sensitive operational details are excluded where the contract requires it.
- Room separation, restart behavior, partial/corrupt trailing records, read-only directory, disk-write failure, and concurrent activity.
- No path traversal or cross-user leakage from room IDs or peer-provided values.

### H. Avatars, Layout, and Optional FFI

- Every built-in preset across all states and compact/normal/expressive sizes.
- `/avatar list`, set, preview, mode, unknown target, unknown preset, and remote avatar metadata.
- Three-pane layout, peers panel, room list, status panel, scroll regions, and resize boundaries.
- Default build behavior with external plugins disabled.
- `avatar-ffi` build and controlled plugin discovery/loading on supported platforms.
- Invalid ABI version, missing symbols, malformed plugin metadata, plugin failure, duplicate presets, and unsupported extension handling without crashing the host.

### I. Configuration, Localization, and Documentation

- Default values and generated config match README, CLI help, and SPEC.
- `LANG` fallback and explicit `ja`, `en`, `zh`, and `ko` AI language settings.
- Implemented UI locale coverage and fallback for unsupported locale values.
- Invalid enum, address, timeout, permission, avatar, and numeric values produce safe and actionable behavior.
- README command tables, config examples, status claims, SPEC acceptance criteria, PLAN completion state, and actual runtime behavior agree.
- All user-visible commands are discoverable through `/help` and documented where intended.

### J. Reliability, Performance, and Resource Behavior

- Startup and idle CPU/memory observations.
- Long transcript rendering and scrolling.
- Sustained message bursts and repeated AI requests without unbounded queue growth.
- Repeated connect/disconnect and room switching.
- Large sidecar output, slow sidecar, and sidecar process cleanup.
- Large or repeated file transfers within safe local limits.
- Graceful handling of terminal resize storms and shutdown during active work.
- No leaked child processes, temporary files, sockets, or locked config/transcript files after exit.

Use measurements to support performance findings. Do not file performance Issues based only on subjective impressions.

## Phase 6: Assess Security and Abuse Cases

Review all trust boundaries with both source inspection and safe runtime checks:

- Untrusted network frames before deserialization and allocation.
- Peer identity, spoofing, replay, trust persistence, and authorization decisions.
- Remote room IDs, usernames, filenames, avatar names, and message content.
- File destination construction, overwrite behavior, traversal, and partial-file cleanup.
- Skill names, arguments, frontmatter, allowed tools, shell/process execution, and remote proposals.
- Claude prompt construction, chat-sourced prompt injection, malformed model output, and data leakage.
- Config permissions, transcript contents, logs, error messages, and environment variables.
- Dynamic library discovery and ABI validation under `avatar-ffi`.
- Resource exhaustion through frames, messages, files, rooms, sidecar output, or repeated connections.

For a credible high-impact vulnerability, minimize public exploit detail. Check for a repository security policy and use the appropriate private reporting path when available. If only public Issues are available, provide enough information to identify and fix the boundary without publishing secrets or a weaponized proof of concept.

## Phase 7: Build the Findings Ledger

Record every candidate in a structured ledger before filing Issues:

| Field | Required content |
|---|---|
| Finding ID | Stable local identifier |
| Type | Bug, security, reliability, performance, UX, accessibility, documentation, test gap, maintenance, or new feature |
| Severity | Critical, high, medium, or low |
| Priority | P0, P1, P2, or P3 |
| Confidence | Confirmed, likely, or hypothesis |
| Requirement | Expected behavior and source |
| Actual behavior | Observed result without interpretation |
| Reproduction | Minimal deterministic steps and frequency |
| Environment | Commit, OS, toolchain, config, and feature flags |
| Evidence | Logs, output, screenshot, capture, test, and file/line references |
| Impact | Affected users, security boundary, data, or workflow |
| Root cause | Confirmed cause or clearly labeled inference |
| Existing coverage | Tests that should or should not have caught it |
| Proposed acceptance criteria | Verifiable completion conditions |
| Duplicate status | Matching Issue/PR or search evidence |
| Recommended disposition | File, comment on existing Issue, retain as hypothesis, or discard |

Severity guidance:

- **Critical:** credible remote code execution, broad data loss, secret exposure, or unusable core product with no workaround.
- **High:** security boundary bypass, common-path corruption, persistent data loss, or a core advertised workflow that reliably fails.
- **Medium:** significant incorrect behavior, reliability problem, inaccessible workflow, or material documentation mismatch with a workaround.
- **Low:** localized defect, polish issue, narrow test gap, or low-impact maintenance problem.

Separate severity from priority. A high-severity finding may be lower priority if unreachable in supported configurations, and a medium-severity issue may be P1 if it affects the primary workflow.

## Phase 8: Review and Refine Findings with Sub-Agents

Give the evidence packet and findings ledger to independent read-only sub-agents. Review the list from these perspectives:

- Correctness and regression risk.
- Security, privacy, and abuse resistance.
- Networking, concurrency, persistence, and operational reliability.
- Product value, terminal UX, accessibility, and localization.
- Test strategy, reproducibility, and missing coverage.
- Architecture, maintainability, dependencies, and documentation.
- GitHub triage quality: root-cause grouping, severity, priority, duplicates, and actionable acceptance criteria.

Require reviewers to challenge unsupported claims, identify false positives, find missing findings, detect duplicate root causes, and distinguish defects from deliberate product decisions. They must cite evidence, not merely vote.

The primary evaluator must:

- Validate reviewer claims against code and runtime evidence.
- Merge findings that share one root cause and split findings that require independent fixes or acceptance criteria.
- Remove speculative or non-actionable items from the Issue queue.
- Reproduce disputed high-impact findings again.
- Update severity, priority, scope, and acceptance criteria.
- Preserve useful new feature proposals only when they solve an observed user or operational gap.

## Phase 9: Duplicate Check and GitHub Issue Publication

Refresh GitHub state immediately before filing:

```sh
gh issue list --state all --limit 200 --json number,title,state,labels,url,body
gh pr list --state all --limit 100 --json number,title,state,url,body
```

For every validated finding:

1. Search open and closed Issues using the feature, symptom, error text, affected file, and likely root-cause keywords.
2. Search PRs and recent commits for an existing or already-merged fix.
3. Re-test against the evaluation commit and, when practical, the latest base branch before claiming an active defect.
4. If an open Issue already covers the root cause, add concise new evidence only when it materially helps; do not create a duplicate.
5. If a closed Issue appears to have regressed, comment with current reproduction evidence or create a clearly linked regression Issue only when reopening/commenting is insufficient.
6. Use only labels that already exist in the repository. Do not invent labels silently.
7. Create one Issue per independently actionable root cause. Avoid one Issue per test case and avoid vague umbrella Issues.
8. Verify every created Issue with `gh issue view` and record its URL.

Use this Issue body structure:

```markdown
## Summary
Concise statement of the validated problem or opportunity.

## Type and Priority
- Type: bug | security | reliability | performance | UX | documentation | test gap | maintenance | feature
- Severity: critical | high | medium | low
- Priority: P0 | P1 | P2 | P3

## Environment
- Commit:
- OS / architecture:
- Rust / Cargo:
- Features and config:

## Reproduction or Evidence
1. Minimal step
2. Minimal step

```text
Relevant output, redacted where necessary
```

## Expected Behavior
Evidence-backed expectation with source reference.

## Actual Behavior
Observed behavior and reproduction frequency.

## Impact
Who or what is affected and why it matters.

## Relevant Code and Tests
- `path/to/file.rs:line`
- Existing or missing test coverage

## Proposed Acceptance Criteria
- [ ] Verifiable criterion
- [ ] Regression or E2E coverage
- [ ] Documentation or compatibility update when applicable

## Evaluation Evidence
- Evaluation commit and scenario ID
- Related Issue/PR links and duplicate-search notes
```

For a new feature proposal, replace reproduction steps with the observed gap, target user, use case, alternatives considered, smallest useful scope, risks, and measurable acceptance criteria. Do not prescribe a large implementation when a smaller change would solve the observed problem.

## Phase 10: Final Evaluation Report

Produce a concise final report with:

- Evaluation commit, environment, date, and scope.
- Overall verdict and confidence.
- Traceability totals: planned, passed, failed, blocked, and not tested.
- Automated verification results with exact commands.
- E2E scenarios executed and repetition counts.
- AI quality metrics and grader limitations.
- Security, reliability, performance, UX, accessibility, portability, and documentation summaries.
- Created Issue numbers and URLs, grouped by priority.
- Existing Issues that matched findings.
- Findings not filed and the reason: duplicate, insufficient evidence, intentional behavior, or hypothesis.
- Blockers and exact resume instructions.
- Residual risk and highest-value next evaluation work.

Do not report the repository as fully evaluated if matrix rows remain `NOT TESTED` or `BLOCKED`. State the actual coverage boundary explicitly.

## Allowed Stop Conditions

Stop the whole workflow only when:

- Repository access is unavailable.
- Continuing would overwrite unrelated user work.
- Evaluation requires destructive access to real user data or infrastructure.
- A credible security test would be unsafe to perform.

GitHub authentication failure blocks Issue publication but not local evaluation. Preserve the refined Issue bodies and report the exact authentication error. Missing Claude credentials block only live-model quality evaluation; complete deterministic mock evaluation and mark live checks `BLOCKED`. Missing multicast support blocks only that transport scenario; complete direct-connect two-node evaluation and record the limitation.
