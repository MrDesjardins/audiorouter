// Repeated-control families for the tool guides: Advanced EQ bands 2-16 repeat
// the controls of band 1, and Soundboard buttons 2-9 repeat those of button 1.
// A member row is covered by its representative row's clips plus one shared
// clip that shows the family at work, instead of one near-identical video per
// member. tool-guide-families.json declares each family.
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

export function loadFamilies(
  path = resolve(here, "tool-guide-families.json"),
) {
  return JSON.parse(readFileSync(path, "utf8"));
}

// The representative row's key for a member key, or null when `key` is not a
// family member (a different control, or the representative itself).
export function representativeKey(family, key) {
  if (!family) return null;
  const match = new RegExp(family.memberPattern).exec(key);
  if (!match?.groups) return null;
  if (Number(match.groups.index) === family.representativeIndex) return null;
  return family.keyTemplate
    .replaceAll("{index}", String(family.representativeIndex))
    .replaceAll("{field}", match.groups.field);
}

// Whether a recording is the family's shared clip.
export const isFamilyClip = (family, recording) =>
  recording?.property === family.clip.property;

// Problems with the family rows of one manifest. A member row may be
// "recorded" only when the representative row is recorded, and the member
// lists the shared clip and every clip of the representative row. `exists`
// answers whether a site-relative media path is on disk.
export function familyIssues(manifest, families, exists) {
  const issues = [];
  for (const tool of manifest.tools) {
    const family = families[tool.kind];
    if (!family) continue;
    const rows = new Map(tool.properties.map((row) => [row.key, row]));
    for (const row of tool.properties) {
      const name = `${tool.kind}.${row.key}`;
      const expected = representativeKey(family, row.key);
      if (row.representativeKey !== (expected ?? undefined))
        issues.push(
          `${name} should name representative ${expected ?? "(none)"}, not ${row.representativeKey ?? "(none)"}; rerun build-tool-guide-property-manifest.mjs`,
        );
      if (!expected || row.status !== "recorded") continue;
      const representative = rows.get(expected);
      if (!representative) {
        issues.push(`${name} names a missing representative row ${expected}`);
        continue;
      }
      if (representative.status !== "recorded") {
        issues.push(
          `${name} is recorded but its representative ${expected} is ${representative.status}`,
        );
        continue;
      }
      const shared = row.recordings.find((clip) => isFamilyClip(family, clip));
      if (!shared) {
        issues.push(`${name} is recorded without the "${family.clip.title}" clip`);
        continue;
      }
      for (const field of ["video", "poster"])
        if (!exists(shared[field]))
          issues.push(`${name}: the "${family.clip.title}" ${field} is missing (${shared[field]})`);
      const have = new Set(row.recordings.map((clip) => clip.video));
      for (const clip of representative.recordings)
        if (!have.has(clip.video))
          issues.push(`${name} does not list its representative clip ${clip.video} (${expected})`);
    }
  }
  return issues;
}
