// Manual device-binding and virtual-device panels of the Advanced tab (moved from App.tsx).
import { useEffect, useRef, useState } from "react";
import type { Node } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";
import type { DeviceListItem } from "@audiorouter/contracts";
import { uiIdempotencyKey } from "./idempotency";
import { PanelMessage } from "./PanelMessage";
import {
  deviceChoiceLabel,
  findVbCableCaptureEndpointId,
  findVbCableEndpointPair,
  readEndpointBindingHint,
  sortDevicesAlphabetically,
  writeEndpointBindingHint,
} from "./endpointBinding";

export function VirtualDeviceLifecyclePanel({
  backend,
  onAddVirtualBusNode,
}: {
  backend: UiBackend;
  onAddVirtualBusNode: (busId: string, direction: "renderSource" | "captureSink") => void;
}) {
  const [devices, setDevices] = useState<import("@audiorouter/contracts").VirtualDeviceInfo[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [action, setAction] = useState<"create" | "rename" | "setEnabled" | "delete">("create");
  const [busId, setBusId] = useState("virtual-bus");
  const [busName, setBusName] = useState("AudioRouter Bus");
  const [instanceId, setInstanceId] = useState("bus-virtual-bus");
  const [plan, setPlan] = useState<import("@audiorouter/contracts").VirtualDevicePlanResult | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const refreshGeneration = useRef(0);
  const refresh = () => {
    const generation = ++refreshGeneration.current;
    setPlan(null);
    void backend
      .listVirtualDevices()
      .then((items) => {
        if (generation !== refreshGeneration.current) return;
        setDevices(items);
        if (!items.some((item) => item.id === selectedId)) {
          const next = items[0];
          setSelectedId(next?.id ?? "");
          setBusId(next?.id ?? "virtual-bus");
          setBusName(next?.name ?? "AudioRouter Bus");
        }
      })
      .catch((error) => {
        if (generation === refreshGeneration.current)
          setMessage(formatUiError(error, "Virtual-device inventory unavailable."));
      });
  };
  useEffect(() => {
    if (backend.connected) refresh();
    else {
      setDevices([]);
      setPlan(null);
    }
  }, [backend, backend.connected]);
  // The buttons that use this value are disabled until an inventory row exists;
  // the non-null assertion keeps their guarded event handlers type-safe.
  const selected = devices.find((device) => device.id === selectedId) ?? devices[0]!;
  const chooseAction = (next: "create" | "rename" | "setEnabled" | "delete") => {
    setAction(next);
    setPlan(null);
    if (next === "create") {
      setBusId("virtual-bus");
      setBusName("AudioRouter Bus");
    } else if (selected) {
      setBusId(selected.id);
      setBusName(selected.name);
    }
  };
  const createPlan = async () => {
    if (busy || !backend.connected) return;
    const operation: import("@audiorouter/contracts").VirtualDeviceOperation =
      action === "create"
        ? { action, id: busId.trim(), name: busName.trim() }
        : action === "rename"
          ? { action, id: selectedId, name: busName.trim() }
          : action === "setEnabled"
            ? { action, id: selectedId, enabled: !(selected?.enabled ?? false) }
            : { action, id: selectedId };
    if (!operation.id || ("name" in operation && !operation.name)) {
      setMessage("Provide a valid bus ID and name.");
      return;
    }
    setBusy(true);
    setMessage("Planning managed virtual-bus change...");
    try {
      const result = await backend.planVirtualDevice(operation);
      setPlan(result);
      setMessage(result.availability.reason);
    } catch (error) {
      setMessage(formatUiError(error, "Unable to plan managed virtual-bus change."));
    } finally {
      setBusy(false);
    }
  };
  const applyPlan = async () => {
    if (!plan || busy || !backend.connected) return;
    setBusy(true);
    setMessage("Applying desired virtual-bus state...");
    try {
      const result = await backend.applyVirtualDevice(plan.planId, uiIdempotencyKey("virtual-device-apply"));
      setPlan(null);
      setMessage(`${result.availability.reason}; no native endpoint is active in this build.`);
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, "Unable to apply virtual-bus plan."));
    } finally {
      setBusy(false);
    }
  };
  const provision = async () => {
    if (!selected || busy || !backend.connected || !instanceId.trim()) return;
    setBusy(true);
    setMessage("Provisioning the explicitly selected managed device...");
    try {
      const result = await backend.provisionVirtualDevice(
        selected.id,
        instanceId.trim(),
        uiIdempotencyKey("virtual-device-provision"),
      );
      setMessage(`${result.availability.reason}; device identity ${result.driverInstanceId} is retained.`);
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, "Unable to provision the managed device."));
    } finally {
      setBusy(false);
    }
  };
  const remove = async () => {
    if (!selected || busy || !backend.connected) return;
    setBusy(true);
    setMessage("Removing the explicitly selected managed device...");
    try {
      const result = await backend.removeVirtualDevice(selected.id, uiIdempotencyKey("virtual-device-remove"));
      setMessage(`${result.availability.reason}; managed device removed.`);
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, "Unable to remove the managed device."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel virtual-device-lifecycle" aria-labelledby="virtual-device-lifecycle-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Managed buses</p>
          <h2 id="virtual-device-lifecycle-heading">Virtual-device lifecycle</h2>
        </div>
        <button type="button" className="secondary" onClick={refresh} disabled={!backend.connected || busy}>
          Refresh
        </button>
      </div>
      {devices.length === 0 ? (
        <p className="muted">No managed virtual buses are currently listed.</p>
      ) : (
        <ul aria-label="Managed virtual devices">
          {devices.map((device) => (
            <li key={device.id}>
              <strong>{device.name}</strong>{" "}
              <small>
                {device.enabled ? "enabled" : "disabled"} · {device.availability.reason} · lease{" "}
                {device.leaseOwner ?? "none"}
              </small>
            </li>
          ))}
        </ul>
      )}
      <fieldset disabled={!backend.connected || busy}>
        <legend>Desired-state operation</legend>
        <label>
          Action
          <select
            aria-label="Virtual-device action"
            value={action}
            onChange={(event) => chooseAction(event.target.value as typeof action)}
          >
            <option value="create">Create</option>
            <option value="rename" disabled={!selected}>
              Rename
            </option>
            <option value="setEnabled" disabled={!selected}>
              Enable/disable
            </option>
            <option value="delete" disabled={!selected}>
              Delete
            </option>
          </select>
        </label>
        {action !== "create" && (
          <label>
            Existing bus
            <select
              aria-label="Existing virtual-device target"
              value={selectedId}
              onChange={(event) => {
                const next = devices.find((device) => device.id === event.target.value);
                setSelectedId(event.target.value);
                setBusName(next?.name ?? "");
              }}
            >
              {devices.map((device) => (
                <option key={device.id} value={device.id}>
                  {device.name}
                </option>
              ))}
            </select>
          </label>
        )}
        <label>
          Bus ID
          <input
            value={busId}
            maxLength={64}
            disabled={action !== "create"}
            onChange={(event) => setBusId(event.target.value)}
          />
        </label>
        {action !== "delete" && (
          <label>
            Bus name
            <input
              value={busName}
              maxLength={120}
              disabled={action === "setEnabled"}
              onChange={(event) => setBusName(event.target.value)}
            />
          </label>
        )}
        <button
          type="button"
          className="secondary"
          onClick={() => void createPlan()}
          disabled={!backend.connected || busy}
        >
          Plan {action}
        </button>
        {plan && (
          <button
            type="button"
            className="secondary"
            onClick={() => void applyPlan()}
            disabled={!backend.connected || busy}
          >
            Apply planned state
          </button>
        )}
      </fieldset>
      <fieldset disabled={!backend.connected || busy || !selected}>
        <legend>Graph bus nodes</legend>
        <div className="actions">
          <button
            type="button"
            className="secondary"
            onClick={() => onAddVirtualBusNode(selected.id, "renderSource")}
            disabled={!backend.connected || busy || !selected}
          >
            Add render source to graph
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() => onAddVirtualBusNode(selected.id, "captureSink")}
            disabled={!backend.connected || busy || !selected}
          >
            Add capture sink to graph
          </button>
        </div>
        <p className="muted">
          Nodes are stopped and bound to this exact bus ID; endpoint availability remains reported by the backend.
        </p>
      </fieldset>
      <fieldset disabled={!backend.connected || busy || !selected}>
        <legend>Native device ownership</legend>
        <label>
          Instance ID
          <input
            aria-label="Managed device instance ID"
            value={instanceId}
            maxLength={256}
            onChange={(event) => setInstanceId(event.target.value)}
          />
        </label>
        <div className="actions">
          <button
            type="button"
            className="secondary"
            onClick={() => void provision()}
            disabled={!backend.connected || busy || !selected || !instanceId.trim()}
          >
            Provision managed device
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() => void remove()}
            disabled={!backend.connected || busy || !selected}
          >
            Remove managed device
          </button>
        </div>
      </fieldset>
      {message && <PanelMessage message={message} />}
      <p className="muted">
        Desired state uses plan/apply. Native ownership is a separate explicit <code>deviceAdministration</code>{" "}
        operation and remains unavailable until the managed driver is loaded and qualified.
      </p>
    </section>
  );
}

