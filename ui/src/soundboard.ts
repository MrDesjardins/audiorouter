// Soundboard slot helpers shared by the canvas card, the Properties editor and the picker.
import type { Node } from "@audiorouter/contracts";

export const SOUNDBOARD_SLOTS = 9;
/** Longest caption the backend accepts (bytes); kept in characters here, which is stricter for ASCII only. */
export const MAX_CAPTION_LENGTH = 64;
export const SOUNDBOARD_MODES = ["once", "loop", "toggle", "hold"] as const;
export type SoundboardMode = (typeof SOUNDBOARD_MODES)[number];
export const SOUNDBOARD_MODE_LABELS: Record<SoundboardMode, string> = {
  once: "Play once",
  loop: "Loop",
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
};

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
    const mediaId = node.parameters[slotParameter(slot, "MediaId")];
    if (typeof mediaId !== "string" || mediaId === "") continue;
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
      label: typeof caption === "string" && caption !== "" ? caption : stripExtension(name) || `Sound ${slot}`,
      gainDb: typeof gain === "number" ? gain : 0,
      mode: SOUNDBOARD_MODES.find((candidate) => candidate === mode) ?? "once",
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

function stripExtension(name: string): string {
  return name.replace(/\.[A-Za-z0-9]{1,5}$/, "");
}
