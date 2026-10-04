/**
 * One shared view of AudioRouter for every key. A single poller reads the
 * selected session's summary (twice a second) and, only while a live key is
 * visible, its meter levels (ten times a second). Keys subscribe; nothing
 * polls per key.
 */
import { ApiError, AudioRouterClient, type Connection, type Fetch } from "./api.js";

export type SummaryNode = { id: string; name: string; kind: string; enabled: boolean; bypass: boolean };
export type Summary = { sessionId: string; name: string; revision: number; playing: boolean; privacyMuted: boolean; nodes: SummaryNode[] };
export type Level = { nodeId: string; name: string; peakDb: number | null; rmsDb: number | null; clipped: boolean; reductionDb: number | null; active: boolean | null };
export type ParameterSpec = { name: string; type?: string; enum?: unknown[]; default?: unknown };
export type CatalogKind = { kind: string; name: string; parameters: ParameterSpec[] };
type RawSession = { id: string; revision: number; nodes: { id: string; kind: string; parameters: Record<string, unknown> }[] };

export type ConnectionState = "unconfigured" | "connecting" | "online" | "offline";

const SUMMARY_MS = 500;
const LEVELS_MS = 100;
const BACKOFF_MS = [1000, 2000, 5000, 10_000];

/** Settings a key can toggle on a tool: Enabled, Bypass, and on/off or two-choice settings. */
export function toggleTargets(kind: CatalogKind | undefined): string[] {
  const settings = (kind?.parameters ?? [])
    .filter((spec) => spec.type === "boolean" || (Array.isArray(spec.enum) && spec.enum.length === 2))
    .map((spec) => spec.name);
  return ["enabled", "bypass", ...settings];
}

export class AudioRouterStore {
  state: ConnectionState = "unconfigured";
  error: string | null = null;
  summary: Summary | null = null;
  catalog: CatalogKind[] = [];
  levels = new Map<string, Level>();
  private parameters = new Map<string, Record<string, unknown>>();
  private client: AudioRouterClient | null = null;
  private generation = 0;
  private failures = 0;
  private levelWatchers = 0;
  private summaryTimer: ReturnType<typeof setTimeout> | null = null;
  private levelsTimer: ReturnType<typeof setTimeout> | null = null;
  private readonly listeners = new Set<() => void>();

  constructor(private readonly fetcher?: Fetch) {}

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  }

  private notify() {
    for (const listener of this.listeners) listener();
  }

  /** Use this connection (null: not set up). Restarts polling. */
  configure(connection: Connection | null) {
    this.generation += 1;
    this.stopTimers();
    this.summary = null;
    this.levels.clear();
    this.parameters.clear();
    this.failures = 0;
    this.client = connection ? new AudioRouterClient(connection, this.fetcher) : null;
    this.state = connection ? "connecting" : "unconfigured";
    this.error = null;
    this.notify();
    if (this.client) void this.pollSummary(this.generation);
  }

  /** Call a method on the connected AudioRouter. */
  call<T>(method: string, params: Record<string, unknown> = {}): Promise<T> {
    if (!this.client) return Promise.reject(new ApiError("Set up the AudioRouter connection in this key's settings", null));
    return this.client.call<T>(method, params);
  }

  /** A key that shows live levels appeared (+1) or disappeared (−1). */
  watchLevels(delta: 1 | -1) {
    this.levelWatchers = Math.max(0, this.levelWatchers + delta);
    if (this.levelWatchers > 0 && !this.levelsTimer && this.state === "online") this.scheduleLevels(this.generation, 0);
    if (this.levelWatchers === 0 && this.levelsTimer) { clearTimeout(this.levelsTimer); this.levelsTimer = null; }
  }

  /** Read the summary now (after a press), instead of at the next tick. */
  refreshSoon() {
    if (!this.client) return;
    if (this.summaryTimer) clearTimeout(this.summaryTimer);
    this.summaryTimer = setTimeout(() => void this.pollSummary(this.generation), 30);
  }

  node(nodeId: string, nodeName = ""): SummaryNode | undefined {
    const nodes = this.summary?.nodes ?? [];
    return nodes.find((node) => node.id === nodeId) ?? (nodeName ? nodes.find((node) => node.name === nodeName) : undefined);
  }

  /** A node's current value of a setting (saved, else the catalog default). */
  setting(node: SummaryNode, name: string): unknown {
    if (name === "enabled") return node.enabled;
    if (name === "bypass") return node.bypass;
    const saved = this.parameters.get(node.id)?.[name];
    if (saved !== undefined) return saved;
    return this.kind(node.kind)?.parameters.find((spec) => spec.name === name)?.default;
  }

  kind(kind: string): CatalogKind | undefined {
    return this.catalog.find((entry) => entry.kind === kind);
  }

  private stopTimers() {
    if (this.summaryTimer) clearTimeout(this.summaryTimer);
    if (this.levelsTimer) clearTimeout(this.levelsTimer);
    this.summaryTimer = this.levelsTimer = null;
  }

  private async pollSummary(generation: number) {
    this.summaryTimer = null;
    try {
      if (this.catalog.length === 0) this.catalog = await this.call<CatalogKind[]>("nodes.catalog");
      const summary = await this.call<Summary>("sessions.summary");
      if (generation !== this.generation) return;
      if (summary.revision !== this.summary?.revision || summary.sessionId !== this.summary?.sessionId) {
        const raw = await this.call<RawSession>("sessions.get", { sessionId: summary.sessionId });
        if (generation !== this.generation) return;
        this.parameters = new Map(raw.nodes.map((node) => [node.id, node.parameters ?? {}]));
      }
      const changed = this.state !== "online" || JSON.stringify(summary) !== JSON.stringify(this.summary);
      this.summary = summary;
      this.failures = 0;
      this.error = null;
      if (this.state !== "online") {
        this.state = "online";
        if (this.levelWatchers > 0 && !this.levelsTimer) this.scheduleLevels(generation, 0);
      }
      if (changed) this.notify();
      if (generation === this.generation) this.summaryTimer = setTimeout(() => void this.pollSummary(generation), SUMMARY_MS);
    } catch (error) {
      if (generation !== this.generation) return;
      this.state = "offline";
      this.error = error instanceof Error ? error.message : "AudioRouter is not reachable";
      this.notify();
      const delay = BACKOFF_MS[Math.min(this.failures, BACKOFF_MS.length - 1)];
      this.failures += 1;
      this.summaryTimer = setTimeout(() => void this.pollSummary(generation), delay);
    }
  }

  private scheduleLevels(generation: number, delay: number) {
    this.levelsTimer = setTimeout(() => void this.pollLevels(generation), delay);
  }

  private async pollLevels(generation: number) {
    this.levelsTimer = null;
    if (generation !== this.generation || this.levelWatchers === 0 || this.state !== "online") return;
    try {
      const result = await this.call<{ levels: Level[] }>("meters.levels");
      if (generation !== this.generation) return;
      this.levels = new Map(result.levels.map((level) => [level.nodeId, level]));
      this.notify();
    } catch {
      // The summary poller reports connection problems and retries.
    }
    if (generation === this.generation && this.levelWatchers > 0 && this.state === "online") this.scheduleLevels(generation, LEVELS_MS);
  }
}
