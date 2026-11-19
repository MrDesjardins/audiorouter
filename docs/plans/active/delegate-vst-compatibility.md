# Delegation brief: finish VST2/VST3 compatibility

Prepared 2026-11-18. Status: ready to delegate; no execution performed by this brief.

## Assignment and scope

VS-01 checkpoint (2026-11-18): the old blanket saved-sidechain refusal is
superseded by current `start_plugin_bridges` in `crates/control/src/plugins.rs`.
It prepares VST3 and applies protected-path policy, while
`start_sidechain_bridge` still refuses VST2 auxiliary execution. Both VST3
restore calls already exist. First verify through disposable storage and the
production preparation/compiler entry with SDK AGain. Rollback is confined
to the opt-in regression. The focused compile exposed E0308 in the previous
test edit: the third preparation argument is a prior session, not a state
override. Use a persisted state asset instead.

Windows run: SDK pin `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` verified
at `C:/code/audiorouter/third_party/vst3sdk`. The existing AGain bundle
was inventoried and read from `third_party/vst3sdk-build/VST3/Release/again.vst3`
in the parent checkout (module SHA-256
`5CD00469CFC047302E7B6805D2A32275C63A13FE88C295EB2615E5EE0D296053`).
Built a local worker with `tools/m06-vst3-worker/build.ps1 -SdkInclude
C:/code/audiorouter/third_party/vst3sdk -Output
target/vst-qualification/audiorouter-vst3-worker.exe` (SHA-256
`C1908E787E2CDA3D95E4EC77A91C1B562B7AC9B9D3FCD27F3A39161E2BCC3C7D`).
Set `AUDIOROUTER_VST3_WORKER_PATH` and `AUDIOROUTER_VST3_FIXTURE` to the
absolute worker and bundle paths. Ran `cargo test -p audiorouter-control
backend_plugin_stage_path_prepares_and_compiles_a_verified_sidechain_route
-- --ignored --nocapture`: 1 passed. Source base `33cdf58d4` plus the local
capture fix and test. No audio endpoint or editor opened. The rest of VS-01
and the brief remain open.

Close the remaining compatibility work for VST2/VST3, x64/x86, effects and
MIDI instruments, using the existing implementation and real-worker evidence.
Requirements: PLUG-01–07, GRAPH-14, applicable GRAPH-01/02/06/15, SEC-07,
AUTO-18 and changed UI requirements. Track VSTC-01–11 and phases P2/P3/P5/P6/P9.
The goal is measured compatibility with an explicit supported boundary, not an
unprovable claim that every plugin works.

Follow AGENTS.md. Do not push, open a PR, publish, purchase licences/signing,
change the owner's saved sessions or start audible routes/editor windows
without the required authorization. Never run Blue Cat plugins. Inventory
every corpus directory before loading it; verify skip behavior. Use licensed,
free or permitted demo binaries, never commit them. Do not build a shared VST3
worker: P7 is deferred unless evidence and owner direction reopen it.

## Read before starting

- [Overview](../../README.md), [current priorities](current.md), M06/M07/M08 in
  [delivery](../../spec/15-delivery.md), and relevant linked specs.
- [VST handoff](vst-handoff.md), [completion work](vst-completion-work.md),
  [full compatibility](vst-full-compatibility.md), all completely. Read the
  final November 15 handoff before following any older open-work list.
- [MIDI/instrument design](vst-midi-instruments-design.md),
  [plugin census](../../operations/plugin-census.md),
  [compatibility snapshot](../../operations/plugin-compatibility.md), and
  [verification runbook](../../operations/verification.md).

## Starting evidence and reconciliation

The older priority summary is coarse. Later handoffs record programs/preset
API/UI, sidechain add/remove/replace controls and guides, VST3 unit hierarchy
and dispatcher evidence, multi-class/sample-format fixtures, validator results,
and an isolated unsigned installer smoke. Do not reimplement these from an
earlier checklist. Conversely, adding a sidechain port or running AGain's
validator does not prove a saved sidechain route actually processes audio.

Historical worktree paths, branches, PIDs, counts and unpushed statements are
not current authority. Inspect Git/worktrees and code before using them; the
current plan records the plugin-experience work as merged. Preserve unrelated
changes. Inventory SDK, CMake, worker architectures, test fixture sources and
corpus availability. Use the pinned Rust toolchain. Keep real-worker, mocked,
dispatcher, browser and attended evidence separately labelled.

## Ordered work packages

1. **VS-01: create an evidence-backed gap list.** Reconcile every VSTC item
   against current code and the latest dated results. Record implemented,
   exercised, missing and externally blocked separately, with test names,
   artifact identity and evidence links. Check remaining VST2 sidechain
   execution refusals, program/preset persistence and actual shell enumeration.
   Update this plan and the completion work plan with the bounded next slice
   and rollback before editing code. Do not remove any playback refusal merely
   because lower-level groundwork exists.
2. **VS-02: finish sidechain lifecycle and production routing (P5).** Close
   missing health, state, parameter/editor control, dynamic-latency and deliberate
   recovery paths identified by VS-01. Preserve exact binary/class/bus binding
   across state restoration and worker replacement. Prove the same compiler and
   backend preparation used by the UI can run a saved main/key graph with a
   real declared-aux VST3. Complete explicitly labelled-pin VST2 execution if
   still missing; never infer sidechain from channel count. Keep unsupported
   cases refused before graph publication. Cover source alone, fan-out, Mixer,
   connected Mixers, bypass/disabled, disconnected key, unavailable connected
   key, state/layout changes, worker crash/hang and protected-path silence.
   Test key isolation, actual main-signal effect and unchanged unrelated output.
   Exercise save/reload, graph transactions, undo/redo, dispatcher, default MCP
   and exact UI requests. Run and extend source/tool combination suites.
