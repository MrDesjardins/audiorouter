// Render one example for each remaining parameter family. Repeated EQ points
// and Soundboard buttons share the same control-family demonstration.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const inventory = JSON.parse(
  readFileSync(
    resolve(root, "tools/site/tool-guide-property-manifest.json"),
    "utf8",
  ),
);
const executable = resolve(
  root,
  "target/debug/examples/tool_guide_property_render.exe",
);
const presetResult = spawnSync(executable, ["--voice-presets"], {
  encoding: "utf8",
});
if (presetResult.status !== 0) throw new Error(presetResult.stderr);
const presets = JSON.parse(presetResult.stdout);
const output = resolve(root, "target/tool-guide-properties");
mkdirSync(output, { recursive: true });
writeFileSync(
  resolve(output, "voice-presets.json"),
  JSON.stringify(presets, null, 2),
);
const selected = process.env.GUIDE_KIND?.split(",");
const reportPath = resolve(output, "render-report.json");
const report = selected
  ? JSON.parse(readFileSync(reportPath, "utf8")).filter(
      (row) => !selected.includes(row.kind),
    )
  : [];
for (const tool of inventory.tools) {
  if (tool.capture !== "production-dsp") continue;
  for (const property of tool.properties) {
    if (
      property.status !== "pending" &&
      !(tool.kind === "soundboard" && property.key === "slot1Mode")
    )
      continue;
    if (
      /^band(?:[1-9]|1[0-5])/.test(property.key) ||
      /^slot[2-9]/.test(property.key)
    )
      continue;
    if (
      process.env.GUIDE_KIND &&
      !process.env.GUIDE_KIND.split(",").includes(tool.kind)
    )
      continue;
    // These are choices of a route reference or an external game-state feed,
    // not synthetic processor behavior. They receive a configuration recording.
    if (
      tool.kind === "duck" &&
      [
        "trigger",
        "keyNodeId",
        "duckMenu",
        "duckPrep",
        "duckAction",
        "duckBetweenRounds",
      ].includes(property.key)
    )
      continue;
    const id = `${tool.kind}--${property.key.replace(/[^a-zA-Z0-9]+/g, "-")}`;
    const directory = resolve(output, id);
    mkdirSync(directory, { recursive: true });
    let before = JSON.parse(
      readFileSync(
        resolve(root, "target/tool-guide-tools", tool.kind, "before.json"),
        "utf8",
      ),
    );
    if (tool.kind === "funVoiceChanger") {
      before = { ...presets.find((p) => p.id === "custom").parameters };
      Object.assign(before, {
        ringHz: 70,
        ringDepthPercent: 70,
        crushBits: 6,
        crushDownsample: 4,
        drivePercent: 45,
        pitchSemitones: 7,
        formantSemitones: 3,
        modDepthPercent: 60,
        spaceLevelPercent: 40,
        peakDb: 9,
        scale: "major",
      });
      const families = [
        ["correctEnabled", ["key", "scale", "retuneMs", "humanizePercent"]],
        ["shiftEnabled", ["pitchSemitones", "pitchCents", "formantSemitones"]],
        ["ringEnabled", ["ringHz", "ringDepthPercent"]],
        ["crushEnabled", ["crushBits", "crushDownsample"]],
        ["driveEnabled", ["drivePercent"]],
        ["toneEnabled", ["lowCutHz", "highCutHz", "peakHz", "peakDb"]],
        [
          "modEnabled",
          ["modType", "modRateHz", "modDepthPercent", "modFeedbackPercent"],
        ],
        [
          "spaceEnabled",
          ["spaceSizePercent", "spaceDecayMs", "spaceLevelPercent"],
        ],
      ];
      for (const [enabled, fields] of families)
        if (fields.includes(property.key) || enabled === property.key)
          before[enabled] = true;
    }
    if (tool.kind === "bassTreble" && property.key === "bassFrequencyHz")
      before.bassDb = 9;
    if (tool.kind === "bassTreble" && property.key === "trebleFrequencyHz")
      before.trebleDb = 9;
    if (tool.kind === "parametricEq") {
      before.band0Enabled = true;
      before.band0GainDb = 9;
    }
    if (tool.kind === "compressor") before.thresholdDb = -35;
    if (tool.kind === "gate") before.thresholdDb = -35;
    if (tool.kind === "limiter") before.ceilingDb = -10;
    let after = { ...before };
    const current = before[property.key] ?? property.default;
    if (property.type === "boolean") after[property.key] = !current;
    else if (property.type === "number") {
      const min = property.minimum,
        max = property.maximum;
      let next = current === min ? max : min;
      const values = {
        ratio: 8,
        attackMs: 100,
        releaseMs: 600,
        holdMs: 600,
        kneeDb: 18,
        hysteresisDb: 9,
        rangeDb: 12,
        lookaheadMs: 10,
        retuneMs: 150,
        humanizePercent: 80,
        pitchSemitones: -7,
        pitchCents: 70,
        formantSemitones: -4,
        ringHz: 160,
        ringDepthPercent: 20,
        crushBits: 12,
        crushDownsample: 12,
        drivePercent: 80,
        whisperPercent: 85,
        lowCutHz: 350,
        highCutHz: 3000,
        peakHz: 2500,
        peakDb: -12,
        modRateHz: 5,
        modDepthPercent: 15,
        modFeedbackPercent: 70,
        spaceSizePercent: 85,
        spaceDecayMs: 3000,
        spaceLevelPercent: 75,
        mixPercent: 30,
        outputDb: -12,
        band0GainDb: -12,
        band0Q: 8,
        bassFrequencyHz: 160,
        trebleFrequencyHz: 5000,
        floorPercent: 60,
        reductionDb: 10,
        gainDb: -12,
        cents: 70,
        levelDb: -30,
        durationMs: 2000,
      };
      if (property.key in values) next = values[property.key];
      next = Math.min(max, Math.max(min, next));
      if (next === current) next = current === min ? max : min;
      after[property.key] = next;
    } else if (property.key === "preset")
      after = { ...presets.find((p) => p.id === "robot").parameters };
    else if (property.key === "slot1Caption")
      after[property.key] = "Bright tone";
    else if (property.key === "slot1Mode") after[property.key] = "toggle";
    else if (property.options?.length)
      after[property.key] = property.options.find(
        (option) => option !== current,
      );
    else throw new Error(`No safe example choice for ${id}`);
    if (property.key === "band0Type") after.band0Type = "lowPass";
    if (property.key === 'fade') after.selected = 'b';
    if (tool.kind === 'gate' && property.key === 'ratio') after.ratio = 1;
    const beforePath = resolve(directory, "before.json");
    const afterPath = resolve(directory, "after.json");
    writeFileSync(beforePath, JSON.stringify(before));
    writeFileSync(afterPath, JSON.stringify(after));
    const fixture = tool.kind === 'limiter' ? 'drum-loop' :
      tool.kind === "funVoiceChanger"
        ? "expressive-voice"
        : tool.kind === "gate"
          ? "noisy-speech"
          : tool.kind === "dehum" && property.key === "harmonics"
            ? "harmonic-hum"
            : property.key === "learning"
              ? "noise-only"
              : ["denoise", "speechDenoise", "spectralGate"].includes(tool.kind)
                ? "noisy-speech"
                : tool.fixture;
    const result = spawnSync(
      executable,
      [directory, tool.kind, fixture, beforePath, afterPath],
      { encoding: "utf8" },
    );
    report.push({
      id,
      kind: tool.kind,
      parameter: property.key,
      directory,
      audible: !["slot1Caption", "slot1Mode"].includes(property.key),
      status:
        result.status === 0 ? "audio-rendered-unreviewed" : "render-failed",
      error: result.status === 0 ? null : result.stderr,
    });
    writeFileSync(
      resolve(output, "render-report.json"),
      JSON.stringify(report, null, 2),
    );
    console.log(`${id}: ${report.at(-1).status}`);
  }
}
if (report.some((row) => row.status === "render-failed")) process.exitCode = 1;
