import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// Simulated speech: ~300 ms words over −60 dB room noise, fed to the harness
// telemetry at its 50 ms poll. Output and reduction follow the tool's curve.
const sessionFor = (kind: "compressor" | "gate", parameters: Record<string, number>) => ({
  ...demoSession,
  nodes: demoSession.nodes.map((n) => n.id === "voice" ? { ...n, kind, name: kind === "gate" ? "Voice Gate" : "Voice Compressor", parameters, ports: [{ name: "in", direction: "input", channels: 2 }, { name: "out", direction: "output", channels: 2 }] } : n),
});

for (const kind of ["compressor", "gate"] as const) {
  for (const theme of ["dark", "light", "high-contrast"]) {
    test(`${kind} live editor while talking in ${theme}`, async ({ page }, testInfo) => {
      const parameters = kind === "gate" ? { thresholdDb: -40, rangeDb: 40, hysteresisDb: 4, ratio: 4, attackMs: 5, holdMs: 80, releaseMs: 150 } : { thresholdDb: -24, ratio: 3, attackMs: 10, releaseMs: 150, kneeDb: 6, makeupDb: 2 };
      await page.addInitScript(({ session, theme, kind, parameters }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        const started = performance.now();
        const frame = () => {
          const t = (performance.now() - started) / 1000;
          const word = (t % 0.9) < 0.32 && Math.floor(t / 0.9) % 4 !== 3;
          const input = word ? -16 - 5 * Math.abs(Math.sin(t * 7)) : -60 - 2 * Math.sin(t * 3);
          let output = input; let reduction = 0; let open = kind === "gate";
          if (kind === "gate") { open = input >= parameters.thresholdDb; if (!open) { reduction = parameters.rangeDb; output = input - reduction; } }
          else { const over = input - parameters.thresholdDb; reduction = over > 0 ? over * (1 - 1 / parameters.ratio) : 0; output = input - reduction + parameters.makeupDb; }
          return [{ nodeId: "voice", kind, meter: null, plugin: null, processor: { gainReductionDb: [reduction, reduction], gateOpen: [open, open], inputLevelDb: [input, input - 1], outputLevelDb: [output, output - 1] } }];
        };
        Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: true, __routeFixtureTelemetry: frame() });
        setInterval(() => Object.assign(window, { __routeFixtureTelemetry: frame() }), 50);
      }, { session: sessionFor(kind, parameters), theme, kind, parameters });
      await page.setViewportSize({ width: 1440, height: 1000 });
      await page.goto("/route-harness.html");
      await page.getByTestId("rf__node-voice").click();
      const editor = page.getByLabel(kind === "gate" ? "Gate live view" : "Compressor live view");
      await expect(editor).toBeVisible();
      await expect(editor.getByRole("slider", { name: kind === "gate" ? "Threshold (opens)" : "Threshold", exact: true })).toHaveAttribute("aria-valuenow", String(parameters.thresholdDb));
      // Let the 8 s history fill with words and pauses.
      await expect(editor.locator(".dynamics-live-dot")).toBeVisible();
      // The pill, status and suggestion change during talk (and the suggestion
      // first appears); nothing below them may move (UI-17).
      const offsets = await editor.evaluate(async (element) => {
        const sketch = element.querySelector(".dynamics-sketch-details")!;
        const seen = new Set<number>();
        const until = performance.now() + 5500;
        while (performance.now() < until) {
          seen.add(Math.round(sketch.getBoundingClientRect().top - element.getBoundingClientRect().top));
          await new Promise((resolve) => setTimeout(resolve, 40));
        }
        return [...seen];
      });
      expect(offsets).toHaveLength(1);
      await expect(editor.locator(".dynamics-suggestion")).toContainText("room noise");
      await editor.scrollIntoViewIfNeeded();
      await editor.screenshot({ path: testInfo.outputPath(`${kind}-${theme}.png`) });
      await editor.locator(".dynamics-sketch-details").screenshot({ path: testInfo.outputPath(`${kind}-${theme}-sketch.png`) });

      // Drag the threshold handle on the curve: the draft parameter follows.
      const handle = editor.getByRole("slider", { name: kind === "gate" ? "Threshold (opens)" : "Threshold", exact: true });
      await handle.scrollIntoViewIfNeeded();
      const box = (await handle.boundingBox())!;
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + 30, box.y + box.height / 2, { steps: 5 });
      await page.mouse.up();
      const moved = Number(await handle.getAttribute("aria-valuenow"));
      expect(moved).toBeGreaterThan(parameters.thresholdDb);
      await expect(editor.getByRole("slider", { name: kind === "limiter" ? "Ceiling line" : "Threshold line" })).toHaveAttribute("aria-valuenow", String(moved));
      await expect(page.getByText(`Unsaved: thresholdDb: ${parameters.thresholdDb} → ${moved}. Save to keep it.`)).toBeVisible();
      expect(await page.evaluate(() => (window as unknown as { __routeFixtureCalls: () => string[] }).__routeFixtureCalls().filter((x) => x === "commit"))).toEqual([]);
    });
  }
}
