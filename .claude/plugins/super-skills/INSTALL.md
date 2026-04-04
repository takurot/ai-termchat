# Install Claude Adapter

This adapter is intentionally opt-in.

## What It Installs

When included through the installer, the Claude adapter is copied into:

```text
.claude/plugins/super-skills/
```

The bundle includes:

- reminder-only hook scripts
- a hook configuration template
- host-specific adapter docs

It does not auto-enable hooks.

## How To Enable

1. Install the adapter with `--with plugin:claude --target claude`
2. Review `.claude/plugins/super-skills/hooks/super-skills.hooks.json`
3. Merge the desired hook entries into your Claude hooks config manually

Suggested commands:

```sh
node scripts/install-plan.mjs --profile core --target claude --with plugin:claude
node scripts/install-apply.mjs --profile core --target claude --with plugin:claude --target-root <target-root>
```

## Safety Model

- Hooks are reminder-only
- Hooks do not auto-approve actions
- Hooks do not send telemetry
- Hooks do not rewrite git state
- Hooks do not auto-run broad command suites
