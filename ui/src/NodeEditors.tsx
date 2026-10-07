// Property editors of the selected node (moved from App.tsx).
import { useContext, useEffect, useState } from "react";
import type { Node, PluginParametersResult } from "@audiorouter/contracts";
import { formatUiError, type ApplicationRow, type UiBackend } from "./backend";
import type { DeviceListItem } from "@audiorouter/contracts";
import { applicationCaptureChoices, applicationChoiceKey, mixerInputs, mixerInputVolumeKey } from "./draft";
import { audioUploadProblem, uploadAudioMedia } from "./audioUpload";
import { formatParameterValue, optionLabel, parameterText } from "./parameterText";
import { LiveLevelBar } from "./ToolVisuals";
import { type ProcessorDescriptor } from "./processorCatalog";
import { networkTelemetryText } from "./NetworkNodeEditor";
import { NumberField } from "./NumberField";
import { AdvancedEqEditor } from "./AdvancedEqEditor";
import { EqBackendContext, PluginParameterContext } from "./appContext";
import { deviceChoiceLabel, sortDevicesAlphabetically } from "./endpointBinding";

/** Duck settings the Duck editor owns (trigger mode and Siege phases). */
export const DUCK_EDITOR_PARAMETERS = new Set(["trigger", "duckMenu", "duckPrep", "duckBetweenRounds"]);

