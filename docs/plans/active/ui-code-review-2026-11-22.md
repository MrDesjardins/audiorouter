# UI code review, 2026-11-22

## Objective and boundary

Owner request: update from main, then review the complete UI codebase only.
Reviewed main `c6f59c195` on Windows. Fetched origin/main, switched from
`fix/api-mcp-start-and-limits` to main and fast-forwarded main. The original
branch and its two local commits remain intact. Existing untracked
`target-preview/`, `target-preview2/` and `ui/tsconfig.tsbuildinfo` were preserved.
The initial review made no runtime changes. The repair work authorized below
does not change owner audio, saved data or publication state.

Concurrent checkout activity: during review, another process switched back
to `fix/api-mcp-start-and-limits` and merged origin/main, creating
`6d5d3f547`. Left that checkout untouched. `git diff main -- ui` is empty,
so the reviewed UI remains identical to main `c6f59c195`.

Requirements: UI-02/04/05/07/08/09/11/17 and the owner source-layout rule.
Review covers UI playback selection, graph/draft reconciliation, history,
canvas interactions, persistence/recovery, source/recorder/plugin editors,
native RPC adapters, settings, localization and asynchronous event/polling
paths. Automated checks cover the complete UI test/type/lint/format scope.
This is a source review with component/hook probes, not an assertion that
every line or every possible execution path has been exhaustively verified.

## Repair execution (owner authorized, 2026-11-22)

The owner requested all seven repairs. Branch: `fix/ui-review-seven-findings`.
Scope: UI lifecycle/routing fixes and existing API use, regression tests and
required guide refreshes. No new NodeKind or public API is planned.
Prerequisites: Windows, installed UI/native build tools and browsers; offline
fixtures qualify UI behavior without touching owner audio or data. Real MIDI,
Narrator and physical-device evidence remain distinct.

Ordered tasks:
1. Make source classification exhaustive; cover standalone Instrument Play.
2. Bind held notes to their original node/session, serialize release behind
   press, and support Enter/Space, blur, selection and unmount cleanup.
3. Stop Sound Check without an inventory dependency; preserve retry state
   until stop/finalization succeeds and keep the ten-second stop on navigation.
4. Discover all saved file/tone sources in bounded rotating batches; discard
   stale polls and give user transport acknowledgements precedence.
5. Scope plugin inspector state/requests to node and session; protect Soundboard
   polling against stop/switch and newer control requests.
6. Run focused then full UI checks; real-entry browser regressions in all
   themes; update required guides and verify Firefox; build and run isolated
   packaged smoke where available. Inspect the full diff and record outcomes.

Validation matrix: R1 component deadline/read/stop/import failures and retry;
R2 classification and App Play entry point plus engine combination suites;
R3/R7 exact MIDI requests on pointer/keyboard, switch and unmount; R4/R6
external source discovery, >16-source rotation and delayed reply ordering;
R5 plugin selection/session changes and stale load replies. Typecheck after
each implementation slice. Final typecheck/lint/format/unit/build/docs checks.
Rollback: revert only this repair's UI/tests/docs and generated guide assets;
storage and API contracts remain unchanged. No PR opening is authorized.

## Ranked findings (review baseline)

### R1 (P1): Sound Check loses Stop when its deadline fails

Location: `ui/src/AudioFileNodeEditor.tsx:50-82`.
After recording starts, reject `listRecorders` at the 10-second timeout.
`stopRecorder` is never called; the timer has already been cleared, but the
catch sets `takeState` to idle. The UI removes Stop and enables Record again
while the previously started recorder has not been stopped. A failed stop
has the same recovery problem. This defeats the promised ten-second bound
and can continue recording beyond the requested take (UI-05/08).
Keep explicit stop/retry state until stop is acknowledged and enforce the
duration independently of successful inventory/finalization reads.

### R2 (P1): A standalone Instrument chooses the input-device playback path

