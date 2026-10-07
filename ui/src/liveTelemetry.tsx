import { useMemo, useState, useSyncExternalStore, type ReactNode } from "react";
import type { DiagnosticsSnapshot } from "@audiorouter/contracts";

/**
 * Live meter telemetry, kept out of App state. While audio plays the
 * diagnostics refresh runs every 50 ms, but between ticks only
 * `nodeTelemetry` changes. Setting App state for it re-rendered the whole
 * app (about 5,000 elements, most of them hidden panels) 20 times a second,
 * which allocated ~47 MB/s and let the WebView2 renderer swell to ~900 MB
 * before garbage collection. Only the views that show live values subscribe.
 */
type LiveValue = { basis: DiagnosticsSnapshot; nodeTelemetry: DiagnosticsSnapshot["nodeTelemetry"] } | null;

export type TelemetryStore = {
  get: () => LiveValue;
  set: (value: LiveValue) => void;
  subscribe: (listener: () => void) => () => void;
};

export function createTelemetryStore(): TelemetryStore {
  let value: LiveValue = null;
  const listeners = new Set<() => void>();
  return {
    get: () => value,
    set: (next) => {
      value = next;
      for (const listener of listeners) listener();
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

export function useTelemetryStore(): TelemetryStore {
  const [store] = useState(createTelemetryStore);
  return store;
}

/** Whether two diagnostics snapshots differ only in their live node telemetry. */
export function differsOnlyInTelemetry(
  previous: DiagnosticsSnapshot | null | undefined,
  next: DiagnosticsSnapshot,
): boolean {
  if (!previous) return false;
  const before: Record<string, unknown> = { ...previous, nodeTelemetry: null };
  const after: Record<string, unknown> = { ...next, nodeTelemetry: null };
  const keys = new Set([...Object.keys(before), ...Object.keys(after)]);
  for (const key of keys) {
    if (JSON.stringify(before[key]) !== JSON.stringify(after[key])) return false;
  }
  return true;
}

const noStore: TelemetryStore = { get: () => null, set: () => {}, subscribe: () => () => {} };

/**
 * `diagnostics` carrying the latest live telemetry. The store value applies
 * only to the diagnostics object it was taken against, so any newer App
 * snapshot (a state change, or Stop) wins at once. Without a store the
 * diagnostics are returned as they are.
 */
export function useLiveDiagnostics(
  store: TelemetryStore | undefined,
  diagnostics: DiagnosticsSnapshot | null,
): DiagnosticsSnapshot | null {
  const source = store ?? noStore;
  const live = useSyncExternalStore(source.subscribe, source.get, source.get);
  return useMemo(
    () =>
      diagnostics && live && live.basis === diagnostics
        ? { ...diagnostics, nodeTelemetry: live.nodeTelemetry }
        : diagnostics,
    [diagnostics, live],
  );
}

/** Renders `children` with the latest live diagnostics (see `useLiveDiagnostics`). */
export function LiveTelemetry({
  store,
  diagnostics,
  children,
}: {
  store: TelemetryStore;
  diagnostics: DiagnosticsSnapshot | null;
  children: (diagnostics: DiagnosticsSnapshot | null) => ReactNode;
}) {
  return <>{children(useLiveDiagnostics(store, diagnostics))}</>;
}
