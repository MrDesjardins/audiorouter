/**
 * New-version notice (UI-18). Normally once a day the app asks GitHub's public
 * releases list for the newest published AudioRouter release and compares it
 * with this build. A failed check is retried with exponential backoff for up
 * to 12 hours; after that it waits for the next day. The request carries no
 * identifier beyond an ordinary HTTPS request, it can be turned off, and a
 * failure never blocks the app. Prereleases count, because AudioRouter
 * publishes prereleases.
 */
import { useCallback, useEffect, useRef, useState } from "react";

export const APP_VERSION: string = typeof __APP_VERSION__ === "string" ? __APP_VERSION__ : "0.0.0";
export const RELEASES_API = "https://api.github.com/repos/MrDesjardins/audiorouter/releases?per_page=20";
export const RELEASE_PAGE_PREFIX = "https://github.com/MrDesjardins/audiorouter/releases/tag/";
export const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;
export const RETRY_WINDOW_MS = 12 * 60 * 60 * 1000;
export const RETRY_BASE_MS = 60 * 1000;
export const RELEASES_PAGE_SIZE = 20;
const FETCH_TIMEOUT_MS = 10_000;
const CLOCK_SKEW_MS = 5 * 60 * 1000;
const MAX_FAILURE_COUNT = 64;
const CACHE_KEY = "audiorouter.ui.update-check";
const PREFERENCE_KEY = "audiorouter.ui.update-check-enabled";
const CHANNEL_KEY = "audiorouter.ui.update-channel";
const RELEASE_TAG_PATTERN = /^v(\d+)\.(\d+)\.(\d+)$/;

export type AvailableUpdate = { version: string; tag: string; url: string };
export type ReleaseChannel = "prerelease" | "stable";
export type UpdateFailure = "offline" | "rate-limited" | "failed";
export type UpdateStatus = "off" | "idle" | "checking" | "current" | "available" | "failed";

export type UpdateCheckState = {
  status: UpdateStatus;
  update: AvailableUpdate | null;
  lastCheckedAt: number | null;
  lastAttemptAt: number | null;
  nextCheckAt: number | null;
  retrying: boolean;
  failure: UpdateFailure | null;
  failureCount: number;
  checkNow: () => void;
};

type StateData = Omit<UpdateCheckState, "checkNow">;

type Cache = {
  checkedAt: number | null;
  forVersion: string;
  channel: ReleaseChannel;
  update: AvailableUpdate | null;
  lastAttemptAt: number | null;
  failureCount: number;
  windowStartedAt: number | null;
  failure: UpdateFailure | null;
  retryAfterMs: number;
};

export type RetryPlan = {
  failureCount: number;
  windowStartedAt: number;
  nextAttemptAt: number;
  retrying: boolean;
};

/**
 * Unit tests and automated browsers never contact GitHub. A Playwright test
 * may supply `window.__updateCheckReleases` (a fake release list) instead;
 * otherwise App passes `noNetworkUpdateFetcher`.
 */
