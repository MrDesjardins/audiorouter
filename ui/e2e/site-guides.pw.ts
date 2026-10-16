import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
const site = new URL("../../site/", import.meta.url);
const manifest = JSON.parse(readFileSync(new URL("assets/guide-manifest.json", site), "utf8")) as {
  guides: { kind: string; title: string; path: string; propertyCount: number; audioRelevant: boolean }[];
};
declare global {
  interface Window {
    __guideMonitorGains?: AudioParam[];
  }
}
test("every guide opens offline with a labelled illustration and complete controls", async ({ page }) => {
  await page.route("https://**", (route) => route.abort());
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  for (const guide of manifest.guides) {
    await page.goto(new URL(guide.path, site).href);
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(guide.title);
    await expect(page.locator(".guide-property")).toHaveCount(Math.max(1, guide.propertyCount));
    await expect(page.locator(".guide-animation svg")).toHaveAttribute("role", "img");
    await expect(page.getByRole("heading", { name: "Use it in AudioRouter", exact: true })).toBeVisible();
    await expect(page.locator(".guide-walkthrough li")).not.toHaveCount(0);
    await expect(page.getByRole("heading", { name: "What success looks like", exact: true })).toBeVisible();
    await expect(page.locator(".guide-signal-panel")).toHaveCount(guide.audioRelevant ? 1 : 0);
    if (guide.kind === "recorder") {
      await expect(page.getByRole("button", { name: "Record example", exact: true })).toBeVisible();
      await expect(page.getByRole("button", { name: "Stop recording", exact: true })).toBeDisabled();
      await expect(page.getByRole("button", { name: "Play recording", exact: true })).toBeDisabled();
    } else {
      await expect(
        page.getByRole("button", { name: guide.audioRelevant ? "Play example" : "Play animation", exact: true }),
      ).toBeVisible();
      await expect(page.locator(".guide-animation button")).toHaveCount(guide.audioRelevant ? 2 : 2);
    }
    await expect(page.locator(".guide-audio-demo")).toHaveCount(0);
    await expect(page.locator("[data-guide-demo]")).not.toHaveAttribute("data-audio-state", "playing");
    expect(await page.evaluate(() => document.getAnimations().filter((a) => a.playState === "running").length)).toBe(0);
    expect(await page.evaluate(() => document.body.scrollWidth > innerWidth)).toBe(false);
  }
  const byKind = new Map(manifest.guides.map((guide) => [guide.kind, guide.audioRelevant]));
  for (const kind of ["physicalInput", "applicationCapture", "endpointLoopback", "physicalOutput", "plugin", "group"])
    expect(byKind.get(kind), `${kind} should not imply an app-controlled audio preview`).toBe(false);
  expect(byKind.get("audioFile")).toBe(true);
  expect(errors).toEqual([]);
});

