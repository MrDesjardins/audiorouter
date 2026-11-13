import { useEffect, useState } from "react";
import {
  APP_VERSION,
  type AvailableUpdate,
  type ReleaseChannel,
  type UpdateCheckState,
  type UpdateFailure,
} from "./updateCheck";

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });
const dateFormat = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" });

/** Open a release page in the default browser (desktop) or a new tab. */
export function openReleasePage(update: AvailableUpdate) {
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  if (invoke)
    void invoke("open_release_page", { tag: update.tag }).catch(() => window.open(update.url, "_blank", "noopener"));
  else window.open(update.url, "_blank", "noopener");
}

/**
 * "AudioRouter 0.0.8" in the header, followed by "0.0.9 available" when a
 * newer release exists. It stays on the header's single line, so the notice
 * never moves other content (UI-17).
 */
export function VersionLine({ update }: { update: AvailableUpdate | null }) {
  return (
    <p className="eyebrow app-version-line">
      AudioRouter{" "}
      <span className="app-version" aria-label={`Version ${APP_VERSION}`}>
        {APP_VERSION}
      </span>
      {update && (
        <>
          {" "}
          ·{" "}
          <button
            type="button"
            className="update-link"
            title={`Open the AudioRouter ${update.version} release page`}
            onClick={() => openReleasePage(update)}
          >
            {update.version} available
          </button>
        </>
      )}
    </p>
  );
}

export function checkedAtLabel(lastCheckedAt: number | null, now = Date.now()): string {
  if (lastCheckedAt === null) return "Not checked yet.";
  const minutes = Math.max(0, Math.floor((now - lastCheckedAt) / 60_000));
  if (minutes < 1) return "Checked just now.";
  if (minutes < 60) return `Checked ${minutes} minute${minutes === 1 ? "" : "s"} ago.`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `Checked ${hours} hour${hours === 1 ? "" : "s"} ago.`;
  const days = Math.floor(hours / 24);
  return `Checked ${days} day${days === 1 ? "" : "s"} ago.`;
}

export function nextCheckLabel(nextCheckAt: number, now = Date.now()): string {
  if (nextCheckAt <= now) return "now";
  const target = new Date(nextCheckAt);
  const current = new Date(now);
  if (target.toDateString() === current.toDateString()) return `at ${timeFormat.format(target)}`;
  const tomorrow = new Date(now);
  tomorrow.setDate(tomorrow.getDate() + 1);
  if (target.toDateString() === tomorrow.toDateString()) return `tomorrow at ${timeFormat.format(target)}`;
  return `on ${dateFormat.format(target)} at ${timeFormat.format(target)}`;
}

function failureDetail(
  failure: UpdateFailure | null,
  retrying: boolean,
  nextCheckAt: number | null,
  now: number,
): string {
  const reason =
    failure === "offline"
      ? "No internet connection."
      : failure === "rate-limited"
        ? "GitHub is limiting checks."
        : "The check didn't finish.";
  if (nextCheckAt === null) return reason;
  return `${reason} ${retrying ? "Trying again" : "The next check is"} ${nextCheckLabel(nextCheckAt, now)}.`;
}

function statusHeadline(
  status: UpdateCheckState["status"],
  update: AvailableUpdate | null,
  channel: ReleaseChannel,
): string {
  switch (status) {
    case "off":
      return "Checking is off.";
    case "idle":
      return "Not checked yet.";
    case "checking":
      return "Checking GitHub…";
    case "available":
      return update ? `Version ${update.version} is available.` : "A new version is available.";
    case "current":
      return channel === "stable" ? "No newer stable version was found." : "You have the newest version.";
    case "failed":
      return "Can't check for new versions right now.";
  }
}

function nextCheckSuffix(nextCheckAt: number | null, now: number): string {
  return nextCheckAt === null ? "" : ` Next check ${nextCheckLabel(nextCheckAt, now)}.`;
}

function statusDetail(state: UpdateCheckState, channel: ReleaseChannel, now: number): string {
  switch (state.status) {
    case "off":
      return "Turn it on to look for new versions.";
    case "idle":
      return "Check now to look for a newer version.";
    case "checking":
      return "Looking for a newer version.";
    case "available":
      return `You have ${APP_VERSION}.${nextCheckSuffix(state.nextCheckAt, now)}`;
    case "current":
      return (
        (channel === "stable"
          ? `AudioRouter ${APP_VERSION} is installed. Only stable releases are checked.`
          : `AudioRouter ${APP_VERSION} is up to date.`) + nextCheckSuffix(state.nextCheckAt, now)
      );
    case "failed":
      return failureDetail(state.failure, state.retrying, state.nextCheckAt, now);
  }
}

/**
 * Settings › New versions. The status slot, action row and controls all keep
 * their space so an asynchronous result never moves the page (UI-17).
 */
export function UpdatesPanel({
  enabled,
  channel,
  onChange,
  onChannelChange,
  updateState,
}: {
  enabled: boolean;
  channel: ReleaseChannel;
  onChange: (enabled: boolean) => void;
  onChannelChange: (channel: ReleaseChannel) => void;
  updateState: UpdateCheckState;
}) {
  // Refresh the relative "Checked ... ago" line while the page stays open.
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 60_000);
    return () => window.clearInterval(timer);
  }, []);
  const checking = updateState.status === "checking";
  const update = updateState.update;
  return (
    <section className="panel updates-panel" aria-labelledby="updates-heading">
      <h3 id="updates-heading">New versions</h3>
      <div className="updates-status" data-status={updateState.status}>
        <div className="updates-announcement" role="status" aria-live="polite" aria-atomic="true">
          <p className="updates-headline">{statusHeadline(updateState.status, update, channel)}</p>
          <p className="updates-detail">{statusDetail(updateState, channel, now)}</p>
        </div>
        <p className="updates-last-checked">
          {enabled ? checkedAtLabel(updateState.lastCheckedAt, now) : "No result is stored while checking is off."}
        </p>
      </div>
      <div className="updates-actions">
        <button
          type="button"
          className="secondary"
          onClick={updateState.checkNow}
          disabled={!enabled || checking}
          aria-busy={checking}
        >
          {checking ? "Checking…" : "Check now"}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={!update}
          title={update ? `Open the AudioRouter ${update.version} release page` : "No release to open"}
          onClick={() => {
            if (update) openReleasePage(update);
          }}
        >
          Open release page
        </button>
      </div>
      <div className="setting-toggle-row">
        <label className="updates-toggle">
          <input
            type="checkbox"
            checked={enabled}
            aria-describedby="update-check-state"
            onChange={(event) => onChange(event.target.checked)}
          />
          Check GitHub once a day for a newer version
        </label>
        <span className="setting-toggle-state" id="update-check-state">
          {enabled ? "On" : "Off"}
        </span>
      </div>
      <label className="updates-channel">
        <span>Versions to check</span>
        <select
          aria-label="Versions to check"
          value={channel}
          onChange={(event) => onChannelChange(event.target.value === "stable" ? "stable" : "prerelease")}
        >
          <option value="prerelease">Stable and prerelease versions</option>
          <option value="stable">Stable versions only</option>
        </select>
      </label>
      <small className="muted">
        Reads the public AudioRouter release list on github.com; nothing about you or your audio is sent. If GitHub
        cannot be reached, AudioRouter retries for up to 12 hours and then waits for the next day. It never downloads or
        installs an update; the release page opens in your browser. AudioRouter currently publishes prerelease builds.
      </small>
    </section>
  );
}
