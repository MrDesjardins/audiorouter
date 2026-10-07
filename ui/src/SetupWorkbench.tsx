// Content of the Setup side-panel tab (moved from App.tsx).
import type { Dispatch, SetStateAction } from "react";
import type { DeviceListItem } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import type { WorkbenchTab } from "./Workbench";
import type { SetupStep } from "./setup";
import type { FlowAnimationMode } from "./preferences";
import type { AvailableUpdate } from "./updateCheck";
import { UpdatesPanel } from "./UpdateNotice";
import { setUpdateCheckEnabled } from "./updateCheck";
import { FlowAnimationSetting } from "./FlowAnimationSetting";
import { ThisPcDevices } from "./ThisPcDevices";
import { FirstRunGuide } from "./FirstRunGuide";
import { DeviceAccessSetting } from "./DeviceAccess";

export function SetupWorkbench({
  backend,
  devices,
  refreshDevices,
  setWorkbenchTab,
  setupSteps,
  flowAnimation,
  changeFlowAnimation,
  updateChecks,
  setUpdateChecks,
  availableUpdate,
}: {
  backend: UiBackend;
  devices: DeviceListItem[];
  refreshDevices: ({ announce }?: { announce?: boolean }) => void;
  setWorkbenchTab: Dispatch<SetStateAction<WorkbenchTab>>;
  setupSteps: SetupStep[];
  flowAnimation: FlowAnimationMode;
  changeFlowAnimation: (mode: FlowAnimationMode) => void;
  updateChecks: boolean;
  setUpdateChecks: Dispatch<SetStateAction<boolean>>;
  availableUpdate: AvailableUpdate | null;
}) {
  return (
    <>
      <DeviceAccessSetting backend={backend} />
      <FirstRunGuide
        devices={devices}
        connected={backend.connected}
        onRefresh={() => refreshDevices()}
        onOpenTools={() => setWorkbenchTab("tools")}
      />
      <section className="panel setup-panel" aria-labelledby="setup-status-heading">
        <h3 id="setup-status-heading">Status</h3>
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
      <section className="panel setup-panel" aria-labelledby="setup-other-apps-heading">
        <h3 id="setup-other-apps-heading">Use with other apps</h3>
        <p className="muted">
          AudioRouter does not change settings in Windows or in other apps. To send your processed microphone to
          Discord, OBS or a game, end the route in an Output Device set to a virtual cable (for example CABLE Input),
          then choose the matching input (CABLE Output) as the microphone in that app.
        </p>
      </section>
      <FlowAnimationSetting mode={flowAnimation} onChange={changeFlowAnimation} />
      <UpdatesPanel
        enabled={updateChecks}
        onChange={(enabled) => {
          setUpdateCheckEnabled(enabled);
          setUpdateChecks(enabled);
        }}
        update={availableUpdate}
      />
    </>
  );
}
