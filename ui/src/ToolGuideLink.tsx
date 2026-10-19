import { useState } from "react";
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
  recorder: "recorder",
  networkSend: "network-send",
  networkReceive: "network-receive",
  plugin: "plugin",
  group: "group",
};
export function ToolGuideLink({
  kind,
  label = "Visual guide · controls & examples ↗",
  compact = false,
  toolName,
}: {
  kind: string;
  label?: string;
  compact?: boolean;
  toolName?: string;
}) {
  const [error, setError] = useState("");
  const slug = Object.hasOwn(TOOL_GUIDES, kind) ? TOOL_GUIDES[kind] : undefined;
  if (!slug) return null;
  const url = "https://audiorouter.org/tools/" + slug + ".html";
  return (
    <span className={`tool-guide-access${compact ? " is-compact" : ""}`}>
      <a
        href={url}
        target="_blank"
        rel="noopener noreferrer"
        aria-label={compact ? `Open ${toolName ?? kind} guide` : undefined}
        title={compact ? `How to use ${toolName ?? kind}` : undefined}
        onClick={(event) => {
          event.stopPropagation();
          const invoke = window.__TAURI_INTERNALS__?.invoke;
          if (invoke) {
            event.preventDefault();
            setError("");
            void invoke("open_tool_guide", { kind }).catch(() => setError("Could not open the guide. Visit " + url));
          }
        }}
      >
        {compact ? <span aria-hidden="true">?</span> : label}
      </a>
      {error && <small role="alert">{error}</small>}
    </span>
  );
}
