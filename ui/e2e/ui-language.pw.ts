import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

for (const [tag, pagesLabel, help, apiIntro, hotkeys, hotkeysIntro] of [
  [
    "fr",
    "Pages de paramètres",
    "Aide",
    "Envoyez des requêtes REST depuis ce PC",
    "Raccourcis clavier",
    "Ces raccourcis fonctionnent dans tout Windows",
  ],
  ["zh-Hans", "设置页面", "帮助", "可从此电脑或家庭网络中的设备", "键盘快捷键", "即使 AudioRouter 已最小化到系统托盘"],
]) {
  test(`${tag}: Settings translates help, API, and keyboard shortcut copy`, async ({ page }) => {
    await page.goto("/route-harness.html");
    await openSettings(page, "Appearance");
    await page.getByRole("combobox", { name: "Language", exact: true }).selectOption(tag);
    const pages = page.getByRole("tablist", { name: pagesLabel });
    await pages.getByRole("tab", { name: help, exact: true }).click();
    await expect(
      page.getByText(tag === "fr" ? "Appuyez sur Ctrl+K où que vous soyez" : "在任意位置按 Ctrl+K", { exact: false }),
    ).toBeVisible();
    await pages.getByRole("tab", { name: "API", exact: true }).click();
    await expect(page.getByText(apiIntro, { exact: false })).toBeVisible();
    await pages.getByRole("tab", { name: hotkeys, exact: true }).click();
    await expect(page.getByText(hotkeysIntro, { exact: false })).toBeVisible();
  });
}

for (const [tag, back, session, chooser, history, file, recording, recorder, library, search] of [
  [
    "fr",
    "Retour à la route",
    "Session",
    "Choisir une session",
    "Historique des modifications",
    "Fichier de session",
    "Enregistrement",
    "Enregistreur",
    "Bibliothèque d’enregistrements",
    "Rechercher des enregistrements",
  ],
  ["zh-Hans", "返回路由", "会话", "选择会话", "更改历史", "会话文件", "录音", "录音器", "录音库", "搜索录音"],
]) {
  test(`${tag}: Session and Recording use the selected language`, async ({ page }) => {
    await page.goto("/route-harness.html");
    await openSettings(page, "Appearance");
    await page.getByRole("combobox", { name: "Language", exact: true }).selectOption(tag);
    await page.getByRole("button", { name: back, exact: true }).click();
    await page.getByRole("tab", { name: session, exact: true }).click();
    await expect(page.getByRole("combobox", { name: chooser, exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: file, exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: history, exact: true })).toBeVisible();
    await page.getByRole("tab", { name: recording, exact: true }).click();
    await expect(page.getByRole("heading", { name: recorder, exact: true })).toBeVisible();
    await expect(page.getByRole("region", { name: library }).getByRole("searchbox", { name: search })).toBeVisible();
  });
}

test("switches between French and Simplified Chinese and restores the selected language", async ({ page }) => {
  await page.goto("/route-harness.html");
  await openSettings(page, "Appearance");
  await page.getByRole("combobox", { name: "Language", exact: true }).selectOption({ label: "Français" });
  await expect(page.locator("html")).toHaveAttribute("lang", "fr");
  await expect(page.getByRole("tab", { name: "Apparence", exact: true })).toBeVisible();
  const frenchSearch = page.getByRole("searchbox", { name: "Rechercher un paramètre" });
  await frenchSearch.fill("lang");
  await expect(page.getByRole("tablist", { name: "Pages de paramètres" }).getByRole("tab")).toHaveText(["Apparence"]);
  await frenchSearch.press("Escape");

  await page.getByRole("combobox", { name: "Langue", exact: true }).selectOption({ label: "简体中文" });
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-Hans");
  await expect(page.getByRole("tab", { name: "外观", exact: true })).toBeVisible();
  await page.getByRole("searchbox", { name: "查找设置" }).fill("语言");
  await expect(page.getByRole("tablist", { name: "设置页面" }).getByRole("tab")).toHaveText(["外观"]);

  await page.reload();
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await page.getByRole("tab", { name: "外观", exact: true }).click();
  await expect(page.getByRole("combobox", { name: "语言", exact: true })).toHaveValue("zh-Hans");
});

for (const [tag, back, heading, undo, redo, legend] of [
  [
    "fr",
    "Retour à la route",
    "Flux du signal",
    "Annuler la disposition",
    "Rétablir la disposition",
    "Légende du canevas",
  ],
  ["zh-Hans", "返回路由", "信号流", "撤销布局", "重做布局", "画布图例"],
]) {
  test(`${tag}: localizes the canvas heading, layout actions, and legend`, async ({ page }) => {
    await page.goto("/route-harness.html");
    await openSettings(page, "Appearance");
    await page.getByRole("combobox", { name: "Language", exact: true }).selectOption(tag);
    await page.getByRole("button", { name: back, exact: true }).click();
    await expect(page.getByText(heading, { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: undo, exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: redo, exact: true })).toBeVisible();
    await expect(page.getByText(legend, { exact: true })).toBeVisible();
  });
}

for (const [tag, tools, search, common, guide, gain, help, properties, enabled] of [
  [
    "fr",
    "Outils",
    "Rechercher un outil",
    "Outils courants",
    "Créer une route audio",
    "Gain",
    "Augmentez ou réduisez le niveau du son en décibels.",
    "Propriétés",
    "Activé",
  ],
  [
    "zh-Hans",
    "工具",
    "查找工具",
    "常用工具",
    "如何建立音频路由",
    "增益",
    "以分贝为单位提高或降低声音的电平。",
    "属性",
    "启用",
  ],
]) {
  test(`${tag}: the tool shelf, search, and selected tool use the language catalog`, async ({ page }) => {
    await page.goto("/route-harness.html");
    await openSettings(page, "Appearance");
    await page.getByRole("combobox", { name: "Language", exact: true }).selectOption(tag);
    await page.getByRole("button", { name: tag === "fr" ? "Retour à la route" : "返回路由" }).click();
    await page.getByRole("tab", { name: tools, exact: true }).click();
    await expect(page.getByRole("heading", { name: common })).toBeVisible();
    await expect(page.getByText(guide, { exact: true })).toBeVisible();
    await page.getByRole("searchbox", { name: search }).fill(gain);
    const tool = page.getByRole("button", { name: gain, exact: true });
    await tool.hover();
    await expect(page.locator(".tool-shelf-description")).toContainText(help);
    await tool.click();
    await page.getByRole("tab", { name: properties, exact: true }).click();
    await expect(page.locator(".tool-description")).toHaveText(help);
    await expect(page.getByRole("checkbox", { name: enabled, exact: true })).toBeVisible();
    const editor = page.getByRole("region", { name: tag === "fr" ? "Éditeur de gain" : "增益编辑器", exact: true });
    await expect(
      editor.getByRole("group", { name: tag === "fr" ? "Réglages rapides du gain" : "增益选项" }),
    ).toBeVisible();
  });
}
