#!/bin/bash

set -euo pipefail

DEV_AGENT_FAMILY="${DEV_AGENT_FAMILY:-opencode}"
DEV_PLAN_AGENT_FAMILY="${DEV_PLAN_AGENT_FAMILY:-codex}"
DEV_IMPL_AGENT_FAMILY="${DEV_IMPL_AGENT_FAMILY:-$DEV_AGENT_FAMILY}"
DEV_VERIFY_AGENT_FAMILY="${DEV_VERIFY_AGENT_FAMILY:-$DEV_IMPL_AGENT_FAMILY}"
DEV_REVIEW_AGENT_FAMILY="${DEV_REVIEW_AGENT_FAMILY:-codex}"
DEV_E2E_AGENT_FAMILY="${DEV_E2E_AGENT_FAMILY:-codex}"
DEV_RELEASE_AGENT_FAMILY="${DEV_RELEASE_AGENT_FAMILY:-opencode}"
DEV_CI_AGENT_FAMILY="${DEV_CI_AGENT_FAMILY:-codex}"
DEV_SCRIPT_NAME="${DEV_SCRIPT_NAME:-script/dev.sh}"
DEV_DRY_RUN="${DEV_DRY_RUN:-0}"

default_skill_dir() {
  if [ -d ".agents/skills" ]; then
    printf '.agents/skills'
  elif [ -d ".agent/skills" ]; then
    printf '.agent/skills'
  else
    printf '%s/.codex/skills' "$HOME"
  fi
}

default_model() {
  local agent_family="$1"

  case "$agent_family" in
    claude)
      printf 'sonnet'
      ;;
    codex)
      printf 'gpt-5.4'
      ;;
    opencode)
      printf 'deepseek/deepseek-chat'
      ;;
    *)
      echo "ERROR: unsupported agent family: $agent_family" >&2
      exit 1
      ;;
  esac
}

default_cleanup_model() {
  local agent_family="$1"

  case "$agent_family" in
    claude)
      printf 'haiku'
      ;;
    codex)
      printf 'gpt-5.4'
      ;;
    opencode)
      printf 'deepseek/deepseek-chat'
      ;;
    *)
      echo "ERROR: unsupported agent family: $agent_family" >&2
      exit 1
      ;;
  esac
}

PRIMARY_SKILL_DIR="${DEV_SKILL_DIR:-$(default_skill_dir)}"
MODEL_PLAN="${MODEL_PLAN:-$(default_model "$DEV_PLAN_AGENT_FAMILY")}"
MODEL_IMPL="${MODEL_IMPL:-$(default_model "$DEV_IMPL_AGENT_FAMILY")}"
MODEL_VERIFY="${MODEL_VERIFY:-$(default_model "$DEV_VERIFY_AGENT_FAMILY")}"
MODEL_CLEANUP="${MODEL_CLEANUP:-$(default_cleanup_model "$DEV_IMPL_AGENT_FAMILY")}"
REVIEW_MODEL="${REVIEW_MODEL:-$(default_model "$DEV_REVIEW_AGENT_FAMILY")}"
MODEL_E2E="${MODEL_E2E:-$(default_model "$DEV_E2E_AGENT_FAMILY")}"
MODEL_RELEASE="${MODEL_RELEASE:-$(default_model "$DEV_RELEASE_AGENT_FAMILY")}"
MODEL_CI="${MODEL_CI:-$(default_model "$DEV_CI_AGENT_FAMILY")}"

PLAN="${1:-}"
TASK="${2:-}"
ISSUE="${3:-}"

SKILL_TDD="$PRIMARY_SKILL_DIR/tdd-workflow/SKILL.md"
SKILL_VERIFY="$PRIMARY_SKILL_DIR/verification-loop/SKILL.md"
SKILL_EVAL="$PRIMARY_SKILL_DIR/eval-harness/SKILL.md"
SKILL_LEARNING="$PRIMARY_SKILL_DIR/continuous-learning-v2/SKILL.md"
SKILL_SECURITY="$PRIMARY_SKILL_DIR/security-review/SKILL.md"
SKILL_E2E="$PRIMARY_SKILL_DIR/e2e-testing/SKILL.md"
SKILL_REVIEW="$PRIMARY_SKILL_DIR/review/SKILL.md"
SKILL_SHIP="$PRIMARY_SKILL_DIR/ship-release/SKILL.md"

