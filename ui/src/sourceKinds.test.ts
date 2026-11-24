import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import type { NodeKind } from "@audiorouter/contracts";
import { isRouteSource, rendersOwnAudio, supportsApplicationMixerRoute } from "./draft";

/** UI-1: every kind, so adding one to `NodeKind` fails typecheck here until it is listed. */
const ALL_KINDS: Record<NodeKind, true> = {
  physicalInput: true,
  applicationCapture: true,
  endpointLoopback: true,
  physicalOutput: true,
  virtualRenderSource: true,
  virtualCaptureSink: true,
  testSignal: true,
  mixer: true,
  gain: true,
  mute: true,
  meter: true,
  parametricEq: true,
  compressor: true,
  gate: true,
  limiter: true,
  delay: true,
  graphicEq: true,
  pitch: true,
  recorder: true,
  plugin: true,
  audioFile: true,
  soundboard: true,
  instrument: true,
  volume: true,
  bassTreble: true,
  dehum: true,
  declick: true,
  inputSwitch: true,
  outputSwitch: true,
  denoise: true,
  speechDenoise: true,
  firFilter: true,
  spectralGate: true,
  timeShift: true,
  networkSend: true,
  networkReceive: true,
  duck: true,
  funVoiceChanger: true,
};
const KINDS = Object.keys(ALL_KINDS) as NodeKind[];

/** The kinds the engine's `renders_own_audio` accepts, read from its source. */
function engineGeneratorKinds(): Set<string> {
  const here = dirname(fileURLToPath(import.meta.url));
  const source = readFileSync(resolve(here, "../../crates/engine/src/lib.rs"), "utf8");
  const start = source.indexOf("pub fn renders_own_audio");
  expect(start).toBeGreaterThan(0);
  const arm = source.slice(start, source.indexOf("=> true", start));
  const kinds = [...arm.matchAll(/NodeKind::(\w+)/g)].map(([, name]) => name[0].toLowerCase() + name.slice(1));
  return new Set(kinds);
}

describe("source kind classification (UI-1)", () => {
  it("renders own audio exactly where the engine does", () => {
    const engine = engineGeneratorKinds();
    expect(engine.size).toBeGreaterThanOrEqual(4);
    for (const kind of engine) expect(KINDS, `engine kind ${kind} is unknown to the UI`).toContain(kind);
    for (const kind of KINDS) expect(rendersOwnAudio(kind), kind).toBe(engine.has(kind));
  });

  it("treats every generator as a route source", () => {
    for (const kind of KINDS) if (rendersOwnAudio(kind)) expect(isRouteSource(kind), kind).toBe(true);
  });

  it("lists the route sources", () => {
    expect(KINDS.filter(isRouteSource).sort()).toEqual(
      [
        "applicationCapture",
        "audioFile",
        "endpointLoopback",
        "instrument",
        "networkReceive",
        "physicalInput",
        "soundboard",
        "testSignal",
        "virtualRenderSource",
      ].sort(),
    );
  });

  it("limits the application Mixer route to devices, applications and generated sound", () => {
    expect(KINDS.filter(supportsApplicationMixerRoute).sort()).toEqual(
      ["applicationCapture", "audioFile", "physicalInput", "soundboard", "testSignal"].sort(),
    );
  });
});
