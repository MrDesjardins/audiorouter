import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError, AudioRouterClient, normalizeBaseUrl, type Fetch } from "../src/api.js";
import { AudioRouterStore, toggleTargets, type CatalogKind } from "../src/store.js";

/** A fake AudioRouter HTTP API with the real response shapes. */
function fakeAudioRouter() {
  const calls: { url: string; body: Record<string, unknown>; auth: string }[] = [];
  let online = true;
  let revision = 3;
  const state = { voiceEnabled: true, privacyMuted: false, switchSelected: "a" };
  const catalog: CatalogKind[] = [
    { kind: "gain", name: "Gain", parameters: [{ name: "gainDb", type: "number", default: 0 }] },
    { kind: "mute", name: "Mute", parameters: [{ name: "muted", type: "boolean", default: false }] },
    { kind: "inputSwitch", name: "Input Switch", parameters: [{ name: "selected", type: "string", enum: ["a", "b"], default: "a" }] },
  ];
  const fetcher: Fetch = async (url, init) => {
    const body = JSON.parse(init.body ?? "{}");
    calls.push({ url, body, auth: init.headers.Authorization });
    if (!online) throw new TypeError("fetch failed");
    const method = url.split("/api/v1/")[1];
    const reply = (value: unknown, status = 200) => ({ ok: status < 400, status, json: async () => value, text: async () => JSON.stringify(value) });
    switch (method) {
      case "nodes/catalog": return reply(catalog);
      case "sessions/summary": return reply({ sessionId: "s1", name: "Gaming", revision, playing: true, privacyMuted: state.privacyMuted, nodes: [
        { id: "voice", name: "Voice gain", kind: "gain", enabled: state.voiceEnabled, bypass: false },
        { id: "switch", name: "Game or chat", kind: "inputSwitch", enabled: true, bypass: false },
      ] });
      case "sessions/get": return reply({ id: "s1", revision, nodes: [{ id: "voice", kind: "gain", parameters: { gainDb: 3 } }, { id: "switch", kind: "inputSwitch", parameters: { selected: state.switchSelected } }] });
      case "meters/levels": return reply({ sessionId: "s1", playing: true, levels: [{ nodeId: "voice", name: "Voice gain", peakDb: -6, rmsDb: -18, clipped: false, reductionDb: null, active: null }] });
      case "nodes/toggle": state.voiceEnabled = !state.voiceEnabled; revision += 1; return reply({ detail: { value: state.voiceEnabled } });
      case "safety/togglePrivacyMute": state.privacyMuted = !state.privacyMuted; return reply({ muted: state.privacyMuted });
      default: return reply({ error: { message: `unknown ${method}` } }, 400);
    }
  };
  return { fetcher, calls, state, setOnline: (value: boolean) => { online = value; } };
}

const settle = async (ms: number) => { await vi.advanceTimersByTimeAsync(ms); };

afterEach(() => { vi.useRealTimers(); });

describe("connection", () => {
  it("accepts only a plain http(s) address", () => {
    expect(normalizeBaseUrl(" http://127.0.0.1:47820/api ")).toBe("http://127.0.0.1:47820");
    expect(normalizeBaseUrl("127.0.0.1:47820")).toBeNull();
    expect(normalizeBaseUrl("ftp://x")).toBeNull();
  });

  it("sends the token and reports AudioRouter's own error message", async () => {
    const { fetcher, calls } = fakeAudioRouter();
    const client = new AudioRouterClient({ baseUrl: "http://127.0.0.1:1", token: "secret" }, fetcher);
    await client.call("nodes.catalog");
    expect(calls[0]).toMatchObject({ url: "http://127.0.0.1:1/api/v1/nodes/catalog", auth: "Bearer secret" });
    await expect(client.call("nope.method")).rejects.toThrow("unknown nope/method");
    await expect(client.call("bad")).rejects.toBeInstanceOf(ApiError);
  });
});

describe("store", () => {
  it("lists only switchable targets: Enabled, Bypass, booleans and two-choice settings", () => {
    const { fetcher } = fakeAudioRouter();
    void fetcher;
    expect(toggleTargets({ kind: "gain", name: "Gain", parameters: [{ name: "gainDb", type: "number" }] })).toEqual(["enabled", "bypass"]);
    expect(toggleTargets({ kind: "mute", name: "Mute", parameters: [{ name: "muted", type: "boolean" }] })).toEqual(["enabled", "bypass", "muted"]);
    expect(toggleTargets({ kind: "x", name: "X", parameters: [{ name: "mode", enum: ["a", "b", "c"] }, { name: "side", enum: ["l", "r"] }] })).toEqual(["enabled", "bypass", "side"]);
  });

  it("follows the selected session, reads raw settings, and confirms a press quickly", async () => {
    vi.useFakeTimers();
    const audio = fakeAudioRouter();
    const store = new AudioRouterStore(audio.fetcher);
    store.configure({ baseUrl: "http://127.0.0.1:1", token: "t" });
    await settle(10);
    expect(store.state).toBe("online");
    const voice = store.node("voice")!;
    expect(store.setting(voice, "enabled")).toBe(true);
    expect(store.setting(voice, "gainDb")).toBe(3);
    expect(store.setting(store.node("", "Game or chat")!, "selected")).toBe("a");
    await store.call("nodes.toggle", { node: "voice", target: "enabled", idempotencyKey: "k" });
    store.refreshSoon();
    await settle(50);
    expect(store.setting(store.node("voice")!, "enabled")).toBe(false);
    store.configure(null);
  });

  it("polls levels only while a live key is visible", async () => {
    vi.useFakeTimers();
    const audio = fakeAudioRouter();
    const store = new AudioRouterStore(audio.fetcher);
    store.configure({ baseUrl: "http://127.0.0.1:1", token: "t" });
    await settle(1000);
    const levelCalls = () => audio.calls.filter((call) => call.url.endsWith("meters/levels")).length;
    expect(levelCalls()).toBe(0);
    store.watchLevels(1);
    await settle(1000);
    expect(levelCalls()).toBeGreaterThanOrEqual(8);
    expect(store.levels.get("voice")?.rmsDb).toBe(-18);
    store.watchLevels(-1);
    const after = levelCalls();
    await settle(1000);
    expect(levelCalls()).toBe(after);
    store.configure(null);
  });

  it("goes offline, backs off, and recovers by itself", async () => {
    vi.useFakeTimers();
    const audio = fakeAudioRouter();
    const store = new AudioRouterStore(audio.fetcher);
    audio.setOnline(false);
    store.configure({ baseUrl: "http://127.0.0.1:1", token: "t" });
    await settle(10);
    expect(store.state).toBe("offline");
    expect(store.error).toContain("not reachable");
    const attempts = audio.calls.length;
    await settle(900);
    expect(audio.calls.length).toBe(attempts);
    audio.setOnline(true);
    await settle(3500);
    expect(store.state).toBe("online");
    store.configure(null);
  });
});
