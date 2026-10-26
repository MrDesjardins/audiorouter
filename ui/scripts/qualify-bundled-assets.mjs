import { spawnSync } from "node:child_process";
import process from "node:process";
import { log } from "node:console";
import { mkdtempSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ui = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const output = mkdtempSync(path.join(process.env.RUNNER_TEMP ?? os.tmpdir(), "audiorouter-bundled-asset-guard-"));
const fixture = path.join(ui, "e2e/browser-test.ts").replaceAll("\\", "/");
writeFileSync(
  path.join(output, "assets.pw.ts"),
  `import { test, expect } from ${JSON.stringify(fixture)};
test("compiled fixture delivery with network unavailable", async ({ page, bundledAssetRequests }) => {
  if (process.env.AUDIOROUTER_ASSET_PROBE === "negative") {
    const response = await page.goto("/assets/guard-intentionally-missing.js");
    expect(response?.status()).toBe(404);
  } else {
    await page.goto("/harness.html");
    await expect(page.getByLabel("Signal-flow graph")).toBeVisible();
    await expect(page.locator(".react-flow__node")).toHaveCount(3);
    expect(bundledAssetRequests.length).toBeGreaterThan(4);
  }
});
`,
);
const results = [];
for (const probe of ["positive", "negative"]) {
  const config = path.join(output, `${probe}.config.mjs`);
  writeFileSync(
    config,
    `export default ${JSON.stringify({ testDir: output, testMatch: "assets.pw.ts", workers: 1, retries: 0, reporter: "line", expect: { timeout: process.env.CI ? 10000 : 5000 }, outputDir: path.join(output, probe), use: { baseURL: "http://127.0.0.1:4186", bundledAssets: path.resolve(ui, "../target/browser-acceptance-ui"), offline: true, browserName: "chromium", channel: "msedge", viewport: { width: 1440, height: 1000 } } })};\n`,
  );
  const child = spawnSync(
    process.execPath,
    [path.join(ui, "node_modules/@playwright/test/cli.js"), "test", "--config", config],
    {
      cwd: ui,
      encoding: "utf8",
      timeout: 60_000,
      maxBuffer: 2 * 1024 * 1024,
      env: { ...process.env, AUDIOROUTER_ASSET_PROBE: probe },
      windowsHide: true,
      shell: false,
    },
  );
  writeFileSync(path.join(output, `${probe}.log`), `${child.stdout ?? ""}\n${child.stderr ?? ""}`);
  const qualified =
    probe === "positive"
      ? child.status === 0
      : child.status === 1 &&
        child.stdout?.includes("No missing bundled fixture assets") &&
        child.stdout?.includes("guard-intentionally-missing.js");
  results.push({ probe, childExitCode: child.status, signal: child.signal, qualified: Boolean(qualified) });
}
writeFileSync(path.join(output, "qualification.json"), `${JSON.stringify(results, null, 2)}\n`);
log(`Bundled asset guard qualification: ${JSON.stringify(results)}; evidence: ${output}`);
if (results.some((result) => !result.qualified)) process.exitCode = 1;
