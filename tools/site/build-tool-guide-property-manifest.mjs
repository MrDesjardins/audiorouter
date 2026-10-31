import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { loadFamilies, representativeKey } from "./tool-guide-families.mjs";

const families = loadFamilies();
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const processing = JSON.parse(
  readFileSync(resolve(root, "site/guide-content/processing.json"), "utf8"),
);
const routing = JSON.parse(
  readFileSync(resolve(root, "site/guide-content/routing.json"), "utf8"),
);
const source = readFileSync(resolve(root, "crates/domain/src/lib.rs"), "utf8");
const all = source.match(/pub const ALL: \[Self; \d+\] = \[(.*?)\];/s)?.[1];
if (!all)
  throw new Error("Could not find NodeKind::ALL in crates/domain/src/lib.rs");
const domainKinds = [...all.matchAll(/Self::([A-Za-z0-9]+)/g)].map(
  (match) => match[1],
);
const kindName = (name) => name[0].toLowerCase() + name.slice(1);
const guideTools = [...processing.tools, ...routing.tools];
const guideKinds = new Set(guideTools.map((tool) => tool.kind));
const domainNames = new Set([...domainKinds.map(kindName), "group"]);
const missingGuides = [...domainNames].filter((kind) => !guideKinds.has(kind));
const staleGuides = [...guideKinds].filter((kind) => !domainNames.has(kind));
if (missingGuides.length || staleGuides.length)
  throw new Error(
    `NodeKind/guide inventory mismatch; missing=${missingGuides.join(",")}; stale=${staleGuides.join(",")}`,
  );

const configurationOnly = new Set([
  "physicalInput",
  "physicalOutput",
  "applicationCapture",
  "endpointLoopback",
  "virtualRenderSource",
  "virtualCaptureSink",
  "networkSend",
  "networkReceive",
  "plugin",
  "recorder",
  "group",
]);
const output = resolve(root, "tools/site/tool-guide-property-manifest.json");
const priorRows = new Map();
if (existsSync(output)) {
  const prior = JSON.parse(readFileSync(output, "utf8"));
  for (const tool of prior.tools ?? [])
    for (const property of tool.properties ?? [])
      priorRows.set(`${tool.kind}\0${property.key}`, property);
}
const fixtureFor = (kind) => {
  if (configurationOnly.has(kind)) return "ui-only";
  if (
    [
      "gate",
      "compressor",
      "limiter",
      "duck",
      "denoise",
      "speechDenoise",
      "spectralGate",
      "funVoiceChanger",
    ].includes(kind)
  )
    return "speech-bursts";
  if (["bassTreble", "graphicEq", "parametricEq", "firFilter"].includes(kind))
    return "pink-noise";
  if (["dehum"].includes(kind)) return "mains-hum";
  if (["declick"].includes(kind)) return "click-train";
  if (["pitch", "testSignal"].includes(kind)) return "tone-steps";
  if (["delay", "timeShift"].includes(kind)) return "drum-loop";
  if (["mixer", "inputSwitch", "outputSwitch"].includes(kind))
    return "two-tones";
  return "level-bursts";
};

function expandProperties(tool) {
  const result = [];
  for (const property of tool.properties ?? []) {
    if (tool.kind === "parametricEq" && property.key.includes("{index}")) {
      for (let index = 0; index < 16; index++)
        result.push({
          ...property,
          key: property.key.replaceAll("{index}", String(index)),
          label: `${property.label} · band ${index + 1}`,
        });
    } else if (tool.kind === "soundboard" && property.key.startsWith("slot")) {
      for (let slot = 1; slot <= 9; slot++)
        result.push({
          ...property,
          key: property.key.replace("slot", `slot${slot}`),
          label: `${property.label} · button ${slot}`,
        });
    } else if (
      tool.kind === "mixer" &&
      property.key === "inputVolume:<upstreamNodeId>"
    ) {
      result.push({
        ...property,
        key: "inputVolume:<upstreamNodeId>",
        label: "Input level · each connected source",
        dynamic: true,
      });
    } else if (
      tool.kind === "plugin" &&
      property.key === "pluginParameter:<parameterId>"
    ) {
      result.push({
        ...property,
        dynamic: true,
        nonAudibleReason:
          "Ranges and effect meaning belong to the exact installed plugin; do not invent a plugin fixture.",
      });
    } else result.push(property);
  }
  return result;
}

const tools = guideTools.map((tool) => ({
  kind: tool.kind,
  title: tool.title,
  sourceFile: processing.tools.includes(tool)
    ? "processing.json"
    : "routing.json",
  fixture: fixtureFor(tool.kind),
  capture: configurationOnly.has(tool.kind)
    ? "configuration"
    : "production-dsp",
  properties: expandProperties(tool).map((property) => {
    const prior = priorRows.get(`${tool.kind}\0${property.key}`);
    const recordings = prior?.recordings ?? [];
    return {
      key: property.key,
      label: property.label,
      type: property.type,
      unit: property.unit ?? null,
      minimum: property.minimum ?? null,
      maximum: property.maximum ?? null,
      default: property.default ?? null,
      options: property.options ?? null,
      dynamic: property.dynamic ?? false,
      nonAudibleReason:
        property.nonAudibleReason ?? prior?.nonAudibleReason ?? null,
      recordingId: `${tool.kind}--${property.key.replace(/[^a-zA-Z0-9]+/g, "-").replace(/^-|-$/g, "")}`,
      // Repeated controls (EQ bands 2-16) point at the row whose clips
      // represent them; see tool-guide-families.mjs.
      ...(representativeKey(families[tool.kind], property.key)
        ? { representativeKey: representativeKey(families[tool.kind], property.key) }
        : {}),
      recordings,
      status: recordings.length ? "recorded" : (prior?.status ?? "pending"),
    };
  }),
}));

const inventory = {
  schemaVersion: 1,
  framing: {
    aspectRatio: "9:16",
    frame: "selected Properties panel only",
    playback:
      "keep the top live visual/status area visible; briefly show a changed lower control, then return to the top",
  },
  source: [
    "crates/domain/src/lib.rs NodeKind::ALL",
    "site/guide-content/processing.json",
    "site/guide-content/routing.json",
  ],
  nodeKindCount: domainKinds.length,
  guideKindCount: tools.length,
  propertyCount: tools.reduce(
    (count, tool) => count + tool.properties.length,
    0,
  ),
  tools,
};

writeFileSync(output, `${JSON.stringify(inventory, null, 2)}\n`, "utf8");
console.log(
  `Wrote ${output}: ${inventory.nodeKindCount} node kinds, ${inventory.guideKindCount} guide kinds, ${inventory.propertyCount} property rows`,
);
