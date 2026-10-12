/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DiagnosticsSnapshot, VirtualCableStatus } from "@audiorouter/contracts";
import { cableStatusPill, isCableNameValid, VirtualCablePanel } from "./VirtualCablePanel";
import { CableBinding } from "./NodeEditors";
import { applyCableAvailability, libraryEntries } from "./library";
import { appendCablePassThrough, appendVirtualBusNode, setVirtualBusDraftCable } from "./draft";
import { demoSession } from "./fixtures";
import {
  cableEndpointName,
  cableLabel,
  cableUnavailableReason,
  defaultCable,
  presentCables,
  setVirtualCableStatus,
} from "./virtualCableStatus";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  setVirtualCableStatus(null);
});

function status(state: VirtualCableStatus["state"], present = [true, true]): VirtualCableStatus {
  return {
    state,
    driverVersion: state === "notInstalled" ? null : "0.1.4.0",
    protocol: state === "notInstalled" ? null : "1.1",
    defaultPeriodFrames: state === "notInstalled" ? null : 480,
    cableCount: present.length,
    cables: present.map((isPresent, index) => {
      const busId = `cable-${String.fromCharCode(97 + index)}`;
      return {
        busId,
        name: `Cable ${String.fromCharCode(65 + index)}`,
        renderEndpointId: isPresent ? `{render-${busId}}` : null,
        captureEndpointId: isPresent ? `{capture-${busId}}` : null,
        present: isPresent,
      };
    }),
  };
}

const ALL_STATES: VirtualCableStatus["state"][] = [
  "installed",
  "notInstalled",
  "needsRepair",
  "needsRestart",
  "incompatible",
];

const route: NonNullable<DiagnosticsSnapshot["cableRoute"]> = {
  sessionId: "s",
  generation: 3,
  renderBus: "cable-a",
  captureBus: "cable-b",
  blockFrames: 480,
  renderChannels: 2,
  captureChannels: 2,
  state: "running",
  failure: null,
  renderBlocks: 100,
  renderSequenceGaps: 0,
  captureBlocks: 100,
  forwarded: 12345,
  silence: 2,
  dropped: 0,
  trimmed: 1,
  engineSilentQuanta: 0,
};

describe("cable status helpers", () => {
  it("names cables and their Windows devices", () => {
    expect(cableLabel("cable-c")).toBe("Cable C");
    expect(cableLabel("bus-1")).toBe("bus-1");
    expect(cableEndpointName("cable-a", "renderSource")).toBe("Cable A Input");
    expect(cableEndpointName("cable-b", "captureSink")).toBe("Cable B Output");
  });

  it("makes cables usable only when installed with a present cable", () => {
    for (const state of ALL_STATES) {
      const reason = cableUnavailableReason(status(state));
      if (state === "installed") expect(reason).toBeNull();
      else expect(reason).toMatch(/\S/);
    }
    expect(cableUnavailableReason(null)).toMatch(/Install AudioRouter cables/);
    expect(cableUnavailableReason(status("installed", [false, false]))).not.toBeNull();
    expect(cableUnavailableReason(status("needsRestart"))).toMatch(/Restart Windows/);
  });

  it("starts sources on the first present cable and destinations on the second", () => {
    expect(defaultCable(status("installed"), "renderSource")).toBe("cable-a");
    expect(defaultCable(status("installed"), "captureSink")).toBe("cable-b");
    expect(defaultCable(status("installed", [false, true, true]), "renderSource")).toBe("cable-b");
    expect(defaultCable(status("installed", [false, true, true]), "captureSink")).toBe("cable-c");
    expect(defaultCable(status("installed", [true]), "captureSink")).toBe("cable-a");
    expect(defaultCable(null, "renderSource")).toBe("cable-a");
    expect(defaultCable(null, "captureSink")).toBe("cable-b");
    expect(presentCables(status("installed", [true, false]))).toEqual(["cable-a"]);
  });

  it("enables the library cable entries exactly when cables are usable", () => {
    const cableEntries = (value: VirtualCableStatus | null) =>
      applyCableAvailability(libraryEntries, value).filter((entry) => entry.virtualKind);
    expect(cableEntries(status("installed")).every((entry) => entry.unavailableReason === undefined)).toBe(true);
    for (const state of ALL_STATES.filter((candidate) => candidate !== "installed")) {
      expect(cableEntries(status(state)).every((entry) => entry.unavailableReason)).toBe(true);
    }
    expect(cableEntries(null).every((entry) => entry.unavailableReason)).toBe(true);
    // Other entries are untouched.
    const others = applyCableAvailability(libraryEntries, status("installed")).filter((entry) => !entry.virtualKind);
    expect(others).toEqual(libraryEntries.filter((entry) => !entry.virtualKind));
  });
});

