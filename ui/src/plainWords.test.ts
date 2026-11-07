import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * The words the app shows people. See "Glossary" in
 * docs/plans/active/ui-quality-review.md: a route is the graph on the canvas, a
 * tool is a card on it, unsaved edits are the changes not saved yet, and an
 * input or output device is a real device. Backend and code keep their own terms.
 */
const RETIRED: readonly [RegExp, string][] = [
  [/\bdrafts?\b/i, "say unsaved edits, or just the action"],
  [/\bnodes?\b/i, "say tool"],
  [/\bphysical (input|output)\b/i, "say input device or output device"],
];

/** Developer diagnostics in Settings, where the exact engine terms are the point. */
const DEVELOPER_FILES = new Set(["NativeDevicePanels.tsx", "AdvancedPanels.tsx", "devHarness.tsx", "routeHarness.tsx"]);

const here = dirname(fileURLToPath(import.meta.url));

/** Replace each template `${…}`, nested ones included: that is code, not text. */
function withoutInterpolations(source: string): string {
  let result = "";
  let index = 0;
  while (index < source.length) {
    if (source.startsWith("${", index)) {
      let depth = 1;
      index += 2;
      while (index < source.length && depth > 0) {
        if (source[index] === "{") depth += 1;
        if (source[index] === "}") depth -= 1;
        index += 1;
      }
      result += " ";
    } else {
      result += source[index];
      index += 1;
    }
  }
  return result;
}

/** Text in JSX between tags, and in the props and calls that reach the screen. */
function visibleStrings(source: string): string[] {
  const found: string[] = [];
  const add = (text: string | undefined) => {
    const cleaned = text?.trim();
    if (cleaned) found.push(cleaned);
  };
  const withoutComments = withoutInterpolations(source)
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
  for (const match of withoutComments.matchAll(/>\s*([A-Z][^<>{}]*?)\s*</g)) add(match[1]);
  for (const match of withoutComments.matchAll(/\b(?:title|placeholder|aria-label|label)=\{?["`]([^"`]+)["`]/g)) {
    add(match[1]);
  }
  for (const match of withoutComments.matchAll(
    /\b(?:setActionMessage|setMessage|setPreviewMessage|onMessage)\(\s*["`]([^"`]+)["`]/g,
  )) {
    add(match[1]);
  }
  for (const match of withoutComments.matchAll(/\bformatUiError\(\s*\w+,\s*["`]([^"`]+)["`]/g)) add(match[1]);
  return found;
}

describe("plain words in visible text", () => {
  it("finds strings at all: the scan must not pass by reading nothing", () => {
    const sample = visibleStrings(readFileSync(join(here, "App.tsx"), "utf8"));
    expect(sample.length).toBeGreaterThan(50);
    expect(sample).toContain("Unsaved edits discarded.");
  });

  it("detects a retired word in each kind of string", () => {
    const strings = visibleStrings(
      `<p>Add this node</p>; setActionMessage("Your draft"); <b title="Physical input">x</b>; formatUiError(e, "Bad node");`,
    );
    expect(strings).toEqual(["Add this node", "Physical input", "Your draft", "Bad node"]);
  });

  it("the daily interface uses the glossary", () => {
    const hits: string[] = [];
    const files = readdirSync(here).filter(
      (name) => /\.tsx?$/.test(name) && !/\.test\./.test(name) && !DEVELOPER_FILES.has(name),
    );
    for (const name of files) {
      for (const text of visibleStrings(readFileSync(join(here, name), "utf8"))) {
        for (const [word, advice] of RETIRED) if (word.test(text)) hits.push(`${name}: "${text}" (${advice})`);
      }
    }
    expect(hits).toEqual([]);
  });
});
