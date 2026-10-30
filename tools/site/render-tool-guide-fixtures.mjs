// Prepare production-DSP audio inputs for the Properties recording pipeline.
// Failed and configuration-only tools stay explicit in the report.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const manifest = JSON.parse(
  readFileSync(
    resolve(root, "tools/site/tool-guide-property-manifest.json"),
    "utf8",
  ),
);
const output = resolve(root, "target/tool-guide-tools");
mkdirSync(output, { recursive: true });
const build = spawnSync(
  "cargo",
  [
    "build",
    "-p",
    "audiorouter-engine",
    "--example",
    "tool_guide_property_render",
  ],
  { cwd: root, stdio: "inherit" },
);
if (build.status !== 0) process.exit(build.status ?? 1);
const selected = process.env.GUIDE_KIND?.split(",");
const reportPath = resolve(output, "render-report.json");
const report = selected
  ? JSON.parse(readFileSync(reportPath, "utf8")).filter(
      (row) => !selected.includes(row.kind),
    )
  : [];
for (const tool of manifest.tools.filter(
  (tool) => !selected || selected.includes(tool.kind),
)) {
  if (tool.capture !== "production-dsp") {
    report.push({
      kind: tool.kind,
      status: "configuration-recording-required",
    });
    continue;
  }
  const directory = resolve(output, tool.kind);
  mkdirSync(directory, { recursive: true });
  const before = Object.fromEntries(
    tool.properties
      .filter(
        (p) =>
          p.default !== null &&
          !p.dynamic &&
          !["file", "selection"].includes(p.type),
      )
      .map((p) => [p.key, p.default]),
  );
  if (tool.kind === "audioFile") before.mediaId = "lib:guide-fixture";
  if (tool.kind === "soundboard") before.slot1MediaId = "lib:guide-fixture";
  if (tool.kind === "parametricEq") {
    before.band0Enabled = true;
    before.band0GainDb = 9;
  }
  if (tool.kind === "testSignal") before.durationMs = 8000;
  if (tool.kind === "firFilter") before.mediaId = "lib:guide-impulse";
  if (tool.kind === "duck") before.keyNodeId = "input";
  let candidate =
    tool.properties.find(
      (p) =>
        !p.dynamic &&
        typeof p.default === "number" &&
        typeof p.minimum === "number" &&
        typeof p.maximum === "number",
    ) ?? tool.properties.find((p) => p.type === "boolean");
  if (tool.kind === "mixer") {
    before["inputVolume:input"] = 1;
    candidate = {
      key: "inputVolume:input",
      default: 1,
      minimum: 0,
      maximum: 1,
    };
  }
  if (["inputSwitch", "outputSwitch"].includes(tool.kind))
    candidate = { key: "selected", type: "string", default: "a" };
  const after = { ...before };
  if (candidate)
    after[candidate.key] =
      candidate.type === "string"
        ? "b"
        : candidate.type === "boolean"
          ? !candidate.default
          : candidate.default === candidate.minimum
            ? candidate.maximum
            : candidate.minimum;
  if (tool.kind === "testSignal") after.frequencyHz = 880;
  const beforePath = resolve(directory, "before.json");
  const afterPath = resolve(directory, "after.json");
  writeFileSync(beforePath, JSON.stringify(before));
  writeFileSync(afterPath, JSON.stringify(after));
  const fixture = ["denoise", "spectralGate", "speechDenoise"].includes(
    tool.kind,
  )
    ? "noisy-speech"
    : tool.fixture;
  const result = spawnSync(
    resolve(root, "target/debug/examples/tool_guide_property_render.exe"),
    [directory, tool.kind, fixture, beforePath, afterPath],
    { cwd: root, encoding: "utf8" },
  );
  report.push({
    kind: tool.kind,
    status: result.status === 0 ? "audio-rendered-unreviewed" : "render-failed",
    parameter: candidate?.key ?? null,
    error: result.status === 0 ? null : result.stderr,
  });
  console.log(`${tool.kind}: ${report.at(-1).status}`);
}
writeFileSync(
  resolve(output, "render-report.json"),
  JSON.stringify(report, null, 2),
);
if (report.some((row) => row.status === "render-failed")) process.exitCode = 1;
