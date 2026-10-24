import { describe, expect, it } from "vitest";
import type { ApplicationInfo, DeviceListItem, ObsAudioInput, ObsStatus, Session } from "@audiorouter/contracts";
import { obsCheck, obsFixes } from "./obsUsage";

function device(id: string, name: string, direction: "capture" | "render", isDefault = false): DeviceListItem {
  return {
    id,
    name,
    direction,
    state: "active",
    defaultRoles: isDefault ? ["console", "multimedia", "communications"] : [],
    format: { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
    periods: { default100ns: 100000, minimum100ns: 30000 },
  };
}

// The owner's devices (names as Windows lists them).
const devices = [
  device("mic", "Microphone (PD200X Podcast Microphone)", "capture", true),
  device("headphones", "Speakers (Focusrite USB Audio)", "render", true),
  device("cable-a-in", "CABLE-A Input (VB-Audio Virtual Cable A)", "render"),
  device("cable-a-out", "CABLE-A Output (VB-Audio Virtual Cable A)", "capture"),
  device("cable-a-out-16", "CABLE-A Out 16ch (VB-Audio Virtual Cable A)", "capture"),
  device("cable-b-in", "CABLE-B Input (VB-Audio Virtual Cable B)", "render"),
  device("cable-b-out", "CABLE-B Output (VB-Audio Virtual Cable B)", "capture"),
];

function node(id: string, kind: string, parameters: Record<string, string> = {}) {
  return { id, kind, typeVersion: 1, name: id, enabled: true, bypass: false, parameters, ports: [] };
}
function edge(source: string, destination: string) {
  return {
    id: `${source}-${destination}`,
    sourceNode: source,
    sourcePort: "out",
    destinationNode: destination,
    destinationPort: "in",
    matrix: [1],
    enabled: true,
  };
}

// Mic → voice chain → CABLE-A Input, and the mic monitored in the
// headphones; Discord captured and sent to CABLE-B Input for the stream.
const session = {
  id: "s",
  name: "Main",
  schemaVersion: 1,
  revision: 1,
  nodes: [
    node("mic", "physicalInput", { endpointId: "mic" }),
    node("chain", "compressor"),
    node("voice-out", "physicalOutput", { endpointId: "cable-a-in" }),
    node("monitor", "physicalOutput", { endpointId: "headphones" }),
    node("discord", "applicationCapture", { executable: "Discord.exe" }),
    node("stream-out", "physicalOutput", { endpointId: "cable-b-in" }),
  ],
  edges: [edge("mic", "chain"), edge("chain", "voice-out"), edge("chain", "monitor"), edge("discord", "stream-out")],
} as unknown as Session;

const obsRunning: ApplicationInfo = {
  processId: 4,
  executable: "obs64.exe",
  executablePath: null,
  creationTime100ns: null,
  audioActivity: "inactive",
  captureCapability: "observed",
  audioSessionCount: 0,
  activeAudioSessionCount: 0,
  captureSessionCount: 0,
  renderSessionCount: 0,
  audioDisplayNames: [],
  audioEndpoints: [],
};

function input(
  name: string,
  kind: ObsAudioInput["kind"],
  deviceId: string | null,
  extra: Partial<ObsAudioInput> = {},
): ObsAudioInput {
  return {
    name,
    uuid: `uuid-${name}`,
    kind,
    special: null,
    deviceId,
    application: null,
    muted: false,
    active: true,
    audioTracks: [1],
    monitorOnly: false,
    ...extra,
  };
}

function status(inputs: ObsAudioInput[], overrides: Partial<ObsStatus> = {}): ObsStatus {
  return {
    installed: true,
    connection: "connected",
    source: "live",
    sceneCollection: "Streaming",
    inputs,
    passwordSource: "obsSettings",
    lastChange: { state: "none", changed: 0, restorable: false, error: null },
    ...overrides,
  };
}

describe("OBS check", () => {
  it("trusts an identified live connection even before process inventory catches up", () => {
    expect(
      obsCheck(session, status([input("Mic/Aux", "inputCapture", "cable-a-out")], { installed: false }), [], devices)
        .view,
    ).toBe("connected");
  });
  it("marks retained facts unknown and read-only after a failed status read", () => {
    const check = obsCheck(session, status([input("Mic/Aux", "inputCapture", "mic")]), [obsRunning], devices, true);
    expect(check.state).toBe("unknown");
    expect(check.summary).toContain("Could not refresh");
    expect(obsFixes(check)).toEqual([]);
  });
  it("uses the communications default microphone and console default playback", () => {
    const split = devices.map((item) => ({
      ...item,
      defaultRoles:
        item.id === "cable-a-out"
          ? ["communications" as const]
          : item.defaultRoles.filter((role) => role !== "communications"),
    }));
    const check = obsCheck(session, status([input("Mic/Aux", "inputCapture", "default")]), [obsRunning], split);
    expect(check.sources[0].device).toBe("Default (CABLE-A Output (VB-Audio Virtual Cable A))");
    expect(check.sources[0].problem).toBeNull();
  });

  it("does not claim a disabled voice output carries voice into OBS", () => {
    const silent = {
      ...session,
      nodes: session.nodes.map((item) => (item.id === "voice-out" ? { ...item, enabled: false } : item)),
    };
    const check = obsCheck(silent, status([input("Mic/Aux", "inputCapture", "cable-a-out")]), [obsRunning], devices);
    expect(check.state).not.toBe("ready");
    expect(check.sources[0].hears).not.toContain("Voice");
  });

  it("keeps sources without UUIDs read-only and saved activity unknown", () => {
    const check = obsCheck(
      session,
      status([input("Mic/Aux", "inputCapture", "mic", { uuid: null })]),
      [obsRunning],
      devices,
    );
    expect(check.sources[0].problem).not.toBeNull();
    expect(obsFixes(check)).toEqual([]);
    const saved = obsCheck(
      session,
      status([input("Mic/Aux", "inputCapture", "cable-a-out"), input("Desktop Audio", "outputCapture", "headphones")], {
        source: "savedFile",
        connection: "off",
      }),
      [],
      devices,
    );
    expect(saved.sources[1].problem).toBeNull();
  });

  it("does not suggest adding another microphone when the existing source's activity is unknown", () => {
    const check = obsCheck(
      session,
      status([input("Mic/Aux", "inputCapture", "cable-a-out", { active: null })]),
      [obsRunning],
      devices,
    );
    expect(check.state).toBe("unknown");
    expect(check.summary).toContain("Voice source found");
    expect(check.summary).not.toContain("Add");
  });

  it.each([
    { active: false },
    { active: null },
    { audioTracks: [2] },
    { audioTracks: null },
    { monitorOnly: true },
    { monitorOnly: null },
    { muted: null },
  ])("does not invent duplicated voice with %j", (extra) => {
    const check = obsCheck(
      session,
      status([
        input("Mic/Aux", "inputCapture", "cable-a-out"),
        input("Desktop Audio", "outputCapture", "headphones", extra),
      ]),
      [obsRunning],
      devices,
    );
    expect(check.sources[1].problem).toBeNull();
    expect(check.sources[1].fix).toBeNull();
  });

  it("is ready when OBS records the voice cable once", () => {
    const check = obsCheck(
      session,
      status([
        input("Mic/Aux", "inputCapture", "cable-a-out", { special: "mic1" }),
        input("Stream audio", "outputCapture", "cable-b-in"),
      ]),
      [obsRunning],
      devices,
    );
    expect(check).toMatchObject({ appId: "obs", state: "ready", view: "connected" });
    expect(check.sources.map((row) => [row.name, row.device, row.hears, row.problem])).toEqual([
      ["Mic/Aux", "CABLE-A Output (VB-Audio Virtual Cable A)", ["Voice"], null],
      ["Stream audio", "CABLE-B Input (VB-Audio Virtual Cable B)", ["Discord"], null],
    ]);
  });

  it("offers the voice cable instead of the raw microphone", () => {
    const check = obsCheck(session, status([input("Mic/Aux", "inputCapture", "default")]), [obsRunning], devices);
    expect(check.state).toBe("check");
    const [row] = check.sources;
    expect(row.device).toBe("Default (Microphone (PD200X Podcast Microphone))");
    expect(row.hears).toEqual(["Voice without effects"]);
    expect(row.fix).toEqual({ inputName: "Mic/Aux", deviceId: "cable-a-out" });
    expect(obsFixes(check)).toHaveLength(1);
  });

  it("finds the voice on the stream twice through Desktop Audio, and offers a device without it", () => {
    const check = obsCheck(
      session,
      status([
        input("Mic/Aux", "inputCapture", "cable-a-out", { special: "mic1" }),
        // The default playback device is the headphones, which also play the voice.
        input("Desktop Audio", "outputCapture", "default", { special: "desktop1" }),
      ]),
      [obsRunning],
      devices,
    );
    expect(check.state).toBe("check");
    const desktop = check.sources[1];
    expect(desktop.problem).toContain("twice");
    expect(desktop.hears).toContain("Voice");
    expect(desktop.fix).toEqual({ inputName: "Desktop Audio", deviceId: "cable-b-in" });
    // A muted Desktop Audio is not on the stream.
    const muted = obsCheck(
      session,
      status([
        input("Mic/Aux", "inputCapture", "cable-a-out"),
        input("Desktop Audio", "outputCapture", "default", { muted: true }),
      ]),
      [obsRunning],
      devices,
    );
    expect(muted.state).toBe("ready");
  });

  it("names the 16-channel side of the voice cable", () => {
    const check = obsCheck(session, status([input("Mic", "inputCapture", "cable-a-out-16")]), [obsRunning], devices);
    expect(check.sources[0].problem).toContain("16-channel");
    expect(check.sources[0].fix).toEqual({ inputName: "Mic", deviceId: "cable-a-out" });
  });

  it("judges the saved setup while OBS is closed, but offers no fix", () => {
    const check = obsCheck(
      session,
      status([input("Mic/Aux", "inputCapture", "mic")], { connection: "off", source: "savedFile" }),
      [],
      devices,
    );
    expect(check).toMatchObject({ state: "notRunning", view: "closed" });
    expect(check.summary).toContain("saved last, with 1 setting to check");
    expect(check.sources[0].problem).not.toBeNull();
    expect(obsFixes(check)).toEqual([]);
  });

  it("asks to connect while OBS runs and is not connected, and never fixes from there", () => {
    const check = obsCheck(
      session,
      status([input("Mic/Aux", "inputCapture", "mic")], { connection: "serverOff", source: "savedFile" }),
      [obsRunning],
      devices,
    );
    expect(check.view).toBe("connect");
    expect(obsFixes(check)).toEqual([]);
  });

  it("says plainly when OBS is not installed or nothing records the voice", () => {
    expect(
      obsCheck(session, status([], { installed: false, source: null, connection: "off" }), [], devices),
    ).toMatchObject({
      view: "notInstalled",
      state: "notRunning",
    });
    const none = obsCheck(session, status([input("Game", "applicationCapture", null)]), [obsRunning], devices);
    expect(none.state).toBe("unknown");
    expect(none.summary).toContain("CABLE-A Output");
  });
});
