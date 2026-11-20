// Report candidate UI strings for review. The broader mode also lists
// literals in helpers, where display text and protocol values can coexist.
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import ts from "typescript";

const root = "src";
const includeHelpers = process.argv.includes("--all");
let files = 0;
let candidates = 0;
const displayAttributes = new Set([
  "aria-label",
  "aria-description",
  "aria-roledescription",
  "alt",
  "title",
  "placeholder",
  "label",
  "summary",
]);

function scan(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "generated") scan(path);
      continue;
    }
    if (!/\.tsx?$/.test(path) || /\.(test|spec|d)\.tsx?$/.test(path)) continue;
    files += 1;
    const source = ts.createSourceFile(path, readFileSync(path, "utf8"), ts.ScriptTarget.Latest, true);
    const matches = [];
    function visit(node) {
      const parent = node.parent;
      const attribute = parent && ts.isJsxAttribute(parent) ? parent : undefined;
      const jsxText = ts.isJsxText(node);
      const literal = ts.isStringLiteralLike(node) || ts.isTemplateExpression(node);
      const directlyRendered = parent && ts.isJsxExpression(parent) && !ts.isJsxAttribute(parent.parent);
      if (
        (jsxText || literal) &&
        (jsxText ||
          directlyRendered ||
          (attribute && displayAttributes.has(attribute.name.getText(source))) ||
          includeHelpers) &&
        !ts.isImportDeclaration(node.parent) &&
        !ts.isExportDeclaration(node.parent)
      ) {
        const value = jsxText || ts.isStringLiteralLike(node) ? node.text : node.getText(source);
        const display = value.replace(/\s+/g, " ").trim();
        if (/[A-Za-z]{2}/.test(display)) {
          const { line } = source.getLineAndCharacterOfPosition(node.getStart(source));
          matches.push(`${relative(root, path)}:${line + 1}: ${display}`);
        }
      }
      ts.forEachChild(node, visit);
    }
    visit(source);
    candidates += matches.length;
    for (const match of matches) console.log(match);
  }
}

scan(root);
console.error(`Scanned ${files} source files, found ${candidates} candidates for review.`);
