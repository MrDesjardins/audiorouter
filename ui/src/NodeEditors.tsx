// Property editors of the selected node (moved from App.tsx).
import { useContext, useEffect, useRef, useState } from "react";
import type { Node, NodeKind, PluginParametersResult } from "@audiorouter/contracts";
import { formatUiError, type ApplicationRow, type UiBackend } from "./backend";
import type { DeviceListItem } from "@audiorouter/contracts";
import { applicationCaptureChoices, applicationChoiceKey, mixerInputs, mixerInputVolumeKey } from "./draft";
import { audioUploadProblem, uploadAudioMedia } from "./audioUpload";
import { optionLabel, parameterText } from "./parameterText";
import { LiveLevelBar } from "./ToolVisuals";
import { type ProcessorDescriptor } from "./processorCatalog";
import { networkTelemetryText } from "./NetworkNodeEditor";
import { NumberField } from "./NumberField";
import { BooleanField } from "./BooleanField";
import { SliderField } from "./Slider";
import { AdvancedEqEditor } from "./AdvancedEqEditor";
import { EqBackendContext, PluginParameterContext } from "./appContext";
import { deviceChoiceLabel, sortDevicesAlphabetically } from "./endpointBinding";
import { scrollBehavior } from "./reducedMotion";
import { useHiddenDevices, visibleDevices } from "./hiddenDevices";
import type { VirtualCableStatus } from "@audiorouter/contracts";
import { cableEndpointName, cableUnavailableReason } from "./virtualCableStatus";
import { useMessages } from "./i18n/useMessages";
import { tx } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";

/**
 * Whether the catalog's settings of a kind are edited by the generic list
 * below. Device and source kinds have dedicated fields (device pickers,
 * channel and spatial fields, network, file and Soundboard editors), so the
 * catalog describes them for API and MCP clients only. A Record keeps the
 * list exhaustive: a new kind fails to typecheck until it is classified.
 */
export const GENERIC_SETTINGS_EDITOR: Record<NodeKind, boolean> = {
  physicalInput: false,
  applicationCapture: false,
  endpointLoopback: false,
  physicalOutput: false,
  virtualRenderSource: false,
  virtualCaptureSink: false,
  audioFile: false,
  soundboard: false,
  networkSend: false,
  networkReceive: false,
  meter: false,
  testSignal: true,
  mixer: true,
  gain: true,
  mute: true,
  parametricEq: true,
  compressor: true,
  gate: true,
  limiter: true,
  delay: true,
  graphicEq: true,
  pitch: true,
  recorder: true,
  plugin: true,
  instrument: true,
  volume: true,
  bassTreble: true,
  dehum: true,
  declick: true,
  inputSwitch: true,
  outputSwitch: true,
  denoise: true,
  speechDenoise: true,
  firFilter: true,
  spectralGate: true,
  timeShift: true,
  duck: true,
  funVoiceChanger: true,
};

/** Duck settings the Duck editor owns (trigger mode and Siege phases). */
export const DUCK_EDITOR_PARAMETERS = new Set(["trigger", "duckMenu", "duckPrep", "duckBetweenRounds", "duckAction"]);

