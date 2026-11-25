import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
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

/** Preview fixtures show invented text; they never ship in the app. */
const PREVIEW_FILES = new Set(["devHarness.tsx"]);

/** Units and symbols a label may display beside a value without a catalog key. */
const UNITS = new Set(["db", "dbfs", "hz", "khz", "ms", "s", "%", "kbps", "ch", "px", "x"]);

/**
 * Whether a literal reads as English prose rather than a technical value.
 * Single words, units, product names and paths are deliberately allowed: the
 * audit lists them for review, but a guard that failed on them would need a
 * brittle allowlist of every technical literal.
 */
function readsAsProse(text: string): boolean {
  if (/\$\{/.test(text)) return false;
  // Paths and route examples are technical values.
  if (/^[A-Za-z]:[\\/]/.test(text) || /[\\/]/.test(text)) return false;
  const words = text
    .split(/\s+/)
    .map((word) => word.replace(/^[.,;:!?«»"“”()…-]+|[.,;:!?«»"“”()…-]+$/g, ""))
    .filter(Boolean);
  if (words.length < 2) return false;
  return words.some((word) => /^[a-z][a-z’'-]+$/.test(word) && !UNITS.has(word));
}

describe("plain words in visible text", () => {
  it("finds strings at all: the scan must not pass by reading nothing", () => {
    // Visible text now lives in the English catalog, so the source scan below sees
    // only what is still written inline. Prove both readers find strings.
    const catalog = readFileSync(join(here, "i18n", "locales", "en.json"), "utf8");
    expect(catalog).toContain('"Unsaved edits discarded."');
    expect(visibleStrings("<p>Some text</p>").length).toBeGreaterThan(0);
  });

  it("detects a retired word in each kind of string", () => {
    const strings = visibleStrings(
      `<p>Add this node</p>; setActionMessage("Your draft"); <b title="Physical input">x</b>; formatUiError(e, "Bad node");`,
    );
    expect(strings).toEqual(["Add this node", "Physical input", "Your draft", "Bad node"]);
  });

  it("the English catalog uses the glossary", () => {
    // Messages of the developer panels keep the engine's terms, like their source files.
    const developerKeys = new Set<string>();
    for (const name of DEVELOPER_FILES) {
      const source = readFileSync(join(here, name), "utf8");
      for (const match of source.matchAll(/"([A-Za-z0-9]+(?:\.[A-Za-z0-9-]+)+)"/g)) developerKeys.add(match[1]);
    }
    // Where the exact API term is the point: the identifier a script sends is called a node ID.
    const exact = new Set([
      "workflowUi.nodeId",
      "workflowUi.copyNode",
      "workflowUi.nodeApi",
      "g6.apiBuilderChooseNode",
    ]);
    const catalog = JSON.parse(readFileSync(join(here, "i18n", "locales", "en.json"), "utf8")) as {
      messages: Record<string, unknown>;
    };
    const hits: string[] = [];
    let scanned = 0;
    const visit = (value: unknown, path: string) => {
      if (typeof value === "string") {
        scanned += 1;
        const base = path.replace(/_(zero|one|two|few|many|other)$/, "");
        if (developerKeys.has(base) || exact.has(base)) return;
        // `{{node}}` names a value the code fills in, not a word the person reads.
        const words = value.replace(/\{\{[^}]*\}\}/g, " ");
        for (const [word, advice] of RETIRED) if (word.test(words)) hits.push(`${path}: "${value}" (${advice})`);
      } else if (value && typeof value === "object") {
        for (const [key, child] of Object.entries(value)) visit(child, path ? `${path}.${key}` : key);
      }
    };
    visit(catalog.messages, "");
    expect(scanned).toBeGreaterThan(1000);
    expect(hits).toEqual([]);
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

  it("new visible prose is written through the catalog", () => {
    // A developer adding `>Save changes<` instead of `{tr("…")}` is caught here;
    // units, product names, paths and preview fixtures stay allowed above.
    const hits: string[] = [];
    const files = readdirSync(here).filter(
      (name) => /\.tsx$/.test(name) && !/\.(test|spec)\./.test(name) && !PREVIEW_FILES.has(name),
    );
    for (const name of files) {
      const path = join(here, name);
      const source = ts.createSourceFile(path, readFileSync(path, "utf8"), ts.ScriptTarget.Latest, true);
      const visit = (node: ts.Node) => {
        if (ts.isJsxText(node)) {
          const text = node.text.replace(/\s+/g, " ").trim();
          if (readsAsProse(text)) hits.push(`${name}: "${text}"`);
        }
        ts.forEachChild(node, visit);
      };
      visit(source);
    }
    expect(hits).toEqual([]);
  });

  it("the prose guard does not pass by reading nothing", () => {
    expect(readsAsProse("Save changes")).toBe(true);
    expect(readsAsProse("48 kHz")).toBe(false);
    expect(readsAsProse("AudioRouter")).toBe(false);
    expect(readsAsProse("C:\\Games\\game.exe")).toBe(false);
  });
});
