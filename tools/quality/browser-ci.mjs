import { spawnSync } from 'node:child_process';

const mode = process.argv[2];
if (!['prepare', 'run'].includes(mode)) throw Error('Expected prepare or run');
const run = (executable, args, cwd) => {
  const env = { ...process.env };
  // Playwright sets FORCE_COLOR for workers; avoid Node's contradictory-color warning.
  if (env.FORCE_COLOR !== undefined) delete env.NO_COLOR;
  // Only fixed npm arguments use cmd.exe on Windows.
  const result = spawnSync(executable, args, { cwd, env, stdio: 'inherit', shell: process.platform === 'win32' && executable === 'npm' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
};
if (mode === 'prepare') {
  run('npm', ['--prefix', 'ui', 'run', 'e2e:guard']);
  run('cargo', ['build', '-p', 'audiorouter-control', '--example', 'e2e_backend', '--locked']);
  run('node', ['node_modules/vite/bin/vite.js', 'build', '--config', 'vite.browser-acceptance.config.ts', '--configLoader', 'runner'], 'ui');
  run('node', ['scripts/qualify-bundled-assets.mjs'], 'ui');
} else {
  const shard = process.env.BROWSER_SHARD;
  if (!/^[1-4]\/4$/.test(shard ?? '')) throw Error('Expected BROWSER_SHARD=1/4 through 4/4');
  run('node', ['node_modules/@playwright/test/cli.js', 'test', '--config=playwright.ci.config.ts', `--shard=${shard}`], 'ui');
}
