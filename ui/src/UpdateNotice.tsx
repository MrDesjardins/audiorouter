import { useEffect, useState } from "react";
import {
  APP_VERSION,
  type AvailableUpdate,
  type ReleaseChannel,
  type UpdateCheckState,
  type UpdateFailure,
} from "./updateCheck";
import { tx } from "./i18n";
import { formatDateTime } from "./i18n/format";
import { useMessages } from "./i18n/useMessages";

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
  const tr = useMessages();
  return (
    <p className="eyebrow app-version-line">
      AudioRouter{" "}
      <span className="app-version" aria-label={tr("version.versionLabel", { version: APP_VERSION })}>
        {APP_VERSION}
      </span>
      {update && (
        <>
          {" "}
          ·{" "}
          <button
            type="button"
            className="update-link"
            title={tr("version.openRelease", { version: update.version })}
            onClick={() => openReleasePage(update)}
          >
            {tr("version.available", { version: update.version })}
          </button>
        </>
      )}
    </p>
  );
}

export function checkedAtLabel(lastCheckedAt: number | null, now = Date.now()): string {
  if (lastCheckedAt === null) return tx("updates.notChecked");
  const minutes = Math.max(0, Math.floor((now - lastCheckedAt) / 60_000));
  if (minutes < 1) return tx("updates.checkedJustNow");
  if (minutes < 60) return tx("updates.checkedMinutes", { count: minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return tx("updates.checkedHours", { count: hours });
  const days = Math.floor(hours / 24);
  return tx("updates.checkedDays", { count: days });
}

export function nextCheckLabel(nextCheckAt: number, now = Date.now()): string {
  if (nextCheckAt <= now) return tx("updates.nextNow");
  const target = new Date(nextCheckAt);
  const current = new Date(now);
  const time = formatDateTime(target, { hour: "numeric", minute: "2-digit" });
  if (target.toDateString() === current.toDateString()) return tx("updates.nextAt", { time });
  const tomorrow = new Date(now);
  tomorrow.setDate(tomorrow.getDate() + 1);
  if (target.toDateString() === tomorrow.toDateString()) return tx("updates.nextTomorrow", { time });
  return tx("updates.nextOn", {
    date: formatDateTime(target, { month: "short", day: "numeric" }),
    time,
  });
}

function failureDetail(
  failure: UpdateFailure | null,
  retrying: boolean,
  nextCheckAt: number | null,
  now: number,
): string {
  const reason =
    failure === "offline"
      ? tx("updates.failureOffline")
      : failure === "rate-limited"
        ? tx("updates.failureRateLimited")
        : tx("updates.failureUnfinished");
  if (nextCheckAt === null) return reason;
  const when = nextCheckLabel(nextCheckAt, now);
  return retrying ? tx("updates.retryingSuffix", { reason, when }) : tx("updates.nextCheckSuffix", { reason, when });
}

function statusHeadline(
  status: UpdateCheckState["status"],
  update: AvailableUpdate | null,
  channel: ReleaseChannel,
): string {
  switch (status) {
    case "off":
      return tx("updates.offHeadline");
    case "idle":
      return tx("updates.idleHeadline");
    case "checking":
      return tx("updates.checkingHeadline");
    case "available":
      return update ? tx("updates.availableHeadline", { version: update.version }) : tx("updates.availableGeneric");
    case "current":
      return channel === "stable" ? tx("updates.currentStable") : tx("updates.currentNewest");
    case "failed":
      return tx("updates.failedHeadline");
  }
}

function nextCheckSuffix(nextCheckAt: number | null, now: number): string {
  return nextCheckAt === null ? "" : ` ${tx("updates.nextCheckOnly", { when: nextCheckLabel(nextCheckAt, now) })}`;
}

function statusDetail(state: UpdateCheckState, channel: ReleaseChannel, now: number): string {
  switch (state.status) {
    case "off":
      return tx("updates.offDetail");
    case "idle":
      return tx("updates.idleDetail");
    case "checking":
      return tx("updates.checkingDetail");
    case "available":
      return `${tx("updates.haveVersion", { version: APP_VERSION })}${nextCheckSuffix(state.nextCheckAt, now)}`;
    case "current":
      return (
        (channel === "stable"
          ? tx("updates.installedStableOnly", { version: APP_VERSION })
          : tx("updates.upToDate", { version: APP_VERSION })) + nextCheckSuffix(state.nextCheckAt, now)
      );
    case "failed":
      return failureDetail(state.failure, state.retrying, state.nextCheckAt, now);
  }
}

/**
 * Settings → New versions. The status slot, action row and controls all keep
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
  const tr = useMessages();
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
      <h3 id="updates-heading">{tr("updates.heading")}</h3>
      <div className="updates-status" data-status={updateState.status}>
        <div className="updates-announcement" role="status" aria-live="polite" aria-atomic="true">
          <p className="updates-headline">{statusHeadline(updateState.status, update, channel)}</p>
          <p className="updates-detail">{statusDetail(updateState, channel, now)}</p>
        </div>
        <p className="updates-last-checked">
          {enabled ? checkedAtLabel(updateState.lastCheckedAt, now) : tr("updates.offStored")}
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
          {checking ? tr("updates.checking") : tr("updates.checkNow")}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={!update}
          title={update ? tr("version.openRelease", { version: update.version }) : tr("updates.noRelease")}
          onClick={() => {
            if (update) openReleasePage(update);
          }}
        >
          {tr("updates.openReleasePage")}
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
          {tr("updates.dailyCheck")}
        </label>
        <span className="setting-toggle-state" id="update-check-state">
          {enabled ? tr("common.on") : tr("common.off")}
        </span>
      </div>
      <label className="updates-channel">
        <span>{tr("updates.channelLabel")}</span>
        <select
          aria-label={tr("updates.channelLabel")}
          value={channel}
          onChange={(event) => onChannelChange(event.target.value === "stable" ? "stable" : "prerelease")}
        >
          <option value="prerelease">{tr("updates.stableAndPrerelease")}</option>
          <option value="stable">{tr("updates.stableOnly")}</option>
        </select>
      </label>
      <small className="muted">{tr("updates.privacyNote")}</small>
    </section>
  );
}
