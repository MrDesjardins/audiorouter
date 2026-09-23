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
- `NFR-01` is closed on one reference device under the revised ≤250 ms target
  (DEC-14). `NFR-02` is closed on the same device under the revised ≤160 ms
  target (DEC-15) — four runs measured p95 of 97.5/102.9/115.5/110.9 ms
  through the real engine route; see
  [the wasapi-probe evidence file](evidence/M00-wasapi-probe.md).
  `NFR-03`–`NFR-06`, `NFR-13`, `QUAL-01`, `QUAL-04`, and `QUAL-05`
  still need the declared physical reference hardware, calibrated
  latency/clock data, or endurance workload; digital impulse timing alone is
  not physical latency evidence.
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
| NFR-01 physical latency | `m00-native-impulse-loopback.ps1`, wired Focusrite loopback | p95 ≈ 155–186 ms across four runs, passes the revised ≤250 ms target (DEC-14); validated on one reference device only |
| NFR-02 virtual-capture latency | `m02-nfr02-mic-virtual-capture.ps1`, live engine route + wired Focusrite loopback | p95 ≈ 97–115 ms across five runs, passes the revised ≤160 ms target (DEC-15); validated on one reference device only |
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
- M05 attended keyboard/Narrator, scaling, and first-run remain open. A
  first attended interaction/drag-drop review happened 2026-09-21 (see
  [the M05 evidence file](evidence/M05-visual-editor.md)). An initial test
  harness flaw (see the M07 methodology-defect entry) produced one false
  finding (a side-panel parameter editor that appeared broken but was not,
  retracted); the canvas mini-fader/EQ drag bug it also surfaced was real,
  found (a 3-argument callback silently misread as 2 arguments in
  `ui/src/App.tsx`), fixed, and confirmed by the user, with `npm run
  typecheck` and the full 282-test UI suite passing. The light-theme
  contrast/consistency bug was also found and fixed (hardcoded-dark CSS
  from a later redesign pass had silently overridden the pre-existing but
  correctly-wired theme system; converted to CSS custom properties so
  theme switching is robust to selector specificity/order) and confirmed
  across several rounds of user retesting, alongside a duplicated
  "Session inventory unavailable" label fix found along the way. The
  connection-anchor and input/output-distinction findings were also fixed
  together (same root cause): the canvas nodes had no registered
  `nodeTypes`, so React Flow silently fell back to its built-in "default"
  node type, which adds its own implicit Top/Bottom handles on top of the
  app's own fully custom ones — producing 3 overlapping handles on the
  top/bottom edges of a 2-port node (2 on left/right) and very plausibly
  explaining the original "top anchor doesn't work" report. Registered a
  proper pass-through custom node type to fix it, confirmed by the user
  after three rounds of color-scheme iteration (final: blue input / orange
  output, plus a permanent on-canvas legend and clear in-app feedback for
  an invalid-direction drag attempt, which previously failed silently).
  Two further findings were also added and fixed the same day: the canvas
  library had no way to add an application-capture source (a working
  "Applications" panel already existed but was hidden by default and
  absent from the library palette; added an "Application" library entry
  opening a focused picker, iterated to a filtered dropdown after user
  feedback that an unfiltered per-row list with disabled entries was
  confusing), and a clarification that Windows has no "application as
  output" concept symmetric with application capture (verified against
  `NodeKind` in the domain crate; corrected the library tooltip to point at
  the already-working "Existing virtual output" mechanism instead of the
  permanently-disabled, deferred-driver "Virtual capture sink"). Still open
  and not yet fixed: an M08 release qualification gap. The
  graph-native Test Signal and destination-meter slice is implemented and
  its exact stopped-to-plan/commit-to-start-to-meter-to-stop workflow passed
  against the authorized exact CABLE/PD200X endpoints, but automated passes
  do not substitute for the remaining attended defects.
