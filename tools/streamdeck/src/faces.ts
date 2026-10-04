/**
 * Key faces drawn as SVG (144 × 144, crisp on every Stream Deck). Pure
 * functions: the same input always gives the same picture, so a face is
 * redrawn only when its text changes. A `label` of null leaves the top line
 * free for Stream Deck's own title (when the user shows it).
 */

const SIZE = 144;

/** Canvas level colours (ui/src/styles.css `--flow-*`, dark theme). */
const COLD = "#3fd2c7";
const WARM = "#ffd166";
const HOT = "#ff9640";
const CLIP = "#ff5d6c";
const OFF = "#5b6573";
const INK = "#edf4ff";
const SOFT = "#9aa6b5";
const PANEL = "#141c27";
const TRACK = "#26303d";

/** Colours a user can pick for a key's on/off look. */
export const PALETTE: Record<string, string> = {
  teal: COLD, green: "#4cd964", yellow: WARM, orange: HOT, red: CLIP, blue: "#5aa9ff", purple: "#b48cff", white: INK, grey: OFF,
};

export function paletteColor(name: string | undefined, fallback: string): string {
  return (name && PALETTE[name]) || fallback;
}

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));

function mix(from: string, to: string, amount: number): string {
  const parse = (hex: string) => [1, 3, 5].map((index) => parseInt(hex.slice(index, index + 2), 16));
  const [a, b] = [parse(from), parse(to)];
  return `#${a.map((value, index) => Math.round(value + (b[index] - value) * clamp(amount, 0, 1)).toString(16).padStart(2, "0")).join("")}`;
}

export type MeterScheme = "canvas" | "classic";

/** The level colour: as the canvas draws live connections, or classic green/yellow/red. */
export function levelColor(levelDb: number | null, scheme: MeterScheme = "canvas"): string {
  if (scheme === "classic") {
    if (levelDb === null || !Number.isFinite(levelDb) || levelDb <= -20) return PALETTE.green;
    return levelDb <= -6 ? WARM : CLIP;
  }
  if (levelDb === null || !Number.isFinite(levelDb) || levelDb <= -40) return COLD;
  if (levelDb <= -20) return mix(COLD, WARM, (levelDb + 40) / 20);
  if (levelDb <= -6) return mix(WARM, HOT, (levelDb + 20) / 14);
  if (levelDb < -1) return mix(HOT, CLIP, (levelDb + 6) / 5);
  return CLIP;
}

/** Fraction of a meter filled by `levelDb` (−60 dB empty, 0 dB full). */
export function meterFraction(levelDb: number | null): number {
  if (levelDb === null || !Number.isFinite(levelDb)) return 0;
  return clamp((levelDb + 60) / 60, 0, 1);
}

function escape(text: string): string {
  return text.replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;" })[char]!);
}

/** Shorten a label to fit a key line. */
export function fit(text: string, max = 12): string {
  const trimmed = text.trim();
  return trimmed.length <= max ? trimmed : `${trimmed.slice(0, max - 1)}…`;
}

function frame(body: string, border: string): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${SIZE}" height="${SIZE}" viewBox="0 0 ${SIZE} ${SIZE}">`
    + `<rect x="3" y="3" width="${SIZE - 6}" height="${SIZE - 6}" rx="18" fill="${PANEL}" stroke="${border}" stroke-width="6"/>`
    + body + "</svg>";
}

function text(content: string, y: number, size: number, color = INK, weight = 700): string {
  return `<text x="${SIZE / 2}" y="${y}" font-family="Segoe UI, Arial, sans-serif" font-size="${size}" font-weight="${weight}" fill="${color}" text-anchor="middle">${escape(content)}</text>`;
}

/** The top label and the smaller line under it; either may be left out. */
function heading(label: string | null, detail: string | null, labelY = 44, detailY = 66): string {
  return (label ? text(fit(label), labelY, 22) : "") + (detail ? text(fit(detail, 14), detailY, 15, SOFT, 600) : "");
}

export function dataUrl(svg: string): string {
  return `data:image/svg+xml;charset=utf8,${encodeURIComponent(svg)}`;
}

/** Shown when AudioRouter is unreachable, not set up, or the tool is gone. */
export function messageFace(title: string, detail: string): string {
  return frame(text(fit(title), 66, 22, INK) + text(fit(detail, 14), 96, 17, SOFT, 600), OFF);
}