Location: `ui/src/draft.ts:783-798`, consumed by `ui/src/App.tsx:2178-2184`
and its endpoint preparation fallback.
For a saved stereo Instrument -> stereo Physical Output, both
`needsNativePaths` and `generatedOnlyRoute` return false. The latter's
source allowlist omits `instrument`. With no remembered input, header Play
refuses the route and asks for an input device; with a remembered input it
attempts the microphone-oriented adapter. The multi-path selection that
works for instruments mixed with other sources is never selected for this
basic route. Classify source kinds exhaustively and cover standalone as
well as Mixer layouts (UI-09 and owner source-layout rule).

### R3 (P1): Instrument note release can target the newly selected node

Location: `ui/src/InstrumentNodeEditor.tsx:82-105` and
`ui/src/PropertiesPanel.tsx:634`.
The editor is reused when switching between instruments, and its held-note
set survives. Press C4 on A, switch to B while it is held, then release:
the exact calls are noteOn(A), noteOff(B). Cleanup occurs only at unmount
and uses the latest target too. A's note can remain held until a separate
panic/route stop. Retain the original target with each hold and release it
on node/session change (UI-08).

### R4 (P2): External source playback never reaches the source controls

Location: `ui/src/useAudioFileStatus.ts:32-41`.
Only sources already marked playing in renderer-local state are polled.
After reopening an already-running route or starting a file/tone through
another adapter, its local state is empty/stopped and no status request is
ever sent. The snapshot/event path does not populate this map. The card
can report stopped and disable Stop while the source plays. Poll eligible
sources, including unknown/paused/stopped state, with bounded batching
(UI-04/07/09).

### R5 (P2): Selecting a plugin retains the previous plugin's programs

Location: `ui/src/PluginPanels.tsx:739-773` and its unkeyed inspector in
`ui/src/PropertiesPanel.tsx`.
Load A's programs and select B: A's program names/options remain visible.
Choosing one now sends its index to B. A late load from A can also populate
B's inspector. Preset path and sidechain result state likewise survive the
selection. Reset node/session-specific state and reject superseded loads
(UI-05/08).

### R6 (P2): Source polling publishes replies after its effect is retired

Location: `ui/src/useAudioFileStatus.ts:52-76`; the analogous Soundboard
poll in `ui/src/soundboardLive.tsx:163-186` also lacks an active/generation
check before publishing.
Begin a delayed status request, stop/switch the route, then resolve the old
request: its setter still runs after cleanup. A source response may overwrite
a newer transport action; a Soundboard reply can repopulate a store just
cleared for Stop or another session. In the audio-file hook, a subsequent
stopped render may clear it again, so the Stop case alone is transient.
Use cancellation/generation checks before publishing and ensure newer user
transport acknowledgements win over older status reads (UI-04/07).

### R7 (P2): Instrument keys cannot be played through keyboard activation

Location: `ui/src/InstrumentNodeEditor.tsx:147-164`.
The note buttons have pointer handlers only. Enter/Space activation and the
generated click send no MIDI event, despite the controls being focusable
native buttons. Add keyboard press/release and blur cleanup using the same
held-note lifecycle (UI-11).

## Review baseline validation and evidence

Windows local results:

- `npm --prefix ui run typecheck:all`: passed.
- `npm --prefix ui run lint`: passed, zero warnings.
- `npm --prefix ui run format:check`: passed.
- `npm --prefix ui test`: 1,146 passed, 2 failed, 132 files. Failures were
  the setup-read wait in App.sessionSelection and the verbose-log timer test.
- `npm --prefix ui test -- src/App.sessionSelection.test.tsx src/LogFilesPanel.test.tsx`:
  all 23 passed. The full-suite run remains recorded as failed; this rerun
  does not prove the initial failures have been fixed.
- Seven focused review probes failed their expected-correct-behavior
  assertions, reproducing R1-R7. These are intentionally failing review
  evidence, not product test failures introduced into the checked-in suite.
  Probe retained locally at
  `target/ui-code-review-2026-11-22/uiReviewProbe.test.tsx`.
  To reproduce, copy it into `ui/src/uiReviewProbe.test.tsx`, run
  `npm --prefix ui test -- src/uiReviewProbe.test.tsx`, then move it back
  out of the suite. The relative imports assume the temporary ui/src location.

