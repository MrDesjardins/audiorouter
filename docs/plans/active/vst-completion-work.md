# VST compatibility completion — work plan (2026-11-14)

Status: execution plan for the remaining work in
[full VST compatibility](vst-full-compatibility.md). It covers P5 sidechain,
P2/P3 conformance and P6 editor completion, then the P9 release gate. The
review fixes already on `feature/vst-plugin-experience` are the starting point;
they are not repeated here.

## Objective

Backend assembly checkpoint (2026-11-14, VSTC-02/03, PLUG-01/03/05):
`start_plugin_bridges` now returns typed standard/sidechain bridge handles.
Ordinary parameter/editor/MIDI registries retain their existing standard
bridges; the compiler receives either implementation through the common
processor trait. Auxiliary nodes are prepared separately and excluded from
legacy group startup. Only nodes with a current preparation request may start
a legacy worker. Main width selection excludes the key port.

The sidechain preparation helper rehashes under the configured root, selects
the explicit VST3 class, restores supplied state in discovery, obtains the live
binding, reaps discovery and launches the exact bound class. Supplied state is
restored again in the multi-bus worker and its bus layout/index checked before
publication. Initial parameters go to the worker owner's bounded template.
Each bridge has its own private scheduler epoch; queues are not reused across
bridge replacements. VST2 auxiliary execution remains refused.

Real AGain regression `backend_prepares_a_verified_sidechain_for_the_real_compiled_route`
passed on Windows using this worktree's debug native worker and the SDK AGain
bundle. It refuses the main-only class, prepares the SideChain class through
the actual backend assembly function, compiles a two-source main/key route and
receives finite nonzero worker output over 40 quanta without opening devices.
It also covers a key-first saved port order and a nonempty initial parameter
template. State restoration code is present but this regression uses no state
asset; restore/layout-change qualification is still pending.

The first run exposed a class-selection error caused by missing optional scan
metadata: backend preparation now forwards the explicit class to the verified
binary's factory, which must accept or refuse it (the prior no-substitution
regression remains authoritative). The next run exposed a legacy grouping
panic on the already-prepared sidechain node; startup now skips groups outside
its request set. The same regression passed after both repairs.

Windows checks: focused real-worker regression passed; full control library
402 passed / 19 opt-in ignored; strict control/plugin-host all-target/all-feature
Clippy passed; workspace/shell formatting passed. Public preparation still
refuses saved sidechain playback, now naming the actual remaining editor/control
and protected-path integration rather than claiming the bridge is absent.
No adapter method or UI control was exposed, no commit/push. Next: sidechain
control/health/state lifecycle and protected unchanged-audio policy, source/tool
exhaustive coverage, then dispatcher/MCP/UI integration and removal of the gate.
Rollback restores the standard return type and removes auxiliary assembly/helper
and the focused regression; keep the public preparation gate until integrated.

Connected key scheduling checkpoint (2026-11-14, VSTC-03, GRAPH-01/02/06/15):
the native path compiler routes any sidechain-bearing component through the
cached DAG compiler, including direct routes with no Mixer. Key edges remain
topological dependencies but are excluded from main-input convergence. Each
plugin cache has a separate preallocated key block and its authored channel
matrix; the key can read a source or an earlier processed/Mixer cache. Missing
key connections provide zeros, while an unavailable connected key clears only
that plugin contribution. Bypassed/disabled plugins skip key processing and
retain main-only dry semantics.

Main and key source cache borrows are released separately before processing,
so fan-out from the same source into both inputs does not collide with itself.
The synthetic processor chain carries main ports only, then restores explicit
SidechainPlugin instructions by node identity; this keeps ordinary width
adaptation from rewriting the key width. Main port selection is by semantics,
not saved port order. Existing output cache readiness and final privacy mute
remain in force.

The new deterministic compiler regression exercises the desktop's
`compile_native_paths_with_plugins_and_audio` entry: mono/stereo key widths,
key-first saved port order, key disconnected/connected, main-only bypass and
disabled flags, a shared raw source, a processed source and a shared authored
Mixer. It verifies exact main attenuation, unchanged independent game output,
one plugin execution across fan-out and direct no-Mixer compilation. These
are deterministic processor tests, not native VST or endpoint evidence.

Windows results: focused compiler test passed; engine library 187 passed /
1 explicit private fixture ignored; source and tool combination suites 6/6
each passed; strict engine Clippy, control all-target compile, workspace/shell
formatting and diff checks passed. The existing combination suites do not yet
enumerate every new sidechain source/processor arrangement. No windows/devices
were opened. Backend preparation still refuses saved sidechain ports; API,
MCP, UI, dynamic latency/control and real compiled-route qualification remain
pending. Next: prepare verified sidechain bridges in the control plane and
extend the source/tool suites before removing that refusal. Rollback reverts
the optional cache field, compiler routing and focused regression together.

Graph instruction checkpoint (2026-11-14, VSTC-02/03, PLUG-03/05): added an
explicit `ProcessingStage::SidechainPlugin` and the separate processor/graph
`process_with_sidechain` entry points. Legacy adapters default to silence on
this entry, never dry/main processing. Ordinary plugin instructions still use
their original method. Plugin diagnostics, reset and latency accounting recognize
both instruction variants.

`plugin_host::sidechain_bridge::SidechainRuntimeBridge` now implements the
realtime processor adapter using the preallocated stage and off-callback worker
owner. It accepts native VST3 multi-bus workers only, enforces frame/queue
bounds, assigns monotonic quantum identities, reports latched failure health,
and exposes initial plugin latency plus one quantum of pipeline delay. Worker
construction/latency query/IPC and owner join remain off-callback. The control
plane must retain the owner through graph retirement; dropping it joins the
worker. Dynamic-latency changes and live parameter/editor control on this
bridge are still pending. Graph reset retires pending audio without resetting
the worker's sequence, so pre-activation results cannot be published afterward.

Real AGain evidence: the class-selection test now constructs a bound worker,
starts the runtime bridge, puts it in an actual `RuntimeGraph`, and calls the
graph with separate main/key blocks for 20 quanta. Finite nonzero worker output,
unchanged key storage, Running health and pipeline latency passed. This opens
no device/editor and is not a user-route compiler test. Focused regressions
also verify legacy-adapter fail-closed behavior and reset rejection of queued
old audio. Five focused sidechain tests passed. Full engine library before
the final reset regression: 185 passed / 1 private fixture ignored. Existing
source/tool combinations: 6/6 each. Strict engine/plugin-host Clippy,
control all-target compilation, and workspace/shell format checks passed.

