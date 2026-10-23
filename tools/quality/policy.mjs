import { execFileSync } from 'node:child_process';
import { appendFileSync, readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export const areas = ['backend', 'contracts', 'ui', 'probe', 'sdk', 'release', 'streamdeck', 'supply', 'docs', 'site'];
export function classifyChanges(files, { full = false } = {}) {
  const flags = Object.fromEntries(areas.map(area => [area, full]));
  for (let file of files) {
    file = file.replaceAll('\\', '/').replace(/^\.\//, '');
    if (!file || file.includes('\0') || file.startsWith('/') || /^[a-z]:/i.test(file) || file.split('/').includes('..')) throw Error('Invalid changed path');
    if (file.endsWith('.md')) { flags.docs = true; continue; }
    if (file === '.github/workflows/ci.yml' || file === '.github/workflows/jev-review.yml' || file.startsWith('tools/quality/')) continue;
    if (file.startsWith('.github/') || file === 'rust-toolchain.toml' || file.startsWith('.cargo/')) {
      for (const area of areas) flags[area] = true;
      continue;
    }
    let known = false;
    const match = (pattern, ...selected) => { if (pattern.test(file)) { known = true; for (const area of selected) flags[area] = true; } };
    match(/^(crates\/|src-tauri\/|tests\/|examples\/|Cargo\.(toml|lock)$)/, 'backend');
    match(/^(contracts\/|tools\/contracts\/)/, 'contracts', 'backend', 'ui', 'streamdeck');
    match(/^ui\//, 'ui');
    match(/^(site\/|tools\/site\/)/, 'site');
    match(/^(tools\/m00-(wasapi-probe|native-wasapi-probe)\/|tests\/acceptance\/m00-)/, 'probe');
    match(/^(tools\/m06-vst3-sdk\/|tests\/acceptance\/m06-)/, 'sdk');
    match(/^(tools\/release\/|drivers\/|tests\/acceptance\/m08-|src-tauri\/|ui\/package\.json$)/, 'release');
    match(/^tools\/streamdeck\//, 'streamdeck');
    match(/^(deny\.toml$|Cargo\.(toml|lock)$|src-tauri\/Cargo\.(toml|lock)$|crates\/[^/]+\/Cargo\.toml$|(ui|contracts|tools\/streamdeck)\/package(-lock)?\.json$)/, 'supply');
    if (file.startsWith('tools/docs/')) { flags.docs = true; known = true; }
    if (file === '.gitignore' || file.startsWith('jev/')) known = true;
    // New executable/build surfaces fail conservatively until explicitly classified.
    if (!known) for (const area of areas) flags[area] = true;
  }
  flags.windows = ['backend', 'contracts', 'ui', 'probe', 'sdk', 'release'].some(area => flags[area]);
  flags.rust = ['backend', 'contracts', 'probe', 'release'].some(area => flags[area]);
  return flags;
}
export function expectedJobs(flags) {
  return { docs: flags.docs, site: flags.site, policy: true, windows: flags.windows,
    browser: flags.ui || flags.backend, 'supply-chain': flags.supply, streamdeck: flags.streamdeck };
}
export function changedFiles(base, head = 'HEAD') {
  return execFileSync('git', ['diff', '--name-only', '-z', base, head], { encoding: 'utf8' }).split('\0').filter(Boolean);
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2);
  const option = name => args.includes(name) ? args[args.indexOf(name) + 1] : undefined;
  const event = option('--event') ?? process.env.GITHUB_EVENT_NAME;
  let files = [];
  let full = args.includes('--full');
  if (option('--base')) files = changedFiles(option('--base'), option('--head') ?? 'HEAD');
  else if (event === 'pull_request') files = changedFiles('HEAD^1', 'HEAD');
  else if (event === 'push') {
    const payload = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8'));
    const base = payload.before, head = payload.after ?? 'HEAD';
    if (!base || /^0+$/.test(base)) full = true;
    else {
      try { execFileSync('git', ['cat-file', '-e', `${base}^{commit}`], { stdio: 'ignore' }); }
      catch { execFileSync('git', ['fetch', '--no-tags', '--depth=1', 'origin', base], { stdio: 'inherit' }); }
      files = changedFiles(base, head);
    }
  } else if (event !== undefined) full = true;
  const flags = classifyChanges(files, { full });
  if (args.includes('--github')) {
    if (!process.env.GITHUB_OUTPUT) throw Error('GITHUB_OUTPUT is required');
    appendFileSync(process.env.GITHUB_OUTPUT, Object.entries(flags).map(([key, value]) => `${key}=${value}\n`).join(''));
  }
  console.log(JSON.stringify(flags, null, 2));
}
