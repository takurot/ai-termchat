#!/usr/bin/env node

const { editedPath, run } = require("./common.js");

run((input) => {
  const filePath = editedPath(input);
  if (!/\.(ts|tsx)$/.test(filePath)) {
    return null;
  }

  return "TypeScript file edited. Run your typecheck command before shipping the change.";
});
