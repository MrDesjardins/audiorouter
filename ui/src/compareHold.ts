import { useSyncExternalStore } from "react";

/**
 * Which tools a hold in Compare by ear has taken out of the running route. A held tool
 * sends no readings, and its live panel must say why rather than report a fault.
 */
let held: readonly string[] = [];
const listeners = new Set<() => void>();

export function setComparing(nodeIds: readonly string[]) {
  held = nodeIds;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useComparing(nodeId: string) {
  return useSyncExternalStore(subscribe, () => held.includes(nodeId));
}
