#!/usr/bin/env node

const { editedPath, run } = require("./common.js");

run((input) => {
  const filePath = editedPath(input);
  if (!filePath) {
    return null;
  }

  if (!/^(manifests\/|mcp\/|\.codex\/|package\.json$|plugins\/)/.test(filePath)) {
    return null;
  }

  return "Config-affecting file changed. Run `npm run validate:configs` or `npm run check` before finishing.";
});
