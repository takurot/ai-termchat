#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEV_WORKFLOW_SCRIPT="$SCRIPT_DIR/dev-workflow.sh"
TEST_ROOT=""

cleanup() {
  if [ -n "$TEST_ROOT" ] && [ -d "$TEST_ROOT" ]; then
    rm -rf "$TEST_ROOT"
  fi
}

trap cleanup EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_contains() {
  local needle="$1"
  local haystack_file="$2"

  if ! grep -Fq -- "$needle" "$haystack_file"; then
    echo "Expected to find: $needle" >&2
    echo "--- output ---" >&2
    cat "$haystack_file" >&2
    fail "missing expected output"
  fi
}

setup_fixture() {
  TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/dev-workflow-test.XXXXXX")"
  mkdir -p "$TEST_ROOT/bin" "$TEST_ROOT/docs" \
    "$TEST_ROOT/.agents/skills/tdd-workflow" \
    "$TEST_ROOT/.agents/skills/verification-loop" \
    "$TEST_ROOT/.agents/skills/e2e-testing"

  printf '# Plan\n' >"$TEST_ROOT/docs/PLAN.md"
  printf 'tdd skill\n' >"$TEST_ROOT/.agents/skills/tdd-workflow/SKILL.md"
  printf 'verify skill\n' >"$TEST_ROOT/.agents/skills/verification-loop/SKILL.md"
  printf 'e2e skill\n' >"$TEST_ROOT/.agents/skills/e2e-testing/SKILL.md"

  cat >"$TEST_ROOT/bin/gh" <<'EOF'
#!/bin/sh
case "$*" in
  *"pr view"*"number"*) echo "1" ;;
  *"pr view"*"url"*) echo "https://example.test/pr/1" ;;
  *"pr checks"*) echo "[]" ;;
  *) echo "{}" ;;
esac
EOF
  chmod +x "$TEST_ROOT/bin/gh"

  (
    cd "$TEST_ROOT"
    git init -q
    git config user.email test@example.com
    git config user.name "Dev Workflow Test"
    git add .
    git commit -qm init
  )
}

run_in_fixture() {
  local output_file="$1"
  shift

  (
    cd "$TEST_ROOT"
    PATH="$TEST_ROOT/bin:$PATH" "$@" >"$output_file" 2>&1
  )
}

test_dry_run_defaults_to_codex() {
  setup_fixture
  local output
  output="$(mktemp "${TMPDIR:-/tmp}/dev-workflow-output.XXXXXX")"

  run_in_fixture "$output" env DEV_DRY_RUN=1 bash "$DEV_WORKFLOW_SCRIPT" docs/PLAN.md "dry run"

  assert_contains "Implementation agent: codex" "$output"
  assert_contains "Release agent: codex" "$output"
  assert_contains "Dry run: enabled" "$output"
}

test_no_diff_implementation_is_not_checkpointed() {
  setup_fixture
  local output
  output="$(mktemp "${TMPDIR:-/tmp}/dev-workflow-output.XXXXXX")"
  printf '1\n' >"$TEST_ROOT/.dev-task-checkpoint"
  printf '# Notes\n' >"$TEST_ROOT/.dev-task-notes.md"

  cat >"$TEST_ROOT/bin/codex" <<'EOF'
#!/bin/sh
cat >/dev/null
echo "fake agent completed without edits"
EOF
  chmod +x "$TEST_ROOT/bin/codex"
  (cd "$TEST_ROOT" && git add bin/codex && git commit -qm "test fake codex")

  if run_in_fixture "$output" env DEV_AGENT_TIMEOUT_SECS=5 bash "$DEV_WORKFLOW_SCRIPT" docs/PLAN.md "no diff"; then
    fail "workflow succeeded despite no implementation changes"
  fi

  assert_contains "implementation step completed with no task file changes" "$output"
  [ "$(cat "$TEST_ROOT/.dev-task-checkpoint")" = "1" ] || fail "checkpoint advanced after no-op implementation"
  assert_contains "implementation	no_op" "$TEST_ROOT/.dev-workflow/status.tsv"
}