export function ProcessorParameterEditor({
  node,
  processors,
  nodeTypes = null,
  pluginParameters = null,
  pluginParameterError = null,
  connected,
  onChange,
  only,
  compact = false,
}: {
  node: Node;
  processors: ProcessorDescriptor[] | null;
  nodeTypes?: import("@audiorouter/contracts").DiscoveryDocument["nodeTypes"] | null;
  pluginParameters?: PluginParametersResult | null;
  pluginParameterError?: string | null;
  connected: boolean;
  onChange: (name: string, value: boolean | number | string) => void;
  /** Render only these parameters, in this order (a tool editor's section). */
  only?: readonly string[];
  /** One row per slider; the explanation moves to the tooltip and the screen-reader description. */
  compact?: boolean;
}) {
  const eqBackend = useContext(EqBackendContext);
  const tr = useMessages();
  const pluginContext = useContext(PluginParameterContext);
  if (node.kind === "parametricEq")
    return <AdvancedEqEditor node={node} backend={eqBackend} connected={connected} onChange={onChange} />;
  if (node.kind === "graphicEq" || node.kind === "inputSwitch" || node.kind === "outputSwitch") return null;
  // The Fun Voice Changer editor shows its settings by section.
  if (node.kind === "funVoiceChanger" && !only) return null;
  pluginParameters ??= pluginContext.parameters;
  pluginParameterError ??= pluginContext.error;
  if (node.kind === "plugin") {
    if (pluginParameterError)
      return (
        <p className="muted" role="status">
          {tr("g5.pluginParamsError", { error: pluginParameterError })}
        </p>
      );
    if (!pluginParameters)
      return (
        <p className="muted" role="status">
          {tr("g5.pluginParamsLoading")}
        </p>
      );
    if (pluginParameters.parameters.length === 0)
      return (
        <p className="muted" role="status">
          {tr("g5.pluginParamsNone")}
        </p>
      );
    return (
      <>
        {pluginParameters.parameters.map((parameter) => {
          const name = `pluginParameter:${parameter.parameterId}`;
          const value =
            typeof node.parameters[name] === "number" && Number.isFinite(node.parameters[name] as number)
              ? (node.parameters[name] as number)
              : parameter.defaultValue;
          return (
            <SliderField
              key={name}
              label={parameter.title}
              name={parameter.title}
              sliderName={tr("g5.pluginParamSlider", { name: parameter.title })}
              value={value}
              min={parameter.minimum}
              max={parameter.maximum}
              step={0.001}
              defaultValue={parameter.defaultValue}
              disabled={!connected}
              help={
                <small className="parameter-help">
                  {tr("g5.pluginParamNormalized", { id: parameter.parameterId })}
                </small>
              }
              onChange={(next) => onChange(name, next)}
            />
          );
        })}
      </>
    );
  }
  if (!GENERIC_SETTINGS_EDITOR[node.kind]) return null;
  const wireKind = node.kind.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
  const descriptor = processors?.find((processor) => processor.id === node.kind);
  // Parameter families such as the Mixer's `inputVolume:<nodeId>` have their own editor.
  // Learning is started and stopped with the Learn buttons, not a checkbox.
  const parameters = (
    descriptor?.parameters ??
    nodeTypes?.find((nodeType) => nodeType.type === `${wireKind}@1` || nodeType.type === `${node.kind}@1`)
      ?.parameters ??
    []
  ).filter(
    (parameter) =>
      !parameter.name.endsWith(":") &&
      parameter.namePattern === undefined &&
      !(parameter.name === "learning" && (node.kind === "denoise" || node.kind === "spectralGate")) &&
      // Node references and the Duck trigger/phase choices have dedicated pickers in the Duck editor.
      !("reference" in parameter && parameter.reference) &&
      !(node.kind === "duck" && DUCK_EDITOR_PARAMETERS.has(parameter.name)) &&
      (!only || only.includes(parameter.name)),
  );
  if (only) parameters.sort((a, b) => only.indexOf(a.name) - only.indexOf(b.name));
  if (parameters.length === 0) return null;
  return (
    <>
      {parameters.map((parameter) => {
        const value = node.parameters[parameter.name];
        const { label, help } = parameterText(node.kind, parameter.name);
        const helpText = help ? <small className="parameter-help">{help}</small> : null;
        if (parameter.type === "boolean") {
          return (
            <BooleanField
              key={parameter.name}
              label={label}
              help={helpText}
              checked={value === true}
              disabled={!connected}
              onChange={(checked) => onChange(parameter.name, checked)}
            />
          );
        }
        if (parameter.type === "string" && parameter.enum) {
          const fallback =
            typeof parameter.default === "string" && parameter.enum.includes(parameter.default)
              ? parameter.default
              : parameter.enum[0];
          const stringValue = typeof value === "string" && parameter.enum.includes(value) ? value : fallback;
          return (
            <label key={parameter.name}>
              {label}
              {helpText}
              <select
                aria-label={label}
                value={stringValue}
                disabled={!connected}
                onChange={(event) => onChange(parameter.name, event.target.value)}
              >
                {parameter.enum.map((option) => (
                  <option key={option} value={option}>
                    {optionLabel(option)}
                  </option>
                ))}
              </select>
            </label>
          );
        }
        if (parameter.type !== "number") return null;
        const fallback = typeof parameter.default === "number" ? parameter.default : 0;
        const numericValue = typeof value === "number" && Number.isFinite(value) ? value : fallback;
        const step =
          typeof parameter.step === "number" && Number.isFinite(parameter.step) && parameter.step > 0
            ? parameter.step
            : parameter.unit === "Hz"
              ? 1
              : 0.1;
        const hasRange =
          Number.isFinite(parameter.minimum) &&
          Number.isFinite(parameter.maximum) &&
          parameter.minimum! < parameter.maximum!;
        if (hasRange)
          return (
            <SliderField
              key={parameter.name}
              label={label}
              name={label}
              sliderName={tr("parameterUi.slider", { name: label })}
              value={numericValue}
              min={parameter.minimum!}
              max={parameter.maximum!}
              step={step}
              unit={parameter.unit}
              defaultValue={typeof parameter.default === "number" ? parameter.default : undefined}
              disabled={!connected}
              help={compact ? undefined : helpText}
              showScale={!compact}
              className={compact ? "is-compact" : undefined}
              description={compact ? help : undefined}
              onChange={(next) => onChange(parameter.name, next)}
            />
          );
        return (
          <label key={parameter.name}>
            <span>
              {label}
              {parameter.unit ? ` (${parameter.unit})` : ""}
            </span>
            {helpText}
            <NumberField
              aria-label={tr("g5.preciseValue", { name: label })}
              value={numericValue}
              min={parameter.minimum}
              max={parameter.maximum}
              step={step}
              disabled={!connected}
              onValue={(next) => onChange(parameter.name, next)}
            />
          </label>
        );
      })}
    </>
  );
}

