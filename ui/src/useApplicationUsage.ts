import { useCallback, useEffect, useRef, useState } from "react";
import type { ApplicationInfo } from "@audiorouter/contracts";

/** While the Apps tab is open; elsewhere only focus and explicit refreshes. */
export const APP_USAGE_POLL_MS = 10_000;

export interface ApplicationUsageState {
  applications: ApplicationInfo[];
  /** False until the first answer; the UI shows a placeholder until then. */
  loaded: boolean;
  /** The last refresh failed; `applications` still holds the last good list. */
  stale: boolean;
}

/** A stable fingerprint of what the checks read, to skip no-op renders. */
export function usageFingerprint(applications: readonly ApplicationInfo[]): string {
  return applications
    .map(
      (application) =>
        `${application.processId}:${application.executable}:${(application.audioEndpoints ?? [])
          .map((endpoint) => `${endpoint.deviceId}/${endpoint.direction}/${endpoint.active ? 1 : 0}`)
          .join(",")}`,
    )
    .sort()
    .join("|");
}

/**
 * Which apps use which devices, refreshed when it matters:
 * - when the window regains focus (the user comes back from changing
 *   Discord's settings and sees the result at once);
 * - every 10 s, only while `polling` (the Apps tab is open) and the window
 *   is visible;
 * - on `refresh()`.
 *
 * `apps.list` runs on the backend's control thread, so it is never polled
 * fast and never while the window is hidden.
 */
export function useApplicationUsage(
  listApplications: (() => Promise<ApplicationInfo[]>) | null,
  { polling }: { polling: boolean },
): ApplicationUsageState & { refresh: () => void } {
  const [state, setState] = useState<ApplicationUsageState>({ applications: [], loaded: false, stale: false });
  const fingerprint = useRef<string | null>(null);
  const generation = useRef(0);
  const inFlight = useRef(false);

  const refresh = useCallback(() => {
    if (!listApplications || inFlight.current) return;
    inFlight.current = true;
    const current = ++generation.current;
    listApplications()
      .then((applications) => {
        if (current !== generation.current) return;
        const next = usageFingerprint(applications);
        if (next === fingerprint.current) {
          setState((previous) =>
            previous.stale || !previous.loaded ? { ...previous, loaded: true, stale: false } : previous,
          );
          return;
        }
        fingerprint.current = next;
        setState({ applications, loaded: true, stale: false });
      })
      .catch(() => {
        if (current !== generation.current) return;
        // Keep the last good list (UI-17); mark it so the UI can say so.
        setState((previous) => (previous.stale ? previous : { ...previous, loaded: true, stale: true }));
      })
      .finally(() => {
        if (current === generation.current) inFlight.current = false;
      });
  }, [listApplications]);

  useEffect(() => {
    // A new backend: forget the old answer and anything still in flight.
    generation.current += 1;
    inFlight.current = false;
    fingerprint.current = null;
    refresh();
    const onFocus = () => refresh();
    const onVisible = () => {
      if (document.visibilityState === "visible") refresh();
    };
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [refresh]);

  useEffect(() => {
    if (!polling) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") refresh();
    }, APP_USAGE_POLL_MS);
    return () => window.clearInterval(timer);
  }, [polling, refresh]);

  return { ...state, refresh };
}
