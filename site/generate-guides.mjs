import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
const site = dirname(fileURLToPath(import.meta.url));
const esc = (value) =>
  String(value ?? "").replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ],
  );
const slug = (kind) => kind.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());
const tools = [];
for (const name of ["processing", "routing"])
  tools.push(
    ...JSON.parse(
      await readFile(resolve(site, "guide-content", name + ".json"), "utf8"),
    ).tools,
  );
if (new Set(tools.map((t) => t.kind)).size !== tools.length)
  throw Error("Duplicate guide kind");
const tutorials = JSON.parse(
  await readFile(resolve(site, "guide-content/tutorials.json"), "utf8"),
);
const recordings = JSON.parse(
  await readFile(resolve(site, "guide-content/recordings.json"), "utf8"),
);
const mediaVersion = (path) => esc(path) + "?v=" + createHash("sha256").update(readFileSync(resolve(site, path))).digest("hex").slice(0, 12);
const recordingSection = (kind) =>
  '<section class="guide-section" id="in-the-app"><h2>In the app.</h2><p>' +
  ((recordings[kind] ?? []).some((clip) => clip.property && !clip.silent)
    ? "These are screen recordings of the real Properties panel. Where a clip has sound, it’s a prepared test signal run through AudioRouter’s own audio engine, not a recording of anyone’s microphone or audio device."
    : "These are screen recordings of the real Properties panel. They have no sound.") +
  "</p>" +
  (recordings[kind] ?? [])
    .map(
      (clip) =>
        '<article class="guide-recording"><div class="guide-recording-copy"><h3>' +
        esc(clip.title) +
        '</h3><p>' + esc(clip.description) + '</p></div><video controls preload="none" playsinline width="720" height="1280" poster="../' +
        mediaVersion(clip.poster) +
        '" aria-label="' +
        esc(clip.title) +
        '"><source src="../' +
        mediaVersion(clip.video) +
        '" type="video/mp4"></video></article>',
    )
    .join("") +
  "</section>";
for (const tool of tools)
  if (!tutorials[tool.kind]) throw Error("Missing tutorial: " + tool.kind);
