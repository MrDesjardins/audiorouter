# Feature confidence qualification — 2026-09-26

## Scope and environment

User explicitly requested broader end-to-end tests and repairs for implemented
features. Windows, Edge Chromium, Rust debug tests, Node 22 and installed local
dependencies. Disposable SQLite databases and generated audio only. RTK was
unavailable. No user session database, device defaults or private audio changed.

## Coverage

- **All 25 creatable library tools:** `ui/e2e/tool-workflows.pw.ts` derives cases
  from the actual registry. Add, remain in Tools, select into Properties, edit
  advertised numeric/boolean/enum controls, rename, Off/Bypass, plan/commit,
  inspect stored parameters, restart backend and reload saved node state.
  Physical endpoint cases prove configuration persistence, not native sound.
- **Session and graph lifecycle (UI/STATE/GRAPH/API):**
  `session-workflows.pw.ts` exercises initial loading, new/duplicate/rename,
  remembered selection, delete, cancellation, undo/redo/revert, export/import,
  malformed input, stale revisions, idempotent commit replay and durable history.
  `media-and-routing.pw.ts` verifies occupied-output Mixer insertion, undo,
  single sink connection, two saved input paths and restart persistence.
- **Live controls (UI-04/05, GRAPH-05/06/08/14):**
  `live-controls.pw.ts` asserts eight live flag changes produce no Stop calls,
  failed commits restore controls, restart-required results remain visible,
  and unsaved topology cannot be silently committed by a flag click. These
  cases use the route fixture; hardware hot publication remains attended.
- **Media and EQ (DSP/STATE):** real backend malformed WAV rejection, generated
  WAV import into Audio File/FIR, durable media identities; Advanced EQ point
  parameters and calculated frequency response. Original 21 canvas scenarios
  also cover insertion, layout, routing and audition controls.
- **Recording (REC-02/03/04/06/12):** real UI creates/arms/starts/pauses/resumes/
  finalizes a float WAV, verifies actual bytes and metadata after restart.
  Synthetic samples enter the real recorder tap; native capture is not opened.
  Rust regression repeats undrained pause/resume in all six offered formats;
  paused taps cannot enqueue audio. A failed legacy encoder publishes failed
  state without stopping the session. Queue admission retirement is tested.
- **Signal processing (DSP-01–18):** `crates/engine/tests/tool_routes.rs` runs
  all 17 processor kinds through prepared stereo routes for 96 quanta, checks
  finite/nonzero output, silence from Off endpoints, dry Off/Bypass effects,
  exact gain/volume scaling and channel isolation. Pitch regression exercises
  512 quanta at -12/0/+7/+12 with silence-to-tone transitions. Existing DSP
  frequency, duration, filter, dynamics, restoration and time-shift tests run.
- **Plugins (PLUG):** contained native VST2 chains verify arbitrary adjacent
  members, masks, state, parameters, editor lifetime and fail-closed crashes,
  hangs and nonfinite results. See [chain evidence](2026-09-26-shared-vst2-chain.md).
  Protocol tests reject malformed masks and preserve old omitted-mask messages.
- **Themes/accessibility (UI):** three themes at 1280×720 verify integer and
  enum controls, accessible captions, common 9 px flat fields and containment.
  Six screenshots in `target/feature-confidence-visual` were visually inspected.
  Existing accessibility/lifecycle unit suites also run.

## Test boundary

`crates/control/examples/e2e_backend.rs` is a test-only bounded JSONL adapter
around the real ControlPlane and Storage. `real-backend.ts` creates an exclusive
temporary directory, intercepts browser fixture RPCs, seeds a graph and restarts
that backend against the same database. It opens no production listener or
native endpoint. Authorization/transport production behavior is covered by
existing Rust tests, not proven by this adapter's direct dispatch. Recording
sample injection exists only in the example. The browser harness is excluded
from the production Vite build. Browser page errors fail fixtures. No retries
or new hardware skips hide failures.

## Repairs found by these tests

1. Microphone + Test Signal compiled stages lacked corresponding meter storage;
   initialize meters with the compiled stages and regress the mixed route.
2. Integer step discovery was omitted from the UI contract; honor it in fields
   and validation, and give enums accessible labels.
3. Pitch's approximate phase calculation returned NaN for zero FFT bins,
   poisoning retained state and eventually silencing output. Adapt the MIT
   implementation locally using standard finite phase math; preserve license
   attribution. Sustained/silence-transition tests now pass.
4. Connected startup exposed an editable demo before receiving saved state;
   gate graph editing on the first snapshot and provide loading/reconnect status.
5. Inspector/EQ fields overrode shared styling and Gain text became dark on its
   dark card in light theme; use shared fields and readable card text.
6. Resume advanced the encoder frame before draining pre-Pause queued audio;
   retire admitted taps and drain before Pause, reopen only after Resume succeeds.
   Encoder errors now expose failed state instead of stale recording status.

Test infrastructure failures were repaired separately: automatic backend fixture
activation, exact tool labels, graph-only edge selectors (excluding the legend),
checkbox click assertions that allow rejected changes, restart serialization
and evidence-preserving page teardown. These are not product fixes.

## Reproducible outcomes

- `npm.cmd --prefix ui test`: **345 passed**, 26 files; log
  `target/e2e-ui-final.log`.
- In `ui`, `npm.cmd run e2e`: **65 passed**; then
  `npm.cmd run e2e -- --repeat-each=2`: **130 passed**, no retries;
  logs `target/e2e-browser-final.log`, `target/e2e-browser-repeat.log`.
- `cargo test --workspace --lib --bins --tests --features
  audiorouter-plugin-host/test-fixtures --locked`: **865 passed**,
  **19 pre-existing ignored** native/hardware qualification cases; zero failures.
  Log `target/e2e-workspace-final.log`.
- `cargo test --workspace --doc --locked`: passed, including public streaming
  pitch example; log `target/e2e-doc-tests.log`.
- `npm.cmd --prefix contracts run typecheck` passed; `check:drift` passed:
  98 methods, 30 node kinds, 16 processors, 21 event categories.
- `tests/acceptance/m06-vst2-chain.ps1`: two contained native tests passed;
  `target/plugin-chain-qualification/qualification.log`.
- UI production build passed; current bundle `index-DuzfCEY2.js`.
  Documentation and final artifact checks are recorded in the active plan.

Final artifact probing caught an additional packaging defect: plain
`cargo build --release` lacked `custom-protocol`, selected the development URL
and contained no current UI asset names. Direct Cargo shell builds must include
`--features custom-protocol`. The final rebuild uses that feature; verify its
current bundle path in the binary and timestamp after the production UI build.

## Remaining evidence and rollback

Microphone crackling is unresolved. Synthetic tests cannot prove audible voice
quality, native deadlines or endpoint clock stability. Tonight's attended task
is the release-2 microphone route, live Off/Bypass, dry comparison and continuity
counters after startup. Calibrated latency, long hardware soak, sleep/rebind,
third-party application routing, vendor compatibility, Narrator/manual usability,
driver signing/installer/clean-machine gates remain separate qualification.
Unavailable virtual-driver tools remain explicitly unavailable.

Rollback fixes by owning layer; revert pitch source/dependencies/lockfiles as a
unit, and recorder admission/lifecycle changes together. Test fixture removal
does not change production data formats. Existing sessions stay compatible.
No M08 or hardware milestone completion is claimed.
