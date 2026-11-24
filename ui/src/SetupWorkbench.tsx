// Content of the Setup side-panel tab (moved from App.tsx).
import { useMemo } from "react";
import type { DeviceListItem, DiagnosticsSnapshot, Session, SetupFinding } from "@audiorouter/contracts";
import type { UiMessage } from "./actionMessage";
import type { UiBackend } from "./backend";
import type { SetupStep } from "./setup";
import { SetupCheck, SetupCheckPanel } from "./SetupCheck";
import type { SetupCheckState } from "./useSetupCheck";
import { ThisPcDevices } from "./ThisPcDevices";
import { VirtualCablePanel } from "./VirtualCablePanel";
import { useMessages } from "./i18n/useMessages";

import { setupCheckRefreshKey } from "./setupCheckRefreshKey";

export function SetupWorkbench({
  section,
  backend,
  devices,
  refreshDevices,
  setupSteps,
  cableRoute,
  onAddCablePassThrough,
  sessions,
  onActionMessage,
  runningSessionIds = [],
  setupCheck,
  onOpenFinding,
}: {
  section: "status" | "virtualCables";
  backend: UiBackend;
  devices: DeviceListItem[];
  refreshDevices: ({ announce }?: { announce?: boolean }) => void;
  setupSteps: SetupStep[];
  cableRoute?: DiagnosticsSnapshot["cableRoute"];
  onAddCablePassThrough?: () => void;
  sessions: Session[];
  onActionMessage?: (message: UiMessage) => void;
  runningSessionIds?: readonly string[];
  setupCheck?: SetupCheckState;
  onOpenFinding?: (finding: SetupFinding) => void;
}) {
  const tr = useMessages();
  // Stable across renders so the panel probes the shell once, not per refresh.
  const cableManagementAvailable = useMemo(() => backend.virtualCableManagementAvailable?.bind(backend), [backend]);
  const checkSetup = useMemo(() => backend.checkSetup?.bind(backend), [backend]);
  // The setup check runs again when a device's format or a saved session
  // changes; a primitive key so telemetry refreshes do not restart it.
  const setupCheckKey = useMemo(
    () => setupCheckRefreshKey(devices, sessions, runningSessionIds),
    [devices, sessions, runningSessionIds],
  );
  return (
    <>
      {section === "status" ? (
        <>
          <section className="panel setup-panel" aria-labelledby="setup-status-heading">
            <h3 id="setup-status-heading">{tr("setupWorkbench.readiness")}</h3>
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
          {setupCheck ? (
            <SetupCheckPanel state={setupCheck} onOpenFinding={onOpenFinding} />
          ) : (
            <SetupCheck
              connected={backend.connected}
              checkSetup={checkSetup}
              refreshKey={setupCheckKey}
              onOpenFinding={onOpenFinding}
            />
          )}
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
            virtualCableManagementAvailable={cableManagementAvailable}
            onActionMessage={onActionMessage}
          />

          <section className="panel setup-panel" aria-labelledby="setup-other-apps-heading">
            <h3 id="setup-other-apps-heading">{tr("setupWorkbench.otherAppsHeading")}</h3>
            <p className="muted">{tr("setupWorkbench.otherAppsHelp")}</p>
          </section>
        </>
      )}
    </>
  );
}