/** Plain words for an application capture's connection state, keyed by the backend's state value. */
const APP_CAPTURE_STATE_TEXT: Record<string, MessageKey | undefined> = {
  connected: "g4Canvas.appStateConnected",
  "configured-stopped": "g4Canvas.appStateConfiguredStopped",
  "app-closed": "g4Canvas.appStateAppClosed",
  reconnecting: "g4Canvas.appStateReconnecting",
  ambiguous: "g4Canvas.appStateAmbiguous",
  "output-unavailable": "g4Canvas.appStateOutputUnavailable",
  unsupported: "g4Canvas.appStateUnsupported",
  failed: "g4Canvas.appStateFailed",
  "not-prepared": "g4Canvas.appStateNotPrepared",
};

export function NodeTelemetryPanel({
  node,
  snapshot,
  running,
}: {
  node: Node;
  snapshot: import("@audiorouter/contracts").DiagnosticsSnapshot | null;
  running: boolean;
}) {
  const observation = snapshot?.nodeTelemetry.find((item) => item.nodeId === node.id);
  const tr = useMessages();
  const applicationCaptureState =
    node.kind === "applicationCapture"
      ? (snapshot?.applicationCaptureStates.find((item) => item.nodeId === node.id) ?? null)
      : null;
  const reason = (snapshot?.audio.reason ?? "").toLocaleLowerCase();
  const endpointOwned = reason.includes("in use") || reason.includes("owned");
  const tiles: Array<{ label: string; value: string; detail?: string; tone?: "good" | "warn" | "bad" }> = [];
  if (running && node.enabled && !node.bypass && observation) {
    const meter = observation.meter;
    if (meter) {
      const current = meter.currentPeakDb ?? meter.peakDb;
      tiles.push({
        label: tr("g5.tileLevel"),
        value: current <= -120 ? tr("g5.tileSilent") : `${current.toFixed(1)} dB`,
        detail: current <= -120 ? tr("g5.tileNoSound") : tr("g5.tilePeakRms", { db: meter.rmsDb.toFixed(1) }),
      });
      tiles.push({
        label: tr("g5.tileClipping"),
        value: String(meter.clippedSamples),
        detail: tr("g5.tileOver0"),
        tone: meter.clippedSamples > 0 ? "warn" : "good",
      });
    }
    if (observation.processor) {
      const reduction = Math.max(0, ...observation.processor.gainReductionDb);
      if (observation.processor.gainReductionDb.length > 0)
        tiles.push({ label: tr("g5.tileGainReduction"), value: `${reduction.toFixed(1)} dB` });
      if (observation.processor.gateOpen.length > 0) {
        const open = observation.processor.gateOpen.some(Boolean);
        tiles.push({
          label: tr("g5.tileGate"),
          value: open ? tr("g5.tileOpen") : tr("g5.tileClosed"),
          tone: open ? "good" : undefined,
        });
      }
    }
    if (observation.timing) {
      tiles.push({
        label: tr("g5.tileDelay"),
        value: `${observation.timing.delayMs.toFixed(1)} ms`,
        detail: tr("g5.tileAddedHere"),
      });
      if (typeof observation.timing.processingUsAvg === "number")
        tiles.push({
          label: tr("g5.tileCpu"),
          value: `${Math.round(observation.timing.processingUsAvg)} µs`,
          detail:
            typeof observation.timing.processingUsMax === "number"
              ? tr("g5.tileMax", { us: Math.round(observation.timing.processingUsMax) })
              : undefined,
        });
    }
    if (observation.plugin) {
      const plugin = observation.plugin;
      const healthy = plugin.state === "running";
      tiles.push({
        label: tr("g5.tilePlugin"),
        value: healthy ? tr("g5.tileRunning") : plugin.state === "failed" ? tr("g5.tileFailed") : plugin.state,
        tone: healthy ? "good" : "bad",
        detail: plugin.failureCount > 0 ? tr("g5.tileFailures", { count: plugin.failureCount }) : undefined,
      });
    }
    if (observation.network) {
      const text = networkTelemetryText(observation.network, tr);
      if (text)
        tiles.push({
          label: tr("g5.tileNetwork"),
          value:
            observation.network.direction === "send"
              ? tr("g5.tileSending")
              : (observation.network.receivedPackets ?? 0) > 0
                ? tr("g5.tileReceiving")
                : tr("g5.tileWaiting"),
          detail: text.split(" · ").slice(1).join(" · ") || undefined,
        });
    }
  }
  const status = !node.enabled
    ? "Off"
    : node.bypass
      ? "Bypass"
      : applicationCaptureState
        ? applicationCaptureState.state
        : !running
          ? "Waiting for Play"
          : endpointOwned
            ? "Device busy"
            : tiles.length > 0
              ? "Live"
              : "No readings";
  const note = !node.enabled
    ? tr("workflowUi.offNote")
    : node.bypass
      ? tr("workflowUi.bypassNote")
      : (applicationCaptureState?.detail ??
        // Only notes that tell the user something; the status pill covers the rest.
        (running && endpointOwned ? tr("workflowUi.exclusiveNote") : null));
  return (
    <section className={`node-telemetry${running ? " is-live" : ""}`} aria-labelledby="node-telemetry-heading">
      <div className="node-telemetry-heading">
        <h3 id="node-telemetry-heading">{tr("workflowUi.live")}</h3>
        <span className={`node-telemetry-status${status === "Live" ? " is-live" : ""}`} role="status">
          {status === "Live" && <span className="node-telemetry-dot" aria-hidden="true" />}
          {status === "Off"
            ? tr("common.off")
            : status === "Bypass"
              ? tr("toolProperties.bypass")
              : status === "Waiting for Play"
                ? tr("workflowUi.waitingPlay")
                : status === "Device busy"
                  ? tr("workflowUi.deviceBusy")
                  : status === "Live"
                    ? tr("workflowUi.liveStatus")
                    : status === "No readings"
                      ? tr("workflowUi.noReadings")
                      : APP_CAPTURE_STATE_TEXT[status]
                        ? tr(APP_CAPTURE_STATE_TEXT[status])
                        : status}
        </span>
      </div>
      {note && <p className="node-telemetry-note">{note}</p>}
      {running && node.enabled && !node.bypass && observation?.meter && (
        <LiveLevelBar
          peakDb={observation.meter.currentPeakDb ?? observation.meter.peakDb}
          rmsDb={observation.meter.rmsDb}
        />
      )}
      {tiles.length > 0 && (
        <dl className="node-telemetry-tiles">
          {tiles.map((tile) => (
            <div key={tile.label} className={`node-telemetry-tile${tile.tone ? ` is-${tile.tone}` : ""}`}>
              <dt>{tile.label}</dt>
              <dd>{tile.value}</dd>
              {tile.detail && <small>{tile.detail}</small>}
            </div>
          ))}
        </dl>
      )}
    </section>
  );
}

