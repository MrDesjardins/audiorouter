import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  copyFileSync,
  existsSync,
} from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const inventory = JSON.parse(
  readFileSync(
    resolve(root, "tools/site/tool-guide-property-manifest.json"),
    "utf8",
  ),
);
const tools = inventory.tools;
const output = resolve(
  root,
  "docs/plans/active/evidence/tool-guide-preview/all-tools",
);
mkdirSync(output, { recursive: true });
const escape = (value) =>
  String(value).replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ],
  );
const entries = [];
const guideSources = ["processing", "routing"].flatMap(
  (name) =>
    JSON.parse(
      readFileSync(resolve(root, `site/guide-content/${name}.json`), "utf8"),
    ).tools,
);
const encode = (source, destination) => {
  if (process.env.GUIDE_METADATA_ONLY === "1") {
    if (!existsSync(destination))
      throw new Error(`Missing encoded asset: ${destination}`);
    return;
  }
  const result = spawnSync(
    "ffmpeg",
    [
      "-y",
      "-i",
      source,
      "-c:v",
      "libx264",
      "-preset",
      "slow",
      "-crf",
      "26",
      "-maxrate",
      "700k",
      "-bufsize",
      "1400k",
      "-pix_fmt",
      "yuv420p",
      "-c:a",
      "copy",
      "-movflags",
      "+faststart",
      destination,
    ],
    { encoding: "utf8" },
  );
  if (result.status !== 0) throw new Error(result.stderr);
};
const recordings = {};
const assets = resolve(root, "site/assets/tool-guides");
mkdirSync(assets, { recursive: true });
for (const tool of tools) {
  const source = resolve(
    root,
    "target/tool-guide-overviews",
    tool.kind,
    "overview.mp4",
  );
  if (!existsSync(source)) throw new Error(`Missing overview: ${tool.kind}`);
  const video = `${tool.kind}.mp4`;
  const poster = `${tool.kind}.jpg`;
  encode(source, resolve(output, video));
  const result = spawnSync(
    "ffmpeg",
    [
      "-y",
      "-i",
      source,
      "-frames:v",
      "1",
      "-q:v",
      "3",
      resolve(output, poster),
    ],
    { encoding: "utf8" },
  );
  if (result.status !== 0) throw new Error(result.stderr);
  copyFileSync(
    resolve(output, video),
    resolve(assets, `${tool.kind}-overview.mp4`),
  );
  copyFileSync(
    resolve(output, poster),
    resolve(assets, `${tool.kind}-overview.jpg`),
  );
  recordings[tool.kind] = [
    {
      title: "Browse Properties",
      video: `assets/tool-guides/${tool.kind}-overview.mp4`,
      poster: `assets/tool-guides/${tool.kind}-overview.jpg`,
      description: `A silent recording of the real ${tool.title} Properties panel. The view browses its settings and returns to the top. Device, network and plugin operation are not demonstrated by this configuration overview.`,
    },
  ];
  const demoPath = resolve(
    root,
    "target/tool-guide-demos",
    tool.kind,
    "overview.mp4",
  );
  if (existsSync(demoPath)) {
    const telemetry = JSON.parse(
      readFileSync(
        resolve(root, "target/tool-guide-tools", tool.kind, "telemetry.json"),
        "utf8",
      ),
    );
    const render = JSON.parse(
      readFileSync(
        resolve(root, "target/tool-guide-tools/render-report.json"),
        "utf8",
      ),
    ).find((r) => r.kind === tool.kind);
    const property = tool.properties.find((p) => p.key === render.parameter);
    const label =
      property?.label ??
      (tool.kind === "mixer" ? "Input volume" : "Live level");
    const before = telemetry.parametersBefore[render.parameter];
    const after = telemetry.parametersAfter[render.parameter];
    const original = guideSources
      .find((t) => t.kind === tool.kind)
      ?.properties.find(
        (p) =>
          p.key === render.parameter ||
          p.key === render.parameter?.replace(/band\d+/, "band{index}") ||
          (tool.kind === "mixer" && p.key.startsWith("inputVolume:")),
      );
    const explanation =
      typeof before === "number" && typeof after === "number"
        ? after < before
          ? original?.decrease
          : original?.increase
        : original?.meaning;
    const description = render.parameter
      ? `${label} changes from ${before} to ${after}${property?.unit ? " " + property.unit : ""} at 3.2 seconds.${explanation ? " " + explanation : ""} The control appears briefly, then the view returns to the top visual while the synthetic audio continues.${tool.kind === "outputSwitch" ? " You hear output A: it becomes silent when the signal moves to B." : ""}${tool.kind === "audioFile" ? " This short example shows the Loop setting; it ends before the full source repeats." : ""}`
      : "A synthetic signal passes through the real processor. Watch the live readings rise and fall with the sound.";
    encode(demoPath, resolve(assets, `${tool.kind}-demo.mp4`));
    const posterResult = spawnSync(
      "ffmpeg",
      [
        "-y",
        "-i",
        demoPath,
        "-frames:v",
        "1",
        "-q:v",
        "3",
        resolve(assets, `${tool.kind}-demo.jpg`),
      ],
      { encoding: "utf8" },
    );
    if (posterResult.status !== 0) throw new Error(posterResult.stderr);
    copyFileSync(
      resolve(assets, `${tool.kind}-demo.mp4`),
      resolve(output, `${tool.kind}-demo.mp4`),
    );
    copyFileSync(
      resolve(assets, `${tool.kind}-demo.jpg`),
      resolve(output, `${tool.kind}-demo.jpg`),
    );
    recordings[tool.kind].unshift({
      title: `${label} in the app`,
      video: `assets/tool-guides/${tool.kind}-demo.mp4`,
      poster: `assets/tool-guides/${tool.kind}-demo.jpg`,
      description,
      property: render.parameter,
    });
  }
  const primary = recordings[tool.kind][0];
  for (const property of tool.properties) {
    const demonstration = recordings[tool.kind].find(
      (clip) =>
        clip.property === property.key ||
        (tool.kind === "mixer" &&
          property.dynamic &&
          clip.property?.startsWith("inputVolume:")),
    );
    if (demonstration) {
      property.recordings = [demonstration];
      property.status = "recorded";
    } else if (
      tool.capture === "configuration" ||
      ["file", "selection"].includes(property.type)
    ) {
      property.recordings = [recordings[tool.kind].at(-1)];
      property.status = "exempted";
      property.nonAudibleReason =
        "Configuration selection is shown in the silent Properties overview. Actual device, file, peer or installed-plugin operation is outside this synthetic recording.";
    }
  }
  const primaryVideo = primary.video.split("/").pop();
  const primaryPoster = primary.poster.split("/").pop();
  copyFileSync(
    resolve(root, "site", primary.video),
    resolve(output, primaryVideo),
  );
  copyFileSync(
    resolve(root, "site", primary.poster),
    resolve(output, primaryPoster),
  );
  entries.push(
    `<article data-title="${escape(tool.title.toLowerCase())}"><h2>${escape(tool.title)}</h2><video controls preload="none" poster="${primaryPoster}" aria-label="${escape(tool.title)} Properties recording"><source src="${primaryVideo}" type="video/mp4"></video><p>${escape(primary.description)}</p>${recordings[tool.kind].length > 1 ? `<details><summary>Browse the full Properties panel (silent)</summary><video controls preload="none" poster="${poster}" aria-label="${escape(tool.title)} configuration overview"><source src="${video}" type="video/mp4"></video></details>` : ""}</article>`,
  );
}
writeFileSync(
  resolve(root, "tools/site/tool-guide-property-manifest.json"),
  JSON.stringify(inventory, null, 2) + "\n",
);
writeFileSync(
  resolve(root, "site/guide-content/recordings.json"),
  JSON.stringify(recordings, null, 2) + "\n",
);
writeFileSync(
  resolve(output, "index.html"),
  `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>All tool Properties recordings</title><style>body{margin:0;background:#121820;color:#eef3f8;font:18px/1.6 system-ui,sans-serif}main{max-width:1150px;margin:auto;padding:24px}h1{line-height:1.2}p{max-width:75ch}label{display:block}input{display:block;box-sizing:border-box;width:100%;padding:12px;border:1px solid #8395a8;border-radius:9px;background:#1d2835;color:inherit;font:inherit}.clips{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,300px),1fr));gap:24px}video{display:block;width:100%;aspect-ratio:9/16;background:#090d12;border-radius:12px}article[hidden]{display:none}</style><main><h1>All 38 tool Properties panels</h1><p>Each video shows the actual application interface in a portrait frame. The canvas and app chrome are outside the recording. All tools have a silent configuration overview. Processing tools also have a synthetic-audio example. Additional property coverage remains in progress.</p><label>Find a tool<input id="filter" type="search" placeholder="Tool name"></label><div class="clips">${entries.join("")}</div></main><script>document.querySelector('#filter').addEventListener('input',e=>{const query=e.target.value.toLowerCase().trim();for(const card of document.querySelectorAll('article')){card.hidden=!card.dataset.title.includes(query);if(card.hidden)for(const v of card.querySelectorAll('video'))v.pause()}});document.addEventListener('play',e=>{if(e.target instanceof HTMLMediaElement)for(const v of document.querySelectorAll('video'))if(v!==e.target)v.pause()},true)</script></html>`,
);
console.log(
  `Exported ${entries.length} real Properties overview videos to ${output}`,
);