NOTES_FILE=".dev-task-notes.md"
CHECKPOINT_FILE=".dev-task-checkpoint"
E2E_REPORT_FILE=".dev-e2e-report.md"
REVIEW_FILE=""
PR_NUMBER=""
PR_URL=""
VERDICT=""

# チェックポイントから再開位置を決定
RESUME_FROM=0
if [ -f "$CHECKPOINT_FILE" ]; then
  _completed=$(cat "$CHECKPOINT_FILE" 2>/dev/null || echo "-1")
  if [ "$_completed" -ge 0 ] 2>/dev/null; then
    RESUME_FROM=$(( _completed + 1 ))
    echo "チェックポイント検出: ステップ $_completed 完了済み → ステップ $RESUME_FROM から再開"
  else
    echo "WARN: チェックポイントファイルが不正な値を含んでいます。ステップ 0 から再開します。" >&2
  fi
fi

is_dry_run() {
  [ "$DEV_DRY_RUN" = "1" ]
}

build_codex_review_prompt() {
  local pr_number="$1"
  local task="$2"
  local notes_file="$3"
  local plan_file="$4"

  cat <<EOF
$(load_skill "$SKILL_REVIEW")

---
Review PR #$pr_number for task: $task

Read:
- $plan_file
- $notes_file if present
- git diff for the PR

Use a findings-first review. Prioritize correctness, regressions, security risks,
missing validation, and missing tests. Cite concrete files and lines where
possible. Keep summaries secondary.

Output:
## Code Review

### Findings
Group findings by severity: CRITICAL, HIGH, MEDIUM, LOW.
Use "none" for an empty severity.

### Verdict
APPROVE, REQUEST_CHANGES, or BLOCK.
EOF
}

usage() {
  echo "Usage: $DEV_SCRIPT_NAME <PLAN.md path> <task> [issue-number]"
  echo "Example: $DEV_SCRIPT_NAME temp/PLAN.md 'Phase 2.1: Dockerfile multi-stage build'"
  echo "Example: $DEV_SCRIPT_NAME temp/PLAN.md 'Phase 2.1: Dockerfile multi-stage build' 42"
}

require_command() {
  local command_name="$1"

  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "ERROR: required command not found: $command_name" >&2
    exit 1
  fi
}

require_file() {
  local file_path="$1"

  if [ ! -f "$file_path" ]; then
    echo "ERROR: required file not found: $file_path" >&2
    exit 1
  fi
}

require_agent_family() {
  local agent_family="$1"

  case "$agent_family" in
    claude|codex|opencode)
      require_command "$agent_family"
      ;;
    *)
      echo "ERROR: unsupported agent family: $agent_family" >&2
      exit 1
      ;;
  esac
}

require_unique_agent_families() {
  local seen=""
  local agent_family

  for agent_family in "$@"; do
    case " $seen " in
      *" $agent_family "*) ;;
      *)
        require_agent_family "$agent_family"
        seen="$seen $agent_family"
        ;;
    esac
  done
}

load_skill() {
  local skill_path="$1"
  local skill_name
  local candidate

  if [ -f "$skill_path" ]; then
    cat "$skill_path"
    return 0
  fi

  skill_name=$(basename "$(dirname "$skill_path")")
  for candidate in \
    ".agents/skills/$skill_name/SKILL.md" \
    ".agent/skills/$skill_name/SKILL.md" \
    "$HOME/.codex/skills/$skill_name/SKILL.md" \
    "$HOME/.claude/skills/$skill_name/SKILL.md"; do
    if [ -f "$candidate" ]; then
      cat "$candidate"
      return 0
    fi
  done

  echo "WARN: skill file not found: $skill_path" >&2
  printf 'Skill file not found: %s\n' "$skill_path"
}

prepare_prompt_for_agent() {
  local agent_family="$1"
  local allowed_tools="$2"
  local prompt_text="$3"

  if [ "$agent_family" = "codex" ] && [ -n "$allowed_tools" ]; then
    printf 'Tool guardrails: Prefer restricting yourself to these tool categories when they are relevant: %s\n\n%s' \
      "$allowed_tools" \
      "$prompt_text"
    return 0
  fi

  if [ "$agent_family" = "opencode" ]; then
    printf 'Agent role: OpenCode executor using DeepSeek. Keep the work mechanical and scoped.\nFollow the supplied plan and repository patterns exactly. Do not broaden architecture, invent unrelated features, or perform release/review decisions unless this prompt explicitly asks for them.\n\n%s' \
      "$prompt_text"
    return 0
  fi

  printf '%s' "$prompt_text"
}

