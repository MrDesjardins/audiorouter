/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { BooleanField } from "./BooleanField";
afterEach(cleanup);

it("caption activation changes the named switch while its state stays a separate description", () => {
  const onChange = vi.fn();
  const view = render(<BooleanField label="Pitch correction" checked={false} onChange={onChange} />);
  fireEvent.click(screen.getByText("Pitch correction"));
  expect(onChange).toHaveBeenCalledExactlyOnceWith(true);
  view.rerender(<BooleanField label="Pitch correction" checked onChange={onChange} />);
  const input = screen.getByRole("checkbox", { name: "Pitch correction" });
  expect(document.getElementById(input.getAttribute("aria-describedby")!)?.textContent).toBe("On");
});
it("disabled fields do not activate from their caption", () => {
  const onChange = vi.fn();
  render(<BooleanField label="Loop" checked={false} disabled onChange={onChange} />);
  fireEvent.click(screen.getByText("Loop"));
  expect(onChange).not.toHaveBeenCalled();
});
