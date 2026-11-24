/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import {
  actionMessageLifetimeMs,
  actionMessageTone,
  BLANK_MESSAGE,
  errorMessage,
  infoMessage,
  offersAction,
  say,
  type ToneKey,
  withTone,
} from "./actionMessage";
import { MESSAGE_TONES } from "./actionMessageTones";
import { uiError } from "./backend";
import { applyLanguage, i18n } from "./i18n";
import en from "./i18n/locales/en.json";

// The severity each English message had under the earlier wording-based
// rules, so a message keeps its colour, expiry and buttons (L10N-06).
describe("status message severity", () => {
  it("marks playback, permission, and route failures as errors", () => {
    for (const key of [
      "g1.appNoAudioSaveApp",
      "g1.appNoAudioWarnings",
      "g1.appChangeRejected",
      "g1.appRefreshFailed",
      "g1.appReconnectNotRunning",
      "g2.nmUnavailable",
      "quit.unresponsiveMessage",
    ] as const) {
      expect(say(key, { error: "x", message: "x" }).tone, key).toBe("error");
    }
  });

  it("marks every caught backend error as an error whatever its wording", () => {
    const failure = new Error("enabled plugin nodes require an attached native endpoint session");
    expect(uiError(failure, "Unable to start session.").tone).toBe("error");
    expect(errorMessage("A saved route with a happy ending").tone).toBe("error");
    expect(infoMessage("Unable to start session; permission denied.").tone).toBe("info");
  });

  it("separates success, warning, and plain progress", () => {
    expect(say("g1.appSessionRunning", { generation: 4 }).tone).toBe("success");
    expect(say("g1.appRouteSaved", { revision: 2 }).tone).toBe("success");
    expect(say("g1.appRouteChangedChecking").tone).toBe("warning");
    expect(say("g1.appReviewWarnings").tone).toBe("warning");
    for (const key of [
      "g1.appCheckingRoute",
      "g1.appPreparingPaths",
      "g1.appStartingSession",
      "g1.appRedid",
    ] as const) {
      expect(say(key).tone, key).toBe("info");
    }
    expect(actionMessageTone(null)).toBe("info");
  });

  it("offers the buttons the English wording offered", () => {
    expect(offersAction(say("g1.appNoAudioSaveApp"), "goToIssueNode")).toBe(true);
    expect(offersAction(say("g1.appNoAudioNoDevices"), "goToIssueNode")).toBe(true);
    expect(offersAction(say("g1.appNoAudioNoDevices"), "openStatus")).toBe(true);
    expect(offersAction(say("g1.appNoAudioNeedInput"), "openStatus")).toBe(true);
    expect(offersAction(say("g1.appEditsRestored"), "discardEdits")).toBe(true);
    expect(offersAction(say("g1.appOlderEdits"), "olderEdits")).toBe(true);
    expect(offersAction(say("g1.appRedid"), "goToIssueNode")).toBe(false);
    expect(offersAction(null, "openStatus")).toBe(false);
  });
});

describe("message lifetime", () => {
  it("closes info after 8 s and warnings after 30 s, but keeps errors, progress and messages with a button", () => {
    expect(actionMessageLifetimeMs(say("g1.appConnectionRemoved"), false)).toBe(8_000);
    expect(actionMessageLifetimeMs(say("g1.appRouteSaved", { revision: 1 }), false)).toBe(8_000);
    expect(actionMessageLifetimeMs(say("g1.appReviewWarnings"), false)).toBe(30_000);
    expect(actionMessageLifetimeMs(errorMessage("Unable to start session."), false)).toBeNull();
    expect(actionMessageLifetimeMs(say("g1.appSavingRoute"), false)).toBeNull();
    expect(actionMessageLifetimeMs(say("g1.appConnectionRemoved"), true)).toBeNull();
    expect(actionMessageLifetimeMs(null, false)).toBeNull();
  });

  it("keeps a warning that is still in progress until it changes", () => {
    const working = { ...infoMessage("Checking…"), tone: "warning", progress: true } as const;
    expect(actionMessageLifetimeMs(working, false)).toBeNull();
  });

  it("holds the status line's space with a blank message that closes like information", () => {
    expect(BLANK_MESSAGE.text).toBe(" ");
    expect(actionMessageLifetimeMs(BLANK_MESSAGE, false)).toBe(8_000);
  });
});

describe("translated status messages", () => {
  afterEach(async () => {
    await applyLanguage("en");
  });

  it("keep the severity, expiry and buttons of the message in every language", async () => {
    for (const language of ["fr", "zh-Hans"]) {
      await applyLanguage(language);
      expect(i18n.language).toBe(language);

      const refused = say("g1.appNoAudioWarnings");
      expect(refused.text).not.toBe(en.messages.g1.appNoAudioWarnings);
      expect(refused.tone).toBe("error");
      expect(actionMessageLifetimeMs(refused, false)).toBeNull();
      expect(offersAction(refused, "goToIssueNode")).toBe(true);

      const warning = say("g1.appRouteChangedChecking");
      expect(warning.text).not.toBe(en.messages.g1.appRouteChangedChecking);
      expect(warning.tone).toBe("warning");
      expect(actionMessageLifetimeMs(warning, false)).toBe(30_000);

      const saved = say("g1.appRouteSaved", { revision: 3 });
      expect(saved.tone).toBe("success");
      expect(actionMessageLifetimeMs(saved, false)).toBe(8_000);

      const progress = say("g1.appStopping");
      expect(progress.progress).toBe(true);
      expect(actionMessageLifetimeMs(progress, false)).toBeNull();
    }
  });

  it("shows a failure that wraps a caught error as an error", () => {
    // The old rules judged the whole sentence, so a reason carrying failure words made these red.
    expect(say("g3Adv.signInFailed", { reason: "Access is denied" }).tone).toBe("error");
    expect(say("quit.refusedMessage", { reason: "a recording is still finishing" }).tone).toBe("error");
    expect(withTone(say("recordingUi.updated", { state: "Failed", frame: 0 }), "error").tone).toBe("error");
    expect(say("recordingUi.updated", { state: "Failed", frame: 0 }).tone).toBe("info");
  });

  it("does not let a name or reason inside the text change the severity", () => {
    const named = say("g1.appOtherPlayingNamed", { name: "Saved failed started" });
    expect(named.tone).toBe("error");
    const done = say("g1.appUndid", { name: "permission denied: failed" });
    expect(done.tone).toBe("info");
  });
});

describe("message table", () => {
  const catalog = (key: string): string =>
    key
      .split(".")
      .reduce<unknown>((node, part) => (node as Record<string, unknown> | undefined)?.[part], en.messages) as string;

  it("only lists messages that exist in the English catalog", () => {
    for (const key of Object.keys(MESSAGE_TONES)) {
      expect(typeof catalog(key), key).toBe("string");
    }
  });

  it("marks progress and buttons the way the English message reads", () => {
    for (const key of Object.keys(MESSAGE_TONES) as ToneKey[]) {
      const text = catalog(key).trim();
      const entry: { tone: string; progress?: true; actions?: readonly string[] } = MESSAGE_TONES[key];
      expect(entry.progress === true, `${key} progress`).toBe(/(\.\.\.|…)$/.test(text));
      expect(entry.actions?.includes("goToIssueNode") === true, `${key} go to node`).toBe(
        text.startsWith("No audio started"),
      );
      expect(entry.actions?.includes("openStatus") === true, `${key} open status`).toBe(text.includes("Troubleshoot"));
    }
  });
});
