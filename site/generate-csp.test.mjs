import test from 'node:test';
import assert from 'node:assert/strict';
import { applyCsp, policy } from './generate-csp.mjs';

test('CSP precedes resource loading and preserves UTF-8 encoding', () => {
  const input = '<!doctype html><html><head><meta charset="utf-8"><script src="assets/site.js"></script></head></html>';
  const output = applyCsp(input);
  assert.ok(output.indexOf('charset=') < output.indexOf('http-equiv='));
  assert.ok(output.indexOf('http-equiv=') < output.indexOf('<script'));
  assert.equal(applyCsp(output), output);
});

test('replaces an obsolete policy instead of accumulating policies', () => {
  const output = applyCsp('<head><meta http-equiv="Content-Security-Policy" content="default-src *"><link href="x.css"></head>');
  assert.equal(output.match(/http-equiv=/g).length, 1);
  assert.ok(output.includes(policy));
  assert.ok(!output.includes('default-src *'));
});

test('scripts cannot use inline execution, eval or third-party hosts', () => {
  assert.ok(policy.includes("script-src 'self';"));
  assert.ok(policy.includes("script-src-attr 'none'"));
  assert.ok(!policy.includes('unsafe-eval'));
  assert.ok(!policy.includes('frame-ancestors')); // Not supported in a meta policy.
  assert.ok(policy.includes('https://github.com/sponsors/MrDesjardins/button;'));
  assert.throws(() => applyCsp('<body>no head</body>'));
});
