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

  it("localizes the retry hint while keeping the backend's technical detail", async () => {
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
    expect(text).toContain("Une nouvelle tentative peut aboutir.");
    expect(text).toContain("[audioFailure]");
    expect(text).toContain("retry the operation");
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
