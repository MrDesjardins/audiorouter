// Claude Code PostToolUse hook: format a Rust file right after an agent edits
// it, so commits never carry formatting drift (CI runs `cargo fmt --check`).
// Reads the hook payload on stdin; ignores non-Rust files and never fails the
// edit (a half-finished file that rustfmt cannot parse is left as is).
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";

let input = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => { input += chunk; });
process.stdin.on("end", () => {
  let file;
  try {
    const payload = JSON.parse(input);
    file = payload?.tool_input?.file_path ?? payload?.tool_response?.filePath;
  } catch {
    return;
  }
  if (typeof file !== "string" || !file.endsWith(".rs") || !existsSync(file)) return;
  const result = spawnSync("rustfmt", ["--edition", "2021", file], { encoding: "utf8" });
  if (result.status !== 0 && result.stderr) {
    process.stderr.write(`rustfmt could not format ${file}; run cargo fmt before committing.\n`);
  }
});