/** Human names for toggle targets. */
export function targetLabel(target: string): string {
  if (target === "enabled") return "Enabled";
  if (target === "bypass") return "Bypass";
  return target.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/^./, (first) => first.toUpperCase());
}

export type ToggleFace = {
  label: string | null;
  detail: string | null;
  target: string;
  on: boolean;
  pending: boolean;
  /** For two-choice settings: the current choice, shown instead of ON/OFF. */
  valueText?: string;
  onColor?: string;
  offColor?: string;
  onText?: string;
  offText?: string;
};

/** A switch: coloured pill and word for the current state. */
export function toggleFace(face: ToggleFace): string {
  const bypass = face.target === "bypass";
  const onColor = paletteColor(face.onColor, bypass ? HOT : COLD);
  const offColor = paletteColor(face.offColor, OFF);
  const color = face.on ? onColor : offColor;
  const word = face.valueText ? fit(face.valueText, 8)
    : face.on ? fit(face.onText || (bypass ? "BYPASS" : "ON"), 8)
    : fit(face.offText || (bypass ? "ACTIVE" : "OFF"), 8);
  const pill = `<rect x="30" y="80" width="84" height="34" rx="17" fill="${face.on ? color : TRACK}" stroke="${face.on ? color : offColor}" stroke-width="2" opacity="${face.pending ? 0.55 : 1}"/>`;
  return frame(heading(face.label, face.detail) + pill + text(word, 104, 18, face.on ? PANEL : INK, 800), color);
}

export type MeterFace = {
  label: string | null;
  peakDb: number | null;
  rmsDb: number | null;
  clipped: boolean;
  muted: boolean;
  playing: boolean;
  showValue?: boolean;
  scheme?: MeterScheme;
};

/** A vertical level meter: average bar in the level colour, peak line, clip dot. */
export function meterFace(face: MeterFace): string {
  const showValue = face.showValue ?? true;
  const top = face.label ? 38 : 16, bottom = 128, width = 48, height = bottom - top;
  const left = showValue ? 48 : 48 + 10;
  const rms = meterFraction(face.rmsDb), peak = meterFraction(face.peakDb);
  const barHeight = Math.round(rms * height);
  const peakY = Math.round(bottom - peak * height);
  const color = face.muted || !face.playing ? OFF : levelColor(face.rmsDb, face.scheme);
  const value = !face.playing ? "stopped" : face.muted ? "muted" : face.rmsDb === null || face.rmsDb <= -60 ? "silent" : `${Math.round(face.rmsDb)} dB`;
  const body = (face.label ? text(fit(face.label), 28, 18) : "")
    + `<rect x="${left}" y="${top}" width="${width}" height="${height}" rx="6" fill="${TRACK}"/>`
    + (barHeight > 0 ? `<rect x="${left}" y="${bottom - barHeight}" width="${width}" height="${barHeight}" rx="6" fill="${color}"/>` : "")
    + (face.playing && peak > 0 ? `<rect x="${left - 4}" y="${peakY - 2}" width="${width + 8}" height="4" rx="2" fill="${INK}"/>` : "")
    + (face.clipped ? `<circle cx="116" cy="${top + 6}" r="8" fill="${CLIP}"/>` : "")
    + (showValue ? `<text x="${SIZE - 14}" y="${bottom}" font-family="Segoe UI, Arial, sans-serif" font-size="14" font-weight="700" fill="${SOFT}" text-anchor="end" transform="rotate(-90 ${SIZE - 14} ${bottom})">${escape(value)}</text>` : "");
  return frame(body, color);
}

export type PrivacyFace = { muted: boolean; mode: "toggle" | "mute" | "unmute"; pending: boolean; mutedText?: string; liveText?: string; showDetail?: boolean };