export function isAutomatedHarness(): boolean {
  if (typeof window !== "undefined" && "__updateCheckReleases" in window) return false;
  const vitest =
    (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env?.VITEST !== undefined;
  return vitest || (typeof navigator !== "undefined" && navigator.webdriver === true);
}

/** A fetcher for tests and automated browsers: a successful, empty release list. */
export const noNetworkUpdateFetcher: typeof fetch = () =>
  Promise.resolve({ ok: true, json: async () => [] } as Response);

/** `1.2.3` or `v1.2.3` as numbers; anything else is null. */
export function parseVersion(text: string): [number, number, number] | null {
  const match = /^v?(\d+)\.(\d+)\.(\d+)$/.exec(text.trim());
  return match ? [Number(match[1]), Number(match[2]), Number(match[3])] : null;
}

export function isNewer(candidate: string, current: string): boolean {
  const a = parseVersion(candidate);
  const b = parseVersion(current);
  if (!a || !b) return false;
  for (let index = 0; index < 3; index += 1) if (a[index] !== b[index]) return a[index] > b[index];
  return false;
}

function releaseTag(release: unknown): string | null {
  if (!release || typeof release !== "object" || Array.isArray(release)) return null;
  const tag = (release as { tag_name?: unknown }).tag_name;
  return typeof tag === "string" && RELEASE_TAG_PATTERN.test(tag) ? tag : null;
}

function releasePublishedAt(release: unknown): number {
  if (!release || typeof release !== "object" || Array.isArray(release)) return 0;
  const { published_at: publishedAt, created_at: createdAt } = release as {
    published_at?: unknown;
    created_at?: unknown;
  };
  for (const value of [publishedAt, createdAt]) {
    if (typeof value === "string") {
      const parsed = Date.parse(value);
      if (Number.isFinite(parsed)) return parsed;
    }
  }
  return 0;
}

/**
 * Newest first by publication date, then by version, then by the API's own
 * order. GitHub already returns newest first; sorting here keeps that true if
 * the endpoint changes, and makes the 20-release window deterministic.
 */
export function sortReleasesNewestFirst(releases: unknown): unknown[] {
  if (!Array.isArray(releases)) return [];
  return releases
    .map((release, index) => ({ release, index, publishedAt: releasePublishedAt(release) }))
    .sort((left, right) => {
      if (left.publishedAt !== right.publishedAt) return right.publishedAt - left.publishedAt;
      const leftTag = releaseTag(left.release);
      const rightTag = releaseTag(right.release);
      if (leftTag && rightTag && isNewer(leftTag, rightTag)) return -1;
      if (leftTag && rightTag && isNewer(rightTag, leftTag)) return 1;
      return left.index - right.index;
    })
    .map((entry) => entry.release);
}

function hasInstaller(release: unknown, version: string): boolean {
  if (!release || typeof release !== "object" || Array.isArray(release)) return false;
  const assets = (release as { assets?: unknown }).assets;
  if (!Array.isArray(assets)) return false;
  const expected = `AudioRouter_${version}_x64-setup.exe`;
  return assets.some((asset) => asset && typeof asset === "object" && (asset as { name?: unknown }).name === expected);
}

export type NewestUpdateOptions = {
  channel?: ReleaseChannel;
  requireInstaller?: boolean;
};

/**
 * The newest published (non-draft) release newer than `current`, looked at
 * newest first and limited to the first page. A stable-only channel ignores
 * prereleases. The installer check keeps a source-only release from producing
 * a "new version" notice with nothing to install.
 */
export function newestUpdate(
  releases: unknown,
  current: string,
  options: NewestUpdateOptions = {},
): AvailableUpdate | null {
  const channel = options.channel ?? "prerelease";
  const requireInstaller = options.requireInstaller ?? true;
  const recent = sortReleasesNewestFirst(releases).slice(0, RELEASES_PAGE_SIZE);
  let best: AvailableUpdate | null = null;
  for (const release of recent) {
    const tag = releaseTag(release);
    if (!tag || !isNewer(tag, current)) continue;
    if (!release || typeof release !== "object" || Array.isArray(release)) continue;
    const record = release as { draft?: unknown; prerelease?: unknown };
    if (record.draft === true) continue;
    if (channel === "stable" && record.prerelease === true) continue;
    const version = tag.slice(1);
    if (requireInstaller && !hasInstaller(release, version)) continue;
    const candidate = { version, tag, url: `${RELEASE_PAGE_PREFIX}${tag}` };
    if (!best || isNewer(candidate.tag, best.tag)) best = candidate;
  }
  return best;
}

function finiteTime(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function clampClock(value: number | null, now: number): number | null {
  return value !== null && value > now + CLOCK_SKEW_MS ? now : value;
}

function failureValue(value: unknown): UpdateFailure | null {
  return value === "offline" || value === "rate-limited" || value === "failed" ? value : null;
}

function readCache(now = Date.now()): Cache | null {
  if (typeof window === "undefined") return null;
  try {
    const value: unknown = JSON.parse(window.localStorage.getItem(CACHE_KEY) ?? "null");
    if (!value || typeof value !== "object" || Array.isArray(value)) return null;
    const record = value as Record<string, unknown>;
    if (typeof record.forVersion !== "string") return null;
    if (record.checkedAt !== undefined && record.checkedAt !== null && finiteTime(record.checkedAt) === null)
      return null;
    let update: AvailableUpdate | null = null;
    if (record.update !== null) {
      if (!record.update || typeof record.update !== "object" || Array.isArray(record.update)) return null;
      const tag = (record.update as { tag?: unknown }).tag;
      if (typeof tag !== "string" || !RELEASE_TAG_PATTERN.test(tag)) return null;
      update = { tag, version: tag.slice(1), url: `${RELEASE_PAGE_PREFIX}${tag}` };
    }
    const rawFailureCount = finiteTime(record.failureCount);
    return {
      checkedAt: clampClock(finiteTime(record.checkedAt), now),
      forVersion: record.forVersion,
      channel: record.channel === "stable" ? "stable" : "prerelease",
      update,
      lastAttemptAt: clampClock(finiteTime(record.lastAttemptAt), now),
      failureCount: Math.max(0, Math.min(MAX_FAILURE_COUNT, Math.floor(rawFailureCount ?? 0))),
      windowStartedAt: clampClock(finiteTime(record.windowStartedAt), now),
      failure: failureValue(record.failure),
      retryAfterMs: Math.max(0, Math.min(RETRY_WINDOW_MS, finiteTime(record.retryAfterMs) ?? 0)),
    };
  } catch {
    return null;
  }
}

function writeCache(cache: Cache) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(CACHE_KEY, JSON.stringify(cache));
  } catch {
    /* best effort */
  }
}

