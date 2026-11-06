import { test, expect, type Page } from "./browser-test";

const THEMES = ["dark", "light", "high-contrast"];

const rgb = (text: string) => text.match(/\d+/g)!.map(Number);
function luminance([r, g, b]: number[]) {
  const channel = (value: number) => {
    const c = value / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}
function contrast(a: number[], b: number[]) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/** A Test Signal has plain catalog settings, so it shows the standard slider field. */
async function openTestSignal(page: Page, theme: string) {
  await page.addInitScript((theme) => {
    localStorage.setItem("audiorouter.ui.theme", theme);
    Object.assign(window, {
      __routeFixtureProcessors: [
        {
          id: "testSignal",
          version: 1,
          category: "native",
          availability: { status: "available" },
          latencySamples: 0,
          parameters: [
            { name: "levelDb", type: "number", minimum: -60, maximum: 0, default: -18, unit: "dBFS", step: 1 },
            { name: "frequencyHz", type: "number", minimum: 20, maximum: 20000, default: 440, unit: "Hz" },
            { name: "durationMs", type: "number", minimum: 1, maximum: 600000, default: 1000, unit: "ms", step: 1 },
          ],
        },
      ],
    });
  }, theme);
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page
    .locator(".tool-card")
    .filter({ has: page.getByText("Test Signal", { exact: true }) })
    .click();
  await page.getByTestId("rf__node-testSignal-1").locator(".flow-node-title").click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  const field = page.locator(".main-content > .inspector .slider-field").first();
  await expect(field).toBeVisible();
  await field.scrollIntoViewIfNeeded();
  return { field, slider: field.locator("input.app-slider") };
}

for (const theme of THEMES) {
  test(`the fill reaches the left end of the rail in ${theme}`, async ({ page }) => {
    // Regression: the fill started under the thumb's lowest position, leaving a grey stub at the end of the rail.
    const { slider } = await openTestSignal(page, theme);
    await slider.fill("-30"); // Halfway, well away from both ends of the -60 to 0 range.
    await page.mouse.move(0, 0);
    await slider.evaluate((element) => (element as HTMLElement).blur());
    const image = (await slider.screenshot()).toString("base64");
    const { nearEnd, midFill, beyondThumb } = await page.evaluate(async (data) => {
      const bytes = Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
      const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
      const canvas = document.createElement("canvas");
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
      const context = canvas.getContext("2d")!;
      context.drawImage(bitmap, 0, 0);
      const at = (x: number) => [...context.getImageData(x, Math.floor(bitmap.height / 2), 1, 1).data].slice(0, 3);
      return { nearEnd: at(4), midFill: at(Math.floor(bitmap.width / 4)), beyondThumb: at(bitmap.width - 12) };
    }, image);
    // The sample points really are on two different colors, so the comparison means something.
    expect(Math.max(...midFill.map((value, index) => Math.abs(value - beyondThumb[index])))).toBeGreaterThan(30);
    expect(Math.max(...nearEnd.map((value, index) => Math.abs(value - midFill[index])))).toBeLessThanOrEqual(6);
  });

  test(`Properties slider is the app control, not a native one, in ${theme}`, async ({ page }, info) => {
    const { slider } = await openTestSignal(page, theme);
    const look = await slider.evaluate((element) => {
      const style = getComputedStyle(element);
      let surface = element.parentElement;
      while (surface && /rgba\(0, 0, 0, 0\)|transparent/.test(getComputedStyle(surface).backgroundColor))
        surface = surface.parentElement;
      // Resolve any color syntax to plain RGB by painting one pixel.
      const paint = document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
      const resolve = (color: string) => {
        paint.fillStyle = color;
        paint.fillRect(0, 0, 1, 1);
        const [red, green, blue] = paint.getImageData(0, 0, 1, 1).data;
        return `rgb(${red}, ${green}, ${blue})`;
      };
      return {
        appearance: style.appearance,
        background: style.backgroundColor,
        height: element.getBoundingClientRect().height,
        track: resolve(style.getPropertyValue("--slider-track")),
        fill: resolve(style.getPropertyValue("--slider-fill")),
        surface: resolve(getComputedStyle(surface ?? document.body).backgroundColor),
      };
    });
    // A native control, or the dark capsule that once sat behind it in the light theme.
    expect(look.appearance).toBe("none");
    expect(look.background).toBe("rgba(0, 0, 0, 0)");
    expect(look.height).toBeGreaterThanOrEqual(24);
    expect(contrast(rgb(look.track), rgb(look.surface))).toBeGreaterThanOrEqual(3);
    expect(contrast(rgb(look.fill), rgb(look.surface))).toBeGreaterThanOrEqual(3);
    await page.screenshot({ path: info.outputPath(`${theme}-slider.png`) });
  });

  test(`slider value, exact field, reset and keys work together in ${theme}`, async ({ page }, info) => {
    const { field, slider } = await openTestSignal(page, theme);
    const precise = field.getByRole("spinbutton");
    const reset = field.getByRole("button", { name: /^Reset / });
    const geometry = async () =>
      field.evaluate((element) => {
        const box = element.getBoundingClientRect();
        const track = element.querySelector(".slider-field-track")!.getBoundingClientRect();
        const exact = element.querySelector(".slider-field-exact")!.getBoundingClientRect();
        // Help text, when a setting has it, sits between the caption and the track.
        const above = element.querySelector(".parameter-help") ?? element.querySelector(".slider-field-caption")!;
        return {
          height: box.height,
          trackLeft: track.left,
          trackWidth: track.width,
          exactLeft: exact.left,
          captionGap: track.top - above.getBoundingClientRect().bottom,
          exactGap: exact.left - track.right,
        };
      });
    await expect(reset).toHaveCount(0);
    const atDefault = await geometry();
    // The value sits next to the track, not across the panel.
    expect(atDefault.exactGap).toBeLessThanOrEqual(12);
    expect(atDefault.captionGap).toBeLessThanOrEqual(8);

    const initial = Number(await slider.inputValue());
    const maximum = Number(await slider.getAttribute("max"));
    await slider.focus();
    await page.keyboard.press("ArrowRight");
    const oneStep = Number(await slider.inputValue()) - initial;
    expect(oneStep).toBeGreaterThan(0);
    await page.keyboard.press("Shift+ArrowRight");
    expect(Number(await slider.inputValue()) - initial).toBeCloseTo(oneStep * 11, 5);
    await expect(precise).toHaveValue(await slider.inputValue());
    await expect(reset).toBeVisible();
    expect(await slider.evaluate((element) => element.style.getPropertyValue("--slider-to"))).not.toBe(
      await slider.evaluate((element) => element.style.getPropertyValue("--slider-from")),
    );
    // Showing the reset button moves nothing (UI-17).
    expect(await geometry()).toEqual(atDefault);
    await page.screenshot({ path: info.outputPath(`${theme}-changed.png`) });

    await reset.click();
    await expect(slider).toHaveValue(String(initial));
    await expect(slider).toBeFocused();
    await expect(reset).toHaveCount(0);
    expect(await geometry()).toEqual(atDefault);

    const typed = String(initial + oneStep * 2);
    await precise.fill(typed);
    await expect(slider).toHaveValue(typed);
    await precise.fill(String(maximum + 1));
    await expect(precise).toHaveAttribute("aria-invalid", "true");
    await expect(slider).toHaveValue(typed);
    await precise.fill(typed);
    await slider.dblclick();
    await expect(slider).toHaveValue(String(initial));
  });
}

test("no native range input is left on the screen", async ({ page }) => {
  await openTestSignal(page, "dark");
  await expect(page.locator('input[type="range"]:not(.app-slider)')).toHaveCount(0);
});

test("a very narrow panel stacks the exact field under the track", async ({ page }, info) => {
  const { field } = await openTestSignal(page, "dark");
  await field.evaluate((element) => {
    (element as HTMLElement).style.width = "14rem";
  });
  const stacked = await field.evaluate((element) => {
    const track = element.querySelector(".slider-field-track")!.getBoundingClientRect();
    const exact = element.querySelector(".slider-field-exact")!.getBoundingClientRect();
    return { below: exact.top >= track.bottom, inside: exact.right <= element.getBoundingClientRect().right };
  });
  expect(stacked).toEqual({ below: true, inside: true });
  await field.screenshot({ path: info.outputPath("narrow-slider.png") });
});