- M07 attended tray review happened 2026-09-21, initially under the same
  flawed test harness noted above; corrected and reran once the harness
  issue was found (see
  [the M07 automation evidence file](evidence/M07-automation-recovery.md#attended-testing-methodology-defect-found-and-corrected-2026-09-21)):
  Close window/reopen from tray passed both times; Quit and stop audio
  initially appeared to fail but passed cleanly on the corrected retest
  (confirmed via `tasklist`) — the original "failure" was a harness
  artifact, not a real defect. The current authenticated transport is a
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
IP/port to display. `docs/spec/10-api.md` explicitly classifies a standalone
browser adapter as future scope; therefore loopback HTTP/WebSocket JSON-RPC,
origin/authentication policy, and a browser-facing tray endpoint are not active
M07/M05 completion tasks. Do not claim browser or remote control support.

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

## Physical-latency loopback cable now available (2026-09-21)

The user connected a wired 1/4" TRS instrument cable between the Focusrite
interface's instrument input and its headphone/speaker output, providing the
NFR-01 required wired mic-to-headphones physical loopback. A read-only
inventory probe confirmed the exact endpoints: render `Speakers (Focusrite
USB Audio)` and capture `Analogue 1 + 2 (Focusrite USB Audio)`. No stream was
opened and no device state changed during inventory.

The guarded `tests/acceptance/m00-native-impulse.ps1 -AllowLiveAudio` run
against that exact wired pair (1,000 impulses, default 10 ms interval) failed
at `render_initialize` with the preserved `AUDCLNT_E_DEVICE_IN_USE`
(`0x8889000A`) diagnostic before any capture/render lifecycle or media-device
state change. Per the existing ownership-diagnostic lesson, this is recorded
as an endpoint-in-use conflict, not a routing, format, or cable failure, and
is not masked as one. The Focusrite `Speakers` render endpoint is currently
held by another process (an existing Voicemeeter hardware-out binding is the
likely candidate given this host's configuration) and must be released before
this endpoint is free for the probe to open.

Blocked on: freeing the Focusrite `Speakers (Focusrite USB Audio)` render
endpoint (check Voicemeeter's hardware-out assignment and any exclusive-mode
application holding that device) before the NFR-01 wired-loopback impulse
acceptance can be retried. Once free, rerun the same command and, if it
passes group detection, extend the analyzer to publish per-impulse
round-trip min/p50/p95/max (the current script only reports spacing-error
and a single onset estimate) before claiming the NFR-01 gate closed.

A follow-up 0-groups run confirmed the WASAPI lifecycle was healthy but no
signal crossed the detector; a physical LED check on the Scarlett Solo's
Instrument gain halo confirmed the wired loop is electrically live once the
dedicated `Instr` input mode is enabled. With `Instr` engaged, the retried
run returned exit code 0 (`detected_groups=8928`, `p95_spacing_error_frames=
471`, `estimated_onset_ms=-156.00`) but this is explicitly rejected as
physical-latency evidence: 8,928 groups for 1,000 impulses, a spacing error
nearly equal to the expected interval, and a physically impossible negative
onset together indicate input-stage clipping producing multiple spurious
threshold crossings per impulse rather than one clean detection per impulse.
This repeats an already-documented rejected-cadence pattern from an earlier
acoustic attempt and must not be promoted to a pass on exit-code alone. After
the user lowered the instrument gain substantially, the same command returned
a clean `detected_groups=1000` with zero spacing-error frames, confirming the
wired path itself is now electrically sound; the reported single onset
estimate is process-launch-tick based and explicitly out of scope for a
calibrated NFR-01 claim (see below).

## NFR-01 calibrated wired loopback measurement implemented (2026-09-21)

Added a new single-process `impulse-loopback` mode to
`tools/m00-native-wasapi-probe/main.cpp` (`impulse_loopback_probe`) plus
`tests/acceptance/m00-native-impulse-loopback.ps1`, replacing the two-process
onset estimate with a calibrated per-impulse round-trip distribution: render
is measured via `IAudioClock` (start/end anchors extrapolated to a common
QPC timeline), capture is measured via `IAudioCaptureClient::GetBuffer`'s own
per-packet device position and QPC timestamp, and each of 1,000 impulses is
paired and converted to a latency sample; the tool publishes min/p50/p95/max
and the negotiated render/capture buffer sizes as NFR-01 requires.

Getting to a trustworthy result required fixing several real bugs found via
live hardware iteration, kept here for a future agent's benefit:
- `IAudioClock::GetFrequency` on this Focusrite driver reports the **byte**
  rate (`nAvgBytesPerSec`), not the frame rate; frame indices must be
  multiplied by `nBlockAlign` before dividing by that frequency.
- The initial `hnsBufferDuration=1000000` (100 ms) copied from this file's
  other diagnostic-only probes adds its own size directly to the measured
  round trip and must not be used for a latency measurement; switched both
  streams to a `0` (minimal engine-period) shared-mode buffer.
- A single-threaded cooperative poll loop alternating between render and
  capture cannot reliably service a ~22 ms buffer; it produced tens of
  thousands of dropped capture frames and corrupted timing. Replaced with
  two event-driven threads (`AUDCLNT_STREAMFLAGS_EVENTCALLBACK` + dedicated
  `WaitForSingleObject` loops), which produced zero dropped frames.
- A `std::cout` precision/format leak in a diagnostic print line caused a
  later value to render in truncated scientific notation, which the
  acceptance wrapper's regex silently mis-parsed as a passing `p95=2` — a
  false pass caught only by cross-checking the raw anchor numbers, not by
  the wrapper's own exit code. Fixed the stream-state leak.

With all of the above fixed, four independent elevated runs against the
user's wired Focusrite `Speakers`→`Analogue 1 + 2` Instrument-mode loopback
all passed group detection (1,000/1,000 pairs, zero dropped capture frames)
and were tightly reproducible (sub-2 ms spread within each run): p95 results
were 206.748 ms, 213.97 ms, and two further runs in the same ~205–214 ms
band. `GetStreamLatency` reported 0 for both streams (queried pre-`Start`,
inconclusive either way).

**NFR-01 result: FAILED on this configuration.** Measured wired physical
loopback p95 ≈ 207–214 ms initially, using WASAPI shared mode (matching
AudioRouter's production sharing model; not exclusive/ASIO mode) on this
Focusrite Scarlett Solo and its current driver. This is a genuine calibrated
measurement, not a parked/blocked gate and not a probe defect: the physical
loopback cable, gain staging, zero-drop capture, and cross-stream QPC
calibration are all independently confirmed sound. The finding itself — that
this shared-mode configuration does not currently meet NFR-01 — is the
actual evidence the gate has been asking for, and must be recorded as a
failing result rather than left parked. No default device, volume, mute,
privacy, or persistent audio configuration was changed by any of these runs.

A same-day follow-up found and fixed a real ~41-45 ms render-clock
calibration bias (a `render-clock-ramp` diagnostic proved this driver's
render position stays at exactly 0 for a genuine engine warm-up window
before advancing at the expected rate) and corrected the result to
**p95 ≈ 185.5-185.6 ms**, reproduced across two full runs. Buffer/engine-
period tuning (including Windows 10+ `IAudioClient3` low-latency shared
mode) was also tested and ruled out as a fix on this hardware — the
device's own low-latency engine-period floor is a fixed 10 ms, and neither
buffer size nor low-latency-mode activation changed the measured round trip.
Full detail, including one unresolved low-latency-path capture-pairing
anomaly, is in
[the wasapi-probe evidence file](evidence/M00-wasapi-probe.md).

## DEC-14: NFR-01 target revised to ≤250 ms p95 (2026-09-21)

User-approved decision, recorded here and in
[15-delivery.md DEC-14](../../spec/15-delivery.md#initial-decision-register)
and [14-quality.md NFR-01](../../spec/14-quality.md#quantitative-requirements):
the original ≤30 ms p95 target was an unvalidated aspirational figure, not
evidence-backed. With buffer size and `IAudioClient3` low-latency shared-mode
tuning both tested and ruled out as a fix on the one reference device
measured (see the evidence-file entries above), meeting ≤30 ms in WASAPI
shared mode is not achievable on this hardware without a different mechanism
entirely (WASAPI exclusive mode, different hardware/driver) that has not
been explored or authorized. Rather than leave the gate silently failing
against an unreachable number, the target is revised to ≤250 ms p95 — about
35% headroom over the worst of three reproducible ~185.5–185.9 ms
measurements on this device.

This is validated on exactly one reference device (Focusrite Scarlett Solo).
It is not yet known whether other supported interfaces perform better or
worse; qualifying additional reference hardware against this target remains
open future work. Exploring WASAPI exclusive mode as a lower-latency option
is explicitly not authorized by this decision and would need its own scope
approval, since exclusive mode would conflict with AudioRouter's multi-app
shared-routing architecture for that endpoint while held.

**NFR-01 gate status: now needs re-verification against the revised ≤250 ms
target.** The 185.5–185.9 ms measurements already on record pass it; a
formal rerun of `tests/acceptance/m00-native-impulse-loopback.ps1` with an
updated `-P95ThresholdMs 250` default is the next concrete step before this
gate can be marked closed in the validation matrix.

QUAL-04 remains unmeasured and should reuse this same calibrated
`impulse-loopback` tool once a target render/capture pair is chosen for
that measurement. NFR-02 has since been measured separately — see DEC-15
below.

## DEC-15: NFR-02 target revised to ≤160 ms p95 (2026-09-21)

User-approved decision, recorded here and in
[15-delivery.md DEC-15](../../spec/15-delivery.md#initial-decision-register)
and [14-quality.md NFR-02](../../spec/14-quality.md#quantitative-requirements),
following the same methodology as DEC-14: the original ≤40 ms p95 target was
an unvalidated aspirational figure. A new `capture-loopback` measurement
(see [the wasapi-probe evidence file](evidence/M00-wasapi-probe.md)) ran the
mic-to-virtual-capture path through the real AudioRouter engine route
(`PhysicalInput → Gain → Recorder → PhysicalOutput`, capturing Focusrite
`Analogue 1 + 2` and rendering to `CABLE Input`) on the same reference
device, with an independent capture client timestamping arrivals at both the
physical mic input and the virtual `CABLE Output` capture endpoint via each
stream's own per-packet QPC timestamp — no render-side anchor or warm-up
bias involved, unlike NFR-01. Four runs measured p95 of 97.521 ms, 102.874
ms, 115.461 ms, and 110.881 ms; each run's own internal spread stayed under
~13 ms, with more variance appearing across separate engine-route launches
than within a single run. The revised target of ≤160 ms p95 gives roughly
35% headroom over the worst observed run (115.461 ms), matching DEC-14's
methodology.

This is validated on exactly one reference device and one microphone/driver
combination. The cross-run variance (97–115 ms) is larger than NFR-01's
(sub-2 ms); whether this narrows with more runs, a different mic, or a
warmed-up (already-running) engine rather than a fresh route launch each
time is open future work, not resolved here.

**NFR-02 gate status: closed against the revised ≤160 ms target.** The
acceptance wrapper `tests/acceptance/m02-nfr02-mic-virtual-capture.ps1` was
updated to default `-P95ThresholdMs 160.0` and reran successfully; see its
result recorded in the evidence file and the validation matrix above.

After the user closed Voicemeeter Banana, the `AUDCLNT_E_DEVICE_IN_USE`
conflict on the Focusrite `Speakers` render endpoint cleared: the retry
opened both endpoints and completed the full capture/render lifecycle
(start/stop/reset all `0x0`, `render_impulse_written=1`). The analyzer then
rejected the run for a different reason: `only 0 impulse groups detected;
expected at least 900`, i.e. no sample on the `Analogue 1 + 2` capture
crossed the 0.05 peak-detection threshold at any point during the 1,000
impulses. This is evidence that the WASAPI stream lifecycle is healthy but no
audible signal is reaching the capture input over the physical wired path;
it is not an ownership, routing, or software conflict. Candidate physical
causes (not yet checked, require the user at the hardware): input gain trim
for the connected instrument-input channel turned down or unset to
instrument mode, output/headphone volume at zero, cable not fully seated in
one of the two jacks, or the `Speakers` WASAPI endpoint being routed to a
different physical output than the jack the cable is plugged into on this
interface model. No media-device state changed and no persistent
configuration was altered by this run.

## Authoritative current-state reconciliation (2026-09-18)

This section supersedes older point-in-time notes above where they describe a
different checkout state. The current Git worktree is clean at commit
`81bd72b0` (`Park physical latency pending loopback cable`). The clean-tree
`tests/acceptance/m08-release.ps1` run passed unsigned optimized Rust/UI
artifact preparation and verification; its temporary outputs were removed.
Documentation validation currently reports 56 Markdown files and 225 local
links, and traceability covers 159 normative requirement IDs.

The remaining blockers are evidence or authority gaps, not uncommitted source:

- The AudioRouter-owned driver, PortCls, production-signing, and clean-machine
  driver track is future work under `docs/plans/future/` and is excluded from
  the active VB-Cable-first completion chain.
- NFR-01/NFR-02/QUAL-04 physical latency remains parked until a wired
  Focusrite output-to-input loopback cable is available. The USB PD200X
  acoustic retries were rejected and are not latency evidence.
- Attended M05 accessibility/scaling/first-run/drag-and-drop and M07 actual
  OS-transition delivery/native reopen remain unverified because the current
  computer-use inventory is `apps: []`, `browsers: []`.
- Plugin redistribution rights/full OS sandboxing and signed installer,
  clean install/upgrade/uninstall, and publication evidence remain open.

Until one of those external prerequisites changes, the next actionable work
is documentation/audit maintenance; the plan must not be archived or marked
complete on the basis of portable or unsigned evidence.

The 2026-09-18 capability recheck also found no `chrome`, `msedge`, or
`firefox` executable and no repository Playwright installation; the
computer-use inventory remained `apps: []`, `browsers: []`. A browser-level
test therefore cannot be started on this host, and even an available browser
would cover only the WebView/page surface—not Narrator, the native Tauri
window/tray, or actual Windows lock/sign-out/sleep transitions.

The subsequent full locked workspace regression passed with 36 CLI, 181
control tests plus four guarded-live tests ignored, 66 domain, 34 DSP, 118
engine, 70 plugin-host, 13 worker-process, 8 protocol, 40 recording, 92
storage, 19 transport, 88 Windows-audio tests, and all doc-tests. This
refreshes portable regression evidence only; the guarded-live tests remain
explicitly opt-in and do not close the physical, attended, or OS-transition
gates.

Strict static verification was rerun afterward and passed: `cargo clippy
--workspace --locked --all-targets --all-features -- -D warnings` and
`cargo fmt --all -- --check`. No source or generated artifact changes were
left by either check.

The native AudioRouter shell was subsequently exposed to the Windows
computer-use surface and an attended M05 check was performed without starting
audio. The initial shell showed backend ready, a stopped revision-0 session,
safe startup, monitoring muted/unarmed, labeled session/canvas/inspector
controls, and deferred virtual-device controls visibly disabled. The user
clicked `+ Test Signal`; the node appeared on the canvas and in the inspector
with bounded frequency, level, and duration controls. No plan was committed
and the session remained stopped. The draft was then discarded and the
original graph was confirmed restored. A maximized-window screenshot also
showed the session list, canvas, shelf, inspector, and draft controls in one
responsive layout. This is attended native-shell evidence for the click path
and safe first-run state; Narrator speech, Windows display-scaling settings,
and live drag-and-drop remain unverified.

The elevated `tests/acceptance/safe-all.ps1` chain was rerun after this
reconciliation and passed end to end on the clean tree. It covered M00
toolchain/native compile and 34-endpoint format inventory, M01 CLI, existing-
device M03 routing, M04 DSP/recording, M05 typecheck/19 UI files/260 tests and
temporary production build, M06 SDK/validator/VST3 worker/VST2 fixture gates,
M07 headless and frontend-owned shell RPC, unsigned M08 artifact preparation
and NSIS smoke, traceability, and documentation. The run reported 1,598
plugin-validator tests passed, 181 control tests with four guarded-live tests
ignored, a 4,895,384-byte disposable unsigned installer, 159 normative IDs,
and 56 Markdown files/225 local links. Run-owned temporary children and
installer output were removed. This refresh does not close the explicitly
listed attended, physical-latency, OS-transition, redistribution/sandbox,
signing, installation, or publication gates.

## Native drag-and-drop defect follow-up (2026-09-18)

The user reproduced that dragging a library control does not add it to the
canvas in the attended shell. The UI drop path was hardened for WebView/native
drag behavior: the shelf now supplies both the AudioRouter MIME type and a
plain-text fallback, while the editable canvas accepts drag-over events even
when the host exposes no drag MIME types and validates the payload at drop
time. A focused regression covers the no-types/plain-text case. UI typecheck,
261 Vitest tests, the production UI build, and `tests/acceptance/m05-ui.ps1`
all pass. Native post-fix drag verification remains open because the
computer-use RPC surface became unavailable after the rebuilt shell was
launched; do not count this change as attended drag evidence until the user or
an available native surface confirms a node appears after dragging.

The 2026-09-18 UI redesign slice, later archived as
[the completed UI redesign plan](../archived/2026-09-19-ui-redesign.md), was
built in commit `d3b2dbdf`. It adds an
Ambient-inspired shared lighting treatment, Audio Hijack-inspired compact
source/processor/output cards, inline bounded meters, an interactive
eight-band EQ preview, and telemetry-driven active edge animation. The UI
suite now passes 263 tests, typecheck, production build, and M05 acceptance.
Native-shell visual inspection at 1280×720/maximized remains open because the
attended computer-use surface is unavailable; do not treat the automated build
as visual acceptance.

The subsequent 2026-09-18 usability pass addressed four user-reported M05
defects: initial canvas fit after draft hydration, physical-input endpoint
selection in the node inspector, recorder format/approved-path visibility, and
inspector width/control layout. The UI typecheck, 263 tests, production build,
and M05 acceptance passed again. Native visual confirmation remains open.

## 2026-09-19 connection-handle fixes and UI redesign plan archival

The UI redesign sub-plan reached the end of its own implementation scope and
is archived at
[docs/plans/archived/2026-09-19-ui-redesign.md](../archived/2026-09-19-ui-redesign.md);
its unique remaining item (attended native shell inspection) duplicated what
this file already tracks below, so it is not repeated in two places.

Three defects were found and fixed in the connection-handle/canvas area while
reviewing whether any four-sided connector combination (e.g. input bottom to
output top) actually works end to end:

1. The connection target side was decoded twice — once correctly in
   `onConnect`, then again in `onConnectEnd` from an already-stripped handle
   id that had lost the side suffix — so every connection rendered as if
   dropped on the input's left side regardless of where it was actually
   released. Fixed by preserving the raw target handle id through to
   `onConnectEnd`.
2. A node with exactly one input and one output port placed both ports' four
   side handles at the same offset per side, so the later-rendered output
   handle always won hit testing and that node's input handle was
   ungrabbable on any side. Fixed by slotting every port's offset against the
   node's full port list instead of per direction.
3. The shared `actionMessage` status banner was silently clobbered: a
   `virtualBridge.expired` or `devices.bindingInvalidated` safety notice
   (explaining that audio had stopped) was immediately overwritten by the
   same automatic resync path's unconditional device-list refresh message.
   This was caught by two tests in `App.accessibility.test.tsx` that had been
   failing on `main` before this session (unrelated to the handle work).
   Fixed by making automatic/background device refreshes silent while
   keeping the manual "Refresh available inputs/outputs" button announced.

All three fixes, plus a handle-decluttering pass (dimmed idle handles,
brighten on hover, fade non-matching handles during an active drag), are in
`ui/src/SessionFlowCanvas.tsx`, `ui/src/App.tsx`, and `ui/src/styles.css`.
`npm run typecheck` is clean and the full UI suite now passes 262/262 (the 2
pre-existing failures above are fixed; no new failures introduced).

A new no-backend harness (`ui/harness.html` + `ui/src/devHarness.tsx`) was
added and is retained as a standing dev tool. It mounts `SessionFlowCanvas`
directly with a local demo session and `canEdit` forced on, which let the
target-side and handle-overlap fixes be verified with real mouse drags in
Chrome instead of only by code inspection. This does not substitute for
attended native-shell evidence: the harness proves the React Flow canvas
logic works in a browser, not that the packaged Tauri shell, Narrator,
Windows scaling, or the native WebView drag path behave the same way. Those
gates remain exactly as open as recorded in the audit boundary below; this
session closes a portable UI-logic and pre-existing-test gap, not an attended
one.

A follow-up pass gave each draggable library node kind its own purposeful
inline visual instead of a generic Ready/Bypassed/Disabled chip, closing a gap
against the UI redesign plan's own design decision that "every graph node has
... one useful visual." Gain, Delay, and Pitch shift now render a compact
draggable/keyboard-adjustable fader routed through the existing draft
parameter path. Mute renders a one-tap Live/Muted toggle. Compressor and
Limiter render a gain-reduction meter driven by the backend's existing
`nodeTelemetry[].processor.gainReductionDb` field, which was already defined
in the contract but unused anywhere in the UI. Gate renders an Open/Closed
indicator from the equally unused `processor.gateOpen` field. Mixer gets a
distinct sum-glyph visual, and Test Signal now shows its configured
frequency/level alongside its existing meter. `npm run typecheck` is clean
and the full suite still passes 262/262. All eight new visuals were verified
rendering and interactive (fader drag, mute toggle click) with real mouse
input against the no-backend harness above, seeded with synthetic telemetry
for the compressor/gate/limiter cases since no native backend is available
here to produce real gain-reduction/gate readings.

A follow-up audit asking "do they all have a useful, appealing node" found
two library nodes that still fell short: Mixer showed a static "Sums
connected inputs" label with no relationship to the actual graph, and
Recorder showed only a generic pass-through meter with no indication of
armed/recording/paused state — arguably the single most important fact about
a recorder node. Mixer now counts real incoming edges
(`session.edges.filter((edge) => edge.destinationNode === node.id).length`)
and shows "Summing N inputs" (or "No inputs connected"), plus its own meter
when the backend reports one. Recorder now looks up its `RecorderStatus` by
`nodeId` (plumbed as a new optional `recorderStatuses` prop from `App.tsx`'s
existing `recorderStatuses` state, not previously passed to the canvas) and
shows a labeled, colored state chip (Idle/Armed/Recording/Paused/Stopping/
Completed/Failed) with a pulsing dot while recording, alongside its meter.
Verified with the harness: seeded two edges into a Mixer node ("Summing 2
inputs" rendered correctly) and one `recording`-state `RecorderStatus`
("Recording" chip with red dot rendered correctly). Typecheck clean, full
suite still 262/262.

The remaining node kinds that are not part of the drag library itself
(`applicationCapture`, `endpointLoopback`, `plugin`, and the deferred
`virtualRenderSource`/`virtualCaptureSink`) were initially left at the
generic Ready/Bypassed/Disabled visual since they are not draggable library
entries — application capture and endpoint loopback nodes are created from
dedicated panels elsewhere in the UI, plugin nodes from the plugin-scan flow,
and the two virtual-bus kinds are disabled in the library pending the
deferred managed driver. The user asked for full coverage regardless, so all
five now have dedicated visuals too: Application Capture shows the captured
executable and instance-selection policy; Endpoint Loopback shows a
truncated bound endpoint ID; the two virtual-bus kinds show their bound bus
identity with a direction glyph; and Plugin shows a VST2/VST3 format badge,
the bound binary's filename, and an active/stopped/bypass state chip reusing
the existing `.node-state` styling. Every one falls back to its meter when
the backend reports telemetry for it. Every node kind in the domain now has
a purpose-built visual; only the fully generic fallback for any future
unknown kind remains. Verified in the harness with one of each kind
(including a seeded `voice-bus` virtual capture sink and a `ReaComp.vst3`
VST3 plugin); typecheck clean, full suite still 262/262.

The user then asked for the canvas to look as good as Audio Hijack and for
the app to at least match VoiceMeeter's core "redirect input, tool, and
output" workflow. The drag shelf (`.canvas-library`) was a flat,
alphabetized button list with no visual distinction between a source, a
processor, and a destination, which worked against both asks at once. Every
`LibraryEntry` in `ui/src/library.ts` now carries an explicit `flow: "input"
| "tool" | "output"` field, and the shelf renders three labeled, colored
groups (Inputs/cyan, Tools/amber, Outputs/green) with a colored monogram
icon per entry. The same three-way classification (`nodeFlowGroup()` in
`SessionFlowCanvas.tsx`, covering the full `NodeKind` enum) now sets a
matching colored left-edge accent on every node card via a new `className`
on the React Flow node object, so the canvas itself reads left-to-right as a
signal path the way Audio Hijack's board does, not just the palette next to
it. Verified in the harness: the shelf renders correctly grouped with
correct colors; clicking "Test Signal" (Inputs) and "Physical output"
(Outputs) both correctly added nodes (confirmed via
`onAddLibraryNode testSignal testSignal-1` in console output, and the new
node appearing after Tidy layout); node cards show the correct cyan/amber/
green left accent per group in zoomed screenshots. Connection-dragging logic
itself was not touched and was not re-verified in this pass since it was
already extensively proven working in prior sessions. Typecheck clean, full
suite still 262/262.

This closes the shelf/board half of "compete with VoiceMeeter Banana and
Audio Hijack" — it does not close the larger, unscoped ask. VoiceMeeter's
actual differentiator is flexible virtual-bus strip routing, which this app
already supports end-to-end through existing VB-Cable/Voicemeeter endpoints
per the VB-Cable-first evidence above; AudioRouter-owned virtual bus
provisioning remains the deferred driver-signing gate, unchanged by this UI
work. Audio Hijack has an entire feature surface (saved snapshots, a
hardware-style routing matrix, global hotkeys) this pass did not attempt.
Treat "make it look amazing" as addressed for the node/shelf visual layer;
a broader feature-parity gap analysis remains a separate, larger initiative
if wanted.

The user then asked, reasonably, whether they could avoid the signed-driver
requirement entirely by routing through their own existing VoiceMeeter
Banana install. The answer is yes, and this was already the supported
VB-Cable-first path documented and qualified throughout this file — the
signed-driver gate only applies to AudioRouter provisioning its *own new*
virtual endpoint (the deferred `virtualRenderSource`/`virtualCaptureSink`
kinds), never to binding a plain `physicalInput`/`physicalOutput` node to an
endpoint VoiceMeeter or VB-Cable already registered with Windows. That
distinction was true but not obvious in the product itself: the shelf had an
"Existing virtual output" entry for exactly this free path, but no input-side
equivalent, and neither the available entries nor the deferred ones said in
plain language that the free path existed. Library search for "voicemeeter"
also returned nothing.

Fixed: added a symmetric "Existing virtual input" library entry
(`kind: physicalInput`, same free binding, no driver) next to the existing
output one; both now carry a `note` field ("Binds to an already-installed
virtual endpoint such as VoiceMeeter or VB-Cable ... no AudioRouter driver
required") shown as a tooltip and matched by library search in both the
canvas shelf and the advanced Library panel; and the two deferred entries'
`unavailableReason` now explicitly says "For VoiceMeeter or VB-Cable today,
use Existing virtual input/output instead" at the exact point of confusion.
Searching "voicemeeter" in the library now surfaces all four related entries.
A new `library.test.ts` case pins this behavior. Typecheck clean, full suite
263/263 (was 262; +1 new test, and one existing assertion updated to match
the improved message text via `toContain` instead of exact match).

## 2026-09-19 permanent scope decision: no AudioRouter-owned driver

The user made an explicit, final scope decision: AudioRouter-owned virtual
endpoints (the managed driver, PortCls integration, and production signing)
are set aside indefinitely for cost reasons — a driver-signing credential is
not funded and none is planned. This is not a re-statement of the existing
"deferred pending signing" language; it changes that language from temporary
to permanent. Updated to match:
[`docs/plans/future/M03-driver-signing.md`](../future/M03-driver-signing.md),
[`docs/spec/06-virtual-devices.md`](../../spec/06-virtual-devices.md), and
[`docs/spec/15-delivery.md`](../../spec/15-delivery.md). VDEV-01/03/09 and
SEC-08 remain textually normative only for a possible future funded signed
track and are explicitly excluded from this project's v1 completion target.

The user's instruction was: complete "everything else" using the existing
VoiceMeeter/VB-Cable endpoint strategy (already the supported path, and the
subject of the prior session's library fix). Given the sheer size of this
codebase and the extensive completion claims already recorded throughout
this file, the next step before writing any more code is an evidence-based
audit — not blind trust in this log's own prior "implemented" claims — to
find any feature described in `docs/spec/*.md` or `docs/milestones/*.md`
that is not actually present in `crates/` or `ui/src/`, excluding: attended
Windows UI verification, physical/acoustic hardware measurement, the
now-permanently-set-aside driver/signing work, and legal/licensing gates
(plugin redistribution rights, OS sandboxing certification) that no amount
of code can resolve. That audit and its resulting task list are recorded in
the next entry below.

## 2026-09-19 audit result and two implemented backend-complete-but-UI-missing gaps

The audit found the Rust workspace genuinely matches this file's completion
claims: zero `todo!()`/`unimplemented!()`/TODO/FIXME hits anywhere in
`crates/` or `ui/src/` outside test fixtures, and every one of the 86
dispatched API methods in `crates/domain/src/lib.rs`'s catalog has real
dispatch logic in `crates/control/src/lib.rs`, not a stub. M04/M06/M07
completion claims were spot-checked directly against `crates/dsp`,
`crates/recording`, `crates/plugin-host`, and `crates/control` source and
hold up.

It found exactly two genuine gaps, both backend-complete with zero UI
surface: **committed graph history/undo** (`graph.history`, `graph.undoPlan`
— the React app only had local pre-commit draft undo, no way to view or
revert an already-committed revision) and **client authorization management**
(`clients.list`, `clients.authorize`, `clients.revoke` — no way to see or
revoke a connected CLI/MCP client from the app, which matters directly for
SEC-01–06 multi-client scoping). Both are now implemented: `backend.ts`
gained `listGraphHistory`/`undoGraphPlan`/`listClients`/`authorizeClient`/
`revokeClient`, wired to the existing live-client `request()` pattern and to
safe disconnected-backend defaults. `App.tsx` gained a `GraphHistoryPanel`
(lists committed revisions; "Undo last committed change" reuses the existing
`graph.plan`→`graph.commit` flow, since `graph.undoPlan` always targets the
single revision immediately before the current one, confirmed from
`domain::Store::undo_plan`'s `.rev().nth(1)`, not an arbitrary past
revision) and a `ClientsPanel` (list/authorize/revoke). A genuine, separate
contract-drift bug was found and fixed in the same pass: `MethodParams`
for `clients.authorize`/`clients.revoke` in `contracts/src/index.ts` were
missing the `idempotencyKey` field that `crates/control/src/lib.rs`'s
dispatch functions actually require — the drift checker only validates
method-name parity, not per-field param shape, so this had never been
caught. Five new focused tests added to `App.accessibility.test.tsx`.
Typecheck clean, full suite 268/268 (was 263), contract drift still passes
(86 methods, 20 node kinds, 7 processors, 20 event categories).

**A far more significant finding surfaced while visually verifying the two
new panels in a real browser (not jsdom, which never applies real CSS):
they rendered but were completely invisible, `display: none`.** Tracing why
found that an entire class of already-implemented, already-tested panels
was silently unreachable in the shipped UI, not just the two new ones:
`ProcessorCatalog`, `PresetCatalog`, `SessionTransferPanel`,
`PluginScanPanel`, `StartupPanel`, `OsTransitionPanel` were hidden by an
unconditional `.main-content > .panel:not(.inspector) { display: none; }`
rule in `styles.css` with no escape hatch at all — and separately,
`RecorderActions` (the entire M04 recorder create/arm/start/pause/split/stop
UI), `RecordingActions`, `VirtualDeviceLifecyclePanel` (the M03 managed-bus
UI), and `VirtualRoutePanel` were gated behind an `.app-shell.compact-status`
class that could only be toggled by a "Compact status" button — which was
*itself* hidden by a separate rule (`.topbar .status-cluster > .secondary
{ display: none; }`). The theme picker (`.theme-picker`, the only way to
satisfy UI-11's dark/light/high-contrast requirement) was hidden by the same
rule. `grep`-ing `App.tsx` confirmed `setCompactStatus` and `setTheme` had
no other call site — no keyboard shortcut, no alternate control. **The
result: theme switching and this entire set of already-built, already-tested
features were completely unreachable by any real user, through any path, in
the shipped UI**, despite this file's own extensive log claiming them
complete — because none of the automated tests check real CSS visibility, so
nothing caught it. This is almost certainly an unfinished half of the
2026-09-18 UI simplification pass, which intended (per its own `styles.css`
comment) that these controls "remain available through... list view... and
selected-node inspector" — a claim that was already inaccurate for most of
them — and left an unused `.advanced-tools`/`<summary>` disclosure pattern in
the CSS that was never wired into `App.tsx` to reveal them again.

## 2026-09-20 full stack code review

The user asked for a full code review, backend then frontend. The `code-review`
skill is diff/PR-oriented and found nothing to review (the working tree was
clean and the last commit didn't touch `crates/`), so this was done manually:
parallel review forks for the backend crates (`domain`+`protocol` and
`control` succeeded cleanly; the `engine`+`dsp` fork got derailed by its own
inherited context — it spent its entire budget reasoning about the earlier
rate-limit failures instead of reviewing anything, a real risk worth knowing
about when forking after a notification-heavy stretch of conversation), then
direct grep-first review of the remaining crates and the full `ui/src` tree
by the top-level session once fork reliability became suspect.

**Backend (~77k lines across 11 crates): no confirmed correctness or security
bugs.** Verified, not just inspected: the named-pipe transport's same-user
claim (real owner-only ACL plus an explicit SID comparison before any request
is read, no TOCTOU, non-Windows stub fails closed); plugin-host's identity
revalidation (every real `control` call site uses the `*_verified_*` spawn
path, never the unverified one); storage's ZIP bundle import (correct
component-based `..` detection, not string matching, plus symlink/reparse
rejection); and engine's `RealtimeDsp` lock-free gate (correct Acquire/Release
ordering with a drop-guard, and fail-closed contention handling applied
consistently everywhere traced). One minor, non-bug duplication note in
`protocol/lib.rs`'s two near-identical frame-decode functions.

**Frontend (~8,200 lines): one confirmed bug, now fixed.**
`ui/src/draft.ts`'s `appendDraftConnection` built its default channel matrix
with `Math.min(destinationChannel, sourcePort.channels - 1)`, which correctly
fans a mono source out to a stereo destination but, for the reverse case,
only ever gave the source's first (left) channel any gain — connecting a
stereo output into a mono input silently dropped the right channel entirely,
with no downmix and no warning. Every other frontend file was read in full
(all small utility/panel files, `host.ts`, `graphView.ts`, `layout.ts`,
`processorCatalog.ts`) and found clean; `App.tsx`/`backend.ts`/
`SessionFlowCanvas.tsx` were not re-read fresh line-by-line since they were
extensively edited and live-browser-verified earlier this same session.

Fixed by extracting a `defaultChannelMatrix(sourceChannels, destinationChannels)`
helper in `draft.ts`: when the source has more channels than the destination,
every destination channel now averages all source channels (gain
`1/sourceChannels` each) instead of keeping only the first. The mono→stereo
and equal-channel-count cases are unchanged (still identity/duplication).
One pre-existing test (`draft-connection.test.ts`) had pinned the buggy
`[1, 0]` default for a stereo→mono case inside an `insertDraftMixer` round
trip; corrected it to the now-correct `[0.5, 0.5]`, and added a direct
`appendDraftConnection` integration test plus four `defaultChannelMatrix`
unit tests covering all four 1/2-channel combinations. Typecheck clean, full
suite 273/273 (was 268; +5 new, 1 corrected).

## 2026-09-20 VST plugin hosting completion, archived

The VST plugin hosting sub-plan (shelf-reachable add/remove/manage, canvas
VST-vs-native visual identity, mixed physical/application multi-input mixing,
isolated-worker health visibility, and edge quick-insert) reached the end of
its own implementation scope and is archived at
[docs/plans/archived/2026-09-20-vst-plugin-hosting.md](../archived/2026-09-20-vst-plugin-hosting.md),
including a dispatch-blocking `validate_method_params` allow-list bug found
and fixed during a full post-implementation review. No attended Windows
evidence was produced or claimed; every check was portable (`cargo test
--workspace`, clippy, the UI suite, typecheck, the contract drift checker,
and the docs validator), so this closes a portable-completeness gap only, not
any attended gate tracked elsewhere in this file. Final counts: control 179
tests (was 177), engine 119 (was 118), plugin-host 71 (was 70), full UI suite
282/282 (was 273).

## 2026-09-20 closed one self-identified gap: multi-input worker plugin/processor telemetry

Follow-up to the archived VST plugin hosting plan above. Asked to scope what
was independently achievable without the deferred driver or attended UI
verification, an audit of the active plan's own tracked non-driver gaps
found most genuinely require hardware, physical measurement, legal review,
or a clean release worktree — none of that is something this session can
do. One gap was a real, portable, code-only completeness item the archived
plan's own "known limits" section had just flagged: the separate multi-input
mixer worker never reported any node telemetry at all (unlike the default
single endpoint/duplex worker, which already reports meter/processor/plugin
telemetry), because `CompiledMixerFanoutGraph` had no telemetry accessors —
only the endpoint-worker's `RuntimeGraph` did.

Fixed by adding `processor_telemetry_for_node`/`plugin_health_for_node` to
`CompiledMixerFanoutGraph` (delegating to its existing `processing_graph:
Option<RuntimeGraph>`, the shared post-mixer chain) and mirrored wrapper
methods on `RealtimeMixerFanout` and `NativeMultiInputWorker`
(`crates/engine`, `crates/windows-audio`). `crates/control` gained a
`native_multi_input_node_telemetry()` mirroring the existing
`native_node_telemetry()`, merged into the same `nodeTelemetry` diagnostics
array (the two native worker types are mutually exclusive per session, so
concatenation is safe and one side is always empty). No contract or UI
change was needed: the UI already looks up telemetry generically by node ID
into one shared array, so a plugin running inside a multi-input session's
post-mixer chain now shows worker health in the canvas, inspector, and
"loaded plugins" list exactly like the default single-chain path, with zero
UI code changes.

Scope explicitly not attempted: adding meter instrumentation for the mixer's
input sources, its own combined signal, or its output branches. None of that
existed before either, and building it is a separate, larger
realtime-instrumentation task (new lock-free meter state threaded through
`RealtimeMixerFanout::pump_inputs`/`process`), not a small extension of what
already existed for the endpoint-worker path the way processor/plugin
telemetry was.

Everything else on the active plan's non-driver list — attended
accessibility/scaling/drag-drop verification, real OS-transition delivery
(lock/sign-out/sleep/resume), physical latency/hardware calibration, VST
redistribution rights and a broader third-party compatibility matrix, and
release artifact preparation (blocked by the current dirty worktree) —
was assessed and is *not* something this session can close alone: each
needs either the user's interactive machine, real hardware, or a legal/rights
decision. The existing Job-Object-based worker sandbox (kill-on-close,
single-process limit, 512 MB memory cap) was also reviewed; it already
provides real resource/lifecycle containment, but true filesystem/network
sandboxing (e.g. AppContainer or a restricted token) is a separate,
security-sensitive design task, not something to bolt on quickly, and was
correctly left out of this pass rather than attempted partially.

Verification: `cargo test --workspace --lib` all green (engine 120, was 119;
control 179 unchanged; every other crate unchanged), `cargo clippy
--workspace --all-targets` zero warnings, `tools/contracts/check-drift.mjs`
clean, UI typecheck clean, full UI suite 282/282 unchanged. Nothing
committed; awaiting the user's go-ahead to push.

## 2026-09-21 goal kickoff: MP3, installed-plugin scan, and M08

The user authorized completion of all three remaining workstreams, beginning
with the easiest bounded investigation. The installed-plugin gap was checked
before any mutation: `C:\Program Files\VSTPlugins\Reaication` is absent, while
`C:\Program Files\VSTPlugins\ReaPlugs` exists with nine DLLs. The existing
read-only installed-VST2 acceptance passed for
`reaeq-standalone.dll` at 44.1/48/96 kHz and both editor-containment checks,
with its SHA-256 unchanged. The scanner/UI root and classification path still
need tracing; no conclusion that the directory scan is correct or defective
has been made. Next: reproduce the scan through the shared API, then implement
the smallest evidence-backed fix or document the precise unsupported boundary.

The scan reproduction is now complete: the actual ReaPlugs root returns all
nine DLLs as x64 VST2 `supportedVst2X64Gated` entries with no errors. The
reported Reaication gap is a path/name mismatch, not a scanner defect; no
code change is warranted.

MP3 implementation progress: added the bundled `mp3lame-encoder` Windows-safe
worker path, bounded queue drain/finalization, non-overwriting `.mp3` path
creation, finalized library metadata, preview structural validation, CLI and
JSON schema format values, and UI format selectors. MP3 is fixed 192 kbps,
supports mono/stereo 44.1/48 kHz, disables dither, and intentionally rejects
split until a gapless part policy is specified. Recording and control tests
cover encoder output, queue frame boundaries, structural inspection, factory
selection, and metadata. The bundled `mp3lame-encoder` dependency reports
LGPL-3.0; release artifacts must include the corresponding notices/source-
offer compliance before publication.

M08 verification on 2026-09-21: documentation validation passed for 58
Markdown files and 264 local links; traceability covered 159 normative IDs;
contract drift passed; CLI tests passed (36), control tests passed (180),
recording tests passed (43), UI typecheck passed, and the UI suite passed
(282). Unsigned installer smoke is blocked by npm registry access/permission
(`@tauri-apps/cli` fetch returned `EACCES`), so no installer or upgrade/
uninstall evidence is claimed. Release artifact preparation is also blocked by
its intentional clean-working-tree guard; the user explicitly requested that
these uncommitted changes remain in place. Signing, clean install/upgrade/
uninstall, hardware/application matrix, and publication remain M08 gates.

## 2026-09-20 ran the portable safe acceptance chain; found and fixed a real `cargo fmt` gap

Asked to keep doing whatever was autonomously possible, ran the project's own
documented "safe" acceptance scripts (native compile, VST3 SDK build +
validator + offline loader, native VST3 worker, VST2 chunk-state fixture,
CLI, headless, M08 traceability, docs) — the exact set the repository
defines as running with no live audio, no driver/installer changes, and no
machine configuration changes, so it was safe to run unattended. All of it
had never been run this session; `cargo test`/`vitest`/clippy/drift-check
are not the same invocation path as these wrapper scripts (they also cover
the native C++ VST3 SDK/CMake build, which cargo never touches).

Result: everything passed except `m04-dsp-recording.ps1`, which failed on
its embedded `cargo fmt --check` step — five files from this session's Rust
edits (`crates/cli`, `crates/control`, `crates/engine`, `crates/plugin-host`)
had never been run through `cargo fmt`. This was a real, if low-severity,
gap: nothing was behaviorally wrong, but the tree did not actually pass the
project's own formatting gate. Fixed by running `cargo fmt` (mechanical,
zero behavior change) and confirming `cargo fmt --check` passes; re-ran the
full workspace test suite, clippy, the drift checker, and the specific
`m04-dsp-recording.ps1` script afterward, all clean.

Everything else confirmed passing and unaffected by this session's work:
M00 native probe compile, M00 native format inventory (34 endpoints), M01
CLI, M06 VST3 SDK (pinned checkout, build, validator — 1,598 + 94 tests,
offline loader, AGain main/auxiliary-bus classes, five-class mda matrix),
M06 native VST3 worker (isolated single-stream/auxiliary-bus processing,
restart/quarantine recovery, state restoration, shutdown), M06 VST2
chunk-state/legacy-main fixture at 44.1/48/96 kHz, M07 headless (CLI 36,
control 179, plugin-host 71, worker-process 13), M08 traceability (159
normative requirement IDs covered), and documentation validation.

This closes out the "do what you can" pass: every other open item on the
active plan's non-driver list genuinely requires the user's interactive
machine, real hardware, or a rights/legal decision, and was not attempted.
Nothing committed; awaiting the user's go-ahead to push.

## 2026-09-22 pushed MP3 changes; mic route requalified after reconnect

Committed and pushed the current MP3 recording and acceptance work as
`08c6b982` (`Add MP3 recording and guarded microphone acceptance`). The
installed-plugin read-only check had found the expected ReaPlugs directory;
the live scenario was then attempted with the guarded NFR-02 wrapper.

The first attempt ran from a non-elevated shell and stopped before opening
audio because Windows denied the required before-run PnP media snapshot. A
retry with elevated execution access passed the snapshot and ran the bounded
AudioRouter `adapter-control-route` for 8 seconds: 801 packets, 384,480
captured frames, 3,003 processed quanta, 384,384 rendered frames, and a
finalized 1,537,580-byte temporary recording. The native probe submitted
314,016 render frames and emitted 654 impulses, but detected zero impulse
groups on either capture stream (`pairs=0/500`), so the wrapper correctly
failed without reporting latency. The user reported that the loopback was
disconnected during this run; treat this as a known setup-related,
non-qualifying attempt, not an NFR-02 regression. Temporary route/probe
artifacts were cleaned by the wrapper. Reconnect the loopback before repeating
this live scenario.

The explicit installed ReaEQ worker acceptance passed at 44.1, 48, and 96
kHz, plus both editor-containment checks. The selected DLL's SHA-256 remained
`c200e540c26ac793b43611aaceb4aa42cdd2829cdfbf0d60494716d9bdde8a7d`.
This confirms the isolated plugin worker separately; it does not prove a
single live microphone graph with ReaEQ inserted.

After the user reconnected the loopback, the same guarded wrapper passed:
`pairs=500/500`, p95 117.403 ms (min 116.139, p50 116.848, max 128.688 ms),
within the 160 ms target. It detected 627 physical-mic and 614 virtual-capture
impulse groups, with zero dropped frames. The internal route reported 800
packets, 384,000 captured and rendered frames, 3,000 processed quanta, and
1,536,044 finalized recording bytes. Its one `rejected_pumps` count is the
acceptance's deliberate stale-generation rejection check before normal
generation pumping, not an audio-path failure. This requalifies the real
microphone-to-VB-Cable route on the reference setup.

The combined live graph check is completed below. Attended UI interaction and
receiving-application listening remain unverified because the computer-use
surface exposed no apps; the earlier idle shell was closed and its temporary
database cleaned without starting audio. The unsigned installer smoke remains
an M08 packaging gate and is not a prerequisite for MP3 recording behavior.

## 2026-09-22 M02/M06 end-to-end installed-plugin workflow

Objective: make installed supported plugins usable end to end: explicitly
scan/select an exact plugin, add/insert it as a graph node, inspect/edit its
parameters in the AudioRouter UI, bind it to the isolated worker, and prove
that a non-default plugin setting changes signal in a real microphone route.
The native vendor editor window is a separate capability; generic parameter
editing is the required configuration path for this task unless existing
contracts reveal a safe already-supported editor route.

Requirements: CAP-02/03, GRAPH-01/06/14, DSP-01, PLUG-01/03/05/07,
SEC-07, NFR-02, QUAL-01/02/03, ENG-04. This is a single reference-machine
integration experiment; it does not qualify all plugins or close the broader
M06 rights, sandbox, W2 timing, or compatibility gates.

Prerequisites: Windows 11 x64, elevated PowerShell for the existing before/
after media inventory, the connected reference microphone loopback, active
Focusrite capture and VB-Cable render/capture endpoints, the existing isolated
plugin worker, and the exact local `reaeq-standalone.dll` whose recorded hash
is `c200e540c26ac793b43611aaceb4aa42cdd2829cdfbf0d60494716d9bdde8a7d`.
The user explicitly authorized live microphone routing. Do not change Windows
defaults, endpoint volume/mute, privacy, driver, or startup state.

Decision: keep the existing non-plugin acceptance behavior unchanged. Use the
existing explicit local scan, draft-node insertion, generic parameter panel,
verified worker binding, and graph processing boundaries; repair only
evidence-backed gaps. VST2 metadata does not publish VST3-style class IDs, so
the graph's existing `default` placeholder token is not plugin identity:
binary path plus the freshly rescanned SHA-256 remain authoritative for VST2
worker authorization. Do not synthesize or present a plugin-reported class ID.
The plugin remains user-installed and is never copied or modified. The guarded
live proof must set a real non-default parameter; a controlled same-input
real-DLL worker regression must demonstrate a processed-signal difference.
Both require worker health and exact binary identity evidence.

Ordered tasks and outcome:

1. Trace and test scanner → UI picker → draft insertion/splice → parameter
   descriptors/editor → commit → runtime worker binding and parameter events.
   Done for the exercised path: VST2 placeholder insertion accepts an empty
   static class-ID list via the same neutral `default` authoring token already
   used by the UI; the scanned canonical binary path and SHA-256 remain the
   actual worker identity. The focused UI test now selects this entry, inserts
   it, loads worker-described generic controls, and changes `1-Gain` in draft.
2. Complete the bounded `adapter-control-vst2-route` live route. Its chain is
   microphone capture → gain → enabled ReaEQ worker → recorder/output branch;
   support a selected non-default plugin parameter. Done; the exact ReaEQ
   route binds parameter ID 1 (`1-Gain`) at normalized 0.75. A real-DLL worker
   regression confirms that changing an exposed `*-Gain` value changes
   processed samples for the same input.
3. Extend the guarded NFR-02 PowerShell wrapper with explicit `-PluginPath`;
   retain exact endpoints, media snapshots, cleanup, latency checks, binary
   fingerprint checks, worker health, and optional described parameter input.
   Done; the route uses the exact scanned identity and checks worker health,
   fingerprint, parameter acknowledgement, paired output, and the NFR-02 gate.
4. Verification completed for this bounded slice: locked probe build, focused
   UI test/typecheck, control and plugin-host suites, opt-in real ReaEQ worker
   parameter-difference test, and guarded live runs with default/non-default
   ReaEQ and ReaComp. Exact results are in [M00 probe evidence](evidence/M00-wasapi-probe.md)
   and the [plugin compatibility snapshot](../../operations/plugin-compatibility.md).

Validation matrix: scanner and UI identify exact supported VST2/VST3 format,
draft insertion keeps identity/path/fingerprint, parameter changes traverse
the backend to the isolated worker, runtime telemetry stays healthy, and the
graph remains fail-closed. Probe builds with `--locked`; no-plugin M02
acceptance stays unchanged; plugin mode rejects unsupported/non-x64/changed
binaries. Live run requires all 500 signal pairs, p95 ≤160 ms,
`plugin_worker_state=running`, zero worker failures, and unchanged DLL/media
identity; the opt-in worker regression must show finite output that differs
for a non-default parameter.
Any failed guard yields no pass claim.

Result: the initial plugin route at a two-slot queue depth was healthy but lost
enough correlated output for only 405/500 pairs. Increasing the already-bounded
plugin handoff to its existing eight-slot maximum recovered the signal path.
The final non-default `1-Gain=0.75` live route passed 500/500 pairs at
139.115 ms p95 (160 ms threshold), with the verified ReaEQ worker running at
zero failures and its SHA-256 unchanged; an earlier repeat was 136.783 ms.
The default-parameter plugin route also passed at 123.773 ms p95, and the
unchanged no-plugin route passed 500/500 at 115.155 ms p95 after the change.
The production route uses an eight-slot
preallocated queue to absorb native capture packet bursts; callback behavior
remains nonblocking and fail-closed.

Risks: VST2 remains a gated local extension; no change here establishes rights,
vendor-editor window support, W2 callback timing/soak, or broad plugin
compatibility. A signal metric plus a real-DLL worker parameter regression
proves processing alteration, but does not replace attended UI interaction or
listening in the receiving application.

The full locked workspace tests and full UI suite passed. Strict all-features
workspace Clippy currently reports an existing `too_many_arguments` lint in
`finalized_mp3_recording` (`crates/control/src/lib.rs`, unrelated to plugin
hosting); the plugin-host package and M00 probe Clippy checks passed cleanly.

Rollback: the new invocation is opt-in and separate; revert only the plugin
workflow/probe changes if they fail or perturb existing M02 behavior. The
original no-plugin route and its evidence remain intact. Next action: retain
this ReaEQ workflow as the reference acceptance, wire a tested native editor
window only if that richer UX is needed, and qualify attended selection/commit
plus listening when the shell is targetable. Do not infer release availability
from this reference-machine result.