describe_exec_command() {
  local agent_family="$1"
  local model_name="$2"
  local allowed_tools="$3"

  case "$agent_family" in
    claude)
      if [ -n "$allowed_tools" ]; then
        printf 'claude -p --model "%s" --allowedTools "%s" -- "<prompt>"' "$model_name" "$allowed_tools"
      else
        printf 'claude -p --model "%s" -- "<prompt>"' "$model_name"
      fi
      ;;
    codex)
      printf 'codex exec -m "%s" --sandbox workspace-write -' "$model_name"
      ;;
    opencode)
      printf 'opencode run -m "%s" --dir "%s" -- "<prompt>"' "$model_name" "$PWD"
      ;;
  esac
}

describe_review_command() {
  local agent_family="$1"
  local model_name="$2"

  case "$agent_family" in
    claude)
      printf 'claude -p --model "%s" -- "<review prompt>"' "$model_name"
      ;;
    codex)
      printf 'codex review -c model="%s" -' "$model_name"
      ;;
    opencode)
      printf 'opencode run -m "%s" --dir "%s" -- "<review prompt>"' "$model_name" "$PWD"
      ;;
  esac
}

run_agent_exec() {
  local agent_family="$1"
  local model_name="$2"
  local allowed_tools="$3"
  local prompt_text
  local prepared_prompt

  prompt_text=$(cat)
  prepared_prompt=$(prepare_prompt_for_agent "$agent_family" "$allowed_tools" "$prompt_text")

  if is_dry_run; then
    echo "DRY RUN EXEC [$agent_family]: $(describe_exec_command "$agent_family" "$model_name" "$allowed_tools")"
    return 0
  fi

  case "$agent_family" in
    claude)
      if [ -n "$allowed_tools" ]; then
        claude -p --model "$model_name" --allowedTools "$allowed_tools" -- "$prepared_prompt"
      else
        claude -p --model "$model_name" -- "$prepared_prompt"
      fi
      ;;
    codex)
      printf '%s' "$prepared_prompt" | codex exec -m "$model_name" --sandbox workspace-write -
      ;;
    opencode)
      opencode run -m "$model_name" --dir "$PWD" -- "$prepared_prompt"
      ;;
  esac
}

run_agent_review() {
  local agent_family="$1"
  local model_name="$2"
  local prompt_text

  prompt_text=$(cat)

  if is_dry_run; then
    echo "DRY RUN REVIEW [$agent_family]: $(describe_review_command "$agent_family" "$model_name")"
    return 0
  fi

  case "$agent_family" in
    claude)
      claude -p --model "$model_name" -- "$prompt_text"
      ;;
    codex)
      printf '%s' "$prompt_text" | codex review -c "model=\"$model_name\"" -
      ;;
    opencode)
      opencode run -m "$model_name" --dir "$PWD" -- "$prompt_text"
      ;;
  esac
}

print_runtime_summary() {
  echo "Plan/research agent: $DEV_PLAN_AGENT_FAMILY"
  echo "Implementation agent: $DEV_IMPL_AGENT_FAMILY"
  echo "Verification agent: $DEV_VERIFY_AGENT_FAMILY"
  echo "Review agent: $DEV_REVIEW_AGENT_FAMILY"
  echo "E2E/debug agent: $DEV_E2E_AGENT_FAMILY"
  echo "Release agent: $DEV_RELEASE_AGENT_FAMILY"
  echo "CI fix agent: $DEV_CI_AGENT_FAMILY"
  echo "Skill directory: $PRIMARY_SKILL_DIR"
  echo "Model (plan): $MODEL_PLAN"
  echo "Model (impl): $MODEL_IMPL"
  echo "Model (verify): $MODEL_VERIFY"
  echo "Model (cleanup): $MODEL_CLEANUP"
  echo "Model (review): $REVIEW_MODEL"
  echo "Model (e2e): $MODEL_E2E"
  echo "Model (release): $MODEL_RELEASE"
  echo "Model (ci): $MODEL_CI"

  if is_dry_run; then
    echo "Dry run: enabled"
  fi
}

initialize_notes_file() {
  cat >"$NOTES_FILE" <<EOF
# Dev Task Notes: $TASK
Started: $(date '+%Y-%m-%d %H:%M:%S')

## Research Findings
(populated in Step 0)

## Known Patterns / Constraints
(populated by steps as discovered)

## CI Fix History
(populated by CI loop when failures occur)
EOF
}

