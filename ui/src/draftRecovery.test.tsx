/** @vitest-environment jsdom */
import type { Session } from "@audiorouter/contracts";
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { decodeStoredDraft, storedDraftAction, useDraftRecovery, type StoredDraft } from "./draftRecovery";
import { demoSession } from "./fixtures";

const saved: Session = { ...demoSession, id: "route-a", revision: 4 };
const edited: Session = { ...saved, name: "Edited" };
const stored = (overrides: Partial<StoredDraft> = {}): StoredDraft => ({
  version: 1,
  sessionId: saved.id,
  baseRevision: saved.revision,
  draft: edited,
  ...overrides,
});

describe("decodeStoredDraft", () => {
  it("keeps only the fields a route has, and the base revision", () => {
    const decoded = decodeStoredDraft({
      ...stored(),
      extra: "dropped",
      draft: { ...edited, revision: 99, extra: "dropped" },
    });
    expect(decoded?.draft.revision).toBe(saved.revision);
    expect(decoded).not.toHaveProperty("extra");
    expect(decoded?.draft).not.toHaveProperty("extra");
    expect(decoded?.draft.nodes).toEqual(edited.nodes);
  });

  it.each([
    ["nothing", null],
    ["a list", []],
    ["another format", { ...stored(), version: 2 }],
    ["no route id", { ...stored(), sessionId: "" }],
    ["a bad revision", { ...stored(), baseRevision: -1 }],
    ["a draft of another route", { ...stored(), sessionId: "route-b" }],
    ["no nodes", { ...stored(), draft: { ...edited, nodes: "none" } }],
    ["a malformed node", { ...stored(), draft: { ...edited, nodes: [{ id: "x" }] } }],
    ["a malformed connection", { ...stored(), draft: { ...edited, edges: [1] } }],
  ])("refuses %s", (_label, value) => {
    expect(decodeStoredDraft(value)).toBeNull();
  });
});

describe("storedDraftAction", () => {
  const ids = [saved.id, "route-b"];
  it("restores edits made on the revision still saved", () => {
    expect(storedDraftAction(stored(), saved, ids)).toBe("restore");
  });
  it("does not offer edits that match what is saved", () => {
    expect(storedDraftAction(stored({ draft: saved }), saved, ids)).toBe("already-saved");
  });
  it("asks when the route was saved after the edits", () => {
    expect(storedDraftAction(stored(), { ...saved, revision: 5 }, ids)).toBe("older");
  });
  it("waits for another route, and gives up on a deleted one", () => {
    expect(storedDraftAction(stored({ sessionId: "route-b" }), saved, ids)).toBe("other-route");
    expect(storedDraftAction(stored({ sessionId: "gone" }), saved, ids)).toBe("route-gone");
    expect(storedDraftAction(stored({ sessionId: "gone" }), saved, []), "list not loaded yet").toBe("other-route");
  });
});

