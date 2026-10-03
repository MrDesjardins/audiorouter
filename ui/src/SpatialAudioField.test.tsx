/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { DeviceListItem } from "@audiorouter/contracts";
import { SpatialAudioField } from "./SpatialAudioField";

afterEach(cleanup);

const device = (id: string, channels: number): DeviceListItem => ({
  id, name: id, direction: "capture", state: "active", defaultRoles: [],
  format: { sampleRateHz: 48000, channels, bitsPerSample: 32, formatTag: 3, bytesPerFrame: channels * 4 },
  periods: { default100ns: 100000, minimum100ns: 30000 },
} as DeviceListItem);
const devices = [device("stereo", 2), device("seven-one", 8), device("five-one", 6)];

it("offers off and surround-to-headphones modes and reports the choice", () => {
  const onChange = vi.fn();
  render(<SpatialAudioField devices={devices} endpointId="seven-one" mode="off" disabled={false} running={false} onChange={onChange} />);
  const select = screen.getByLabelText("Spatial audio mode") as HTMLSelectElement;
  expect(select.value).toBe("off");
  fireEvent.change(select, { target: { value: "headphones" } });
  expect(onChange).toHaveBeenCalledWith("headphones");
});

it("explains whether the selected device can carry surround", () => {
  const view = render(<SpatialAudioField devices={devices} endpointId="seven-one" mode="headphones" disabled={false} running={false} onChange={() => {}} />);
  expect(screen.getByLabelText("Spatial audio settings").textContent).toContain("7.1 input");
  view.rerender(<SpatialAudioField devices={devices} endpointId="five-one" mode="headphones" disabled={false} running={false} onChange={() => {}} />);
  expect(screen.getByLabelText("Spatial audio settings").textContent).toContain("5.1 input");
  view.rerender(<SpatialAudioField devices={devices} endpointId="stereo" mode="headphones" disabled={false} running={true} onChange={() => {}} />);
  const text = screen.getByLabelText("Spatial audio settings").textContent ?? "";
  expect(text).toContain("2 channel(s); Play will refuse it");
  expect(text).toContain("after Stop and Play");
  view.rerender(<SpatialAudioField devices={devices} endpointId="" mode="headphones" disabled={false} running={false} onChange={() => {}} />);
  expect(screen.getByLabelText("Spatial audio settings").textContent).toContain("Choose the input device above");
});
