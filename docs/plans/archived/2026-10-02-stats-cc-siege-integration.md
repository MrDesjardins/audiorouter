# Stats.cc → Siege mixer integration

Archived 2026-10-03: **completed.** The standalone example is implemented and
tested (11/11), the user enabled the Stats.cc feed and confirmed it working in
Siege, and the 30% policy is in place. Since 0.0.5 the built-in Duck
`siegeRound` trigger (DSP-19) covers the same need without a separate
service; the example remains a reference for REST integrations. The
undocumented Stats.cc feed can change; see the active plan for risks.

Date: 2026-10-02. User authorizes investigation, a plan and a standalone example
service. This is an external integration; no AudioRouter backend/VST changes.

## Objective and requirements

Use Stats.cc's Siege state transitions to set the Siege input of mixer
**Siege + Discord** to 100% during play and 30% otherwise. Discord input stays
at 100%. Resolve the direct inputs **Siege Advanced EQ** and **Discord.exe**
to real IDs; never adjust the mixer's master volume. Actual Discord node name
is **Discord.exe capture 1**. Scope: AUTO-15,
API-01/07/09, HTTP-01/03/04/06, GRAPH-05/06/08 and SEC-10.

## Prerequisites and findings

- Clean main at `7a783e12`; Windows and Node 22 available. User's AudioRouter
  runs as PID 48764; do not restart it or touch its database directly.
- Installed Stats.cc is Electron 1.8.1-stable. Read-only inspection of local
  `resources/app.asar` found a state broadcast and native event handlers.
  Extracted vendor code stays under ignored `target/stats-inspection`.
- State has `status`, `startedQueuingAt` and `match` with `phase`, `started_at`
  and `ended_at`. Match start creates a match with null phase; phase updates
  include `planning`, `prep`, `action`, `results`. Match end clears state.
- Optional WebSocket server reads a port from user-data file
  `ceb44052-d616-4bdc-993c-70bae41091e9.txt`. No file/listener currently found.
  It broadcasts state on change, pings every 30 seconds, and does not send an
  initial snapshot on connection. It binds `0.0.0.0`, without auth. Enabling
  it requires a normal Stats.cc restart. User says do not alter Stats.cc unless
  setup is automated for everyone: provide an automated, explicit setup command
  for the existing setting, leave the installed app unchanged during development.
- AudioRouter HTTP listener is currently stopped. User starts API and supplies
  current token privately at launch. Never save tokens, match/player payloads
  or account profiles. Real Siege transitions require attended gameplay.

## Decisions

- Separate Node service under `examples/integrations/stats-cc-siege/` with a
  hierarchy ready for more integrations. Stats.cc source adapter connects only
  to loopback. Status endpoint also binds only loopback, read-only/no token.
- User decision: only **action** is 100%. Preparation stays 30%, as do
  menu/queue/map/operator selection/planning/results and other non-action phases.
  Whole-match mode is not implemented because it contradicts this choice.
- Unknown/unavailable state restores Siege to 100%, to avoid suppressing game
  audio after source failure. No initial invented game state; wait for a fresh
  snapshot. Shutdown restores the explicitly chosen 100/100 levels.
- Pin the resolved session; do not follow later UI session switches. Validate
  exact node kind and enabled direct connections before mutation. One
  `nodes.set` updates both input percentages atomically. Serialized, coalesced
  writes; retry the same event key, bound HTTP timeouts and reconnect backoff.
- Dry-run/inspect modes allow review without changing audio. State snapshots
  are reduced immediately to status/phase/level; no raw payload logging.

## Ordered implementation

1. Document local reverse-engineering evidence and exact state/transition map;
   resolve boundary/feed-enablement decisions.
2. Implement state normalization and validated mixer target discovery using
   existing HTTP sessions/get and nodes/set contracts.
3. Implement independent service: WS adapter, retries/latest-state queue,
   redacted status endpoint, token prompt and graceful restore; example config.
4. Add useful regression fixtures/tests for state transitions, graph ambiguity,
   unchanged Discord, retry/idempotency, failure and shutdown. AGENTS.md requires
   these checks; they prove control behavior, not live game/audio evidence.
