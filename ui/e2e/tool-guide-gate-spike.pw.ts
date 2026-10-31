import { test, expect } from "./browser-test";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { demoSession } from "../src/fixtures";

type Mode = "threshold" | "hold" | "range";
type Telemetry = {
  atSeconds: number;
  gainReductionDb: number[];
  gateOpen: boolean[];
  inputLevelDb: number[];
  outputLevelDb: number[];
};

const root = resolve("../target/tool-guide-spike");
const definitions: Record<Mode, { initial: Record<string, number>; changed: Record<string, number>; slider: string }> =
  {
    threshold: {
      initial: { thresholdDb: -80, rangeDb: 60, hysteresisDb: 0, ratio: 4, attackMs: 5, holdMs: 100, releaseMs: 150 },
      changed: { thresholdDb: -52 },
      slider: "Threshold precise value",
    },
    hold: {
      initial: { thresholdDb: -45, rangeDb: 60, hysteresisDb: 0, ratio: 4, attackMs: 5, holdMs: 100, releaseMs: 150 },
      changed: { holdMs: 600 },
      slider: "Hold precise value",
    },
    range: {
      initial: { thresholdDb: -45, rangeDb: 12, hysteresisDb: 0, ratio: 4, attackMs: 5, holdMs: 100, releaseMs: 150 },
      changed: { rangeDb: 60 },
      slider: "Range precise value",
    },
  };

const waitUntil = async (targetMs: number) => {
  const delay = targetMs - performance.now();
  if (delay > 0) await new Promise((resolveDelay) => setTimeout(resolveDelay, delay));
};

