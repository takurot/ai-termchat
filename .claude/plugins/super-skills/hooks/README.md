# Claude Hook Template

This directory contains a safe, reminder-only Claude hooks template for the Super Skills adapter.

## Design Rules

- opt-in only
- no auto-approval
- no telemetry
- no hidden execution
- no mutation of git state
- visible fallback commands for every reminder

## Included Reminders

- formatting reminder after edits
- TypeScript typecheck reminder after TS edits
- config validation reminder after manifest or MCP edits
- MCP health reminder before unsafe-local workflows
- checkpoint reminder before long-running commands
- compact reminder before context compaction

## Activation

Review `super-skills.hooks.json` and merge only the entries you want into your Claude hooks configuration.
