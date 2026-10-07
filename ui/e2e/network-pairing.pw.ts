import { test, expect, type Locator } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// P2-5: a Network Receive/Send pairing key. The field uses the app style,
// Generate fills it, and the "Not paired" warning, the paired note and the
// validation message share one reserved slot (UI-17): typing, generating or
// clearing a key never moves the controls below it.
const port = (name: string, direction: string) => ({ name, direction, channels: 2 });
const receiving = {
  ...demoSession,
  nodes: [
    { id: "net-receive", kind: "networkReceive", typeVersion: 1, name: "From gaming PC", enabled: true, bypass: false, parameters: { sender: "192.168.1.20", port: 47800, bufferMs: 40 }, ports: [port("out", "output")] },
    { id: "speakers", kind: "physicalOutput", typeVersion: 1, name: "Speakers", enabled: true, bypass: false, parameters: {}, ports: [port("in", "input")] },
  ],
  edges: [{ id: "net-speakers", sourceNode: "net-receive", sourcePort: "out", destinationNode: "speakers", destinationPort: "in", matrix: [1, 0, 0, 1], enabled: true }],
};
const sending = {
  ...demoSession,
  nodes: [
    { id: "mic", kind: "physicalInput", typeVersion: 1, name: "Microphone", enabled: true, bypass: false, parameters: {}, ports: [port("out", "output")] },
    { id: "net-send", kind: "networkSend", typeVersion: 1, name: "To streaming PC", enabled: true, bypass: false, parameters: { host: "192.168.1.30", port: 47800 }, ports: [port("in", "input")] },
  ],
  edges: [{ id: "mic-send", sourceNode: "mic", sourcePort: "out", destinationNode: "net-send", destinationPort: "in", matrix: [1, 0, 0, 1], enabled: true }],
};
// Live counters: this receiver has no key, but the sending computer has one.
const telemetry = [{ nodeId: "net-receive", meter: null, network: { direction: "receive", receivedPackets: 0, authFailures: 42, authProblem: "receiverNotPaired", bufferedMs: 0, thisAddress: "192.168.1.30", paired: false } }];

const top = async (locator: Locator) => (await locator.boundingBox())?.y ?? Number.NaN;

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`network pairing key field keeps its warning slot in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(({ session, telemetry, theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: true, __routeFixtureTelemetry: telemetry });
    }, { session: receiving, telemetry, theme });
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-net-receive").locator(".flow-node-title").click();
    const editor = page.getByLabel("Network receive settings");
    const note = editor.getByTestId("network-pairing-note");
    const key = editor.getByLabel("Pairing key");
    const below = editor.getByLabel("Buffer (ms)");
    // The canvas and inspector status stays short; the advice uses the
    // pairing note's reserved slot.
    await expect(editor.getByRole("status")).toHaveText("Waiting for audio · 42 packets had a pairing key");
    await expect(page.getByTestId("rf__node-net-receive")).toContainText("42 packets had a pairing key");
    await expect(note).toHaveText("The sending computer uses a pairing key. Enter the same key here.");
    // App field style: flat, 9 px radius, full width under its caption.
    const style = await key.evaluate((element) => {
      const computed = getComputedStyle(element);
      return { radius: computed.borderTopLeftRadius, shadow: computed.boxShadow, image: computed.backgroundImage };
    });
    expect(style).toEqual({ radius: "9px", shadow: "none", image: "none" });
    const [field, group] = await Promise.all([key.boundingBox(), editor.locator(".network-pairing").boundingBox()]);
    expect(Math.abs((field?.width ?? 0) - (group?.width ?? 0))).toBeLessThan(2);
    await editor.screenshot({ path: testInfo.outputPath(`network-pairing-unpaired-${theme}.png`) });

    const offsets = [await top(below)];
    await editor.getByRole("button", { name: "Generate" }).click();
    await expect(key).toHaveValue(/^[A-HJ-NP-Z2-9]{24}$/);
    await expect(note).toHaveText("Paired: only audio sent with this key is played.");
    offsets.push(await top(below));
    await editor.screenshot({ path: testInfo.outputPath(`network-pairing-paired-${theme}.png`) });
    await key.fill("short");
    await expect(note).toContainText("16 to 128");
    offsets.push(await top(below));
    await key.fill("");
    await expect(note).toContainText("uses a pairing key");
    offsets.push(await top(below));
    expect(new Set(offsets).size, `controls below the note stay put: ${offsets.join(", ")}`).toBe(1);

  });

  test(`network send pairing key warns that its stream can be heard in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(({ session, theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: false });
    }, { session: sending, theme });
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-net-send").locator(".flow-node-title").click();
    const send = page.getByLabel("Network send settings");
    await expect(send.getByTestId("network-pairing-note")).toHaveText("Not paired: anyone on your network can listen to this stream or send audio in its place.");
    await send.getByRole("button", { name: "Generate" }).click();
    await expect(send.getByTestId("network-pairing-note")).toContainText("The audio is not encrypted");
    await send.screenshot({ path: testInfo.outputPath(`network-pairing-send-${theme}.png`) });
  });
}
