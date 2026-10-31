/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import type { DiscoveryDocument, Node } from "@audiorouter/contracts";
import { ProcessorParameterEditor } from "./NodeEditors";

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
describe("generic settings editor", () => {
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
