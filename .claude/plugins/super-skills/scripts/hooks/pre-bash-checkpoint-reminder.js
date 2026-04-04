#!/usr/bin/env node

const { bashCommand, run } = require("./common.js");

const LONG_RUNNING =
  /\b(npm|pnpm|yarn|bun)\s+(run\s+)?(dev|build|test|typecheck|lint)\b|\b(pytest|go test|cargo test|docker compose up|docker-compose up)\b/i;

run((input) => {
  const command = bashCommand(input);
  if (!LONG_RUNNING.test(command)) {
    return null;
  }
  return "Long-running workflow detected. Consider creating a checkpoint first if this work changes multiple files.";
});
