/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AdvancedEqEditor, layoutPointCallouts } from "./AdvancedEqEditor";
import { appendLibraryNode } from "./draft";
import { demoSession } from "./fixtures";
import type { UiBackend } from "./backend";

afterEach(cleanup);

describe("Advanced EQ editor", () => {
  it("zooms the graph to ±12 dB and separates dense numbered points with leaders", () => {
    const node = appendLibraryNode(demoSession, "parametricEq").nodes.at(-1)!;
    node.parameters = { ...node.parameters };
    for (let index = 0; index < 16; index += 1) {
      node.parameters[`band${index}Enabled`] = true;
      node.parameters[`band${index}Type`] = "peaking";
      node.parameters[`band${index}FrequencyHz`] = 1000;
      node.parameters[`band${index}GainDb`] = index % 2 === 0 ? 6 : -6;
      node.parameters[`band${index}Q`] = 1;
    }
    const view = render(<AdvancedEqEditor node={node} backend={{ processorResponse: vi.fn() } as unknown as UiBackend} connected onChange={vi.fn()} />);
    const graph = view.getByRole("img", { name: "EQ frequency response and movable filter points" });
    const axisLabels = [...graph.querySelectorAll(".advanced-eq-axis")].map((label) => label.textContent);
    expect(axisLabels).toContain("+12");
    expect(axisLabels).toContain("-12");
    expect(axisLabels).not.toContain("+24");
    expect(graph.querySelectorAll(".advanced-eq-leader")).toHaveLength(16);
    expect(graph.querySelectorAll(".advanced-eq-anchor")).toHaveLength(16);
    expect(graph.querySelectorAll(".advanced-eq-label-circle")).toHaveLength(16);

    const bubbles = [...graph.querySelectorAll<SVGCircleElement>(".advanced-eq-label-circle")];
    for (let left = 0; left < bubbles.length; left += 1) {
      for (let right = left + 1; right < bubbles.length; right += 1) {
        const dx = Number(bubbles[left].getAttribute("cx")) - Number(bubbles[right].getAttribute("cx"));
        const dy = Number(bubbles[left].getAttribute("cy")) - Number(bubbles[right].getAttribute("cy"));
        expect(Math.hypot(dx, dy)).toBeGreaterThanOrEqual(18);
      }
    }
  });

  it("places same-frequency callouts without overlaps and within the chart gutters", () => {
    const points = Array.from({ length: 16 }, (_, index) => ({ index, x: 200, y: 103 }));
    const callouts = layoutPointCallouts(points);
    expect(callouts).toHaveLength(points.length);
    for (const point of callouts) {
      expect(point.labelX).toBeGreaterThanOrEqual(43);
      expect(point.labelX).toBeLessThanOrEqual(357);
      expect([8, 27, 198, 217]).toContain(point.labelY);
    }
    for (let left = 0; left < callouts.length; left += 1) {
      for (let right = left + 1; right < callouts.length; right += 1) {
        expect(Math.hypot(callouts[left].labelX - callouts[right].labelX, callouts[left].labelY - callouts[right].labelY)).toBeGreaterThanOrEqual(18);
      }
    }
  });

  it("selects an existing point outside the graph without changing parameters", () => {
    const node = appendLibraryNode(demoSession, "parametricEq").nodes.at(-1)!;
    node.parameters = { ...node.parameters, band0Enabled: true, band0Type: "peaking", band0FrequencyHz: 1000, band2Enabled: true, band2Type: "allPass", band2FrequencyHz: 250 };
    const onChange = vi.fn();
    const backend = { processorResponse: vi.fn().mockResolvedValue({ frequenciesHz: [20], magnitudeDb: [0] }) } as unknown as UiBackend;
    const view = render(<AdvancedEqEditor node={node} backend={backend} connected onChange={onChange} />);
    expect(view.getByRole("option", { name: "Peaking/Band" })).toBeTruthy();
    fireEvent.change(view.getByLabelText("EQ point"), { target: { value: "2" } });
    expect((view.getByLabelText("EQ frequency Hz") as HTMLInputElement).value).toBe("250");
    expect((view.getByLabelText("EQ filter type") as HTMLSelectElement).value).toBe("allPass");
    expect(onChange).not.toHaveBeenCalled();
  });
  it.each(["bandPass", "allPass"])("preserves %s and requests its backend response without gain editing", async (type) => {
    const node = appendLibraryNode(demoSession, "parametricEq").nodes.at(-1)!;
    node.parameters = { ...node.parameters, band0Enabled: true, band0Type: type, band0GainDb: 12 };
    const processorResponse = vi.fn().mockResolvedValue({ frequenciesHz: [20, 20000], magnitudeDb: [0, 0] });
    const onChange = vi.fn();
    const view = render(<AdvancedEqEditor node={node} backend={{ processorResponse } as unknown as UiBackend} connected onChange={onChange} />);
    expect((view.getByLabelText("EQ filter type") as HTMLSelectElement).value).toBe(type);
    expect((view.getByLabelText("EQ gain dB") as HTMLInputElement).disabled).toBe(true);
    expect((view.getByLabelText("EQ Q width") as HTMLInputElement).disabled).toBe(false);
    fireEvent.change(view.getByLabelText("EQ filter type"), { target: { value: type === "allPass" ? "bandPass" : "allPass" } });
    expect(onChange).toHaveBeenCalledWith("band0Type", type === "allPass" ? "bandPass" : "allPass");
    await waitFor(() => expect(processorResponse).toHaveBeenCalled());
    expect(processorResponse.mock.calls.at(-1)?.[0].bands[0].type).toBe(type);
  });
  it("adds, selects, changes and removes a bounded point using node parameters", async () => {
    const node = appendLibraryNode(demoSession, "parametricEq").nodes.at(-1)!;
    const onChange = vi.fn();
    const processorResponse = vi.fn().mockResolvedValue({ frequenciesHz: [20, 20000], magnitudeDb: [0, 0] });
    const backend = { processorResponse } as unknown as UiBackend;
    const view = render(<AdvancedEqEditor node={node} backend={backend} connected onChange={onChange} />);
    fireEvent.click(view.getByRole("button", { name: "Add point" }));
    expect(onChange).toHaveBeenCalledWith("band0Enabled", true);
    expect(onChange).toHaveBeenCalledWith("band0FrequencyHz", 1000);
    const activeNode = { ...node, parameters: { ...node.parameters, band0Enabled: true } };
    view.rerender(<AdvancedEqEditor node={activeNode} backend={backend} connected onChange={onChange} />);
    fireEvent.change(view.getByLabelText("EQ filter type"), { target: { value: "notch" } });
    expect(onChange).toHaveBeenCalledWith("band0Type", "notch");
    fireEvent.change(view.getByLabelText("EQ frequency Hz"), { target: { value: "120" } });
    expect(onChange).toHaveBeenCalledWith("band0FrequencyHz", 120);
    fireEvent.click(view.getByRole("button", { name: "Remove point" }));
    expect(onChange).toHaveBeenCalledWith("band0Enabled", false);
    await waitFor(() => expect(processorResponse).toHaveBeenCalled());
    expect(processorResponse.mock.calls.at(-1)?.[0].bands).toHaveLength(16);
  });

  it("keeps editing disabled without a backend connection", () => {
    const node = appendLibraryNode(demoSession, "parametricEq").nodes.at(-1)!;
    const backend = { processorResponse: vi.fn() } as unknown as UiBackend;
    const view = render(<AdvancedEqEditor node={node} backend={backend} connected={false} onChange={vi.fn()} />);
    expect((view.getByRole("button", { name: "Add point" }) as HTMLButtonElement).disabled).toBe(true);
    expect(backend.processorResponse).not.toHaveBeenCalled();
  });
});