export function clearUpdateCache() {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.removeItem(CACHE_KEY);
  } catch {
    /* best effort */
  }
}

export function updateCheckEnabled(): boolean {
  if (typeof window === "undefined") return true;
  try {
    return window.localStorage.getItem(PREFERENCE_KEY) !== "off";
  } catch {
    return true;
  }
}

export function setUpdateCheckEnabled(enabled: boolean) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(PREFERENCE_KEY, enabled ? "on" : "off");
  } catch {
    /* best effort */
  }
  // Turning the check off also forgets the last result (UI-18 privacy).
  if (!enabled) clearUpdateCache();
}

export function readUpdateChannel(): ReleaseChannel {
  if (typeof window === "undefined") return "prerelease";
  try {
    return window.localStorage.getItem(CHANNEL_KEY) === "stable" ? "stable" : "prerelease";
  } catch {
    return "prerelease";
  }
}

export function setUpdateChannel(channel: ReleaseChannel) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(CHANNEL_KEY, channel);
  } catch {
    /* best effort */
  }
}

/** Exponential backoff: 1, 2, 4, ... minutes, capped by the 12-hour window. */
export function retryDelayMs(failureCount: number, retryAfterMs = 0): number {
  const exponent = Math.max(0, Math.min(failureCount - 1, 20));
  const backoff = Math.min(RETRY_BASE_MS * 2 ** exponent, RETRY_WINDOW_MS);
  return Math.min(Math.max(backoff, retryAfterMs), RETRY_WINDOW_MS);
}

/**
 * The next attempt after a failed check. While the 12-hour retry window is
 * open, retry with exponential backoff; the last attempt uses the end of the
 * window, then the next daily check (24 hours after the window started).
 */
export function nextRetryPlan(input: {
  now: number;
  failureCount: number;
  windowStartedAt: number | null;
  retryAfterMs?: number;
}): RetryPlan {
  const failureCount = Math.min(MAX_FAILURE_COUNT, Math.max(0, input.failureCount) + 1);
  const windowStartedAt = input.windowStartedAt ?? input.now;
  const windowEnd = windowStartedAt + RETRY_WINDOW_MS;
  if (windowEnd <= input.now) {
    return { failureCount, windowStartedAt, nextAttemptAt: windowStartedAt + CHECK_INTERVAL_MS, retrying: false };
  }
  const due = input.now + retryDelayMs(failureCount, input.retryAfterMs ?? 0);
  // Use the rest of the window for a final attempt instead of giving it up.
  const nextAttemptAt = Math.min(due, windowEnd);
  return { failureCount, windowStartedAt, nextAttemptAt, retrying: true };
}

