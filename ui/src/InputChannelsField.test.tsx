/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { DeviceListItem } from "@audiorouter/contracts";
import { InputChannelsField } from "./InputChannelsField";

afterEach(cleanup);

const device = (id: string, channels: number): DeviceListItem => ({
  id, name: id, direction: "capture", state: "active", defaultRoles: [],
  format: { sampleRateHz: 48000, channels, bitsPerSample: 32, formatTag: 3, bytesPerFrame: channels * 4 },
  periods: { default100ns: 100000, minimum100ns: 30000 },
} as DeviceListItem);
const devices = [device("interface", 2), device("headset", 1)];

it("shows the node's saved mode and reports a mono choice", () => {
  const onChange = vi.fn();
  const view = render(<InputChannelsField devices={devices} endpointId="interface" mode={undefined} surround={false} disabled={false} onChange={onChange} />);
  const select = screen.getByLabelText("Input channels") as HTMLSelectElement;
  expect(select.value).toBe("stereo");
  expect(screen.getByLabelText("Input channel settings").textContent).toContain("only in one ear");
  fireEvent.change(select, { target: { value: "left" } });
  expect(onChange).toHaveBeenCalledWith("channelMode", "left");
  view.rerender(<InputChannelsField devices={devices} endpointId="interface" mode="left" surround={false} disabled={false} onChange={onChange} />);
  expect(select.value).toBe("left");
  expect(screen.getByLabelText("Input channel settings").textContent).toContain("only the left channel to both ears");
  view.rerender(<InputChannelsField devices={devices} endpointId="interface" mode="mono" surround={false} disabled={false} onChange={onChange} />);
  expect(select.value).toBe("mono");
});

it("explains mono devices and is unavailable while surround renders", () => {
  const view = render(<InputChannelsField devices={devices} endpointId="headset" mode="stereo" surround={false} disabled={false} onChange={() => {}} />);
  expect(screen.getByLabelText("Input channel settings").textContent).toContain("device is mono");
  view.rerender(<InputChannelsField devices={devices} endpointId="interface" mode="mono" surround={true} disabled={false} onChange={() => {}} />);
  expect((screen.getByLabelText("Input channels") as HTMLSelectElement).disabled).toBe(true);
  expect(screen.getByLabelText("Input channel settings").textContent).toContain("Not used while spatial audio");
});