5. Run fixture/local service checks and documentation checks; live read-only
   discovery and attended transitions when Stats.cc/API are available.

## Validation matrix

| Scenario | Evidence required |
| --- | --- |
| Menu → queue → selection → prep → action → results → menu | Synthetic minimal snapshots: 30/30/30/30/100/30/30; confirm live phases later |
| Mixer target | Unique Mixer and two direct enabled inputs; only inputVolume IDs mutated; Discord 100 |
| Restart/disconnect/malformed data | No stale initial snapshot, fail-safe 100, bounded reconnect and diagnostic fields |
| API failure/token/session switch | Clear redacted error, serialized retry, pinned session retained |
| Stop | Bounded restore to 100/100; explicit error if API unavailable |
| Real route and Stats.cc | User-run read-only inspect plus attended Siege transitions; no mock claimed as hardware proof |

## Risks and rollback

Private Stats.cc protocol can change; version compatibility checks and captured
minimal schema fixtures needed. Its optional listener exposes raw state to LAN;
do not enable automatically or claim loopback-only vendor behavior. API edits
persist and may create undo history. Source can stop broadcasting when unchanged,
so use socket liveness, not inactivity timeout. Stop service and restore both
inputs to 100%; remove only its own optional setup file if explicitly authorized.

## Implementation and evidence

Implemented [standalone example](../../../examples/integrations/stats-cc-siege/README.md):
version-pinned WS adapter, pure phase policy, graph resolver, authenticated REST
client, one latest-state queue, retry/idempotency, read-only loopback status,
hidden token prompt, inspect/observe/dry-run, quit/interrupt restore and automated
feed setup/removal. Package lock pins `ws` 8.21.3; no AudioRouter/VST code changed.

Read-only named-pipe `sessions.get` against the running real app passed the same
resolver used by the script: session `patrick-main-native`, mixer `mixer-1`,
Siege direct input `siege-eq`, Discord direct input `application-capture-1`.
Current levels observed: Siege 80%, Discord 100%, left unchanged. The ordinary
CLI `api call` uses an independent control plane, so it did not inspect the
running graph; corrected inspection used the production named pipe directly.
No Stats.cc app code or setting, AudioRouter level, API listener or user process
was changed. Vendor source stays in ignored scratch files, not checked in.

Windows verification: `npm ci --ignore-scripts --offline`, `npm test`: **11/11
passed**, including a spawned standalone service receiving fixture WS snapshots
and calling a loopback HTTP server; actual REST bodies set correct input IDs,
Discord stays 100%, initial connection makes no invented edit, disconnect and
normal quit restore 100%. `node --check server.mjs` and diff checks passed.
The first shutdown test used SIGTERM, which force-kills Windows children rather
than exercising a normal console interrupt. The next quit test exposed lingering
stdin/HTTP references; cleanup now waits for restoration and drains stdout before
exiting. Final quit regression passes; only the failed fixture's verified child
was cleaned up. No user-owned app was terminated.

Follow-up (2026-10-02): user supplied the active API credential. Saved privately
under current-user Windows DPAPI, outside Git. Ignored `config.local.json` pins
the real session and three target IDs. The example now reads that saved credential
without prompting. `npm run inspect` authenticated to the running real HTTP API
and validated all target IDs; no audio levels changed. The eleven fixture tests
passed again. Token persistence and UI IDs are a separately authorized AudioRouter
change; see [evidence](../active/evidence/2026-10-02-identity-and-persistent-token.md).

Attended follow-up: user enabled the feed and reports integration working in
Siege. On 2026-10-02 the user requested non-action volume 30% instead of the
original 50%. Example and ignored local configuration, README and phase/service
fixtures now use 30%; action, Discord and restoration remain 100%. Windows
`npm.cmd test`: 11/11 passed after this update. Script reads config at startup:
type `quit`, then run `npm.cmd start` to apply the new policy. No user process
was stopped by the agent; new-policy attended confirmation remains next.
