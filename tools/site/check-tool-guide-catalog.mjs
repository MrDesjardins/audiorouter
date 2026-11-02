import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { guideWireType } from './tool-guide-guards.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const catalog = JSON.parse(readFileSync(resolve(root, 'tools/site/tool-guide-catalog.json'), 'utf8'));
const inventory = JSON.parse(readFileSync(resolve(root, 'tools/site/tool-guide-property-manifest.json'), 'utf8'));
const bindings = JSON.parse(readFileSync(resolve(root, 'tools/site/tool-guide-api-bindings.json'), 'utf8'));
const reachedBindings = new Set();
let apiOnly = 0;
for (const [source, expected] of Object.entries(catalog.sourceHashes)) {
  const contract = readFileSync(resolve(root, source), 'utf8').replace(/\r\n/g, '\n').split('#[cfg(test)]')[0];
  assert.equal(createHash('sha256').update(contract).digest('hex'), expected, `Discovery snapshot is stale: ${source}; run export-tool-guide-catalog.mjs and audit coverage`);
}
let parameters = 0;
for (const node of catalog.nodeTypes) {
  const kind = node.type.split('@')[0].replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
  const tool = inventory.tools.find(tool => tool.kind === kind);
  assert(tool, `Missing guide for public node kind ${kind}`);
  for (const parameter of node.parameters) {
    let name = parameter.namePattern ?? parameter.name;
    const bindingName = `${kind}.${name}`;
    const binding = bindings[bindingName];
    if (binding) {
      reachedBindings.add(bindingName);
      assert(binding.reason?.length > 20, `${bindingName}: binding needs its UI evidence/reason`);
      if (binding.apiOnly) { apiOnly++; parameters++; continue; }
      assert(tool.properties.some(property => property.key === binding.property), `${bindingName}: mapped UI control is missing`);
      parameters++;
      continue;
    }
    // Each numbered control family is exercised by its first representative.
    name = name.replaceAll('<N>', '1');
    // These three compatibility aliases are edited by the current first-point
    // controls; they are not additional fields in the Properties window.
    if (kind === 'parametricEq') name = ({ frequencyHz: 'band0FrequencyHz', gainDb: 'band0GainDb', q: 'band0Q' })[name] ?? name;
    const property = tool.properties.find(property => property.key === name);
    assert(property, `${kind}.${name}: public parameter missing from recording inventory`);
    assert.equal(guideWireType(property.type), parameter.type, `${kind}.${name}: type differs from discovery`);
    for (const field of ['minimum','maximum','default','unit']) if (parameter[field] !== undefined) assert.deepEqual(property[field], parameter[field], `${kind}.${name}: ${field} differs from discovery`);
    if (parameter.enum) assert.deepEqual(property.options?.slice().sort(), parameter.enum.slice().sort(), `${kind}.${name}: choices differ from discovery`);
    parameters++;
  }
}
assert.equal(catalog.nodeTypes.length, inventory.nodeKindCount);
assert.deepEqual([...reachedBindings].sort(), Object.keys(bindings).sort(), 'Stale or misspelled API/UI binding');
console.log(`PASS: authoritative discovery accounts for ${inventory.nodeKindCount} node kinds and ${parameters} public parameter descriptors (${apiOnly} explicitly API-only), including selector bindings and representative control families.`);
