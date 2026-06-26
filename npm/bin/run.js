#!/usr/bin/env node
const { execFileSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const ext = process.platform === "win32" ? ".exe" : "";
const binary = path.join(__dirname, `nimvault-mcp${ext}`);

if (!fs.existsSync(binary)) {
  console.error(
    "nimvault-mcp binary not found. Try reinstalling: npm install @haozeke/nimvault-mcp"
  );
  process.exit(1);
}

try {
  execFileSync(binary, process.argv.slice(2), { stdio: "inherit" });
} catch (e) {
  if (e.status !== null && e.status !== undefined) process.exit(e.status);
  throw e;
}
