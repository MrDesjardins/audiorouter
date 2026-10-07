/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FirstRunGuide } from "./FirstRunGuide";

afterEach(cleanup);

const device = (id: string, direction: "capture" | "render") => ({
  id,
  name: id,
  direction,
  state: "active" as const,
  defaultRoles: [],
  format: { sampleRateHz: 48_000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
  periods: { default100ns: 100_000, minimum100ns: 30_000 },
});

describe("FirstRunGuide", () => {
  it("explains how to recover when the PC has no active audio endpoints", () => {
    const onRefresh = vi.fn();
    render(<FirstRunGuide devices={[]} connected onRefresh={onRefresh} onOpenTools={vi.fn()} />);
    expect(screen.getByRole("status").textContent).toContain("No complete input-and-output pair");
    expect(screen.getByText(/does not install virtual audio drivers/i)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Refresh devices" }));
    expect(onRefresh).toHaveBeenCalledOnce();
  });

  it("shows discovered input/output counts and offers the next route step", () => {
    const onOpenTools = vi.fn();
    render(
      <FirstRunGuide
        devices={[device("mic", "capture"), device("headphones", "render")]}
        connected
        onRefresh={vi.fn()}
        onOpenTools={onOpenTools}
      />,
    );
    expect(screen.getByRole("status").textContent).toContain("Found 1 input and 1 output");
    fireEvent.click(screen.getByRole("button", { name: "Open Tools" }));
    expect(onOpenTools).toHaveBeenCalledOnce();
  });
});
