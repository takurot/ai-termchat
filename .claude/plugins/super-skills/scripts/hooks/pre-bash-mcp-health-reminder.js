#!/usr/bin/env node

const { bashCommand, run } = require("./common.js");

run((input) => {
  const command = bashCommand(input);
  const mentionsUnsafeProfile =
    /install-(plan|apply|validate)\.mjs/.test(command) &&
    (/\bunsafe-local\b/.test(command) || /\bmcp:unsafe-local\b/.test(command));

  if (!mentionsUnsafeProfile) {
    return null;
  }

  return "Unsafe-local MCP workflow detected. Verify localhost targets, secrets, and MCP health before continuing.";
});
