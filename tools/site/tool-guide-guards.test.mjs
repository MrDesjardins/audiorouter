import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  nodeTypeFor,
  selectedKinds,
  exampleNumber,
  exampleOption,
} from "./tool-guide-guards.mjs";
const read = (name) => JSON.parse(readFileSync(new URL(name, import.meta.url), "utf8"));
const catalog = read("./tool-guide-catalog.json");
const manifest = read("./tool-guide-property-manifest.json");

test("every manifest tool resolves to one catalog node type", () => {
  for (const tool of manifest.tools) {
    const { wireType, version, parameters } = nodeTypeFor(tool.kind, catalog);
    assert.equal(version, 1, tool.kind);
    assert.ok(Array.isArray(parameters), tool.kind);
    if (tool.kind !== "group") assert.ok(wireType.endsWith("@1"), tool.kind);
  }
  assert.equal(nodeTypeFor("bassTreble", catalog).wireType, "bass-treble@1");
});

test("a kind missing from the catalog throws instead of recording an empty panel", () => {
  assert.throws(() => nodeTypeFor("noSuchTool", catalog), /noSuchTool \(no-such-tool\) is not in tool-guide-catalog/);
  const twoVersions = { nodeTypes: [{ type: "gain@1" }, { type: "gain@2" }] };
  assert.throws(() => nodeTypeFor("gain", twoVersions), /several node types/);
  assert.equal(nodeTypeFor("gain", { nodeTypes: [{ type: "gain@3", parameters: [] }] }).version, 3);
});

test("GUIDE_KIND names known tools", () => {
  const known = ["gain", "gate"];
  assert.equal(selectedKinds(undefined, known), null);
  assert.deepEqual(selectedKinds("gain, gate", known), ["gain", "gate"]);
  assert.throws(() => selectedKinds("", known), /names no tool/);
  assert.throws(() => selectedKinds("gain,bogus", known), /unknown tools: bogus/);
});

test("number examples are finite and differ from the current value", () => {
  const ranged = { minimum: -60, maximum: 12 };
  assert.equal(exampleNumber(ranged, 0, -12, "gain--gainDb"), -12);
  assert.equal(exampleNumber(ranged, 0, 40, "x"), 12);
  assert.equal(exampleNumber(ranged, 12, 40, "x"), -60);
  assert.equal(exampleNumber(ranged, -60, undefined, "x"), 12);
  assert.equal(exampleNumber({ minimum: null, maximum: null }, 0, 5, "x"), 5);
  assert.throws(() => exampleNumber({}, 3, undefined, "a--b"), /No safe example choice for a--b/);
  assert.throws(() => exampleNumber({ maximum: 10 }, 5, undefined, "a--b"), /No safe example choice/);
  assert.throws(() => exampleNumber({ minimum: 1, maximum: 1 }, 1, undefined, "a--b"), /No safe example choice/);
});

test("option examples pick another option or throw", () => {
  assert.equal(exampleOption({ options: ["a", "b"] }, "a", "x"), "b");
  assert.throws(() => exampleOption({ options: ["a"] }, "a", "k--p"), /No safe example choice for k--p/);
});

test("number examples match the previous choice for every manifest property", () => {
  const legacy = (property, current, preferred) => {
    const min = property.minimum,
      max = property.maximum;
    let next = preferred ?? (current === min ? max : min);
    next = Math.min(max, Math.max(min, next));
    if (next === current) next = current === min ? max : min;
    return next;
  };
  for (const tool of manifest.tools)
    for (const property of tool.properties)
      if (property.type === "number" && Number.isFinite(property.minimum) && Number.isFinite(property.maximum))
        for (const preferred of [undefined, property.minimum, property.maximum])
          assert.equal(
            exampleNumber(property, property.default, preferred, property.key),
            legacy(property, property.default, preferred),
            `${tool.kind}.${property.key}`,
          );
});
