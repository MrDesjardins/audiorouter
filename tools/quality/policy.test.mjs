import test from 'node:test';
import assert from 'node:assert/strict';
import { classifyChanges, expectedJobs, areas } from './policy.mjs';
import { evaluateGate } from './gate.mjs';
import { spawnSync, execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checks, fullChecks } from './checks.mjs';

const fixtures = [
  ['docs', ['docs/spec/09-interface.md'], ['docs']],
  ['agent instructions', ['AGENTS.md'], ['docs']],
  ['ui', ['ui/src/App.tsx'], ['ui', 'windows']],
  ['backend', ['crates/control/src/service.rs'], ['backend', 'rust', 'windows']],
  ['contracts consumers', ['contracts/src/index.ts'], ['contracts', 'backend', 'ui', 'streamdeck', 'rust']],
  ['streamdeck source', ['tools/streamdeck/src/index.ts'], ['streamdeck']],
  ['streamdeck lock', ['tools/streamdeck/package-lock.json'], ['streamdeck', 'supply']],
  ['ui lock', ['ui/package-lock.json'], ['ui', 'supply']],
  ['root lock', ['Cargo.lock'], ['backend', 'supply']],
  ['shell lock', ['src-tauri/Cargo.lock'], ['backend', 'release', 'supply']],
  ['website guide', ['site/tools/group.html'], ['site']],
  ['website styles', ['site/assets/compare.css'], ['site']],
  ['CI change', ['.github/workflows/ci.yml'], []],
  ['Jev workflow', ['.github/workflows/jev-review.yml'], []],
  ['quality workflow', ['.github/workflows/quality.yml'], areas],
  ['policy change', ['tools/quality/policy.mjs'], []],
  ['documentation validator', ['tools/docs/validate.mjs'], ['docs']],
  ['mixed website and backend', ['site/compare.html', 'crates/engine/src/lib.rs'], ['site', 'backend', 'rust', 'windows']],
  ['toolchain', ['rust-toolchain.toml'], areas],
  ['future executable', ['new-runner/config.js'], areas],
];
for (const [name, files, enabled] of fixtures) test(`classifies ${name}`, () => {
  const flags = classifyChanges(files);
  for (const area of enabled) assert.equal(flags[area], true, area);
  if (name === 'docs' || name === 'agent instructions') {
    assert.equal(flags.docs, true);
    for (const [key, value] of Object.entries(flags)) if (key !== 'docs') assert.equal(value, false, key);
  }
  if (name.startsWith('website')) {
    assert.equal(flags.site, true);
    assert.equal(flags.windows, false);
    for (const area of ['backend', 'contracts', 'ui', 'probe', 'sdk', 'release', 'streamdeck', 'supply', 'docs']) assert.equal(flags[area], false, area);
  }
  if (name.startsWith('streamdeck')) { assert.equal(flags.backend, false); assert.equal(flags.ui, false); }
});
test('full manual verification enables all areas', () => { for (const value of Object.values(classifyChanges([], { full: true }))) assert.equal(value, true); });
test('main push classifies the pushed before/after diff', () => {
  const root = mkdtempSync(join(tmpdir(), 'audiorouter-policy-'));
  try {
    execFileSync('git', ['init'], { cwd: root, stdio: 'ignore' });
    execFileSync('git', ['config', 'user.email', 'ci@example.invalid'], { cwd: root });
    execFileSync('git', ['config', 'user.name', 'CI policy test'], { cwd: root });
    writeFileSync(join(root, 'README.md'), 'baseline\n');
    execFileSync('git', ['add', '.'], { cwd: root });
    execFileSync('git', ['commit', '-m', 'baseline'], { cwd: root, stdio: 'ignore' });
    const before = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
    mkdirSync(join(root, 'site'), { recursive: true });
    writeFileSync(join(root, 'site', 'compare.html'), '<title>site</title>\n');
    execFileSync('git', ['add', '.'], { cwd: root });
    execFileSync('git', ['commit', '-m', 'site update'], { cwd: root, stdio: 'ignore' });
    const after = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
    const eventPath = join(root, 'event.json'), outputPath = join(root, 'github-output.txt');
    writeFileSync(eventPath, JSON.stringify({ before, after }));
    const script = fileURLToPath(new URL('./policy.mjs', import.meta.url));
    const result = spawnSync(process.execPath, [script, '--github', '--event', 'push'], {
      cwd: root, encoding: 'utf8',
      env: { ...process.env, GITHUB_EVENT_PATH: eventPath, GITHUB_OUTPUT: outputPath },
    });
    assert.equal(result.status, 0, result.stderr);
    const flags = JSON.parse(result.stdout);
    assert.equal(flags.docs, false);
    assert.equal(flags.site, true);
    assert.equal(flags.windows, false);
    assert.equal(flags.backend, false);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
test('handles backslashes and paths with spaces; rejects path traversal', () => {
  assert.equal(classifyChanges(['ui\\src\\a b.ts']).ui, true);
  assert.throws(() => classifyChanges(['../ui/a.ts']));
  assert.throws(() => classifyChanges(['C:\\ui\\a.ts']));
  assert.throws(() => classifyChanges(['/ui/a.ts']));
});
function needsFor(flags) {
  return { changes: { result: 'success', outputs: Object.fromEntries(Object.entries(flags).map(([key, value]) => [key, String(value)])) },
    ...Object.fromEntries(Object.entries(expectedJobs(flags)).map(([job, required]) => [job, { result: required ? 'success' : 'skipped' }])) };
}
for (const [name, files] of fixtures) test(`aggregate accepts only declared unaffected skips: ${name}`, () => { assert.deepEqual(evaluateGate(needsFor(classifyChanges(files))), []); });
for (const result of ['failure', 'cancelled', 'skipped', 'missing', 'neutral', 'timed_out']) test(`aggregate rejects required ${result}`, () => {
  const needs = needsFor(classifyChanges(['ui/src/App.tsx']));
  needs.browser = result === 'missing' ? undefined : { result };
  assert.ok(evaluateGate(needs).length);
});
test('aggregate rejects missing/failed classification and inconsistent derived flags', () => {
  assert.ok(evaluateGate({}).length);
  const needs = needsFor(classifyChanges(['crates/control/src/service.rs']));
  needs.changes.outputs.windows = 'false'; needs.windows.result = 'skipped';
  assert.ok(evaluateGate(needs).length);
  delete needs.changes.outputs.ui; assert.ok(evaluateGate(needs).length);
});
test('aggregate requires only the affected documentation or website check', () => {
  const docs = needsFor(classifyChanges(['docs/README.md']));
  assert.equal(docs.docs.result, 'success');
  assert.equal(docs.site.result, 'skipped');
  const site = needsFor(classifyChanges(['site/compare.html']));
  assert.equal(site.site.result, 'success');
  assert.equal(site.docs.result, 'skipped');
  assert.equal(site.windows.result, 'skipped');
  assert.equal(site.browser.result, 'skipped');
});
test('aggregate rejects even an unnecessary failed check', () => {
  const needs = needsFor(classifyChanges(['docs/README.md'])); needs.browser.result = 'failure'; assert.ok(evaluateGate(needs).length);
});

test('gate process fails closed for invalid JSON, cancellation and skipped required jobs', () => {
  const needs = needsFor(classifyChanges(['ui/src/App.tsx']));
  const gate = new URL('./gate.mjs', import.meta.url);
  const run = value => spawnSync(process.execPath, [fileURLToPath(gate)], { env: { ...process.env, NEEDS_RESULTS: value, GITHUB_STEP_SUMMARY: '' } });
  assert.equal(run(JSON.stringify(needs)).status, 0);
  for (const value of ['null', '{', '{}']) assert.notEqual(run(value).status, 0);
  for (const result of ['cancelled', 'skipped', 'failure']) {
    needs.browser.result = result;
    assert.notEqual(run(JSON.stringify(needs)).status, 0);
  }
});

// Keep this independent of npm packages: the policy job must run before installs.
// actionlint validates workflow syntax; these assertions pin the policy wiring.
test('CI invokes the shared full inventory and gates every classified job', () => {
  const workflow = readFileSync(new URL('../../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const selected = new Set([...workflow.matchAll(/verify\.mjs --checks ([\w,-]+)/g)].flatMap(match => match[1].split(',')));
  const expected = fullChecks(classifyChanges([], { full: true }));
  assert.deepEqual([...selected].sort(), expected.sort());
  for (const id of selected) assert.ok(Object.hasOwn(checks, id));
  assert.match(workflow, /quality-gate:\s+name: Quality gate\s+if: always\(\)\s+needs: \[changes, docs, site, policy, supply-chain, windows, browser, streamdeck\]/);
  assert.match(workflow, /NEEDS_RESULTS: \$\{\{ toJSON\(needs\) \}\}/);
  assert.match(workflow, /node tools\/quality\/gate\.mjs/);
  assert.match(workflow, /node tools\/quality\/policy\.mjs --github --event/);
  for (const area of [...areas, 'windows', 'rust']) assert.ok(workflow.includes(`${area}: \${{ steps.detect.outputs.${area} }}`), area);
});

test('nightly metrics use read-only permissions and artifacts instead of pushing main', () => {
  const workflow = readFileSync(new URL('../../.github/workflows/quality.yml', import.meta.url), 'utf8');
  assert.doesNotMatch(workflow, /contents: write|git push|git commit/);
  assert.match(workflow, /name: quality-history-update/);
  assert.match(workflow, /ref: \$\{\{ github.sha \}\}/);
});

test('strict shell Clippy covers every feature after building current embedded UI', () => {
  const commands = checks['rust-clippy'];
  assert.deepEqual(commands[0].args, ['--prefix', 'ui', 'run', 'build']);
  for (const command of commands.filter(command => command.executable === 'cargo')) assert.ok(command.args.includes('--all-features'));
  const workflow = readFileSync(new URL('../../.github/workflows/ci.yml', import.meta.url), 'utf8');
  assert.ok(workflow.indexOf('name: Install UI dependencies') < workflow.indexOf('name: Strict Clippy'));
  assert.match(workflow, /name: Install UI dependencies\s+if: needs.changes.outputs.backend == 'true' \|\| needs.changes.outputs.ui == 'true' \|\| needs.changes.outputs.release == 'true'/);
});
