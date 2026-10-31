import { existsSync, readFileSync } from "node:fs";
import { isAbsolute, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { familyIssues, loadFamilies } from "./tool-guide-families.mjs";

const root = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const manifestPath = resolve(root, "tools/site/tool-guide-property-manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const issues = [];
const keys = new Set();
for (const tool of manifest.tools) {
  for (const property of tool.properties) {
    const key = `${tool.kind}\0${property.key}`;
    if (keys.has(key)) issues.push(`duplicate inventory entry: ${tool.kind}.${property.key}`);
    keys.add(key);
    if (!property.recordingId) issues.push(`missing recording id: ${tool.kind}.${property.key}`);
    if (property.status === "exempted" && !property.nonAudibleReason)
      issues.push(`exemption has no rationale: ${tool.kind}.${property.key}`);
    if (property.status === "recorded" && !property.recordings?.length)
      issues.push(`recorded property has no media reference: ${tool.kind}.${property.key}`);
    for (const recording of property.recordings ?? []) {
      for (const field of ["video", "poster"]) {
        const path = recording[field];
        if (!path || isAbsolute(path)) {
          issues.push(`invalid ${field} path: ${tool.kind}.${property.key}`);
          continue;
        }
        const absolute = resolve(root, "site", path);
        const fromSite = relative(resolve(root, "site"), absolute);
        if (fromSite === ".." || fromSite.startsWith(`..${sep}`) || !existsSync(absolute))
          issues.push(`missing or out-of-site ${field}: ${tool.kind}.${property.key} -> ${path}`);
      }
    }
    if (!["pending", "recorded", "exempted"].includes(property.status))
      issues.push(`unknown coverage status ${property.status}: ${tool.kind}.${property.key}`);
  }
}
if (manifest.tools.length !== manifest.guideKindCount)
  issues.push(`manifest declares ${manifest.guideKindCount} guide kinds but contains ${manifest.tools.length}`);
if (keys.size !== manifest.propertyCount)
  issues.push(`manifest declares ${manifest.propertyCount} properties but contains ${keys.size}`);
// A repeated-control row (EQ bands 2-16) counts as recorded only through its
// representative row's clips and the family's shared clip.
issues.push(
  ...familyIssues(manifest, loadFamilies(), (path) =>
    Boolean(path) && !isAbsolute(path) && existsSync(resolve(root, "site", path)),
  ),
);
const pending = manifest.tools.flatMap((tool) =>
  tool.properties.filter((property) => property.status === "pending").map((property) => `${tool.kind}.${property.key}`),
);
if (pending.length) issues.push(`${pending.length} properties still need a recording or justified exemption (first: ${pending.slice(0, 12).join(", ")})`);
if (issues.length) {
  console.error(issues.join("\n"));
  process.exitCode = 1;
} else console.log(`Coverage complete: ${manifest.guideKindCount} guide kinds, ${keys.size} properties.`);
