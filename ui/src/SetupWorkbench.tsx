// Content of the Setup side-panel tab (moved from App.tsx).
import type { DeviceListItem, DiagnosticsSnapshot, Session } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import type { SetupStep } from "./setup";
import { ThisPcDevices } from "./ThisPcDevices";
import { VirtualCablePanel } from "./VirtualCablePanel";

export function SetupWorkbench({
  section,
  backend,
  devices,
  refreshDevices,
  setupSteps,
  cableRoute,
  onAddCablePassThrough,
  sessions,
}: {
  section: "status" | "virtualCables";
  backend: UiBackend;
  devices: DeviceListItem[];
  refreshDevices: ({ announce }?: { announce?: boolean }) => void;
  setupSteps: SetupStep[];
  cableRoute?: DiagnosticsSnapshot["cableRoute"];
  onAddCablePassThrough?: () => void;
  sessions: Session[];
}) {
  return (
    <>
      {section === "status" ? (
        <>
          <section className="panel setup-panel" aria-labelledby="setup-status-heading">
            <h3 id="setup-status-heading">Readiness</h3>
            <ul className="setup-status-list">
              {setupSteps.map((step) => (
                <li key={step.id} className={`is-${step.state}`}>
                  <span className="setup-status-dot" aria-hidden="true" />
                  <span>
                    <strong>{step.label}</strong>
                    <small>{step.detail}</small>
                  </span>
                </li>
              ))}
            </ul>
          </section>
          <ThisPcDevices devices={devices} connected={backend.connected} onRefresh={() => refreshDevices()} />
        </>
      ) : (
        <>
          <VirtualCablePanel
            connected={backend.connected}
            cableRoute={cableRoute}
            onAddPassThrough={onAddCablePassThrough}
            sessions={sessions}
            devices={devices}
            manageVirtualCable={backend.manageVirtualCable?.bind(backend)}
            refreshVirtualCableStatus={backend.getVirtualCableStatus?.bind(backend)}
          />

          <section className="panel setup-panel" aria-labelledby="setup-other-apps-heading">
            <h3 id="setup-other-apps-heading">Use with other apps</h3>
            <p className="muted">
              AudioRouter does not change settings in Windows or in other apps. To send your processed microphone to
              Discord, OBS or a game, end the route in an Output Device set to a virtual cable (for example CABLE
              Input), then choose the matching input (CABLE Output) as the microphone in that app.
            </p>
          </section>
        </>
      )}
    </>
  );
}
