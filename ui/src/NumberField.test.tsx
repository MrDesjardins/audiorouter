/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { NumberField } from "./NumberField";

afterEach(cleanup);

describe("NumberField", () => {
  it("lets a value be deleted and retyped through out-of-range steps", () => {
    const onValue = vi.fn();
    render(<NumberField aria-label="Frequency" value={1000} min={20} max={20000} onValue={onValue} />);
    const field = screen.getByRole("spinbutton", { name: "Frequency" }) as HTMLInputElement;
    fireEvent.focus(field);
    for (const text of ["100", "10", "1", ""]) {
      fireEvent.change(field, { target: { value: text } });
      expect(field.value).toBe(text);
    }
    // 100 is valid and applied; 10, 1 and "" are below 20 and wait.
    expect(onValue.mock.calls).toEqual([[100]]);
    fireEvent.change(field, { target: { value: "2" } });
    fireEvent.change(field, { target: { value: "250" } });
    expect(onValue).toHaveBeenLastCalledWith(250);
  });

  it("accepts a leading minus sign for negative values", () => {
    const onValue = vi.fn();
    render(<NumberField aria-label="Gain" value={0} min={-24} max={24} step={0.1} onValue={onValue} />);
    const field = screen.getByRole("spinbutton", { name: "Gain" }) as HTMLInputElement;
    fireEvent.focus(field);
    fireEvent.change(field, { target: { value: "-" } });
    expect(field.value).toBe("-");
    expect(onValue).not.toHaveBeenCalled();
    fireEvent.change(field, { target: { value: "-6.5" } });
    expect(onValue).toHaveBeenLastCalledWith(-6.5);
  });

  it("ignores letters, restores the saved value on leaving invalid text, and steps with arrows", () => {
    const onValue = vi.fn();
    const { rerender } = render(
      <NumberField aria-label="Q" value={1} min={0.1} max={20} step={0.1} onValue={onValue} />,
    );
    const field = screen.getByRole("spinbutton", { name: "Q" }) as HTMLInputElement;
    fireEvent.focus(field);
    fireEvent.change(field, { target: { value: "abc" } });
    expect(field.value).toBe("1");
    fireEvent.change(field, { target: { value: "0" } });
    fireEvent.blur(field);
    expect(field.value).toBe("1");
    fireEvent.focus(field);
    fireEvent.keyDown(field, { key: "ArrowUp" });
    expect(onValue).toHaveBeenLastCalledWith(1.1);
    // A new saved value shows once the field is not being edited.
    fireEvent.blur(field);
    rerender(<NumberField aria-label="Q" value={3} min={0.1} max={20} step={0.1} onValue={onValue} />);
    expect(field.value).toBe("3");
  });
});
