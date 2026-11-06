/** @vitest-environment jsdom */
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  SliderField,
  logTrackPosition,
  logTrackValue,
  sliderBoundLabel,
  sliderFill,
  sliderFillOrigin,
  sliderUnit,
  usesLogTrack,
} from "./Slider";

afterEach(cleanup);

function Field({ initial = 0, onChange }: { initial?: number; onChange?: (value: number) => void }) {
  const [value, setValue] = useState(initial);
  return (
    <SliderField
      label="Level"
      name="Level"
      value={value}
      min={-24}
      max={6}
      step={1}
      unit="dB"
      defaultValue={0}
      onChange={(next) => {
        setValue(next);
        onChange?.(next);
      }}
    />
  );
}

describe("slider fill", () => {
  it("starts at zero on a range that crosses it, otherwise at the minimum", () => {
    expect(sliderFillOrigin(-24, 6)).toBe(0);
    expect(sliderFillOrigin(0, 100)).toBe(0);
    expect(sliderFillOrigin(20, 500)).toBe(20);
    expect(sliderFillOrigin(-60, 0)).toBe(-60);
  });
  it("runs from the origin to the value on either side", () => {
    expect(sliderFill(0, -24, 6)).toEqual({ from: 0.8, to: 0.8 });
    expect(sliderFill(-24, -24, 6)).toEqual({ from: 0, to: 0.8 });
    expect(sliderFill(6, -24, 6)).toEqual({ from: 0.8, to: 1 });
    expect(sliderFill(50, 0, 100)).toEqual({ from: 0, to: 0.5 });
  });
  it("clamps an out-of-range saved value and survives an empty range", () => {
    expect(sliderFill(99, 0, 10).to).toBe(1);
    expect(sliderFill(-99, 0, 10).to).toBe(0);
    expect(sliderFill(5, 5, 5)).toEqual({ from: 0, to: 0 });
  });
});

describe("slider labels", () => {
  it("writes limits the way a person does", () => {
    expect(sliderBoundLabel(-24, "dB", true)).toBe("−24 dB");
    expect(sliderBoundLabel(6, "dB", true)).toBe("+6 dB");
    expect(sliderBoundLabel(20_000, "Hz", false)).toBe("20 kHz");
    expect(sliderBoundLabel(500, "Hz", false)).toBe("500 Hz");
    expect(sliderBoundLabel(100, "%", false)).toBe("100 %");
    expect(sliderBoundLabel(1, undefined, false)).toBe("1");
  });
  it("writes long durations in larger units", () => {
    expect(sliderBoundLabel(600_000, "ms", false)).toBe("10 min");
    expect(sliderBoundLabel(2000, "ms", false)).toBe("2 s");
    expect(sliderBoundLabel(10, "ms", false)).toBe("10 ms");
  });
  it("shortens long units and drops the meaningless one", () => {
    expect(sliderUnit("semitones")).toBe("st");
    expect(sliderUnit("cents")).toBe("ct");
    expect(sliderUnit("normalized")).toBe("");
    expect(sliderUnit(undefined)).toBe("");
  });
});

describe("SliderField", () => {
  const slider = () => screen.getByRole("slider", { name: "Level" });
  const exact = () => screen.getByRole("spinbutton", { name: "Level precise value" });
  const reset = () => screen.queryByRole("button", { name: "Reset Level to 0 dB" });

  it("offers reset only once the value differs from the default", () => {
    render(<Field />);
    expect(reset()).toBeNull();
    fireEvent.change(slider(), { target: { value: "-6" } });
    expect((exact() as HTMLInputElement).value).toBe("-6");
    fireEvent.click(reset()!);
    expect((slider() as HTMLInputElement).value).toBe("0");
    expect(reset()).toBeNull();
    // The button is gone, so keyboard focus moves to the slider instead of the page.
    expect(document.activeElement).toBe(slider());
  });

  it("names the value with its unit for assistive technology", () => {
    render(<Field initial={-6} />);
    expect(slider().getAttribute("aria-valuetext")).toBe("-6 dB");
    expect(exact().getAttribute("aria-valuetext")).toBe("-6 dB");
  });

  it("moves ten steps with Shift and clamps at the limits", () => {
    const onChange = vi.fn();
    render(<Field initial={0} onChange={onChange} />);
    fireEvent.keyDown(slider(), { key: "ArrowLeft", shiftKey: true });
    expect(onChange).toHaveBeenLastCalledWith(-10);
    fireEvent.keyDown(slider(), { key: "ArrowRight", shiftKey: true });
    fireEvent.keyDown(slider(), { key: "ArrowRight", shiftKey: true });
    expect(onChange).toHaveBeenLastCalledWith(6);
  });

  it("restores the default on double-click, but not while disabled", () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <SliderField
        label="Level"
        name="Level"
        value={-6}
        min={-24}
        max={6}
        step={1}
        defaultValue={0}
        onChange={onChange}
      />,
    );
    fireEvent.doubleClick(slider());
    expect(onChange).toHaveBeenCalledWith(0);
    onChange.mockClear();
    rerender(
      <SliderField
        label="Level"
        name="Level"
        value={-6}
        min={-24}
        max={6}
        step={1}
        defaultValue={0}
        disabled
        onChange={onChange}
      />,
    );
    fireEvent.doubleClick(slider());
    expect(onChange).not.toHaveBeenCalled();
  });

  it("keeps the exact field's rejected text out of the value", () => {
    const onChange = vi.fn();
    render(<Field onChange={onChange} />);
    act(() => exact().focus());
    fireEvent.change(exact(), { target: { value: "99" } });
    expect(onChange).not.toHaveBeenCalled();
    expect(exact().getAttribute("aria-invalid")).toBe("true");
  });

  it("does not offer a reset to a default outside the range", () => {
    render(
      <SliderField
        label="Level"
        name="Level"
        value={2}
        min={1}
        max={5}
        step={1}
        defaultValue={9}
        onChange={() => {}}
      />,
    );
    expect(screen.queryByRole("button", { name: /Reset/ })).toBeNull();
  });
});

