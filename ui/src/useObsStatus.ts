import { useCallback, useEffect, useRef, useState } from "react";
import type { ObsStatus } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";

/** While the Apps tab is open. */
export const OBS_STATUS_POLL_MS = 2_000;
/** Elsewhere, for the top-bar chip. */
export const OBS_STATUS_IDLE_POLL_MS = 15_000;
/** While a connection or change is in progress. */
export const OBS_STATUS_BUSY_POLL_MS = 400;
/** Read OBS itself, rather than only the cached backend snapshot. */
export const OBS_INSPECTION_POLL_MS = 10_000;

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
 * runs periodically while visible, on window focus and on `refresh()`.
 */
export function useObsStatus(
  backend: ObsBackend | null,
  { active }: { active: boolean },
): ObsStatusState & { refresh: () => void; poll: () => void } {
  const [state, setState] = useState<ObsStatusState>({ status: null, stale: false });
  const generation = useRef(0);
  const last = useRef<string | null>(null);
  const inFlight = useRef(false);
  const refreshInFlight = useRef(false);
  const delayedPoll = useRef<number | null>(null);
  const mounted = useRef(false);
  const currentBackend = useRef(backend);

  const poll = useCallback(() => {
    if (!mounted.current || backend !== currentBackend.current || !backend?.obsStatus || inFlight.current) return;
    const current = generation.current;
    inFlight.current = true;
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
      })
      .finally(() => {
        if (current === generation.current) inFlight.current = false;
      });
  }, [backend]);

  const refresh = useCallback(() => {
    if (!mounted.current || backend !== currentBackend.current) return;
    if (!backend?.obsRefresh) {
      poll();
      return;
    }
    if (refreshInFlight.current) return;
    const current = generation.current;
    refreshInFlight.current = true;
    backend
      .obsRefresh()
      .then(() => {
        if (current !== generation.current) return;
        if (delayedPoll.current !== null) window.clearTimeout(delayedPoll.current);
        delayedPoll.current = window.setTimeout(() => {
          delayedPoll.current = null;
          if (document.visibilityState === "visible") poll();
        }, OBS_STATUS_BUSY_POLL_MS);
      })
      .catch(() => {
        if (current === generation.current) poll();
      })
      .finally(() => {
        if (current === generation.current) refreshInFlight.current = false;
      });
  }, [backend, poll]);

  useEffect(() => {
    mounted.current = true;
    currentBackend.current = backend;
    generation.current += 1;
    last.current = null;
    inFlight.current = false;
    refreshInFlight.current = false;
    setState({ status: null, stale: false });
    poll();
    const onFocus = () => {
      if (document.visibilityState === "visible") refresh();
    };
    const onVisible = () => {
      if (document.visibilityState === "visible") refresh();
    };
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      mounted.current = false;
      generation.current += 1;
      if (delayedPoll.current !== null) window.clearTimeout(delayedPoll.current);
      delayedPoll.current = null;
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [backend, poll, refresh]);

  const inProgress = busy(state.status);
  useEffect(() => {
    if (!backend?.obsStatus) return;
    const interval = inProgress ? OBS_STATUS_BUSY_POLL_MS : active ? OBS_STATUS_POLL_MS : OBS_STATUS_IDLE_POLL_MS;
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") poll();
    }, interval);
    return () => window.clearInterval(timer);
  }, [active, backend, inProgress, poll]);

  useEffect(() => {
    if (!backend?.obsRefresh || inProgress) return;
    const timer = window.setInterval(
      () => {
        if (document.visibilityState === "visible") refresh();
      },
      active ? OBS_INSPECTION_POLL_MS : OBS_STATUS_IDLE_POLL_MS,
    );
    return () => window.clearInterval(timer);
  }, [active, backend, inProgress, refresh]);

  // Opening the Apps tab reads OBS again.
  useEffect(() => {
    if (active && document.visibilityState === "visible") refresh();
  }, [active, refresh]);

  return { ...state, refresh, poll };
}