3. **VS-03: close VST2 conformance gaps (P2).** Verify programs and `.fxp/.fxb`
   imports through real workers, dispatcher/MCP and Properties, including saved
   state, denied authorization, malformed/oversized files and no route mutation
   on refusal. Qualify shell sub-plugin enumeration/selection with a fixture
   DLL and missing-sub-ID refusal. Confirm identities survive save/reload and
   one sub-plugin never substitutes for another. Check opcode constants against
   pinned authoritative ABI definitions. Document legacy-process refusals;
   do not silently weaken the existing support policy.
4. **VS-04: close VST3 conformance gaps (P3).** Reuse existing unit/program,
   multi-class and sample-format fixtures. Prove class selection and state
   isolation, `kSample32` negotiation and clear `kSample64`-only refusal. Check
   no-unit and malformed-metadata paths, safe context-menu limitations, and
   I/O/latency-change handling. Run the official validator where available as
   supplemental evidence; AudioRouter's own worker/dispatcher tests remain
   required. Record any acceptance-wrapper/tool-version discrepancy explicitly.
5. **VS-05: qualify editors (P6).** Add or complete a pumped-parent harness
   with audio processing active and bounded repeated open/resize/close and
   crash/hang recovery. Open VST2 editors on their loader/UI thread. Verify
   VST3 resize constraints, VST2 fixed-size behavior, focus and parent ownership,
   and DPI refit. Real visible windows require an agreed owner test window.
   Separately record attended 100/150/200 percent scale and monitor moves on
   representative frameworks, including exact plugin/worker versions.
6. **VS-06: corpus, packaging and handoff (P9).** Re-run a bounded census after
   worker changes for both bitnesses and effects/instruments; exclude Blue Cat.
   Measure nonzero transformed output or note response, state behavior and
   contained failures rather than load success alone. Generate compatibility
   docs/site from retained JSON with denominators, exclusions and known issues.
   Rebuild shell, CLI and four workers; verify fresh embedded UI and exact
   artifact hashes. Run isolated `fresh_install_shell` without closing another
   session's shell. Supply the release agent an immutable artifact manifest.
   Hosted i686 CI, annotations, hardware MIDI unplug/replug, licensed-plugin
   sign-in, owner listening and attended checks remain individually recorded
   gates. Do all independent work before requesting external access/approval.

## Verification and evidence

Compile/typecheck immediately after each bounded edit. Every defect needs a
regression that fails without its fix. Run focused tests first, then applicable
workspace/shell tests, strict Clippy and both Rust format checks. For UI changes
run format, lint, typecheck, unit/build and real browser scenarios in dark,
light and high contrast. Rebuild `e2e_backend` before real-backend UI tests.

Run `npm --prefix contracts run check:drift`, default-MCP reachability and
explicit `mcp_tool_call`/`ControlPlane::dispatch` tests for changed methods.
Keep API_METHODS, schemas, authorization, contracts, default tools/exclusions,
node schemas and recipes current. Regenerate API reference and site catalogs.
Changed controls need real portrait Properties guide coverage and Firefox
playback verification under AGENTS.md; read the media/video skills when doing
that work. Run docs acceptance and inspect the complete diff.

Retain commands, environment, fixture/plugin fingerprints, worker paths/hashes,
source commit, results and limitations under new evidence directories. Link
redacted summaries under `docs/plans/active/evidence/`. Record ignored tests as
unrun unless explicitly executed with the correct fixture. Never generalize
one fixture's success to all plugins, endpoint continuity or hardware MIDI.

## Deliverables, coordination and rollback

Deliver the reconciled gap list, scoped fixes/tests, reproducible real-worker
reports, updated compatibility docs/guides, artifact manifest and a concise
owner-test checklist. Commit by completed work item, stage explicit files and
preserve unrelated changes. Before PR approval, report branch/commit, complete
diff summary, checks, limitations and exact next gate. No automatic PR/push.

Own plugin-host/native-worker/engine/control/API/UI changes and VST plans.
Release qualification owns the installer VM harness. Coordinate shared build
scripts/contracts rather than editing them concurrently. Serialize expensive
builds, visible editor tests and VM work. Any later artifact change requires
the release agent to requalify the resulting candidate.

Risks: stale completion lists, fixture-only proof, wrong class substitution,
key leakage, callback blocking and visible/audible disruption. Roll back scoped
commits; revert coupled sidechain domain/compiler/adapters together and retain
safe refusals. Keep saved-session/protocol compatibility or reject explicitly.
Do not overwrite owner databases or remove worktrees directly.

Assignment complete only when scoped behavior is exercised and documented;
external gates remain explicitly blocked/not run. An unsigned app-only package
is allowed under DEC-17; a signed-package gate in older VST notes must be
reported separately, not silently waived or made a blocker for unsigned tests.

Next action: VS-01 reconciliation, then VS-02 production sidechain proof and
VS-05 pumped-parent preparation, following actual dependencies found in code.
