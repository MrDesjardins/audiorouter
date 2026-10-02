import { randomUUID } from 'node:crypto';

export function localUrl(value, protocol) {
  const url = new URL(value);
  if (url.protocol !== protocol || url.hostname !== '127.0.0.1' ||
      url.username || url.password || url.search || url.hash || url.pathname !== '/') {
    throw new Error(`Use a ${protocol}//127.0.0.1:PORT address without credentials or a path.`);
  }
  return url.origin;
}

export function validateConfig(c) {
  localUrl(c.audioRouterUrl, 'http:'); localUrl(c.statsUrl, 'ws:');
  for (const key of ['mixer', 'gameInput', 'discordInput']) {
    if (typeof c[key] !== 'string' || !c[key].trim() || c[key].length > 256) throw new Error(`Invalid ${key}.`);
  }
  if (c.sessionId !== null && (typeof c.sessionId !== 'string' || !c.sessionId || c.sessionId.length > 256)) throw new Error('Invalid sessionId.');
  for (const key of ['quietPercent', 'actionPercent', 'discordPercent', 'restorePercent']) {
    if (!Number.isFinite(c[key]) || c[key] < 0 || c[key] > 100) throw new Error(`Invalid ${key}; use 0–100.`);
  }
  if (c.gameInput === c.discordInput) throw new Error('Game and Discord must use distinct inputs.');
  if (!Number.isInteger(c.statusPort) || c.statusPort < 1024 || c.statusPort > 65535) throw new Error('Invalid statusPort.');
  return c;
}

// Reduce raw vendor state immediately: never retain players, profiles or match IDs.
export function phaseState(snapshot, c) {
  if (!snapshot || typeof snapshot !== 'object' || Array.isArray(snapshot) ||
      !['connected', 'disconnected', 'loading', 'error'].includes(snapshot.status)) {
    return { phase: 'unknown', percent: c.restorePercent };
  }
  if (snapshot.status !== 'connected') return { phase: 'unavailable', percent: c.restorePercent };
  if (snapshot.match === null) return { phase: snapshot.startedQueuingAt ? 'queue' : 'menu', percent: c.quietPercent };
  const match = snapshot.match;
  if (!match || typeof match !== 'object' || Array.isArray(match) || !Object.hasOwn(match, 'phase')) {
    return { phase: 'unknown', percent: c.restorePercent };
  }
  if (match.ended_at != null) return { phase: 'results', percent: c.quietPercent };
  if (match.phase === 'action') return { phase: 'action', percent: c.actionPercent };
  if (match.phase === null) return { phase: 'match-start', percent: c.quietPercent };
  if (['planning', 'prep', 'results'].includes(match.phase)) return { phase: match.phase, percent: c.quietPercent };
  // The vendor defines phase as a string, not an exhaustive enum. Map/ban
  // selection and future non-action phases must remain quiet, not get boosted.
  if (typeof match.phase === 'string' && match.phase.length > 0 && match.phase.length <= 80) {
    return { phase: 'other-phase', percent: c.quietPercent };
  }
  return { phase: 'unknown', percent: c.restorePercent };
}

function findNode(nodes, ref) {
  const byId = nodes.filter(n => n.id === ref);
  const matches = byId.length ? byId : nodes.filter(n => n.name?.toLowerCase() === ref.toLowerCase());
  if (matches.length !== 1) throw new Error(`Expected exactly one node for ${ref}; found ${matches.length}. Use its ID if names repeat.`);
  return matches[0];
}

export function resolveMixer(graph, c) {
  if (!graph || !Array.isArray(graph.nodes) || !Array.isArray(graph.edges) || typeof graph.id !== 'string') throw new Error('Invalid session graph.');
  const mixer = findNode(graph.nodes, c.mixer);
  const game = findNode(graph.nodes, c.gameInput);
  const discord = findNode(graph.nodes, c.discordInput);
  if (mixer.kind !== 'mixer' || !mixer.enabled || mixer.bypass) throw new Error('Target must be an enabled Mixer with bypass off.');
  if (game.id === discord.id) throw new Error('Game and Discord resolve to the same node.');
  for (const input of [game, discord]) {
    const connections = graph.edges.filter(e => e.enabled && e.sourceNode === input.id && e.destinationNode === mixer.id);
    if (!input.enabled || connections.length !== 1) throw new Error(`${input.name} must directly feed the Mixer through one enabled connection.`);
  }
  return { sessionId: graph.id, mixerId: mixer.id, gameId: game.id, discordId: discord.id };
}

