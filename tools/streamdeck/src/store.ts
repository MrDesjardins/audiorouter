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
export type ParameterSpec = { name: string; type?: string; enum?: unknown[]; default?: unknown; minimum?: number; maximum?: number; unit?: string };
export type SessionItem = { id: string; name: string };
/** A Recorder node's state, from `recorders.list`. */
export type RecorderState = "idle" | "armed" | "recording" | "paused" | "stopping" | "completed" | "failed";
type RecorderItem = { sessionId: string; nodeId: string | null; state: RecorderState };
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
  /** All sessions (read while a Session key is visible). */
  sessions: SessionItem[] = [];
  private sessionWatchers = 0;
  /** Recorder states of the selected session by node (read while a Record key is visible). */
  recorders = new Map<string, RecorderState>();
  /** When each recording was first seen running, for the elapsed time. */
  recordingSince = new Map<string, number>();
  private recorderWatchers = 0;
  private sessionsReadAt = 0;
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
    this.recorders.clear();
    this.recordingSince.clear();
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

  /** A Session key appeared (+1) or disappeared (−1). */
  watchSessions(delta: 1 | -1) {
    this.sessionWatchers = Math.max(0, this.sessionWatchers + delta);
    if (delta > 0) this.sessionsReadAt = 0;
  }

  /** A Record key appeared (+1) or disappeared (−1). */
  watchRecorders(delta: 1 | -1) {
    this.recorderWatchers = Math.max(0, this.recorderWatchers + delta);
  }

  /** True while the node records (or is paused in a recording). */
  isRecording(nodeId: string): boolean {
    const state = this.recorders.get(nodeId);
    return state === "recording" || state === "paused";
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

  /** A setting's catalog description (range, unit, choices). */
  spec(node: SummaryNode, name: string): ParameterSpec | undefined {
    return this.kind(node.kind)?.parameters.find((spec) => spec.name === name);
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
      // The session list changes rarely: read it every 2 s while a Session key shows.
      let sessionsChanged = false;
      if (this.sessionWatchers > 0 && Date.now() - this.sessionsReadAt >= 2000) {
        const list = await this.call<{ items: SessionItem[] }>("sessions.list", { limit: 100 });
        if (generation !== this.generation) return;
        const sessions = list.items.map((item) => ({ id: item.id, name: item.name }));
        sessionsChanged = JSON.stringify(sessions) !== JSON.stringify(this.sessions);
        this.sessions = sessions;
        this.sessionsReadAt = Date.now();
      }
      // Recorder states, while a Record key shows: same tick as the summary.
      let recordersChanged = false;
      if (this.recorderWatchers > 0) {
        const list = await this.call<RecorderItem[]>("recorders.list");
        if (generation !== this.generation) return;
        const next = new Map<string, RecorderState>();
        for (const item of list) if (item.sessionId === summary.sessionId && item.nodeId) next.set(item.nodeId, item.state);
        recordersChanged = JSON.stringify([...next]) !== JSON.stringify([...this.recorders]);
        this.recorders = next;
        const now = Date.now();
        for (const [nodeId] of next) if (this.isRecording(nodeId) && !this.recordingSince.has(nodeId)) this.recordingSince.set(nodeId, now);
        for (const nodeId of [...this.recordingSince.keys()]) if (!this.isRecording(nodeId)) this.recordingSince.delete(nodeId);
        // Recording keys show elapsed time: redraw every tick while one runs.
        if (this.recordingSince.size > 0) recordersChanged = true;
      }
      const changed = recordersChanged || sessionsChanged || this.state !== "online" || JSON.stringify(summary) !== JSON.stringify(this.summary);
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
