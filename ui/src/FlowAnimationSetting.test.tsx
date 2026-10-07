/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { FlowAnimationSetting } from "./FlowAnimationSetting";
import { FlowActiveLayers, FlowMotionProvider } from "./flowLine";
import { readFlowAnimation, writeFlowAnimation, type FlowAnimationMode } from "./preferences";

afterEach(cleanup);

it("stores the choice, defaulting to on for anything unknown", () => {
  const values = new Map<string, string>();
  const storage = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
  expect(readFlowAnimation(storage)).toBe("on");
  writeFlowAnimation(storage, "focus");
  expect(readFlowAnimation(storage)).toBe("focus");
  values.set("audiorouter.ui.flow-animation", "sometimes");
  expect(readFlowAnimation(storage)).toBe("on");
  expect(
    readFlowAnimation({
      getItem: () => {
        throw new Error("blocked");
      },
    }),
  ).toBe("on");
  expect(() =>
    writeFlowAnimation(
      {
        setItem: () => {
          throw new Error("blocked");
        },
      },
      "off",
    ),
  ).not.toThrow();
  expect(readFlowAnimation(null)).toBe("on");
});

it("offers the three choices and explains the selected one", () => {
  const onChange = vi.fn();
  const view = render(<FlowAnimationSetting mode="on" onChange={onChange} />);
  fireEvent.change(screen.getByLabelText("Travelling lights on connections"), { target: { value: "focus" } });
  expect(onChange).toHaveBeenCalledWith("focus");
  view.rerender(<FlowAnimationSetting mode="focus" onChange={onChange} />);
  expect(screen.getByText(/pause while you use another program/)).toBeTruthy();
});

function lights(mode: FlowAnimationMode) {
  return render(
    <svg>
      <FlowMotionProvider mode={mode}>
        <FlowActiveLayers
          id="e1"
          path="M0,0 C10,0 20,0 30,0"
          source={{ x: 0, y: 0 }}
          target={{ x: 30, y: 0 }}
          targetSide="left"
          levelDb={-20}
          core={() => <path className="core" />}
        />
      </FlowMotionProvider>
    </svg>,
  );
}

it("moves comets when on, never when off, and pauses them while unfocused", () => {
  const hasFocus = vi.spyOn(document, "hasFocus").mockReturnValue(true);
  const comets = (view: ReturnType<typeof render>) => view.container.querySelectorAll(".flow-comet").length;
  const on = lights("on");
  expect(comets(on)).toBeGreaterThan(0);
  on.unmount();
  const off = lights("off");
  expect(comets(off)).toBe(0);
  expect(off.container.querySelector(".flow-glow"), "colour and glow stay").not.toBeNull();
  off.unmount();
  const focus = lights("focus");
  expect(comets(focus)).toBeGreaterThan(0);
  hasFocus.mockReturnValue(false);
  act(() => {
    window.dispatchEvent(new Event("blur"));
  });
  expect(comets(focus)).toBe(0);
  hasFocus.mockReturnValue(true);
  act(() => {
    window.dispatchEvent(new Event("focus"));
  });
  expect(comets(focus)).toBeGreaterThan(0);
  hasFocus.mockRestore();
});
