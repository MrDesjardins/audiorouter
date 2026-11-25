import { expect, test, type Page } from "./browser-test";
import { demoSession } from "../src/fixtures";

// A live route reports a lost device as a red, lasting message. The message
// floats outside the workspace layout (UI-17), so nothing below it moves.
const lostDevice = /audio device this route uses was disconnected or changed/i;

async function openLiveRoute(page: Page, theme: string) {
  await page.addInitScript(
    ({ theme, session }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixtureWorkspaceEvents: [],
      });
    },
    { theme, session: demoSession },
  );
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const dismiss = page.getByRole("button", { name: "Dismiss route warning", exact: true });
  if (await dismiss.isVisible()) await dismiss.click();
}

/** Snapshot the geometry that must not move when a message appears. */
async function workspaceBounds(page: Page) {
  const box = async (selector: string) => {
    const bounds = await page.locator(selector).first().boundingBox();
    return (
      bounds && {
        x: Math.round(bounds.x),
        y: Math.round(bounds.y),
        width: Math.round(bounds.width),
        height: Math.round(bounds.height),
      }
    );
  };
  return {
    topbar: await box(".topbar"),
    tabs: await box(".workbench-tabs"),
    canvas: await box(".session-flow-canvas"),
  };
}

async function injectLostDevice(page: Page) {
  await page.evaluate(() => {
    const state = window as unknown as {
      __routeFixtureWorkspaceEvents: Array<Array<{ category: string; sessionId: string | null }>>;
    };
    state.__routeFixtureWorkspaceEvents.push([{ category: "devices.bindingInvalidated", sessionId: null }]);
  });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: a lost device on a live route shows a lasting red message without moving the workspace`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width: 1440, height: 1000 });
    await openLiveRoute(page, theme);
    const before = await workspaceBounds(page);

    await injectLostDevice(page);

    const alert = page.locator(".global-action-message.is-error").filter({ hasText: lostDevice });
    await expect(alert).toBeVisible();
    await expect(alert).toHaveAttribute("role", "alert");
    await expect(alert).toHaveAttribute("aria-live", "assertive");
    // An error must not close itself: re-check after every other message kind
    // would have expired.
    await page.waitForTimeout(1_500);
    await expect(alert).toBeVisible();
    await expect(alert).toContainText("reconnect it and press Play");

    expect(await workspaceBounds(page)).toEqual(before);
    await alert.screenshot({ path: testInfo.outputPath(`${theme}-device-loss-message.png`) });
  });

  test(`${theme}: the lost-device message covers neither tabs nor top bar in a narrow window`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width: 640, height: 690 });
    await openLiveRoute(page, theme);
    await injectLostDevice(page);

    const alert = page.locator(".global-action-message.is-error").filter({ hasText: lostDevice });
    await expect(alert).toBeVisible();
    // The narrow layout moves the notification off the tabs (the window's
    // message-placement rule); give the 250 ms recheck a moment to run.
    await expect
      .poll(() =>
        page.evaluate(() => {
          const message = document.querySelector(".workspace-notifications > *");
          const box = message?.getBoundingClientRect();
          if (!box) return ["missing"];
          const covered = [
            ...document.querySelectorAll<HTMLElement>('[aria-label="Right sidebar"] [role="tab"]'),
            ...document.querySelectorAll<HTMLElement>(".topbar button"),
          ]
            .filter((element) => {
              const other = element.getBoundingClientRect();
              return (
                other.width > 0 &&
                other.height > 0 &&
                box.left < other.right &&
                box.right > other.left &&
                box.top < other.bottom &&
                box.bottom > other.top
              );
            })
            .map((element) => element.textContent?.trim() || element.getAttribute("aria-label") || "control");
          return covered;
        }),
      )
      .toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.screenshot({ path: testInfo.outputPath(`${theme}-device-loss-narrow.png`) });
  });
}
