/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AdvancedEqEditor } from "./AdvancedEqEditor";
import { appendLibraryNode } from "./draft";
import { demoSession } from "./fixtures";
import type { UiBackend } from "./backend";

afterEach(cleanup);

describe("Advanced EQ editor", () => {
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