const visualTypes = {
  gain: "gain",
  volume: "volume",
  bassTreble: "bassTreble",
  graphicEq: "graphicEq",
  parametricEq: "parametricEq",
  compressor: "dynamics",
  gate: "gate",
  limiter: "limiter",
  delay: "delay",
  pitch: "pitch",
  funVoiceChanger: "pitch",
  dehum: "dehum",
  declick: "declick",
  denoise: "denoise",
  speechDenoise: "speechDenoise",
  spectralGate: "spectralGate",
  firFilter: "fir",
  mute: "mute",
  inputSwitch: "inputSwitch",
  outputSwitch: "outputSwitch",
  duck: "duck",
  timeShift: "buffer",
  mixer: "mixerTwoTrack",
  meter: "meter",
  recorder: "recording",
  plugin: "plugin",
  testSignal: "tone",
  audioFile: "media",
  soundboard: "media",
  physicalInput: "channels",
  networkSend: "network",
  networkReceive: "network",
  group: "group",
};
const demos = {
  gain: [["gainDb", "Gain", -18, 12, 0, 1, "dB"]],
  volume: [["percent", "Volume", 0, 200, 100, 1, "%"]],
  bassTreble: [
    ["bassDb", "Bass", -12, 12, 0, 1, "dB"],
    ["bassFrequencyHz", "Bass frequency", 80, 1000, 500, 10, "Hz"],
    ["trebleDb", "Treble", -12, 12, 0, 1, "dB"],
    ["trebleFrequencyHz", "Treble frequency", 800, 12000, 1500, 50, "Hz"],
  ],
  compressor: [
    ["thresholdDb", "Threshold", -60, 0, -18, 1, "dB"],
    ["ratio", "Ratio", 1, 20, 3, 0.5, ":1"],
  ],
  gate: [
    ["thresholdDb", "Threshold", -80, 0, -45, 1, "dBFS"],
    ["rangeDb", "Maximum reduction", 0, 80, 60, 1, "dB"],
  ],
  limiter: [["ceilingDb", "Ceiling", -12, 0, -1, 0.5, "dB"]],
  parametricEq: [
    ["frequencyHz", "Selected band frequency", 20, 20000, 100, 10, "Hz"],
    ["gainDb", "Selected band gain", -18, 18, 4, 0.5, "dB"],
    ["q", "Selected band Q", 0.1, 20, 0.7, 0.1, ""],
  ],
  graphicEq: [
    ...[
      "31.5 Hz",
      "63 Hz",
      "125 Hz",
      "250 Hz",
      "500 Hz",
      "1 kHz",
      "2 kHz",
      "4 kHz",
      "8 kHz",
      "16 kHz",
    ].map((label, index) => [`band${index}Db`, label, -18, 18, 0, 1, "dB"]),
  ],
  firFilter: [
    ["wetPercent", "Wet mix", 0, 100, 100, 1, "%"],
    ["gainDb", "Output gain", -24, 12, 0, 1, "dB"],
  ],
  networkReceive: [["bufferMs", "Receive buffer", 10, 500, 40, 5, "ms"]],
  dehum: [
    ["frequencyHz", "Mains frequency", 45, 65, 60, 1, "Hz"],
    ["harmonics", "Harmonics", 1, 8, 4, 1, ""],
    ["amountPercent", "Notch depth", 0, 100, 50, 1, "%"],
  ],
  declick: [["thresholdPercent", "Click threshold", 0, 100, 50, 1, "%"]],
  denoise: [
    ["reductionPercent", "Reduction", 0, 100, 70, 1, "%"],
    ["floorPercent", "Noise left", 0, 100, 10, 1, "%"],
  ],
  speechDenoise: [["strengthPercent", "Strength", 0, 100, 70, 1, "%"]],
  spectralGate: [
    ["thresholdDb", "Threshold above noise", -20, 20, 3, 1, "dB"],
    ["reductionDb", "Blocked-bin reduction", 0, 80, 40, 1, "dB"],
  ],
  delay: [["delayMs", "Delay", 0, 1000, 0, 10, "ms"]],
  pitch: [["semitones", "Pitch", -12, 12, 0, 1, "semitones"]],
  funVoiceChanger: [
    ["pitchSemitones", "Pitch", -12, 12, -5, 1, "st"],
    ["formantSemitones", "Formant", -12, 12, -3, 1, "st"],
    ["ringDepthPercent", "Robot buzz", 0, 100, 0, 5, "%"],
  ],
  duck: [["amountDb", "Target attenuation", 0, 40, 6, 1, "dB"]],
  testSignal: [
    ["frequencyHz", "Frequency", 100, 3000, 440, 20, "Hz"],
    ["levelDb", "Level", -36, 0, -12, 1, "dBFS"],
  ],
  mixer: [
    ["track1Percent", "Input A volume", 0, 100, 80, 1, "%"],
    ["track2Percent", "Input B volume", 0, 100, 55, 1, "%"],
  ],
};
// Pictures with a time axis and a playhead, and pictures with no motion.
const timelineKinds = new Set([
  "gain",
  "volume",
  "compressor",
  "limiter",
  "gate",
  "duck",
  "delay",
  "audioFile",
  "soundboard",
]);
const stillKinds = new Set(["group", "physicalInput"]);
const audioKinds = new Set([
  "gain",
  "volume",
  "bassTreble",
  "graphicEq",
  "parametricEq",
  "compressor",
  "gate",
  "limiter",
  "delay",
  "pitch",
  "funVoiceChanger",
  "dehum",
  "declick",
  "denoise",
  "speechDenoise",
  "spectralGate",
  "firFilter",
  "mute",
  "inputSwitch",
  "outputSwitch",
  "duck",
  "timeShift",
  "mixer",
  "meter",
  "recorder",
  "testSignal",
  "audioFile",
  "networkSend",
  "networkReceive",
]);
// Browser and search-result titles, phrased as the problem people search for.
// The page heading keeps the tool's name as it appears in the app.
const searchTitles = {
  gain: "Gain: boost or cut audio level in decibels",
  volume: "Volume control for any audio source on Windows",
  bassTreble: "Bass and treble control for Windows audio",
  graphicEq: "10-band graphic equalizer for Windows audio",
  parametricEq: "Parametric EQ for your microphone on Windows",
  compressor: "Audio compressor for your microphone and stream",
  gate: "Noise gate for your microphone (Discord, OBS)",
  limiter: "Audio limiter: stop clipping and loud peaks",
  delay: "Audio delay: sync sound with video or another source",
  pitch: "Pitch shifter for live audio on Windows",
  dehum: "Remove 50/60 Hz hum from your microphone",
  declick: "Remove clicks and pops from live audio",
  denoise: "Remove background noise from your microphone",
  speechDenoise: "Speech noise reduction for voice chat and streams",
  spectralGate: "Spectral noise gate: remove fan noise and hiss",
  firFilter: "Convolution: apply an impulse response (IR) live",
  mute: "Mute one audio path without touching the others",
  inputSwitch: "Switch between two audio inputs instantly",
  outputSwitch: "Switch one audio signal between two output destinations",
  duck: "Audio ducking: lower music or game audio automatically",
  timeShift: "Pause and replay live audio (time shift)",
  mixer: "Mix several audio sources into one on Windows",
  meter: "Audio level meter for any point of your route",
  funVoiceChanger: "Free real-time voice changer for Discord and OBS",
  physicalInput: "Use your microphone or audio interface as a source",
  physicalOutput: "Send audio to your headphones or speakers",
  applicationCapture: "Capture audio from one app only (game, browser)",
  endpointLoopback: "Capture what plays on a Windows output (loopback)",
  virtualRenderSource: "Take app audio from an AudioRouter virtual cable",
  virtualCaptureSink: "Send audio to an AudioRouter virtual cable",
  testSignal: "Test tone generator to check your audio routing",
  audioFile: "Play a WAV or MP3 file through your audio route",
  soundboard: "Soundboard: play sounds on demand from buttons or a Stream Deck",
  recorder: "Record any audio route to WAV, FLAC or MP3",
  networkSend: "Send audio to another PC over your home network",
  networkReceive: "Receive audio from another PC (two-PC streaming)",
  plugin: "Use VST plugins on your microphone, crash-safe",
  group: "Organize your audio routing canvas with groups",
};
const pageTitle = (t) => {
  if (!searchTitles[t.kind]) throw new Error("No search title for " + t.kind);
  return searchTitles[t.kind] + " — AudioRouter";
};
const header =
  '<a class="guide-skip" href="#guide-main">Skip to guide</a><div data-header></div>';
