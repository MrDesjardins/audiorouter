// One inventory for the local runner and CI. Installation is a separate step.
const npm = (directory, ...args) => ({ executable: 'npm', args: ['--prefix', directory, ...args] });
const cargo = (...args) => ({ executable: 'cargo', args, windows: true });
const ps = script => ({ executable: 'powershell', args: ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', script], windows: true });
export const checks = {
  docs: [{ executable: 'node', args: ['tools/docs/validate.mjs'] }],
  policy: [{ executable: 'node', args: ['--test', 'tools/quality/policy.test.mjs', 'tools/quality/verify.test.mjs'] }],
  'rust-format': [{ executable: 'cargo', args: ['fmt', '--all', '--', '--check'] }, { executable: 'cargo', args: ['fmt', '--manifest-path', 'src-tauri/Cargo.toml', '--', '--check'] }],
  // custom-protocol embeds frontendDist, so build current assets before shell lint.
  'rust-clippy': [{ ...npm('ui', 'run', 'build'), windows: true }, cargo('clippy', '--workspace', '--all-targets', '--all-features', '--locked', '--', '-D', 'warnings'), cargo('clippy', '--manifest-path', 'src-tauri/Cargo.toml', '--all-targets', '--all-features', '--locked', '--', '-D', 'warnings')],
  'rust-tests': [cargo('test', '--workspace', '--locked')],
  'shell-tests': [cargo('test', '--manifest-path', 'src-tauri/Cargo.toml', '--locked')],
  'contract-type': [npm('contracts', 'run', 'typecheck')],
  'contract-drift': [{ ...npm('contracts', 'run', 'check:drift'), windows: true }],
  'ui-format': [npm('ui', 'run', 'format:check')],
  'ui-lint': [npm('ui', 'run', 'lint')],
  'ui-type': [npm('ui', 'run', 'typecheck:all')],
  'ui-tests': [npm('ui', 'test')],
  'ui-build': [npm('ui', 'run', 'build')],
  browser: [{ ...npm('ui', 'run', 'e2e'), windows: true }],
  'streamdeck-type': [npm('tools/streamdeck', 'run', 'typecheck')],
  'streamdeck-tests': [npm('tools/streamdeck', 'test')],
  'streamdeck-build': [npm('tools/streamdeck', 'run', 'build')],
  'probe-build': [cargo('check', '--manifest-path', 'tools/m00-wasapi-probe/Cargo.toml', '--locked'), ps('tests/acceptance/m00-native-build.ps1')],
  'cli-parity': [{ ...ps('tests/acceptance/m01-cli.ps1'), args: [...ps('tests/acceptance/m01-cli.ps1').args, '-AllowNoAudioEndpoints'] }],
  'sdk-provenance': [ps('tests/acceptance/m06-sdk-installer.ps1')],
  'release-tests': ['prepare-artifacts', 'verify-artifacts', 'runbook-command', 'app-only-release'].map(name => ps(`tools/release/test-${name}.ps1`)),
  'release-build': [{ ...ps('tests/acceptance/m08-release.ps1'), args: [...ps('tests/acceptance/m08-release.ps1').args, '-AppOnly'] }],
  'cargo-deny': [{ ...cargo('deny', '--manifest-path', 'Cargo.toml', 'check', 'advisories', 'licenses', 'sources', 'bans'), windows: false }, { ...cargo('deny', '--manifest-path', 'src-tauri/Cargo.toml', 'check', 'advisories', 'licenses', 'sources', 'bans'), windows: false }],
  'npm-audit': ['ui', 'contracts', 'tools/streamdeck'].map(dir => npm(dir, 'audit', '--omit=dev', '--audit-level=high')),
};
export const fast = ['docs', 'policy', 'rust-format', 'contract-type', 'ui-format', 'ui-lint', 'ui-type', 'streamdeck-type'];
export function fullChecks(flags) {
  return [...new Set(['docs', 'policy',
    ...(flags.backend ? ['rust-format', 'rust-clippy', 'rust-tests', 'shell-tests', 'cli-parity'] : []),
    ...(flags.backend || flags.contracts ? ['contract-type', 'contract-drift'] : []),
    ...(flags.ui ? ['ui-format', 'ui-lint', 'ui-tests', 'ui-build'] : []),
    ...(flags.backend || flags.ui ? ['browser'] : []),
    ...(flags.streamdeck ? ['streamdeck-type', 'streamdeck-tests', 'streamdeck-build'] : []),
    ...(flags.probe ? ['probe-build'] : []), ...(flags.sdk ? ['sdk-provenance'] : []),
    ...(flags.release ? ['release-tests', 'release-build'] : []), ...(flags.supply ? ['cargo-deny', 'npm-audit'] : []),
  ])];
}
