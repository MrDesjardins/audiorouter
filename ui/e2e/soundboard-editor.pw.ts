// Soundboard card and Properties in Edge: layout stays still while sounds play (UI-17),
// files dropped on the card or a button fill a slot (SND-19), and the picker searches,
// filters and previews on this computer only (SND-12).
import { test, expect, type Page } from "./browser-test";
import type { Session } from "@audiorouter/contracts";
import { setColorTheme } from "./workbench";

type SlotStatus = { slot: number; loaded: boolean; playing: boolean; progress: number };

const library = {
  installed: true,
  categories: [
    { id: "horns-alerts", title: "Horns and alerts" },
    { id: "bodily-humor", title: "Bodily humor" },
  ],
  sounds: [
    ["lib:air-horn-01", "Air horn 01", "horns-alerts", ["horn"]],
    ["lib:fart-02", "Fart 02", "bodily-humor", ["gross"]],
    ["lib:burp-01", "Burp 01", "bodily-humor", ["gross", "belch"]],
  ].map(([id, title, category, tags]) => ({
    id,
    title,
    category,
    tags,
    durationMs: 900,
    license: "CC0",
    author: "AudioRouter",
  })),
};

function boardSession(parameters: Record<string, string | number>): Session {
  return {
    id: "soundboard-editor",
    name: "Soundboard editor",
    schemaVersion: 1,
    revision: 1,
    nodes: [
      {
        id: "board",
        kind: "soundboard",
        typeVersion: 1,
        name: "Soundboard 1",
        enabled: true,
        bypass: false,
        parameters,
        ports: [{ name: "out", direction: "output", channels: 2 }],
      },
      {
        id: "cable",
        kind: "physicalOutput",
        typeVersion: 1,
        name: "Cable-A",
        enabled: true,
        bypass: false,
        parameters: {},
        ports: [{ name: "in", direction: "input", channels: 2 }],
      },
    ],
    edges: [
      {
        id: "board-cable",
        sourceNode: "board",
        sourcePort: "out",
        destinationNode: "cable",
        destinationPort: "in",
        matrix: [1, 0, 0, 1],
        enabled: true,
      },
    ],
  };
}

const threeSounds = {
  slot1MediaId: "lib:burp-01",
  slot1FileName: "Burp 01",
  slot1Caption: "A very long caption that the card truncates",
  slot2MediaId: "lib:fart-02",
  slot2FileName: "Fart 02",
  slot3MediaId: "lib:air-horn-01",
  slot3FileName: "Air horn 01",
  slot3Mode: "hold",
};

const idle = (): SlotStatus[] =>
  Array.from({ length: 9 }, (_, index) => ({ slot: index + 1, loaded: index < 3, playing: false, progress: 0 }));

async function openBoard(
  page: Page,
  theme: string,
  parameters: Record<string, string | number>,
  extra: Record<string, unknown> = {},
) {
  const session = boardSession(parameters);
  await page.addInitScript(
    ({ session, theme, slots, library, extra }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixturePrivacyMuted: false,
        __routeFixtureSoundboardStatus: { slots, ignoredTriggers: 0, droppedCommands: 0 },
        __routeFixtureSoundLibrary: library,
        __routeFixtureUploadDurationMs: 2_000,
        ...extra,
      });
    },
    { session, theme, slots: idle(), library, extra },
  );
  await page.goto("/route-harness.html");
  await setColorTheme(page, theme);
  const card = page.getByTestId("rf__node-board");
  await expect(card).toBeVisible();
  await card.click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  const editor = page.locator(".soundboard-editor");
  await expect(editor).toBeVisible();
  return { card, editor };
}

/** Sets the status the next `soundboard.status` polls return. */
async function setStatus(page: Page, slots: SlotStatus[]) {
  await page.evaluate((slots) => {
    (window as unknown as { __routeFixtureSoundboardStatus: { slots: unknown } }).__routeFixtureSoundboardStatus.slots =
      slots;
  }, slots);
}

