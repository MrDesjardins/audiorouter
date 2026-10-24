import { describe, expect, it } from "vitest";
import type { ApplicationAudioEndpoint, ApplicationInfo, DeviceListItem, Session } from "@audiorouter/contracts";
import {
  appDevices,
  cablePartner,
  deviceUsage,
  discordCheck,
  knownAppOf,
  sixteenChannelTwin,
  voiceCableOutputs,
} from "./appUsage";

function device(id: string, name: string, direction: "capture" | "render"): DeviceListItem {
  return {
    id,
    name,
    direction,
    state: "active",
    defaultRoles: [],
    format: { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
    periods: { default100ns: 100000, minimum100ns: 30000 },
  };
}

// The owner's devices on 2026-10-23 (names as Windows lists them).
const devices = [
  device("mic", "Microphone (PD200X Podcast Microphone)", "capture"),
  device("headphones", "Speakers (Focusrite USB Audio)", "render"),
  device("cable-a-in", "CABLE-A Input (VB-Audio Virtual Cable A)", "render"),
  device("cable-a-out", "CABLE-A Output (VB-Audio Virtual Cable A)", "capture"),
  device("cable-a-in-16", "CABLE-A In 16ch (VB-Audio Virtual Cable A)", "render"),
  device("cable-a-out-16", "CABLE-A Out 16ch (VB-Audio Virtual Cable A)", "capture"),
  device("cable-in", "CABLE Input (VB-Audio Virtual Cable)", "render"),
  device("cable-in-16", "CABLE In 16ch (VB-Audio Virtual Cable)", "render"),
  device("cable-out", "CABLE Output (VB-Audio Virtual Cable)", "capture"),
];

let nextPid = 100;
function app(executable: string, endpoints: ApplicationAudioEndpoint[]): ApplicationInfo {
  return {
    processId: nextPid++,
    executable,
    executablePath: null,
    creationTime100ns: null,
    audioActivity: endpoints.some((endpoint) => endpoint.active) ? "active" : "inactive",
    captureCapability: "observed",
    audioSessionCount: endpoints.length,
    activeAudioSessionCount: endpoints.filter((endpoint) => endpoint.active).length,
    captureSessionCount: endpoints.filter((endpoint) => endpoint.direction === "capture").length,
    renderSessionCount: endpoints.filter((endpoint) => endpoint.direction === "render").length,
    audioDisplayNames: [],
    audioEndpoints: endpoints,
  };
}
const records = (deviceId: string, active = true): ApplicationAudioEndpoint => ({
  deviceId,
  direction: "capture",
  active,
});
const plays = (deviceId: string, active = true): ApplicationAudioEndpoint => ({ deviceId, direction: "render", active });

function node(id: string, kind: string, parameters: Record<string, string> = {}) {
  return { id, kind, typeVersion: 1, name: id, enabled: true, bypass: false, parameters, ports: [] };
}
function edge(source: string, destination: string, enabled = true) {
  return {
    id: `${source}-${destination}`,
    sourceNode: source,
    sourcePort: "out",
    destinationNode: destination,
    destinationPort: "in",
    matrix: [1],
    enabled,
  };
}

// The owner's route: mic → voice chain → CABLE-A Input; Discord captured by
// app and mixed into the headphones with the game.
const session = {
  id: "s",
  name: "Main",
  schemaVersion: 1,
  revision: 1,
  nodes: [
    node("mic", "physicalInput", { endpointId: "mic" }),
    node("chain", "compressor"),
    node("voice-out", "physicalOutput", { endpointId: "cable-a-in" }),
    node("discord", "applicationCapture", { executable: "Discord.exe" }),
    node("mix", "mixer"),
    node("headphones", "physicalOutput", { endpointId: "headphones" }),
    node("game-out", "physicalOutput", { endpointId: "cable-in" }),
  ],
  edges: [
    edge("mic", "chain"),
    edge("chain", "voice-out"),
    edge("discord", "mix"),
    edge("mix", "headphones"),
  ],
} as unknown as Session;

describe("device usage", () => {
  it("merges an app's processes and leaves AudioRouter out", () => {
    const usage = deviceUsage([
      app("Discord.exe", [records("cable-a-out"), plays("cable-in", false)]),
      app("Discord.exe", [plays("cable-in")]),
      app("RainbowSix.exe", [records("cable-a-out")]),
      app("audiorouter-shell.exe", [plays("cable-a-in")]),
    ]);
    expect(usage.get("cable-in")).toEqual([
      { executable: "discord.exe", label: "Discord", appId: "discord", direction: "render", active: true },
    ]);
    // Known apps first, then by name.
    expect(usage.get("cable-a-out")?.map((use) => use.label)).toEqual(["Discord", "RainbowSix"]);
    expect(usage.has("cable-a-in")).toBe(false);
  });

  it("recognizes Discord builds and OBS by executable, case-insensitively", () => {
    expect(knownAppOf("DiscordPTB.exe")).toBe("discord");
    expect(knownAppOf("obs64.exe")).toBe("obs");
    expect(knownAppOf("ow-obs.exe")).toBeNull();
  });

  it("lists an app's active devices before idle ones", () => {
    const use = appDevices(
      [app("Discord.exe", [plays("headphones", false)]), app("Discord.exe", [plays("cable-in")])],
      "discord",
    );
    expect(use).toEqual({ running: true, capture: [], render: ["cable-in", "headphones"] });
  });
});

describe("cables", () => {
  it("pairs each cable's playback and recording sides, never a 16ch twin", () => {
    expect(cablePartner(devices[2], devices)?.id).toBe("cable-a-out");
    expect(cablePartner(devices[3], devices)?.id).toBe("cable-a-in");
    expect(cablePartner(devices[4], devices)).toBeNull();
    expect(cablePartner(devices[1], devices)).toBeNull();
    expect(sixteenChannelTwin(devices[6], devices)?.id).toBe("cable-in-16");
    expect(sixteenChannelTwin(devices[7], devices)?.id).toBe("cable-in");
  });

  it("finds the cable that carries the processed voice", () => {
    expect(voiceCableOutputs(session, devices).map((item) => item.id)).toEqual(["cable-a-out"]);
    const disabled = { ...session, edges: [edge("mic", "chain"), edge("chain", "voice-out", false)] };
    expect(voiceCableOutputs(disabled, devices)).toEqual([]);
  });
});

describe("Discord check", () => {
  it("is ready on the owner's working setup", () => {
    const check = discordCheck(
      session,
      [app("Discord.exe", [records("cable-a-out"), plays("cable-in")])],
      devices,
    );
    expect(check.state).toBe("ready");
    expect(check.rows.map((row) => [row.id, row.state, row.deviceName])).toEqual([
      ["microphone", "ready", "CABLE-A Output (VB-Audio Virtual Cable A)"],
      ["speaker", "ready", "CABLE Input (VB-Audio Virtual Cable)"],
    ]);
  });

  it("names the 16ch speaker that blocked Game Sound Out on 2026-10-03", () => {
    const check = discordCheck(
      session,
      [app("Discord.exe", [records("cable-a-out"), plays("cable-in-16")])],
      devices,
    );
    expect(check.state).toBe("check");
    const speaker = check.rows.find((row) => row.id === "speaker")!;
    expect(speaker.state).toBe("check");
    expect(speaker.expectedName).toBe("CABLE Input (VB-Audio Virtual Cable)");
    expect(speaker.message).toContain("16-channel");
  });

  it("explains a raw microphone, a 16ch microphone and another device differently", () => {
    const message = (deviceId: string) =>
      discordCheck(session, [app("Discord.exe", [records(deviceId)])], devices).rows[0];
    expect(message("mic")).toMatchObject({ state: "check", expectedName: "CABLE-A Output (VB-Audio Virtual Cable A)" });
    expect(message("mic").message).toContain("without AudioRouter's effects");
    expect(message("cable-a-out-16").message).toContain("16-channel");
    expect(message("cable-out").message).toContain("AudioRouter sends your processed voice to");
  });

  it("warns when Discord is heard twice in the headphones", () => {
    const check = discordCheck(session, [app("Discord.exe", [records("cable-a-out"), plays("headphones")])], devices);
    expect(check.rows[1]).toMatchObject({ state: "check" });
    expect(check.rows[1].message).toContain("twice");
  });

  it("never judges what Windows does not report", () => {
    expect(discordCheck(session, [], devices)).toMatchObject({ state: "notRunning", rows: [] });
    const idle = discordCheck(session, [app("Discord.exe", [])], devices);
    expect(idle.state).toBe("unknown");
    expect(idle.rows.map((row) => row.state)).toEqual(["info", "info"]);
    const noVoiceCable = discordCheck(null, [app("Discord.exe", [records("mic")])], devices);
    expect(noVoiceCable.rows[0].state).toBe("info");
  });
});
