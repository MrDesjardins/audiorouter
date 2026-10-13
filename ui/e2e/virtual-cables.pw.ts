import type { Page } from "@playwright/test";
import { test, expect, type RealBackend } from "./real-backend";
import type { DiagnosticsSnapshot, VirtualCableStatus } from "../../contracts/src/index";

// The offline backend has no cable driver, so these tests answer
// `virtualCable.status` (and add a cable route to `system.diagnostics`)
// themselves and pass every other request to the real backend.

function cableStatus(state: VirtualCableStatus["state"]): VirtualCableStatus {
  const installed = state !== "notInstalled";
  return {
    state,
    driverVersion: installed ? "0.1.4.0" : null,
    protocol: installed ? "1.1" : null,
    defaultPeriodFrames: installed ? 480 : null,
    cableCount: installed ? 2 : 0,
    cables: installed
      ? ["cable-a", "cable-b"].map((busId) => ({
          busId,
          name: `Cable ${busId.endsWith("a") ? "A" : "B"}`,
          renderEndpointId: `{0.0.0.00000000}.{render-${busId}}`,
          captureEndpointId: `{0.0.1.00000000}.{capture-${busId}}`,
          present: state !== "needsRepair" || busId === "cable-a",
        }))
      : [],
  };
}

type CableRoute = NonNullable<DiagnosticsSnapshot["cableRoute"]>;

function cableRoute(forwarded: number, failure: string | null = null): CableRoute {
  return {
    sessionId: "e2e-session",
    generation: 1,
    renderBus: "cable-a",
    captureBus: "cable-b",
    blockFrames: 480,
    renderChannels: 2,
    captureChannels: 2,
    state: failure ? "failed" : "running",
    failure,
    renderBlocks: forwarded,
    renderSequenceGaps: 0,
    captureBlocks: forwarded,
    forwarded,
    silence: 3,
    dropped: 0,
    trimmed: 1,
    engineSilentQuanta: 0,
  };
}

async function fakeCables(page: Page, backend: RealBackend) {
  const fake = { status: cableStatus("installed") as VirtualCableStatus | null, route: null as CableRoute | null };
  await page.route("**/__e2e_rpc", async (route) => {
    const request = route.request().postDataJSON();
    if (request.method === "virtualCable.status") {
      await route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({ jsonrpc: "2.0", id: request.id, result: fake.status }),
      });
      return;
    }
    if (request.method === "devices.list") {
      const rows = [
        {
          id: "{0.0.0.00000000}.{render-cable-a}",
          name: "Cable A Input",
          direction: "render",
          state: "active",
          defaultRoles: [],
          format: {
            sampleRateHz: 44_100,
            channels: 2,
            bitsPerSample: 32,
            formatTag: 0xfffe,
            isFloat32: true,
            bytesPerFrame: 8,
          },
          periods: { default100ns: 100_000, minimum100ns: 26_666 },
        },
        {
          id: "{0.0.1.00000000}.{capture-cable-a}",
          name: "Cable A Output",
          direction: "capture",
          state: "active",
          defaultRoles: [],
          format: {
            sampleRateHz: 48_000,
            channels: 2,
            bitsPerSample: 32,
            formatTag: 3,
            isFloat32: true,
            bytesPerFrame: 8,
          },
          periods: { default100ns: 100_000, minimum100ns: 26_666 },
        },
      ];
      await route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({ jsonrpc: "2.0", id: request.id, result: rows }),
      });
      return;
    }
    if ((request.method === "system.diagnostics" || request.method === "system.snapshot") && fake.route) {
      const response = await backend.send(request);
      if (response.result?.diagnostics)
        response.result = {
          ...response.result,
          diagnostics: { ...response.result.diagnostics, cableRoute: fake.route },
        };
      else if (response.result) response.result = { ...response.result, cableRoute: fake.route };
      await route.fulfill({ contentType: "application/json", body: JSON.stringify(response) });
      return;
    }
    await route.fallback();
  });
  return fake;
}

