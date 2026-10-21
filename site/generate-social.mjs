// Add link-preview tags (Open Graph, Twitter card, canonical URL) to every
// page, and rebuild sitemap.xml and robots.txt from the pages that exist.
// Run from any directory after adding a page or regenerating the guides:
//   node site/generate-social.mjs          rewrite the files
//   node site/generate-social.mjs --check  fail if any file is out of date
import { readFile, readdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const site = dirname(fileURLToPath(import.meta.url));
const origin = "https://audiorouter.org";
const image = { url: origin + "/assets/social-preview.png", width: 1280, height: 640 };
const fallbackDescription = "Free, open source visual audio router for Windows 11.";
const check = process.argv.includes("--check");

const pages = [
  ...(await readdir(site)).filter((name) => name.endsWith(".html")),
  ...(await readdir(resolve(site, "tools")))
    .filter((name) => name.endsWith(".html"))
    .map((name) => "tools/" + name),
].sort();

// Escape a bare "&" but leave existing entities alone.
const attr = (text) =>
  text.replace(/&(?![a-z]+;|#\d+;)/gi, "&amp;").replaceAll('"', "&quot;");
const pageUrl = (path) => origin + "/" + (path === "index.html" ? "" : path);

const stale = [];
async function update(path, text) {
  const file = resolve(site, path);
  const old = await readFile(file, "utf8").catch(() => null);
  if (old === text) return;
  if (check) stale.push(path);
  else await writeFile(file, text);
}

for (const path of pages) {
  const html = await readFile(resolve(site, path), "utf8");
  const title = html.match(/<title>([^<]*)<\/title>/)?.[1]?.trim();
  if (!title) throw new Error(path + " has no <title>");
  const description =
    html.match(/<meta name="description" content="([^"]*)">/)?.[1] ?? fallbackDescription;
  const url = pageUrl(path);
  const tags =
    "<!-- social -->" +
    `<link rel="canonical" href="${url}">` +
    `<meta property="og:type" content="website">` +
    `<meta property="og:site_name" content="AudioRouter">` +
    `<meta property="og:title" content="${attr(title)}">` +
    `<meta property="og:description" content="${description}">` +
    `<meta property="og:url" content="${url}">` +
    `<meta property="og:image" content="${image.url}">` +
    `<meta property="og:image:width" content="${image.width}">` +
    `<meta property="og:image:height" content="${image.height}">` +
    `<meta property="og:image:alt" content="AudioRouter: see your sound, shape it, send it anywhere">` +
    `<meta name="twitter:card" content="summary_large_image">` +
    "<!-- /social -->";
  const without = html.replace(/<!-- social -->[\s\S]*?<!-- \/social -->/, "");
  await update(path, without.replace("</head>", tags + "</head>"));
}

await update(
  "sitemap.xml",
  '<?xml version="1.0" encoding="UTF-8"?>\n' +
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n' +
    pages.map((path) => `  <url><loc>${pageUrl(path)}</loc></url>\n`).join("") +
    "</urlset>\n",
);
await update("robots.txt", `User-agent: *\nAllow: /\n\nSitemap: ${origin}/sitemap.xml\n`);

if (stale.length) {
  console.error("Out of date; run node site/generate-social.mjs:\n  " + stale.join("\n  "));
  process.exit(1);
}
console.log((check ? "Checked " : "Updated ") + pages.length + " pages, sitemap.xml and robots.txt");
