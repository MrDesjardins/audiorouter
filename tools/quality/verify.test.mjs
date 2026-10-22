import test from 'node:test';
import assert from 'node:assert/strict';
import { runChecks, execute, hardwareCheck } from './verify.mjs';
import { fullChecks, checks } from './checks.mjs';
import { classifyChanges } from './policy.mjs';
const command = code => ({ executable: 'node', args: ['-e', `process.exit(${code})`] });
test('records actual failed commands and does not falsely pass unrun checks', async () => {
  const results = await runChecks(['a','b','c'], { inventory: { a:[command(0)], b:[command(7)], c:[command(0)] } });
  assert.deepEqual(results.map(result => result.status), ['passed','failed','not-run']);
  assert.equal(results[1].commands[0].exitCode, 7);
});
test('keep-going records a failure even if later checks pass', async () => {
  const results = await runChecks(['a','b'], { inventory:{a:[command(1)],b:[command(0)]},keepGoing:true });
  assert.deepEqual(results.map(result=>result.status), ['failed','passed']);
});
test('blocked Windows checks are not passes and stop later work', async () => {
  const results = await runChecks(['a','b'], {inventory:{a:[{...command(0),windows:true}],b:[command(0)]},platform:'linux'});
  assert.deepEqual(results.map(result=>result.status),['blocked','not-run']);
});
test('unknown checks fail before executing anything', async () => { await assert.rejects(runChecks(['docs','typo'])); });
test('an unavailable executable is blocked, not successful', async () => { assert.equal((await execute({executable:'audiorouter-not-a-real-command-123',args:[]})).status,'blocked'); });
test('full selection contains only real checks and preserves consumer coverage', () => {
  const ids=fullChecks(classifyChanges([], {full:true}));
  for(const id of ids) assert.ok(Object.hasOwn(checks,id));
  for(const id of ['browser','contract-drift','streamdeck-tests','rust-tests']) assert.ok(ids.includes(id));
});
test('hardware profile refuses implicit devices and preserves endpoint arguments without a shell', () => {
  assert.throws(()=>hardwareCheck({}));
  assert.throws(()=>hardwareCheck({allowLiveAudio:true,capture:'a',render:'b',duration:99}));
  const result=hardwareCheck({allowLiveAudio:true,capture:'capture id',render:'render id',duration:500});
  assert.ok(result.windows); assert.deepEqual(result.args.slice(-4),['-CaptureEndpointId','capture id','-RenderEndpointId','render id']);
});