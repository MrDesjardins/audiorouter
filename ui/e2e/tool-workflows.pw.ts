import { test, expect } from "./real-backend";
import { libraryEntries } from "../src/library";
import { parameterText } from "../src/parameterText";
import type { DiscoveryDocument, Session } from "../../contracts/src/index";
import { syntheticWav } from "./audio-fixtures";

for (const entry of libraryEntries.filter(entry => entry.kind)) {
  test(`${entry.label}: add, Properties, flags, save and durable reload through real backend`, async ({ page, backend }) => {
    await page.goto("/backend-harness.html");
    await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
    const tools = page.getByRole("tab", { name: "Tools", exact: true });
    await tools.click();
    await page.locator(".tool-card").filter({ has: page.getByText(entry.label, { exact: true }) }).click();
    await expect(tools).toHaveAttribute("aria-selected", "true");
    const node = page.getByTestId(`rf__node-${entry.kind}-1`);
    await expect(node).toBeVisible();
    await node.locator(".flow-node-title").click();
    await expect(page.getByRole("tab", { name: "Properties", exact: true })).toHaveAttribute("aria-selected", "true");
    const inspector = page.locator(".main-content > .inspector");
    const discovery = await backend.call<DiscoveryDocument>("system.describe");
    const descriptor = discovery.processors.find(item => item.id === entry.kind)
      ?? discovery.nodeTypes.find(item => item.type.split("@")[0].replace(/-/g, "").toLowerCase() === entry.kind!.toLowerCase());
    const changedParameters: Record<string, boolean | number | string> = {};
    // EQ has a dedicated response/point workflow; opaque paths and media IDs
    // are exercised through their dedicated import/binding workflows.
    if (entry.kind !== "parametricEq") {
      for (const parameter of descriptor?.parameters ?? []) {
        // Learning is driven by the Learn buttons, not a settings field.
        if (parameter.name.endsWith(":") || parameter.name === "learning") continue;
        // Node references and the Duck trigger/phase choices live in the Duck
        // editor (duck-widget and duck-siege-round tests), not generic fields.
        if ("reference" in parameter || (entry.kind === "duck" && ["trigger", "duckMenu", "duckPrep", "duckBetweenRounds"].includes(parameter.name))) continue;
        if (parameter.type === "number") {
          // Graphic EQ edits one selected band exactly; select it by its fader.
          if (entry.kind === "graphicEq") await inspector.getByRole("slider", { name: `${parameterText(entry.kind, parameter.name).label} band`, exact: true }).focus();
          const control = inspector.getByLabel(`${parameterText(entry.kind, parameter.name).label} precise value`, { exact: true });
          await expect(control, `${entry.kind}.${parameter.name} editable`).toBeVisible();
          const step = parameter.step ?? (parameter.unit === "Hz" ? 1 : 0.1);
          const current = Number(await control.inputValue());
          const minimum = parameter.minimum ?? -1000;
          const maximum = parameter.maximum ?? 1000;
          const value = Math.max(minimum, Math.min(maximum, Number((current + (current + step <= maximum ? step : -step)).toFixed(4))));
          await control.fill(String(value));
          await expect(control).toHaveValue(String(value));
          changedParameters[parameter.name] = value;
        } else if (parameter.type === "boolean") {
          const control = inspector.getByLabel(parameterText(entry.kind, parameter.name).label, { exact: true });
          await expect(control).toBeVisible();
          const value = !(await control.isChecked());
          await control.setChecked(value);
          changedParameters[parameter.name] = value;
        } else if (entry.kind === "inputSwitch" && parameter.type === "string") {
          // The Input Switch editor uses source buttons instead of selects.
          const value = parameter.name === "selected" ? "b" : "slow";
          await (parameter.name === "selected" ? inspector.getByRole("radio", { name: /^B/ }) : inspector.getByRole("button", { name: "Slow fade (2 s)" })).click();
          changedParameters[parameter.name] = value;
        } else if (parameter.type === "string" && parameter.enum && parameter.enum.length > 1) {
          const control = inspector.getByLabel(parameterText(entry.kind, parameter.name).label, { exact: true });
          await expect(control).toBeVisible();
          const current = await control.inputValue();
          const value = parameter.enum.find(value => value !== current)!;
          await control.selectOption(value);
          changedParameters[parameter.name] = value;
        }
      }
    }
    if (entry.kind === "audioFile") {
      await inspector.getByLabel("Choose WAV or MP3").setInputFiles({ name: "qualification.wav", mimeType: "audio/wav", buffer: syntheticWav() });
      await expect(inspector).toContainText("qualification.wav");
      await expect(inspector.getByRole("button", { name: "Play", exact: true })).toBeEnabled();
    }
    if (entry.kind === "networkSend" || entry.kind === "networkReceive") {
      const sending = entry.kind === "networkSend";
      const address = inspector.getByLabel(sending ? "Receiving computer's IP address" : "Sending computer's IP address", { exact: true });
      await address.fill("streaming-pc");
      await expect(inspector.getByRole("alert")).toContainText("numeric IP address");
      await address.fill("192.168.1.20");
      await inspector.getByLabel("Port", { exact: true }).fill("47810");
      changedParameters[sending ? "host" : "sender"] = "192.168.1.20";
      changedParameters.port = 47810;
      if (!sending) {
        await inspector.getByLabel("Buffer (ms)", { exact: true }).fill("60");
        changedParameters.bufferMs = 60;
      }
    }
    await inspector.getByLabel("Node name", { exact: true }).fill(`Qualified ${entry.kind}`);
    await inspector.getByLabel("Enabled", { exact: true }).uncheck();
    await inspector.getByLabel("Bypass", { exact: true }).check();
    await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
    const saved = await backend.call<Session>("sessions.get", { sessionId: "e2e-session" });
    const inserted = saved.nodes.find(candidate => candidate.id === `${entry.kind}-1`);
    expect(inserted).toMatchObject({ kind: entry.kind, name: `Qualified ${entry.kind}`, enabled: false, bypass: true });
    expect(inserted!.parameters).toMatchObject(changedParameters);
    expect(saved.edges).toHaveLength(2);
    await backend.restart();
    await page.reload();
    await expect(page.getByTestId(`rf__node-${entry.kind}-1`)).toContainText(`Qualified ${entry.kind}`);
    await page.getByTestId(`rf__node-${entry.kind}-1`).locator(".flow-node-title").click();
    await expect(inspector.getByLabel("Enabled", { exact: true })).not.toBeChecked();
    await expect(inspector.getByLabel("Bypass", { exact: true })).toBeChecked();
    expect(backend.methods).toContain("graph.plan");
    expect(backend.methods).toContain("graph.commit");
  });
}
