import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { loadGuideRecordings } from "./guide-recordings.mjs";

test("every backend tool keeps its own clips and gets one labelled shared-flags example", () => {
  const source = readFileSync(
    new URL("../crates/domain/src/lib.rs", import.meta.url),
    "utf8",
  );
  const all = source.match(/pub const ALL: \[Self; \d+\] = \[(.*?)\];/s)?.[1];
  assert(all, "domain kind inventory must exist");
  const kinds = [...all.matchAll(/Self::([A-Za-z0-9]+)/g)].map(
    (match) => match[1][0].toLowerCase() + match[1].slice(1),
  );
  const original = JSON.parse(
    readFileSync(
      new URL("./guide-content/recordings.json", import.meta.url),
      "utf8",
    ),
  );
  const recordings = loadGuideRecordings();
  for (const kind of kinds) {
    assert(recordings[kind], `missing guide: ${kind}`);
    assert.deepEqual(
      recordings[kind].slice(0, original[kind].length),
      original[kind],
      `${kind}: original effect clips changed`,
    );
    const shared = recordings[kind].filter((clip) => clip.sharedControls);
    assert.equal(shared.length, 1, `${kind}: needs one shared example`);
    assert.equal(shared[0].silent, true);
    assert.match(shared[0].title, /Gain example.*silent/);
    assert.deepEqual(shared[0].sharedControls, ["enabled", "bypass"]);
  }
  assert.deepEqual(
    recordings.group,
    original.group,
    "annotations have no audio flags",
  );
});
