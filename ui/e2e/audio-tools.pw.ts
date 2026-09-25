import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => { await page.goto("/harness.html"); await expect(page.getByLabel("Signal-flow graph")).toBeVisible(); });

test("every supported source, processor, and destination can be added to the live graph UI", async ({ page }) => {
  const labels = ["Test Signal", "Audio file", "Input device", "Output device", "Gain", "Mixer", "Recorder", "Mute", "Meter", "Advanced EQ", "Compressor", "Gate", "Limiter", "Delay", "Graphic EQ", "Pitch shift"];
  for (const label of labels) await page.locator(".canvas-library button").filter({ hasText: new RegExp(label, "i") }).first().click();
  for (const label of labels.map((item) => item === "Input device" ? "Physical input" : item === "Output device" ? "Physical output" : item)) await expect(page.locator(".react-flow__node").filter({ hasText: label }).first(), `node ${label}`).toBeVisible();
});

test("a complex multi-source processing set preserves node types and supports modifier edits", async ({ page }) => {
  for (const label of ["Input device", "Input device", "Mixer", "Gain", "Advanced EQ", "Compressor", "Limiter", "Meter", "Mute", "Output device", "Recorder"]) {
    await page.locator(".canvas-library button").filter({ hasText: new RegExp(label, "i") }).first().click();
  }
  await page.addStyleTag({ content: ".canvas-library { display: none !important; }" });
  await expect(page.locator(".react-flow__node")).toHaveCount(14);
  for (const existing of ["Microphone", "Voice gain", "Headphones"]) await expect(page.locator(".react-flow__node").filter({ hasText: existing }).first()).toBeAttached();
  await expect(page.getByText(/visual check only/)).toBeVisible();
  const node = (name: string) => page.getByText(name, { exact: true }).locator("xpath=ancestor::div[contains(@class,'react-flow__node')][1]");
  for (const label of ["Physical input 1", "Physical input 2", "Mixer 1", "Gain 1", "Advanced EQ 1", "Compressor 1", "Limiter 1", "Meter 1", "Physical output 1", "Recorder 1"]) {
    await expect(page.locator(".react-flow__node").filter({ hasText: label }).first()).toBeAttached();
  }
  const addedMute = node("Mute");
  const mute = addedMute.getByRole("button", { name: /live, click to mute/i });
  await mute.click();
  await expect(addedMute.getByRole("button", { name: /muted, click to unmute/i })).toHaveAttribute("aria-pressed", "true");
});