async function openSetup(page: Page) {
  await page.getByRole("tab", { name: "Setup" }).click();
  const panel = page.locator(".virtual-cable-panel");
  await expect(panel).toBeVisible();
  return panel;
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: the cable panel keeps its layout while status and counters change`, async ({
    page,
    backend,
  }, testInfo) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    const fake = await fakeCables(page, backend);
    await page.goto("/backend-harness.html");
    await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
    await page.getByLabel("Color theme").selectOption(theme);
    const panel = await openSetup(page);
    const pill = panel.getByRole("status");
    await expect(pill).toHaveText("Installed");
    await expect(panel).toContainText("Driver 0.1.4.0 · protocol 1.1");
    await expect(panel).toContainText("Cable A Input → Cable A Output");
    await expect(panel).toContainText("Input: 44,100 Hz, 2 ch, 32-bit float");
    await expect(panel).toContainText("Use 48 kHz float for the best match.");
    await expect(panel.getByRole("button", { name: /Add route: Cable A → Cable B/ })).toBeEnabled();
    await panel.scrollIntoViewIfNeeded();
    await panel.screenshot({ path: testInfo.outputPath(`${theme}-cables-installed.png`) });
    await panel
      .locator(".virtual-cable-formats")
      .first()
      .screenshot({
        path: testInfo.outputPath(`${theme}-cable-format-hint.png`),
      });
    // 17 §5.5: names are AudioRouter labels; the note says how to rename in Windows.
    const nameNote = panel.locator(".virtual-cable-name-note");
    await expect(nameNote).toContainText("These names are AudioRouter's labels.");
    await nameNote.scrollIntoViewIfNeeded();
    await nameNote.locator("..").screenshot({ path: testInfo.outputPath(`${theme}-cable-names.png`) });

    // Content below the panel and the pill's size must not move while the
    // status changes state (same cables) and a route starts, counts and fails.
    const below = page.locator(".this-pc-devices").first();
    const samples = new Set<string>();
    const sample = async () => {
      const [panelBox, belowBox, pillBox] = await Promise.all([
        panel.boundingBox(),
        below.boundingBox(),
        pill.boundingBox(),
      ]);
      // Relative to the panel's top, so scrolling does not count as a shift.
      samples.add(`${belowBox!.y - panelBox!.y}|${pillBox!.width}x${pillBox!.height}`);
    };
    await sample();
    for (const state of ["needsRestart", "incompatible", "installed"] as const) {
      fake.status = cableStatus(state);
      await expect(pill).toHaveText(
        { needsRestart: "Restart needed", incompatible: "Update needed", installed: "Installed" }[state],
        { timeout: 5_000 },
      );
      await sample();
    }
    // Diagnostics refresh only while playing, so each route state is loaded
    // with the page; the theme is remembered across reloads.
    for (const [route, text] of [
      [cableRoute(9_876_543), "9,876,543"],
      [cableRoute(9_876_543, "The cable bridge lease was lost."), "The cable bridge lease was lost."],
      [null, "Not playing"],
    ] as const) {
      fake.route = route;
      await page.reload();
      await openSetup(page);
      await expect(pill).toHaveText("Installed", { timeout: 5_000 });
      await expect(panel).toContainText(text);
      await sample();
      if (route && !route.failure) {
        await panel.scrollIntoViewIfNeeded();
        await panel.screenshot({ path: testInfo.outputPath(`${theme}-cables-running.png`) });
      }
    }
    expect([...samples]).toHaveLength(1);
  });
}

test("cable nodes are offered only with installed cables and take the default cables", async ({ page, backend }) => {
  const fake = await fakeCables(page, backend);
  fake.status = cableStatus("notInstalled");
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  const toolCard = (name: string) => page.locator(".tool-card").filter({ has: page.getByText(name, { exact: true }) });
  await expect(toolCard("Cable input")).toBeDisabled();
  await expect(toolCard("Cable input")).toContainText("Install AudioRouter cables in Setup");
  const panel = await openSetup(page);
  await expect(panel.getByRole("status")).toHaveText("Not installed");
  await expect(panel.getByRole("button", { name: /Add route:/ })).toBeDisabled();

  fake.status = cableStatus("installed");
  await expect(panel.getByRole("status")).toHaveText("Installed", { timeout: 5_000 });
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await expect(toolCard("Cable input")).toBeEnabled();
  await toolCard("Cable input").click();
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await toolCard("Cable output").click();
  const source = page.getByTestId("rf__node-virtual-render-source-1");
  const sink = page.getByTestId("rf__node-virtual-capture-sink-1");
  await expect(source).toContainText("Cable A Input");
  await expect(sink).toContainText("Cable B Output");

  // Properties move the destination to Cable A; its default name follows.
  await sink.locator(".flow-node-title").click();
  const picker = page.locator(".main-content > .inspector").getByRole("combobox", { name: "Cable", exact: true });
  await expect(picker).toHaveValue("cable-b");
  const style = await picker.evaluate((element) => {
    const computed = getComputedStyle(element);
    return { radius: computed.borderRadius, shadow: computed.boxShadow, background: computed.backgroundImage };
  });
  expect(style).toEqual({ radius: "9px", shadow: "none", background: "none" });
  await picker.selectOption("cable-a");
  await expect(sink).toContainText("Cable A Output");

  // Setup adds the whole Cable A → Cable B pass-through in one step.
  await (await openSetup(page)).getByRole("button", { name: "Add route: Cable A → Cable B" }).click();
  await expect(page.getByTestId("rf__node-virtual-render-source-2")).toContainText("Cable A Input");
  await expect(page.getByTestId("rf__node-virtual-capture-sink-2")).toContainText("Cable B Output");
  await expect(page.locator(".global-action-message")).toContainText("Added Cable A → Cable B to the draft");
});
