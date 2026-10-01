# Active plan — VB-Cable-first completion and delivery planning

Status: active. Rewritten 2026-09-27 as a current-state plan; current task
updated 2026-09-28 for the requested installer and manual draft-release path. The
append-only log it replaces (2026-09-17 → 2026-09-26: every decision,
defect, experiment and superseded handoff) is archived verbatim as the
[execution log](../archived/2026-09-26-vb-cable-first-execution-log.md).
Add new entries under "Log" below, and keep the sections above it current.

## Objective and scope

## Current task: manual desktop rebuild (2026-09-30)

User requests a local rebuild for further manual testing of the Dehum fix and
Spectral Gate rename. Requirements: DSP-12, UI-04, ENG-05 artifact provenance
(development build only). Build current UI, then locked optimized shell with
`custom-protocol` and matching plugin worker; verify embedded UI freshness,
executable timestamp and checksum. Preserve the running desktop and database;
do not launch another shell or publish a release. Rollback: run the prior build.
Next action: build and hand off the full executable path.

Build result: UI production build and locked release shell (`custom-protocol`)
pass. Shell path:
`C:\code\audiorouter\target\groups-http-20260929\release\audiorouter-shell.exe`.
Current UI `index-CsSSg0c2.js` contains "Spectral Gate"; shell timestamp
2026-10-01 00:16:19 UTC is newer than the rebuilt UI. Shell size 22,533,632 bytes,
SHA256 `1F4C300842EA3F51BB2C7779A63B4678D2883EFE07A0DB03304FA8243355E11D`.
Matching default-feature plugin worker beside it: 968,192 bytes, SHA256
`487E3EA53B6826429EE43CE4F3EBE98E90D4890D2E6A4EA7DF7E9925B07B0E88`.
Locked companion CLI build also passes: sibling `audiorouter-cli.exe`,
12,715,008 bytes, SHA256
`2993256426474FEA31281472F4A1AE2B82B33B9C165B94CDF8C972736188908A`.
All three pass the repository x64 PE validator. Source baseline: `8a2a01e0`.
Docs acceptance: 79 files/421 links; diff check passes.
No shell was running during the initial process check; none was launched here.
Manual testing is next; these build checks do not qualify native audio or M08.

## Current task: clearer tool naming and deterministic two-tool composition

User authorizes renaming "FIR Filter Hz" and adding connected-pair signal tests.
Decision: display "Spectral Gate" for existing `spectralGate`; retain `firFilter`
as "FIR Filter", identifiers, parameters and user-defined saved names.
Requirements: DSP-01/02/03/04/05/06/08/11/12/13/15/16/17, GRAPH-02/10/14,
QUAL-01/02/03 and UI-04. Prerequisites: existing offline Windows toolchain and
qualified single-tool harness. Native endpoints and external plugins stay
separate. Ordered work: update display names/contracts/docs; build ordered
two-tool graphs; compare with independently executed single-stage composition
across mono/stereo, rates and bypass; add analytical transfer/order/latency
checks; run signal and UI checks, inspect label rendering, record evidence.
No saved-name migration or audio-device mutation. Rollback: revert display
labels and added tests/docs; preserve stable node identifiers. Next action:
implement naming and pair fixtures, then diagnose any deterministic mismatch.

Result: completed. "Spectral Gate" labels/default names replace "FIR Filter Hz";
saved node names and identifiers are retained. 2,749 new deterministic pair
cases pass: 2,312 active combinations, 408 bypass cases and 29 analytical
checks. Full suite: 33 signal tests, 197 DSP/engine units, two native plugin
tests; focused UI: 60 tests; three headless Edge theme checks pass and their
screenshots were inspected. UI build succeeds in disposable output; no desktop
exe rebuilt. See [pair evidence](evidence/2026-09-30-two-tool-composition.md).
Next task: guarded native continuity/performance and listening qualification;
offline pair success does not replace hardware or arbitrary-plugin evidence.

## Current task (2026-09-30): deterministic audio transformation qualification

Current fix, explicitly authorized by user: close failing DSP-12 preservation
cases, without lowering the 5% criterion. Reproduction: eight 60 Hz/full Amount
peaking cuts retain only 92.31% of a 1003 Hz tone. Decision: replace the
gain-dependent-width peaking cascade with finite-depth constant-bandwidth
notches. Fundamental notch Q is 20, harmonic h uses Q=20*h; Amount retains
0..−36 dB center depth. Implement in DSP, wire the graph compiler, update the
transfer-law specification and independent oracle together, unignore the
regression and remove runner's ignored-only invocation. Sweep hum centers,
inter-harmonic midpoints and passband boundaries, rates/Amount/harmonics,
mono/stereo and bypass/reset. No added latency, callback allocation, device,
desktop or saved-session mutation. Validation: failing test first, focused
Dehum sweeps, full qualification/native fixtures, unit/docs/diff checks.
Compatibility: existing Dehum settings remove less wanted audio; no migration.
Rollback: revert DSP/compiler/contract together and re-open the preservation
gate; retain regressions. Native continuity/performance remains separate.

Fix result: the unchanged wanted-tone regression retains 0.9999558431 amplitude
(previously 0.9231109735). Full runner passes 30 signal tests with no ignored
cases, 197 DSP/engine unit regressions and two actual native plugin tests.
The added passband sweep covers 132 configurations, plus six mixed stereo
program/hum cases. Details and reproducible commands:
[Dehum evidence](evidence/2026-09-30-dehum-preservation.md).
Review caught and corrected a generic EQ Q-limit conflict during constructor
cleanup; the final full runner passes again. Docs: 78 files/419 links; test-file
format and diff checks pass.
Next task: guarded native Dehum continuity/callback performance and listening
qualification, preserving the user's running desktop and endpoint routes.
These are unrun; offline success is not a release/hardware qualification.

Prior continuation (superseded Dehum status below): expanded offline DSP-02/03/05/06/14/17 and
QUAL-02/03/05 evidence.
Ordered tasks: exercise public running Input Switch graph replacement for normal
and slow fades, both directions and reversal; test filter frequency/Q and graph
rate endpoints; verify 60-second pitch duration and silence recovery; compare
Delay's live 64-frame tap transition with an independent waveform. Preserve
Dehum's failing gate and normative algorithm. Run focused then full signal/unit
checks, docs validation and diff checks, record outcomes, commit/push. Artifacts
remain generated under target; no device/desktop changes. Rollback only these
test/documentation additions unless a reproduced defect requires a scoped fix.

Result: 27 normal tests pass with 925 unique retained signal cases. New checks
cover public running Input Switch replacement/reversal, 8/192 kHz EQ endpoints,
60-second pitch and silence recovery, live Delay crossfades, linked stereo Gate
attack/hold/release, and imported WAV IR decode/resampling/convolution. No new
DSP defects were found or implementation changes needed. The full runner again
passes 197 DSP/engine regressions and both native plugin signal tests; it returns
failure solely for the explicitly executed Dehum preservation gate. Docs check:
77 Markdown files, 414 links pass before this documentation update. See updated
dated evidence for final validation. Exact next task: decide the DSP-12 algorithm
and preservation frequency domain, then qualify that gate; physical/callback
performance and real speech listening remain separate unrun work.

User authorizes planning and immediate unattended execution for every tool that
changes audio, using generated/reused input, configurations, output capture and
input/output comparison. Requirements: DSP-01–18, QUAL-01–05, GRAPH-02/04/10/14,
PLUG-03/04/07 where installed fixtures permit. No personal audio is needed.
Prerequisites: Windows Rust toolchain and existing pinned dependencies available;
hardware, physical latency, subjective speech intelligibility and third-party
plugin compatibility remain separate evidence. Preserve the running desktop,
user database, microphone and endpoint configuration.

Ordered tasks:
1. Inventory every modifying tool and existing meaningful signal checks.
2. Add shared deterministic fixtures (fixed-seed noise, DC, impulses, tones,
   stereo markers) and optional float-WAV/JSON artifacts to graph-level tests.
3. Sweep one tool at a time through neutral, representative, boundary and
   bypass configurations. Compare with independent gain/transfer/delay/
   convolution/pitch/noise expectations, not merely “output changed”.
4. Cover multi-input Mixer/Input Switch, timing transport, channel isolation,
   learned profiles and real available plugin fixtures; label any gaps honestly.
5. Run relevant DSP/engine tests, inspect failures, fix proven defects in the
   owning layer, rerun affected checks, document exact commands and results.
6. Commit/push the qualified work and preserve a resumable backlog if needed.

Validation matrix: exact sample comparisons for gain/mute/routing/delay;
frequency projection and ±0.5 dB for filters/dynamics; ≥30 dB notch rejection;
≤0.1 dB limiter ceiling; latency-aligned FIR direct convolution; settled noise
attenuation plus retained wanted tone; pitch within 10 cents after warmup;
repeatability/reset checks. Artifact output belongs in ignored
`target/deterministic-audio-evidence`, never source-controlled audio. Windows
offline execution is signal/graph evidence, not WASAPI/hardware qualification.
Rollback: revert only the new harness and separately documented proven fixes;
no storage migration or device changes. Execution and remaining gates are recorded
in [dated evidence](evidence/2026-09-30-deterministic-audio.md) and the
[repeatable qualification guide](../../operations/deterministic-audio-tests.md).

Final sweeps: 27 normal graph signal tests pass (925 unique retained cases),
197 DSP/engine unit tests pass, and actual native VST2/VST3 gain fixtures pass
three-rate transfer/automation/bypass checks. Four proven fixes: exact limiter
lookahead, complete Speech Denoise reset, double-precision biquad coefficients
and recursive state to avoid near-DC cancellation, and Gate hold countdown
which previously restarted forever below threshold. Both mono and linked
stereo Gate paths are corrected. Gate timing and compressor attack/release
now have independent envelope oracles.

The unattended qualification runner executes the known Dehum gate explicitly
and returns failure while that gate is open; native plugin runs are opt-in.
No endpoint, microphone, user session or desktop changes were made. Generated
audio is ignored. Compatibility: saved gate settings now actually close after
hold; limiter delay is one frame shorter and matches telemetry; biquad numeric
output improves precision; reset no longer preserves Speech Denoise estimates.
Risk: callback CPU/continuity and subjective speech quality are not qualified
by these offline runs. No runtime allocation was added by the fixes.

Exact next task: resolve DSP-12 wanted-band preservation versus the normative
Q20/cut cascade without relaxing acceptance; then extend remaining parameter
endpoints and hardware/performance qualification. Representative configuration
coverage does not mean every
possible signal, configuration or combination of tools is qualified.

Second execution found a second real defect: Speech Denoise reset cleared the
STFT and frame counter but retained smoothed/noise estimates. Clear both so a
reset reproduces a fresh processor. The independent Dehum response passes its
Q20 cascade law, but eight 60 Hz harmonics at 100% attenuate a 1003 Hz wanted tone
by about 0.69 dB (~7.7%), violating DSP-12's 5% preservation requirement. Keep
this as an explicitly ignored known-failure acceptance test, run it separately
and report the failing result. Do not weaken the 5% gate or change the normative
Q20/−36 dB transfer law without a specification decision. Other signal tests
(pitch, learned profiles, FIR, click repair) passed the second run.