test("audible fixtures match the tools rather than a common effect", async ({ page }) => {
  test.setTimeout(60_000);
  await page.addInitScript(() => {
    const createGain = AudioContext.prototype.createGain;
    AudioContext.prototype.createGain = function () {
      const node = createGain.call(this);
      window.__guideMonitorGains = [...(window.__guideMonitorGains ?? []), node.gain];
      return node;
    };
  });
  await page.goto(new URL("tools/mixer.html", site).href);
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-signal-ready", "true");
  const checks = await page.evaluate(async () => {
    type Result = { inputs: { channels: Float32Array[] }[]; output: { channels: Float32Array[] } };
    const signal = (
      window as unknown as {
        AudioRouterGuideSignal: {
          render: (
            kind: string,
            params?: Record<string, number | boolean | string>,
            options?: Record<string, unknown>,
          ) => Promise<Result>;
        };
      }
    ).AudioRouterGuideSignal;
    const energy = (samples: Float32Array, start = 0, end = samples.length) => {
      let sum = 0;
      for (let i = start; i < end; i++) sum += samples[i] ** 2;
      return sum / (end - start);
    };
    const diff = (a: Float32Array, b: Float32Array) =>
      Math.max(...Array.from({ length: 1000 }, (_, i) => Math.abs(a[i * 300] - b[i * 300])));
    const mix = await signal.render("mixer", { track1Percent: 100, track2Percent: 0 });
    const a = mix.inputs[0].channels[0],
      b = mix.inputs[1].channels[0];
    let cross = 0,
      aa = 0,
      bb = 0;
    for (let i = 0; i < a.length; i++) {
      cross += a[i] * b[i];
      aa += a[i] ** 2;
      bb += b[i] ** 2;
    }
    const mixed = await signal.render("mixer", { track1Percent: 40, track2Percent: 70 });
    let mixError = 0;
    for (let i = 0; i < a.length; i++)
      mixError = Math.max(mixError, Math.abs(mixed.output.channels[0][i] - 0.4 * a[i] - 0.7 * b[i]));
    const delay = await signal.render("delay", { delayMs: 250 });
    let delayError = 0;
    for (let i = 12000; i < a.length; i++)
      delayError = Math.max(delayError, Math.abs(delay.output.channels[0][i] - delay.inputs[0].channels[0][i - 12000]));
    const duck = await signal.render("duck", { amountDb: 18 });
    const ducked = duck.output.channels[0],
      music = duck.inputs[0].channels[0];
    const denoise = await signal.render("denoise", { reductionPercent: 100, floorPercent: 0 });
    const rawNoise = energy(denoise.inputs[0].channels[0], 0, 48000),
      remainingNoise = energy(denoise.output.channels[0], 0, 48000);
    const gate = await signal.render("gate", { thresholdDb: -35, rangeDb: 60 });
    const dehum = await signal.render("dehum", { frequencyHz: 60, harmonics: 8, amountPercent: 100 });
    const declickLow = await signal.render("declick", { thresholdPercent: 0 });
    const declickHigh = await signal.render("declick", { thresholdPercent: 100 });
    const pitch0 = await signal.render("pitch", { semitones: 0 }),
      pitch12 = await signal.render("pitch", { semitones: 12 });
    const noteEnergy = (samples: Float32Array) =>
      Array.from({ length: 16 }, (_, i) => energy(samples, i * 24000, (i + 1) * 24000));
    const quietTone = await signal.render("testSignal", { levelDb: -24 }),
      loudTone = await signal.render("testSignal", { levelDb: -12 });
    const limiter = await signal.render("limiter", { ceilingDb: -6 });
    let limitPeak = 0;
    for (const sample of limiter.output.channels[0]) limitPeak = Math.max(limitPeak, Math.abs(sample));
    const switchB = await signal.render("inputSwitch", { selected: "b" });
    const received = await signal.render("networkReceive", { bufferMs: 120 });
    return {
      distinctCorrelation: Math.abs(cross / Math.sqrt(aa * bb)),
      soloAError: diff(a, mix.output.channels[0]),
      mixError,
      delayError,
      delayStart: energy(delay.output.channels[0], 0, 12000),
      duckQuietRatio: energy(ducked, 0, 48000) / energy(music, 0, 48000),
      duckSpeechRatio: energy(ducked, 80000, 144000) / energy(music, 80000, 144000),
      rawNoise,
      remainingNoise,
      gateQuietRatio: energy(gate.output.channels[0], 24000, 48000) / energy(gate.inputs[0].channels[0], 24000, 48000),
      dehumRatio: energy(dehum.output.channels[0], 24000, 48000) / energy(dehum.inputs[0].channels[0], 24000, 48000),
      clickChange:
        diff(declickLow.output.channels[0], declickHigh.output.channels[0]) +
        Math.abs(declickLow.output.channels[0][33600] - declickHigh.output.channels[0][33600]),
      pitchTimes0: noteEnergy(pitch0.output.channels[0]),
      pitchTimes12: noteEnergy(pitch12.output.channels[0]),
      pitchDifference: diff(pitch0.output.channels[0], pitch12.output.channels[0]),
      toneRatio: energy(loudTone.output.channels[0]) / energy(quietTone.output.channels[0]),
      limitPeak,
      switchError: diff(switchB.output.channels[0], switchB.inputs[1].channels[0]),
      networkBeforePair: energy(received.output.channels[0], 0, 115200),
      networkAfterPair: energy(received.output.channels[0], 144000, 192000),
    };
  });
  expect(checks.distinctCorrelation).toBeLessThan(0.1);
  expect(checks.soloAError).toBe(0);
  expect(checks.mixError).toBeLessThan(1e-6);
  expect(checks.delayError).toBe(0);
  expect(checks.delayStart).toBe(0);
  expect(checks.duckQuietRatio).toBeCloseTo(1, 3);
  expect(checks.duckSpeechRatio).toBeLessThan(0.1);
  expect(checks.remainingNoise).toBeLessThan(checks.rawNoise * 0.01);
  expect(checks.gateQuietRatio).toBeLessThan(0.01);
  expect(checks.dehumRatio).toBeLessThan(0.1);
  expect(checks.clickChange).toBeGreaterThan(0.1);
  expect(checks.pitchDifference).toBeGreaterThan(0.1);
  checks.pitchTimes0.forEach((energy, i) => expect(checks.pitchTimes12[i]).toBeCloseTo(energy, 3));
  expect(checks.toneRatio).toBeCloseTo(10 ** (12 / 10), 2);
  expect(checks.limitPeak).toBeLessThanOrEqual(10 ** (-6 / 20) + 1e-6);
  expect(checks.switchError).toBeLessThan(1e-7);
  expect(checks.networkBeforePair).toBe(0);
  expect(checks.networkAfterPair).toBeGreaterThan(0.001);
  await expect(page.locator(".guide-signal-row[data-signal=input0]")).toContainText("drums & bass");
  await expect(page.locator(".guide-signal-row[data-signal=input1]")).toContainText("bell melody");
  await page.getByLabel("Listen to example signal").selectOption("input1");
  await page.getByRole("button", { name: "Play example", exact: true }).click();
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audition", "input1");
  await expect.poll(() => page.evaluate(() => window.__guideMonitorGains?.[0]?.value)).toBe(0.5);
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "playing");
  await page.getByLabel("Listen to example signal").selectOption("output");
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audition", "output");
  await page.goto(new URL("tools/graphic-eq.html", site).href);
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-signal-ready", "true");
  const eqRms = await page.evaluate(async () => {
    type Signal = { render: (kind: string) => Promise<{ output: { channels: Float32Array[] } }> };
    const samples = await (window as unknown as { AudioRouterGuideSignal: Signal }).AudioRouterGuideSignal.render(
      "graphicEq",
    );
    let sum = 0;
    for (const sample of samples.output.channels[0]) sum += sample * sample;
    return Math.sqrt(sum / samples.output.channels[0].length);
  });
  expect(eqRms).toBeGreaterThan(0.1);
  await page.getByRole("button", { name: "Play example", exact: true }).click();
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "playing");
  await expect.poll(() => page.evaluate(() => window.__guideMonitorGains?.[0]?.value)).toBe(0.5);
});
test("FIR, Graphic EQ, Advanced EQ and Network Receive use tool-shaped diagrams and one synced transport", async ({
  page,
}, info) => {
  test.setTimeout(60_000);
  await page.route("https://**", (route) => route.abort());
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));

  await page.goto(new URL("tools/fir-filter.html", site).href);
  const fir = page.locator(".guide-animation svg");
  await expect(fir).toContainText("IMPORTED IMPULSE RESPONSE");
  await expect(fir).toContainText("Wet mix 100%");
  await page.getByRole("button", { name: "Play example", exact: true }).click();
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "playing");
  await expect(page.getByRole("button", { name: "Pause example", exact: true })).toBeVisible();
  const animationTime = () =>
    page.locator("[data-guide-demo]").evaluate((element) => {
      const host = element as HTMLElement & {
        __guideAnimation?: { currentTime: () => number };
      };
      return host.__guideAnimation?.currentTime() ?? 0;
    });
  await expect.poll(animationTime).toBeGreaterThan(200);
  await page.getByLabel("Seek example").fill("3.2");
  await expect.poll(animationTime).toBeGreaterThan(3200);
  await page.getByLabel("Illustration Wet mix").fill("40");
  await expect(fir).toContainText("Wet mix 40%");
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "playing");
  await page.getByRole("button", { name: "Pause example", exact: true }).click();
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "paused");
  await page.locator(".guide-animation").screenshot({ path: info.outputPath("fir-filter-interactive.png") });
  await expect(page.locator(".guide-animation svg [data-guide-playhead]")).toBeAttached();

  await page.goto(new URL("tools/graphic-eq.html", site).href);
  await expect(page.locator(".guide-animation svg [data-graphic-band]")).toHaveCount(10);
  await expect(page.locator("[data-demo-key^='band']")).toHaveCount(10);
  for (const phrase of [
    "sub-bass",
    "kick-drum",
    "bass body",
    "boxy",
    "lower-mid",
    "honk",
    "articulation",
    "presence",
    "brightness",
    "extreme top end",
  ])
    await expect(page.locator("#configuration")).toContainText(phrase);
  await page.locator(".guide-animation").screenshot({ path: info.outputPath("graphic-eq-bands.png") });

  await page.goto(new URL("tools/parametric-eq.html", site).href);
  await expect(page.locator(".guide-animation svg [data-parametric-band]")).toHaveCount(4);
  await expect(page.getByLabel("Filter shape for selected band").locator("option")).toHaveCount(8);
  await page.getByLabel("Select band to edit").selectOption("2");
  await page.getByLabel("Filter shape for selected band").selectOption("allPass");
  await expect(page.locator(".guide-animation")).toContainText("changes phase");
  await page.locator(".guide-animation").screenshot({ path: info.outputPath("advanced-eq-bands.png") });

  await page.goto(new URL("tools/network-receive.html", site).href);
  const network = page.locator(".guide-animation svg");
  await expect(network).toContainText("WAITING FOR NETWORK SEND");
  await expect(network.locator("[data-network-buffer-slot]")).toHaveCount(6);
  await page.getByLabel("Seek example").fill("3.2");
  await expect(network).toContainText("PAIRED · BUFFERING INCOMING AUDIO");
  await page.locator(".guide-animation").screenshot({ path: info.outputPath("network-receive-buffer.png") });
  await page.getByRole("button", { name: "Play example", exact: true }).click();
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "playing");
  await page.getByLabel("Illustration Receive buffer").fill("120");
  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent("pagehide")));
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "stopped");
  expect(errors).toEqual([]);
});
test("keyboard animation controls, changing geometry, pause and reduced motion", async ({ page }, info) => {
  await page.route("https://**", (route) => route.abort());
  await page.goto(new URL("tools/compressor.html", site).href);
  const play = page.getByRole("button", { name: "Play example", exact: true });
  await play.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: "Pause example", exact: true })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(play).toBeVisible();
  await page.getByLabel("Seek example").fill("4");
  expect(await page.evaluate(() => document.getAnimations().every((a) => a.playState !== "running"))).toBe(true);
  await page.getByLabel("Illustration Ratio").focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.locator("#demo-value-ratio")).toContainText("3.5");
  await page.locator(".guide-animation").screenshot({ path: info.outputPath("compressor-interactive.png") });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 390, height: 844 });
  for (const slug of ["parametric-eq", "physical-input", "recorder", "network-receive", "network-send", "group"]) {
    await page.goto(new URL("tools/" + slug + ".html", site).href);
    await page.locator("#see-it").scrollIntoViewIfNeeded();
    expect(await page.evaluate(() => document.getAnimations().filter((a) => a.playState === "running").length)).toBe(0);
    expect(await page.evaluate(() => document.body.scrollWidth > innerWidth)).toBe(false);
    if (slug === "parametric-eq") {
      await page.getByLabel("Filter shape for selected band").selectOption("allPass");
      await expect(page.locator(".guide-animation")).toContainText("changes phase");
    }
    await page.screenshot({ path: info.outputPath(slug + "-mobile.png") });
  }
});
test("Mixer, Meter and Recorder examples match their AudioRouter workflows", async ({ page }, info) => {
  await page.route("https://**", (route) => route.abort());

  await page.goto(new URL("tools/mixer.html", site).href);
  await expect(page.locator("[data-demo-key^='track']")).toHaveCount(2);
  await expect(page.locator(".guide-animation svg [data-mixer-level]")).toHaveCount(2);
  await page.getByLabel("Illustration Input B volume").fill("25");
  await expect(page.locator(".guide-animation svg [data-mixer-level='b']")).toHaveAttribute("width", "42.5");
  await page.getByRole("button", { name: "Play example", exact: true }).click();
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "playing");
  await page.locator(".guide-animation").screenshot({ path: info.outputPath("mixer-two-tracks.png") });

  await page.goto(new URL("tools/meter.html", site).href);
  await expect(page.locator(".guide-animation svg [data-meter-rms]")).toHaveCount(2);
  await expect(page.locator(".guide-animation svg")).toContainText("RMS");
  await expect(page.locator(".guide-animation")).toContainText("current peak");
  await expect(page.locator(".guide-animation")).toContainText("peak hold");
  await expect(page.locator(".guide-animation svg")).toContainText("dBFS");
  await expect(page.locator(".guide-animation svg")).toContainText("CLIP > 0");
  await expect(page.locator("#see-it")).toContainText("synthetic");
  await page.locator(".guide-animation").screenshot({ path: info.outputPath("meter-stereo-scale.png") });

  await page.goto(new URL("tools/recorder.html", site).href);
  await page.getByRole("button", { name: "Record example", exact: true }).click();
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "playing");
  await expect(page.locator(".guide-animation svg")).toContainText("CAPTURING SYNTHETIC LOOP");
  await page.locator(".guide-animation").screenshot({ path: info.outputPath("recorder-recording.png") });
  await page.getByLabel("Seek example").fill("2");
  await page.getByRole("button", { name: "Stop recording", exact: true }).click();
  await expect(page.locator(".guide-animation svg")).toContainText("TAKE READY");
  await expect(page.getByRole("button", { name: "Play recording", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Play recording", exact: true }).click();
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "playing");
  await expect(page.locator(".guide-animation svg")).toContainText("PLAYING COMPLETED TAKE");
  const takeLength = Number(await page.getByLabel("Seek example").getAttribute("max"));
  expect(takeLength).toBeGreaterThanOrEqual(2);
  expect(takeLength).toBeLessThan(4);
  await expect(page.locator(".guide-animation svg [data-guide-playhead]")).toBeAttached();
  await page.getByRole("button", { name: "Pause recording", exact: true }).click();
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "paused");
  await page.getByRole("button", { name: "Play recording", exact: true }).click();
  await expect(page.locator("[data-guide-demo]")).toHaveAttribute("data-audio-state", "playing");
});
test("Network Send waits for a paired AudioRouter on PC B before packets cross the network", async ({ page }, info) => {
  await page.route("https://**", (route) => route.abort());
  await page.goto(new URL("tools/network-send.html", site).href);
  const diagram = page.locator(".guide-animation svg");
  await expect(diagram.locator("desc")).toContainText("second AudioRouter session on PC B");
  await expect(diagram.locator("desc")).toContainText("After pairing, audio packets cross the IP network");
  await expect(diagram).toContainText("PC A · THIS COMPUTER");
  await expect(diagram).toContainText("PC B · ANOTHER COMPUTER");

  const seek = page.getByLabel("Seek example");
  const waiting = diagram.locator('[data-network-stage="waiting"]');
  const paired = diagram.locator('[data-network-stage="paired"]');
  const firstPacket = diagram.locator("[data-network-packet]").first();
  const opacity = (locator: typeof waiting) => locator.evaluate((element) => Number(getComputedStyle(element).opacity));
  await seek.fill("0");
  expect(await opacity(waiting)).toBe(1);
  expect(await opacity(paired)).toBe(0);

  await seek.fill("3.2");
  expect(await opacity(waiting)).toBe(0);
  expect(await opacity(paired)).toBe(1);
  await page.locator(".guide-animation").screenshot({ path: info.outputPath("network-send-paired.png") });
  const before = await firstPacket.boundingBox();
  await seek.fill("4");
  const after = await firstPacket.boundingBox();
  expect(after!.x).toBeGreaterThan(before!.x + 30);
});
test("the guide index searches property names and links to individual pages", async ({ page }, info) => {
  await page.route("https://**", (route) => route.abort());
  await page.goto(new URL("tools/index.html", site).href);
  await expect(page.locator(".guide-index-card")).toHaveCount(35);
  await page.getByLabel("Find a tool or property").fill("threshold");
  await expect(page.locator(".guide-index-card:visible")).toHaveCount(4);
  await page.getByRole("link", { name: /Compressor.*Explore this tool/ }).click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Compressor");
  await page.screenshot({ path: info.outputPath("compressor-hero.png") });
});