const doc = (title, description, body, scripts = "") =>
  '<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>' +
  esc(title) +
  '</title><meta name="description" content="' +
  esc(description) +
  '"><link rel="icon" href="../assets/mark.svg"><link rel="stylesheet" href="../assets/site.css"><link rel="stylesheet" href="../assets/guides.css?v=heading-links-3"><script defer src="../assets/site.js"></script>' +
  scripts +
  '</head><body class="guide-body">' +
  header +
  body +
  '<footer class="guide-footer"><a href="index.html">Explore all visual guides →</a><p>AudioRouter · Local audio. Visible signal flow.</p><a href="../manual.html">Manual</a> · <a href="https://github.com/MrDesjardins/audiorouter">Source & feedback ↗</a></footer></body></html>';
const value = (v, unit) =>
  esc(typeof v === "object" ? JSON.stringify(v) : v) +
  (unit ? " " + esc(unit) : "");
function property(p, i) {
  const bounded = Number.isFinite(p.minimum) && Number.isFinite(p.maximum);
  return (
    '<article class="guide-property" id="property-' +
    esc(slug(p.key).replace(/[^a-z0-9-]/g, "")) +
    '"><div class="property-heading"><span class="property-number">' +
    String(i + 1).padStart(2, "0") +
    "</span><div><h3>" +
    esc(p.label) +
    "</h3><code>" +
    esc(p.key) +
    '</code></div></div><p class="property-meaning">' +
    esc(p.meaning) +
    '</p><div class="property-facts"><span><small>Range / choices</small><strong>' +
    (bounded
      ? value(p.minimum, p.unit) + " → " + value(p.maximum, p.unit)
      : p.options
        ? esc(p.options.join(" · "))
        : p.type === "boolean"
          ? "On / off"
          : p.type === "action"
            ? "Explicit action"
            : "Depends on your selection") +
    "</strong></span><span><small>Default</small><strong>" +
    (p.default === undefined || p.default === null
      ? "Choose explicitly"
      : value(p.default, p.unit)) +
    "</strong></span></div>" +
    (p.increase || p.decrease
      ? '<div class="property-direction">' +
        (p.decrease
          ? "<p><b>↓ Lower / alternative</b>" + esc(p.decrease) + "</p>"
          : "") +
        (p.increase
          ? "<p><b>↑ Higher / alternative</b>" + esc(p.increase) + "</p>"
          : "") +
        "</div>"
      : "") +
    (p.example
      ? '<p class="property-example"><b>Try this</b>' + esc(p.example) + "</p>"
      : "") +
    (p.caution
      ? '<p class="property-note"><b>Keep in mind</b> ' +
        esc(p.caution) +
        "</p>"
      : "") +
    "</article>"
  );
}
await mkdir(resolve(site, "tools"), { recursive: true });
for (let i = 0; i < tools.length; i++) {
  const t = tools[i],
    tutorial = tutorials[t.kind],
    s = slug(t.kind),
    controls =
      t.kind === "graphicEq"
        ? t.properties.map((p) => [p.key, p.label, -18, 18, p.default, 1, "dB"])
        : demos[t.kind] || [];
  const notes =
    t.kind === "group"
      ? "Groups organize the canvas and never process audio."
      : "Select the node to open Properties. Click its name to rename it. While playing, supported parameter-only edits save after a short pause. For structural edits, use Save when the canvas marks a change. Enabled and Bypass are distinct live controls; see the in-app help for their behavior.";
  let extras = "";
  if (t.filterTypes)
    extras +=
      '<section class="guide-section" id="filter-shapes"><div class="eyebrow">Choose the right shape</div><h2>Every filter has a different job.</h2><div class="guide-shape-grid">' +
      t.filterTypes
        .map(
          (v) =>
            "<article><h3>" +
            esc(v.key) +
            "</h3><p>" +
            esc(v.meaning) +
            "</p></article>",
        )
        .join("") +
      "</div></section>";
  if (t.actions)
    extras +=
      '<section class="guide-section" id="actions"><div class="eyebrow">Beyond the controls</div><h2>Actions & transport</h2><div class="guide-actions">' +
      (Array.isArray(t.actions)
        ? t.actions
        : Object.entries(t.actions).map(([k, v]) => ({ label: k, meaning: v }))
      )
        .map((a) =>
          typeof a === "string"
            ? "<p>" + esc(a) + "</p>"
            : "<article><h3>" +
              esc(a.label || a.title || a.key || a.action || "Action") +
              "</h3><p>" +
              esc(a.meaning || a.description || a.effect || "") +
              "</p>" +
              (a.caution ? "<p>" + esc(a.caution) + "</p>" : "") +
              "</article>",
        )
        .join("") +
      "</div></section>";
  const routeMap = {
    physicalOutput: ["AudioRouter route", "Output device", "Speakers / cable"],
    applicationCapture: ["Chosen app", "Process capture", "AudioRouter route"],
    endpointLoopback: [
      "Windows output",
      "Loopback capture",
      "AudioRouter route",
    ],
    virtualRenderSource: [
      "App playback",
      "Cable A render",
      "AudioRouter source",
    ],
    virtualCaptureSink: [
      "AudioRouter route",
      "Cable A capture",
      "Receiving app",
    ],
    networkSend: [
      "PC A · Network Send",
      "UDP / IP network",
      "PC B · AudioRouter",
    ],
    networkReceive: [
      "PC A · AudioRouter",
      "UDP / IP network",
      "PC B · Network Receive",
    ],
    mute: ["Route branch", "Mute / restore", "Same destination"],
  };
  const illustrationBands =
    t.kind === "parametricEq"
      ? [
          {
            label: "Low",
            frequencyHz: 100,
            gainDb: 4,
            q: 0.7,
            filterType: "lowShelf",
          },
          {
            label: "Body",
            frequencyHz: 650,
            gainDb: 4,
            q: 1,
            filterType: "peaking",
          },
          {
            label: "Presence",
            frequencyHz: 2800,
            gainDb: -6,
            q: 4,
            filterType: "notch",
          },
          {
            label: "Air",
            frequencyHz: 10000,
            gainDb: 3,
            q: 0.7,
            filterType: "highShelf",
          },
        ]
      : [];
  const graphicBands =
    t.kind === "graphicEq"
      ? t.properties.map((p, j) => ({
          key: p.key,
          label: p.label,
          frequencyHz: [31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000][
            j
          ],
        }))
      : [];
  const data = {
    kind: t.kind,
    title: t.title,
    type: visualTypes[t.kind] || "routing",
    description: t.concept,
    routeLabels: routeMap[t.kind],
    tutorial,
    controls,
    filterTypes: t.filterTypes || [],
    illustrationBands,
    graphicBands,
    audioRelevant: audioKinds.has(t.kind),
  };
  const json = JSON.stringify(data).replace(/</g, "\\u003c");
  const shapeLabels = {
    peaking: "Peaking / bell",
    lowShelf: "Low shelf",
    highShelf: "High shelf",
    lowPass: "Low pass",
    highPass: "High pass",
    bandPass: "Band pass",
    allPass: "All pass",
    notch: "Notch",
  };
  const parametricBandSelectors =
    t.kind === "parametricEq"
      ? '<div class="guide-band-selects" aria-label="Example parametric EQ band settings"><label>Band to edit<select data-demo-option="selectedBand" aria-label="Select band to edit">' +
        illustrationBands
          .map(
            (band, j) =>
              '<option value="' +
              j +
              '"' +
              (j === 0 ? " selected" : "") +
              ">Band " +
              (j + 1) +
              " · " +
              esc(band.label + " · " + band.frequencyHz + " Hz") +
              "</option>",
          )
          .join("") +
        '</select></label><label>Filter shape<select data-demo-option="filterType" aria-label="Filter shape for selected band">' +
        t.filterTypes
          .map(
            (v) =>
              '<option value="' +
              v.key +
              '"' +
              (v.key === illustrationBands[0].filterType ? " selected" : "") +
              ">" +
              esc(shapeLabels[v.key] || v.key) +
              "</option>",
          )
          .join("") +
        "</select></label></div>"
      : "";
  const body =
    '<main id="guide-main"><div class="guide-hero"><div><a class="guide-back" href="index.html">← Tool guides</a><p class="eyebrow">Audio, explained · ' +
    String(i + 1).padStart(2, "0") +
    " / " +
    tools.length +
    "</p><h1>" +
    esc(t.title) +
    '</h1><p class="guide-lede">' +
    esc(t.purpose) +
    '</p><div class="guide-tags"><span>Visual walkthrough</span><span>' +
    t.properties.length +
    " configuration controls</span>" +
    (audioKinds.has(t.kind)
      ? "<span>Synced audio walkthrough</span>"
      : "<span>Animated walkthrough</span>") +
    '</div><a class="button primary" href="#see-it">See how it works ↓</a></div><div class="guide-hero-note"><span class="guide-orbit" aria-hidden="true">∿</span><p>' +
    esc(t.signalBehavior) +
    '</p></div></div><div class="guide-layout"><aside class="guide-toc"><span class="eyebrow">On this page</span><a href="#understand">The idea</a><a href="#see-it">See it in motion</a><a href="#configuration">Every control</a>' +
    t.properties
      .map(
        (p) =>
          '<a class="toc-property" href="#property-' +
          esc(slug(p.key).replace(/[^a-z0-9-]/g, "")) +
          '">' +
          esc(p.label) +
          "</a>",
      )
      .join("") +
    '<a href="#in-the-app">In the app</a><a href="#examples">Worked examples</a><a href="#avoid">Common mistakes</a></aside><div class="guide-article"><section class="guide-section" id="understand"><div class="eyebrow">01 / Understand</div><h2>How it works.</h2><p class="guide-concept">' +
    esc(t.concept) +
    '</p><div class="guide-route" aria-label="Example signal path">' +
    tutorial.route
      .map(
        (label, index) =>
          (index ? '<b aria-hidden="true">→</b>' : "") +
          "<span>" +
          esc(label) +
          "</span>",
      )
      .join("") +
    '</div><h3>Use it in AudioRouter</h3><ol class="guide-walkthrough">' +
    tutorial.steps.map((step) => "<li>" + esc(step) + "</li>").join("") +
    '</ol><div class="guide-outcomes"><article><h3>What success looks like</h3><p>' +
    esc(tutorial.success) +
    "</p></article><article><h3>If it does not work</h3><p>" +
    esc(tutorial.check) +
    '</p></article></div><p class="guide-note">' +
    esc(notes) +
    '</p></section><section class="guide-section" id="see-it"><div class="eyebrow">02 / Explore</div><h2>Try the example.</h2><p>' +
    esc(tutorial.listen) +
    "</p><div data-guide-demo></div>" +
    (t.kind === "parametricEq"
      ? parametricBandSelectors
      : t.kind === "physicalInput"
        ? '<label class="guide-demo-select">Channel mode<select data-demo-option="channelMode" aria-label="Illustration channel mode"><option value="mono">Mono average</option><option value="stereo">Stereo</option><option value="left">Left only</option><option value="right">Right only</option></select></label>'
        : t.kind === "mute"
          ? '<label class="guide-demo-select">Branch state<select data-demo-option="muted" aria-label="Illustration mute state"><option value="false">Signal passes</option><option value="true">Muted</option></select></label>'
          : t.kind === "inputSwitch"
            ? '<div class="guide-band-selects"><label>Selected input<select data-demo-option="selected" aria-label="Illustration selected input"><option value="a">Input A</option><option value="b">Input B</option></select></label><label>Fade<select data-demo-option="fade" aria-label="Illustration switch fade"><option value="normal">Normal · 0.5 s</option><option value="slow">Slow · 2 s</option></select></label></div>'
            : t.kind === "outputSwitch"
              ? '<label class="guide-demo-select">Selected destination<select data-demo-option="selected" aria-label="Illustration selected output"><option value="a">A · Headphones</option><option value="b">B · OBS cable</option></select></label>'
              : "") +
    "" +
    (controls.length
      ? '<div class="guide-demo-controls" aria-label="Illustration controls">' +
        controls
          .map(
            ([key, label, min, max, initial, step, unit]) =>
              "<label>" +
              esc(label) +
              '<output id="demo-value-' +
              key +
              '">' +
              initial +
              " " +
              esc(unit) +
              '</output><input type="range" data-demo-key="' +
              key +
              '" data-unit="' +
              esc(unit) +
              '" min="' +
              min +
              '" max="' +
              max +
              '" step="' +
              step +
              '" value="' +
              initial +
              '" aria-label="Illustration ' +
              esc(label) +
              '"></label>',
          )
          .join("") +
        "</div>"
      : "") +
    '<p class="guide-note">' +
    // Keep in step with guide-animation.js: which pictures have a time axis
    // (playhead), which only move, and which are still.
    (t.kind === "recorder"
      ? "Record example captures the synthetic source once; Stop recording ends the take and Play recording plays it back from the Audio File stage. This is not a DSP measurement or control for your running AudioRouter session."
      : timelineKinds.has(t.kind)
        ? "The example repeats until you press Stop, and control changes are heard immediately. The traces are drawn from that audio, and the line marks the moment you hear. This is not a DSP measurement or control for your running AudioRouter session."
        : audioKinds.has(t.kind)
          ? "The example repeats until you press Stop, and control changes are heard immediately. The picture shows the tool's shape rather than time; follow the timing in Follow the sound below. This is not a DSP measurement or control for your running AudioRouter session."
          : stillKinds.has(t.kind)
            ? "This diagram is conceptual and does not move. It is not a DSP measurement or control for your running AudioRouter session."
            : "This animated diagram is conceptual. It is not a DSP measurement or control for your running AudioRouter session.") +
    '</p></section><section class="guide-section" id="configuration"><div class="eyebrow">03 / Configure</div><h2>Settings.</h2>' +
    (t.propertyFamily
      ? "<p>" +
        esc(
          "Each band has these controls. Add up to 16 bands (displayed as 1–16), select a band, then adjust its shape, frequency, Q and gain where applicable." +
            (t.propertyFamily.recordingNote ? " " + t.propertyFamily.recordingNote : ""),
        ) +
        "</p>"
      : "") +
    (t.properties.length
      ? t.properties.map(property).join("")
      : '<div class="guide-property"><h3>No numeric settings to guess</h3><p>' +
        esc(
          t.kind === "meter"
            ? "Meter observes the signal. Read RMS, current peak, held peak and clipping in Properties; it does not change the level."
            : "This node is configured through the source or endpoint selection described above. No dedicated numeric property editor is exposed.",
        ) +
        "</p></div>") +
    "</section>" +
    extras +
    recordingSection(t.kind) +
    '<section class="guide-section" id="examples"><div class="eyebrow">04 / Examples</div><h2>Example setups.</h2><div class="guide-recipes">' +
    t.recipes
      .map(
        (r, j) =>
          '<article><span class="recipe-index">Example ' +
          (j + 1) +
          "</span><h3>" +
          esc(r.title) +
          "</h3><ol>" +
          r.steps.map((step) => "<li>" + esc(step) + "</li>").join("") +
          "</ol></article>",
      )
      .join("") +
    '</div><p class="guide-note">These are starting points. Compare at a comfortable monitoring level and adjust for your own source.</p></section><section class="guide-section" id="avoid"><div class="eyebrow">05 / Troubleshooting</div><h2>Common mistakes.</h2><ul class="guide-mistakes">' +
    t.commonMistakes.map((m) => "<li>" + esc(m) + "</li>").join("") +
    '</ul></section><nav class="guide-next" aria-label="More guides"><a href="' +
    slug(tools[(i + tools.length - 1) % tools.length].kind) +
    '.html">← ' +
    esc(tools[(i + tools.length - 1) % tools.length].title) +
    '</a><a href="' +
    slug(tools[(i + 1) % tools.length].kind) +
    '.html">' +
    esc(tools[(i + 1) % tools.length].title) +
    ' →</a></nav></div></div></main><script type="application/json" id="guide-data">' +
    json +
    "</script>";
  await writeFile(
    resolve(site, "tools", s + ".html"),
    doc(
      pageTitle(t),
      t.purpose,
      body,
      '<script defer src="../assets/guide-animation.js"></script><script defer src="../assets/guide-signal.js"></script><script defer src="../assets/guide-audio.js"></script><script defer src="../assets/guide-playback.js"></script><script defer src="../assets/guides.js?v=heading-links-4"></script>',
    ),
  );
}
const cards = tools
  .map(
    (t) =>
      '<a class="guide-index-card" href="' +
      slug(t.kind) +
      '.html" data-guide-search="' +
      esc(
        t.title +
          " " +
          t.purpose +
          " " +
          t.properties.map((p) => p.label).join(" "),
      ).toLowerCase() +
      '"><span class="guide-card-symbol" aria-hidden="true">∿</span><h2>' +
      esc(t.title) +
      "</h2><p>" +
      esc(t.purpose) +
      "</p><small>" +
      t.properties.length +
      " controls · " +
      (audioKinds.has(t.kind)
        ? "synced audio & animation"
        : "animation & examples") +
      "</small><b>Explore this tool ↗</b></a>",
  )
  .join("");