First execution: 7/9 graph-level signal tests passed. Limiter impulse output
appears one sample after its declared lookahead because its ring has N+1 slots
and reads before writing. Fix the ring to N slots (one inert slot for zero
lookahead); retain exact impulse timing/ceiling checks across three rates.
Dehum's first oracle incorrectly assumed each harmonic cut was isolated; replace
it with an independent f64 cascade response and retain wanted-band preservation.
Evidence: `target/deterministic-audio-first-run.log` and per-case WAV/metrics.

Current task (2026-09-29): clear cable-feedback warnings while allowing device
selection/save; visual named canvas groups; document then implement optional
localhost HTTP and bundled Swagger. User confirmed localhost-only. Requirements:
GRAPH-11, UI-03/16, API-01–12, SEC-01–04/10/12, HTTP-01–06.
Decisions: known closed-loop playback remains blocked; selection/save carry a
warning. Groups are local presentation state, 5% default opacity, no ports.
HTTP is disabled at launch, activation-scoped token and existing desktop grant.
Prerequisites: Windows/browser toolchains available; no driver or credentials.
Ordered work: update contracts; expose feedback warning; add groups and focused
interaction/persistence tests; implement shared-backend HTTP/OpenAPI/Swagger and
API tab; verify negative cases and frontend refresh, three themes; build a fresh
shell/worker pair. Validation: focused Rust/UI tests, real loopback HTTP, headed
Edge, docs/diff checks. Audio code is unchanged; native continuity required only
if that scope changes. Risks: annotation hit testing, HTTP token/origin/bounds,
stale frontend draft. Rollback each focused slice; disable HTTP immediately if
needed. Preserve user database, running shell and unrelated changes.
Result: documented contracts implemented; feedback requires explicit warning
acknowledgment to save and remains blocked at playback; groups persist locally;
HTTP forwards all 102 methods to the existing backend with offline Swagger.
Windows Rust/UI regressions and seven headed Edge tests passed, including
Swagger authorization/Try it out and clean/dirty frontend refresh. Three themes
reviewed. Native 15 s, 47 Hz pre-Mixer continuity with four live bypass changes:
zero reference, mixed-output and Recorder glitches. Complete shell/worker pair:
`target/groups-http-20260929/release`; embedded assets and checksums verified.
Final control/domain rerun passed; contract drift, 75-file/408-link documentation
acceptance and authored-source diff checks passed. Four upstream trailing spaces
in the bundled Swagger license comments are retained verbatim (see evidence).
See [task evidence](evidence/2026-09-29-groups-http.md) for commands, limitations,
experiments and artifact hashes. Next: attended testing of the new executable,
API Start/Stop and the user's corrected cable routing. M08 gates remain open;
this is a local test build, not an official release.

## Completed follow-up (2026-09-29, UI-16)

User asks to drag a group by its background as well as its caption and sets the
new-group opacity default to 5%. Updated the spec and implementation: the group
surface accepts pointer input and participates in React Flow dragging, while
remaining behind nodes and audio edges; removed the caption-only drag constraint.
Updated the existing component/browser opacity expectations to 5%. Build:
`npm.cmd run build` passed, embedding `index-B0NmQyWc.js`; release shell and
matching worker compiled in `target/groups-http-drag-20260929/release`. Shell
SHA-256 `1F31BC92A7AD3E90F10AE95CD5166EC92376C0119A8FAB30E1BC96C5E47F713E`;
worker SHA-256 `1C3147C9941FCFFF122E1D08FBB38F2800DE40D87DFB0E3047095F1729986D1A`.
No UI tests or headed interaction were run for this follow-up. Next: confirm
background dragging over blank canvas, wires and nodes in three themes, ensure
those interactions remain available, and verify no audio nodes/connections move.

## Current task (2026-09-29, UI-16)

User requests a group caption font larger than audio-node text, adjustable in
Properties, and a whole-number opacity slider from 1% to 100%. Set the default
caption to 18 px (audio node title is 16 px), permit 12–48 px, and keep opacity
default 5%. Preserve existing local annotations by supplying 18 px when older
stored groups load without a font-size field. No backend/audio graph changes.
Update UI-16 and quickstart. Build updated UI/shell/worker for manual testing;
rollback is a local presentation-only revert. Next: confirm slider endpoints,
font scaling, existing-group migration and readability in three themes.

Result: UI typecheck/production build passed (`index-juGfcH9F.js`); release shell
rebuilt with that bundle, worker unchanged. Shell SHA-256
`03B9AAB8789F5A923BEF697B4B446CFA22C0A473FADDD249EB88F6E78BADBD60`;
worker SHA-256 `1C3147C9941FCFFF122E1D08FBB38F2800DE40D87DFB0E3047095F1729986D1A`.
Saved groups missing font size receive 18 px; existing opacity is rounded and
clamped to the new 1–100 slider range. No tests or UI interaction were run.
Next: manual verification of migration, slider endpoints and caption sizing.

## Current task (2026-09-29, HTTP-07)

User reports the API session list is empty while the UI shows three sessions,
and requests GET/SET for the active editing session. Reconciled durable and
in-memory backend inventories. Added `GET /api/v1/sessions/active` and
`PUT /api/v1/sessions/active` through backend methods; selection emits a global
state event, the UI follows it and preserves its preference, and audio start
remains separate. PUT uses the standard idempotency key. Added schemas,
permissions, OpenAPI, reference docs and PowerShell examples. Existing session
IDs are validated; deleted/stale IDs return a not-found error.

Verification: `npm.cmd exec tsc --noEmit` passed;
`cargo check --manifest-path src-tauri/Cargo.toml --release --features
custom-protocol --locked --target-dir target/groups-http-20260929` passed;
isolated Vite production bundle and release shell build passed. The executable
is `target/groups-http-20260929/release/audiorouter-shell.exe`, SHA-256
`E0116C9B080D1B00564DB50767FDF816F0034901ADDCEDAF577A82069F07C30C`; it
embeds the updated frontend. No automated or manual API behavior checks were
run. The default Vite output is locked by the user's running app, so the shell
was built from an isolated bundle directory; the running shell was not touched.
`cargo fmt --all -- --check` reports pre-existing formatting differences across
the workspace. Next: once the existing shell is closed, manually verify that
the three sessions appear in GET `/api/v1/sessions`, GET returns the current
editor ID, PUT switches the open editor, invalid IDs fail clearly, and this does
not start audio.

Completed prior follow-up (2026-09-29, GRAPH-05/06/08/11/15, CAP-06/10,
UI-04): bypass must work during playback without an explicit Save or Stop.
Investigate revision 123's native path rejection and Discord mic-test repeats.
Read-only revision 124 shows Discord capture → Mixer with Siege EQ → Outplayed,
and a separate microphone processor chain → CABLE-A plus Scarlett monitor.
No Discord edge reaches CABLE-A. Process capture includes mic-test playback;
the output endpoint and external return route require identification before
claiming a complete feedback cause. Preserve the user's graph and app settings.
Tasks: reproduce flag changes with pre-Mixer fan-out and disabled inputs;
fix compilation/resource identity in the owning layer; allow an immediate
flag-only save while preserving unrelated draft edits; add focused native/UI
regressions, document application-playback scope and safe mic-test routing;
run relevant suites/virtual-cable continuity and build a fresh complete pair.
Prerequisites: Windows toolchains and virtual endpoints available; real Discord
Mic Test/Scarlett confirmation remains attended. No microphone recording or
driver work. Rollback: previous sender-routing executable, revert only this
slice; no database migration. Next: reproduce before changing runtime logic.

Diagnosis: Windows endpoint metadata identifies Outplayed as CABLE-B Input
(driver description/INF section), paired with Siege's CABLE-B Output. This
closes a native external loop through Siege EQ → Mixer → Outplayed; disabling
Discord removes its injected mic-test playback but does not repair the return.
Implement a preflight refusal for driver-classified VB-Cable returns and same
endpoint loopback; use exact endpoint IDs and driver descriptions, never node
names or editable endpoint labels. Unknown drivers/external app routes remain
outside this proof. Do not automatically rebind the user's output.
Additional live-state fixes: a Mixer keeps its one remaining input after a
disabled source is pruned; source/sink/Mixer bypass on a prepared native path
means silence while retaining stream identities. Compatible effect bypass is
still dry. The UI submits only the selected saved node's flag, preserving other
unsaved draft edits. A source excluded at preparation still needs preparation
before enabling; no extra microphone is opened to make toggles convenient.

Result: immediate flag-only UI commits preserve unsaved draft edits. Singleton
Mixer preparation/live EQ toggles and silent prepared source bypass pass native
regressions. Installed Cable B metadata preflight rejects the saved return
before audio opens. Backend/shell commit logs retain native activation state
without private reason text. UI 380; engine 145; control 200; transport 23;
Windows adapter 100 passed. Seven opt-in control checks ignored; installed-cable
read-only check passed separately. Three headed Edge theme tests plus live Undo
passed, screenshots reviewed. Native 15 s test with a disabled Mixer input:
four live toggles applied, zero reference/mixed/direct-recording glitches.
One earlier mixed-output gap and corrected sandbox/test experiments remain
documented in [evidence](evidence/2026-09-29-live-bypass-feedback.md).
Complete pair: `target/live-bypass-20260929/release`, current UI embedded and
hashes recorded. Docs acceptance/diff checks passed. Next: attended corrected
Outplayed-cable selection, Discord mic-test and live bypass; no saved binding
was changed. This supersedes the sender-routing build handoff below.

Current follow-up (2026-09-29, UI-03, GRAPH-01/02/03): user confirms starting
EQ → Mixer at the EQ's blue dot. Receiver-first idle handles make that gesture
wire backwards. Decision: make blue a universal sending/start affordance on
every node output (including tools), and reveal orange receiving/input handles
only during that drag. Preserve port roles and canonical edges, generic fan-out,
explicit Mixer and occupied-output reuse. Tasks: update handle styling/guidance,
regress the exact sender-first EQ → Mixer → Output chain and generic tools,
verify three themes and checks, rebuild a complete pair without touching the
running shell/database. Rollback: prior banner-timing build; no audio changes.

Expanded verification found native compiler rejection: the output branching
from the final pre-Mixer tool is not counted as a participating output. User's
clarified audio goal authorizes this correction (GRAPH-02/03, UI-03). Plan:
reuse each already-processed Mixer input for direct output branches before
Mixer input/master gain, with preallocated scratch and nonblocking access;
never process stateful tools twice. Keep Discord out of Scarlett, apply privacy
mute to branches, preserve ring/tap single-write semantics. Add deterministic
signal/gain/isolation and single-execution regressions, run engine and native
continuity checks, rebuild the complete pair. Do not open the running user's
endpoints or alter their session. Rollback now also includes input-branch
compilation/runtime; native hardware evidence must remain explicitly bounded.

