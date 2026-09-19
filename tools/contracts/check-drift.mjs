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

const declaredSet = new Set(declared);
const mapMethods = (typeName, source) => {
  const map = source.match(new RegExp(`export type ${typeName} =\\s*\\{([\\s\\S]*?)\\n\\};`));
  if (!map) throw new Error(`${typeName} map is missing from contracts/src/index.ts`);
  return [...map[1].matchAll(/^\s*"([^"]+)":/gm)].map((match) => match[1]);
};
for (const typeName of ["MethodParams", "MethodResult"]) {
  const mapped = mapMethods(typeName, contracts);
  const missing = declared.filter((method) => !mapped.includes(method));
  const extra = mapped.filter((method) => !declaredSet.has(method));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `${typeName} drift detected: ${[
        missing.length > 0 ? `missing ${missing.join(", ")}` : "",
        extra.length > 0 ? `extra ${extra.join(", ")}` : "",
      ].filter(Boolean).join("; ")}`,
    );
  }
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

const library = readFileSync(join(repositoryRoot, "ui", "src", "library.ts"), "utf8");
const libraryKinds = [...library.matchAll(/kind:\s*"([^"]+)"/g)].map(
  (match) => match[1],
);
const discoveredProcessors = (schema.processors ?? []).map((processor) => processor.id);
// The UI may expose several endpoint presentations that intentionally map to
// one authoritative node kind (for example Physical output and Existing
// virtual output both use the physicalOutput contract). Processor entries
// must remain one-to-one so a duplicated processor cannot hide catalog drift.
const duplicateLibraryKinds = libraryKinds.filter(
  (kind, index) =>
    !["physicalInput", "physicalOutput"].includes(kind) &&
    libraryKinds.indexOf(kind) !== index,
);
if (duplicateLibraryKinds.length > 0) {
  throw new Error(
    `duplicate processor entries in the UI library: ${[...new Set(duplicateLibraryKinds)].join(", ")}`,
  );
}
const missingProcessors = discoveredProcessors.filter(
  (processor) => !libraryKinds.includes(processor),
);
const extraProcessors = libraryKinds.filter(
  (kind) => ![
    "physicalInput",
    "physicalOutput",
    "testSignal",
    "recorder",
    "mixer",
    "gain",
    "mute",
    "meter",
  ].includes(kind)
    && !discoveredProcessors.includes(kind),
);
if (missingProcessors.length > 0 || extraProcessors.length > 0) {
  throw new Error(
    `processor/UI catalog drift: ${[
      missingProcessors.length > 0 ? `missing from UI: ${missingProcessors.join(", ")}` : "",
      extraProcessors.length > 0 ? `not advertised by CLI: ${extraProcessors.join(", ")}` : "",
    ].filter(Boolean).join("; ")}`,
  );
}

const eventCategoryUnion = contracts.match(
  /export type StateEventCategory\s*=([\s\S]*?);/,
);
if (!eventCategoryUnion) {
  throw new Error("StateEventCategory union is missing from contracts/src/index.ts");
}
const declaredEventCategories = [
  ...eventCategoryUnion[1].matchAll(/"([^\"]+)"/g),
].map((match) => match[1]);
const duplicateEventCategories = declaredEventCategories.filter(
  (category, index) => declaredEventCategories.indexOf(category) !== index,
);
if (duplicateEventCategories.length > 0) {
  throw new Error(
    `duplicate StateEventCategory values: ${[...new Set(duplicateEventCategories)].join(", ")}`,
  );
}
const discoveredEventCategories = schema.events?.stateCategories ?? [];
const declaredEventCategorySet = new Set(declaredEventCategories);
const discoveredEventCategorySet = new Set(discoveredEventCategories);
const missingEventCategories = discoveredEventCategories.filter(
  (category) => !declaredEventCategorySet.has(category),
);
const extraEventCategories = declaredEventCategories.filter(
  (category) => !discoveredEventCategorySet.has(category),
);
if (missingEventCategories.length > 0 || extraEventCategories.length > 0) {
  throw new Error(
    `event category drift detected: ${[
      missingEventCategories.length > 0 ? `missing from TypeScript: ${missingEventCategories.join(", ")}` : "",
      extraEventCategories.length > 0 ? `not discovered by Rust: ${extraEventCategories.join(", ")}` : "",
    ].filter(Boolean).join("; ")}`,
  );
}

console.log(
  `Contract drift check passed: ${declared.length} methods, ${declaredNodeKinds.length} node kinds, ${discoveredProcessors.length} processors, and ${declaredEventCategories.length} event categories match the UI/CLI/Rust catalogs.`,
);