checkpoint() {
  echo "$1" > "$CHECKPOINT_FILE"
}

inject_issue_context() {
  local issue_number="$1"

  if ! [[ "$issue_number" =~ ^[0-9]+$ ]]; then
    echo "ERROR: issue番号が数値ではありません: '$issue_number'" >&2
    exit 1
  fi

  echo "Issue #$issue_number の情報を取得中..."

  local issue_title issue_body issue_labels issue_url

  gh issue view "$issue_number" --json title,body,labels,url > /dev/null 2>&1 || {
    echo "ERROR: Issue #$issue_number の取得に失敗しました。番号とリポジトリを確認してください。" >&2
    exit 1
  }

  issue_title=$(gh issue view "$issue_number" --json title --jq '.title' 2>/dev/null || echo "(title unavailable)")
  issue_body=$(gh issue view "$issue_number" --json body --jq '.body' 2>/dev/null || echo "(body unavailable)")
  issue_labels=$(gh issue view "$issue_number" --json labels --jq '[.labels[].name] | join(", ")' 2>/dev/null || echo "none")
  issue_url=$(gh issue view "$issue_number" --json url --jq '.url' 2>/dev/null || echo "unavailable")

  cat >>"$NOTES_FILE" <<EOF

## GitHub Issue Context

**Issue #$issue_number**: $issue_title
**Labels**: $issue_labels
**URL**: $issue_url

### Issue Body

$issue_body
EOF

  echo "Issue #$issue_number の情報を $NOTES_FILE に注入しました。"
}

cleanup_files() {
  : # intentionally empty — NOTES_FILE is removed only on success (line ~909)
}

trap cleanup_files EXIT

if [ -z "$PLAN" ] || [ -z "$TASK" ]; then
  usage
  exit 1
fi

require_file "$PLAN"

require_file "$SKILL_TDD"
require_file "$SKILL_VERIFY"
require_file "$SKILL_E2E"

if ! is_dry_run; then
  require_command gh
  require_unique_agent_families \
    "$DEV_PLAN_AGENT_FAMILY" \
    "$DEV_IMPL_AGENT_FAMILY" \
    "$DEV_VERIFY_AGENT_FAMILY" \
    "$DEV_REVIEW_AGENT_FAMILY" \
    "$DEV_E2E_AGENT_FAMILY" \
    "$DEV_RELEASE_AGENT_FAMILY" \
    "$DEV_CI_AGENT_FAMILY"
fi

if [ "$RESUME_FROM" -eq 0 ]; then
  initialize_notes_file
  if [ -n "$ISSUE" ]; then
    if is_dry_run; then
      echo "DRY RUN: gh issue view $ISSUE を取得して $NOTES_FILE に注入"
    else
      inject_issue_context "$ISSUE"
    fi
  fi
fi

