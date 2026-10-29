// Reads each downloaded Freesound file's id and user from its name, then reads the license from the sound's page.
// Usage: node tools/soundboard/check-freesound-license.mjs <folder-of-subfolders> <output.json>
// One request every 1.2 s with an honest user agent. The page is the evidence; keep the output with the provenance record.
import { readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
const root = process.argv[2];
const UA = "AudioRouterSourcing/0.1 (license check; https://github.com/MrDesjardins/audiorouter)";
const rows = [];
for (const dir of readdirSync(root, { withFileTypes: true }).filter((d) => d.isDirectory())) {
  for (const f of readdirSync(join(root, dir.name))) {
    const m = f.match(/^(\d+)__([^_]+)__(.+)\.[a-z0-9]+$/i) ?? f.match(/^(\d+)_+([^_]+)_+(.+)\.[a-z0-9]+$/i);
    if (!m) { rows.push({ folder: dir.name, file: f, error: "name does not match id__user__name" }); continue; }
    const [, id, user] = m;
    const url = `https://freesound.org/people/${user}/sounds/${id}/`;
    let license = null, title = null, author = null, status = 0;
    try {
      const res = await fetch(url, { headers: { "User-Agent": UA }, redirect: "follow" });
      status = res.status;
      const html = await res.text();
      license = html.match(/creativecommons\.org\/(publicdomain\/zero\/1\.0|licenses\/[a-z-]+\/[0-9.]+)/)?.[1] ?? null;
      title = html.match(/<meta property="og:title" content="([^"]*)"/)?.[1] ?? null;
      author = html.match(/\/people\/([^/"]+)\//)?.[1] ?? null;
    } catch (e) { license = "fetch failed: " + e.message; }
    rows.push({ folder: dir.name, file: f, id, user, url, status, license, title });
    console.log(`${dir.name}\t${id}\t${license ?? "NO LICENSE FOUND"}\t${status}`);
    await new Promise((r) => setTimeout(r, 1200));
  }
}
writeFileSync(process.argv[3], JSON.stringify(rows, null, 2));
