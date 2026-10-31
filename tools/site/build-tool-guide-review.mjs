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
import { selectedKinds } from "./tool-guide-guards.mjs";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const inventory = JSON.parse(
  readFileSync(
    resolve(root, "tools/site/tool-guide-property-manifest.json"),
    "utf8",
  ),
);
const tools = inventory.tools;
// GUIDE_KIND=a,b rebuilds only those tools. Every other tool keeps its
// recordings from site/guide-content/recordings.json and its encoded media.
const kinds = selectedKinds(
  process.env.GUIDE_KIND,
  tools.map((tool) => tool.kind),
);
// GUIDE_REUSE_ENCODED=1 keeps media that is already encoded and encodes only
// the files that do not exist yet.
const reuseEncoded = process.env.GUIDE_REUSE_ENCODED === "1";
const previousRecordings = kinds
  ? JSON.parse(
      readFileSync(resolve(root, "site/guide-content/recordings.json"), "utf8"),
    )
  : {};
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
// Plain names for the synthetic fixtures in
// crates/engine/examples/tool_guide_property_render.rs.
const testSounds = {
  "noise-only": "quiet background hiss",
  "speech-bursts": "a short, buzzy test voice",
  "noisy-speech": "a buzzy test voice over background hiss",
  "expressive-voice": "a test voice that glides up and down in pitch",
  "pink-noise": "steady pink noise",
  "drum-loop": "a simple drum loop",
  "tone-steps": "a tone that steps up from 220 Hz to 1,760 Hz",
  "mains-hum": "a test voice over 50 Hz mains hum",
  "harmonic-hum": "a test voice over 50 Hz hum and its overtones",
  "click-train": "quiet hiss with sharp clicks",
  "two-tones": "a two-tone chord of 110 Hz and 880 Hz",
  "level-bursts": "a 220 Hz tone that jumps between quiet and loud",
};
// Tools that make the sound themselves rather than process the fixture.
const selfPlaying = {
  testSignal: "its test tone",
  audioFile: "a prepared tone file",
  soundboard: "a prepared tone file",
};
const noteNames = {
  c: "C",
  cs: "C♯",
  d: "D",
  ds: "D♯",
  e: "E",
  f: "F",
  fs: "F♯",
  g: "G",
  gs: "G♯",
  a: "A",
  as: "A♯",
  b: "B",
};
// The switch renders ignore the fixture: input A carries 110 Hz and input B
// 880 Hz (tool_guide_property_render.rs), and Output Switch has one input.
const switchSounds = {
  inputSwitch: "while input A carries a low 110 Hz tone and input B a high 880 Hz tone",
  outputSwitch: "while a low 110 Hz tone plays through the Output Switch",
};
const formatValue = (value, property) => {
  if (typeof value === "number") {
    const number = value
      .toLocaleString("en-US", { maximumFractionDigits: 3 })
      .replace("-", "−");
    const unit = property?.unit;
    if (property?.key === "ratio") return `${number}:1`;
    if (!unit || unit === "normalized" || unit === property.label?.toLowerCase())
      return number;
    return unit === "%" ? `${number}%` : `${number} ${unit}`;
  }
  if (property?.key === "key") return noteNames[value] ?? value;
  if (typeof value === "string" && value.length <= 2)
    return value.toUpperCase();
  const words = String(value).replace(/([A-Z])/g, " $1").toLowerCase();
  return words.charAt(0).toUpperCase() + words.slice(1);
};
const guideProperty = (kind, key) =>
  guideSources
    .find((t) => t.kind === kind)
    ?.properties.find(
      (p) =>
        p.key === key ||
        p.key === key?.replace(/(band|slot)\d+/, "$1{index}") ||
        p.key === key?.replace(/slot\d+/, "slot") ||
        (kind === "mixer" && p.key.startsWith("inputVolume:")),
    );