Still pending: the connected graph compiler must schedule the key's separate
source/cache branch, call this graph entry and handle bypass/disabled flags
with main-only dry semantics. Backend preparation remains blocked until that
path, health/control integration and every source/layout regression are ready.
No REST/MCP/UI exposure, commit or push. Rollback removes the new instruction,
trait method, adapter module and tests together. Next: connected-graph key
branch compilation; do not represent this checkpoint as playable sidechain
support for ordinary sessions.

Realtime handoff checkpoint (2026-11-14, PLUG-03/05, GRAPH-14, VSTC-03):
added `engine::sidechain::SidechainStage`, backed by the existing preallocated
`RuntimeBusScheduler`. Construction validates two inputs/one main-width output,
generation, frame budget and at most eight queue slots. Processing submits one
coherent main/key identity, substitutes a preallocated zero block only for an
unconnected key, and copies only the previous quantum's matching main result.
No plugin IPC occurs on this path. Runtime scratch ownership uses `try_lock`
only; contention, wrong shapes, queue saturation, missing/mismatched output
and repeated sequences fail closed. Catch-up discards stale results with a
hard eight-slot limit, preventing a missed result from leaving the pipeline
permanently one identity behind. The stage introduces one quantum of pipeline
delay; compiler latency accounting is pending.

Evidence on Windows: three focused stage tests passed (bus isolation and silent
unconnected key; late-result recovery; contention/wrong key width). The new
`sidechain_handoff_never_allocates_for_ready_late_or_full_queues` test uses the
existing thread-local allocation counter and measured zero callback allocations
across 96 quanta, including successful publication and saturated queues.
Engine library: 184 passed / 1 explicit private-session test ignored. Source
combinations: 6/6; tool combinations: 6/6 including the allocation regression.
These existing layout suites still do not compile plugin sidechain routes.

The shared worker test helper now exercises the actual stage against both the
echo owner and the native AGain async owner. Both passed, the latter via
`cargo test -p audiorouter-plugin-host --features test-fixtures --test worker_process verified_native_vst3_async_bus_worker_bridges_the_graph_scheduler -- --ignored --nocapture`
with the existing AGain/worker environment. Initial echo test failed because
the helper reused sequence 9 after the fixture had processed sequence 42;
the worker correctly rejected it. Helper sequences are now 100/101 and the
echo and native reruns passed. No protocol validation was relaxed.

Strict engine/plugin-host Clippy, workspace/shell formatting, docs and diff
checks passed. No audio endpoint or editor opened. The stage is not yet in the
graph compiler schedule: key-source compilation, processor flags and latency,
all-layout sidechain regressions, failure-latch/health plumbing and API/MCP/UI
remain pending. The backend preparation refusal stays. Rollback removes this
isolated stage, its export and the new tests/helper calls. Next: integrate the
stage with prepared graph instructions and compile the separate key branch.

Backend transaction checkpoint (2026-11-14, GRAPH-01/02/03/09, VSTC-01):
`ControlPlane::plan_bound_plugin_sidechain` now takes the non-serializable
worker binding, clones the current session and checks the target Plugin's
fingerprint, VST3 class and existing main input/output widths. It adds only the
named input port, retaining all existing edges, or refuses a conflicting
existing key width. It delegates to the normal graph planner for full-session
validation, revision conflicts and durable plan handling. No saved graph is
mutated during planning and no worker/audio generation is published.

The new opt-in control regression obtains a binding from the actual VST2
fixture, scans through `ControlPlane::dispatch`, prepares the bound-port plan,
then commits/replays/undoes through the normal dispatched graph API. It verifies
the unchanged pre-commit session, one committed port, idempotent replay, stale
revision refusal, and exact restoration of prior ports/edges. Wrong fingerprint
and main width fail without changing their sessions. Windows command:
`cargo test -p audiorouter-control --lib discovered_sidechain_port_plans -- --ignored --nocapture`,
with `AUDIOROUTER_TEST_VST2_WORKER` set to the rebuilt debug worker and
`AUDIOROUTER_TEST_VST2_BUS_FIXTURE` set to the isolated bus-fixture DLL; passed.
Strict control all-target/all-feature Clippy, workspace/shell formatting,
documentation validation and diff checks passed.

Full `cargo test -p audiorouter-control --lib` also passed: 402 passed,
18 opt-in tests ignored, no failures (Windows). The new real-worker transaction
test is one of those opt-in tests and passed separately as recorded above.

This is a backend building block, not a new REST/MCP method. Adapter access,
proof freshness across a queued plan, rejection of unverified raw port edits,
port/edge deletion evidence and user-authored graph playback remain pending.
The preparation guard is retained; no playable sidechain or completed VSTC-01
claim is made. Next: wire the realtime processor/graph input path, then expose
the verified transaction through API/MCP/UI with authorization and freshness
checks. Rollback removes the new planner entry and its integration test only;
it leaves prior saved-session validation and playback refusal intact.

Binding prerequisite found 2026-11-14 (PLUG-01/03/05, VSTC-09): an explicit
VST3 class ID absent from the current binary silently falls back to another
effect. Real Dragonfly regression `a_missing_requested_class_never_substitutes_another_effect`
fails against the existing worker: a zero class ID still loads successfully.
Before binding graph ports to discovered metadata, remove that fallback, then
exercise both AGain audio classes and require distinct main/aux layouts.
Validate native worker builds, real-worker tests and strict Rust checks.
Rollback is the worker selection change and its regression; retain the
unavailable sidechain graph guard throughout. No new public API is introduced.

Implemented and verified: explicit absent VST3 class IDs now refuse with a
rescan/reselect diagnostic. The real Dragonfly regression failed before the
change and passed after rebuilding the native x64 worker. AGain's main and
sidechain audio classes were each explicitly selected in a fresh worker and
reported `[2]` versus `[2,1]` input layouts respectively.

Added `SupervisedWorkerProcess::verify_sidechain_binding`: rechecks the current
binary against allowed roots, expected fingerprint, explicit VST3 class and
requested main/key widths, then queries the live worker. It returns an internal
non-serializable `VerifiedSidechainBinding` with private fields and read-only
accessors. Only exactly one main and one auxiliary input with the expected
main output is accepted. Wrong fingerprint/class/width, an effect without aux,
instruments and the legacy implicitly selected multi-bus constructors are
refused. The VST2 labeled-pin worker also produced a binding with native first
key pin 2. This is the verified binding primitive; no graph transaction consumes
it yet, and a binding must be revalidated after any worker replacement.