/** Resume a retry plan saved before the app restarted, without double-counting. */
export function resumeRetryPlan(input: {
  now: number;
  failureCount: number;
  windowStartedAt: number | null;
  lastAttemptAt: number | null;
  retryAfterMs?: number;
}): RetryPlan {
  const failureCount = Math.max(1, Math.min(MAX_FAILURE_COUNT, Math.floor(input.failureCount)));
  const windowStartedAt = input.windowStartedAt ?? input.lastAttemptAt ?? input.now;
  const windowEnd = windowStartedAt + RETRY_WINDOW_MS;
  if (windowEnd <= input.now) {
    return { failureCount, windowStartedAt, nextAttemptAt: windowStartedAt + CHECK_INTERVAL_MS, retrying: false };
  }
  const due = (input.lastAttemptAt ?? input.now) + retryDelayMs(failureCount, input.retryAfterMs ?? 0);
  // A resumed retry that would cross the window waits for its last moment.
  return {
    failureCount,
    windowStartedAt,
    nextAttemptAt: Math.max(input.now, Math.min(due, windowEnd)),
    retrying: true,
  };
}

class OfflineError extends Error {}

class UpdateFetchError extends Error {
  readonly kind: UpdateFailure;
  readonly retryAfterMs: number;
  constructor(kind: UpdateFailure, retryAfterMs = 0) {
    super(kind);
    this.kind = kind;
    this.retryAfterMs = retryAfterMs;
  }
}

function retryAfterMs(response: Response, now: number): number {
  const header = response.headers?.get?.("retry-after");
  if (header) {
    const seconds = Number(header);
    if (Number.isFinite(seconds) && seconds > 0) return seconds * 1000;
    const date = Date.parse(header);
    if (Number.isFinite(date)) return Math.max(0, date - now);
  }
  const reset = response.headers?.get?.("x-ratelimit-reset");
  if (reset) {
    const seconds = Number(reset);
    if (Number.isFinite(seconds) && seconds > 0) return Math.max(0, seconds * 1000 - now);
  }
  return 0;
}

function failureOf(error: unknown): UpdateFailure {
  if (error instanceof OfflineError) return "offline";
  if (error instanceof UpdateFetchError) return error.kind;
  if (typeof navigator !== "undefined" && navigator.onLine === false) return "offline";
  return "failed";
}

function errorRetryAfterMs(error: unknown): number {
  return error instanceof UpdateFetchError ? error.retryAfterMs : 0;
}

function offData(): StateData {
  return {
    status: "off",
    update: null,
    lastCheckedAt: null,
    lastAttemptAt: null,
    nextCheckAt: null,
    retrying: false,
    failure: null,
    failureCount: 0,
  };
}

function idleData(update: AvailableUpdate | null = null, cache: Cache | null = null): StateData {
  return {
    status: "idle",
    update,
    lastCheckedAt: cache?.checkedAt ?? null,
    lastAttemptAt: cache?.lastAttemptAt ?? null,
    nextCheckAt: null,
    retrying: false,
    failure: null,
    failureCount: 0,
  };
}

function usableUpdate(cache: Cache | null): AvailableUpdate | null {
  return cache?.update && isNewer(cache.update.tag, APP_VERSION) ? cache.update : null;
}

function stateFromCache(cache: Cache, now: number): StateData {
  const update = usableUpdate(cache);
  if (cache.checkedAt !== null && now - cache.checkedAt < CHECK_INTERVAL_MS) {
    return {
      status: update ? "available" : "current",
      update,
      lastCheckedAt: cache.checkedAt,
      lastAttemptAt: cache.lastAttemptAt,
      nextCheckAt: cache.checkedAt + CHECK_INTERVAL_MS,
      retrying: false,
      failure: null,
      failureCount: 0,
    };
  }
  if (cache.failureCount > 0) {
    const plan = resumeRetryPlan({
      now,
      failureCount: cache.failureCount,
      windowStartedAt: cache.windowStartedAt,
      lastAttemptAt: cache.lastAttemptAt,
      retryAfterMs: cache.retryAfterMs,
    });
    return {
      status: "failed",
      update,
      lastCheckedAt: cache.checkedAt,
      lastAttemptAt: cache.lastAttemptAt,
      nextCheckAt: plan.nextAttemptAt,
      retrying: plan.retrying,
      failure: cache.failure ?? "failed",
      failureCount: plan.failureCount,
    };
  }
  return idleData(update, cache);
}

