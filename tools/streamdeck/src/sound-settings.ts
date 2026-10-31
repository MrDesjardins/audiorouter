// Settings and naming of the Sound key, kept free of the Stream Deck SDK so they can be tested.

export type SoundBehavior = "slot" | "once" | "loop" | "hold";

export type SoundSettings = {
  /** The Soundboard tool, by ID, with its name to find it again if it is re-created. */
  nodeId?: string;
  nodeName?: string;
  /** 1 to 9: the button on the Soundboard. */
  slot?: number;
  /** How a press plays: the slot's own setting, or once, loop until pressed again, or while held. */
  behavior?: SoundBehavior;
  /** A name for the key; empty shows the sound's caption from AudioRouter. */
  label?: string;
};

const BEHAVIORS: readonly SoundBehavior[] = ["slot", "once", "loop", "hold"];

/** The chosen behavior, "slot" when none or an unknown one is saved. */
export function soundBehavior(settings: SoundSettings): SoundBehavior {
  return BEHAVIORS.find((behavior) => behavior === settings.behavior) ?? "slot";
}

/** The slot number, or null when the key has none chosen. */
export function soundSlot(settings: SoundSettings): number | null {
  const slot = Number(settings.slot);
  return Number.isInteger(slot) && slot >= 1 && slot <= 9 ? slot : null;
}

/** What the Soundboard calls the button: its caption, else the file name; null when it has neither. */
export function slotName(read: (name: string) => unknown, slot: number): string | null {
  const caption = read(`slot${slot}Caption`);
  if (typeof caption === "string" && caption.trim()) return caption.trim();
  const file = read(`slot${slot}FileName`);
  return typeof file === "string" && file.trim() ? file.replace(/\.[A-Za-z0-9]{1,5}$/, "") : null;
}

/** What the button shows in AudioRouter: its caption, else the file name, else "Sound n". */
export function slotCaption(read: (name: string) => unknown, slot: number): string {
  return slotName(read, slot) ?? `Sound ${slot}`;
}

/**
 * The name of a Sound key, in priority order: the title typed in Stream Deck,
 * the key's own label, the Soundboard's caption or file name. `null` when none
 * exists, and the face then falls back to "SOUND n".
 */
export function keyName(title: string, settings: SoundSettings, read: (name: string) => unknown, slot: number): string | null {
  return title.trim() || settings.label?.trim() || slotName(read, slot);
}

/** The nine slots of a Soundboard, for the settings panel. */
export function soundSlotChoices(read: (name: string) => unknown): { slot: number; label: string; filled: boolean }[] {
  return Array.from({ length: 9 }, (_, index) => {
    const slot = index + 1;
    const media = read(`slot${slot}MediaId`);
    const filled = typeof media === "string" && media !== "";
    return { slot, filled, label: filled ? slotCaption(read, slot) : "(empty)" };
  });
}

/**
 * The `soundboard.trigger` parameters for a press or release. A key with its
 * own behavior sends it as `mode`; "slot" leaves the slot's own setting.
 */
export function triggerParams(
  sessionId: string,
  nodeId: string,
  slot: number,
  settings: SoundSettings,
  press: boolean,
): Record<string, unknown> {
  const behavior = soundBehavior(settings);
  return {
    sessionId,
    nodeId,
    slot,
    action: press ? "press" : "release",
    ...(behavior === "slot" ? {} : { mode: behavior }),
  };
}

/** Whether letting go of the key should send a release: only "play while held" listens. */
export function releasesOnKeyUp(settings: SoundSettings, slotMode: unknown): boolean {
  const behavior = soundBehavior(settings);
  return (behavior === "slot" ? slotMode : behavior) === "hold";
}