export function ProcessorParameterEditor({
  node,
  processors,
  nodeTypes = null,
  pluginParameters = null,
  pluginParameterError = null,
  connected,
  onChange,
}: {
  node: Node;
  processors: ProcessorDescriptor[] | null;
  nodeTypes?: import("@audiorouter/contracts").DiscoveryDocument["nodeTypes"] | null;
  pluginParameters?: PluginParametersResult | null;
  pluginParameterError?: string | null;
  connected: boolean;
  onChange: (name: string, value: boolean | number | string) => void;
}) {
  const eqBackend = useContext(EqBackendContext);
  const pluginContext = useContext(PluginParameterContext);
  if (node.kind === "parametricEq")
    return <AdvancedEqEditor node={node} backend={eqBackend} connected={connected} onChange={onChange} />;
  if (node.kind === "graphicEq" || node.kind === "inputSwitch") return null;
  pluginParameters ??= pluginContext.parameters;
  pluginParameterError ??= pluginContext.error;
  if (node.kind === "plugin") {
    if (pluginParameterError)
      return (
        <p className="muted" role="status">
          Plugin parameters unavailable: {pluginParameterError}
        </p>
      );
    if (!pluginParameters)
      return (
        <p className="muted" role="status">
          Loading bounded parameters from the exact scanned plugin...
        </p>
      );
    if (pluginParameters.parameters.length === 0)
      return (
        <p className="muted" role="status">
          This plugin exposes no automatable parameters.
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
            <label key={name}>
              <span>{parameter.title}</span>
              <input
                type="range"
                aria-label={`${parameter.title} slider`}
                value={value}
                min={parameter.minimum}
                max={parameter.maximum}
                step={0.001}
                disabled={!connected}
                onChange={(event) => onChange(name, Number(event.target.value))}
              />
              <NumberField
                aria-label={`${parameter.title} precise value`}
                value={value}
                min={parameter.minimum}
                max={parameter.maximum}
                step={0.001}
                disabled={!connected}
                onValue={(next) => onChange(name, next)}
              />
              <small>normalized parameter {parameter.parameterId}</small>
            </label>
          );
        })}
      </>
    );
  }
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
      !(parameter.name === "learning" && (node.kind === "denoise" || node.kind === "spectralGate")) &&
      // Node references and the Duck trigger/phase choices have dedicated pickers in the Duck editor.
      !("reference" in parameter && parameter.reference) &&
      !(node.kind === "duck" && DUCK_EDITOR_PARAMETERS.has(parameter.name)),
  );
  if (parameters.length === 0) return null;
  return (
    <>
      {parameters.map((parameter) => {
        const value = node.parameters[parameter.name];
        const { label, help } = parameterText(node.kind, parameter.name);
        const helpText = help ? <small className="parameter-help">{help}</small> : null;
        if (parameter.type === "boolean") {
          return (
            <label key={parameter.name}>
              {label}
              {helpText}
              <input
                aria-label={label}
                type="checkbox"
                checked={value === true}
                disabled={!connected}
                onChange={(event) => onChange(parameter.name, event.target.checked)}
              />
            </label>
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
        const sliderValue = hasRange
          ? Math.min(parameter.maximum!, Math.max(parameter.minimum!, numericValue))
          : numericValue;
        // Caption with a readable value, then the slider for quick changes and
        // the exact field for experts. Double-click the slider for the default.
        return (
          <label key={parameter.name} className="param-row">
            <span className="param-caption">
              <span>
                {label}
                {parameter.unit ? ` (${parameter.unit})` : ""}
              </span>
              <b aria-hidden="true">{formatParameterValue(numericValue, parameter.unit, step)}</b>
            </span>
            {helpText}
            <span className={`param-control${hasRange ? "" : " is-field-only"}`}>
              {hasRange && (
                <input
                  type="range"
                  aria-label={`${label} slider`}
                  title={
                    typeof parameter.default === "number"
                      ? `Double-click to restore the default (${formatParameterValue(parameter.default, parameter.unit, step)})`
                      : undefined
                  }
                  value={sliderValue}
                  min={parameter.minimum}
                  max={parameter.maximum}
                  step={step}
                  disabled={!connected}
                  onChange={(event) => onChange(parameter.name, Number(event.target.value))}
                  onDoubleClick={() => {
                    if (typeof parameter.default === "number" && connected) onChange(parameter.name, parameter.default);
                  }}
                />
              )}
              <NumberField
                aria-label={`${label} precise value`}
                value={numericValue}
                min={parameter.minimum}
                max={parameter.maximum}
                step={step}
                disabled={!connected}
                onValue={(next) => onChange(parameter.name, next)}
              />
            </span>
          </label>
        );
      })}
    </>
  );
}

export function InspectorChangeSummary({
  draftNode,
  authoritativeNode,
}: {
  draftNode: Node;
  authoritativeNode?: Node;
}) {
  if (!authoritativeNode) {
    return (
      <p className="muted inspector-change-summary" role="status">
        New node. Save to keep it.
      </p>
    );
  }
  const changes: string[] = [];
  if (draftNode.name !== authoritativeNode.name) changes.push('rename to "' + draftNode.name + '"');
  if (draftNode.enabled !== authoritativeNode.enabled) changes.push(draftNode.enabled ? "enable node" : "disable node");
  if (draftNode.bypass !== authoritativeNode.bypass)
    changes.push(draftNode.bypass ? "bypass processing" : "resume processing");
  const parameterNames = [
    ...new Set([...Object.keys(authoritativeNode.parameters), ...Object.keys(draftNode.parameters)]),
  ].sort();
  for (const name of parameterNames) {
    if (!Object.is(authoritativeNode.parameters[name], draftNode.parameters[name])) {
      // A network pairing key is a secret: name the change, never the key.
      changes.push(
        name === "pairingKey"
          ? draftNode.parameters[name]
            ? "change pairing key"
            : "remove pairing key"
          : name +
              ": " +
              String(authoritativeNode.parameters[name] ?? "unset") +
              " → " +
              String(draftNode.parameters[name] ?? "unset"),
      );
    }
  }
  // One fixed line, so editing never moves the controls below it; the full
  // list is in the tooltip.
  const summary =
    changes.length === 0
      ? "No unsaved changes to this node."
      : changes.length === 1
        ? `Unsaved: ${changes[0]}. Save to keep it.`
        : `${changes.length} unsaved changes. Save to keep them.`;
  return (
    <p className="muted inspector-change-summary" role="status" title={changes.join("\n") || undefined}>
      {summary}
    </p>
  );
}

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
        label: "Level",
        value: current <= -120 ? "Silent" : `${current.toFixed(1)} dB`,
        detail: current <= -120 ? "no sound right now" : `peak · RMS ${meter.rmsDb.toFixed(1)} dB`,
      });
      tiles.push({
        label: "Clipping",
        value: String(meter.clippedSamples),
        detail: "samples over 0 dB",
        tone: meter.clippedSamples > 0 ? "warn" : "good",
      });
    }
    if (observation.processor) {
      const reduction = Math.max(0, ...observation.processor.gainReductionDb);
      if (observation.processor.gainReductionDb.length > 0)
        tiles.push({ label: "Gain reduction", value: `${reduction.toFixed(1)} dB` });
      if (observation.processor.gateOpen.length > 0) {
        const open = observation.processor.gateOpen.some(Boolean);
        tiles.push({ label: "Gate", value: open ? "Open" : "Closed", tone: open ? "good" : undefined });
      }
    }
    if (observation.timing) {
      tiles.push({
        label: "Delay",
        value: `${observation.timing.delayMs.toFixed(1)} ms`,
        detail: "added at this step",
      });
      if (typeof observation.timing.processingUsAvg === "number")
        tiles.push({
          label: "CPU per block",
          value: `${Math.round(observation.timing.processingUsAvg)} µs`,
          detail:
            typeof observation.timing.processingUsMax === "number"
              ? `max ${Math.round(observation.timing.processingUsMax)} µs`
              : undefined,
        });
    }
    if (observation.plugin) {
      const plugin = observation.plugin;
      const healthy = plugin.state === "running";
      tiles.push({
        label: "Plugin",
        value: healthy ? "Running" : plugin.state === "failed" ? "Failed" : plugin.state,
        tone: healthy ? "good" : "bad",
        detail: plugin.failureCount > 0 ? `${plugin.failureCount} failures` : undefined,
      });
    }
    if (observation.network) {
      const text = networkTelemetryText(observation.network);
      if (text)
        tiles.push({
          label: "Network",
          value:
            observation.network.direction === "send"
              ? "Sending"
              : (observation.network.receivedPackets ?? 0) > 0
                ? "Receiving"
                : "Waiting",
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
    ? "This node is off. Effects pass sound through; inputs and outputs contribute silence."
    : node.bypass
      ? "This node is bypassed. Its processing and live analysis are inactive; bypassed inputs, outputs and Mixers contribute silence."
      : (applicationCaptureState?.detail ??
        // Only notes that tell the user something; the status pill covers the rest.
        (running && endpointOwned
          ? "Another program has exclusive use of this device. Close it, then press Play again."
          : null));
  return (
    <section className={`node-telemetry${running ? " is-live" : ""}`} aria-labelledby="node-telemetry-heading">
      <div className="node-telemetry-heading">
        <h3 id="node-telemetry-heading">Live readings</h3>
        <span className={`node-telemetry-status${status === "Live" ? " is-live" : ""}`} role="status">
          {status === "Live" && <span className="node-telemetry-dot" aria-hidden="true" />}
          {status}
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
          setError(formatUiError(reason, "EQ response unavailable."));
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
      <h3 id="eq-response-heading">EQ response</h3>
      {error ? (
        <p className="muted" role="status">
          {error}
        </p>
      ) : !response ? (
        <p className="muted">Loading the authoritative response...</p>
      ) : (
        <svg viewBox="0 0 300 64" role="img" aria-label="Parametric EQ magnitude response">
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
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const choose = async (file?: File) => {
    if (!file) return;
    const problem = audioUploadProblem(file);
    if (problem) {
      setMessage(problem);
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
        `${media.fileName} · ${(media.durationMs / 1000).toFixed(2)} s${media.durationMs > 2000 ? " (only the first 2 s are used)" : ""}. Save the route to apply it.`,
      );
    } catch (error) {
      setMessage(formatUiError(error, "Impulse response import failed."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="node-binding-editor" aria-label="FIR impulse response">
      <div>
        <p className="eyebrow">Impulse response</p>
        <strong>
          {typeof node.parameters.fileName === "string" ? node.parameters.fileName : "No impulse response selected"}
        </strong>
      </div>
      <label className="secondary file-picker">
        Choose WAV or MP3
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
      <small>
        {message ??
          "The response is normalized to unit energy. Wet mix blends it with the original (dry) sound; Output gain sets the level. Until a file is chosen, audio passes unchanged."}
      </small>
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
  const learning = node.parameters.learning === true;
  const hasProfile = typeof node.parameters.noiseProfile === "string";
  return (
    <div className="node-binding-editor" aria-label="Denoise noise profile">
      <div>
        <p className="eyebrow">Noise profile</p>
        <strong>
          {learning
            ? "Learning the noise now"
            : hasProfile
              ? "A noise profile is stored"
              : "Teach this tool the noise to remove"}
        </strong>
      </div>
      {!learning ? (
        <button
          type="button"
          className="secondary"
          disabled={disabled || !running || !node.enabled || node.bypass}
          onClick={() => onChange([["learning", true]])}
        >
          {hasProfile ? "Learn again" : "Learn noise"}
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
          Stop learning and keep profile
        </button>
      )}
      <small>
        {!node.enabled
          ? "Off: enable this tool before learning noise."
          : node.bypass
            ? "Bypass: turn Bypass off before learning noise."
            : !running
              ? "Start the route first. Then play only the unwanted noise (for example room tone or fan hiss) for a few seconds while learning."
              : learning
                ? liveProfile
                  ? "Keep only the noise playing for 3–5 seconds, then stop learning. Audio passes unchanged while learning."
                  : "Waiting for the first noise measurement…"
                : "Reduction sets how aggressively the profile is removed; the remaining floor keeps some noise to avoid unnatural silence."}
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
  const inputs = mixerInputs(session, mixer.id);
  return (
    <div className="node-binding-editor mixer-inputs-editor" aria-label="Mixer input volumes">
      <div>
        <p className="eyebrow">Mixer inputs</p>
        <strong>Volume per source</strong>
      </div>
      {inputs.length === 0 ? (
        <small>Connect sources to this Mixer to set their volumes.</small>
      ) : (
        inputs.map((input) => (
          <label key={input.edgeId} className="mixer-input-volume">
            <span>
              {input.upstream.name}
              {input.enabled ? "" : " (off)"}
            </span>
            <input
              type="range"
              min={0}
              max={100}
              step={1}
              value={input.percent}
              disabled={disabled}
              aria-label={`Volume for ${input.upstream.name}`}
              onChange={(event) => onChange(mixerInputVolumeKey(input.upstream.id), Number(event.target.value))}
            />
            <output>{Math.round(input.percent)} %</output>
          </label>
        ))
      )}
      <small>0–100 % per input. For a boost above 100 %, put a Volume tool between the source and the Mixer.</small>
    </div>
  );
}

export function applicationChoiceLabel(application: ApplicationRow): string {
  return `${application.audioDisplayNames[0] ?? application.executable} - PID ${application.processId}`;
}

export function ApplicationChoiceOptions({ applications }: { applications: ApplicationRow[] }) {
  const { withAudio, other } = applicationCaptureChoices(applications);
  return (
    <>
      {withAudio.length > 0 && (
        <optgroup label="Using audio now">
          {withAudio.map((application) => (
            <option key={applicationChoiceKey(application)} value={applicationChoiceKey(application)}>
              {applicationChoiceLabel(application)}
            </option>
          ))}
        </optgroup>
      )}
      {other.length > 0 && (
        <optgroup label="Other running applications">
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
  const processId = typeof node.parameters.processId === "number" ? node.parameters.processId : null;
  const creationTime = typeof node.parameters.creationTime100ns === "string" ? node.parameters.creationTime100ns : null;
  const executable = typeof node.parameters.executable === "string" ? node.parameters.executable : "application";
  const current =
    applications.find(
      (application) => application.processId === processId && application.creationTime100ns === creationTime,
    ) ?? null;
  const currentKey = current ? applicationChoiceKey(current) : "";
  return (
    <div className="node-binding-editor" aria-label="Application capture binding">
      <div>
        <p className="eyebrow">Application source</p>
        <strong>Choose the running application to capture</strong>
      </div>
      <select
        aria-label="Application to capture"
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
              ? `${executable} (any verified instance)`
              : `${executable} - PID ${processId} (not running)`}
          </option>
        )}
        <ApplicationChoiceOptions applications={applications} />
      </select>
      <small>
        {error
          ? `Application inventory unavailable: ${error}`
          : disabled
            ? "Stop the session to change the application."
            : "Changing the application keeps this node and its connections. Commit the draft to apply it."}
      </small>
      <button type="button" className="secondary" onClick={onRefresh} disabled={disabled}>
        Refresh applications
      </button>
    </div>
  );
}

export function PhysicalInputBinding({
  devices,
  value,
  disabled,
  onChange,
  onRefresh,
  surround = false,
}: {
  devices: DeviceListItem[];
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onRefresh: () => void;
  surround?: boolean;
}) {
  const activeCapture = sortDevicesAlphabetically(
    devices.filter(
      (device): device is Extract<DeviceListItem, { state: "active" }> =>
        device.state === "active" && device.direction === "capture",
    ),
  );
  // Surround to headphones can also loopback-capture a 5.1/7.1 playback device.
  const surroundPlayback = surround
    ? sortDevicesAlphabetically(
        devices.filter(
          (device): device is Extract<DeviceListItem, { state: "active" }> =>
            device.state === "active" &&
            device.direction === "render" &&
            (device.format.channels === 6 || device.format.channels === 8),
        ),
      )
    : [];
  return (
    <div className="node-binding-editor" aria-label="Physical input binding">
      <div>
        <p className="eyebrow">Capture source</p>
        <strong>Choose a microphone, input, or virtual capture bus</strong>
      </div>
      <select
        aria-label="Physical input endpoint"
        value={value}
        disabled={disabled || activeCapture.length === 0}
        onChange={(event) => onChange(event.target.value)}
      >
        <option value="">Select capture endpoint</option>
        {surroundPlayback.length > 0 ? (
          <optgroup label="Recording devices">
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
          <optgroup label="Surround playback devices (loopback)">
            {surroundPlayback.map((device) => (
              <option key={device.id} value={device.id}>
                {"Loopback · " + deviceChoiceLabel(device)}
              </option>
            ))}
          </optgroup>
        )}
      </select>
      <small className={!value && activeCapture.length > 0 ? "node-binding-missing" : undefined}>
        {activeCapture.length === 0
          ? "No active capture endpoints are available. Refresh the device list; a reboot is not normally required."
          : !value
            ? "No device is chosen for this node yet. Choose one, then Save."
            : "Voicemeeter Out B1 receives system sound only while the Voicemeeter mixer runs with B1 enabled. For an app-independent virtual cable, send Windows sound to CABLE Input and select CABLE Output here."}
      </small>
      <button type="button" className="secondary" onClick={onRefresh} disabled={disabled}>
        Refresh available inputs
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
  const activeRender = sortDevicesAlphabetically(
    devices.filter(
      (device): device is Extract<DeviceListItem, { state: "active" }> =>
        device.state === "active" && device.direction === "render",
    ),
  );
  return (
    <div className="node-binding-editor" aria-label="Physical output binding">
      <div>
        <p className="eyebrow">Render destination</p>
        <strong>Choose the headphones, speakers, or virtual output</strong>
      </div>
      <select
        aria-label="Physical output endpoint"
        value={value}
        disabled={disabled || activeRender.length === 0}
        onChange={(event) => onChange(event.target.value)}
      >
        <option value="">Select render endpoint</option>
        {activeRender.map((device) => (
          <option key={device.id} value={device.id}>
            {deviceChoiceLabel(device)}
          </option>
        ))}
      </select>
      <small className={!value && activeRender.length > 0 ? "node-binding-missing" : undefined}>
        {activeRender.length === 0
          ? "No active render endpoints are available. Refresh the device list; a reboot is not normally required."
          : !value
            ? "No device is chosen for this node yet. Choose one, then Save."
            : "This selection is used by the session's stopped native render binding."}
      </small>
      <button type="button" className="secondary" onClick={onRefresh} disabled={disabled}>
        Refresh available outputs
      </button>
    </div>
  );
}