await writeFile(
  resolve(site, "tools/index.html"),
  doc(
    "Audio tool guides with live examples — AudioRouter",
    "Understand every AudioRouter tool through animated signal diagrams, configuration explanations and worked examples.",
    '<main id="guide-main"><section class="guide-index-hero"><p class="eyebrow">AudioRouter tool guides</p><h1>AudioRouter tool guides</h1><p>One focused guide for every tool. See the signal move, learn what each setting means, and follow an example you can try.</p><label class="guide-search">Find a tool or property<input type="search" id="guide-search" placeholder="Try threshold, noise, delay, cable…" autocomplete="off"></label><p id="guide-count" role="status">' +
      tools.length +
      ' visual guides</p></section><section class="guide-index-grid" aria-label="Visual tool guides">' +
      cards +
      '</section><p class="guide-no-results" hidden>No guide matches. Try a tool name or a property such as threshold.</p></main>',
    '<script defer src="../assets/guides.js?v=heading-links-4"></script>',
  ),
);
await writeFile(
  resolve(site, "assets/guide-manifest.json"),
  JSON.stringify(
    {
      version: 1,
      guides: tools.map((t) => ({
        kind: t.kind,
        title: t.title,
        path: "tools/" + slug(t.kind) + ".html",
        propertyCount: t.properties.length,
        audioRelevant: audioKinds.has(t.kind),
      })),
    },
    null,
    2,
  ) + "\n",
);
console.log(
  "Generated " +
    tools.length +
    " visual tool guides with " +
    tools.reduce((n, t) => n + t.properties.length, 0) +
    " configuration entries.",
);

