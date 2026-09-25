import { describe, expect, it } from "vitest";
import { demoSession } from "./fixtures";
import { mergeSessionInventory, reconcileSessionDraft, sameSessionDraft } from "./sessionInventory";

describe("session inventory merge", () => {
  it("keeps the latest revision over stale snapshots and creation responses", () => {
    const latest = { ...demoSession, revision: 9 };
    expect(mergeSessionInventory([latest], { ...demoSession, revision: 7 }, [demoSession])).toEqual([latest]);
  });

  it("ignores JSON property ordering when comparing backend graphs", () => {
    const reordered = Object.fromEntries(Object.entries(demoSession).reverse()) as typeof demoSession;
    expect(sameSessionDraft(demoSession, reordered)).toBe(true);
  });

  it("retains a local committed revision until inventory catches up", () => {
    const committed = { ...demoSession, revision: 9 };
    expect(mergeSessionInventory([demoSession], demoSession, [committed])).toEqual([committed]);
  });

  it("preserves dirty drafts when telemetry returns a fresh unchanged session object", () => {
    const draft = { ...demoSession, name: "Unsaved edit" };
    expect(reconcileSessionDraft(draft, demoSession, structuredClone(demoSession))).toBe("unchanged");
    expect(reconcileSessionDraft(draft, demoSession, { ...demoSession, revision: demoSession.revision + 1 })).toBe("conflict");
    expect(reconcileSessionDraft(demoSession, demoSession, { ...demoSession, revision: demoSession.revision + 1 })).toBe("adopt");
  });
  it("prefers the point-in-time snapshot over a stale listed copy", () => {
    const listed = { ...demoSession, name: "stale", revision: 2 };
    const snapshot = { ...demoSession, name: "current", revision: 3 };
    expect(mergeSessionInventory([listed], snapshot, [])).toEqual([snapshot]);
  });

  it("preserves other listed sessions and gives local creations precedence", () => {
    const other = { ...demoSession, id: "other", name: "Other" };
    const created = { ...demoSession, id: "created", name: "Created" };
    expect(mergeSessionInventory([demoSession, other], null, [created])).toEqual([demoSession, other, created]);
  });
});
