/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { DeviceListItem } from "@audiorouter/contracts";
import {
  MAX_HIDDEN_DEVICES,
  readHiddenDevices,
  resetHiddenDevicesForTests,
  setDeviceHidden,
  visibleDevices,
  writeHiddenDevices,
} from "./hiddenDevices";
import { PhysicalInputBinding, PhysicalOutputBinding } from "./NodeEditors";
import { ThisPcDevices } from "./ThisPcDevices";

const KEY = "audiorouter.ui.hidden-devices";
const format = { sampleRateHz: 48_000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 };
const periods = { default100ns: 100_000, minimum100ns: 30_000 };
const device = (id: string, name: string, direction: "capture" | "render", channels = 2): DeviceListItem => ({
  id,
  name,
  direction,
  state: "active",
  defaultRoles: [],
  format: { ...format, channels, bytesPerFrame: channels * 4 },
  periods,
});
const devices: DeviceListItem[] = [
  device("mic", "Studio Mic", "capture"),
  device("webcam", "Webcam Mic", "capture"),
  device("phones", "Headphones", "render"),
  device("monitor", "Monitor Speakers", "render"),
  device("game71", "Game 7.1", "render", 8),
];

const optionValues = (label: string) =>
  [...(screen.getByLabelText(label) as HTMLSelectElement).options].map((option) => option.value);

beforeEach(() => {
  localStorage.clear();
  resetHiddenDevicesForTests();
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("hidden device storage", () => {
  it("hides nothing when storage is missing, corrupt, the wrong shape or throws", () => {
    expect(readHiddenDevices(null).size).toBe(0);
    for (const raw of ["not json", "{}", '"mic"', "null", "42"]) {
      expect(readHiddenDevices({ getItem: () => raw }).size).toBe(0);
    }
    expect(
      readHiddenDevices({
        getItem: () => {
          throw new Error("SecurityError");
        },
      }).size,
    ).toBe(0);
  });

  it("keeps only bounded string IDs", () => {
    const raw = JSON.stringify(["mic", "", 7, null, "x".repeat(513), "y".repeat(512), "mic"]);
    expect([...readHiddenDevices({ getItem: () => raw })]).toEqual(["mic", "y".repeat(512)]);
    const many = JSON.stringify(Array.from({ length: 1000 }, (_, index) => `id-${index}`));
    expect(readHiddenDevices({ getItem: () => many }).size).toBe(MAX_HIDDEN_DEVICES);
  });

  it("never throws when writing fails", () => {
    expect(() =>
      writeHiddenDevices(
        {
          setItem: () => {
            throw new Error("QuotaExceededError");
          },
        },
        new Set(["mic"]),
      ),
    ).not.toThrow();
  });

  it("persists changes and refuses more than the bound", () => {
    setDeviceHidden("mic", true);
    expect(JSON.parse(localStorage.getItem(KEY)!)).toEqual(["mic"]);
    setDeviceHidden("mic", false);
    expect(JSON.parse(localStorage.getItem(KEY)!)).toEqual([]);
    for (let index = 0; index < MAX_HIDDEN_DEVICES + 10; index += 1) setDeviceHidden(`id-${index}`, true);
    expect(JSON.parse(localStorage.getItem(KEY)!)).toHaveLength(MAX_HIDDEN_DEVICES);
    setDeviceHidden("", true);
    expect(JSON.parse(localStorage.getItem(KEY)!)).not.toContain("");
  });

  it("keeps working in memory when storage writes fail", () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("QuotaExceededError");
    });
    expect(() => setDeviceHidden("mic", true)).not.toThrow();
    render(
      <PhysicalInputBinding devices={devices} value="" disabled={false} onChange={() => {}} onRefresh={() => {}} />,
    );
    expect(optionValues("Physical input endpoint")).not.toContain("mic");
  });

  it("filters choices but always keeps the bound device", () => {
    const list = [{ id: "a" }, { id: "b" }, { id: "c" }];
    expect(visibleDevices(list, new Set(["a", "b"]), "")).toEqual({ visible: [{ id: "c" }], hiddenCount: 2 });
    expect(visibleDevices(list, new Set(["a", "b"]), "b")).toEqual({
      visible: [{ id: "b" }, { id: "c" }],
      hiddenCount: 1,
    });
    expect(visibleDevices(list, new Set(), "")).toEqual({ visible: list, hiddenCount: 0 });
  });
});

