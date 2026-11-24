/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LiveAutosaveSetting } from "./LiveAutosaveSetting";
import { readLiveAutosave, writeLiveAutosave } from "./preferences";

afterEach(cleanup);

describe("live autosave preference", () => {
  it("defaults on, survives an off choice and tolerates unavailable storage", () => {
    const storage = new Map<string, string>();
    const store = {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => void storage.set(key, value),
    };
    expect(readLiveAutosave(store)).toBe(true);
    writeLiveAutosave(store, false);
    expect(readLiveAutosave(store)).toBe(false);
    writeLiveAutosave(store, true);
    expect(readLiveAutosave(store)).toBe(true);
    expect(readLiveAutosave(null)).toBe(true);
    expect(
      readLiveAutosave({
        getItem: () => {
          throw new Error("denied");
        },
      }),
    ).toBe(true);
  });

  it("switches the setting and describes both states", () => {
    const onChange = vi.fn();
    const view = render(<LiveAutosaveSetting enabled onChange={onChange} />);
    expect(screen.getByText(/saved automatically and applied/)).toBeTruthy();
    fireEvent.click(screen.getByRole("checkbox", { name: /Save parameter changes while audio plays/ }));
    expect(onChange).toHaveBeenCalledWith(false);
    view.rerender(<LiveAutosaveSetting enabled={false} onChange={onChange} />);
    expect(screen.getByText(/stay as unsaved edits/)).toBeTruthy();
    expect(screen.getByText("Off")).toBeTruthy();
  });
});
