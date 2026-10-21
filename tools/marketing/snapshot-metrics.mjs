// Append one row of public project numbers to docs/marketing-metrics.csv.
// GitHub keeps traffic (views, clones, referrers) for 14 days only, so run
// this about once a week; see docs/plans/future/marketing.md.
// Needs the GitHub CLI signed in with push access to the repository.
//   node tools/marketing/snapshot-metrics.mjs
//   node tools/marketing/snapshot-metrics.mjs --dry-run   print the row only
import { execFileSync } from "node:child_process";
import { appendFile, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const repo = "MrDesjardins/audiorouter";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const csv = resolve(root, "docs/marketing-metrics.csv");
const header = [
  "date",
  "stars",
  "forks",
  "watchers",
  "open_issues",
  "views_14d",
  "unique_visitors_14d",
  "clones_14d",
  "unique_cloners_14d",
  "latest_release",
  "latest_installer_downloads",
  "installer_downloads_total",
  "top_referrers_14d",
];

const gh = (path) =>
  JSON.parse(execFileSync("gh", ["api", path], { encoding: "utf8", maxBuffer: 1 << 26 }));
const ghPaged = (path) =>
  JSON.parse(execFileSync("gh", ["api", "--paginate", "--slurp", path], { encoding: "utf8", maxBuffer: 1 << 26 })).flat();

const info = gh(`repos/${repo}`);
const views = gh(`repos/${repo}/traffic/views`);
const clones = gh(`repos/${repo}/traffic/clones`);
const referrers = gh(`repos/${repo}/traffic/popular/referrers`);
const releases = ghPaged(`repos/${repo}/releases?per_page=100`).filter((r) => !r.draft);
const installerDownloads = (release) =>
  release.assets
    .filter((a) => /^AudioRouter_.*_x64-setup\.exe$/.test(a.name))
    .reduce((sum, a) => sum + a.download_count, 0);
const latest = releases[0];

const row = [
  new Date().toISOString().slice(0, 10),
  info.stargazers_count,
  info.forks_count,
  info.subscribers_count,
  info.open_issues_count,
  views.count,
  views.uniques,
  clones.count,
  clones.uniques,
  latest?.tag_name ?? "",
  latest ? installerDownloads(latest) : 0,
  releases.reduce((sum, r) => sum + installerDownloads(r), 0),
  // "site:unique visitors" pairs, separated by spaces so the CSV needs no quoting.
  referrers.map((r) => `${r.referrer}:${r.uniques}`).join(" "),
].join(",");

if (process.argv.includes("--dry-run")) {
  console.log(header.join(","));
  console.log(row);
} else {
  const existing = await readFile(csv, "utf8").catch(() => "");
  if (!existing) await writeFile(csv, header.join(",") + "\n");
  await appendFile(csv, row + "\n");
  console.log("Added to docs/marketing-metrics.csv:\n" + row);
}