describe("device pickers with hidden devices", () => {
  it("lists every device and no note when nothing is hidden (unchanged behavior)", () => {
    render(
      <PhysicalInputBinding devices={devices} value="" disabled={false} onChange={() => {}} onRefresh={() => {}} />,
    );
    expect(optionValues("Physical input endpoint")).toEqual(["", "mic", "webcam"]);
    expect(screen.queryByText(/hidden/)).toBeNull();
  });

  it("leaves hidden inputs out and says how many are hidden", () => {
    setDeviceHidden("webcam", true);
    render(
      <PhysicalInputBinding devices={devices} value="" disabled={false} onChange={() => {}} onRefresh={() => {}} />,
    );
    expect(optionValues("Physical input endpoint")).toEqual(["", "mic", "hidden-devices-note"]);
    const note = screen.getByRole("option", { name: /1 device hidden/ }) as HTMLOptionElement;
    expect(note.disabled).toBe(true);
  });

  it("keeps a node's own saved device even when it is hidden", () => {
    setDeviceHidden("mic", true);
    setDeviceHidden("webcam", true);
    const onChange = vi.fn();
    render(
      <PhysicalInputBinding devices={devices} value="mic" disabled={false} onChange={onChange} onRefresh={() => {}} />,
    );
    const picker = screen.getByLabelText("Physical input endpoint") as HTMLSelectElement;
    expect(picker.value).toBe("mic");
    expect(picker.disabled).toBe(false);
    expect(optionValues("Physical input endpoint")).toEqual(["", "mic", "hidden-devices-note"]);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("explains where to look when every device is hidden", () => {
    setDeviceHidden("phones", true);
    setDeviceHidden("monitor", true);
    setDeviceHidden("game71", true);
    render(
      <PhysicalOutputBinding devices={devices} value="" disabled={false} onChange={() => {}} onRefresh={() => {}} />,
    );
    expect((screen.getByLabelText("Physical output endpoint") as HTMLSelectElement).disabled).toBe(true);
    expect(screen.getByText(/Every available device is hidden on this PC/)).toBeTruthy();
    expect(screen.queryByText(/No active render endpoints/)).toBeNull();
    expect(screen.getByRole("option", { name: /3 devices hidden/ })).toBeTruthy();
  });

  it("filters the surround loopback group too", () => {
    setDeviceHidden("game71", true);
    render(
      <PhysicalInputBinding
        devices={devices}
        value=""
        surround
        disabled={false}
        onChange={() => {}}
        onRefresh={() => {}}
      />,
    );
    expect(optionValues("Physical input endpoint")).toEqual(["", "mic", "webcam", "hidden-devices-note"]);
    expect(screen.queryByRole("group", { name: "Surround playback devices (loopback)" })).toBeNull();
  });

  it("keeps a hidden surround device that the node already uses", () => {
    setDeviceHidden("game71", true);
    render(
      <PhysicalInputBinding
        devices={devices}
        value="game71"
        surround
        disabled={false}
        onChange={() => {}}
        onRefresh={() => {}}
      />,
    );
    expect((screen.getByLabelText("Physical input endpoint") as HTMLSelectElement).value).toBe("game71");
  });
});

describe("Audio devices on this PC", () => {
  it("switches a device out of an open picker and back, live", () => {
    render(
      <>
        <ThisPcDevices devices={devices} connected onRefresh={() => {}} />
        <PhysicalOutputBinding devices={devices} value="" disabled={false} onChange={() => {}} onRefresh={() => {}} />
      </>,
    );
    const toggle = screen.getByLabelText("Show Monitor Speakers in device pickers") as HTMLInputElement;
    expect(toggle.checked).toBe(true);
    expect(optionValues("Physical output endpoint")).toContain("monitor");

    act(() => {
      fireEvent.click(toggle);
    });
    expect(toggle.checked).toBe(false);
    expect(toggle.closest("li")!.className).toContain("is-hidden-device");
    expect(optionValues("Physical output endpoint")).not.toContain("monitor");
    expect(JSON.parse(localStorage.getItem(KEY)!)).toEqual(["monitor"]);

    act(() => {
      fireEvent.click(toggle);
    });
    expect(optionValues("Physical output endpoint")).toContain("monitor");
    expect(screen.queryByRole("option", { name: /hidden/ })).toBeNull();
  });

  it("still lists hidden devices so they can be shown again", () => {
    setDeviceHidden("webcam", true);
    render(<ThisPcDevices devices={devices} connected onRefresh={() => {}} />);
    expect((screen.getByLabelText("Show Webcam Mic in device pickers") as HTMLInputElement).checked).toBe(false);
    expect((screen.getByLabelText("Show Studio Mic in device pickers") as HTMLInputElement).checked).toBe(true);
  });
});
