// The AudioRouter cable driver's status (`virtualCable.status`, spec 17 §7.1),
// shared by the library, the canvas palette, node properties and Setup. App
// polls the backend and publishes here; readers subscribe without threading
// props through every panel.
import { useSyncExternalStore } from "react";
import type { VirtualCableStatus } from "@audiorouter/contracts";

let current: VirtualCableStatus | null = null;
const listeners = new Set<() => void>();

export function setVirtualCableStatus(status: VirtualCableStatus | null): void {
  if (JSON.stringify(status) === JSON.stringify(current)) return;
  current = status;
  for (const listener of listeners) listener();
}

export function getVirtualCableStatus(): VirtualCableStatus | null {
  return current;
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The latest status, or null before the first answer (or when disconnected). */
export function useVirtualCableStatus(): VirtualCableStatus | null {
  return useSyncExternalStore(subscribe, getVirtualCableStatus, getVirtualCableStatus);
}

/** "cable-a" → "Cable A". Unknown IDs are shown as they are. */
export function cableLabel(busId: string): string {
  const match = /^cable-([a-h])$/.exec(busId);
  return match ? `Cable ${match[1].toUpperCase()}` : busId;
}

/** The Windows device name a cable node stands for: apps play into a cable's
 * Input (an AudioRouter source) and record from its Output (a destination). */
export function cableEndpointName(busId: string, direction: "renderSource" | "captureSink"): string {
  return `${cableLabel(busId)} ${direction === "renderSource" ? "Input" : "Output"}`;
}

/** Enabled cables whose two Windows devices are both present. */
export function presentCables(status: VirtualCableStatus | null): string[] {
  return status?.cables.filter((cable) => cable.present).map((cable) => cable.busId) ?? [];
}

/** The cable a new node starts on: sources take the first present cable,
 * destinations the second (so Cable A → Cable B is the default route), each
 * falling back to Cable A when the status is unknown. */
export function defaultCable(status: VirtualCableStatus | null, direction: "renderSource" | "captureSink"): string {
  const cables = presentCables(status);
  if (direction === "renderSource") return cables[0] ?? "cable-a";
  return cables[1] ?? cables[0] ?? "cable-b";
}

/** Why cable nodes cannot be used, or null when they can. */
export function cableUnavailableReason(status: VirtualCableStatus | null): string | null {
  if (status?.state === "installed" && presentCables(status).length > 0) return null;
  switch (status?.state) {
    case "needsRestart":
      return "Restart Windows to finish installing AudioRouter cables.";
    case "needsRepair":
      return "AudioRouter cables need a repair. Reinstall them, or use Input device with a virtual cable such as CABLE Output.";
    case "incompatible":
      return "The installed AudioRouter cables are too old for this version. Update them, or use Input device with a virtual cable such as CABLE Output.";
    default:
      return "Install AudioRouter cables in Setup, or use Input device with a virtual cable such as CABLE Output.";
  }
}
