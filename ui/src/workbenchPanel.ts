import type { KeyboardEvent } from "react";

/** Return from the out-of-order Properties overlay to its controlling tab. */
export function returnToPropertiesTab(event: KeyboardEvent<HTMLElement>) {
  if (event.key === "Tab" && event.shiftKey && event.target === event.currentTarget) {
    const tab = document.getElementById("workbench-tab-properties");
    if (tab) {
      event.preventDefault();
      tab.focus();
    }
  }
}
