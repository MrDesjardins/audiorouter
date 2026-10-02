import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// Two computers: audio arrives from another address than the one entered
// (for example the sending PC's second network adapter). The receive node
// names the real address and fixes the sender in one click.
const port = (name: string, direction: string) => ({ name, direction, channels: 2 });
const session = {
  ...demoSession,
  nodes: [
    { id: "net-receive", kind: "networkReceive", typeVersion: 1, name: "From gaming PC", enabled: true, bypass: false, parameters: { sender: "192.168.1.20", port: 47800, bufferMs: 40 }, ports: [port("out", "output")] },
    { id: "speakers", kind: "physicalOutput", typeVersion: 1, name: "Speakers", enabled: true, bypass: false, parameters: {}, ports: [port("in", "input")] },
  ],
  edges: [{ id: "net-speakers", sourceNode: "net-receive", sourcePort: "out", destinationNode: "speakers", destinationPort: "in", matrix: [1, 0, 0, 1], enabled: true }],
};
const telemetry = [{ nodeId: "net-receive", meter: null, network: { direction: "receive", receivedPackets: 0, rejectedDatagrams: 120, rejectedFrom: "192.168.1.51", bufferedMs: 0 } }];

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`network receive names the real sender and fixes it in one click in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(({ session, telemetry, theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: true, __routeFixtureTelemetry: telemetry });
    }, { session, telemetry, theme });
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-net-receive").locator(".flow-node-title").click();
    const editor = page.getByLabel("Network receive settings");
    await expect(editor.getByRole("status")).toContainText("audio from 192.168.1.51 was ignored");
    const fix = editor.getByRole("alert");
    await expect(fix).toContainText("arriving from 192.168.1.51");
    await editor.screenshot({ path: testInfo.outputPath(`network-receive-hint-${theme}.png`) });
    await fix.getByRole("button", { name: "Use 192.168.1.51" }).click();
    await expect(editor.getByLabel("Sending computer's IP address")).toHaveValue("192.168.1.51");
  });
}
