#!/usr/bin/env node

function readStdin() {
  return new Promise((resolve, reject) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => {
      data += chunk;
    });
    process.stdin.on("end", () => resolve(data));
    process.stdin.on("error", reject);
  });
}

function warn(message) {
  process.stderr.write(`[Super Skills Claude Adapter] ${message}\n`);
}

async function run(handler) {
  const raw = await readStdin();
  let input = {};
  if (raw.trim()) {
    try {
      input = JSON.parse(raw);
    } catch (error) {
      warn(`Failed to parse hook input JSON: ${error.message}`);
      process.stdout.write(raw);
      process.exit(0);
    }
  }

  try {
    const message = handler(input);
    if (message) {
      warn(message);
    }
  } catch (error) {
    warn(`Hook failed safely: ${error.message}`);
  }

  process.stdout.write(raw);
}

function editedPath(input) {
  return (
    input?.tool_input?.file_path ||
    input?.tool_input?.path ||
    input?.tool_input?.target_file ||
    ""
  );
}

function bashCommand(input) {
  return input?.tool_input?.command || "";
}

module.exports = {
  bashCommand,
  editedPath,
  run,
};
