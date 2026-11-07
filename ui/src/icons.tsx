import type { NodeKind } from "@audiorouter/contracts";
import { ICON_SHAPES, type IconName } from "./iconShapes";

export type { IconName };

/**
 * One icon from the app's set. Colour follows the text around it, so every
 * theme works without per-icon rules. The shapes are constants in this
 * repository (iconShapes.ts); nothing from a session or a plug-in ever
 * reaches this markup.
 */
export function Icon({ name, size = 16, className }: { name: IconName; size?: number; className?: string }) {
  return (
    <svg
      className={className ? `icon ${className}` : "icon"}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      dangerouslySetInnerHTML={{ __html: ICON_SHAPES[name] }}
    />
  );
}

/** Record, not a list with a fallback: a new tool kind fails to compile until it has an icon. */
export const NODE_KIND_ICONS: Record<NodeKind, IconName> = {
  physicalInput: "mic",
  applicationCapture: "app-window",
  endpointLoopback: "repeat",
  physicalOutput: "speaker",
  virtualRenderSource: "cable",
  virtualCaptureSink: "plug-2",
  testSignal: "audio-waveform",
  mixer: "merge",
  gain: "triangle-right",
  mute: "volume-x",
  meter: "gauge",
  parametricEq: "sliders-horizontal",
  compressor: "fold-vertical",
  gate: "door-open",
  limiter: "arrow-up-to-line",
  delay: "timer",
  graphicEq: "sliders-vertical",
  pitch: "arrow-up-down",
  recorder: "circle-dot",
  plugin: "puzzle",
  audioFile: "file-audio",
  soundboard: "layout-grid",
  volume: "volume-2",
  bassTreble: "audio-lines",
  dehum: "dehum",
  declick: "declick",
  inputSwitch: "git-merge",
  outputSwitch: "git-fork",
  denoise: "brush-cleaning",
  speechDenoise: "speech",
  firFilter: "funnel",
  spectralGate: "spectralGate",
  timeShift: "history",
  networkSend: "radio-tower",
  networkReceive: "radio-receiver",
  duck: "duck",
  funVoiceChanger: "smile",
};

/** The icon for a Tools entry. Entries without a kind yet (not installed) still name the kind they will add. */
export function libraryEntryIcon(entry: { id: string; kind?: NodeKind; virtualKind?: NodeKind }): IconName {
  if (entry.id === "endpoint-loopback") return NODE_KIND_ICONS.endpointLoopback;
  const kind = entry.kind ?? entry.virtualKind;
  return kind ? NODE_KIND_ICONS[kind] : "circle-dashed";
}