wait_for_ci_green() {
  local label="${1:-CI}"
  local attempt=0

  if is_dry_run; then
    echo "DRY RUN CI WAIT: $label"
    return 0
  fi

  echo ""
  echo "--- CI 監視開始: $label ---"

  while [ $attempt -lt "${CI_MAX_ATTEMPTS:-10}" ]; do
    attempt=$((attempt + 1))
    echo ""
    echo "  CI チェック (試行 $attempt/${CI_MAX_ATTEMPTS:-10})"
    echo "  CI 完了を待機中..."
    gh pr checks "$PR_NUMBER" --watch 2>/dev/null || true

    CI_STATUS=$(gh pr checks "$PR_NUMBER" --json name,state \
      --jq '[.[] | select(.state != "SUCCESS" and .state != "SKIPPED")] | length' 2>/dev/null || echo "error")

    if [ "$CI_STATUS" = "error" ]; then
      echo "ERROR: gh pr checks の実行に失敗しました (ネットワーク/認証エラーの可能性)。手動確認が必要です: $PR_URL" >&2
      exit 1
    fi

    if [ "$CI_STATUS" = "0" ]; then
      echo "  CI オールグリーン ✓ ($label)"
      return 0
    fi

    echo "  CI 失敗あり ($CI_STATUS 件)。ログを取得して修正します..."

    CI_FAILURES=$(gh pr checks "$PR_NUMBER" --json name,state,link \
      --jq '.[] | select(.state != "SUCCESS" and .state != "SKIPPED") | "- \(.name): \(.state) \(.link)"' \
      2>/dev/null || echo "")

    {
      echo ""
      echo "### CI Fix Attempt $attempt ($label) — $(date '+%H:%M:%S')"
      echo "$CI_FAILURES"
    } >>"$NOTES_FILE"

    run_agent_exec "$DEV_CI_AGENT_FAMILY" "$MODEL_CI" "" <<EOF
$(load_skill "$SKILL_VERIFY")

---
CI checks are failing on PR #$PR_NUMBER ($label, attempt $attempt).

Failing checks:
$CI_FAILURES

Prior fix history (avoid repeating same approach):
$(grep -A5 "CI Fix Attempt" "$NOTES_FILE" 2>/dev/null | tail -20 || echo "none")

Steps:
1. Fetch failure logs:
   gh run list --branch \$(git branch --show-current) --json databaseId,name,conclusion \
     --jq '.[] | select(.conclusion == "failure") | .databaseId' \
     | head -5 | xargs -I{} gh run view {} --log-failed 2>/dev/null | head -300
2. Analyze the root cause of each failure
3. Fix the issues — do not add new features, fix failures only
4. Run the verification loop locally to confirm fixes pass
5. Make the smallest fix that addresses the failure.
6. Stage and commit:
   git add -A && git commit -m 'fix(ci): fix CI failures [$label attempt $attempt]'
7. Push: git push

Focus strictly on CI failures. Do not change unrelated code.
EOF

    echo "  修正をプッシュしました。CI の起動を待ちます (${CI_PUSH_SETTLE_DELAY:-30}s)..."
    sleep "${CI_PUSH_SETTLE_DELAY:-30}"
  done

  echo ""
  echo "ERROR: ${CI_MAX_ATTEMPTS:-10} 回試行しましたが CI がグリーンになりませんでした ($label)。"
  echo "手動での確認が必要です: $PR_URL"
  exit 1
}

echo ""
echo "======================================================"
echo " Task: $TASK"
echo "======================================================"
print_runtime_summary

