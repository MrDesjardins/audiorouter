import { describe, expect, it } from "vitest";
import type { ApplicationAudioEndpoint, ApplicationInfo, DeviceListItem, Session } from "@audiorouter/contracts";
import {
  appDevices,
  cablePartner,
  deviceUsage,
  discordCheck,
  knownAppOf,
  nodeDeviceUsers,
  sessionInvolvesDiscord,
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
const plays = (deviceId: string, active = true): ApplicationAudioEndpoint => ({
  deviceId,
  direction: "render",
  active,
});

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
  edges: [edge("mic", "chain"), edge("chain", "voice-out"), edge("discord", "mix"), edge("mix", "headphones")],
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

  it("judges only active sessions, not devices an app used before", () => {
    const use = appDevices(
      [app("Discord.exe", [plays("headphones", false)]), app("Discord.exe", [plays("cable-in")])],
      "discord",
    );
    expect(use).toEqual({
      running: true,
      capture: [],
      render: ["cable-in"],
      idleCapture: [],
      idleRender: ["headphones"],
    });
    // The stale headphone session must not read as "heard twice".
    const check = discordCheck(
      session,
      [app("Discord.exe", [records("cable-a-out"), plays("headphones", false), plays("cable-in")])],
      devices,
    );
    expect(check.state).toBe("ready");
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

  it("does not take sound coming in from a cable for your voice", () => {
    // Game audio read from CABLE Output and sent on to CABLE-A Input is not a microphone.
    const relay = {
      ...session,
      nodes: [
        node("game-in", "physicalInput", { endpointId: "cable-out" }),
        node("relay-out", "physicalOutput", { endpointId: "cable-a-in" }),
      ],
      edges: [edge("game-in", "relay-out")],
    } as unknown as Session;
    expect(voiceCableOutputs(relay, devices)).toEqual([]);
    expect(sessionInvolvesDiscord(relay, devices)).toBe(false);
  });
});

describe("Discord check", () => {
  it("does not keep a Ready badge after the endpoint inventory fails", () => {
    const check = discordCheck(
      session,
      [app("Discord.exe", [records("cable-a-out"), plays("cable-in")])],
      devices,
      true,
    );
    expect(check.state).toBe("unknown");
    expect(check.summary).toContain("Could not refresh Discord");
  });
  it.each(["mic", "voice-out"])("does not call a disabled %s voice path ready", (id) => {
    const silent = {
      ...session,
      nodes: session.nodes.map((item) => (item.id === id ? { ...item, enabled: false } : item)),
    };
    expect(voiceCableOutputs(silent, devices)).toEqual([]);
    const check = discordCheck(silent, [app("Discord.exe", [records("cable-a-out"), plays("cable-in")])], devices);
    expect(check.rows[0].state).not.toBe("ready");
    expect(check.state).not.toBe("ready");
  });

  it("follows only the selected switch ports, and bypasses ordinary disabled effects dry", () => {
    const switched = {
      ...session,
      nodes: [...session.nodes, node("switch", "outputSwitch", { selected: "a" })],
      edges: [
        edge("mic", "switch"),
        { ...edge("switch", "voice-out"), sourcePort: "b" },
        { ...edge("switch", "headphones"), sourcePort: "a" },
      ],
    } as unknown as Session;
    expect(voiceCableOutputs(switched, devices)).toEqual([]);
    const selected = {
      ...switched,
      nodes: switched.nodes.map((item) => (item.id === "switch" ? { ...item, parameters: { selected: "b" } } : item)),
    };
    expect(voiceCableOutputs(selected, devices).map((item) => item.id)).toEqual(["cable-a-out"]);
    const dry = {
      ...session,
      nodes: session.nodes.map((item) => (item.id === "chain" ? { ...item, enabled: false } : item)),
    };
    expect(voiceCableOutputs(dry, devices).map((item) => item.id)).toEqual(["cable-a-out"]);
  });

  it("does not follow an unselected Input Switch input or a disabled Mixer", () => {
    const switched = {
      ...session,
      nodes: [
        node("mic", "physicalInput", { endpointId: "mic" }),
        node("switch", "inputSwitch", { selected: "a" }),
        node("voice-out", "physicalOutput", { endpointId: "cable-a-in" }),
      ],
      edges: [{ ...edge("mic", "switch"), destinationPort: "b" }, edge("switch", "voice-out")],
    } as unknown as Session;
    expect(voiceCableOutputs(switched, devices)).toEqual([]);
    const silent = {
      ...session,
      nodes: session.nodes.map((item) =>
        item.id === "chain" ? { ...item, kind: "mixer" as const, enabled: false } : item,
      ),
    };
    expect(voiceCableOutputs(silent, devices)).toEqual([]);
  });

  it("is ready on the owner's working setup", () => {
    const check = discordCheck(session, [app("Discord.exe", [records("cable-a-out"), plays("cable-in")])], devices);
    expect(check.state).toBe("ready");
    expect(check.rows.map((row) => [row.id, row.state, row.deviceName])).toEqual([
      ["microphone", "ready", "CABLE-A Output (VB-Audio Virtual Cable A)"],
      ["speaker", "ready", "CABLE Input (VB-Audio Virtual Cable)"],
    ]);
  });

  it("judges the matching microphone when Discord records from two devices", () => {
    // Windows sorts endpoint IDs, so the raw mic can come first.
    const check = discordCheck(
      session,
      [app("Discord.exe", [records("mic"), records("cable-a-out"), plays("cable-in")])],
      devices,
    );
    expect(check.rows[0]).toMatchObject({ state: "ready", deviceName: "CABLE-A Output (VB-Audio Virtual Cable A)" });
  });

  it("names the 16ch speaker that blocked Game Sound Out on 2026-10-03", () => {
    const check = discordCheck(session, [app("Discord.exe", [records("cable-a-out"), plays("cable-in-16")])], devices);
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

  it("warns when the call's audio would come back as Discord's microphone", () => {
    const check = discordCheck(session, [app("Discord.exe", [records("cable-a-out"), plays("cable-a-in")])], devices);
    expect(check.rows[1]).toMatchObject({ state: "check" });
    expect(check.rows[1].message).toContain("hear themselves");
    // Without a session the same loop is still found from Discord's own devices.
    const alone = discordCheck(null, [app("Discord.exe", [records("cable-out"), plays("cable-in")])], devices);
    expect(alone.rows[1].message).toContain("hear themselves");
  });

  it("warns when Discord is heard twice in the headphones", () => {
    const check = discordCheck(session, [app("Discord.exe", [records("cable-a-out"), plays("headphones")])], devices);
    expect(check.rows[1]).toMatchObject({ state: "check" });
    expect(check.rows[1].message).toContain("twice");
  });

  it("shows the top-bar chip from the session alone", () => {
    expect(sessionInvolvesDiscord(session, devices)).toBe(true);
    const plain = {
      ...session,
      nodes: session.nodes.filter((item) => item.id !== "discord" && item.id !== "voice-out"),
    };
    expect(sessionInvolvesDiscord(plain, devices)).toBe(false);
    expect(sessionInvolvesDiscord(null, devices)).toBe(false);
  });

  it("confirms the last call's devices out of a call, and says to join a voice channel", () => {
    // The owner's Discord out of a call on 2026-10-23: every session idle,
    // with older outputs kept beside the one the call used.
    const idle = discordCheck(
      session,
      [
        app("Discord.exe", [records("cable-a-out", false), plays("cable-in", false), plays("headphones", false)]),
        app("Discord.exe", [plays("headphones", false)]),
      ],
      devices,
    );
    expect(idle.state).toBe("ready");
    expect(idle.summary).toContain("last call");
    expect(idle.rows[0]).toMatchObject({ state: "ready", deviceName: "CABLE-A Output (VB-Audio Virtual Cable A)" });
    expect(idle.rows[0].message).toContain("last call");
    // Two idle outputs: Windows does not say which one the call used.
    expect(idle.rows[1]).toMatchObject({ state: "info", deviceName: null });
    expect(idle.rows[1].message).toContain("voice channel");

    const oneOutput = discordCheck(
      session,
      [app("Discord.exe", [records("cable-a-out", false), plays("cable-in", false)])],
      devices,
    );
    expect(oneOutput.rows[1]).toMatchObject({ state: "ready", deviceName: "CABLE Input (VB-Audio Virtual Cable)" });
    expect(oneOutput.rows[1].message).toContain("last call");
  });

  it("never warns from idle sessions", () => {
    // A raw-mic session beside the cable may be the newer choice: wait for a call.
    const mixed = discordCheck(
      session,
      [app("Discord.exe", [records("mic", false), records("cable-a-out", false), plays("headphones", false)])],
      devices,
    );
    expect(mixed.state).toBe("unknown");
    expect(mixed.summary).toBe("Waiting for you to join a voice channel in Discord");
    expect(mixed.rows.map((row) => row.state)).toEqual(["info", "info"]);
    // An idle output that would be heard twice is not judged either.
    const heardTwice = discordCheck(
      session,
      [app("Discord.exe", [records("cable-a-out", false), plays("headphones", false)])],
      devices,
    );
    expect(heardTwice.state).toBe("ready");
    expect(heardTwice.rows[1].state).toBe("info");
  });

  it("never judges what Windows does not report", () => {
    const notRunning = discordCheck(session, [], devices);
    expect(notRunning.state).toBe("notRunning");
    // The same two rows in every state, so the card never changes size.
    expect(notRunning.rows.map((row) => [row.id, row.state])).toEqual([
      ["microphone", "info"],
      ["speaker", "info"],
    ]);
    const idle = discordCheck(session, [app("Discord.exe", [])], devices);
    expect(idle.state).toBe("unknown");
    expect(idle.rows.map((row) => row.state)).toEqual(["info", "info"]);
    const noVoiceCable = discordCheck(null, [app("Discord.exe", [records("mic")])], devices);
    // Not a warning, but a way forward: the row offers the cable setup.
    expect(noVoiceCable.rows[0]).toMatchObject({ state: "info", action: "virtualCables" });
  });
});

describe("device node users (D1 footers)", () => {
  it("lists the app that records from the other end of a voice cable", () => {
    const users = nodeDeviceUsers(session, devices, [
      app("Discord.exe", [records("cable-a-out"), plays("headphones")]),
      app("obs64.exe", [records("cable-a-out")]),
      app("RainbowSix.exe", [plays("cable-in")]),
    ]);
    expect(users.get("voice-out")).toEqual({
      summary: "Used by Discord · OBS",
      uses: [
        { label: "Discord", appId: "discord", role: "records from CABLE-A Output (VB-Audio Virtual Cable A)" },
        { label: "OBS", appId: "obs", role: "records from CABLE-A Output (VB-Audio Virtual Cable A)" },
      ],
      warning: null,
    });
    expect(users.get("headphones")?.summary).toBe("Used by Discord");
    expect(users.get("game-out")?.uses).toEqual([
      { label: "RainbowSix", appId: null, role: "plays to it or records it" },
    ]);
    // Only device nodes get a line.
    expect(users.has("chain")).toBe(false);
  });

  it("counts only active sessions, so an app out of a call is not listed", () => {
    const users = nodeDeviceUsers(session, devices, [app("Discord.exe", [records("cable-a-out", false)])]);
    expect(users.get("voice-out")).toMatchObject({ summary: "No app using it right now", warning: null });
  });

  it("warns when an app records from the cable's 16ch twin instead", () => {
    const users = nodeDeviceUsers(session, devices, [app("Discord.exe", [records("cable-a-out-16")])]);
    expect(users.get("voice-out")).toMatchObject({
      summary: "Discord is on CABLE-A Out 16ch",
      warning:
        "Discord records from CABLE-A Out 16ch (VB-Audio Virtual Cable A), not CABLE-A Output (VB-Audio Virtual Cable A)",
    });
  });

  it("warns when Discord records the raw microphone instead of the voice cable", () => {
    const users = nodeDeviceUsers(session, devices, [
      app("Discord.exe", [records("mic")]),
      app("Audacity.exe", [records("mic")]),
    ]);
    expect(users.get("mic")).toEqual({
      // The footer names the problem; the full warning is in Properties.
      summary: "Discord skips your processing",
      uses: [
        { label: "Discord", appId: "discord", role: "records from it" },
        { label: "Audacity", appId: null, role: "records from it" },
      ],
      warning:
        "Discord records this microphone without your processing; choose CABLE-A Output (VB-Audio Virtual Cable A) instead",
    });
  });

  it("says when a node has no device yet", () => {
    const unbound = { ...session, nodes: [node("speakers", "physicalOutput")] } as unknown as Session;
    expect(nodeDeviceUsers(unbound, devices, []).get("speakers")?.summary).toBe("No device chosen");
  });
});
