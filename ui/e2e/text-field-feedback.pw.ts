import type { Locator, Page } from "@playwright/test";
import { AUDIT_THEMES, eachAuditedPage, openAuditHarness } from "./audit-pages";
import { expect, test } from "./browser-test";
import { backToRoute, fitCanvas } from "./workbench";

// Inputs criterion, Round 28 gap "text-field validation beyond numbers not inventoried".
// 1. Inventory: every visible text field (not a number) on the panels, the Settings pages and the
//    Properties of every tool must be classified below, so a new text field cannot arrive without
//    someone deciding whether it validates.
// 2. Feedback: a field that can refuse its input must say so the same way the number field does
//    (aria-invalid, a visible reason reachable through aria-describedby) and the reason appearing
//    must not move anything below it (UI-17).

/** Fields that check what is typed. Each has a feedback test below, or a reason it has none. */
const VALIDATED = [
  /^Receiving computer's IP address$/,
  /^Sending computer's IP address$/,
  /^Pairing key$/,
  /^Application executable path$/,
  // Plug-in path and directory are checked by the backend (Inspect path, Scan directory); the route
  // harness has no backend for plug-ins, so their feedback is recorded in the Round entry, not tested.
  /^Absolute plugin (path|directory)$/,
  /^(Session|Tool) name$/,
];
/** Fields where any text is acceptable (a search, an identifier the backend looks up, read-only output). */
const FREE_TEXT = [
  /^Find a tool$/,
  /^Find a setting/,
  /^Request URL$/,
  /^Generated request$/,
  /^Assistant client ID$/,
  /^New client ID$/,
];

const textFields = (page: Page) =>
  page.evaluate(() =>
    [...document.querySelectorAll<HTMLInputElement>("input, textarea")]
      .filter(
        (field) =>
          field.getBoundingClientRect().width > 0 &&
          !field.closest("[aria-hidden=true]") &&
          !["number", "checkbox", "radio", "range", "hidden", "file", "color", "button"].includes(field.type) &&
          field.getAttribute("role") !== "spinbutton",
      )
      .map((field) => {
        const labelled = (field.getAttribute("aria-labelledby") ?? "")
          .split(/\s+/)
          .map((id) => document.getElementById(id)?.textContent?.trim() ?? "")
          .join("");
        return (
          field.getAttribute("aria-label")?.trim() ||
          labelled ||
          (field.labels?.[0]?.textContent ?? "").trim() ||
          field.placeholder ||
          `(unnamed ${field.className})`
        );
      }),
  );

const unclassified = (names: string[]) =>
  names.filter((name) => ![...VALIDATED, ...FREE_TEXT].some((known) => known.test(name)));

test("every text field on the panels, Settings and tool Properties is classified", async ({ page }) => {
  test.setTimeout(150_000);
  await openAuditHarness(page, "dark");
  const seen = new Set<string>();
  await eachAuditedPage(page, async () => void (await textFields(page)).forEach((name) => seen.add(name)));
  await backToRoute(page);
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  const tools = page.locator(".tool-card");
  const toolCount = await tools.count();
  expect(toolCount).toBeGreaterThan(20);
  for (let index = 0; index < toolCount; index += 1) {
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await tools.nth(index).focus();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Escape");
  }
  await fitCanvas(page);
  for (const card of await page.locator(".react-flow__node .flow-node-title").all()) {
    await card.dispatchEvent("click");
    (await textFields(page)).forEach((name) => seen.add(name));
  }
  expect(seen.size, "text fields found").toBeGreaterThanOrEqual(10);
  expect(unclassified([...seen])).toEqual([]);
});

test("the inventory reports a text field nobody classified", async ({ page }) => {
  await openAuditHarness(page, "dark");
  await page.evaluate(() => {
    const field = document.createElement("input");
    field.setAttribute("aria-label", "Mystery value");
    field.style.cssText = "position:fixed;top:0;left:0;z-index:9999";
    document.body.append(field);
  });
  expect(unclassified(await textFields(page))).toEqual(["Mystery value"]);
});

/**
 * Makes the field invalid with `makeInvalid`, then returns what is wrong with the feedback: the field
 * must be aria-invalid, name a visible reason through aria-describedby, and `below` must not move.
 */
async function feedbackProblems(
  page: Page,
  label: string,
  field: Locator,
  below: Locator,
  makeInvalid: () => Promise<void>,
) {
  await expect(below).toBeVisible();
  await expect(field).toBeVisible();
  const before = await below.boundingBox();
  await makeInvalid();
  const after = await below.boundingBox();
  const problems: string[] = [];
  if (!before || !after) return [`${label}: the content below the field is not visible`];
  if (before.y !== after.y || before.height !== after.height)
    problems.push(`${label}: content below moved ${Math.round(after.y - before.y)}px`);
  if ((await field.getAttribute("aria-invalid")) !== "true") problems.push(`${label}: field is not aria-invalid`);
  const reason = await field.evaluate((element) => {
    const parts = (element.getAttribute("aria-describedby") ?? "")
      .split(/\s+/)
      .map((id) => document.getElementById(id))
      .filter((node): node is HTMLElement => !!node);
    const visible = parts.filter((node) => {
      const box = node.getBoundingClientRect();
      return box.width > 0 && box.height > 0 && getComputedStyle(node).visibility !== "hidden";
    });
    return visible
      .map((node) => node.innerText.trim())
      .join(" ")
      .trim();
  });
  if (!reason) problems.push(`${label}: no visible reason reachable through aria-describedby`);
  return problems;
}

