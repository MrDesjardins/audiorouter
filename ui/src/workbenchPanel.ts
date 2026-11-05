import type { KeyboardEvent } from "react";

/** Return from the out-of-order Properties overlay to its controlling tab. */
export function returnToPropertiesTab(event: KeyboardEvent<HTMLElement>) {
  if (event.key === "Tab" && !event.shiftKey) {
    const controls = Array.from(
      event.currentTarget.querySelectorAll<HTMLElement>(
        "button, input, select, textarea, a[href], summary, [tabindex]",
      ),
    ).filter(
      (element) => element.tabIndex >= 0 && !element.matches(":disabled") && element.getClientRects().length > 0,
    );
    if (event.target === controls.at(-1) || (controls.length === 0 && event.target === event.currentTarget)) {
      const tab = document.getElementById("workbench-tab-properties");
      if (tab) {
        event.preventDefault();
        tab.dataset.propertiesExit = "true";
        tab.focus();
      }
    }
  }
  if (event.key === "Tab" && event.shiftKey && event.target === event.currentTarget) {
    const tab = document.getElementById("workbench-tab-properties");
    if (tab) {
      event.preventDefault();
      tab.focus();
    }
  }
}
