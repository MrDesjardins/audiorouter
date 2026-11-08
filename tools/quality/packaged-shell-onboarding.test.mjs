import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "../../ui/node_modules/@playwright/test/index.mjs";
import { dismissFirstLaunchGoal } from "./packaged-shell-onboarding.mjs";

test("packaged smoke declines the delayed goal picker before Session navigation", async () => {
  const browser = await chromium.launch({ channel: "msedge" });
  try {
    const page = await browser.newPage();
    await page.setContent(`
      <button role="tab" onclick="document.body.dataset.session = 'open'">Session</button>
      <script>
        setTimeout(() => {
          const dialog = document.createElement('dialog');
          dialog.setAttribute('aria-label', 'What do you want to do?');
          const button = document.createElement('button');
          button.textContent = 'Not now';
          button.onclick = () => {
            document.body.dataset.declined = 'true';
            dialog.remove();
          };
          dialog.append(button);
          document.body.append(dialog);
          dialog.showModal();
        }, 100);
      </script>
    `);
    await dismissFirstLaunchGoal(page, 5_000);
    assert.equal(await page.locator("body").getAttribute("data-declined"), "true");
    await page.getByRole("tab", { name: "Session", exact: true }).click();
    assert.equal(await page.locator("body").getAttribute("data-session"), "open");
    assert.equal(await page.getByRole("dialog").count(), 0);
  } finally {
    await browser.close();
  }
});
