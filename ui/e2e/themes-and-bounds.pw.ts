import { test, expect } from "./real-backend";
import { mkdir } from "node:fs/promises";
import path from "node:path";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: new processor controls are readable, labelled and contained at 1280x720`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/backend-harness.html");
    await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
    await page.getByLabel("Color theme").selectOption(theme);
    // Node cards follow the theme: strong card text is light on the dark
    // themes and dark on the light theme.
    const strongText = { dark: "rgb(246, 248, 251)", light: "rgb(20, 32, 51)", "high-contrast": "rgb(255, 255, 255)" }[theme]!;
    await expect(page.getByTestId("rf__node-voice").locator(".node-fader-readout strong")).toHaveCSS("color", strongText);
    await expect(page.getByTestId("rf__node-voice").locator(".flow-node-title strong")).toHaveCSS("color", strongText);
    const directory = path.resolve("../target/feature-confidence-visual");
    await mkdir(directory, { recursive: true });
    for (const [name, kind, control] of [["Dehum", "dehum", "Harmonics precise value"], ["Input Switch", "inputSwitch", "Active input"], ["Network Send", "networkSend", "Receiving computer's IP address"], ["Network Receive", "networkReceive", "Sending computer's IP address"]]) {
      await page.getByRole("tab", { name: "Tools", exact: true }).click();
      await page.locator(".tool-card").filter({ has: page.getByText(name, { exact: true }) }).click();
      await page.getByTestId(`rf__node-${kind}-1`).locator(".flow-node-title").click();
      const field = page.locator(".main-content > .inspector").getByLabel(control, { exact: true });
      await expect(field).toBeVisible();
      await field.scrollIntoViewIfNeeded();
      const box = await field.boundingBox();
      expect(box!.x).toBeGreaterThanOrEqual(0);
      expect(box!.x + box!.width).toBeLessThanOrEqual(1280);
      const style = await field.evaluate(element => {
        const style = getComputedStyle(element);
        return { radius: style.borderRadius, shadow: style.boxShadow, background: style.backgroundImage };
      });
      expect(style).toEqual({ radius: "9px", shadow: "none", background: "none" });
      if (kind === "networkSend" || kind === "networkReceive") {
        await field.fill("192.168.1.20");
        await expect(page.getByTestId(`rf__node-${kind}-1`)).toContainText(`${kind === "networkSend" ? "To" : "From"} 192.168.1.20:47800`);
      }
      await page.screenshot({ path: path.join(directory, `${theme}-${kind}.png`) });
    }
  });
}

test("integer processor bounds reject invalid edits and arrow keys use the advertised step", async ({ page, backend }) => {
  await page.goto("/backend-harness.html");
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page.locator(".tool-card").filter({ has: page.getByText("Dehum", { exact: true }) }).click();
  await page.getByTestId("rf__node-dehum-1").locator(".flow-node-title").click();
  const field = page.locator(".main-content > .inspector").getByLabel("Harmonics precise value", { exact: true });
  await expect(field).toHaveAttribute("step", "1");
  await field.fill("4.1");
  await expect(page.locator(".global-action-message")).toContainText(/step|increment/i);
  await field.fill("4");
  await field.press("ArrowUp");
  await expect(field).toHaveValue("5");
  await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
  const session = await backend.call("sessions.get", { sessionId: "e2e-session" });
  expect(session.nodes.find((node: { id: string }) => node.id === "dehum-1").parameters.harmonics).toBe(5);
});