if [ 0 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [0/13] リサーチ — 既存実装・パターン調査"
run_agent_exec "$DEV_PLAN_AGENT_FAMILY" "$MODEL_PLAN" "Read,Grep,Glob,Bash" <<EOF
Task: $TASK
Read $PLAN for context.

Research phase — do NOT write any code yet.

1. Search the codebase for existing similar implementations:
   - rg through relevant modules for patterns related to this task
   - Identify reusable utilities, helpers, or abstractions already present

2. Identify applicable patterns from the plan:
   - Which design patterns are already in use in this codebase?
   - Are there skeleton implementations or templates to follow?

3. Flag potential AI regression risks (sandbox/production path parity,
   SELECT clause completeness, optimistic update rollbacks) if relevant.

Output a brief research summary (5-10 bullet points) covering:
- Relevant existing code to reuse or extend
- Patterns to follow for consistency
- Potential pitfalls specific to this task
- A narrow implementation handoff for OpenCode: exact files/functions to inspect first,
  likely tests to add/update, and commands to run

Append the summary to $NOTES_FILE under '## Research Findings'.
EOF
checkpoint 0
else
  echo ""
  echo "==> [0/13] リサーチ — スキップ (チェックポイント済み)"
fi

if [ 1 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [1/13] Eval 定義 + タスク分解"
run_agent_exec "$DEV_PLAN_AGENT_FAMILY" "$MODEL_PLAN" "" <<EOF
$(load_skill "$SKILL_EVAL")

---
Task: $TASK
Read $PLAN for context.
Read research findings in $NOTES_FILE for codebase patterns.

1. Define capability evals (what must work after implementation)
2. Define regression evals (what must NOT break)
3. Break the task into independently verifiable units (15-minute rule)
4. Run baseline: capture current test/build status
5. Write a concrete implementation handoff for OpenCode:
   - files it may edit
   - tests it must create or update first
   - acceptance checks it must run
   - what decisions are already made and should not be revisited

Output the eval definitions and task units. Do not implement yet.
EOF
checkpoint 1
else
  echo ""
  echo "==> [1/13] Eval 定義 — スキップ (チェックポイント済み)"
fi

if [ 2 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [2/13] TDD 実装"
run_agent_exec "$DEV_IMPL_AGENT_FAMILY" "$MODEL_IMPL" "" <<EOF
$(load_skill "$SKILL_TDD")

---
Task: $TASK
Read $PLAN for context.
Read $NOTES_FILE for research findings and patterns to follow.

You are the implementation executor. Treat Step 0/1 notes as the planning authority.
Do not redesign the task unless implementation is blocked; if blocked, write the blocker
clearly in $NOTES_FILE and stop.

Follow the TDD cycle strictly:
1. Define interfaces/types first
2. Write failing tests (RED) — unit + integration + edge cases
   - Include sandbox/production path parity tests if applicable (ai-regression-testing)
   - Test API response shapes explicitly for any new endpoints
3. Run tests and confirm they FAIL
4. Implement minimal code to pass (GREEN)
5. Run tests and confirm they PASS
6. Refactor while keeping tests green (REFACTOR)
7. Verify ≥80% coverage (100% for security/financial logic)

Do NOT create documentation files.
Do NOT write implementation before tests.
Keep changes surgical and limited to the task.
EOF
checkpoint 2
else
  echo ""
  echo "==> [2/13] TDD 実装 — スキップ (チェックポイント済み)"
fi

if [ 3 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [3/13] クリーンアップ"
run_agent_exec "$DEV_IMPL_AGENT_FAMILY" "$MODEL_CLEANUP" "" <<EOF
Review all files changed since the last commit (git diff HEAD).
Remove test slop:
- Tests verifying language/framework behavior (not business logic)
- Overly defensive runtime checks for impossible states
- Redundant type assertions the type system already enforces
- console.log / debug print statements
- Commented-out code

Keep all business logic tests and edge case coverage.
Run the test suite after cleanup and confirm it still passes.
Do not change architecture or add new behavior.
EOF
checkpoint 3
else
  echo ""
  echo "==> [3/13] クリーンアップ — スキップ (チェックポイント済み)"
fi

if [ 4 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [4/13] 多段検証"
run_agent_exec "$DEV_VERIFY_AGENT_FAMILY" "$MODEL_VERIFY" "" <<EOF
$(load_skill "$SKILL_VERIFY")

---
Run all verification phases and fix deterministic local failures.
Do not add new features. Fix failures only.
Output a VERIFICATION REPORT with PASS/FAIL per phase.
EOF
checkpoint 4
else
  echo ""
  echo "==> [4/13] 多段検証 — スキップ (チェックポイント済み)"
fi

if [ 5 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [5/13] E2E テスト"
run_agent_exec "$DEV_E2E_AGENT_FAMILY" "$MODEL_E2E" "" <<EOF
$(load_skill "$SKILL_E2E")

---
Task: $TASK
Read $PLAN for context.

1. Identify the critical user flows affected by this task
   (auth, key feature flows, payment/financial flows if applicable)

2. Check if E2E tests already exist for these flows (tests/e2e/):
   - If yes: run them and fix any failures before adding new tests
   - If no: create minimal Page Object Model tests for the affected flows

3. Run the E2E suite:
   npx playwright test --reporter=list 2>&1 | tail -40

4. If Playwright is not installed in this project, write the following to $E2E_REPORT_FILE
   and stop:

   ## Status
   SKIPPED
   ## Reason
   Playwright not configured in this project.

5. If all tests pass, write the following to $E2E_REPORT_FILE:

   ## Status
   PASS
   ## Flows Tested
   - <list of flows>
   ## Summary
   <pass count> passed, 0 failed.

6. If any tests fail, perform root cause analysis:
   - Collect error messages, stack traces, and screenshots/traces
   - Identify whether the failure is in app code or test code
   - Determine the specific component, function, or selector causing the failure
   Then write the following to $E2E_REPORT_FILE:

   ## Status
   FAIL
   ## Flows Tested
   - <list of flows>
   ## Summary
   <pass count> passed, <fail count> failed.
   ## Root Cause Analysis
   ### <test name>
   - **Error**: <error message>
   - **Location**: <file:line>
   - **Cause**: <root cause description>
   - **Recommended fix**: <specific fix>
   (repeat for each failing test)

   Do NOT attempt to fix app or test code when failures occur — report only.
EOF

if ! is_dry_run; then
  if [ ! -f "$E2E_REPORT_FILE" ]; then
    echo "ERROR: E2E レポートファイルが生成されませんでした: $E2E_REPORT_FILE" >&2
    exit 1
  fi

  E2E_STATUS=$(grep -i "^## Status" -A1 "$E2E_REPORT_FILE" | tail -1 | tr -d ' \n' 2>/dev/null || echo "UNKNOWN")

  if echo "$E2E_STATUS" | grep -qiE "^FAIL$"; then
    echo ""
    echo "ERROR: E2E テストが失敗しました。レポートを確認してください: $E2E_REPORT_FILE" >&2
    echo ""
    cat "$E2E_REPORT_FILE"
    exit 1
  fi

  echo "E2E ステータス: $E2E_STATUS"
fi
checkpoint 5
else
  echo ""
  echo "==> [5/13] E2E テスト — スキップ (チェックポイント済み)"
fi

if [ 6 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [6/13] セキュリティレビュー"
SECURITY_PROMPT=""
if [ -f "$SKILL_SECURITY" ]; then
  SECURITY_PROMPT="$(load_skill "$SKILL_SECURITY")

---"
fi

run_agent_exec "$DEV_REVIEW_AGENT_FAMILY" "$REVIEW_MODEL" "Read,Grep,Glob,Bash,Edit,Write" <<EOF
$SECURITY_PROMPT
Task: $TASK — Security review before commit.

Review all changes since the last commit (git diff HEAD):

MANDATORY CHECKS:
- [ ] No hardcoded secrets, API keys, or tokens
- [ ] All user inputs validated at system boundaries
- [ ] SQL injection prevention (parameterized queries only)
- [ ] XSS prevention (sanitized HTML output)
- [ ] Authentication / authorization checks in place
- [ ] No sensitive data in logs or error messages
- [ ] No command injection via string interpolation

If any CRITICAL or HIGH issue is found:
1. Fix it immediately before proceeding
2. Run the test suite to confirm the fix doesn't break anything
3. Note the finding in $NOTES_FILE under '## Security Fixes'

If clean, output: 'Security review PASSED — no critical/high issues found.'
EOF
checkpoint 6
else
  echo ""
  echo "==> [6/13] セキュリティレビュー — スキップ (チェックポイント済み)"
fi

if [ 7 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [7/13] Eval 検証"
run_agent_exec "$DEV_PLAN_AGENT_FAMILY" "$MODEL_PLAN" "Read,Grep,Glob,Bash" <<EOF
$(load_skill "$SKILL_EVAL")

---
Task: $TASK

Re-run the capability and regression evals defined in Step 1.
Report pass@k delta vs baseline.
If any eval fails: output what needs fixing and exit with code 1.
EOF
checkpoint 7
else
  echo ""
  echo "==> [7/13] Eval 検証 — スキップ (チェックポイント済み)"
fi

if [ 8 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [8/13] コミット → プッシュ → PR 作成"
run_agent_exec "$DEV_RELEASE_AGENT_FAMILY" "$MODEL_RELEASE" "" <<EOF
$(load_skill "$SKILL_SHIP")

---
Task: $TASK

This is a mechanical release step. Do not alter product code unless a command fails
and the failure is directly caused by release metadata.

1. Review git diff and git status.
2. Stage all changed files: git add -A
3. Create a conventional commit:
   - Format: type(scope): description
   - Types: feat / fix / test / refactor / chore / ci
   - Task for reference: $TASK
4. Push to remote: git push -u origin HEAD
5. Create a pull request using gh:
   gh pr create --title '...' --body '...'
   - Title: concise (under 70 chars), derived from the task
   - Body must include:
     ## Summary
     - What was implemented and why
     ## Changes
     - Key files changed
     ## Test plan
     - How to verify the changes

Output the PR URL at the end.
EOF
checkpoint 8
else
  echo ""
  echo "==> [8/13] コミット → PR — スキップ (チェックポイント済み)"
fi

# ステップ8完了またはスキップ後にPR情報を取得
if is_dry_run; then
  PR_URL="https://example.invalid/dry-run-pr"
  PR_NUMBER="0"
else
  PR_URL=$(gh pr view --json url -q .url 2>/dev/null || echo "")
  PR_NUMBER=$(gh pr view --json number -q .number 2>/dev/null || echo "")

  if [ -z "$PR_NUMBER" ]; then
    echo "ERROR: PR が見つかりません。PR 作成を確認してください。"
    exit 1
  fi

  if ! [[ "$PR_NUMBER" =~ ^[0-9]+$ ]]; then
    echo "ERROR: PR_NUMBER が数値ではありません: '$PR_NUMBER'" >&2
    exit 1
  fi
fi

echo "PR: $PR_URL"

if [ 9 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [9/13] CI 監視ループ① (PR作成後)"
wait_for_ci_green "PR作成後"
checkpoint 9
else
  echo ""
  echo "==> [9/13] CI 監視ループ① — スキップ (チェックポイント済み)"
fi

REVIEW_FILE="review-${PR_NUMBER}.md"

if [ 10 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [10/13] コードレビュー"
if is_dry_run; then
  echo "DRY RUN REVIEW [$DEV_REVIEW_AGENT_FAMILY]: $(describe_review_command "$DEV_REVIEW_AGENT_FAMILY" "$REVIEW_MODEL")"
  cat >"$REVIEW_FILE" <<EOF
## Code Review

### Summary
Dry run placeholder review.

### Findings

#### CRITICAL
- none

#### HIGH
- none

#### MEDIUM
- none

#### LOW
- none

### Verdict
APPROVE
EOF
else
  run_agent_review "$DEV_REVIEW_AGENT_FAMILY" "$REVIEW_MODEL" <<EOF | tee "$REVIEW_FILE"
$(build_codex_review_prompt "$PR_NUMBER" "$TASK" "$NOTES_FILE" "$PLAN")
EOF
fi
checkpoint 10
else
  echo ""
  echo "==> [10/13] コードレビュー — スキップ (チェックポイント済み)"
fi

if [ 11 -ge "$RESUME_FROM" ]; then
echo ""
echo "==> [11/13] レビューを PR に投稿"
run_agent_exec "$DEV_RELEASE_AGENT_FAMILY" "$MODEL_RELEASE" "" <<EOF
Read the review file at $REVIEW_FILE.

Post the review as a PR comment:
  gh pr comment $PR_NUMBER --body-file $REVIEW_FILE

Then check the Verdict line:
- If APPROVE: output 'Review passed.'
- If REQUEST_CHANGES or BLOCK: output 'Review requires changes.' and list CRITICAL and HIGH findings only.
EOF
checkpoint 11
else
  echo ""
  echo "==> [11/13] レビュー投稿 — スキップ (チェックポイント済み)"
fi

# VERDICTをファイルから取得（スキップ時も含む）
if [ -f "$REVIEW_FILE" ]; then
  VERDICT=$(grep -i "^### Verdict" -A1 "$REVIEW_FILE" | tail -1 | tr -d ' \n' 2>/dev/null || echo "UNKNOWN")
fi

if [ 12 -ge "$RESUME_FROM" ] && echo "$VERDICT" | grep -qiE "BLOCK|REQUEST_CHANGES"; then
  echo ""
  echo "==> [12/13] レビュー指摘対応 + CI 監視ループ②"

  run_agent_exec "$DEV_IMPL_AGENT_FAMILY" "$MODEL_IMPL" "" <<EOF
$(load_skill "$SKILL_VERIFY")

---
Read the code review findings at $REVIEW_FILE.

Address ALL CRITICAL and HIGH findings:
1. For each finding: read the file, understand the issue, apply the fix
2. After all fixes: run the verification loop (build, types, lint, tests)
3. Stage fixed files: git add -A
4. Create a follow-up commit:
   fix(review): address code review findings from PR #$PR_NUMBER
5. Push: git push

Then post a follow-up comment summarizing what was fixed:
  gh pr comment $PR_NUMBER --body '## Review Fixes

  Addressed the following findings:
  - [finding 1 and how it was fixed]
  - [finding 2 and how it was fixed]
  ...'
EOF

  wait_for_ci_green "レビュー対応後"
  checkpoint 12
  echo ""
  echo "指摘対応 + CI グリーン確認完了。PR をマージしてください: $PR_URL"
else
  echo ""
  echo "==> [12/13] レビュー指摘対応 — スキップ (承認済みまたはチェックポイント済み)"
  echo "レビュー承認 + CI グリーン確認済み。PR をマージしてください: $PR_URL"
fi

echo ""
echo "==> [post] 学習記録 (continuous-learning-v2)"
run_agent_exec "$DEV_RELEASE_AGENT_FAMILY" "$MODEL_RELEASE" "" <<EOF
$(load_skill "$SKILL_LEARNING")

---
Task completed: $TASK

Extract 1-2 instincts learned from this session:
- What pattern worked well and should be remembered?
- Any project-specific convention discovered?
Save each as an instinct with: trigger, action, confidence, domain, scope.
EOF

rm -f "$CHECKPOINT_FILE"
rm -f "$NOTES_FILE"
rm -f "$E2E_REPORT_FILE"

echo ""
echo "======================================================"
echo " Completed: $TASK"
echo " PR: $PR_URL"
echo "======================================================"
