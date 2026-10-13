// Setup → AudioRouter cables (spec 17 §7.4): the cable driver's status, each
// cable's Windows devices, and the live counters of a playing cable route.
// Every slot renders at a fixed size so status and counters never move the
// panel (UI-17).
import { useEffect, useMemo, useState } from "react";
import type { DeviceListItem, DiagnosticsSnapshot, VirtualCableStatus } from "@audiorouter/contracts";
import type { CableManagementRequest } from "./backend";
import {
  cableLabel,
  cableName,
  cableUnavailableReason,
  defaultCable,
  setVirtualCableStatus,
  useVirtualCableStatus,
} from "./virtualCableStatus";

type CableRoute = NonNullable<DiagnosticsSnapshot["cableRoute"]>;

export function isCableNameValid(name: string): boolean {
  const length = [...name].length;
  return length >= 1 && length <= 48 && !/[()\p{Cc}]/u.test(name);
}

const STATE_LABELS: Record<VirtualCableStatus["state"], string> = {
  installed: "Installed",
  notInstalled: "Not installed",
  needsRepair: "Needs repair",
  needsRestart: "Restart needed",
  incompatible: "Update needed",
};

const STATE_DETAILS: Record<VirtualCableStatus["state"], string> = {
  installed: "Apps can play into a cable's Input and record from its Output.",
  notInstalled: "AudioRouter cables are not installed. You can still route through another virtual cable.",
  needsRepair: "Some cable files or Windows devices are missing. Reinstall the cables to repair them.",
  needsRestart: "Restart Windows to finish installing or removing the cables.",
  incompatible: "The installed cables are too old for this version of AudioRouter.",
};

/** The pill's text and tone for a status (null: still asking, or offline). */
export function cableStatusPill(
  status: VirtualCableStatus | null,
  connected: boolean,
): { label: string; tone: "ready" | "attention" | "idle" } {
  if (!status) return { label: connected ? "Checking" : "Unavailable", tone: "idle" };
  if (status.state === "installed") return { label: STATE_LABELS.installed, tone: "ready" };
  return { label: STATE_LABELS[status.state], tone: status.state === "notInstalled" ? "idle" : "attention" };
}

