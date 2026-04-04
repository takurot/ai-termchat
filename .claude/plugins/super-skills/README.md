# Claude Adapter

Status: implemented opt-in adapter.

Claude support is now grounded around a real target layout:

- shared authored skills in `skills/`
- generated Claude-ready skills in `.claude/skills/`
- Claude host guidance template in `plugins/claude/templates/AGENTS.md`

This adapter directory remains intentionally thin and exists for host-specific notes and reminder-only hook templates.

## Scope

- host-facing notes only
- opt-in setup guidance only
- no hidden behavior
- shared skills, not forked skills
- reminder-only hooks

## Constraints

- No auto-approval
- No telemetry
- No background execution
- No core workflow logic

## Install Shape

The installer should treat Claude support as:

- `full` for `.claude/skills`
- `full` for `.claude/AGENTS.md`, generated from `plugins/claude/templates/AGENTS.md`
- `full` for `.claude/plugins/super-skills` when `plugin:claude` is selected
- opt-in for hook activation

## Included Files

- `INSTALL.md`
- `hooks/super-skills.hooks.json`
- `hooks/README.md`
- `scripts/hooks/*.js`
