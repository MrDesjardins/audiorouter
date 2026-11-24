/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import { applyLanguage } from "./i18n";
import { backendText, tm } from "./actionText";

describe("translated text outside React", () => {
  afterEach(async () => {
    await applyLanguage("en");
  });

  it("translates a message in the active language", async () => {
    expect(tm("g1.appStopped")).toBe("Session stopped.");
    await applyLanguage("zh-Hans");
    expect(tm("g1.appStopped")).not.toBe("Session stopped.");
  });

  it("translates the fixed disconnected-backend texts and leaves other backend text alone", async () => {
    await applyLanguage("fr");
    expect(backendText("The control backend is disconnected.")).toBe(tm("h3.backendDisconnected"));
    expect(backendText("Some other backend reason")).toBe("Some other backend reason");
    expect(backendText(null)).toBeNull();
  });
});