Result: sender-first blue → orange gestures work for EQ, Mixer and generic
tools. Exact Siege/Discord fixture retains the EQ → Scarlett branch while
adding EQ → Mixer → Physical Output, including recovery of an existing direct
output edge. Native input branches reuse processed audio before mixing without
re-executing DSP. UI 379 passed; browser 31 passed; engine 144, control 198,
transport 22 and Windows adapter 99 passed (six opt-in control tests ignored).
15 s native virtual-cable continuity: clean reference, zero glitches in mixed
output and direct Recorder branch. [Detailed evidence](evidence/2026-09-29-sender-routing.md).
Next: hand off the rebuilt `target/sender-routing-20260929/release` pair for
the user's attended Scarlett/Discord test. No saved session, device defaults
or existing binaries changed. This supersedes receiver-first guidance below.

Completed prior banner defect (2026-09-29, UI-03/08/12): release
`mixer-topbar-20260928` shows an occupied-EQ-input message when the user tries
EQ → Mixer, and simultaneous warning/info banners overlap the canvas, blocking
Replace and Dismiss hit targets (user screenshot in Downloads). Steps: replace
the fixed three-row shell assumption with content-sized notices and a remaining
height workspace; clarify input-start drag direction; reproduce a mistaken
Mixer → occupied EQ gesture, click/dismiss its notice, then connect EQ → Mixer.
Verify browser geometry and actual button clicks in dark/light/high contrast,
run UI checks, and build a new shell/worker pair. Browser fixtures open no audio
devices; native playback remains attended. Preserve the user's running shell
and database. Rollback: only shell layout, connection guidance, and regressions.

Result: fixed shell notice sizing and sidebar intrinsic-height expansion;
clarified receiving-input drag guidance and cleared pending replacement on
dismissal. 379 UI tests, typecheck, seven targeted Edge tests and final four
regressions passed. Dark/light/high-contrast screenshots inspected. New shell
and matching worker built in `target/banner-timing-20260929/release`; final
embedded asset names verified. See [reproduction, validation and hashes](evidence/2026-09-29-banner-timing.md).
Next action: user closes the running old build and manually checks banner
actions, EQ → Mixer and Timing in the new build. Native playback not rerun;
no audio implementation changed. Existing database and shell left untouched.

Finish AudioRouter's non-driver scope. Audio routes use existing endpoints
(VB-Cable, Voicemeeter, physical WASAPI devices) and are driven by one
backend-owned graph. The desktop UI, CLI and MCP are adapters over the same
versioned API. **Permanent scope decision (user, 2026-09-19):** no
AudioRouter-owned driver, PortCls endpoint or production driver signing.
VDEV-01/03/09 and SEC-08 stay normative only for a possible future funded
track ([future plan](../future/M03-driver-signing.md)).

## Current task — Windows installer and manual release path (2026-09-28)

- Completed UI follow-up (user, 2026-09-28): recurring Route status height
  changes are resolved by removing that panel and its viewport CSS; the live
  run status remains in the header and the immediate microphone privacy-mute
  action is now beside Play/Stop. The desktop viewport regression opens Tools →
  Logs and confirms there is no summary panel and the graph stays visible. If a
  Physical Output is directly fed by a source that also feeds the requested
  Mixer, connecting Mixer → output now removes the redundant direct edge and
  reuses that Mixer, avoiding a duplicate signal and an unnecessary nested
  Mixer. Endpoint choices lead with channel layout, keeping separate stereo and
  multichannel endpoint IDs visible and understandable. Requirements:
  UI-02/03/04 and GRAPH-01/02/03/15. Verification: typecheck and production UI
  build passed; Vitest 377/377; focused Playwright mixer drag/reroute and
  top-bar/Logs behavior; visual screenshots in dark, light and high-contrast
  reviewed at [dark](evidence/2026-09-28-topbar-mute-dark.png),
  [light](evidence/2026-09-28-topbar-mute-light.png), and
  [high contrast](evidence/2026-09-28-topbar-mute-high-contrast.png). Docs
  acceptance passed (69 Markdown files, 373 local links); `git diff --check`
  passed. An isolated release build produced shell and matching plugin worker
  under `target/mixer-topbar-20260928/release/`, with the 2026-09-28 UI bundle
  (`index-BXkPP73A.js`, `index-DoTQAPW0.css`). SHA-256 shell
  `E7CA6B4057DC11D021FFC807F488423F08E5D68B06DDC1530EE46FA760357652`, worker
  `81EC4892834ADAC0ED25EB359A3FA731FCFED203F3D8714287469368E77D8F8C`.
  The standard E2E wrapper could not acquire Cargo's standard target lock;
  direct Playwright passed on its real-backend harness. The endpoint label
  fixture verifies stereo vs 16-channel wording; exact local endpoint
  identities and live audio remain attended Windows checks. Rollback is limited
  to the header, mixer-reuse draft logic, endpoint labels, focused tests, and
  interface wording.

- Completed follow-up (user, 2026-09-28): fan-out is a generic port behavior,
  not an EQ-specific feature. `appendDraftConnection` has no tool-kind gate:
  any output-capable library node can feed multiple destinations; ordinary
  inputs still take one source, and sum paths stay explicit through Mixer.
  Exact source node/port matching also lets the existing occupied-output
  recovery route any such source through a Mixer without duplicating its direct
  branch. An exhaustive type-checked library-node matrix plus a scanned-plugin
  case verify multi-output fan-out, Mixer routing and occupied-output reuse.
  `draft.test.ts` passed (48/48), TypeScript typecheck passed, docs acceptance
  passed (69 Markdown files, 373 links), and `git diff --check` passed. The
  generic Gain browser regression and the EQ/Mixer reroute regression both
  passed in Edge (2/2). The normal Playwright-managed Vite server stalled after
  browser close; running Vite separately and using a temporary config with
  `reuseExistingServer` produced the final results. The temporary config and
  server were removed/stopped after the run. This is browser UI harness evidence,
  not live endpoint/audio qualification. Requirements:
  GRAPH-02/03 and UI-03. No per-tool routing code was needed. Rollback: revert
  the focused regression and specification clarification.

- Objective: implement a per-user Windows installer and manual GitHub draft
  release path, then document and qualify within available Windows evidence.
- Current subtask (user, 2026-09-28): simplify canvas wire creation. Idle
  canvas shows input handles only; once a drag starts at an input, show output
  handles only. Preserve canonical output-to-input graph edges, keep the
  keyboard connection form, and reset handle visibility when the gesture ends.
  Requirement: UI-03 / M05. Rollback is limited to canvas handle state/styles
  and their focused UI tests.
- Current follow-up (user, 2026-09-28): diagnose the `plugin worker executable
  is unavailable` error from the manual test build. The shell was built in a
  fresh target directory without its adjacent worker. Requirements: PLUG-03/05,
  M06/M08. Immediate recovery is to build the matching worker beside that
  shell. Add a focused test for an actionable missing-worker error, then build
  a fresh complete shell/worker pair in a separate target directory. Do not
  stop the user's running shell; rollback is reverting the error wording/spec
  change and using the prior build.
- Current UI follow-up (user, 2026-09-28): idle hides every connector on
  source/input nodes such as Physical Input, because those graph ports are
  outputs. Keep a blue, visible start handle on source nodes while idle and
  reveal compatible destination handles during the drag; preserve hidden
  orange outputs on tools/destinations and canonical edge direction. Complete:
  source-start and destination-start gestures pass the focused browser test;
  screenshots were reviewed in dark, light and high-contrast themes. Typecheck
  and all 372 UI unit tests pass. Requirement: UI-03 / M05.
- Current UI follow-up (user, 2026-09-28): compact Route status became too tall
  after opening Logs from Tools. Reproduce Tools → Logs with collapsed “What
  happens next”, measure status panel geometry at the user's desktop viewport,
  then add a focused regression and correct the responsible layout constraint.
  Requirement: UI-03 / M05. Rollback: revert only the compact-status layout
  correction and its regression.
- Current UI follow-up (user, 2026-09-28): make graph revision conflicts
  actionable. A stale save returned raw Store debug details and suggested a
  retry that could not work against the stale revision. Now the UI explains
  that the save was rejected, the draft remains, and the user must review and
  plan again; conflict refresh also reloads session inventory. Add an App-level
  regression to prove draft preservation and latest revision refresh.
  Requirement: STATE-03 / UI-03. Rollback: revert conflict presentation/refresh
  changes and their focused coverage.
- Current UI slice (user, 2026-09-28): make the Advanced EQ chart's useful
  range easier to read. Complete: chart axis/drag span is ±12 dB, precise
  controls retain ±24 dB; numbered circles move to non-overlapping gutters and
  point to exact band positions with leader lines and small dots. Coverage:
  clustered 16-band layout and axis/markers (AdvancedEqEditor tests), plus
  real-backend UI checks and screenshots for dark, light and high contrast.
  Evidence: all 375 UI tests pass; AdvancedEqEditor tests 7/7; typecheck passes;
  the Advanced EQ drag-range browser regression and filter/callout checks pass
  in dark, light and high contrast. Screenshots:
  [dark](evidence/2026-09-28-advanced-eq-dark.png),
  [light](evidence/2026-09-28-advanced-eq-light.png),
  [high contrast](evidence/2026-09-28-advanced-eq-high-contrast.png). Release
  shell build at `target/advanced-eq-callout-20260928/release/` embeds
  `index-ClMVy-ZD.js` / `index-Ce_DktWe.css`; matching worker is adjacent.
  SHA-256 shell `455E0098A2FB31EDAD826E2C5569FDACC2B8EBBFD6297552FF295832888E873E`,
  worker `A8ED473893705ABD3C9AD2F6C31C03898181931750EF46F95180F193E4F9143B`.
  Requirements: DSP-02 / UI-05; M04/M05. No DSP, API, saved-value or audio-path
  change. Rollback: revert the Advanced EQ chart scale/callouts and focused
  tests/spec note.
- Requirements: PROD-01/03/04/06/07, SEC-09/11, ENG-04/05, and new DIST-01–08
  in [delivery traceability](../../spec/15-delivery.md). User direction on
  2026-09-28 promotes installer/release work into M08. DEC-16 still excludes
  the AudioRouter-owned driver and driver signing. User selected an explicitly
  unsigned first app release; no signing credential purchase is authorized.
- Prerequisites/evidence: Windows x64 build host and pinned local Tauri 2.11.4
  CLI; unsigned NSIS build succeeded at 8,217,235 bytes. It was not installed.
  M08 clean-machine/install gates remain open. Existing endpoints are still
  needed for real audio routes.
- Decisions: per-user NSIS, WebView2 downloaded bootstrapper fallback, CLI and
  plugin worker bundled as resources, state remains in `%LOCALAPPDATA%`, no
  updater, no OS default changes or startup audio. Release workflow is
  `workflow_dispatch` only and creates a draft; publishing stays manual.
- Ordered tasks: (1) promote DIST acceptance and revise M08/DEC-16 boundary;
  (2) bundle shell/CLI/worker and resolve installed resource paths; (3) add a
  concise setup guide and no-endpoint state; (4) extend artifact preparation,
  verification, and focused regression tests; (5) add manual PowerShell and
  GitHub draft-release path; (6) run portable/docs/release checks and NSIS
  build/inspection; (7) record install/signing/clean-machine work that needs a
  disposable second Windows environment before claiming full M08 completion.