export function VirtualCablePanel({
  connected,
  cableRoute,
  onAddPassThrough,
  sessions = [],
  devices = [],
  manageVirtualCable,
  refreshVirtualCableStatus,
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
}) {
  const status = useVirtualCableStatus();
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
        const match = /^cable-([a-h])$/.exec(String(node.parameters.busId ?? ""));
        return match !== null && match[1].charCodeAt(0) - 96 > pendingCount;
      }),
    );
  }, [pendingCount, sessions]);
  const pill = cableStatusPill(status, connected);
  const route = cableRoute ?? null;
  const counters: [string, number | null][] = [
    ["Forwarded", route?.forwarded ?? null],
    ["Silence", route?.silence ?? null],
    ["Dropped", route?.dropped ?? null],
    ["Trimmed", route?.trimmed ?? null],
  ];
  const runManagement = async (request: CableManagementRequest, successText: string) => {
    if (!manageVirtualCable || busy) return;
    setBusy(true);
    setMessage("Working…");
    try {
      await manageVirtualCable(request);
      // The backend caches device inventory for two seconds. Wait out that
      // window before requesting the post-operation state.
      await new Promise((resolve) => window.setTimeout(resolve, 2100));
      const refreshed = await refreshVirtualCableStatus?.();
      if (refreshed) setVirtualCableStatus(refreshed);
      setPendingCount(null);
      setMessage(successText);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "The cable operation failed.");
    } finally {
      setBusy(false);
    }
  };
  const managementAvailable = connected && manageVirtualCable !== undefined;
  const canInstall = status?.state === "notInstalled";
  const canUpdate =
    status?.state === "installed" || status?.state === "incompatible" || status?.state === "needsRepair";
  const canRepair = status?.state === "needsRepair";
  const canRemove = status !== null && status.state !== "notInstalled" && status.state !== "needsRestart";
  const canConfigure = status?.state === "installed";
  return (
    <section className="panel setup-panel virtual-cable-panel" aria-labelledby="setup-cables-heading">
      <div className="virtual-cable-heading">
        <h3 id="setup-cables-heading">AudioRouter cables</h3>
        <span className={`virtual-cable-pill is-${pill.tone}`} role="status">
          {pill.label}
        </span>
      </div>
      <p className="muted virtual-cable-detail">
        {status
          ? STATE_DETAILS[status.state]
          : connected
            ? "Asking the backend for the cable status."
            : "Connect to the backend to see the cable status."}
      </p>
      <p className="muted virtual-cable-version">
        Driver {status?.driverVersion ?? "—"} · protocol {status?.protocol ?? "—"}
      </p>
      <ul className="setup-status-list" aria-label="Cables">
        {status && status.cables.length > 0 ? (
          status.cables.map((cable) => (
            <li key={cable.busId} className={cable.present ? "is-ready" : "is-needs-attention"}>
              <span className="setup-status-dot" aria-hidden="true" />
              <div>
                <strong>{cable.name}</strong>
                <small>
                  {cable.present ? `${cable.name} Input → ${cable.name} Output` : "Missing from Windows sound devices"}
                </small>
                <div className="virtual-cable-formats" aria-label={`${cable.name} endpoint formats`}>
                  {[cable.renderEndpointId, cable.captureEndpointId].map((endpointId, index) => {
                    const endpoint = devices.find((device) => device.id === endpointId && device.state === "active");
                    const format = endpoint && "format" in endpoint ? endpoint.format : null;
                    const float32 = format ? (format.isFloat32 ?? format.formatTag === 3) : false;
                    const supported = format?.sampleRateHz === 48_000 && float32;
                    const label = index === 0 ? "Input" : "Output";
                    return (
                      <div key={label} className="virtual-cable-format-line">
                        <span>
                          {label}:{" "}
                          {format
                            ? `${format.sampleRateHz.toLocaleString()} Hz, ${format.channels} ch, ${format.bitsPerSample}-bit ${float32 ? "float" : "PCM/other"}`
                            : "format unavailable"}
                        </span>
                        <small className="virtual-cable-format-hint" aria-live="polite">
                          {format && !supported ? "Use 48 kHz float for the best match." : " "}
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
              <strong>No cables</strong>
              <small>{status ? "No AudioRouter cable devices were found." : "Waiting for the cable status."}</small>
            </span>
          </li>
        )}
      </ul>
      <div className="virtual-cable-management" aria-label="Cable management">
        <div className="virtual-cable-actions">
          <button
            type="button"
            className="secondary"
            disabled={!managementAvailable || !canInstall || busy}
            onClick={() => void runManagement({ operation: "install" }, "Installation complete.")}
          >
            Install
          </button>
          <button
            type="button"
            className="secondary"
            disabled={!managementAvailable || !canUpdate || busy}
            onClick={() => void runManagement({ operation: "update" }, "Update complete.")}
          >
            Update
          </button>
          <button
            type="button"
            className="secondary"
            disabled={!managementAvailable || !canRepair || busy}
            onClick={() => void runManagement({ operation: "repair" }, "Repair complete.")}
          >
            Repair
          </button>
          <button
            type="button"
            className="secondary"
            disabled={!managementAvailable || !canRemove || busy}
            onClick={() => void runManagement({ operation: "remove" }, "AudioRouter cables removed.")}
          >
            Remove
          </button>
        </div>
        <p className="muted virtual-cable-action-message" aria-live="polite">
          {message}
        </p>
        {status && (
          <div className="virtual-cable-configuration">
            <label>
              Number of cables
              <select
                aria-label="Number of cables"
                value={countDraft}
                disabled={!managementAvailable || !canConfigure || busy}
                onChange={(event) => {
                  setCountDraft(Number(event.target.value));
                  setPendingCount(Number(event.target.value));
                }}
              >
                {Array.from({ length: 8 }, (_, index) => index + 1).map((count) => (
                  <option key={count} value={count}>
                    {count}
                  </option>
                ))}
              </select>
            </label>
            <label className="virtual-cable-low-latency">
              <span>Low latency (128-frame cable blocks)</span>
              <input
                type="checkbox"
                aria-label="Low latency cable blocks"
                checked={status.defaultPeriodFrames === 128}
                disabled={
                  !managementAvailable ||
                  !canConfigure ||
                  busy ||
                  status.defaultPeriodFrames === null ||
                  status.defaultPeriodFrames === undefined
                }
                onChange={(event) =>
                  void runManagement(
                    { operation: event.target.checked ? "setLowLatencyOn" : "setLowLatencyOff" },
                    event.target.checked ? "Low-latency cable blocks enabled." : "Default cable block size restored.",
                  )
                }
              />
              <small>
                Current preferred block:{" "}
                {status.defaultPeriodFrames === null || status.defaultPeriodFrames === undefined
                  ? "unavailable"
                  : `${status.defaultPeriodFrames} frames`}
                . Changing it briefly restarts the cable device.
              </small>
            </label>
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
                  void runManagement({ operation: "setCount", count: pendingCount }, "Cable count updated.");
              }}
            >
              Apply number of cables
            </button>
            {pendingCount !== null && pendingCount < status.cableCount && (
              <div className="virtual-cable-count-confirmation" role="group" aria-label="Confirm cable removal">
                <p>
                  Reducing the count removes {status.cableCount - pendingCount} cable
                  {status.cableCount - pendingCount === 1 ? "" : "s"} from Windows. Saved sessions that use those
                  cables:
                </p>
                {affectedSessions.length ? (
                  <ul>
                    {affectedSessions.map((session) => (
                      <li key={session.id}>{session.name}</li>
                    ))}
                  </ul>
                ) : (
                  <p>No saved sessions use those cables.</p>
                )}
                <button
                  type="button"
                  className="secondary"
                  disabled={!managementAvailable || busy}
                  onClick={() =>
                    void runManagement({ operation: "setCount", count: pendingCount }, "Cable count updated.")
                  }
                >
                  Confirm and remove cables
                </button>
              </div>
            )}
            <ul className="virtual-cable-name-list" aria-label="Cable names">
              {status.cables.map((cable) => (
                <li key={cable.busId}>
                  <label>
                    {cable.name} name
                    <input
                      aria-label={`${cable.name} name`}
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
                        `${cableLabel(cable.busId)} renamed.`,
                      );
                    }}
                  >
                    Save name
                  </button>
                </li>
              ))}
            </ul>
            <small className="virtual-cable-name-note">
              These names are AudioRouter&apos;s labels. Other apps keep seeing &quot;AudioRouter Cable A Input&quot;
              and so on; to rename a cable there, use Windows Sound settings → Playback or Recording → the cable →
              Properties.
            </small>
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
            Add route: {cableName(status, defaultCable(status, "renderSource"))} →{" "}
            {cableName(status, defaultCable(status, "captureSink"))}
          </button>
        </div>
      )}
      <div className="virtual-cable-route" aria-label="Cable route">
        <p className="virtual-cable-route-title">
          <strong>Cable route</strong>
          <span className="muted">
            {route
              ? `${cableName(status, route.renderBus)} → ${cableName(status, route.captureBus)} · ${route.state} · ${route.blockFrames}-frame blocks`
              : "Not playing"}
          </span>
        </p>
        <dl className="virtual-cable-counters">
          {counters.map(([label, value]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>{value === null ? "—" : value.toLocaleString()}</dd>
            </div>
          ))}
        </dl>
        <p className="muted virtual-cable-failure">{route?.failure ?? " "}</p>
      </div>
    </section>
  );
}
