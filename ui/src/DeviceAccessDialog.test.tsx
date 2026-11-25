/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { AudioRouterRpcError, type PermissionScope } from "@audiorouter/contracts";
import { DeviceAccessDialog, isDeviceAccessDenied } from "./DeviceAccess";

afterEach(cleanup);

it("recognizes the device-consent refusal from its structured scope, not its wording", () => {
  const refusal = (message: string, permissionScope?: PermissionScope) =>
    new AudioRouterRpcError({
      code: -32001,
      message,
      data: {
        code: "permissionDenied",
        fieldPath: null,
        resourceIds: [],
        retryable: false,
        remediation: "request the required permission scope",
        ...(permissionScope ? { permissionScope } : {}),
      },
    });
  expect(isDeviceAccessDenied(refusal("refused", "deviceAdministration"))).toBe(true);
  expect(isDeviceAccessDenied(refusal("permission denied: DeviceAdministration"))).toBe(false);
  expect(isDeviceAccessDenied(refusal("refused", "graphWrite"))).toBe(false);
  expect(isDeviceAccessDenied(new Error("permission denied: DeviceAdministration"))).toBe(false);
});

it("lists the decline first, the confirming button last, and starts on the confirming one", () => {
  render(<DeviceAccessDialog busy={false} error={null} onAllow={vi.fn()} onCancel={vi.fn()} />);
  const buttons = screen.getAllByRole("button");
  expect(buttons.map((button) => button.textContent)).toEqual(["Not now", "Allow and play"]);
  expect(document.activeElement).toBe(buttons[1]);
});

it("keeps Tab and Shift+Tab inside the dialog (it is modal)", () => {
  render(<DeviceAccessDialog busy={false} error={null} onAllow={vi.fn()} onCancel={vi.fn()} />);
  const [notNow, allow] = screen.getAllByRole("button");
  allow.focus();
  expect(fireEvent.keyDown(allow, { key: "Tab" }), "Tab from the last button is taken").toBe(false);
  expect(document.activeElement).toBe(notNow);
  expect(fireEvent.keyDown(notNow, { key: "Tab", shiftKey: true }), "Shift+Tab from the first button is taken").toBe(
    false,
  );
  expect(document.activeElement).toBe(allow);
});

it("asks nothing of the keyboard while it is busy", () => {
  render(<DeviceAccessDialog busy error={null} onAllow={vi.fn()} onCancel={vi.fn()} />);
  const [button] = screen.getAllByRole("button");
  expect(fireEvent.keyDown(button, { key: "Tab" }), "no enabled control to move to").toBe(false);
});
