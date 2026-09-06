import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const workspace = path.resolve(scriptDirectory, "../..");
function collectMarkdown(directory, result = new Set()) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (entry.name === ".git" || entry.name === "target" || entry.name === "node_modules") continue;
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) collectMarkdown(entryPath, result);
    else if (entry.isFile() && entry.name.toLowerCase().endsWith(".md")) result.add(path.resolve(entryPath));
  }
  return result;
}

function slugify(heading) {
  return heading
    .replace(/<[^>]*>/g, "")
    .replace(/[`*_~]/g, "")
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s-]/gu, "")
    .replace(/\s+/g, "-");
}

function headings(content) {
  const anchors = new Set();
  for (const match of content.matchAll(/\bid\s*=\s*["']([^"']+)["']/gi)) anchors.add(match[1].toLowerCase());
  const duplicates = new Map();
  for (const line of content.split(/\r?\n/)) {
    const match = /^(#{1,6})\s+(.+?)\s*#*\s*$/.exec(line);
    if (!match) continue;
    const base = slugify(match[2]);
    const count = duplicates.get(base) ?? 0;
    duplicates.set(base, count + 1);
    anchors.add(count === 0 ? base : `${base}-${count}`);
  }
  return anchors;
}

const topLevelMarkdown = fs.readdirSync(workspace, { withFileTypes: true })
  .filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith(".md"))
  .map((entry) => path.resolve(workspace, entry.name));
const files = [...new Set([...topLevelMarkdown, ...collectMarkdown(path.join(workspace, "docs"))])].sort();
const contents = new Map(files.map((file) => [file, fs.readFileSync(file, "utf8")]));
const errors = [];
let linkCount = 0;

function methodNamesFromSource(source) {
  return [...source.matchAll(/ApiMethodSpec\s*\{\s*name:\s*"([^"]+)"/g)].map((match) => match[1]);
}

function methodNamesFromReference(reference) {
  const section = reference.match(/## Methods\r?\n([\s\S]*?)(?:\r?\n## |$)/)?.[1] ?? "";
  return [...section.matchAll(/^\| `([^`]+)` \|/gm)].map((match) => match[1]);
}

const apiSourcePath = path.join(workspace, "crates", "domain", "src", "lib.rs");
const apiReferencePath = path.join(workspace, "docs", "operations", "api-reference.md");
if (fs.existsSync(apiSourcePath) && fs.existsSync(apiReferencePath)) {
  const sourceMethods = methodNamesFromSource(fs.readFileSync(apiSourcePath, "utf8"));
  const referenceMethods = methodNamesFromReference(fs.readFileSync(apiReferencePath, "utf8"));
  const sourceSet = new Set(sourceMethods);
  const referenceSet = new Set(referenceMethods);
  for (const method of sourceSet) {
    if (!referenceSet.has(method)) errors.push(`docs/operations/api-reference.md: missing API method ${method}`);
  }
  for (const method of referenceSet) {
    if (!sourceSet.has(method)) errors.push(`docs/operations/api-reference.md: undocumented API method ${method}`);
  }
  if (sourceMethods.length !== sourceSet.size) errors.push("crates/domain/src/lib.rs: duplicate API method definitions");
  if (referenceMethods.length !== referenceSet.size) errors.push("docs/operations/api-reference.md: duplicate API method rows");
}

for (const [file, content] of contents) {
  const fenceCount = (content.match(/^\s*(```|~~~)/gm) ?? []).length;
  if (fenceCount % 2 !== 0) errors.push(`${path.relative(workspace, file)}: unbalanced code fences`);

  const linkPattern = /\[[^\]]*\]\(([^)]+)\)/g;
  for (const match of content.matchAll(linkPattern)) {
    let target = match[1].trim().split(/\s+['"]/, 1)[0];
    if (target.startsWith("<") && target.includes(">")) target = target.slice(1, target.indexOf(">"));
    if (/^(?:https?:|mailto:|#?$)/i.test(target)) continue;
    linkCount += 1;
    const [relativeTarget, fragment] = target.split("#", 2);
    const destination = path.resolve(path.dirname(file), relativeTarget || path.basename(file));
    if (!fs.existsSync(destination)) {
      errors.push(`${path.relative(workspace, file)}: missing link target ${target}`);
      continue;
    }
    if (fragment) {
      const destinationContent = fs.statSync(destination).isFile() ? fs.readFileSync(destination, "utf8") : "";
      if (!headings(destinationContent).has(decodeURIComponent(fragment).toLowerCase())) {
        errors.push(`${path.relative(workspace, file)}: missing heading anchor ${target}`);
      }
    }
  }
}

if (errors.length) {
  for (const error of errors) console.error(error);
  process.exitCode = 1;
} else {
  console.log(`Documentation validation passed: ${files.length} Markdown files, ${linkCount} local links.`);
}
