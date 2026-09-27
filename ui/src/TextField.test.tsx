/** @vitest-environment jsdom */

import { afterEach, describe, expect, it } from "vitest";
import { useState } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { TextField } from "./TextField";

function Harness({ initial = "Mic" }: { initial?: string }) {
  const [name, setName] = useState(initial);
  return <><TextField aria-label="Name" value={name} onValue={setName} /><output>{name}</output><button onClick={() => setName("Reverted")}>revert</button></>;
}

afterEach(cleanup);

describe("TextField", () => {
  it("keeps typed spaces while storing the trimmed name", () => {
    render(<Harness />);
    const field = screen.getByLabelText("Name") as HTMLInputElement;
    fireEvent.change(field, { target: { value: "My " } });
    expect(field.value).toBe("My ");
    expect(screen.getByRole("status").textContent).toBe("My");
    fireEvent.change(field, { target: { value: "My Mic" } });
    expect(field.value).toBe("My Mic");
    expect(screen.getByRole("status").textContent).toBe("My Mic");
  });

  it("lets the field be cleared while typing and restores the name on blur", () => {
    render(<Harness />);
    const field = screen.getByLabelText("Name") as HTMLInputElement;
    fireEvent.change(field, { target: { value: "" } });
    expect(field.value).toBe("");
    expect(screen.getByRole("status").textContent).toBe("Mic");
    fireEvent.blur(field);
    expect(field.value).toBe("Mic");
  });

  it("follows a name changed elsewhere", () => {
    render(<Harness />);
    const field = screen.getByLabelText("Name") as HTMLInputElement;
    fireEvent.change(field, { target: { value: "Draft " } });
    fireEvent.click(screen.getByRole("button", { name: "revert" }));
    expect(field.value).toBe("Reverted");
  });
});
