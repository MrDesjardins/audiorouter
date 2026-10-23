import { useSyncExternalStore } from "react";

/**
 * Devices hidden from the Input Device and Output Device pickers on this PC
 * (UI-22). A local view preference: it never changes sessions, bindings,
 * Windows or the backend. Unreadable or invalid storage hides nothing.
 */
const HIDDEN_DEVICES_KEY = "audiorouter.ui.hidden-devices";
export const MAX_HIDDEN_DEVICES = 256;
const MAX_DEVICE_ID_LENGTH = 512;

type StorageLike = Pick<Storage, "getItem" | "setItem">;

const validId = (value: unknown): value is string =>
  typeof value === "string" && value.length > 0 && value.length <= MAX_DEVICE_ID_LENGTH;

export function readHiddenDevices(storage: Pick<StorageLike, "getItem"> | null): ReadonlySet<string> {
  try {
    const value: unknown = JSON.parse(storage?.getItem(HIDDEN_DEVICES_KEY) ?? "[]");
    if (!Array.isArray(value)) return new Set();
    return new Set(value.filter(validId).slice(0, MAX_HIDDEN_DEVICES));
  } catch {
    return new Set();
  }
}

export function writeHiddenDevices(storage: Pick<StorageLike, "setItem"> | null, ids: ReadonlySet<string>): void {
  try {
    storage?.setItem(HIDDEN_DEVICES_KEY, JSON.stringify([...ids].filter(validId).slice(0, MAX_HIDDEN_DEVICES)));
  } catch {
    /* Optional presentation preference. */
  }
}

const defaultStorage = (): StorageLike | null => {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
};

let current: ReadonlySet<string> | null = null;
const listeners = new Set<() => void>();

function snapshot(): ReadonlySet<string> {
  current ??= readHiddenDevices(defaultStorage());
  return current;
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Show or hide one device in the pickers. Adding past the bound is refused. */
export function setDeviceHidden(id: string, hidden: boolean): void {
  if (!validId(id)) return;
  const before = snapshot();
  if (before.has(id) === hidden) return;
  if (hidden && before.size >= MAX_HIDDEN_DEVICES) return;
  const next = new Set(before);
  if (hidden) next.add(id);
  else next.delete(id);
  current = next;
  writeHiddenDevices(defaultStorage(), next);
  for (const listener of [...listeners]) listener();
}

/** Tests only: forget the cached set so the next read comes from storage. */
export function resetHiddenDevicesForTests(): void {
  current = null;
}

/** The hidden device IDs, re-rendering when they change. */
export function useHiddenDevices(): ReadonlySet<string> {
  return useSyncExternalStore(subscribe, snapshot, snapshot);
}

/**
 * Split picker choices: the hidden ones are dropped, except `keep` (the
 * node's own saved device), which is always listed.
 */
export function visibleDevices<T extends { id: string }>(
  devices: T[],
  hidden: ReadonlySet<string>,
  keep: string,
): { visible: T[]; hiddenCount: number } {
  const visible = devices.filter((device) => !hidden.has(device.id) || device.id === keep);
  return { visible, hiddenCount: devices.length - visible.length };
}
