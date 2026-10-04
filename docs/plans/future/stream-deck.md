# Stream Deck control

Status: plan written 2026-10-04 at the user's request ("control AudioRouter
from my Elgato Stream Deck while playing, without reaching the window").
Not authorized for implementation; each phase below needs the user's go-ahead.
Related: [hardware control panel](hardware-control-panel.md) (same API, a
different surface), [external app integrations](external-app-integrations.md)
(scenes and temporary overrides).

## Decisions (user, 2026-10-04)

1. Device: Stream Deck MK.2, 15 keys (72 px keys, 144 px @2x).
2. Keys follow the selected session by default.
3. Privacy mute: plain clicks. A key is Toggle, Mute only, or Unmute only
   (two keys for separate mute and unmute buttons). No hold.
4. Distribution: ship the packed `.streamDeckPlugin` with AudioRouter's
   GitHub releases while in alpha; Elgato Marketplace later (same package).
5. Phase 0 approved and started.

## Progress

- 2026-10-04, phase 0 started: `tools/streamdeck/` (TypeScript,
  `@elgato/streamdeck` 3.0.1, Node 24, Stream Deck 7.1+; the user has 7.4.2).
  Actions Toggle (latch or momentary; Enabled, Bypass, on/off and
  two-choice settings via `nodes.toggle` / explicit `nodes.set`), Level
  meter (`meters.levels`, polled only while visible) and Privacy mute
  (toggle, mute only, unmute only). One shared store polls
  `sessions.summary` every 500 ms (raw settings from `sessions.get` when the
  revision changes); offline and not-found faces; backoff 1–10 s. Settings
  panels use the property-inspector WebSocket protocol directly (no
  third-party code) with live tool and setting lists. The existing API
  already covers the plan's first gap: `meters.levels` is the compact meter
  read. `npm run pack` validates and packs an 80 KB `.streamDeckPlugin`.
  Tests: 12 Vitest tests (faces, client, store: follow session, raw
  settings, quick confirm, level polling only while visible, offline backoff
  and recovery). Live routes confirmed on the user's AudioRouter (401
  without a token). Not yet: attended test on the device (latency, CPU,
  look), release integration.
- 2026-10-04, attended defects: (1) a Session key switched the selection
  but left the previous session prepared, so Play on the new one failed
  ("Audio is already prepared"). The key now stops every playing session
  (`status.get` `activeSessionIds`, then `session.stop`), selects, and plays
  the new one when audio was playing or the key asks to
  (`src/switching.ts`, `test/switching.test.ts`). (2) After AudioRouter
  restarted, every key showed Offline: the local API only ran after Start
  in the API panel. By user request this is an explicit option, like
  autoplay: Advanced → When AudioRouter starts → "Start the local API
  automatically" (`shell-settings.json` `apiAutoStart`, commands
  `api_autostart_get`/`_set`). It uses the port and network the API last
  started with (`api`), or this PC only on 17891; local-network access
  resumes only on an address the PC still has.
- 2026-10-04, Record key (user request): Record / Stop on a Recorder node of
  the selected session, like its Record button (`recorders.startRecording`
  / `recorders.stopRecording`). Press: toggle, only start, only stop. The
  key names a Recorder, or follows the session's only one. The store reads
  `recorders.list` with the summary only while a Record key shows; the face
  shows RECORD, REC m:ss while recording, or PLAY FIRST when the session is
  stopped (a press then alerts). Two states for custom images. Tests:
  `test/record.test.ts`. Not yet: attended test on the device.

## Objective

While a game runs full screen, every common AudioRouter action is one key or
one dial away, and the keys show the truth: what is muted, bypassed, playing,
how loud each path is, whether the voice reaches Discord, whether ducking is
active. Nothing requires alt-tabbing to the window.

Success means, on the user's own session:

- Any tool's Enabled, Bypass or on/off setting toggles from a key in under
  100 ms from press to audible change, and the key reflects the backend state
  (also when changed elsewhere: window, tray, API) within 200 ms.
- Level meters on keys and dials move smoothly (about 10 updates per second)
  with AudioRouter and the Stream Deck app together using no more CPU than
  today's open AudioRouter window.
- Choosing what a key controls never needs IDs: sessions, tools and settings
  are picked by name from lists the plugin reads from AudioRouter.

