import { useState } from "react";
import type { ApplicationInfo, DeviceListItem } from "@audiorouter/contracts";
import {
  DISCORD_RECOMMENDED_SETTINGS,
  deviceUsage,
  type AppCheck,
  type AppCheckRow,
  type AppCheckState,
} from "./appUsage";

const STATE_WORD: Record<AppCheckState, string> = {
  ready: "Ready",
  check: "Check",
  notRunning: "Not running",
  unknown: "Waiting",
};
const STATE_ICON: Record<AppCheckState, string> = { ready: "✓", check: "!", notRunning: "–", unknown: "…" };
const ROW_ICON: Record<AppCheckRow["state"], string> = { ready: "✓", check: "!", info: "i" };

/** The Discord menu path for each row, used by "Show me how". */
const DISCORD_STEPS: Record<AppCheckRow["id"], string> = {
  microphone: "In Discord, open User Settings › Voice & Video and set Input Device to",
  speaker: "In Discord, open User Settings › Voice & Video and set Output Device to",
};

export function AppStateBadge({ state }: { state: AppCheckState }) {
  return (
    <span className={`app-state-badge is-${state}`}>
      <span aria-hidden="true">{STATE_ICON[state]}</span> {STATE_WORD[state]}
    </span>
  );
}

function CopyName({ name }: { name: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      className="secondary app-copy-name"
      onClick={() => {
        void navigator.clipboard
          ?.writeText(name)
          .then(() => setCopied(true))
          .catch(() => setCopied(false));
      }}
    >
      {/* Both labels share one grid cell, so the button keeps its width (UI-17). */}
      <span className={copied ? "app-label-hidden" : undefined} aria-hidden={copied}>
        Copy device name
      </span>
      <span className={copied ? undefined : "app-label-hidden"} aria-hidden={!copied}>
        Copied
      </span>
    </button>
  );
}

function CheckRow({ row }: { row: AppCheckRow }) {
  const [open, setOpen] = useState(false);
  return (
    <li className={`app-check-row is-${row.state}`}>
      <span className="app-check-icon" aria-hidden="true">
        {ROW_ICON[row.state]}
      </span>
      <span className="app-check-body">
        <span className="app-check-line">
          <strong>{row.label}</strong>
          <span className="app-check-device" title={row.deviceName ?? undefined}>
            {row.deviceName ?? "Not in use"}
          </span>
        </span>
        <small>{row.message}</small>
        {row.state === "check" && row.expectedName && (
          <>
            <button
              type="button"
              className="app-link-button"
              aria-expanded={open}
              onClick={() => setOpen((value) => !value)}
            >
              Show me how
            </button>
            {open && (
              <span className="app-check-steps">
                <span>
                  {DISCORD_STEPS[row.id]} <strong>{row.expectedName}</strong>.
                </span>
                <CopyName name={row.expectedName} />
              </span>
            )}
          </>
        )}
      </span>
    </li>
  );
}

function DiscordCard({ check }: { check: AppCheck }) {
  return (
    <section className="panel app-card" aria-labelledby="app-card-discord" data-app="discord">
      <header className="app-card-heading">
        <span className="app-monogram" aria-hidden="true">
          D
        </span>
        <h3 id="app-card-discord">Discord</h3>
        <AppStateBadge state={check.state} />
      </header>
      <p className="muted app-card-summary" role="status">
        {check.summary}
      </p>
      <ul className="app-check-list">
        {check.rows.map((row) => (
          <CheckRow key={row.id} row={row} />
        ))}
      </ul>
      <details className="app-recommended">
        <summary>Recommended Discord settings</summary>
        <p className="muted">In Discord, open User Settings › Voice & Video:</p>
        <ul>
          {DISCORD_RECOMMENDED_SETTINGS.map((item) => (
            <li key={item.setting}>
              <strong>
                {item.setting}: {item.value}
              </strong>
              <small>{item.why}</small>
            </li>
          ))}
        </ul>
      </details>
    </section>
  );
}

function OtherApps({ applications, devices }: { applications: ApplicationInfo[]; devices: DeviceListItem[] }) {
  const usage = deviceUsage(applications);
  const rows = devices
    .filter((device) => device.state === "active")
    .map((device) => ({
      device,
      uses: (usage.get(device.id) ?? []).filter((use) => use.active && use.appId !== "discord"),
    }))
    .filter((row) => row.uses.length > 0);
  return (
    <section className="panel app-card" aria-labelledby="app-card-others">
      <header className="app-card-heading">
        <h3 id="app-card-others">Other apps using audio</h3>
      </header>
      {rows.length === 0 ? (
        <p className="muted">No other app is using an audio device right now.</p>
      ) : (
        <ul className="app-usage-list">
          {rows.map(({ device, uses }) => (
            <li key={device.id}>
              <strong title={device.name}>{device.name}</strong>
              <small>
                {uses
                  .map(
                    (use) =>
                      `${use.label} ${use.direction === "capture" ? "records from it" : "plays to it or records it"}`,
                  )
                  .join(" · ")}
              </small>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

export function AppsPanel({
  discord,
  devices,
  applications,
  stale,
  connected,
  onRefresh,
}: {
  discord: AppCheck;
  devices: DeviceListItem[];
  applications: ApplicationInfo[];
  stale: boolean;
  connected: boolean;
  onRefresh: () => void;
}) {
  return (
    <>
      <div className="apps-toolbar">
        <p className="muted" role="status">
          {!connected
            ? "Connect to AudioRouter to check your apps."
            : stale
              ? "Could not refresh; showing the last check."
              : "Checked when you switch back to AudioRouter."}
        </p>
        <button type="button" className="secondary" onClick={onRefresh} disabled={!connected}>
          Check now
        </button>
      </div>
      <DiscordCard check={discord} />
      <OtherApps applications={applications} devices={devices} />
    </>
  );
}
