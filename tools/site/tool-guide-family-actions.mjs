// What the recorder does in the real Properties panel for a family's shared
// clip (tool-guide-families.json). record-tool-guide-overviews.mjs calls
// act() when the render's parameters change, reveal() to show the controls
// that changed, and verify() once the clip is captured. verify() must throw
// unless the panel ended up showing exactly the values the audio was
// rendered with.

// Set a number field the way the recorder does elsewhere: through the native
// setter and an input event, then blur.
async function setNumber(locator, value) {
  await locator.evaluate((element, next) => {
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
    setter.call(element, String(next));
    element.dispatchEvent(new Event("input", { bubbles: true }));
  }, value);
  await locator.evaluate((element) => element.blur());
}

// The band the clip adds: the one that is off before and on after.
function addedBand(before, after) {
  const indexes = Object.keys(after)
    .map((key) => /^band(\d+)Enabled$/.exec(key))
    .filter((match) => match && after[match[0]] === true && before[match[0]] !== true)
    .map((match) => Number(match[1]));
  if (indexes.length !== 1)
    throw new Error(`Expected exactly one added band, found ${indexes.length}`);
  return indexes[0];
}

// Geometry of the editor's graph, from ui/src/AdvancedEqEditor.tsx. The
// verification below fails if these drift from the component.
const graph = { width: 380, height: 248, left: 34, right: 366, top: 16, bottom: 190, dbRange: 12 };

const controls = (inspector) => ({
  frequency: inspector.getByRole("spinbutton", { name: "EQ frequency Hz", exact: true }),
  gain: inspector.getByRole("spinbutton", { name: "EQ gain dB", exact: true }),
  q: inspector.getByRole("spinbutton", { name: "EQ Q width", exact: true }),
  type: inspector.getByRole("combobox", { name: "EQ filter type", exact: true }),
});

export const familyActions = {
  soundboard: {
    async focus({ inspector }) {
      await inspector.locator(".soundboard-slot").nth(1).scrollIntoViewIfNeeded();
    },
    async act({ inspector, after }) {
      const row = inspector.locator(".soundboard-slot").nth(1);
      await row.getByRole("textbox", { name: "Caption" }).fill(after.slot2Caption);
      await setNumber(row.getByRole("slider"), after.slot2GainDb);
      await row.getByRole("combobox", { name: "When pressed" }).selectOption(after.slot2Mode);
    },
    async reveal({ inspector }) {
      await inspector.locator(".soundboard-slot").nth(1).scrollIntoViewIfNeeded();
    },
    async verify({ inspector, after }) {
      const row = inspector.locator(".soundboard-slot").nth(1);
      const actual = [
        await row.getByRole("textbox", { name: "Caption" }).inputValue(),
        Number(await row.getByRole("slider").inputValue()),
        await row.getByRole("combobox", { name: "When pressed" }).inputValue(),
      ];
      const expected = [after.slot2Caption, after.slot2GainDb, after.slot2Mode];
      if (JSON.stringify(actual) !== JSON.stringify(expected))
        throw new Error(`Soundboard controls show ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
    },
  },
  parametricEq: {
    // Bring the graph into view before the change; the live readings stay at
    // the top of the panel the rest of the time.
    async focus({ inspector }) {
      await inspector
        .getByRole("img", { name: "EQ frequency response and movable filter points" })
        .evaluate((element) => element.scrollIntoView({ block: "center" }));
    },
    // Add the band as the editor documents it (double-click the graph where
    // the band should sit), then enter the exact rendered values: the click
    // can only land on the nearest pixel.
    async act({ page, inspector, before, after }) {
      const index = addedBand(before, after);
      const frequency = after[`band${index}FrequencyHz`];
      const gain = after[`band${index}GainDb`];
      const svg = inspector.getByRole("img", { name: "EQ frequency response and movable filter points" });
      const box = await svg.boundingBox();
      if (!box) throw new Error("The EQ graph is not visible");
      const x = graph.left + (Math.log10(frequency / 20) / 3) * (graph.right - graph.left);
      const y = graph.top + ((graph.dbRange - gain) / (2 * graph.dbRange)) * (graph.bottom - graph.top);
      const pointer = { x: box.x + (x / graph.width) * box.width, y: box.y + (y / graph.height) * box.height };
      const viewport = page.viewportSize();
      if (pointer.x < 0 || pointer.y < 0 || pointer.x > viewport.width || pointer.y > viewport.height)
        throw new Error(`The graph position for band ${index + 1} is outside the viewport`);
      await page.mouse.dblclick(pointer.x, pointer.y);
      // The last of the editor's five writes enables the band; wait for it so
      // a late write cannot overwrite the exact values entered next.
      const remove = inspector.getByRole("button", { name: "Remove point", exact: true });
      const deadline = Date.now() + 3000;
      while (!(await remove.isEnabled()) && Date.now() < deadline)
        await new Promise((resolve) => setTimeout(resolve, 20));
      if (!(await remove.isEnabled())) throw new Error(`Double-clicking the graph did not add point ${index + 1}`);
      const { frequency: frequencyField, gain: gainField } = controls(inspector);
      // The click must land near the target, or the geometry above is stale.
      const placed = {
        frequency: Number(await frequencyField.inputValue()),
        gain: Number(await gainField.inputValue()),
      };
      if (Math.abs(Math.log(placed.frequency / frequency)) > 0.05 || Math.abs(placed.gain - gain) > 1)
        throw new Error(
          `Double-click placed ${placed.frequency} Hz at ${placed.gain} dB, not near ${frequency} Hz at ${gain} dB`,
        );
      await setNumber(frequencyField, frequency);
      await setNumber(gainField, gain);
    },
    // Show the new band's controls, centered.
    async reveal({ inspector }) {
      await inspector.locator(".advanced-eq-controls").evaluate((element) => element.scrollIntoView({ block: "center" }));
    },
    async verify({ inspector, before, after }) {
      const index = addedBand(before, after);
      const field = controls(inspector);
      const shown = {
        enabled: await inspector.getByRole("button", { name: "Remove point", exact: true }).isEnabled(),
        type: await field.type.inputValue(),
        frequencyHz: Number(await field.frequency.inputValue()),
        gainDb: Number(await field.gain.inputValue()),
        q: Number(await field.q.inputValue()),
        heading: await inspector.locator(".advanced-eq-selected strong").innerText(),
      };
      const expected = {
        enabled: true,
        type: after[`band${index}Type`],
        frequencyHz: after[`band${index}FrequencyHz`],
        gainDb: after[`band${index}GainDb`],
        q: after[`band${index}Q`],
        heading: `Point ${index + 1}`,
      };
      if (JSON.stringify(shown) !== JSON.stringify(expected))
        throw new Error(
          `The panel shows ${JSON.stringify(shown)} but the audio was rendered with ${JSON.stringify(expected)}`,
        );
    },
  },
};
