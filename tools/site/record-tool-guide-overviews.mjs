// Record the real React Properties panels. Requires the local Vite harness.
import { chromium } from "../../ui/node_modules/playwright-core/index.mjs";
import { readFileSync, mkdirSync, writeFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import ts from "../../ui/node_modules/typescript/lib/typescript.js";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const manifest = JSON.parse(
  readFileSync(
    resolve(root, "tools/site/tool-guide-property-manifest.json"),
    "utf8",
  ),
);
const properties = process.env.GUIDE_PROPERTIES === "1";
const publicCatalog = JSON.parse(readFileSync(resolve(root, 'tools/site/tool-guide-catalog.json'), 'utf8'));
const config = process.env.GUIDE_CONFIG;
if (config && (!process.env.GUIDE_KIND || properties)) throw new Error('Select exactly one tool for a configuration capture');
const demo = !config && (properties || process.env.GUIDE_DEMO === "1");
const parameterLabels = await import(
  "data:text/javascript;base64," +
    Buffer.from(
      ts.transpileModule(
        readFileSync(resolve(root, "ui/src/parameterText.ts"), "utf8"),
        { compilerOptions: { module: ts.ModuleKind.ESNext } },
      ).outputText,
    ).toString("base64")
);
const voicePresets = publicCatalog.voicePresets;
const renders = JSON.parse(
  readFileSync(
    resolve(
      root,
      properties
        ? "target/tool-guide-properties/render-report.json"
        : "target/tool-guide-tools/render-report.json",
    ),
    "utf8",
  ),
);
const out = resolve(
  root,
  config ? 'target/tool-guide-configs' : properties
    ? "target/tool-guide-property-recordings"
    : demo
      ? "target/tool-guide-demos"
      : "target/tool-guide-overviews",
);
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
const report = properties
  ? renders
      .filter((r) => existsSync(resolve(out, r.id, "overview.mp4")))
      .map((r) => ({
        id: r.id,
        kind: r.kind,
        status: "demo-recorded-unreviewed",
      }))
  : manifest.tools
      .filter((t) => existsSync(resolve(out, t.kind, "overview.mp4")))
      .map((t) => ({
        kind: t.kind,
        status: demo ? "demo-recorded-unreviewed" : "overview-recorded",
      }));
const targets = properties
  ? renders
      .filter((r) => r.status === "audio-rendered-unreviewed")
      .map((render) => ({
        tool: manifest.tools.find((t) => t.kind === render.kind),
        render,
      }))
  : manifest.tools.map((tool) => ({
      tool,
      render: renders.find((r) => r.kind === tool.kind),
    }));
for (const { tool, render } of targets.filter(
  ({ tool, render }) =>
    (!process.env.GUIDE_KIND ||
      process.env.GUIDE_KIND.split(",").includes(tool.kind)) &&
    (!process.env.GUIDE_PROPERTY ||
      render.parameter === process.env.GUIDE_PROPERTY),
)) {
  const clipId = config ? `${tool.kind}--${config}` : properties ? render.id : tool.kind;
  const previousIndex = report.findIndex((r) =>
    properties ? r.id === clipId : r.kind === tool.kind,
  );
  if (previousIndex >= 0) report.splice(previousIndex, 1);
  if (
    demo &&
    (render?.status !== "audio-rendered-unreviewed" ||
      (!render.parameter && tool.kind !== "meter"))
  )
    continue;
  const audible = demo && render.audible !== false;
  const wireType = tool.kind.replace(/[A-Z]/g, letter => '-' + letter.toLowerCase()) + '@1';
  const parameterSchema = publicCatalog.nodeTypes.find(node => node.type === wireType)?.parameters ?? [];
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
  });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  const directory = resolve(out, clipId);
  const renderDirectory = properties
    ? render.directory
    : resolve(root, "target/tool-guide-tools", tool.kind);
  mkdirSync(directory, { recursive: true });
  writeFileSync(
    resolve(directory, "capture.json"),
    JSON.stringify({ id: clipId, status: "recording" }),
  );
  try {
    const parameters = demo
      ? JSON.parse(
          readFileSync(resolve(renderDirectory, "before.json"), "utf8"),
        )
      : Object.fromEntries(
          tool.properties
            .filter((p) => p.default !== null && !p.dynamic)
            .map((p) => [p.key, p.default]),
        );
    const telemetry = demo
      ? JSON.parse(
          readFileSync(resolve(renderDirectory, "telemetry.json"), "utf8"),
        )
      : null;
    if (demo) Object.assign(parameters, telemetry.parametersBefore);
    if (config && tool.kind === 'duck') { parameters.trigger = config === 'game' ? 'siegeRound' : 'level'; parameters.keyNodeId = 'input'; }
    if (!demo && tool.kind === 'soundboard') for (let slot=1;slot<=9;slot++) { parameters[`slot${slot}MediaId`]='lib:guide-fixture'; parameters[`slot${slot}FileName`]='Prepared synthetic tone'; parameters[`slot${slot}Caption`]=`Tone ${slot}`; }
    const sourceHash = demo
      ? createHash("sha256")
          .update(readFileSync(resolve(renderDirectory, "telemetry.json")))
          .digest("hex")
      : null;
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
    if (demo || config) {
      const endpoint = (id, kind, direction) => ({
        id,
        kind,
          name: id === "input" ? "Synthetic input" : id === 'input-b' ? 'High tone input' : id === 'output-b' ? 'Output B' : 'Output A',
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
      ({ session, tool, telemetry, voicePresets, audible, parameterSchema }) => {
        localStorage.setItem("audiorouter.ui.theme", "dark");
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: audible,
          __routeFixturePrivacyMuted: false,
          __routeFixtureProcessors: [
            {
              id: tool.kind,
              version: 1,
              category: "native",
              availability: { status: "available" },
              latencySamples: 0,
              parameters: parameterSchema,
            },
          ],
          ...(voicePresets.length
            ? {
                __routeFixtureDiscovery: {
                  presets: { voiceChanger: voicePresets },
                },
              }
            : {}),
          ...(tool.kind === "soundboard" && telemetry
            ? {
                __routeFixtureSoundboardStatus: {
                  sessionId: session.id,
                  nodeId: "guide",
                  slots: [],
                  ignoredTriggers: 0,
                  droppedCommands: 0,
                },
              }
            : {}),
        });
        if (telemetry) {
          window.__guideClock = { start: performance.now() };
          setInterval(() => {
            const elapsed =
              (performance.now() - window.__guideClock.start) / 1000;
            const latest = (values) =>
              values.findLast((v) => v.atSeconds <= elapsed) ?? values[0];
            const m = latest(telemetry.meterSamples);
            const board = latest(telemetry.soundboardSamples ?? []);
            if (board)
              window.__routeFixtureSoundboardStatus = {
                ...board,
                sessionId: session.id,
                nodeId: "guide",
              };
            window.__routeFixtureTelemetry = [
              {
                nodeId: "guide",
                kind: tool.kind,
                plugin: null,
                voiceChanger: latest(telemetry.voiceSamples ?? []),
              noiseProfile: latest(telemetry.noiseProfiles ?? [])?.noiseProfile ?? null,
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
      { session, tool, telemetry, voicePresets, audible, parameterSchema },
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
    if (demo && tool.kind === 'audioFile') await page.locator('.inspector').getByRole('button', { name: 'Play', exact: true }).click();
    if (tool.kind === "applicationCapture" && !demo) {
      await page.setViewportSize({ width: 480, height: 854 });
      await page.waitForTimeout(500);
      await page.getByRole("tab", { name: "Tools", exact: true }).click();
    }
    await page.addStyleTag({
      content: tool.kind === "applicationCapture" ? `
      .topbar,.canvas-panel,.workspace-notifications{display:none!important}
      .right-workbench{position:fixed!important;inset:0!important;width:100vw!important;height:100vh!important;z-index:100!important;overflow:auto!important}
      .dialog-backdrop{z-index:9999!important}.connection-dialog{width:100%!important;max-width:100%!important;max-height:100vh!important;overflow:auto!important}
      ` : `
      .topbar,.workspace-notifications,.workspace-grid>.sidebar{display:none!important}
      .workspace-grid{display:block!important;width:100vw!important;height:100vh!important}
      .main-content{display:block!important;width:100vw!important;height:100vh!important;padding:0!important;margin:0!important}
      .main-content>:not(.inspector):not(.canvas-panel){display:none!important}
      .main-content>.canvas-panel{position:fixed!important;left:-20000px!important;top:0!important;width:800px!important;height:600px!important;visibility:hidden!important}
      .main-content>.inspector{display:block!important;width:100%!important;height:100%!important;max-height:none!important;overflow:auto!important;margin:0!important}
      .dialog-backdrop{z-index:9999!important}.dialog-backdrop .connection-dialog{width:100%!important;max-width:100%!important;max-height:100vh!important;overflow:auto!important;margin:0!important}
    `,
    });
    const inspector = page.locator(".inspector");
    await page.setViewportSize({ width: 480, height: 854 });
    if (tool.kind === "applicationCapture" && !demo) {
      await page.addStyleTag({ content: ".main-content>.right-workbench{display:block!important;width:100%!important;height:100vh!important;overflow:auto!important}.right-workbench .workbench-page{display:block!important}" });
      await page.getByRole("button", { name: /Choose an application source/ }).scrollIntoViewIfNeeded();
    }
    await inspector.evaluate((e) => (e.scrollTop = 0));
    if (properties)
      await inspector.locator("details").evaluateAll((elements) => {
        for (const element of elements) element.open = true;
      });
    let control;
    if (demo && render.parameter) {
      const property = tool.properties.find(
        (p) => p.key === render.parameter,
      ) ?? { type: "number", label: "Volume for Synthetic input" };
      if (tool.kind === "graphicEq")
        await inspector.getByRole("slider").nth(Number(render.parameter.match(/band(\d+)/)?.[1] ?? 0)).focus();
      const eqLabels = {
        band0FrequencyHz: "EQ frequency Hz",
        band0GainDb: "EQ gain dB",
        band0Q: "EQ Q width",
        band0Type: "EQ filter type",
      };
      const label =
        tool.kind === "parametricEq"
          ? eqLabels[render.parameter]
          : properties
            ? parameterLabels.parameterText(tool.kind, render.parameter).label
            : property.label;
      if (properties && render.parameter === "band0Enabled")
        control = inspector.getByRole("button", {
          name: "Remove point",
          exact: true,
        });
      else if (properties && render.parameter === "learning")
        control = inspector
          .getByRole("button", { name: /Learn noise|Learn again/ })
          .first();
      else if (properties && tool.kind === "soundboard")
        control = inspector
          .getByRole(
            render.parameter === "slot1Caption" ? "textbox" : "combobox",
            {
              name:
                render.parameter === "slot1Caption"
                  ? /Caption/
                  : /When pressed/,
            },
          )
          .first();
      else if (properties && render.parameter === "fade")
        control = inspector.getByRole("button", {
          name: "Slow fade (2 s)",
          exact: true,
        });
      else if (properties && render.parameter === "preset")
        control = inspector.getByRole("combobox", {
          name: "Voice preset",
          exact: true,
        });
      else
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
    let eventAtSeconds = null;
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
            readFileSync(resolve(renderDirectory, "after.json"), "utf8"),
          );
          const actionButton =
            properties &&
            ["band0Enabled", "learning", "fade"].includes(render.parameter);
          if (actionButton) {
            await control.click({ force: true });
            if (render.parameter === 'fade') await inspector.getByRole('radio').nth(1).evaluate(element => element.click());
          }
          else if (typeof after[render.parameter] === "boolean")
            await control.setChecked(after[render.parameter]);
          else if (
            await control.evaluate((element) => element.tagName === "SELECT")
          )
            await control.selectOption(String(after[render.parameter]));
          else if (properties && render.parameter === "slot1Caption")
            await control.fill(String(after[render.parameter]));
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
          if (!actionButton)
            await control.evaluate((element) => element.blur());
          eventAtSeconds = (performance.now() - start) / 1000;
          if (Math.abs(eventAtSeconds - 3.2) > 1 / 15)
            throw new Error(
              `Parameter action missed its video frame: ${eventAtSeconds}`,
            );
        }
        if (frame === 60) await inspector.evaluate((e) => (e.scrollTop = 0));
      } else if (tool.kind === "applicationCapture") {
        if (frame === 22) {
          await page.getByRole("button", { name: /Choose an application source/ }).click();
          const dialog = page.getByRole("dialog", { name: "Add an application capture source", includeHidden: true });
          await dialog.waitFor({ state: "attached" });
          await dialog.evaluate(element => document.body.appendChild(element.closest(".dialog-backdrop")));
        }
        if (frame === 65) await page.getByRole("button", { name: "Add running app", exact: true }).scrollIntoViewIfNeeded();
        if (frame === 90) await page.getByRole("button", { name: "Add running app", exact: true }).click();
      } else if (tool.kind === "physicalInput") {
        const capture = inspector.getByLabel("Physical input endpoint", { exact: true });
        if (frame === 8) { await capture.scrollIntoViewIfNeeded(); await capture.click(); await page.keyboard.press("Alt+ArrowDown"); }
        if (frame === 30) { await page.keyboard.press("Escape"); await capture.selectOption("capture-preview"); }
        if (frame === 40) { const channels = inspector.getByLabel("Input channels", { exact: true }); await channels.scrollIntoViewIfNeeded(); await channels.selectOption("mono"); }
        if (frame === 65) { const spatial = inspector.getByLabel("Spatial audio mode", { exact: true }); await spatial.scrollIntoViewIfNeeded(); await spatial.selectOption("headphones"); }
        if (frame === 80) { await capture.selectOption("surround-preview"); }
        if (frame === 100) await inspector.getByLabel("Spatial audio settings", { exact: true }).scrollIntoViewIfNeeded();
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
    if (pageErrors.length)
      throw new Error(`Uncaught Properties error: ${pageErrors.join("; ")}`);
    if (
      demo &&
      sourceHash !==
        createHash("sha256")
          .update(readFileSync(resolve(renderDirectory, "telemetry.json")))
          .digest("hex")
    )
      throw new Error("Render changed during capture");
    for (let frame = 0; frame < captured.length; frame++)
      writeFileSync(
        resolve(directory, `frame-${String(frame).padStart(4, "0")}.jpg`),
        Buffer.from(captured[frame], "base64"),
      );
    const audio = audible
      ? [
          "-f",
          "f32le",
          "-ar",
          "48000",
          "-ac",
          "2",
          "-i",
          resolve(renderDirectory, "output.f32le"),
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
      id: clipId,
      audible,
      kind: tool.kind,
      status: demo ? "demo-recorded-unreviewed" : "overview-recorded",
      parameter: demo ? render.parameter : null,
      scope: demo
        ? "Production DSP fixture with the selected parameter changed at 3.2 seconds; requires review."
        : "Silent Properties configuration overview; no DSP or live behavior claim.",
    });
    writeFileSync(
      resolve(directory, "capture.json"),
      JSON.stringify(
        {
          id: clipId,
          status: "complete",
          kind: tool.kind,
          parameter: render?.parameter ?? null,
          sourceHash,
          eventAtSeconds,
          frameRate: 15,
          frames: 120,
          pageErrors,
        },
        null,
        2,
      ),
    );
    console.log(`${clipId}: recorded`);
  } catch (error) {
    await page
      .screenshot({ path: resolve(directory, "failure.png") })
      .catch(() => {});
    report.push({
      id: clipId,
      kind: tool.kind,
      status: "failed",
      error: String(error),
    });
    writeFileSync(
      resolve(directory, "capture.json"),
      JSON.stringify(
        { id: clipId, status: "failed", error: String(error) },
        null,
        2,
      ),
    );
    console.error(`${tool.kind}: ${error}`);
  } finally {
    await context.close();
  }
  writeFileSync(resolve(out, "report.json"), JSON.stringify(report, null, 2));
}
await browser.close();
if (report.some((r) => r.status === "failed")) process.exitCode = 1;