test("a source can be connected to a destination by dragging output to input", async ({ page }) => {
  for (const label of ["Test Signal", "Output device"]) await page.locator(".canvas-library button").filter({ hasText: new RegExp(label, "i") }).first().click();
  await page.addStyleTag({ content: ".canvas-library { display: none !important; }" });
  const sourceNode = page.locator(".react-flow__node").filter({ hasText: "Test Signal" }).last();
  const targetNode = page.locator(".react-flow__node").filter({ hasText: "Physical output" }).last();
  const canvas = await page.locator(".session-flow-canvas").boundingBox();
  if (!canvas) throw new Error("Signal-flow canvas is not visible");
  const moveNode = async (target: typeof sourceNode, x: number, y: number) => {
    const box = await target.locator(".flow-node-title").boundingBox();
    if (!box) throw new Error("Graph node is not visible");
    await page.mouse.move(box.x + Math.min(box.width / 2, 80), box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(x, y, { steps: 12 });
    await page.mouse.up();
  };
  await moveNode(page.locator(".react-flow__node").filter({ hasText: "Microphone" }).first(), canvas.x + 140, canvas.y + 130);
  await moveNode(page.locator(".react-flow__node").filter({ hasText: "Voice gain" }).first(), canvas.x + canvas.width / 2, canvas.y + 130);
  await moveNode(page.locator(".react-flow__node").filter({ hasText: "Headphones" }).first(), canvas.x + canvas.width - 140, canvas.y + 130);
  await moveNode(sourceNode, canvas.x + 200, canvas.y + canvas.height * .72);
  await moveNode(targetNode, canvas.x + canvas.width - 200, canvas.y + canvas.height * .72);
  await expect(page.locator(".react-flow__edge")).toHaveCount(2);
  const before = await page.locator(".react-flow__edge").count();
  await sourceNode.locator('.react-flow__handle.source[data-debug-side="right"]').dragTo(targetNode.locator('.react-flow__handle.target[data-debug-side="left"]'));
  await expect(page.locator(".react-flow__edge")).toHaveCount(before + 1, { timeout: 3000 });
});

test("a second source to a physical output gets an undoable visible mixer", async ({ page }) => {
  await page.goto("/route-harness.html");
  await page.getByRole("tab", { name: "Tools" }).click();
  await page.locator(".tool-card").filter({ hasText: "Input device" }).first().click();
  await page.locator(".tool-card").filter({ hasText: "Test Signal" }).first().click();
  const editor = page.locator(".workbench-connection-editor");
  await page.getByRole("tab", { name: "Setup" }).click();
  await editor.getByRole("combobox", { name: "Source output port" }).selectOption({ label: "Physical input 1 · out · 2ch" });
  await editor.getByRole("combobox", { name: "Destination input port" }).selectOption({ label: "Headphones · in · 2ch" });
  await editor.getByRole("button", { name: "Add connection" }).click();
  await editor.getByRole("combobox", { name: "Source output port" }).selectOption({ label: "Test Signal 1 · out · 2ch" });
  await editor.getByRole("button", { name: "Add connection" }).click();
  await expect(page.locator(".workbench-action-message")).toContainText("Added a Mixer so Physical input 1 and Test Signal 1 can share Headphones");
  await expect(page.locator(".react-flow__node").filter({ hasText: "Mixer" })).toBeVisible();
  await expect(page.locator(".react-flow__edge")).toHaveCount(3);
  await page.getByRole("tab", { name: "Session" }).click();
  await page.getByRole("button", { name: "Undo" }).click();
  await expect(page.locator(".workbench-action-message")).toContainText("Undid the last draft change");
  await expect(page.locator(".react-flow__node").filter({ hasText: "Mixer" })).toHaveCount(0);
});

test("Properties keeps selected node controls in a full-height sidebar", async ({ page }) => {
  await page.goto("/route-harness.html");
  const canvasBefore = await page.locator("#signal-flow-panel").boundingBox();
  const sidebarBefore = await page.locator(".right-workbench").boundingBox();
  await page.getByRole("tab", { name: "Properties" }).click();
  await page.locator(".react-flow__node").filter({ hasText: "Headphones" }).click();
  const sidebar = page.locator(".right-workbench");
  const inspector = page.locator(".main-content > .inspector");
  await expect(inspector.getByRole("heading", { name: "Headphones" })).toBeVisible();
  const bounds = await Promise.all([sidebar.boundingBox(), inspector.boundingBox()]);
  if (!bounds[0] || !bounds[1]) throw new Error("Properties sidebar has no measurable layout");
  expect(bounds[1].height).toBeGreaterThan(bounds[0].height * 0.55);
  expect(Math.abs(bounds[0].height - sidebarBefore!.height)).toBeLessThan(3);
  expect(Math.abs((await page.locator("#signal-flow-panel").boundingBox())!.height - canvasBefore!.height)).toBeLessThan(3);
});

test("a complex multi-source set can be added and its mute control responds", async ({ page }) => {
  const kinds = ["Input device", "Input device", "Mixer", "Gain", "Advanced EQ", "Compressor", "Limiter", "Meter", "Mute", "Output device", "Recorder"];
  for (const label of kinds) await page.locator(".canvas-library button").filter({ hasText: new RegExp(label, "i") }).first().click();
  await page.addStyleTag({ content: ".canvas-library { display: none !important; }" });
  const node = (label: string, index = -1) => {
    const matches = page.locator(".react-flow__node").filter({ hasText: label });
    return index < 0 ? matches.last() : matches.nth(index);
  };
  const connect = async (source: ReturnType<typeof node>, target: ReturnType<typeof node>, inputIndex = 0) => {
    const outputs = source.locator('.react-flow__handle.source[data-debug-side="right"]');
    const inputs = target.locator('.react-flow__handle.target[data-debug-side="left"]');
    const count = await inputs.count();
    const before = await page.locator(".react-flow__edge").count();
    await outputs.last().dragTo(inputs.nth(Math.min(inputIndex, count - 1)));
    await expect(page.locator(".react-flow__edge"), `${await source.innerText()} → ${await target.innerText()}`).toHaveCount(before + 1, { timeout: 1500 });
  };
  const moveNode = async (target: ReturnType<typeof node>, x: number, y: number) => {
    const box = await target.boundingBox();
    if (!box) throw new Error("Graph node is not visible");
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(x, y, { steps: 12 });
    await page.mouse.up();
  };
  const initialEdges = await page.locator(".react-flow__edge").count();
  const source = node("Physical input 1");
  const sourceTwo = node("Physical input 2");
  const mixer = node("Mixer 1");
  const chain = [mixer, node("Gain 1"), node("Advanced EQ 1"), node("Compressor 1"), node("Limiter 1")];
  const muteNode = page.getByTestId("rf__node-mute-1");
  const meterNode = node("Meter 1");
  const outputNode = node("Physical output 1");
  const recorderNode = node("Recorder 1");
  await expect(page.getByText("No inputs connected")).toBeVisible();
  const mute = muteNode.getByRole("button", { name: /live, click to mute/i });
  await mute.click();
  await expect(muteNode.getByRole("button", { name: /muted, click to unmute/i })).toHaveAttribute("aria-pressed", "true");
});

test("every built-in modifier exposes working enable and bypass draft controls", async ({ page }) => {
  await page.goto("/route-harness.html");
  const modifiers: [string, string][] = [["Gain", "gain"], ["Mute", "mute"], ["Meter", "meter"], ["Advanced EQ", "parametricEq"], ["Compressor", "compressor"], ["Gate", "gate"], ["Limiter", "limiter"], ["Delay", "delay"], ["Graphic EQ", "graphicEq"], ["Pitch shift", "pitch"]];
  for (const [label] of modifiers) {
    await page.getByRole("tab", { name: "Tools" }).click();
    await page.locator(".tool-card").filter({ hasText: label }).first().click();
  }
  const inspector = page.locator(".main-content > .inspector");
  for (const [, kind] of modifiers) {
    const node = page.getByTestId(`rf__node-${kind}-1`);
    await node.click();
    await page.getByRole("tab", { name: "Properties" }).click();
    await expect(page.getByRole("tab", { name: "Properties" })).toHaveAttribute("aria-selected", "true");
    const enabled = inspector.getByLabel("Enabled");
    await enabled.uncheck();
    await expect(enabled).not.toBeChecked();
    await enabled.check();
    await expect(enabled).toBeChecked();
    const bypass = inspector.getByLabel("Bypass");
    await bypass.check();
    await expect(bypass).toBeChecked();
    await bypass.uncheck();
    await expect(bypass).not.toBeChecked();
  }
});

test("adding tools stays on Tools, and Delete can be undone before saving", async ({ page }) => {
  await page.goto("/route-harness.html");
  const tools = page.getByRole("tab", { name: "Tools" });
  await tools.click();
  await page.locator(".tool-card").filter({ hasText: "Gain" }).first().click();
  await expect(tools).toHaveAttribute("aria-selected", "true");
  await page.locator(".tool-card").filter({ hasText: "Mute" }).first().click();
  await expect(tools).toHaveAttribute("aria-selected", "true");
  const gain = page.getByTestId("rf__node-gain-1");
  await gain.click();
  await gain.press("Delete");
  await expect(gain).toHaveCount(0);
  await page.getByRole("tab", { name: "Session" }).click();
  await page.locator(".right-workbench").getByRole("button", { name: "Undo", exact: true }).click();
  await expect(page.getByTestId("rf__node-gain-1")).toBeVisible();
});

test("Audio File imports WAV media through the upload contract and exposes loop and transport state", async ({ page }) => {
  await page.goto("/route-harness.html");
  await page.getByRole("tab", { name: "Tools" }).click();
  await page.locator(".tool-card").filter({ hasText: "Audio file" }).first().click();
  const node = page.getByTestId("rf__node-audioFile-1");
  await node.click();
  await page.getByRole("tab", { name: "Properties" }).click();
  const inspector = page.locator(".main-content > .inspector");
  const filePicker = inspector.getByLabel("Choose WAV or MP3");
  await filePicker.setInputFiles({ name: "calibration.wav", mimeType: "audio/wav", buffer: Buffer.from([82, 73, 70, 70]) });
  await expect(inspector.getByText(/calibration\.wav · 1 sec · 1 channel/)).toBeVisible();
  await expect(inspector.getByRole("button", { name: "Play", exact: true })).toBeEnabled();
  await expect(inspector.getByRole("button", { name: "Stop", exact: true })).toBeDisabled();
  await inspector.getByLabel("Loop").check();
  await expect(inspector.getByLabel("Loop")).toBeChecked();
  await page.getByRole("tab", { name: "Tools" }).click();
  await page.locator(".tool-card").filter({ hasText: "Output device" }).first().click();
  await page.getByRole("tab", { name: "Setup" }).click();
  const setup = page.locator(".right-workbench");
  await setup.getByLabel("Source output port").selectOption({ label: "Audio file 1 · out · 2ch" });
  await setup.getByLabel("Destination input port").selectOption({ label: "Physical output 1 · in · 2ch" });
  await setup.getByRole("button", { name: "Add connection", exact: true }).click();
  await page.getByRole("tab", { name: "Session" }).click();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText(/Route saved \(revision 8\)/);
  // A filename-only update used to erase mediaId, leaving this disabled forever.
  await expect(node.getByRole("button", { name: /Play audio file/i })).toBeEnabled();
});

test("Advanced EQ edits filter points in properties", async ({ page }) => {
  await page.goto("/route-harness.html");
  await page.getByRole("tab", { name: "Tools" }).click();
  await page.locator(".tool-card").filter({ has: page.locator("strong", { hasText: /^Advanced EQ$/ }) }).click();
  await page.getByRole("tab", { name: "Properties" }).click();
  const editor = page.getByRole("region", { name: "Advanced EQ editor" });
  await expect(editor.getByText("0 of 16 points active")).toBeVisible();
  await editor.getByRole("button", { name: "Add point" }).click();
  await expect(editor.getByText("1 of 16 points active")).toBeVisible();
  await editor.getByLabel("EQ filter type").selectOption("notch");
  await editor.getByLabel("EQ frequency Hz").fill("120");
  await expect(editor.getByLabel("EQ frequency Hz")).toHaveValue("120");
  await expect(editor.locator(".advanced-eq-curve")).toBeVisible();
  await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/advanced-eq.png`, fullPage: false });
  const point = editor.locator(".advanced-eq-point circle").first();
  const bounds = await point.boundingBox();
  if (!bounds) throw new Error("EQ point is not visible for dragging");
  const centerX = bounds.x + bounds.width / 2;
  const centerY = bounds.y + bounds.height / 2;
  await page.mouse.move(centerX, centerY);
  await page.mouse.down();
  await page.mouse.move(centerX + 36, centerY, { steps: 5 });
  await page.mouse.up();
  await expect(editor.getByLabel("EQ frequency Hz")).not.toHaveValue("120");
  await editor.getByRole("button", { name: "Remove point" }).click();
  await expect(editor.getByText("0 of 16 points active")).toBeVisible();
});

test("the workspace plans and commits a connected multi-input processing route", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.locator(".audio-run-state")).toContainText("Audio stopped");
  const addTool = async (name: string) => {
    await page.getByRole("tab", { name: "Tools" }).click();
    await page.locator(".tool-card").filter({ has: page.locator("strong", { hasText: new RegExp(`^${name}$`) }) }).click();
  };
  for (const name of ["Test Signal", "Input device", "Audio file", "Mixer", "Gain", "Advanced EQ", "Compressor", "Limiter", "Mute", "Meter", "Recorder", "Output device"]) {
    await addTool(name);
  }
  await page.getByRole("tab", { name: "Tools" }).click();
  await page.getByRole("button", { name: "Choose an application source" }).click();
  await page.getByRole("button", { name: "Add capture source" }).click();
  await expect(page.getByTestId("rf__node-application-capture-1")).toBeVisible();
  await page.getByRole("tab", { name: "Devices" }).click();
  await page.locator(".right-workbench").getByLabel("Native render endpoint").selectOption("render-preview");
  await page.locator(".right-workbench").getByRole("button", { name: "Add loopback source to graph" }).click();
  await expect(page.getByTestId("rf__node-endpoint-loopback-1")).toBeVisible();
  await page.getByRole("tab", { name: "Session" }).click();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText(/Preview planner: connect an input to an output/);

  await page.getByRole("tab", { name: "Setup" }).click();
  const connect = async (sourcePort: string, destinationPort: string) => {
    const editor = page.locator(".workbench-connection-editor");
    await editor.getByLabel("Source output port").selectOption({ label: sourcePort });
    await editor.getByLabel("Destination input port").selectOption({ label: destinationPort });
    await editor.getByRole("button", { name: "Add connection", exact: true }).click();
    await expect(page.locator(".workbench-action-message")).toContainText(/Connection added to the draft/);
  };
  await connect("Test Signal 1 · out · 2ch", "Mixer 1 · in · 2ch");
  await connect("Physical input 1 · out · 2ch", "Mixer 1 · in · 2ch");
  await connect("Audio file 1 · out · 2ch", "Mixer 1 · in · 2ch");
  await connect("Music.exe capture 1 · out · 2ch", "Mixer 1 · in · 2ch");
  await connect("Endpoint loopback 1 · out · 2ch", "Mixer 1 · in · 2ch");
  await connect("Mixer 1 · out · 2ch", "Gain 1 · in · 2ch");
  await connect("Gain 1 · out · 2ch", "Advanced EQ 1 · in · 2ch");
  await connect("Advanced EQ 1 · out · 2ch", "Compressor 1 · in · 2ch");
  await connect("Compressor 1 · out · 2ch", "Limiter 1 · in · 2ch");
  await connect("Limiter 1 · out · 2ch", "Mute 1 · in · 2ch");
  await connect("Mute 1 · out · 2ch", "Meter 1 · in · 2ch");
  await connect("Mute 1 · out · 2ch", "Recorder 1 · in · 2ch");
  await connect("Mute 1 · out · 2ch", "Physical output 1 · in · 2ch");
  await expect(page.locator(".react-flow__edge")).toHaveCount(13);
  await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/workspace-complex-route.png`, fullPage: false });
  await page.getByRole("tab", { name: "Session" }).click();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText(/Route saved \(revision 8\)/);
});

