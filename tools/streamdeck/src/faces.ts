/**
 * Key faces drawn as SVG (144 × 144, crisp on every Stream Deck). Pure
 * functions: the same input always gives the same picture, so a face is
 * redrawn only when its text changes.
 */

const SIZE = 144;

/** Canvas level colours (ui/src/styles.css `--flow-*`, dark theme). */
const COLD = "#3fd2c7";
const WARM = "#ffd166";
const HOT = "#ff9640";
const CLIP = "#ff5d6c";
const OFF = "#5b6573";
const INK = "#edf4ff";
const PANEL = "#141c27";

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));

function mix(from: string, to: string, amount: number): string {
  const parse = (hex: string) => [1, 3, 5].map((index) => parseInt(hex.slice(index, index + 2), 16));
  const [a, b] = [parse(from), parse(to)];
  return `#${a.map((value, index) => Math.round(value + (b[index] - value) * clamp(amount, 0, 1)).toString(16).padStart(2, "0")).join("")}`;
}

/** The level colour, as the canvas draws live connections. */
export function levelColor(levelDb: number | null): string {
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

export function dataUrl(svg: string): string {
  return `data:image/svg+xml;charset=utf8,${encodeURIComponent(svg)}`;
}

/** Shown when AudioRouter is unreachable, not set up, or the tool is gone. */
export function messageFace(title: string, detail: string): string {
  return frame(text(fit(title), 66, 22, INK) + text(fit(detail, 14), 96, 17, "#9aa6b5", 600), OFF);
}

export type ToggleFace = { label: string; target: string; on: boolean; pending: boolean; valueText?: string };

/** A switch: green ring and ON when the setting is on (for Bypass: amber BYPASS). */
export function toggleFace({ label, target, on, pending, valueText }: ToggleFace): string {
  const bypass = target === "bypass";
  const color = on ? (bypass ? HOT : COLD) : OFF;
  const word = valueText ? fit(valueText, 8) : bypass ? (on ? "BYPASS" : "ACTIVE") : (on ? "ON" : "OFF");
  const pill = `<rect x="34" y="78" width="76" height="34" rx="17" fill="${on ? color : "#26303d"}" opacity="${pending ? 0.55 : 1}"/>`;
  return frame(text(fit(label), 46, 22) + text(fit(targetLabel(target), 14), 68, 15, "#9aa6b5", 600) + pill + text(word, 102, 18, on ? PANEL : INK, 800), color);
}

/** Human names for toggle targets. */
export function targetLabel(target: string): string {
  if (target === "enabled") return "Enabled";
  if (target === "bypass") return "Bypass";
  return target.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/^./, (first) => first.toUpperCase());
}

export type MeterFace = { label: string; peakDb: number | null; rmsDb: number | null; clipped: boolean; muted: boolean; playing: boolean };

/** A vertical level meter: RMS bar in the level colour, peak line, clip dot. */
export function meterFace({ label, peakDb, rmsDb, clipped, muted, playing }: MeterFace): string {
  const top = 38, bottom = 128, left = 48, width = 48, height = bottom - top;
  const rms = meterFraction(rmsDb), peak = meterFraction(peakDb);
  const barHeight = Math.round(rms * height);
  const peakY = Math.round(bottom - peak * height);
  const color = muted || !playing ? OFF : levelColor(rmsDb);
  const value = !playing ? "stopped" : muted ? "muted" : rmsDb === null || rmsDb <= -60 ? "silent" : `${Math.round(rmsDb)} dB`;
  const body = text(fit(label), 28, 18)
    + `<rect x="${left}" y="${top}" width="${width}" height="${height}" rx="6" fill="#26303d"/>`
    + (barHeight > 0 ? `<rect x="${left}" y="${bottom - barHeight}" width="${width}" height="${barHeight}" rx="6" fill="${color}"/>` : "")
    + (playing && peak > 0 ? `<rect x="${left - 4}" y="${peakY - 2}" width="${width + 8}" height="4" rx="2" fill="${INK}"/>` : "")
    + (clipped ? `<circle cx="116" cy="44" r="8" fill="${CLIP}"/>` : "")
    + `<text x="${SIZE - 14}" y="${bottom}" font-family="Segoe UI, Arial, sans-serif" font-size="14" font-weight="700" fill="#9aa6b5" text-anchor="end" transform="rotate(-90 ${SIZE - 14} ${bottom})">${escape(value)}</text>`;
  return frame(body, color);
}

/** The privacy mute key: red and crossed when the microphone is muted. */
export function privacyFace({ muted, mode, pending }: { muted: boolean; mode: "toggle" | "mute" | "unmute"; pending: boolean }): string {
  const color = muted ? CLIP : COLD;
  const mic = `<g transform="translate(52 22)" opacity="${pending ? 0.55 : 1}"><rect x="10" y="0" width="20" height="38" rx="10" fill="${color}"/><path d="M2 26a18 18 0 0 0 36 0M20 44v12M10 56h20" fill="none" stroke="${color}" stroke-width="5" stroke-linecap="round"/>`
    + (muted ? `<path d="M-4 -2L44 56" stroke="${CLIP}" stroke-width="7" stroke-linecap="round"/><path d="M-4 -2L44 56" stroke="${PANEL}" stroke-width="2.5"/>` : "") + "</g>";
  const action = mode === "mute" ? "MUTE MIC" : mode === "unmute" ? "UNMUTE MIC" : "PRIVACY";
  return frame(mic + text(muted ? "MUTED" : "LIVE", 104, 22, color, 800) + text(action, 126, 14, "#9aa6b5", 700), color);
}
