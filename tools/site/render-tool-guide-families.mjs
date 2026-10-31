// Render the shared clip of each repeated-control family (see
// tool-guide-families.mjs) with the production engine. It adds or replaces
// only the family's own rows in render-report.json, so the per-property
// renders of the same tool stay listed. GUIDE_KIND=<kind>[,<kind>] limits it.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { selectedKinds } from "./tool-guide-guards.mjs";
import { loadFamilies } from "./tool-guide-families.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const inventory = JSON.parse(
  readFileSync(resolve(root, "tools/site/tool-guide-property-manifest.json"), "utf8"),
);
const families = loadFamilies();
const kinds =
  selectedKinds(process.env.GUIDE_KIND, Object.keys(families)) ?? Object.keys(families);
const executable = resolve(root, "target/debug/examples/tool_guide_property_render.exe");
const output = resolve(root, "target/tool-guide-properties");
const reportPath = resolve(output, "render-report.json");
mkdirSync(output, { recursive: true });
let report = JSON.parse(readFileSync(reportPath, "utf8"));

for (const kind of kinds) {
  const { clip } = families[kind];
  const tool = inventory.tools.find((candidate) => candidate.kind === kind);
  if (!tool) throw new Error(`Family ${kind} is not in the property manifest`);
  const directory = resolve(output, clip.id);
  mkdirSync(directory, { recursive: true });
  const before = {
    ...JSON.parse(readFileSync(resolve(root, "target/tool-guide-tools", kind, "before.json"), "utf8")),
    ...clip.before,
  };
  const after = { ...before, ...clip.after };
  // Every changed key must be a real row, so a typo cannot render a no-op.
  for (const key of Object.keys(clip.after))
    if (!(key in before) || !tool.properties.some((row) => row.key === key))
      throw new Error(`${clip.id} changes unknown parameter ${key}`);
  if (JSON.stringify(before) === JSON.stringify(after))
    throw new Error(`${clip.id} does not change anything`);
  writeFileSync(resolve(directory, "before.json"), JSON.stringify(before));
  writeFileSync(resolve(directory, "after.json"), JSON.stringify(after));
  const result = spawnSync(
    executable,
    [
      directory,
      kind,
      clip.fixture ?? tool.fixture,
      resolve(directory, "before.json"),
      resolve(directory, "after.json"),
    ],
    { encoding: "utf8" },
  );
  report = report.filter((row) => row.id !== clip.id);
  report.push({
    id: clip.id,
    kind,
    parameter: clip.property,
    family: true,
    directory,
    audible: clip.audible !== false,
    status: result.status === 0 ? "audio-rendered-unreviewed" : "render-failed",
    error: result.status === 0 ? null : result.stderr,
  });
  writeFileSync(reportPath, JSON.stringify(report, null, 2));
  console.log(`${clip.id}: ${report.at(-1).status}`);
}
if (report.some((row) => row.family && row.status === "render-failed")) process.exitCode = 1;
