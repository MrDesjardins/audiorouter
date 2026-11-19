import { describe, expect, it } from "vitest";
import { pseudolocalize, pseudolocalizeMessages } from "./pseudo";

describe("pseudolocale", () => {
  it("accents letters, expands width and keeps placeholders and markup", () => {
    const value = pseudolocalize("Delete session {{name}}?");
    expect(value).toContain("{{name}}");
    expect(value).not.toContain("Delete");
    expect([...value].length).toBeGreaterThan("Delete session {{name}}?".length);
  });

  it("walks every nested message", () => {
    const result = pseudolocalizeMessages({ a: { b: "Save" }, count: 2 });
    expect(result).toEqual({ a: { b: expect.not.stringContaining("Save") }, count: 2 });
  });
});