describe("useDraftRecovery", () => {
  let file: unknown;
  let invoke: ReturnType<typeof vi.fn>;
  const handlers = () => ({
    onRestore: vi.fn(),
    onOlder: vi.fn(),
    onOtherRoute: vi.fn(),
    onFailure: vi.fn(),
  });
  const settle = () => act(async () => void (await Promise.resolve()));

  beforeEach(() => {
    vi.useFakeTimers();
    file = null;
    invoke = vi.fn(async (command: string, args?: { draft: unknown }) => {
      if (command === "get_ui_draft") return file;
      if (command === "set_ui_draft") file = args?.draft ?? null;
      return undefined;
    });
    window.__TAURI_INTERNALS__ = { invoke } as unknown as typeof window.__TAURI_INTERNALS__;
  });
  afterEach(() => {
    vi.useRealTimers();
    delete window.__TAURI_INTERNALS__;
  });

  const props = (over: Partial<Parameters<typeof useDraftRecovery>[0]> = {}) => ({
    ready: true,
    draft: saved,
    session: saved,
    unsaved: false,
    knownSessionIds: [saved.id],
    ...handlers(),
    ...over,
  });

  it("offers kept edits back only once the real route is open", async () => {
    file = stored();
    const initial = props({ ready: false });
    const view = renderHook((current) => useDraftRecovery(current), { initialProps: initial });
    await settle();
    expect(initial.onRestore).not.toHaveBeenCalled();
    const ready = { ...initial, ready: true };
    view.rerender(ready);
    await settle();
    expect(ready.onRestore).toHaveBeenCalledTimes(1);
    expect((ready.onRestore as ReturnType<typeof vi.fn>).mock.calls[0]?.[0]).toMatchObject({
      name: "Edited",
      revision: saved.revision,
    });
  });

  it("does not delete kept edits when a clean page opens", async () => {
    file = stored({ sessionId: "route-b", draft: { ...edited, id: "route-b" } });
    const initial = { ...props(), knownSessionIds: [saved.id, "route-b"] };
    renderHook((current) => useDraftRecovery(current), { initialProps: initial });
    await settle();
    await act(async () => void vi.advanceTimersByTime(2000));
    expect(initial.onOtherRoute).toHaveBeenCalledWith("route-b");
    expect(file).toMatchObject({ sessionId: "route-b" });
  });

  it("keeps a copy while edits are unsaved, and removes it once they are saved", async () => {
    const view = renderHook((current) => useDraftRecovery(current), { initialProps: props() });
    await settle();
    view.rerender(props({ draft: edited, unsaved: true }));
    await act(async () => void vi.advanceTimersByTime(399));
    expect(file, "writes are debounced").toBeNull();
    await act(async () => void vi.advanceTimersByTime(1));
    expect(file).toMatchObject({ sessionId: saved.id, baseRevision: saved.revision, draft: { name: "Edited" } });

    const after = { ...edited, revision: 5 };
    view.rerender(props({ draft: after, session: after, unsaved: false }));
    await settle();
    expect(file).toBeNull();
  });

  it("writes at once when Quit asks, ahead of the debounce", async () => {
    const view = renderHook((current) => useDraftRecovery(current), { initialProps: props() });
    await settle();
    view.rerender(props({ draft: edited, unsaved: true }));
    await act(async () => view.result.current.flush());
    expect(file).toMatchObject({ draft: { name: "Edited" } });
  });

  it("flushes nothing when there is nothing unsaved", async () => {
    file = stored({ sessionId: "route-b", draft: { ...edited, id: "route-b" } });
    const view = renderHook((current) => useDraftRecovery(current), {
      initialProps: { ...props(), knownSessionIds: [saved.id, "route-b"] },
    });
    await settle();
    await act(async () => view.result.current.flush());
    expect(file, "a quit with no edits leaves another route's copy alone").toMatchObject({ sessionId: "route-b" });
  });

  it("removes a copy for a deleted route and reports a failed write", async () => {
    file = stored({ sessionId: "gone", draft: { ...edited, id: "gone" } });
    renderHook((current) => useDraftRecovery(current), { initialProps: props() });
    await settle();
    await settle();
    expect(file).toBeNull();

    invoke.mockImplementation(async (command: string) => {
      if (command === "set_ui_draft") throw new Error("disk full");
      return null;
    });
    const failing = props();
    const view = renderHook((current) => useDraftRecovery(current), { initialProps: failing });
    await settle();
    view.rerender({ ...failing, draft: edited, unsaved: true });
    await act(async () => void vi.advanceTimersByTime(400));
    await settle();
    expect(failing.onFailure).toHaveBeenCalledWith("Keeping unsaved edits failed (Error)");
  });

  it("does nothing outside the desktop shell", async () => {
    delete window.__TAURI_INTERNALS__;
    const initial = props({ draft: edited, unsaved: true });
    const view = renderHook((current) => useDraftRecovery(current), { initialProps: initial });
    await act(async () => void vi.advanceTimersByTime(1000));
    await act(async () => view.result.current.flush());
    expect(invoke).not.toHaveBeenCalled();
    expect(initial.onFailure).not.toHaveBeenCalled();
  });
});
