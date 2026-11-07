/** What a five-second listen found. It only knows the level at this input (UI-21's voice zone). */
export type MicVerdict = "good" | "quiet" | "loud" | "silent";

export const MIC_TEST_MS = 5000;
/** Below this the input is treated as no sound, as in UI-21's "waiting for sound". */
const SILENT_DB = -50;
const ZONE_LOW_DB = -18;
const ZONE_HIGH_DB = -6;

/** Judged on the loudest peak heard, so one quiet moment in a sentence does not fail the test. */
export function micVerdict(loudestPeakDb: number): MicVerdict {
  if (!(loudestPeakDb >= SILENT_DB)) return "silent";
  if (loudestPeakDb < ZONE_LOW_DB) return "quiet";
  if (loudestPeakDb <= ZONE_HIGH_DB) return "good";
  return "loud";
}

export const MIC_VERDICT_TEXT: Record<MicVerdict, { title: string; detail: string }> = {
  good: {
    title: "Good: we hear you, in the voice zone",
    detail: "This checks the level at this input only. It cannot tell what other apps hear.",
  },
  quiet: {
    title: "We hear you, but quietly",
    detail: "Move closer or raise the input level, then test again. This checks this input only.",
  },
  loud: {
    title: "We hear you, but it is loud",
    detail: "Lower the input level so words stay under −6 dB, then test again. This checks this input only.",
  },
  silent: {
    title: "No sound from this mic",
    detail: "It may be muted in Windows or on the mic itself. Check, then test again.",
  },
};

/** The loudest current peak across channels, or null when the input reports nothing. */
export function currentPeak(meter: { currentPeakDb?: number; channelCurrentPeakDb?: number[] } | null | undefined) {
  if (!meter) return null;
  const values = [meter.currentPeakDb, ...(meter.channelCurrentPeakDb ?? [])].filter(
    (value): value is number => typeof value === "number" && Number.isFinite(value),
  );
  return values.length > 0 ? Math.max(...values) : null;
}