export function VirtualRoutePanel({ backend }: { backend: UiBackend }) {
  const [state, setState] = useState<import("@audiorouter/contracts").VirtualRouteListResult | null>(null);
  const [routeText, setRouteText] = useState("[]");
  const [revisionText, setRevisionText] = useState("0");
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const refreshGeneration = useRef(0);
  const refresh = () => {
    const generation = ++refreshGeneration.current;
    void backend
      .listVirtualRoutes()
      .then((result) => {
        if (generation !== refreshGeneration.current) return;
        setState(result);
        setRouteText(JSON.stringify(result.routes, null, 2));
        setRevisionText(String(result.revision));
        setMessage(null);
      })
      .catch((error) => {
        if (generation === refreshGeneration.current)
          setMessage(formatUiError(error, "Virtual-route inventory unavailable."));
      });
  };
  useEffect(() => {
    if (backend.connected) refresh();
    else {
      setState(null);
      setRouteText("[]");
      setRevisionText("0");
      setMessage(null);
    }
  }, [backend, backend.connected]);
  const replace = async () => {
    if (busy || !backend.connected) return;
    const baseRevision = Number.parseInt(revisionText, 10);
    if (!Number.isSafeInteger(baseRevision) || baseRevision < 0) {
      setMessage("Base revision must be a non-negative integer.");
      return;
    }
    let routes: import("@audiorouter/contracts").VirtualBusRoute[];
    try {
      const parsed: unknown = JSON.parse(routeText);
      if (!Array.isArray(parsed)) throw new Error("route JSON must be an array");
      routes = parsed as import("@audiorouter/contracts").VirtualBusRoute[];
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Route JSON is invalid.");
      return;
    }
    setBusy(true);
    setMessage("Replacing explicit virtual-bus routes...");
    try {
      const result = await backend.replaceVirtualRoutes(
        baseRevision,
        routes,
        uiIdempotencyKey("virtual-routes-replace"),
      );
      setState({ revision: result.revision, routes: result.routes });
      setRouteText(JSON.stringify(result.routes, null, 2));
      setRevisionText(String(result.revision));
      setMessage(`Virtual routes ${result.state} at revision ${result.revision}.`);
    } catch (error) {
      setMessage(formatUiError(error, "Unable to replace virtual-bus routes."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel virtual-route-panel" aria-labelledby="virtual-route-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Explicit cross-session routing</p>
          <h2 id="virtual-route-heading">Virtual-bus routes</h2>
        </div>
        <div className="actions">
          <span className="badge">rev {state?.revision ?? "-"}</span>
          <button type="button" className="secondary" onClick={refresh} disabled={!backend.connected || busy}>
            Refresh
          </button>
        </div>
      </div>
      <p className="muted">
        Routes are replaced as one revisioned document. The backend validates bus identities, sessions, cycles,
        authorization, and idempotency before changing desired state.
      </p>
      {state?.routes.length ? (
        <ul aria-label="Explicit cross-session routes">
          {state.routes.map((route) => (
            <li key={`${route.busId}-${route.producerSessionId}-${route.consumerSessionId}`}>
              <code>{route.busId}</code> · {route.producerSessionId} → {route.consumerSessionId}
            </li>
          ))}
        </ul>
      ) : (
        <p className="muted">No explicit cross-session routes are currently listed.</p>
      )}
      <fieldset disabled={!backend.connected || busy}>
        <legend>Revisioned replacement</legend>
        <label>
          Base revision
          <input
            aria-label="Virtual-route base revision"
            inputMode="numeric"
            value={revisionText}
            onChange={(event) => setRevisionText(event.target.value)}
          />
        </label>
        <label>
          Routes JSON
          <textarea
            aria-label="Virtual-route JSON"
            value={routeText}
            onChange={(event) => setRouteText(event.target.value)}
            rows={6}
            spellCheck={false}
          />
        </label>
        <button
          type="button"
          className="secondary"
          onClick={() => void replace()}
          disabled={!backend.connected || busy}
        >
          Replace routes
        </button>
      </fieldset>
      <PanelMessage message={message} />
      <p className="muted">
        Disconnected preview mode never mutates route state. Replacement requires the backend’s{" "}
        <code>deviceAdministration</code> permission and does not activate endpoints by itself.
      </p>
    </section>
  );
}

export function NativeEndpointPanel({
  backend,
  sessionId,
  devices,
  sessionRunning,
  onStart,
  onStop,
  onAddEndpointLoopback,
  captureEndpointId,
  setCaptureEndpointId,
  renderEndpointId,
  setRenderEndpointId,
}: {
  backend: UiBackend;
  sessionId: string;
  devices: DeviceListItem[];
  sessionRunning: boolean;
  onStart: () => Promise<void>;
  onStop: () => Promise<void>;
  onAddEndpointLoopback: (endpointId: string) => void;
  captureEndpointId: string;
  setCaptureEndpointId: (value: string) => void;
  renderEndpointId: string;
  setRenderEndpointId: (value: string) => void;
}) {
  const activeCapture = sortDevicesAlphabetically(
    devices.filter(
      (device): device is Extract<DeviceListItem, { state: "active" }> =>
        device.state === "active" && device.direction === "capture",
    ),
  );
  const activeRender = sortDevicesAlphabetically(
    devices.filter(
      (device): device is Extract<DeviceListItem, { state: "active" }> =>
        device.state === "active" && device.direction === "render",
    ),
  );
  const savedHint = readEndpointBindingHint(sessionId);
  const missingCaptureHint =
    devices.length > 0 &&
    Boolean(savedHint.captureEndpointId) &&
    !activeCapture.some((device) => device.id === savedHint.captureEndpointId);
  const missingRenderHint =
    devices.length > 0 &&
    Boolean(savedHint.renderEndpointId) &&
    !activeRender.some((device) => device.id === savedHint.renderEndpointId);
  const vbCablePair = findVbCableEndpointPair(devices);
  const vbCableCaptureEndpointId = findVbCableCaptureEndpointId(devices);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const hint = readEndpointBindingHint(sessionId);
    setCaptureEndpointId(hint.captureEndpointId ?? "");
    setRenderEndpointId(hint.renderEndpointId ?? "");
  }, [sessionId]);
  useEffect(() => {
    if (devices.length === 0) return;
    // An empty value with a saved hint is deliberate: it represents a stale
    // binding awaiting replacement. Do not let an inventory effect scheduled
    // before a button click overwrite a newly selected pair.
    if (captureEndpointId && !activeCapture.some((device) => device.id === captureEndpointId)) setCaptureEndpointId("");
    if (renderEndpointId && !activeRender.some((device) => device.id === renderEndpointId)) setRenderEndpointId("");
  }, [devices, sessionId, captureEndpointId, renderEndpointId]);
  const prepare = async () => {
    if (busy || !backend.connected) return;
    if (!backend.prepareNativeEndpoint) {
      setMessage("Native endpoint preparation is unavailable in this backend.");
      return;
    }
    if (!captureEndpointId || !renderEndpointId) {
      setMessage("Select both an active capture and render endpoint.");
      return;
    }
    setBusy(true);
    setMessage("Preparing exact endpoints in stopped state...");
    try {
      const result = await backend.prepareNativeEndpoint(sessionId, captureEndpointId, renderEndpointId);
      setMessage(`Prepared ${result.state}; start the session to activate audio.`);
    } catch (error) {
      setMessage(formatUiError(error, "Native endpoint preparation failed."));
    } finally {
      setBusy(false);
    }
  };
  const rebind = async () => {
    if (busy || !backend.connected) return;
    if (!backend.rebindNativeEndpoint) {
      setMessage("Native endpoint rebinding is unavailable in this backend.");
      return;
    }
    if (!captureEndpointId || !renderEndpointId) {
      setMessage("Select both active endpoints before rebinding.");
      return;
    }
    setBusy(true);
    setMessage("Refreshing and rebinding exact endpoints in stopped state...");
    try {
      const result = await backend.rebindNativeEndpoint(sessionId, captureEndpointId, renderEndpointId);
      setMessage(`Rebound ${result.state}; start the session to activate audio.`);
    } catch (error) {
      setMessage(formatUiError(error, "Native endpoint rebind failed."));
    } finally {
      setBusy(false);
    }
  };
  const detach = async () => {
    if (busy || !backend.connected) return;
    if (!backend.detachNativeEndpoint) {
      setMessage("Native endpoint detachment is unavailable in this backend.");
      return;
    }
    setBusy(true);
    setMessage("Detaching the stopped native worker...");
    try {
      const result = await backend.detachNativeEndpoint(sessionId);
      setMessage(`Native worker ${result.state}; select new endpoints before preparing again.`);
    } catch (error) {
      setMessage(formatUiError(error, "Native endpoint detachment failed."));
    } finally {
      setBusy(false);
    }
  };
  const selectVbCable = () => {
    if (!vbCablePair) {
      setMessage("An unambiguous active VB-Cable input/output pair was not found.");
      return;
    }
    setCaptureEndpointId(vbCablePair.captureEndpointId);
    setRenderEndpointId(vbCablePair.renderEndpointId);
    writeEndpointBindingHint(sessionId, vbCablePair.captureEndpointId, vbCablePair.renderEndpointId);
    setMessage("VB-Cable pair selected. Review the graph, then prepare and start the session.");
  };
  const selectVbCableCapture = () => {
    if (!vbCableCaptureEndpointId) {
      setMessage("An unambiguous active VB-Cable capture endpoint was not found.");
      return;
    }
    setCaptureEndpointId(vbCableCaptureEndpointId);
    writeEndpointBindingHint(sessionId, vbCableCaptureEndpointId, renderEndpointId);
    setMessage("VB-Cable capture selected. Choose the physical render output, then prepare and start the session.");
  };
  return (
    <section
      id="native-endpoint-panel"
      className="panel native-endpoint-panel"
      aria-labelledby="native-endpoint-heading"
    >
      <div className="section-heading">
        <div>
          <p className="eyebrow">Native adapter</p>
          <h2 id="native-endpoint-heading">Endpoint binding</h2>
        </div>
        <span className="badge">{sessionRunning ? "running" : "stopped"}</span>
      </div>
      <ol className="native-audio-steps">
        <li>Saving is optional for a temporary preview. Save the route only when you want to keep it.</li>
        <li>
          Select the capture and render endpoints below. The current adapter needs both, including for Test Signal; no
          microphone is chosen automatically.
        </li>
        <li>
          Click Prepare native endpoints, then press Play to preview the current route. Save it in Session only if you
          want to keep these edits.
        </li>
      </ol>
      {missingCaptureHint && (
        <p className="muted" role="status">
          Saved capture endpoint is unavailable. Select a replacement deliberately.
        </p>
      )}
      {missingRenderHint && (
        <p className="muted" role="status">
          Saved render endpoint is unavailable. Select a replacement deliberately.
        </p>
      )}
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={selectVbCableCapture}
          disabled={!backend.connected || !vbCableCaptureEndpointId || sessionRunning}
          title={
            vbCableCaptureEndpointId
              ? "Select the exact active VB-Cable capture endpoint"
              : "No unambiguous active VB-Cable capture endpoint found"
          }
        >
          Select VB-Cable capture
        </button>
        <button
          type="button"
          className="secondary"
          onClick={selectVbCable}
          disabled={!backend.connected || !vbCablePair || sessionRunning}
          title={
            vbCablePair
              ? "Select the exact active VB-Cable loopback endpoints"
              : "No unambiguous active VB-Cable loopback pair found"
          }
        >
          Select VB-Cable loopback pair
        </button>
        {vbCablePair && <small>Active loopback pair detected</small>}
      </div>
      <label>
        Capture endpoint
        <select
          aria-label="Native capture endpoint"
          value={captureEndpointId}
          disabled={!backend.connected || activeCapture.length === 0 || sessionRunning}
          onChange={(event) => {
            setCaptureEndpointId(event.target.value);
            writeEndpointBindingHint(sessionId, event.target.value, renderEndpointId);
          }}
        >
          <option value="">Select capture endpoint</option>
          {activeCapture.map((device) => (
            <option key={device.id} value={device.id}>
              {deviceChoiceLabel(device)}
            </option>
          ))}
        </select>
      </label>
      <label>
        Render endpoint
        <select
          aria-label="Native render endpoint"
          value={renderEndpointId}
          disabled={!backend.connected || activeRender.length === 0 || sessionRunning}
          onChange={(event) => {
            setRenderEndpointId(event.target.value);
            writeEndpointBindingHint(sessionId, captureEndpointId, event.target.value);
          }}
        >
          <option value="">Select render endpoint</option>
          {activeRender.map((device) => (
            <option key={device.id} value={device.id}>
              {deviceChoiceLabel(device)}
            </option>
          ))}
        </select>
      </label>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={() => {
            if (!renderEndpointId) {
              setMessage("Select an exact active render endpoint before adding an endpoint-loopback source.");
              return;
            }
            onAddEndpointLoopback(renderEndpointId);
          }}
          disabled={!backend.connected || !renderEndpointId || sessionRunning}
        >
          Add loopback source to graph
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void prepare()}
          disabled={
            !backend.connected ||
            !backend.prepareNativeEndpoint ||
            !captureEndpointId ||
            !renderEndpointId ||
            sessionRunning
          }
        >
          Prepare native endpoints
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void rebind()}
          disabled={
            !backend.connected ||
            !backend.rebindNativeEndpoint ||
            !captureEndpointId ||
            !renderEndpointId ||
            sessionRunning
          }
        >
          Rebind exact endpoints
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void detach()}
          disabled={!backend.connected || !backend.detachNativeEndpoint || sessionRunning}
        >
          Detach stopped worker
        </button>
        <button
          type="button"
          className={sessionRunning ? "secondary" : "primary"}
          onClick={() => void (sessionRunning ? onStop() : onStart())}
          disabled={!backend.connected}
        >
          {sessionRunning ? "Stop session" : "Start session"}
        </button>
      </div>
      <PanelMessage message={message} />
      <p className="muted">
        Preparation, rebinding, and detachment require <code>deviceAdministration</code>. If preparation reports
        permission denied, close the desktop shell and relaunch it from PowerShell with{" "}
        <code>$env:AUDIOROUTER_ALLOW_DEVICE_ADMIN = "1"</code> set for that process. This requires an enrolled operator
        account. Physical endpoints and existing VB-Cable devices do not require the AudioRouter virtual driver.
        Endpoint defaults, volume, and mute are never changed. Selected IDs are retained only as local UI hints; a
        missing saved ID stays unselected until you deliberately choose a replacement. Use the loopback action only for
        a deliberate CABLE Input → CABLE Output test; for normal monitoring choose a physical render endpoint.
      </p>
    </section>
  );
}

