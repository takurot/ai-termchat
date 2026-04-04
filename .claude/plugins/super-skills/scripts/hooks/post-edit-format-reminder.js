#!/usr/bin/env node

const { editedPath, run } = require("./common.js");

run((input) => {
  const filePath = editedPath(input);
  if (!/\.(js|jsx|ts|tsx|css|scss|json|md)$/.test(filePath)) {
    return null;
  }

  return "Edited file may need formatting. Run your formatter or `npm run check` if formatting is project-managed.";
});
