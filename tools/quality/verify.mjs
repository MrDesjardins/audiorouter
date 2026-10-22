import { spawn, execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { randomUUID } from 'node:crypto';
import { checks, fast, fullChecks } from './checks.mjs';
import { classifyChanges, changedFiles } from './policy.mjs';
export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

function npmCli() {
  const candidates = [process.env.npm_execpath, path.join(path.dirname(process.execPath), 'node_modules/npm/bin/npm-cli.js')];
  if (process.platform === 'win32') {
    try { candidates.push(path.join(path.dirname(execFileSync('where.exe', ['npm.cmd'], { encoding: 'utf8' }).split(/\r?\n/)[0]), 'node_modules/npm/bin/npm-cli.js')); } catch { /* diagnosed below */ }
  }
  const result = candidates.find(candidate => candidate && candidate.endsWith('npm-cli.js') && existsSync(candidate));
  if (!result) throw Error('Cannot locate npm-cli.js. Install the pinned Node distribution or invoke through npm.');
  return result;
}
export function resolveCommand(command) {
  if (command.executable === 'node') return [process.execPath, ...command.args];
  if (command.executable === 'npm') return process.platform === 'win32'
    ? [process.execPath, npmCli(), ...command.args] : ['npm', ...command.args];
  if (command.executable === 'powershell') {
    let shell = 'powershell.exe';
    try { execFileSync('where.exe', ['pwsh.exe'], { stdio: 'ignore' }); shell = 'pwsh.exe'; } catch { /* PS 5.1 is supported */ }
    return [shell, ...command.args];
  }
  return [process.platform === 'win32' && command.executable === 'cargo' ? 'cargo.exe' : command.executable, ...command.args];
}
export function execute(command) {
  return new Promise(resolve => {
    let resolved;
    try { resolved = resolveCommand(command); } catch (error) { resolve({ status: 'blocked', detail: error.message }); return; }
    const child = spawn(resolved[0], resolved.slice(1), { cwd: command.cwd ? path.resolve(root, command.cwd) : root, stdio: 'inherit', shell: false });
    child.on('error', error => resolve({ status: 'blocked', detail: error.code ?? error.message }));
    child.on('close', (code, signal) => resolve({ status: code === 0 ? 'passed' : 'failed', exitCode: code, signal }));
  });
}
export async function runChecks(ids, { inventory = checks, run = execute, platform = process.platform, keepGoing = false } = {}) {
  for (const id of ids) if (!Object.hasOwn(inventory, id)) throw Error(`Unknown check: ${id}`);
  const results = [];
  let stopped = false;
  for (const id of ids) {
    const startedAt = new Date().toISOString(), started = performance.now();
    let status = stopped ? 'not-run' : 'passed';
    const commands = [];
    if (!stopped) for (const command of inventory[id]) {
      let result;
      if (command.windows && platform !== 'win32') result = { status: 'blocked', detail: 'Requires Windows; not portable/hardware evidence' };
      else { console.log(`\n[${id}] ${command.executable} ${command.args.join(' ')}`); result = await run(command); }
      commands.push({ command, ...result });
      if (result.status !== 'passed') { status = result.status; break; }
    }
    results.push({ id, status, startedAt, durationMs: Math.round(performance.now() - started), commands });
    if (status !== 'passed' && !keepGoing) stopped = true;
  }
  return results;
}
export function hardwareCheck(options) {
  if (!options.allowLiveAudio || !options.capture || !options.render) throw Error('Hardware profile is blocked: require --allow-live-audio, --capture-endpoint and --render-endpoint. It uses real audio; no endpoint is inferred.');
  const duration = Number(options.duration ?? 500);
  if (!Number.isInteger(duration) || duration < 100 || duration > 2000) throw Error('Hardware duration must be 100-2000 ms');
  for (const value of [options.capture, options.render]) if (!value.trim() || /[\r\n\0]/.test(value)) throw Error('Invalid endpoint ID');
  return { executable: 'powershell', windows: true, args: ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'tests/acceptance/m02-control-route-live.ps1', '-AllowLiveAudio', '-DurationMilliseconds', String(duration), '-CaptureEndpointId', options.capture, '-RenderEndpointId', options.render] };
}
async function main() {
  const args = process.argv.slice(2);
  const values = new Map();
  const switches = new Set(['--list', '--keep-going', '--allow-live-audio']);
  const valued = new Set(['--profile', '--checks', '--base', '--head', '--output', '--capture-endpoint', '--render-endpoint', '--duration-ms']);
  for (let index = 0; index < args.length; index++) {
    const key = args[index];
    if (switches.has(key)) values.set(key, true);
    else if (valued.has(key) && args[index + 1] && !args[index + 1].startsWith('--')) values.set(key, args[++index]);
    else throw Error(`Unknown or incomplete option: ${key}`);
  }
  if (values.has('--checks') && values.has('--profile')) throw Error('Choose --checks or --profile');
  const profile = values.get('--profile') ?? (values.has('--checks') ? 'explicit' : 'fast');
  if (!['explicit', 'fast', 'full', 'hardware'].includes(profile)) throw Error(`Unknown profile: ${profile}`);
  const flags = classifyChanges(values.has('--base') ? changedFiles(values.get('--base'), values.get('--head') ?? 'HEAD') : [], { full: !values.has('--base') });
  const inventory = { ...checks };
  let ids = profile === 'explicit' ? values.get('--checks').split(',') : profile === 'full' ? fullChecks(flags) : fast;
  if (profile === 'hardware') {
    if (values.has('--list')) { console.log('Guarded existing-endpoint control-route probe. Requires Windows, explicit live-audio consent, capture/render IDs and available devices. This short probe is not the full audio qualification.'); return; }
    inventory['hardware-route'] = [hardwareCheck({ allowLiveAudio: values.has('--allow-live-audio'), capture: values.get('--capture-endpoint'), render: values.get('--render-endpoint'), duration: values.get('--duration-ms') })];
    ids = ['hardware-route'];
  }
  for (const id of ids) if (!Object.hasOwn(inventory, id)) throw Error(`Unknown check: ${id}`);
  if (values.has('--list')) { console.log(JSON.stringify({ profile, flags, checks: ids.map(id => ({ id, commands: inventory[id] })) }, null, 2)); return; }
  const output = path.resolve(values.get('--output') ?? path.join(process.env.RUNNER_TEMP ?? os.tmpdir(), 'audiorouter-verification', `${Date.now()}-${randomUUID()}.json`));
  if (existsSync(output)) throw Error('Refusing to overwrite verification evidence');
  const startedAt = new Date().toISOString();
  const results = await runChecks(ids, { inventory, keepGoing: values.has('--keep-going') });
  let sha = 'unavailable';
  let dirty = null;
  try {
    sha = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
    dirty = execFileSync('git', ['status', '--porcelain', '--untracked-files=normal'], { cwd: root, encoding: 'utf8' }).trim().length > 0;
  } catch { /* unavailable metadata stays explicit */ }
  const report = { version: 1, sha, dirty, node: process.version, profile, platform: `${process.platform}/${process.arch}`, startedAt, finishedAt: new Date().toISOString(), flags, results };
  mkdirSync(path.dirname(output), { recursive: true });
  writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' });
  for (const result of results) console.log(`${result.status}: ${result.id} (${result.durationMs} ms)`);
  console.log(`Verification evidence: ${output}`);
  if (results.some(result => result.status !== 'passed')) process.exitCode = 1;
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch(error => { console.error(error.message); process.exitCode = 1; });
}
