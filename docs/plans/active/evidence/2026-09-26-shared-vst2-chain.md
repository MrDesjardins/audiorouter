# Shared VST2 chain qualification — 2026-09-26

## Subsequent attended defects

After the initial qualification below, the user heard scratchy voice and
reported no yellow connection activity. Source review identified a full
pipeline dropping new input before retrieving ready output, and null meters
throughout multi-path diagnostics. The bridge now swaps completed output
storage directly with incoming audio and requeues it. The backend publishes
preallocated lock-free source/stage/destination meters using existing fields;
shared plugin boundaries measure group output. Windows all-target compile,
contract drift and documentation checks passed; these follow-up changes have
not had tests or attended listening/activity confirmation. Original test
results and hashes below apply to the earlier build, not this follow-up.
Current handoff is in the [active plan](../current.md).

## Scope and result

User-authorized optimization for Patrick Main Session: ReaFIR → ReaEQ →
ReaComp → ReaGate now share one isolated plugin process and one realtime
queue. VST3 remains on its existing worker path. Requirements covered:
PLUG-03/04/05/07, GRAPH-14/15, SEC-07, UI-04/05 and API-08.

Grouping follows exclusive identity connections with equal channel shapes;
branches, bypassed plugins, native processors and channel conversions split
groups. Each group is bounded to eight plugins. State, parameters, editors and
fingerprints remain per instance. A member fault silences and fails every
member; an independent worker continues. Recovery requires full preparation.
The callback uses bounded queues and does not wait for the worker.

## Windows evidence

Environment: Windows 11 build 26200, x64, existing installed ReaPlugs and
PD200X/CABLE-A/CABLE-B/Scarlett endpoints. The shell was stopped throughout.
Live checks used a disposable database copy under `target`, privacy mute,
and elevated device access. Media-device identity/state was unchanged before
and after the guarded check. No driver installation or settings changes.

Reproduce the final release-worker check from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m06-vst2-chain.ps1 -AllowLiveAudio -LiveDatabase target/plugin-chain-qualification/state.sqlite -WorkerExecutable target/patrick-main-release-2/release/audiorouter-plugin-worker.exe
```

The database must already be a disposable copy containing Patrick Main
Session. Fixture DLLs must first exist as described by the acceptance wrapper.
Local transcript: `target/plugin-chain-qualification/qualification.log`.
The wrapper restores environment variables and checks media state in finally.

- Both opted-in shared-chain fixture tests passed: processing order, distinct
  parameters/state, bridge lifetime, fingerprint mismatch rejection, crash,
  hang and nonfinite containment, plus survival of an already-running separate
  worker. The protected callback remained bounded and produced silence on fault.
- The final native Start/pump passed: all four plugins reported `running`,
  zero failures, 4099 delivered branch blocks, and all four handles explicitly
  verified the same worker identity.
- Timing snapshot in milliseconds: mic 14.7425; shared plugin queue 18.6667;
  remaining three plugin queues 0; CABLE-A output 26.2397; monitor 27.4206.
  Estimated voice-to-CABLE-A total is 59.65 ms versus the previous agent's
  approximately 97 ms snapshot. Game capture/output total was 51.17 ms.
- ReaEQ and ReaComp each opened/closed its own editor in a two-instance worker
  using `runtime_bridge_saves_state_and_opens_the_editor_in_a_pumping_parent`
  with `AUDIOROUTER_VST2_EDITOR_CHAIN_MEMBER` set to the second DLL.

These are route telemetry snapshots, not calibrated audible latency or p95
evidence. Plugin-reported algorithmic latency remains outside this display.
Long-running soak, attended listening and broad shared-chain vendor coverage
remain unqualified. The user's actual four VST2 plugins are covered.

## Portable checks and artifacts

- `cargo check -p audiorouter-control -p audiorouter-plugin-host --all-targets --features audiorouter-plugin-host/test-fixtures --locked`: passed.
- Package tests: control 193 passed/5 ignored; plugin-host library 72 passed;
  worker binary 2 passed; worker-process integration 35 passed/14 ignored.
  Opted-in Windows tests above were run separately.
- `node tools/contracts/check-drift.mjs`: passed (98 methods, 30 node kinds,
  16 processors, 21 events).
- Clippy without dependency linting passed with existing warnings. Strict
  clippy remains blocked by existing `spectral.rs` needless-range-loop lint.
  Workspace formatting check exposes extensive pre-existing formatting drift;
  formatting was limited to changed lines.
- `npm.cmd run build` in `ui`: passed after elevated retry of denied dist
  unlink. UI index timestamp 12:59:42 precedes release worker 13:03:54 and
  shell 13:05:07 (local time). No UI source changes.
- Release worker and shell built with `--release --locked` into
  `target/patrick-main-release-2`. The shell was not launched afterward.
- Documentation acceptance passed: 61 Markdown files, 297 local links.
  `git diff --check` passed; the embedded JS contains current Timing and
  native-path preparation strings. M08 traceability failed on existing missing
  CAP-13 and GRAPH-15 delivery-map entries; no release gate is claimed.
- Artifact SHA-256: shell
  `A10B0567192C47D76BF3589CA3CDEE25EBA04FFBB3E5BB892DD3DD003A931E48`;
  worker `CD498DCEF98B7221F26DB14BBAFB83C92E07959E6E39E3BA349363D4BE85817D`.

## Failed experiments and corrections

- Unelevated capture initialization returned E_ACCESSDENIED; elevated exact
  endpoint qualification passed. No permission error was treated as success.
- Reusing fixed idempotency keys with a persisted test database replayed an
  old Start response without starting a new runtime. Acceptance now generates
  per-run keys; repeated live runs passed afterward.
- Concurrent test builds briefly locked the debug worker executable; a
  sequential retry passed. PowerShell external stderr redirection initially
  terminated the wrapper; internal transcript capture resolved it.

## Handoff and rollback

The optimized executable is ready at
`C:\code\audiorouter\target\patrick-main-release-2\release\audiorouter-shell.exe`.
Next task: user launches this build, plays Patrick Main Session and confirms
processed voice, game routing and Timing by ear. Saved graphs/state assets
remain compatible with the previous per-instance worker implementation;
rollback is reverting the chain protocol, bridge and grouping changes together.
