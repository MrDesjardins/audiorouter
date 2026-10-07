import { describe, expect, it } from "vitest";
import type { DiscoveryDocument, Session } from "@audiorouter/contracts";
import { buildRequest, builderNodes, builderTargets, parseValue, TOKEN_PLACEHOLDER } from "./requestBuilder";

const node = (id: string, kind: string, name: string) =>
  ({
    id,
    kind,
    name,
    typeVersion: 1,
    enabled: true,
    bypass: false,
    parameters: {},
    ports: [],
  }) as unknown as Session["nodes"][number];
const edge = (id: string, sourceNode: string, destinationNode: string) => ({
  id,
  sourceNode,
  sourcePort: "out",
  destinationNode,
  destinationPort: "in",
  enabled: true,
  matrix: [1, 0, 0, 1],
});
const session = {
  id: "main",
  name: "Main",
  revision: 3,
  schemaVersion: 1,
  nodes: [
    node("siege", "physicalInput", "Siege game"),
    node("discord", "applicationCapture", "Discord"),
    node("mic", "physicalInput", "Microphone"),
    node("mix", "mixer", "Game and Chat"),
    node("duck", "duck", "Duck Siege"),
    node("gain", "gain", "Gain"),
    node("gain-2", "gain", "Gain"),
  ],
  edges: [edge("e1", "siege", "mix"), edge("e2", "discord", "mix"), edge("e3", "siege", "duck")],
} as unknown as Session;
const nodeTypes = [
  {
    type: "mixer@1",
    availability: { status: "available" },
    realtimeCostClass: "low",
    latencySamples: 0,
    parameters: [
      {
        name: "inputVolume:",
        namePattern: "inputVolume:<upstreamNodeId>",
        type: "number",
        unit: "%",
        minimum: 0,
        maximum: 100,
        default: 100,
      },
    ],
  },
  {
    type: "duck@1",
    availability: { status: "available" },
    realtimeCostClass: "low",
    latencySamples: 0,
    parameters: [
      { name: "amountDb", type: "number", unit: "dB", minimum: 0, maximum: 40, default: 6 },
      { name: "trigger", type: "string", enum: ["level", "siegeRound"], default: "level" },
      { name: "keyNodeId", type: "string", reference: "node" },
      { name: "duckMenu", type: "boolean", default: true },
    ],
  },
] as unknown as DiscoveryDocument["nodeTypes"];
const find = (targets: ReturnType<typeof builderTargets>, key: string) => targets.find((target) => target.key === key)!;

describe("request builder", () => {
  it("lists nodes by name and disambiguates repeated names", () => {
    const labels = builderNodes(session).map((item) => item.label);
    expect(labels).toContain("Siege game");
    expect(labels).toContain("Gain (gain, gain)");
    expect(labels).toContain("Gain (gain, gain-2)");
  });

  it("expands a Mixer's per-input volume into one setting per connected input", () => {
    const targets = builderTargets(session, session.nodes[3], nodeTypes);
    expect(targets.map((target) => target.label)).toEqual([
      "Enabled (on/off)",
      "Bypass",
      "Input volume: Siege game",
      "Input volume: Discord",
    ]);
    expect(find(targets, "inputVolume:siege").kind).toBe("parameter");
  });

  it("offers the Duck trigger node by name and sends its ID", () => {
    const targets = builderTargets(session, session.nodes[4], nodeTypes);
    const key = find(targets, "keyNodeId");
    expect(key.kind === "parameter" && key.choices?.map((choice) => choice.label)).toContain("Microphone");
    expect(key.kind === "parameter" && key.choices?.some((choice) => choice.value === "duck")).toBe(false);
    expect(parseValue(key, "mic")).toEqual({ value: "mic" });
    expect(parseValue(key, "nobody")).toEqual({ error: expect.stringContaining("Microphone") });
  });

  it("validates values against the catalog", () => {
    const targets = builderTargets(session, session.nodes[4], nodeTypes);
    expect(parseValue(find(targets, "amountDb"), "12")).toEqual({ value: 12 });
    expect(parseValue(find(targets, "amountDb"), "41")).toEqual({ error: "Enter a value from 0 to 40 dB." });
    expect(parseValue(find(targets, "trigger"), "siegeRound")).toEqual({ value: "siegeRound" });
    expect(parseValue(find(targets, "trigger"), "always")).toEqual({ error: "Choose one of: level, siegeRound." });
    expect(parseValue(find(targets, "duckMenu"), "false")).toEqual({ value: false });
    expect(parseValue(find(targets, "bypass"), "yes")).toEqual({ error: "Choose true or false." });
  });

  it("builds a nodes.set request that follows the active session or pins one, without the token", () => {
    const target = find(builderTargets(session, session.nodes[3], nodeTypes), "inputVolume:siege");
    const follow = buildRequest({
      baseUrl: "http://127.0.0.1:17891/",
      sessionId: null,
      nodeId: "mix",
      target,
      value: 30,
      idempotencyKey: "menu-1",
    });
    expect(follow.url).toBe("http://127.0.0.1:17891/api/v1/nodes/set");
    expect(follow.body).toEqual({ node: "mix", parameters: { "inputVolume:siege": 30 }, idempotencyKey: "menu-1" });
    const pinned = buildRequest({
      baseUrl: "http://127.0.0.1:17891",
      sessionId: "main",
      nodeId: "mix",
      target,
      value: 30,
      idempotencyKey: "menu-1",
    });
    expect(pinned.body.sessionId).toBe("main");
    const flag = buildRequest({
      baseUrl: "http://127.0.0.1:17891",
      sessionId: null,
      nodeId: "duck",
      target: { kind: "flag", key: "bypass", label: "Bypass" },
      value: true,
      idempotencyKey: "k",
    });
    expect(flag.body).toEqual({ node: "duck", bypass: true, idempotencyKey: "k" });
    for (const text of [follow.curl, follow.powershell]) {
      expect(text).toContain(`Bearer ${TOKEN_PLACEHOLDER}`);
      expect(text).toContain('"inputVolume:siege":30');
    }
  });
});
