# Issue #136 - VST experience investigation and execution plan

Status: planning only. This task did not reproduce or fix a reported defect.
Branch `feature/vst-plugin-experience` is isolated from `fix/resilience` and
based on `origin/main` at `7d70d502958b86d307cce4e4be847207e6725c19`.
Source: [GitHub issue #136](https://github.com/MrDesjardins/audiorouter/issues/136)
and its comments.

## Objective

Investigate the plugin experience in #136, then improve it as a coherent user
workflow: configure a supported effect while a session is stopped; reliably
open, resize, and close its native editor; insert effects in series without
changing route meaning; and use a real sidechain only when the plugin format
and bus layout support one. Preserve backend graph authority, worker
containment, explicit mixing, authorization, and realtime failure policies.

## User reports and unknowns

| ID | Report | Evidence status / reproduction needs |
| --- | --- | --- |
| I136-R1 | The issue started as "Can't use ANY VST plugin"; after trying v0.0.29, the reporter said it "kinda works." | The broad report and later qualification are confirmed as user reports, not compatibility findings. No exact plugin inventory, binary, format, architecture, build, OS details, or support bundle is attached. |
| I136-R2 | The native editor opens at the wrong size; closing it or changing controls can sometimes crash the plugin. | Confirmed report and screenshot. Screenshot is symptom evidence only. Plugin identity, expected view size, Windows build, DPI/scaling, monitor, action sequence, and crash signature are unknown. Not independently reproduced. |
| I136-R3 | Connecting one plugin output to another appeared to add a Mixer and sum at the second plugin, preventing the intended sidechain/Duck setup. | Confirmed report, cause unknown. Need source graph, exact gesture/direction, existing edges/input occupancy, intended key source, plugin identity/format/buses, and resulting graph. The UI-03 Mixer proposal is a hypothesis, not a proven cause. |
| I136-R4 | Wants to open/configure the plugin UI before Play; wants a straightforward sound-card mixer with VST effects and sidechain. | Confirmed goal. Stopped editor lifecycle, bus support, and headless/API boundaries need verification before promising behavior. |
| I136-R5 | The reporter has returned to Voicemeeter plus Elka.VoicemeeterFxHost and compared Wave Link. | Context only. Do not infer their behavior/compatibility model or copy their UI/assets. |

The issue comments show the maintainer requested plugin names and a Logs and
support bundle, offered v0.0.29, and acknowledged the later editor, routing,
and stopped-setup feedback. No support bundle or reporter database is available
in this task. Further diagnostics must use the user-led support path; do not
assume access to the reporter's machine.

Record for each attempt: app release/build/hash; Windows edition/build; plugin
name, format, architecture, vendor/version or file size and SHA-256 if no
version resource exists; scan/class identity; monitor resolution, Windows
scale, DPI mode and active monitor; exact action sequence; graph before/after;
time and bounded redacted log/support-bundle outcome. Keep third-party plugin
binaries user-owned and opt-in. Label results `reproduced`, `not reproduced`,
`blocked`, or `not run`.

## Traceability

- Product/workflows: PROD-01/02/03/04/07/08; UC-02, UC-06, UC-07, UC-10.
- Graph/DSP/plugins: GRAPH-01/02/03/06/08/09/10/12/14/15; DSP-19; PLUG-01-07.
- UI/API/automation/state/security: UI-02/03/04/05/06/07/08/09/11/12/13/17/20;
  API-01/04/05/06/07/09/11/12; AUTO-01/03/04/05/06/07/09/10/12/15/18;
  STATE-01/02/03/05/06/07; SEC-05/07/10/12.
- Architecture/quality/delivery: ARCH-01/03/04/06/09/10/11;
  NFR-03/04/05/06/08/11/12; QUAL-01/02/04/05; ENG-01-05; M05/M06/M07/M08.
- `I136-*` scenarios below add issue-specific acceptance coverage without
  changing stable workflow/spec IDs in this planning task.

## Current contract and decisions before implementation

Treat these as constraints unless the owner approves a synchronized spec change:

1. **Formats.** Investigate native x64 VST3 audio effects for auxiliary-bus
   support. VST2 is only the explicitly approved, gated PLUG-07 native x64
   audio-effect extension. Its production `processReplacing` contract is
   single-stream: do not label it sidechain-capable or merge an auxiliary
   signal into that stream. x86 bridging, instruments/MIDI, Audio Units,
   downloads, and redistribution stay out of scope.
2. **Serial insertion vs. summing.** An explicit "Insert processor on this
   connection" transforms A -> B into A -> P -> B in one previewable,
   undoable transaction and creates no Mixer. Summation requires an actual,
   named, visible Mixer with explicit input edges. Any UI-03 proposal to
   create a Mixer must be shown and accepted as a visible transaction, never
   inferred from a serial insertion gesture.
3. **Sidechain.** Only a VST3 effect whose inspected/activated layout declares
   an auxiliary input may expose a separately named auxiliary port (for
   example, "Side-chain"). The main input/output remains serial. A separate
   graph source connects to the auxiliary port; it is not a Mixer input and
   must not be summed into the program signal. Reject undeclared sidechain or
   unsupported format/layout with actionable feedback and no graph mutation.
4. **Stopped setup.** Permit adding/selecting a supported plugin, editing typed
   generic parameters, opening a supported desktop editor, and saving state
   while stopped without starting audio, preparing/opening endpoints, or
   requesting device-administration consent. Headless clients can use
   supported discovery/configuration/state APIs but have no desktop HWND and
   cannot open native windows. Preserve existing grants; do not widen MCP or
   HTTP rights to satisfy HWND ownership.
5. **Editor lifecycle.** Editor open/close is independent of graph processing.
   Verify the plugin-required thread, message pumping, and authorized parent
   HWND. Respect plugin-reported view size and Windows/DPI scaling. Do not
   infer a universal target size from the screenshot.
6. **Duck.** Test DSP-19 separately as a control-only key-node tool. Its level
   trigger is not evidence of VST auxiliary-bus support.

Decisions required before source work:

- Obtain or explicitly proceed without the reporter's plugin/build/OS-scaling
  details and optional support bundle. Any installed-plugin repro must be an
  opt-in local selection; otherwise use repository fixtures and leave the
  reporter-specific gate blocked.
- Confirm whether UI-03's occupied-input Mixer proposal remains acceptable
  when visibly previewed, while processor insertion is strictly serial; or
  require manual Mixer creation for summation.
- Confirm stopped-session native-editor lifecycle and whether
  `plugins.openEditor`/`plugins.closeEditor` API inputs/auth can support it.
  Define any new scope and MCP tool/exclusion before changing code.
- Define VST3 graph-facing bus identities/labels and saved-layout compatibility;
  require exact bus negotiation. Existing worker/scheduler plumbing alone is
  not end-to-end graph/runtime sidechain evidence.
- Keep PLUG-01/07 boundaries and SEC-07 limits. Compatibility, sandbox,
  auto-restart, or rights expansion needs a separate explicit decision.

### Decision update (owner, 2026-11-14)

- **Visible occupied-input Mixer proposal accepted.** Keep the existing
  previewable, undoable Mixer for explicit summation; serial VST insertion is
  the separate "Insert processor" operation.
- **VST2 sidechain is in scope only with explicit pin labels.** VST2 has no
  VST3-style named auxiliary bus. Expose a sidechain only when the plugin's
  input-pin metadata explicitly labels that pin as sidechain/key; never infer it
  from a surround or extra-input count. VST3 auxiliary buses follow the named
  bus contract.
- These decisions resolve the earlier open questions but do not by themselves
  implement the graph/runtime auxiliary path; the implementation and native
  editor evidence below remain required.

If decisions alter requirements, update affected specs, generated contracts,
catalogs, tests and this plan together before implementation. Likely sections:
`docs/spec/07-processing.md` (PLUG-03/04/07, DSP-19, multi-bus status),
`docs/spec/04-graph.md` (GRAPH-01/02/06/09), `docs/spec/09-interface.md`
(UI-03/05/07/13), `docs/spec/10-api.md`, `docs/spec/11-automation.md`,
`docs/spec/12-persistence.md`, `docs/spec/13-security.md`,
`docs/spec/14-quality.md`, and, if a stable workflow is added,
`docs/spec/02-workflows.md` and `docs/spec/15-delivery.md`. In particular,
reconcile UI-03's automatic Mixer proposal and PLUG-03's older single-stream
wording with current multi-bus status and the selected user intent. No spec
changes are part of this planning task.

## Workflows and acceptance scenarios

| Scenario | Required outcome |
| --- | --- |
| I136-SETUP | In a stopped session, select/add a supported plugin, edit a typed parameter, and save. Close/reopen editor, verify saved state, then Play and verify restored value. No implicit start, endpoint preparation/open, capture stream, or Windows-default change. Unsupported plugins remain explicit. |
| I136-SERIAL | For A -> B, use "Insert processor" on that edge. Preview/commit A -> P -> B; replace the original edge exactly once, create no Mixer, sum no extra source, and preserve exact topology through undo/redo and save/reload. |
| I136-SUM | Sum A and C only through a visibly named Mixer M with explicit A -> M and C -> M edges, then M -> P main -> B. Both sources contribute only at M; no hidden ordinary plugin-input mix. |
| I136-AUX | For an auxiliary-capable VST3, retain A -> P main -> B and connect key K separately to P's named auxiliary input. Deterministic main/key signals prove K changes P's processed main output without K being directly summed/audible at B. Save/reload and undo/redo preserve named port and edges. |
| I136-UNSUPPORTED | Sidechain attempt on a non-sidechain VST3, VST2 single-stream plugin, or unsupported layout is rejected with plugin/port and remedy. Revision, graph, state, running generation and edges are unchanged. Never flatten buses. |
| I136-EDITOR | Open, interact, resize and close a real native editor repeatedly; verify geometry, scaling and parameter/state sync. Inject editor/worker crash and hang. Contain/reap only the failing worker; unrelated workers, control plane, graph authority and independent audio remain usable. |
| I136-HEADLESS | Through versioned API/CLI, discover/add/configure/save a plugin in a stopped session. Verify no endpoint preparation or start. Native-window calls fail explicitly without an HWND. Verify allowed and denied grants through the real dispatcher. |

## Ordered implementation and verification gates

1. **Reproduce and baseline.** Record exact branch/head. Request/receive
   optional support bundle and missing inventory through the user; inspect
   redacted JSONL and a session copy read-only. Record release, Windows build,
   DPI, plugin identity and action sequence. Reproduce R2/R3/R4 separately
   before source edits. If details are unavailable, use opt-in fixtures for
   platform coverage without claiming reporter reproduction.
2. **Trace contracts and decisions.** Trace editor methods/lifecycle,
   stopped-state parameters/state, persistence, graph insertion/connection
   resolution, bus metadata, compiler/runtime scheduling and UI/REST/MCP entry
   points. Resolve decisions above. Map each source change to a named
   fail-before/fix-after regression. Record approved spec changes first.
3. **Native editor.** With repository fixtures and opt-in installed x64
   VST2/VST3 effects, test initial bounds, plugin-reported view size,
   monitor/DPI transitions, resize negotiation, close/reopen, repeated
   attach/detach, parent HWND/PID authorization and message pumping. Include
   control interaction and state capture while audio processes. Inject
   crash/hang at open, resize, interaction and close; prove bounded cleanup
   and unrelated worker/control/audio survival. Do not infer expected size
   from the screenshot.
4. **Stopped setup.** Add/select, edit generic parameters, open the desktop
   editor, save, close/reopen, reload stopped session and inspect authoritative
   values; then Play and verify restoration. Add API/auth/storage/dispatcher
   tests as needed. Assert stopped before/after, no
   `nativeEndpoints.prepare`/`nativePaths.prepare`, no endpoint opened, and no
   implicit permission/default changes. Exercise headless setup separately.
5. **Topology and buses.** Test serial insertion, explicit Mixer summing and
   auxiliary wiring independently. Validate full graph candidates atomically
   through planner/compiler. Bind plugin fingerprint and exact sidechain layout
   in Hello/Ready. Require complete coherent main+aux quanta (frame count,
   sequence, deadline, declared layout) through bounded scheduler/transport.
   Add end-to-end graph ownership and production runtime coverage, not only
   protocol/echo/offline-loader probes. Fail closed on missing/malformed/late/
   mismatched buses under GRAPH-14. Auxiliary audio never reaches main output
   except over a separately explicit Mixer edge.
6. **API/UI parity.** For any public methods/parameters/ports/kinds, update
   Rust `API_METHODS` schemas, contracts, REST/OpenAPI, `node_parameter_schema`,
   default MCP tool or `MCP_EXCLUDED_METHODS` reason, recipes/role advice and
   user/API docs together. Exercise `ControlPlane::dispatch`, `mcp_tool_call`
   and the API adapter. Add UI unit and headed Edge workflows in dark/light/
   high-contrast; verify keyboard, actionable errors, stable layout, save/reload
   and undo. Refresh guide/video/property coverage if visible controls/behavior
   change.
7. **Acceptance/handoff.** Run affected Rust tests; engine source/tool-combination
   suites if classification/kinds change; worker/IPC; control/API/MCP/contracts;
   UI unit, format/lint/typecheck/build and Edge; docs validation; both Rust
   format checks and strict Clippy; full diff inspection and `git diff --check`.
   On Windows, exercise native editor/plugin acceptance with isolated
   database/log/WebView profiles and no other shell. Record blocked native,
   hosted, hardware and reporter-specific gates. No PR/release claim follows.

## Test and acceptance matrix

| Layer / environment | Concrete coverage | Pass condition / evidence boundary |
| --- | --- | --- |
| Domain and graph compiler (portable Rust) | Candidate validation for insert edge A -> B, explicit Mixer sum, named aux port, occupied ordinary input, unsupported sidechain, cycles, matrices, flags and rollback. | Serial insertion yields only A -> P -> B; Mixer exists only in accepted visible candidate; unsupported aux has no revision/topology mutation; error names plugin/port. Unit tests do not prove worker or Windows editor behavior. |
| Engine and signal correctness (portable Rust) | Production compiler/runtime graph with fixed-seed main/key tones, silence, phase changes, late/missing/malformed aux, bypass and processor fault. Exercise scheduler coherence, samples, sequences, deadlines, branch cache and failure silence. | Key changes effect response; key-only component is not summed to main; no NaN/Inf or stale frames; only coherent buses; protected output fails closed; independent output/worker remains live. Save sample comparisons, frames, counters and graph revision. |
| Plugin worker and IPC (fixtures plus portable protocol) | `cargo test -p audiorouter-plugin-host`; crash/hang/state fixtures; encode/decode bounds, Hello/Ready identity+bus layout, coherent `ProcessBuses`, timeout/reap. Use repository AGain main/sidechain VST3 and VST2 single-stream fixtures. | No flattening, partial publication, callback wait or unbounded wait. Protocol/echo tests are not native plugin execution or production graph evidence. |
| Native editor window (Windows 11 x64) | Opt-in repository/installed x64 VST2/VST3 fixtures and actual window in authenticated parent. Record plugin-reported/observed rect, scaling (100/125/150/200% where available), resolution, monitor, DPI mode; move monitors if available; resize, reopen/close, interact, save/compare state. Pump parent messages while fixture audio flows. Inject hang/crash at open/resize/interaction/close. | Geometry follows plugin constraints, fits parent/work area without clipping/wrong scale; editor does not restart processing; cleanup bounded; HWND/PID correct; independent worker/control/audio healthy. Say `not run` if Windows/fixture unavailable. Screenshot, mock, or timeout containment alone is not a real-window pass. |
| Stopped session and persistence | Add/select; set generic parameter; save graph/state; open/interact editor; close/reopen; reload stopped session; verify parameter/opaque state; Play and query applied value. Repeat supported setup through headless API/CLI. Test unsupported/denied cases. | Saved value survives and is used on Play; stopped until deliberate Play; no endpoint open/preparation, device grant, unrelated topology edit or permission widening. Headless native-window limitation is explicit. |
| Control plane and adapters (Windows Rust) | `cargo test -p audiorouter-control`; exercise changed methods through `ControlPlane::dispatch`, editor lifecycle, graph methods, exact auth grant/denial, stopped state, storage and outcomes. For additions, exercise REST/OpenAPI and default MCP or documented exclusion. | Backend authoritative; no adapter bypass. If methods change, run `npm --prefix contracts run check:drift` and API-method reachability test. Default MCP grants unchanged. |
| UI component and Edge (Windows) | Plugin inspector/picker, insert edge, Mixer preview, sidechain connect, real request payloads through backend harness, keyboard, saved-state feedback and three themes. Include actual repeated editor and stopped flow in native packaged acceptance; browser fixtures cannot qualify HWND. | UI matches authoritative plan; actionable errors; no unintended Mixer; no layout shift; themes legible; interaction/save/reload works. Screenshots prove UI only. |
| Combination and repository parity | `cargo test -p audiorouter-engine --test source_combinations` and `--test tool_combinations`; exhaustive classification/combination coverage for any added kind. Run complete affected suites and strict Clippy/format. | All source/tool combinations and flag/layout patterns compile. Run when kind/classification changes; no handwritten support list. |
| Windows real entry point / quality | Rebuild e2e backend before real-backend Edge; exact REST/MCP/dispatcher calls; matching native shell/worker; M06 VST3 worker and opt-in editor/sidechain acceptance. Apply QUAL-01/02/05; run sine-continuity if production audio bridge/graph/callback changes. | Distinguish fixture, simulated, offline worker, native window and hardware evidence. Screenshot/mock cannot claim native editor, routing isolation or physical latency. Live gates need a safe isolated/attended setup. |
| Hosted/platform/hardware | Current-head Windows/Edge CI and annotations; supported Windows 11 builds. Host runners lack reporter plugin, DPI setup and physical topology. | Missing reporter/plugin/OS evidence remains blocked. Hosted builds do not imply reporter reproduction or soundcard success. Hardware latency/soak and release/install/security remain separate. |

Existing anchors: `supervised_vst2_worker_survives_its_editor_window_being_built`,
`native_vst3_editor_opens_in_a_pumping_parent`, editor dispatcher tests,
`RuntimeBusScheduler`/`ProcessBuses`, `crates/engine/tests/source_combinations.rs`,
`crates/engine/tests/tool_combinations.rs`, and
`a_stereo_built_in_tool_in_a_mono_plugin_chain_runs_at_the_chain_width`.
Add focused regressions for confirmed defects; nearby coverage does not prove
the issue's cause.

## Prerequisites

- Selected Windows worktree is `feature/vst-plugin-experience` from current
  `origin/main`; preserve other worktrees and do not touch/integrate/copy
  `fix/resilience` changes.
- Before code: owner decisions above; pinned Rust 1.96.0 and Node 22.22.3;
  Windows MSVC/SDK, Edge, native worker build, UI dependencies and local VST3
  SDK checkout.
- Use repository AGain sidechain and crash/hang fixtures first. Installed
  plugins require opt-in selection and before/after fingerprint checks. Do
  not download/install/register a third-party plugin for missing evidence.
- Native editor acceptance needs Windows, scaling-aware display, available
  repository or selected installed x64 fixtures, and no conflicting shell.
  Stopped setup/private deterministic signals need no hardware; live soundcard
  or loopback tests need an explicitly safe isolated setup.
- Synchronize REST/MCP/contracts/API docs for any public contract change.
  Refresh user guide and guide-video coverage with user-visible control or
  behavior changes.

## Failure containment, privacy, and realtime safety

Plugin code stays outside the backend in bounded workers/jobs with no inherited
backend credentials/handles. Revalidate parent HWND ownership/authorization;
pump editor messages; close/hang deadlines end only the owned child. Other
workers, control plane and independent paths remain usable. Audio callback never
waits for UI/plugin/IPC/disk/control locks. Multi-bus data is preallocated,
bounded and published only as a complete coherent quantum. Missing/invalid aux
data follows declared failure policy, never main-bus substitution or flattening.
Failed protected voice path remains silent until deliberate recovery; no
automatic dry fallback/restart loop. Opening a stopped session does not scan or
load a plugin.

Use disposable session/database copies and isolated logs/WebView profiles. Do
not log raw plugin paths, parameter values, credentials, HWND capabilities,
audio samples or support-bundle contents; keep bounded stable error categories
and operation IDs. Check device state before/after attended tests; do not change
Windows defaults, install drivers, or interrupt a user's route. Plugin license
prompts/editor child processes are untrusted UI, not grounds to widen scope or
sandbox limits.

## Risks and rollback

| Risk | Mitigation / rollback |
| --- | --- |
| Editor geometry depends on plugin, OS, monitor and DPI; a generic size fix may crop valid views. | Record exact platform/view dimensions; avoid a universal pixel assertion. Roll back geometry/lifecycle changes while retaining isolation. |
| Crash only occurs for one plugin/control or after repeated cycles. | Preserve fingerprint/action sequence; use opt-in installed cases plus deterministic faults; do not generalize from one pass. Revert focused fix if other plugins regress. |
| Gesture conflates serial insertion, summing and aux routing. | Separate typed operations; preview canonical diff and check saved graph. Roll back insertion/port mapping if any intent creates unrequested Mixer/topology. |
| Multi-bus protocol exists without graph/runtime ownership or callback qualification. | Require compiler-to-worker-to-scheduler-to-engine gate; label staging/offline probes partial. Revert graph/bus activation together; never flatten to single stream. |
| Persisted port/layout fields break older builds/history. | Decide versioning/migration/downgrade before adding fields; add before/after fixtures and backup plan. No migration is assumed here. |
| API/editor change widens plugin permission or default MCP reachability. | Retain grants, desktop parent rule and backend auth. Add no method without contract/dispatcher/parity tests. Roll back method/schema/client changes together. |
| Test affects real mic/device/default/live route/private audio. | Prefer offline deterministic fixtures and stopped isolated shell. Guard/document live tests; preserve mute/exact bindings; clean only owned resources. |

Implementation rollback is a scoped revert of approved code/contracts/spec/UI;
preserve prior executable, saved database, plugin-state assets and unrelated
branches. Any persisted schema/port change needs an explicit compatibility and
backup plan. Planning-only rollback reverts this plan and its index entry.

## Evidence format and next action

For every `I136-*` row record requirement/scenario IDs; branch/source commit;
date; exact app binary/hash; Windows build, display/scaling, driver and plugin
identity/fingerprint; graph revision and user action sequence; commands;
expected/observed graph diff, window bounds, parameter/state values,
frames/deadlines and signal measurements; artifact paths; result status
(`reproduced`, `not reproduced`, `blocked`, `not run`); conclusion/limits.
Redact usernames, full paths, IDs/capabilities and audio. Keep raw numeric
artifacts only in ignored local `target` or approved private evidence. Separate
hosted/hardware results from portable/unit/UI-mock results.

**Next action:** owner supplies or explicitly declines the reporter's plugin,
build, OS/scaling details and optional sanitized support bundle. Reproduce
R2/R3/R4 separately on an isolated current-main build, or record the
reporter-specific gate as blocked; finish contract/API/topology trace and
resolve the decisions above. Only then propose an authorized implementation
task with approved spec changes and fail-before/fix-after regressions. This
planning task authorizes no implementation, commit, push, PR, release, or
issue reply.

## Continued verification (2026-11-15)

Owner authorized continuing and installing/running the required tools. CMake
4.4.4 was installed with `winget` (user scope). The pinned SDK checkout is at
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`. The repository acceptance wrapper
`tests/acceptance/m06-vst3-sdk.ps1` could not start because it requires the
ignored repository-local `third_party/cmake-4.4.0/bin/cmake.exe`; its configure
and build commands were therefore run using installed CMake 4.4.4 with the
same Visual Studio 18 2026 x64 generator and targets. `mda-vst3` and `again`
built. The Steinberg validator passed on both bundles, reporting 94 tests
passed and 0 failed for each. This is offline SDK/fixture evidence, not
AudioRouter graph/runtime or reporter-plugin evidence.

Rebuilt `audiorouter-vst3-worker.exe` from the branch and ran
`add_sidechain_port_adds_the_real_again_auxiliary_input_once_and_undoes`
through `ControlPlane::dispatch` against the built AGain bundle: passed. The
first run found the test expected a two-channel auxiliary input; discovery
showed AGain's main bus is stereo and its auxiliary bus is mono. Corrected the
test to assert the actual declared mono bus, then reran successfully. The
test proves port discovery/binding and graph edit behavior only; it does not
prove production engine scheduling or audible ducking.

The release desktop build completed via
`tools/build-desktop.ps1 -Profile release` on Windows. The fresh-install
`a_fresh_install_keeps_device_and_folder_consent_exclusive_to_its_window`
test passed against `src-tauri/target/release/audiorouter-shell.exe` with a
temporary database and WebView profile: backend first response 77 ms; raw
pipe device/recording-consent requests were denied. No audio endpoint was
opened. The unsigned installer has not been built: the release preparer
requires a clean Git tree, while the focused mono-bus regression and this plan
evidence are uncommitted. The visible Sidechain label passed headed Playwright
in dark, light and high-contrast, including screenshot inspection. UI format,
lint, 1,072 tests, typecheck and production build pass; control Clippy, 405
control tests, both engine combination suites, contract drift, docs links and
default-MCP API reachability pass. `check-tool-guide-catalog.mjs` passes; the
property-coverage check still reports the previously pending
`plugin.recovery` recording. The release build still emits the main-chunk
warning (631 kB above the 500 kB threshold); the Sidechain change does not
materially affect the bundle. Remaining local tasks: Sidechain guide and
manifest/video coverage, occupied-port replace/reject flow, VST3 unit
hierarchy/context-menu, installer packaging, and a final full diff audit.
Owner/hosted gates remain real editor focus/DPI and pumped-parent, live-ear,
hardware MIDI, PolyMAX sign-in, hosted i686 CI/annotations, signed package,
and approval to push/open a PR. See `vst-handoff.md` for exact next action.

## Local completion continuation (2026-11-15)

The guide/property coverage and occupied-sidechain replace/keep interaction are
now implemented and validated. Replacing a connected key is one draft edit and
never proposes a Mixer. The VST3 inspector now displays bounded unit hierarchy
and per-unit read-only program names; `IComponentHandler3::createContextMenu`
returns no menu and is covered by the worker self-test. UI, protocol, REST
output schema, contracts and default MCP description agree. Real effects on
this PC reported no units, so a real unit-bearing plugin path still needs
qualification.

An unsigned app-only NSIS installer was built, installed in a temporary
directory, checked for all four workers, exercised through `fresh_install_shell`
and uninstalled. The app binaries are removed; a temporary profile directory
remains at `%TEMP%\audiorouter-nsis-smoke-issue136`. This does not satisfy
signing, driver package, hosted CI, real-endpoint audio or attended editor
gates. Current commands and outcomes are recorded at the end of
`vst-completion-work.md`.