test("a simple Test Signal to Physical Output route can be planned and committed", async ({ page }) => {
  await page.goto("/route-harness.html");
  const addTool = async (name: string) => {
    await page.getByRole("tab", { name: "Tools" }).click();
    await page.locator(".tool-card").filter({ hasText: name }).first().click();
  };
  await addTool("Test Signal");
  await addTool("Output device");
  await page.getByRole("tab", { name: "Setup" }).click();
  const setup = page.locator(".right-workbench");
  await setup.getByLabel("Source output port").selectOption({ label: "Test Signal 1 · out · 2ch" });
  await setup.getByLabel("Destination input port").selectOption({ label: "Physical output 1 · in · 2ch" });
  await setup.getByRole("button", { name: "Add connection", exact: true }).click();
  await expect(page.locator(".react-flow__edge")).toHaveCount(1);
  await page.getByRole("tab", { name: "Session" }).click();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText(/Route saved \(revision 8\)/);
});

test("an unsaved Test Signal route can be auditioned without changing the saved revision", async ({ page }) => {
  await page.goto("/route-harness.html");
  for (const name of ["Test Signal", "Output device"]) {
    await page.getByRole("tab", { name: "Tools" }).click();
    await page.locator(".tool-card").filter({ hasText: name }).first().click();
  }
  await page.getByRole("tab", { name: "Setup" }).click();
  const sidebar = page.locator(".right-workbench");
  await sidebar.getByLabel("Source output port").selectOption({ label: "Test Signal 1 · out · 2ch" });
  await sidebar.getByLabel("Destination input port").selectOption({ label: "Physical output 1 · in · 2ch" });
  await sidebar.getByRole("button", { name: "Add connection", exact: true }).click();
  await page.getByRole("button", { name: "Play Test Signal", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText(/Select the speaker or headphone device/);
  await page.getByRole("tab", { name: "Devices" }).click();
  await sidebar.getByLabel("Native capture endpoint").selectOption("capture-preview");
  await sidebar.getByLabel("Native render endpoint").selectOption("render-preview");
  await page.getByRole("button", { name: "Play Test Signal", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText(/Test Signal 1 playing/);
  await page.getByRole("tab", { name: "Session" }).click();
  await expect(page.getByRole("button", { name: "Save", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Stop Test Signal", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText("Test Signal 1 stopped.");
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
  await expect(page.getByRole("button", { name: "Save", exact: true })).toBeEnabled();
});

test("Test Signal lifecycle refreshes preserve the committed route and explain missing audio setup", async ({ page }) => {
  await page.goto("/route-harness.html");
  for (const name of ["Test Signal", "Output device"]) {
    await page.getByRole("tab", { name: "Tools" }).click();
    await page.locator(".tool-card").filter({ hasText: name }).first().click();
  }
  await page.getByRole("tab", { name: "Setup" }).click();
  const sidebar = page.locator(".right-workbench");
  await sidebar.getByLabel("Source output port").selectOption({ label: "Test Signal 1 · out · 2ch" });
  await sidebar.getByLabel("Destination input port").selectOption({ label: "Physical output 1 · in · 2ch" });
  await sidebar.getByRole("button", { name: "Add connection", exact: true }).click();
  await page.getByRole("tab", { name: "Session" }).click();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText(/Route saved \(revision 8\)/);
  const ids = await page.locator(".react-flow__node").evaluateAll((nodes) => nodes.map((node) => node.getAttribute("data-id")));
  expect(ids).toHaveLength(5);
  await page.getByRole("button", { name: "Play Test Signal", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText(/Select the speaker or headphone device/);
  await expect(page.getByRole("tab", { name: "Session" })).toHaveAttribute("aria-selected", "true");
  await page.getByRole("tab", { name: "Devices" }).click();
  await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/playback-setup-guidance.png`, fullPage: false });
  await sidebar.getByLabel("Native capture endpoint").selectOption("capture-preview");
  await sidebar.getByLabel("Native render endpoint").selectOption("render-preview");
  await page.getByRole("button", { name: "Play Test Signal", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText(/Test Signal 1 playing/);
  await expect(page.getByRole("button", { name: "Stop Test Signal", exact: true })).toBeEnabled();
  const dashes = page.locator(".flow-edge-active .signal-flow-edge-dashes");
  await expect(dashes).toHaveCount(1);
  const initialOffset = await dashes.evaluate((edge) => getComputedStyle(edge).strokeDashoffset);
  await expect.poll(() => dashes.evaluate((edge) => getComputedStyle(edge).strokeDashoffset)).not.toBe(initialOffset);
  const flowStroke = page.locator(".flow-edge-active .react-flow__edge-path").first();
  const initialWidth = await flowStroke.evaluate((edge) => getComputedStyle(edge).strokeWidth);
  await expect.poll(() => flowStroke.evaluate((edge) => getComputedStyle(edge).strokeWidth)).not.toBe(initialWidth);
  await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/playback-lifecycle-fixture.png`, fullPage: false });
  // Cross multiple event polls. The fixture returns new JSON objects on each refresh.
  await page.waitForTimeout(2200);
  expect(await page.locator(".react-flow__node").evaluateAll((nodes) => nodes.map((node) => node.getAttribute("data-id")))).toEqual(ids);
  await expect(page.locator(".react-flow__edge")).toHaveCount(1);
  for (const id of ids) await expect(page.getByTestId(`rf__node-${id}`)).toBeVisible();
  await expect(page.locator(".workbench-action-message")).toContainText(/Test Signal 1 playing/);
  await page.getByRole("button", { name: "Stop Test Signal", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText("Test Signal 1 stopped.");
  await expect(page.getByRole("button", { name: "Play Test Signal", exact: true })).toBeEnabled();
  await expect(page.locator(".flow-edge-active")).toHaveCount(0);
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
});

test("top Play runs the canvas route without starting the Test Signal tone", async ({ page }) => {
  await page.goto("/route-harness.html");
  for (const name of ["Test Signal", "Output device"]) {
    await page.getByRole("tab", { name: "Tools" }).click();
    await page.locator(".tool-card").filter({ hasText: name }).first().click();
  }
  await page.getByRole("tab", { name: "Setup" }).click();
  const sidebar = page.locator(".right-workbench");
  await sidebar.getByLabel("Source output port").selectOption({ label: "Test Signal 1 · out · 2ch" });
  await sidebar.getByLabel("Destination input port").selectOption({ label: "Physical output 1 · in · 2ch" });
  await sidebar.getByRole("button", { name: "Add connection", exact: true }).click();
  await page.getByRole("tab", { name: "Devices" }).click();
  await sidebar.getByLabel("Native capture endpoint").selectOption("capture-preview");
  await sidebar.getByLabel("Native render endpoint").selectOption("render-preview");
  await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
  await expect(page.locator(".flow-edge-active")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Play Test Signal", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Play Test Signal", exact: true }).click();
  await expect.poll(() => page.locator(".flow-edge-active").count()).toBeGreaterThan(0);
  await page.getByRole("button", { name: "Stop Test Signal", exact: true }).click();
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
});

test("device tools accept installed virtual endpoints while managed-driver tools explain their limit", async ({ page }) => {
  await page.goto("/route-harness.html");
  await page.getByRole("tab", { name: "Tools" }).click();
  const tools = page.locator(".right-workbench");
  const add = async (label: string) => {
    await page.getByRole("tab", { name: "Tools" }).click();
    await tools.locator(".tool-card").filter({ hasText: new RegExp(label, "i") }).first().click();
  };
  await add("Input device");
  await expect(page.getByTestId("rf__node-physicalInput-1")).toBeVisible();
  await add("Gain");
  await expect(page.getByTestId("rf__node-gain-1")).toBeVisible();
  await add("Output device");
  await expect(page.getByTestId("rf__node-physicalOutput-1")).toBeVisible();
  await page.getByRole("tab", { name: "Tools" }).click();
  await expect(tools.getByRole("button", { name: /Virtual render source/ })).toBeDisabled();
  await expect(tools.getByRole("button", { name: /Virtual capture sink/ })).toBeDisabled();
  await expect(tools.getByRole("button", { name: /Virtual render source/ })).toHaveAttribute("title", /Input device/);
  const setup = async (source: string, destination: string) => {
    await page.getByRole("tab", { name: "Setup" }).click();
    const editor = page.locator(".workbench-connection-editor");
    await editor.getByLabel("Source output port").selectOption({ label: source });
    await editor.getByLabel("Destination input port").selectOption({ label: destination });
    await editor.getByRole("button", { name: "Add connection", exact: true }).click();
  };
  await setup("Physical input 1 · out · 2ch", "Gain 1 · in · 2ch");
  await setup("Gain 1 · out · 2ch", "Physical output 1 · in · 2ch");
  await expect(page.locator(".react-flow__edge")).toHaveCount(2);
  await page.getByRole("tab", { name: "Session" }).click();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".workbench-action-message")).toContainText(/Route saved \(revision 8\)/);
});

test("the canvas remains usable at 1280 by 720", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  const graph = page.getByLabel("Signal-flow graph");
  await expect(graph).toBeVisible();
  const bounds = await graph.boundingBox();
  expect(bounds).not.toBeNull();
  expect(bounds!.width).toBeGreaterThan(900);
  await expect(page.locator(".canvas-library")).toBeVisible();
});

test("workspace and recording tab fit desktop viewports without page scrolling", async ({ page }) => {
  await page.goto("/route-harness.html");
  const report: unknown[] = [];
  for (const viewport of [{ width: 1280, height: 720 }, { width: 1440, height: 900 }, { width: 1920, height: 1080 }]) {
    await page.setViewportSize(viewport);
    await page.waitForTimeout(80);
    const geometry = await page.evaluate(() => {
      const rect = (selector: string) => document.querySelector(selector)?.getBoundingClientRect().toJSON();
      return { viewport: [innerWidth, innerHeight], document: [document.documentElement.scrollWidth, document.documentElement.scrollHeight], app: rect(".app-shell"), topbar: rect(".topbar"), workspace: rect(".workspace-grid"), main: rect(".main-content"), canvasPanel: rect("#signal-flow-panel"), canvas: rect(".session-flow-canvas"), workbench: rect(".right-workbench") };
    });
    report.push(geometry);
    expect(await page.evaluate(() => document.documentElement.scrollHeight), `page overflow at ${viewport.width}x${viewport.height}`).toBeLessThanOrEqual(viewport.height + 1);
    expect(await page.locator(".main-content").evaluate((element) => element.getBoundingClientRect().bottom), `workspace clipped at ${viewport.width}x${viewport.height}`).toBeLessThanOrEqual(viewport.height + 1);
    await expect(page.locator(".session-flow-canvas")).toBeInViewport();
    await expect(page.locator(".workbench-tabs")).toBeInViewport();
    if (viewport.width === 1280) await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/workspace-1280x720.png` });
  }
  console.log("DESIGN_GEOMETRY", JSON.stringify(report));
  await page.getByRole("tab", { name: "Recording" }).click();
  await expect(page.getByRole("heading", { name: "Recording" })).toBeVisible();
  const recordingPanel = page.locator(".right-workbench");
  await expect(recordingPanel.getByLabel("Recorder ID")).toHaveValue("voice-recording");
  await expect(recordingPanel.getByLabel("Recorder sample rate")).toHaveValue("48000");
  const recorderIdBox = await recordingPanel.getByLabel("Recorder ID").boundingBox();
  const sampleRateBox = await recordingPanel.getByLabel("Recorder sample rate").boundingBox();
  expect(recorderIdBox && sampleRateBox && Math.abs(recorderIdBox.width - sampleRateBox.width)).toBeLessThan(1);
  await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/recording-tab-current.png` });
});

test("compact status keeps the route guide and primary actions readable on a short desktop window", async ({ page }) => {
  await page.goto("/");
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.getByRole("button", { name: "Show status" }).click();
  await page.waitForTimeout(300);
  const status = page.getByLabel("Compact route status");
  await expect(status).toBeVisible();
  await expect(status.getByText("What happens next")).toBeVisible();
  await expect(status.getByRole("button", { name: "Start" })).toHaveCount(0);
  const geometry = await page.evaluate(() => ({
    viewport: [innerWidth, innerHeight],
    document: [document.documentElement.scrollWidth, document.documentElement.scrollHeight],
    status: document.querySelector(".compact-status-panel")?.getBoundingClientRect().toJSON(),
    guide: document.querySelector(".compact-status-guide")?.getBoundingClientRect().toJSON(),
    actions: document.querySelector(".compact-status-actions")?.getBoundingClientRect().toJSON(),
    canvasPanel: document.querySelector("#signal-flow-panel")?.getBoundingClientRect().toJSON(),
    canvas: document.querySelector(".session-flow-canvas")?.getBoundingClientRect().toJSON(),
    workbench: document.querySelector(".right-workbench")?.getBoundingClientRect().toJSON(),
  }));
  console.log("COMPACT_STATUS_GEOMETRY", JSON.stringify(geometry));
  const containment = await page.evaluate(() => {
    const panel = document.querySelector("#signal-flow-panel")!.getBoundingClientRect();
    const canvas = document.querySelector(".session-flow-canvas")!.getBoundingClientRect();
    return { top: canvas.top >= panel.top, bottom: canvas.bottom <= panel.bottom + 1 };
  });
  expect(containment, "graph viewport should stay inside its panel").toEqual({ top: true, bottom: true });
  const nodesInsideViewport = await page.evaluate(() => {
    const viewport = document.querySelector(".session-flow-canvas")!.getBoundingClientRect();
    const nodes = [...document.querySelectorAll<HTMLElement>(".react-flow__node")];
    return nodes.length > 0 && nodes.every((node) => {
      const bounds = node.getBoundingClientRect();
      return bounds.left >= viewport.left && bounds.right <= viewport.right && bounds.top >= viewport.top && bounds.bottom <= viewport.bottom;
    });
  });
  expect(nodesInsideViewport, "all graph nodes should be visible after compact status resizes the canvas").toBe(true);
  await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/workspace-status-1280x720.png`, fullPage: false });
  await page.getByRole("tab", { name: "Properties" }).click();
  await expect(page.locator(".main-content > .inspector")).toBeVisible();
  const propertiesGeometry = await page.evaluate(() => ({
    canvasPanel: document.querySelector("#signal-flow-panel")!.getBoundingClientRect().toJSON(),
    workbench: document.querySelector(".right-workbench")!.getBoundingClientRect().toJSON(),
  }));
  expect(Math.abs(propertiesGeometry.canvasPanel.height - geometry.canvasPanel!.height)).toBeLessThan(3);
  expect(Math.abs(propertiesGeometry.workbench.height - geometry.workbench!.height)).toBeLessThan(3);
  expect(await page.evaluate(() => document.documentElement.scrollHeight)).toBeLessThanOrEqual(720);
  await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/workspace-status-properties-1280x720.png`, fullPage: false });
});

test("desktop workspace keeps task-focused tabs, MCP setup, and diagnostics reachable", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".topbar h1")).toBeVisible();
  await expect(page.getByLabel("Sessions")).toBeHidden();
  await expect(page.getByRole("tab", { name: "Tools" })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByRole("heading", { name: "Add tools" })).toBeVisible();
  await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/workspace-tools.png`, fullPage: false });
  const tools = await page.locator(".right-workbench").boundingBox();
  const graph = await page.locator("#signal-flow-panel").boundingBox();
  expect(tools?.x).toBeGreaterThan(graph!.x);
  expect(graph!.width).toBeGreaterThan(800);
  await page.getByRole("tab", { name: "Session" }).click();
  await expect(page.locator(".right-workbench").getByLabel("Choose session")).toBeVisible();
  await expect(page.locator(".right-workbench").getByRole("button", { name: "Rename" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "Save", exact: true })).toBeVisible();
  const shotBase = `${process.env.TEMP}/audiorouter-designer-review`;
  await page.screenshot({ path: `${shotBase}/workspace-session.png`, fullPage: false });
  await page.getByRole("button", { name: "Show status" }).click();
  await expect(page.getByLabel("Compact route status")).toBeVisible();
  await expect(page.getByRole("tab", { name: "Session" })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByText("What happens next")).toBeVisible();
  const statusActions = await page.locator(".compact-status-actions").boundingBox();
  const statusGuide = await page.locator(".compact-status-guide").boundingBox();
  expect(statusActions && statusGuide && statusActions.x + statusActions.width).toBeLessThanOrEqual(statusGuide!.x + 1);
  const legacyFullWorkspacePanels = page.locator(".main-content > .panel:not(.inspector):not(.canvas-panel)");
  const allLegacyPanelsHidden = await legacyFullWorkspacePanels.evaluateAll((panels) => panels.every((panel) => getComputedStyle(panel).display === "none"));
  expect(allLegacyPanelsHidden).toBe(true);
  await expect(page.locator(".right-workbench")).toBeVisible();
  await page.screenshot({ path: `${shotBase}/workspace-status.png`, fullPage: true });
  await page.getByRole("button", { name: "Hide status" }).click();
  await page.getByRole("tab", { name: "Tools" }).click();
  await page.locator(".react-flow__node").nth(1).click();
  await page.getByRole("tab", { name: "Properties" }).click();
  await expect(page.getByRole("tab", { name: "Properties" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(".main-content > .inspector")).toBeVisible();
  await page.screenshot({ path: `${shotBase}/workspace-properties.png`, fullPage: false });
  await page.getByRole("tab", { name: "Session" }).click();
  for (const [tab, heading] of [["Setup", "First route"], ["Devices", "Audio devices"], ["Recording", "Recording"], ["Advanced", "Advanced controls"]]) {
    await page.getByRole("tab", { name: tab }).click();
    await expect(page.getByRole("heading", { name: heading }).first()).toBeVisible();
    if (tab === "Advanced") {
      await page.locator(".right-workbench").getByText("Keyboard shortcuts", { exact: true }).click();
      await expect(page.locator(".right-workbench").getByLabel("Start or stop session shortcut")).toBeVisible();
      await page.locator(".right-workbench").getByText("Built-in processors and presets", { exact: true }).click();
      await expect(page.getByRole("heading", { name: "Built-in processors" })).toBeVisible();
      await page.locator(".right-workbench").getByText("Session import and export", { exact: true }).click();
      await expect(page.getByRole("heading", { name: "Session transfer" })).toBeVisible();
    }
    await page.screenshot({ path: `${shotBase}/workspace-${tab.toLowerCase()}.png`, fullPage: false });
  }
  await page.getByRole("tab", { name: "MCP" }).click();
  await expect(page.getByRole("heading", { name: "Incoming tool calls" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Use this installation" })).toBeVisible();
  await expect(page.getByLabel("Assistant client ID")).toBeVisible();
  await page.getByText("Codex config.toml").click();
  await expect(page.locator(".mcp-command").first()).toContainText("mcp_servers.audiorouter");
  await page.getByRole("button", { name: "Copy Codex config" }).click();
  await expect(page.getByRole("status").filter({ hasText: /Copied|Clipboard unavailable/ })).toBeVisible();
  await page.locator(".mcp-howto").scrollIntoViewIfNeeded();
  await expect(page.locator(".mcp-howto")).toBeInViewport();
  await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/workspace-mcp.png`, fullPage: false });
  await page.getByRole("tab", { name: "Logs" }).click();
  await expect(page.getByRole("heading", { name: "Client diagnostics" })).toBeVisible();
  await page.getByLabel("Color theme").selectOption("dark");
  await page.screenshot({ path: `${shotBase}/workspace-dark.png`, fullPage: false });
  await page.getByLabel("Color theme").selectOption("light");
  await page.screenshot({ path: `${shotBase}/workspace-light.png`, fullPage: false });
  await page.getByLabel("Color theme").selectOption("high-contrast");
  await page.screenshot({ path: `${shotBase}/workspace-high-contrast.png`, fullPage: false });
});

