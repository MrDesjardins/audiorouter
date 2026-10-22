import { describe, expect, it } from "vitest";
import {
  clearLayout,
  readLayout,
  writeLayout,
  readNodeSizes,
  writeNodeSizes,
  readEdgeLayout,
  writeEdgeLayout,
} from "./layout";

function storage() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
    removeItem: (key: string) => {
      values.delete(key);
    },
  };
}

describe("canvas layout persistence", () => {
  it("normalizes writes and validates entity keys without treating prototypes as records", () => {
    const state = storage();
    const extra = { tool: { x: 1, y: 2, id: "forged" } };
    writeLayout(state, "positions", extra);
    expect(state.getItem("positions")).toBe('{"tool":{"x":1,"y":2}}');
    const good = { width: 400, height: 300, minWidth: 218, minHeight: 150 };
    writeNodeSizes(state, "sizes", { tool: { ...good, id: "forged" } } as Record<string, typeof good>);
    expect(readNodeSizes(state, "sizes")).toEqual({ tool: good });
    state.setItem(
      "positions",
      JSON.stringify({
        "": { x: 1, y: 2 },
        ["x".repeat(129)]: { x: 1, y: 2 },
        ["x".repeat(128)]: { x: 3, y: 4 },
        ["\u00e9".repeat(65)]: { x: 5, y: 6 },
      }),
    );
    expect(Object.keys(readLayout(state, "positions"))).toEqual(["x".repeat(128)]);
    state.setItem("positions", '{"__proto__":{"x":1,"y":2,"id":"forged"},"constructor":{"x":3,"y":4}}');
    const result = readLayout(state, "positions");
    expect(Object.hasOwn(result, "__proto__")).toBe(true);
    expect(result.__proto__).toEqual({ x: 1, y: 2 });
    expect(Object.hasOwn(result, "constructor")).toBe(true);
  });
  it("contains storage faults across all presentation codecs", () => {
    const unavailable = {
      getItem: () => {
        throw Error("read");
      },
      setItem: () => {
        throw Error("quota");
      },
      removeItem: () => {
        throw Error("remove");
      },
    };
    expect(readLayout(unavailable, "key")).toEqual({});
    expect(readNodeSizes(unavailable, "key")).toEqual({});
    expect(readEdgeLayout(unavailable, "key")).toEqual({});
    expect(() => writeLayout(unavailable, "key", {})).not.toThrow();
    expect(() => writeNodeSizes(unavailable, "key", {})).not.toThrow();
    expect(() => writeEdgeLayout(unavailable, "key", {})).not.toThrow();
    expect(() => clearLayout(unavailable, "key")).not.toThrow();
  });
  it("decodes only coordinates so saved fields cannot replace a graph identity", () => {
    const state = storage();
    state.setItem("session", JSON.stringify({ tool: { x: 1, y: 2, id: "forged", group: true, width: 9999 } }));
    expect(readLayout(state, "session")).toEqual({ tool: { x: 1, y: 2 } });
  });
  it("round-trips bounded presentation positions", () => {
    const state = storage();
    writeLayout(state, "session", { mic: { x: 10, y: -20 }, gain: { x: 30, y: 40 } });
    expect(readLayout(state, "session")).toEqual({ mic: { x: 10, y: -20 }, gain: { x: 30, y: 40 } });
  });

  it("drops malformed or unbounded positions without throwing", () => {
    const state = storage();
    state.setItem(
      "session",
      JSON.stringify({ good: { x: 1, y: 2 }, bad: { x: Infinity, y: 0 }, huge: { x: 100001, y: 0 }, text: "bad" }),
    );
    expect(readLayout(state, "session")).toEqual({ good: { x: 1, y: 2 } });
    expect(() => readLayout(state, "missing")).not.toThrow();
  });

  it("clears only the selected session layout", () => {
    const state = storage();
    writeLayout(state, "session", { mic: { x: 1, y: 2 } });
    clearLayout(state, "session");
    expect(readLayout(state, "session")).toEqual({});
  });
  it("roundtrips usable sizes while rejecting malformed, inverted or excessive dimensions", () => {
    const state = storage();
    const good = { width: 500, height: 300, minWidth: 218, minHeight: 170 };
    writeNodeSizes(state, "sizes", { gain: good });
    expect(readNodeSizes(state, "sizes")).toEqual({ gain: good });
    state.setItem(
      "sizes",
      JSON.stringify({
        gain: good,
        bad: { ...good, width: 100 },
        huge: { ...good, height: 10001 },
        text: { ...good, width: "500" },
      }),
    );
    expect(readNodeSizes(state, "sizes")).toEqual({ gain: good });
  });
});

describe("connector presentation persistence", () => {
  it.each(["null", "[]", "false", "3", '"text"', "{"])("ignores invalid saved container %s", (value) => {
    const state = storage();
    state.setItem("edges", value);
    expect(readEdgeLayout(state, "edges")).toEqual({});
  });
  it("retains only complete allowed sides and roundtrips valid values", () => {
    const state = storage();
    state.setItem(
      "edges",
      JSON.stringify({
        good: { source: "bottom", target: "top" },
        null: null,
        partial: { source: "left" },
        wrong: { source: "outside", target: "right" },
        array: [],
      }),
    );
    expect(readEdgeLayout(state, "edges")).toEqual({ good: { source: "bottom", target: "top" } });
    writeEdgeLayout(state, "edges", { link: { source: "left", target: "right" } });
    expect(readEdgeLayout(state, "edges")).toEqual({ link: { source: "left", target: "right" } });
  });
  it("keeps storage failures optional", () => {
    const broken = {
      getItem: () => {
        throw Error("unavailable");
      },
      setItem: () => {
        throw Error("quota");
      },
    };
    expect(readEdgeLayout(broken, "edges")).toEqual({});
    expect(() => writeEdgeLayout(broken, "edges", {})).not.toThrow();
  });
});
