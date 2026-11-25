// Virtual Cables → AudioRouter cables (spec 17 §7.4): the cable driver's status, each
// cable's Windows devices, and the live counters of a playing cable route.
// Every slot renders at a fixed size so status and counters never move the
// panel (UI-17).
import { useEffect, useMemo, useState } from "react";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";
import type { DeviceListItem, DiagnosticsSnapshot, VirtualCableStatus } from "@audiorouter/contracts";
import type { CableManagementRequest } from "./backend";
import { errorMessage, say, type UiMessage } from "./actionMessage";
import { BooleanField } from "./BooleanField";
import {
  CABLE_BUS_ID,
  cableLabel,
  cableName,
  cableUnavailableReason,
  defaultCable,
  maxCables,
  setVirtualCableStatus,
  useVirtualCableStatus,
} from "./virtualCableStatus";

type CableRoute = NonNullable<DiagnosticsSnapshot["cableRoute"]>;

export function isCableNameValid(name: string): boolean {
  const length = [...name].length;
  return length >= 1 && length <= 48 && !/[()\p{Cc}]/u.test(name);
}

const STATE_LABELS: Record<VirtualCableStatus["state"], MessageKey> = {
  installed: "g3Cable.stInstalled",
  notInstalled: "g3Cable.stNotInstalled",
  needsRepair: "g3Cable.stNeedsRepair",
  needsRestart: "g3Cable.stNeedsRestart",
  incompatible: "g3Cable.stIncompatible",
};

const STATE_DETAILS: Record<VirtualCableStatus["state"], MessageKey> = {
  installed: "g3Cable.dInstalled",
  notInstalled: "g3Cable.dNotInstalled",
  needsRepair: "g3Cable.dNeedsRepair",
  needsRestart: "g3Cable.dNeedsRestart",
  incompatible: "g3Cable.dIncompatible",
};

/** The pill's message key and tone for a status (null: still asking, or offline). */
export function cableStatusPill(
  status: VirtualCableStatus | null,
  connected: boolean,
): { labelKey: MessageKey; tone: "ready" | "attention" | "idle" } {
  if (!status) return { labelKey: connected ? "g3Cable.checking" : "g3Cable.unavailable", tone: "idle" };
  if (status.state === "installed") return { labelKey: STATE_LABELS.installed, tone: "ready" };
  return { labelKey: STATE_LABELS[status.state], tone: status.state === "notInstalled" ? "idle" : "attention" };
}

/** Whether this installation ships the cable driver, remembered for the app
 * session so reopening the tab renders its final layout at once (UI-17). */
let knownManagementAvailable: boolean | null = null;

