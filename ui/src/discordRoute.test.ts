import { describe, expect, it } from "vitest";
import type { DeviceListItem, Session } from "@audiorouter/contracts";
import { discordCheck, voiceCableOutputs } from "./appUsage";
import { appendDiscordRoute, discordSetupOptions } from "./discordRoute";

function device(
  id: string,
  name: string,
  direction: "capture" | "render",
  defaultRoles: DeviceListItem["defaultRoles"] = [],
): DeviceListItem {
  return {
    id,
    name,
    direction,
    state: "active",
    defaultRoles,
    format: { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
    periods: { default100ns: 100000, minimum100ns: 30000 },
  };
}

const devices = [
  device("webcam", "Microphone (Webcam)", "capture", ["console"]),
  device("mic", "Microphone (PD200X Podcast Microphone)", "capture", ["communications"]),
  device("headphones", "Speakers (Focusrite USB Audio)", "render", ["console"]),
  device("cable-a-in", "CABLE-A Input (VB-Audio Virtual Cable A)", "render"),
  device("cable-a-out", "CABLE-A Output (VB-Audio Virtual Cable A)", "capture"),
  device("cable-a-in-16", "CABLE-A In 16ch (VB-Audio Virtual Cable A)", "render"),
  device("cable-a-out-16", "CABLE-A Out 16ch (VB-Audio Virtual Cable A)", "capture"),
  device("cable-b-in", "CABLE-B Input (VB-Audio Virtual Cable B)", "render"),
  device("cable-b-out", "CABLE-B Output (VB-Audio Virtual Cable B)", "capture"),
];

const port = (name: string, direction: "input" | "output") => ({ name, direction, channels: 2 as const });
function node(id: string, kind: "physicalInput" | "physicalOutput", endpointId?: string): Session["nodes"][number] {
  return {
    id,
    kind,
    typeVersion: 1,
    name: id,
    enabled: true,
    bypass: false,
    parameters: endpointId ? { endpointId } : {},
    ports: [kind === "physicalInput" ? port("out", "output") : port("in", "input")],
  };
}
const empty: Session = { id: "s", name: "Session", revision: 3, schemaVersion: 1, nodes: [], edges: [] };
const edge = (source: string, destination: string) => ({
  id: `${source}-${destination}`,
  sourceNode: source,
  sourcePort: "out",
  destinationNode: destination,
  destinationPort: "in",
  matrix: [1, 0, 0, 1],
  enabled: true,
});

describe("Set up for Discord (D2)", () => {
  it("offers real microphones and whole cables, preselecting the call default and a free cable", () => {
    const options = discordSetupOptions(empty, devices);
    expect(options.microphones.map((item) => item.id)).toEqual(["webcam", "mic"]);
    // 16ch twins are never offered; each choice pairs both ends.
    expect(options.cables.map((item) => [item.input.id, item.output.id])).toEqual([
      ["cable-a-in", "cable-a-out"],
      ["cable-b-in", "cable-b-out"],
    ]);
    expect(options.microphone).toBe("mic");
    expect(options.cable).toBe("cable-a-in");
  });

  it("prefers the session's microphone and skips a cable the session already uses", () => {
    const session: Session = {
      ...empty,
      nodes: [node("Webcam", "physicalInput", "webcam"), node("Game", "physicalInput", "cable-a-out")],
    };
    const options = discordSetupOptions(session, devices);
    expect(options.microphone).toBe("webcam");
    expect(options.cables[0].busy).toBe("Game records from it");
    expect(options.cable).toBe("cable-b-in");
  });

  it("adds one route the Discord check then asks Discord to use", () => {
    const { session, discordMicrophone } = appendDiscordRoute(empty, devices, {
      microphone: "mic",
      cable: "cable-a-in",
      cleanup: false,
    });
    expect(discordMicrophone.id).toBe("cable-a-out");
    expect(session.revision).toBe(3);
    expect(session.nodes.map((item) => [item.kind, item.name, item.parameters.endpointId])).toEqual([
      ["physicalInput", "Microphone", "mic"],
      ["physicalOutput", "Voice to Discord", "cable-a-in"],
    ]);
    expect(session.edges).toHaveLength(1);
    expect(voiceCableOutputs(session, devices).map((item) => item.id)).toEqual(["cable-a-out"]);
    const check = discordCheck(session, [], devices);
    expect(check.state).toBe("notRunning");
  });

  it("puts the cleanup chain between the microphone and the cable", () => {
    const { session } = appendDiscordRoute(empty, devices, { microphone: "mic", cable: "cable-a-in", cleanup: true });
    const order: string[] = [];
    let at = session.nodes.find((item) => item.kind === "physicalInput")!.id;
    for (;;) {
      order.push(session.nodes.find((item) => item.id === at)!.kind);
      const next = session.edges.find((item) => item.sourceNode === at);
      if (!next) break;
      at = next.destinationNode;
    }
    expect(order).toEqual(["physicalInput", "gate", "compressor", "limiter", "physicalOutput"]);
  });

  it("reuses the session's free nodes for the same devices", () => {
    const session: Session = {
      ...empty,
      nodes: [node("Mic", "physicalInput", "mic"), node("Headphones", "physicalOutput", "headphones")],
      edges: [edge("Mic", "Headphones")],
    };
    const result = appendDiscordRoute(session, devices, { microphone: "mic", cable: "cable-b-in", cleanup: false });
    expect(result.session.nodes.filter((item) => item.kind === "physicalInput")).toHaveLength(1);
    expect(result.session.edges.map((item) => item.sourceNode)).toEqual(["Mic", "Mic"]);
  });

  it("refuses a cable that already carries other sound, or the voice", () => {
    const session: Session = {
      ...empty,
      nodes: [node("Music", "physicalInput", "cable-b-out"), node("Cable", "physicalOutput", "cable-a-in")],
      edges: [edge("Music", "Cable")],
    };
    expect(() =>
      appendDiscordRoute(session, devices, { microphone: "mic", cable: "cable-a-in", cleanup: false }),
    ).toThrow("Music already plays into CABLE-A Input (VB-Audio Virtual Cable A). Choose another cable.");
    const done = appendDiscordRoute(empty, devices, { microphone: "mic", cable: "cable-a-in", cleanup: false }).session;
    expect(() => appendDiscordRoute(done, devices, { microphone: "mic", cable: "cable-a-in", cleanup: false })).toThrow(
      "already sends your voice",
    );
  });
});
