import { useEffect } from "react";

/**
 * Tell the desktop shell whether this page holds unsaved route edits.
 * Closing the window to the tray frees the WebView only when it does not;
 * with unsaved edits the window is just hidden, so nothing is lost. Outside
 * the shell (browser, tests without the bridge) this does nothing.
 */
export function useReportUnsaved(unsaved: boolean) {
  useEffect(() => {
    const invoke = typeof window === "undefined" ? undefined : window.__TAURI_INTERNALS__?.invoke;
    if (invoke) void Promise.resolve(invoke("set_ui_unsaved", { unsaved })).catch(() => {});
  }, [unsaved]);
}