describe("logarithmic track", () => {
  function FrequencyField({ onChange }: { onChange?: (value: number) => void }) {
    const [value, setValue] = useState(440);
    return (
      <SliderField
        label="Frequency"
        name="Frequency"
        value={value}
        min={20}
        max={20_000}
        step={1}
        unit="Hz"
        defaultValue={440}
        onChange={(next) => {
          setValue(next);
          onChange?.(next);
        }}
      />
    );
  }
  it("is used only for wide positive ranges", () => {
    expect(usesLogTrack(20, 20_000, "Hz")).toBe(true);
    expect(usesLogTrack(1, 600_000, "ms")).toBe(true);
    expect(usesLogTrack(80, 1000, "Hz")).toBe(false);
    expect(usesLogTrack(0, 100, "ms")).toBe(false);
    expect(usesLogTrack(-60, 0, "dB")).toBe(false);
    // Group opacity once took the log track only because its range happens to span 100 times.
    expect(usesLogTrack(1, 100, "%")).toBe(false);
  });
  it("reaches both exact limits and round-trips closely", () => {
    expect(logTrackValue(0, 20, 20_000, 1)).toBe(20);
    expect(logTrackValue(1000, 20, 20_000, 1)).toBe(20_000);
    expect(logTrackPosition(20_000, 20, 20_000)).toBe(1000);
    expect(logTrackPosition(1, 20, 20_000)).toBe(0);
    const near = logTrackValue(logTrackPosition(440, 20, 20_000), 20, 20_000, 1);
    expect(Math.abs(near - 440)).toBeLessThanOrEqual(5);
    // 440 Hz is usably far along the track, not crowded against the low end.
    expect(logTrackPosition(440, 20, 20_000)).toBeGreaterThan(400);
  });
  it("tells assistive technology the real value, not the track position", () => {
    render(<FrequencyField />);
    const slider = screen.getByRole("slider", { name: "Frequency" });
    expect(slider.getAttribute("aria-valuenow")).toBe("440");
    expect(slider.getAttribute("aria-valuemax")).toBe("20000");
    expect(slider.getAttribute("aria-valuetext")).toBe("440 Hz");
  });
  it("always moves on a key press, even at the crowded low end", () => {
    const onChange = vi.fn();
    render(
      <SliderField
        label="Frequency"
        name="Frequency"
        value={20}
        min={20}
        max={20_000}
        step={1}
        unit="Hz"
        onChange={onChange}
      />,
    );
    fireEvent.keyDown(screen.getByRole("slider", { name: "Frequency" }), { key: "ArrowRight" });
    expect(onChange.mock.calls[0][0]).toBeGreaterThan(20);
  });
  it("maps a drag to a rounded value and keeps the exact field in step", () => {
    const onChange = vi.fn();
    render(<FrequencyField onChange={onChange} />);
    fireEvent.change(screen.getByRole("slider", { name: "Frequency" }), { target: { value: "1000" } });
    expect(onChange).toHaveBeenLastCalledWith(20_000);
    expect((screen.getByRole("spinbutton", { name: "Frequency precise value" }) as HTMLInputElement).value).toBe(
      "20000",
    );
  });
});

describe("source rule", () => {
  it("renders every range input through the shared control", () => {
    const root = path.resolve(__dirname);
    const offenders = readdirSync(root)
      .filter((name) => name.endsWith(".tsx") && !name.endsWith(".test.tsx") && name !== "Slider.tsx")
      .filter((name) => /\btype="range"/.test(readFileSync(path.join(root, name), "utf8")));
    expect(offenders).toEqual([]);
  });
});
