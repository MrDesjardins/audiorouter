import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createInterface } from "node:readline";
import { once } from "node:events";
import { expect, test as base } from "@playwright/test";
import type { Session } from "../../contracts/src/index";

export const initialSession: Session = {
  id: "e2e-session",
  name: "Offline qualification",
  schemaVersion: 1,
  revision: 0,
  nodes: [
    {
      id: "mic",
      name: "Microphone",
      kind: "physicalInput",
      typeVersion: 1,
      enabled: true,
      bypass: false,
      parameters: {},
      ports: [{ name: "out", direction: "output", channels: 2 }],
    },
    {
      id: "voice",
      name: "Voice gain",
      kind: "gain",
      typeVersion: 1,
      enabled: true,
      bypass: false,
      parameters: { gainDb: 0 },
      ports: [
        { name: "in", direction: "input", channels: 2 },
        { name: "out", direction: "output", channels: 2 },
      ],
    },
    {
      id: "headphones",
      name: "Headphones",
      kind: "physicalOutput",
      typeVersion: 1,
      enabled: true,
      bypass: false,
      parameters: {},
      ports: [{ name: "in", direction: "input", channels: 2 }],
    },
  ],
  edges: [
    {
      id: "mic-voice",
      sourceNode: "mic",
      sourcePort: "out",
      destinationNode: "voice",
      destinationPort: "in",
      matrix: [1, 0, 0, 1],
      enabled: true,
    },
    {
      id: "voice-out",
      sourceNode: "voice",
      sourcePort: "out",
      destinationNode: "headphones",
      destinationPort: "in",
      matrix: [1, 0, 0, 1],
      enabled: true,
    },
  ],
};