/** The privacy mute key: red and crossed when the microphone is muted. */
export function privacyFace(face: PrivacyFace): string {
  const color = face.muted ? CLIP : COLD;
  const mic = `<g transform="translate(52 22)" opacity="${face.pending ? 0.55 : 1}"><rect x="10" y="0" width="20" height="38" rx="10" fill="${color}"/><path d="M2 26a18 18 0 0 0 36 0M20 44v12M10 56h20" fill="none" stroke="${color}" stroke-width="5" stroke-linecap="round"/>`
    + (face.muted ? `<path d="M-4 -2L44 56" stroke="${CLIP}" stroke-width="7" stroke-linecap="round"/><path d="M-4 -2L44 56" stroke="${PANEL}" stroke-width="2.5"/>` : "") + "</g>";
  const action = face.mode === "mute" ? "MUTE MIC" : face.mode === "unmute" ? "UNMUTE MIC" : "PRIVACY";
  const word = face.muted ? fit(face.mutedText || "MUTED", 10) : fit(face.liveText || "LIVE", 10);
  return frame(mic + text(word, 104, 22, color, 800) + ((face.showDetail ?? true) ? text(action, 126, 14, SOFT, 700) : ""), color);
}

/** Play / Stop for the selected session. */
export function playFace({ label, playing, pending }: { label: string | null; playing: boolean; pending: boolean }): string {
  const color = playing ? COLD : OFF;
  const symbol = playing
    ? `<rect x="54" y="52" width="36" height="36" rx="5" fill="${color}"/>`
    : `<path d="M58 50L94 70L58 90Z" fill="${INK}"/>`;
  return frame((label ? text(fit(label), 34, 18, SOFT, 700) : "") + `<g opacity="${pending ? 0.55 : 1}">${symbol}</g>` + text(playing ? "PLAYING" : "STOPPED", 120, 18, playing ? color : INK, 800), color);
}

/** A session key: the session's name, lit when it is the selected one. */
export function sessionFace({ name, selected, playing, cycle, pending }: { name: string; selected: boolean; playing: boolean; cycle: boolean; pending: boolean }): string {
  const color = selected ? (playing ? COLD : WARM) : OFF;
  const words = name.trim().split(/\s+/);
  const first = fit(words.slice(0, 2).join(" "), 11);
  const second = words.length > 2 ? fit(words.slice(2).join(" "), 11) : "";
  const status = cycle ? "NEXT ▸" : selected ? (playing ? "PLAYING" : "SELECTED") : "SELECT";
  return frame(`<g opacity="${pending ? 0.55 : 1}">` + text(first, second ? 58 : 70, 21) + (second ? text(second, 82, 21) : "") + "</g>" + text(status, 120, 15, selected ? color : SOFT, 800), color);
}

/** A value step key: the setting's current value and what one press adds. */
export function stepFace({ label, detail, valueText, step, pending, atLimit }: { label: string | null; detail: string | null; valueText: string; step: string; pending: boolean; atLimit: boolean }): string {
  const color = atLimit ? OFF : COLD;
  return frame(heading(label, detail, 38, 58) + `<g opacity="${pending ? 0.55 : 1}">` + text(valueText, 94, 26, INK, 800) + "</g>" + text(step, 124, 17, color, 800), color);
}

export type RecordFace = { label: string | null; recording: boolean; elapsed: string; canStart: boolean; pending: boolean };

/** A Record key: a red dot to start, a square and the running time while recording. */
export function recordFace(face: RecordFace): string {
  const color = face.recording ? CLIP : face.canStart ? CLIP : OFF;
  const symbol = face.recording
    ? `<rect x="56" y="54" width="32" height="32" rx="5" fill="${CLIP}"/>`
    : `<circle cx="72" cy="70" r="19" fill="${face.canStart ? CLIP : OFF}"/>`;
  const status = face.recording ? `REC ${face.elapsed}` : face.canStart ? "RECORD" : "PLAY FIRST";
  return frame((face.label ? text(fit(face.label), 34, 18, SOFT, 700) : "") + `<g opacity="${face.pending ? 0.55 : 1}">${symbol}</g>` + text(status, 120, 17, face.recording ? CLIP : face.canStart ? INK : SOFT, 800), face.recording ? color : OFF);
}

/** Elapsed seconds as m:ss or h:mm:ss, like AudioRouter's Record button. */
export function formatElapsed(seconds: number): string {
  const hours = Math.floor(seconds / 3600), minutes = Math.floor((seconds % 3600) / 60), rest = seconds % 60;
  return `${hours > 0 ? `${hours}:` : ""}${String(minutes).padStart(hours > 0 ? 2 : 1, "0")}:${String(rest).padStart(2, "0")}`;
}