export function VirtualCablePanel({
  connected,
  cableRoute,
  onAddPassThrough,
  sessions = [],
  devices = [],
  manageVirtualCable,
  refreshVirtualCableStatus,
  virtualCableManagementAvailable,
  onActionMessage,
}: {
  connected: boolean;
  cableRoute: CableRoute | null | undefined;
  onAddPassThrough?: () => void;
  sessions?: Array<{
    id: string;
    name: string;
    nodes: Array<{ kind: string; parameters: Record<string, boolean | number | string> }>;
  }>;
  devices?: DeviceListItem[];
  manageVirtualCable?: (request: CableManagementRequest) => Promise<unknown>;
  refreshVirtualCableStatus?: () => Promise<VirtualCableStatus>;
  /** Shell probe; without it a provided manageVirtualCable is assumed usable. */
  virtualCableManagementAvailable?: () => Promise<boolean>;
  /** Shows an outcome in the workspace notification area. */
  onActionMessage?: (message: UiMessage) => void;
}) {
  const tr = useMessages();
  const status = useVirtualCableStatus();
  const [packageAvailable, setPackageAvailable] = useState<boolean | null>(
    virtualCableManagementAvailable ? knownManagementAvailable : true,
  );
  useEffect(() => {
    if (!virtualCableManagementAvailable) return;
    let current = true;
    virtualCableManagementAvailable()
      .catch(() => false)
      .then((available) => {
        knownManagementAvailable = available;
        if (current) setPackageAvailable(available);
      });
    return () => {
      current = false;
    };
  }, [virtualCableManagementAvailable]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState(" ");
  const [countDraft, setCountDraft] = useState(status?.cableCount ?? 2);
  const [pendingCount, setPendingCount] = useState<number | null>(null);
  const [nameDrafts, setNameDrafts] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!status) return;
    setCountDraft(status.cableCount);
    setNameDrafts(Object.fromEntries(status.cables.map((cable) => [cable.busId, cable.name])));
  }, [status]);
  const affectedSessions = useMemo(() => {
    if (pendingCount === null) return [];
    return sessions.filter((session) =>
      session.nodes.some((node) => {
        if (node.kind !== "virtualRenderSource" && node.kind !== "virtualCaptureSink") return false;
        const match = CABLE_BUS_ID.exec(String(node.parameters.busId ?? ""));
        return match !== null && match[1].charCodeAt(0) - 96 > pendingCount;
      }),
    );
  }, [pendingCount, sessions]);
  const pill = cableStatusPill(status, connected);
  const route = cableRoute ?? null;
  const counters: [MessageKey, number | null][] = [
    ["g3Cable.forwarded", route?.forwarded ?? null],
    ["g3Cable.silence", route?.silence ?? null],
    ["g3Cable.dropped", route?.dropped ?? null],
    ["g3Cable.trimmed", route?.trimmed ?? null],
  ];
  const runManagement = async (request: CableManagementRequest, success: UiMessage) => {
    if (!manageVirtualCable || busy) return;
    setBusy(true);
    setMessage(tr("g3Cable.working"));
    try {
      await manageVirtualCable(request);
      // The backend caches device inventory for two seconds. Wait out that
      // window before requesting the post-operation state.
      await new Promise((resolve) => window.setTimeout(resolve, 2100));
      const refreshed = await refreshVirtualCableStatus?.();
      if (refreshed) setVirtualCableStatus(refreshed);
      setPendingCount(null);
      if (onActionMessage) {
        setMessage(" ");
        onActionMessage(success);
      } else setMessage(success.text);
    } catch (error) {
      const failure = errorMessage(
        error instanceof Error && error.message
          ? tr("g3Cable.opFailedWith", { message: error.message })
          : tr("g3Cable.opFailed"),
      );
      if (onActionMessage) {
        setMessage(" ");
        onActionMessage(failure);
      } else setMessage(failure.text);
    } finally {
      setBusy(false);
    }
  };
  // Not shipped in this installation (app-only builds) or not in the shell.
  const cablesShipped = manageVirtualCable !== undefined && packageAvailable === true;
  const managementAvailable = connected && cablesShipped;
  const canInstall = status?.state === "notInstalled";
  const canUpdate =
    status?.state === "installed" || status?.state === "incompatible" || status?.state === "needsRepair";
  const canRepair = status?.state === "needsRepair";
  const canRemove = status !== null && status.state !== "notInstalled" && status.state !== "needsRestart";
  const canConfigure = status?.state === "installed";
  return (
    <section className="panel setup-panel virtual-cable-panel" aria-labelledby="setup-cables-heading">
      <div className="virtual-cable-heading">
        <h3 id="setup-cables-heading">{tr("g3Cable.heading")}</h3>
        <span className={`virtual-cable-pill is-${pill.tone}`} role="status">
          {tr(pill.labelKey)}
        </span>
      </div>
      <p className="muted virtual-cable-detail">
        {status ? tr(STATE_DETAILS[status.state]) : connected ? tr("g3Cable.asking") : tr("g3Cable.connectToSee")}
      </p>
      {!cablesShipped && (
        <div className="virtual-cable-unavailable" role="note">
          <strong>{tr("g3Cable.notAvailableYet")}</strong>
          <p>
            {packageAvailable === null && manageVirtualCable !== undefined
              ? tr("g3Cable.checkingDriver")
              : tr("settingsCopy.cablePreview")}
          </p>
        </div>
      )}
      <p className="muted virtual-cable-version">
        {status?.driverVersion
          ? status.protocol
            ? tr("g3Cable.driverProto", { version: status.driverVersion, protocol: status.protocol })
            : tr("g3Cable.driver", { version: status.driverVersion })
          : tr("g3Cable.noDriver")}
      </p>
      <ul className="setup-status-list" aria-label={tr("g3Cable.cables")}>
        {status && status.cables.length > 0 ? (
          status.cables.map((cable) => (
            <li key={cable.busId} className={cable.present ? "is-ready" : "is-needs-attention"}>
              <span className="setup-status-dot" aria-hidden="true" />
              <div>
                <strong>{cable.name}</strong>
                <small>{cable.present ? tr("g3Cable.inOut", { name: cable.name }) : tr("g3Cable.missing")}</small>
                <div className="virtual-cable-formats" aria-label={tr("g3Cable.formatsAria", { name: cable.name })}>
                  {[cable.renderEndpointId, cable.captureEndpointId].map((endpointId, index) => {
                    const endpoint = devices.find((device) => device.id === endpointId && device.state === "active");
                    const format = endpoint && "format" in endpoint ? endpoint.format : null;
                    const float32 = format ? (format.isFloat32 ?? format.formatTag === 3) : false;
                    const supported = format?.sampleRateHz === 48_000 && float32;
                    const label = index === 0 ? tr("g3Cable.input") : tr("g3Cable.output");
                    return (
                      <div key={index} className="virtual-cable-format-line">
                        <span>
                          {tr("g3Cable.formatLine", {
                            label,
                            format: format
                              ? tr("g3Cable.formatText", {
                                  rate: format.sampleRateHz.toLocaleString(),
                                  channels: format.channels,
                                  bits: format.bitsPerSample,
                                  type: float32 ? tr("g3Cable.float") : tr("g3Cable.pcmOther"),
                                })
                              : tr("g3Cable.formatUnavailable"),
                          })}
                        </span>
                        <small className="virtual-cable-format-hint" aria-live="polite">
                          {format && !supported ? tr("g3Cable.use48") : " "}
                        </small>
                      </div>
                    );
                  })}
                </div>
              </div>
            </li>
          ))
        ) : (
          <li>
            <span className="setup-status-dot" aria-hidden="true" />
            <span>
              <strong>{tr("g3Cable.noCables")}</strong>
              <small>{status ? tr("g3Cable.noneFound") : tr("g3Cable.waitingStatus")}</small>
            </span>
          </li>
        )}
      </ul>
      <div
        className={`virtual-cable-management${cablesShipped ? "" : " is-unavailable"}`}
        aria-label={tr("g3Cable.management")}
        aria-disabled={!cablesShipped}
      >
        <div className="virtual-cable-actions">
          <button
            type="button"
            className="secondary"
            disabled={!managementAvailable || !canInstall || busy}
            onClick={() => void runManagement({ operation: "install" }, say("g3Cable.installDone"))}
          >
            {tr("g3Cable.install")}
          </button>
          <button
            type="button"
            className="secondary"
            disabled={!managementAvailable || !canUpdate || busy}
            onClick={() => void runManagement({ operation: "update" }, say("g3Cable.updateDone"))}
          >
            {tr("g3Cable.update")}
          </button>
          <button
            type="button"
            className="secondary"
            disabled={!managementAvailable || !canRepair || busy}
            onClick={() => void runManagement({ operation: "repair" }, say("g3Cable.repairDone"))}
          >
            {tr("g3Cable.repair")}
          </button>
          <button
            type="button"
            className="secondary"
            disabled={!managementAvailable || !canRemove || busy}
            onClick={() => void runManagement({ operation: "remove" }, say("g3Cable.removeDone"))}
          >
            {tr("g3Cable.remove")}
          </button>
        </div>
        <p className="muted virtual-cable-action-message" aria-live="polite">
          {message}
        </p>
        {status && (
          <div className="virtual-cable-configuration">
            <label>
              {tr("g3Cable.count")}
              <select
                aria-label={tr("g3Cable.count")}
                value={countDraft}
                disabled={!managementAvailable || !canConfigure || busy}
                onChange={(event) => {
                  setCountDraft(Number(event.target.value));
                  setPendingCount(Number(event.target.value));
                }}
              >
                {Array.from({ length: maxCables(status) }, (_, index) => index + 1).map((count) => (
                  <option key={count} value={count}>
                    {count}
                  </option>
                ))}
              </select>
            </label>
            <BooleanField
              label={tr("g3Cable.lowLatency")}
              name={tr("g3Cable.lowLatencyName")}
              checked={status.defaultPeriodFrames === 128}
              stateText={
                status.defaultPeriodFrames == null
                  ? tr("g3Cable.unavailable")
                  : status.defaultPeriodFrames === 128
                    ? tr("g3Cable.on")
                    : tr("g3Cable.off")
              }
              disabled={
                !managementAvailable ||
                !canConfigure ||
                busy ||
                status.defaultPeriodFrames === null ||
                status.defaultPeriodFrames === undefined
              }
              onChange={(checked) =>
                void runManagement(
                  { operation: checked ? "setLowLatencyOn" : "setLowLatencyOff" },
                  say(checked ? "g3Cable.lowOnDone" : "g3Cable.lowOffDone"),
                )
              }
              help={
                <small>
                  {tr("g3Cable.currentBlock", {
                    value:
                      status.defaultPeriodFrames === null || status.defaultPeriodFrames === undefined
                        ? tr("g3Cable.blockUnavailable")
                        : tr("g3Cable.blockFrames", { n: status.defaultPeriodFrames }),
                  })}
                </small>
              }
            />
            <button
              type="button"
              className="secondary"
              disabled={
                !managementAvailable ||
                !canConfigure ||
                busy ||
                pendingCount === null ||
                pendingCount === status.cableCount ||
                pendingCount < status.cableCount
              }
              onClick={() => {
                if (pendingCount !== null && pendingCount < status.cableCount) return;
                if (pendingCount !== null)
                  void runManagement({ operation: "setCount", count: pendingCount }, say("g3Cable.countDone"));
              }}
            >
              {tr("g3Cable.applyCount")}
            </button>
            {pendingCount !== null && pendingCount < status.cableCount && (
              <div className="virtual-cable-count-confirmation" role="group" aria-label={tr("g3Cable.confirmGroup")}>
                <p>
                  {tr(status.cableCount - pendingCount === 1 ? "g3Cable.reducing" : "g3Cable.reducingOther", {
                    n: status.cableCount - pendingCount,
                  })}
                </p>
                {affectedSessions.length ? (
                  <ul>
                    {affectedSessions.map((session) => (
                      <li key={session.id}>{session.name}</li>
                    ))}
                  </ul>
                ) : (
                  <p>{tr("g3Cable.noSessions")}</p>
                )}
                <button
                  type="button"
                  className="secondary"
                  disabled={!managementAvailable || busy}
                  onClick={() =>
                    void runManagement({ operation: "setCount", count: pendingCount }, say("g3Cable.countDone"))
                  }
                >
                  {tr("g3Cable.confirmRemove")}
                </button>
              </div>
            )}
            <ul className="virtual-cable-name-list" aria-label={tr("g3Cable.names")}>
              {status.cables.map((cable) => (
                <li key={cable.busId}>
                  <label>
                    {tr("g3Cable.nameField", { name: cable.name })}
                    <input
                      aria-label={tr("g3Cable.nameField", { name: cable.name })}
                      value={nameDrafts[cable.busId] ?? cable.name}
                      maxLength={48}
                      disabled={!managementAvailable || !canConfigure || busy}
                      onChange={(event) =>
                        setNameDrafts((current) => ({ ...current, [cable.busId]: event.target.value }))
                      }
                    />
                  </label>
                  <button
                    type="button"
                    className="secondary"
                    disabled={
                      !managementAvailable ||
                      !canConfigure ||
                      busy ||
                      !isCableNameValid(nameDrafts[cable.busId] ?? cable.name) ||
                      (nameDrafts[cable.busId] ?? cable.name) === cable.name
                    }
                    onClick={() => {
                      const index = cable.busId.charCodeAt(cable.busId.length - 1) - 96;
                      void runManagement(
                        { operation: "configure", names: [[index, nameDrafts[cable.busId] ?? cable.name]] },
                        say("g3Cable.renamed", { name: cableLabel(cable.busId) }),
                      );
                    }}
                  >
                    {tr("g3Cable.saveName")}
                  </button>
                </li>
              ))}
            </ul>
            <small className="virtual-cable-name-note">{tr("g3Cable.nameNote")}</small>
          </div>
        )}
      </div>
      {onAddPassThrough && (
        <div className="virtual-cable-actions">
          <button
            type="button"
            className="secondary"
            disabled={!connected || cableUnavailableReason(status) !== null}
            onClick={onAddPassThrough}
          >
            {tr("g3Cable.addRoute", {
              from: cableName(status, defaultCable(status, "renderSource")),
              to: cableName(status, defaultCable(status, "captureSink")),
            })}
          </button>
        </div>
      )}
      <div className="virtual-cable-route" aria-label={tr("g3Cable.routeAria")}>
        <p className="virtual-cable-route-title">
          <strong>{tr("g3Cable.routeTitle")}</strong>
          <span className="muted">
            {route
              ? tr("g3Cable.routeLine", {
                  from: cableName(status, route.renderBus),
                  to: cableName(status, route.captureBus),
                  state: route.state,
                  frames: route.blockFrames,
                })
              : tr("g3Cable.notPlaying")}
          </span>
        </p>
        <dl className="virtual-cable-counters">
          {counters.map(([label, value]) => (
            <div key={label}>
              <dt>{tr(label)}</dt>
              <dd>{value === null ? "—" : value.toLocaleString()}</dd>
            </div>
          ))}
        </dl>
        <p className="muted virtual-cable-failure">{route?.failure ?? " "}</p>
      </div>
    </section>
  );
}
