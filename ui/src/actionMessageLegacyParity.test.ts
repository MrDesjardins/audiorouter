/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { type ActionMessageTone, recordingStoppedEarly, say, type ToneKey } from "./actionMessage";
import { actionMessageLifetimeMs } from "./actionMessage";
import { MESSAGE_TONES } from "./actionMessageTones";
import en from "./i18n/locales/en.json";

/**
 * The English-wording rules that decided severity before L10N-06 (copied from
 * `actionMessage.ts` as it was on 2026-11-23, kept here only to prove that every
 * declared severity still equals what the old rules gave the English message).
 */
const LEGACY_ERROR = [
  /^no audio (started|played)/i,
  /^change rejected/i,
  /^(could not|unable to|cannot)\b/i,
  /\b(permission denied|not permitted|access denied)\b/i,
  /\b(failed|rejected|unavailable|cannot play|cannot prepare|is not attached|not changed)\b/i,
];
const LEGACY_WARNING = [
  /\bwarnings?\b/i,
  /\b(press play again|stop and press play|stop the session|changed while|not measured|is missing)\b/i,
  /^review\b/i,
];
const LEGACY_SUCCESS = [
  /\b(is running|was saved|route saved|saved\b|started|stopped|prepared|restored|applied|copied|connected|deleted|created)\b/i,
];
const LEGACY_PROGRESS = /(\.\.\.|…)$/;

function legacyTone(text: string): ActionMessageTone {
  if (LEGACY_ERROR.some((pattern) => pattern.test(text))) return "error";
  if (LEGACY_WARNING.some((pattern) => pattern.test(text))) return "warning";
  if (LEGACY_PROGRESS.test(text.trim())) return "info";
  if (LEGACY_SUCCESS.some((pattern) => pattern.test(text))) return "success";
  return "info";
}

const catalog = (key: string): string =>
  key
    .split(".")
    .reduce<unknown>((node, part) => (node as Record<string, unknown> | undefined)?.[part], en.messages) as string;

const render = (key: string, args: Record<string, string> = {}): string =>
  catalog(key).replace(/\{\{(\w+)[^}]*\}\}/g, (_match, name: string) => args[name] ?? "X");

/**
 * Messages that differ from the old rules on purpose (the old rules judged the
 * whole rendered sentence, so a caught error's own words changed the colour).
 * Documented in docs/plans/active/ui-localization.md, 2026-11-24.
 */
const DELIBERATE: Partial<Record<ToneKey, ActionMessageTone>> = {
  "g3Adv.signInFailed": "error",
  "quit.refusedMessage": "error",
  // The four route-recovery notices of useWorkspaceEvents were marked as errors
  // by hand before (the owner's 2026-11-07 report: "stopped" read as success and
  // closed itself); their wording alone would have read as information or success.
  "workspaceEvents.recoveryFailed": "error",
  "workspaceEvents.deviceChanged": "error",
  "workspaceEvents.bridgeTimedOut": "error",
  "workspaceEvents.bridgeTimedOutUnnamed": "error",
};

/**
 * Keys whose placeholder is a closed set the backend returns. The old rules saw
 * these words in the rendered sentence, so the declared tone must agree for every
 * reachable value (contracts/src/index.ts). `override` lists values the caller
 * handles itself, with the tone it gives them.
 */
const REACHABLE_VALUES: Partial<
  Record<ToneKey, { name: string; values: string[]; override?: Record<string, ActionMessageTone> }>
> = {
  "g2.nePrepared": { name: "state", values: ["configured-stopped"] },
  "g2.neRebound": { name: "state", values: ["configured-stopped"] },
  "g2.neDetached": { name: "state", values: ["detached"] },
  "g2.vrReplaced": { name: "state", values: ["applied"] },
  "g1.appAudioFileState": {
    name: "state",
    values: ["playing", "paused", "stopped"],
    // App.tsx keeps the green check for "stopped" (withTone).
    override: { stopped: "success" },
  },
  "recordingUi.updated": {
    name: "state",
    values: ["Idle", "Armed", "Recording", "Paused", "Stopping", "Completed", "Failed"],
    // RecorderActions shows a recorder that reports failed as an error (withTone).
    override: { Failed: "error" },
  },
};

describe("declared severity equals the old English-wording result", () => {
  // Their tone follows the value the backend returns, checked in the next test.
  const valueDriven = (key: ToneKey) => Object.hasOwn(REACHABLE_VALUES, key);

  it("for every message, with its placeholders left blank", () => {
    for (const key of (Object.keys(MESSAGE_TONES) as ToneKey[]).filter((key) => !valueDriven(key))) {
      const expected = DELIBERATE[key] ?? legacyTone(render(key));
      expect(MESSAGE_TONES[key].tone, key).toBe(expected);
      expect("progress" in MESSAGE_TONES[key], `${key} progress`).toBe(LEGACY_PROGRESS.test(render(key).trim()));
    }
  });

  it("for every value a closed-set placeholder can really take", () => {
    for (const [key, spec] of Object.entries(REACHABLE_VALUES) as [
      ToneKey,
      NonNullable<(typeof REACHABLE_VALUES)[ToneKey]>,
    ][]) {
      for (const value of spec.values) {
        const old = legacyTone(render(key, { [spec.name]: value }));
        const shown = spec.override?.[value] ?? MESSAGE_TONES[key].tone;
        expect(shown, `${key} with ${spec.name}=${value}`).toBe(old);
      }
    }
  });

  it("lists exactly the exceptions it documents", () => {
    const differing = (Object.keys(MESSAGE_TONES) as ToneKey[]).filter(
      (key) => !valueDriven(key) && MESSAGE_TONES[key].tone !== legacyTone(render(key)),
    );
    expect(differing.sort()).toEqual(Object.keys(DELIBERATE).sort());
  });
});

describe("recording stopped early", () => {
  it("looks the same with and without the backend's own reason", () => {
    const fallback = recordingStoppedEarly(undefined);
    const withReason = recordingStoppedEarly(
      "Recording stopped saving after 0:05 because audio was lost. The file keeps everything up to that point.",
    );
    // The backend's reason matched the old success rule ("stopped"), as did the fallback text.
    expect(legacyTone(withReason.text)).toBe("success");
    expect(legacyTone(fallback.text)).toBe("success");
    expect(withReason.tone).toBe(fallback.tone);
    expect(withReason.progress).toBe(fallback.progress);
    expect(actionMessageLifetimeMs(withReason, false)).toBe(actionMessageLifetimeMs(fallback, false));
    expect(withReason.text).toContain("because audio was lost");
    expect(fallback.text).toBe(say("g1.appRecordingEarly").text);
  });
});
