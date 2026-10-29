// Builds a local page for listening to sourcing candidates and ticking the ones to keep.
// Usage: node tools/soundboard/make-audition-page.mjs <sourcing-folder>
// Writes <sourcing-folder>/audition.html (or the file name given as a second argument). Nothing leaves the computer; the page only plays local files.
import { readdirSync, statSync, writeFileSync } from "node:fs";
import { join, relative, extname } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: node tools/soundboard/make-audition-page.mjs <sourcing-folder>");
const AUDIO = new Set([".wav", ".ogg", ".flac", ".m4a", ".aiff", ".mp3"]);
const walk = (dir) => readdirSync(dir).flatMap((name) => {
  const full = join(dir, name);
  return statSync(full).isDirectory() ? walk(full) : [full];
});
// A folder whose name starts with "_" (for example _excluded) is kept out of the page.
const files = walk(root)
  .filter((f) => AUDIO.has(extname(f).toLowerCase()) && !/Preview\.ogg$/i.test(f) && !relative(root, f).startsWith("_"))
  .sort();
const groups = new Map();
for (const f of files) {
  const rel = relative(root, f).replaceAll("\\", "/");
  const parts = rel.split("/");
  const key = parts.length > 2 ? parts.slice(0, 2).join(" / ") : parts[0];
  if (!groups.has(key)) groups.set(key, []);
  groups.get(key).push(rel);
}
const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const sections = [...groups].map(([key, list]) => `<section><h2>${esc(key)} <small>${list.length}</small></h2>${list.map((rel) =>
  `<div class="row"><label><input type="checkbox" data-path="${esc(rel)}"> ${esc(rel.split("/").pop())}</label><audio controls preload="none" src="${esc(encodeURI(rel))}"></audio></div>`).join("")}</section>`).join("");
writeFileSync(join(root, process.argv[3] ?? "audition.html"), `<!doctype html><meta charset="utf-8"><title>Soundboard audition</title>
<style>body{font:14px system-ui;background:#16181d;color:#e6e6e6;margin:0 auto;max-width:900px;padding:16px}h2{position:sticky;top:0;background:#16181d;padding:8px 0;margin:24px 0 4px}small{color:#9aa}
.row{display:flex;align-items:center;gap:12px;padding:3px 0;border-bottom:1px solid #262a33}.row label{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}audio{height:30px;width:320px}
#bar{position:fixed;right:16px;bottom:16px;background:#2b3140;padding:10px 14px;border-radius:9px}button{font:inherit}</style>
<h1>Soundboard audition</h1><p>Press play, tick what you want in the library, then use the button at the bottom right and paste the list to the agent.</p>${sections}
<div id="bar"><span id="count">0</span> picked <button id="copy">Copy picked list</button> <button id="save">Save as file</button></div>
<script>
const KEY="soundboard-picks";
const boxes=[...document.querySelectorAll("input[data-path]")];
// Storage can be blocked for local files; the page must work (and count) without it.
let store=null;try{store=window.localStorage}catch(e){}
const saved=new Set((()=>{try{return JSON.parse(store&&store.getItem(KEY)||"[]")}catch(e){return[]}})());
const picked=()=>boxes.filter(b=>b.checked).map(b=>b.dataset.path).sort();
const refresh=()=>{const list=picked();document.getElementById("count").textContent=list.length;try{store&&store.setItem(KEY,JSON.stringify(list))}catch(e){}};
boxes.forEach(b=>{b.checked=saved.has(b.dataset.path);b.addEventListener("change",refresh)});
document.addEventListener("play",e=>{document.querySelectorAll("audio").forEach(a=>{if(a!==e.target)a.pause()})},true);
const text=()=>picked().join("\\n");
document.getElementById("copy").onclick=async()=>{try{await navigator.clipboard.writeText(text());document.getElementById("copy").textContent="Copied"}catch(e){document.getElementById("copy").textContent="Copy failed, use Save"}};
document.getElementById("save").onclick=()=>{const a=document.createElement("a");a.href=URL.createObjectURL(new Blob([text()],{type:"text/plain"}));a.download="soundboard-picks.txt";a.click()};
refresh();
</script>`);
console.log(`${process.argv[3] ?? "audition.html"}: ${files.length} files in ${groups.size} groups`);