## What AudioRouter already offers (no backend change needed)

The plugin talks to the local HTTP API (spec 16) on `127.0.0.1`, with the
bearer token from the API tab. Every backend method is
`POST /api/v1/{namespace}/{operation}`.

| Need | Method today | Notes |
| --- | --- | --- |
| Sessions and the selected one | `GET /api/v1/sessions`, `GET`/`PUT /api/v1/sessions/active` | Selection survives restart (backend, 0.0.10). |
| Play / Stop | `sessions.togglePlay`, `POST /api/v1/sessions/start`, `session.stop` | `togglePlay` is ideal for one key. |
| Privacy mute | `safety.togglePrivacyMute`, `safety.setPrivacyMute` | The global microphone kill switch. |
| Toggle any on/off thing | `nodes.toggle` | Enabled, Bypass, any on/off setting, or a two-choice setting (Input Switch A/B). Node found by ID or name. |
| Set a value | `nodes.set` | Parameters (gain, volume, Mixer input levels by upstream name, Duck amount…), enabled, bypass, name. Live and saved. |
| What can be set | `nodes.catalog` | Per tool kind: settings, types, ranges, choices; the request builder already filters it. |
| Live levels and states | `system.diagnostics` → `nodeTelemetry` | Per node: meter (peak/RMS per channel, clips), processor gain reduction and gate state (Compressor, Gate, Duck), network counters, plugin health; also `gameRound` (Siege) and privacy mute. |
| Audio files / Test Signal | `audioSources.transport` | Play/stop one source without stopping others (soundboard). |
| Time Shift | `timeShift.transport` | Pause, jump ±10 s, back to live. |
| Recording | `recorders.startRecording` / `stopRecording`, `recorders.list` | One-click recording already exists in the window. |
| Changes made elsewhere | `events.subscribe` (cursor replay) | Polled; no push yet (see gaps). |

Budget: the HTTP API refills 20 tokens per second up to 40; a read costs 0.1,
a write 1. A 10 Hz meter poll costs about 1 token per second; presses and dial
turns (coalesced, below) stay far under the write rate.

## Architecture

```
Stream Deck app ── plugin (Node.js, @elgato/streamdeck SDK) ── HTTP 127.0.0.1 ── AudioRouter backend
     keys/dials         one shared store + poller                bearer token
```

- **One connection, one store.** The plugin keeps a single client and a single
  state store; actions subscribe to the slice they show. It never polls per
  key.
- **Two poll rates.** `system.diagnostics` at 10 Hz *only while a visible key
  or dial shows live data* (meters, ducking, network); status, sessions and
  `events.subscribe` at 1 Hz otherwise. Nothing is polled when no AudioRouter
  action is on the current Stream Deck page (`willAppear`/`willDisappear`).
- **Truth on the key.** A press sends the command, shows the expected state at
  once, and confirms or rolls back from the next poll (within 200 ms). A
  rejected command shows `showAlert` and the backend's reason in the
  property inspector.
- **Addressing.** Each action stores *session* (follow the selected session, or
  a pinned session ID) and *tool* (node ID plus its name and kind, so a
  renamed or re-created tool is found again by name, as the request builder
  does). A missing tool shows a grey "not found" face, never a guess.
- **Offline.** AudioRouter closed or token wrong: every key shows a dimmed
  face and "offline"; the plugin retries with backoff (1, 2, 5, 10 s).
- **Coalescing.** Dial turns and repeated presses send at most one write per
  50 ms per control, always the latest value.
- **Rendering.** Faces are SVG drawn by the plugin (crisp on every model);
  a face is redrawn only when its quantized content changes (meter to 1 dB
  and 2 px, numbers to their displayed precision), at most 15 times a second
  per key.

### Setup and pairing

1. AudioRouter: API tab → Start API → **Connect a Stream Deck** shows the port
   and a copy button for a setup code (`http://127.0.0.1:PORT` + token).