export const EQ_RESPONSE_FREQUENCIES = Array.from({ length: 48 }, (_, index) => 20 * Math.pow(1000, index / 47));

export function EqResponsePreview({ node, backend }: { node: Node; backend: UiBackend }) {
  const tr = useMessages();
  const [response, setResponse] = useState<import("./backend").ProcessorResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (node.kind !== "parametricEq" || !backend.connected) {
      setResponse(null);
      setError(null);
      return;
    }
    let active = true;
    const bands = Array.from({ length: 16 }, (_, index) => {
      const prefix = `band${index}`;
      const legacy = index === 0;
      return {
        enabled:
          typeof node.parameters[`${prefix}Enabled`] === "boolean"
            ? (node.parameters[`${prefix}Enabled`] as boolean)
            : legacy && node.parameters.frequencyHz !== undefined,
        type: (node.parameters[`${prefix}Type`] ?? "peaking") as
          "peaking" | "lowShelf" | "highShelf" | "lowPass" | "highPass" | "bandPass" | "allPass" | "notch",
        frequencyHz: Number(node.parameters[`${prefix}FrequencyHz`] ?? (legacy ? node.parameters.frequencyHz : 1000)),
        q: Number(node.parameters[`${prefix}Q`] ?? (legacy ? node.parameters.q : 1)),
        gainDb: Number(node.parameters[`${prefix}GainDb`] ?? (legacy ? node.parameters.gainDb : 0)),
      };
    });
    void backend
      .processorResponse({ sampleRateHz: 48000, bands, frequenciesHz: EQ_RESPONSE_FREQUENCIES })
      .then((value) => {
        if (active) {
          setResponse(value);
          setError(null);
        }
      })
      .catch((reason) => {
        if (active) {
          setResponse(null);
          setError(formatUiError(reason, tx("g5.eqResponseUnavailable")));
        }
      });
    return () => {
      active = false;
    };
  }, [backend, node.kind, node.parameters]);
  if (node.kind !== "parametricEq") return null;
  const points = response?.frequenciesHz
    .map((frequency, index) => {
      const magnitude = response.magnitudeDb[index] ?? 0;
      const x = 8 + (Math.log10(frequency / 20) / 3) * 284;
      const bounded = Math.max(-24, Math.min(24, magnitude));
      const y = 56 - ((bounded + 24) / 48) * 48;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  return (
    <section className="eq-response" aria-labelledby="eq-response-heading">
      <h3 id="eq-response-heading">{tr("g5.eqResponseHeading")}</h3>
      {error ? (
        <p className="muted" role="status">
          {error}
        </p>
      ) : !response ? (
        <p className="muted">{tr("g5.eqResponseLoading")}</p>
      ) : (
        <svg viewBox="0 0 300 64" role="img" aria-label={tr("g5.eqResponseAria")}>
          <line x1="8" y1="32" x2="292" y2="32" stroke="currentColor" opacity="0.35" />
          <polyline points={points} fill="none" stroke="currentColor" strokeWidth="1.5" />
        </svg>
      )}
    </section>
  );
}

export function FirFilterEditor({
  node,
  backend,
  disabled,
  onChange,
}: {
  node: Node;
  backend: UiBackend;
  disabled: boolean;
  onChange: (changes: Array<[string, boolean | number | string]>) => void;
}) {
  const tr = useMessages();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const choose = async (file?: File) => {
    if (!file) return;
    const problem = audioUploadProblem(file);
    if (problem) {
      setMessage(problem.text);
      return;
    }
    setBusy(true);
    try {
      const media = await uploadAudioMedia(backend, file);
      onChange([
        ["mediaId", media.mediaId],
        ["fileName", media.fileName],
      ]);
      setMessage(
        tr("g5.firLoaded", {
          file: media.fileName,
          seconds: (media.durationMs / 1000).toFixed(2),
          note: media.durationMs > 2000 ? tr("g5.firOnly2s") : "",
        }),
      );
    } catch (error) {
      setMessage(formatUiError(error, tr("g5.firFailed")));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="node-binding-editor" aria-label={tr("g5.firAria")}>
      <div>
        <p className="eyebrow">{tr("g5.firEyebrow")}</p>
        <strong>{typeof node.parameters.fileName === "string" ? node.parameters.fileName : tr("g5.firNone")}</strong>
      </div>
      <label className="secondary file-picker">
        {tr("g5.firChoose")}
        <input
          type="file"
          accept=".wav,.mp3,audio/wav,audio/mpeg"
          disabled={disabled || busy}
          onChange={(event) => {
            void choose(event.target.files?.[0]);
            event.target.value = "";
          }}
        />
      </label>
      <small>{message ?? tr("g5.firHelp")}</small>
    </div>
  );
}

export function DenoiseLearnEditor({
  node,
  running,
  liveProfile,
  disabled,
  onChange,
}: {
  node: Node;
  running: boolean;
  liveProfile: string | null;
  disabled: boolean;
  onChange: (changes: Array<[string, boolean | number | string]>) => void;
}) {
  const tr = useMessages();
  const learning = node.parameters.learning === true;
  const hasProfile = typeof node.parameters.noiseProfile === "string";
  return (
    <div className="node-binding-editor" aria-label={tr("g5.denoiseAria")}>
      <div>
        <p className="eyebrow">{tr("g5.denoiseEyebrow")}</p>
        <strong>
          {learning ? tr("g5.denoiseLearning") : hasProfile ? tr("g5.denoiseStored") : tr("g5.denoiseTeach")}
        </strong>
      </div>
      {!learning ? (
        <button
          type="button"
          className="secondary"
          disabled={disabled || !running || !node.enabled || node.bypass}
          onClick={() => onChange([["learning", true]])}
        >
          {hasProfile ? tr("g5.denoiseAgain") : tr("g5.denoiseLearn")}
        </button>
      ) : (
        <button
          type="button"
          className="primary"
          disabled={disabled || !running || !node.enabled || node.bypass || !liveProfile}
          onClick={() =>
            liveProfile &&
            onChange([
              ["noiseProfile", liveProfile],
              ["learning", false],
            ])
          }
        >
          {tr("g5.denoiseStop")}
        </button>
      )}
      <small>
        {!node.enabled
          ? tr("g5.denoiseOffHelp")
          : node.bypass
            ? tr("g5.denoiseBypassHelp")
            : !running
              ? tr("g5.denoiseStartHelp")
              : learning
                ? liveProfile
                  ? tr("g5.denoiseKeepHelp")
                  : tr("g5.denoiseWaiting")
                : tr("g5.denoiseReductionHelp")}
      </small>
    </div>
  );
}

export function MixerInputsEditor({
  session,
  mixer,
  disabled,
  onChange,
}: {
  session: import("@audiorouter/contracts").Session;
  mixer: Node;
  disabled: boolean;
  onChange: (name: string, value: number) => void;
}) {
  const tr = useMessages();
  const inputs = mixerInputs(session, mixer.id);
  return (
    <div className="node-binding-editor mixer-inputs-editor" aria-label={tr("g5.mixerAria")}>
      <div>
        <p className="eyebrow">{tr("g5.mixerEyebrow")}</p>
        <strong>{tr("g5.mixerPerSource")}</strong>
      </div>
      {inputs.length === 0 ? (
        <small>{tr("g5.mixerConnect")}</small>
      ) : (
        inputs.map((input) => (
          <SliderField
            key={input.edgeId}
            label={`${input.upstream.name}${input.enabled ? "" : tr("g5.mixerOffSuffix")}`}
            name={tr("g5.mixerVolumeFor", { name: input.upstream.name })}
            value={input.percent}
            min={0}
            max={100}
            step={1}
            unit="%"
            defaultValue={100}
            disabled={disabled}
            showScale={false}
            onChange={(next) => onChange(mixerInputVolumeKey(input.upstream.id), next)}
          />
        ))
      )}
      <small>{tr("g5.mixerRangeHelp")}</small>
    </div>
  );
}

export function applicationChoiceLabel(application: ApplicationRow): string {
  return `${application.audioDisplayNames[0] ?? application.executable} - PID ${application.processId}`;
}

export function ApplicationChoiceOptions({ applications }: { applications: ApplicationRow[] }) {
  const tr = useMessages();
  const { withAudio, other } = applicationCaptureChoices(applications);
  return (
    <>
      {withAudio.length > 0 && (
        <optgroup label={tr("g5.appUsingAudio")}>
          {withAudio.map((application) => (
            <option key={applicationChoiceKey(application)} value={applicationChoiceKey(application)}>
              {applicationChoiceLabel(application)}
            </option>
          ))}
        </optgroup>
      )}
      {other.length > 0 && (
        <optgroup label={tr("g5.appOther")}>
          {other.map((application) => (
            <option key={applicationChoiceKey(application)} value={applicationChoiceKey(application)}>
              {applicationChoiceLabel(application)}
            </option>
          ))}
        </optgroup>
      )}
    </>
  );
}

export function ApplicationCaptureBinding({
  node,
  applications,
  error,
  disabled,
  onRefresh,
  onSelect,
}: {
  node: Node;
  applications: ApplicationRow[];
  error: string | null;
  disabled: boolean;
  onRefresh: () => void;
  onSelect: (application: ApplicationRow) => void;
}) {
  const tr = useMessages();
  const processId = typeof node.parameters.processId === "number" ? node.parameters.processId : null;
  const creationTime = typeof node.parameters.creationTime100ns === "string" ? node.parameters.creationTime100ns : null;
  const executable =
    typeof node.parameters.executable === "string" ? node.parameters.executable : tr("g5.appDefaultName");
  const current =
    applications.find(
      (application) => application.processId === processId && application.creationTime100ns === creationTime,
    ) ?? null;
  const currentKey = current ? applicationChoiceKey(current) : "";
  return (
    <div className="node-binding-editor" aria-label={tr("g5.appAria")}>
      <div>
        <p className="eyebrow">{tr("g5.appEyebrow")}</p>
        <strong>{tr("g5.appChoose")}</strong>
      </div>
      <select
        aria-label={tr("g5.appSelectAria")}
        value={currentKey}
        disabled={disabled || applications.length === 0}
        onChange={(event) => {
          const next = applications.find((application) => applicationChoiceKey(application) === event.target.value);
          if (next) onSelect(next);
        }}
      >
        {!current && (
          <option value="">
            {processId === null
              ? tr("g5.appAnyInstance", { name: executable })
              : tr("g5.appNotRunning", { name: executable, pid: processId })}
          </option>
        )}
        <ApplicationChoiceOptions applications={applications} />
      </select>
      <small>
        {error ? tr("g5.appInventoryError", { error }) : disabled ? tr("g5.appStopSession") : tr("g5.appChangeHelp")}
      </small>
      <button type="button" className="secondary" onClick={onRefresh} disabled={disabled}>
        {tr("g5.appRefresh")}
      </button>
    </div>
  );
}

/** A disabled last line saying how many devices this PC hides (UI-22). */
function HiddenDevicesOption({ count }: { count: number }) {
  if (count === 0) return null;
  return (
    <option value="hidden-devices-note" disabled>
      {count === 1 ? tx("g5.hiddenOne") : tx("g5.hiddenMany", { count })}
    </option>
  );
}

export function PhysicalInputBinding({
  devices,
  value,
  disabled,
  onChange,
  onRefresh,
  focusRequest = 0,
  surround = false,
}: {
  devices: DeviceListItem[];
  value: string;
  disabled: boolean;
  onChange: (value: string, name: string) => void;
  onRefresh: () => void;
  focusRequest?: number;
  surround?: boolean;
}) {
  const hidden = useHiddenDevices();
  const capture = visibleDevices(
    sortDevicesAlphabetically(
      devices.filter(
        (device): device is Extract<DeviceListItem, { state: "active" }> =>
          device.state === "active" && device.direction === "capture",
      ),
    ),
    hidden,
    value,
  );
  const activeCapture = capture.visible;
  // Surround to headphones can also loopback-capture a 5.1/7.1 playback device.
  const playback = visibleDevices(
    surround
      ? sortDevicesAlphabetically(
          devices.filter(
            (device): device is Extract<DeviceListItem, { state: "active" }> =>
              device.state === "active" &&
              device.direction === "render" &&
              (device.format.channels === 6 || device.format.channels === 8),
          ),
        )
      : [],
    hidden,
    value,
  );
  const surroundPlayback = playback.visible;
  const hiddenCount = capture.hiddenCount + playback.hiddenCount;
  const bindingRef = useRef<HTMLDivElement>(null);
  const selectRef = useRef<HTMLSelectElement>(null);
  useEffect(() => {
    if (focusRequest === 0) return;
    const container = bindingRef.current;
    const selector = selectRef.current;
    if (!container || !selector) return;
    container.scrollIntoView?.({ block: "center", behavior: scrollBehavior() });
    if (selector.disabled) container.focus({ preventScroll: true });
    else selector.focus({ preventScroll: true });
    container.classList.add("is-focus-target");
    const timeout = window.setTimeout(() => container.classList.remove("is-focus-target"), 2400);
    return () => window.clearTimeout(timeout);
  }, [focusRequest]);
  return (
    <div ref={bindingRef} className="node-binding-editor" aria-label={tx("workspaceAudit.inputBinding")} tabIndex={-1}>
      <div>
        <p className="eyebrow">{tx("workspaceAudit.capture")}</p>
        <strong>{tx("workspaceAudit.chooseCapture")}</strong>
      </div>
      <select
        ref={selectRef}
        aria-label={tx("workspaceAudit.inputEndpoint")}
        value={value}
        disabled={disabled || activeCapture.length === 0}
        onChange={(event) => {
          const device =
            activeCapture.find((item) => item.id === event.target.value) ??
            surroundPlayback.find((item) => item.id === event.target.value);
          if (device) onChange(device.id, device.name);
        }}
      >
        <option value="">{tx("workspaceAudit.chooseInput")}</option>
        {value &&
          !activeCapture.some((device) => device.id === value) &&
          !surroundPlayback.some((device) => device.id === value) && (
            <option value={value} disabled>
              {tx("workspaceAudit.oldInput")}
            </option>
          )}
        {surroundPlayback.length > 0 ? (
          <optgroup label={tx("workspaceAudit.recordingDevices")}>
            {activeCapture.map((device) => (
              <option key={device.id} value={device.id}>
                {deviceChoiceLabel(device)}
              </option>
            ))}
          </optgroup>
        ) : (
          activeCapture.map((device) => (
            <option key={device.id} value={device.id}>
              {deviceChoiceLabel(device)}
            </option>
          ))
        )}
        {surroundPlayback.length > 0 && (
          <optgroup label={tx("workspaceAudit.loopbackDevices")}>
            {surroundPlayback.map((device) => (
              <option key={device.id} value={device.id}>
                {tx("g5.loopbackPrefix", { name: deviceChoiceLabel(device) })}
              </option>
            ))}
          </optgroup>
        )}
        <HiddenDevicesOption count={hiddenCount} />
      </select>
      <small className={!value && activeCapture.length > 0 ? "node-binding-missing" : undefined}>
        {activeCapture.length === 0
          ? hiddenCount > 0
            ? tx("g5.hiddenAll")
            : tx("workspaceAudit.noCapture")
          : !value
            ? tx("workspaceAudit.notChosen")
            : tx("workspaceAudit.voicemeeter")}
      </small>
      <button type="button" className="secondary" onClick={onRefresh} disabled={disabled}>
        {tx("workflowUi.refreshInputs")}
      </button>
    </div>
  );
}

export function PhysicalOutputBinding({
  devices,
  value,
  disabled,
  onChange,
  onRefresh,
}: {
  devices: DeviceListItem[];
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onRefresh: () => void;
}) {
  const { visible: activeRender, hiddenCount } = visibleDevices(
    sortDevicesAlphabetically(
      devices.filter(
        (device): device is Extract<DeviceListItem, { state: "active" }> =>
          device.state === "active" && device.direction === "render",
      ),
    ),
    useHiddenDevices(),
    value,
  );
  return (
    <div className="node-binding-editor" aria-label={tx("workspaceAudit.outputBinding")}>
      <div>
        <p className="eyebrow">{tx("workspaceAudit.render")}</p>
        <strong>{tx("workspaceAudit.chooseRender")}</strong>
      </div>
      <select
        aria-label={tx("workspaceAudit.outputEndpoint")}
        value={value}
        disabled={disabled || activeRender.length === 0}
        onChange={(event) => onChange(event.target.value)}
      >
        <option value="">{tx("workspaceAudit.chooseOutput")}</option>
        {activeRender.map((device) => (
          <option key={device.id} value={device.id}>
            {deviceChoiceLabel(device)}
          </option>
        ))}
        <HiddenDevicesOption count={hiddenCount} />
      </select>
      <small className={!value && activeRender.length > 0 ? "node-binding-missing" : undefined}>
        {activeRender.length === 0
          ? hiddenCount > 0
            ? tx("g5.hiddenAll")
            : tx("workspaceAudit.noRender")
          : !value
            ? tx("workspaceAudit.notChosen")
            : tx("workspaceAudit.playsThrough")}
      </small>
      <button type="button" className="secondary" onClick={onRefresh} disabled={disabled}>
        {tx("workspaceAudit.refreshOutputs")}
      </button>
    </div>
  );
}

/** Chooses the AudioRouter cable a cable source/destination uses. The node's
 * own cable is always shown, even when the status does not list it. */
export function CableBinding({
  status,
  value,
  direction,
  disabled,
  onChange,
}: {
  status: VirtualCableStatus | null;
  value: string;
  direction: "renderSource" | "captureSink";
  disabled: boolean;
  onChange: (busId: string) => void;
}) {
  const tr = useMessages();
  const cables = status?.cables ?? [];
  const listed = cables.some((cable) => cable.busId === value);
  const unavailable = cableUnavailableReason(status);
  return (
    <div className="node-binding-editor" aria-label={tr("g5.cableAria")}>
      <div>
        <p className="eyebrow">
          {direction === "renderSource" ? tr("g5.cableSourceEyebrow") : tr("g5.cableDestEyebrow")}
        </p>
        <strong>{direction === "renderSource" ? tr("g5.cableChooseIn") : tr("g5.cableChooseOut")}</strong>
      </div>
      <select
        aria-label={tr("g5.cableSelectAria")}
        value={value}
        disabled={disabled || cables.length === 0}
        onChange={(event) => onChange(event.target.value)}
      >
        {!listed && (
          <option value={value}>
            {tr("g5.cableNotInstalled", { name: cableEndpointName(value, direction, status) })}
          </option>
        )}
        {cables.map((cable) => {
          const name = cableEndpointName(cable.busId, direction, status);
          return (
            <option key={cable.busId} value={cable.busId} disabled={!cable.present && cable.busId !== value}>
              {cable.present ? name : tr("g5.cableMissing", { name })}
            </option>
          );
        })}
      </select>
      <p className="muted">
        {unavailable ??
          tr(direction === "renderSource" ? "g5.cableSpeaker" : "g5.cableMic", {
            name: cableEndpointName(value, direction, status),
          })}
      </p>
    </div>
  );
}
