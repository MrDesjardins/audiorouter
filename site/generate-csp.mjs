// GitHub Pages cannot configure response headers. Apply the supported HTML CSP
// before any resource tags; see README.md for the remaining header limitation.
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const policy = "default-src 'self'; script-src 'self'; script-src-attr 'none'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data:; media-src 'self' blob:; connect-src 'self' https://api.github.com; frame-src https://www.youtube.com https://github.com/sponsors/MrDesjardins/button; object-src 'none'; base-uri 'none'; form-action 'none'";

export function applyCsp(html) {
  const without = html.replace(/<meta\b[^>]*http-equiv\s*=\s*["']Content-Security-Policy["'][^>]*>\s*/gi, '');
  if (!/<head\s*>/i.test(without)) throw Error('Page has no plain <head> element');
  // Keep encoding declaration first. No scripts/styles/preloads may precede CSP.
  return without.replace(/(<head\s*>\s*(?:<meta\s+charset\s*=\s*["']?utf-8["']?\s*\/?>\s*)?)/i,
    `$1<meta http-equiv="Content-Security-Policy" content="${policy}">`);
}

async function generate() {
  const site = dirname(fileURLToPath(import.meta.url));
  const check = process.argv.includes('--check');
  const stale = [];
  for (const name of (await readdir(site, { recursive: true })).filter(name => name.endsWith('.html')).sort()) {
    const file = resolve(site, name);
    const html = await readFile(file, 'utf8');
    const next = applyCsp(html);
    if (next === html) continue;
    if (check) stale.push(name);
    else await writeFile(file, next);
  }
  if (stale.length) throw Error(`Missing/stale CSP: ${stale.join(', ')}. Run node site/generate-csp.mjs`);
  console.log(check ? 'All HTML CSP policies are current.' : 'Generated HTML CSP policies.');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await generate();
