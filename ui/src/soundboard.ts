// Soundboard slot helpers shared by the canvas card, the Properties editor and the picker.
import type { Node, Session } from "@audiorouter/contracts";
import { removeNodeDraftParameters, setNodeDraftParameter } from "./draft";

export const SOUNDBOARD_SLOTS = 9;
/** Longest caption the backend accepts (bytes); kept in characters here, which is stricter for ASCII only. */
export const MAX_CAPTION_LENGTH = 64;
export const SOUNDBOARD_MODES = ["once", "loop", "toggle", "hold"] as const;
export type SoundboardMode = (typeof SOUNDBOARD_MODES)[number];
export const SOUNDBOARD_MODE_LABELS: Record<SoundboardMode, string> = {
  once: "Play once",
  loop: "Loop until pressed again",
  toggle: "Toggle on/off",
  hold: "Play while held",
};
/** Library references are stored as `lib:<id>` instead of copying the sound into the media store. */
export const SOUND_LIBRARY_PREFIX = "lib:";

export type SoundboardSlot = {
  /** 1 to 9, the number used by the API and the Stream Deck action. */
  slot: number;
  mediaId: string;
  fileName: string;
  /** What the button shows: the caption, or the file name when no caption is set. */
  label: string;
  caption: string;
  gainDb: number;
  mode: SoundboardMode;
  /**
   * A random set (SND-17): a library category id, or a list of sounds. Set through the
   * API; it plays instead of `mediaId`, so assigning a sound clears it.
   */
  set: string | readonly string[] | null;
};

/** A slot's set, or null when it has none (contracts type parameters as scalars; the backend also sends lists). */
function slotSet(node: Node, slot: number): string | readonly string[] | null {
  const value: unknown = node.parameters[slotParameter(slot, "Set")];
  if (typeof value === "string" && value !== "") return value;
  if (Array.isArray(value)) {
    const items = value.filter((item): item is string => typeof item === "string" && item !== "");
    return items.length > 0 ? items : null;
  }
  return null;
}

const SLOT_PARAMETER = /^slot[1-9](MediaId|FileName|Caption|GainDb|Mode|Set)$/;

/** Whether a node parameter name belongs to a Soundboard slot. */
export function isSoundboardParameter(name: string): boolean {
  return SLOT_PARAMETER.test(name);
}

export function slotParameter(slot: number, field: "MediaId" | "FileName" | "Caption" | "GainDb" | "Mode" | "Set") {
  return `slot${slot}${field}`;
}

/** The slots that have a sound, in order. */
export function soundboardSlots(node: Node): SoundboardSlot[] {
  const slots: SoundboardSlot[] = [];
  for (let slot = 1; slot <= SOUNDBOARD_SLOTS; slot += 1) {
    const media = node.parameters[slotParameter(slot, "MediaId")];
    const mediaId = typeof media === "string" ? media : "";
    const set = slotSet(node, slot);
    if (mediaId === "" && set === null) continue;
    const fileName = node.parameters[slotParameter(slot, "FileName")];
    const caption = node.parameters[slotParameter(slot, "Caption")];
    const gain = node.parameters[slotParameter(slot, "GainDb")];
    const mode = node.parameters[slotParameter(slot, "Mode")];
    const name = typeof fileName === "string" ? fileName : "";
    slots.push({
      slot,
      mediaId,
      fileName: name,
      caption: typeof caption === "string" ? caption : "",
      label:
        typeof caption === "string" && caption !== ""
          ? caption
          : stripExtension(name) || (set === null ? `Sound ${slot}` : setLabel(set)),
      gainDb: typeof gain === "number" ? gain : 0,
      mode: SOUNDBOARD_MODES.find((candidate) => candidate === mode) ?? "once",
      set,
    });
  }
  return slots;
}

/** The lowest unused slot number, or null when all nine are taken. */
export function firstFreeSlot(node: Node): number | null {
  const used = new Set(soundboardSlots(node).map((item) => item.slot));
  for (let slot = 1; slot <= SOUNDBOARD_SLOTS; slot += 1) if (!used.has(slot)) return slot;
  return null;
}

/** What a random set is called when its slot has no caption or file name. */
export function setLabel(set: string | readonly string[]): string {
  return typeof set === "string" ? `Random: ${set}` : `Random: ${set.length} sounds`;
}

/**
 * Puts one sound on a slot as a single draft change. The slot's random set is removed,
 * because a set plays instead of the sound; the caption is cleared unless `keepCaption`
 * (Change and a drop on a filled slot keep it, Add does not).
 */
export function assignSoundboardSlotDraft(
  session: Session,
  nodeId: string,
  slot: number,
  sound: { mediaId: string; fileName: string },
  keepCaption: boolean,
): Session {
  const removed = [slotParameter(slot, "Set"), ...(keepCaption ? [] : [slotParameter(slot, "Caption")])];
  return setNodeDraftParameter(
    setNodeDraftParameter(
      removeNodeDraftParameters(session, nodeId, removed),
      nodeId,
      slotParameter(slot, "MediaId"),
      sound.mediaId,
    ),
    nodeId,
    slotParameter(slot, "FileName"),
    sound.fileName,
  );
}

function stripExtension(name: string): string {
  return name.replace(/\.[A-Za-z0-9]{1,5}$/, "");
}