export function mixerRequest(target, percent, c, key = randomUUID()) {
  return { sessionId: target.sessionId, node: target.mixerId,
    parameters: { [`inputVolume:${target.gameId}`]: percent, [`inputVolume:${target.discordId}`]: c.discordPercent },
    idempotencyKey: key };
}

export class ApiError extends Error {
  constructor(status, category) { super(`AudioRouter API ${category} (HTTP ${status}).`); this.status = status; }
}
export class TargetError extends Error {}

export class AudioRouter {
  constructor(c, token, fetcher = fetch) { this.c = c; this.token = token; this.fetcher = fetcher; this.target = null; }
  async call(route, body) {
    const response = await this.fetcher(`${localUrl(this.c.audioRouterUrl, 'http:')}/api/v1/${route}`, {
      method: body === undefined ? 'GET' : 'POST', redirect: 'error',
      headers: { Authorization: `Bearer ${this.token}`, 'Content-Type': 'application/json' },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }), signal: AbortSignal.timeout(3000)
    });
    // Bound streamed responses before retaining arbitrary graph/error data.
    const reader = response.body.getReader(); let bytes = 0; const parts = [];
    try { while (true) { const { value, done } = await reader.read(); if (done) break;
      bytes += value.length; if (bytes > 4 * 1024 * 1024) { await reader.cancel(); throw new Error('API response exceeds limit.'); } parts.push(value);
    } } finally { reader.releaseLock(); }
    if (!response.ok) throw new ApiError(response.status, [401,403].includes(response.status) ? 'authorization denied; restart API and supply its current token' : 'request failed');
    return JSON.parse(Buffer.concat(parts).toString('utf8'));
  }
  async discover() {
    let id = this.target?.sessionId ?? this.c.sessionId;
    if (!id) id = (await this.call('sessions/active')).sessionId;
    if (!id) throw new Error('Select a session in AudioRouter or configure sessionId.');
    const graph = await this.call('sessions/get', { sessionId: id });
    let target;
    try { target = resolveMixer(graph, this.c); } catch(error) { throw new TargetError(error.message); }
    if (this.target && JSON.stringify(target) !== JSON.stringify(this.target)) throw new TargetError('Target graph changed. Stop and inspect the integration again.');
    this.target = target; return target;
  }
  async set(percent, key) {
    const target = await this.discover();
    const result = await this.call('nodes/set', mixerRequest(target, percent, this.c, key));
    const state = result.activation?.native?.state;
    if (state && !['applied', 'notPrepared', 'notRunning', 'unchanged'].includes(state)) {
      throw new TargetError(`Saved mixer edit requires attention: activation ${state}. Check AudioRouter before continuing.`);
    }
    return result;
  }
}

// One in-flight mutation. New snapshots replace queued levels, never build a backlog.
export class LevelController {
  constructor(write, report = () => {}, retryMs = 1000) {
    this.write = write; this.report = report; this.retryMs = retryMs;
    this.desired = null; this.applied = null; this.pending = null; this.running = null; this.stopped = false;
  }
  desire(percent) {
    if (this.stopped || this.desired === percent) return;
    this.desired = percent; this.pending = { percent, key: randomUUID() }; this.kick();
  }
  kick() {
    if (this.running || this.stopped) return;
    this.running = this.drain().finally(() => { this.running = null; if (this.pending && !this.stopped) this.kick(); });
  }
  async drain() {
    while (this.pending && !this.stopped) {
      const job = this.pending;
      if (this.applied === job.percent) { if (this.pending === job) this.pending = null; continue; }
      try { await this.write(job.percent, job.key); this.applied = job.percent; this.report('applied', job.percent);
        if (this.pending === job) this.pending = null;
      } catch (error) { this.report('error', job.percent, error);
        if (error instanceof TargetError || (error instanceof ApiError && [400,401,403,404].includes(error.status))) { this.stopped = true; return; }
        await new Promise(r => setTimeout(r, this.retryMs));
      }
    }
  }
  async stop(restore) {
    this.stopped = true; this.pending = null;
    if (this.running) await this.running;
    await this.write(restore, randomUUID());
  }
}
