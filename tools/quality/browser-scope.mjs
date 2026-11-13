import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { resolve, dirname, basename } from 'node:path';

// Follow helper imports too. Unknown module shapes select conservatively.
export function backendSpecs(directory) {
  const visit = (file, seen = new Set()) => {
    if (basename(file) === 'real-backend.ts') return true;
    if (seen.has(file)) return false;
    seen.add(file);
    const original = readFileSync(file, 'utf8');
    if (/real-backend|\b(?:require|import)\s*\(/.test(original)) return true;
    const source = original.replace(/\b(?:import|export)\s+type\s+[\s\S]*?\bfrom\s*["'][^"']+["'];?/g, '');
    return [...source.matchAll(/(?:\bfrom\s*|\bimport\s*)["']([^"']+)["']/g)].some(([, specifier]) => {
      if (!specifier.startsWith('.')) return !specifier.startsWith('node:') && specifier !== '@playwright/test';
      const base = resolve(dirname(file), specifier);
      const target = [base, `${base}.ts`, `${base}.tsx`, `${base}.mjs`, resolve(base, 'index.ts')].find(candidate => existsSync(candidate) && /\.(ts|tsx|mjs)$/.test(candidate));
      return !target || visit(target, seen);
    });
  };
  const selected = readdirSync(directory, { recursive: true })
    .filter(file => file.endsWith('.pw.ts') && visit(resolve(directory, file)))
    .map(file => file.replaceAll('\\', '/'));
  if (!selected.length) throw Error('Backend browser selection is empty');
  return selected;
}