function initialState(enabled: boolean, channel: ReleaseChannel, now: number): StateData {
  if (!enabled) return offData();
  const cache = readCache(now);
  return cache && cache.forVersion === APP_VERSION && cache.channel === channel
    ? stateFromCache(cache, now)
    : idleData();
}

const defaultFetcher: typeof fetch = (...args) => fetch(...args);
const defaultNow = () => Date.now();

/**
 * The new-version state, refreshed once a day, on window focus/visibility
 * when due, after the connection returns, or on demand through `checkNow`.
 * Failures retry for up to 12 hours, then wait for the next day.
 */
export function useUpdateCheck(
  enabled: boolean,
  channel: ReleaseChannel = "prerelease",
  fetcher: typeof fetch = defaultFetcher,
  now: () => number = defaultNow,
): UpdateCheckState {
  const [state, setState] = useState<StateData>(() => initialState(enabled, channel, now()));
  const checkingRef = useRef<object | null>(null);
  const mountedRef = useRef(true);
  const controllerRef = useRef<{ controller: AbortController; cancelled: boolean } | null>(null);
  const stateRef = useRef(state);
  const nowRef = useRef(now);
  const fetcherRef = useRef(fetcher);
  const enabledRef = useRef(enabled);
  const channelRef = useRef(channel);

  stateRef.current = state;
  nowRef.current = now;
  fetcherRef.current = fetcher;
  enabledRef.current = enabled;
  channelRef.current = channel;

  const runCheck = useCallback(async () => {
    if (!enabledRef.current || !mountedRef.current || checkingRef.current) return;
    const token = {};
    checkingRef.current = token;
    const startedAt = nowRef.current();
    const cache = readCache(startedAt);
    const previous = cache && cache.forVersion === APP_VERSION && cache.channel === channelRef.current ? cache : null;
    const controller = new AbortController();
    const run = { controller, cancelled: false };
    controllerRef.current = run;
    setState((current) => ({
      ...current,
      status: "checking",
      failure: null,
      nextCheckAt: null,
      lastAttemptAt: startedAt,
    }));
    const timeout = window.setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS);
    try {
      let payload: unknown;
      if (typeof navigator !== "undefined" && navigator.onLine === false) throw new OfflineError();
      const injected = (window as { __updateCheckReleases?: unknown }).__updateCheckReleases;
      if (injected !== undefined) {
        payload = await Promise.resolve(injected);
      } else {
        const response = await fetcherRef.current(RELEASES_API, {
          headers: {
            Accept: "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
          },
          cache: "no-store",
          credentials: "omit",
          referrerPolicy: "no-referrer",
          signal: controller.signal,
        });
        if (!response.ok) {
          const kind: UpdateFailure = response.status === 403 || response.status === 429 ? "rate-limited" : "failed";
          throw new UpdateFetchError(kind, retryAfterMs(response, nowRef.current()));
        }
        payload = await response.json();
      }
      if (!Array.isArray(payload)) throw new UpdateFetchError("failed", 0);
      // A disable or unmount can race the last await; never write a result the
      // user asked us to abandon.
      if (run.cancelled || !enabledRef.current) return;
      const finishedAt = nowRef.current();
      const found = newestUpdate(payload, APP_VERSION, { channel: channelRef.current, requireInstaller: true });
      writeCache({
        checkedAt: finishedAt,
        forVersion: APP_VERSION,
        channel: channelRef.current,
        update: found,
        lastAttemptAt: finishedAt,
        failureCount: 0,
        windowStartedAt: null,
        failure: null,
        retryAfterMs: 0,
      });
      if (run.cancelled || !mountedRef.current || !enabledRef.current) return;
      setState({
        status: found ? "available" : "current",
        update: found,
        lastCheckedAt: finishedAt,
        lastAttemptAt: finishedAt,
        nextCheckAt: finishedAt + CHECK_INTERVAL_MS,
        retrying: false,
        failure: null,
        failureCount: 0,
      });
    } catch (error) {
      if (run.cancelled || !mountedRef.current || !enabledRef.current) return;
      const finishedAt = nowRef.current();
      const failure = failureOf(error);
      const retryAfter = errorRetryAfterMs(error);
      const plan = nextRetryPlan({
        now: finishedAt,
        failureCount: previous?.failureCount ?? 0,
        windowStartedAt: previous?.windowStartedAt ?? null,
        retryAfterMs: retryAfter,
      });
      const keepUpdate = usableUpdate(previous);
      writeCache({
        checkedAt: previous?.checkedAt ?? null,
        forVersion: APP_VERSION,
        channel: channelRef.current,
        update: keepUpdate,
        lastAttemptAt: finishedAt,
        failureCount: plan.failureCount,
        windowStartedAt: plan.windowStartedAt,
        failure,
        retryAfterMs: retryAfter,
      });
      setState({
        status: "failed",
        update: keepUpdate,
        lastCheckedAt: previous?.checkedAt ?? null,
        lastAttemptAt: finishedAt,
        nextCheckAt: plan.nextAttemptAt,
        retrying: plan.retrying,
        failure,
        failureCount: plan.failureCount,
      });
    } finally {
      window.clearTimeout(timeout);
      if (controllerRef.current === run) controllerRef.current = null;
      if (checkingRef.current === token) checkingRef.current = null;
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      const active = controllerRef.current;
      if (active) {
        active.cancelled = true;
        active.controller.abort();
      }
      controllerRef.current = null;
      checkingRef.current = null;
    };
  }, []);

  // Enabled/channel changes rebuild the visible state from the cache right
  // away, then check now when there is no fresh answer for this version.
  useEffect(() => {
    if (!enabled) {
      const active = controllerRef.current;
      if (active) {
        active.cancelled = true;
        active.controller.abort();
      }
      controllerRef.current = null;
      checkingRef.current = null;
      clearUpdateCache();
      setState(offData());
      return;
    }
    const at = nowRef.current();
    const cache = readCache(at);
    const next =
      cache && cache.forVersion === APP_VERSION && cache.channel === channel ? stateFromCache(cache, at) : idleData();
    setState(next);
    if (next.nextCheckAt !== null && next.nextCheckAt > at) return;
    void runCheck();
  }, [enabled, channel, runCheck]);

  // One timer for the next due attempt. A completed check replaces the time.
  useEffect(() => {
    if (!enabled || state.nextCheckAt === null) return;
    const delay = Math.max(0, state.nextCheckAt - nowRef.current());
    const timer = window.setTimeout(() => void runCheck(), delay);
    return () => window.clearTimeout(timer);
  }, [enabled, state.nextCheckAt, runCheck]);

  // A suspended or hidden window may miss its timer. Check when it comes back
  // if the day is due, and when the connection returns after a failure.
  useEffect(() => {
    if (!enabled) return;
    const checkIfDue = () => {
      if (!enabledRef.current || checkingRef.current) return;
      const current = stateRef.current;
      if (current.nextCheckAt === null || nowRef.current() >= current.nextCheckAt) void runCheck();
    };
    const checkAfterReconnect = () => {
      if (!enabledRef.current || checkingRef.current) return;
      const current = stateRef.current;
      if (current.status === "failed" || current.nextCheckAt === null || nowRef.current() >= current.nextCheckAt) {
        void runCheck();
      }
    };
    const onVisibility = () => {
      if (typeof document !== "undefined" && document.visibilityState === "visible") checkIfDue();
    };
    window.addEventListener("focus", checkIfDue);
    window.addEventListener("online", checkAfterReconnect);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.removeEventListener("focus", checkIfDue);
      window.removeEventListener("online", checkAfterReconnect);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [enabled, runCheck]);

  const checkNow = useCallback(() => {
    void runCheck();
  }, [runCheck]);

  return { ...state, checkNow };
}
