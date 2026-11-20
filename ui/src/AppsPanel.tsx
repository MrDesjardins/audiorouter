import { useEffect, useState, type ReactNode } from "react";
import { AppLogo } from "./appLogos";
import type { ApplicationInfo, DeviceListItem } from "@audiorouter/contracts";
import {
  DISCORD_RECOMMENDED_SETTINGS,
  deviceUsage,
  type AppCheck,
  type AppCheckRow,
  type AppCheckState,
} from "./appUsage";
import { Icon, type IconName } from "./icons";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

const STATE_WORD: Record<AppCheckState, MessageKey> = {
  ready: "g1.appsReady",
  check: "g1.appsCheck",
  notRunning: "g1.appsNotRunning",
  unknown: "g1.appsWaiting",
};
const STATE_ICON: Record<AppCheckState, IconName> = {
  ready: "check",
  check: "triangle-alert",
  notRunning: "minus",
  unknown: "ellipsis",
};
const ROW_ICON: Record<AppCheckRow["state"], IconName> = { ready: "check", check: "triangle-alert", info: "info" };

/** The Discord menu path for each row, used by "Show me how". */
const DISCORD_STEPS: Record<AppCheckRow["id"], MessageKey> = {
  microphone: "g1.appsStepMic",
  speaker: "g1.appsStepSpeaker",
};

export function AppStateBadge({ state }: { state: AppCheckState }) {
  const tr = useMessages();
  return (
    <span className={`app-state-badge is-${state}`}>
      <Icon name={STATE_ICON[state]} size={14} /> {tr(STATE_WORD[state])}
    </span>
  );
}

export function CopyName({ name }: { name: string }) {
  const tr = useMessages();
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 2_000);
    return () => window.clearTimeout(timer);
  }, [copied]);
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
        {tr("g1.appsCopyName")}
      </span>
      <span className={copied ? undefined : "app-label-hidden"} aria-hidden={!copied}>
        {tr("g1.appsCopied")}
      </span>
    </button>
  );
}

function CheckRow({
  row,
  onOpenVirtualCables,
  onSetUp,
}: {
  row: AppCheckRow;
  onOpenVirtualCables?: () => void;
  onSetUp?: () => void;
}) {
  const tr = useMessages();
  const [open, setOpen] = useState(false);
  const fix = row.state === "check" ? (row.expectedName ?? null) : null;
  // "Set up for Discord" builds the missing route; without it, the cable tab helps.
  const setUp = row.action === "virtualCables" ? (onSetUp ?? null) : null;
  const openCables = row.action === "virtualCables" && !setUp ? (onOpenVirtualCables ?? null) : null;
  return (
    <li className={`app-check-row is-${row.state}`}>
      <span className="app-check-icon" aria-hidden="true">
        <Icon name={ROW_ICON[row.state]} size={14} />
      </span>
      <span className="app-check-body">
        <span className="app-check-line">
          <strong>{row.label}</strong>
          <span className="app-check-device" title={row.deviceName ?? undefined}>
            {row.deviceName ?? tr("g1.appsNotInUse")}
          </span>
        </span>
        <small>{row.message}</small>
        {/* The link's line is kept on every row, so a row turning from
            ready to check does not grow (UI-17); only opening the steps does. */}
        {setUp ? (
          <button type="button" className="app-link-button" onClick={setUp}>
            {tr("g1.appsSetUpDiscord")}
          </button>
        ) : openCables ? (
          <button type="button" className="app-link-button" onClick={openCables}>
            {tr("g1.appsSetUpCable")}
          </button>
        ) : (
          <button
            type="button"
            className={fix ? "app-link-button" : "app-link-button app-label-hidden"}
            aria-expanded={fix ? open : undefined}
            aria-hidden={!fix}
            disabled={!fix}
            onClick={() => setOpen((value) => !value)}
          >
            {tr("g1.appsShowHow")}
          </button>
        )}
        {fix && open && (
          <span className="app-check-steps">
            <span>
              {tr(DISCORD_STEPS[row.id])} <strong>{fix}</strong>.
            </span>
            <CopyName name={fix} />
          </span>
        )}
      </span>
    </li>
  );
}

function DiscordCard({
  check,
  onOpenVirtualCables,
  onSetUp,
}: {
  check: AppCheck;
  onOpenVirtualCables?: () => void;
  onSetUp?: () => void;
}) {
  const tr = useMessages();
  return (
    <section className="panel app-card" aria-labelledby="app-card-discord" data-app="discord">
      <header className="app-card-heading">
        <AppLogo app="discord" />
        <h3 id="app-card-discord">Discord</h3>
        <AppStateBadge state={check.state} />
      </header>
      <p className="muted app-card-summary" role="status">
        {check.summary}
      </p>
      <ul className="app-check-list">
        {check.rows.map((row) => (
          <CheckRow key={row.id} row={row} onOpenVirtualCables={onOpenVirtualCables} onSetUp={onSetUp} />
        ))}
      </ul>
      <details className="app-recommended">
        <summary>{tr("g1.appsRecommended")}</summary>
        <p className="muted">{tr("g1.appsRecommendedIntro")}</p>
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
  const tr = useMessages();
  const usage = deviceUsage(applications);
  const rows = devices
    .filter((device) => device.state === "active")
    .map((device) => ({
      device,
      // Discord and OBS have their own cards.
      uses: (usage.get(device.id) ?? []).filter((use) => use.active && use.appId === null),
    }))
    .filter((row) => row.uses.length > 0);
  return (
    <section className="panel app-card" aria-labelledby="app-card-others">
      <header className="app-card-heading">
        <h3 id="app-card-others">{tr("g1.appsOthersHeading")}</h3>
      </header>
      {rows.length === 0 ? (
        <p className="muted">{tr("g1.appsOthersNone")}</p>
      ) : (
        <ul className="app-usage-list">
          {rows.map(({ device, uses }) => (
            <li key={device.id}>
              <strong title={device.name}>{device.name}</strong>
              <small>
                {uses
                  .map((use) =>
                    tr(use.direction === "capture" ? "g1.appsUseCapture" : "g1.appsUseRender", { label: use.label }),
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
  onOpenVirtualCables,
  onSetUpDiscord,
  discordSetup,
  obs,
}: {
  discord: AppCheck;
  devices: DeviceListItem[];
  applications: ApplicationInfo[];
  stale: boolean;
  connected: boolean;
  onRefresh: () => void;
  /** Opens the Virtual Cables tab, for a session that has no voice cable yet. */
  onOpenVirtualCables?: () => void;
  /** Opens the "Set up for Discord" flow (D2). */
  onSetUpDiscord?: () => void;
  /** The open "Set up for Discord" flow, shown above the Discord card. */
  discordSetup?: ReactNode;
  /** The OBS card, built by the caller from `obs.status`. */
  obs?: ReactNode;
}) {
  const tr = useMessages();
  return (
    <>
      <div className="apps-toolbar">
        <p className="muted" role="status">
          {!connected ? tr("g1.appsNotConnected") : stale ? tr("g1.appsStale") : tr("g1.appsCheckedOnReturn")}
        </p>
        <button type="button" className="secondary" onClick={onRefresh} disabled={!connected}>
          {tr("g1.appsCheckNow")}
        </button>
      </div>
      {discordSetup}
      <DiscordCard check={discord} onOpenVirtualCables={onOpenVirtualCables} onSetUp={onSetUpDiscord} />
      {obs}
      <OtherApps applications={applications} devices={devices} />
    </>
  );
}