- Validation matrix: portable tests for resource resolution/release metadata;
  Windows NSIS bundle contents and checksums; docs acceptance; no-endpoint UI
  setup state. Standard-user install, upgrade, uninstall, missing-WebView2,
  signature reputation, multiple Windows builds, and hardware routes require
  an isolated Windows test account/machine and remain release gates.
- Risks: install layout can strand CLI/worker; a setup EXE can be unsigned but
  still cause trust prompts; endpoint drivers remain external; the bundle may
  require internet to bootstrap WebView2. Roll back only release/config/UI
  changes; preserve `%LOCALAPPDATA%\AudioRouter` and do not touch audio drivers
  or Windows endpoint defaults.
- Verification so far: UI suite 372/372, UI typecheck/build, Rust workspace
  tests (locked, Windows audio adapter excluded), packaged-worker resource-path
  test, artifact preparation/verifier tests, docs acceptance (69 files/364
  links), and unsigned x64 NSIS smoke passed. The bundle is build evidence only.
- UI-03 handle interaction: the canvas defaults to inputs and source start
  affordances, reveals outputs
  while an input-start drag is active, accepts that reverse gesture, normalizes
  the saved connection to output-to-input, and restores idle state after
  completion. Playwright checks every handle in dark, light and high-contrast
  modes and captures six screenshots for review.
- UI verification: `npm.cmd run typecheck` passed; Vitest passed (31 files,
  372 tests); Playwright passed the connector direction/theme scenario and the
  multi-source workflow. A follow-up `npm.cmd run build` was blocked while
  Vite tried to remove the existing `ui/dist/assets/index-C4G0bSnz.js`
  (`EPERM`, Windows file lock). Built the same UI sources successfully into
  `%TEMP%\audiorouter-ui-connector-ux-20260928`; its Vite asset hashes match
  `ui/dist`. The optimized custom-protocol shell then built with the current
  `ui/dist` into `target/connector-ux-20260928`; its binary includes the
  content-hashed `index-C4G0bSnz.js` bundle name. Shell SHA-256:
  `FCFDF1FE04B2A660DE39EE86A1F4408EA60F9A8852A1CD73EEE61BCDCFA3D5EA`.
  No app launch or live endpoint test was performed.
- Plugin-worker recovery (2026-09-28): confirmed the new test shell directory
  initially lacked `audiorouter-plugin-worker.exe`, while the resolver checks
  beside-shell and `resources` locations. Built the matching worker beside
  that running shell; it now exists at the resolver's first candidate. Added
  actionable missing-path/remediation text to both plugin preparation paths
  and PLUG-05. Focused control tests pass (2/2); docs acceptance passes (69
  Markdown files, 364 links). Complete rebuilt pair:
  `src-tauri/target/release/audiorouter-shell.exe` and adjacent
  `audiorouter-plugin-worker.exe`; shell SHA-256
  `E76CA033A10298A6FFC01F17D1FE9F3F0AED35B21CAAC0E70390684EEFE1C533`,
  worker SHA-256
  `1AF8FC540D2D7F9211F9F6A3AF760A692659736DB3268D4F427BD6EB8A058CC1`.
  Neither build has been manually exercised with a plugin. The currently
  running shell is still the earlier connector test build; it can use the
  newly placed adjacent worker, but it will retain the old generic message.
- Formatting: `cargo fmt --all -- --check` reports pre-existing workspace
  formatting differences across unrelated crates; this change's added helper
  and test were aligned with rustfmt. The manual release workflow could not be
  parsed with a YAML tool because no parser is installed; inspected structurally.
- Limitations: no standard-user install/upgrade/uninstall, clean-machine,
  missing-WebView2, reputation prompt, or live endpoint qualification yet.
- Visual review: the canvas harness screenshots were reviewed in dark, light
  and high-contrast modes. Attended packaged-shell acceptance remains open.
- UI defect follow-up (2026-09-28): reproducing Tools → Logs with compact Route
  status enabled showed the status panel at 16 px before Logs and an 81 px
  increase after Logs, despite the guide being closed. Set a stable 52 px
  desktop row and full-width bounded panel; allow natural wrapping on narrow
  screens. Added Playwright geometry regression. At 1280×720 the corrected
  panel measured 52 px. App-level conflict regression preserves an edited
  session name and checks the actionable error; backend formatter regression
  rejects raw Store debug text and stale-plan retry advice. Typecheck passed;
  all 373 UI unit tests passed, including the new conflict regression. The
  compact-status Playwright test passed at 1280×720 and checked Tools → Logs
  with the guide collapsed; its status row stayed 52 px. Theme screenshots
  were reviewed and saved: [dark](evidence/2026-09-28-route-status-dark.png),
  [light](evidence/2026-09-28-route-status-light.png), and
  [high contrast](evidence/2026-09-28-route-status-high-contrast.png). The
  Playwright test reported success, but its Vite child remained alive until
  the test process was interrupted. A release shell and adjacent worker were
  built under `target/status-conflict-20260928/release`; the shell contains
  the expected `index-B3Cq7s68.js` and `index-Dihsqrkt.css` assets. SHA-256:
  shell `0B8F8DBBD0DA647D5100BC0B26848EE66B55AEE8569714BCB41AD32FB732BE84`;
  worker `10F669A8C20C551B32334ED3103A8C2781889BD0B09768EC38721355DE7081C9`.
  No attended shell launch or live audio validation was done.
- Next action: produce a matching shell/worker pair embedding the current UI in
  a separate target directory; then manually exercise the reported EQ → Mixer
  → Physical Output route and inspect the host's exact Cable-B endpoint IDs and
  channel layouts. Continue M08 install testing on an isolated Windows
  account/machine after that attended graph check.

