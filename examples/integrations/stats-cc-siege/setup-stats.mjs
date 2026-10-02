import { readFile, writeFile, unlink, lstat } from 'node:fs/promises';
import path from 'node:path';
import net from 'node:net';
import { fileURLToPath } from 'node:url';

export const feedFile = 'ceb44052-d616-4bdc-993c-70bae41091e9.txt';
export async function configureFeed(directory, port, remove = false) {
  if (!Number.isInteger(port) || port < 1024 || port > 65535) throw new Error('Invalid feed port.');
  const item = await lstat(directory);
  if (!item.isDirectory() || item.isSymbolicLink()) throw new Error('Stats.cc user-data directory must already exist and be a real directory.');
  const filename = path.join(directory, feedFile);
  if (remove) {
    const existing = await readFile(filename, 'utf8');
    if (existing.trim() !== String(port)) throw new Error('Feed file differs from this configuration; refusing removal.');
    const file = await lstat(filename); if (!file.isFile() || file.isSymbolicLink()) throw new Error('Feed file is not a regular file.');
    await unlink(filename); return 'removed';
  }
  try {
    const existing = await readFile(filename, 'utf8');
    if (existing.trim() !== String(port)) throw new Error('Stats.cc already has a different feed port. Adjust statsUrl rather than overwrite it.');
    return 'already configured';
  } catch (error) { if (error.code !== 'ENOENT') throw error; }
  // Test loopback availability without creating an externally reachable listener.
  await new Promise((resolve, reject) => { const probe=net.createServer(); probe.once('error',reject); probe.listen(port,'127.0.0.1',()=>probe.close(resolve)); });
  await writeFile(filename, `${port}\n`, { flag: 'wx' }); return 'configured';
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const { localUrl } = await import('./core.mjs');
    const c = JSON.parse(await readFile(new URL('./config.example.json',import.meta.url),'utf8'));
    const localPath = new URL('./config.local.json',import.meta.url);
    try { Object.assign(c, JSON.parse(await readFile(localPath,'utf8'))); } catch(e) { if(e.code!=='ENOENT')throw e; }
    const port=Number(new URL(localUrl(c.statsUrl,'ws:')).port);
    if (!process.env.APPDATA) throw new Error('Run setup on Windows with Stats.cc already installed.');
    const install = path.join(process.env.LOCALAPPDATA ?? '', 'Programs', 'stats.cc', 'resources');
    const meta = JSON.parse(await readFile(path.join(install,'meta.json'),'utf8'));
    if (meta.release?.version !== '1.8.1') throw new Error('Setup was inspected for Stats.cc 1.8.1 only. Recheck its feed contract before enabling another version.');
    const result = await configureFeed(path.join(process.env.APPDATA,'stats.cc'),port,process.argv.includes('--remove'));
    console.log(`Stats.cc state feed ${result}. Close Stats.cc normally, including its tray app, then reopen it.`);
    console.log('Stats.cc 1.8.1 binds this unauthenticated feed to all interfaces. Keep its port blocked for inbound network access. No app code or firewall rule was changed.');
  } catch(error) { console.error(error.message); process.exitCode=1; }
}
