import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const catalog = JSON.parse(readFileSync(resolve(root, 'tools/site/tool-guide-catalog.json'), 'utf8'));
const inventory = JSON.parse(readFileSync(resolve(root, 'tools/site/tool-guide-property-manifest.json'), 'utf8'));
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
    // These three compatibility aliases are edited by the current first-point
    // controls; they are not additional fields in the Properties window.
    if (kind === 'parametricEq') name = ({ frequencyHz: 'band0FrequencyHz', gainDb: 'band0GainDb', q: 'band0Q' })[name] ?? name;
    const property = tool.properties.find(property => property.key === name);
    assert(property, `${kind}.${name}: public parameter missing from recording inventory`);
    assert.equal(property.type === 'enum' ? 'string' : property.type, parameter.type, `${kind}.${name}: type differs from discovery`);
    for (const field of ['minimum','maximum','default','unit']) if (parameter[field] !== undefined) assert.deepEqual(property[field], parameter[field], `${kind}.${name}: ${field} differs from discovery`);
    if (parameter.enum) assert.deepEqual(property.options?.slice().sort(), parameter.enum.slice().sort(), `${kind}.${name}: choices differ from discovery`);
    parameters++;
  }
}
assert.equal(catalog.nodeTypes.length, inventory.nodeKindCount);
console.log(`PASS: authoritative discovery matches ${inventory.nodeKindCount} node kinds and ${parameters} public parameter descriptors, including the documented EQ aliases.`);
