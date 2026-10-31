import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const result = spawnSync('cargo', ['run','-q','-p','audiorouter-control','--example','tool_guide_catalog'], { cwd: root, encoding: 'utf8' });
if (result.status !== 0) throw new Error(result.stderr);
const catalog = JSON.parse(result.stdout);
catalog.sourceHashes = {};
for (const source of ['crates/control/src/catalog.rs','crates/domain/src/voice_changer.rs']) {
  const contract = readFileSync(resolve(root, source), 'utf8').replace(/\r\n/g, '\n').split('#[cfg(test)]')[0];
  catalog.sourceHashes[source] = createHash('sha256').update(contract).digest('hex');
}
writeFileSync(resolve(root, 'tools/site/tool-guide-catalog.json'), JSON.stringify(catalog, null, 2) + '\n');
console.log(`Exported authoritative discovery for ${catalog.nodeTypes.length} node kinds.`);
