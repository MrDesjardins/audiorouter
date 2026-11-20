import { mkdir, writeFile } from "node:fs/promises";
import type { Locator, Page } from "@playwright/test";
import type { Session } from "@audiorouter/contracts";
import { expect, test } from "./browser-test";
import {
  appendApplicationCapturePathNode,
  appendEndpointLoopbackNode,
  appendLibraryNode,
  appendVirtualBusNode,
} from "../src/draft";
import { demoSession } from "../src/fixtures";
import { libraryEntries } from "../src/library";
import { SETTINGS_PAGES } from "../src/Workbench";
import { openSettings } from "./workbench";

const output = "../target/chinese-ui-audit";
test.skip(!process.env.AUDIOROUTER_CHINESE_AUDIT, "Opt-in screenshot inventory");

async function capture(page: Page, panel: Locator, id: string) {
  await expect(panel).toBeVisible();
  await panel.locator("details").evaluateAll((elements) => {
    for (const element of elements) (element as HTMLDetailsElement).open = true;
  });
  const text = await panel.innerText();
  const labels = await panel.locator("[aria-label], [aria-description], [title], [placeholder]").evaluateAll((items) =>
    items.flatMap((item) =>
      ["aria-label", "aria-description", "title", "placeholder"].flatMap((attribute) => {
        const value = item.getAttribute(attribute);
        return value ? [value] : [];
      }),
    ),
  );
  const latin = [...new Set([...text.split("\n"), ...labels].map((line) => line.trim()))].filter((line) =>
    /[a-zA-Z]{2}/.test(line),
  );
  const dimensions = await panel.evaluate((element) => ({
    height: element.clientHeight,
    total: element.scrollHeight,
  }));
  const screenshots: string[] = [];
  const step = Math.max(1, dimensions.height - 64);
  for (let offset = 0; ; offset += step) {
    await panel.evaluate((element, position) => {
      element.scrollTop = position;
    }, offset);
    const file = `${id}-${screenshots.length + 1}.png`;
    await panel.screenshot({ path: `${output}/${file}` });
    screenshots.push(file);
    if (offset + dimensions.height >= dimensions.total) break;
  }
  await panel.evaluate((element) => {
    element.scrollTop = 0;
  });
  return { id, language: await page.locator("html").getAttribute("lang"), screenshots, latin };
}

test("collect Simplified Chinese tool and panel screenshots and displayed text", async ({ page }) => {
  test.setTimeout(300_000);
  page.setDefaultTimeout(8_000);
  await mkdir(output, { recursive: true });
  let session = libraryEntries.reduce<Session>(
    (current, entry) => (entry.kind ? appendLibraryNode(current, entry.kind) : current),
    { ...demoSession, id: "chinese-audit", name: "中文检查", nodes: [], edges: [] },
  );
  session = appendApplicationCapturePathNode(session, "C:\\Audit\\example.exe");
  session = appendEndpointLoopbackNode(session, "audit-render");
  session = appendVirtualBusNode(session, "cable-a", "renderSource");
  session = appendVirtualBusNode(session, "cable-b", "captureSink");
  for (const kind of ["plugin", "instrument"] as const) {
    session.nodes.push({
      id: `audit-${kind}`,
      kind,
      typeVersion: 1,
      name: "",
      enabled: false,
      bypass: false,
      parameters: { path: "C:\\Audit\\example.vst3", format: "vst3", classId: "default" },
      ports: [{ name: "out", direction: "output", channels: 2 }],
    });
  }
  session.nodes.forEach((node, index) => {
    node.name = `测试工具 ${index + 1}`;
  });
  await page.addInitScript((fixture) => {
    Object.assign(window, { __routeFixtureSession: fixture });
  }, session);
  await page.goto("/route-harness.html");
  await openSettings(page, "Appearance");
  await page.getByRole("combobox", { name: "Language", exact: true }).selectOption("zh-Hans");
  await page.getByRole("button", { name: "返回路由", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-Hans");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await expect(page.getByText("音频已停止", { exact: true })).toBeVisible();
  const rows: Awaited<ReturnType<typeof capture>>[] = [];
  async function collect(panel: Locator, id: string) {
    rows.push(await capture(page, panel, id));
    await writeFile(`${output}/inventory.json`, JSON.stringify(rows, null, 2));
  }
  await page.screenshot({ path: `${output}/workspace.png` });
  const routeWarning = page.getByRole("button", { name: "关闭路由警告", exact: true });
  if (await routeWarning.isVisible()) await routeWarning.click();
  await page.getByRole("button", { name: "路由列表", exact: true }).click();
  const list = page.locator(".canvas-keyboard-panel .graph-list");
  for (const node of session.nodes) {
    await list.getByRole("button", { name: new RegExp(`^${node.name} `) }).click();
    const panel = page.locator(".main-content > .inspector");
    await expect(panel.locator("#inspector-heading")).toHaveAttribute("aria-label", node.name);
    if (node.kind === "physicalInput") {
      await expect(panel.getByRole("combobox", { name: "输入设备端点", exact: true })).toBeVisible();
    }
    if (node.kind === "physicalOutput") {
      await expect(panel.getByRole("combobox", { name: "输出设备端点", exact: true })).toBeVisible();
    }
    await collect(panel, `tool-${node.kind}`);
  }
  await page.getByRole("button", { name: "路由列表", exact: true }).click();
  for (const id of ["tools", "session", "recording", "status"]) {
    await page.locator(`#workbench-tab-${id}`).click();
    await collect(page.locator(`#workbench-panel-${id}`), `page-${id}`);
  }
  for (const id of ["apps", "timing"]) {
    await page.locator(`#workbench-tab-status-view-${id}`).click();
    await collect(page.locator(`#workbench-panel-${id}`), `page-${id}`);
  }
  await page.getByRole("button", { name: "设置", exact: true }).click();
  for (const [id] of SETTINGS_PAGES) {
    await page.locator(`#workbench-tab-${id}`).click();
    await collect(page.locator(`#workbench-panel-${id}`), `settings-${id}`);
  }
});