2. Stream Deck: any AudioRouter action → paste the code once (stored in the
   plugin's global settings) → "Connected to AudioRouter 0.0.x".
3. Later (phase 4): a pairing flow and a reduced *control surface* grant
   instead of the full API token, shared with the hardware panel plan.

## Actions

Every action's settings panel (property inspector) offers live dropdowns
filled from AudioRouter: Session (Selected session / a named session), Tool
(by name, grouped by path), Setting (filtered to what the action can drive).

### Keys

**Control**

1. **Toggle** — the universal key. Pick a tool and Enabled, Bypass, or any
   on/off or two-choice setting (`nodes.toggle`). Two states with the tool's
   icon; off/bypassed faces are clearly different (strike-through, colour).
   Options: *latch* (press toggles) or *momentary* (held = on, release =
   off: push-to-talk on a microphone path, hold-to-bypass for an A/B
   comparison of an EQ or plugin).
2. **Mute** — a Volume, Gain or Mute tool's mute, with that path's live level
   drawn behind the label, so a muted-but-talking path is obvious.
3. **Privacy mute** — the global microphone kill switch. Red and pulsing when
   muted. Muting is instant; *unmuting* needs a 0.5 s hold (a stray press can
   never open the microphone).
4. **Play / Stop** — `sessions.togglePlay` for the selected session; shows
   running, stopped, or "starting".
5. **Session** — shows a session's name; press selects it (option: and
   plays it). A *session cycle* variant steps through all sessions.
6. **Value step** — Volume/Gain/Mixer input up or down by a chosen step, with
   the current value on the key; long-press jumps to a preset value.
7. **Scene** — applies several settings at once ("Ranked": game −6 dB,
   Discord +3, footstep EQ on, music off). Phase 1 sends the writes in
   sequence and reports partial failure; an atomic backend scene is in the
   external integrations plan.

**Show (live visuals)**

8. **Level meter** — vertical peak/RMS bar(s) in the canvas colours (teal
   quiet, gold speech, orange loud, red near clipping), peak hold, clip dot.
   Mono or L/R. Press = mute toggle (optional).
9. **Signal present** — one LED per chosen path: "is my voice reaching
   Discord right now?" Green when the output's level is above −60 dBFS.
10. **Gain reduction** — Compressor/Gate/Duck: how many dB are being removed;
    a Duck key glows while ducking and names the trigger.
11. **Network link** — a Network Send/Receive's health: packets per second,
    losses, "waiting", or "audio from another address" (the Joe case),
    without opening the window.
12. **Status** — backend online, audio running, privacy mute and recording at
    a glance; press opens the AudioRouter window.

**Fun and pro**

13. **Record** — start/stop a recorder; red with elapsed time while
    recording; refuses with a reason if no recording folder is set.
14. **Instant replay (Time Shift)** — shows "LIVE" or "−00:12"; press pauses
    or resumes, long-press returns to live.
15. **Soundboard** — plays an Audio File tool (`audioSources.transport`)
    with a progress ring; a page of them is a soundboard.
16. **Siege round** — the Duck's game-round trigger: menu / preparation /
    action / results, coloured, so you know when ducking will engage.
17. **Test tone** — plays the route's Test Signal to check a path before a
    match.

### Dials (Stream Deck +)

Touch strip feedback uses the built-in layouts (200 × 100 px per dial; `$B1`
is title, icon, value and a bar; `$C1` has two bars) or a plugin layout.

- **Level dial** — rotate = volume/gain/Mixer input (step per tick, faster
  when turned quickly); press = mute; the strip shows the value and the live
  level bar; touch = reset to the saved value.
- **Meter strip** — L/R bars plus gain reduction for one path; press = bypass
  its processing for an A/B check.
- **Duck dial** — rotate = Duck amount; strip shows "ducking −12 dB" live.
- **Session dial** — rotate through sessions, press to select and play.
- **Replay dial** — rotate = jump back/forward 10 s, press = pause/resume,
  long touch = live.

### Pages and profiles

Ship a ready-made "AudioRouter" profile (Play/Stop, Privacy mute, three
meters, three toggles, Record, Session). Stream Deck's own per-application
profile switching can bring it up when the game starts; the plugin does not
need to watch games itself.

## Gaps and proposed backend helpers

Each is optional: phase 1 works without them.

