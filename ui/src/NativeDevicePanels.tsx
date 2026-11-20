// Manual device-binding and virtual-device panels of the Advanced tab (moved from App.tsx).
import { useEffect, useEffectEvent, useRef, useState } from "react";
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
import { boldMarkup } from "./boldMarkup";
import { useMessages } from "./i18n/useMessages";

export function VirtualDeviceLifecyclePanel({
  backend,
  onAddVirtualBusNode,
}: {
  backend: UiBackend;
  onAddVirtualBusNode: (busId: string, direction: "renderSource" | "captureSink") => void;
}) {
  const tr = useMessages();
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
        if (generation === refreshGeneration.current) setMessage(formatUiError(error, tr("g2.vdInventoryFail")));
      });
  };
  // Refresh when the backend connects or changes, with the latest state.
  const refreshOnChange = useEffectEvent(() => refresh());
  useEffect(() => {
    if (backend.connected) refreshOnChange();
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
      setMessage(tr("g2.vdNeedIdName"));
      return;
    }
    setBusy(true);
    setMessage(tr("g2.vdPlanning"));
    try {
      const result = await backend.planVirtualDevice(operation);
      setPlan(result);
      setMessage(result.availability.reason);
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.vdPlanFail")));
    } finally {
      setBusy(false);
    }
  };
  const applyPlan = async () => {
    if (!plan || busy || !backend.connected) return;
    setBusy(true);
    setMessage(tr("g2.vdApplying"));
    try {
      const result = await backend.applyVirtualDevice(plan.planId, uiIdempotencyKey("virtual-device-apply"));
      setPlan(null);
      setMessage(tr("g2.vdApplied", { reason: result.availability.reason }));
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.vdApplyFail")));
    } finally {
      setBusy(false);
    }
  };
  const provision = async () => {
    if (!selected || busy || !backend.connected || !instanceId.trim()) return;
    setBusy(true);
    setMessage(tr("g2.vdProvisioning"));
    try {
      const result = await backend.provisionVirtualDevice(
        selected.id,
        instanceId.trim(),
        uiIdempotencyKey("virtual-device-provision"),
      );
      setMessage(tr("g2.vdProvisioned", { reason: result.availability.reason, id: result.driverInstanceId }));
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.vdProvisionFail")));
    } finally {
      setBusy(false);
    }
  };
  const remove = async () => {
    if (!selected || busy || !backend.connected) return;
    setBusy(true);
    setMessage(tr("g2.vdRemoving"));
    try {
      const result = await backend.removeVirtualDevice(selected.id, uiIdempotencyKey("virtual-device-remove"));
      setMessage(tr("g2.vdRemoved", { reason: result.availability.reason }));
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.vdRemoveFail")));
    } finally {
      setBusy(false);
    }
  };
  const planLabel = {
    create: tr("g2.vdPlanCreate"),
    rename: tr("g2.vdPlanRename"),
    setEnabled: tr("g2.vdPlanSetEnabled"),
    delete: tr("g2.vdPlanDelete"),
  }[action];
  return (
    <section className="panel virtual-device-lifecycle" aria-labelledby="virtual-device-lifecycle-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g2.vdEyebrow")}</p>
          <h2 id="virtual-device-lifecycle-heading">{tr("g2.vdTitle")}</h2>
        </div>
        <button type="button" className="secondary" onClick={refresh} disabled={!backend.connected || busy}>
          {tr("g2.refresh")}
        </button>
      </div>
      {devices.length === 0 ? (
        <p className="muted">{tr("g2.vdNone")}</p>
      ) : (
        <ul aria-label={tr("g2.vdListAria")}>
          {devices.map((device) => (
            <li key={device.id}>
              <strong>{device.name}</strong>{" "}
              <small>
                {tr("g2.vdItem", {
                  state: device.enabled ? tr("g2.enabled") : tr("g2.disabled"),
                  reason: device.availability.reason,
                  lease: device.leaseOwner ?? tr("g2.none"),
                })}
              </small>
            </li>
          ))}
        </ul>
      )}
      <fieldset disabled={!backend.connected || busy}>
        <legend>{tr("g2.vdOperation")}</legend>
        <label>
          {tr("g2.vdAction")}
          <select
            aria-label={tr("g2.vdActionAria")}
            value={action}
            onChange={(event) => chooseAction(event.target.value as typeof action)}
          >
            <option value="create">{tr("g2.vdCreate")}</option>
            <option value="rename" disabled={!selected}>
              {tr("g2.vdRename")}
            </option>
            <option value="setEnabled" disabled={!selected}>
              {tr("g2.vdEnableDisable")}
            </option>
            <option value="delete" disabled={!selected}>
              {tr("g2.vdDelete")}
            </option>
          </select>
        </label>
        {action !== "create" && (
          <label>
            {tr("g2.vdExisting")}
            <select
              aria-label={tr("g2.vdExistingAria")}
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
          {tr("g2.vdBusId")}
          <input
            value={busId}
            maxLength={64}
            disabled={action !== "create"}
            onChange={(event) => setBusId(event.target.value)}
          />
        </label>
        {action !== "delete" && (
          <label>
            {tr("g2.vdBusName")}
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
          {planLabel}
        </button>
        {plan && (
          <button
            type="button"
            className="secondary"
            onClick={() => void applyPlan()}
            disabled={!backend.connected || busy}
          >
            {tr("g2.vdApplyPlan")}
          </button>
        )}
      </fieldset>
      <fieldset disabled={!backend.connected || busy || !selected}>
        <legend>{tr("g2.vdGraphNodes")}</legend>
        <div className="actions">
          <button
            type="button"
            className="secondary"
            onClick={() => onAddVirtualBusNode(selected.id, "renderSource")}
            disabled={!backend.connected || busy || !selected}
          >
            {tr("g2.vdAddSource")}
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() => onAddVirtualBusNode(selected.id, "captureSink")}
            disabled={!backend.connected || busy || !selected}
          >
            {tr("g2.vdAddSink")}
          </button>
        </div>
        <p className="muted">{tr("g2.vdNodesNote")}</p>
      </fieldset>
      <fieldset disabled={!backend.connected || busy || !selected}>
        <legend>{tr("g2.vdOwnership")}</legend>
        <label>
          {tr("g2.vdInstanceId")}
          <input
            aria-label={tr("g2.vdInstanceAria")}
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
            {tr("g2.vdProvision")}
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() => void remove()}
            disabled={!backend.connected || busy || !selected}
          >
            {tr("g2.vdRemove")}
          </button>
        </div>
      </fieldset>
      {message && <PanelMessage message={message} />}
      <p className="muted">{boldMarkup(tr("g2.vdFootnote"))}</p>
    </section>
  );
}