No browser/theme screenshots, packaged-shell test, real instrument/audio,
Narrator, driver or hardware validation was run. Backend code was outside
the review scope; backend runtime limits were not independently qualified.
The probes use component rerenders, fake timers and mocked backend methods.

## Repair results and verification

All seven findings have source fixes and passing regressions:

- R1: Sound Check calls the existing `recorders.stopRecording` action at
  its deadline without reading inventory first. The backend chooses its
  current frame. The original recorder identity and stop idempotency key stay
  attached to the take; rejected stops retry every second, including after
  leaving Properties. Stop/import failures retain a retry action and prevent
  a replacement take. Successful stop is not repeated on an import retry.
- R2: source classification is exhaustive. Instrument, loopback, virtual and
  network sources select native paths independently of a physical input hint.
  The App header Play regression proves standalone Instrument preparation
  never requests the remembered microphone.
- R3/R7: note requests retain their original target; cleanup runs on node,
  session, availability, window blur and unmount changes. Note release waits
  behind a pending press. Enter/Space, focus loss and assistive activation use
  the same lifecycle. English/French/Chinese instructions describe the keys.
- R4/R6: source status polls rotate through every file/tone in batches of at
  most sixteen, including unknown/stopped/paused sources. Individual failures
  do not hide other answers. Retired effects cannot publish. Local transport
  generations/pending calls prevent older polls overwriting user actions;
  Soundboard polls also discard replies predating a control call.
- R5: plugin inspector instances are keyed by session/node/binary identity,
  isolating programs, preset paths, sidechain feedback and late async loads.

Windows 11 build 26300, Node 22.22.3 verification:

- `npm --prefix ui test`: final **1,165/1,165**, 133 files. The first repair
  run reproduced the verbose-log timer timeout (1,163 passed/1 failed).
  Its test now jumps the clock and checks actual timer ticks inside `act`,
  preserving countdown/expiration assertions without simulating 3,600 renders.
  Both subsequent full runs passed (1,164 before the last added regression;
  1,165 after it). No timeout increase or warning suppression.
- `uiReviewRegressions.test.tsx`: 16 passing cases, including a failed deadline
  stop retried after unmount. `App.lifecycle.test.tsx` adds the exact header
  Play entry regression. Focused combined suites passed 40 cases before the
  additional automatic-retry case.
- `npm --prefix ui run typecheck:all`, `lint`, `format:check`, `i18n:check`:
  passed. `npm --prefix ui run format` touched only edited files.
- `npm --prefix contracts run check:drift`: passed, 156 methods/38 kinds.
  No public API, MCP tool or node parameter schema changed.
- `cargo test -p audiorouter-engine --test source_combinations --test tool_combinations`:
  12/12 passed, covering standalone/fan-out, Mixer/connected-Mixer sources
  and processor flag/layout combinations.
- From ui: `npx playwright test --config=playwright.config.ts
  ui-review-repairs.pw.ts instrument-editor.pw.ts --workers=1`: 13/13 passed.
  After updating instruction copy, reran `ui-review-repairs.pw.ts`: 9/9 passed.
  Inspected dark/light/high-contrast captures under
  `target/ui-code-review-2026-11-22/{instrument,audio-file,plugin}-*.png`;
  fields, note focus and controls are readable/contained. Simulated backend,
  exact MIDI target requests and real rendered card Stop were exercised.

### Guide refresh

Refreshed the five affected silent portrait Properties overviews: Instrument,
Audio File, Test Signal, Soundboard and Plugin. Instrument now demonstrates
Enter, pointer and Space. Existing production-engine audible demonstrations
are retained unchanged. Updated the property coverage manifest, recording
descriptions and generated guide pages, then restored social/CSP generation.

The recorder's data-URL import of `parameterText.ts` failed after localization
introduced a relative i18n import. It now resolves labels in the actual Vite
page, including the language module. A Test Signal `levelDb` property capture
exercised this branch successfully without republishing that audible clip.
One Instrument capture was interrupted by catalog regeneration's page reload;
its replacement capture completed with no page errors. Never edit UI/catalogs
while recording. Python's `python` alias was unavailable, so static validation
used installed `py -m http.server`; a root Vite server timed out on the preview
page and was stopped before the successful static-server run.

