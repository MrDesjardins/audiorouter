// Every audible tool guide in stock Firefox over WebDriver BiDi: a real click
// on Play, the level reaching the output, then a control change. Chrome and
// Edge tolerate Web Audio usage that Firefox does not (see guide-signal.js),
// and the Playwright suite only runs Edge. Usage, with Firefox installed:
//   node tools/site/firefox-guide-audio.mjs file:///C:/code/audiorouter/site/tools/ [kind]
// Exit code 1 when any guide is silent, not ready, or reports a page error.
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [base, only] = process.argv.slice(2);
const manifest = JSON.parse(
  readFileSync(
    new URL("../../site/assets/guide-manifest.json", import.meta.url),
    "utf8",
  ),
);
const profile = mkdtempSync(join(tmpdir(), "ffsweep-"));
const port = 9334;
const ff = spawn("C:/Program Files/Mozilla Firefox/firefox.exe", [
  "--headless",
  "--no-remote",
  "--profile",
  profile,
  `--remote-debugging-port=${port}`,
  "--width=1280",
  "--height=900",
]);
let ws;
for (let i = 0; i < 60 && !ws; i++) {
  await new Promise((r) => setTimeout(r, 500));
  try {
    const s = new WebSocket(`ws://127.0.0.1:${port}/session`);
    await new Promise((res, rej) => {
      s.onopen = res;
      s.onerror = rej;
    });
    ws = s;
  } catch {}
}
let id = 0;
const pending = new Map();
ws.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) {
    pending.get(m.id)(m);
    pending.delete(m.id);
  }
};
const send = (method, params) =>
  new Promise((resolve) => {
    const n = ++id;
    pending.set(n, resolve);
    ws.send(JSON.stringify({ id: n, method, params }));
  });
await send("session.new", { capabilities: {} });
const tree = await send("browsingContext.getTree", {});
const context = tree.result.contexts[0].context;
const evaluate = async (expression) => {
  const r = await send("script.evaluate", {
    expression,
    target: { context },
    awaitPromise: true,
  });
  return r.result?.result?.value ?? JSON.stringify(r.result ?? r.error ?? r);
};
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const hook = `(() => {
  const make = AudioContext.prototype.createDynamicsCompressor;
  AudioContext.prototype.createDynamicsCompressor = function () {
    const node = make.call(this);
    const a = this.createAnalyser(); a.fftSize = 2048; node.connect(a);
    window.__a = a; window.__ctx = this; return node;
  };
  window.__errors = [];
  addEventListener("error", (e) => window.__errors.push(e.message));
  addEventListener("unhandledrejection", (e) => window.__errors.push(String(e.reason)));
  return "ok";
})()`;
const level = `(async () => {
  const a = window.__a; if (!a) return "no-context";
  const b = new Float32Array(a.fftSize); let max = 0;
  for (let i = 0; i < 10; i++) { a.getFloatTimeDomainData(b); for (const v of b) max = Math.max(max, Math.abs(v)); await new Promise((r) => setTimeout(r, 100)); }
  return window.__ctx.state + "/" + max.toFixed(3);
})()`;
let failures = 0;
for (const guide of manifest.guides.filter(
  (g) => g.audioRelevant && (!only || g.kind === only),
)) {
  await send("browsingContext.navigate", {
    context,
    url: base + guide.path.split("/").pop(),
    wait: "complete",
  });
  await evaluate(hook);
  const ready = await evaluate(
    `(async () => { for (let i = 0; i < 100; i++) { if (document.querySelector("[data-guide-demo]")?.dataset.signalReady === "true") return "ready"; await new Promise((r) => setTimeout(r, 100)); } return "not-ready"; })()`,
  );
  const label = guide.kind === "recorder" ? "Record example" : "Play example";
  const node = await send("script.evaluate", {
    expression: `[...document.querySelectorAll(".guide-animation button")].find((b) => b.textContent.trim() === ${JSON.stringify(label)})`,
    target: { context },
    awaitPromise: false,
  });
  const button = node.result?.result;
  let clicked;
  if (button?.sharedId) {
    await send("script.callFunction", {
      functionDeclaration:
        "(el) => el.scrollIntoView({ block: 'center', behavior: 'instant' })",
      arguments: [{ sharedId: button.sharedId }],
      target: { context },
      awaitPromise: false,
    });
    await wait(200);
    clicked = await send("input.performActions", {
      context,
      actions: [
        {
          type: "pointer",
          id: "mouse",
          actions: [
            {
              type: "pointerMove",
              x: 0,
              y: 0,
              origin: {
                type: "element",
                element: { sharedId: button.sharedId },
              },
            },
            { type: "pointerDown", button: 0 },
            { type: "pointerUp", button: 0 },
          ],
        },
      ],
    });
  }
  await wait(700);
  // Network examples stay silent until pairing at 2.4 s.
  if (guide.kind !== "recorder")
    await evaluate(
      `(() => { const s = [...document.querySelectorAll("input[type=range]")].find((i) => i.closest(".guide-animation")); if (s) { s.value = "3"; s.dispatchEvent(new Event("input", { bubbles: true })); } return "ok"; })()`,
    );
  await wait(300);
  const before = await evaluate(level);
  const state1 = await evaluate(
    `document.querySelector("[data-guide-demo]").dataset.audioState`,
  );
  let after = "-";
  if (guide.kind !== "recorder") {
    const changed = await evaluate(
      `(() => { const el = document.querySelector("[data-demo-key]"); if (!el) return "none"; const min = +el.min, max = +el.max, step = +el.step || 1; el.value = String(Math.round(((min + max) / 2 + (max - min) / 5) / step) * step); el.dispatchEvent(new Event("input", { bubbles: true })); return el.dataset.demoKey; })()`,
    );
    if (changed !== "none") {
      await wait(500);
      after = await evaluate(level);
    }
  }
  const state2 = await evaluate(
    `document.querySelector("[data-guide-demo]").dataset.audioState`,
  );
  const errors = await evaluate(`JSON.stringify(window.__errors)`);
  const peakBefore = Number(String(before).split("/")[1] || 0);
  const peakAfter =
    after === "-" ? peakBefore : Number(String(after).split("/")[1] || 0);
  const ok =
    ready === "ready" &&
    peakBefore > 0.01 &&
    peakAfter > 0.001 &&
    errors === "[]";
  if (!ok) failures++;
  console.log(
    (ok ? "OK  " : "FAIL") + " " + guide.kind.padEnd(16),
    ready,
    state1,
    "->",
    state2,
    "| level",
    before,
    "->",
    after,
    errors === "[]" ? "" : "errors " + errors,
  );
}
console.log("failures", failures);
ff.kill();
setTimeout(() => {
  try {
    rmSync(profile, { recursive: true, force: true });
  } catch {}
  process.exit(failures ? 1 : 0);
}, 1500);