describe("cable draft helpers", () => {
  it("adds a connected Cable A → Cable B pass-through", () => {
    const next = appendCablePassThrough(demoSession, "cable-a", "cable-b");
    const [source, sink] = next.nodes.slice(-2);
    expect(source).toMatchObject({ kind: "virtualRenderSource", name: "Cable A Input" });
    expect(sink).toMatchObject({ kind: "virtualCaptureSink", name: "Cable B Output" });
    expect(next.edges.at(-1)).toMatchObject({
      sourceNode: source.id,
      sourcePort: "out",
      destinationNode: sink.id,
      destinationPort: "in",
    });
  });

  it("moves a node to another cable, keeping a name the user chose", () => {
    const added = appendVirtualBusNode(demoSession, "cable-a", "renderSource");
    const id = added.nodes.at(-1)!.id;
    const moved = setVirtualBusDraftCable(added, id, "cable-c");
    expect(moved.nodes.at(-1)).toMatchObject({ name: "Cable C Input", parameters: { busId: "cable-c" } });
    const renamed = { ...added, nodes: added.nodes.map((node) => (node.id === id ? { ...node, name: "Game" } : node)) };
    expect(setVirtualBusDraftCable(renamed, id, "cable-b").nodes.at(-1)).toMatchObject({
      name: "Game",
      parameters: { busId: "cable-b" },
    });
    expect(() => setVirtualBusDraftCable(added, "mic", "cable-b")).toThrow(/Not a cable node/);
  });
});

