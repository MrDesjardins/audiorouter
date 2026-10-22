import { spawnSync } from "node:child_process";
import process from "node:process";
import { log } from "node:console";
import { mkdtempSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ui = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const output = mkdtempSync(path.join(process.env.RUNNER_TEMP ?? os.tmpdir(), "audiorouter-browser-guard-"));
const fixture = path.join(ui, "e2e/browser-test.ts").replaceAll("\\", "/");
writeFileSync(path.join(output, "guard.pw.ts"), `import { test } from ${JSON.stringify(fixture)};
test("browser error guard qualification", async ({ page }) => {
  await page.goto("data:text/html,<title>Isolated guard qualification</title>");
  if (process.env.AUDIOROUTER_GUARD_PROBE === "negative") {
    const observed = page.waitForEvent("pageerror");
    await page.evaluate(() => { setTimeout(() => { throw Error("intentional-browser-guard-probe"); }, 0); });
    await observed;
  }
});
`);
const results = [];
for (const probe of ["positive", "negative"]) {
  const config = path.join(output, `${probe}.config.mjs`);
  writeFileSync(config, `export default ${JSON.stringify({ testDir: output, testMatch: "guard.pw.ts", workers: 1, retries: 0, reporter: "line", outputDir: path.join(output, probe), use: { browserName: "chromium", channel: "msedge" } })};\n`);
  const child = spawnSync(process.execPath, [path.join(ui, "node_modules/@playwright/test/cli.js"), "test", "--config", config], {
    cwd: ui, encoding: "utf8", timeout: 60_000, maxBuffer: 2 * 1024 * 1024,
    env: { ...process.env, AUDIOROUTER_GUARD_PROBE: probe }, windowsHide: true, shell: false,
  });
  writeFileSync(path.join(output, `${probe}.log`), `${child.stdout ?? ""}\n${child.stderr ?? ""}`);
  const detected = probe === "positive" ? child.status === 0 : child.status === 1 && child.stdout?.includes("No uncaught UI exceptions") && child.stdout?.includes("intentional-browser-guard-probe");
  results.push({ probe, childExitCode: child.status, signal: child.signal, qualified: Boolean(detected) });
}
writeFileSync(path.join(output, "qualification.json"), `${JSON.stringify(results, null, 2)}\n`);
log(`Browser guard qualification: ${JSON.stringify(results)}; evidence: ${output}`);
if (results.some(result => !result.qualified)) process.exitCode = 1;