Requirement families in scope: PROD, ARCH, GRAPH, CAP, DSP, REC, PLUG, UI,
API, AUTO, STATE, SEC (non-driver), NFR, QUAL, ENG, as mapped in
[delivery traceability](../../spec/15-delivery.md#requirement-traceability).

## Where things stand

Legend: **Implemented** = code exists with automated tests. **Qualified** =
measured on real Windows devices (evidence linked). **Open** = required
evidence is missing. Nothing here is a release claim: M08 is not done.

| Milestone | Implemented | Qualified on Windows | Open |
| --- | --- | --- | --- |
| [M00](../../milestones/M00-feasibility.md) feasibility | WASAPI probes, calibrated loopback tooling | [WASAPI probe](evidence/M00-wasapi-probe.md); NFR-01 p95 ≈155–186 ms (target revised to ≤250 ms, DEC-14) | Owned-driver feasibility (permanently out of scope) |
| [M01](../../milestones/M01-contracts.md) contracts | Domain, storage, authorization, CLI/MCP parity | [M01 evidence](evidence/M01-contracts.md) | Final release acceptance only |
| [M02](../../milestones/M02-audio-engine.md) audio engine | Realtime graph, capture/render adapters, multi-input/many-output paths, backend audio service, Network Send/Receive (GRAPH-16) | Guarded routes ([M02](evidence/M02-audio-engine.md)); **continuity 0 glitches for 30–60 s on four route shapes** ([audio continuity](evidence/2026-09-26-audio-continuity.md)); NFR-02 p95 ≈97–115 ms (≤160 ms, DEC-15) | Clock-drift correction between independent devices; endurance/soak |
| [M03](../../milestones/M03-virtual-routing.md) virtual routing | Exact endpoint identity, VB-Cable/Voicemeeter routes, rebind | [M03](evidence/M03-virtual-routing.md) | Owned virtual devices (out of scope) |
| [M04](../../milestones/M04-effects-recording.md) effects/recording | 17 built-in processors incl. pitch; recorder (WAV/FLAC/MP3), library | Synthetic DSP vectors; [feature confidence](evidence/2026-09-26-feature-confidence.md); [voice/tool behavior](evidence/2026-09-27-voice-tools.md) | Attended/long-duration recording on real devices |
| [M05](../../milestones/M05-visual-editor.md) visual editor | Canvas, Properties/Tools, live flags, timing, three themes | 92 browser E2E + 375 UI unit tests; Edge visual review | Attended Narrator, 200 % scaling, first-run, live drag/drop; UI-15 attended edge activity |
| [M06](../../milestones/M06-plugins-pitch.md) plugins/pitch | VST3 worker, x64 VST2 worker, shared adjacent-VST2 chain, editors, saved state | ReaPlugs chain live ([shared chain](evidence/2026-09-26-shared-vst2-chain.md)); 60 s glitch-free with the user's saved ReaPlugs nodes | Rights/sandbox review, multi-vendor matrix |
| [M07](../../milestones/M07-automation-recovery.md) automation/recovery | MCP, persistence, crash journal, safe mode, sign-in helper | [M07](evidence/M07-automation-recovery.md), [OS transitions](evidence/M07-os-transitions.md) | OS power/session delivery and native reopen (attended) |
| [M08](../../milestones/M08-release.md) release | Unsigned artifact preparation | [M08](evidence/M08-release.md) | Clean-checkout release run and CAP-13/GRAPH-15 evidence; installer and clean-machine gates are excluded from v1 by DEC-16; driver signing is excluded, app/installer signing is undecided |

## 2026-09-26/27 overnight session — outcome

User request: commit everything, then verify features, above all sound
quality (crackling), clean up the Markdown, and add network send/receive
tools once everything else is in order. Full permission; the user tests
by hand in the morning.

- Committed the pending work (`cf50d8ff`). Build outputs `ui/dist-review-*`
  are now ignored rather than committed.
- **Crackling root cause found and fixed**
  ([evidence](evidence/2026-09-26-audio-continuity.md)). Each processed
  quantum was written twice into the output ring: about 275 repeated or
  skipped 128-frame blocks per second. Audio also depended on UI-timed
  RPC pumps, and the render device had no jitter margin. Measured after
  the fixes: 0 glitches over 30–60 s for a direct route, a five-processor
  chain, the single-endpoint worker, and the user's saved ReaPlugs chain.
- **Test Signal and Audio File were chopped** (~100 discontinuities/s)
  because generated sources were not paced in real time. They now are:
  30 s clean.
- **Network Send / Network Receive tools added** (user request, see the log
  and [quickstart how-to](../../operations/quickstart.md#stream-audio-to-another-computer-network-send-network-receive)).
  Verified on one machine over UDP 127.0.0.1: 2 Ã— 30 s, 0 packets lost,
  0 glitches.
- **Bypassed plugins no longer block Play.** The user's saved session has
  ReaEQ bypassed, which made `nativePaths.prepare` fail. Regression test
  added.
- Plugin bridge and worker threads now use MMCSS "Pro Audio".
- Status line now shows output underruns and late backend audio-service
  gaps.
- **Recording on multi-path sessions fixed.** Before, stop failed and only
  about 21 ms was kept. Routes with a Recorder branch now use the
  multi-path worker. Live: a 15 s WAV with 0 glitches
  ([evidence](evidence/2026-09-26-audio-continuity.md), finding 7).
- The render jitter cushion is adaptive: 10 ms, growing 5 ms per real
  underrun, up to 40 ms.
- Checks (final, 2026-09-27): `cargo test --workspace` 0 failures; UI vitest
  353 passed; browser E2E 67/67; contract drift, documentation and M08
  traceability (175 IDs) all pass.
- Artifact for the attended test (built 03:10 local on 2026-09-27; UI
  bundle `index-DAgNSICR.js` confirmed embedded; shell newer than
  `ui/dist`):
  `C:\code\audiorouter\target\patrick-main-release-2\release\audiorouter-shell.exe`
  SHA256 `67D6E97696984209360CF7599895267AA051773BF9CE6DCAC75517727CFA9FEF`.
  Worker beside it: SHA256
  `275190960CDF8E6ADEF5BEEFF596A474B084CE72494D6FF7399D9DE683DDBCA8`.
- Since about 23:00 the machine has shown occasional ~10 ms gaps on the
  virtual-cable harness. An earlier commit that measured clean at 22:40
  shows the same rate, so this is environmental; see the
  [evidence](evidence/2026-09-26-audio-continuity.md#environment-observation-2300-onwards).

## Open defects and known gaps

1. Clock drift is not corrected in code (`DriftController` is unused). On
   the user's devices it measured under 1 ppm (flat 17 ms queues over
   3 minutes, [drift survey](evidence/2026-09-26-audio-continuity.md#clock-drift-survey-of-the-users-saved-session)),
   so it is low priority here. Other hardware can drift more.
2. Latency does not shrink by itself after a stall. The output queue can
   stay up to about 70 ms until Stop/Play.
3. Signal timing covers multi-input routes only. A plugin's own reported
   VST latency is not added to its queue time.
4. Plugin failure reasons are not shown in the UI (only the `failed`
   state).
5. Workspace-wide `cargo fmt`/clippy drift; CAP-13 and GRAPH-15 still need
   their required evidence before the M08 gate.
6. Network audio is unencrypted and filters by sender address (SEC-13).
   Only localhost is qualified; two-PC delivery, firewall behavior, and
   receiver clock drift remain unmeasured. Do not expose it beyond a trusted
   local network.

## Next actions, in order

1. Review the proposed Windows distribution plan and explicitly promote or
   retain it as future-only before implementation.
2. Continue the M05 attended accessibility/scaling review and close the M08
   clean-checkout, security, performance and traceability gates within scope.
3. Qualify network send/receive on two physical computers when that setup is
   available; localhost tests do not establish cross-device behavior.
4. Consider live clock-drift correction for cross-device paths after
   measurement shows a need.

## Validation commands

| Area | Command |
| --- | --- |
| Rust | `cargo test --workspace --locked` |
| UI | `cd ui; npx vitest run --configLoader runner --exclude "e2e/**"`; `npm run build` |
| Browser E2E | `cd ui; npm run e2e` |
| Contracts | `cd contracts; npm run typecheck; npm run check:drift` |
| Docs | `powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\docs.ps1` |
| Audio continuity (live, VB-Cable) | `AUDIOROUTER_LIVE_CONTINUITY=1 cargo test -p audiorouter-transport --test live_audio_continuity -- --ignored --nocapture` ([options](evidence/2026-09-26-audio-continuity.md#method)) |
| Saved-session drift (live, privacy-muted, DB copy) | `AUDIOROUTER_DRIFT_DATABASE=<copy> cargo test -p audiorouter-transport --test live_audio_continuity live_saved_session_output_queue_drift -- --ignored --nocapture` |
| Release shell | `cargo build --manifest-path src-tauri/Cargo.toml --release --features custom-protocol`, then confirm the embedded bundle (AGENTS.md lessons) |

## Decisions in force

- No AudioRouter-owned driver (2026-09-19, permanent).
- DEC-14 NFR-01 ≤250 ms p95; DEC-15 NFR-02 ≤160 ms p95 (2026-09-21).
- Adjacent compatible VST2 plugins share one isolated worker. A fault
  silences the whole group (2026-09-26).
- Live flag changes apply without stopping Play when topology is prepared
  (2026-09-26).
- Network audio (user request, 2026-09-26): UDP on the LAN, uncompressed
  48 kHz float32, one quantum per datagram. Addresses are IP literals only;
  the receiver accepts one sender address. No encryption. Streaming exists
  only while a session with a network node is prepared (GRAPH-16, SEC-13).
- The desktop shell grant includes Record (2026-09-22) and PluginScan
  (2026-09-25). DeviceAdministration requires the explicit
  `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` opt-in.

## Risks and rollback

- Backend audio service: reverting `serve_control_connections_forever_with_grant`
  to the single-threaded loop brings back UI-dependent audio. Keep the 5 ms
  UI pump fallback for backends that do not report `audioService.active`.
- The render cushion and headroom add up to 10 ms of steady latency, bounded
  by about 70 ms. Revert with `RENDER_JITTER_CUSHION_FRAMES = 0` and
  `SharedRender::open` in the fan-out.
- Saved sessions and database formats are unchanged tonight.

## Log

### 2026-09-28 — Windows distribution and manual release plan

- User requested planning for a simple integrated install and a manual
  GitHub-release mechanism. Added the future plan and a current-state
  operations page; no installer or workflow was implemented.
- The plan targets per-user setup and manual draft creation with separate
  human publication. It preserves DEC-16's driver/signing boundary and records
  that M08 must be explicitly revised before implementation.
- Verification: documentation acceptance passed for 69 Markdown files and
  363 local links; `git diff --check` passed. No Windows install or release
  evidence was generated.
- Next: resolve M08 scope, signing expectations, WebView2 policy, and whether
  the existing-endpoint prerequisite meets the desired install experience.

### 2026-09-28 — Markdown and combined-tool coverage review

- Reconciled the plugin-scan permission text, processor qualification
  traceability, current M04/M05 test counts, release handoff, and M00/M03/M08
  wording with the VB-Cable-first scope decision (DEC-16). Historical dated
  logs and artifacts remain historical.
- Mapped processor, mixer, branch, plugin, recorder, independent-path and
  localhost-network combinations to existing regression tests. No test was
  added because the reviewed combinations already have meaningful coverage;
  remaining scenarios require separate Windows, hardware, second-PC or release
  gates. See [coverage evidence](evidence/2026-09-28-tool-combination-coverage.md).
- Verification: engine 141 + 5 integration tests, transport 22 + 1 detector,
  and the control/domain/DSP/plugin-host suites passed. Private-audio and
  live-device opt-ins remained ignored. Documentation acceptance passed for
  67 Markdown files / 358 links; `git diff --check` passed. No code behavior
  changed.
- Next: M05 attended accessibility/scaling; in-scope M08 qualification. The
  unsigned artifact, installation and clean-machine gates remain excluded by
  DEC-16 and are not release claims.

### 2026-09-27 — Voice-sample qualification of all audio tools

- User explicitly rejects level-only qualification: Bass & Treble must
  meaningfully shape speech like their other tool. Authorizes reuse of the
  local voice sample, then configuration/behavior checks of every audio tool.
  Supersedes the earlier conclusion that the Bass/Treble report was complete.
- Scope/IDs: DSP-01–18, GRAPH-04/14/15, REC/PLUG behavior where applicable.
  First measure separate bass/treble spectral changes and compare broader
  shelf candidates on identical, loudness-matched voice. Determine intended
  reference sound (other-tool name requested), fix controls/contracts together,
  then qualify every built-in effect, routing/source/sink tool and installed
  plugin within its supported boundaries.
- Ordered tasks: build opt-in private voice harness with behavior-specific
  assertions and private audition files; Bass/Treble comparison and owning
  layer correction; remaining tools with real speech and controlled added
  defects (hum/click/noise); resolve defects; focused checks, documentation,
  final release build. No private audio committed or uploaded; no recapture
  necessary. Preserve current user session and unrelated working-tree edits.
- Prerequisites: existing five-second WAV present in temp directory. Offline
  tests do not prove live continuity, network delivery on another PC or vendor
  plugin compatibility. Synthetic defects supplement speech when speech alone
  cannot establish restoration behavior. Native tests need exact devices and
  existing authorization. Rollback: revert only new behavior changes; retain
  prior executable, no persistence migration unless explicitly documented.
- Decision/evidence: the original treble shelf changes upper speech probes
  only +3.25 dB and total voice RMS +0.03 dB at +12. Broader 300 Hz/3 kHz
  shelves produce +11.15 dB low / +9.13 dB high changes independently;
  opposite settings produce over 18 dB spectral tilt. Update DSP-11 and
  production tuning to these shelves, retaining the two simple controls.
  Saved nonzero settings become stronger; Advanced EQ recreates old tuning.
  Loudness-matched files remain private in temp. Clean speech denoising
  correctly makes little change; test added noise rather than requiring damage.
- Next: finish behavior-specific voice checks, audition the stronger shelves,
  verify UI descriptions in all themes, and build a new executable.
- Attended result: the user heard the loudness-matched 300/3000 Hz comparison
  but still found it too subtle. Supersede that tuning with adjustable shelves
  (bass 80–1000 Hz, default 500; treble 800–12000 Hz, default 1500). Existing
  +12/-12 dB range retained. Original response is available at 120/6000 Hz.
  Backend validation/catalog, UI defaults/help and DSP-11 updated together.
  Speech harness also passes hum/click removal and learned/adaptive noise
  suppression using controlled contamination of the same saved voice.
- Completed: user confirmed final 500/1500 Hz audition is strong enough.
  Representative speech configurations passed for all built-in processors,
  routing and transport tools; nine installed ReaPlugs processed the sample
  and exposed audio parameters changed it. Audio File, WAV/MP3 recording,
  and localhost Network Send/Receive passed sample-preservation checks.
  Unsupported virtual-device/placeholder entries remain unavailable; this
  is not second-PC network or exhaustive vendor-plugin qualification.
  [Evidence and limitations](evidence/2026-09-27-voice-tools.md).
- Checks: domain 70, control 196 (six opt-ins ignored), engine 141 plus five
  route integrations, private voice/tool/delivery/plugin opt-ins, UI 370,
  three Edge themes and documentation 66 files/348 links passed. Native
  47 Hz Bass/Treble route: reference and result 30 s, zero glitches/silence/
  discontinuity flags; backend late gaps zero. No user session edits.
- Artifact: `C:\code\audiorouter\target\patrick-main-release-10\release\audiorouter-shell.exe`.
  Custom-protocol release; current `index-3aXwyTtB.js` verified embedded;
  bundle newer than changed UI, shell newer than bundle. Matching CLI and
  plugin worker adjacent. Prior release-9 retained for rollback.
  Shell SHA-256: `5EEEB8C23BA37E7750B67F3E5BF8C8035537A0035AC4767BA0D869206FEC9520`.
  CLI SHA-256: `A5459947E9FA3181F15515960E4D4033B19575B7024104508D186FC5D478D6DD`.
  Worker SHA-256: `338426BFAEA73E7A8DB27DC9686304020CFD3DC78CFD2155B14882D4B64AF634`.
- Next: hand off the executable for live microphone use; broader milestone
  gates stay listed above and are not waived by this qualification.

### 2026-09-27 — Source-less processor chains block Play

- Authorized defect fix (GRAPH-12/15, UI-07): detaching Bass & Treble from
  FIR Filter Hz leaves the latter chain without input. Native preparation
  mistakes its first processor for a capture source and requests a device.
- Decision: exclude input-bearing nodes with no enabled upstream feed,
  transitively, from runtime preparation only. Preserve the saved graph,
  real sources with missing device bindings, and fed failed processors.
  Show a non-blocking warning for enabled excluded nodes in the editor.
- Steps: extend backend pruning/compiler and UI route classification;
  regress direct tone/output plus detached filter chain; check UI warning
  in all three themes; build a new embedded-UI executable.
- Validation: engine/compiler and UI tests on Windows; simulated browser
  playback is not Scarlett evidence. Do not launch alongside the user's app.
- Rollback: revert only this pruning/warning change; no schema migration or
  persisted graph edits. Next: implement and verify the reproduced shape.
- Implemented: engine runtime pruning excludes inputless chains before
  single-route/multi-path compilation; control plugin preparation also skips
  excluded nodes. Single-endpoint paths preserve disabled capture mute stages.
  UI routing helpers mirror pruning and show a non-blocking warning above the
  canvas. Saved nodes/connections remain intact. Graph specification and
  quickstart updated; processor-only block compilation remains compatible.
- Final Windows checks: `cargo test -p audiorouter-engine -p
  audiorouter-control --locked --quiet`: engine 140 + 4 integration tests,
  control 196 passed (6 hardware tests ignored). UI vitest 370 passed;
  `npm.cmd run build` passed. `npx.cmd playwright test
  e2e/unfed-processors.pw.ts`: 3 passed; screenshots visually reviewed at
  `%TEMP%/audiorouter-unfed-{dark,light,high-contrast}.png`. Browser fixture
  starts/stops a simulated direct route; compiler regression processes actual
  sample blocks and checks output identity. Scarlett acceptance remains
  attended; no real microphone captured or app launched by the agent.
- Experiments corrected: generic UI fixture gave capture sources input ports
  and was fixed to match real source ports; dropping an entire processor-only
  graph broke two existing DSP tests, so that existing compile API fallback
  was retained. Sandbox denied replacing generated UI assets; normal-permission
  build passed. Browser tests initially passed but stalled during temporary
  Vite teardown; terminated only that verified test server and reran with
  normal process permissions (3 passed, clean exit).
- Documentation check: 65 files / 346 local links; `git diff --check` passed.
  RTK remains unavailable. Next: user tests the direct Bass & Treble path in
  the new release build; Bass/Treble audibility remains a separate open report.
- Artifact: `C:\code\audiorouter\target\patrick-main-release-9\release\audiorouter-shell.exe`,
  built with `custom-protocol`; embedded bundle `index-DoB_9MLY.js` verified
  (contains the new warning), binary newer than UI assets. Shell SHA256
  `BF907177C0B76C01F860952B23329F48775CC6FE016549C2E091324E63DCF12F`.
  Existing release folders retained for rollback. No installer/signing claim.
  Matching CLI and plugin worker rebuilt beside the shell with
  `cargo build --release --locked -p audiorouter-cli -p
  audiorouter-plugin-host --target-dir target/patrick-main-release-9` (passed).

### 2026-09-27 — Bass & Treble reported inaudible

- Latest attended reproduction: release-9 direct Microphone → Bass & Treble
  → Scarlett still has no perceived tone change, with downstream tools
  detached. Read-only revision 106 confirms Bass +11.5 / Treble +12, enabled
  and not bypassed. Retained stopped telemetry shows processor timing and
  matching Bass/Scarlett levels; do not present retained RMS as audible proof.
- Next authorized investigation (DSP-11, GRAPH-15, UI-07): reproduce the
  mono-source/stereo-tool direct path in the actual multi-path runtime with
  low/high known tones, including parameter replacement; then investigate
  native delivery if sample-domain behavior passes. No user route changes,
  microphone recording or shell launches. Rollback: diagnostic tests only
  until a owning-layer defect is established. Hardware confirmation remains
  separate from offline checks; preserve all current working-tree changes.
- Direct mono-source/stereo-tool runtime regression passed: 60 Hz at Bass
  +/-12 changes by +/-11.11 dB; 10 kHz Treble +/-12 changes by +/-10.83 dB.
  Recompiles via `replace_paths` and verifies identical stereo output and no
  crossfeed. `cargo test -p audiorouter-engine --locked --quiet`: 141 unit +
  4 integration tests passed. Initial test needed to recycle its ring blocks;
  that harness ownership error was corrected before interpreting results.
- User explicitly authorized a quiet synthetic Scarlett comparison. Added
  opt-in `live_physical_output_bass_treble_comparison` to the transport harness:
  private control pipe/in-memory session, Test Signal -> Bass & Treble -> exact
  Scarlett render endpoint, base -40 dBFS, 2.4-second maximum tones. No microphone
  opened, no samples recorded, no user DB/session edits, no shell launched.
  Initial mono-generator shape was unsupported and stopped before audio;
  supported stereo generator rerun passed (8.87 s). Command: set
  `AUDIOROUTER_TONE_COMPARISON=1`,
  `AUDIOROUTER_TONE_COMPARISON_OUTPUT={0.0.0.00000000}.{829bab15-21b8-47d1-964a-f843aa3b37d6}`;
  `cargo test -p audiorouter-transport --test live_audio_continuity --locked
  live_physical_output_bass_treble_comparison -- --ignored --nocapture`.
  Branch RMS dBFS: Bass cut -52.43, boost -30.19 (22.23 dB difference);
  Treble cut -52.16, boost -33.86 (18.30 dB difference); zero clipped samples.
  These are native branch meters, not external Scarlett loopback measurements.
  User confirms both audible level changes in Scarlett headphones. This
  establishes audible native synthetic processing on that device; the separate
  report about speaking through the microphone remains unresolved. Requested
  explicit consent for a five-second local microphone sample with flat and
  processed versions of identical input. Do not capture until consent arrives.
  Runtime production code unchanged; release-9 remains current.
- User explicitly consented to a five-second local microphone sample.
  Added ignored, opt-in `live_local_voice_tone_comparison` (exact endpoint,
  verified float32 format, 5-second/7-second deadline, first voice channel).
  Capture passed in 5.20 s. Identical sampled voice processed through the
  engine's Bass & Treble: both +12 gives +5.38 dB overall RMS, both -12
  gives -3.52 dB. Three PCM WAVs kept only in
  `%TEMP%/audiorouter-voice-comparison-6788/` (`flat`, `boost`, `cut`);
  common scaling prevents clipping and preserves level differences.
  No voice samples/transcript logged, committed or sent off this computer.
- Attended playback of those files on the exact Scarlett endpoint passed
  (16.52 s): flat, boost, cut, common peak cap 0.05, no user-session changes.
  Playback helper validates temp-directory containment and device format.
  Awaiting user's listening result. Only qualification tests and their engine
  dev-dependency/Cargo.lock changed; production DSP behavior is unchanged.
  `cargo test -p audiorouter-transport --test live_audio_continuity --locked
  --quiet`: 1 deterministic detector passed, 6 opt-in hardware tests ignored.
  New private comparison files may be removed only within the requested
  local-sample lifecycle; no automatic deletion of unrelated user audio.
- User confirms audible differences between flat, boosted and cut versions
  of the identical voice sample through Scarlett. Qualification confirms
  DSP-11 processing, live parameter replacement in the mono path, native
  synthetic output and voice playback audibility. No production filter defect
  reproduced. Why changes were not perceived while speaking remains an
  inference (own unprocessed voice can mask monitoring; shelf bands leave
  middle speech frequencies mostly intact), not a proved routing defect.
  Diagnostic slice complete; keep release-9. Next: return to M05 attended
  testing / M08 release gates unless the user requests a broader tone response.
- User reports no audible change at about +10 dB Bass (DSP-11, UI-07).
- Read-only inspection of the current saved session (revision 88):
  bassTreble-1 enabled, not bypassed, bassDb 10.6, trebleDb 0; connected
  mic → Bass & Treble → FIR Filter Hz → Advanced EQ → ReaComp → ReaGate.
  EQ has an enabled 72 Hz high pass. This tool is on the microphone path,
  not the game path. No session edits, native launches or audio capture made.
- Verified compiler reads bassDb/trebleDb into fixed 120 Hz / 6 kHz shelves.
  `cargo test -p audiorouter-engine
  restoration_and_tone_tools_compile_and_shape_known_signals --locked`
  passed on Windows: the deterministic vector verifies bass boost and
  treble cut. This is offline DSP evidence, not confirmation of the user's
  currently playing graph or an attended audible result.
- Explanation remains conditional: little sub-120-Hz source energy and
  downstream filtering/dynamics can mask the boost; it cannot change audio
  on the separate game route. A saved value alone does not prove live
  adoption after adding topology. Next: identify which sound the user is
  monitoring and check live adoption if the microphone also has no change.
  No implementation defect established; rollback unnecessary.

Follow-up: user confirms speaking into the microphone with no perceived
change. Two read-only `system.diagnostics` requests to the existing release-8
backend show `configured-stopped`, multi-input. Retained telemetry includes
bassTreble-1 processing time (2.09 us average) and RMS -61.36 dB versus mic
-71.79 dB (about +10.4 dB); this establishes execution and a prior level
difference, not live speech quality or response to the latest slider edit.
Did not start/stop the user's route, launch a shell, or change parameters.
User confirms the sound is their voice in Scarlett headphones. Their Stop
test removes the voice, establishing that the monitored voice depends on
AudioRouter. Treble -12/+12 while speaking also produces no perceived
change. Read-only SQLite inspection now shows revision 97, Bass 1.5 dB,
Treble 0 dB: parameter edits have been persisted, but this does not prove
their corresponding audio activation. Next attended check: Treble -12 dB,
wait for save, Stop/Play, then compare; distinguishes live replacement from
the freshly prepared chain. No microphone samples captured or route changed
by the agent. Cause remains unresolved; do not dismiss this as direct
monitoring or low bass energy. User reports no audible change after the
Treble -12 dB Stop/Play comparison. Read-only revision 98 confirms Treble
-12 dB, enabled/unbypassed tool and intact serial microphone connections.
Next attended comparison temporarily bypasses the four downstream
processors (FIR Filter Hz, microphone Advanced EQ, ReaComp, ReaGate) and
compares Treble extremes. User performs edits; the agent does not weaken
their microphone processing or change the native route automatically.

### 2026-09-27 — EQ point selection and visible Undo/Redo

- User requests Band (distinct from Band pass), selection outside the EQ
  graph, and toolbar Undo/Redo with Ctrl+Z/Ctrl+Y. UI-02/05/11, DSP-02.
- Confirm Band terminology with the user while implementing independent
  controls. Add an active-point selector and a small drag threshold.
- Undo exists only in Session and is cleared by Save/live autosave. Keep
  bounded local history through own saves, apply restored drafts against
  the current backend revision, and clear on session switch/external edits.
  Coalesce rapid edits of one numeric setting; preserve ordinary text undo.
- Verify parameter undo/redo before and after Save, shortcut focus handling,
  point selection without mutation, and three-theme toolbar/inspector layout.
  Browser tests need Edge/local backend; no audio/driver/credentials needed
  for this UI slice. Rollback only this slice; preserve prior filter edits.

- Decision (user): label the existing bell filter **Peaking/Band**; retain
  `peaking` internally. Added an active-point dropdown and 5 px drag threshold.
- Implemented toolbar SVG icons and Ctrl+Z/Ctrl+Y (also Ctrl+Shift+Z),
  preservation through own saves, current-revision restore, and 750 ms
  coalescing of edits to the same parameter. History is still bounded to 20
  entries and resets on session changes/external replacements/reload.
- Verification: UI 369 tests pass; typecheck and production UI build pass.
  Real-backend Edge point/history tests pass in all three themes, including
  undo after Save, save after Undo, Redo after Save, numeric-field shortcuts,
  click jitter, and reload reset. Three full screenshots visually reviewed
  at `%TEMP%/audiorouter-designer-review/undo-eq-<theme>.png`.
  Simulated playing-route test passes through autosave and a snapshot refresh
  (`undo-live-parameters.pw.ts`); this is UI evidence, not native audio proof.
- Failed experiments: point selection retained a focused NumberField's typed
  text; remount controls by selected point identity. The first playing-route
  fixture had no processor catalog/controls or connected edges; use a
  connected EQ fixture. Typecheck caught an unsupported Testing Library
  `exact` option; removed it. These checks now pass.
- Package next: rebuild the UI-only change into release-8 with
  custom-protocol, retain matching companion binaries and prior release-7;
  verify embedding and checksum before handing it to the user.

Packaging complete on Windows: `npm.cmd run build` in ui and
`cargo build --manifest-path src-tauri/Cargo.toml --release --features
custom-protocol --locked --target-dir target/patrick-main-release-8` passed.
Latest UI bundle `index-CTvu355D.js` is confirmed embedded; shell timestamp
19:42:11 is newer than UI 19:41:28. Executable:
`C:\code\audiorouter\target\patrick-main-release-8\release\audiorouter-shell.exe`.
SHA256 `A5919C8E0ACE2EA3C95AE04BA83A28701DD20F60C8B5B654BE5F2BFDA0FA3609`.
Unchanged release-7 worker/CLI were copied beside it; backend source is
unchanged in this slice. Previous build folders remain available. No shell
was launched, endpoint opened or user database modified by this task.
Final Edge command: `npx.cmd playwright test --config=playwright.config.ts
undo-eq-points.pw.ts undo-live-parameters.pw.ts` (4 passed), including
shortcuts with focus on a filter dropdown. Documentation acceptance and
`git diff --check` pass. Next: attended release-8 point selection and
parameter Undo/Redo test; existing Band pass live-continuity gate remains
open. Local history does not survive app reload or session switches.

### 2026-09-27 — Build Advanced EQ filter update for attended use

- User requests the release executable and full path (DSP-02/08, UI-05).
- Task: rebuild ui/dist, build shell with custom-protocol in a new
  target/patrick-main-release-7 folder, build the matching plugin worker and
  CLI alongside it, verify filter strings in assets and their embedding,
  record timestamps and SHA256 hashes. Preserve prior build folders.
- Prerequisites: local Windows toolchains; no credentials, driver changes
  or live audio needed for packaging. Prior automated filter checks pass;
  Band pass live continuity remains open as recorded in filter evidence.
- Rollback: use the previous release-6 artifact; no database is changed by
  building. Next: hand off the exact new executable for attended testing.

Shell build succeeded (`cargo build --manifest-path src-tauri/Cargo.toml
--release --features custom-protocol --locked --target-dir
target/patrick-main-release-7`). `npm.cmd run build` in ui passed; bundle
`index-B6qG1ZEp.js` contains both filter labels and its filename is confirmed
inside the shell executable. UI timestamp 19:23:57, shell 19:25:36 local.
Shell path: `C:\code\audiorouter\target\patrick-main-release-7\release\audiorouter-shell.exe`.
SHA256: `479E9345575E9991571D6F41159EB8E5850F6D6B4EB397B40216878470003144`.
This is an unsigned attended-test build; prior live qualification limitations
remain in force. No shell was launched or database modified.

Companion build passed (`cargo build --release --locked -p audiorouter-cli
-p audiorouter-plugin-host --target-dir target/patrick-main-release-7`).
Both binaries are beside the shell. Worker SHA256:
`4640954E2E4EA8BAC8F61A964E9D0221E172D4FEBBFBDE396DD6CA6F238FA58D`;
CLI SHA256: `44FF58EC991F420CBFDE54A74826C5F1738E4418255CB7A4DFB840399AC6EBE6`.
Documentation acceptance and diff whitespace checks passed. Next task:
attended release-7 filter test; Band pass clean-reference continuity rerun
remains an open qualification task.

### 2026-09-27 — Advanced EQ missing Band pass and All pass

- Objective: record the user's missing filters; interpret "Band" as Band
  pass. Requirements: DSP-02/08, UI-05/11; M04 DSP and M05 inspector.
- Reproduction: AdvancedEqEditor FILTERS and domain validation admit only
  peaking, shelves, low/high pass and notch; DSP FilterKind has the same
  six types. This affects the current source and release-6 feature set.
- Specification decision: add constant-0-dB-peak band pass and second-order
  all pass, with shared coefficients, frequency/Q controls and no applicable
  gain/slope. Existing saved values/defaults stay compatible. Implementation
  is pending under the repository's specification-only task instruction.
- Ordered next task: extend registry/domain/contracts and DSP together;
  wire engine parsing and response API; extend the editor choices and help;
  add signal, round-trip and UI regressions; update operational docs.
- Validation matrix: offline magnitude/phase and stability vectors need no
  hardware; contract drift and saved-session tests need local toolchains;
  three-theme UI checks need a browser; live sine-continuity qualification
  needs Windows and the exact VB-Cable endpoints, with a clean reference.
  No credentials or owned driver are required. No implementation checks
  have been run for these new types.
- Risks: a flat all-pass magnitude curve can be mistaken for bypass; gain
  or slope retained from another type must remain inert. Rollback: revert
  this documentation slice; once implemented, preserve/reject new enum
  values explicitly rather than silently substituting peaking.
- Next action: implement the M04/M05 DSP-02 completion when requested.

Implementation authorized by the user on 2026-09-27. Execute the ordered
task above, retain existing enum values and disabled defaults, and verify
DSP, domain/engine/control, contracts and editor before handoff. Revert only
this filter slice for rollback; preserve unrelated working-tree changes.

Implemented: both types in DSP, domain validation, engine parsing, discovery,
response API and TypeScript contracts; editor choices/help, inert gain and
frequency-only dragging; quickstart/library description. Existing sessions
retain their values. Verification and failed experiments are recorded in
[filter evidence](evidence/2026-09-27-advanced-eq-filters.md): Rust checks,
368 UI tests, contract drift, review build, 3 real-backend Edge cases and
six visually reviewed theme screenshots pass. All pass live continuity is
clean for 30 s. Band pass continuity remains open (3 glitches on a clean
reference); unchanged Gain also glitched (2), so do not claim the new filter
is live-qualified. Next task: clean-reference Band pass rerun, then package
for attended use. No desktop release artifact was rebuilt for this slice.

New dated entries go here (objective, requirement IDs, work, verification,
result, next action). Keep them short, and move settled facts into the
sections above.

### 2026-09-26 — Network Send / Network Receive (GRAPH-16, SEC-13)

- Objective (user): a gaming PC sends audio to a streaming PC over IP. One
  send tool whose property is the destination IP. One receive tool (a
  source) whose property is the sender's address.
- Implemented: domain kinds and validation; `network_audio` (wire format,
  pooled sender tap plus I/O thread, receiver thread with sender filter,
  concealment, and a paced jitter buffer with ±1-frame drift correction);
  native-path source and branch wiring; node telemetry; UI library, editor,
  canvas card, and docs (spec, quickstart, privacy, API reference).
- Verification: 4 network unit tests, including loopback tone continuity
  (0 discontinuities in 3 runs), sender filtering, and gap concealment.
  Live CABLE → Network Send → UDP 127.0.0.1 → Network Receive → CABLE-B:
  2 Ã— 30 s, 0 glitches, 11,640 of 11,640 packets. UI: 6 unit tests,
  2 E2E real-backend cases, three-theme bounds and screenshots.
  Workspace Rust, UI unit, E2E (67) and contract drift all pass.
- Also fixed on the way: generated sources (Test Signal, Audio File) were
  not paced (chopped audio). Card status text was unreadable in the light
  theme.
- Rollback: remove the two node kinds and `network_audio`. Saved sessions
  without network nodes are unaffected.
- Next: the attended two-PC test (see Next actions).

### 2026-09-27 — Live Bypass toggles stalled a playing route (GRAPH-05/08, UI-04)

- Report (user): toggling Bypass while playing did nothing; Stop then Play
  was needed.
- Reproduced live on a privacy-muted copy of the saved session, and with the
  continuity harness (`AUDIOROUTER_CONTINUITY_TOGGLE`). The commit was
  applied, but it started a new runtime generation. The multi-path worker
  kept its prepared generation, so the backend audio service and the pump
  rejected it as stale and stopped pumping.
- Fix: the worker serves both the prepared and the live-applied generation,
  and the UI adopts the committed generation. Bypassed plugins are prepared
  in the graph with the bridge passing audio dry. `graph.commit` lets audio
  advance between the durable save and the graph rebuild, so the maximum
  service gap during toggles is 6–8 ms (it was 18.8 ms).
- Verification: all five effects toggled both ways while playing, all
  applied, with pumping continuing on both generations
  (`live_saved_session_bypass_toggles_apply_while_playing`). A Gain at
  −12 dB follows every toggle with zero time jump on both workers. A
  ReaEQ toggle adds no plugin misses. Workspace Rust tests (0 failures),
  UI tests 353, E2E 67/67, and drift checks pass.
- Known: a bypass switch is instantaneous, not crossfaded, so it can click
  slightly on loud material.
- Artifact: `target/patrick-main-release-3/release/audiorouter-shell.exe`,
  built 11:05, bundle `index-Cbk6z3dN.js` embedded. It is a new folder
  because the user's `release-2` shell was running.

### 2026-09-27 — VST2 editor showed a copy that received no audio (PLUG-03/05)

- Report (user): ReaFIR's editor did not pick up audio. After closing it,
  the node reported 18 missing output blocks and 10 dropped input blocks.
- Cause: the worker opened the editor on a *second* plugin instance loaded
  on its UI thread, and copied its state to the processing instance on
  close. The editor therefore never saw audio (ReaFIR's noise profile and
  analyser need it), and edits were not heard until close. Open and close
  also waited for the plugin's window on the worker's audio loop, so blocks
  were missed and dropped meanwhile.
- Fix: `Vst2EditorAccess` gives the UI thread the editor opcodes of the
  processing instance (the VST2 threading model: GUI thread plus audio
  thread). Open and close are requested without waiting for the window, and
  the editor thread closes the editor and is joined (bounded to 2 s) before
  the plugin can unload. The editor acceptance tests were rewritten: the
  editor opens the processing instance without blocking its caller, and the
  worker keeps processing while an editor hangs.
- Verification: `m06-vst2-editor.ps1` passes for all 6 ReaPlugs fixtures;
  the shared-chain acceptance, the pumping-parent editor/state test, and
  workspace Rust tests (0 failures) all pass.
- Artifact: `target/patrick-main-release-4/release/audiorouter-shell.exe`,
  built 12:08. It is a new folder because `release-3` is running.

### 2026-09-27 — UI polish list from attended review (UI-01/04, persistence)

- Report (user): node colors ignored the theme; long Input Device note;
  network tools lacked icons; tools unsorted; "Observed runtime" unclear and
  unpolished; Setup and Devices tabs confusing; no session file to back up or
  move a setup; Advanced EQ status line moved the sidebar; sidebar too narrow;
  number fields could not be retyped or take negative values.
- Done (committed earlier today): themed node cards, shorter notes, unique
  network icons, alphabetical tools, "Live readings" card, fixed-height
  inspector status lines, resizable sidebar, `NumberField` for all precise
  values. E2E: `inspector-stability.pw.ts`, `sidebar-resize.pw.ts`.
- Session files: `sessions.exportFile` / `sessions.importFile` write/read the
  versioned `.audiorouter` bundle, now carrying the imported audio and plugin
  states the session references. Import never replaces a session (used IDs
  become `-imported-N`, name "(imported)"). An existing file is replaced only
  with `replace: true` after the Windows Save dialog confirmed it, staged
  beside it first. The shell adds native Save/Open dialogs
  (`src-tauri/src/session_file_dialog.rs`). Tests: storage
  `session_file_carries_imported_audio_and_plugin_state_to_another_database`,
  control `session_file_export_imports_on_another_database_without_replacing_sessions`,
  E2E `session-file.pw.ts`.
- Setup/Devices decision: Setup is "Set up this PC" (app-wide status, device
  list, other-app guidance, start at sign-in). The Devices tab is removed;
  its controls live in Advanced → Troubleshooting. The connection form stays
  (keyboard access) in Advanced → Connect nodes without dragging. Play reads
  a single route's devices from its nodes first. A generated-only route
  (Test Signal/Audio File → Output) with no chosen input, or saved with its
  output chosen, runs on the multi-path worker, so no input device is needed.
- Verification: vitest 358/358 in `src` (the Playwright file
  `e2e/audio-tools.spec.ts` that vitest also collects fails as before);
  Playwright 88+ pass; control/storage/domain/CLI Rust tests pass; contract
  drift and docs validation pass; three-theme screenshots reviewed.
- Not yet verified live: a generated-only route through the multi-path worker
  on the release shell (covered by the continuity harness `testSignal` mode
  on that worker, but not re-run for this UI change).

### 2026-09-27 — Plugin settings were lost on Stop/Play (PLUG-04)

- Report (user): ReaFIR settings made in its editor survived reopening the
  editor but were gone after Stop and Play; after relaunching, every
  plugin's settings were at defaults.
- Cause: editor edits lived only in the running plugin. They were stored
  only by an explicit Save plugin settings plus Save, which the user never
  had reason to do (no `plugin-states` folder existed). ReaComp and ReaGate
  could not be saved at all: they lack VST2 chunk support
  (`vst2StateSave:StateUnsupported`).
- Fix: the backend records each plugin node's latest captured state
  (`plugin_node_states` table). It captures on editor close, Save plugin
  settings, and before Stop (including tray Quit). Play restores that
  capture first, then the node's `stateId`. Automatic captures
  (`plugin-autostate-*`) replace the node's previous one, so they do not
  accumulate. Duplicated sessions and session files keep them. VST2 plugins
  without chunks save/restore a parameter bank.
- Verification: new live test `live_plugin_settings_survive_stop_and_play`
  (copy of the user database, privacy-muted) passes for ReaFIR, ReaEQ,
  ReaComp and ReaGate. ReaEQ reloads one stored frequency with a last-bit
  rounding difference; the test tolerates two differing bytes. About 15 runs
  left exactly 4 state files. Storage 96, control 196, plugin host, CLI,
  transport and UI 358 tests pass.
- Artifact: `target/patrick-main-release-5/release/` (shell, plugin worker,
  CLI), built 16:28 with `custom-protocol`, embedding UI bundle
  `index-CYp4WBYo.js`. A new folder because `release-4` was running. Rebuilt
  17:09 with FIR Filter Hz, readable labels and name fields (bundle
  `index-Zg_vPOaz.js`).

### 2026-09-27 — Names with spaces, readable settings, FIR Filter Hz (UI-04, GRAPH, DSP)

- Report (user): node names rejected spaces; FIR Filter showed `wetPercent`
  with no explanation; asked for a ReaFIR-like learned per-frequency noise
  gate with a live spectrum, named "FIR Filter Hz".
- Names: every keystroke was trimmed, so a typed space vanished. New
  `TextField` keeps the typed text and stores the trimmed name (node name and
  session rename). Tests: `TextField.test.tsx`.
- Labels: `parameterText.ts` gives every built-in setting a label, a
  one-line explanation and readable choices; Properties shows the tool's
  description and name instead of the raw kind. Accessible names use the
  labels (`Wet mix precise value`).
- FIR Filter Hz: new node kind `spectralGate` (`spectral-gate@1`), 33 kinds,
  17 processors. DSP `SpectralGate` (STFT, 64 log-spaced bands): peak-hold
  learning, per-band gate with `thresholdDb` (−20…20, default 3) and
  `reductionDb` (0…80, default 40), open at once and close over ~20 ms.
  Live levels are published lock-free (`SpectrumTap` atomics), so reading the
  spectrum can never make the audio thread skip a block. Telemetry adds
  `spectrum { levelsDb, bandFrequenciesHz }`; the learned profile reuses
  `noiseProfile`. The control crate's `json!` recursion limit was raised to
  256 for the larger discovery schema.
- Verification: DSP tests (learn, gate noise, pass louder tone, band
  round-trip), engine tool-route test with the new kind, UI editor tests,
  vitest 364, Playwright suite, contract drift (33 kinds, 17 processors),
  three-theme screenshots of the editor with a synthetic spectrum.
- Live continuity (30 s, VB-Cable → CABLE-B): inconclusive this evening.
  FIR Filter Hz 2 glitches per run (4 runs); plain Gain 3–4 per run on the
  current build; this morning's commit `65b7c704` built in a worktree gave 0
  and then 5. The reference tone stayed clean. This is the recorded
  environment-dependent gap pattern, not a regression of this change; rerun
  on a quiet machine before release.

### 2026-09-27 — Stereo tool in a mono plugin chain refused the path (GRAPH)

- Report (user): Play said `path 1 (starting at "Microphone (PD200X)") is not
  supported: a path needs one source or one Mixer, then a single chain, then
  its outputs` after FIR Filter Hz replaced ReaFIR.
- Cause: the voice path is mono (PD200X and the ReaPlugs nodes are 1 ch) but
  library tools are created stereo. The path compiler only lets the width
  change from the source into the first tool, so mic (1) → FIR Filter Hz (2)
  → ReaEQ (1) was refused. Any built-in tool added to that chain would fail.
- Fix (`harmonize_chain_widths`, engine): built-in tools are width-agnostic,
  so in each linear chain they take the chain's width: its plugins' input
  width, otherwise the first tool's. Only edges whose matrix no longer fits
  are rebuilt (mono copied to every channel, averaged into mono, else
  identity). Applied before both compilers; a no-op when nothing differs.
- Verification: engine regression
  `a_stereo_built_in_tool_in_a_mono_plugin_chain_runs_at_the_chain_width`
  reproduces the exact error without the fix. Live: the user's saved session
  (copy) prepares, starts and delivers audio with FIR Filter Hz running and
  ReaEQ → ReaComp → ReaGate sharing one worker
  (`live_native_paths_start_pump_and_report_signal_timing`, now reading the
  session's plugin nodes instead of a fixed list).
- Known, unchanged: the single-output (endpoint worker) compiler refuses a
  mono plugin chain feeding a stereo output even when every node is mono.

### 2026-09-27 — Save flickered "Saving…" every second while playing (UI-04)

- Report (user): after Play, Save showed "Saving…" about once a second, and
  FIR Filter Hz showed a blue line after Stop but no live movement.
- Cause: while playing, a parameter-only difference between the draft and
  the saved route is applied live after 0.4 s. `isParameterOnlyChange`
  compared parameters with `JSON.stringify`, which depends on key order.
  The backend returns parameters with sorted keys, and FIR Filter Hz's
  defaults are not alphabetical, so each 1 s snapshot looked like a change.
  Each re-apply started a new runtime generation, rebuilding the tool and
  resetting its spectrum smoothing. After Stop the editor still drew the
  last telemetry.
- Fix: compare with the existing order-insensitive `sameJsonValue`; draw
  the live line only while the route plays.
- Verification: regression tests in `draft.test.ts` and
  `SpectralGateEditor.test.tsx`; vitest 366, Playwright suite. Live: the
  user's session copy reports `spectrum.levelsDb` for FIR Filter Hz while
  playing (live test now prints it).
- Artifact: `target/patrick-main-release-6/release/` (shell, plugin worker,
  CLI), built 17:30 with `custom-protocol`, embedding UI bundle
  `index-BUK9DptH.js`. A new folder because `release-5` was running. Rebuilt
  17:53 with the empty-profile fix (bundle `index-DTGlFEFV.js`).

### 2026-09-27 — Canvas lines reported gone; empty FIR Filter Hz profile (UI, DSP)

- Report (user): lines Microphone → FIR Filter Hz → ReaEQ and Siege game →
  Siege Advanced EQ were gone and could not be dragged back.
- Findings: the saved session (revision 74) still has all 8 connections.
  With the user's exact session, saved canvas layout and live telemetry
  recorded from their route (`AUDIOROUTER_LIVE_DIAGNOSTICS_DUMP`), every line
  renders stopped and playing, and a removed line drags back, in the current
  build. The client log shows ~230 `UI error (unknown)` entries at 5:24 PM on
  the previous build, while the live-save loop rebuilt the route every
  second; the 5:35 PM run of the fixed build logged none. Not reproduced on
  the fixed build; awaiting the user's confirmation.
- Found meanwhile: the stored FIR Filter Hz profile was all zeros. The
  spectrum tap offered a "learned profile" before any frame was analysed,
  so an early Stop and keep stored silence. Fix: no profile until a frame is
  analysed; an all-floor profile counts as none (gate passes; UI offers
  Learn noise).
- Tests: route harness accepts an injected session, telemetry and running
  state (test hooks); `e2e/playing-canvas-lines.pw.ts` (lines drawn while
  playing, removed line dragged back); DSP and editor regressions. vitest
  366, Playwright 92, DSP 51, engine 139, control 196.