const capitalize = (text) => text.charAt(0).toUpperCase() + text.slice(1);
// Describe one recorded change in plain words: what moves, when, what is
// playing, and what the reader should hear.
const describeChange = ({
  tool,
  key,
  label,
  property,
  before,
  after,
  fixture,
  silent,
}) => {
  const original = guideProperty(tool.kind, key);
  let change;
  if (typeof before === "number" && typeof after === "number")
    change = `${label} moves from ${formatValue(before, property)} to ${formatValue(after, property)}`;
  else if (typeof after === "boolean")
    change = `${label} is turned ${after ? "on" : "off"}`;
  else if (before === undefined || before === "")
    change = `${label} is set to ${typeof after === "string" && /\s/.test(after) ? `“${after}”` : formatValue(after, property)}`;
  else
    change = `${label} switches from ${formatValue(before, property)} to ${formatValue(after, property)}`;
  // Guide rows describe booleans as "On: …" / "Off: …"; keep the side shown.
  const toggled =
    typeof after === "boolean" &&
    (after ? original?.increase : original?.decrease)?.replace(
      /^(On|Off): (.)/,
      (_, _state, first) => first.toUpperCase(),
    );
  const explanation =
    typeof before === "number" && typeof after === "number"
      ? after < before
        ? original?.decrease
        : original?.increase
      : toggled || original?.meaning;
  if (silent)
    return `${change} about three seconds in. The change has no sound of its own, so this clip is silent.${explanation ? " " + explanation : ""}`;
  const playing = selfPlaying[tool.kind]
    ? `while the ${tool.title} plays ${selfPlaying[tool.kind]}`
    : (switchSounds[tool.kind] ??
      `while ${testSounds[fixture] ?? "a test sound"} plays through the ${tool.title}`);
  return `${change} about three seconds in, ${playing}.${explanation ? " " + explanation : ""}`;
};
const needsYourSetup = {
  physicalInput: "your own audio device",
  physicalOutput: "your own audio device",
  endpointLoopback: "your own audio device",
  applicationCapture: "an app running on your PC",
  virtualRenderSource: "a virtual cable on your PC",
  virtualCaptureSink: "a virtual cable on your PC",
  networkSend: "another computer on your network",
  networkReceive: "another computer on your network",
  plugin: "a plugin installed on your PC",
  recorder: "the files the Recorder saves on your PC",
};
const exemptionReason = (tool, property) => {
  const tour = "The silent Properties tour shows where to find it.";
  if (tool.kind === "group")
    return `${property.label} only changes how the group looks on the canvas, so there is nothing to hear. ${tour}`;
  if (property.type === "file")
    return `${property.label} plays a WAV or MP3 file you choose, so there is no fixed sound to record. ${tour}`;
  if (needsYourSetup[tool.kind])
    return `${property.label} works with ${needsYourSetup[tool.kind]}, which a prepared recording can't stand in for. ${tour}`;
  if (/^slot\dMode$/.test(property.key))
    return `${property.label} works the same way as button 1, which has its own clip. ${tour}`;
  return `${property.label} depends on other nodes in your own route, so no separate clip plays it. ${tour}`;
};
const encode = (source, destination) => {
  if (process.env.GUIDE_METADATA_ONLY === "1") {
    if (!existsSync(destination))
      throw new Error(`Missing encoded asset: ${destination}`);
    return;
  }
  if (reuseEncoded && existsSync(destination)) return;
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
const extractPoster = (source, destination) => {
  if (reuseEncoded && existsSync(destination)) return;
  const result = spawnSync(
    "ffmpeg",
    ["-y", "-i", source, "-frames:v", "1", "-q:v", "3", destination],
    { encoding: "utf8" },
  );
  if (result.status !== 0) throw new Error(result.stderr);
};
const galleryEntry = ({ tool, primary, clipCount, video, poster }) => {
  const primaryVideo = primary.video.split("/").pop();
  const primaryPoster = primary.poster.split("/").pop();
  return `<article data-title="${escape(tool.title.toLowerCase())}"><h2>${escape(tool.title)}</h2><video controls preload="none" poster="${primaryPoster}" aria-label="${escape(tool.title)} Properties recording"><source src="${primaryVideo}" type="video/mp4"></video><p>${escape(primary.description)}</p>${clipCount > 1 ? `<details><summary>Browse the full Properties panel (silent)</summary><video controls preload="none" poster="${poster}" aria-label="${escape(tool.title)} configuration overview"><source src="${video}" type="video/mp4"></video></details>` : ""}</article>`;
};
const recordings = {};
const assets = resolve(root, "site/assets/tool-guides");
mkdirSync(assets, { recursive: true });
for (const tool of tools) {
  const video = `${tool.kind}.mp4`;
  const poster = `${tool.kind}.jpg`;
  if (kinds && !kinds.includes(tool.kind)) {
    const kept = previousRecordings[tool.kind];
    if (!kept?.length)
      throw new Error(`No recordings.json entry to keep for ${tool.kind}`);
    recordings[tool.kind] = kept;
    entries.push(
      galleryEntry({
        tool,
        primary:
          kept.find((clip) => clip.video.endsWith(`/${tool.kind}-demo.mp4`)) ??
          kept[0],
        clipCount: kept.length,
        video,
        poster,
      }),
    );
    continue;
  }
  const source = resolve(
    root,
    "target/tool-guide-overviews",
    tool.kind,
    "overview.mp4",
  );
  if (!existsSync(source)) throw new Error(`Missing overview: ${tool.kind}`);
  encode(source, resolve(output, video));
  extractPoster(source, resolve(output, poster));
  copyFileSync(
    resolve(output, video),
    resolve(assets, `${tool.kind}-overview.mp4`),
  );
  copyFileSync(
    resolve(output, poster),
    resolve(assets, `${tool.kind}-overview.jpg`),
  );
  const overview = {
    title: "Every setting",
    video: `assets/tool-guides/${tool.kind}-overview.mp4`,
    poster: `assets/tool-guides/${tool.kind}-overview.jpg`,
    description: `A silent tour of the ${tool.title} Properties panel. It scrolls through every setting, then returns to the top.`,
  };
  // Clips that change one property, kept in Properties-panel order.
  const changeClips = [];
  let demo;
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
    // Mixer input levels are stored as 0–1 gains and shown as percentages.
    const mixerInput =
      tool.kind === "mixer" && render.parameter?.startsWith("inputVolume:");
    const shown = (value) =>
      mixerInput && typeof value === "number" ? value * 100 : value;
    const description = render.parameter
      ? `${describeChange({
          tool,
          key: render.parameter,
          label,
          property: mixerInput
            ? tool.properties.find((row) => row.dynamic)
            : property,
          before: shown(telemetry.parametersBefore[render.parameter]),
          after: shown(telemetry.parametersAfter[render.parameter]),
          fixture: telemetry.fixture,
        })}${tool.kind === "outputSwitch" ? " You're listening to output A, so the sound stops when the switch moves to B." : ""}${tool.kind === "audioFile" ? " The clip is shorter than the file, so you won't hear it start over." : ""}`
      : `${capitalize(testSounds[telemetry.fixture] ?? "a test sound")} plays through the ${tool.title}. Watch the readings rise and fall with it.`;
    encode(demoPath, resolve(assets, `${tool.kind}-demo.mp4`));
    extractPoster(demoPath, resolve(assets, `${tool.kind}-demo.jpg`));
    copyFileSync(
      resolve(assets, `${tool.kind}-demo.mp4`),
      resolve(output, `${tool.kind}-demo.mp4`),
    );
    copyFileSync(
      resolve(assets, `${tool.kind}-demo.jpg`),
      resolve(output, `${tool.kind}-demo.jpg`),
    );
    demo = {
      title: label,
      video: `assets/tool-guides/${tool.kind}-demo.mp4`,
      poster: `assets/tool-guides/${tool.kind}-demo.jpg`,
      description,
      property: render.parameter,
    };
    changeClips.push(demo);
  }
  // Publish every verified per-property capture, not only the representative
  // per-tool demo. These clips are generated by the production renderer and
  // real Properties UI capture path; capture.json ties each to its property.
  const propertyRenders = JSON.parse(
    readFileSync(resolve(root, "target/tool-guide-properties/render-report.json"), "utf8"),
  ).filter((render) => render.kind === tool.kind && render.status === "audio-rendered-unreviewed");
  for (const render of propertyRenders) {
    const sourceDir = resolve(root, "target/tool-guide-property-recordings", render.id);
    const videoSource = resolve(sourceDir, "overview.mp4");
    const capture = JSON.parse(readFileSync(resolve(sourceDir, "capture.json"), "utf8"));
    if (capture.status !== "complete" || capture.id !== render.id)
      throw new Error(`Property capture metadata is incomplete: ${render.id}`);
    const videoName = `${render.id}.mp4`;
    const posterName = `${render.id}.jpg`;
    encode(videoSource, resolve(assets, videoName));
    extractPoster(videoSource, resolve(assets, posterName));
    const property = tool.properties.find((row) => row.key === render.parameter);
    if (!property) throw new Error(`No inventory row for ${render.id}`);
    const telemetry = JSON.parse(
      readFileSync(
        resolve(root, "target/tool-guide-properties", render.id, "telemetry.json"),
        "utf8",
      ),
    );
    const recording = {
      title: property.label,
      video: `assets/tool-guides/${videoName}`,
      poster: `assets/tool-guides/${posterName}`,
      description: describeChange({
        tool,
        key: render.parameter,
        label: property.label,
        property,
        before: telemetry.parametersBefore[render.parameter],
        after: telemetry.parametersAfter[render.parameter],
        fixture: telemetry.fixture,
        silent: render.audible === false,
      }),
      property: render.parameter,
      silent: render.audible === false,
    };
    changeClips.push(recording);
  }
  // Sort by the property's place in the Properties panel. A demo whose
  // parameter has no inventory row (Mixer input level, Meter) leads.
  const panelIndex = (clip) =>
    tool.properties.findIndex(
      (row) =>
        row.key === clip.property ||
        (tool.kind === "mixer" &&
          row.dynamic &&
          clip.property?.startsWith("inputVolume:")),
    );
  changeClips.sort((a, b) => panelIndex(a) - panelIndex(b));
  recordings[tool.kind] = [...changeClips, overview];
  // The review page leads with the representative per-tool demo.
  const primary = demo ?? recordings[tool.kind][0];
  if (tool.kind === "duck") {
    for (const mode of ["game", "level"]) {
      const id = `duck--${mode}`;
      const sourceDir = resolve(root, "target/tool-guide-configs", id);
      const capture = JSON.parse(readFileSync(resolve(sourceDir, "capture.json"), "utf8"));
      if (capture.status !== "complete") throw new Error(`Incomplete configuration capture: ${id}`);
      encode(resolve(sourceDir, "overview.mp4"), resolve(assets, `${id}.mp4`));
      const result = spawnSync("ffmpeg", ["-y", "-i", resolve(sourceDir, "overview.mp4"), "-frames:v", "1", "-q:v", "3", resolve(assets, `${id}.jpg`)], { encoding: "utf8" });
      if (result.status !== 0) throw new Error(result.stderr);
      const clip = {
        title: mode === "game" ? "Siege round trigger" : "Triggered by another node",
        video: `assets/tool-guides/${id}.mp4`,
        poster: `assets/tool-guides/${id}.jpg`,
        silent: true,
        description:
          mode === "game"
            ? "A silent look at the Siege round trigger: choose which parts of a Siege match lower the sound. Hearing it work needs live game data from a match in progress, so this clip has no audio."
            : "A silent look at the Triggered by setting: pick another node in the same session, such as your processed voice, whose level lowers this sound. Hearing it work needs that second source playing, so this clip has no audio.",
      };
      recordings[tool.kind].push(clip);
      for (const property of tool.properties.filter(row => mode === "game" ? ["trigger", "duckMenu", "duckPrep", "duckAction", "duckBetweenRounds"].includes(row.key) : row.key === "keyNodeId")) {
        property.recordings = [clip];
        property.status = "exempted";
        property.nonAudibleReason = clip.description;
      }
    }
  }
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
      property.nonAudibleReason = null;
    } else if (
      tool.capture === "configuration" ||
      ["file", "selection"].includes(property.type)
    ) {
      property.recordings = [overview];
      property.status = "exempted";
      property.nonAudibleReason = exemptionReason(tool, property);
    } else if (
      !property.recordings?.some((clip) => recordings[tool.kind].includes(clip))
    ) {
      // A clip carried over from an earlier run no longer shows this
      // property (for example a demo that now changes another band).
      property.recordings = [];
      property.status = "pending";
      property.nonAudibleReason = null;
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
    galleryEntry({
      tool,
      primary,
      clipCount: recordings[tool.kind].length,
      video,
      poster,
    }),
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
