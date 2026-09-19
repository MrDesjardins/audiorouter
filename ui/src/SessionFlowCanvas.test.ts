/** @vitest-environment jsdom */

import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { createElement } from "react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { demoSession } from "./fixtures";
import { deletedConnectionIds, deletedNodeIds, eqBandCoordinates, libraryDropPosition, telemetrySignalActive } from "./SessionFlowCanvas";
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
  it("maps EQ bands across the audible frequency range", () => {
    expect(eqBandCoordinates(20, 0).x).toBeCloseTo(8);
    expect(eqBandCoordinates(20_000, 0).x).toBeCloseTo(192);
    expect(eqBandCoordinates(1_000, 12).y).toBeLessThan(eqBandCoordinates(1_000, -12).y);
  });

  it("only marks a link active for a real bounded meter signal", () => {
    expect(telemetrySignalActive({ nodeId: "meter", kind: "meter", meter: null, processor: null })).toBe(false);
    expect(telemetrySignalActive({ nodeId: "meter", kind: "meter", meter: { peakDb: -60, rmsDb: -70, clippedSamples: 0, channelPeakDb: [], channelRmsDb: [], channelClippedSamples: [] }, processor: null })).toBe(false);
    expect(telemetrySignalActive({ nodeId: "meter", kind: "meter", meter: { peakDb: -12, rmsDb: -18, clippedSamples: 0, channelPeakDb: [], channelRmsDb: [], channelClippedSamples: [] }, processor: null })).toBe(true);
  });

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

  it("rejects stale or malformed library drop kinds before backend mutation", () => {
    const onAddLibraryNode = vi.fn(() => "invalid-1");
    const onConnect = vi.fn();
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect,
      onAddLibraryNode,
    }));
    const canvas = getByLabelText("Signal-flow graph");
    const drop = new Event("drop", { bubbles: true });
    Object.defineProperty(drop, "dataTransfer", {
      value: {
        types: ["application/x-audiorouter-library-kind"],
        getData: () => "stale-processor-kind",
      },
    });

    fireEvent(canvas, drop);

    expect(onAddLibraryNode).not.toHaveBeenCalled();
    expect(onConnect).not.toHaveBeenCalled();
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

  it("offers physical and virtual endpoint nodes in the drag shelf", () => {
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode: vi.fn(() => "processor-1"),
    }));
    const shelf = within(getByLabelText("Drag processors to canvas"));

    expect(shelf.getByRole("button", { name: "Gain" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Physical input" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Physical output" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Existing virtual output" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Virtual capture sink" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Virtual render source" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Recorder" })).toBeTruthy();
  });

  it("keeps deferred managed virtual entries visible but unavailable", () => {
    const onConnect = vi.fn();
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect,
      onAddLibraryNode: vi.fn(() => "processor-1"),
    }));
    const shelf = within(getByLabelText("Drag processors to canvas"));
    expect(shelf.getByRole("button", { name: "Virtual capture sink" })).toHaveProperty("disabled", true);
    expect(shelf.getByRole("button", { name: "Virtual render source" })).toHaveProperty("disabled", true);
    expect(onConnect).not.toHaveBeenCalled();
  });

  it.each([
    ["Physical input", "physicalInput"],
    ["Physical output", "physicalOutput"],
    ["Mixer", "mixer"],
    ["Compressor", "compressor"],
    ["Recorder", "recorder"],
  ])("places %s from the shelf through the draft adapter", (label, kind) => {
    const onAddLibraryNode = vi.fn(() => `${kind}-1`);
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode,
    }));
    const canvas = getByLabelText("Signal-flow graph");
    vi.spyOn(canvas, "getBoundingClientRect").mockReturnValue({
      left: 100, top: 50, right: 900, bottom: 650, width: 800, height: 600,
      x: 100, y: 50, toJSON: () => ({}),
    });
    const shelf = within(getByLabelText("Drag processors to canvas"));
    const button = shelf.getByRole("button", { name: label });
    const dataTransfer = {
      types: ["application/x-audiorouter-library-kind"],
      setData: vi.fn(),
      getData: vi.fn(() => kind),
    };
    fireEvent.dragStart(button, { dataTransfer });
    const drop = new Event("drop", { bubbles: true });
    Object.defineProperties(drop, {
      clientX: { value: 240 },
      clientY: { value: 180 },
      dataTransfer: { value: dataTransfer },
    });
    fireEvent(canvas, drop);

    expect(onAddLibraryNode).toHaveBeenCalledWith(kind, { x: 120, y: 110 });
    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.layout.demo-session") ?? "null")).toMatchObject({
      [`${kind}-1`]: { x: 120, y: 110 },
    });
  });

  it("retains the drop position when the app creates a virtual bus node", () => {
    window.localStorage.clear();
    const onConnect = vi.fn(() => "virtual-capture-sink-1");
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect,
      onAddLibraryNode: vi.fn(),
    }));
    const canvas = getByLabelText("Signal-flow graph");
    vi.spyOn(canvas, "getBoundingClientRect").mockReturnValue({
      left: 100, top: 50, right: 900, bottom: 650, width: 800, height: 600,
      x: 100, y: 50, toJSON: () => ({}),
    });
    const drop = new Event("drop", { bubbles: true });
    Object.defineProperties(drop, {
      clientX: { value: 240 },
      clientY: { value: 180 },
      dataTransfer: { value: { types: ["application/x-audiorouter-library-kind"], getData: () => "virtualCaptureSink" } },
    });
    fireEvent(canvas, drop);

    expect(onConnect).toHaveBeenCalledWith(expect.objectContaining({ sourceHandle: "virtualCaptureSink" }), { x: 120, y: 110 });
    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.layout.demo-session") ?? "null")).toEqual({
      "virtual-capture-sink-1": { x: 120, y: 110 },
    });
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