Commands/results:

- `GUIDE_KIND=instrument,audioFile,testSignal,soundboard,plugin node
  tools/site/record-tool-guide-overviews.mjs`: five captures completed
  (Instrument's final rerun selected only Instrument).
- Matching `build-tool-guide-review.mjs`, then `site/generate-guides.mjs`,
  `site/generate-social.mjs`, `site/generate-csp.mjs`: completed.
- `tools/site/check-tool-guide-video-firefox.mjs` with the five kinds and
  private debug port 9347: 18/18 playback checks passed, including retained
  audible clips and shared-controls entries. Evidence:
  `target/tool-guide-firefox-results.json`.
- `check-tool-guide-videos.mjs`: 204/204 videos meet codec/duration/size rules;
  `check-tool-guide-property-coverage.mjs`: all 258 properties covered.
  Guide guards/family unit checks: 19/19. Social/CSP checks passed.
- Docs acceptance: 289 Markdown files/1,619 links passed; diff whitespace
  check passed and complete code/test/generated-page diff inspected.

### Built executable and limits

`tools/build-desktop.ps1` rebuilt CLI and x64/x86 VST2/VST3 workers and passed
the UI production build. Cargo compiled the shell dependency executable but
could not replace `src-tauri/target/release/audiorouter-shell.exe`: OS error 5
because the owner's running app (PID 6736) holds it. Did not close that app.
The main executable at that path remains the older build.

Copied the freshly linked `src-tauri/target/release/deps/audiorouter_shell.exe`
and matching companions to `target/ui-code-review-2026-11-22/desktop/`.
Shell SHA-256:
`255754EB77A8F0CD0484EE3F3A0A141B7FE9B8D56F0A54DA637F44FE9DE1B3CA`.
Shell timestamp 2026-11-23 03:49:56 UTC is newer than UI bundle
03:48:27 UTC. Build retains the main/language chunk size advisories; this
repair does not change chunking policy. Edge reports its attribution notice
and the runner's NO_COLOR/FORCE_COLOR notice; no uncaught UI errors.

Standard packaged smoke refused before launching because the owner's app owns
the default pipe: `target/ui-code-review-2026-11-22/packaged-smoke/result.json`.
An independent probe ran the fresh executable at Medium/non-admin with a
private pipe, database and WebView profile. Native discovery (156 methods),
canvas readiness, Add Gain and a permission-denied Save retaining the draft
passed with zero page errors. A separate-process unenveloped shell correctly
gets a read-only grant: initial attempts incorrectly expected a successful
save; those harness timeouts are not application regressions. The final probe
checks refusal instead. Quit is also denied by that backend; only probe-owned
process trees were terminated. Evidence:
`target/ui-code-review-2026-11-22/private-smoke-1853e292-ee0d-4c2a-a1ca-a93bd984a6d3/result.json`.
This does **not** qualify owned-backend save/restart persistence or native audio.

## Next task, risks and rollback

Implementation of R1-R7 is repaired and locally verified within the scopes
above. Leave this plan active for the remaining owned-backend packaged
save/restart check after the owner closes the running app. Live instrument,
physical audio and Narrator were not run and remain separate attended gates.
If the backend/renderer is unavailable, a UI timer cannot guarantee a hard
audio deadline; this repair removes the inventory dependency and retries stop,
but does not add a new backend recording-duration contract.

Branch `fix/ui-review-seven-findings` was based on pre-existing `c402086eb`
(the prior API/CLI work), which is preserved. The owner authorized opening
the UI PR. Before pushing, rebased only the UI repair commit onto current
main `1266f03d1`, excluding the unrelated API commits; the UI/tool-guide source
is unchanged by that rebase. Original repair retained locally on
`backup/ui-review-before-pr`. No site publication.
Rollback: revert this repair's UI/tests/docs and five refreshed overviews;
there is no storage migration or API change. Do not replace the running
owner's executable until it can be closed deliberately.
