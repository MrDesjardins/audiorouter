import { useState } from "react";
import { useMessages } from "./i18n/useMessages";
const TOOL_GUIDES: Record<string, string> = {
  gain: "gain",
  volume: "volume",
  bassTreble: "bass-treble",
  graphicEq: "graphic-eq",
  parametricEq: "parametric-eq",
  compressor: "compressor",
  gate: "gate",
  limiter: "limiter",
  delay: "delay",
  pitch: "pitch",
  dehum: "dehum",
  declick: "declick",
  denoise: "denoise",
  speechDenoise: "speech-denoise",
  spectralGate: "spectral-gate",
  funVoiceChanger: "fun-voice-changer",
  firFilter: "fir-filter",
  mute: "mute",
  inputSwitch: "input-switch",
  outputSwitch: "output-switch",
  duck: "duck",
  timeShift: "time-shift",
  mixer: "mixer",
  meter: "meter",
  physicalInput: "physical-input",
  physicalOutput: "physical-output",
  applicationCapture: "application-capture",
  endpointLoopback: "endpoint-loopback",
  virtualRenderSource: "virtual-render-source",
  virtualCaptureSink: "virtual-capture-sink",
  testSignal: "test-signal",
  audioFile: "audio-file",
  soundboard: "soundboard",
  recorder: "recorder",
  networkSend: "network-send",
  networkReceive: "network-receive",
  plugin: "plugin",
  instrument: "instrument",
  group: "group",
};
export function ToolGuideLink({
  kind,
  label,
  compact = false,
  toolName,
}: {
  kind: string;
  label?: string;
  compact?: boolean;
  toolName?: string;
}) {
  const tr = useMessages();
  const [error, setError] = useState(false);
  const slug = Object.hasOwn(TOOL_GUIDES, kind) ? TOOL_GUIDES[kind] : undefined;
  if (!slug) return null;
  const url = "https://audiorouter.org/tools/" + slug + ".html";
  return (
    <span className={`tool-guide-access${compact ? " is-compact" : ""}`}>
      <a
        href={url}
        target="_blank"
        rel="noopener noreferrer"
        aria-label={compact ? tr("toolProperties.openGuide", { name: toolName ?? kind }) : undefined}
        title={compact ? tr("toolProperties.guideTitle", { name: toolName ?? kind }) : undefined}
        onClick={(event) => {
          event.stopPropagation();
          const invoke = window.__TAURI_INTERNALS__?.invoke;
          if (invoke) {
            event.preventDefault();
            setError(false);
            void invoke("open_tool_guide", { kind }).catch(() => setError(true));
          }
        }}
      >
        {compact ? <span aria-hidden="true">?</span> : (label ?? tr("toolProperties.guide"))}
      </a>
      {error && <small role="alert">{tr("toolProperties.guideError", { url })}</small>}
    </span>
  );
}
