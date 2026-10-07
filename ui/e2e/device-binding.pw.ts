import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// A device node with no saved device must show an empty picker, not the device
// remembered from another node or an earlier choice. Showing a borrowed device
// made choosing it a no-op, so Save stored the node without a device and Play
// refused with "Choose the device for …" (support report 2026-10-03).
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`unbound output shows no borrowed device in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        localStorage.setItem(
          `audiorouter.ui.endpoint-binding.${session.id}`,
          JSON.stringify({ captureEndpointId: "", renderEndpointId: "render-preview" }),
        );
        Object.assign(window, { __routeFixtureSession: session });
      },
      { session: demoSession, theme },
    );
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-headphones").click();
    const picker = page.getByLabel("Physical output endpoint");
    await expect(picker).toHaveValue("");
    const binding = page.getByLabel("Physical output binding");
    await expect(binding).toContainText("No device is chosen for this node yet. Choose one, then Save.");
    await binding.scrollIntoViewIfNeeded();
    await binding.screenshot({ path: testInfo.outputPath(`device-binding-${theme}.png`) });

    await picker.selectOption("render-preview");
    await expect(picker).toHaveValue("render-preview");
    await expect(binding).not.toContainText("No device is chosen");
    await expect(page.getByText(/Unsaved: endpointId/)).toBeVisible();
    expect(
      await page.evaluate(() =>
        (window as unknown as { __routeFixtureCalls: () => string[] })
          .__routeFixtureCalls()
          .filter((call) => call === "commit"),
      ),
    ).toEqual([]);
  });
}
