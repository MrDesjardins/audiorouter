import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, copyFileSync, rmSync, realpathSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const repo = fileURLToPath(new URL('../', import.meta.url));
const hooks = JSON.parse(readFileSync(path.join(repo, 'src-tauri/tauri.conf.json'), 'utf8')).build;

for (const [hook, task] of [['beforeBuildCommand', 'build'], ['beforeDevCommand', 'dev']]) {
  for (const exitCode of [0, 7]) {
    test(`${hook} executes npm ${task} through cmd and preserves exit ${exitCode}`, { skip: process.platform !== 'win32' }, () => {
      // The hook reports the long final path. CI's TEMP is an 8.3 short path
      // (C:\Users\RUNNER~1\...), so compare against the resolved long form.
      const temp = realpathSync.native(tmpdir());
      const fixture = realpathSync.native(mkdtempSync(path.join(temp, 'ar ui hook ')));
      try {
        mkdirSync(path.join(fixture, 'tools'));
        mkdirSync(path.join(fixture, 'ui'));
        mkdirSync(path.join(fixture, 'bin'));
        copyFileSync(path.join(repo, 'tools/run-ui.ps1'), path.join(fixture, 'tools/run-ui.ps1'));
        const report = path.join(fixture, 'called.txt');
        writeFileSync(path.join(fixture, 'bin/npm.cmd'), '@echo off\r\n> "%AR_UI_HOOK_REPORT%" echo %*\r\nexit /b %AR_UI_HOOK_EXIT%\r\n');
        const env = { ...process.env };
        const pathKey = Object.keys(env).find(key => key.toLowerCase() === 'path') ?? 'PATH';
        env[pathKey] = `${path.join(fixture, 'bin')};${env[pathKey] ?? ''}`;
        env.AR_UI_HOOK_REPORT = report;
        env.AR_UI_HOOK_EXIT = String(exitCode);
        const result = spawnSync('cmd.exe', ['/d', '/s', '/c', hooks[hook]], {
          // Tauri selects ui/ as frontendDir (the directory with package.json).
          cwd: path.join(fixture, 'ui'), env, encoding: 'utf8', windowsHide: true, timeout: 15000,
        });
        assert.equal(result.error, undefined);
        assert.equal(result.status, exitCode, result.stdout + result.stderr);
        assert.equal(readFileSync(report, 'utf8').trim(), `--prefix "${path.join(fixture, 'ui')}" run ${task}`);
      } finally {
        const resolved = path.resolve(fixture);
        assert.equal(path.dirname(resolved), temp);
        rmSync(resolved, { recursive: true, force: true });
      }
    });
  }
}