for (const mode of Object.keys(definitions) as Mode[]) {
  test(`capture real Gate Properties with production ${mode} render`, async ({ page }) => {
    test.skip(
      process.env.AUDIOROUTER_RECORD_TOOL_GUIDES !== "1",
      "Local media generator requires prepared render files",
    );
    test.setTimeout(90_000);
    const data = JSON.parse(await readFile(resolve(root, mode, `${mode}.telemetry.json`), "utf8")) as {
      samples: Telemetry[];
      durationSeconds: number;
      changeSeconds: number;
    };
    const settings = definitions[mode];
    const session = {
      ...demoSession,
      edges: [
        {
          id: "guide-mic-to-gate",
          sourceNode: "mic",
          sourcePort: "out",
          destinationNode: "voice",
          destinationPort: "in",
          matrix: [1, 1],
          enabled: true,
        },
        {
          id: "guide-gate-to-headphones",
          sourceNode: "voice",
          sourcePort: "out",
          destinationNode: "headphones",
          destinationPort: "in",
          matrix: [1, 0, 0, 1],
          enabled: true,
        },
      ],
      nodes: demoSession.nodes.map((node) =>
        node.id === "voice"
          ? {
              ...node,
              kind: "gate" as const,
              name: "Voice Gate",
              parameters: settings.initial,
              ports: [
                { name: "in", direction: "input" as const, channels: 2 },
                { name: "out", direction: "output" as const, channels: 2 },
              ],
            }
          : node,
      ),
    };
    await page.addInitScript(
      ({ session, samples }) => {
        localStorage.setItem("audiorouter.ui.theme", "dark");
        const control = { start: performance.now() };
        Object.assign(window, {
          __guideGateClock: control,
          __routeFixtureSession: session,
          __routeFixtureRunning: true,
          __routeFixtureProcessors: [
            {
              id: "gate",
              version: 1,
              category: "Dynamics",
              availability: { status: "available" },
              latencySamples: 0,
              parameters: [
                { name: "thresholdDb", type: "number", unit: "dB", step: 1, minimum: -80, maximum: 0, default: -45 },
                { name: "rangeDb", type: "number", unit: "dB", step: 1, minimum: 0, maximum: 80, default: 60 },
                { name: "hysteresisDb", type: "number", unit: "dB", step: 1, minimum: 0, maximum: 12, default: 3 },
                { name: "ratio", type: "number", step: 0.1, minimum: 1, maximum: 20, default: 4 },
                { name: "attackMs", type: "number", unit: "ms", step: 0.1, minimum: 0.1, maximum: 100, default: 5 },
                { name: "holdMs", type: "number", unit: "ms", step: 1, minimum: 0, maximum: 1000, default: 50 },
                { name: "releaseMs", type: "number", unit: "ms", step: 1, minimum: 10, maximum: 2000, default: 150 },
              ],
            },
          ],
          __routeFixtureTelemetry: [
            { nodeId: "voice", kind: "gate", meter: null, plugin: null, processor: samples[0] },
          ],
        });
        setInterval(() => {
          const elapsed = (performance.now() - control.start) / 1000;
          let index = 0;
          while (index + 1 < samples.length && samples[index + 1].atSeconds <= elapsed) index++;
          Object.assign(window, {
            __routeFixtureTelemetry: [
              { nodeId: "voice", kind: "gate", meter: null, plugin: null, processor: samples[index] },
            ],
          });
        }, 50);
      },
      { session, samples: data.samples },
    );

    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").click();
    await page.getByRole("button", { name: "Fit View" }).click();
    const editor = page.getByLabel("Gate live view");
    await expect(editor).toBeVisible();
    await page.addStyleTag({
      content: `
        .topbar, .workspace-notifications, .workspace-grid > .sidebar { display: none !important; }
        .workspace-grid { display: block !important; width: 100vw !important; height: 100vh !important; }
        .main-content {
          display: grid !important; grid-template-columns: minmax(0, 1fr) !important;
          grid-template-rows: minmax(0, 1fr) !important; width: 100vw !important;
          height: 100vh !important; max-width: none !important; padding: 0 !important;
          margin: 0 !important; gap: 0 !important;
        }
        .main-content > :not(.inspector):not(.canvas-panel) { display: none !important; }
        .main-content > .canvas-panel {
          display: block !important; position: fixed !important; left: -20000px !important;
          top: 0 !important; width: 800px !important; height: 600px !important;
          visibility: hidden !important; pointer-events: none !important;
        }
        .main-content > .inspector {
          display: block !important; grid-column: 1 !important; grid-row: 1 !important;
          width: 100% !important; height: 100% !important; min-height: 0 !important;
          max-height: none !important; margin: 0 !important; overflow: auto !important;
        }
      `,
    });
    await page.setViewportSize({ width: 900, height: 1600 });
    await expect(page.locator(".canvas-panel")).toBeHidden();
    await page.locator(".inspector").evaluate((element) => {
      element.scrollTop = 0;
    });
    const slider = page.getByRole("spinbutton", { name: settings.slider, exact: true });
    const parameterName = mode === "threshold" ? "thresholdDb" : mode === "hold" ? "holdMs" : "rangeDb";
    const initialValue = settings.initial[parameterName];
    await expect(slider).toHaveValue(String(initialValue));
    await slider.scrollIntoViewIfNeeded();
    await slider.focus();
    await page.keyboard.press("ArrowUp");
    await expect(slider).toHaveValue(String(initialValue + 1));
    await page.keyboard.press("ArrowDown");
    await expect(slider).toHaveValue(String(initialValue));
    await page.locator(".inspector").evaluate((element) => {
      element.scrollTop = 0;
    });
    await expect(page.locator(".dynamics-top")).toBeInViewport();

    const cdp = await page.context().newCDPSession(page);
    await cdp.send("Page.enable");
    const latestFrame: { current: { data: string; sequence: number } | null } = { current: null };
    let sequence = 0;
    cdp.on("Page.screencastFrame", (frame) => {
      latestFrame.current = { data: frame.data, sequence: ++sequence };
      void cdp.send("Page.screencastFrameAck", { sessionId: frame.sessionId });
    });
    await cdp.send("Page.startScreencast", {
      format: "jpeg",
      quality: 92,
      maxWidth: 900,
      maxHeight: 1600,
      everyNthFrame: 1,
    });
    const firstFrameDeadline = performance.now() + 3000;
    while (!latestFrame.current && performance.now() < firstFrameDeadline)
      await new Promise((resolveDelay) => setTimeout(resolveDelay, 25));
    expect(latestFrame.current, "Edge should provide the first rendered frame").not.toBeNull();

    const frames = resolve(root, "frames", mode);
    await mkdir(frames, { recursive: true });
    await page.evaluate(() => {
      const control = (window as unknown as { __guideGateClock: { start: number } }).__guideGateClock;
      control.start = performance.now();
    });
    const start = performance.now();
    const frameCount = data.durationSeconds * 30;
    const eventFrame = Math.round(data.changeSeconds * 30);
    const eventActionFrame = Math.max(0, eventFrame - 1);
    const revealParameterFrame = Math.max(0, eventActionFrame - 15);
    const returnToLiveVisualFrame = Math.min(frameCount - 1, eventFrame + 15);
    let eventAtMs: number | null = null;
    const frameTimes: number[] = [];
    const capturedFrames: string[] = [];

    for (let frame = 0; frame < frameCount; frame++) {
      const targetMs = start + (frame * 1000) / 30;
      await waitUntil(targetMs);
      if (frame === revealParameterFrame) await slider.scrollIntoViewIfNeeded();
      if (frame === eventActionFrame) {
        const previousSequence = latestFrame.current?.sequence ?? 0;
        const value = settings.changed[parameterName];
        await slider.evaluate((element, nextValue) => {
          const input = element as HTMLInputElement;
          const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
          if (!setValue) throw new Error("Native input setter unavailable");
          setValue.call(input, String(nextValue));
          input.dispatchEvent(new Event("input", { bubbles: true }));
        }, value);
        eventAtMs = performance.now() - start;
        await expect(slider).toHaveValue(String(value));
        expect(Math.abs(eventAtMs - data.changeSeconds * 1000)).toBeLessThanOrEqual(1000 / 30);
        const updateDeadline = performance.now() + 1000;
        while ((latestFrame.current?.sequence ?? 0) <= previousSequence && performance.now() < updateDeadline)
          await new Promise((resolveDelay) => setTimeout(resolveDelay, 4));
      }
      if (frame === returnToLiveVisualFrame) {
        await page.locator(".inspector").evaluate((element) => {
          element.scrollTop = 0;
        });
      }
      expect(latestFrame.current).not.toBeNull();
      frameTimes.push(performance.now() - start);
      capturedFrames.push(latestFrame.current!.data);
    }
    await cdp.send("Page.stopScreencast");
    await cdp.detach();
    for (let frame = 0; frame < capturedFrames.length; frame++)
      await writeFile(
        resolve(frames, `frame-${String(frame).padStart(4, "0")}.jpg`),
        Buffer.from(capturedFrames[frame], "base64"),
      );
    await writeFile(
      resolve(root, `${mode}.capture.json`),
      JSON.stringify(
        {
          mode,
          frameRate: 30,
          durationSeconds: data.durationSeconds,
          changeSeconds: data.changeSeconds,
          eventActionFrame,
          eventAtMs,
          frameTimes,
        },
        null,
        2,
      ),
    );
    await page.screenshot({ path: resolve(root, `${mode}-inspector.png`) });
  });
}
