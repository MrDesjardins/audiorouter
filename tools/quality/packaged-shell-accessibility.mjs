import assert from "node:assert/strict";
import path from "node:path";
import { readFile, writeFile } from "node:fs/promises";
import { setTimeout as delay } from "node:timers/promises";
import { expect } from "../../ui/node_modules/@playwright/test/index.mjs";

// Optional, device-free checks on the real packaged shell. No fixture adapter,
// endpoint preparation, playback, OS settings, or owner profile is used.
export async function checkPackagedAccessibility(
  page,
  rpc,
  output,
  settingsFile,
  manualKeyboard = false,
) {
  const failures = [];
  const manualStep = async (phase) => {
    await page.screenshot({
      path: path.join(output, `manual-${phase}-ready.png`),
    });
    await writeFile(
      path.join(output, "manual-keyboard-ready.json"),
      JSON.stringify({ phase }),
    );
    const deadline = Date.now() + 180000;
    while (Date.now() < deadline) {
      const confirmation = await readFile(
        path.join(output, "manual-keyboard-continue.json"),
        "utf8",
      ).catch(() => "{}");
      if (JSON.parse(confirmation).phase === phase) return;
      await delay(250);
    }
    throw new Error(
      `Manual keyboard ${phase} was not confirmed within three minutes`,
    );
  };
  const session = async () => {
    const response = await rpc("sessions.get", {
      sessionId: "desktop-session",
    });
    assert.ok(response.result, JSON.stringify(response.error));
    return response.result;
  };
  const initial = await session();
  const outputNode = initial.nodes.find(
    (node) => node.kind === "physicalOutput",
  );
  assert.ok(outputNode, "fresh desktop session has an output device tool");
  await page.locator(".react-flow__controls-fitview").click();
  await page.getByTestId(`rf__node-${outputNode.id}`).click();
  const picker = page.getByLabel("Output device endpoint", { exact: true });
  await expect(picker).toBeEnabled();
  const choices = await picker
    .locator("option:not([disabled])")
    .evaluateAll((options) =>
      options.map((option) => option.value).filter(Boolean),
    );
  assert.ok(choices.length, "at least one real output endpoint must be listed");
  const savedValue = await picker.inputValue();
  await picker.focus();
  if (manualKeyboard) await manualStep("escape");
  else {
    await picker.press("Alt+ArrowDown");
    await picker.press("ArrowDown");
    await picker.press("Escape");
  }
  const afterEscape = await picker.inputValue();
  if (afterEscape !== savedValue)
    failures.push(
      `F2: Escape retained a changed unsaved picker value (${manualKeyboard ? "physical keys" : "CDP"}).`,
    );
  await expect(picker).toBeFocused();
  assert.deepEqual(
    await session(),
    initial,
    "Escape must not commit a route edit",
  );

  if (manualKeyboard) await manualStep("tab");
  else {
    await picker.press("Alt+ArrowDown");
    await picker.press("ArrowDown");
    await picker.press("Tab");
  }
  const tabLeftField = await picker.evaluate(
    (element) => document.activeElement !== element,
  );
  if (!tabLeftField)
    failures.push(
      `F2: Tab did not leave the picker (${manualKeyboard ? "physical keys" : "CDP"}).`,
    );
  assert.deepEqual(
    await session(),
    initial,
    "Tab must not commit a route edit",
  );

  // Dismiss any native popup before independent save/language checks. CDP
  // popup keys alone do not qualify the user's physical Enter path.
  if (manualKeyboard) await manualStep("enter-save");
  else {
    await picker.press("Escape");
    await page
      .getByLabel("Output device binding", { exact: true })
      .locator("strong")
      .click();
    await picker.selectOption(choices[0]);
  }
  const selected = await picker.inputValue();
  assert.ok(choices.includes(selected), "selected a listed exact endpoint");
  if (!manualKeyboard)
    await page
      .locator(".topbar")
      .getByRole("button", { name: "Save", exact: true })
      .click();
  await expect
    .poll(
      async () =>
        (await session()).nodes.find((node) => node.id === outputNode.id)
          ?.parameters.endpointId,
    )
    .toBe(selected);
  const saved = await session();
  const keyboard = {
    escape: afterEscape === savedValue ? "passed" : "failed",
    backendUnchangedAfterEscape: true,
    tab: tabLeftField ? "passed" : "failed",
    enterAndSave: manualKeyboard ? "passed" : "not qualified",
    input: manualKeyboard ? "physical keys, owner confirmed" : "CDP",
    explicitSave: "passed",
    realOutputChoices: choices.length,
  };
  await page.screenshot({
    path: path.join(output, "native-device-keyboard.png"),
  });

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("tab", { name: "Appearance", exact: true }).click();
  const selects = page.locator("section.theme-options select");
  await expect(selects).toHaveCount(2);
  const languageRows = [];
  for (const language of ["en", "fr", "zh-Hans"]) {
    await selects.nth(0).selectOption(language);
    await expect(page.locator("html")).toHaveAttribute("lang", language);
    await expect
      .poll(
        async () => JSON.parse(await readFile(settingsFile, "utf8")).language,
      )
      .toBe(language);
    for (const theme of ["dark", "light", "high-contrast"]) {
      await selects.nth(1).selectOption(theme);
      await page.screenshot({
        path: path.join(output, `native-appearance-${language}-${theme}.png`),
      });
      assert.deepEqual(
        await session(),
        saved,
        "presentation changes must not edit the backend graph",
      );
    }
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(page.locator("html")).toHaveAttribute("lang", language);
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute(
      "data-ready",
      "true",
    );
    assert.deepEqual(
      await session(),
      saved,
      "reload must preserve the saved route",
    );
    languageRows.push({
      language,
      nativeSettingsPersisted: true,
      reloadRetainedLanguage: true,
      graphUnchanged: true,
    });
    const settingsNames = {
      en: "Settings",
      fr: "Paramètres",
      "zh-Hans": "设置",
    };
    await page
      .getByRole("button", { name: settingsNames[language], exact: true })
      .click();
    await page.locator("#workbench-tab-uiOptions").click();
    await expect(selects).toHaveCount(2);
  }
  await selects.nth(0).selectOption("en");
  await selects.nth(1).selectOption("dark");
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await page.locator(".settings-back").click();
  const status = await rpc("status.get");
  const access = await rpc("devices.getAccess");
  assert.equal(status.result.activeSessionCount, 0);
  assert.equal(access.result.allowed, false);
  return {
    failures,
    keyboard,
    languageRows,
    stopped: true,
    deviceAccessAllowed: false,
    limitations: [
      "No Narrator or native tray/menu observation.",
      "No actual OS scaling, live audio, unplug or sleep test.",
      "Reload persistence is not process-restart/no-flash evidence.",
    ],
  };
}