describe("VirtualCablePanel", () => {
  it("shows a pill for every status and while waiting", () => {
    expect(cableStatusPill(null, true)).toEqual({ label: "Checking", tone: "idle" });
    expect(cableStatusPill(null, false)).toEqual({ label: "Unavailable", tone: "idle" });
    expect(cableStatusPill(status("installed"), true).tone).toBe("ready");
    expect(cableStatusPill(status("notInstalled"), true).tone).toBe("idle");
    for (const state of ["needsRepair", "needsRestart", "incompatible"] as const) {
      expect(cableStatusPill(status(state), true).tone).toBe("attention");
    }
  });

  it("lists cables, the driver version and the live route counters", () => {
    setVirtualCableStatus(status("installed", [true, false]));
    render(<VirtualCablePanel connected cableRoute={route} />);
    expect(screen.getByRole("status").textContent).toBe("Installed");
    expect(screen.getByText("Driver 0.1.4.0 · protocol 1.1")).toBeTruthy();
    expect(screen.getByText("Cable A Input → Cable A Output")).toBeTruthy();
    expect(screen.getByText("Missing from Windows sound devices")).toBeTruthy();
    expect(screen.getByText(/Cable A → Cable B · running · 480-frame blocks/)).toBeTruthy();
    expect(screen.getByText((12345).toLocaleString())).toBeTruthy();
  });

  it("shows endpoint formats and a reserved hint for formats outside 48 kHz float", () => {
    setVirtualCableStatus(status("installed"));
    render(
      <VirtualCablePanel
        connected
        cableRoute={null}
        devices={[
          {
            id: "{render-cable-a}",
            name: "Cable A Input",
            direction: "render",
            state: "active",
            defaultRoles: [],
            format: { sampleRateHz: 44_100, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
            periods: { default100ns: 100_000, minimum100ns: 26_666 },
          },
          {
            id: "{capture-cable-a}",
            name: "Cable A Output",
            direction: "capture",
            state: "active",
            defaultRoles: [],
            format: { sampleRateHz: 48_000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
            periods: { default100ns: 100_000, minimum100ns: 26_666 },
          },
        ]}
      />,
    );
    expect(screen.getByText(/44,100 Hz, 2 ch, 32-bit float/)).toBeTruthy();
    expect(screen.getAllByText("Use 48 kHz float for the best match.")).toHaveLength(1);
  });

  it("keeps the counter slots while no route plays and follows status changes", () => {
    render(<VirtualCablePanel connected cableRoute={null} />);
    expect(screen.getByRole("status").textContent).toBe("Checking");
    expect(screen.getByText("Not playing")).toBeTruthy();
    expect(screen.getAllByText("—").length).toBeGreaterThanOrEqual(4);
    act(() => setVirtualCableStatus(status("needsRestart")));
    expect(screen.getByRole("status").textContent).toBe("Restart needed");
  });

  it("offers the pass-through route only when cables are usable", () => {
    const add = vi.fn();
    setVirtualCableStatus(status("notInstalled", []));
    render(<VirtualCablePanel connected cableRoute={null} onAddPassThrough={add} />);
    const button = screen.getByRole("button", { name: /Add route:/ });
    expect(button).toHaveProperty("disabled", true);
    act(() => setVirtualCableStatus(status("installed")));
    expect(button.textContent).toContain("Cable A → Cable B");
    expect(button).toHaveProperty("disabled", false);
    fireEvent.click(button);
    expect(add).toHaveBeenCalledOnce();
  });

  it("enables only the management action that matches the current status", () => {
    setVirtualCableStatus(status("needsRepair"));
    render(<VirtualCablePanel connected cableRoute={null} manageVirtualCable={vi.fn()} />);
    expect(screen.getByRole("button", { name: "Repair" })).toHaveProperty("disabled", false);
    expect(screen.getByRole("button", { name: "Install" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Update" })).toHaveProperty("disabled", false);
    expect(screen.getByRole("button", { name: "Remove" })).toHaveProperty("disabled", false);
  });

  it("writes the selected low-latency period through the shell helper", () => {
    setVirtualCableStatus(status("installed"));
    const manage = vi.fn().mockResolvedValue({ ok: true });
    render(<VirtualCablePanel connected cableRoute={null} manageVirtualCable={manage} />);
    fireEvent.click(screen.getByRole("checkbox", { name: "Low latency cable blocks" }));
    expect(manage).toHaveBeenCalledWith({ operation: "setLowLatencyOn" });
  });

  it("lists affected sessions before confirming a cable-count reduction", () => {
    vi.useFakeTimers();
    setVirtualCableStatus(status("installed", [true, true, true]));
    const manage = vi.fn().mockResolvedValue({ ok: true });
    const refresh = vi.fn().mockResolvedValue(status("installed", [true, true]));
    render(
      <VirtualCablePanel
        connected
        cableRoute={null}
        manageVirtualCable={manage}
        refreshVirtualCableStatus={refresh}
        sessions={[
          {
            id: "session-1",
            name: "Game and chat",
            nodes: [{ kind: "virtualRenderSource", parameters: { busId: "cable-c" } }],
          },
        ]}
      />,
    );
    fireEvent.change(screen.getByRole("combobox", { name: "Number of cables" }), { target: { value: "2" } });
    expect(screen.getByText("Game and chat")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Confirm and remove cables" }));
    expect(manage).toHaveBeenCalledWith({ operation: "setCount", count: 2 });
  });

  it("validates the helper's cable-name limits before saving", () => {
    expect(isCableNameValid("Discord")).toBe(true);
    expect(isCableNameValid("")).toBe(false);
    expect(isCableNameValid("bad(name)")).toBe(false);
    expect(isCableNameValid("x".repeat(49))).toBe(false);
    expect(isCableNameValid("bad\nname")).toBe(false);
  });

  it("saves a validated cable name through the shell operation", async () => {
    vi.useFakeTimers();
    setVirtualCableStatus(status("installed"));
    const manage = vi.fn().mockResolvedValue({ ok: true });
    const renamed = status("installed");
    renamed.cables[0].name = "Discord";
    const refresh = vi.fn().mockResolvedValue(renamed);
    render(
      <VirtualCablePanel connected cableRoute={null} manageVirtualCable={manage} refreshVirtualCableStatus={refresh} />,
    );
    fireEvent.change(screen.getByRole("textbox", { name: "Cable A name" }), { target: { value: "Discord" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Save name" })[0]);
    expect(manage).toHaveBeenCalledWith({ operation: "configure", names: [[1, "Discord"]] });
    await act(async () => vi.advanceTimersByTimeAsync(2100));
    expect(refresh).toHaveBeenCalledOnce();
    expect((screen.getByRole("textbox", { name: "Discord name" }) as HTMLInputElement).value).toBe("Discord");
  });
});

describe("CableBinding", () => {
  it("chooses among present cables and always shows the node's own cable", () => {
    const change = vi.fn();
    const { rerender } = render(
      <CableBinding
        status={status("installed", [true, true, false])}
        value="cable-a"
        direction="captureSink"
        disabled={false}
        onChange={change}
      />,
    );
    const select = screen.getByRole("combobox", { name: "Cable" }) as HTMLSelectElement;
    expect([...select.options].map((option) => [option.text, option.disabled])).toEqual([
      ["Cable A Output", false],
      ["Cable B Output", false],
      ["Cable C Output (missing)", true],
    ]);
    fireEvent.change(select, { target: { value: "cable-b" } });
    expect(change).toHaveBeenCalledWith("cable-b");
    expect(screen.getByText("In the app, choose Cable A Output as its microphone.")).toBeTruthy();

    rerender(
      <CableBinding status={null} value="cable-d" direction="renderSource" disabled={false} onChange={change} />,
    );
    const unknown = screen.getByRole("combobox", { name: "Cable" }) as HTMLSelectElement;
    expect(unknown.value).toBe("cable-d");
    expect(unknown.options[0].text).toBe("Cable D Input (not installed)");
    expect(unknown.disabled).toBe(true);
  });
});
