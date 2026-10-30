// Record the real React Properties panels. Requires the local Vite harness.
import { chromium } from "../../ui/node_modules/playwright-core/index.mjs";
import { readFileSync, mkdirSync, writeFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const manifest = JSON.parse(
  readFileSync(
    resolve(root, "tools/site/tool-guide-property-manifest.json"),
    "utf8",
  ),
);
const demo = process.env.GUIDE_DEMO === "1";
const renders = JSON.parse(
  readFileSync(
    resolve(root, "target/tool-guide-tools/render-report.json"),
    "utf8",
  ),
);
const out = resolve(
  root,
  demo ? "target/tool-guide-demos" : "target/tool-guide-overviews",
);
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
const report = manifest.tools
  .filter((t) => existsSync(resolve(out, t.kind, "overview.mp4")))
  .map((t) => ({
    kind: t.kind,
    status: demo ? "demo-recorded-unreviewed" : "overview-recorded",
  }));
for (const tool of manifest.tools.filter(
  (t) => !process.env.GUIDE_KIND || t.kind === process.env.GUIDE_KIND,
)) {
  const previousIndex = report.findIndex((r) => r.kind === tool.kind);
  if (previousIndex >= 0) report.splice(previousIndex, 1);
  const render = renders.find((r) => r.kind === tool.kind);
  if (
    demo &&
    (render?.status !== "audio-rendered-unreviewed" ||
      (!render.parameter && tool.kind !== "meter"))
  )
    continue;
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
  });
  const page = await context.newPage();
  const directory = resolve(out, tool.kind);
  mkdirSync(directory, { recursive: true });
  try {
    const parameters = demo
      ? JSON.parse(
          readFileSync(
            resolve(root, "target/tool-guide-tools", tool.kind, "before.json"),
            "utf8",
          ),
        )
      : Object.fromEntries(
          tool.properties
            .filter((p) => p.default !== null && !p.dynamic)
            .map((p) => [p.key, p.default]),
        );
    const telemetry = demo
      ? JSON.parse(
          readFileSync(
            resolve(
              root,
              "target/tool-guide-tools",
              tool.kind,
              "telemetry.json",
            ),
            "utf8",
          ),
        )
      : null;
    if (demo) Object.assign(parameters, telemetry.parametersBefore);
    const sourceKinds = [
      "physicalInput",
      "applicationCapture",
      "endpointLoopback",
      "virtualRenderSource",
      "testSignal",
      "audioFile",
      "networkReceive",
      "soundboard",
    ];
    const sinkKinds = ["physicalOutput", "virtualCaptureSink", "networkSend"];
    const ports = [];
    if (!sourceKinds.includes(tool.kind))
      ports.push({ name: "in", direction: "input", channels: 2 });
    if (!sinkKinds.includes(tool.kind))
      ports.push({ name: "out", direction: "output", channels: 2 });
    if (tool.kind === "inputSwitch") {
      ports[0].name = "a";
      ports.push({ name: "b", direction: "input", channels: 2 });
    }
    if (tool.kind === "outputSwitch") {
      ports[1].name = "a";
      ports.push({ name: "b", direction: "output", channels: 2 });
    }
    const node = {
      id: "guide",
      kind: tool.kind,
      typeVersion: 1,
      name: tool.title,
      enabled: true,
      bypass: false,
      parameters,
      ports,
    };
    const session = {
      id: "guide-session",
      name: "Properties guide",
      schemaVersion: 1,
      revision: 1,
      nodes:
        tool.kind === "group"
          ? [
              {
                ...node,
                kind: "gain",
                name: "Example signal",
                parameters: { gainDb: 0 },
              },
            ]
          : [node],
      edges: [],
    };
    if (demo) {
      const endpoint = (id, kind, direction) => ({
        id,
        kind,
        name: id === "input" ? "Synthetic input" : "Fixture output",
        typeVersion: 1,
        enabled: true,
        bypass: false,
        parameters: {},
        ports: [
          {
            name: direction === "input" ? "in" : "out",
            direction,
            channels: 2,
          },
        ],
      });
      const edge = (id, sourceNode, sourcePort, destinationNode) => ({
        id,
        sourceNode,
        sourcePort,
        destinationNode,
        destinationPort: "in",
        matrix: [1, 0, 0, 1],
        enabled: true,
      });
      if (!sourceKinds.includes(tool.kind)) {
        session.nodes.unshift(endpoint("input", "physicalInput", "output"));
        session.edges.push(edge("input-tool", "input", "out", "guide"));
      }
      if (tool.kind === "inputSwitch") {
        session.edges[0].destinationPort = "a";
        session.nodes.unshift(endpoint("input-b", "physicalInput", "output"));
        const second = edge("input-b-tool", "input-b", "out", "guide");
        second.destinationPort = "b";
        session.edges.push(second);
      }
      if (!sinkKinds.includes(tool.kind)) {
        session.nodes.push(endpoint("output", "physicalOutput", "input"));
        session.edges.push(
          edge(
            "tool-output",
            "guide",
            tool.kind === "outputSwitch" ? "a" : "out",
            "output",
          ),
        );
      }
      if (tool.kind === "outputSwitch") {
        session.nodes.push(endpoint("output-b", "physicalOutput", "input"));
        session.edges.push(edge("tool-output-b", "guide", "b", "output-b"));
      }
    }
    await page.addInitScript(
      ({ session, tool, telemetry }) => {
        localStorage.setItem("audiorouter.ui.theme", "dark");
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: !!telemetry,
          __routeFixturePrivacyMuted: false,
          __routeFixtureProcessors: [
            {
              id: tool.kind,
              version: 1,
              category: "native",
              availability: { status: "available" },
              latencySamples: 0,
              parameters: tool.properties
                .filter((p) => !p.dynamic)
                .map((p) => ({
                  name: p.key,
                  type: p.type,
                  minimum: p.minimum ?? undefined,
                  maximum: p.maximum ?? undefined,
                  default: p.default,
                  unit: p.unit ?? undefined,
                  enum: p.options ?? undefined,
                })),
            },
          ],
        });
        if (telemetry) {
          window.__guideClock = { start: performance.now() };
          setInterval(() => {
            const elapsed =
              (performance.now() - window.__guideClock.start) / 1000;
            const latest = (values) =>
              values.findLast((v) => v.atSeconds <= elapsed) ?? values[0];
            const m = latest(telemetry.meterSamples);
            window.__routeFixtureTelemetry = [
              {
                nodeId: "guide",
                kind: tool.kind,
                plugin: null,
                spectrum: latest(telemetry.spectrumSamples ?? []),
                processor: latest(telemetry.samples),
                meter: {
                  peakDb: Math.max(...m.peakDb),
                  rmsDb: Math.max(...m.rmsDb),
                  channelPeakDb: m.peakDb,
                  channelRmsDb: m.rmsDb,
                  clippedSamples: 0,
                  channelClippedSamples: [0, 0],
                },
              },
            ];
          }, 50);
        }
      },
      { session, tool, telemetry },
    );
    await page.goto("http://127.0.0.1:4186/route-harness.html");
    if (tool.kind !== "group")
      await page.getByRole("button", { name: "Fit View" }).click();
    if (tool.kind === "group")
      await page
        .getByRole("button", {
          name: "Group A named visual background; no audio routing",
        })
        .click();
    else
      await page
        .getByTestId("rf__node-guide")
        .locator(".flow-node-title")
        .click();
    if (demo && tool.kind === "testSignal")
      await page
        .getByRole("button", { name: "Play Test Signal", exact: true })
        .click();
    await page.locator(".inspector").waitFor({ state: "visible" });
    await page.addStyleTag({
      content: `
      .topbar,.workspace-notifications,.workspace-grid>.sidebar{display:none!important}
      .workspace-grid{display:block!important;width:100vw!important;height:100vh!important}
      .main-content{display:block!important;width:100vw!important;height:100vh!important;padding:0!important;margin:0!important}
      .main-content>:not(.inspector):not(.canvas-panel){display:none!important}
      .main-content>.canvas-panel{position:fixed!important;left:-20000px!important;top:0!important;width:800px!important;height:600px!important;visibility:hidden!important}
      .main-content>.inspector{display:block!important;width:100%!important;height:100%!important;max-height:none!important;overflow:auto!important;margin:0!important}
    `,
    });
    const inspector = page.locator(".inspector");
    await page.setViewportSize({ width: 480, height: 854 });
    await inspector.evaluate((e) => (e.scrollTop = 0));
    let control;
    if (demo && render.parameter) {
      const property = tool.properties.find(
        (p) => p.key === render.parameter,
      ) ?? { type: "number", label: "Volume for Synthetic input" };
      if (tool.kind === "graphicEq")
        await inspector.getByRole("slider").first().focus();
      const label =
        tool.kind === "parametricEq" ? "EQ frequency Hz" : property.label;
      control = ["inputSwitch", "outputSwitch"].includes(tool.kind)
        ? inspector.getByRole("radio").nth(1)
        : tool.kind === "soundboard"
          ? inspector.getByRole("slider").first()
          : tool.kind === "mixer"
            ? inspector.getByRole("slider", {
                name: "Volume for Synthetic input",
              })
            : inspector
                .getByRole(
                  property.type === "boolean"
                    ? "checkbox"
                    : property.type === "string"
                      ? "combobox"
                      : "spinbutton",
                  {
                    name: new RegExp(
                      label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
                      "i",
                    ),
                  },
                )
                .first();
      await control.waitFor({ state: "attached", timeout: 2000 });
    }
    await page.screenshot({ path: resolve(directory, "poster.png") });
    const cdp = await context.newCDPSession(page);
    let latestFrame;
    cdp.on("Page.screencastFrame", (frame) => {
      latestFrame = frame.data;
      void cdp
        .send("Page.screencastFrameAck", { sessionId: frame.sessionId })
        .catch(() => {});
    });
    await cdp.send("Page.startScreencast", {
      format: "jpeg",
      quality: 85,
      everyNthFrame: 1,
    });
    const deadline = performance.now() + 5000;
    while (!latestFrame && performance.now() < deadline)
      await new Promise((r) => setTimeout(r, 20));
    if (!latestFrame) throw new Error("Edge did not supply a video frame");
    const captured = [];
    // Silent configuration overview: browse the actual fields, then return to
    // the top. Audio/parameter demonstrations are separate recordings.
    const start = performance.now();
    if (demo)
      await page.evaluate(() => {
        window.__guideClock.start = performance.now();
      });
    for (let frame = 0; frame < 120; frame++) {
      const time = frame / 15;
      if (demo) {
        if (frame === 40 && control) await control.scrollIntoViewIfNeeded();
        if (frame === 48 && control) {
          const after = JSON.parse(
            readFileSync(
              resolve(root, "target/tool-guide-tools", tool.kind, "after.json"),
              "utf8",
            ),
          );
          if (typeof after[render.parameter] === "boolean")
            await control.setChecked(after[render.parameter]);
          else if (["inputSwitch", "outputSwitch"].includes(tool.kind))
            await control.click();
          else
            await control.evaluate((element, value) => {
              const setter = Object.getOwnPropertyDescriptor(
                HTMLInputElement.prototype,
                "value",
              ).set;
              setter.call(element, String(value));
              element.dispatchEvent(new Event("input", { bubbles: true }));
            }, after[render.parameter]);
          await control.evaluate((element) => element.blur());
        }
        if (frame === 60) await inspector.evaluate((e) => (e.scrollTop = 0));
      } else
        await inspector.evaluate((e, t) => {
          const distance = Math.max(0, e.scrollHeight - e.clientHeight);
          const position =
            t < 1 ? 0 : t < 4 ? (t - 1) / 3 : t < 5 ? 1 - (t - 4) : 0;
          e.scrollTop = distance * position;
        }, time);
      captured.push(latestFrame);
      const delay = start + ((frame + 1) * 1000) / 15 - performance.now();
      if (delay > 0) await new Promise((r) => setTimeout(r, delay));
    }
    await cdp.send("Page.stopScreencast");
    await cdp.detach();
    for (let frame = 0; frame < captured.length; frame++)
      writeFileSync(
        resolve(directory, `frame-${String(frame).padStart(4, "0")}.jpg`),
        Buffer.from(captured[frame], "base64"),
      );
    const audio = demo
      ? [
          "-f",
          "f32le",
          "-ar",
          "48000",
          "-ac",
          "2",
          "-i",
          resolve(root, "target/tool-guide-tools", tool.kind, "output.f32le"),
          "-c:a",
          "aac",
          "-b:a",
          "96k",
        ]
      : [];
    const result = spawnSync(
      "ffmpeg",
      [
        "-y",
        "-framerate",
        "15",
        "-i",
        resolve(directory, "frame-%04d.jpg"),
        ...audio,
        "-vf",
        "scale=720:1280",
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "24",
        "-pix_fmt",
        "yuv420p",
        "-movflags",
        "+faststart",
        resolve(directory, "overview.mp4"),
      ],
      { encoding: "utf8" },
    );
    if (result.status !== 0 || !existsSync(resolve(directory, "overview.mp4")))
      throw new Error(result.stderr);
    report.push({
      kind: tool.kind,
      status: demo ? "demo-recorded-unreviewed" : "overview-recorded",
      parameter: demo ? render.parameter : null,
      scope: demo
        ? "Production DSP fixture with the selected parameter changed at 3.2 seconds; requires review."
        : "Silent Properties configuration overview; no DSP or live behavior claim.",
    });
    console.log(`${tool.kind}: overview recorded`);
  } catch (error) {
    await page
      .screenshot({ path: resolve(directory, "failure.png") })
      .catch(() => {});
    report.push({ kind: tool.kind, status: "failed", error: String(error) });
    console.error(`${tool.kind}: ${error}`);
  } finally {
    await context.close();
  }
  writeFileSync(resolve(out, "report.json"), JSON.stringify(report, null, 2));
}
await browser.close();
if (report.some((r) => r.status === "failed")) process.exitCode = 1;
