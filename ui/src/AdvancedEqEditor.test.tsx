/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AdvancedEqEditor } from "./AdvancedEqEditor";
import { appendLibraryNode } from "./draft";
import { demoSession } from "./fixtures";
import type { UiBackend } from "./backend";

afterEach(cleanup);

describe("Advanced EQ editor", () => {
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