/** Bounds of the card, its buttons, and every editor control and status line, rounded to 0.1 px. */
async function layout(page: Page) {
  return page.evaluate(() => {
    const round = (value: number) => Math.round(value * 10) / 10;
    const box = (element: Element) => {
      const rect = element.getBoundingClientRect();
      return [round(rect.left), round(rect.top), round(rect.width), round(rect.height)].join(",");
    };
    const card = document.querySelector('[data-testid="rf__node-board"]')!;
    const editor = document.querySelector(".soundboard-editor")!;
    return [
      box(card),
      ...[...card.querySelectorAll(".soundboard-button, .soundboard-stop")].map(box),
      ...[
        ...editor.querySelectorAll(
          ".soundboard-slot, .soundboard-slot-head > *, input, select, button, .soundboard-editor-status",
        ),
      ].map(box),
    ];
  });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Soundboard card and Properties keep their layout while sounds play (${theme})`, async ({ page }, testInfo) => {
    const { card, editor } = await openBoard(page, theme, threeSounds);
    const playing = (progress: Record<number, number>) =>
      idle().map((slot) => (slot.slot in progress ? { ...slot, playing: true, progress: progress[slot.slot] } : slot));
    // Idle, one sound rising, two overlapping, a drag hovering, idle again: several 150 ms
    // polls each. Every sample must give the same bounds.
    const phases: { name: string; slots: SlotStatus[]; drag?: boolean }[] = [
      { name: "idle", slots: idle() },
      { name: "slot 1 at 10 %", slots: playing({ 1: 0.1 }) },
      { name: "slot 1 at 60 %", slots: playing({ 1: 0.6 }) },
      { name: "slots 1 and 3", slots: playing({ 1: 0.9, 3: 0.3 }) },
      { name: "drag over the card", slots: playing({ 3: 0.5 }), drag: true },
      { name: "idle again", slots: idle() },
    ];
    const baseline = await layout(page);
    const samples: { phase: string; layout: string[] }[] = [];
    for (const phase of phases) {
      await setStatus(page, phase.slots);
      if (phase.drag)
        await card.locator(".soundboard-node").evaluate((element) => {
          const transfer = new DataTransfer();
          transfer.items.add(new File([new Uint8Array(8)], "drag.wav", { type: "audio/wav" }));
          element.dispatchEvent(
            new DragEvent("dragenter", { bubbles: true, cancelable: true, dataTransfer: transfer }),
          );
        });
      for (let poll = 0; poll < 4; poll += 1) {
        await page.waitForTimeout(120);
        samples.push({ phase: phase.name, layout: await layout(page) });
      }
      if (phase.name === "slots 1 and 3") {
        await expect(editor.locator(".soundboard-slot-state").nth(0)).toHaveText("Playing");
        await expect(editor.locator(".soundboard-slot-state").nth(2)).toHaveText("Playing");
        await expect(card.locator(".soundboard-button.is-playing")).toHaveCount(2);
        await card.screenshot({ path: testInfo.outputPath(`soundboard-card-playing-${theme}.png`) });
        await editor.screenshot({ path: testInfo.outputPath(`soundboard-properties-playing-${theme}.png`) });
      }
      if (phase.drag) {
        await expect(card.locator(".soundboard-node.is-drop-target")).toHaveCount(1);
        await card.screenshot({ path: testInfo.outputPath(`soundboard-card-drop-${theme}.png`) });
        await card
          .locator(".soundboard-node")
          .evaluate((element) =>
            element.dispatchEvent(new DragEvent("dragleave", { bubbles: true, relatedTarget: null })),
          );
      }
    }
    await expect(editor.locator(".soundboard-slot-state").nth(0)).toHaveText("");
    await expect(card.locator(".soundboard-button.is-playing")).toHaveCount(0);
    const moved = samples.filter((sample) => JSON.stringify(sample.layout) !== JSON.stringify(baseline));
    expect(moved.map((sample) => sample.phase)).toEqual([]);
    expect(samples).toHaveLength(phases.length * 4);
  });
}

/** Drops one file (built in the page) on `selector`, as Explorer would through WebView2's HTML5 events. */
async function dropFile(page: Page, selector: string, fileName: string) {
  return page.evaluate(
    ({ selector, fileName }) => {
      const transfer = new DataTransfer();
      transfer.items.add(new File([new Uint8Array(64)], fileName, { type: "audio/wav" }));
      const target = document.querySelector(selector)!;
      const results = ["dragenter", "dragover", "drop"].map((type) =>
        target.dispatchEvent(new DragEvent(type, { bubbles: true, cancelable: true, dataTransfer: transfer })),
      );
      // dispatchEvent is false when the target took the event (preventDefault).
      return results.every((result) => !result);
    },
    { selector, fileName },
  );
}

test("a file dropped on a button replaces its sound, and on the card fills the next slot", async ({ page }) => {
  const { card, editor } = await openBoard(page, "dark", {
    slot1MediaId: "lib:burp-01",
    slot1FileName: "Burp 01",
    slot1Caption: "Belch",
    slot2MediaId: "lib:fart-02",
    slot2FileName: "Fart 02",
  });
  expect(await dropFile(page, '[data-testid="rf__node-board"] [aria-label="Play Fart 02"]', "horn.wav")).toBe(true);
  await expect(card.getByRole("button", { name: "Play horn" })).toBeVisible();
  await expect(card.getByRole("button", { name: "Play Fart 02" })).toHaveCount(0);
  expect(await dropFile(page, '[data-testid="rf__node-board"] .soundboard-add', "clap.wav")).toBe(true);
  await expect(card.getByRole("button", { name: "Play clap" })).toBeVisible();
  await expect(editor.locator(".soundboard-slot")).toHaveCount(3);
  // Properties takes drops too: on a slot it replaces that sound and keeps its caption.
  expect(await dropFile(page, ".soundboard-editor .soundboard-slot:first-child", "boom.wav")).toBe(true);
  await expect(editor.locator(".soundboard-slot").first().locator(".soundboard-slot-file")).toHaveText("boom.wav");
  await expect(card.getByRole("button", { name: "Play Belch" })).toBeVisible();
  // A file the picker would refuse is refused before any upload.
  expect(await dropFile(page, ".soundboard-editor", "notes.txt")).toBe(true);
  await expect(editor.locator(".soundboard-editor-status")).toContainText("Choose a .wav or .mp3 file.");
  const uploads = await page.evaluate(() =>
    (
      window as unknown as { __routeFixtureUploads: () => { fileName: string; chunks: number }[] }
    ).__routeFixtureUploads(),
  );
  expect(uploads.map((upload) => upload.fileName)).toEqual(["horn.wav", "clap.wav", "boom.wav"]);
  expect(uploads.every((upload) => upload.chunks === 1)).toBe(true);
  // A drag without files (a tool from the library) passes through the card to the canvas,
  // while a file drag stops at the card.
  const reachesCanvas = (withFile: boolean) =>
    page.evaluate((withFile) => {
      // Above the React root: reached unless a React handler stopped propagation.
      const root = document;
      let reached = false;
      const listen = () => (reached = true);
      root.addEventListener("dragover", listen);
      const transfer = new DataTransfer();
      if (withFile) transfer.items.add(new File([new Uint8Array(8)], "x.wav", { type: "audio/wav" }));
      else transfer.setData("text/plain", "gain");
      document
        .querySelector('[data-testid="rf__node-board"] .soundboard-node')!
        .dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: transfer }));
      root.removeEventListener("dragover", listen);
      return reached;
    }, withFile);
  expect(await reachesCanvas(false)).toBe(true);
  expect(await reachesCanvas(true)).toBe(false);
  await expect(card.locator(".soundboard-node.is-drop-target")).toHaveCount(1);
});

test("the picker searches, filters, previews on this computer only, and fills a slot", async ({ page }) => {
  const { card, editor } = await openBoard(page, "dark", { slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" });
  const lifecycleBefore = await page.evaluate(() =>
    (window as unknown as { __routeFixtureCalls: () => string[] }).__routeFixtureCalls(),
  );
  await editor.getByRole("button", { name: "Add sound" }).click();
  const dialog = page.getByRole("dialog", { name: "Add a sound" });
  const rows = dialog.getByRole("list", { name: "Built-in sounds" }).getByRole("listitem");
  await expect(rows).toHaveCount(3);
  await expect(rows.first()).toContainText("Burp 01");
  await dialog.getByRole("searchbox", { name: "Search sounds" }).fill("horns and");
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("Air horn 01");
  await dialog.getByRole("searchbox", { name: "Search sounds" }).fill("");
  await dialog.getByRole("combobox", { name: "Category" }).selectOption("bodily-humor");
  await expect(rows).toHaveCount(2);
  const statusBox = await dialog.locator(".sound-picker-status").boundingBox();
  await dialog.getByRole("button", { name: "Preview Fart 02" }).click();
  await expect(dialog.getByRole("button", { name: "Preview Fart 02" })).toBeEnabled();
  await expect(dialog.locator(".sound-picker-status")).toContainText("Previews play on this computer only");
  expect(await dialog.locator(".sound-picker-status").boundingBox()).toEqual(statusBox);
  const libraryCalls = await page.evaluate(() =>
    (window as unknown as { __routeFixtureLibraryCalls: () => string[] }).__routeFixtureLibraryCalls(),
  );
  expect(libraryCalls).toEqual(["soundLibrary.list", "soundLibrary.read:lib:fart-02"]);
  expect(
    await page.evaluate(() => (window as unknown as { __routeFixtureCalls: () => string[] }).__routeFixtureCalls()),
  ).toEqual(lifecycleBefore);
  await dialog.getByRole("button", { name: "Use Fart 02" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(card.getByRole("button", { name: "Play Fart 02" })).toBeVisible();
  // Change the mode and remove the first sound; the card follows the draft.
  await editor.getByRole("combobox", { name: "When pressed" }).nth(1).selectOption("loop");
  await expect(editor.getByRole("combobox", { name: "When pressed" }).nth(1)).toHaveValue("loop");
  await editor.getByRole("button", { name: "Delete sound 1" }).click();
  await expect(card.getByRole("button", { name: "Play Burp 01" })).toHaveCount(0);
  await expect(editor.locator(".soundboard-slot")).toHaveCount(1);
});
