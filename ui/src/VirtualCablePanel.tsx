// Setup → AudioRouter cables (spec 17 §7.4): the cable driver's status, each
// cable's Windows devices, and the live counters of a playing cable route.
// Every slot renders at a fixed size so status and counters never move the
// panel (UI-17).
import type { DiagnosticsSnapshot, VirtualCableStatus } from "@audiorouter/contracts";
import {
  cableEndpointName,
  cableLabel,
  cableUnavailableReason,
  defaultCable,
  useVirtualCableStatus,
} from "./virtualCableStatus";

type CableRoute = NonNullable<DiagnosticsSnapshot["cableRoute"]>;

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
}: {
  connected: boolean;
  cableRoute: CableRoute | null | undefined;
  onAddPassThrough?: () => void;
}) {
  const status = useVirtualCableStatus();
  const pill = cableStatusPill(status, connected);
  const route = cableRoute ?? null;
  const counters: [string, number | null][] = [
    ["Forwarded", route?.forwarded ?? null],
    ["Silence", route?.silence ?? null],
    ["Dropped", route?.dropped ?? null],
    ["Trimmed", route?.trimmed ?? null],
  ];
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
              <span>
                <strong>{cableLabel(cable.busId)}</strong>
                <small>
                  {cable.present
                    ? `${cableEndpointName(cable.busId, "renderSource")} → ${cableEndpointName(cable.busId, "captureSink")}`
                    : "Missing from Windows sound devices"}
                </small>
              </span>
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
      {onAddPassThrough && (
        <div className="virtual-cable-actions">
          <button
            type="button"
            className="secondary"
            disabled={!connected || cableUnavailableReason(status) !== null}
            onClick={onAddPassThrough}
          >
            Add route: {cableLabel(defaultCable(status, "renderSource"))} →{" "}
            {cableLabel(defaultCable(status, "captureSink"))}
          </button>
        </div>
      )}
      <div className="virtual-cable-route" aria-label="Cable route">
        <p className="virtual-cable-route-title">
          <strong>Cable route</strong>
          <span className="muted">
            {route
              ? `${cableLabel(route.renderBus)} → ${cableLabel(route.captureBus)} · ${route.state} · ${route.blockFrames}-frame blocks`
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
