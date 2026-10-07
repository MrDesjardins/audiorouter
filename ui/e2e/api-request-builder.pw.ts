import { test, expect } from "./real-backend";

// API tab request builder against the real backend catalog: choose a tool and
// a setting, check the generated request, and Send it once.
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`request builder generates and sends nodes.set in ${theme}`, async ({ page, backend }, testInfo) => {
    await page.goto("/backend-harness.html");
    await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
    await page.getByLabel("Color theme").selectOption(theme);
    await page.getByRole("tab", { name: "API", exact: true }).click();
    const builder = page.getByRole("region", { name: "Request builder" });
    await expect(builder).toBeVisible();

    await builder.getByLabel("Request tool").selectOption({ label: "Voice gain" });
    await builder.getByLabel("Request setting").selectOption({ label: "gainDb (dB)" });
    const value = builder.getByLabel("Request value");
    await value.fill("30");
    await expect(builder.getByRole("alert")).toHaveText("Enter a value from -60 to 24 dB.");
    await expect(builder.getByLabel("Generated request")).toHaveCount(0);
    await value.fill("3");
    const generated = builder.getByLabel("Generated request");
    await expect(generated).toHaveValue(/"node": "voice"/);
    await expect(generated).toHaveValue(/"gainDb": 3/);
    await expect(generated).not.toHaveValue(/sessionId/);
    await expect(builder.getByLabel("Request URL")).toHaveValue("POST http://127.0.0.1:17891/api/v1/nodes/set");

    await builder.getByText("Always this session").click();
    await expect(generated).toHaveValue(/"sessionId": "e2e-session"/);
    await builder.getByRole("button", { name: "curl" }).click();
    await expect(generated).toHaveValue(/Authorization: Bearer <your API token>/);
    // Choosing and generating never changes anything.
    expect(backend.methods.filter((method) => method === "nodes.set")).toHaveLength(0);

    await builder.scrollIntoViewIfNeeded();
    await builder.screenshot({ path: testInfo.outputPath(`request-builder-${theme}.png`) });

    await builder.getByRole("button", { name: "Send now" }).click();
    await expect(builder.getByRole("status")).toContainText("Sent. Saved as revision");
    expect(backend.methods.filter((method) => method === "nodes.set")).toHaveLength(1);
    const saved = await backend.call("sessions.get", { sessionId: "e2e-session" });
    expect(saved.nodes.find((node: { id: string }) => node.id === "voice").parameters.gainDb).toBe(3);
  });
}

// The catalog now describes Duck's trigger settings for API clients; the
// inspector keeps them in the Duck editor and shows no raw duplicate fields.
test("Duck inspector shows no raw trigger fields from the catalog", async ({ page }) => {
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page
    .locator(".tool-card")
    .filter({ has: page.getByText("Duck", { exact: true }) })
    .click();
  await page.getByTestId("rf__node-duck-1").locator(".flow-node-title").click();
  const inspector = page.locator(".main-content > .inspector");
  await expect(inspector.getByRole("slider", { name: "Duck amount" })).toBeVisible();
  // Without the filter the generic editor adds a second Trigger select and
  // "Duck menu/prep/between rounds" checkboxes under the Duck editor.
  for (const raw of ["Duck menu", "Duck prep", "Duck between rounds"])
    await expect(inspector.getByText(raw, { exact: true })).toHaveCount(0);
  await expect(inspector.locator("label", { hasText: /^Trigger\s*Level\s*Siege round$/ })).toHaveCount(0);
  await expect(inspector.getByRole("textbox", { name: /keyNodeId/i })).toHaveCount(0);
});