export function VirtualRoutePanel({ backend }: { backend: UiBackend }) {
  const tr = useMessages();
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
        if (generation === refreshGeneration.current) setMessage(formatUiError(error, tr("g2.vrInventoryFail")));
      });
  };
  // Refresh when the backend connects or changes, with the latest state.
  const refreshOnChange = useEffectEvent(() => refresh());
  useEffect(() => {
    if (backend.connected) refreshOnChange();
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
      setMessage(tr("g2.vrRevisionInvalid"));
      return;
    }
    let routes: import("@audiorouter/contracts").VirtualBusRoute[];
    try {
      const parsed: unknown = JSON.parse(routeText);
      if (!Array.isArray(parsed)) throw new Error(tr("g2.vrNotArray"));
      routes = parsed as import("@audiorouter/contracts").VirtualBusRoute[];
    } catch (error) {
      setMessage(error instanceof Error ? error.message : tr("g2.vrJsonInvalid"));
      return;
    }
    setBusy(true);
    setMessage(tr("g2.vrReplacing"));
    try {
      const result = await backend.replaceVirtualRoutes(
        baseRevision,
        routes,
        uiIdempotencyKey("virtual-routes-replace"),
      );
      setState({ revision: result.revision, routes: result.routes });
      setRouteText(JSON.stringify(result.routes, null, 2));
      setRevisionText(String(result.revision));
      setMessage(tr("g2.vrReplaced", { state: result.state, revision: result.revision }));
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.vrReplaceFail")));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel virtual-route-panel" aria-labelledby="virtual-route-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g2.vrEyebrow")}</p>
          <h2 id="virtual-route-heading">{tr("g2.vrTitle")}</h2>
        </div>
        <div className="actions">
          <span className="badge">{tr("g2.vrRev", { n: state?.revision ?? "-" })}</span>
          <button type="button" className="secondary" onClick={refresh} disabled={!backend.connected || busy}>
            {tr("g2.refresh")}
          </button>
        </div>
      </div>
      <p className="muted">{tr("g2.vrIntro")}</p>
      {state?.routes.length ? (
        <ul aria-label={tr("g2.vrListAria")}>
          {state.routes.map((route) => (
            <li key={`${route.busId}-${route.producerSessionId}-${route.consumerSessionId}`}>
              <code>{route.busId}</code> · {route.producerSessionId} → {route.consumerSessionId}
            </li>
          ))}
        </ul>
      ) : (
        <p className="muted">{tr("g2.vrNone")}</p>
      )}
      <fieldset disabled={!backend.connected || busy}>
        <legend>{tr("g2.vrReplacement")}</legend>
        <label>
          {tr("g2.vrBaseRevision")}
          <input
            aria-label={tr("g2.vrBaseRevisionAria")}
            inputMode="numeric"
            value={revisionText}
            onChange={(event) => setRevisionText(event.target.value)}
          />
        </label>
        <label>
          {tr("g2.vrRoutesJson")}
          <textarea
            aria-label={tr("g2.vrJsonAria")}
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
          {tr("g2.vrReplace")}
        </button>
      </fieldset>
      <PanelMessage message={message} />
      <p className="muted">{boldMarkup(tr("g2.vrFootnote"))}</p>
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
  const tr = useMessages();
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
  }, [sessionId, setCaptureEndpointId, setRenderEndpointId]);
  useEffect(() => {
    if (devices.length === 0) return;
    const isActive = (id: string, direction: "capture" | "render") =>
      devices.some((device) => device.id === id && device.state === "active" && device.direction === direction);
    // An empty value with a saved hint is deliberate: it represents a stale
    // binding awaiting replacement. Do not let an inventory effect scheduled
    // before a button click overwrite a newly selected pair.
    if (captureEndpointId && !isActive(captureEndpointId, "capture")) setCaptureEndpointId("");
    if (renderEndpointId && !isActive(renderEndpointId, "render")) setRenderEndpointId("");
  }, [devices, sessionId, captureEndpointId, renderEndpointId, setCaptureEndpointId, setRenderEndpointId]);
  const prepare = async () => {
    if (busy || !backend.connected) return;
    if (!backend.prepareNativeEndpoint) {
      setMessage(tr("g2.neUnavailable"));
      return;
    }
    if (!captureEndpointId || !renderEndpointId) {
      setMessage(tr("g2.neSelectBoth"));
      return;
    }
    setBusy(true);
    setMessage(tr("g2.nePreparing"));
    try {
      const result = await backend.prepareNativeEndpoint(sessionId, captureEndpointId, renderEndpointId);
      setMessage(tr("g2.nePrepared", { state: result.state }));
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.nePrepareFail")));
    } finally {
      setBusy(false);
    }
  };
  const rebind = async () => {
    if (busy || !backend.connected) return;
    if (!backend.rebindNativeEndpoint) {
      setMessage(tr("g2.neRebindUnavailable"));
      return;
    }
    if (!captureEndpointId || !renderEndpointId) {
      setMessage(tr("g2.neSelectBothRebind"));
      return;
    }
    setBusy(true);
    setMessage(tr("g2.neRebinding"));
    try {
      const result = await backend.rebindNativeEndpoint(sessionId, captureEndpointId, renderEndpointId);
      setMessage(tr("g2.neRebound", { state: result.state }));
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.neRebindFail")));
    } finally {
      setBusy(false);
    }
  };
  const detach = async () => {
    if (busy || !backend.connected) return;
    if (!backend.detachNativeEndpoint) {
      setMessage(tr("g2.neDetachUnavailable"));
      return;
    }
    setBusy(true);
    setMessage(tr("g2.neDetaching"));
    try {
      const result = await backend.detachNativeEndpoint(sessionId);
      setMessage(tr("g2.neDetached", { state: result.state }));
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.neDetachFail")));
    } finally {
      setBusy(false);
    }
  };
  const selectVbCable = () => {
    if (!vbCablePair) {
      setMessage(tr("g2.neNoVbPair"));
      return;
    }
    setCaptureEndpointId(vbCablePair.captureEndpointId);
    setRenderEndpointId(vbCablePair.renderEndpointId);
    writeEndpointBindingHint(sessionId, vbCablePair.captureEndpointId, vbCablePair.renderEndpointId);
    setMessage(tr("g2.neVbPairSelected"));
  };
  const selectVbCableCapture = () => {
    if (!vbCableCaptureEndpointId) {
      setMessage(tr("g2.neNoVbCapture"));
      return;
    }
    setCaptureEndpointId(vbCableCaptureEndpointId);
    writeEndpointBindingHint(sessionId, vbCableCaptureEndpointId, renderEndpointId);
    setMessage(tr("g2.neVbCaptureSelected"));
  };
  return (
    <section
      id="native-endpoint-panel"
      className="panel native-endpoint-panel"
      aria-labelledby="native-endpoint-heading"
    >
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g2.neEyebrow")}</p>
          <h2 id="native-endpoint-heading">{tr("g2.neTitle")}</h2>
        </div>
        <span className="badge">{sessionRunning ? tr("g2.neRunning") : tr("g2.neStopped")}</span>
      </div>
      <ol className="native-audio-steps">
        <li>{tr("g2.neStep1")}</li>
        <li>{tr("g2.neStep2")}</li>
        <li>{tr("g2.neStep3")}</li>
      </ol>
      {missingCaptureHint && (
        <p className="muted" role="status">
          {tr("g2.neMissingCapture")}
        </p>
      )}
      {missingRenderHint && (
        <p className="muted" role="status">
          {tr("g2.neMissingRender")}
        </p>
      )}
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={selectVbCableCapture}
          disabled={!backend.connected || !vbCableCaptureEndpointId || sessionRunning}
          title={vbCableCaptureEndpointId ? tr("g2.neTitleVbCapture") : tr("g2.neTitleNoVbCapture")}
        >
          {tr("g2.neSelectVbCapture")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={selectVbCable}
          disabled={!backend.connected || !vbCablePair || sessionRunning}
          title={vbCablePair ? tr("g2.neTitleVbPair") : tr("g2.neTitleNoVbPair")}
        >
          {tr("g2.neSelectVbPair")}
        </button>
        {vbCablePair && <small>{tr("g2.neLoopbackDetected")}</small>}
      </div>
      <label>
        {tr("g2.neCaptureEndpoint")}
        <select
          aria-label={tr("g2.neCaptureAria")}
          value={captureEndpointId}
          disabled={!backend.connected || activeCapture.length === 0 || sessionRunning}
          onChange={(event) => {
            setCaptureEndpointId(event.target.value);
            writeEndpointBindingHint(sessionId, event.target.value, renderEndpointId);
          }}
        >
          <option value="">{tr("g2.neSelectCapture")}</option>
          {activeCapture.map((device) => (
            <option key={device.id} value={device.id}>
              {deviceChoiceLabel(device)}
            </option>
          ))}
        </select>
      </label>
      <label>
        {tr("g2.neRenderEndpoint")}
        <select
          aria-label={tr("g2.neRenderAria")}
          value={renderEndpointId}
          disabled={!backend.connected || activeRender.length === 0 || sessionRunning}
          onChange={(event) => {
            setRenderEndpointId(event.target.value);
            writeEndpointBindingHint(sessionId, captureEndpointId, event.target.value);
          }}
        >
          <option value="">{tr("g2.neSelectRender")}</option>
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
              setMessage(tr("g2.neNeedRenderLoopback"));
              return;
            }
            onAddEndpointLoopback(renderEndpointId);
          }}
          disabled={!backend.connected || !renderEndpointId || sessionRunning}
        >
          {tr("g2.neAddLoopback")}
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
          {tr("g2.nePrepare")}
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
          {tr("g2.neRebind")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void detach()}
          disabled={!backend.connected || !backend.detachNativeEndpoint || sessionRunning}
        >
          {tr("g2.neDetach")}
        </button>
        <button
          type="button"
          className={sessionRunning ? "secondary" : "primary"}
          onClick={() => void (sessionRunning ? onStop() : onStart())}
          disabled={!backend.connected}
        >
          {sessionRunning ? tr("g2.neStopSession") : tr("g2.neStartSession")}
        </button>
      </div>
      <PanelMessage message={message} />
      <p className="muted">{boldMarkup(tr("g2.neFootnote"))}</p>
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
  const tr = useMessages();
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
      setMessage(tr("g2.nfUnavailable"));
      return;
    }
    const parsedGeneration = Number(generation);
    if (!Number.isSafeInteger(parsedGeneration) || parsedGeneration < 1) {
      setMessage(tr("g2.nfGeneration"));
      return;
    }
    if (endpointIds.length < 1 || endpointIds.length > 8) {
      setMessage(tr("g2.nfSelectRange"));
      return;
    }
    setBusy(true);
    setMessage(tr("g2.nfPreparing"));
    try {
      const result = await backend.prepareNativeOutputs(sessionId, parsedGeneration, endpointIds);
      setMessage(tr("g2.nfPrepared", { n: result.outputCount, generation: result.generation }));
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.nfPrepareFail")));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel native-output-fanout-panel" aria-labelledby="native-output-fanout-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g2.nfEyebrow")}</p>
          <h2 id="native-output-fanout-heading">{tr("g2.nfTitle")}</h2>
        </div>
        <span className="badge">{endpointIds.length}/8</span>
      </div>
      <p className="muted">{tr("g2.nfIntro")}</p>
      <label>
        {tr("g2.nfGenerationLabel")}
        <input
          aria-label={tr("g2.nfGenerationAria")}
          type="number"
          min={1}
          step={1}
          value={generation}
          disabled={!backend.connected || sessionRunning || busy}
          onChange={(event) => setGeneration(event.target.value)}
        />
      </label>
      <label>
        {tr("g2.nfBranches")}
        <select
          aria-label={tr("g2.nfBranchesAria")}
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
        {tr("g2.nfPrepare")}
      </button>
      <PanelMessage message={message} />
      <p className="muted">{tr("g2.nfFootnote")}</p>
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
  const tr = useMessages();
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
    return tr("g2.nmApplicationSuffix", { name: node ? node.name : entry.id });
  };
  const prepare = async () => {
    if (busy || !backend.connected) return;
    if (!backend.prepareNativeMultiInputs) {
      setMessage(tr("g2.nmUnavailable"));
      return;
    }
    const parsedGeneration = Number(generation);
    if (!Number.isSafeInteger(parsedGeneration) || parsedGeneration < 1) {
      setMessage(tr("g2.nfGeneration"));
      return;
    }
    if (sources.length < 2 || sources.length > 8) {
      setMessage(tr("g2.nmSelectRange"));
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
          throw new Error(tr("g2.nmNoIdentity", { id: entry.id }));
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
      setMessage(error instanceof Error ? error.message : tr("g2.nmBindFail"));
      return;
    }
    setBusy(true);
    setMessage(tr("g2.nmPreparing"));
    try {
      const result = await backend.prepareNativeMultiInputs(sessionId, parsedGeneration, bindings);
      setMessage(tr("g2.nmPrepared", { n: result.sources.length, generation: result.generation }));
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.nmPrepareFail")));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel native-multi-input-panel" aria-labelledby="native-multi-input-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g2.nmEyebrow")}</p>
          <h2 id="native-multi-input-heading">{tr("g2.nmTitle")}</h2>
        </div>
        <span className="badge">{sources.length}/8</span>
      </div>
      <p className="muted">{tr("g2.nmIntro")}</p>
      <label>
        {tr("g2.nfGenerationLabel")}
        <input
          aria-label={tr("g2.nmGenerationAria")}
          type="number"
          min={1}
          step={1}
          value={generation}
          disabled={!backend.connected || sessionRunning || busy}
          onChange={(event) => setGeneration(event.target.value)}
        />
      </label>
      <label>
        {tr("g2.nmPhysical")}
        <select
          aria-label={tr("g2.nmPhysicalAria")}
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
        {tr("g2.nmApplications")}
        <select
          aria-label={tr("g2.nmApplicationsAria")}
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
        {tr("g2.nmPolicy")}
        <select
          aria-label={tr("g2.nmPolicyAria")}
          value={mode}
          disabled={!backend.connected || sessionRunning || busy}
          onChange={(event) => setMode(event.target.value as "include" | "exclude")}
        >
          <option value="include">{tr("g2.nmInclude")}</option>
          <option value="exclude">{tr("g2.nmExclude")}</option>
        </select>
      </label>
      {sources.length > 0 && (
        <fieldset disabled={sessionRunning || busy}>
          <legend>{tr("g2.nmOrder")}</legend>
          <ol aria-label={tr("g2.nmOrderAria")}>
            {sources.map((entry, index) => (
              <li key={`${entry.kind}-${entry.id}`}>
                <span>{describe(entry)}</span>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => reorder(index, -1)}
                  disabled={index === 0}
                  aria-label={tr("g2.nmMoveUpAria", { id: entry.id })}
                >
                  {tr("g2.nmMoveUp")}
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => reorder(index, 1)}
                  disabled={index === sources.length - 1}
                  aria-label={tr("g2.nmMoveDownAria", { id: entry.id })}
                >
                  {tr("g2.nmMoveDown")}
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
        {tr("g2.nmPrepare")}
      </button>
      <PanelMessage message={message} />
      <p className="muted">{tr("g2.nmFootnote")}</p>
    </section>
  );
}
