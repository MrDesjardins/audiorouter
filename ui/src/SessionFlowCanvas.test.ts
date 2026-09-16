/** @vitest-environment jsdom */

import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { createElement } from "react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { demoSession } from "./fixtures";
import { deletedConnectionIds, deletedNodeIds, libraryDropPosition } from "./SessionFlowCanvas";
import { SessionFlowCanvas } from "./SessionFlowCanvas";

beforeAll(() => {
  Object.defineProperty(globalThis, "ResizeObserver", {
    configurable: true,
    value: class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  });
});

afterEach(cleanup);

describe("canvas library drop positions", () => {
  it("converts viewport coordinates into bounded canvas coordinates", () => {
    expect(libraryDropPosition(240, 180, { left: 100, top: 50 })).toEqual({ x: 120, y: 110 });
  });

  it("falls back to the canvas origin for non-finite event coordinates", () => {
    expect(libraryDropPosition(Number.NaN, Number.POSITIVE_INFINITY, { left: 100, top: 50 })).toEqual({ x: 0, y: 0 });
  });

  it("extracts only non-empty edge identities for draft deletion", () => {
    expect(deletedConnectionIds([{ id: "edge-1" }, { id: "" }, { id: "edge-2" }])).toEqual(["edge-1", "edge-2"]);
  });

  it("extracts only non-empty node identities for draft deletion", () => {
    expect(deletedNodeIds([{ id: "node-1" }, { id: "" }, { id: "node-2" }])).toEqual(["node-1", "node-2"]);
  });

  it("routes a library drop to the backend draft callback at canvas coordinates", () => {
    const onAddLibraryNode = vi.fn(() => "compressor-1");
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode,
    }));
    const canvas = getByLabelText("Signal-flow graph");
    vi.spyOn(canvas, "getBoundingClientRect").mockReturnValue({
      left: 100,
      top: 50,
      right: 900,
      bottom: 650,
      width: 800,
      height: 600,
      x: 100,
      y: 50,
      toJSON: () => ({}),
    });

    const drop = new Event("drop", { bubbles: true });
    Object.defineProperties(drop, {
      clientX: { value: 240 },
      clientY: { value: 180 },
      dataTransfer: {
        value: {
          types: ["application/x-audiorouter-library-kind"],
          getData: () => "compressor",
        },
      },
    });
    fireEvent(canvas, drop);

    expect(onAddLibraryNode).toHaveBeenCalledWith("compressor", { x: 120, y: 110 });
  });

  it("offers a keyboard-accessible click path for adding a processor", () => {
    const onAddLibraryNode = vi.fn(() => "gate-1");
    const { getByRole } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode,
    }));

    fireEvent.click(getByRole("button", { name: "Gate" }));

    expect(onAddLibraryNode).toHaveBeenCalledWith("gate", { x: 0, y: 150 });
  });

  it("keeps unavailable source metadata out of the processor drop shelf", () => {
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode: vi.fn(() => "processor-1"),
    }));
    const shelf = within(getByLabelText("Drag processors to canvas"));

    expect(shelf.getByRole("button", { name: "Gain" })).toBeTruthy();
    expect(shelf.queryByRole("button", { name: "Physical input" })).toBeNull();
    expect(shelf.queryByRole("button", { name: "Virtual capture sink" })).toBeNull();
  });

  it("preserves both layout entries when processors are added rapidly", () => {
    window.localStorage.clear();
    let nextId = 1;
    const onAddLibraryNode = vi.fn(() => `gain-${nextId++}`);
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode,
    }));
    const shelf = within(getByLabelText("Drag processors to canvas"));

    fireEvent.click(shelf.getByRole("button", { name: "Gain" }));
    fireEvent.click(shelf.getByRole("button", { name: "Gain" }));

    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.layout.demo-session") ?? "null")).toEqual({
      "gain-1": { x: 0, y: 150 },
      "gain-2": { x: 0, y: 150 },
    });
  });
});
