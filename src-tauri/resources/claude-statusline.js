#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function recodeDir() {
  return path.join(os.homedir(), ".recode");
}

function appendLine(file, line) {
  const dir = path.dirname(file);
  fs.mkdirSync(dir, { recursive: true });
  try {
    const st = fs.statSync(file);
    if (st.size > 5 * 1024 * 1024) {
      fs.writeFileSync(file, "");
    }
  } catch {
    // file may not exist yet
  }
  fs.appendFileSync(file, line + "\n");
}

let data = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (c) => {
  data += c;
});
process.stdin.on("end", () => {
  let j = null;
  try {
    j = JSON.parse(data);
    const out = {
      t: Date.now(),
      rate_limits: j.rate_limits,
      context_window: j.context_window,
      model: j.model,
      session_id: j.session_id,
    };
    appendLine(path.join(recodeDir(), "claude-status.jsonl"), JSON.stringify(out));
  } catch {
    // ignore parse errors
  }
  const p = j && j.rate_limits && j.rate_limits.five_hour
    ? j.rate_limits.five_hour.used_percentage
    : null;
  if (p != null && Number.isFinite(Number(p))) {
    process.stdout.write(`5h ${Number(p).toFixed(0)}%`);
  }
});