Windows real-worker tests: all three `vst3_bus_discovery` tests passed with
Dragonfly and the SDK AGain bundle; the labeled VST2 `vst2_bus_discovery` test
passed with the existing fixture. Both now exercise binding validation as well
as discovery. `cargo check -p audiorouter-plugin-host --all-targets` and strict
plugin-host all-target/all-feature Clippy passed. No editor/audio device opened.
Next: consume this binding in a backend-owned port transaction and use an
explicitly selected class for the multi-bus runtime launch; then connect the
engine. VSTC-01/02/09 remain incomplete, including API/MCP/UI and full corpus.

Final checks for this checkpoint: plugin-host library 117/117, both Rust format
checks, documentation validation (271 files / 1,512 links), and x86 native VST3
worker build passed. The x86 worker was built but not executed here. A final
diff check caught an extra documentation EOF blank line; it was removed before
handoff. The compatibility guide now documents missing-class refusal and the
rescan/reselection remedy. No commit or push.

Bound runtime launch checkpoint (2026-11-14, PLUG-03/05, VSTC-02):
`spawn_verified_bound_vst3` now consumes the verified binding when preparing a
runtime worker. It refuses wrong format/fingerprint/class and noncontiguous
auxiliary index, revalidates the binary under the caller's roots, then rechecks
the new worker's buses before returning it. Native supervised multi-bus launches
now pass the identity's selected class explicitly. Deliberate recovery uses
that same path and retains class identity instead of selecting by layout.
The existing AGain multi-bus acceptance fixtures now select their SideChain
class explicitly. This does not yet mutate or activate a user graph; the
backend preparation refusal remains in place.

Real-worker evidence: `each_again_class_keeps_its_own_bus_identity` now verifies
wrong-class refusal, launches the bound auxiliary class, records a failure,
restarts deliberately, rediscovers the same layout and processes a coherent
main/key quantum with finite output. It passed with the SDK AGain module and
this worktree's debug native worker. A first test attempt called restart on a
running worker and correctly failed; the regression now records a failure
first, following the lifecycle contract.

The broad `worker_process verified_native_vst3_ -- --ignored` filter was an
incorrect fixture selection: controllerless and crash-fixture tests failed
because AGain is neither fixture. No product fix was inferred. Explicit
`verified_native_vst3_worker_processes_an_opt_in_multi_bus_fixture` and
`verified_native_vst3_async_bus_worker_bridges_the_graph_scheduler` reruns
both passed with AGain, as did the ordinary AGain native fixture in that broad
run. Plugin-host library 117/117 and strict all-target/all-feature Clippy passed;
workspace/shell formatting checks passed. No audio endpoint/editor opened.
No new REST/MCP method or user-visible control was exposed. Rollback removes
the bound constructor and restores the multi-bus launch/class-test changes.
Next remains the backend-owned graph transaction consuming the binding and
the realtime graph processor integration. VST2 multi-bus execution, route
layout combinations with sidechains, API/MCP/UI, and release gates remain open.

Resume checkpoint (2026-11-14): implementation is authorized. Start VSTC-02
with a bounded, format-independent discovered-bus contract, retaining native
bus indices and explicit main/auxiliary roles. Validate metadata before deriving
a processing layout; channel count alone must never imply a sidechain. Then
wire worker discovery, domain ports and the multi-bus bridge, followed by API,
MCP and UI. Focused contract tests and plugin-host compilation run first on
Windows; real plugin/editor/audio evidence remains separate. No UI port or new
public method is exposed until that path works. Rollback of this first slice
removes the isolated metadata module and its export only.

VSTC-01 domain checkpoint (2026-11-14): chose the port-derived representation,
with no new boolean/channel parameter or session schema field. The optional
`sidechain` input belongs only to an effect Plugin, has width 1/2, and sits
beside one main input and one same-width output. Ordinary plugin nodes without
that port retain their prior contract. Domain validation rejects duplicates,
wrong direction/width, extra inputs/outputs and reserved sidechain ports on
other kinds. The helper `Node::plugin_sidechain_port` is a desired-topology
query, not capability evidence. GRAPH-01/02/03/06 specification text records
the format boundary and main-only bypass mapping.

Regressions cover exact session serialization roundtrip with separate main/key
edges, old-session load without mutation, occupied-main rejection when the key
edge is incorrectly redirected, and six malformed-port variants. The control
preparation path refuses a saved sidechain port with `NodeNotReady`, naming
the plugin, until the discovered bus can be bound to the graph bridge. A focused
preparation test proves refusal before a worker is launched. This is a staging
guard, not successful playback or verified port creation; VSTC-01 is not yet
complete. Missing: verified discovery-to-port transaction, deletion/undo/redo
through dispatch, catalog descriptions and full user-path evidence.

Windows checks: domain library 82/82, focused control preparation test 1/1,
source combinations 6/6 and processor combinations 5/5 passed. Control all-target
compile passed. Strict domain/control all-target/all-feature Clippy passed after
replacing an `.err().expect()` test expression. Workspace/shell formatting
checks passed. These combination suites verify the existing route layouts;
they do not yet cover runnable plugin sidechain graphs. No audio endpoint,
editor or built shell was opened. Rollback: revert this port validation/helper,
temporary preparation guard/tests and spec paragraph together; no migration.
Next: implement the verified port-binding transaction and multi-bus bridge,
then replace the preparation refusal with actual discovered-layout validation.

First slice implemented: `crates/plugin-host/src/bus_discovery.rs` defines
serializable discovered input/output buses with native indices, names, channel
counts and explicit roles. `processing_layout` validates bounded counts and
names, unique native indices per direction, a main bus at index zero, mono or
stereo widths, auxiliary-only additional inputs, and the single-main-output
execution boundary before constructing `WorkerAudioBusLayout`. No assumption
about auxiliary identity is made from extra channels. GRAPH-01/03 and PLUG-03
are the relevant contracts. This is metadata groundwork, not completed
VSTC-01/02 or working user sidechain routing.

Windows verification (2026-11-14, pinned Rust):

- `cargo test -p audiorouter-plugin-host bus_discovery --lib`: 5 passed.
- `cargo test -p audiorouter-plugin-host --lib`: 113 passed.
- `cargo clippy -p audiorouter-plugin-host --all-targets --all-features -- -D warnings`: passed.
- Both workspace and shell `cargo fmt` runs and `-- --check`: passed.
- `git diff --check`: passed (Git reports an LF normalization notice on an
  existing modified compatibility plan).
- `node tools/docs/validate.mjs`: passed, 271 files / 1,511 links.

Existing uncommitted MIDI/review changes were retained. No commit, push, PR,
editor window, audible route or owner application launch. Native discovery,
REST/MCP, engine layout combinations and user entry points have not been
exercised for this slice because they are not wired yet. Next: implement
`DescribeBuses` responses in the isolated workers and supervised request path;
then bind discovered metadata to plugin identity before graph ports are added.

