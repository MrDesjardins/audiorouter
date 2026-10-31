import assert from "node:assert/strict";
import { readFileSync, statSync, writeFileSync } from "node:fs";
import { resolve, dirname, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const tools = JSON.parse(
  readFileSync(
    resolve(root, "tools/site/tool-guide-property-manifest.json"),
    "utf8",
  ),
).tools;
const clips = JSON.parse(
  readFileSync(resolve(root, "site/guide-content/recordings.json"), "utf8"),
);
assert.deepEqual(Object.keys(clips).sort(), tools.map((t) => t.kind).sort());
const report = [];
for (const tool of tools) {
  assert(clips[tool.kind].length >= 1, `${tool.kind}: missing recording`);
  if (tool.capture === "production-dsp")
    assert(
      clips[tool.kind].some((c) => c.video.endsWith("-demo.mp4")),
      `${tool.kind}: missing audio demonstration`,
    );
  for (const clip of clips[tool.kind]) {
    assert(
      clip.title && clip.description.length > 40,
      `${tool.kind}: missing accessible explanation`,
    );
    for (const field of ["video", "poster"])
      assert(
        resolve(root, "site", clip[field]).startsWith(
          resolve(root, "site/assets/tool-guides") + sep,
        ),
        `${tool.kind}: invalid asset path`,
      );
    const path = resolve(root, "site", clip.video);
    const bytes = statSync(path).size;
    statSync(resolve(root, "site", clip.poster));
    assert(bytes < 1_000_000, `${clip.video}: exceeds 1 MB budget`);
    const result = spawnSync(
      "ffprobe",
      [
        "-v",
        "error",
        "-show_entries",
        "format=duration:stream=codec_name,codec_type,width,height",
        "-of",
        "json",
        path,
      ],
      { encoding: "utf8" },
    );
    assert.equal(result.status, 0, result.stderr);
    const metadata = JSON.parse(result.stdout);
    const video = metadata.streams.find((s) => s.codec_type === "video");
    assert(
      video.codec_name === "h264" &&
        video.width === 720 &&
        video.height === 1280,
      `${clip.video}: invalid video format`,
    );
    assert(
      Math.abs(Number(metadata.format.duration) - 8) < 0.1,
      `${clip.video}: invalid duration`,
    );
    const audio = metadata.streams.find((s) => s.codec_type === "audio");
    if (clip.silent !== true && (clip.video.endsWith("-demo.mp4") || clip.video.includes("--")))
      assert.equal(
        audio?.codec_name,
        "aac",
        `${clip.video}: missing AAC audio`,
      );
    else assert(!audio, `${clip.video}: configuration overview must be silent`);
    report.push({
      kind: tool.kind,
      video: clip.video,
      bytes,
      sha256: createHash("sha256").update(readFileSync(path)).digest("hex"),
      ...metadata,
    });
  }
}
writeFileSync(
  resolve(root, "target/tool-guide-media-report.json"),
  JSON.stringify(report, null, 2),
);
console.log(
  `PASS: ${tools.length} tools, ${report.length} videos, ${report.reduce((sum, c) => sum + c.bytes, 0)} video bytes; all clips fit the format, duration and size budgets.`,
);
