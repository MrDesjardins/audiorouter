import type { Session } from "@audiorouter/contracts";

/** Safe, local-only data used until a backend transport is explicitly connected. */
export const demoSession: Session = {
  id: "demo-session",
  name: "Gaming + Discord",
  schemaVersion: 1,
  revision: 7,
  nodes: [
    { id: "mic", kind: "physicalInput", name: "Microphone", enabled: true, bypass: false, parameters: {}, ports: [{ name: "out", direction: "output", channels: 1 }] },
    { id: "voice", kind: "gain", name: "Voice gain", enabled: true, bypass: false, parameters: { gainDb: 0 }, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "headphones", kind: "physicalOutput", name: "Headphones", enabled: true, bypass: false, parameters: {}, ports: [{ name: "in", direction: "input", channels: 2 }] },
  ],
  edges: [],
};

export const demoSessions: Session[] = [
  demoSession,
  { ...demoSession, id: "processed-microphone", name: "Processed microphone", revision: 2 },
  { ...demoSession, id: "desktop-recording", name: "Desktop recording", revision: 1 },
];
