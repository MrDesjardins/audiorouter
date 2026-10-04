import { describe, expect, it } from "vitest";
import { switchSession } from "../src/switching.js";

function fake(playing: string[]) {
  const calls: string[] = [];
  const call = async (method: string, params?: Record<string, unknown>) => {
    calls.push(params?.sessionId ? `${method} ${params.sessionId}` : method);
    return method === "status.get" ? { activeSessionIds: playing } : {};
  };
  return { calls, call };
}

describe("switchSession", () => {
  it("stops the playing session and carries playback to the new one", async () => {
    const api = fake(["a"]);
    await switchSession(api.call, "b", "a", false);
    expect(api.calls).toEqual(["status.get", "session.stop a", "sessions.active.set b", "sessions.play b"]);
  });

  it("only selects when nothing plays and the key does not ask to play", async () => {
    const api = fake([]);
    await switchSession(api.call, "b", "a", false);
    expect(api.calls).toEqual(["status.get", "sessions.active.set b"]);
  });

  it("plays when the key asks to, even when nothing played", async () => {
    const api = fake([]);
    await switchSession(api.call, "b", "a", true);
    expect(api.calls).toEqual(["status.get", "sessions.active.set b", "sessions.play b"]);
  });

  it("stops another session that plays even when the selection does not change", async () => {
    const api = fake(["a"]);
    await switchSession(api.call, "b", "b", false);
    expect(api.calls).toEqual(["status.get", "session.stop a", "sessions.play b"]);
  });

  it("does nothing when the chosen session already plays", async () => {
    const api = fake(["b"]);
    await switchSession(api.call, "b", "b", true);
    expect(api.calls).toEqual(["status.get"]);
  });
});
