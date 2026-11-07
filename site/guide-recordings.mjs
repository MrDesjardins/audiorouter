import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/** One source for published clips, including the explicitly labelled shared example. */
export function loadGuideRecordings(
  site = dirname(fileURLToPath(import.meta.url)),
) {
  const read = (name) =>
    JSON.parse(readFileSync(resolve(site, "guide-content", name), "utf8"));
  const recordings = read("recordings.json");
  const shared = read("shared-controls.json").recordings;
  const tools = [
    ...read("processing.json").tools,
    ...read("routing.json").tools,
  ];
  for (const tool of tools) {
    // Group is a presentation annotation, with no Enabled or Bypass audio flags.
    if (tool.kind === "group") continue;
    const clips = recordings[tool.kind] ?? [];
    recordings[tool.kind] = [
      ...clips,
      ...shared.filter(
        (clip) => !clips.some((prior) => prior.video === clip.video),
      ),
    ];
  }
  return recordings;
}