test("the MCP activity stream displays redacted incoming bridge events", async ({ page }) => {
  await page.addInitScript(() => {
    const activity = [{ timeUnixMs: Date.now(), clientId: "codex-local", tool: "routes.inspect", argumentFields: ["sessionId", "destinationNode"], outcome: "ok", errorKind: null }];
    const backendRows = [{ timeUnixMs: Date.now(), method: "graph.commit", outcome: "error", errorKind: "permissionDenied", summary: { revision: 7, nodeCount: 3 } }];
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: { invoke: async (command: string) => command === "mcp_activity_list" ? activity : command === "backend_diagnostics_list" ? backendRows : null },
    });
  });
  await page.goto("/route-harness.html");
  await page.getByRole("tab", { name: "MCP" }).click();
  await expect(page.getByLabel("MCP tool activity")).toContainText("routes.inspect");
  await expect(page.getByLabel("MCP tool activity")).toContainText("sessionId, destinationNode");
  await expect(page.getByLabel("MCP tool activity")).not.toContainText("argument values");
  await page.getByRole("tab", { name: "Logs" }).click();
  await expect(page.locator(".right-workbench")).toContainText("Graph checkpoint:");
  await expect(page.getByLabel("Backend RPC activity")).toContainText("graph.commit");
  await expect(page.getByLabel("Backend RPC activity")).toContainText("permissionDenied");
  await expect(page.getByLabel("Backend RPC activity")).toContainText('"nodeCount":3');
});
