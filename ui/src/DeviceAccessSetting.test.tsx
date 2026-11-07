/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { DeviceAccessSetting } from "./DeviceAccess";

afterEach(cleanup);

it("keeps failed reads visible and retries through Check again", async () => {
  const backend = {
    connected: true,
    getDeviceAccess: vi.fn().mockRejectedValueOnce(new Error("offline")).mockResolvedValue({ allowed: true }),
    setDeviceAccess: vi.fn(),
  };
  render(<DeviceAccessSetting backend={backend} />);
  expect(screen.getByRole("group", { name: "Audio device access" })).toBeTruthy();
  await screen.findByText(/Could not read audio device access/);
  expect((screen.getByRole("button", { name: "Allow access" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Check again" }));
  await waitFor(() =>
    expect((screen.getByRole("button", { name: "Withdraw access" }) as HTMLButtonElement).disabled).toBe(false),
  );
  expect(backend.getDeviceAccess).toHaveBeenCalledTimes(2);
});

it("handles an uncertain write without pretending access stayed unchanged", async () => {
  const backend = {
    connected: true,
    getDeviceAccess: vi.fn().mockResolvedValueOnce({ allowed: true }).mockResolvedValue({ allowed: false }),
    setDeviceAccess: vi.fn().mockRejectedValue(new Error("response lost")),
  };
  render(<DeviceAccessSetting backend={backend} />);
  fireEvent.click(await screen.findByRole("button", { name: "Withdraw access" }));
  await screen.findByText(/Could not confirm the change/);
  expect((screen.getByRole("button", { name: "Withdraw access" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Check again" }));
  await waitFor(() =>
    expect((screen.getByRole("button", { name: "Allow access" }) as HTMLButtonElement).disabled).toBe(false),
  );
  expect(backend.setDeviceAccess).toHaveBeenCalledTimes(1);
});

it("keeps a pending write single and shows only its acknowledged value", async () => {
  let finish: ((value: { allowed: boolean }) => void) | undefined;
  const backend = {
    connected: true,
    getDeviceAccess: vi.fn().mockResolvedValue({ allowed: true }),
    setDeviceAccess: vi.fn(
      () =>
        new Promise<{ allowed: boolean }>((resolve) => {
          finish = resolve;
        }),
    ),
  };
  render(<DeviceAccessSetting backend={backend} />);
  const withdraw = await screen.findByRole("button", { name: "Withdraw access" });
  fireEvent.click(withdraw);
  fireEvent.click(withdraw);
  expect(backend.setDeviceAccess).toHaveBeenCalledTimes(1);
  expect((screen.getByRole("button", { name: "Check again" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByTestId("device-access-state").textContent).toBe("Allowed");
  finish?.({ allowed: false });
  await waitFor(() => expect(screen.getByTestId("device-access-state").textContent).toBe("Ask first"));
});

it("ignores a late read from the previous backend", async () => {
  let finish: ((value: { allowed: boolean }) => void) | undefined;
  const old = {
    connected: true,
    getDeviceAccess: vi.fn(
      () =>
        new Promise<{ allowed: boolean }>((resolve) => {
          finish = resolve;
        }),
    ),
    setDeviceAccess: vi.fn(),
  };
  const next = {
    connected: true,
    getDeviceAccess: vi.fn().mockResolvedValue({ allowed: false }),
    setDeviceAccess: vi.fn(),
  };
  const view = render(<DeviceAccessSetting backend={old} />);
  await waitFor(() => expect(finish).toBeDefined());
  view.rerender(<DeviceAccessSetting backend={next} />);
  await waitFor(() => expect(screen.getByTestId("device-access-state").textContent).toBe("Ask first"));
  finish?.({ allowed: true });
  await waitFor(() => expect(screen.getByTestId("device-access-state").textContent).toBe("Ask first"));
});

it("ignores an old write acknowledgement after the backend changes", async () => {
  let finish: ((value: { allowed: boolean }) => void) | undefined;
  const old = {
    connected: true,
    getDeviceAccess: vi.fn().mockResolvedValue({ allowed: true }),
    setDeviceAccess: vi.fn(
      () =>
        new Promise<{ allowed: boolean }>((resolve) => {
          finish = resolve;
        }),
    ),
  };
  const next = {
    connected: true,
    getDeviceAccess: vi.fn().mockResolvedValue({ allowed: false }),
    setDeviceAccess: vi.fn(),
  };
  const view = render(<DeviceAccessSetting backend={old} />);
  fireEvent.click(await screen.findByRole("button", { name: "Withdraw access" }));
  expect(finish).toBeDefined();
  view.rerender(<DeviceAccessSetting backend={next} />);
  await waitFor(() => expect(screen.getByTestId("device-access-state").textContent).toBe("Ask first"));
  finish?.({ allowed: true });
  await waitFor(() => expect(screen.getByTestId("device-access-state").textContent).toBe("Ask first"));
  expect(next.setDeviceAccess).not.toHaveBeenCalled();
});
