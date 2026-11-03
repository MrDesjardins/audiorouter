import { useEffect, useEffectEvent } from "react";

/**
 * Tell the desktop shell whether this page holds unsaved route edits.
 * Closing the window to the tray frees the WebView only when it does not;
 * with unsaved edits the window is just hidden, so nothing is lost. Outside
 * the shell (browser, tests without the bridge) this does nothing. A failed
 * report is passed to `onFailure` (a UI diagnostic): the shell may then free
 * the WebView with unsaved edits, so it must leave a trace.
 */
export function useReportUnsaved(unsaved: boolean, onFailure?: (message: string) => void) {
  const reportFailure = useEffectEvent((error: unknown) => {
    const type = error instanceof Error ? error.name.slice(0, 48) : typeof error;
    onFailure?.(`Unsaved-state report failed (${type})`);
  });
  useEffect(() => {
    const invoke = typeof window === "undefined" ? undefined : window.__TAURI_INTERNALS__?.invoke;
    if (!invoke) return;
    try {
      void Promise.resolve(invoke("set_ui_unsaved", { unsaved })).catch((error: unknown) => reportFailure(error));
    } catch (error) {
      reportFailure(error);
    }
  }, [unsaved]);
}
