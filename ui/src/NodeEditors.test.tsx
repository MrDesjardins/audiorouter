/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import type { DiagnosticsSnapshot, DiscoveryDocument, Node } from "@audiorouter/contracts";
import { NodeTelemetryPanel, ProcessorParameterEditor } from "./NodeEditors";

afterEach(cleanup);

const node = (kind: Node["kind"], parameters: Node["parameters"] = {}): Node => ({
  id: `${kind}-1`,
  kind,
  typeVersion: 1,
  name: kind,
  enabled: true,
  bypass: false,
  parameters,
  ports: [{ name: "out", direction: "output", channels: 2 }],
});

const nodeType = (
  kind: Node["kind"],
  parameters: DiscoveryDocument["nodeTypes"][number]["parameters"],
): DiscoveryDocument["nodeTypes"][number] => ({
  type: `${kind}@1`,
  availability: { status: "available" },
  realtimeCostClass: "io",
  latencySamples: 0,
  parameters,
});

// The catalog now describes device and source settings for API and MCP
// clients; the Properties panel keeps its dedicated fields for them.
it("does not read the English audio reason to label a running tool busy", () => {
  // "in use" in the backend sentence used to choose the Device busy pill; the
  // status is a structured fact, so live readings decide it instead.
  const snapshot = {
    audio: { state: "available", reason: "audio endpoint is in use by another application" },
    applicationCaptureStates: [],
    nodeTelemetry: [{ nodeId: "gain-1", meter: { currentPeakDb: -12, peakDb: -10, rmsDb: -18, clippedSamples: 0 } }],
  } as unknown as DiagnosticsSnapshot;
  const { container } = render(<NodeTelemetryPanel node={node("gain")} snapshot={snapshot} running />);
  const status = container.querySelector(".node-telemetry-status");
  expect(status?.textContent).toContain("Live");
  expect(status?.textContent).not.toContain("Device busy");
});

describe("generic settings editor", () => {
  it("gives boolean properties a stable name and an explicit state description", () => {
    const props = {
      node: node("mute", { muted: false }),
      processors: [],
      nodeTypes: [nodeType("mute", [{ name: "muted", type: "boolean" }])],
      connected: true,
      onChange: vi.fn(),
    };
    const view = render(<ProcessorParameterEditor {...props} />);
    const state = () =>
      document.getElementById(screen.getByRole("checkbox", { name: "Muted" }).getAttribute("aria-describedby") ?? "")
        ?.textContent;
    expect(state()).toBe("Off");
    view.rerender(<ProcessorParameterEditor {...props} node={node("mute", { muted: true })} />);
    expect(state()).toBe("On");
  });
  it("adds no controls for device and source kinds the catalog describes", () => {
    const nodeTypes = [
      nodeType("physicalInput", [
        { name: "endpointId", type: "string", maxLength: 128 },
        { name: "channelMode", type: "string", enum: ["stereo", "mono", "left", "right"] },
        { name: "spatialRoomPercent", type: "number", unit: "%", minimum: 0, maximum: 100 },
      ]),
      nodeType("networkReceive", [{ name: "bufferMs", type: "number", unit: "ms", minimum: 10, maximum: 500 }]),
      nodeType("audioFile", [{ name: "loop", type: "boolean" }]),
    ];
    for (const kind of ["physicalInput", "networkReceive", "audioFile"] as const) {
      const { container } = render(
        <ProcessorParameterEditor
          node={node(kind)}
          processors={[]}
          nodeTypes={nodeTypes}
          connected
          onChange={vi.fn()}
        />,
      );
      expect(container.innerHTML).toBe("");
      cleanup();
    }
  });

  it("never renders a parameter family as a literal setting", () => {
    render(
      <ProcessorParameterEditor
        node={node("testSignal")}
        processors={[]}
        nodeTypes={[
          nodeType("testSignal", [
            { name: "frequencyHz", type: "number", unit: "Hz", minimum: 20, maximum: 20000, default: 440 },
            { name: "slot<N>GainDb", namePattern: "slot<N>GainDb", type: "number", minimum: -24, maximum: 6 },
          ]),
        ]}
        connected
        onChange={vi.fn()}
      />,
    );
    expect(screen.queryAllByRole("slider")).toHaveLength(1);
    expect(document.body.textContent).not.toContain("slot");
  });
});
