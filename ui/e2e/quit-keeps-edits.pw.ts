import { test, expect } from "./browser-test";
import type { Page } from "@playwright/test";

// Quit exits without asking the page, so unsaved edits go to a file through the
// shell as they are made. The bridge is faked with localStorage, which survives
// the reload that stands in for the next launch. The real exit is exercised
// against the built exe (see Round 23 in docs/plans/active/ui-quality-review.md).
const fakeShell = (page: Page) =>
  page.addInitScript(() => {
    localStorage.setItem("audiorouter.ui.first-run-done", "1");
    Object.assign(window, {
      __TAURI_INTERNALS__: {
        invoke: async (command: string, args?: { draft?: unknown }) => {
          if (command === "get_ui_draft") return JSON.parse(localStorage.getItem("test.unsaved") ?? "null");
          if (command === "set_ui_draft") {
            if (args?.draft) localStorage.setItem("test.unsaved", JSON.stringify(args.draft));
            else localStorage.removeItem("test.unsaved");
            return null;
          }
          if (command === "quit_app") {
            sessionStorage.setItem("test.quit-saw", localStorage.getItem("test.unsaved") ?? "nothing");
            return null;
          }
          if (command === "set_ui_unsaved") return null;
          throw new Error(`not available in this test: ${command}`);
        },
      },
    });
  });

const open = async (page: Page) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
};
const message = (page: Page) => page.locator(".global-action-message");
const saveButton = (page: Page) => page.locator(".topbar").getByRole("button", { name: "Save", exact: true });
const editVoiceGain = async (page: Page) => {
  const slider = page.getByTestId("rf__node-voice").getByRole("slider");
  await slider.focus();
  await page.keyboard.press("ArrowRight");
  await expect(saveButton(page)).toBeEnabled();
};
const storedName = (page: Page) => page.evaluate(() => localStorage.getItem("test.unsaved"));

test("unsaved edits come back after a restart, and Undo or Discard can drop them", async ({ page }) => {
  await fakeShell(page);
  await open(page);
  await editVoiceGain(page);
  await expect.poll(() => storedName(page)).not.toBeNull();
  const value = await page.getByTestId("rf__node-voice").getByRole("slider").getAttribute("aria-valuenow");

  await page.reload();
  await expect(message(page)).toContainText("Your unsaved edits from last time are back");
  await expect(page.getByTestId("rf__node-voice").getByRole("slider")).toHaveAttribute("aria-valuenow", value ?? "");
  await expect(saveButton(page)).toBeEnabled();

  await message(page).getByRole("button", { name: "Discard edits" }).click();
  await expect(saveButton(page)).toBeDisabled();
  await expect.poll(() => storedName(page)).toBeNull();

  await page.reload();
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await expect(message(page)).not.toContainText("from last time");
});

test("the window's Quit writes the very latest edit before it exits", async ({ page }) => {
  await fakeShell(page);
  await open(page);
  // No wait: the edit is still inside the write delay when Quit is pressed.
  const slider = page.getByTestId("rf__node-voice").getByRole("slider");
  await slider.focus();
  await page.keyboard.press("ArrowRight");
  const quit = page.getByRole("button", { name: "Quit AudioRouter" });
  await quit.click();
  await page.getByRole("button", { name: "Confirm quit AudioRouter" }).click();
  await expect.poll(() => page.evaluate(() => sessionStorage.getItem("test.quit-saw"))).toContain('"sessionId"');
});

test("edits on a route saved since then wait for a choice", async ({ page }) => {
  await fakeShell(page);
  await open(page);
  await editVoiceGain(page);
  await expect.poll(() => storedName(page)).not.toBeNull();
  // The saved route moved on while the app was closed.
  await page.evaluate(() => {
    const stored = JSON.parse(localStorage.getItem("test.unsaved") ?? "{}");
    localStorage.setItem("test.unsaved", JSON.stringify({ ...stored, baseRevision: stored.baseRevision + 7 }));
  });
  await page.reload();
  await expect(message(page)).toContainText("this route was saved since then");
  await expect(saveButton(page), "nothing is applied until the user chooses").toBeDisabled();

  await message(page).getByRole("button", { name: "Keep saved route" }).click();
  await expect.poll(() => storedName(page)).toBeNull();
  await expect(saveButton(page)).toBeDisabled();
});

