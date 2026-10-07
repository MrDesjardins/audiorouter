import { test, expect } from "./real-backend";
import { openConnectionForm } from "./workbench";
import { syntheticWav } from "./audio-fixtures";
import type { Session } from "../../contracts/src/index";

for (const [label, kind] of [
  ["Audio file", "audioFile"],
  ["FIR Filter", "firFilter"],
]) {
  test(`${label}: malformed media fails visibly, then valid synthetic media saves and reloads`, async ({
    page,
    backend,
  }) => {
    await page.goto("/backend-harness.html");
    await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await page
      .locator(".tool-card")
      .filter({ has: page.getByText(label, { exact: true }) })
      .click();
    await page.getByTestId(`rf__node-${kind}-1`).locator(".flow-node-title").click();
    const inspector = page.locator(".main-content > .inspector");
    const picker = inspector.getByLabel("Choose WAV or MP3");
    await picker.setInputFiles({ name: "broken.wav", mimeType: "audio/wav", buffer: Buffer.from("not audio") });
    await expect(inspector).toContainText(/failed|invalid|unsupported|decode/i);
    await picker.setInputFiles({ name: "synthetic.wav", mimeType: "audio/wav", buffer: syntheticWav() });
    await expect(inspector).toContainText("synthetic.wav");
    await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
    const session = await backend.call<Session>("sessions.get", { sessionId: "e2e-session" });
    expect(session.nodes.find((node) => node.id === `${kind}-1`)!.parameters).toMatchObject({
      fileName: "synthetic.wav",
      mediaId: expect.any(String),
    });
    await backend.restart();
    await page.reload();
    await page.getByTestId(`rf__node-${kind}-1`).locator(".flow-node-title").click();
    await expect(inspector).toContainText("synthetic.wav");
  });
}

test("occupied output inserts a real Mixer, undo restores edges, and saved routing survives restart", async ({
  page,
  backend,
}) => {
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page
    .locator(".tool-card")
    .filter({ has: page.getByText("Test Signal", { exact: true }) })
    .click();
  const connect = async () => {
    await openConnectionForm(page);
    const editor = page.locator(".workbench-connection-editor");
    await editor.getByLabel("Source output port").selectOption({ label: "Test Signal 1 · out · 2ch" });
    await editor.getByLabel("Destination input port").selectOption({ label: "Headphones · in · 2ch" });
    await editor.getByRole("button", { name: "Add connection", exact: true }).click();
  };
  await connect();
  await expect(page.locator(".global-action-message")).toContainText("Added a Mixer");
  await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(4);
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  await page.locator(".session-page").getByRole("button", { name: "Undo", exact: true }).click();
  await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(2);
  await connect();
  await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
  const session = await backend.call<Session>("sessions.get", { sessionId: "e2e-session" });
  const mixer = session.nodes.find((node) => node.kind === "mixer")!;
  expect(mixer).toBeTruthy();
  expect(session.edges.filter((edge) => edge.destinationNode === mixer.id)).toHaveLength(2);
  expect(session.edges.filter((edge) => edge.destinationNode === "headphones")).toHaveLength(1);
  const route = await backend.call("routes.inspect", { sessionId: session.id, destinationNode: "headphones" });
  expect(route.reachable).toBe(true);
  expect(route.paths).toHaveLength(2);
  await backend.restart();
  await page.reload();
  await expect(page.getByTestId(`rf__node-${mixer.id}`)).toBeVisible();
  await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(4);
});
