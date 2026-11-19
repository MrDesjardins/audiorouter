// Manual device binding controls of the Advanced tab (moved from App.tsx).
import type { Dispatch, SetStateAction } from "react";
import type { DeviceListItem, Session } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import {
  NativeEndpointPanel,
  NativeMultiInputPanel,
  NativeOutputFanoutPanel,
  VirtualDeviceLifecyclePanel,
  VirtualRoutePanel,
} from "./NativeDevicePanels";
import { useMessages } from "./i18n/useMessages";

export function DeviceTroubleshooting({
  backend,
  session,
  devices,
  sessionRunning,
  startSession,
  stopSession,
  addEndpointLoopback,
  captureEndpointId,
  setCaptureEndpointId,
  renderEndpointId,
  setRenderEndpointId,
  addVirtualBusNode,
}: {
  backend: UiBackend;
  session: Session;
  devices: DeviceListItem[];
  sessionRunning: boolean;
  startSession: () => Promise<boolean>;
  stopSession: (propagateFailure?: boolean) => Promise<void>;
  addEndpointLoopback: (endpointId: string) => void;
  captureEndpointId: string;
  setCaptureEndpointId: Dispatch<SetStateAction<string>>;
  renderEndpointId: string;
  setRenderEndpointId: Dispatch<SetStateAction<string>>;
  addVirtualBusNode: (busId: string, direction: "renderSource" | "captureSink") => string | undefined;
}) {
  const tr = useMessages();
  return (
    <>
      <p className="muted">{tr("deviceTroubleshooting.bindingHelp")}</p>
      <NativeEndpointPanel
        backend={backend}
        sessionId={session.id}
        devices={devices}
        sessionRunning={sessionRunning}
        onStart={async () => {
          await startSession();
        }}
        onStop={stopSession}
        onAddEndpointLoopback={addEndpointLoopback}
        captureEndpointId={captureEndpointId}
        setCaptureEndpointId={setCaptureEndpointId}
        renderEndpointId={renderEndpointId}
        setRenderEndpointId={setRenderEndpointId}
      />
      <details>
        <summary>{tr("deviceTroubleshooting.multipleCapture")}</summary>
        <NativeMultiInputPanel
          backend={backend}
          sessionId={session.id}
          devices={devices}
          sessionRunning={sessionRunning}
          applicationNodes={session.nodes}
        />
      </details>
      <details>
        <summary>{tr("deviceTroubleshooting.multipleOutput")}</summary>
        <NativeOutputFanoutPanel
          backend={backend}
          sessionId={session.id}
          devices={devices}
          sessionRunning={sessionRunning}
        />
      </details>
      <details>
        <summary>{tr("deviceTroubleshooting.managedDevices")}</summary>
        <VirtualDeviceLifecyclePanel backend={backend} onAddVirtualBusNode={addVirtualBusNode} />
        <VirtualRoutePanel backend={backend} />
      </details>
    </>
  );
}
