/** @vitest-environment jsdom */

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { EditableNodeName } from "./EditableNodeName";

afterEach(cleanup);

describe("inline node name", () => {
  it("opens on demand, trims and commits on Enter, then removes the field", () => {
    const rename = vi.fn();
    render(<EditableNodeName name="Voice" disabled={false} onRename={rename} />);
    expect(screen.queryByLabelText("Node name")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Rename Voice" }));
    const field = screen.getByLabelText("Node name");
    fireEvent.change(field, { target: { value: "  Radio voice  " } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(rename).toHaveBeenCalledExactlyOnceWith("Radio voice");
    expect(screen.queryByLabelText("Node name")).toBeNull();
  });

  it("cancels on Escape and preserves the name after an empty edit", () => {
    const rename = vi.fn();
    render(<EditableNodeName name="Voice" disabled={false} onRename={rename} />);
    fireEvent.click(screen.getByRole("button", { name: "Rename Voice" }));
    const field = screen.getByLabelText("Node name");
    fireEvent.change(field, { target: { value: "Discard this" } });
    fireEvent.keyDown(field, { key: "Escape" });
    expect(rename).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Rename Voice" }));
    fireEvent.change(screen.getByLabelText("Node name"), { target: { value: "  " } });
    fireEvent.blur(screen.getByLabelText("Node name"));
    expect(rename).not.toHaveBeenCalled();
    expect(screen.queryByLabelText("Node name")).toBeNull();
  });
});
