import { afterEach, describe, expect, it, vi } from "vitest";
import type { Fetch } from "../src/api.js";
import { formatElapsed, recordFace } from "../src/faces.js";
import { recorderFor } from "../src/recorders.js";
import { AudioRouterStore } from "../src/store.js";

/** A fake AudioRouter with Recorder nodes, using the real response shapes. */
function fakeAudioRouter(recorders: { id: string; name: string }[]) {
  const calls: string[] = [];
  const states = new Map<string, string>();
  const fetcher: Fetch = async (url, init) => {
    const method = url.split("/api/v1/")[1];
    const body = JSON.parse(init.body ?? "{}");
    calls.push(method);
    const reply = (value: unknown) => ({ ok: true, status: 200, json: async () => value, text: async () => JSON.stringify(value) });
    switch (method) {
      case "nodes/catalog": return reply([]);
      case "sessions/summary": return reply({ sessionId: "s1", name: "Gaming", revision: 1, playing: true, privacyMuted: false, nodes: [
        { id: "voice", name: "Voice", kind: "gain", enabled: true, bypass: false },
        ...recorders.map((item) => ({ ...item, kind: "recorder", enabled: true, bypass: false })),
      ] });
      case "sessions/get": return reply({ id: "s1", revision: 1, nodes: [] });
      case "recorders/list": return reply([
        ...[...states].map(([nodeId, state]) => ({ sessionId: "s1", nodeId, state, lastFrame: null })),
        { sessionId: "other", nodeId: "rec", state: "recording", lastFrame: null },
      ]);
      default: throw new Error(`unexpected ${method} ${JSON.stringify(body)}`);
    }
  };
  return { fetcher, calls, states };
}

const settle = async (ms: number) => { await vi.advanceTimersByTimeAsync(ms); };

afterEach(() => { vi.useRealTimers(); });

describe("Record key", () => {
  it("reads recorder states only while a Record key shows, for the selected session only", async () => {
    vi.useFakeTimers();
    const api = fakeAudioRouter([{ id: "rec", name: "Clip recorder" }]);
    const store = new AudioRouterStore(api.fetcher);
    store.configure({ baseUrl: "http://127.0.0.1:1", token: "t" });
    await settle(1100);
    expect(api.calls).not.toContain("recorders/list");
    expect(store.isRecording("rec")).toBe(false);

    store.watchRecorders(1);
    api.states.set("rec", "recording");
    await settle(600);
    expect(store.isRecording("rec")).toBe(true);
    expect(store.recordingSince.has("rec")).toBe(true);

    api.states.set("rec", "completed");
    await settle(600);
    expect(store.isRecording("rec")).toBe(false);
    expect(store.recordingSince.has("rec")).toBe(false);

    store.watchRecorders(-1);
    const count = api.calls.filter((call) => call === "recorders/list").length;
    await settle(1500);
    expect(api.calls.filter((call) => call === "recorders/list").length).toBe(count);
    store.configure(null);
  });

  it("uses the chosen Recorder, else the session's only one", async () => {
    vi.useFakeTimers();
    const one = new AudioRouterStore(fakeAudioRouter([{ id: "rec", name: "Clips" }]).fetcher);
    one.configure({ baseUrl: "http://127.0.0.1:1", token: "t" });
    await settle(100);
    expect(recorderFor(one, {})?.id).toBe("rec");
    expect(recorderFor(one, { nodeId: "voice" })).toBeUndefined();
    expect(recorderFor(one, { nodeId: "gone", nodeName: "Clips" })?.id).toBe("rec");
    one.configure(null);

    const two = new AudioRouterStore(fakeAudioRouter([{ id: "a", name: "Game" }, { id: "b", name: "Voice" }]).fetcher);
    two.configure({ baseUrl: "http://127.0.0.1:1", token: "t" });
    await settle(100);
    expect(recorderFor(two, {}), "two Recorders: the key must name one").toBeUndefined();
    expect(recorderFor(two, { nodeId: "b" })?.name).toBe("Voice");
    two.configure(null);
  });

  it("shows the running time while recording and asks for Play when stopped", () => {
    expect(formatElapsed(0)).toBe("0:00");
    expect(formatElapsed(83)).toBe("1:23");
    expect(formatElapsed(3725)).toBe("1:02:05");
    expect(recordFace({ label: "Clips", recording: true, elapsed: "1:23", canStart: true, pending: false })).toContain("REC 1:23");
    expect(recordFace({ label: "Clips", recording: false, elapsed: "0:00", canStart: true, pending: false })).toContain("RECORD");
    expect(recordFace({ label: null, recording: false, elapsed: "0:00", canStart: false, pending: false })).toContain("PLAY FIRST");
  });
});
