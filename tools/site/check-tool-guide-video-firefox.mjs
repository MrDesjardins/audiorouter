// Real pointer-click playback in installed Firefox, including the media output.
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const profile = mkdtempSync(join(tmpdir(), "audiorouter-video-firefox-"));
const ff = spawn("C:/Program Files/Mozilla Firefox/firefox.exe", [
  "--headless",
  "--no-remote",
  "--profile",
  profile,
  "--remote-debugging-port=9337",
]);
let ws;
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
for (let retry = 0; retry < 40 && !ws; retry++) {
  await wait(250);
  try {
    const socket = new WebSocket("ws://127.0.0.1:9337/session");
    await new Promise((res, rej) => {
      socket.onopen = res;
      socket.onerror = rej;
    });
    ws = socket;
  } catch {}
}
if (!ws) {
  ff.kill();
  throw new Error("Firefox BiDi startup failed");
}
let sequence = 0;
const pending = new Map();
ws.onmessage = (e) => {
  const reply = JSON.parse(e.data);
  const entry = pending.get(reply.id);
  if (entry) {
    pending.delete(reply.id);
    reply.type === "error"
      ? entry.reject(new Error(JSON.stringify(reply)))
      : entry.resolve(reply.result);
  }
};
const send = (method, params) =>
  new Promise((resolve, reject) => {
    const id = ++sequence;
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
    setTimeout(() => {
      if (pending.delete(id)) reject(new Error(`Timeout: ${method}`));
    }, 15000).unref();
  });
const selected = process.env.GUIDE_KIND?.split(",");
const recordings = JSON.parse(
  readFileSync(resolve(root, "site/guide-content/recordings.json"), "utf8"),
);
const resultPath = resolve(root, "target/tool-guide-firefox-results.json");
const results =
  selected && existsSync(resultPath)
    ? JSON.parse(readFileSync(resultPath, "utf8")).filter(
        (row) => !selected.includes(row.kind),
      )
    : [];
try {
  await send("session.new", { capabilities: {} });
  const { context } = await send("browsingContext.create", { type: "tab" });
  const evaluate = async (expression) => {
    const reply = await send("script.evaluate", {
      expression,
      target: { context },
      awaitPromise: true,
    });
    if (reply.type === "exception") throw new Error(JSON.stringify(reply));
    return reply.result.value;
  };
  const tools = JSON.parse(
    readFileSync(
      resolve(root, "tools/site/tool-guide-property-manifest.json"),
      "utf8",
    ),
  ).tools;
  for (const tool of tools.filter(
    (tool) => !selected || selected.includes(tool.kind),
  )) {
    // Every published clip: the overview, the demo and each per-property clip.
    for (const clip of recordings[tool.kind] ?? []) {
      const path = `site/${clip.video}`;
      const type = clip.video.endsWith("-overview.mp4")
        ? "overviews"
        : clip.video.endsWith("-demo.mp4")
          ? "demos"
          : "properties";
      const audible = type !== "overviews" && clip.silent !== true;
      if (!existsSync(resolve(root, path))) continue;
      try {
        await send("browsingContext.navigate", {
          context,
          url: "http://127.0.0.1:4187/docs/plans/active/evidence/tool-guide-preview/index.html",
          wait: "complete",
        });
        await evaluate(
          `(() => { document.body.innerHTML='<button id="play">Play clip</button><video controls width="240" src="/${path}"></video>'; const v=document.querySelector('video'); const ctx=new AudioContext(); const a=ctx.createAnalyser();ctx.createMediaElementSource(v).connect(a);a.connect(ctx.destination);window.__videoProbe={v,ctx,a,error:null};document.querySelector('#play').onclick=async()=>{try{await ctx.resume();await v.play()}catch(e){window.__videoProbe.error=String(e)}};return true })()`,
        );
        const box = JSON.parse(
          await evaluate(
            `JSON.stringify((()=>{const r=document.querySelector('#play').getBoundingClientRect();return {x:Math.round(r.x+r.width/2),y:Math.round(r.y+r.height/2)}})())`,
          ),
        );
        await send("input.performActions", {
          context,
          actions: [
            {
              type: "pointer",
              id: "mouse",
              parameters: { pointerType: "mouse" },
              actions: [
                { type: "pointerMove", origin: "viewport", ...box },
                { type: "pointerDown", button: 0 },
                { type: "pointerUp", button: 0 },
              ],
            },
          ],
        });
        const state = JSON.parse(
          await evaluate(
            `(async()=>{const {v,ctx,a}=window.__videoProbe;let peak=0;const data=new Float32Array(a.fftSize);for(let i=0;i<15;i++){await new Promise(r=>setTimeout(r,100));a.getFloatTimeDomainData(data);for(const s of data)peak=Math.max(peak,Math.abs(s))}return JSON.stringify({time:v.currentTime,paused:v.paused,peak,state:ctx.state,error:window.__videoProbe.error||v.error?.message||null})})()`,
          ),
        );
        if (
          state.error ||
          state.paused ||
          state.time < 0.5 ||
          (audible && state.peak < 0.00001)
        )
          throw new Error(JSON.stringify(state));
        results.push({
          kind: tool.kind,
          type,
          video: clip.video,
          passed: true,
          ...state,
        });
        console.log(`${clip.video}: PASS`);
      } catch (error) {
        results.push({
          kind: tool.kind,
          type,
          video: clip.video,
          passed: false,
          error: String(error),
        });
        console.error(`${clip.video}: ${error}`);
      }
    }
  }
} finally {
  writeFileSync(
    resolve(root, "target/tool-guide-firefox-results.json"),
    JSON.stringify(results, null, 2),
  );
  ws.close();
  ff.kill();
}
if (results.some((result) => !result.passed)) process.exitCode = 1;