test("a damaged copy is ignored and the route opens clean", async ({ page }) => {
  await fakeShell(page);
  await page.addInitScript(() => {
    if (!sessionStorage.getItem("test.seeded")) {
      sessionStorage.setItem("test.seeded", "1");
      localStorage.setItem("test.unsaved", JSON.stringify({ version: 1, sessionId: "x", draft: { nodes: "broken" } }));
    }
  });
  await open(page);
  await expect(saveButton(page)).toBeDisabled();
  await expect(message(page)).not.toContainText("from last time");
});

test("a message moves off the tabs when the page changes without a scroll or a resize", async ({ page }) => {
  await page.setViewportSize({ width: 640, height: 1400 });
  await page.addInitScript(() => localStorage.setItem("audiorouter.ui.first-run-done", "1"));
  await open(page);
  const notifications = page.locator(".workspace-notifications");
  await expect(notifications.locator("> *").first()).toBeVisible();
  // Content above the tabs grows, as a panel opening does, until they reach the message.
  const before = await page.evaluate(() => {
    const tabs = document.querySelector(".workbench-tabs")!;
    const message = document.querySelector<HTMLElement>(".workspace-notifications")!;
    const gap = message.getBoundingClientRect().top + 10 - tabs.getBoundingClientRect().bottom;
    const spacer = document.createElement("div");
    spacer.style.height = `${Math.max(gap, 0)}px`;
    const before = { gap, top: message.style.top };
    tabs.parentElement!.insertBefore(spacer, tabs);
    return before;
  });
  expect(before.gap).toBeGreaterThan(0);
  expect(before.top).toBe("");
  await expect.poll(() => notifications.evaluate((element: HTMLElement) => element.style.top)).not.toBe("");
});

// 640×690 puts the tabs in the corner with the top bar still on screen, so moving up must clear that too.
for (const [width, height] of [
  [640, 360],
  [640, 690],
  [900, 720],
] as const) {
  test(`a message covers neither tabs nor top bar at ${width}×${height}`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    await page.addInitScript(() => localStorage.setItem("audiorouter.ui.first-run-done", "1"));
    await open(page);
    // Every scroll position, with the route warning on screen.
    const scrollHeight = await page.evaluate(() => document.documentElement.scrollHeight);
    for (let top = 0; top <= scrollHeight; top += 40) {
      await page.evaluate((value) => window.scrollTo(0, value), top);
      await expect
        .poll(
          () =>
            page.evaluate(() => {
              const message = document.querySelector(".workspace-notifications > *");
              const box = message?.getBoundingClientRect();
              if (!box) return [];
              const navigation = [
                ...document.querySelectorAll('[aria-label="Right sidebar"] [role="tab"]'),
                ...document.querySelectorAll(".topbar button"),
              ];
              return navigation
                .filter((item) => {
                  const rect = item.getBoundingClientRect();
                  return (
                    rect.width > 0 &&
                    rect.left < box.right &&
                    rect.right > box.left &&
                    rect.top < box.bottom &&
                    rect.bottom > box.top
                  );
                })
                .map((item) => item.textContent);
            }),
          { message: `navigation covered at scroll ${top}` },
        )
        .toEqual([]);
    }
  });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`question dialogs and the card in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      localStorage.setItem("audiorouter.ui.first-run-done", "1");
    }, theme);
    await open(page);
    // Port detail and the built-in badge left the card; the word for the kind did not.
    const card = page.getByTestId("rf__node-voice");
    await expect(card.locator(".node-kind")).toContainText("Gain");
    await expect(card.locator(".node-kind-family")).toHaveCount(0);
    await expect(card).not.toContainText(/\bports?\b/);
    await expect(card.locator(".node-kind")).toHaveAttribute("title", /in \(1ch\)/);

    await page.getByRole("tab", { name: "Session", exact: true }).click();
    await page.getByRole("button", { name: "New", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "New session" });
    const field = dialog.getByLabel("Session name");
    await expect(field).toBeFocused();
    await field.fill("");
    await dialog.getByRole("button", { name: "Create session" }).click();
    await expect(dialog).toContainText("Enter a name first.");
    await dialog.screenshot({ path: testInfo.outputPath(`new-session-${theme}.png`) });
    const colors = await field.evaluate((element) => getComputedStyle(element).colorScheme);
    expect(colors).toBe(theme === "light" ? "light" : "dark");
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  });
}
