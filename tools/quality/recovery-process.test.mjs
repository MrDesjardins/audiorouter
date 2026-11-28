import test from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createInterface } from "node:readline";

// A real, freshly built executable, synthetic audio and an isolated database.
// The fixture cannot open audio devices or touch the user's AudioRouter profile.
test("recovery lifecycle survives real backend process boundaries", { timeout: 25_000 }, async () => {
  const directory = await mkdtemp(path.join(tmpdir(), "audiorouter-e2e-recovery-"));
  const executable = path.resolve("target/debug/examples/e2e_backend.exe");
  let process;
  let lines;
  let sequence = 0;
  let stderr = "";
  const exchanges = [];
  const start = () => {
    process = spawn(executable, [directory], { windowsHide: true, stdio: "pipe" });
    process.stderr.on("data", (bytes) => { stderr = (stderr + bytes).slice(-4000); });
    lines = createInterface({ input: process.stdout })[Symbol.asyncIterator]();
  };
  const send = async (method, params = {}) => {
    const mutating = new Set(["sessions.create", "sessions.start", "sessions.stop",
      "system.osTransition", "recorders.create", "recorders.arm", "recorders.start"]);
    const request = { jsonrpc: "2.0", id: ++sequence, method,
      params: { ...params, ...(mutating.has(method) ? { idempotencyKey: `qualification-${sequence}` } : {}) } };
    process.stdin.write(`${JSON.stringify(request)}\n`);
    let timeout;
    let line;
    try {
      line = await Promise.race([lines.next(), new Promise((_, reject) => {
        timeout = setTimeout(() => {
          process.kill();
          reject(new Error(`backend deadline: ${method}; ${stderr}`));
        }, 5000);
      })]);
    } finally { clearTimeout(timeout); }
    assert.equal(line.done, false, `backend exited: ${stderr}`);
    const reply = JSON.parse(line.value);
    assert.equal(reply.id, request.id);
    exchanges.push({ method, reply });
    return reply;
  };
  const call = async (method, params) => {
    const reply = await send(method, params);
    assert.equal(reply.error, undefined, JSON.stringify(reply));
    return reply.result;
  };
  const close = async () => {
    if (!process || process.exitCode !== null) return;
    const child = process;
    const exited = once(child, "exit");
    child.stdin.end();
    const timeout = setTimeout(() => child.kill(), 2000);
    try {
      const [code] = await exited;
      assert.equal(code, 0, stderr);
    } finally { clearTimeout(timeout); }
  };
  const id = "recovery-process";
  const port = (name, direction) => ({ name, direction, channels: 2 });
  const session = {
    id, name: "Disposable recovery qualification", schemaVersion: 1, revision: 0,
    nodes: [
      { id: "tone", name: "Synthetic tone", kind: "testSignal", typeVersion: 1,
        enabled: true, bypass: false, parameters: {}, ports: [port("out", "output")] },
      { id: "meter", name: "Meter", kind: "meter", typeVersion: 1,
        enabled: true, bypass: false, parameters: {}, ports: [port("in", "input"), port("out", "output")] },
    ],
    edges: [{ id: "tone-meter", sourceNode: "tone", sourcePort: "out",
      destinationNode: "meter", destinationPort: "in", matrix: [1, 0, 0, 1], enabled: true }],
  };
  try {
    start();
    await call("sessions.create", { session });
    await call("sessions.start", { sessionId: id });
    await call("system.osTransition", { transition: "sleep" });
    await call("system.osTransition", { transition: "sleep" });
    assert.deepEqual((await call("system.osTransition", { transition: "resume" })).sessionIds, [id]);
    assert.equal((await call("status.get")).activeSessionCount, 0);
    await call("sessions.start", { sessionId: id });
    const recorder = await call("recorders.create", { sessionId: id, recorderId: "synthetic",
      sequence: 1, channels: 2, sampleRate: 48000, queueCapacity: 32,
      maximumChunksPerPass: 32, format: "wavFloat32" });
    await call("recorders.arm", { sessionId: id });
    assert.ok((await send("system.osTransition", { transition: "sleep" })).error);
    assert.equal((await call("status.get")).activeSessionCount, 1);
    await call("recorders.start", { sessionId: id, frame: 0 });
    await call("fixture.recordSyntheticQuantum", { sessionId: id, frame: 0 });
    await call("fixture.recordSyntheticQuantum", { sessionId: id, frame: 128 });
    const stopped = await call("sessions.stop", { sessionId: id });
    assert.equal(stopped.state, "stopped");
    assert.equal(stopped.recorders[0].fileFinalized, true);
    const recordings = await call("recordings.list", { sessionId: id });
    assert.equal(recordings.length, 1);
    assert.equal(recordings[0].frames, 256);
    const wav = await readFile(recorder.path);
    assert.equal(wav.subarray(0, 4).toString(), "RIFF");
    assert.equal(wav.subarray(8, 12).toString(), "WAVE");
    let dataOffset;
    let dataBytes;
    for (let offset = 12; offset + 8 <= wav.length;) {
      const bytes = wav.readUInt32LE(offset + 4);
      assert.ok(offset + 8 + bytes <= wav.length, "complete WAV chunk");
      if (wav.subarray(offset, offset + 4).toString() === "data") {
        dataOffset = offset + 8;
        dataBytes = bytes;
        break;
      }
      offset += 8 + bytes + (bytes % 2);
    }
    assert.equal(dataBytes, 256 * 2 * 4);
    for (let frame = 0; frame < 256; frame++) {
      assert.ok(Math.abs(wav.readFloatLE(dataOffset + frame * 8) - 0.1) < 1e-6);
      assert.ok(Math.abs(wav.readFloatLE(dataOffset + frame * 8 + 4) + 0.1) < 1e-6);
    }
    await close();
    start();
    assert.equal((await call("status.get")).activeSessionCount, 0);
    assert.deepEqual(await call("recordings.list", { sessionId: id }), recordings);
    assert.deepEqual(await call("sessions.get", { sessionId: id }), session);
    assert.deepEqual((await call("system.osTransition", { transition: "resume" })).sessionIds, []);
    console.log(JSON.stringify({ directory, wavBytes: wav.length, exchanges }, null, 2));
  } finally { await close(); }
});
