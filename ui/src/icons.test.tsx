/** @vitest-environment jsdom */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Icon, libraryEntryIcon, NODE_KIND_ICONS } from "./icons";
import { ICON_SHAPES } from "./iconShapes";
import { libraryEntries } from "./library";

describe("icon set", () => {
  it("every kind and every Tools entry resolves to a real drawing", () => {
    for (const name of Object.values(NODE_KIND_ICONS)) expect(ICON_SHAPES[name], name).toBeTruthy();
    for (const entry of libraryEntries) {
      const name = libraryEntryIcon(entry);
      expect(name, entry.id).not.toBe("circle-dashed");
      expect(ICON_SHAPES[name], entry.id).toBeTruthy();
    }
  });

  it("two different tools never look the same", () => {
    const names = Object.values(NODE_KIND_ICONS);
    expect(new Set(names).size).toBe(names.length);
  });

  it("draws in the surrounding colour and stays out of the accessibility tree", () => {
    const { container } = render(<Icon name="mic" size={20} />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("stroke")).toBe("currentColor");
    expect(svg.getAttribute("aria-hidden")).toBe("true");
    expect(svg.getAttribute("width")).toBe("20");
    expect(svg.children.length).toBeGreaterThan(0);
  });

  it("holds only static SVG shapes, with the Lucide licence beside it", () => {
    for (const [name, shape] of Object.entries(ICON_SHAPES)) {
      expect(shape, name).not.toMatch(/<script|on\w+=|href=|url\(/i);
      expect(shape, name).toMatch(/^<(path|circle|rect|line|polyline|polygon|ellipse)\b/);
    }
    const licence = readFileSync(resolve(__dirname, "icons-LICENSE.txt"), "utf8");
    expect(licence).toContain("ISC License");
    expect(licence).toContain("Lucide");
  });
});