test_permission_rejection_is_not_checkpointed() {
  setup_fixture
  local output
  output="$(mktemp "${TMPDIR:-/tmp}/dev-workflow-output.XXXXXX")"
  printf '1\n' >"$TEST_ROOT/.dev-task-checkpoint"
  printf '# Notes\n' >"$TEST_ROOT/.dev-task-notes.md"

  cat >"$TEST_ROOT/bin/codex" <<'EOF'
#!/bin/sh
cat >/dev/null
echo "permission request for bash command"
echo "auto-rejecting permission prompt"
EOF
  chmod +x "$TEST_ROOT/bin/codex"
  (cd "$TEST_ROOT" && git add bin/codex && git commit -qm "test fake codex")

  if run_in_fixture "$output" env DEV_AGENT_TIMEOUT_SECS=5 bash "$DEV_WORKFLOW_SCRIPT" docs/PLAN.md "permission rejection"; then
    fail "workflow succeeded despite permission rejection"
  fi

  assert_contains "agent permission prompt/rejection detected" "$output"
  [ "$(cat "$TEST_ROOT/.dev-task-checkpoint")" = "1" ] || fail "checkpoint advanced after permission rejection"
  assert_contains "permission_rejected" "$TEST_ROOT/.dev-workflow/status.tsv"
}

test_agent_timeout_is_not_checkpointed() {
  setup_fixture
  local output
  output="$(mktemp "${TMPDIR:-/tmp}/dev-workflow-output.XXXXXX")"
  printf '1\n' >"$TEST_ROOT/.dev-task-checkpoint"
  printf '# Notes\n' >"$TEST_ROOT/.dev-task-notes.md"

  cat >"$TEST_ROOT/bin/codex" <<'EOF'
#!/bin/sh
cat >/dev/null
sleep 5
EOF
  chmod +x "$TEST_ROOT/bin/codex"
  (cd "$TEST_ROOT" && git add bin/codex && git commit -qm "test fake codex")

  if run_in_fixture "$output" env DEV_AGENT_TIMEOUT_SECS=1 bash "$DEV_WORKFLOW_SCRIPT" docs/PLAN.md "timeout"; then
    fail "workflow succeeded despite agent timeout"
  fi

  assert_contains "agent command timed out after 1s" "$output"
  [ "$(cat "$TEST_ROOT/.dev-task-checkpoint")" = "1" ] || fail "checkpoint advanced after timeout"
  assert_contains "timeout" "$TEST_ROOT/.dev-workflow/status.tsv"
}

test_opencode_allow_mode_is_wired() {
  setup_fixture
  local output
  output="$(mktemp "${TMPDIR:-/tmp}/dev-workflow-output.XXXXXX")"
  printf '1\n' >"$TEST_ROOT/.dev-task-checkpoint"

  run_in_fixture "$output" env DEV_DRY_RUN=1 DEV_IMPL_AGENT_FAMILY=opencode DEV_OPENCODE_PERMISSION_MODE=allow bash "$DEV_WORKFLOW_SCRIPT" docs/PLAN.md "opencode allow"

  assert_contains "OpenCode permission mode: allow" "$output"
  assert_contains "--dangerously-skip-permissions" "$output"
}

test_dry_run_defaults_to_codex
cleanup
TEST_ROOT=""

test_no_diff_implementation_is_not_checkpointed
cleanup
TEST_ROOT=""

test_permission_rejection_is_not_checkpointed
cleanup
TEST_ROOT=""

test_agent_timeout_is_not_checkpointed
cleanup
TEST_ROOT=""

test_opencode_allow_mode_is_wired
cleanup
TEST_ROOT=""

echo "dev-workflow tests passed"
