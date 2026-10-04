/**
 * New-version notice (UI-18). At most once a day the app asks GitHub's public
 * releases list for the newest published AudioRouter release and compares it
 * with this build. The request carries no identifier beyond an ordinary HTTPS
 * request; it can be turned off, and any failure is silent. Prereleases count,
 * because AudioRouter publishes prereleases.
 */
import { useEffect, useState } from "react";

export const APP_VERSION: string = typeof __APP_VERSION__ === "string" ? __APP_VERSION__ : "0.0.0";
export const RELEASES_API = "https://api.github.com/repos/MrDesjardins/audiorouter/releases?per_page=20";
export const RELEASE_PAGE_PREFIX = "https://github.com/MrDesjardins/audiorouter/releases/tag/";
const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;
const CACHE_KEY = "audiorouter.ui.update-check";
const PREFERENCE_KEY = "audiorouter.ui.update-check-enabled";

export type AvailableUpdate = { version: string; tag: string; url: string };

/**
 * Unit tests and automated browsers never contact GitHub. A Playwright test
 * may supply `window.__updateCheckReleases` (a fake release list) instead.
 */
export function isAutomatedHarness(): boolean {
  if (typeof window !== "undefined" && "__updateCheckReleases" in window) return false;
  const vitest = (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env?.VITEST !== undefined;
  return vitest || (typeof navigator !== "undefined" && navigator.webdriver === true);
}

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

/** The newest published (non-draft) release newer than `current`, if any. */
export function newestUpdate(releases: unknown, current: string): AvailableUpdate | null {
  if (!Array.isArray(releases)) return null;
  let best: AvailableUpdate | null = null;
  for (const release of releases) {
    if (!release || typeof release !== "object") continue;
    const { tag_name: tag, draft } = release as { tag_name?: unknown; draft?: unknown };
    if (draft === true || typeof tag !== "string" || !/^v\d+\.\d+\.\d+$/.test(tag) || !isNewer(tag, current)) continue;
    if (!best || isNewer(tag, best.tag)) best = { version: tag.slice(1), tag, url: `${RELEASE_PAGE_PREFIX}${tag}` };
  }
  return best;
}

type Cache = { checkedAt: number; update: AvailableUpdate | null; forVersion: string };

function readCache(): Cache | null {
  try {
    const value = JSON.parse(window.localStorage.getItem(CACHE_KEY) ?? "null") as Cache | null;
    return value && typeof value.checkedAt === "number" ? value : null;
  } catch { return null; }
}

function writeCache(cache: Cache) {
  try { window.localStorage.setItem(CACHE_KEY, JSON.stringify(cache)); } catch { /* best effort */ }
}

export function updateCheckEnabled(): boolean {
  try { return window.localStorage.getItem(PREFERENCE_KEY) !== "off"; } catch { return true; }
}

export function setUpdateCheckEnabled(enabled: boolean) {
  try { window.localStorage.setItem(PREFERENCE_KEY, enabled ? "on" : "off"); } catch { /* best effort */ }
}

/**
 * The available update, checked at most once a day and cached. A cached
 * answer for an older app version is ignored after an upgrade.
 */
export function useUpdateCheck(enabled: boolean, fetcher: typeof fetch = (...args) => fetch(...args), now = () => Date.now()): AvailableUpdate | null {
  const [update, setUpdate] = useState<AvailableUpdate | null>(() => {
    const cache = readCache();
    return enabled && cache?.forVersion === APP_VERSION && cache.update && isNewer(cache.update.tag, APP_VERSION) ? cache.update : null;
  });
  useEffect(() => {
    if (!enabled) { setUpdate(null); return; }
    const cache = readCache();
    if (cache && cache.forVersion === APP_VERSION && now() - cache.checkedAt < CHECK_INTERVAL_MS) {
      setUpdate(cache.update && isNewer(cache.update.tag, APP_VERSION) ? cache.update : null);
      return;
    }
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), 10_000);
    const injected = (window as { __updateCheckReleases?: unknown }).__updateCheckReleases;
    const releases = injected !== undefined
      ? Promise.resolve(injected)
      : fetcher(RELEASES_API, { headers: { Accept: "application/vnd.github+json" }, signal: controller.signal })
        .then((response) => (response.ok ? response.json() : Promise.reject(new Error(String(response.status)))));
    void releases
      .then((releases) => {
        const found = newestUpdate(releases, APP_VERSION);
        writeCache({ checkedAt: now(), update: found, forVersion: APP_VERSION });
        setUpdate(found);
      })
      .catch(() => { /* offline or rate-limited: no notice, try again next start */ })
      .finally(() => window.clearTimeout(timeout));
    return () => { controller.abort(); window.clearTimeout(timeout); };
  }, [enabled]); // eslint-disable-line react-hooks/exhaustive-deps
  return update;
}
