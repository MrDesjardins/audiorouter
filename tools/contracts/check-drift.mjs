import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const toolsRoot = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = join(toolsRoot, "..", "..");
const contracts = readFileSync(
  join(repositoryRoot, "contracts", "src", "index.ts"),
  "utf8",
);
const union = contracts.match(
  /export type ImplementedMethod\s*=([\s\S]*?);/,
);
if (!union) {
  throw new Error("ImplementedMethod union is missing from contracts/src/index.ts");
}

const declared = [...union[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
const duplicateDeclared = declared.filter(
  (method, index) => declared.indexOf(method) !== index,
);
if (duplicateDeclared.length > 0) {
  throw new Error(
    `duplicate methods in ImplementedMethod: ${[...new Set(duplicateDeclared)].join(", ")}`,
  );
}

let schemaText;
try {
  schemaText = execFileSync(
    process.platform === "win32" ? "cargo.exe" : "cargo",
    ["run", "--quiet", "-p", "audiorouter-cli", "--", "--json", "schema"],
    { cwd: repositoryRoot, encoding: "utf8", maxBuffer: 4 * 1024 * 1024 },
  );
} catch (error) {
  throw new Error(`unable to obtain authoritative CLI schema: ${error.message}`);
}

let schema;
try {
  schema = JSON.parse(schemaText);
} catch (error) {
  throw new Error(`CLI schema was not valid JSON: ${error.message}`);
}
const discovered = (schema.methods ?? []).map((method) => method.name);
const declaredSet = new Set(declared);
const discoveredSet = new Set(discovered);
const missing = discovered.filter((method) => !declaredSet.has(method));
const extra = declared.filter((method) => !discoveredSet.has(method));
if (missing.length > 0 || extra.length > 0) {
  const details = [];
  if (missing.length > 0) details.push(`missing from TypeScript: ${missing.join(", ")}`);
  if (extra.length > 0) details.push(`not discovered by CLI: ${extra.join(", ")}`);
  throw new Error(`contract method drift detected (${details.join("; ")})`);
}

if (schema.protocolVersion?.major !== 1) {
  throw new Error(`unexpected CLI protocol major: ${schema.protocolVersion?.major}`);
}

const nodeKindUnion = contracts.match(/export type NodeKind\s*=([\s\S]*?);/);
if (!nodeKindUnion) {
  throw new Error("NodeKind union is missing from contracts/src/index.ts");
}
const declaredNodeKinds = [...nodeKindUnion[1].matchAll(/"([^"]+)"/g)].map(
  (match) => match[1],
);
const toWireNodeType = (kind) => kind.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
const discoveredNodeKinds = (schema.nodeTypes ?? []).map((node) => {
  const type = String(node.type);
  return type.endsWith("@1") ? type.slice(0, -2) : type;
});
const declaredWireNodeKinds = declaredNodeKinds.map(toWireNodeType);
const missingNodeKinds = declaredWireNodeKinds.filter(
  (kind) => !discoveredNodeKinds.includes(kind),
);
const extraNodeKinds = discoveredNodeKinds.filter(
  (kind) => !declaredWireNodeKinds.includes(kind),
);
if (missingNodeKinds.length > 0 || extraNodeKinds.length > 0) {
  const details = [];
  if (missingNodeKinds.length > 0) details.push(`missing from CLI: ${missingNodeKinds.join(", ")}`);
  if (extraNodeKinds.length > 0) details.push(`not declared in TypeScript: ${extraNodeKinds.join(", ")}`);
  throw new Error(`contract node-kind drift detected (${details.join("; ")})`);
}

console.log(
  `Contract drift check passed: ${declared.length} methods and ${declaredNodeKinds.length} node kinds match the CLI catalog.`,
);