const openNetworkNode = async (page: Page, tool: string) => {
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page.locator(".tool-card").filter({ hasText: tool }).click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
};

for (const theme of AUDIT_THEMES) {
  test(`a wrong network address or pairing key explains itself without moving anything in ${theme}`, async ({
    page,
  }, testInfo) => {
    await openAuditHarness(page, theme);
    const problems: string[] = [];
    for (const [tool, address] of [
      ["Network Send", "Receiving computer's IP address"],
      ["Network Receive", "Sending computer's IP address"],
    ]) {
      await openNetworkNode(page, tool);
      const field = page.getByLabel(address);
      const key = page.getByLabel("Pairing key");
      problems.push(
        ...(await feedbackProblems(page, `${tool} address`, field, key, () => field.fill("not an address"))),
      );
      await page.screenshot({ path: testInfo.outputPath(`${tool}-address-${theme}.png`) });
      await field.fill("192.168.1.20");
      await expect(field).toHaveAttribute("aria-invalid", "false");
      problems.push(
        ...(await feedbackProblems(
          page,
          `${tool} key`,
          key,
          page.getByRole("button", { name: "Generate" }).first(),
          () => key.fill("abc"),
        )),
      );
    }
    expect([...new Set(problems)]).toEqual([]);
  });

  test(`a bad application path is refused inside the picker, next to the field, in ${theme}`, async ({
    page,
  }, testInfo) => {
    await openAuditHarness(page, theme);
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await page.locator(".tool-card").filter({ hasText: "Application source" }).click();
    const dialog = page.getByRole("dialog").filter({ has: page.getByRole("button", { name: "App group" }) });
    const field = dialog.getByLabel("Application executable path");
    await field.fill("not a path");
    const problems = await feedbackProblems(
      page,
      "Application path",
      field,
      dialog.getByRole("button", { name: "Refresh applications" }),
      () => dialog.getByRole("button", { name: "Add this app source" }).click(),
    );
    await expect(dialog.locator("#application-picker-status")).toContainText("full Windows path");
    await dialog.screenshot({ path: testInfo.outputPath(`application-path-${theme}.png`) });
    expect(problems).toEqual([]);
    await field.fill("C:\\Games\\game.exe");
    await expect(field).toHaveAttribute("aria-invalid", "false");
  });

  test(`an empty name is refused inside the prompt, next to the field, in ${theme}`, async ({ page }, testInfo) => {
    await openAuditHarness(page, theme);
    await page.getByRole("tab", { name: "Session", exact: true }).click();
    await page.getByRole("button", { name: "New", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "New session" });
    const field = dialog.getByLabel("Session name");
    await field.fill("");
    const problems = await feedbackProblems(
      page,
      "Session name",
      field,
      dialog.getByRole("button", { name: "Create session" }),
      () => dialog.getByRole("button", { name: "Create session" }).click(),
    );
    await dialog.screenshot({ path: testInfo.outputPath(`session-name-${theme}.png`) });
    expect(problems).toEqual([]);
  });
}

test.describe("negative controls", () => {
  test("the feedback audit reports a field that only turns red and a message that pushes content down", async ({
    page,
  }) => {
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.evaluate(() => {
      const box = document.createElement("div");
      box.style.cssText = "position:fixed;top:0;left:0;z-index:9999;background:#222;padding:8px;color:#fff";
      box.innerHTML = `<label>Probe <input id="probe-field" aria-label="Probe field"></label><button id="probe-below">Below</button>`;
      document.body.append(box);
    });
    const field = page.locator("#probe-field");
    const problems = await feedbackProblems(page, "probe", field, page.locator("#probe-below"), () =>
      page.evaluate(() => {
        const input = document.getElementById("probe-field")!;
        input.setAttribute("aria-invalid", "true");
        // A message appended in the flow, not tied to the field.
        input.parentElement!.after(
          Object.assign(document.createElement("p"), { textContent: "Wrong value", style: "margin:20px" }),
        );
      }),
    );
    expect(problems.some((entry) => entry.includes("moved"))).toBe(true);
    expect(problems.some((entry) => entry.includes("no visible reason"))).toBe(true);
    expect(problems.some((entry) => entry.includes("not aria-invalid"))).toBe(false);
  });
});
