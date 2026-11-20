// Report candidate UI strings for review. The broader mode also lists
// literals in helpers, where display text and protocol values can coexist.
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import ts from "typescript";

const root = "src";
const includeHelpers = process.argv.includes("--all");
const tsxOnly = process.argv.includes("--tsx");
const byFile = process.argv.includes("--by-file");
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

function rendersLiteral(node) {
  let child = node;
  for (let parent = node.parent; parent; child = parent, parent = parent.parent) {
    if (ts.isJsxAttribute(parent)) return displayAttributes.has(parent.name.getText());
    if (ts.isJsxExpression(parent) && !ts.isJsxAttribute(parent.parent)) return true;
    if (
      ts.isParenthesizedExpression(parent) ||
      (ts.isConditionalExpression(parent) && child !== parent.condition) ||
      (ts.isBinaryExpression(parent) &&
        child === parent.right &&
        [
          ts.SyntaxKind.QuestionQuestionToken,
          ts.SyntaxKind.BarBarToken,
          ts.SyntaxKind.AmpersandAmpersandToken,
        ].includes(parent.operatorToken.kind)) ||
      ts.isJsxExpression(parent)
    )
      continue;
    return false;
  }
  return false;
}

function scan(directory) {
  const entries = readdirSync(directory, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name));
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "generated") scan(path);
      continue;
    }
    if (!/\.tsx?$/.test(path) || /\.(test|spec|d)\.tsx?$/.test(path)) continue;
    if (tsxOnly && !path.endsWith(".tsx")) continue;
    files += 1;
    const source = ts.createSourceFile(path, readFileSync(path, "utf8"), ts.ScriptTarget.Latest, true);
    const matches = [];
    function visit(node) {
      const jsxText = ts.isJsxText(node);
      const literal = ts.isStringLiteralLike(node) || ts.isTemplateExpression(node);
      if (
        (jsxText || literal) &&
        (jsxText || rendersLiteral(node) || includeHelpers) &&
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
    if (byFile) console.log(`\n${relative(root, path)} (${matches.length} candidates)`);
    for (const match of matches) console.log(match);
  }
}

scan(root);
console.error(`Scanned ${files} source files, found ${candidates} candidates for review.`);
