# Duck follows the Siege round (DSP-19)

Date: 2026-10-02. Windows 11 x64, MSVC, Node 22, Edge Playwright harness.
User request: replace "Stats.cc + Node script + AudioRouter" with Stats.cc +
AudioRouter only; a Duck lowers the game outside rounds by its own Amount.

## Implementation

- Engine: `RoundPhase`, process-wide `RoundSignal` (atomics only) and
  `siege_round_signal()`. A Duck with `trigger: "siegeRound"` reads the signal
  once per block and engages for ticked phases (`duckMenu`, `duckPrep`,
  `duckBetweenRounds`, default true); `action`, `Unknown` or a phase not
  refreshed for 1 s (48 000 frames) releases. Level mode is unchanged.
- Domain: `trigger` (`level`|`siegeRound`) and the three phase flags validate.
- Control: `siege_round.rs` maps Stats.cc snapshots exactly like the example's
  `phaseState`, connects as a WebSocket client to `127.0.0.1:17892` only while a
  running session has an enabled round Duck (refreshed after mutating
  requests), refreshes the signal every 250 ms, pings every 30 s, drops after
  75 s of silence, backs off 1–15 s, and never logs snapshots.
  `system.diagnostics.gameRound` reports state, phase and whether Stats.cc's
  feed file exists. New dependency `tungstenite` 0.26.2 (`handshake` feature
  only; adds bytes, data-encoding, http, httparse, rand family, utf-8,
  zerocopy to the locks; resolved offline from the local cache).
- UI: Duck Trigger field, phase checkboxes, Stats.cc status line/pill; the
  history shows only the turned-down strip in round mode.
- Not built: the planned consented "Enable Stats.cc feed" button (task 3). The
  status line names the existing `npm run setup:stats` step instead; the user's
  feed is already enabled.

## Checks (Windows)

| Check | Result |
| --- | --- |
| Engine `round_duck_follows_ticked_game_phases_and_releases_when_the_feed_stops` | passed (ticked/unticked phases, 1 s staleness, unknown) |
| Engine/domain/dsp/windows-audio/transport/control libraries | 156/71/59/109/25/227 passed (1/0/1/1/0/9 ignored) |
| `siege_round` unit tests | 3 passed, including a local WebSocket server closing mid-stream |
| Live read-only connect to the user's Stats.cc 1.8.1 | connected, `feedConfigured: true`, phase `action` (user in a round) |
| UI `npm test` / typecheck | 45 files, 447 tests passed |
| `e2e/duck-siege-round.pw.ts` | 3 passed; screenshots [dark](screenshots/2026-10-02-duck-siege-round/duck-siege-round-dark.png), [light](screenshots/2026-10-02-duck-siege-round/duck-siege-round-light.png), [high contrast](screenshots/2026-10-02-duck-siege-round/duck-siege-round-high-contrast.png) |
| Contracts typecheck / drift | passed (121 methods, 34 node kinds) |
| Shell tests | 44 passed, 1 ignored |
| Documentation acceptance | 97 files, 514 links |

Debugging note: the first engine test assumed 480-frame blocks; the processing
quantum is 128 frames, so "100 blocks" was 0.27 s and looked like a stuck
release. Durations are now expressed in seconds.

Review build `target/reviews/duck-siege-round-20261002/` (shell SHA-256
`4410bef7cf3585a642e89db118d0c834b564b3dd9ed38c5d6919a127dc8ce05e`, worker
`57726dd588f686614db90ae1967b78750f5e079e1acf9fadfb40b4153e87827f`),
`AUDIOROUTER_BUILD_ID=review-duck-siege-round-20261002`, release,
custom-protocol, `--offline`. UI bundle 20:33, shell 20:36. Not launched: the
user's spatial review shell (PID 49624) is running.

## Defect: bypassing a Duck between two Mixers (user report, 20:41)

Reproduction: user revision 315 (Game and Mic Mixer → Duck → Discord, Game,
Mic Mixer). Bypass saved; live update failed; automatic restart stopped audio
and prepare failed with `path 1 (starting at "Microphone (PD200X)") is not
supported`. Offline compile of the saved graph: as saved and with the Duck
disabled compile; bypassed was rejected. Cause: the compiler's dry-bypass
allow-lists (source and destination edge checks) listed every chain tool except
Duck. Fix: add Duck to both; regression
`connected_mixers_pass_a_bypassed_duck_dry`. Engine library 157 passed. Saved
graph re-check: as saved / bypassed / disabled all compile. Corrected review
build `target/reviews/duck-siege-round-fix-20261002/`.
Follow-up risk: a restart whose prepare fails leaves the route stopped; a
pre-flight compile before stopping would keep the old route playing.

Open: attended match with the real route (duck engages in menu/prep, releases
in action, releases when Stats.cc closes); the consented feed-enable button.
