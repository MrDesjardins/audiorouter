/** @vitest-environment jsdom */
import { afterEach, expect, it } from "vitest";
import { keepNativeContextMenu } from "./contextMenuPolicy";

afterEach(() => {
  document.body.replaceChildren();
  window.getSelection()?.removeAllRanges();
});
it("suppresses browser commands on app chrome and non-text controls", () => {
  document.body.innerHTML = '<button>Map</button><input type="checkbox"><input type="range">';
  for (const element of document.body.children) expect(keepNativeContextMenu(element, null)).toBe(false);
});
it("retains field editing, link actions and diagnostic copying", () => {
  document.body.innerHTML =
    '<input><input type="number"><textarea></textarea><a href="/guide">Guide</a><pre>Error</pre><div contenteditable="true"><span>Edit</span></div>';
  for (const element of document.body.children) expect(keepNativeContextMenu(element, null)).toBe(true);
  expect(keepNativeContextMenu(document.querySelector("span"), null)).toBe(true);
});
it("retains the copy menu for selected text without enabling it on unrelated chrome", () => {
  document.body.innerHTML = "<p>Error details</p><button>Map</button>";
  const paragraph = document.querySelector("p")!;
  const range = document.createRange();
  range.selectNodeContents(paragraph);
  const selection = window.getSelection()!;
  selection.addRange(range);
  expect(keepNativeContextMenu(paragraph, selection)).toBe(true);
  expect(keepNativeContextMenu(document.querySelector("button"), selection)).toBe(false);
  selection.removeAllRanges();
  expect(keepNativeContextMenu(paragraph, selection)).toBe(false);
});