Second slice (2026-11-14): added worker `DescribeBuses` / `Buses` messages,
metadata validation on encode and decode, and `describe_buses` on the worker
process and supervised owner. Requests use the existing bounded control wait;
failure terminates the child and records the supervisor failure. The native
VST3 worker reads actual component bus metadata on its UI thread and reports
main input/output plus explicitly declared auxiliary inputs, retaining native
indices. Additional main inputs are not exposed as sidechains. Counts and
channel widths are bounded; names use the worker's existing bounded title
conversion. This currently requires a successfully loaded effect; it is not
yet a pre-load discovery mode or instrument discovery.

Windows checks: native x64 build via
`tools/m06-vst3-worker/build.ps1 -Output target/debug/audiorouter-vst3-worker.exe`
passed; plugin-host library tests 114/114; all-target compilation and strict
plugin-host Clippy passed; both format checks passed. Real-worker integration
`cargo test -p audiorouter-plugin-host --test vst3_bus_discovery -- --ignored --nocapture`
passed with `AUDIOROUTER_TEST_VST3_WORKER` set to that debug worker and
`AUDIOROUTER_TEST_VST3_EFFECT` set to
`C:/Users/miste/AppData/Local/Temp/vstcorpus/df64/dragonfly-reverb-3.2.10/DragonflyPlateReverb.vst3`.
The test discovers, processes a finite stereo block, then rediscovers identical
metadata. No editor or audio endpoint opened. AGain auxiliary discovery,
VST2 labeled pins, malformed native metadata, x86 build, graph/API/MCP/UI,
and full release gates remain unrun or unimplemented. Next: VST2 pin discovery
and an AGain real-worker discovery regression before domain port wiring.