export function NativeOutputFanoutPanel({
  backend,
  sessionId,
  devices,
  sessionRunning,
}: {
  backend: UiBackend;
  sessionId: string;
  devices: DeviceListItem[];
  sessionRunning: boolean;
}) {
  const activeRender = sortDevicesAlphabetically(
    devices.filter(
      (device): device is Extract<DeviceListItem, { state: "active" }> =>
        device.state === "active" && device.direction === "render",
    ),
  );
  const [endpointIds, setEndpointIds] = useState<string[]>([]);
  const [generation, setGeneration] = useState("1");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const prepare = async () => {
    if (busy || !backend.connected) return;
    if (!backend.prepareNativeOutputs) {
      setMessage("Native output fan-out preparation is unavailable in this backend.");
      return;
    }
    const parsedGeneration = Number(generation);
    if (!Number.isSafeInteger(parsedGeneration) || parsedGeneration < 1) {
      setMessage("Generation must be a positive integer.");
      return;
    }
    if (endpointIds.length < 1 || endpointIds.length > 8) {
      setMessage("Select one to eight exact render endpoints.");
      return;
    }
    setBusy(true);
    setMessage("Preparing exact stopped render branches...");
    try {
      const result = await backend.prepareNativeOutputs(sessionId, parsedGeneration, endpointIds);
      setMessage(`Prepared ${result.outputCount} stopped render branches for generation ${result.generation}.`);
    } catch (error) {
      setMessage(formatUiError(error, "Native output fan-out preparation failed."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel native-output-fanout-panel" aria-labelledby="native-output-fanout-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Many-output routing</p>
          <h2 id="native-output-fanout-heading">Physical render fan-out</h2>
        </div>
        <span className="badge">{endpointIds.length}/8</span>
      </div>
      <p className="muted">
        Select exact active render endpoints to receive the same processed graph output. Preparation is stopped; the
        backend validates format, ownership, and generation.
      </p>
      <label>
        Graph generation
        <input
          aria-label="Native output fan-out generation"
          type="number"
          min={1}
          step={1}
          value={generation}
          disabled={!backend.connected || sessionRunning || busy}
          onChange={(event) => setGeneration(event.target.value)}
        />
      </label>
      <label>
        Render branches
        <select
          aria-label="Native output fan-out render endpoints"
          multiple
          size={Math.min(8, Math.max(3, activeRender.length))}
          value={endpointIds}
          disabled={!backend.connected || activeRender.length === 0 || sessionRunning || busy}
          onChange={(event) => setEndpointIds(Array.from(event.target.selectedOptions, (option) => option.value))}
        >
          {activeRender.map((device) => (
            <option key={device.id} value={device.id}>
              {deviceChoiceLabel(device)}
            </option>
          ))}
        </select>
      </label>
      <button
        type="button"
        className="secondary"
        onClick={() => void prepare()}
        disabled={
          !backend.connected || !backend.prepareNativeOutputs || endpointIds.length === 0 || sessionRunning || busy
        }
      >
        Prepare stopped fan-out
      </button>
      <PanelMessage message={message} />
      <p className="muted">
        This prepares explicit branches only; start/stop and graph validity remain backend-owned. It does not change
        Windows defaults or volume.
      </p>
    </section>
  );
}

export type MultiInputSourceRef = { kind: "physical"; id: string } | { kind: "application"; id: string };

export function NativeMultiInputPanel({
  backend,
  sessionId,
  devices,
  sessionRunning,
  applicationNodes,
}: {
  backend: UiBackend;
  sessionId: string;
  devices: DeviceListItem[];
  sessionRunning: boolean;
  applicationNodes: Node[];
}) {
  const activeCapture = sortDevicesAlphabetically(
    devices.filter(
      (device): device is Extract<DeviceListItem, { state: "active" }> =>
        device.state === "active" && device.direction === "capture",
    ),
  );
  const boundApplicationNodes = applicationNodes.filter(
    (node) =>
      node.kind === "applicationCapture" &&
      node.enabled &&
      node.parameters.processPolicy === "selectedInstance" &&
      typeof node.parameters.processId === "number" &&
      typeof node.parameters.creationTime100ns === "string",
  );
  const [sources, setSources] = useState<MultiInputSourceRef[]>([]);
  const [generation, setGeneration] = useState("1");
  const [mode, setMode] = useState<"include" | "exclude">("include");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const reorder = (index: number, delta: -1 | 1) => {
    setSources((current) => {
      const target = index + delta;
      if (target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  };
  const toggleSelection = (kind: MultiInputSourceRef["kind"], selectedIds: Set<string>) => {
    setSources((current) => [
      ...current.filter((entry) => entry.kind !== kind || selectedIds.has(entry.id)),
      ...Array.from(selectedIds)
        .filter((id) => !current.some((entry) => entry.kind === kind && entry.id === id))
        .map((id): MultiInputSourceRef => ({ kind, id })),
    ]);
  };
  const describe = (entry: MultiInputSourceRef): string => {
    if (entry.kind === "physical") return activeCapture.find((device) => device.id === entry.id)?.name ?? entry.id;
    const node = boundApplicationNodes.find((candidate) => candidate.id === entry.id);
    return node ? `${node.name} (application)` : `${entry.id} (application)`;
  };
  const prepare = async () => {
    if (busy || !backend.connected) return;
    if (!backend.prepareNativeMultiInputs) {
      setMessage("Native multi-input preparation is unavailable in this backend.");
      return;
    }
    const parsedGeneration = Number(generation);
    if (!Number.isSafeInteger(parsedGeneration) || parsedGeneration < 1) {
      setMessage("Generation must be a positive integer.");
      return;
    }
    if (sources.length < 2 || sources.length > 8) {
      setMessage("Select two to eight exact capture sources.");
      return;
    }
    let bindings: import("@audiorouter/contracts").NativeMultiInputSourceBinding[];
    try {
      bindings = sources.map((entry) => {
        if (entry.kind === "physical") return { kind: "physical" as const, endpointId: entry.id };
        const node = boundApplicationNodes.find((candidate) => candidate.id === entry.id);
        if (
          !node ||
          typeof node.parameters.processId !== "number" ||
          typeof node.parameters.creationTime100ns !== "string" ||
          typeof node.parameters.executable !== "string"
        ) {
          throw new Error(`Application source ${entry.id} no longer has a bound process identity.`);
        }
        return {
          kind: "application" as const,
          processId: node.parameters.processId,
          executable: node.parameters.executable,
          executablePath: typeof node.parameters.executablePath === "string" ? node.parameters.executablePath : null,
          creationTime100ns: node.parameters.creationTime100ns,
          mode,
        };
      });
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Could not build source bindings.");
      return;
    }
    setBusy(true);
    setMessage("Preparing exact stopped capture sources...");
    try {
      const result = await backend.prepareNativeMultiInputs(sessionId, parsedGeneration, bindings);
      setMessage(
        `Prepared ${result.sources.length} capture sources for generation ${result.generation}; start the session to activate routing.`,
      );
    } catch (error) {
      setMessage(formatUiError(error, "Native multi-input preparation failed."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel native-multi-input-panel" aria-labelledby="native-multi-input-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Many-input routing</p>
          <h2 id="native-multi-input-heading">Multiple capture sources</h2>
        </div>
        <span className="badge">{sources.length}/8</span>
      </div>
      <p className="muted">
        Select exact active capture endpoints and/or bound application-capture nodes, then verify their explicit order
        matches the committed mixer-input nodes. Preparation is stopped; session start binds the graph’s physical,
        virtual, recorder, tool, and pre-bound plugin stages.
      </p>
      <label>
        Graph generation
        <input
          aria-label="Native multi-input graph generation"
          type="number"
          min={1}
          step={1}
          value={generation}
          disabled={!backend.connected || sessionRunning || busy}
          onChange={(event) => setGeneration(event.target.value)}
        />
      </label>
      <label>
        Physical capture sources
        <select
          aria-label="Native multi-input capture endpoints"
          multiple
          size={Math.min(8, Math.max(3, activeCapture.length))}
          value={sources.filter((entry) => entry.kind === "physical").map((entry) => entry.id)}
          disabled={!backend.connected || activeCapture.length === 0 || sessionRunning || busy}
          onChange={(event) =>
            toggleSelection("physical", new Set(Array.from(event.target.selectedOptions, (option) => option.value)))
          }
        >
          {activeCapture.map((device) => (
            <option key={device.id} value={device.id}>
              {deviceChoiceLabel(device)}
            </option>
          ))}
        </select>
      </label>
      <label>
        Application capture sources
        <select
          aria-label="Native multi-input application sources"
          multiple
          size={Math.min(8, Math.max(3, boundApplicationNodes.length))}
          value={sources.filter((entry) => entry.kind === "application").map((entry) => entry.id)}
          disabled={!backend.connected || boundApplicationNodes.length === 0 || sessionRunning || busy}
          onChange={(event) =>
            toggleSelection("application", new Set(Array.from(event.target.selectedOptions, (option) => option.value)))
          }
        >
          {boundApplicationNodes.map((node) => (
            <option key={node.id} value={node.id}>
              {node.name} · {String(node.parameters.executable)}
            </option>
          ))}
        </select>
      </label>
      <label>
        Application capture policy
        <select
          aria-label="Native multi-input application capture policy"
          value={mode}
          disabled={!backend.connected || sessionRunning || busy}
          onChange={(event) => setMode(event.target.value as "include" | "exclude")}
        >
          <option value="include">Include selected application</option>
          <option value="exclude">Exclude selected application</option>
        </select>
      </label>
      {sources.length > 0 && (
        <fieldset disabled={sessionRunning || busy}>
          <legend>Source order</legend>
          <ol aria-label="Selected multi-input source order">
            {sources.map((entry, index) => (
              <li key={`${entry.kind}-${entry.id}`}>
                <span>{describe(entry)}</span>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => reorder(index, -1)}
                  disabled={index === 0}
                  aria-label={`Move ${entry.id} up`}
                >
                  Move up
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => reorder(index, 1)}
                  disabled={index === sources.length - 1}
                  aria-label={`Move ${entry.id} down`}
                >
                  Move down
                </button>
              </li>
            ))}
          </ol>
        </fieldset>
      )}
      <button
        type="button"
        className="secondary"
        onClick={() => void prepare()}
        disabled={
          !backend.connected || !backend.prepareNativeMultiInputs || sources.length < 2 || sessionRunning || busy
        }
      >
        Prepare stopped multi-input
      </button>
      <PanelMessage message={message} />
      <p className="muted">
        No fallback microphone or application is selected. If an exact source changes, disappears, or exits, the backend
        stops the worker and requires deliberate rebind.
      </p>
    </section>
  );
}