export class RealBackend {
  private process!: ChildProcessWithoutNullStreams;
  private sequence = 0;
  private restarting: Promise<void> | null = null;
  private pending = new Map<
    number,
    // JSON-RPC test client: responses are untyped JSON by design.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    { resolve: (value: any) => void; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> }
  >();
  readonly methods: string[] = [];
  /** Last stderr of the current fixture process and how it ended, for failure reports. */
  stderrTail = "";
  exitCode: number | null = null;
  constructor(readonly directory: string) {
    this.start();
  }
  private start() {
    const executable = path.resolve(
      "../target/debug/examples",
      process.platform === "win32" ? "e2e_backend.exe" : "e2e_backend",
    );
    this.process = spawn(executable, [this.directory], { windowsHide: true, stdio: "pipe" });
    let stderr = "";
    this.stderrTail = "";
    this.exitCode = null;
    this.process.stderr.on("data", (bytes) => {
      stderr = (stderr + bytes.toString()).slice(-4000);
      this.stderrTail = stderr;
    });
    this.process.on("exit", (code) => {
      this.exitCode = code;
    });
    createInterface({ input: this.process.stdout }).on("line", (line) => {
      try {
        const response = JSON.parse(line);
        const pending = this.pending.get(response.id);
        if (pending) {
          clearTimeout(pending.timer);
          this.pending.delete(response.id);
          pending.resolve(response);
        }
      } catch (error) {
        this.rejectPending(new Error(`Invalid fixture output: ${error}`));
      }
    });
    this.process.on("error", (error) => this.rejectPending(error));
    this.process.stdin.on("error", (error) => this.rejectPending(error));
    this.process.on("exit", (code) => this.rejectPending(new Error(`Rust fixture exited ${code}: ${stderr}`)));
  }
  private rejectPending(error: Error) {
    for (const item of this.pending.values()) {
      clearTimeout(item.timer);
      item.reject(error);
    }
    this.pending.clear();
  }
  async send(request: { method: string; params?: unknown; id?: unknown }) {
    if (this.restarting) await this.restarting;
    const id = ++this.sequence;
    this.methods.push(request.method);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any -- untyped JSON-RPC response
    const response: any = await new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Fixture RPC timeout: ${request.method}`));
      }, 10_000);
      this.pending.set(id, { resolve, reject, timer });
      this.process.stdin.write(
        `${JSON.stringify({ jsonrpc: "2.0", id, method: request.method, params: request.params })}\n`,
      );
    });
    return { ...response, id: request.id ?? id };
  }
  // Tests read ad hoc result fields, so the default result type stays `any`.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  async call<T = any>(method: string, params?: unknown): Promise<T> {
    const response = await this.send({ method, params });
    if (response.error)
      throw new Error(`${method}: ${response.error.message} ${JSON.stringify(response.error.data ?? {})}`);
    return response.result;
  }
  async close() {
    if (this.process.exitCode !== null) return;
    const exited = once(this.process, "exit");
    this.process.stdin.end();
    await exited;
  }
  async restart() {
    this.restarting = (async () => {
      await this.close();
      this.start();
    })();
    try {
      await this.restarting;
    } finally {
      this.restarting = null;
    }
  }
}

export const test = base.extend<{ backend: RealBackend }>({
  backend: [
    async ({ page }, use, testInfo) => {
      const directory = await mkdtemp(path.join(tmpdir(), "audiorouter-e2e-"));
      const backend = new RealBackend(directory);
      const pageErrors: string[] = [];
      page.on("pageerror", (error) => pageErrors.push(error.message));
      try {
        await backend.call("sessions.create", { session: initialSession, idempotencyKey: "fixture-create" });
        await page.route("**/__e2e_rpc", async (route) => {
          // Answer each request exactly once: a backend failure becomes a 503,
          // while a fulfil that fails because the page already abandoned the
          // request has nobody left to answer (answering again would throw
          // "Route is already handled" and hide the real outcome).
          let response: { status: number; contentType?: string; body: string };
          try {
            response = {
              status: 200,
              contentType: "application/json",
              body: JSON.stringify(await backend.send(route.request().postDataJSON())),
            };
          } catch (error) {
            response = { status: 503, body: String(error) };
          }
          if (page.isClosed()) return;
          await route.fulfill(response).catch(() => undefined);
        });
        await use(backend);
        if (testInfo.status !== testInfo.expectedStatus) await reportFailureState(page, backend);
        expect(pageErrors, "No uncaught UI exceptions").toEqual([]);
      } finally {
        await page.unroute("**/__e2e_rpc");
        await backend.close();
        // This exact directory was created exclusively above, beneath tmpdir.
        if (
          path.dirname(directory) !== path.resolve(tmpdir()) ||
          !path.basename(directory).startsWith("audiorouter-e2e-")
        )
          // Deliberate: refusing to delete an unexpected path must fail the
          // test even when it would hide the test's own error.
          // eslint-disable-next-line no-unsafe-finally
          throw new Error("Invalid cleanup target");
        await rm(directory, { recursive: true, force: true });
      }
    },
    { auto: true },
  ],
});
export { expect };

/** On a failed test, print what the page shows and the fixture backend's state (CI has no screen). */
async function reportFailureState(page: import("@playwright/test").Page, backend: RealBackend) {
  const visible = await page
    .evaluate(() =>
      [...document.querySelectorAll("h1, h2, [role=alert], [role=status]")]
        .map(
          (element) =>
            `${element.tagName.toLowerCase()}${element.getAttribute("role") ? `[${element.getAttribute("role")}]` : ""}: ${(element.textContent ?? "").trim().slice(0, 160)}`,
        )
        .filter((line) => !line.endsWith(": "))
        .slice(0, 25),
    )
    .catch((error) => [`page unavailable: ${error}`]);
  const sessions = await backend
    .call("sessions.list")
    .then((list) => JSON.stringify(list.items?.map((item: { id: string; name: string }) => `${item.id}=${item.name}`)))
    .catch((error) => `sessions.list failed: ${error}`);
  console.log(
    [
      "E2E FAILURE STATE",
      ...visible,
      `backend exit: ${backend.exitCode ?? "running"}; sessions: ${sessions}`,
      `backend stderr: ${backend.stderrTail.slice(-1500) || "(empty)"}`,
    ].join("\n  "),
  );
}