Third slice (2026-11-14): VST2 `DescribeBuses` now queries bounded input pins
with opcode 33. Tests pin its 128-byte ABI and offsets, checked against the
[Steinberg header](https://raw.githubusercontent.com/R-Tur/VST_SDK_2.4/master/pluginterfaces/vst2.x/aeffectx.h).
One/two main pins may be followed by one/two pins explicitly labeled sidechain,
side-chain, side chain or key (whole words). Keyboard/monkey/aux/surround are
not key labels. Unlabeled extra pins and interleaved main/key layouts are
refused. VST2 native indices identify the first pin of each derived bus;
VST3 native indices identify component buses. No graph mutation or VST2
multi-bus processing is introduced.

Windows evidence: library tests 117 passed; debug VST2 worker built; real
`vst2_bus_discovery` integration passed using `AUDIOROUTER_TEST_VST2_WORKER`
set to this worktree's `target/debug/audiorouter-plugin-worker.exe` and
`AUDIOROUTER_TEST_VST2_EFFECT` set to the local corpus's
`df64/dragonfly-reverb-3.2.10/DragonflyPlateReverb-vst.dll`. Discovery, processing,
and repeat discovery passed with no window or audio device. Strict plugin-host
Clippy passed after fixing its zero-length-array test initializer. All-target
i686 Windows compile passed (not x86 runtime evidence).

The real AGain test now also asserts discovered `[2, 1]` input buses, stereo
output and auxiliary native index 1 before processing separate main/key frames.
Passed command: `cargo test -p audiorouter-plugin-host --features test-fixtures --test worker_process verified_native_vst3_worker_processes_an_opt_in_multi_bus_fixture -- --ignored --nocapture`.
`AUDIOROUTER_VST3_FIXTURE` was
`C:/code/audiorouter/third_party/vst3sdk-build/VST3/Release/again.vst3` and
`AUDIOROUTER_VST3_NATIVE_WORKER` was this worktree's rebuilt debug VST3 worker.
Next: labeled-pin VST2 fixture through the real dispatcher, then discovered
domain ports and the nonblocking graph bridge. P2/P3/P6/P9 remain open. No
commit, push, release or PR.

Fourth slice (2026-11-14): added the `bus-fixture` feature to the existing
repository-owned VST2 test instrument. This builds a metadata-only effect
with four input pins; parameter 0 changes the extra pair's labels from Key L/R
to Rear L/R. It is built into a separate target directory so it cannot replace
the default instrument fixture used by MIDI regressions. The new opt-in
`labeled_pins_are_discovered_but_unlabeled_surround_is_refused` test goes through
inspection, verified launch, real worker discovery, a parameter-bearing process
request and a second discovery. It asserts `[2,2]`, first key pin index 2 and
the actual returned label, then requires `unsupportedVst2BusLayout` for the
same four channels without explicit key labels. This exercises opcode 33 at
the real dispatcher boundary; no SDK/plugin binary was added to source.

Windows verification, all passed:

- `cargo build --manifest-path tools/vst2-test-instrument/Cargo.toml --features bus-fixture --target-dir target/vst2-bus-fixture`.
- `cargo test -p audiorouter-plugin-host --test vst2_bus_discovery labeled_pins -- --ignored --nocapture` with
  `AUDIOROUTER_TEST_VST2_WORKER` set to the rebuilt debug worker and
  `AUDIOROUTER_TEST_VST2_BUS_FIXTURE` set to
  `target/vst2-bus-fixture/debug/ar_test_instrument.dll` (both absolute paths).
- `cargo test -p audiorouter-plugin-host --test vst2_instrument`: 7 passed,
  including real default-fixture MIDI playback/release and runtime bridge.
- Strict plugin-host all-target/all-feature Clippy, strict fixture Clippy,
  and workspace/shell/fixture formatting checks passed.

The fixture is silent and opens no device or window. Native x86 fixture
execution and malformed/hung pin discovery remain unrun. Rollback removes
the fixture feature, conditional dispatcher arm and focused acceptance test;
production behavior is unchanged by this slice. Next: VSTC-01 discovered-port
binding with identity verification and saved-session regressions, then the
VSTC-02 multi-bus bridge. Do not expose a usable sidechain control yet.

Finish the pending VST work without weakening the existing graph, realtime,
privacy or authorization contracts:

- a VST3 auxiliary input, or a VST2 input whose pin metadata explicitly names a
  sidechain/key input, can be connected as a separate graph port;
- the sidechain is never summed into the program signal and an unconnected
  sidechain is silence;
- VST2 programs, shell plugins and `.fxp`/`.fxb` presets are usable;
- VST3 units/program lists, multi-class modules and sample-size refusal are
  verified;
- plugin editors resize, focus and follow monitor/DPI changes;
- release evidence and the compatibility page match the result.

Sidechain control/lifecycle checkpoint (2026-11-14, VSTC-02/03, PLUG-03/04/05,
GRAPH-14): the multi-bus owner now has the control surface the ordinary bridge
already had. `SupervisedBusWorkerLoop` serves bounded `SaveState`,
`OpenEditor`, `CloseEditor` and live `SetParameters` requests between quanta,
reports a first-failure reason and continuity counters, re-reads the plugin's
reported delay every half second and after the editor closes, and runs the
unchanged-audio watchdog on the main bus. `SidechainRuntimeBridge` exposes the
same state/editor/parameter/resume calls and computes engine latency as the
reported plugin delay plus its one-quantum pipeline.

The protected-microphone policy is now applied to a prepared sidechain:
an unlicensed effect that returns loud input unchanged is silenced until a
deliberate resume, exactly as on the ordinary bridge. Control keeps a weak
multi-bus registry, so `plugins.saveState`, the native editor (including the
stopped-session editor), `plugins.resume`, live flag changes and plugin status
telemetry reach either bridge kind without an adapter change. The multi-bus
test fixture now answers a `Latency` request like the real VST3 worker, so the
periodic poll cannot end it.

Windows evidence: new
`supervised_bus_loop_silences_protected_unchanged_audio_until_resume` passed
(the echo fixture latches a protected silence and refuses resume while still
unchanged). Plugin-host library 117 passed; `worker_process` with
`--features test-fixtures` 42 passed / 18 opt-in ignored; control library 402
passed / 19 ignored. The real
`backend_prepares_a_verified_sidechain_for_the_real_compiled_route` was rerun
with the debug native worker and the SDK AGain bundle and passed. Workspace and
shell Clippy (`-D warnings`), both format checks, and `git diff --check` passed.

Still open for VSTC-02/03: VST2 auxiliary execution for explicitly labeled key
pins (discovery and binding exist, processing does not); stop/restart, graph
replacement, worker retirement and state-replacement regressions; exhaustive
source/tool sidechain layouts. The public preparation gate stays until the
adapter and UI path in VSTC-04 exists. Rollback removes the bus owner's
request/health/policy fields and the control weak registry together.

Transaction-freshness and sample-format checkpoint (2026-11-14, VSTC-01/09):
`graph.commit` now re-validates every plugin placeholder against a current
explicit scan result before applying a plan, so a plan queued while discovery
evidence was valid cannot be committed after that evidence disappeared. This
covers both ordinary graph plans and a prepared sidechain-port plan; the
existing stale-revision, replay and undo checks are unchanged.

The VST3 worker now refuses a plugin that does not accept the required 32-bit
sample format with `sampleFormatUnsupported`, instead of a generic VST3 result
code, and never converts samples silently. The debug native worker was rebuilt
and the real AGain compiled sidechain regression still passed.

Windows evidence: control library 402 passed / 19 ignored after the freshness
change; real AGain sidechain regression passed with the rebuilt worker. Full
workspace and shell Clippy, both format checks and `git diff --check` passed.

Still open for VSTC-01: an adapter/UI path for port creation and removal,
raw-client refusal with a named capability reason, export/import and durable
reload regressions. Still open for VSTC-09: a 64-bit-only fixture that proves
the new refusal, a multi-class effect corpus run, state isolation between
classes, and the Steinberg validator run.

Preset/parser and program-model checkpoint (2026-11-14, VSTC-05/07): added
`crates/plugin-host/src/vst2_preset.rs`, a bounds-checked parser for `CcnK`
`.fxp`/`.fxb` files covering `FxCk`, `FPCh`, `FxBk` and `FBCh`, with 16 MiB
file, 4096-parameter and 256-program ceilings and seven unit tests (valid
program/chunk/bank, bad magic, truncated body, oversized file, non-finite
values, trailing bytes, invalid name). Nothing applies a parsed preset to a
plugin yet.

Added a bounded `ProgramDescriptor` (`MAX_PROGRAMS = 256`) and
`Vst2Library::{program_count, current_program, programs, set_program}` using
VST2 opcodes 2/3/29, with out-of-range selection refused. The worker protocol,
control API/MCP/UI and the real fixture test for these are not wired yet, so
VSTC-05 remains incomplete.

Still open for VSTC-07: applying a parsed program/bank through `effSetChunk`
or parameter events, the authorized import API/MCP/UI, and an import into a
running route.

Shell/preset checkpoint (2026-11-14, VSTC-05/06/07):
`Vst2Library::apply_preset` applies parameter programs, raw chunks and bounded
banks; `is_shell`, `shell_sub_plugins` and `load_shell_sub_plugin_as` implement
`effGetPlugCategory`, `effShellGetNextPlugin` (indexed then cursor form) and
`audioMasterCurrentId` through a load-scoped thread-local. The repository shell
fixture exposes three named sub-effects and the opt-in discovery test passes.

Real-shell result: `C:\Program Files\VSTPlugins\dxshell_v1.0.5b\dxshell.x64.dll`
reports `kPlugCategShell` but returns no sub-plugins through either enumeration
form; `mfxshell.x64.dll` fails its effect header validation and `dxishell.x64.dll`
also enumerates none. These wrappers do not implement the standard shell
opcode, so a different VST2 shell DLL is still needed for real-load evidence.
No Blue Cat plugin was loaded.

Worker protocol for VST2 programs and presets (2026-11-14, VSTC-05/07):
added `DescribePrograms`/`Programs`, `SelectProgram`/`ProgramSelected` and
`ApplyPreset`/`PresetApplied` to the worker protocol with bounded validation
(unique IDs, name limits, at most 256 programs, 16 MiB presets). The Rust VST2
worker answers them with the real library methods; the supervised and
unsupervised process clients expose `programs`, `select_program` and
`apply_preset`. A caller-side out-of-range selection is refused without
marking the worker failed.

Real evidence with TDR Nova:
`the_worker_protocol_lists_and_selects_real_programs` passed using the rebuilt
debug VST2 worker and `C:\Program Files\Steinberg\VSTPlugins\TDR Nova.dll`
(listing, selection, an out-of-range refusal that keeps the worker alive, and
a synthetic one-program preset through the protocol). The repository shell
fixture still passes its discovery test.

Not done: REST/OpenAPI, default MCP tools, UI controls and authorization for
program selection and preset import; a real `.fxp`/`.fxb` file; and a real
shell DLL that implements `effShellGetNextPlugin`. Clippy, formatting and the
124 plugin-host library tests pass.

Programs/presets API and MCP wiring (2026-11-14, VSTC-05/07, AUTO-18):
`plugins.programs`, `plugins.selectProgram` and `plugins.applyPreset` are in
`API_METHODS`, the control dispatcher, input/output schemas, required-field
authorization, `contracts/src/index.ts`, and the default MCP tools
`list_plugin_programs`, `select_plugin_program` and `apply_plugin_preset`.
The runtime bridge serves the requests from the worker thread; VST3 and
multi-bus bridges refuse them with a plain reason. Preset import canonicalizes
the file and requires it to sit in the plugin binary's own directory, with a
bounded size.

Windows checks: contract drift 153 methods, 38 kinds; the default-MCP
reachability test passes; control library 402 passed / 19 ignored; workspace
Clippy `-D warnings` and formatting pass.

Still open: UI controls for programs/presets and the sidechain port; a real
`.fxp`/`.fxb` file; a shell DLL that implements `effShellGetNextPlugin`.

VST2 programs/presets UI (2026-11-14, VSTC-05/07): the plugin Properties
inspector now shows a VST2-only "VST2 programs" section: with the route
playing, Load programs reads the bounded list, the program select calls
`plugins.selectProgram`, and a preset path beside the plugin binary is imported
through `plugins.applyPreset`. Errors are shown inline without moving the
panel. `UiBackend` and the route harness expose the three calls.

UI checks: `npm --prefix ui run lint` (zero warnings), TypeScript, and the
PluginNodeInspector tests (10 passed, including the new program/preset test)
pass.

Still open: the canvas/Properties sidechain port UX and the "no Mixer on a
sidechain port" interaction; VST3 units/programs; editor resize/focus/DPI;
release gates.

Bus discovery API (2026-11-14, VSTC-02/04): `plugins.buses` loads a scanned
plugin in its isolated worker and returns exact input/output buses with native
indices, names, widths and main/auxiliary roles. An explicit VST3 class is
forwarded to the verified worker even when optional module metadata omits it;
the worker must accept or refuse it. Registered in `API_METHODS`, dispatch,
schemas, authorization, contracts and the default MCP tool
`list_plugin_buses`. Real AGain test
`plugins_buses_reports_the_real_again_auxiliary_input` passed through
`ControlPlane::dispatch` (main + auxiliary at native index 1).

Contract drift: 154 methods. Workspace Clippy, UI lint, formatting and diff
checks pass.

## Decisions that govern the work

- **Visible Mixer proposal accepted (owner, 2026-11-14).** A second source on an
  occupied ordinary input may keep proposing a previewable, undoable Mixer.
  Serial insertion remains the explicit "Insert processor" path.
- **VST2 sidechain only by explicit pin label.** VST2 has no VST3-style named
  bus. A pin is a sidechain only when its input-pin metadata explicitly names
  it sidechain/key. Extra channels or surround layouts are never inferred.
- **VST3 auxiliary bus only by declared bus.** Bus 0 is main; only additional
  `kAux` input buses may be exposed. A plugin with no declared auxiliary bus
  gets no sidechain port.
- **No fake support.** An unsupported format/layout is refused before the graph
  changes, with the plugin and port named.
- **Worker containment stays.** Plugin code remains in the isolated worker; no
  audio callback may wait for UI, IPC, disk or control locks.
- **Owner gates remain owner gates.** Hosted CI, live-ear tests, a signed
  installer and the attended editor pass require the owner.

## Work item overview

| ID | Workstream | Depends on | Exit evidence |
| --- | --- | --- | --- |
| VSTC-01 | Sidechain domain and saved-session contract | — | Domain tests, migration fixtures |
| VSTC-02 | Worker bus discovery and plugin multi-bus bridge | VSTC-01 | Worker protocol tests, AGain VST3 |
| VSTC-03 | Engine sidechain graph execution | VSTC-01, VSTC-02 | Engine/main+key tone tests |
| VSTC-04 | Control, API, MCP and UI sidechain path | VSTC-02, VSTC-03 | Dispatcher, MCP, Edge, three themes |
| VSTC-05 | P2 VST2 programs and presets | — | Worker fixtures, API/MCP tests |
| VSTC-06 | P2 VST2 shell plugins | VSTC-05 | Fixture shell DLL, scan/load tests |
| VSTC-07 | P2 `.fxp`/`.fxb` import | VSTC-05 | Parser bounds, round-trip tests |
| VSTC-08 | P3 VST3 units/program lists and context menu | VSTC-02 | Real VST3 metadata test |
| VSTC-09 | P3 multi-class and sample-size verification | VSTC-08 | Corpus test, refusal test |
| VSTC-10 | P6 editor resize/focus/DPI | — | Pumped-window test, owner by-eye |
| VSTC-11 | P9 compatibility/release gate | all above | Generated page, installer, fresh install |

## VSTC-01 — Sidechain domain and saved-session contract

Goal: one plugin node can own an optional named input port without breaking old
sessions.

Tasks:

1. Extend the plugin node contract with sidechain metadata:
   - `sidechain` (boolean or absent) and `sidechainChannels` (1 or 2), or an
     equivalent port-derived representation; choose one and document it.
   - Add the `sidechain` input port only when discovery confirmed it.
   - Validate the port is unique, input-direction, and 1/2 channels.
2. Make older sessions valid:
   - A saved plugin node without the port behaves as "not connected".
   - Loading never edits a user file; adding/removing the port is an explicit
     graph transaction.
   - Undo/redo and export preserve the port and its edges exactly.
3. Update `docs/spec/04-graph.md` (GRAPH-01/02/06), the node registry text and
   `nodes.catalog`/`node_parameter_schema` descriptions.
4. Regression tests:
   - A saved session without the new field loads and plays.
   - A sidechain edge cannot target an ordinary input.
   - Removing the plugin removes the sidechain edge atomically.
   - A duplicate sidechain port or invalid channel count is rejected.

Files: `crates/domain/src/lib.rs`, `crates/control/src/catalog.rs`,
`crates/control/src/persist*`, domain/session tests, specs.

Done when: the domain model, old-session fixture and catalog agree, and an
unsupported node cannot acquire a port by guessing.

## VSTC-02 — Worker bus discovery and plugin multi-bus bridge

Goal: a worker tells the host its exact input buses before a graph is prepared,
and the host can run a main+sidechain quantum through one worker.

Tasks:

1. Add a bounded worker `DescribeBuses` request (or an equivalent describe
   mode) for VST3 and VST2:
   - VST3: enumerate `kAudio` input buses; bus 0 is main, later `kAux` buses
     only; return bus index, channel count and name.
   - VST2: call `effGetInputProperties` (33) for each input pin and treat a pin
     as sidechain only when its short/long label explicitly names
     sidechain/key; return pin channels and labels.
   - Bound the request and kill a hung worker by the existing supervisor
     timeout.
2. Expose the result as a read-only API method, proposed `plugins.buses`, with
   an input schema (`path`, `format`, `classId`, `fingerprint` or a scan
   identity) and an output schema for main/aux buses. Add it to `API_METHODS`,
   `contracts/src/index.ts`, output schemas, OpenAPI and docs.
3. Add a multi-bus plugin processor in `crates/plugin-host`:
   - start the existing verified multi-bus worker;
   - preallocate main and sidechain input queues plus output storage;
   - submit one coherent `WorkerFrame` per bus per quantum, with matching
     sequence/deadline/frame count;
   - fail closed (silence) on a missing, late or malformed bus; never flatten
     the sidechain into the main input;
   - preserve health, latency, state/editor control and quarantine behavior.
4. Extend control preparation:
   - `PluginStageRequest` carries the requested bus layout;
   - a node with a confirmed sidechain port spawns the multi-bus bridge;
   - editor-only and stopped-session paths use the same bus layout;
   - a plugin that changes bus layout reports `restartRequired`/unsupported
     instead of silently changing the graph.
5. Tests:
   - protocol encode/decode, bounds, mismatched bus identities;
   - supervised worker crash/hang on the sidechain bridge;
   - AGain VST3 `[stereo, mono]` through main+aux with a deterministic tone;
   - VST2 labeled-pin fixture and a surround VST2 fixture that must be refused.

Files: `crates/plugin-host/src/lib.rs`, `vst2.rs`, worker binaries,
`tools/m06-vst3-worker/main.cpp`, `crates/control/src/plugins.rs`, API schemas,
contracts, `crates/plugin-host/tests/*`.

Done when: a worker exposes exact buses, a main+aux quantum is coherent, and
unsupported layouts never reach graph publication.

## VSTC-03 — Engine sidechain graph execution

Goal: the realtime engine routes a separate sidechain source into the plugin’s
auxiliary bus and keeps the program path independent.

Tasks:

1. Add sidechain execution state to the prepared graph:
   - one optional sidechain subgraph per plugin stage;
   - a preallocated sidechain scratch block;
   - fail-closed behavior when the branch is unavailable.
2. Extend `ProcessingStage`/`RuntimeGraph` with a sidechain-aware plugin stage
   (or an equivalent immutable description). `process` remains allocation-free,
   nonblocking and non-panicking.
3. Teach the compiler:
   - a sidechain edge is not a main input and does not consume the ordinary
     input’s single-edge rule;
   - compile the sidechain source path to its own bounded graph;
   - reject a sidechain path with cycles, wrong width, plugin faults or an
     unbound worker;
   - unconnected sidechain = silence, not a dry or main substitute.
4. Cover all layouts:
   - source alone, fan-out, Mixer, connected Mixers;
   - bypass and disabled plugin;
   - late/missing sidechain frames;
   - save/reload and live recompilation.
5. Add fixed-seed signal tests:
   - main tone alone gives the baseline;
   - key tone alone does not appear at the main output;
   - key changes the processed main output;
   - a missing key silences only the plugin path, not unrelated outputs.

Files: `crates/engine/src/lib.rs`, `crates/engine/src/connected_mixers.rs`,
`crates/engine/tests/source_combinations.rs`,
`crates/engine/tests/tool_combinations.rs`.

Done when: graph ownership is explicit, the audio callback never waits, and the
sidechain cannot leak into main.

## VSTC-04 — Control, API, MCP and UI sidechain path

Goal: a user or assistant can discover, connect, save and run a supported
sidechain without a hidden Mixer.

Tasks:

1. Control/API:
   - `plugins.buses` discovery method;
   - `node_parameter_schema` entries for sidechain settings;
   - node-targeted runtime errors naming plugin, port and remedy;
   - backend validation of format, declared bus and channel width.
2. MCP:
   - a default read tool for plugin buses (for example
     `list_plugin_buses`);
   - a default editor flow for connecting the sidechain port, or a documented
     `MCP_EXCLUDED_METHODS` reason if a native window is required;
   - a `get_recipes` entry for "add a sidechain duck";
   - refresh role guidance so Editor builds, Operator runs.
3. UI:
   - add/remove sidechain port on a supported plugin;
   - show the named port on the canvas and in Properties;
   - connecting a sidechain source must not insert a Mixer;
   - an occupied sidechain input must offer replace/reject, not sum;
   - three themes, no layout shift, keyboard access;
   - guides/property coverage/video manifest updates for the changed panel.
4. Tests:
   - `ControlPlane::dispatch` for success and every refusal;
   - `mcp_tool_call` and API-method reachability;
   - UI unit tests and a headed browser scenario;
   - save/reload and undo/redo of the port and edge.

Files: `crates/control/src/plugins.rs`, `api_schema.rs`,
`api_output_schema.rs`, `catalog.rs`, `dispatch_methods.rs`, `crates/cli`,
`contracts/src/index.ts`, `ui/src/*`, site guide files.

Done when: no hidden Mixer is created for a sidechain, REST/MCP/UI all reach it,
and the refusal paths are tested.

## VSTC-05 — P2 VST2 programs

Goal: users can list and choose a VST2 program through the host.

Tasks:

1. Add VST2 dispatcher calls: `effGetProgram` (3), `effSetProgram` (2),
   `effGetProgramNameIndexed` (29), bounded names.
2. Add worker messages `Programs`/`SelectProgram` and host-side selection with
   state capture.
3. Add API methods proposed `plugins.programs` (read) and
   `plugins.selectProgram` (session/graph write as appropriate), schemas,
   contracts, MCP default tool or documented exclusion, and a recipe.
4. UI: a program list in the plugin inspector, with loading/error states and
   no layout shift.
5. Tests: a fixture with bounded program names, out-of-range selection,
   state/parameter consistency, denied permission and dispatcher/MCP parity.

Files: `crates/plugin-host/src/vst2.rs`, worker binary, control/API/UI,
fixtures.

## VSTC-06 — P2 VST2 shell plugins

Goal: one shell DLL with many effects is listed and loaded by sub-plugin.

Tasks:

1. Read `effGetPlugCategory` (35) and `kPlugCategShell` (10).
2. Enumerate with `effShellGetNextPlugin` (70); establish each sub-plugin’s
   identity and display name without loading plugin code in the app process.
3. Pass the sub-ID through `audioMasterCurrentId` and load exactly that
   sub-plugin in the worker.
4. Add a test shell DLL fixture; scan must list every sub-plugin, selection must
   load the right one, and a missing sub-ID must fail with a plain reason.
5. Update identity, persistence, UI labels and compatibility output.

Files: `vst2.rs`, worker, scan identity, domain parameters, fixtures, catalog,
API, docs.

## VSTC-07 — P2 `.fxp`/`.fxb` import

Goal: a user can import a VST2 preset file through an explicit, bounded action.

Tasks:

1. Add a bounds-checked parser for `CcnK`, `FxCk`, `FPCh`, `FxBk`, `FBCh`,
   sizes, version and chunk boundaries; no unbounded allocation.
2. Apply a program/bank through `effSetChunk`, or parameters where a chunk is
   absent; verify the resulting state.
3. Add a file-picker/import API with authorization and root checks; MCP tool or
   exclusion and a recipe.
4. UI: import action in the plugin inspector, errors without changing the
   running route.
5. Tests: valid fixtures, truncated/oversized/traversal files, bad magic,
   state round trip, denied scope, and import into a running route.

Files: new `crates/plugin-host/src/vst2_preset.rs` (or equivalent), control,
UI, schemas, contracts.

## VSTC-08 — P3 VST3 units/program lists and context menu

Goal: expose read-only VST3 unit/program metadata and a safe context-menu stub.

Tasks:

1. Query `IUnitInfo` and `IProgramListData` defensively; bound unit/program
   counts and string sizes.
2. Return units/programs through the worker protocol and `plugins.buses`-style
   read API; no mutation unless a program-selection API is also approved.
3. Add a context-menu stub through `IPlugFrame`/component handler that opens
   no arbitrary UI; document the limitation.
4. Tests: a real VST3 with units, a plugin without units, malformed counts,
   dispatcher/MCP/UI parity, no callback blocking.

Files: `tools/m06-vst3-worker/main.cpp`, worker protocol, control/API/UI, docs.

## VSTC-09 — P3 multi-class and sample-size verification

Goal: load the chosen class from a multi-class module and refuse unsupported
sample modes.

Tasks:

1. Verify scan lists every audio effect class and load uses the selected
   `classId`; add a multi-class corpus fixture that is not an instrument.
2. Verify the worker always requests `kSample32`; a `kSample64`-only plugin gets
   a plain refusal and no silent conversion.
3. Add tests for class mismatch, missing class, sample-mode refusal and state
   isolation between classes.
4. Run Steinberg’s validator against the corpus and record the result in the
   plan; remember that validator success is not AudioRouter evidence.

Files: VST3 worker, plugin-host identity/worker, tests, compatibility page.

## VSTC-10 — P6 editor resize/focus/DPI

Goal: a real plugin editor can be used at multiple sizes and monitor scales.

Tasks:

1. VST3:
   - user resize through `IPlugView::canResize`/`checkSizeConstraint`/`onSize`;
   - focus the child automatically after attach and restore focus on reopen;
   - handle `WM_DPICHANGED` and monitor moves (content scale, parent refit);
   - prevent layout shift while the editor rebuilds.
2. VST2:
   - keep fixed-size editors from being stretched;
   - set focus after attach and on window activation;
   - refit on DPI change without moving the child outside the client area.
3. Test harness: a pumped parent window with audio flowing, repeated
   open/resize/close, crash/hang injection, and a bounded close.
4. Attended evidence: owner opens three frameworks on the built exe; capture
   window bounds and scale; record any plugin-specific limitation.

Files: `crates/plugin-host/src/vst2.rs`, `src-tauri/src/plugin_editor_windows.rs`,
`tools/m06-vst3-worker/main.cpp`, editor tests, evidence file.

## VSTC-11 — P9 compatibility and release gate

Goal: publish what was tested and hand over a package.

Tasks:

1. Regenerate `docs/operations/plugin-census.md` and the compatibility page from
   the latest census, including known issues and pass rates.
2. Build the installer and verify its hooks; rebuild the shell/workers and
   confirm the embedded UI is current.
3. Run `fresh_install_shell` on a machine with no other AudioRouter running;
   the current owner shell must be closed first.
4. Live-ear check with three frameworks on a real route; owner approval and
   explicit test conditions.
5. Run the hosted CI job for the i686 target and read every annotation.
6. Update the release notes, checksums, limitations and rollback instructions.

Files: `docs/operations/plugin-census.md`, `plugin-compatibility.md`,
`tools/release/*`, `tools/build-desktop.ps1`, release evidence.

## Order and parallelism

1. **First:** VSTC-01, VSTC-02 and VSTC-10 can start in parallel; VSTC-01 is the
   only shared contract.
2. **Then:** VSTC-03, then VSTC-04. Do not expose a sidechain port in the UI
   before the engine/worker path is green.
3. **In parallel:** VSTC-05 -> VSTC-06 -> VSTC-07; VSTC-08 -> VSTC-09.
4. **Last:** VSTC-11, after all code and tests pass.
5. Each item is a separate commit; run its focused tests and the full gate
   before committing. Do not mix mechanical formatting with behavior.

## Definition of done per item

A work item is complete only when all of these are true:

- the production path is implemented, not a test-only or mock path;
- the exact real entry point is exercised (`ControlPlane::dispatch`,
  `mcp_tool_call`, the UI request, the real worker);
- fail-before/fix-after regressions exist for defects;
- unsupported cases fail closed with a plain reason and no graph mutation;
- REST/OpenAPI, default MCP or documented exclusion, catalogs, guides and user
  docs are updated together;
- the relevant tests, formatting, Clippy, contract drift, UI checks and docs
  validation pass;
- the plan records commands, environment, results, limitations and evidence.

## Owner/hosted gates

These cannot be completed by local code alone:

- attended editor geometry, focus and DPI on the owner’s display setup;
- live-ear routing through the owner’s real endpoints and speakers;
- a signed installer and hosted i686/Windows CI;
- `fresh_install_shell` on a machine with no other AudioRouter running;
- hardware MIDI and any paid/licensed plugin named by a reporter.

Record each as `not run` or `blocked`, with the exact reason; never infer it
from a compile, mock or browser test.

## Rollback

Each work item is a scoped commit. Roll back VSTC-01/03/04 together if the
sidechain contract changes; keep worker protocol and graph schema versions
compatible or reject old assets explicitly. A failed editor change rolls back
to the existing fixed-size host without changing processing. A failed release
gate leaves the previous signed artifact and database untouched.

VSTC-09 sample size (2026-11-14): the VST3 worker now asks `canProcessSampleSize(kSample32)` before `setupProcessing` and refuses with `sampleFormatUnsupported` on `kResultFalse`; it always requests `kSample32`. Worker builds and its `--self-test-host` passes. Not done: a 64-bit-only fixture proving the refusal, a multi-class effect corpus run, class state isolation, the Steinberg validator run, VSTC-08 units/programs, VSTC-10 and VSTC-11 (several are owner-gated).
