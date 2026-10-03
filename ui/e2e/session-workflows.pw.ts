import { test, expect, initialSession } from "./real-backend";
import type { Session } from "../../contracts/src/index";
import { readFile } from "node:fs/promises";

const save = async (page: import("@playwright/test").Page) => {
  await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
};

test("initial backend loading cannot expose an editable demo graph", async ({ page }) => {
  let release!: () => void;
  const ready = new Promise<void>(resolve => { release = resolve; });
  await page.route("**/__e2e_rpc", async route => {
    if (route.request().postDataJSON().method === "status.get") await ready;
    await route.fallback();
  });
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("main", { name: "Loading saved session" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Tools", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Play", exact: true })).toHaveCount(0);
  release();
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  await expect(page.getByTestId("rf__node-mic")).toBeVisible();
});

test("session duplicate, rename, remembered selection and deletion survive backend restart", async ({ page, backend }) => {
  await page.goto("/backend-harness.html");
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  await page.getByRole("button", { name: "Duplicate", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Offline qualification (copy)", exact: true })).toBeVisible();
  const copiedId = await page.getByLabel("Choose session").inputValue();
  await page.getByRole("button", { name: "Rename", exact: true }).click();
  await page.locator(".right-workbench").getByLabel("Session name", { exact: true }).fill("Durable studio setup");
  await save(page);
  const copy = await backend.call<Session>("sessions.get", { sessionId: copiedId });
  expect(copy.name).toBe("Durable studio setup");
  expect(copy.nodes).toEqual(initialSession.nodes);
  expect(copy.edges).toEqual(initialSession.edges);
  await backend.restart();
  await page.reload();
  await expect(page.getByRole("heading", { name: "Durable studio setup", exact: true })).toBeVisible();
  await expect(page.locator(".audio-run-state")).toContainText("Audio stopped");
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  page.once("dialog", dialog => dialog.accept());
  await page.getByRole("button", { name: "Delete session", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  await backend.restart();
  await page.reload();
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  const deleted = await backend.send({ method: "sessions.get", params: { sessionId: copiedId } });
  expect(deleted.error).toBeTruthy();
});

test("new session and cancelled dialogs never mutate the existing setup", async ({ page, backend }) => {
  await page.goto("/backend-harness.html");
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  page.once("dialog", dialog => dialog.dismiss());
  await page.getByRole("button", { name: "New", exact: true }).click();
  expect((await backend.call("sessions.list")).items).toHaveLength(1);
  page.once("dialog", dialog => dialog.accept("Practice setup"));
  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Practice setup", exact: true })).toBeVisible();
  page.once("dialog", dialog => dialog.dismiss());
  await page.getByRole("button", { name: "Delete session", exact: true }).click();
  expect((await backend.call("sessions.list")).items).toHaveLength(2);
  expect(await backend.call("sessions.get", { sessionId: "e2e-session" })).toMatchObject(initialSession);
});

test("draft undo, redo and revert restore exact nodes without saving accidental edits", async ({ page, backend }) => {
  await page.goto("/backend-harness.html");
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page.locator(".tool-card").filter({ has: page.getByText("Gain", { exact: true }) }).click();
  await expect(page.getByTestId("rf__node-gain-1")).toBeVisible();
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  await page.locator(".session-page").getByRole("button", { name: "Undo", exact: true }).click();
  await expect(page.getByTestId("rf__node-gain-1")).toHaveCount(0);
  await page.locator(".session-page").getByRole("button", { name: "Redo", exact: true }).click();
  await expect(page.getByTestId("rf__node-gain-1")).toBeVisible();
  await page.getByRole("button", { name: "Revert edits", exact: true }).click();
  await expect(page.getByTestId("rf__node-gain-1")).toHaveCount(0);
  expect(await backend.call("sessions.get", { sessionId: "e2e-session" })).toMatchObject(initialSession);
  expect(backend.methods.filter(method => method === "graph.commit")).toHaveLength(0);
});

test("real graph transactions reject stale commits, replay idempotently and preserve undo history", async ({ backend }) => {
  const candidate = structuredClone(initialSession);
  candidate.nodes[1].parameters.gainDb = -6;
  const plan = await backend.call("graph.plan", { sessionId: candidate.id, baseRevision: 0, candidate });
  const competing = await backend.call("graph.plan", { sessionId: candidate.id, baseRevision: 0, candidate: { ...candidate, name: "Competing setup" } });
  const params = { planId: plan.planId, baseRevision: 0, idempotencyKey: "commit-gain", acknowledgments: [] };
  const result = await backend.call("graph.commit", params);
  expect(await backend.call("graph.commit", params)).toMatchObject({ ...result, idempotentReplay: true });
  const stale = await backend.send({ method: "graph.commit", params: { ...params, planId: competing.planId, idempotencyKey: "stale" } });
  expect(stale.error).toBeTruthy();
  expect(await backend.call("sessions.get", { sessionId: candidate.id })).toMatchObject({ revision: 1, name: initialSession.name });
  await backend.restart();
  const history = await backend.call("graph.history", { sessionId: candidate.id });
  expect(history.items.map((item: Session) => item.revision)).toContain(0);
  const undo = await backend.call("graph.undoPlan", { sessionId: candidate.id, baseRevision: 1 });
  await backend.call("graph.commit", { planId: undo.planId, baseRevision: 1, idempotencyKey: "undo-gain", acknowledgments: [] });
  const restored = await backend.call<Session>("sessions.get", { sessionId: candidate.id });
  expect(restored.revision).toBe(2);
  expect(restored.nodes).toEqual(initialSession.nodes);
});

test("Advanced EQ changes use real DSP response and persist exact filter parameters", async ({ page, backend }) => {
  await page.goto("/backend-harness.html");
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page.locator(".tool-card").filter({ has: page.getByText("Advanced EQ", { exact: true }) }).click();
  await page.getByTestId("rf__node-parametricEq-1").locator(".flow-node-title").click();
  const editor = page.getByRole("region", { name: "Advanced EQ editor" });
  await editor.getByRole("button", { name: "Add point", exact: true }).click();
  await editor.getByLabel("EQ filter type").selectOption("peaking");
  await editor.getByLabel("EQ frequency Hz").fill("1000");
  await editor.getByLabel("EQ gain dB").fill("6");
  await editor.getByLabel("EQ Q width").fill("2");
  await expect(editor.locator(".advanced-eq-curve")).toBeVisible();
  await save(page);
  const saved = await backend.call<Session>("sessions.get", { sessionId: "e2e-session" });
  expect(saved.nodes.find(node => node.id === "parametricEq-1")!.parameters).toMatchObject({ band0Enabled: true, band0Type: "peaking", band0FrequencyHz: 1000, band0GainDb: 6, band0Q: 2 });
  expect(backend.methods).toContain("processors.response");
});

test("session export and validated import roundtrip through the UI without starting audio", async ({ page, backend }) => {
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Advanced", exact: true }).click();
  await page.locator(".right-workbench summary").filter({ hasText: "JSON graph transfer (for scripts)" }).click();
  const panel = page.locator(".right-workbench .session-transfer-panel");
  const downloadReady = page.waitForEvent("download");
  await panel.getByRole("button", { name: "Export session", exact: true }).click();
  const download = await downloadReady;
  expect(download.suggestedFilename()).toBe("Offline-qualification.audiorouter.json");
  const exported = JSON.parse(await readFile((await download.path())!, "utf8"));
  expect(exported).toMatchObject(initialSession);
  const picker = panel.getByLabel("Import session configuration");
  await picker.setInputFiles({ name: "bad.json", mimeType: "application/json", buffer: Buffer.from("{") });
  await expect(panel).toContainText(/validate|JSON|syntax/i);
  await expect(panel.getByRole("button", { name: "Commit stopped import", exact: true })).toHaveCount(0);
  await picker.setInputFiles({ name: "studio.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify({ ...exported, id: "portable-studio", name: "Portable studio" })) });
  await expect(panel).toContainText("Validated import: Portable studio");
  expect((await backend.call("sessions.list")).items).toHaveLength(1);
  await panel.getByRole("button", { name: "Commit stopped import", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Portable studio", exact: true })).toBeVisible();
  await expect(page.locator(".audio-run-state")).toContainText("Audio stopped");
  expect((await backend.call("status.get")).activeSessionCount).toBe(0);
  await backend.restart();
  await page.reload();
  await expect(page.getByRole("heading", { name: "Portable studio", exact: true })).toBeVisible();
  expect(backend.methods.filter(method => method === "session.start")).toHaveLength(0);
});
