// Exercise the actual HTML and assets without opening a server or changing DNS.
import { createRequire } from 'node:module';
import { readFile, readdir } from 'node:fs/promises';
import { dirname, extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const site = dirname(fileURLToPath(import.meta.url));
const require = createRequire(resolve(site, '../ui/package.json'));
const { chromium } = require('playwright');
const browser = await chromium.launch({ channel: 'msedge', headless: true });
const origin = 'https://audiorouter.test';
const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.mp4': 'video/mp4' };
try {
  const page = await browser.newPage();
  const violations = [];
  const errors = [];
  await page.exposeFunction('recordViolation', event => violations.push(event));
  await page.addInitScript(() => document.addEventListener('securitypolicyviolation', event => {
    window.recordViolation({ directive: event.effectiveDirective, blocked: event.blockedURI });
  }));
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', async route => {
    const url = new URL(route.request().url());
    if (url.origin !== origin) {
      // External services are stubbed: this verifies CSP permissions, not uptime.
      if (url.hostname === 'api.github.com') return route.fulfill({ json: [] });
      return route.fulfill({ body: '', contentType: 'text/plain' });
    }
    const file = resolve(site, '.' + decodeURIComponent(url.pathname));
    assert.ok(file.startsWith(site + sep), 'request must remain inside site');
    try {
      await route.fulfill({ body: await readFile(file), contentType: types[extname(file)] ?? 'application/octet-stream' });
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
      await route.fulfill({ status: 404, body: 'Not found' });
    }
  });
  const pages = (await readdir(site, { recursive: true })).filter(name => name.endsWith('.html'));
  for (const name of pages) {
    console.log(`Checking ${name}`);
    await page.goto(`${origin}/${name.replaceAll('\\', '/')}`, { waitUntil: 'domcontentloaded' });
    await page.waitForFunction(() => !document.querySelector('[data-header]') || document.querySelector('[data-header]').children.length > 0);
  }
  assert.deepEqual(errors, [], 'local scripts must run without page errors');
  assert.deepEqual(violations, [], 'normal pages must not violate CSP');
  await page.evaluate(async () => {
    const script = document.createElement('script');
    script.textContent = 'window.injectedScriptRan = true';
    document.body.append(script);
    try { await fetch('https://blocked.example/collect'); } catch { /* expected */ }
  });
  await page.waitForFunction(() => window.injectedScriptRan !== true);
  await page.waitForTimeout(100);
  assert.equal(await page.evaluate(() => window.injectedScriptRan), undefined);
  assert.ok(violations.some(event => event.directive === 'script-src-elem'));
  assert.ok(violations.some(event => event.directive === 'connect-src'));
  console.log(`PASS: ${pages.length} pages load local scripts without CSP violations; inline script and unapproved fetch blocked (Edge). External services stubbed; playback not qualified.`);
} finally {
  await browser.close();
}
