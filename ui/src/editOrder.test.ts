import { describe, expect, it } from "vitest";
import { emptyEditOrder, nextEditKind, recordEdit, recordRestore } from "./editOrder";

const both = { route: true, layout: true };

describe("edit order", () => {
  it("undoes the newest change first, then the older kind", () => {
    let order = recordEdit(recordEdit(emptyEditOrder(), "route"), "layout");
    expect(nextEditKind(order, "undo", both)).toBe("layout");
    order = recordRestore(order, "layout", "undo");
    expect(nextEditKind(order, "undo", both)).toBe("route");
    order = recordRestore(order, "route", "undo");
    // Redo replays in the opposite order.
    expect(nextEditKind(order, "redo", both)).toBe("route");
    order = recordRestore(order, "route", "redo");
    expect(nextEditKind(order, "redo", both)).toBe("layout");
  });

  it("a new edit clears only its own kind's redo", () => {
    let order = recordEdit(recordEdit(emptyEditOrder(), "route"), "layout");
    order = recordRestore(recordRestore(order, "layout", "undo"), "route", "undo");
    order = recordEdit(order, "layout");
    expect(order.future).toEqual(["route"]);
  });

  it("stays right when a dedicated Undo layout button moved the layout history", () => {
    let order = recordEdit(recordEdit(recordEdit(emptyEditOrder(), "layout"), "route"), "layout");
    order = recordRestore(order, "layout", "undo");
    expect(order.past).toEqual(["layout", "route"]);
    expect(nextEditKind(order, "undo", both)).toBe("route");
  });

  it("skips a kind whose history cannot act, such as layout after a session switch", () => {
    const order = recordEdit(recordEdit(emptyEditOrder(), "route"), "layout");
    expect(nextEditKind(order, "undo", { route: true, layout: false })).toBe("route");
    expect(nextEditKind(order, "undo", { route: false, layout: false })).toBeNull();
  });

  it("still reaches a history the log has not seen", () => {
    expect(nextEditKind(emptyEditOrder(), "undo", { route: true, layout: false })).toBe("route");
    expect(nextEditKind(emptyEditOrder(), "redo", { route: false, layout: true })).toBe("layout");
  });

  it("is bounded", () => {
    let order = emptyEditOrder();
    for (let index = 0; index < 500; index += 1) order = recordEdit(order, "route");
    expect(order.past.length).toBeLessThanOrEqual(200);
  });
});
