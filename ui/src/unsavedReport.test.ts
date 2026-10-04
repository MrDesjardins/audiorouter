/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { renderHook } from "@testing-library/react";
import { useReportUnsaved } from "./unsavedReport";

afterEach(() => { delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__; });

it("reports each change of the unsaved state to the shell, once", () => {
  const invoke = vi.fn(() => Promise.resolve());
  Object.assign(window, { __TAURI_INTERNALS__: { invoke } });
  const { rerender } = renderHook(({ unsaved }) => useReportUnsaved(unsaved), { initialProps: { unsaved: false } });
  rerender({ unsaved: false });
  rerender({ unsaved: true });
  rerender({ unsaved: true });
  rerender({ unsaved: false });
  expect(invoke.mock.calls).toEqual([
    ["set_ui_unsaved", { unsaved: false }],
    ["set_ui_unsaved", { unsaved: true }],
    ["set_ui_unsaved", { unsaved: false }],
  ]);
});

it("does nothing outside the desktop shell", () => {
  expect(() => renderHook(() => useReportUnsaved(true))).not.toThrow();
});