const catalogPath = resolve(site, "assets/tool-catalog.json");
const catalog = JSON.parse(await readFile(catalogPath, "utf8"));
const alias = {
  "Physical input": "physicalInput",
  "Application capture": "applicationCapture",
  "Endpoint loopback": "endpointLoopback",
  "Physical output": "physicalOutput",
  "Parametric EQ": "parametricEq",
  "Gate / Expander": "gate",
  Delay: "delay",
};
for (const item of catalog.tools) {
  const t = tools.find(
    (t) =>
      t.kind === alias[item.name] ||
      t.title.toLowerCase() === item.name.toLowerCase(),
  );
  if (!t) throw Error("No guide for " + item.name);
  item.kind = t.kind;
  item.guide = "tools/" + slug(t.kind) + ".html";
}
for (const kind of ["virtualRenderSource", "virtualCaptureSink", "group"]) {
  const t = tools.find((t) => t.kind === kind);
  if (!catalog.tools.some((x) => x.kind === kind))
    catalog.tools.push({
      name: t.title,
      category: kind === "group" ? "Organization" : "Virtual cables",
      description: t.purpose,
      icon: kind === "group" ? "▢" : "⇄",
      kind,
      guide: "tools/" + slug(kind) + ".html",
    });
}
catalog.nodeKinds = tools.filter((t) => t.kind !== "group").length;
catalog.guideCount = tools.length;
await writeFile(catalogPath, JSON.stringify(catalog, null, 2) + "\n");
