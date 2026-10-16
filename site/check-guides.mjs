import assert from "node:assert/strict";
import { readFile, access } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const site = dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(
  await readFile(resolve(site, "assets/guide-manifest.json"), "utf8"),
);
const data = [];
for (const name of ["processing", "routing"])
  data.push(
    ...JSON.parse(
      await readFile(resolve(site, "guide-content", name + ".json"), "utf8"),
    ).tools,
  );
assert.equal(manifest.guides.length, data.length);
const tutorials = JSON.parse(
  await readFile(resolve(site, "guide-content/tutorials.json"), "utf8"),
);
assert.deepEqual(Object.keys(tutorials).sort(), data.map((t) => t.kind).sort());
assert.equal(
  new Set(Object.values(tutorials).map((t) => t.listen)).size,
  data.length,
  "Each tool needs its own example explanation",
);
let links = 0,
  properties = 0;
for (const guide of manifest.guides) {
  const source = data.find((t) => t.kind === guide.kind);
  assert(source);
  const tutorial = tutorials[guide.kind];
  assert(tutorial.steps.length >= 3);
  for (const key of ["success", "check", "listen"])
    assert(tutorial[key].length > 50, guide.kind + " incomplete " + key);
  const file = resolve(site, guide.path),
    html = await readFile(file, "utf8");
  assert.equal(guide.propertyCount, source.properties.length);
  assert(
    html.includes("data-guide-demo"),
    guide.kind + " missing illustration",
  );
  assert(
    html.includes("Use it in AudioRouter") &&
      html.includes(
        tutorial.success
          .replace(/&/g, "&amp;")
          .replace(/</g, "&lt;")
          .replace(/>/g, "&gt;")
          .replace(/"/g, "&quot;")
          .replace(/'/g, "&#39;"),
      ),
    guide.kind + " missing tutorial",
  );
  if (guide.audioRelevant)
    assert(
      html.includes("guide-signal.js") && html.includes("guide-playback.js"),
    );
  assert(
    !html.includes("[object Object]"),
    guide.kind + " contains an unrendered object",
  );
  const ids = new Set([...html.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1]));
  for (const property of source.properties) {
    const id =
      "property-" +
      property.key
        .replace(/[A-Z]/g, (c) => "-" + c.toLowerCase())
        .replace(/[^a-z0-9-]/g, "");
    assert(ids.has(id), guide.kind + " missing " + property.key);
    properties++;
  }
  for (const match of html.matchAll(/\b(?:href|src)="([^"]+)"/g)) {
    const url = match[1];
    if (/^(https?:|data:|mailto:)/.test(url)) continue;
    const [path, fragment] = url.split("#");
    const target = path
      ? resolve(dirname(file), decodeURIComponent(path))
      : file;
    await access(target);
    links++;
    if (fragment) {
      const text = target === file ? html : await readFile(target, "utf8");
      assert(
        text.includes('id="' + fragment + '"'),
        guide.kind + " missing anchor " + url,
      );
    }
  }
  const json = html.match(
    /<script type="application\/json" id="guide-data">([\s\S]*?)<\/script>/,
  )?.[1];
  assert(json);
  assert.equal(JSON.parse(json).kind, guide.kind);
}
const catalog = JSON.parse(
  await readFile(resolve(site, "assets/tool-catalog.json"), "utf8"),
);
for (const tool of catalog.tools)
  assert(
    manifest.guides.some((g) => g.path === tool.guide),
    "Catalog missing guide for " + tool.name,
  );
console.log(
  "PASS: " +
    manifest.guides.length +
    " tool pages, " +
    properties +
    " property anchors, " +
    links +
    " local links/assets, " +
    catalog.tools.length +
    " catalog guide links.",
);

const native = await readFile(
  resolve(site, "../src-tauri/src/tool_guides.rs"),
  "utf8",
);
const ui = await readFile(resolve(site, "../ui/src/ToolGuideLink.tsx"), "utf8");
for (const guide of manifest.guides) {
  const slug = guide.path.slice(6, -5);
  assert(
    native.includes('"' + guide.kind + '" => "' + slug + '"'),
    "Native guide link missing " + guide.kind,
  );
  assert(
    ui.includes(guide.kind + ': "' + slug + '"'),
    "UI guide link missing " + guide.kind,
  );
}
console.log("PASS: all 35 guides match UI and native destination allowlists.");
