import { useCallback, useEffect, useRef, useState } from "react";
import type { ObsStatus } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";

/** While the Apps tab is open. */
export const OBS_STATUS_POLL_MS = 2_000;
/** Elsewhere, for the top-bar chip. */
export const OBS_STATUS_IDLE_POLL_MS = 15_000;
/** While a connection or change is in progress. */
export const OBS_STATUS_BUSY_POLL_MS = 400;

export interface ObsStatusState {
  status: ObsStatus | null;
  /** The last read failed; `status` still holds the last good one. */
  stale: boolean;
}

export type ObsBackend = Pick<UiBackend, "obsStatus" | "obsRefresh">;

function busy(status: ObsStatus | null): boolean {
  return (
    status?.connection === "connecting" ||
    status?.lastChange.state === "applying" ||
    status?.lastChange.state === "restoring"
  );
}

/**
 * The backend's OBS facts. `obs.status` only reads the backend's OBS thread
 * state, so it is cheap; `obs.refresh` asks that thread to read OBS again and
 * runs when the window regains focus (the user may have changed OBS) and on
 * `refresh()`.
 */
export function useObsStatus(
  backend: ObsBackend | null,
  { active }: { active: boolean },
): ObsStatusState & { refresh: () => void; poll: () => void } {
  const [state, setState] = useState<ObsStatusState>({ status: null, stale: false });
  const generation = useRef(0);
  const last = useRef<string | null>(null);

  const poll = useCallback(() => {
    if (!backend?.obsStatus) return;
    const current = generation.current;
    backend
      .obsStatus()
      .then((status) => {
        if (current !== generation.current) return;
        const fingerprint = JSON.stringify(status);
        if (fingerprint === last.current) {
          setState((previous) => (previous.stale ? { ...previous, stale: false } : previous));
          return;
        }
        last.current = fingerprint;
        setState({ status, stale: false });
      })
      .catch(() => {
        if (current !== generation.current) return;
        setState((previous) => (previous.stale ? previous : { ...previous, stale: true }));
      });
  }, [backend]);

  const refresh = useCallback(() => {
    if (!backend?.obsRefresh) return;
    backend
      .obsRefresh()
      .then(() => window.setTimeout(poll, OBS_STATUS_BUSY_POLL_MS))
      .catch(() => poll());
  }, [backend, poll]);

  useEffect(() => {
    generation.current += 1;
    last.current = null;
    poll();
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
  }, [poll, refresh]);

  const inProgress = busy(state.status);
  useEffect(() => {
    if (!backend?.obsStatus) return;
    const interval = inProgress ? OBS_STATUS_BUSY_POLL_MS : active ? OBS_STATUS_POLL_MS : OBS_STATUS_IDLE_POLL_MS;
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") poll();
    }, interval);
    return () => window.clearInterval(timer);
  }, [active, backend, inProgress, poll]);

  // Opening the Apps tab reads OBS again.
  useEffect(() => {
    if (active) refresh();
  }, [active, refresh]);

  return { ...state, refresh, poll };
}
