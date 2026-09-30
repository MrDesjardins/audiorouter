# Live bypass and Discord mic-test feedback — 2026-09-29

Requirements: GRAPH-05/06/08/11/15, CAP-06/10, UI-04/07/08.
Affected build: `target/sender-routing-20260929/release` (`bd4380ff`).

## Reproduction and cause

Read-only SQLite inspection found revision 124 with the independent processed
microphone → CABLE-A plus Scarlett monitor, Siege CABLE-B Output → Siege EQ →
Scarlett, and Discord capture + Siege EQ → Mixer → output named Outplayed.
Windows endpoint property inventory identifies Outplayed's exact render ID as
CABLE-B Input, with `CABLE-B Input` driver description and
`VBCableInst.NTamd64` INF section. Siege's capture has the corresponding
`CABLE-B Output` driver description. Thus the Mixer feeds back into its own
Siege input. Discord's mic-test playback injects the user's voice into this
return; application loopback captures all process playback, not remote speech
alone. No Discord graph edge feeds CABLE-A. No user graph, default, app setting
or endpoint binding was changed by this investigation.

Shell log inspection confirms revisions 122–124 and 13 nodes/11 edges but does
not preserve the full native activation reason/flag history. The quoted
revision-123 failure is user evidence. Native regressions establish two gaps:
the compiler required two remaining Mixer inputs after disabled-source pruning;
prepared source/sink/Mixer bypass was not normalized to its silent transport
shape. Pending draft edits also blocked UI live flags outright.

## Changes

- Live UI submits only the selected saved node's flag and preserves pending
  names/parameters/wiring, including edits made while the request is pending.
- A Mixer may retain one active input, with its gain and pre-Mixer branches.
- Prepared source/sink/Mixer flags preserve stream identities and silence
  matrices; ordinary processor bypass stays dry. Sources excluded during
  preparation are not opened by unrelated toggles.
- Plan/native-path preflight recognizes supported VB-Cable returns from
  driver properties and exact endpoint IDs. It rejects a reachable return
  before audio opens with named source/output and separate-cable guidance.
  Same-endpoint loopback is also rejected. Unknown drivers, external app or
  acoustic paths and cross-session returns remain outside this bounded check.
- Quickstart explains separate voice/game/recording cables and Discord's
  mic-test playback scope. No persistence schema change.
- Shell/backend commit logs now record the revision, runtime generation and
  allowlisted native activation state, so a durable save with restartRequired
  can be distinguished from a live-applied change. Reason strings stay omitted
  to avoid recording private node names or paths.

## Windows verification

- `cargo test -p audiorouter-engine -p audiorouter-control -p
  audiorouter-windows-audio -p audiorouter-transport --lib --locked
  --target-dir target/sender-routing-20260929 --quiet`: engine 145, control
  200, Windows adapter 100 and transport 23 passed; seven opt-in control tests
  ignored. Existing unrelated Windows test unused-mut warning remains.
- `cargo test -p audiorouter-control --lib --locked --target-dir
  target/sender-routing-20260929 installed_cable_feedback_is_rejected_before_opening_audio
  -- --ignored --nocapture`: passed against installed Cable B metadata; no
  capture/render client opened. This verifies the actual property classification.
- UI Vitest: 380 passed in 31 files; typecheck passed. Regression checks live
  bypass both ways, no Stop/restart, flag-only submission and unsaved name.
  Final focused lifecycle rerun: 11 passed, followed by typecheck. Packaged
  shell `cargo test --manifest-path src-tauri/Cargo.toml --release --features
  custom-protocol --locked --target-dir target/live-bypass-20260929 shell_rpc
  -- --nocapture`: two logging regressions passed.
- Edge Playwright `live-bypass-draft.pw.ts` and `undo-live-parameters.pw.ts`:
  four passed. Final three-theme run used `--headed`: three passed; screenshots
  inspected [dark](2026-09-29-live-bypass-dark.png),
  [light](2026-09-29-live-bypass-light.png),
  [high contrast](2026-09-29-live-bypass-high-contrast.png).
- Native continuity: set `AUDIOROUTER_LIVE_CONTINUITY=1`,
  `AUDIOROUTER_CONTINUITY_PRE_MIXER=1`,
  `AUDIOROUTER_CONTINUITY_DISABLED_MIX_INPUT=1`,
  `AUDIOROUTER_CONTINUITY_CHAIN=parametricEq`,
  `AUDIOROUTER_CONTINUITY_TONE_HZ=47`,
  `AUDIOROUTER_CONTINUITY_SECONDS=15`,
  `AUDIOROUTER_CONTINUITY_TOGGLE=tool-0`; run
  `cargo test -p audiorouter-transport --test live_audio_continuity --locked
  --target-dir target/sender-routing-20260929
  live_backend_service_keeps_a_routed_tone_continuous -- --ignored --nocapture`.
  Passed: one prepared input, four bypass/unbypass commits applied while
  running, clean 15 s reference/mixed output/direct Recorder branch, zero
  glitches or silent runs or discontinuity flags; lateGaps=0, maxGap=6537 µs.
  One output startup underrun counter did not create a measured steady gap.
  No microphone, Scarlett or user database opened; no shell launched.

## Corrected experiments and limits

Initial sandbox audio capture returned access denied; reran the same authorized
virtual-only test with normal process permissions. A first native toggle run
with browser workload had two mixed-output discontinuity edges around a 1040
sample silent gap at 4.91 s (between toggles); reference and direct recording
were clean. It failed and is not passing evidence. The later quiet run with the
reported disabled-input shape passed. Earlier browser screenshot writes were
denied by filesystem permissions; reran with permitted writes. Theme selectors
were explicitly verified after correcting the preference key. Test fixture
schema/type mistakes were corrected before interpreting runtime results.

Actual Discord Mic Test and the user's Scarlett/Outplayed selection need
attended confirmation after changing the return destination. The new build
deliberately refuses the saved CABLE-B loop; it does not silently choose another
recording cable or alter microphone processing. Roll back with the prior
sender-routing shell/worker pair; that also restores the old feedback gap.

## Artifact

Complete isolated build in `target/live-bypass-20260929/release`:
`audiorouter-shell.exe` and adjacent `audiorouter-plugin-worker.exe`.
UI build, locked release shell with `custom-protocol`, and worker build passed.
Temporary frontend configuration was restored. `index-BtzZjOrD.js` and
`index-C83LaAIt.css` were verified inside the shell. UTC timestamps on
2026-09-30: UI index 03:11:25, shell 03:17:46, worker 03:14:56.

SHA-256:

- Shell: `2569A2B19FD26CA9B1B0A6EFA3B75F1BB5CCFBF0F42DB21F6C59FD339949D39F`
- Worker: `107384353A5209F02E1F25BC5800F629480A5F1924325249D248B478F5B01CAB`

Docs acceptance: 72 Markdown files, 388 local links; `git diff --check` passed.
The owned Vite server was stopped and temporary test/inspection files removed.
Next: user launches the new shell, deliberately changes Outplayed to a separate
recording cable, and confirms Discord Mic Test plus live EQ bypass on their
actual route. Keep the worker beside the shell; close any older shell first.
