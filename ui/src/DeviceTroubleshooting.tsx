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
  return (
    <>
      <p className="muted">
        Devices chosen here are used only when a single route's Input Device or Output Device node has no device of its
        own.
      </p>
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
        <summary>Multiple capture devices</summary>
        <NativeMultiInputPanel
          backend={backend}
          sessionId={session.id}
          devices={devices}
          sessionRunning={sessionRunning}
          applicationNodes={session.nodes}
        />
      </details>
      <details>
        <summary>Multiple output devices</summary>
        <NativeOutputFanoutPanel
          backend={backend}
          sessionId={session.id}
          devices={devices}
          sessionRunning={sessionRunning}
        />
      </details>
      <details>
        <summary>Managed virtual devices and routes</summary>
        <VirtualDeviceLifecyclePanel backend={backend} onAddVirtualBusNode={addVirtualBusNode} />
        <VirtualRoutePanel backend={backend} />
      </details>
    </>
  );
}
