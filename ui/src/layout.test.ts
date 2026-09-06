import { describe, expect, it } from "vitest";
import { readLayout, writeLayout } from "./layout";

function storage() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => { values.set(key, value); },
  };
}

describe("canvas layout persistence", () => {
  it("round-trips bounded presentation positions", () => {
    const state = storage();
    writeLayout(state, "session", { mic: { x: 10, y: -20 }, gain: { x: 30, y: 40 } });
    expect(readLayout(state, "session")).toEqual({ mic: { x: 10, y: -20 }, gain: { x: 30, y: 40 } });
  });

  it("drops malformed or unbounded positions without throwing", () => {
    const state = storage();
    state.setItem("session", JSON.stringify({ good: { x: 1, y: 2 }, bad: { x: Infinity, y: 0 }, huge: { x: 100001, y: 0 }, text: "bad" }));
    expect(readLayout(state, "session")).toEqual({ good: { x: 1, y: 2 } });
    expect(() => readLayout(state, "missing")).not.toThrow();
  });
});
