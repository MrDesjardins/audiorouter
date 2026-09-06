import { describe, expect, it } from "vitest";
import { demoSession } from "./fixtures";
import { recordDraft, redoDraft, undoDraft, type DraftHistory } from "./history";

const renamed = (name: string) => ({ ...demoSession, name });

describe("draft history", () => {
  it("undoes, redoes, and clears redo after a new edit", () => {
    const first = renamed("First");
    const second = renamed("Second");
    let history: DraftHistory = { past: [], future: [] };
    history = recordDraft(history, demoSession, first);
    history = recordDraft(history, first, second);
    let transition = undoDraft(history, second);
    expect(transition.current.name).toBe("First");
    transition = redoDraft(transition.history, transition.current);
    expect(transition.current.name).toBe("Second");
    const third = renamed("Third");
    const nextHistory = recordDraft(transition.history, transition.current, third);
    expect(redoDraft(nextHistory, third).current).toBe(third);
  });

  it("bounds past history to twenty entries", () => {
    let history: DraftHistory = { past: [], future: [] };
    let current = demoSession;
    for (let index = 0; index < 25; index += 1) {
      const next = renamed(`Revision ${index}`);
      history = recordDraft(history, current, next);
      current = next;
    }
    expect(history.past).toHaveLength(20);
    expect(history.past[0].name).toBe("Revision 4");
  });
});
