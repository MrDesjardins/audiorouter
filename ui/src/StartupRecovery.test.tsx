// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { StartupRecovery } from "./StartupRecovery";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
});

const backup = { name: "audiorouter-backup-0000020000.sqlite", bytes: 1048576 };
const shell = (restore = async () => undefined) => {
  const invoke = vi.fn(async (command: string) => {
    if (command === "backend_state")
      return { phase: "stopped", reason: "corruptDatabase", message: "Saved data is damaged." };
    if (command === "recovery_backups") return [backup];
    if (command === "restore_recovery_backup") return restore();
    throw new Error("Unexpected shell command");
  });
  Object.assign(window, { __TAURI_INTERNALS__: { invoke } });
  return invoke;
};

it("offers only shell-validated backups and requires deliberate confirmation", async () => {
  const invoke = shell();
  const onRestored = vi.fn().mockResolvedValue(undefined);
  render(<StartupRecovery onRestored={onRestored} />);
  await screen.findByRole("option", { name: `${backup.name} (1 MiB)` });
  expect(screen.getByText(/resets client access and capture consent/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Restore selected backup" }));
  expect(invoke).not.toHaveBeenCalledWith("restore_recovery_backup", expect.anything());
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Confirm restore selected backup" })));
  expect(invoke).toHaveBeenCalledWith("restore_recovery_backup", { name: backup.name });
  expect(onRestored).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("status").textContent).toContain("Routes are stopped and privacy mute is on");
});

it("keeps a restore failure visible through later diagnostic polls", async () => {
  vi.useFakeTimers();
  shell(async () => {
    throw new Error("Cannot preserve all damaged files");
  });
  render(<StartupRecovery onRestored={async () => undefined} />);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
  });
  fireEvent.click(screen.getByRole("button"));
  await act(async () => fireEvent.click(screen.getByRole("button")));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(2000);
  });
  expect(screen.getByRole("status").textContent).toBe("Cannot preserve all damaged files");
});

it("does not expose recovery controls outside the desktop shell", () => {
  render(<StartupRecovery onRestored={async () => undefined} />);
  expect(screen.queryByRole("button")).toBeNull();
});

it("tells the startup screen once saved data is damaged, so it stops saying Loading", async () => {
  shell();
  const onDamaged = vi.fn();
  render(<StartupRecovery onRestored={async () => undefined} onDamaged={onDamaged} />);
  await screen.findByRole("option", { name: `${backup.name} (1 MiB)` });
  expect(onDamaged).toHaveBeenCalledTimes(1);
});
