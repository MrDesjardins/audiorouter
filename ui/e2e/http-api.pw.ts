import { test, expect } from "@playwright/test";

test("HTTP mutation reaches the authoritative backend and refreshes the frontend; Swagger works offline", async ({
  page,
  request,
  context,
}, testInfo) => {
  test.skip(!process.env.AUDIOROUTER_HTTP_FIXTURE_TOKEN, "Requires the disposable Rust HTTP/pipe fixture");
  const url = "http://127.0.0.1:17893";
  const headers = { Authorization: `Bearer ${process.env.AUDIOROUTER_HTTP_FIXTURE_TOKEN}` };
  const call = async (method: string, data: unknown) => {
    const response = await request.post(`${url}/api/v1/${method.replaceAll(".", "/")}`, { headers, data });
    expect(response.status()).toBe(200);
    return response.json();
  };
  // The browser harness adapts its RPC requests to the real HTTP listener;
  // that listener forwards through a real isolated Windows pipe. No audio opens.
  await page.route("**/__e2e_rpc", async (route) => {
    const rpc = route.request().postDataJSON();
    const response = await request.post(`${url}/api/v1/${rpc.method.replaceAll(".", "/")}`, {
      headers,
      data: rpc.params ?? {},
    });
    const body = await response.json();
    await route.fulfill({
      json: { jsonrpc: "2.0", id: rpc.id, ...(response.ok() ? { result: body } : { error: body.error }) },
    });
  });
  await page.goto("/backend-harness.html");
  await expect(page.locator("h1")).toHaveText("HTTP qualification");
  const session = await call("sessions.get", { sessionId: "e2e-session" });
  const gain = session.nodes.find((node: { kind: string }) => node.kind === "gain");
  expect(gain).toBeTruthy();
  await page.getByTestId(`rf__node-${gain.id}`).locator(".flow-node-title").click();
  gain.parameters.gainDb = gain.parameters.gainDb === -9 ? -12 : -9;
  const plan = await call("graph.plan", { sessionId: session.id, baseRevision: session.revision, candidate: session });
  await call("graph.commit", {
    planId: plan.planId,
    baseRevision: session.revision,
    idempotencyKey: `http-browser-edit-${session.revision}`,
  });
  await expect(page.getByLabel("Gain precise value", { exact: true })).toHaveValue(String(gain.parameters.gainDb), {
    timeout: 10000,
  });
  await page.getByLabel("Node name", { exact: true }).fill("Unsaved local label");
  const latest = await call("sessions.get", { sessionId: session.id });
  latest.nodes.find((node: { kind: string }) => node.kind === "gain").parameters.gainDb = -6;
  const next = await call("graph.plan", { sessionId: session.id, baseRevision: latest.revision, candidate: latest });
  await call("graph.commit", {
    planId: next.planId,
    baseRevision: latest.revision,
    idempotencyKey: `http-dirty-edit-${latest.revision}`,
  });
  await expect(page.locator(".global-action-message")).toContainText("This session changed elsewhere", {
    timeout: 10000,
  });
  await expect(page.getByLabel("Node name", { exact: true })).toHaveValue("Unsaved local label");
  await page.screenshot({ path: testInfo.outputPath("2026-09-29-http-refresh.png") });
  const docs = await context.newPage();
  const remote: string[] = [];
  docs.on("request", (request) => {
    if (!request.url().startsWith(url)) remote.push(request.url());
  });
  await docs.goto(`${url}/docs`);
  await expect(docs.locator(".swagger-ui .info .title")).toContainText("AudioRouter local API");
  await expect(docs.locator(".opblock")).toHaveCount(103);
  expect(remote).toEqual([]);
  await docs.screenshot({ path: testInfo.outputPath("2026-09-29-swagger.png") });
  await docs.getByRole("button", { name: "Authorize", exact: true }).click();
  await docs.locator(".dialog-ux .auth-container input").fill(process.env.AUDIOROUTER_HTTP_FIXTURE_TOKEN!);
  await docs.locator(".dialog-ux button.authorize").click();
  await docs.locator(".dialog-ux .btn-done").click();
  const status = docs.locator(".opblock-get").filter({ hasText: "/api/v1/status" });
  await status.locator(".opblock-summary-control").click();
  await status.getByRole("button", { name: "Try it out", exact: true }).click();
  const result = docs.waitForResponse((response) => response.url() === `${url}/api/v1/status`);
  await status.getByRole("button", { name: "Execute", exact: true }).click();
  expect((await result).status()).toBe(200);
  expect(remote).toEqual([]);
});