1. **Lightweight meters (`meters.read`).** `system.diagnostics` is about
   12 KB per call (measured on the user's session); at 10 Hz the plugin
   parses 120 KB/s. A read-only method returning only the asked nodes'
   peak/RMS, gain reduction and ducking would cut that tenfold.
2. **Change notifications.** `events.subscribe` is replay-and-poll. A
   long-poll (`waitMs`) or a local event stream would make keys update the
   moment something changes in the window, with fewer requests.
3. **Toggleable-settings list.** `nodes.catalog` describes settings; a
   `toggleable: true` marker (what `nodes.toggle` accepts) would let the
   property inspector list exactly the valid choices.
4. **Control-surface grant and pairing.** A revocable token limited to read,
   graph parameter writes, session control and privacy mute (never device
   administration, recording roots or plugin scans) — shared with the
   hardware panel plan, phase 5.
5. **Atomic scenes.** Several settings in one call, all or nothing.

## Phases

0. **Spike (no product change).** Plugin skeleton in `tools/streamdeck/`
   (TypeScript, Elgato's `@elgato/streamdeck` SDK and CLI): setup code, one
   Toggle and one Level meter against the real app. Measure press-to-audio
   latency, CPU of the plugin plus AudioRouter at 10 Hz, and the key
   update smoothness on the user's Stream Deck model.
1. **Core keys.** Toggle (latch/momentary), Mute with meter, Privacy mute,
   Play/Stop, Session, Level meter, Signal present, Status. Offline and
   not-found faces. Property inspector with live dropdowns.
2. **Dials** (if the user has a Stream Deck +). Level dial, Meter strip, Duck
   dial, Session dial.
3. **Backend helpers** 1–3 above (spec, API, contract and tests), then switch
   the plugin to them.
4. **Pairing** and the control-surface grant (helper 4).
5. **Extras.** Record, Instant replay, Soundboard, Siege round, Network link,
   Test tone, Scenes (atomic once helper 5 exists), Value step.
6. **Distribution.** Package with the Stream Deck CLI as a
   `.streamDeckPlugin` attached to AudioRouter releases; optional Elgato
   Marketplace submission later.

## Requirements (proposed, not yet in the specification)

- SD-01 Connection: loopback HTTP with the API token; offline face and
  backoff; no audio, files or credentials beyond the token leave the plugin.
- SD-02 Truth: every control shows backend state within 200 ms of any change,
  from any client.
- SD-03 Addressing: session follow/pin, tool by ID with name fallback, never
  a silent substitute.
- SD-04 Privacy: privacy mute is instant; unmute needs a deliberate hold.
- SD-05 Load: one shared poller; live polling only while live faces are
  visible; at most 15 face updates per second per key.
- SD-06 Meters: colours and thresholds match the canvas (UI consistency).

## Testing

- Plugin unit tests (Vitest): the store, the coalescer, the addressing and
  name fallback, and the SVG face renderers (snapshot per state and theme).
- Contract tests against the repository's `e2e_backend` (real backend over
  HTTP): every action's requests and the state it expects back.
- On the device: the spike's latency and CPU measurements; then attended use
  in a real match. Elgato's software has no headless emulator; its virtual
  Stream Deck (if available in the user's version) helps for layout checks.

## Decisions needed from the user

1. Which Stream Deck model(s): MK.2 / XL / Mini, Stream Deck +, Neo, Mobile.
2. Default addressing: follow the selected session (recommended) or pin.
3. Privacy unmute hold (recommended 0.5 s) or a plain press.
4. Personal plugin from AudioRouter releases, or also the Elgato Marketplace.
5. Approve phase 0 (spike), and later the backend helpers.

## Risks

- The Stream Deck SDK and app versions move (SDK 3.0 exists; layouts, Neo
  info bar); pin the SDK version and test on the user's app version.
- Image update cost on the Stream Deck app side is undocumented; the spike
  measures it before meters ship.
- The token in Stream Deck's global settings is stored by Elgato's app on
  disk; the reduced control-surface grant limits the damage if it leaks.

## Rollback

The plugin is separate from AudioRouter: uninstalling it, or turning the API
off (or regenerating the token), removes all control. Backend helpers are
additive read/control methods with their own rollback in the active plan.

## Sources

- Elgato Stream Deck SDK: [Keys](https://docs.elgato.com/streamdeck/sdk/guides/keys/),
  [Dials & touch strip](https://docs.elgato.com/streamdeck/sdk/guides/dials/),
  [Layouts](https://docs.elgato.com/streamdeck/sdk/references/layouts/)
  (consulted 2026-10-04).
