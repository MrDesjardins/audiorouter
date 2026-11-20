/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import { applyLanguage } from "./i18n";
import { actionMessageLifetimeMs, actionMessageTone } from "./actionMessage";
import { englishOf, tm } from "./actionText";

describe("translated status messages", () => {
  afterEach(async () => {
    await applyLanguage("en");
  });

  it("keep the tone and button rules of their English wording", async () => {
    await applyLanguage("zh-Hans");
    const refused = tm("g1.appNoAudioWarnings");
    expect(refused).not.toBe(englishOf(refused));
    expect(englishOf(refused).startsWith("No audio started")).toBe(true);
    expect(actionMessageTone(refused)).toBe("error");
    expect(actionMessageLifetimeMs(refused, false)).toBeNull();

    const progress = tm("g1.appStopping");
    expect(actionMessageTone(progress)).toBe("info");
    expect(actionMessageLifetimeMs(progress, false)).toBeNull();

    expect(actionMessageTone(tm("g1.appRedid"))).toBe(actionMessageTone("Redid the change."));
  });

  it("leaves English text and unknown text unchanged", () => {
    expect(englishOf("Plain text")).toBe("Plain text");
    expect(englishOf(null)).toBe("");
    expect(tm("g1.appStopped")).toBe("Session stopped.");
  });
});
