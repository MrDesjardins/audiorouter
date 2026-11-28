/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import { AudioRouterRpcError } from "@audiorouter/contracts";
import { applyLanguage } from "./i18n";
import { formatUiError } from "./backend";

afterEach(async () => {
  await applyLanguage("en");
});

describe("localized recovery sentences", () => {
  it("uses the selected language for the fixed sentences", async () => {
    await applyLanguage("fr");
    expect(formatUiError(new Error("native graph rejected: UnsupportedTopology"), "Start failed")).toContain(
      "combinaison audio",
    );
    expect(formatUiError(new Error("nodes[0].parameters.gainDb: parameter is invalid"), "Save failed")).toContain(
      "Un réglage de l’outil (gainDb)",
    );
  });

  it("localizes recovery guidance without exposing wire codes or generic remediation", async () => {
    await applyLanguage("fr");
    const retryable = new AudioRouterRpcError({
      code: -32000,
      message: "audio endpoint failed",
      data: {
        code: "audioFailure",
        fieldPath: null,
        resourceIds: [],
        retryable: true,
        remediation: "retry the operation",
      },
    });
    const text = formatUiError(retryable, "fallback");
    expect(text).toContain("Audio endpoint failed.");
    expect(text).toContain("Réessayez.");
    expect(text).toContain("Journaux et assistance");
    expect(text).not.toContain("[audioFailure]");
    expect(text).not.toContain("retry the operation");
  });

  it("translates the new errors on every language switch and preserves Chinese punctuation", async () => {
    const failure = new AudioRouterRpcError({
      code: -32000,
      message: "unsupported format",
      data: { code: "unsupportedFormat", fieldPath: null, resourceIds: [], retryable: false, remediation: "" },
    });
    await applyLanguage("en");
    expect(formatUiError(failure, "fallback")).toContain("This device uses a sound format");
    await applyLanguage("fr");
    expect(formatUiError(failure, "fallback")).toContain("AudioRouter ne peut pas ouvrir");
    await applyLanguage("zh-Hans");
    expect(formatUiError(failure, "fallback")).toContain("AudioRouter 无法打开");
    expect(formatUiError(new Error("设备不可用。"), "fallback")).toBe("设备不可用。");
    await applyLanguage("en");
    expect(formatUiError(failure, "fallback")).toContain("This device uses a sound format");
  });

  it("localizes the named busy-endpoint guidance in Simplified Chinese", async () => {
    await applyLanguage("zh-Hans");
    const { rememberDeviceNames } = await import("./backend");
    rememberDeviceNames([
      { id: "cable", name: "CABLE Input (VB-Audio Virtual Cable)", direction: "render", defaultRoles: [] } as never,
    ]);
    const busy = new AudioRouterRpcError({
      code: -32000,
      message: "audio endpoint is busy",
      data: {
        code: "audioEndpointBusy",
        fieldPath: null,
        resourceIds: ["cable"],
        retryable: false,
        remediation: "release the endpoint",
        hresult: 0x8889000a,
      } as never,
    });
    expect(formatUiError(busy, "fallback")).toContain("正被另一个应用独占使用");
  });
});
