/**
 * The AudioRouter local HTTP API (spec 16): every backend method is
 * `POST /api/v1/{namespace}/{operation}` with a bearer token. The plugin
 * only talks to the address the user pasted (normally 127.0.0.1).
 */

export type Connection = { baseUrl: string; token: string };

export class ApiError extends Error {
  constructor(message: string, readonly status: number | null) {
    super(message);
  }
}

/** The API base URL, accepted only as http(s) with a host and no path. */
export function normalizeBaseUrl(text: string): string | null {
  try {
    const url = new URL(text.trim());
    if ((url.protocol !== "http:" && url.protocol !== "https:") || !url.host) return null;
    return `${url.protocol}//${url.host}`;
  } catch {
    return null;
  }
}

let keyCounter = 0;
/** A fresh idempotency key: each press is a new operation. */
export function idempotencyKey(prefix: string): string {
  keyCounter = (keyCounter + 1) % 1_000_000;
  return `streamdeck-${prefix}-${Date.now()}-${keyCounter}`;
}

export type Fetch = (input: string, init: { method: string; headers: Record<string, string>; body?: string; signal: AbortSignal }) => Promise<{ ok: boolean; status: number; json(): Promise<unknown>; text(): Promise<string> }>;

export class AudioRouterClient {
  constructor(private readonly connection: Connection, private readonly fetcher: Fetch = fetch as unknown as Fetch, private readonly timeoutMs = 2000) {}

  /** Call one backend method, e.g. `call("meters.levels", {})`. */
  async call<T>(method: string, params: Record<string, unknown> = {}): Promise<T> {
    // The HTTP API maps POST /api/v1/a/b/c to method a.b.c (e.g. sessions.active.set).
    const parts = method.split(".");
    if (parts.length < 2 || parts.some((part) => !/^[A-Za-z]+$/.test(part))) throw new ApiError(`bad method ${method}`, null);
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);
    try {
      const response = await this.fetcher(`${this.connection.baseUrl}/api/v1/${parts.join("/")}`, {
        method: "POST",
        headers: { Authorization: `Bearer ${this.connection.token}`, "Content-Type": "application/json" },
        body: JSON.stringify(params),
        signal: controller.signal,
      });
      const body = await response.json().catch(() => null) as { error?: { message?: string } } | null;
      if (!response.ok) {
        const reason = body?.error?.message ?? (response.status === 401 ? "The API token was not accepted. Copy it again from AudioRouter's API tab." : `HTTP ${response.status}`);
        throw new ApiError(reason, response.status);
      }
      return body as T;
    } catch (error) {
      if (error instanceof ApiError) throw error;
      throw new ApiError(controller.signal.aborted ? "AudioRouter did not answer in time" : "AudioRouter is not reachable (is its API started?)", null);
    } finally {
      clearTimeout(timer);
    }
  }
}
