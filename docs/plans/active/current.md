# Active plan — VB-Cable-first completion

Status: active, rebaselined 2026-09-17.

## Objective

Complete the non-driver AudioRouter scope using existing VB-Cable,
Voicemeeter, physical WASAPI, and other installed virtual endpoints. The
AudioRouter-owned kernel driver, PortCls activation, production signing, and
clean-machine driver qualification are deferred; they remain documented
requirements but are not active completion gates for this plan.

## Requirement coverage

The stable specifications and milestone contracts remain authoritative. This
plan covers the non-driver portions of PROD, ARCH, GRAPH, CAP, DSP, REC, PLUG,
UI, API, AUTO, STATE, SEC, NFR, QUAL, and ENG requirements mapped in
[M08 delivery traceability](../../spec/15-delivery.md#requirement-traceability).
Managed virtual-device requirements and SEC-08 are deferred in
[the driver/signing plan](../future/M03-driver-signing.md).

## Current evidence

- Backend graph ownership, validation, persistence, authorization, routing, and
  bridge lease/generation contracts are covered by the locked Rust suites and
  guarded VB-Cable/physical endpoint runs. Privacy coverage is split: the
  authorized durable control contract, allocation-free realtime mute primitive,
  and live native-worker latch propagation are tested. The control latch now
  propagates into attached native endpoint, process-loopback, multi-input,
  render-source, and duplex workers. The latest guarded user-mode
  dispatch-to-processed-block sample measured 16.734 ms p95; calibrated
  effective-output timing and attended microphone-path measurement remain open.
- Existing VB-Cable routing has live multi-input/many-output evidence,
  application capture, physical output, recorder taps, Voicemeeter tool
  integration, and cleanup without persistent audio configuration changes.
  The 2026-09-17 refresh fixed bounded multi-input packet backpressure and
  requalified the exact CABLE plus Focusrite capture / Voicemeeter In 2 plus
  In 5 render route with 47,520 captured frames, 256 delivered quanta, and
  47,232 rendered frames; the feeder now retains unread packet slices across
  pumps.
  Corrected current guarded application capture passes both include and
  exclude modes for Voicemeeter and Zoom against exact process identities and
  the existing VB-Cable render endpoint, including same-process worker
  restart. An earlier retry supplied the full executable path in the adapter's
  basename field and correctly failed closed as an identity mismatch; the
  corrected basename/path split now qualifies both applications without
  fallback or persistent audio configuration changes.
  The exact Zoom application-capture acceptance was rerun elevated in both
  include and exclude modes after the capture polling compatibility retry;
  both modes passed with unchanged media-device identity/state.
- M04 DSP/recording and built-in pitch, M05 drag-and-drop editor, M06 local
  VST3/VST2 worker boundaries, and M07 CLI/MCP/headless/recovery foundations
  are implemented and tested. M04 pitch/recording, M06 fixture, M07 shell RPC,
  and M07 headless requalifications were refreshed on 2026-09-17.
- The 2026-09-17 VB-Cable digital impulse run detected 97/100 groups with
  zero p95 spacing error and a 76.56 ms estimated digital onset. This is not
  calibrated physical or acoustic latency evidence.
- Documentation validation covers 56 Markdown files and 224 local links;
  traceability covers 159 normative requirement IDs.

The post-privacy-propagation acceptance refresh on 2026-09-17 passed M07
headless (CLI 36, MCP 3, control 177 plus 3 guarded-live ignored,
plugin-host 70, worker-process 13, M01 CLI, and strict Rust checks), M05 UI
(typecheck, 257 tests, temporary production build), and contract drift (86
methods, 20 node kinds, 7 processors, 20 event categories). No device or
persistent machine configuration was changed by these checks.
The full workspace `cargo clippy --workspace --locked --all-targets
--all-features -- -D warnings` also passed after the privacy propagation
changes.
A subsequent `cargo test --workspace --locked` refresh passed across CLI/MCP,
control (177 passed, 3 guarded live ignored), domain (65), DSP (34), engine
(116), plugin-host (70), worker-process (13), protocol (8), recording (40),
storage (92), transport (19), Windows audio (88), and all doc-tests.
The release preparation command was also invoked against a new workspace-local
output path and stopped before any build or artifact creation with its
authoritative clean-tree error: `release inputs must come from a clean Git
working tree`. No release output directory was created.

The post-rebaseline full locked workspace run passed on 2026-09-17: CLI 36,
MCP stdio 3, control 177 with 3 guarded live tests ignored, domain 65, DSP
34, engine 116, plugin-host 70, storage 92, transport 19, Windows audio 88,
all doc-tests, and the remaining workspace suites. This is portable regression
evidence; it does not close attended UI, physical hardware, sandbox, clean
release, or deferred driver gates.

The latest current-tree refresh also passed `cargo test --workspace --locked`
and `cargo clippy --workspace --locked --all-targets --all-features -- -D
warnings`. The workspace counts were CLI 36, MCP stdio 3, control 177 plus 3
guarded-live ignored, domain 65, DSP 34, engine 116, plugin-host 70,
worker-process 13, protocol 8, recording 40, storage 92, transport 19, and
Windows-audio 88, with all doc-tests passing. This remains portable regression
evidence; it does not close attended, hardware, sandbox, clean-release, or
deferred driver gates.

The disposable elevated M08 NSIS smoke also passed on the current tree,
producing and removing a 4,888,262-byte x64 unsigned bundle while preserving
the source manifest. It did not execute or install the bundle; clean-tree
artifact preparation and production release gates remain blocked as documented.

The current M03 virtual-bus CLI acceptance also passed bounded desired-state
and persistence lifecycle, eight-bus capacity, cross-session route
validation, and managed-driver-unavailable reporting. Its temporary database
was removed; no native device or machine audio configuration was touched.

An earlier elevated M00 VB-Cable loopback also passed with 213,044 nonzero
captured payload bytes from a 1,000 ms capture while a 1,500 ms tone rendered
through the exact CABLE pair. Temporary probe resources were removed and no
machine audio state changed. This remains digital signal-path evidence, not
physical or acoustic latency evidence.

The current elevated event-driven M00 acceptance also passed on the exact
CABLE pair with 24,480 captured frames and 28,320 submitted render frames over
500 ms. Event registration and full stream lifecycle cleanup passed without
changing machine audio state.

The 2026-09-17 Rust adapter-bridge retry initially returned
`adapter_bridge_error=audio frame size was invalid` before telemetry because
the probe's current active inventory did not contain the supplied DELL/P32p-30
physical render IDs. The exact current CABLE Output/CABLE Input pair then
passed two bounded cycles with 24,480/24,960 captured frames, 191/195
processed quanta, 24,448/24,960 rendered frames, zero drops/xruns/deadline
misses, and finalized temporary recordings. Physical-monitor bridge selection
remains an endpoint-inventory limitation, not a new routing pass claim.

A later 2026-09-17 direct recheck found the same CABLE endpoint IDs still
enumerated, but `adapter-bridge` now fails during capture polling
initialization with Windows HRESULT `0x80070057` before any frames or
processing telemetry. That probe was non-elevated; it is not evidence of an
endpoint-format regression. The same endpoint pair succeeds when the bounded
probe is run from the documented elevated `deviceAdministration` context.
A same-day elevated rerun of the raw Rust initialization and full adapter
bridge succeeded: 4,800 captured frames, 37 processed quanta, 4,736 rendered
frames, zero drops/xruns/deadline misses, and a 25,072-byte finalized
temporary recording. The earlier failure was the non-elevated execution
context; guarded native adapter qualification requires the documented
`deviceAdministration` elevation boundary. The capture adapter's bounded
`E_INVALIDARG` duration retry remains in place, but is not treated as the
reason for this successful elevated run.
The authoritative elevated two-cycle acceptance then passed on the same exact
CABLE pair: each 500 ms cycle produced 51 packets, 24,480 captured frames,
191 processed quanta, 24,448 rendered frames, zero drops/xruns/deadline
misses, and a 25,072-byte finalized recording, with temporary resources and
media-device state cleaned unchanged.
An earlier bounded five-cycle endurance rerun passed with identical per-cycle
counts and zero drops/xruns/deadline misses or non-finite samples; the latest
five-cycle rerun has bounded packet/frame variation but the same zero-failure
result. All temporary streams/recordings were removed and media-device state
remained unchanged.

The same current tree passed `cargo clippy --workspace --locked
--all-targets --all-features -- -D warnings` on 2026-09-17. An implementation
scan found no in-scope TODO, `unimplemented!`, or unfinished feature path; the
remaining future/unsupported markers are documented capability boundaries or
deferred driver work.

After the OS-transition mapping changes, the locked workspace regression was
rerun on 2026-09-17 and passed again: CLI 36, MCP stdio 3, control 177 plus 3
guarded-live ignored, domain 65, DSP 34, engine 116, plugin-host 70,
worker-process 13, recording 40, storage 92, transport 19, Windows-audio 88,
and all doc-tests.

The current-tree refresh also passed the locked workspace regression and
`tests/acceptance/m05-ui.ps1` on 2026-09-17. M05 reported TypeScript
typecheck, 19 UI files/257 tests, and a temporary four-file production build.
The automated gate does not replace attended accessibility, first-run, or
live drag-and-drop evidence.
The 2026-09-18 resume checkpoint reran `tests/acceptance/docs.ps1` and
`tests/acceptance/m08-traceability.ps1`, which passed with 56 Markdown files,
224 local links, and 159 normative requirement IDs. It also reran
`cargo test --workspace --locked`: all current workspace suites and doc-tests
passed, with 3 guarded-live control tests correctly ignored. This refresh does
not close the unavailable interactive-surface, hardware, sandbox,
clean-checkout release, or deferred driver/signing gates.
After the polling-capture compatibility retry, the full locked workspace
regression was rerun: all workspace tests and doc-tests passed, strict
workspace Clippy passed, contract drift passed with 86 methods/20 node kinds/
7 processors/20 event categories, and documentation/traceability passed with
56 Markdown files/224 links/159 normative IDs.
The current editor slice also routes existing virtual-output shelf drops
through the supported physical-output branch, while managed virtual-bus drops
remain explicitly deferred; physical endpoints, processors, and recorder
destinations remain covered by the same draft shelf and connection tests.

`tests/acceptance/m07-headless.ps1` also passed on 2026-09-17 with 36 CLI,
3 MCP stdio, 177 control plus 3 guarded-live ignored, 70 plugin-host, and 13
worker-process tests, plus doc-tests. This refresh does not promote the
attended UI, physical-latency, managed-driver, signing, or clean-machine gates.
The contract drift checker was rerun with the same recovery change and passed
with 86 methods, 20 node kinds, 7 processors, and 20 event categories.
The shell-owned per-user startup helper is implemented and its exact
enable/disable round trip passed in the authorized elevated context on
2026-09-17; a read-only follow-up confirmed the `AudioRouter` value was absent
after cleanup. Attended startup usability and native audio restart remain
separate open gates.

Recovery implementation was tightened so a resume resnapshot now retains exact
endpoint changes for the deliberate rebind boundary and emits the bounded
`devices.changed` event immediately. Resume still never selects a replacement
or reopens a native stream automatically; the control regression suite passed
177 tests after this change, with three guarded live tests ignored.
The subsequent full locked workspace regression also passed after the live
privacy acceptance change: CLI 36, MCP 3, control 177 plus 3 guarded-live
ignored, domain 65, DSP 34, engine 116, plugin-host 70, worker-process 13,
protocol 8, recording 40, storage 92, transport 19, Windows audio 88, and all
doc-tests.

Specification audit correction: `PROD-03`, the product release slices,
`02-workflows.md`, `06-virtual-devices.md`, and the delivery decision register
now distinguish the supported current VB-Cable-first profile from the deferred
AudioRouter-owned managed-driver profile. Existing-device routing is therefore
not described as a driver or as managed-bus provisioning, while the original
managed requirements remain traceable for future signed delivery.

Security audit note: the plugin worker boundary currently revalidates binary
identity, runs outside the backend, uses a Windows kill-on-close job with one
active process and a 512 MiB process-memory cap, and fails closed on worker
failure. This is process/resource containment evidence. It is not full
filesystem/network sandboxing or a broad commercial-plugin compatibility claim;
those remain explicit limitations under SEC-07 and the M06 operations notes.
The explicitly selected installed ReaJS x64 VST2 candidate was also rejected
at the worker boundary because its state asset was empty; its fingerprint and
the temporary acceptance environment were restored. It remains an unsupported
candidate rather than an expanded compatibility claim.

Attended M05 review was attempted on 2026-09-17 through the Windows
computer-use surface, but its authoritative inventory returned no targetable
Windows applications or browsers. Therefore keyboard, Narrator, scaling,
first-run, and live drag/drop observations remain unverified; automated UI
tests must not be promoted to attended acceptance evidence. Reattempt when a
targetable interactive shell is available.

## Explicit non-driver audit boundary

The implementation/evidence audit leaves these requirement groups open or
externally blocked, rather than treating broad suite success as completion:

- `UI-01`–`UI-14` automated editor behavior is covered, but attended keyboard,
  Narrator, scaling, first-run, and live drag/drop acceptance remains open
  (`NFR-08`, `NFR-14`).
- `STATE-08` shell startup ownership and cleanup are qualified by the elevated
  exact-value round trip; attended sign-in/tray behavior and automatic native
  audio restart remain open.
- `STATE-11`, `CAP-11`, `CAP-12`, and `NFR-10`/`NFR-11` have bounded policy,
  authenticated delivery, exact notification mapping, listener
  startup/teardown, and guarded exact stopped rebind evidence. Actual
  lock/sign-out/sleep/resume delivery, endpoint re-enumeration caused by an
  OS transition, native reopen after that transition, and endurance cycles
  remain unverified.
- `NFR-01`–`NFR-06`, `NFR-13`, `QUAL-01`, `QUAL-04`, and `QUAL-05` still need
  the declared physical reference hardware, calibrated latency/clock data, or
  endurance workload; digital impulse timing is not physical latency evidence.
- `PLUG-07`/`SEC-07` have local VST2/VST3 worker and resource-containment
  evidence, but redistribution rights, broad third-party compatibility, and a
  full filesystem/network sandbox remain open by documented boundary.
- `SEC-06`/`NFR-09` have focused evidence for durable authorization/state,
  process-local block silencing, repeated next-block fan-out silence, and
  native-worker propagation. The latest guarded user-mode
  processing-boundary sample measured 16.734 ms p95, but calibrated
  effective-output timing and an attended microphone-path measurement remain
  open; do not promote the sample to the full physical NFR-09 claim.
- `ENG-05`/M08 artifact preparation remains blocked by the dirty user worktree;
  no reset, stash, commit, or clean-tree bypass is authorized.

## Remaining tasks, in order

1. Keep the repository-wide non-driver requirement/evidence audit current;
   the present audit has recorded the supported VB-Cable-first routing,
   application, DSP/recording, plugin, API, recovery-policy, and packaging
   evidence in their owning milestone files. Privacy propagation into active
   native workers is implemented and regression tested; the next
   implementation slice is guarded endpoint revalidation and recovery after
   an allowed OS transition.
2. Preserve the completed local plugin qualification and keep redistribution
   rights, OS sandboxing, native editors, and unsupported formats explicitly
   bounded; qualify additional vendor binaries only when available and
   authorized.
3. Complete portable and Windows user-mode recovery evidence, including
   guarded OS-transition delivery and endpoint revalidation where the host
   permits it. Exact stopped rebind is now qualified; do not claim power-state
   delivery or transition-triggered native reopen from it.
4. Perform attended M05 accessibility, scaling, first-run, and drag/drop
   review when an interactive target is available; retain automated UI tests
   as separate evidence.
5. Prepare and verify release artifacts from a clean checkout. Do not call an
   unsigned or dirty-tree preparation a release qualification.
6. At completion, archive this plan with a dated milestone/task name and an
   index entry. If a task remains blocked, record the exact evidence and
   replace this file with the next actionable plan rather than claiming done.

M08 audit result: `tools/release/prepare-artifacts.ps1` has no override for
the clean-tree check and stops before building whenever `git status --porcelain
--untracked-files=all` is non-empty. The current checkout contains unrelated
user changes and this plan's documentation changes, so release preparation is
an evidence-backed clean-checkout prerequisite, not a code failure. Do not
stash, reset, commit, or discard the current worktree to bypass it.

An earlier elevated multi-input/many-output VB-Cable qualification on
2026-09-17 used `CABLE Output` plus Focusrite capture and `CABLE Input` plus
DELL physical render. It captured 47,040 frames, delivered 244 quanta,
rendered 47,232 frames, and cleaned up the same-process streams. This refreshes
the supported existing-device route evidence without changing defaults,
volume, mute, privacy, drivers, or persistent audio settings.

A subsequent earlier rerun used exact CABLE Output plus Focusrite capture and
Voicemeeter In 2 plus CABLE Input render endpoints, directly covering an
installed virtual output and an existing VB-Cable output. It delivered 48,480
captured frames through 259 graph quanta and rendered 47,712 frames; temporary
streams were cleaned and no persistent audio configuration changed.

A subsequent mixed-topology rerun used the same two captures with Voicemeeter
In 5 plus CABLE Input renders and passed with 48,000 captured frames, 273 graph
quanta, and 47,712 rendered frames. Current attempts using CABLE Input,
Voicemeeter In 2, or Focusrite render endpoints also preserved the expected
`deviceInUse` diagnostic when those endpoints were externally occupied; no
external clients were closed and no ownership conflict was hidden.

The elevated control-owned rebind lifecycle was refreshed again against exact
Focusrite capture, DELL render, and P32p-30 fan-out endpoints: 23,520 capture
frames, 183 graph quanta, 23,424 primary rendered frames, 169 fan-out packets,
and 21,632 fan-out rendered frames passed through stop, exact stopped rebind,
restart, and cleanup. The separate
multi-input/many-output run passed with exact Focusrite plus Voicemeeter
captures and DELL plus P32p-30 renders: 47,040 capture frames, 302 quanta,
and 46,752 rendered frames. The initial CABLE Input render attempt returned
the preserved `AUDCLNT_E_DEVICE_IN_USE` diagnostic; no ownership conflict was
masked as a successful route.

A further guarded control-owned lifecycle retry on 2026-09-17 selected exact
CABLE Output capture, P32p-30 render, and DELL fan-out render after the default
CABLE render reported `AUDCLNT_E_DEVICE_IN_USE`. The alternate run passed
stop, exact rebind, restart, and cleanup with 24,480 captured frames, 191
graph quanta, 24,448 primary rendered frames, 155 fan-out packets, and 19,840
fan-out rendered frames. No persistent audio state changed.

The elevated, explicitly authorized `m02-control-application-live.ps1`
acceptance was also refreshed against the currently running Zoom process using
its exact PID, `Zoom.exe` basename, verified full executable path, and creation
timestamp. Include mode passed two bounded start/pump/stop cycles,
same-process worker restart, and unchanged media-device state. The harness
restored its temporary environment; no endpoint default, volume, mute, privacy,
or persistent audio configuration changed.

An earlier 2026-09-17 guarded native privacy sample measured the explicit
control-dispatch-to-next-processed-block boundary on exact `CABLE Output`
capture, `P32p-30` render, and `DELL U3223QE` fan-out endpoints. Eight bounded
mute transitions produced a p95 of 22.132 ms and the route completed cleanup.
This superseded sample remains historical user-mode processing-boundary
evidence; the latest 16.734 ms sample is authoritative. Calibrated
effective-output timing and attended microphone-path measurement remain open.

The preceding elevated control-owned lifecycle used exact CABLE Output
capture, Voicemeeter In 5 primary render, and CABLE In 16ch fan-out. It delivered
24,000 captured frames, 187 processed quanta, 23,648 primary rendered frames,
and 166 fan-out packets covering 21,248 rendered frames. Privacy dispatch to
the next processed block measured 14.076 ms p95; exact rebind, restart, and
cleanup passed without persistent audio configuration changes. The default
CABLE Input retry remains recorded as the preserved `deviceInUse` diagnostic;
the latest physical PD200X fan-out run is authoritative for the current live
sample.

An earlier same-day elevated multi-input/many-output acceptance used exact `CABLE
Output` and Focusrite capture endpoints with DELL U3223QE and P32p-30 render
endpoints. It delivered 47,520 captured frames through 277 graph quanta and
rendered 48,512 frames; same-process cleanup passed without changing defaults,
volume, mute, privacy, or persistent audio configuration.

The earlier guarded application-capture acceptance also passed both include
and exclude modes against a prior exact Zoom identity (PID 21768, basename
`Zoom.exe`, verified executable path and creation time) and the existing CABLE
Input render endpoint. Each mode completed two bounded lifecycle cycles and a
same-process worker restart; media-device identity/state remained unchanged.

An additional elevated include-mode run used the same verified Zoom identity
with exact Voicemeeter In 5 as the render destination. Two bounded lifecycle
cycles and same-process restart passed with unchanged media-device state,
extending application/tool routing evidence to an installed virtual endpoint.
The matching exclude-mode run against Voicemeeter In 5 also passed the same
two-cycle and restart boundary with unchanged media-device state.

On 2026-09-17 the focused control recovery regression
`recovery_endpoint_resnapshot_publishes_a_bounded_device_change_event` passed,
and the elevated `m07-shell-rpc.ps1` acceptance passed through WebView2 startup,
the native command, and authenticated `system.describe`. These refresh
portable recovery and shell-transport evidence only; actual OS power/session
delivery, endpoint re-enumeration, and automatic native reopen remain open.

## Validation matrix

| Area | Evidence | Current result |
| --- | --- | --- |
| Docs/traceability | `docs.ps1`, `m08-traceability.ps1` | 56 files/224 links; 159 IDs |
| DSP/recording | `m04-dsp-recording.ps1` | 34 DSP + 40 recording passed |
| UI | `m05-ui.ps1` | typecheck, 257 tests, production build passed |
| Headless | `m07-headless.ps1` | CLI/MCP/control/plugin/worker passed; guarded live tests ignored |
| M06 | VST3/VST2 worker and editor wrappers | local fixture boundaries passed |
| VB-Cable route | guarded M02/M03 wrappers | multi-input/fan-out/tool routes passed |
| OS transitions | 29 shell tests, `m07-shell-rpc.ps1`, guarded exact rebind | policy/shell transport and stopped rebind passed; attended OS delivery/reopen open |
| Privacy mute | focused engine/control Rust tests; guarded native timing sample | realtime block gate, fan-out silence, durable authorized state, worker propagation, and 16.734 ms user-mode processing-boundary p95 passed; calibrated output-boundary timing open |
| Release | M08 preparation | blocked until clean checkout is supplied |

## 2026-09-18 completion audit

The attended-surface recheck on 2026-09-18 returned `apps: [], browsers: []`
again from the computer-use inventory. No Windows application or browser was
targetable, so no keyboard, Narrator, scaling, first-run, live drag/drop, or
native Test Signal plan/commit/start/meter/stop observation was attempted or
claimed. Reattempt when an interactive shell is exposed; portable acceptance
remains authoritative for the implementation slice.

The M05 Test Signal/destination-meter slice was implemented and refreshed on
2026-09-18. The locked workspace regression passed: CLI 36, MCP stdio 3,
control 177 plus 3 guarded-live ignored, domain 66, DSP 34, engine 118,
plugin-host 70, worker-process 13, protocol 8, recording 40, storage 92,
transport 19, Windows audio 88, and all doc-tests. M05 UI acceptance passed
typecheck, 19 UI files/260 tests, and a temporary four-file production build.
Contract drift passed with 86 methods, 20 node kinds, 7 processors, and 20
event categories; documentation validation passed with 56 Markdown files and
224 local links. These are portable/editor checks only; native plan/commit,
endpoint playback, attended UI, physical-latency, driver, signing, and clean
release gates remain separate.

The disposable `tests/acceptance/m08-installer-smoke.ps1` also passed on
2026-09-18, producing a 4,887,249-byte unsigned x64 NSIS installer after
verifying the source manifest hash and cleaning its bundle output. This is
installer-generation evidence only; the clean-checkout, signing, install,
driver, hardware, and release-publication gates remain open.

The elevated `tests/acceptance/m07-shell-rpc.ps1` refresh also passed on
2026-09-18, covering WebView initialization, the Tauri command boundary, and
authenticated `system.describe` with disposable state. It changed no audio
endpoint or persistent machine configuration and does not close attended UI,
OS-transition delivery, native-reopen, or release gates.

The elevated M02 native lifecycle requalification on 2026-09-18 preserved an
endpoint-specific `AUDCLNT_E_DEVICE_IN_USE` (`0x8889000A`) result for the
occupied CABLE render endpoint, then passed against the exact CABLE capture and
already qualified PD200X render endpoint: 24,000 captured frames, 187
processed quanta, 23,936 rendered frames, 187 fan-out packets, 23,936
fan-out frames, and 11.215 ms privacy dispatch-to-processed p95. This is
existing-device lifecycle evidence only; physical latency, OS-transition
reopen, managed-driver, and release gates remain open.

The current `tests/acceptance/m07-headless.ps1` refresh passed on 2026-09-18:
CLI 36, MCP stdio 3, control 177 with 4 guarded-live tests ignored,
plugin-host 70, worker-process 13, associated doc-tests, the M01 CLI
acceptance, and `git diff --check`. This refresh strengthens the headless
control-plane evidence but does not close attended UI, actual OS-transition,
physical, driver, signing, or clean-checkout release gates.

The active-plan audit found one active execution plan (`current.md`) and no
additional active execution-plan files. The milestone documents are stable
contracts, not separate plans to archive. The current VB-Cable-first scope has
portable and guarded existing-device evidence through M07, but the plan is not
complete because the following gates remain authoritative and open:

- M00/M03 managed virtual-device provisioning, production signing, and
  clean-machine driver qualification remain deferred to the future driver
  plan.
- M02/M04 physical or acoustic latency, long-duration/endurance, and attended
  listening evidence remain open even where digital or portable substitutes
  pass.
- M05 attended keyboard/Narrator, scaling, first-run, and live drag/drop
  remain open. The graph-native Test Signal and destination-meter slice is
  implemented and its exact stopped-to-plan/commit-to-start-to-meter-to-stop
  workflow passed against the authorized exact CABLE/PD200X endpoints; this
  does not substitute for attended UI evidence.
- M07 attended tray/startup usability, actual OS-transition delivery, and
  native audio restart remain open. The current authenticated transport is a
  same-user Windows named pipe; Firefox, Claude/ChatGPT Work, and remote MCP
  clients do not have browser transport access.
- M08 clean-tree artifact preparation, signed installer, clean install/upgrade
  and uninstall, hardware/app matrix, and release publication evidence remain
  open. The worktree is intentionally dirty and was not reset, stashed, or
  bypassed.

Current portable refresh on 2026-09-18 passed:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\docs.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m08-traceability.ps1
npm.cmd --prefix ui run typecheck
npm.cmd --prefix ui test -- --run
```

Results were 56 Markdown files/224 local links, 159 normative IDs, and 19 UI
test files/257 tests. These checks are portable documentation/UI evidence only;
they do not close the open Windows-attended, physical, driver, signing, or
clean-release gates. The plan remains active with the exact next slice in the
actionable handoff below. It must not be archived as a completed milestone
until those gates are either satisfied with matching evidence or explicitly
superseded by a new authorized scope decision.

## Risks and rollback

Do not alter default devices, volume, mute, privacy, driver state, startup
registration, or persistent audio settings during guarded qualification.
Temporary endpoints, processes, databases, recordings, and build outputs must
be exact-run-owned and removed. Revert documentation-only status changes by
restoring the archived plan; revert implementation changes with focused
patches and rerun the affected acceptance suite. Never reset or discard
unrelated user changes.

## Next action

### 2026-09-18 M05 Test Signal and destination-meter execution slice

Objective: implement the graph-native Test Signal source and expose bounded
destination meter observations through the existing backend/editor contracts.

Requirement IDs: UI-01, UI-05, UI-12, GRAPH-08, DSP-07, NFR-08, ENG-01.

Prerequisites and decisions: preserve the stopped/unarmed default, use the
existing validated node-parameter path, keep generation in the realtime graph
allocation-free, and do not add Web Audio, browser microphone access, or
endpoint-default changes. The source is an ordinary versioned node with
bounded frequency, level, and duration parameters. Existing Meter nodes remain
the destination observation boundary.

Ordered tasks:

1. Extend the domain, shared TypeScript contract, discovery metadata, graph
   compiler, and UI library with Test Signal while preserving unknown-node and
   parameter rejection behavior.
2. Add a prepared realtime source stage whose phase/duration state is bounded
   and reset at activation/stop; add focused domain/engine regression tests.
3. Add the Test Signal acceptance shape to the editor and expose existing
   bounded meter observations with explicit stopped/no-signal/not-prepared
   wording; add focused UI tests.
4. Run contract drift, domain/engine/control tests, UI typecheck/tests, and
documentation validation. Record exact results and remaining Windows gates
here before changing plan status.

The administrator-authorized `tests/acceptance/safe-all.ps1` chain was rerun
on 2026-09-18. Its available M00 through M07 steps passed, and its M08 release
step stopped at `tools/release/prepare-artifacts.ps1:39` with `release inputs
must come from a clean Git working tree`; the runner removed 15 run-owned
temporary children. The existing dirty worktree was preserved, so clean-tree
artifact preparation remains an external prerequisite and no release artifact
claim is made.

Validation matrix: portable contract/domain/engine/control/UI tests first;
guarded Windows route validation only if the existing authorized endpoint
context is available. The latter must prove plan/commit, start, meter signal,
stop, unchanged endpoint identity/default state, and cleanup. A portable pass
does not close attended UI, physical-latency, driver, signing, or release gates.

Rollback: revert only the Test Signal/meter patches and their evidence entry;
do not reset, stash, commit, or discard unrelated worktree changes. If a
native gate is unavailable, retain the implementation as unqualified and
document the exact missing evidence.

The 2026-09-18 UI simplification pass applies the M05 visual-editor direction
from the supplied Audio Hijack reference: the default editor foregrounds the
session list, signal-flow canvas, drag/drop node library, and transport/status
actions. Dense setup, endpoint inventory, plugin, recovery, recording-detail,
and diagnostic panels are visually removed from the default path while their
existing backend-backed controls remain in the source for the advanced/editor
workflow. The pass is presentation-only and does not change graph semantics,
audio lifecycle, or authorization. UI typecheck and the full 257-test suite
passed after the change. Attended visual, scaling, Narrator, and live drag/drop
acceptance remain open because the interactive Windows surface is unavailable.

The same review identified a separate transport boundary for browser clients:
the current shell/backend uses an owner-authenticated Windows named pipe, not a
TCP listener, so Firefox, Claude/ChatGPT Work, and MCP cannot yet be treated as
equivalent browser clients. The existing tray already exposes Open, close,
quit, privacy, status, and recorder actions, but it does not have a truthful
IP/port to display. Add loopback-only HTTP/WebSocket JSON-RPC with explicit
origin/authentication policy and a tray endpoint/status item as a subsequent
M07/M05 integration task; do not claim browser or remote control support until
that transport is implemented and qualified.

The follow-up M05 polish on 2026-09-18 added a generated waveform tray icon,
an expanding canvas layout, an initialization-time ReactFlow fit pass, and
clearer draft/VoiceMeeter guidance in the editor. UI typecheck and all 257 UI tests
passed; the Tauri shell also passed `cargo check`. Live tray rendering and
interactive signal playback remain Windows-attended evidence rather than
portable test evidence.

The documentation reconciliation on 2026-09-18 updated the interface,
M05/M07 milestone contracts, and quickstart to define the graph-native
Test Signal plus meter slice, explain VoiceMeeter endpoint ownership, give the
current raw VB-Cable signal smoke, and preserve the named-pipe/browser
transport boundary. The Test Signal/meter implementation is now portable-
tested; no browser listener is being represented as implemented.

The 2026-09-17 headless, documentation/traceability, plugin-worker,
application-capture, and existing-device rebind gates are current. Continue
with the next actionable non-driver slice: advance guarded Windows recovery
evidence where the host permits it, and prepare a clean-checkout release run
when an authorized clean checkout is supplied. Do not promote automated UI
tests or policy-only transition tests to attended acceptance, and retain the
documented driver, signing, physical-latency, sandbox, accessibility, and
clean-release limits.

### Actionable handoff

- The supported exact CABLE bridge is now requalified through five elevated
  cycles with zero drops, xruns, deadline misses, or non-finite samples; keep
  the documented elevated `deviceAdministration` boundary and do not use
  Windows-default or driver changes as a workaround for non-elevated
  `0x80070057`.
- Existing physical output is qualified through the PD200X fan-out route;
  Focusrite primary-render attempts remain endpoint-specific
  `deviceInUse` evidence and must not be represented as a general hardware
  pass.
- Reattempt attended M05 and actual OS transition delivery only when the
  computer-use inventory exposes an interactive Windows surface; the latest
  authoritative inventory was `apps: [], browsers: []`.
- A fresh computer-use inventory recheck on 2026-09-18 again returned
  `apps: []` and `browsers: []`; no attended UI or real lock/sign-out/
  sleep/resume transition was attempted, and the corresponding gates remain
  open rather than being inferred from portable policy tests.
- Run `m08-release.ps1` only from an authorized clean checkout; the current
  worktree is intentionally dirty and no reset, stash, commit, or bypass is
  authorized.
- Next M05/M02 qualification slice: qualify the Test Signal workflow against
  bounded per-node/destination meters, then qualify the stopped → plan/commit →
  start → meter → stop workflow with exact existing endpoints. Keep VoiceMeeter
  coexistence and `deviceInUse` ownership diagnostics explicit.

This handoff is superseded by the 2026-09-18 native Test Signal acceptance:
the exact CABLE/PD200X plan/commit/start/meter/stop workflow passed. The next
actionable non-driver slice is guarded OS-transition delivery and endpoint
revalidation where the host permits it.

The elevated M02 multi-input/many-output acceptance also passed on 2026-09-18
with exact CABLE Output plus Focusrite capture and DELL plus PD200X render
endpoints: 47,520 captured frames, 270 delivered quanta, and 48,000 rendered
frames. This refreshes bounded existing-device fan-out evidence without
changing persistent audio configuration; physical latency, endurance,
OS-transition reopen, and managed-driver gates remain open.

The elevated M00 event-driven WASAPI refresh also passed on 2026-09-18 using
exact CABLE Output capture and silent PD200X render endpoints: 24,480 capture
frames and 28,320 submitted render frames. Event initialization/start/stop/
reset and cleanup passed with no media-device identity/state change. This is
not calibrated physical-latency or managed-driver evidence.

The elevated M02 application-capture refresh also passed on 2026-09-18 against
the observed `voicemeeterpro.exe` identity (PID 71412, creation timestamp
`134342414024784043`, verified path) and exact PD200X render endpoint. Include
mode completed two bounded start/pump/stop cycles plus same-process worker
restart with unchanged media-device state. No process or persistent audio
configuration was changed; this is not universal application compatibility
evidence.

The current local M06 VST2 acceptance also passed for six x64 fixtures at
44.1, 48, and 96 kHz with worker, parameter-offset, and fixture-integrity
checks. No registration, audio stream, or machine audio configuration action
occurred. Redistribution rights, full sandboxing, broad vendor compatibility,
native editors, and release gates remain explicitly open.

The current `cargo test --workspace --locked` regression passed on 2026-09-18:
CLI 36, MCP stdio 3, control 177 plus 4 explicitly guarded live tests
ignored, domain 66, DSP 34, engine 118, plugin-host 70, worker-process 13,
recording 40, storage 92, transport 19, Windows audio 88, and all workspace
doc-tests. This confirms the current shared tree after the native Test Signal
acceptance; it does not close attended, physical, OS-transition, driver,
signing, sandbox, or clean-release gates.

The elevated pinned VST3 SDK acceptance also passed on 2026-09-18 after the
initial non-elevated MSBuild FileTracker access-denied failure. SDK/fixture
build, validator self-tests, offline loader, parameter descriptors,
auxiliary-bus handling, explicit single-bus rejection, and the five-class mda
matrix passed. No system registration, audio stream, or machine audio
configuration changed; third-party rights, sandbox, native-editor, and
release gates remain open.

The current M04 DSP/recording acceptance also passed on 2026-09-18: 34 DSP
tests, 40 recording tests, associated doc-tests, formatting, strict package
Clippy, and diff checks. This refreshes portable processing and file-boundary
evidence only; physical pitch/latency and long-duration live-performance
gates remain open.

The current `tests/acceptance/m01-cli.ps1` parity acceptance also passed on
2026-09-18, refreshing offline discovery/help/schema, shared dispatch,
persistence, authorization, virtual-route, recording, recovery, startup,
and MCP/CLI parity without audio-endpoint or machine-configuration changes.

The current `tests/acceptance/m03-virtual-buses.ps1` acceptance also passed,
refreshing eight-bus desired-state capacity, lifecycle persistence,
cross-session route validation, cycle/conflicting-writer rejection, and stale
revision safety. No native device provisioning, driver installation/loading,
or audio configuration action occurred.

The current x64 M03 prototype build acceptance also passed, covering WDK
source-contract compilation, INF/package qualification, signability/catalog
checks, and lifecycle guards without installation, loading, signing-mode,
boot-policy, service, or audio configuration changes. The read-only signing
prerequisite check remains unresolved because `Win32_DeviceGuard` is
unavailable on this host; no VBS or signing conclusion is claimed.

The non-installing ARM64 M03 driver-build acceptance also passed on
2026-09-18, covering the ARM64 WDK compile, INF/package, signability/catalog,
and lifecycle guard path. No installation, loading, signing-mode, boot-policy,
service, or audio configuration action occurred.

The no-side-effect M03 software-device API probe also passed on 2026-09-18,
compiling and reporting the bounded `SWD\\AudioRouterVirtual` dry-run plan
before removing its temporary artifacts. No software device, driver,
endpoint, or machine audio configuration was created.

The elevated read-only M00 native-format inventory also passed on 2026-09-18
for 34 active endpoints. Mix-format metadata was successful for every endpoint
with observed 48 kHz mono/stereo and 96 kHz mono/eight-channel float layouts;
media state and temporary-artifact cleanup were unchanged. No stream, driver,
or machine configuration action occurred.

The read-only M00 toolchain compatibility acceptance also passed on
2026-09-18 with Visual Studio 18 Community, MSVC 14.51.36231, Windows
SDK/WDK `10.0.28000.0`, and matching `signtool.exe`. No SDK installation,
driver action, signing-mode change, or audio configuration action occurred.

The focused recovery refresh on 2026-09-18 passed 7 locked control
OS-transition tests and all 29 locked `src-tauri` shell tests. The fresh
computer-use inventory remained empty (`apps: []`, `browsers: []`), so no
attended lock/sign-out/sleep/resume or UI acceptance was attempted; real
transition delivery, endpoint re-enumeration, and native reopen remain open.

The M08 traceability acceptance was rerun on 2026-09-18 and passed with all
159 normative requirement IDs covered by the delivery map. This confirms
documentation coverage only and does not promote the remaining hardware,
driver, signing, attended, sandbox, or clean-release gates.

The current-tree `cargo test --workspace --locked` rerun on 2026-09-18 also
passed: CLI 36, MCP stdio 3, control 177 plus 4 explicitly guarded live tests
ignored, domain 66, DSP 34, engine 118, plugin-host 70, worker-process 13,
protocol 8, recording 40, storage 92, transport 19, Windows audio 88, and all
workspace doc-tests. No live-test credentials or endpoint state were changed.

The same current-tree strict static gates also passed on 2026-09-18:
`cargo fmt --all -- --check` and
`cargo clippy --workspace --locked --all-targets --all-features -- -D warnings`.
Milestone review found no additional portable implementation task authorized
by the current VB-Cable-first/non-driver boundary; the remaining tasks are
explicitly attended, hardware, signed-driver, rights/sandbox, or clean-release
gates.

The external prerequisite recheck on 2026-09-18 again found no interactive
Windows surface (`apps: []`, `browsers: []`), and the read-only M03 signing
check again failed because `Win32_DeviceGuard` is unavailable. No OS security,
signing mode, driver, or audio configuration was changed.

The direct rerun of `tests/acceptance/m03-signing-prerequisites.ps1` on
2026-09-18 produced the same explicit `Win32_DeviceGuard is unavailable;
cannot record the VBS prerequisite` result. This is a host-capability blocker,
not evidence of signing or HVCI readiness.

The current UI verification also passed on 2026-09-18 through `npm.cmd`:
TypeScript typecheck succeeded and Vitest reported 19 test files and 260 tests
passed. The initial `npm` wrapper invocation was blocked by PowerShell
execution policy; no policy change was made. This remains automated UI evidence
only, separate from attended accessibility, scaling, first-run, and drag/drop
acceptance.

The dedicated `tests/acceptance/m05-ui.ps1` wrapper then passed on 2026-09-18,
covering the same typecheck and 19-file/260-test suite plus a temporary
production Vite build (4 output files). The wrapper removed its temporary
output; no audio, driver, or machine configuration changed. This strengthens
automated M05 evidence only and does not close attended UI gates.

The dedicated `tests/acceptance/m07-headless.ps1` wrapper also passed on
2026-09-18: 36 CLI tests, 3 MCP stdio tests, 177 control tests plus 4
explicitly guarded live tests ignored, 70 plugin-host tests, 13 worker-process
tests, and associated doc-tests. The wrapper also passed the M01 CLI
acceptance; no audio-device, driver, or machine configuration changed.

The current `tests/acceptance/m03-virtual-buses.ps1` acceptance also passed,
refreshing bounded desired-state capacity, route persistence, cycle and
conflicting-writer rejection, and revision safety. It performed no native
device provisioning, driver installation/loading, or machine audio
configuration action.

The current `tests/acceptance/m04-dsp-recording.ps1` acceptance also passed:
34 DSP tests, 40 recording tests, both crate doc-test sets, and its formatting,
strict package Clippy, and diff checks. This refreshes portable transfer,
pitch, recording, checkpoint, recovery, segmentation, and library evidence;
physical pitch/latency and long-duration live-performance gates remain open.

The current `tests/acceptance/m06-vst2-state-fixture.ps1` acceptance passed at
44.1, 48, and 96 kHz. Repository-owned ignored DLL fixtures covered chunk
state restore, legacy-main handling, non-finite output rejection, and native
fault containment through disposable workers. No plugin registration, audio
stream, or machine audio configuration changed; rights, full sandboxing,
native editors, and broad compatibility remain open.

The current `tests/acceptance/m00-native-build.ps1` acceptance also passed,
revalidating the compile-only native WASAPI probe boundary. No audio stream,
driver, signing-mode, or machine-configuration action occurred.

The current ARM64 M03 driver-build acceptance also passed, covering the
project-owned WDK compile, signability, and catalog qualification path. No
installation, loading, signing-mode, boot-policy, service, or audio
configuration action occurred; production signing and clean-machine
qualification remain open.

The matching x64 M03 driver-build acceptance also passed the project-owned
WDK compile, signability, and catalog qualification path. No installation,
loading, signing-mode, boot-policy, service, or audio configuration action
occurred; production signing and clean-machine qualification remain open.

The current M03 Software Device API probe also passed its native compile and
no-side-effect `SWD\\AudioRouterVirtual` dry-run; the temporary executable was
removed. No software device, driver, endpoint, or machine audio configuration
was created.

The current `tests/acceptance/m06-vst3-worker.ps1` acceptance also passed for
the pinned local `again.vst3` fixture. It covered isolated single-stream and
auxiliary-bus processing, asynchronous graph staging, bounded
restart/quarantine recovery, validated state restoration where supported,
repeated-quantum timing, finite output, and bounded shutdown. No plugin
registration, audio stream, or machine audio configuration changed; rights,
full sandboxing, native editors, and release qualification remain open.

The current M06 SDK installer-provenance acceptance also passed using
disposable Git metadata checks. No SDK, plugin, driver, or audio configuration
was changed.

The unsigned M08 NSIS smoke was requalified on 2026-09-18: its restricted
attempt hit npm registry/cache `EACCES`, while the authorized elevated retry
passed and produced a disposable x64 `AudioRouter_0.1.0_x64-setup.exe` bundle
of 4,892,281 bytes with manifest-hash verification. Temporary bundle output
was removed. No installer execution, installation, signing, driver, or audio
configuration action occurred; clean-checkout, signing, install/upgrade/
rollback, and managed-driver gates remain open.

The post-acceptance cleanup inspection found pre-existing `audiorouter-*`
temporary databases/cache entries under the user temp directory. Ownership
could not be established safely from the current state, so none were deleted;
the acceptance-owned installer output itself was removed. This is a cleanup
limitation, not release evidence.

The M08 traceability and documentation gates were rerun on 2026-09-18:
159 normative IDs remained covered, and documentation validation passed for
56 Markdown files and 224 local links. `git diff --check` passed.

The read-only M00 format inventory was rerun on 2026-09-18. Its non-elevated
attempt received `Get-PnpDevice` access denied (`0x80041003`); the authorized
elevated retry passed for 34 active endpoints with bounded 48 kHz mono/stereo
and 96 kHz mono/eight-channel float formats. No stream, driver, or machine
configuration action occurred.

The current M06 VST2 acceptance also passed for six x64 local fixtures at
44.1, 48, and 96 kHz, including intra-block parameter-offset and before/after
binary-integrity checks through disposable workers. No plugin registration or
audio configuration changed; rights, full sandboxing, native editors, and
broad compatibility remain open.

The read-only M00 toolchain compatibility check also passed on 2026-09-18:
Visual Studio 18 Community, MSVC 14.51.36231, matching SDK/WDK 10.0.28000.0,
and matching x64 `signtool.exe` were discovered. No SDK installation, driver
action, signing-mode change, or audio configuration action occurred.

The M07 shell-RPC acceptance was rerun on 2026-09-18. The non-elevated wrapper
correctly refused the administrator-only WebView setup; the authorized
elevated retry passed WebView initialization through Tauri to authenticated
backend `system.describe` using disposable state. No audio endpoint or
persistent machine configuration changed.

The dedicated `tests/acceptance/m01-cli.ps1` acceptance was rerun on
2026-09-18 and passed offline discovery/help/schema, shared dispatch,
persistence, authorization, virtual-route, recording, recovery, startup, and
MCP/CLI parity checks. No audio endpoint, driver, or machine audio
configuration was accessed.

The current contract-drift check also passed: 86 methods, 20 node kinds, 7
processors, and 20 event categories match the UI/CLI/Rust catalogs.

The guarded Rust adapter route also passed on the exact CABLE Output/PD200X
pair: 48 kHz capture/render, 128-frame graph quantum, 2.666667 ms deadline,
24,000 captured frames, 187 graph blocks, 23,936 scheduler/routed frames,
zero deadline misses, and maximum processing time 57,700 ns (p999 upper bound
65,536 ns). Temporary probe artifacts were removed and defaults, volume, mute,
privacy, drivers, signing, and startup configuration were unchanged.

The guarded M02 native lifecycle requalification preserved the default CABLE
render `deviceInUse` diagnostic (`0x8889000A`), then passed with exact CABLE
Output capture and PD200X render: 24,000 captured frames, 187 processed
quanta, 23,936 rendered frames, 187 fan-out packets, and 23,936 fan-out
frames. Privacy mute dispatch-to-processed-block p95 was 11.007 ms; stop,
reset, exact rebind/restart, and cleanup passed without persistent audio
configuration changes.

The guarded application-capture retry preserved a rounded-DMTF identity
failure, then passed after a direct `Process.StartTime.ToFileTimeUtc()` query
supplied exact identity `134342414024784043` for `voicemeeterpro.exe` PID
71412. Include mode completed two bounded start/pump/stop cycles and a
same-process worker restart against the exact PD200X render; media-device
state and persistent audio configuration remained unchanged.

The guarded M02 multi-input/many-output acceptance also passed with exact
CABLE Output plus Focusrite captures and CABLE Input plus DELL renders:
48,000 captured frames, 283 delivered quanta, and 47,808 rendered frames.
Same-process cleanup passed without persistent audio configuration changes.

The complementary guarded application-capture run also passed in `exclude`
mode for the same verified Voicemeeter identity and PD200X render, including
two bounded start/pump/stop cycles and same-process worker restart with
unchanged media-device state and no persistent audio configuration.

The guarded M02 control-owned route adapter also passed for exact CABLE Output
capture and PD200X render: generation 1, 50 packets, 24,000 captured frames,
187 processed quanta, 23,936 rendered frames, 95,788 recording bytes, one
successful start/stop, one reset, one rejected pump, and 48 kHz on both
directions. The worker stopped/detached without changing endpoint defaults,
volume, mute, privacy, drivers, signing, startup, registration, or persistent
audio configuration.

The guarded Rust adapter bridge also passed two 500 ms cycles on exact CABLE
Output capture and PD200X render: cycle 1 captured/rendered 24,960 frames in
195 quanta; cycle 2 captured 24,480 and rendered 24,448 in 191 quanta. Both
were 48 kHz stereo with zero non-finite tap samples, dropped frames, xruns,
and deadline misses; temporary resources were removed and media-device state
remained unchanged.

The guarded native digital impulse correlation refresh also passed against the
existing VB-Cable pair: 97 of 100 expected groups detected, zero-frame p95
spacing error, and estimated 81.02 ms digital onset. Temporary probe artifacts
were removed. This refresh is bounded software signal-correlation evidence,
not calibrated acoustic/physical latency or managed-driver timing evidence.

The current `tests/acceptance/m07-headless.ps1` refresh also passed: 36 CLI
tests, 3 MCP stdio tests, 177 control tests with 4 guarded live tests ignored,
70 plugin-host tests, 13 worker-process tests, and the M01 CLI acceptance. It
changes no endpoint or machine configuration and does not close the real
OS-transition, native-reopen, attended-shell, or endurance gates.

## Current handoff refresh (2026-09-18)

The previously listed M05 Test Signal/destination-meter execution slice is
implemented and its portable, UI, contract-drift, and guarded exact-endpoint
evidence is recorded above. The next actionable slice is guarded user-mode
recovery after an allowed OS transition: first qualify notification delivery
and endpoint revalidation/reopen where the host permits it, then update the
M07 recovery evidence without claiming unattended lock/sign-out/sleep,
endpoint re-enumeration, or endurance gates that were not observed. If no
interactive or authorized transition surface is available, retain the exact
blocked evidence and proceed with documentation-only audit maintenance; do
not substitute portable tests for those Windows gates.

The current committed clean tree (`5d2123cd`, `Complete VB-Cable-first
milestone work`) passed `tests/acceptance/m08-release.ps1`. Optimized Rust
release binaries and the 214-module production UI were built, unsigned
artifacts were prepared and verified, and disposable outputs were removed.
This closes unsigned artifact preparation only; signing, installer execution,
clean-machine, managed-driver, and publication gates remain open.

The active `tests/acceptance/safe-all.ps1` harness was narrowed to this
VB-Cable-first track: deferred M03 driver-build, ARM64-driver, signing,
Software Device API, and SysVAD steps remain available as separate future
track scripts but are no longer run as active completion gates.

The corrected active safe chain then passed end-to-end on 2026-09-18. It
covered M00, M01, existing-device M03 routing, M04, M05, M06, M07, unsigned
M08 artifact preparation and NSIS smoke, frontend-owned shell RPC, M08
traceability, and documentation. No driver was installed or loaded and no
persistent audio configuration changed.

The clean-commit guarded M05 Test Signal acceptance then passed against exact
CABLE Output capture and CABLE Input render endpoints: 187 processed quanta,
destination peak `-18.000 dB`, and successful plan/commit, start, meter,
stop, and cleanup. This is existing-device graph evidence, not attended UI or
physical-latency qualification.

The exact physical PD200X speaker/microphone impulse attempt was also run with
100 impulses and detected 0 groups, so the analyzer rejected it. Temporary
artifacts were cleaned. The result preserves the current evidence boundary:
digital CABLE correlation is repeatable, but calibrated acoustic/physical
latency remains unverified.

The user-authorized higher-volume Focusrite-headset retry detected 733 of
1,000 physical impulse groups and correctly failed the ≥900 acceptance
threshold. It remains insufficient for NFR-01; a wired Focusrite output-to-
input loopback is still required for a stable calibrated measurement.

The subsequent retry detected 2,006 groups for 1,000 impulses with 471-frame
p95 spacing error and an estimated 188.98 ms onset. It is explicitly rejected
as a physical-latency result despite the wrapper exit code being zero because
the cadence is not valid.

## Physical-latency gate parked (2026-09-18)

The calibrated physical-latency gate is intentionally deferred until the user
has the required loopback cable available. The current USB PD200X microphone
and Focusrite headset arrangement can produce audible output, but it cannot
provide a stable wired reference; repeated acoustic runs were therefore
rejected for insufficient detection or invalid cadence. This is a recorded
prerequisite, not a waiver of NFR-01/NFR-02/QUAL-04. Continue with the next
available non-physical acceptance work.
