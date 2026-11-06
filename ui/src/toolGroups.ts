import type { NodeKind } from "@audiorouter/contracts";
import type { LibraryEntry } from "./library";
import { optionalLocalStorage, readPresentationJson, writePresentationJson } from "./presentationStorage";

export type ToolGroupId = "input" | "output" | "clean" | "level" | "tone" | "routing" | "plugins";
export type ShelfGroupId = ToolGroupId | "canvas" | "setup";

/**
 * Where each kind sits in Tools. A Record, so a new kind fails to compile until it is
 * placed. The key order is also the order inside a group: the likeliest tool comes first.
 */
export const TOOL_GROUP_OF: Record<NodeKind, ToolGroupId> = {
  physicalInput: "input",
  applicationCapture: "input",
  audioFile: "input",
  soundboard: "input",
  testSignal: "input",
  networkReceive: "input",
  virtualRenderSource: "input",
  endpointLoopback: "input",
  physicalOutput: "output",
  networkSend: "output",
  virtualCaptureSink: "output",
  speechDenoise: "clean",
  denoise: "clean",
  spectralGate: "clean",
  gate: "clean",
  declick: "clean",
  dehum: "clean",
  gain: "level",
  volume: "level",
  compressor: "level",
  limiter: "level",
  duck: "level",
  mute: "level",
  meter: "level",
  parametricEq: "tone",
  graphicEq: "tone",
  bassTreble: "tone",
  firFilter: "tone",
  pitch: "tone",
  funVoiceChanger: "tone",
  mixer: "routing",
  inputSwitch: "routing",
  outputSwitch: "routing",
  delay: "routing",
  timeShift: "routing",
  recorder: "routing",
  plugin: "plugins",
};

const KIND_ORDER = new Map((Object.keys(TOOL_GROUP_OF) as NodeKind[]).map((kind, index) => [kind, index]));

export const SHELF_GROUPS: readonly { id: ShelfGroupId; label: string }[] = [
  { id: "input", label: "Inputs" },
  { id: "output", label: "Outputs" },
  { id: "clean", label: "Clean up" },
  { id: "level", label: "Level" },
  { id: "tone", label: "Tone and voice" },
  { id: "routing", label: "Routing and time" },
  { id: "canvas", label: "Canvas" },
  { id: "plugins", label: "Plug-ins" },
  { id: "setup", label: "Needs setup" },
];

/** Shown until there is history, and used to keep the top group at five rows. */
export const COMMON_TOOL_IDS = ["physical-input", "physical-output", "gain", "compressor", "mixer"] as const;
export const SHELF_ROWS = 5;
export const DEFAULT_OPEN_GROUPS: readonly ShelfGroupId[] = ["input", "output"];

/** Entries that have no kind of their own yet still belong to the kind they will add. */
export function libraryEntryKind(entry: Pick<LibraryEntry, "id" | "kind" | "virtualKind">): NodeKind {
  if (entry.kind) return entry.kind;
  if (entry.virtualKind) return entry.virtualKind;
  return "endpointLoopback";
}

export function kindOrder(kind: NodeKind): number {
  return KIND_ORDER.get(kind) ?? KIND_ORDER.size;
}

export function needsSetup(entry: Pick<LibraryEntry, "kind" | "virtualKind" | "unavailableReason">): boolean {
  return entry.unavailableReason !== undefined || (!entry.kind && !entry.virtualKind);
}

/** Newest first, no repeats, never more than the rows that can show. */
export function rememberRecent(recents: readonly string[], id: string): string[] {
  return [id, ...recents.filter((item) => item !== id)].slice(0, SHELF_ROWS);
}

export function togglePin(pins: readonly string[], id: string): string[] {
  return pins.includes(id) ? pins.filter((item) => item !== id) : [...pins, id];
}

/**
 * The rows of the top group: pins, then recent tools, then common tools to fill it, so it
 * always has the same height. Tools that cannot be added right now are skipped.
 */
export function shelfRows(pins: readonly string[], recents: readonly string[], usable: ReadonlySet<string>): string[] {
  const shownPins = pins.filter((id) => usable.has(id));
  const ordered = [...new Set([...shownPins, ...recents, ...COMMON_TOOL_IDS])].filter((id) => usable.has(id));
  return ordered.slice(0, Math.max(SHELF_ROWS, shownPins.length));
}

const RECENTS_KEY = "audiorouter.ui.tool-recents";
const PINS_KEY = "audiorouter.ui.tool-pins";
const OPEN_GROUPS_KEY = "audiorouter.ui.tools-open-groups";
const MAX_STORED_IDS = 64;
const MAX_ID_LENGTH = 128;

/** Stored values are rebuilt from allowed fields; anything else reads as nothing stored. */
function readIds(key: string): string[] | null {
  const value = readPresentationJson(optionalLocalStorage(), key);
  if (!Array.isArray(value)) return null;
  return [
    ...new Set(value.filter((item): item is string => typeof item === "string" && item.length <= MAX_ID_LENGTH)),
  ].slice(0, MAX_STORED_IDS);
}

export const readRecents = () => (readIds(RECENTS_KEY) ?? []).slice(0, SHELF_ROWS);
export const readPins = () => readIds(PINS_KEY) ?? [];
export const writeRecents = (ids: readonly string[]) => writePresentationJson(optionalLocalStorage(), RECENTS_KEY, ids);
export const writePins = (ids: readonly string[]) => writePresentationJson(optionalLocalStorage(), PINS_KEY, ids);

export function readOpenGroups(): ShelfGroupId[] {
  const known = new Set<string>(SHELF_GROUPS.map((group) => group.id));
  const stored = readIds(OPEN_GROUPS_KEY);
  return stored ? (stored.filter((id) => known.has(id)) as ShelfGroupId[]) : [...DEFAULT_OPEN_GROUPS];
}
export const writeOpenGroups = (ids: readonly ShelfGroupId[]) =>
  writePresentationJson(optionalLocalStorage(), OPEN_GROUPS_KEY, ids);
