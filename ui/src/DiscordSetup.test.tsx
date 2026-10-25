/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DeviceListItem, Session } from "@audiorouter/contracts";
import { DiscordSetup } from "./DiscordSetup";
import { discordSetupOptions } from "./discordRoute";

afterEach(cleanup);

const device = (id: string, name: string, direction: "capture" | "render"): DeviceListItem => ({
  id,
  name,
  direction,
  state: "active",
  defaultRoles: [],
  format: { sampleRateHz: 48_000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
  periods: { default100ns: 100_000, minimum100ns: 30_000 },
});
const microphone = device("mic", "Microphone (PD200X Podcast Microphone)", "capture");
const cableOut = device("cable-a-out", "CABLE-A Output (VB-Audio Virtual Cable A)", "capture");
const devices = [
  microphone,
  device("webcam", "Microphone (Webcam)", "capture"),
  device("cable-a-in", "CABLE-A Input (VB-Audio Virtual Cable A)", "render"),
  cableOut,
];
const session: Session = { id: "s", name: "Session", revision: 1, schemaVersion: 1, nodes: [], edges: [] };

function setup(props: Partial<Parameters<typeof DiscordSetup>[0]> = {}) {
  const handlers = { onAdd: vi.fn(() => null as string | null), onClose: vi.fn(), onOpenVirtualCables: vi.fn() };
  render(
    <DiscordSetup options={discordSetupOptions(session, devices)} connected added={null} {...handlers} {...props} />,
  );
  return handlers;
}

describe("DiscordSetup", () => {
  it("previews the route and adds it with the chosen devices", () => {
    const { onAdd } = setup();
    expect(screen.getByRole("status").textContent).toContain(
      "Adds Microphone (PD200X Podcast Microphone) → Gate → Compressor → Limiter → CABLE-A Input. Discord's microphone: CABLE-A Output.",
    );
    fireEvent.click(screen.getByRole("checkbox"));
    expect(screen.getByRole("status").textContent).not.toContain("Gate");
    fireEvent.click(screen.getByRole("button", { name: "Add route" }));
    expect(onAdd).toHaveBeenCalledWith({ microphone: "mic", cable: "cable-a-in", cleanup: false });
  });

  it("shows a refusal in the reserved line and clears it when the choice changes", () => {
    setup({ onAdd: () => "Music already plays into CABLE-A Input. Choose another cable." });
    fireEvent.click(screen.getByRole("button", { name: "Add route" }));
    expect(screen.getByRole("status").textContent).toContain("Music already plays into");
    expect(screen.getByRole("status").classList.contains("is-error")).toBe(true);
    fireEvent.change(screen.getByLabelText("Microphone"), { target: { value: "webcam" } });
    expect(screen.getByRole("status").classList.contains("is-error")).toBe(false);
  });

  it("sends a PC without a cable to Virtual Cables", () => {
    const { onOpenVirtualCables } = setup({ options: discordSetupOptions(session, [microphone]) });
    expect(screen.queryByRole("button", { name: "Add route" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Set up a virtual cable" }));
    expect(onOpenVirtualCables).toHaveBeenCalled();
  });

  it("names Discord's microphone once the route is added", () => {
    const { onClose } = setup({ added: cableOut });
    screen.getByText("Step 2 of 2");
    screen.getByText(cableOut.name);
    screen.getByRole("button", { name: "Copy device name" });
    fireEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(onClose).toHaveBeenCalled();
  });
});
