# Active plan

Updated: 2026-09-09.

## Current state

The specification baseline has been implemented incrementally on `main`. Portable
domain, control, storage, engine, DSP, recording, UI, MCP, plugin-worker, and
release-preparation slices are present and continuously validated. Native
Windows endpoint, process-loopback, VST3, and SysVAD evaluation evidence is
recorded below; production routing, owned-driver distribution, signing,
installer, clean-machine, and manual UI gates remain open. Read the
[documentation index](../../README.md) and [delivery map](../../spec/15-delivery.md).

The latest clean full M00-M08 acceptance passed at pushed head `16aed8a1`; the
latest portable metering checkpoint is `ca4aafd0`; the latest guarded/live
qualification checkpoint is `977188a5`; later
commits only update the execution evidence below. The focused native auxiliary-bus
transformation and validated state-restoration regressions and all-features
plugin-host checks pass at this checkpoint. The repository is clean; no driver or
startup/plugin registration, signing-mode change, audio stream, or persistent
machine audio configuration has been performed. The gated x64 VST2 boundary
is implemented, but rights/editor/release qualification remains open.

The active branch is currently pushed through `fb527a02`; later commits after
the last full M00-M08 run are contained worker/control-boundary and evidence
updates. The repository remains clean, and the M07 headless acceptance has
passed at this line without audio, driver, registration, signing, or machine
configuration changes.

Next action: qualify the first supplied rights-cleared independent x64 VST2 or
VST3 effect through the existing contained worker matrix. If none is supplied,
continue only with portable hardening and preserve the native-shell, production
driver callback, signing, installer, clean-machine, physical-latency, editor,
and independent-plugin gates as blocked prerequisites rather than claiming
release completion.

- Performed read-only identity inspection of the installed Pitchproof x64 DLL
  on 2026-09-09: the file is 1,077,760 bytes, PE machine `0x8664` (x64),
  eight sections, SHA-256
  `1974A3033B53AE72DA5F419A9F37056D44C1610591BFD11A615ACE0C448CF050`.
  Execution through the native worker was deliberately not attempted because
  arbitrary installed DLL execution cannot be made safe by rollback alone.
  This is identity evidence only and does not qualify Pitchproof or close the
  independent-vendor plugin gate.
- Ran the repository `plugins inspect --path` command against that same file on
  2026-09-09. The read-only inspector classified it as VST2 x64 with
  compatibility `supportedVst2X64Gated`, matching the PE and SHA-256 evidence;
  vendor, version, and class IDs were unavailable. The inspector did not load
  or execute plugin code, and no machine or audio state changed.
- Re-ran the focused CLI regression `cargo test -p audiorouter-cli
  plugin_scan_cli -- --nocapture` on 2026-09-09; the invalid-candidate
  visibility test passed. This confirms the bounded scan/inspection boundary
  remains test-covered without loading arbitrary plugin code.
- Re-ran the complete plugin-host qualification at the current head on
  2026-09-09: 67 library tests and 13 worker-process tests passed. Coverage
  includes VST2/VST3 identity and layout boundaries, worker containment,
  identity revalidation, state integrity, latency, editor lifecycle,
  protected-path failure policy, and shared-bus transport. No arbitrary
  installed plugin was loaded and no machine or audio configuration changed.
- Next safe task: continue with rights-cleared fixtures or portable/native
  gates whose execution boundary is already authorized; retain independent
  plugin execution, production driver, signing, installer, shell/HWND,
  physical-latency, and manual accessibility blockers.

- Requalified the authorized Rust process-loopback adapter on 2026-09-09:
  include mode converted 4,410 source frames at 44.1 kHz into 4,736 engine
  frames at 48 kHz across 37 scheduler quanta; exclude mode converted 3,969
  source frames into 4,224 engine frames across 33 quanta. Both modes reported
  zero rejected packets, xruns, and queue overruns, with bounded resampling
  and generation-1 scheduling. Streams stopped/reset and media state remained
  unchanged.
- Next safe task: continue with remaining physical-latency and native adapter
  evidence while retaining production driver, signing, installer, shell/HWND,
  accessibility, and independent-vendor blockers.

- Completed UI-02 multi-selection state propagation on 2026-09-09. React Flow
  drag selection now updates the application selection set, visibly marks every
  selected node, reports the count through an accessible live status, and keeps
  the primary inspector selection synchronized. Disconnected mode remains
  presentation-only. UI typecheck, all 111 UI tests, and the production Vite
  build (210 modules in a disposable output directory) passed. The normal
  `ui/dist` output was locked by an existing process, so it was not overwritten;
  the temporary build output was removed and no audio or machine configuration
  changed.
- Next safe task: continue portable hardening only where a concrete uncovered
  contract is found; retain the native driver callback, signing, installer,
  shell/HWND, physical-latency, manual accessibility, and independent-plugin
  gates.

- Requalified the local legacy VST2 ReaPlugs boundary on 2026-09-09: six x64
  effects (ReaComp, ReaDelay, ReaEQ, ReaFIR, ReaGate, and ReaXComp) passed
  contained worker processing at 44.1, 48, and 96 kHz, including parameter
  offset coverage and before/after DLL integrity checks. The script restored
  `AUDIOROUTER_VST2_FIXTURE` and `AUDIOROUTER_VST2_SAMPLE_RATE`; no plugin
  registration, audio stream, or machine configuration changed. Independent
  rights-cleared VST2 qualification remains open.
- Next safe task: run the installed-plugin boundary check, then retain the
  independent-plugin, editor, native driver callback, signing, installer,
  shell/HWND, physical-latency, and manual accessibility gates.

- Requalified one explicitly selected x64 VST2 binary through the installed
  boundary on 2026-09-09: ReaComp processing passed at 44.1, 48, and 96 kHz;
  dedicated editor-thread containment and supervised editor-timeout tests
  passed; and the SHA-256 fingerprint remained unchanged. The runner restored
  both VST2 environment variables and performed no copy, registration, audio
  stream, or machine configuration action. This is local fixture evidence, not
  independent-vendor rights or release qualification.
- Next safe task: continue with the next concrete portable/native qualification
  gate while retaining independent-plugin, native driver callback, signing,
  installer, shell/HWND, physical-latency, and manual accessibility blockers.

- Requalified M06 native VST3 worker acceptance on 2026-09-09: isolated AGain
  single-stream and auxiliary-bus processing, asynchronous graph staging,
  bounded failure/restart/quarantine recovery, validated state restoration,
  repeated-quantum timing, finite transformed output, and bounded shutdown all
  passed. This is repository-local fixture evidence; no plugin registration,
  audio stream, or machine configuration changed.
- Next safe task: continue independently observable acceptance gates while
  retaining the independent-vendor, native driver callback, signing,
  installer, shell/HWND, physical-latency, and manual accessibility blockers.

- Requalified M07 headless acceptance on 2026-09-09: 29 CLI tests, 2 MCP stdio
  interoperability tests, 99 control tests, 67 plugin-host tests, 13 worker
  process tests, doc-tests, and M01 CLI parity all passed. The run covered
  recovery, privacy mute, recording, authorization, idempotency, paging,
  plugin boundaries, and persisted operations without audio-device access or
  machine configuration changes.
- Next safe task: continue with the next independently observable acceptance
  gate while retaining the native driver callback, signing, installer,
  shell/HWND, physical-latency, manual accessibility, and independent-vendor
  plugin blockers.

- Requalified M08 unsigned release preparation on 2026-09-09: optimized CLI
  artifacts, production UI bundle (210 modules), SBOM/notice material, hashes,
  archive contents, and unsigned/publication-blocker assertions passed. All
  release output was disposable and removed by the runner; no installer,
  driver, signing operation, or audio configuration action occurred.
- Next safe task: continue with any remaining independently observable plan
  gate; do not represent the product as releasable until signing, installer,
  clean-machine, driver, shell/HWND, physical-latency, accessibility, and
  independent-vendor plugin evidence exists.

- Revalidated documentation gates on 2026-09-09: M08 traceability covered all
  159 normative requirement IDs, and documentation validation passed for 51
  Markdown files with 163 local links. A full workspace test rerun reached the
  later target stages but its final exit summary was not observable through the
  execution wrapper, so it is intentionally not recorded as a complete pass.
  No audio, driver, signing, or machine configuration action occurred.
- Next safe task: continue with independently observable package gates or
  concrete portable hardening; retain all native production and independent-
  vendor qualification blockers.

- Revalidated M06 SDK installer provenance on 2026-09-09: the checked-in
  provenance/lock metadata acceptance passed using disposable Git metadata.
  No SDK installation, plugin registration, driver action, or audio
  configuration change occurred.
- Next safe task: continue with the remaining independently observable native
  qualification scripts while retaining production driver, signing, installer,
  shell/HWND, physical-latency, accessibility, and independent-vendor gates.

- Revalidated the read-only M00 native format inventory on 2026-09-09:
  31 active endpoints were enumerated, including validated 48 kHz mono/stereo
  and 96 kHz mono/eight-channel format metadata. GetMixFormat and endpoint
  activation probes passed without starting an audio stream or modifying
  driver, defaults, volume, mute, privacy, signing, or startup configuration.
- Next safe task: continue with any remaining independently observable native
  adapter evidence while retaining differing-rate, production-driver,
  signing, installer, shell/HWND, physical-latency, accessibility, and
  independent-vendor blockers.

- Requalified M06 VST3 SDK acceptance on 2026-09-09: the pinned checkout built,
  validator self-tests passed, offline loader and AGain main/auxiliary-bus
  classes passed, explicit single-bus rejection passed, and the five-class mda
  matrix passed. Outputs were repository-local/disposable; no system install,
  plugin registration, audio stream, or machine configuration changed.
- Next safe task: continue with remaining independently observable native
  qualification while retaining production driver, signing, installer,
  shell/HWND, physical-latency, manual accessibility, and independent-vendor
  plugin blockers.

- Closed the portable workspace test evidence gap on 2026-09-09 with
  independently observable locked package runs: engine (89), recording (30),
  storage (80), Windows audio (33), protocol (6), and transport (19) tests all
  passed with exit code 0. Together with the independently verified CLI (29),
  MCP (2), control (99), domain (59), DSP (30), plugin-host (67), and worker
  process (13) suites, the current portable evidence covers 556 passing tests.
  No audio stream, driver, signing, or machine configuration action occurred.
- Next safe task: continue with any remaining independently observable plan
  gate; native production, signing, installer, shell/HWND, physical-latency,
  manual accessibility, and independent-vendor plugin gates remain open.

- Revalidated strict portable lint on 2026-09-09: `cargo clippy --workspace
  --all-targets --all-features --locked -- -D warnings` completed successfully
  with no warnings or errors. No audio, driver, signing, or machine
  configuration action occurred.
- Next safe task: continue with the remaining independently observable native
  qualification gates while retaining production driver, signing, installer,
  shell/HWND, physical-latency, manual accessibility, and independent-vendor
  plugin blockers.

- Requalified the focused M05 UI acceptance at the current head on 2026-09-09:
  TypeScript typecheck, all 16 UI test files/111 tests, and the temporary
  production Vite bundle (210 modules, three output files) passed. The runner
  removed its temporary output; no audio, driver, registration, or machine
  configuration changed.
- Next safe task: continue portable hardening only where a concrete uncovered
  contract is found; retain the native driver callback, signing, installer,
  shell/HWND, physical-latency, manual accessibility, and independent-plugin
  gates.

- Requalified M07 headless behavior on 2026-09-09: 29 CLI tests, 2 MCP stdio
  interoperability tests, 99 control tests, 67 plugin-host tests, doc-tests,
  strict all-features Clippy, M01 CLI parity, and diff checks passed. The run
  exercised recovery, startup plans, privacy mute, recording checkpoints,
  permissions, idempotency, paging, MCP dispatch, and persisted operations.
  No audio device, driver, plugin registration, signing, or machine audio
  configuration was changed.
- Next safe task: continue portable hardening only where a concrete uncovered
  contract is found; otherwise retain the explicit native driver, signing,
  installer, shell/HWND, physical-latency, and independent-plugin gates.

- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at
  pushed head `8476aeae` on 2026-09-09 after the UI-03 topology editor
  integration. Toolchain/native compile, read-only endpoint inventory,
  disposable SysVAD qualification, M01/M04/M05, pinned VST3 SDK/native
  workers, repository VST2 modern/legacy/state/fault fixtures, M07 (including
  67 plugin-host and 13 worker-process tests), unsigned M08 artifacts, 159
  traceability IDs, and documentation validation (51 Markdown files/163 local
  links) passed. The run removed 13 run-owned temporary children and used
  logs outside the repository; no driver installation/loading, signing-mode
  change, plugin/startup registration, audio stream, or persistent machine
  audio configuration occurred.
- Next safe task: continue portable hardening only where a concrete uncovered
  contract is found; otherwise retain independent-plugin, native-shell/HWND,
  production-driver, signing, installer, clean-machine, and physical-latency
  gates.

- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at
  pushed head `0ee952ad` on 2026-09-09. Toolchain/native compile, read-only
  endpoint inventory, disposable SysVAD x64 compile/package/API/signability,
  M01/M04/M05 (including the current UI build and tests), pinned VST3 SDK and
  native workers, repository VST2 modern/legacy/state/fault fixtures, M07,
  unsigned M08 artifacts, 159 traceability IDs, and documentation validation
  (51 Markdown files/163 local links) all passed. The runner removed 13
  run-owned temporary children and used logs outside the repository; no driver
  installation/loading, signing-mode change, plugin/startup registration,
  audio stream, or persistent machine audio configuration occurred.
- Next safe task: continue portable hardening only where a concrete uncovered
  contract is found; otherwise retain the independent-plugin, native-shell/
  HWND, production-driver, signing, installer, clean-machine, and physical-
  latency gates.

- Hardened M06/PLUG-04 generic VST3 controls on 2026-09-09: the native worker
  now rejects duplicate controller parameter IDs before emitting its bounded
  descriptor response, matching the wire-contract uniqueness invariant. The
  native VST3 acceptance passed with descriptor uniqueness, fail-closed editor
  requests, and continued processing; plugin-host tests (67), formatting, and
  diff checks also passed. No audio or machine configuration changed.
- Next safe task: continue portable hardening only where a concrete uncovered
  contract is found; retain independent-plugin, native-shell/HWND, production
  driver, signing, installer, and physical-latency gates.

- Requalified M08 unsigned release preparation on 2026-09-09: optimized CLI and
  plugin-worker artifacts, production UI archive, Cargo/npm SBOMs, third-party
  notices, manifest hashes, archive contents, and unsigned/publication-blocker
  assertions all passed. Temporary release output was removed; no installer,
  driver, signing, or audio configuration action occurred.
- Next safe task: continue portable hardening only where a concrete uncovered
  contract is found; retain independent-plugin, native-shell/HWND, production
  driver, signing, installer, clean-machine, and physical-latency gates.

- Fixed disposable native-qualification process leakage on 2026-09-09: the
  SysVAD MSBuild wrapper and VST3 SDK CMake/Visual Studio acceptance now disable
  MSBuild node reuse, preventing reusable child nodes from outliving temporary
  checkouts. Isolated SysVAD qualification and M06 VST3 SDK acceptance both
  passed, and post-run process checks found no MSBuild/compiler children or
  temporary SysVAD checkout. No driver was installed or loaded and no audio
  configuration changed.
- Next safe task: complete the full guarded M00-M08 chain at this cleanup-fixed
  head, then retain independent-plugin, native-shell/HWND, production driver,
  signing, installer, clean-machine, and physical-latency gates.

- Hardened M00-M08 acceptance orchestration on 2026-09-09: `safe-all.ps1` now
  invokes checked-in step scripts in-process, keeping cleanup and failures
  attached to one runner instead of hiding them behind nested PowerShell
  children. The isolated SysVAD and M06 SDK gates passed with no residual
  compiler/build processes or temporary checkout. A full-chain attempt reached
  later milestones, but its terminal output was lost by the execution wrapper,
  so it is intentionally not recorded as an end-to-end pass.
- Next safe task: validate the direct-runner chain through independently
  observable milestone gates; retain all native production and release blockers.

- Revalidated M08 traceability on 2026-09-09: the delivery map covers all 159
  normative requirement IDs. This is documentation coverage evidence only and
  does not waive implementation, hardware, driver, signing, or release gates.
- Next safe task: continue portable hardening only where a concrete uncovered
  contract is found; retain independent-plugin, native-shell/HWND, production
  driver, signing, installer, clean-machine, and physical-latency gates.

- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at
  pushed head `f6ec7a1c` on 2026-09-09. VS2026/WDK discovery and native compile,
  read-only 31-endpoint inventory, disposable SysVAD x64 compile/package/API/
  signability qualification, M01/M04/M05, pinned VST3 SDK/validator and native
  AGain single-/multi-bus workers, repository VST2 modern/legacy/state/fault
  fixtures at 44.1/48/96 kHz, M07, unsigned M08 preparation, 159 traceability
  mappings, and documentation validation (51 Markdown files/163 local links)
  passed. Temporary outputs and environment overrides were restored; no driver
  installation/loading, signing-mode change, plugin/startup registration, audio
  stream, or persistent machine audio configuration occurred.

- Requalified the locked all-features workspace after that acceptance run on
  2026-09-09: all workspace unit/integration tests and doc-tests passed,
  including the expected fixture-dependent skips, and strict all-target Clippy passed with warnings
  denied. The repository remained clean and no runtime or machine audio
  configuration was changed.

- Closed a UI-07 stale-graph feedback gap on 2026-09-09. Graph planning and
  acknowledged commit failures now preserve structured backend remediation,
  recognize the typed `revisionConflict` code, refresh the authoritative
  session snapshot, discard stale warning state, and ask the operator to review
  the draft again; errors are never resolved by retrying a stale plan. UI
  typecheck and 98 tests pass. Native shell/manual conflict and accessibility
  acceptance remain open.

- Extended UI structured-error presentation on 2026-09-09 across startup,
  plugin discovery/inspection, session and recording inventory, route
  inspection, and recovery inspection. These paths now preserve typed backend
  codes, HRESULTs, retryability, and remediation through the same formatter as
  graph operations. UI typecheck and 98 tests pass; native shell/manual
  accessibility acceptance remains open.

- Completed the remaining high-value UI action/error paths on 2026-09-09:
  recorder lifecycle, virtual-device inventory/planning/apply, and the
  corresponding graph-adjacent refreshes now use the structured backend error
  formatter. UI typecheck, 98 tests, and diff checks pass; native endpoint and
  manual shell acceptance remain open.

- Completed the broader UI structured-error audit on 2026-09-09. Recorder
  controls, virtual-device operations, session lifecycle, privacy/recovery
  actions, recording file actions, and catalog/inventory refreshes now retain
  typed backend diagnostics through the shared formatter. UI typecheck, 98
  tests, documentation validation, and diff checks pass; native shell/manual
  accessibility acceptance remains open.

- Finished the UI draft-edit error audit on 2026-09-09. Node/session naming,
  connection creation/removal/toggling, and node add/remove/duplicate failures
  now use the same structured formatter, leaving no direct raw `Error.message`
  presentation path in `App.tsx`. UI typecheck and 98 tests pass; native
  shell/manual accessibility acceptance remains open.

- Revalidated the UI production artifact path on 2026-09-09 after the
  structured-error changes. TypeScript compilation and Vite production output
  passed into a validated temporary directory containing the expected
  `index.html` and asset files; the locked repository `ui/dist` output was not
  changed, and the temporary directory was removed.

- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at
  pushed head `0c9641bd` on 2026-09-09 after the UI diagnostics audit. The
  elevated read-only run passed VS2026/WDK discovery and native compile,
  31-endpoint inventory, disposable SysVAD x64 compile/package/API/signability,
  M01/M04/M05, pinned VST3 SDK/validator and native workers, repository VST2
  modern/legacy/state/fault fixtures, M07, unsigned M08 preparation, 159
  traceability mappings, and documentation validation (51 Markdown files/163
  local links). Temporary outputs and environment overrides were restored; no
  driver installation/loading, signing-mode change, plugin/startup
  registration, audio stream, or persistent machine audio configuration
  occurred.

- Closed the final user-visible UI raw-error path on 2026-09-09: the
  backend-derived EQ response request now preserves structured diagnostics
  through the shared formatter. The UI audit reports no remaining direct
  `Error.message` presentation expressions in `App.tsx`; typecheck and 98 tests
  pass. Native shell/manual accessibility acceptance remains open.

- Probed the repository-local mda VST3 bundle through the generic native
  AudioRouter worker on 2026-09-09. Its x64 binary is 3,375,616 bytes with
  SHA-256 `727b8396f9092755f18a62d7a9bfec588b7cea5c136cca297a8010aad3fa467f`;
  single-stream processing, finite transformation, state save, supervised
  restart/restoration, and shutdown passed at 48 kHz. This is native
  multi-vendor processing/state evidence only; editor, additional-class, and
  full M06 acceptance gates remain open.

- Generalized the native single-stream VST3 worker regression on 2026-09-09 to
  accept the bounded `AUDIOROUTER_VST3_SAMPLE_RATE` override, preserving the
  48 kHz default. This enables reproducible mda/AGain rate-matrix runs without
  changing production audio settings.

- Requalified the mda native VST3 worker at 44.1/48/96 kHz on 2026-09-09 using
  the new rate override. All three isolated processing/state/restart runs
  passed; plugin-host all-features validation also passed (67 unit tests, 35
  worker-process tests with 9 fixture-dependent ignores, doc-tests, and strict
  Clippy). Temporary worker outputs and environment overrides were restored.

- Requalified the Steinberg AGain native VST3 worker at 44.1/48/96 kHz on
  2026-09-09 using the same explicit-rate seam. Its x64 binary is 4,990,976
  bytes with SHA-256
  `7dabe7771290361b418bc2aa4ae6246c8660ca09baa63aea61f41e27026fa8a6`;
  processing, finite output, state save, supervised restart/restoration, and
  shutdown passed at each rate. Native editor and complete M06 gates remain
  open.

- Extended the native VST3 auxiliary-bus launch boundary on 2026-09-09 with an
  explicit sample-rate variant while preserving the 48 kHz API default. AGain's
  two-input/one-output side-chain worker and graph-result staging passed at
  44.1/48/96 kHz; worker-process multi-bus tests and strict plugin-host checks
  pass. Production driver and physical-latency gates remain open.

- Closed a DSP-01/03 contract drift on 2026-09-09. The compressor's supported
  knee control and the gate's supported hysteresis, ratio, and hold controls are
  now accepted by domain validation, described by both node and processor API
  schemas, and consumed by engine compilation. Defaults now match the processing
  specification (6 dB knee, 4:1 gate ratio, 50 ms hold). Focused domain,
  control-description, and engine compilation tests pass; this is portable
  contract evidence and does not advance the separate native, plugin-rights, or
  release gates.

- Closed the portable graph portion of the DSP-02 gap on 2026-09-09. The
  `parametric-eq@1` API now exposes eight bounded, independently enabled bands
  with peaking, shelf, pass, and notch filter types; the engine prepares all
  enabled bands, while the legacy one-band fields remain band-0 aliases.
  Domain/engine/control regressions, formatting, and strict Clippy pass. The
  remaining DSP-02 gates are backend-derived UI curve delivery, native callback
  timing, and hardware/release qualification.

- Added the combined `ParametricEq::magnitude_db_at` response boundary and a
  deterministic six-shape response vector on 2026-09-09. The vector exercises
  enabled peaking, low/high shelf, low/high pass, and notch bands using the
  same coefficients as processing; 29 DSP tests, doc-tests, and strict Clippy
  pass. A backend-derived UI curve contract is still required before the UI
  can render response data without duplicating DSP math.

- Added `processors.response` on 2026-09-09 as the backend-derived UI curve
  transport. It accepts bounded sample-rate, eight-band, and 256-frequency
  inputs and evaluates the shared DSP coefficient path without touching graph,
  audio, plugin, or machine state. Control regression coverage confirms the
  response shape, a peaking response, and empty-frequency rejection. The UI
  renderer still needs to consume this contract; native callback and release
  gates remain separate.

- Completed the first UI consumer on 2026-09-09. The selected-node processor
  catalog now renders a backend-derived logarithmic 20 Hz–20 kHz response SVG
  for parametric EQ, with loading, disconnected, and failure-safe states. It
  translates the existing eight-band/legacy band-0 draft fields into the typed
  response request and never computes filter coefficients in TypeScript. UI
  typecheck, 97 tests, and a disposable production build passed; the locked
  existing `ui/dist` output was not changed.

- Closed the portable DSP-04 limiter contract gap on 2026-09-09. Limiter
  parameters now include bounded 0–10 ms lookahead and 10–1,000 ms release;
  the DSP preallocates per-channel delay storage, applies immediate limiting,
  and releases gain exponentially. The engine constructs it at the negotiated
  sample rate behind a realtime try-lock, reset clears delay/gain state, and
  the catalog discloses the 5 ms default as 240 samples at 48 kHz. Focused DSP
  and engine tests cover ceiling, finite repair, latency, and reset behavior.
  True-peak protection remains explicitly unsupported.

- Closed the portable runtime portion of DSP-07 on 2026-09-09. Prepared graph
  meters now retain per-channel peak, RMS, and clipping counters alongside the
  aggregate compatibility fields, with a documented zero floor for silence.
  The callback updates fixed-size atomics only; activation reset clears all
  channels. Engine regression coverage verifies stereo separation, RMS, and
  clipping. Gate-state and processor-specific gain-reduction telemetry remain
  separate follow-up API work; this does not advance native or release gates.

- Closed the engine-side portion of that DSP-07 follow-up on 2026-09-09.
  `RuntimeGraph::processor_telemetry` and `RuntimeProcessor::processor_telemetry`
  now provide best-effort, non-blocking dynamics reads: compressor/limiter
  reduction and gate reduction/open state are exposed per channel. A busy
  callback-owned state lock returns no sample rather than making the control
  plane wait. Engine tests cover compressor reduction and gate transitions;
  control-plane serialization and bounded telemetry delivery remain separate
  integration work.

- Extended the prepared meter snapshot on 2026-09-09 with finite aggregate and
  per-channel peak/RMS dB projections while preserving the existing linear and
  clipping fields. The lock-free callback still performs only fixed-size atomic
  updates; dB conversion is read-side work and uses a documented -120 dB floor
  after reset or silence. Engine regressions cover populated values, reset
  silence, and channel separation. The windowed 300 ms RMS/1 second peak-hold
  defaults remain in the DSP signal-meter layer.

- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at
  pushed head `6d8e6ad2` on 2026-09-09. M00 toolchain/native format inventory
  and disposable SysVAD, M01, M04 (30 DSP/30 recording tests), M05 UI,
  M06 VST3/VST2, M07, unsigned M08 preparation, traceability, and
  documentation all passed. Temporary outputs were cleaned; driver
  installation/loading, signing-mode changes, plugin/startup registration,
  audio streams, and persistent machine audio configuration remained out of
  scope.

- Requalified the guarded `safe-all.ps1` chain at `20bc70b6` on 2026-09-09
  after the finite meter dB contract change. Elevated native read-only access
  was required for the 31-endpoint PnP inventory; all stages then passed,
  including M04, native VST3 workers, VST2 chunk-state and legacy-main fixture
  coverage at 44.1/48/96 kHz, M07, M08, 159 requirement mappings, and 51-file
  documentation validation. Temporary outputs were removed. No driver was
  installed or loaded, and no plugin registration, audio stream, signing-mode,
  startup, or persistent machine audio configuration action occurred.

- Requalified the user-installed ReaComp VST2 binary on 2026-09-09 through
  `m06-vst2-installed.ps1`. SHA-256 was
  `4c0862ab3cfd8a0345481b4792c07bf8d5a9761014f217d4e13669bf8143c7a0`; real
  worker processing passed at 44.1/48/96 kHz, and the bounded native-editor
  containment and supervised timeout checks passed. The wrapper used process
  environment overrides and made no registration, copy, audio, or persistent
  machine configuration change. This advances local compatibility evidence,
  not third-party rights or release qualification.

- Ran the installed-directory ReaPlugs VST2 wrapper on 2026-09-09 with
  `-SkipIncompatibleCandidates`. Seven x64 binaries passed all 44.1/48/96 kHz
  worker cases (ReaComp, ReaDelay, ReaEQ, ReaFIR, ReaGate, ReaStream, and
  ReaXComp); ReaControlMIDI and ReaJS were isolated and explicitly rejected
  for incompatibility with the bounded audio-effect/state contract. No plugin
  registration, copy, audio stream, or machine configuration occurred.

- Hardened the installed VST2 acceptance wrapper on 2026-09-09 by removing its
  machine-specific Pitchproof default. It now requires an explicitly selected
  absolute DLL path and the compatibility runbook provides a ReaComp example,
  preventing accidental qualification of an unrelated or stale installation.

- Completed the exact-binary record for the installed ReaPlugs matrix on
  2026-09-09: all nine DLL sizes and SHA-256 fingerprints are now documented,
  with seven passed and two explicitly rejected. Embedded file-version fields
  were absent for every binary, so hashes and sizes are retained as the local
  reproducible identifiers; no third-party rights or redistribution claim is
  inferred.

- Hardened the DSP-07 meter dB projection on 2026-09-09: non-finite read-side
  values now fail closed to the documented -120 dB floor, with regression
  coverage for NaN and positive infinity. The realtime path remains unchanged;
  all 89 engine tests and strict engine Clippy pass.

- Hardened `m06-vst2-installed.ps1` on 2026-09-09 with a post-run size and
  SHA-256 equality check for the explicitly selected DLL. This extends the
  contained-worker evidence to verify that plugin execution did not mutate its
  binary; explicit ReaComp processing/editor containment still passes.

- Extended the directory-wide ReaPlugs wrapper on 2026-09-09 with before/after
  size and SHA-256 checks for every x64 candidate. This preserves isolated
  incompatible-candidate reporting while failing closed if any loaded plugin
  mutates its binary; the existing worker matrix and environment restoration
  remain unchanged.

- Performed a read-only inventory of standard installed plugin directories on
  2026-09-09. No additional VST3 bundle was present beyond the known Pitchproof
  x64/x86 VST2 pair; the ReaPlugs directory remains VST2-only. The independent
  second-vendor VST3 gate is therefore still externally dependent, and no
  plugin was loaded, copied, registered, or modified by the inventory.

- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain after
  processor telemetry at working head `efd5a5f4` on 2026-09-09. M00 toolchain,
  native format inventory, disposable SysVAD compile/package/API/signability,
  M01, M04 (30 DSP/30 recording tests), M05 (97 UI tests and temporary build),
  M06 SDK/native VST3 worker and modern/legacy/fault VST2 fixtures, M07, M08
  unsigned artifacts, 159 traceability mappings, and documentation validation
  (51 Markdown files/163 local links) passed. The run emitted only the known
  disposable SysVAD INF encoding warnings and cleaned its temporary outputs;
  no driver installation/loading, signing-mode change, plugin/startup
  registration, audio stream, or persistent machine audio configuration
  occurred.

- Requalified the explicitly authorized live M00/M02 paths after the portable
  telemetry work on 2026-09-09. Native C++ capture/render lifecycle passed for
  13 capture and 18 render endpoints; Rust process-loopback include/exclude
  passed with 10,584/11,025 source frames and 11,392/11,904 engine frames; and
  the Rust adapter smoke passed with 24,000 capture frames and 25,152 silent
  render frames. The selected VB-Audio route also passed with 24,480 capture,
  24,448 scheduler, and 23,488 routed frames, zero scheduler XRuns/deadline
  misses, and bounded processing telemetry. Every wrapper stopped/reset its
  streams, removed temporary outputs, and observed unchanged media-device
  state. This proves current shared-mode digital adapter/route behavior; it
  does not close managed-driver, physical-latency, signing, or native-shell
  gates.

- Extended the authorized native signal-path evidence on 2026-09-09 using the
  existing VB-Audio endpoints: event-driven capture/render passed with 24,000
  capture and 28,800 render frames; native digital tone loopback captured
  67,438 nonzero payload bytes; and bounded impulse correlation detected 99 of
  100 impulses with zero p95 spacing error and an estimated 50.19 ms onset.
  Each run stopped/reset/released its streams, removed temporary files, and
  observed unchanged media-device state. The onset estimate is not physical
  acoustic latency evidence; managed-driver, signing, and native-shell gates
  remain open.

- Completed the remaining bounded native process-loopback probes on 2026-09-09:
  process attribution captured 21,609 frames/76,370 nonzero bytes, and
  process exclusion captured 22,050 frames; both child-process lifecycle runs
  passed with unchanged media state and cleanup. These are API/lifecycle and
  controlled-tree evidence only, not a full cross-process isolation threshold
  or production virtual-driver claim.

- Requalified the current endpoint-ID-selected differing-rate route on
  2026-09-09: 96 kHz mono capture fed 48 kHz stereo render for 48,000 capture,
  23,936 scheduler, and 23,936 routed frames over 500 ms. The run reported a
  1,333,334 ns graph deadline, 32,768 ns processing p99.9 upper bound, zero
  deadline misses/XRuns, and bounded resampling; stream cleanup and unchanged
  media state passed. This remains short-duration shared-mode evidence, not
  independent-clock lock, managed-driver callback, or physical-latency proof.

- Completed the corresponding UI contract slice on 2026-09-09. Shared discovery
  metadata now carries enumerated filter choices, and the processor editor
  renders them as bounded selects instead of dropping string parameters. UI
  typecheck, 97 UI tests, and an elevated disposable production build passed;
  the repository's existing `ui/dist` output was left untouched after its
  Windows file lock caused the default build cleanup to return `EPERM`.

- Requalified the clean M00/M01/M04/M05/M06/M07 chain and M08 release
  preparation at pushed head `669380e1` on 2026-09-09. The read-only endpoint
  inventory found 31 endpoints; disposable x64 SysVAD compilation/package and
  signability passed; UI, CLI/MCP, native VST3 worker, and modern/legacy VST2
  fixture gates passed; and unsigned optimized Rust/UI artifacts were prepared
  and verified. The first run stopped at M08 because a formatter-only change
  made the tree dirty; after committing that correction, M08 passed from the
  clean tree. Temporary checkouts/artifacts were removed and no driver,
  plugin-registration, signing-mode, stream, or persistent audio change was
  made.

- Ran the existing contained-worker acceptance against four user-supplied local
  ReaPlugs x64 VST2 effects on 2026-09-09: ReaComp, ReaEQ, ReaDelay, and ReaGate
  each loaded and processed through the native VST2 adapter at 44.1, 48, and
  96 kHz (12 real-plugin processing cases passed). The check used only
  process-local `AUDIOROUTER_VST2_FIXTURE` and `AUDIOROUTER_VST2_SAMPLE_RATE`
  values and restored both afterward; it did not copy, register, edit, or
  commit the installed binaries and did not change machine audio state. This
  is compatibility evidence for PLUG-07, not rights, editor, latency, or
  release qualification.

- Completed a direct recheck of the remaining local ReaPlugs audio effects on
  2026-09-09: ReaFIR and ReaXComp each loaded and processed at 44.1, 48, and
  96 kHz through the contained VST2 worker (six additional real-plugin cases
  passed). Together with the preceding four-effect run, all 18 combinations
  in the documented six-effect matrix passed. Process-local environment
  overrides were restored and no plugin registration or machine audio change
  occurred; rights, editor, physical-latency, and release gates remain open.

- Audited the real-plugin state path on 2026-09-09. The generic opt-in VST2
  processing acceptance exercises optional state save/restore for third-party
  effects, while the stricter chunk-state regression intentionally rejects
  non-repository filenames because it asserts the fixture's exact one-parameter
  behavior. A direct ReaComp run therefore stopped at that test precondition,
  not in native loading or processing; no adapter defect was reproduced. A
  future state qualification should use a behavior-specific independent test
  rather than weakening the repository fixture's assertions.

- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at
  pushed head `e2b9cfc3` on 2026-09-09. M00–M08 passed: VS2026/MSVC/SDK/WDK
  discovery, read-only 31-endpoint inventory, disposable pinned SysVAD
  compile/package/API/signability, M01/M04/M05, VST3 SDK and native worker
  paths, modern/legacy/fault VST2 fixtures, M07, unsigned M08 artifacts,
  159 traceability mappings, and documentation validation (51 Markdown files,
  163 local links). Temporary outputs were cleaned; driver installation or
  loading, signing-mode changes, plugin/startup registration, audio streams,
  and persistent machine audio configuration remained out of scope.

- Requalified the authorized native M00 live paths on 2026-09-09. Shared
  capture/render passed all 13 capture and 18 render endpoints at 100 ms, with
  one occupied render endpoint correctly classified; event capture/render on
  the explicit VB-Audio pair passed with 24,000/28,800 frames. The reversible
  VB-Audio tone-to-capture path passed with 215,332 nonzero bytes, while
  disposable process attribution and exclusion passed with 21,609/22,050
  frames and 77,823 nonzero bytes for the include path. Media snapshots were
  unchanged and all temporary processes/artifacts were removed. This advances
  CAP-01/02/05/06 evidence but does not claim managed-driver routing,
  calibrated physical latency, or signing completion.

- Extended M00 live evidence on 2026-09-09 with 1,000-impulse digital
  correlation: all groups were detected with zero p95 spacing error and a
  55.75 ms estimated digital onset. The Rust process-loopback adapter also
  passed include/exclude conversion from 44.1 kHz to 48 kHz with 93/89
  scheduler quanta, zero rejected packets, XRuns, and queue overruns or
  underruns. These are repeatable data-path measurements; the onset is not
  calibrated physical latency and no persistent audio configuration changed.

- Requalified the M02 Rust adapter and explicit built-in route on 2026-09-09.
  At 48 kHz and 128-frame quanta, the smoke path processed 191 graph blocks
  and 24,448 scheduler frames; the routed path processed 187 blocks and
  delivered 23,520 routed frames. Both reported zero XRuns, queue
  overruns/underruns, deadline misses, and deadline lateness. Processing-time
  maxima were 25,100 ns and 38,800 ns. This is current user-mode built-in DSP
  and scheduler evidence only; managed-driver callback and physical-latency
  gates remain open.

- Requalified the installed rights-unreviewed Pitchproof x64 VST2 binary on
  2026-09-09 through `m06-vst2-installed.ps1`. SHA-256 was
  `1974a3033b53ae72da5f419a9f37056d44c1610591bfd11a615ace0c448cf050`;
  processing passed at 44.1, 48, and 96 kHz, and both bounded editor tests
  passed, including worker termination/reaping for the non-returning editor
  path. The environment was restored and no copy, registration, audio stream,
  or machine audio configuration change occurred. This is independent local
  compatibility and failure-containment evidence, not rights or release
  qualification.

- Ran the canonical `m06-vst2-reaplugs.ps1` wrapper on 2026-09-09 after the
  direct matrix checks. Its x64 PE classifier and isolated worker loop passed
  all six ignored ReaPlugs audio effects at 44.1, 48, and 96 kHz (18 cases),
  including bounded intra-block parameter automation. The wrapper restored
  both VST2 environment variables; no plugin registration, audio stream, or
  machine configuration changed.

- Clarified the M06 milestone status on 2026-09-09: the approved PLUG-07 native
  x64 VST2 extension is implemented through its separate ABI adapter and
  contained worker, with modern `VSTPluginMain` and legacy `main` fixtures
  passing. Rights/editor review, sandbox acceptance, callback timing, physical
  latency, and release qualification remain gated; documentation validation
  passed and no plugin or machine configuration changed.

- Extended PLUG-07 fixture coverage on 2026-09-09: the repository-owned
  `VSTPluginMain` and legacy `main` x64 VST2 fixtures now run processing and
  chunk-state restoration at 44.1, 48, and 96 kHz. The acceptance wrapper also
  retains non-finite-output, crash, and hang containment checks and restores
  both `AUDIOROUTER_VST2_FIXTURE` and `AUDIOROUTER_VST2_SAMPLE_RATE`. The
  expanded matrix passed using ignored disposable DLLs; no plugin registration,
  audio stream, or machine configuration changed.

- Synchronized the operator compatibility guide on 2026-09-09 with the
  expanded PLUG-07 matrix: it now states that both VST2 entry-point fixtures
  cover processing and chunk-state restoration at 44.1, 48, and 96 kHz, while
  retaining the non-finite and crash/hang containment boundaries. Documentation
  validation passed; no plugin or machine configuration changed.

- Closed a PLUG-03/PLUG-07 bounded-buffer gap on 2026-09-09: the native VST2
  adapter now rejects processing blocks larger than the shared 2,048-frame
  worker limit before invoking plugin code, preventing an oversized length
  from crossing the ABI boundary. The focused VST2 tests (6), strict
  plugin-host Clippy, formatting, and diff checks passed; no plugin or machine
  configuration changed.

- Closed a second PLUG-03/PLUG-07 format-boundary gap on 2026-09-09: the native
  VST2 adapter now enforces the worker's 8–192 kHz negotiated-rate range and
  rejects oversized block sizes before dispatching setup opcodes. Boundary
  regressions verify invalid rates/sizes cause zero dispatcher calls; focused
  VST2 tests, strict Clippy, formatting, and diff checks pass.

- Closed a PLUG-03/PLUG-07 channel-shape gap on 2026-09-09: the native VST2
  process adapter now requires caller channel counts to match the validated
  effect's declared input/output counts before constructing ABI pointer arrays.
  A mismatched-cardinality regression passes with the focused VST2 tests,
  strict Clippy, formatting, and diff checks; no plugin or machine
  configuration changed.

- Closed the remaining PLUG-03/PLUG-07 rate-shape gap on 2026-09-09: the native
  VST2 format setter now rejects fractional sample rates, matching the worker's
  integer-Hz negotiated contract, in addition to the 8–192 kHz range. Focused
  VST2 tests, strict Clippy, formatting, and diff checks pass; no plugin or
  machine configuration changed.

- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at
  pushed head `2844d5f4` on 2026-09-09 after the VST2 ABI hardening: VS2026 /
  MSVC / SDK / WDK discovery, read-only 31-endpoint inventory, disposable
  pinned SysVAD x64 compile/package/API/signability, M01/M04/M05, VST3 SDK and
  native worker paths, modern/legacy/fault VST2 fixtures, M07, unsigned M08
  preparation, 159 traceability mappings, and documentation all passed.
  Temporary outputs were cleaned; no driver installation/loading,
  signing-mode change, plugin/startup registration, audio stream, or persistent
  machine audio configuration occurred.

- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at
  pushed head `2844d5f4` on 2026-09-09 after the VST2 ABI hardening: VS2026 /
  MSVC / SDK / WDK discovery, read-only 31-endpoint inventory, disposable
  pinned SysVAD x64 compile/package/API/signability, M01/M04/M05, VST3 SDK and
  native worker paths, modern/legacy/fault VST2 fixtures, M07, unsigned M08
  preparation, 159 traceability mappings, and documentation all passed.
  Temporary outputs were cleaned; no driver installation/loading,
  signing-mode change, plugin/startup registration, audio stream, or persistent
  machine audio configuration occurred.

- Requalified the locked workspace and guarded M00-M08 chain at pushed head
  `4dc9b67a` on 2026-09-09: workspace tests/doc-tests, strict all-target
  Clippy, formatting, 31-endpoint read-only inventory, disposable pinned
  SysVAD x64 compile/package/API/signability checks, M01/M04/M05, VST3 and
  VST2 fixtures, M07, unsigned M08 preparation, traceability, and documentation
  validation all passed. Temporary outputs/checkouts were cleaned; no driver
  installation/loading, signing-mode change, plugin/startup registration,
  stream, or persistent machine audio configuration occurred.

- Closed the MCP schema-parity regression on 2026-09-09: the CLI test now
  asserts required idempotency-key fields for startup, plugin refresh,
  virtual-device, session-import, recorder, recording, privacy, recovery, and
  session-control tools, while preserving keyless recording preview semantics.
  CLI/control tests (29/98), MCP interoperability tests (2), strict Clippy,
  formatting, and diff checks passed. No audio or machine configuration
  changed.

- Requalified the authorized bounded M00 native live lifecycle on 2026-09-09:
  13 capture endpoints and 18 render endpoints were exercised for 50 ms each;
  one occupied render endpoint was reported and handled as expected. Capture,
  silent render, stop/reset cleanup, and media identity/state comparison passed.
  Defaults, volume, mute, privacy, drivers, signing, and startup configuration
  remained unchanged.
- Next safe task: continue with the remaining independently observable native
  data-path/latency gates while retaining production driver, signing,
  installer, shell/HWND, manual accessibility, and independent-vendor blockers.

- Requalified the authorized event-driven M00 path on 2026-09-09 using the
  existing VB-Audio Cable endpoints: capture produced 4,800 frames and silent
  render submitted 9,600 frames during the 100 ms bounded run. Initialize,
  event setup, start/stop/reset cleanup, and media identity/state comparison
  passed. Defaults, volume, mute, privacy, drivers, signing, and startup
  configuration remained unchanged.
- Next safe task: continue with the remaining virtual-loopback and calibrated
  latency evidence while retaining production driver, signing, installer,
  shell/HWND, manual accessibility, and independent-vendor blockers.

- Requalified the authorized bounded M00 virtual-cable signal path on
  2026-09-09: a 500 ms temporary tone rendered through the selected VB-Audio
  Cable output, and the paired capture collected 2,078 nonzero bytes during a
  250 ms capture. The temporary processes, logs, and endpoint state were
  cleaned up; defaults, volume, mute, privacy, drivers, signing, and startup
  configuration remained unchanged. Calibrated acoustic/physical latency is
  still not established by this result.
- Next safe task: continue with the remaining process-loopback and latency
  evidence while retaining production driver, signing, installer, shell/HWND,
  manual accessibility, and independent-vendor blockers.

- Requalified the authorized bounded process-loopback paths on 2026-09-09:
  controlled attribution captured 3,969 frames and 8,290 nonzero bytes, while
  the explicit child-exclusion mode captured 4,410 frames and reported the
  expected exclude mode. Disposable child processes, probes, and media-state
  checks completed cleanly; no persistent audio configuration changed. The
  exclusion result validates API mode/lifecycle, not a full isolation threshold.
- Next safe task: continue with the remaining physical-latency and native
  adapter evidence while retaining production driver, signing, installer,
  shell/HWND, manual accessibility, and independent-vendor blockers.

- Requalified the bounded M00 impulse path on 2026-09-09. A 20-impulse
  exploratory run correctly failed its 90% detection criterion after 17 groups
  (a useful sensitivity result, not a waived failure). The intended 100-
  impulse run then passed with 97 detected groups, zero p95 spacing error
  frames, and a 69.56 ms estimated onset. Temporary capture/raw/log files and
  endpoint state were cleaned up. This remains signal-correlation evidence;
  calibrated physical acoustic p95 latency is still open.
- Next safe task: continue with the remaining latency/adapter evidence while
  retaining production driver, signing, installer, shell/HWND, accessibility,
  and independent-vendor plugin blockers.

- Requalified the authorized M02 Rust adapter paths on 2026-09-09: the live
  generation-1 gain graph processed 5,760 capture frames at 48 kHz/128-frame
  quanta with zero scheduler deadline misses; the selected routed path
  processed 4,736 routed frames with zero deadline misses. Processing-time and
  deadline histograms were complete and internally consistent, and stream
  cleanup/media-state comparison passed. Defaults, volume, mute, privacy,
  drivers, signing, and startup configuration remained unchanged.
- Next safe task: continue with differing-rate/resampler or physical-latency
  qualification while retaining production driver, signing, installer,
  shell/HWND, manual accessibility, and independent-vendor blockers.

- Rechecked the explicit differing-rate M02 route on 2026-09-09 using the
  available 48 kHz stereo capture and 96 kHz eight-channel render endpoints.
  The adapter rejected the route at its intentional mono/stereo frame boundary
  with `adapter_route_error=audio frame size was invalid`; the route script
  cleaned up successfully. Inventory also shows the only 96 kHz mono capture
  candidate failing initialization with `E_INVALIDARG`. This is reproducible
  endpoint-capability evidence, not a product failure or a passed resampler
  gate; a valid differing-rate mono/stereo pair remains required.
- Next safe task: continue with physical-latency or other independently
  observable gates while retaining the differing-rate, production driver,
  signing, installer, shell/HWND, accessibility, and independent-vendor
  blockers.

- Completed the API reference parity pass on 2026-09-09: every documented
  mutating recorder, startup, plugin-retry, virtual-device, import, graph,
  session-lifecycle, and recording operation now states its idempotency-key
  requirement, with confirmed recycle explicitly distinguished from preview.
  Documentation validation passed with 51 Markdown files and 163 local links.
  The schema regression is pushed at `a6ce827a`; this documentation follow-up
  remains portable and does not change audio or machine configuration.

- Fixed a stale M01 acceptance fixture on 2026-09-09: session start/stop now
  supply the required idempotency keys after API-07 hardening. M01 acceptance
  passes, and the complete guarded M00-M08 chain passes at `e1b2584e`, including
  native toolchain/compile and read-only inventory, disposable SysVAD package
  qualification, portable milestones, VST3/VST2 matrices, M07, unsigned M08,
  traceability, and documentation. Temporary outputs were cleaned; no driver
  installation/loading, signing-mode change, registration, audio stream, or
  persistent machine audio configuration occurred.

- Closed the remaining recorder CLI contract drift on 2026-09-09: recorder
  lifecycle commands now require and forward a non-empty idempotency key at the
  CLI boundary, matching backend discovery and MCP schemas. CLI tests (29), MCP
  interoperability tests (2), strict Clippy, formatting, and diff checks pass;
  no audio stream or machine configuration changed.

- Revalidated M07 headless acceptance at pushed head `32256834` on 2026-09-09:
  CLI (29), control (98), MCP interoperability (2), plugin-host (67), and
  worker-process (13) tests passed with strict Clippy. No audio device, driver,
  plugin registration, startup registration, or machine configuration action
  occurred.

- Corrected stale AUTO-03/AUTO-05 command examples on 2026-09-09: the
  automation specification now uses the implemented positional arguments,
  absolute paths, database options, graph-plan output, and required mutation
  keys for session, graph, route, watch, and export commands. Documentation
  validation passed with 51 Markdown files and 163 local links; no runtime or
  machine configuration changed.

- Rechecked the current pushed automation/documentation head `4fec8c35` on
  2026-09-09: the M07 headless acceptance remains green after the CLI contract
  fixes, and documentation validation passes with 51 Markdown files and 163
  local links. No audio device, driver, registration, or machine configuration
  action occurred.

- Closed the AUTO-07 MCP naming gap on 2026-09-09: added compatibility-focused
  `control_recorder`, `plan_virtual_device_change`, and
  `apply_virtual_device_change` tools, preserving the existing authorized
  backend dispatch and conditional recorder frame validation. MCP discovery now
  exposes 43 tools; CLI/control tests, MCP stdio/pipe interoperability, strict
  Clippy, formatting, and diff checks pass. No audio or machine configuration
  changed.

- Strengthened AUTO-07 coverage on 2026-09-09: the CLI MCP regression now
  dispatches `control_recorder` through an explicit read-plus-record grant and
  verifies the canonical `armed` result, in addition to checking all three
  compatibility-tool schemas. Focused CLI/MCP tests, strict Clippy, formatting,
  and diff checks pass; no audio stream or machine configuration changed.

- Closed the AUTO-07 MCP annotation gap on 2026-09-09: every focused tool now
  publishes machine-readable `readOnlyHint`, `destructiveHint`, and
  `idempotentHint` metadata derived from its actual boundary. Read-only,
  planning, destructive recording/virtual-device, external, and generic
  dispatch cases are covered by regression assertions and real MCP
  interoperability. No audio or machine configuration changed.

- Closed the AUTO-08 MCP resource gap on 2026-09-09: MCP now exposes bounded
  `audiorouter://nodes` and `audiorouter://sessions` read resources backed by
  `nodes.describe` and `sessions.list`, while retaining capabilities,
  diagnostics, and workflow resources. Resource discovery and reads are covered
  through the real stdio/pipe interoperability tests; CLI tests (29), strict
  Clippy, formatting, and diff checks pass. No audio or machine configuration
  changed.

- Reconciled M07 evidence and operator guidance on 2026-09-09: historical
  references no longer claim the obsolete 22-tool/3-resource catalog, and the
  runbook documents node schemas, session snapshots, and MCP side-effect hints.
  Documentation validation passes with 51 Markdown files and 163 local links;
  no runtime or machine configuration changed.

- Revalidated M07 headless acceptance at pushed head `44fdce63` on 2026-09-09:
  CLI (29), control (98), MCP stdio/pipe interoperability (2), plugin-host
  (67), and worker-process (13) tests passed with strict Clippy. No audio
  device, driver, registration, or machine configuration action occurred.

- Added an AUTO-10/SEC-04 generic-dispatch regression on 2026-09-09: a
  read-only MCP client attempting `clients.authorize` through `call_api` is
  denied by the backend with stable `permissionDenied` data and cannot create
  an enrollment. Focused CLI/MCP tests, strict Clippy, formatting, and diff
  checks pass; no authorization or machine state changed.

- Added the companion AUTO-08/SEC-04 resource-boundary regression on
  2026-09-09: an empty-scope MCP client reading `audiorouter://sessions`
  receives the backend's stable `permissionDenied` error instead of a session
  snapshot. Focused CLI/MCP tests, strict Clippy, formatting, and diff checks
  pass; no authorization or machine state changed.

- Requalified the complete guarded M00-M08 acceptance chain at pushed head
  `2aa83044` on 2026-09-09 with authorized native Windows elevation: VS2026 /
  MSVC / SDK / WDK discovery, read-only 31-endpoint inventory, disposable
  pinned SysVAD x64 compile/package/API/signability, M01/M04/M05, VST3 SDK and
  worker paths, modern/legacy/fault VST2 fixtures, M07, unsigned M08
  preparation, traceability, and documentation all passed. Temporary outputs
  were cleaned; no driver installation/loading, signing-mode change,
  plugin/startup registration, audio stream, or persistent machine audio
  configuration occurred.

- Rechecked the authorized standard Windows plugin roots on 2026-09-09 with a
  read-only file inventory: the available candidates remain the six qualified
  ReaPlugs x64 audio effects, Pitchproof x64 plus its rejected x86 sibling, and
  ReaPlugs utility/MIDI/streaming DLLs. No VST3 bundle or independent second
  vendor audio-effect binary is present. No plugin was copied, registered,
  loaded, or modified, and no audio or machine configuration changed.

- Probed the complete installed ReaPlugs directory on 2026-09-09 using the
  bounded VST2 worker wrapper. The six audio-effect DLLs remain qualified, but
  `reacontrolmidi-standalone.dll` stopped the state-restore probe with a
  contained five-second worker timeout; it is therefore not accepted as an
  audio-effect/state-compatible candidate. The wrapper restored
  `AUDIOROUTER_VST2_FIXTURE` and `AUDIOROUTER_VST2_SAMPLE_RATE` to unset after
  the failure. This exposes a qualification gap for generic mixed VST2
  directories: utility/MIDI/streaming binaries must be classified or excluded
  before the audio-effect matrix; no release support is claimed for them.

- Hardened mixed-directory VST2 qualification on 2026-09-09: the acceptance
  wrapper now has an explicit `-SkipIncompatibleCandidates` mode that runs
  each x64 DLL in an isolated worker, reports incompatible utility/MIDI/state
  candidates, and continues qualifying independent audio effects. The installed
  ReaPlugs run qualified seven x64 candidates at 44.1/48/96 kHz (including
  ReaStream) and explicitly rejected ReaControlMIDI and ReaJS; default mode
  still fails fast on candidate regressions. Environment variables were
  restored and no plugin registration or audio configuration changed.

- Closed the API-07 cancellation-key gap on 2026-09-09: the backend schema and
  dispatcher, CLI convenience command, and MCP `cancel_operation` tool now
  require a non-empty idempotency key. Missing keys return JSON-RPC invalid
  params, successful cancellation outcomes are durably journaled and replayed
  by the scoped key, and completed operations remain non-undoable. Focused CLI
  and control tests (129 total) plus all-target/all-features Clippy pass.

- Closed the remaining safety/recovery key-contract gap on 2026-09-09:
  safety.setPrivacyMute and recovery.clearSafeMode now require bounded
  non-empty idempotency keys in the backend schema, CLI, MCP, and transport
  paths. Outcomes are durably journaled and replayable by scoped key; focused
  CLI/control/transport tests (148 total) and formatting pass. Existing
  authorization, privacy, and safe-mode behavior remains unchanged.

- Closed the enrollment key-contract gap on 2026-09-09: clients.authorize and
  clients.revoke now require bounded non-empty idempotency keys in their input
  schemas and dispatchers, and journal/replay outcomes by scoped key. The
  existing control/CLI focused suites pass after updating enrollment fixtures;
  no authorization scope or enrolled-client state was changed by validation.

- Closed the session-lifecycle key-contract gap on 2026-09-09: session start,
  stop, and delete schemas and dispatchers now reject missing or empty
  idempotency keys while preserving scoped durable replay. The focused control
  suite passed all 98 tests with formatting and diff checks; no runtime audio
  session or persistent machine configuration was changed.

- Closed the recorder and MCP session-control key-contract gap on 2026-09-09:
  all recorder input schemas/dispatch paths now require durable idempotency
  keys, and the MCP control_session adapter forwards its required key instead
  of dropping it. Recorder lifecycle/checkpoint regressions, control tests,
  formatting, and diff checks remain the validation target; no audio stream was
  opened.

- Closed the recording-library key-contract gap on 2026-09-09: metadata update,
  rename, and remove-entry schemas, CLI commands, MCP tools, and dispatchers
  now require bounded non-empty idempotency keys and preserve durable replay.
  Read-only get/preview behavior remains keyless; the confirmed recycle path is
  intentionally tracked as a separate platform-dependent mutation slice.

- Closed the confirmed-recycle key boundary on 2026-09-09: recycle previews
  remain read-only without a key, while confirmed recycle requests—including
  already-missing files—require a non-empty idempotency key before the file
  action decision. Backend, CLI, MCP, focused tests, Clippy, formatting, diff,
  and documentation checks passed; no real recording file was touched.

- Revalidated the complete recording-library key boundary on 2026-09-09:
  metadata, rename, remove-entry, and confirmed-recycle paths passed 29 CLI,
  98 control, and 2 MCP tests, strict Clippy, formatting, diff, and
  documentation checks. Preview/get paths remain read-only; no real recording
  file or machine audio configuration was accessed.

- Requalified the default repository VST2 matrix after the mixed-directory
  hardening on 2026-09-09: all six checked-in x64 ReaPlugs audio effects passed
  at 44.1, 48, and 96 kHz, and documentation validation passed for 51 Markdown
  files and 163 local links. The default wrapper remains fail-fast; the
  explicit skip mode is required only for mixed directories containing
  incompatible utility/MIDI candidates.

- Closed an M07/AUTO-01 parity defect on 2026-09-09: the CLI `operation get`
  command no longer sends a null cancellation-only parameter that the shared
  dispatcher rejects, cancellation help documents its optional idempotency
  key, and backend operation errors are preserved in CLI failures. Added a
  SQLite-backed get/cancel regression. This does not claim asynchronous
  cancellation; current operations complete synchronously and cancellation
  correctly reports `alreadyCompleted` without undoing side effects.

- Plan decision recorded: retain the approved M06/PLUG-07 legacy VST2
  extension as an explicit gated workstream. Modern `VSTPluginMain` and
  legacy `main` x64 audio-effect binaries are supported by the contained
  single-stream worker and repository acceptance fixtures; user-installed
  ReaPlugs/Pitchproof remain local qualification inputs and are never copied
  into source control. Rights, independent-vendor coverage, editor success,
  latency/soak, and release qualification remain prerequisites.

- Closed the companion M07/AUTO-06 MCP discovery gap on 2026-09-09: the
  `cancel_operation` tool now advertises its optional idempotency key with the
  same bounded string contract as the backend and CLI. Added schema regression
  coverage; MCP stdio interoperability and the full headless suite remain
  green.

- Requalified the guarded M00-M08 `safe-all.ps1` chain at pushed head
  `a28829af` on 2026-09-09: VS2026/WDK discovery and native compile, read-only
  31-endpoint inventory, disposable pinned SysVAD x64 package/API validation,
  M01/M04/M05, pinned VST3 SDK/native worker and auxiliary-bus checks, modern/
  legacy/fault VST2 fixtures, M07, unsigned M08 artifacts, 159 normative
  mappings, and documentation validation (51 Markdown files/163 links) passed.
  Temporary outputs were cleaned by the guarded harness. No driver
  installation/loading, signing-mode change, plugin/startup registration,
  audio stream, or persistent machine audio configuration occurred.

- Requalified the full locked workspace at pushed head `740d2305` on
  2026-09-09: 466 unit/integration tests and all workspace doc-tests passed;
  all-target all-features Clippy, formatting, and diff checks also passed.
  This confirms the CLI/MCP parity changes did not regress other adapters or
  realtime safety boundaries.

- Added M06/PLUG-04 validated state restoration across deliberate worker
  replacement on 2026-09-09: `restart_with_state` preserves the verified
  plugin path, restores a version/size/hash-checked opaque asset before
  returning the replacement, and returns its supervisor if restoration fails.
  The fixture regression restored exact bytes after replacement; all-features
  plugin-host tests passed 67 library and 34 worker cases with nine expected
  skips, strict Clippy, and documentation validation passed. Native third-party
  state/editor, callback-timing, soak, and physical-latency gates remain open.

- Hardened that M06/PLUG-04 boundary on 2026-09-09: `restart_with_state`
  validates the opaque asset before spawning a replacement, and returns the
  existing failed supervisor unchanged when the version, size, or integrity
  check rejects it. The focused feature-enabled regression passed for both
  valid restoration and invalid-version fail-closed behavior; formatting,
  strict plugin-host Clippy, and diff checks passed.

- Extended M06/PLUG-07 state coverage on 2026-09-09: both repository VST2
  fixtures (modern `VSTPluginMain` and legacy `main`) now restore their chunk
  state across deliberate supervised worker replacement, in addition to live
  restore. The focused fixture acceptance remains Windows-only and
  single-stream; rights, editor, latency, and release qualification remain
  gated.

- Extended native M06/PLUG-04 evidence on 2026-09-09: the real AGain VST3
  single-stream acceptance now saves opaque plugin state, deliberately fails
  the worker, restores that state during verified replacement, and confirms
  finite transformed output without re-sending automation. Native editor,
  callback-timing, soak, physical-latency, and independent-plugin gates remain
  open.

- Fixed the native M06/PLUG-04 state boundary on 2026-09-09 after acceptance
  exposed an EOF: the Windows worker now handles bounded `StateSave` and
  `StateRestore` messages with a local `IBStream`, CNG SHA-256 metadata, and
  size limits. The real AGain replacement-state acceptance passed, along with
  native single-/multi-bus, asynchronous, failure, timing, Clippy, and docs
  checks. No plugin registration, audio stream, or machine configuration was
  used.

- Extended the native state asset on 2026-09-09 to preserve both VST3 component
  and edit-controller streams in a bounded worker-owned envelope, while
  retaining component-only restore compatibility. AGain native acceptance,
  asynchronous/multi-bus coverage, strict Clippy, formatting, and docs checks
  passed; no audio device or machine configuration was accessed.

- Requalified the guarded M00-M08 `safe-all.ps1` chain at pushed head
  `38141ef1` on 2026-09-09: VS2026/MSVC/SDK/WDK discovery and native compile,
  read-only endpoint inventory, disposable pinned SysVAD x64 qualification,
  portable/UI suites, native VST3 state/multi-bus/async acceptance, modern/
  legacy/fault VST2 fixtures, M07, unsigned M08 artifacts, 159 traceability
  mappings, and documentation validation (51 Markdown files/163 local links)
  passed. Temporary checkouts/artifacts were cleaned; no driver installation or
  loading, signing-mode change, plugin/startup registration, audio stream, or
  persistent machine audio configuration occurred. Production callback,
  signing, installer, clean-machine, physical-latency, manual UI, and
  rights-cleared independent-plugin gates remain open.

- Qualified the installed x64 Pitchproof VST2 binary on 2026-09-09 at 44.1,
  48, and 96 kHz through the isolated worker; its sibling `pitchproof.dll`
  was rejected as x86 before loading. Hardened the VST2 acceptance wrapper to
  classify mixed directories and report/skip unsupported PE architectures while
  retaining failure for supported x64 regressions. Rights, editor, and release
  gates remain open.

- Requalified installed Pitchproof x64 VST2 containment on 2026-09-09 with
  SHA-256 `1974a3033b53ae72da5f419a9f37056d44c1610591bfd11a615ace0c448cf050`:
  processing passed at 44.1, 48, and 96 kHz, while both editor-thread tests
  bounded the non-returning editor at five seconds and reaped the worker.
  This is contained editor-failure evidence, not native editor compatibility;
  the wrapper restored its environment and made no registration or audio
  configuration change.

- Extended the general M06/PLUG-07 VST2 matrix on 2026-09-09: every
  state-capable candidate now performs a deliberate fail, supervised
  replacement, opaque-state restore, and finite-output check; state-unsupported
  candidates retain the clean shutdown path. ReaPlugs and installed x64
  Pitchproof acceptance passed with environment restoration.

- Hardened the M06 VST2 mixed-directory classifier on 2026-09-09 to read only
  the bounded DOS/PE headers instead of allocating the entire candidate DLL.
  The mixed Pitchproof directory still reports the x86 sibling and qualifies
  the x64 binary at all three rates; the six-ReaPlugs matrix also passed.

- Requalified the guarded M00-M08 `safe-all.ps1` chain at pushed head
  `8335f5a7` on 2026-09-09: VS2026/MSVC/SDK/WDK discovery and native compile,
  read-only endpoint inventory, disposable pinned SysVAD x64 qualification,
  M01/M04/M05, native VST3 including validated state restoration, modern/
  legacy/fault VST2 fixtures, M07, unsigned M08 artifacts, 159 traceability
  mappings, and documentation validation (51 Markdown files/163 local links)
  passed. Temporary checkouts/artifacts were cleaned; no driver installation or
  loading, signing-mode change, plugin/startup registration, audio stream, or
  persistent machine audio configuration occurred. Production callback,
  signing, installer, clean-machine, physical-latency, manual UI, and
  rights-cleared independent-plugin gates remain open.

- Requalified the complete guarded M00-M08 chain from clean pushed head
  `8026dd1e` on 2026-09-09 after fixture-preserving quarantine coverage.
  VS2026/MSVC/SDK/WDK checks, read-only 31-endpoint inventory, disposable
  SysVAD x64 qualification, M01/M04/M05/M06/M07, native VST3 and local VST2
  acceptance, unsigned M08 artifacts, 159 traceability mappings, and
  documentation validation (51 Markdown files/163 local links) passed.
  Temporary outputs/checkouts were cleaned; no driver installation/loading,
  signing-mode change, plugin/startup registration, audio stream, or
  persistent machine audio configuration occurred.

- Requalified the full guarded M00-M08 chain from clean pushed head
  `601f576b` on 2026-09-09 after bounded owner restart-policy integration.
  VS2026/MSVC/SDK/WDK checks, read-only 31-endpoint inventory, disposable
  SysVAD x64 qualification, M01/M04/M05/M06/M07, native VST3 and available
  VST2 fixtures, unsigned M08 artifacts, 159 traceability mappings, and
  documentation validation (51 Markdown files/163 local links) passed.
  Temporary outputs were cleaned; no driver installation/loading,
  signing-mode change, plugin/startup registration, audio stream, or
  persistent machine audio configuration occurred.

- Requalified the full guarded M00-M08 chain from clean pushed head
  `88481216` on 2026-09-09 after native restart-path coverage: VS2026/MSVC/
  SDK/WDK discovery and compile, read-only 31-endpoint inventory, disposable
  SysVAD x64 qualification, M01/M04/M05/M06/M07, native VST3 single-/multi-bus
  processing, available VST2 fixtures, unsigned M08 artifacts, 159
  traceability mappings, and documentation validation (51 Markdown files/163
  local links) passed. Temporary outputs were cleaned; no driver installation
  or loading, signing-mode change, plugin/startup registration, audio stream,
  or persistent machine audio configuration occurred.

- Hardened the M06/PLUG-03 graph staging boundary on 2026-09-09: runtime
  multi-bus processing now rejects mixed quantum frame counts across any
  declared input or output bus before mutating destinations. The focused
  all-features engine suite passed 85 tests, strict engine Clippy and diff
  checks passed, and no plugin, driver, stream, or machine audio configuration
  was used.
- Promoted the M06/PLUG-03 negotiated multi-bus process owner to normal builds
  on 2026-09-09: `WorkerProcess::spawn_multi_bus`, supervised restart, and
  bounded `process_buses` are no longer test-fixture-only APIs. Default and
  all-features plugin-host tests, strict Clippy, formatting, and diff checks
  passed. The shipped worker's multi-bus implementation is still an echo
  protocol owner, not native VST3 execution; VST2 remains single-stream.
- Requalified the locked all-features workspace after the normal-build owner
  promotion on 2026-09-09: all crate tests and doc-tests passed, including 28
  CLI, 98 control, 58 domain, 28 DSP, 85 engine, 66 plugin-host, 29 passing
  plus six skipped worker-process, 6 protocol, 30 recording, 80 storage, 19
  transport, and 33 Windows-audio tests. No driver, plugin registration,
  audio stream, or machine audio configuration was changed.
- Added an explicit M06/PLUG-07 boundary on 2026-09-09: the supervised
  multi-bus owner rejects VST2 identities before process launch, ensuring the
  approved legacy adapter cannot be routed through auxiliary-bus protocol
  messages. Plugin-host tests passed 67/67 in default and all-features library
  runs, worker-process coverage remained green, and strict Clippy passed.
- Added a second M06/PLUG-07 guard on 2026-09-09: supervised multi-bus launch
  now rejects VST2 identities with an explicit single-stream diagnostic before
  executable launch. This prevents accidental legacy-format bus flattening;
  focused plugin-host tests passed 67/67 in both feature modes.
- Extended M06/PLUG-03 parameter plumbing on 2026-09-09: negotiated multi-bus
  quanta now accept the existing bounded per-frame parameter events after
  validation instead of rejecting every non-empty list. Plugin-host library
  tests passed 67/67 in default and all-features modes; worker-process tests
  passed 13 default and 29 all-features cases with six expected skips. This is
  protocol evidence only; native VST3 parameter application remains gated.
- Requalified the locked all-features workspace after multi-bus parameter
  plumbing on 2026-09-09: all crate tests and doc-tests passed, including 67
  plugin-host library tests, 29 passing plus six skipped worker-process tests,
  85 engine, 98 control, 58 domain, 28 DSP, 30 recording, 80 storage, 19
  transport, 33 Windows-audio, 28 CLI, and 2 MCP interoperability tests. No
  driver, plugin registration, audio stream, or machine audio configuration
  was changed.
- Extended the native M06 VST3 offline probe on 2026-09-09: it now constructs
  `IParameterChanges` and sends a bounded normalized event to the first
  controller parameter before `IAudioProcessor::process`. The VS2026/Windows
  SDK build passed; mda, AGain main, and AGain's genuine two-input/one-output
  side-chain class all produced finite output, with no audio device or machine
  configuration access. This is native offline parameter-event evidence, not
  supervised realtime VST3 execution.
- Requalified the complete guarded M06 VST3 acceptance after native parameter
  event support on 2026-09-09: pinned SDK build, 51 validator self-tests,
  mda validator, AGain validator, AGain main and auxiliary-bus probes, explicit
  single-bus rejection, and the five-class mda matrix all passed. The wrapper
  removed generated loader artifacts; no plugin registration, audio device, or
  machine configuration change occurred.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain after
  native VST3 parameter delivery on 2026-09-09. VS2026/MSVC/SDK/WDK discovery
  and native compile, read-only 31-endpoint inventory, disposable pinned SysVAD
  x64 qualification, M01/M04/M05, VST3 validators and parameter-event probes,
  VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159
  traceability mappings, and documentation validation (51 Markdown files/163
  local links) passed. Temporary outputs were cleaned; no driver installation
  or loading, signing-mode change, plugin/startup registration, audio stream,
  or persistent machine audio configuration occurred.
- Strengthened the native VST3 transformation evidence on 2026-09-09: the
  offline loader now supports an explicit `--require-output-change` assertion
  with a bounded parameter value, and AGain's main class passed with its gain
  parameter set to normalized 0.75 while processing finite output different
  from the 0.25 probe input. The generated executable/object were removed
  after the native test; no audio device or machine configuration was touched.
- Corrected VST2 automation timing on 2026-09-09: the Windows worker now sorts
  bounded parameter events and processes separate audio segments at each
  sample offset, rather than silently applying every event at block start. The
  opt-in ReaPlugs matrix now exercises offsets 0 and 64 in a 128-frame stereo
  block across all six local fixtures and three sample rates; finite processing
  passed, with environment variables restored and no audio configuration change.
- Implemented the first production-shaped native VST3 worker on 2026-09-09:
  `tools/m06-vst3-worker` loads a verified x64 bundle in a separate Windows
  process, speaks the existing length-prefixed JSON `Hello`/`Ready` and
  `Process`/`Processed` protocol, applies parameter events at bounded sample
  offsets (including 0 and 64), and rejects non-single-bus layouts. The supervised Rust launch now
  forwards the VST3 plugin path; an end-to-end AGain regression passed with
  finite transformed output and clean shutdown. This is single-stream worker
  evidence, not realtime graph scheduling, auxiliary-bus execution, or
  physical-latency evidence.
- Extended the native VST3 worker to the bounded auxiliary-bus contract on
  2026-09-09: it selects an effect matching the requested bus counts,
  activates each declared mono/stereo bus, processes `HelloBuses`/
  `ProcessBuses`, and preserves coherent per-bus sequence/deadline identity.
  AGain's genuine `[stereo, mono]` input to stereo output side-chain class
  passed through the supervised Rust owner with finite output and clean
  shutdown. This is native worker-process evidence; realtime graph scheduling,
  soak/physical latency, and independent rights-cleared plugin gates remain open.
- Strengthened the native auxiliary-bus regression on 2026-09-09: AGain's
  supervised `[stereo, mono]` path must produce finite output that differs from
  the main input, proving a real native transformation rather than a worker
  echo. The focused acceptance passed and restored its fixture environment.
- Connected the native auxiliary-bus acceptance to the existing graph-owned
  handoff on 2026-09-09: the supervised AGain result is copied into
  caller-prepared engine storage and accepted by `RuntimeBusGeneration` with
  its sequence/deadline identity intact. The focused native worker and
  all-features plugin-host checks passed. This remains worker-thread staging;
  realtime callback scheduling and physical latency are not claimed.
- Added the M06/PLUG-03 `RuntimeBusScheduler` staging primitive on 2026-09-09:
  preallocated input/output quantum slots transfer complete mono/stereo bus
  sets without waiting or callback allocation, reject queue pressure before
  partial publication, and silence missing or stale output quanta. Engine
  tests (86), strict engine Clippy, formatting, and diff checks passed. This
  is bounded graph staging; a background native worker loop, callback timing,
  and physical-latency evidence remain open.
- Added `SupervisedBusWorkerLoop` on 2026-09-09: a dedicated owner thread
  drains complete preallocated bus quanta, performs worker IPC and conversion
  off the callback, and publishes validated output slots back to the graph.
  The fixture-backed loop regression passed alongside 67 plugin-host unit
  tests, 30 worker-process tests, strict Clippy, formatting, and diff checks.
  Worker-thread staging is proven; native callback timing, soak, and physical
  latency remain unqualified.
- Extended the native VST3 acceptance through `SupervisedBusWorkerLoop` on
  2026-09-09: the real AGain `[stereo, mono]` side-chain worker now receives
  graph-scheduled blocks and bounded normalized gain automation off the
  callback, then publishes finite transformed output back to the scheduler.
  The opt-in Windows acceptance passed with clean shutdown and restored its
  environment. Native callback timing, long-run soak, and physical latency
  remain unqualified.
- Added bounded asynchronous failure recovery on 2026-09-09: a supervised
  hanging multi-bus worker is reaped at its quantum deadline, latches owner
  failure, and leaves the callback-facing scheduler with silence instead of a
  stale result. The focused regression passed in 120 ms; restart policy,
  callback timing, long-run soak, and physical latency remain open.
- Verified native VST3 restart identity on 2026-09-09: the supervised
  single-stream worker now preserves the exact scanned plugin path when a
  failed worker is deliberately replaced, and the restarted AGain worker
  again produced finite transformed output. The focused native M06 acceptance,
  67 all-features plugin-host library tests, 31 worker-process tests, strict
  Clippy, formatting, and diff checks passed. Automatic restart policy,
  quarantine integration, callback timing, soak, and physical latency remain
  open.
- Added an explicit bounded restart policy for the asynchronous multi-bus
  owner on 2026-09-09: callers may opt into at most two owner-thread worker
  replacements, while the default remains zero automatic retries. Each retry
  uses `SupervisedWorkerProcess::restart`, so the existing failure ledger and
  quarantine state remain authoritative; terminal failures still leave the
  graph-facing scheduler fail-closed. Plugin-host all-features tests (67
  library/32 worker-process passing plus nine expected skips), strict Clippy,
  formatting, and native M06 acceptance passed. Restart/quarantine soak,
  callback timing, and physical latency remain open.
- Verified repeated owner failure and quarantine on 2026-09-09: the controlled
  hanging multi-bus fixture survived two bounded replacements, then reached
  the existing three-failure quarantine threshold; the owner exposed terminal
  failure and the graph-facing result remained silence. The focused native M06
  acceptance passed after this regression, including native single-stream
  recovery. Automatic policy is now bounded and fail-closed; native fault soak,
  callback timing, and physical latency remain open.
- Added a bounded repeated-quantum owner regression on 2026-09-09: 16,384
  consecutive `[stereo, mono]` fixture quanta crossed the preallocated graph
  scheduler and returned finite output without worker failure; the measured
  per-quantum staging/worker round trip stayed below 100 ms in the guarded
  run. This is a short deterministic worker-thread bound, not realtime
  callback, eight-hour soak, or physical-latency evidence. This advances the
  short-run M06 worker-soak evidence without claiming the NFR-11 W2 gate.
- Requalified the legacy x64 VST2 extension on 2026-09-09 with the supplied
  local ReaPlugs directory: all six audio effects passed isolated worker
  load/process checks at 44.1, 48, and 96 kHz (18 runs), including the existing
  offset-automation coverage. The wrapper restored both VST2 environment
  variables; no plugin registration, audio stream, or machine configuration
  changed. Rights, editor, and release qualification remain gated.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain from
  clean pushed head `f9636235` on 2026-09-09 after the scheduler change. The
  installed VS2026/WDK native checks, disposable SysVAD qualification, M01,
  M04, M05, VST3 SDK/native single-/multi-bus worker probes, VST2
  modern/legacy/state/fault fixtures, M07, unsigned M08 preparation, 159
  traceability mappings, and documentation validation (51 Markdown files/163
  local links) passed. Temporary outputs were removed; no driver installation
  or loading, signing-mode change, registration, audio stream, or persistent
  machine audio configuration occurred.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain after
  the native VST3 worker integration on 2026-09-09. The chain passed native
  toolchain/endpoint checks, disposable pinned SysVAD x64 qualification, M01,
  M04, M05, pinned VST3 validator and native worker probes, VST2
  modern/legacy/state/fault fixtures, M07, unsigned M08 preparation, 159
  traceability mappings, and documentation validation (51 Markdown files/163
  local links). Temporary outputs were cleaned; no driver installation/loading,
  plugin registration, audio stream, signing-mode change, or persistent
  machine audio configuration occurred.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain from
  clean pushed commit `eb0ad978` on 2026-09-09 after native auxiliary-bus
  worker integration. Native toolchain/endpoint checks, disposable pinned
  SysVAD qualification, M01/M04/M05, VST3 SDK and native single-/multi-bus
  worker probes, VST2 modern/legacy/state/fault fixtures, M07, unsigned M08
  preparation, 159 traceability mappings, and documentation validation (51
  Markdown files/163 local links) passed. Temporary outputs were cleaned; no
  driver installation/loading, signing-mode change, plugin/startup
  registration, audio stream, or persistent machine audio configuration
  occurred. Production realtime scheduling, physical-latency, signing,
  installer, and rights/editor gates remain open.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain after
  the VST2 automation fix on 2026-09-09. VS2026/MSVC/SDK/WDK discovery and
  native compile, read-only 31-endpoint inventory, disposable pinned SysVAD
  x64 qualification, M01/M04/M05, VST3 validators and transformation probes,
  VST2 modern/legacy/state/fault fixtures, M07, unsigned M08 artifacts, 159
  traceability mappings, and documentation validation (51 Markdown files/163
  local links) passed. Temporary outputs were cleaned; no driver installation
  or loading, signing-mode change, plugin/startup registration, audio stream,
  or persistent machine audio configuration occurred.
- Hardened the M06/PLUG-03 typed multi-bus worker deadline on 2026-09-09: response reads now stop at the quantum deadline (still capped by the five-second IPC bound), and a controlled no-result worker returned within 100 ms instead of waiting for the global timeout. The supervised expired-quantum path remains fail-closed and records the worker failure. Feature-enabled worker-process tests passed 26 tests with six expected fixture-dependent skips; strict Clippy, formatting, and diff checks passed. No production VST3 worker, realtime callback, plugin registration, audio stream, or machine audio configuration was used.
- Exercised the M06/PLUG-03 production-shaped handoff on 2026-09-09: a supervised multi-bus result now passes through the validated worker client, caller-owned staging storage, and `RuntimeBusGeneration`, preserving sequence identity and publishing both main and auxiliary output blocks. The feature-enabled worker-process suite passed 27 tests with six expected fixture-dependent skips; strict Clippy, formatting, and diff checks passed. This is still echo-fixture integration evidence, not production VST3 execution or realtime callback evidence.
- Extended the M06/PLUG-03 worker fixture on 2026-09-09 to accept bounded asymmetric layouts such as two input buses to one output bus, preserving only the declared main output and rejecting layouts that would require synthesizing an undeclared input. The feature-enabled worker-process suite passed 28 tests with six expected fixture-dependent skips; strict feature-enabled and default workspace Clippy, formatting, and diff checks passed. This remains protocol/fixture evidence; production VST3 execution and realtime graph scheduling remain gated.
- Added the M06/PLUG-03 undeclared-output regression on 2026-09-09: a multi-bus worker requesting more output buses than available input buses is rejected during startup rather than synthesizing audio. The feature-enabled worker-process suite passed 29 tests with six expected fixture-dependent skips; strict all-features Clippy, formatting, diff checks, and documentation validation passed. Production VST3 execution and realtime graph scheduling remain gated.
- Revalidated the M06/PLUG-03 asymmetric-output boundary on 2026-09-09 after correcting the fixture hash setup: the worker-process suite passed 29 tests with six expected fixture-dependent skips, and the startup rejection is now covered by the passing regression. No production VST3 execution, realtime callback, plugin registration, or machine audio configuration was used.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain on 2026-09-09 after the multi-bus safety regressions: VS2026/WDK discovery and native compile, read-only 31-endpoint inventory, disposable pinned SysVAD x64 package/API/signability qualification, M01/M04/M05, VST3 validators and AGain auxiliary-bus probe, VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159 mappings, and documentation validation (51 Markdown files/163 local links) passed. No driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine audio configuration occurred.
- Requalified the same guarded M00–M08 chain at the current multi-bus safety tip on 2026-09-09: all native, portable, UI, VST3, VST2, release-preparation, traceability, and documentation checks passed, including the repository's 29-test worker-process fixture suite in the all-features workspace. This remains qualification evidence only; production VST3 worker execution, managed driver callback, signing, and physical-latency gates remain open.
- Added explicit M06/PLUG-03 worker pipeline-latency accounting on 2026-09-09: `WorkerLatency::total_samples_with_pipeline` combines validated plugin latency with caller-owned scheduling delay, rejects overflow and the ten-second bound, and avoids guessing physical/device latency. Plugin-host unit tests (66), feature-enabled worker-process tests (29 plus six expected skips), both workspace Clippy modes, formatting, and diff checks passed.
- Requalified the locked all-features workspace after the latency-accounting change on 2026-09-09: all crate unit/integration tests and doc-tests passed, including engine (84), plugin-host (66), worker-process (29 plus six expected skips), control (98), domain (58), DSP (28), recording (30), storage (80), transport (19), Windows audio (33), CLI (28), and MCP interoperability (2). Formatting, strict all-features Clippy, and diff checks passed; no audio, driver, registration, signing, or machine configuration action occurred.
- Corrected the current M06 compatibility documentation on 2026-09-09: AGain's two-input/one-output side-chain class is now described as passing the explicit offline `--multi-bus` probe, while the default single-bus probe and supervised realtime multi-bus worker remain gated. Documentation validation (51 Markdown files/163 local links) passed.
- Requalified the available native VST2 fixtures on 2026-09-09 with `m06-vst2-reaplugs.ps1`: all six local ReaPlugs effects passed isolated worker load/process checks at 44.1, 48, and 96 kHz (18 combinations). The wrapper restored both `AUDIOROUTER_VST2_FIXTURE` and `AUDIOROUTER_VST2_SAMPLE_RATE`; no plugin registration, audio stream, or machine configuration changed. Rights/editor/release qualification remains gated.
- Next M06/PLUG-04/PLUG-06 task: qualify state save/restore on a supplied
  rights-cleared independent plugin and retain native editor, callback-timing,
  physical-latency, and release gates; keep VST2 single-stream and fail-closed
  boundaries in force.
- Requalified the guarded `tests/acceptance/safe-all.ps1` chain on 2026-09-09 at the supervised multi-bus handoff checkpoint: VS2026/WDK discovery and native compile, read-only 31-endpoint inventory, disposable pinned SysVAD x64 package/API/signability qualification, M01/M04/M05, VST3 validators and AGain auxiliary-bus probe, VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159 mappings, and documentation validation (51 Markdown files/163 local links) passed. The follow-up strict all-features and no-feature workspace Clippy checks also passed after removing test-only build warnings. No driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine audio configuration occurred.

- Requalified the complete guarded M00–M08 `safe-all.ps1` chain at pushed head `4475a5d0` on 2026-09-08: VS/WDK discovery/native compile, read-only 31-endpoint inventory, disposable SysVAD x64 package/API qualification, portable milestone checks, VST3 SDK/validator, modern/legacy/fault VST2 fixtures, M07, unsigned M08 artifacts, 159 traceability mappings, and documentation validation (51 files/161 links) passed. Temporary outputs/checkouts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred. Production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M02/M03/M06 task: retain this validated user-mode boundary while awaiting the production managed-driver callback, independent plugin fixture/rights evidence, physical latency setup, and authenticated native-shell HWND owner.
- Hardened the UI audio-inventory adapter on 2026-09-09: device and application discovery failures now preserve the backend's structured category, unsigned HRESULT, remediation, and retryability in the visible error message. Added a focused `deviceInUse` formatting regression covering HRESULT `0x8889000A`; contracts typecheck, UI typecheck, and all 95 UI tests passed. No audio or machine configuration changed.
- Next M01/API-09 task: carry the same structured error presentation to any future native-shell status surface without weakening the disconnected preview behavior.
- Extended the M05 status refresh path on 2026-09-09: `SnapshotCache` now preserves structured RPC audio category, HRESULT, remediation, and retryability when a connected snapshot fails, with an `accessDenied` regression covering `0x80070005`. UI typecheck and all 96 UI tests passed; disconnected preview behavior remains unchanged and no machine configuration changed.
- Next M05/M00 task: retain structured status diagnostics while integrating the authenticated native-shell transport and managed endpoint callback; those native gates remain externally blocked.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at pushed head `d71c3152` on 2026-09-09: VS2026/WDK discovery and native compile, read-only 31-endpoint format inventory, disposable pinned SysVAD x64 package/API/signability qualification, M01/M04/M05, pinned VST3 SDK/validator, repository VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159 normative mappings, and documentation validation (51 Markdown files/162 local links) passed. The run left 23 direct repository-named temporary SQLite fixtures; exact validated cleanup removed all 23 and a follow-up scan found zero. No driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine audio configuration occurred.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture or integrate the authenticated native-shell HWND owner when available; retain the gated VST2 rights/editor/release boundary.

- Hardened the native VST3 worker's PLUG-04 editor boundary on 2026-09-09: `DescribeEditor` now returns a bounded no-editor descriptor, while `EditorOpen` and `EditorClose` return the specified `editorUnavailable` failure without falling through to audio parsing or terminating the worker. The native VST3 acceptance now requests the descriptor before processing and confirms processing still succeeds; native build and acceptance passed. No authenticated desktop-shell HWND exists yet, so this is fail-closed compatibility behavior rather than editor qualification. No plugin registration, audio stream, or machine configuration changed.
- Next M06 task: obtain a rights-cleared independent x64 VST2/VST3 fixture or integrate a real authenticated native-shell HWND owner before attempting editor support; retain the fail-closed VST3/VST2 editor behavior and gated VST2 release boundary.

- Completed the adjacent M06 generic-control boundary on 2026-09-09: the native VST3 worker now answers `DescribeParameters` from the controller with a bounded normalized descriptor list (finite defaults, safe titles, and a 64-parameter cap). Native acceptance verifies descriptors, unsupported editor responses, and continued processing in one worker lifecycle. Native build and the full VST3 worker acceptance passed; no plugin registration, audio stream, or machine configuration changed.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture or integrate a real authenticated native-shell HWND owner before attempting native editor support; retain the bounded generic-parameter and fail-closed editor contracts.

- Synchronized the M06 compatibility documentation on 2026-09-09 after the native worker changes: the operations guide and worker README now document native VST3 controller descriptors, the 64-entry worker cap, generic controls when no editor is available, and explicit `editorUnavailable` behavior. Documentation validation passed with 51 Markdown files and 163 local links; no machine configuration changed.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture or integrate a real authenticated native-shell HWND owner before attempting native editor support; retain the documented bounded generic-parameter and fail-closed editor contracts.
- Improved M06 VST3 lifecycle compatibility on 2026-09-09: the offline loader now accepts only `kNotImplemented` from `setProcessing`, matching the pinned SDK's base implementation while retaining fatal handling for every other result. Built-in SDK `AGain` passed the official validator (94/94) and AudioRouter's main stereo class probe; its side-chain class was correctly excluded by the one-input/one-output fixture boundary. The existing mda validator and five-class loader matrix also passed. No system plugin link, registration, audio stream, or machine configuration changed.
- Next M06 task: qualify a rights-cleared independent non-SDK x64 VST3/VST2 fixture and extend bus-layout coverage before treating vendor/editor/latency compatibility as release evidence.
- Read-only M06 fixture inventory on 2026-09-09 found no independent VST3 bundle under the checked machine roots (`C:\Program Files\Common Files\VST3`, `C:\Program Files\VST3`, and `C:\Program Files\VSTPlugins`); only the already-qualified Pitchproof VST2 binary and ReaPlugs VST2 DLLs were present. No plugin was loaded, registered, copied, or executed, and audio configuration was unchanged.
- Next M06 task: qualify the first user-supplied rights-cleared independent x64 VST3 fixture when available; until then retain AGain plus mda as repository-local SDK evidence and keep vendor/editor/latency/release claims gated.
- Extended M06 bus-layout regression coverage on 2026-09-09: the acceptance script now requires AGain's side-chain class to fail with the specific one-input/one-output boundary diagnostic, while its main stereo class must pass. This prevents a future compatibility claim from silently ignoring multi-bus effects. The test remains offline and repository-local; no plugin registration, audio stream, or machine configuration changed.
- Next M06 task: implement and validate a bounded multi-bus/side-chain host contract only when the graph and worker schemas define its ownership; retain the explicit rejection until then.
- Corrected the M06 expected-failure harness on 2026-09-09: native loader stderr and exit status are captured through an explicit bounded process object on the installed Windows PowerShell/.NET runtime, avoiding a terminating error for the intentional side-chain rejection. The full M06 VST3 acceptance then passed with exit code 0, including both validators, AGain main-class processing, the explicit side-chain rejection, and the mda matrix. No machine configuration changed.
- Next M06 task: define the bounded multi-bus worker/graph contract before expanding side-chain support; retain the passing rejection regression as the current safety boundary.
- Implemented the M06/PLUG-03 bounded multi-bus control-plane contract on 2026-09-09: `WorkerAudioBusLayout` requires main input/output buses, caps each direction at four mono/stereo buses and eight aggregate channels, and explicitly leaves the single-stream worker wire path unchanged until graph/shared-memory ownership exists. Plugin-host tests (60), worker-process tests (21), doc-tests, formatting, and strict Clippy passed. No plugin registration, audio stream, driver action, or machine configuration changed.
- Next M06/PLUG-03 task: extend the worker protocol and graph ownership for auxiliary buses with fixed shared-memory slots and fail-closed late-frame policy; do not promote the control-plane descriptor to runtime side-chain support yet.
- Added the M06/PLUG-03 bounded bus-frame foundation on 2026-09-09: `WorkerAudioBusFrames` validates exact bus cardinality, expected mono/stereo channels, equal quantum sizes, and shared sequence/deadline identity for each declared input or output side. Plugin-host tests (61), worker-process tests (21), doc-tests, formatting, and strict Clippy passed. The existing single-stream wire/runtime path remains unchanged and auxiliary layouts remain rejected until graph and shared-memory ownership are implemented.
- Next M06/PLUG-03 task: add serialized multi-bus worker messages and fixed shared-memory slot ownership, including bounded late/missing-bus failure behavior; preserve the current single-stream compatibility path.
- Added serialized M06/PLUG-03 `ProcessBuses`/`ProcessedBuses` message shapes on 2026-09-09. Encode/decode validation rechecks the bounded layout and all bus frames, including channel, quantum-size, sequence, deadline, finite-sample, and parameter-offset invariants. A round-trip regression covers a main-plus-side-chain request/response and a malformed identity rejection. Plugin-host tests (62), worker-process tests (21), doc-tests, formatting, and strict Clippy passed. The existing session/shared-memory runtime remains single-stream and side-chain execution is not claimed.
- Next M06/PLUG-03 task: add a separate multi-bus session handshake and fixed slot ownership, with bounded late/missing-bus failure handling; retain backward compatibility for existing single-stream workers.
- Added the M06/PLUG-03 multi-bus session handshake on 2026-09-09: `WorkerBusSession` binds plugin fingerprint and exact layout through `HelloBuses`/`Ready`, and accepts only coherent `ProcessBuses` input quanta whose buses share identity and deadline. Existing single-stream sessions remain unchanged. Plugin-host tests (63), worker-process tests (21), doc-tests, formatting, and strict Clippy passed. No plugin registration, audio stream, driver action, or machine configuration changed.
- Next M06/PLUG-03 task: implement fixed shared-memory slot ownership and bounded late/missing-bus outcomes, then connect those messages to graph scheduling; retain the backward-compatible single-stream path.
- Implemented the M06/PLUG-03 fixed shared-memory bus-slot transport on 2026-09-09: `SharedAudioBusTransport` uses one caller-owned slot per declared bus, rejects aliases/reparse paths and invalid cardinality, reports missing slots before promotion, and revalidates sequence/deadline/quantum coherence across the set. The transport regression round-trips main-plus-side-chain input/output sets. Plugin-host tests (64), worker-process tests (21), doc-tests, formatting, and strict Clippy passed. It remains a transport foundation and is not wired into realtime scheduling.
- Next M06/PLUG-03 task: connect multi-bus transport to a graph-owned processing generation with bounded late/missing-bus silence policy; preserve the single-stream worker compatibility path.
- Added the M06/PLUG-03 graph-owned staging boundary on 2026-09-09: `RuntimeBusLayout` and `RuntimeBusGeneration` in the engine bind bounded mono/stereo bus shapes to a nonzero generation, validate caller-owned blocks without realtime allocation, pass through only the main bus, and silence every output when the required main input is missing. Engine tests (83), formatting, and strict Clippy passed. Auxiliary effect execution remains intentionally open; the existing single-stream worker compatibility path is unchanged and no audio or machine configuration changed.
- Next M06/PLUG-03 task: replace the staging pass-through with an explicitly owned worker result handoff that carries sequence/deadline identity into the graph generation, while retaining fail-closed protected-path silence and the single-stream compatibility path.
- Hardened M06/PLUG-03 shared bus opening on 2026-09-09: existing multi-bus slots are now checked pairwise by canonical path and native file identity, so hard-link aliases cannot make two logical buses share one backing slot. Plugin-host tests (64), worker-process tests (21), doc-tests, formatting, and strict Clippy passed. No plugin registration, audio stream, or machine configuration changed.
- Next M06/PLUG-03 task: implement the explicitly owned worker-result handoff carrying sequence/deadline identity into `RuntimeBusGeneration`; preserve fail-closed protected-path silence and backward-compatible single-stream workers.
- Implemented the M06/PLUG-03 worker-result handoff on 2026-09-09: `RuntimeBusQuantumIdentity` carries sequence, deadline, and frame count into `RuntimeBusGeneration`; `RuntimeBusWorkerResult` accepts caller-owned per-bus outputs, rejects shape/cardinality errors before mutation, and converts late, mismatched, or missing results to all-output silence. Engine tests (84), formatting, and strict Clippy passed. The engine remains dependency-independent from plugin-host, and auxiliary effect execution is not claimed.
- Next M06/PLUG-03 task: add a bounded adapter at the worker/engine integration owner that converts validated plugin-host bus frames into the engine result envelope without copying on the realtime callback; preserve legacy single-stream workers and protected-path silence.
- Implemented the M06/PLUG-03 worker/engine adapter on 2026-09-09: `plugin-host` now has a one-way dependency on `engine`; `stage_engine_worker_result` copies validated bus frames into caller-prepared `AudioBlock` and reference storage, preserving sequence/deadline/frame count for `RuntimeBusGeneration` without allocation at the result handoff. Plugin-host tests (65), worker-process tests (21), doc-tests, formatting, and strict Clippy passed. Legacy single-stream workers and protected-path silence remain unchanged; no audio or machine configuration changed.
- The supervised worker execution audit on 2026-09-09 confirmed that the shipped worker executable negotiates the legacy single-stream protocol and VST2's one replacing stereo stream; it does not yet negotiate `HelloBuses` or execute `ProcessBuses`. The new adapter is therefore intentionally exercised at the validated protocol/engine boundary only. No side-chain support is claimed for VST2, and no protocol downgrade or implicit bus flattening is allowed.
- Next M06/PLUG-03 task: add a separately negotiated multi-bus worker executable path (with an effect format that actually exposes auxiliary buses) and a bounded end-to-end late/missing-result regression; preserve the existing VST2 single-stream worker path.
- Added the M06/PLUG-03 separately negotiated multi-bus executable fixture on 2026-09-09: `--input-buses`/`--output-buses` emits `HelloBuses`, requires `Ready`, validates complete symmetric `ProcessBuses` sets, and returns `ProcessedBuses` with sequence/deadline identity preserved. A real framed process regression passed 22 worker-process tests (six fixture-dependent tests ignored) and clean shutdown. This is protocol execution evidence only; it does not load VST2 or claim auxiliary effect processing. No audio or machine configuration changed.
- Next M06/PLUG-03 task: replace the symmetric echo fixture with a worker backed by a rights-cleared effect format that genuinely exposes auxiliary buses, then add bounded late/missing-result supervision; preserve the existing VST2 single-stream worker path.
- Implemented the M06/PLUG-03 multi-bus pending-result gate on 2026-09-09: `WorkerBusSession` now binds each `ProcessedBuses` response to the exact outstanding sequence/deadline/frame-count identity, rejects unsolicited or mismatched responses before graph storage, and expires overdue work so late output cannot be reused. The focused plugin-host suite passed 66 tests, ordinary worker-process tests passed 13, doc-tests, formatting, and strict Clippy passed. This remains protocol/session supervision evidence; it does not claim a real auxiliary-bus effect or VST2 side-chain support, and no audio or machine configuration changed.
- Next M06/PLUG-03 task: exercise the pending-result gate through a separately supervised multi-bus process owner, then qualify a rights-cleared effect format that genuinely exposes auxiliary buses; preserve the existing VST2 single-stream worker path.
- Extended M06/PLUG-03 VST3 activation on 2026-09-09: the native offline loader now has explicit bounded `--multi-bus` support, activates every declared mono/stereo bus, supplies all buses to `IAudioProcessor::process`, and checks all outputs for finite samples. The pinned AGain side-chain class (two inputs, one output) processed successfully, while the default single-bus invocation still rejects it. M06 VST3 acceptance passed with validators, main AGain, side-chain, and mda matrix coverage; no plugin registration, audio stream, or machine configuration changed. Worker/realtime multi-bus scheduling and VST2 side-chain support remain gated.
- Next M06/PLUG-03 task: connect the validated auxiliary-bus activation to a separately supervised worker/process owner, then qualify an independent rights-cleared effect; preserve the existing VST2 single-stream worker path.
- Extended the M06/PLUG-03 framed process regression on 2026-09-09: the actual multi-bus worker test now drives `WorkerBusSession` through Hello/Ready, outstanding request tracking, matching response acceptance, and expired-work handling. The feature-enabled process suite passed 22 tests with six expected fixture-dependent skips; formatting, strict Clippy, and diff checks passed. No realtime callback or machine audio configuration changed.
- Next M06/PLUG-03 task: connect this session gate to a production-shaped supervised multi-bus process owner and the validated VST3 auxiliary-bus effect; retain the existing VST2 single-stream worker path.
- Added a fixture-gated typed multi-bus `WorkerProcess` client on 2026-09-09: it launches the separately negotiated worker, validates the exact `HelloBuses` layout, exchanges a complete `ProcessBuses` quantum, validates `ProcessedBuses`, and performs bounded shutdown. The feature-enabled worker-process suite passed 23 tests with six expected fixture-dependent skips; strict Clippy, formatting, and diff checks passed. Single-stream worker constructors remain unchanged and no audio or machine configuration changed.
- Next M06/PLUG-03 task: connect the typed client to a supervised multi-bus lifecycle owner and the validated VST3 auxiliary-bus effect; retain the existing VST2 single-stream worker path.
- Implemented the fixture-gated supervised multi-bus lifecycle owner on 2026-09-09: `SupervisedWorkerProcess` now uses the typed client, refreshes heartbeat only after a validated complete bus result, records protocol failures, restarts with the exact negotiated layout, and preserves the quarantine ledger. The feature-enabled worker-process suite passed 24 tests with six expected fixture-dependent skips; strict Clippy, formatting, and diff checks passed. The production VST3 worker and realtime graph scheduler remain gated; VST2 stays single-stream and no audio or machine configuration changed.
- Next M06/PLUG-03 task: connect the supervised owner to the native VST3 auxiliary-bus effect path, with bounded shared-memory/graph scheduling; retain the existing VST2 single-stream worker path.
- Added the supervised expired-quantum regression on 2026-09-09: a real multi-bus worker failure with `multiBusIdentity` now transitions the typed owner to `Failed`, terminates the worker, and retains the verified plugin hash/failure count in diagnostics. The feature-enabled worker-process suite passed 25 tests with six expected fixture-dependent skips; strict Clippy, formatting, and diff checks passed. No audio or machine configuration changed.
- Next M06/PLUG-03 task: connect the supervised owner to the native VST3 auxiliary-bus effect path, with bounded shared-memory/graph scheduling; retain the existing VST2 single-stream worker path.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain after the supervised multi-bus worker changes at pushed head `3de4e3e4` on 2026-09-09: VS2026/MSVC/SDK/WDK discovery/native compile, read-only 31-endpoint inventory, disposable pinned SysVAD x64 package/API/signability, M01/M04/M05, VST3 validators plus auxiliary-bus probe, VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159 normative mappings, and documentation validation (51 Markdown files/163 local links) passed. Temporary checkouts/artifacts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine audio configuration occurred. Production VST3 plugin-worker integration, realtime graph scheduling, independent rights-cleared plugin, and production release gates remain open.
- Next M06/PLUG-03 task: connect the supervised owner to the native VST3 auxiliary-bus effect path through a production-shaped worker and bounded graph scheduling; retain the existing VST2 single-stream worker path.
- Requalified the locked all-features workspace after the supervised multi-bus lifecycle change on 2026-09-09: all workspace unit/integration tests and doc-tests passed, including the 24 passing feature-enabled worker-process tests plus six expected skips; strict all-target Clippy, formatting, and diff checks passed. No driver, plugin registration, audio stream, signing-mode, or machine audio configuration action occurred.
- Next M06/PLUG-03 task: connect the supervised owner to the native VST3 auxiliary-bus effect path, with bounded shared-memory/graph scheduling; retain the existing VST2 single-stream worker path.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain after the VST3 auxiliary-bus probe at pushed head `04f696a1` on 2026-09-09: VS2026/MSVC/SDK/WDK discovery and native compile, read-only 31-endpoint inventory, disposable pinned SysVAD x64 compile/package/API/signability qualification, M01/M04/M05, VST3 validators plus AGain main and genuine auxiliary-bus activation, VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159 normative mappings, and documentation validation (51 Markdown files/163 local links) passed. Temporary checkouts/artifacts were cleaned. No driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine audio configuration occurred. Worker/realtime multi-bus scheduling, production driver/signing, installer, clean-machine, physical-latency, manual UI, and independent rights-cleared plugin gates remain open.
- Next M06/PLUG-03 task: connect the validated auxiliary-bus activation to a separately supervised worker/process owner, then qualify an independent rights-cleared effect; preserve the existing VST2 single-stream worker path.
- Clarified the normative M06/PLUG-03 specification on 2026-09-09: the bounded layout/frame/transport/engine/echo-worker contracts are now documented as protocol evidence, while VST2 remains explicitly single-stream and auxiliary effect execution, supervised multi-bus recovery, and rights-cleared multi-bus qualification remain release gates. No bus flattening or stale protected-path fallback is permitted.
- Added the M06/PLUG-03 late-result process regression on 2026-09-09: an expired multi-bus quantum now produces a bounded `multiBusIdentity` failure and non-successful worker exit instead of stale output. The process suite passed 22 tests (six fixture-dependent tests ignored), with formatting and strict Clippy green. This verifies worker-side fail-closed handling only; supervised restart/quarantine and real auxiliary-bus effect execution remain gated. No audio or machine configuration changed.
- Next M06/PLUG-03 task: replace the symmetric echo fixture with a worker backed by a rights-cleared effect format that genuinely exposes auxiliary buses, then add bounded late/missing-result supervision; preserve the existing VST2 single-stream worker path.
- Hardened the M06/PLUG-03 executable fixture on 2026-09-09: multi-bus quanta now pass through the monotonic sequence/deadline guard before a result is emitted, matching the single-stream worker's late-frame policy. The live-deadline process regression and worker-process suite passed 22 tests (six fixture-dependent tests ignored), with formatting and strict Clippy green. This remains protocol-fixture evidence only; no VST2 side-chain claim or machine audio change was made.
- Next M06/PLUG-03 task: replace the symmetric echo fixture with a worker backed by a rights-cleared effect format that genuinely exposes auxiliary buses, then add bounded late/missing-result supervision; preserve the existing VST2 single-stream worker path.
- Hardened M06/PLUG-03 result safety on 2026-09-09: `RuntimeBusGeneration::accept_worker_result` now validates every present source bus before copying any output, preventing partial publication when a later bus has a shape mismatch. The focused regression preserves existing destination samples on that failure; engine tests (84), formatting, and strict Clippy passed. No audio or machine configuration changed.
- Next M06/PLUG-03 task: add a separately negotiated multi-bus worker executable path (with an effect format that actually exposes auxiliary buses) and a bounded end-to-end late/missing-result regression; preserve the existing VST2 single-stream worker path.
- Requalified pushed head `0d5dd4d6` on 2026-09-09: `cargo test --workspace --all-features --locked` passed all workspace unit/integration tests and doc-tests, including engine (83), plugin-host (64), worker-process (21 plus six ignored fixture tests), control (98), domain (58), DSP (28), recording (30), storage (80), transport (19), Windows audio (33), CLI (28), and MCP interoperability (2). Strict all-features Clippy, formatting, diff checks, and documentation validation (51 Markdown files/163 local links) also passed. No driver, plugin registration, audio stream, or machine configuration changed.
- Requalified the pushed head on 2026-09-09 with `cargo test --workspace --all-features --locked`: all workspace unit/integration tests and doc-tests passed, including 57 plugin-host tests, 27 VST2 worker-process tests (with six fixture-dependent tests ignored by default), and 33 Windows-audio tests. `cargo fmt --all -- --check`, strict all-features Clippy, and `git diff --check` also passed. No plugin registration, audio stream, driver action, or machine configuration changed.
- Next M06/M00 task: continue with rights-cleared independent plugin and production native-driver evidence when available; portable and repository-local qualification remains green.
- Hardened M01/API-09 CLI discovery failures on 2026-09-09: the devices and application list adapters now preserve the complete JSON-RPC error envelope instead of converting backend failures to empty arrays; successful legacy array responses remain unchanged. CLI tests (27), doc-tests, strict Clippy, formatting, and diff checks passed. No audio or machine configuration changed.
- Next M07 task: retain structured discovery errors through the MCP result wrapper and authenticated remote transport; no additional native or plugin authority is inferred.
- Added M07/API-09 MCP stdio transport regression on 2026-09-09: an intentionally invalid bounded `list_devices` request crossed the real CLI MCP process boundary and retained its MCP error envelope in both structured and textual JSON views. The two MCP interoperability tests, CLI tests (28), doc-tests, strict Clippy, formatting, and diff checks passed; no audio or machine configuration changed.
- Next M07 task: retain this envelope across any future remote transport implementation and continue native-gated work only when its prerequisites become available.
- Documented M07/API-09 discovery-error behavior on 2026-09-09 in the API reference and headless runbook: CLI/MCP consumers are directed to stable codes, optional unsigned HRESULTs, retryability, and remediation, with `deviceInUse` explicitly distinguished from `invalidArgument`. Documentation validation passed with 51 Markdown files and 162 local links; no runtime or machine configuration changed.
- Next M07 task: retain the documented error contract while any future remote transport is added; continue native-gated work only when prerequisites become available.
- Probed the pinned SDK's second-vendor advanced-techniques tutorial on 2026-09-09: the disposable x64 Steinberg bundle passed the official validator (47/47) but AudioRouter activation returned `0x80004001` (`E_NOTIMPL`); the sibling data-exchange tutorial was not buildable because its Windows source references missing `FDebugPrint`. Both results are recorded as unsupported fixture evidence, with no system link, registration, plugin install, or audio configuration change. The multi-vendor M06 gate remains open.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture through the existing worker matrix; retain the unsupported second-vendor probe and gated release boundary.
- Completed M07/API-09 MCP discovery-error parity on 2026-09-09: MCP tool results now retain structured backend error data in both `structuredContent` and the textual JSON content, including category, HRESULT, retryability, and remediation. CLI tests (28), MCP interoperability tests (2), doc-tests, strict Clippy, formatting, and diff checks passed; no audio or machine configuration changed.
- Next M07 task: retain this error envelope across any future remote transport implementation and continue native-gated work only when its prerequisites become available.
- Hardened M01/API-09 CLI discovery parity on 2026-09-09: `devices list`, `apps list`, and `applications list` now preserve the complete JSON-RPC error envelope, including structured audio category, HRESULT, retryability, and remediation, instead of silently returning an empty array on failure. Successful legacy array responses remain unchanged. CLI tests (27), doc-tests, strict Clippy, formatting, and diff checks passed; no audio or machine configuration changed.
- Next M01/M07 task: carry the same structured discovery-error envelope through any future remote CLI/MCP presentation while retaining the authenticated transport boundary.

- Requalified the guarded M00–M08 acceptance chain again on 2026-09-08 at pushed head `4475a5d0`: native toolchain/SysVAD qualification, portable tests, UI, VST3/VST2 matrices, M07, unsigned release artifacts, traceability, and documentation all passed. Disposable outputs were cleaned and no driver, signing, registration, stream, or machine audio configuration was changed.
- Next M02/M03/M06 task: retain the validated boundary while awaiting the production managed-driver callback, physical latency setup, independent plugin rights/fixture evidence, and authenticated native-shell HWND owner.

M00 feasibility began with a read-only inventory and now includes native
Windows validation from the installed VS/WDK toolchain. All probes preserve
the user's audio configuration and do not install drivers or alter defaults.

- Hardened M00 capture diagnostics on 2026-09-08: `IAudioClient::Initialize` failures now identify whether event-callback or polling delivery was being initialized. The fallback remains restricted to exact `E_INVALIDARG`; device-in-use, access-denied, and other HRESULTs remain visible without reclassification. Windows-audio tests (31), strict Clippy, formatting, and diff checks passed; no stream or machine configuration changed.
- Next M00/M02 task: retain the explicit retry diagnostics while connecting the adapter to the managed endpoint-owned callback after the production driver boundary exists.

- Strengthened CAP-05/CAP-06 process identity on 2026-09-08: Windows application discovery now reports a bounded nullable full executable path when permitted, the control contract exposes `executablePath`, and path-aware restart/bind helpers reject same-named binaries from another location while retaining creation-time checks. Workspace control (97) and Windows-audio (32) tests, strict Clippy, formatting, and docs validation passed; no audio or machine configuration changed.
- Next CAP-06/M02 task: use the path-aware selector in durable process-tree binding once the managed process-capture owner is integrated; retain the basename compatibility wrapper only for legacy callers.

- Requalified the Rust application identity inventory on 2026-09-08 through `cargo run --quiet -p audiorouter-cli -- --json applications list`: 443 records returned, 195 with both full executable path and creation timestamp, and 13 with observed audio sessions. Unavailable process metadata remained null rather than guessed. This was read-only discovery; no audio stream or machine configuration changed.
- Next CAP-06/M02 task: use the path-aware selector in durable process-tree binding once the managed process-capture owner is integrated; retain unavailable identity as fail-closed.

- Requalified the M05 UI against the expanded application contract on 2026-09-08: TypeScript typecheck, 14 Vitest files/91 tests, and a temporary production build all passed. The temporary build was removed; no backend, audio, or machine configuration changed.
- Next M05/M06 task: retain executable-path identity as read-only discovery data until the authenticated native shell and managed process-capture owner are available.

- Corrected CAP-05 metadata retention on 2026-09-08: executable path and creation-time queries are now tracked independently, so a failed creation-time query no longer discards a successfully observed path. Control (97) and Windows-audio (32) tests, strict workspace Clippy, formatting, and diff checks passed; no audio or machine configuration changed.
- Next CAP-06 task: require both path and creation identity for durable process binding when the managed process-capture owner is integrated; preserve null metadata as fail-closed.

- Requalified the guarded differing-rate M02 route on 2026-09-08: explicitly selected 96 kHz capture to 48 kHz render produced 47,040 capture frames, 183 graph blocks, 23,424 scheduler frames, and 23,424 routed frames. The 128-frame negotiated deadline was 1,333,334 ns; processing p99.9 was 65,536 ns with zero deadline misses/lateness. Stream cleanup and media-state preservation passed. This remains shared-mode user-space evidence, not managed-driver callback or physical-latency qualification.
- Next M02/M03 task: connect the rate-aware scheduler to the managed endpoint-owned callback after driver lifecycle exists; retain the guarded differing-rate route as repeatable user-mode evidence.

The installed ReaPlugs effect DLLs were copied only to the ignored
`third_party/local-test-fixtures/ReaPlugs` directory for compatibility testing.
The scanner identifies x64 binaries as `vst2` by either established read-only
PE export (`VSTPluginMain` or legacy `main`) while retaining `unsupportedFormat`
compatibility and no VST3 class IDs. This confirms the legacy VST2 entry-point
boundary; scanning itself loads no plugin code. Built-in native transformation is the supported path: DSP and engine
revalidation passed gain/EQ, gate,
compression, limiting, delay, pitch, metering, finite-sample repair, and
allocation-free prepared processing.

Requalified the native built-in transformation route with the guarded
`m02-rust-adapter-route-live.ps1` wrapper at 300 ms. AudioRouter applied its
built-in gain graph between the explicitly selected VB-Audio endpoints and
processed 14,880 capture frames into 116 graph blocks and 14,304 routed frames,
with zero deadline misses and a 32,768 ns p99.9 processing bound. Streams were
stopped/reset and endpoint/media/configuration snapshots were unchanged. This
is shared-mode built-in DSP evidence, not managed-driver callback or physical
latency evidence.

## Approved legacy VST2 extension (2026-09-08)

The user explicitly authorized adding legacy VST2 binaries to the goal. This is
an M06 extension to the original VST3-only baseline; the current host now has
a Windows-only, gated VST2 adapter and worker path. Requirement scope is `PLUG-01` through
`PLUG-07`, `SEC-07`, and `SEC-12`: user-installed native x64 VST2 audio effects
only, discovered by explicit scan and hosted behind the existing disposable
worker boundary. x86 bridging, instruments/MIDI, Audio Units, scripts,
redistribution, auto-download, and protected-voice dry fallback remain out of
scope.

The six copied ReaPlugs fixtures expose `VSTPluginMain`; the scanner identifies
them as VST2. The adapter now loads and processes compatible effects only inside
the disposable worker, and the complete local matrix passes its bounded
processing/control checks. They remain ignored, disposable local fixtures and
were not registered. The gate still requires crash/hang/invalid-sample/layout,
native editor, chunk-state, rights, and release-compatibility evidence. The
built-in DSP chain remains the supported native transformation path while this
gate is open.

The public identity contract now distinguishes `supportedVst2X64Gated` from
`supportedVst3X64` and `unsupportedFormat`. This reports the approved worker
boundary accurately without making the VST2 extension release-qualified or
enabling it in a route without the remaining gates.

The gated VST2 status is Windows-only, matching the worker adapter: a
non-Windows discovery build reports the format as unsupported rather than
advertising a capability it cannot launch. Platform-specific regression
coverage protects both branches.

The VST2 worker now checks every native output sample for finiteness before
copying it into the framed response. A NaN/Inf result is a contained worker
failure and therefore follows the existing failure/quarantine policy; it is
never serialized as audio or allowed onto a protected path.

The same finite-output invariant is now enforced inside `Vst2Library::process_replacing`
before control returns from the native callback boundary. A focused Windows
regression injects NaN output and verifies `NonFiniteOutput`; the worker-level
check remains as defense in depth before framing.

The same boundary regression verifies a mismatched output frame layout is
rejected as `InvalidPath` before the native process callback is entered.

Per-binary worker diagnostics now retain a stable last-failure category
(`Immediate` or `HeartbeatTimeout`) alongside verified identity, count, and
quarantine state. Regressions cover both categories without exposing paths or
audio data.

`SupervisedWorkerProcess` now exposes that bounded diagnostic snapshot to its
owning control plane, so recovery code can report the verified binary and
failure category without consuming or reconstructing process internals.

Added `tests/acceptance/m06-vst2-reaplugs.ps1` to rerun every ignored local
VST2 DLL independently through the verified worker test. It requires Windows,
restores any pre-existing `AUDIOROUTER_VST2_FIXTURE` value, and changes no
plugin registration or audio configuration.

Added the repository-owned source fixture `tests/fixtures/vst2-state-fixture.c`
and `tests/acceptance/m06-vst2-state-fixture.ps1`. The installed VS2026 x64
compiler built its ignored DLL, which passed the verified worker acceptance
with opaque VST2 program-chunk save/restore. This supplies chunk-state evidence
without redistributing a third-party binary.

The same source fixture can now be built with a legacy `main` export. The
state-fixture acceptance compiles that ignored x64 DLL and runs the verified
worker load/process test against it, providing runtime evidence for the
fallback entry point without adding a third-party binary.

The legacy `main` fixture also runs the opaque chunk-state save/restore
acceptance, confirming that the fallback covers stateful processing as well as
basic audio transformation.

The fixture build also emits a deliberate non-finite-output variant. The
verified worker returns a bounded failure frame and the supervisor records the
native invalid-sample fault; no NaN/Inf sample is serialized as audio.

The same fixture build emits deliberate processing-crash and processing-hang
variants. The supervised acceptance confirms both faults are contained and
reaped within the worker boundary, with no host panic or unbounded wait.

The stronger behavioral round trip initially exposed that `effSetChunk` was
being called with a hard-coded byte count of one. The adapter now passes the
bounded chunk length; the fixture test changes its mix parameter, verifies the
changed audio, restores the saved chunk, and verifies the original mix returns.

The VST2 adapter now caches the negotiated sample-rate/block-size pair and
avoids repeating `effSetSampleRate`, `effSetBlockSize`, and mains-on for every
block. A format change performs a bounded mains-off/reconfigure/mains-on
transition; normal blocks do not re-enter plugin lifecycle callbacks.
The Windows dispatcher-count regression covers both the cached repeat and the
format-change transition.

The worker now applies the cached format before VST2 parameter automation on a
block, ensuring legacy setters see the negotiated sample rate and block size.
The six-binary ReaPlugs matrix and chunk-state behavioral fixture both passed
after this ordering change.

Added bounded VST2 `effEditOpen`, `effEditClose`, and `effEditIdle` primitives
with explicit editor-open state and RAII close ordering. They are intentionally
invoked only by the dedicated Windows editor thread; parent-window
authorization remains required before exposing editor controls.

Editor opens now reject zero or stale parent handles through a read-only
`IsWindow` check before calling plugin code; explicit control-plane
authorization and dedicated UI-thread/message-pump ownership remain required.

Added a disposable `Vst2EditorThread` owner that loads a separate editor
instance, pumps the Windows queue, and serializes editor open/close/idle calls
on its own thread. The worker protocol now delegates editor requests to this
owner without sharing the processing instance; the remaining integration gate
is to pass an explicitly authorized parent from the control plane.

Worker messages now carry bounded `EditorOpen`/`EditorClose` requests and
explicit opened/closed responses. A worker without a VST2 editor returns
`editorUnavailable` without being terminated; the generic worker regression
covers that fail-closed behavior and confirms processing remains available.

The first native ReaPlugs editor probe exposed and fixed an ABI defect in the
editor capability flag: `effFlagsHasEditor` is bit 0, not bit 2. After that
fix, all six local binaries entered native editor dispatch but did not return
within the five-second UI-thread bound when attached to the synthetic hidden
parent. The ignored Windows acceptance now records this as a bounded native
plugin failure; it does not claim successful editor-window compatibility.
The worker/editor thread remains disposable and the processing path is not
replaced or reconfigured by this probe.

Added an end-to-end ignored Windows regression for the same behavior: a
nonreturning native editor causes `SupervisedWorkerProcess` to terminate the
contained worker and record one failure. The host does not wait indefinitely,
restart automatically, or change the protected-path fallback decision.

Added `tests/acceptance/m06-vst2-editor.ps1`, which repeats both bounded
editor-containment checks across every ignored ReaPlugs fixture and restores
the caller's existing fixture environment variable.

Requalified the full workspace at this VST2 head: all locked workspace tests
and doc-tests, strict workspace Clippy, formatting, diff checks, and
documentation validation passed. The ignored native editor timeout probe and
six-binary VST2 processing matrix were also rerun separately. No driver,
plugin registration, audio stream, or machine audio configuration was changed.

Ordered next tasks: (1) integrate an explicitly authorized control-plane/UI
parent-window token with the worker-owned native editor path; (2) obtain an
additional legally usable independent VST2 fixture for compatibility and
chunk-state evidence; (3) keep VST2 user-facing availability gated until the
rights, editor, compatibility, and release matrix is complete. Invalid-output,
layout, crash/hang, and bounded per-binary failure-diagnostic regressions are
implemented and requalified below.
Rollback is limited to reverting the adapter/tests/docs and removing ignored
fixture copies; no plugin registration, driver, stream, default endpoint, or
machine audio setting may change.

Added the portable VST2 ABI contract and Windows worker adapter at
`crates/plugin-host/src/vst2.rs`: the VST 2.4 `AEffect` C layout, entry-point
and callback signatures, replacing-process requirement, and bounded
mono/stereo/parameter validation are covered by two focused tests. No unsafe
block, DLL load, callback invocation, or machine-state operation was added.

The same module contains a Windows-only RAII loader used by the worker: it
resolves `VSTPluginMain` with legacy `main` fallback, validates the returned header, sets bounded
format values, processes caller-owned fixed blocks, and closes the
effect/library on drop. Unsafe FFI invariants are documented.

The loader now accepts both established VST2 export spellings,
`VSTPluginMain` and legacy `main`, while retaining the same x64 identity,
header-validation, and disposable-worker gates. The existing ReaPlugs matrix
continues to use `VSTPluginMain`; the repository-owned `main`-export fixture
now supplies runtime fallback qualification.

The scanner has a focused synthetic x64-PE regression proving a `main`-only
export is classified as VST2, keeping inspection and loading entry-point
support aligned.

The worker supervisor now passes only verified x64 VST2 identities to the
contained worker. Plugin-host tests (50), worker-process tests (13), and strict
Clippy passed.

The opt-in native VST2 matrix now loads and processes all six copied ReaPlugs
fixtures successfully inside the bounded worker. The cause of the earlier
four-fixture failures was an adapter ABI defect: VST2 dispatcher opcodes for
sample-rate, block-size, and mains lifecycle had been shifted onto the chunk
state opcodes. Correcting them restored all six bounded acceptance runs; no
audio-device access was involved. This is fixture evidence, not blanket VST2
compatibility or release qualification.

The worker host callback now answers only bounded version, 48 kHz sample-rate,
and block-size queries. ReaComp, ReaGate, ReaDelay, ReaXComp, ReaEQ, and ReaFIR
all passed the opt-in 128-frame worker acceptance after the dispatcher-opcode
correction. The regression test remains per-binary and isolated so a future
fixture failure cannot be generalized away.

The opt-in ReaComp acceptance now also discovers its VST2 parameter descriptors
and applies the first descriptor's bounded default-value event before the
128-frame process call. That parameter/control-plane path passed in the
contained worker. Its VST2 flags report no program chunks, so state save returns
an explicit `UnsupportedFeature` without killing the worker; plugin editor
ownership remains a separate qualification task. The same run now returns the
plugin's bounded `AEffect::initial_delay` through the worker latency contract.

ReaGate also passes the opt-in descriptor, parameter, state-capability,
processing, and latency acceptance. VST2 parameter offsets outside the current
block are now rejected before a setter call; valid offsets retain the current
worker's block-boundary automation semantics rather than pretending to be
sample-accurate.

The worker also exposes a read-only `DescribeEditor` response. ReaComp and
all five editor-capable ReaPlugs passed bounded editor-capability/rectangle
discovery. No HWND was created and no native editor was opened; UI-thread
ownership, close/retry semantics, and editor isolation remain a separate gate.

## VST2 dispatcher opcode regression (2026-09-08)

Corrected the Windows VST2 adapter's lifecycle dispatch constants to the VST2
2.4 values: `effSetSampleRate=10`, `effSetBlockSize=11`, and
`effMainsChanged=12`. The previous values overlapped the `effGetChunk` and
`effSetChunk` state opcodes, so valid plugins could receive the wrong operation
during setup and appear to hang or fail during processing. A Windows opcode
regression test locks the mapping. The isolated opt-in matrix was rerun with
each ignored ReaPlugs fixture: all six passed load, bounded parameter/control,
process, editor-capability, latency, state behavior, and shutdown acceptance.
This does not open an editor or qualify chunk-capable state because the current
fixtures do not provide that evidence; no audio endpoint or machine setting
was accessed.

## Per-binary quarantine diagnostics (2026-09-08)

`WorkerSupervisor` now retains the verified `PluginIdentity` associated with
its failure ledger and exposes a bounded `WorkerFailureDiagnostic`. The
diagnostic includes the canonical path, binary path, format, architecture,
fingerprint, failure count, and quarantine state, so a replacement cannot be
mistaken for the binary that failed. A regression verifies identity retention
across the first failure. This is control-plane metadata only; it does not
alter restart, quarantine, protected-voice silence, or audio-device behavior.

The locked all-workspace regression sweep then passed, including control (97),
domain (58), DSP (28), engine (78), plugin-host (48), storage (80), transport
(19), Windows audio (30), and the remaining package suites. Workspace strict
Clippy, formatting, documentation validation (51 Markdown files/160 links),
and `git diff --check` passed. No audio endpoint or machine configuration was
accessed.

Closed an M01/SEC-12 transport boundary gap: Windows named-pipe read and write
loops now validate the byte count returned by Win32 before slicing the
remaining buffer. Zero-byte results still map to bounded EOF, while
over-reported counts return a protocol error instead of panicking the control
server. Transport tests (19), strict Clippy, and formatting pass; no audio or
machine configuration was accessed.

Requalified the complete guarded M00-M08 safe acceptance chain at pushed head
`cf0c54a1`: installed VS/WDK compatibility, native compile and 34-endpoint
format inventory, disposable pinned SysVAD x64 compile/package/API checks,
portable milestone suites, UI, pinned VST3 SDK/validator/offline loader,
headless control, unsigned release preparation, 158-ID traceability, and
documentation validation all passed. Temporary outputs and the disposable
reference checkout were removed. No driver was installed or loaded, no
signing mode or registration changed, and no machine audio configuration was
modified. Native production-driver ownership, callback deadline, physical
latency, signing, installer, and manual UI gates remain open.

## Durable graph-plan persistence atomicity (2026-09-08)

Closed an M01/GRAPH-03 persistence consistency gap: if durable graph-plan
storage rejects a newly planned graph, `ControlPlane::plan_graph` now restores
the in-memory store before returning the bounded request error. A regression
fills the durable plan inventory, forces this failure, and verifies the
in-memory plan cannot subsequently be committed. Control tests (94), strict
Clippy, and formatting pass; no audio endpoint or machine configuration was
accessed.

The complete elevated safe acceptance chain was requalified at pushed head
`2c8ab54d`: native toolchain/format inventory, disposable SysVAD x64
qualification, all portable milestone suites, UI, SDK/VST3, headless,
unsigned release preparation, traceability, and documentation all passed.
Temporary outputs were cleaned; no driver was installed or loaded and no audio
configuration changed.

The opt-in M06 worker-fixture qualification also passed with
`--features test-fixtures`: plugin-host unit coverage (47) and worker-process
coverage (20) exercised controlled crash, hang, invalid output, state failure,
dynamic latency, and quarantine/replacement behavior under bounded cleanup.
This is deterministic containment evidence only; third-party plugin/editor
and OS sandbox gates remain open.

Closed an M01/SEC-12 numeric-boundary gap in `GraphStore`: exhausted graph-plan
ID counters now return the existing bounded `PlanLimitReached` error instead of
performing unchecked integer addition. A regression verifies the counter and
pending-plan state remain unchanged. Domain tests (58), strict Clippy, and
formatting pass; no audio or machine configuration was accessed.

Closed an M01/GRAPH-03 restart collision: a storage-backed controller now
advances its generated graph-plan counter past retained durable `plan-N` IDs.
This prevents a new plan after restart from replacing an older uncommitted
candidate. The restart regression verifies `plan-1` and `plan-2` coexist, and
a deterministic allocator regression covers both timestamped plan families;
control tests (97), strict Clippy, and formatting pass without audio or
machine configuration access.

Closed an M01/M03 persistence-safety gap in timestamped pending-plan
allocation: startup and virtual-device plans now skip IDs already retained
after restart before inserting a new plan, while preserving explicit same-ID
replacement at the storage API boundary. Control tests (97), strict Clippy,
and formatting pass; no audio or machine configuration was accessed.

Closed an M01/SEC-12 persistence conversion gap: pending startup and
virtual-device plan inventory counts now use checked SQLite-to-`usize`
conversion instead of unchecked casts. This keeps malformed count values
fail-closed at the same storage boundary as other bounded inventories. Storage
tests (80), strict Clippy, and formatting pass; no audio or machine
configuration was accessed.

## Fan-out topology safety (2026-09-08)

Closed an M02/GRAPH-01/ARCH-07 fail-closed gap in the portable fan-out
compiler. A fan-out plan now rejects any unrelated enabled node instead of
silently omitting it from the prepared runtime. The new regression and the
77-test engine suite pass with strict Clippy, formatting, and diff checks.
This remains portable graph evidence; native endpoint routing is unchanged and
the managed-driver gate remains open.

The same fail-closed participant check now covers the portable mixer
compiler: an isolated enabled node cannot be silently dropped from a prepared
source-to-mixer-to-output graph. The regression passes with the 77-test engine
suite and strict Clippy. This remains portable topology evidence and does not
change native routing behavior.

The linear compiler now applies the same participant closure: a valid route
cannot compile while an unrelated enabled node is silently omitted. The new
regression passes with the 78-test engine suite and strict Clippy. This is
portable graph validation only; native routing and machine audio configuration
are unchanged.

The post-change workspace requalification also passed with all workspace
features and the locked dependency set; the result is recorded in the M08
release evidence. No live endpoint was opened for this source-only check.

The elevated complete safe acceptance chain also passed at the current head,
including native read-only inventory and disposable SysVAD qualification. Its
temporary outputs were removed and it did not install/load a driver or alter
machine audio configuration.

## Durable plan expiry reload hardening (2026-09-08)

Control restart now converts persisted plan expiry to an in-memory duration
only when the remaining lifetime is strictly positive and representable. This
prevents expired or malformed timestamps from becoming huge `u64` durations
after restart. The 93-test control suite, strict Clippy, and formatting pass;
no audio endpoint or machine configuration is accessed.

The subsequent full locked workspace tests and strict workspace Clippy also
passed at `1b782fd3`; the M08 evidence records the complete result.

Persisted plan hydration now also caps a far-future timestamp at the
contractual five-minute TTL, preventing malformed database values from
overflowing `Instant` construction. The control suite remains 93/93 with
strict Clippy and formatting; no audio or machine state is accessed.

The complete elevated safe acceptance chain was requalified at `c4995aae`:
native toolchain/format inventory, disposable SysVAD x64 qualification, all
portable milestone checks, unsigned release preparation, traceability, and
documentation passed. Temporary outputs/checkouts were removed; no driver or
audio configuration changed.

Durable graph-plan rehydration now uses the same checked, five-minute-capped
expiry conversion as virtual-device and startup plans, preventing malformed
far-future SQLite timestamps from overflowing `Instant` during `graph.commit`.
The control suite remains 93/93 with strict Clippy and formatting.

## Capture retry classification regression (2026-09-08)

Added a Windows-audio regression proving the capture fallback retries only
exact `E_INVALIDARG` (`0x80070057`). `AUDCLNT_E_DEVICE_IN_USE` and access
denied remain non-fallback errors, so an occupied endpoint cannot be silently
reclassified as a format/event-mode incompatibility. The 30-test Windows-audio
suite and strict Clippy pass; this test opens no audio stream.

## Current handoff correction (2026-09-08)

The older historical notes below that describe missing Visual Studio/WDK or
deny live capture are superseded by the dated evidence above. The host now has
the VS Community, Windows SDK, and WDK toolchain, and the user has explicitly
authorized bounded live capture/render testing with rollback. Existing native
lifecycle, process-loopback, event-driven, format, digital-loopback, and Rust
adapter evidence has been requalified. Remaining safe evidence is calibrated
physical latency and restart/PID-reuse behavior; the managed endpoint-owned
driver, production callback, signing, installer, clean-machine, and manual UI
gates remain open.

## Native lifecycle and process-attribution acceptance (2026-09-08)

With explicit live-audio authorization, the guarded `m00-native-live.ps1`
acceptance passed across all 13 active capture and 21 render endpoints at
100 ms. Capture start/stop/reset and silent render lifecycle completed for all
usable endpoints; one render endpoint was correctly classified as occupied.
The media-device snapshot before and after the run was identical.

The guarded `m00-native-process-live.ps1` acceptance also passed at 500 ms.
Its disposable tone-producing child and selected process tree produced
19,845 captured frames and 70,847 nonzero payload bytes, then exited cleanly.
This closes the controlled process-attribution data-path evidence for M00;
physical acoustic latency, managed-driver ownership, signing, and installer
gates remain open. No persistent audio configuration changed.

The complementary guarded exclude-tree run also passed at 500 ms, capturing
22,050 frames with the disposable child excluded. Include and exclude modes
now both have native lifecycle/data-path evidence; the remaining M00 audio
gates are calibrated physical latency and managed virtual-driver integration.

The read-only native format inventory also passed across all 34 active
endpoints. It captured complete mix-format descriptors, including 48 kHz
mono/stereo extensible float, 96 kHz mono PCM, and 96 kHz eight-channel
extensible float variants, with unchanged media-device identity and temporary
artifact cleanup. This is negotiation evidence only; it does not imply every
endpoint accepts every requested format.

The authorized 500 ms Rust adapter requalification also passed: the
production adapter processed 24,480 captured frames into 191 generation-1
graph blocks, with 24,448 scheduler frames, 25,536 silent render frames, zero
xruns, and zero deadline misses. The explicitly selected VB-Audio route then
processed 24,000 capture frames into 187 blocks and routed 23,936 frames with
zero deadline misses. Both wrappers verified stream cleanup and unchanged
media state. This remains user-mode adapter/digital-route evidence, not
managed-driver callback or physical-latency evidence.

The explicit differing-rate route then passed using the inventory-selected
96 kHz mono capture endpoint and 48 kHz stereo render endpoint: 48,000
capture frames were reduced to 187 fixed graph blocks and 23,936 routed
frames, with a 16,384 ns processing p99.9 upper bound and zero deadline
misses. Endpoint/media cleanup checks passed. This is bounded resampler and
digital-route evidence; clock drift, physical latency, and production-driver
ownership remain open.

The guarded 1,000-impulse digital loopback acceptance also passed over the
existing VB-Audio cable: 996 impulse groups were detected, p95 spacing error
was 0 frames, and estimated onset was 75.33 ms. Temporary capture/log files
were removed and the media state remained unchanged. This validates digital
signal propagation and cadence only; calibrated physical acoustic latency and
managed-driver ownership remain open.

The guarded Rust asynchronous process-loopback acceptance also passed at
250 ms in both include and exclude modes. Include converted 10,584 source
frames into 11,392 engine frames across 89 generation-1 quanta; exclude
converted 11,025 into 11,904 across 93 quanta. Both reported zero rejected
packets and zero scheduler XRuns/overruns/underruns, with stream cleanup and
unchanged media state. This is Rust adapter/process-loopback evidence, not
production-driver callback timing or physical-latency evidence.

The guarded event-driven native acceptance passed over the selected VB-Audio
pair at 500 ms: capture read 24,480 frames and silent render submitted 28,800
frames. Both event clients started/stopped/reset successfully, temporary
artifacts were removed, and the media snapshot was unchanged. This validates
the event-driven lifecycle shape only; production driver ownership and
physical latency remain open.

## Portable liveness and recovery boundary audit (2026-09-08)

The adapter recovery policy is now explicitly requalified against CAP-06,
CAP-11, and CAP-12: exact endpoint identity and mix-format changes fail closed
before activation; transient device/service failures retry only within a
bounded policy; non-transient failures are not retried; and process restart
rebinding requires one verified executable plus creation-time identity. The
focused regressions pass without opening a stream or changing machine audio
configuration. Sleep/resume, reboot, Windows Audio service restart,
multi-user transitions, and an actual PID-reuse occurrence remain native
lifecycle gates, so no production recovery claim is made.

## Safe acceptance requalification at `4d9ce1f4` (2026-09-08)

The complete `tests/acceptance/safe-all.ps1` chain passed at the current
pushed implementation head. It covered the installed VS/SDK/WDK toolchain,
native compile and 34-endpoint format inventory, disposable pinned SysVAD
x64 build/package/API validation, M01/M04/M05/M06/M07 acceptance, unsigned
M08 artifact preparation, 158 normative requirement IDs, and documentation
validation (51 Markdown files and 160 local links). Temporary outputs and
checkouts were removed. Driver installation/loading, signing-mode changes,
plugin/startup registration, and machine audio configuration were excluded
and unchanged.

## Safe acceptance requalification at current head (2026-09-08)

After a sandbox-only `Get-PnpDevice` permission failure, the complete
`tests/acceptance/safe-all.ps1` chain was rerun with authorized native access
and passed. It covered M00 toolchain/native compile and 34-endpoint format
inventory, disposable pinned SysVAD x64 compile/package/API validation, M01
CLI, M04 DSP/recording, M05 UI (91 tests plus typecheck/build), M06 SDK/VST3,
M07 headless control/CLI/MCP/plugin validation, unsigned M08 artifact
preparation, 158-ID traceability, and documentation validation (51 Markdown
files/160 links). Temporary artifacts and checkouts were removed. No driver
was installed or loaded and no machine audio configuration changed.

## Durable graph-history retention bound (2026-09-08)

Closed a persistence retention gap in M01/STATE foundations. The SQLite
`session_history` table now trims transactionally after both ordinary session
writes and journaled writes, retaining the newest 100 revisions per session,
matching `GraphStore`'s existing undo/history budget. The trim is scoped to the
session and occurs before transaction commit; a SQL failure therefore rolls
back the current document, history, and journal outcome together.

Storage regressions cover both write paths, verify newest-first boundaries
(revisions 101 through 2 after 102 writes), and confirm the durable row count
is exactly 100. Domain history tests still pass after replacing hard-coded
limits with the shared `MAX_GRAPH_HISTORY_ENTRIES` constant. No audio or
machine configuration was accessed.

Closed an M01/M07/SEC-12 session-import boundary gap: import-plan IDs now use
the bounded collision-aware allocator and fail closed when the counter is
exhausted, preventing a saturated counter from replacing an existing pending
candidate. Control tests (97), strict Clippy, and formatting pass; no audio or
machine configuration was accessed.

## Expired pending-plan retention (2026-09-08)

Closed an M01/M07/SEC-12 durable-retention gap. Expired virtual-device and
startup plan rows are now removed during their save/load paths and during
general recovery maintenance, so filtering expired rows cannot leave an
unbounded SQLite tail. Active plans remain subject to the existing 100-item
fail-closed inventory limit. Storage regression coverage verifies expired
rows are physically deleted while live rows survive; this change does not
touch audio or machine configuration.

## Durable graph-plan admission bound (2026-09-08)

Closed the remaining M01/SEC-12 lower-layer pending-plan bypass. Direct SQLite
graph-plan writes now prune expired rows and enforce the shared 100-live-plan
limit, while replacing an existing plan remains allowed. This matches domain
and control admission behavior and prevents callers that bypass control from
creating an unbounded durable plan table. Storage coverage verifies overflow
rejection and replacement at capacity; no audio or machine configuration was
accessed.

## M06 independent VST3 fixture qualification (2026-09-08)

Built the official ChowMatrix VST3 source fixture from Chowdhury DSP commit
`40d8e0ef1f752a6843099ff3dfc3d99132b332eb` in a disposable checkout using the
repository-local CMake 4.4.0, Visual Studio Community 2026/MSVC 14.51.36231,
and Windows SDK `10.0.28000.0`. The resulting x64 bundle was recognized by
`plugins scan` as `supportedVst3X64`; its binary SHA-256 is
`9ed07c61c3ddba6504b7307a92087ee37ec6236e2989954b4cc2e8053ee0d457`.

The corrected native loader loaded the real bundle, enumerated its audio
effect class, processed a finite offline stereo block, and round-tripped 3364
bytes of component state. The loader now accepts the valid zero-parameter
controller case; this fixes a probe false negative. The official SDK validator
reported 45 passed and 2 failed (32-bit state transition and bus activation),
so this independent fixture is useful compatibility evidence but does not
close the M06 three-effect/two-vendor gate. The temporary checkout was not
installed or registered and was removed after evidence capture; no audio or
machine configuration changed.

The corrected loader also requalified distinct mda classes from the pinned
Steinberg fixture: Ambience (class 0), BeatBox (4), Combo (6), DeEsser (8),
and Degrade (10) each completed finite stereo processing, parameter
round-trip, and state round-trip. Together with ChowMatrix this provides
six loader-compatible effects from two independently sourced vendors, while
the ChowMatrix SDK-validator failures remain an open qualification risk.

The M06 acceptance wrapper now reproduces the five-class mda matrix after its
standard SDK validator and loader checks. The `-SkipBuild` requalification
passed on the installed VS/SDK toolchain; generated loader artifacts were
removed by the wrapper's `finally` cleanup.

Closed an M06/PLUG-02/PLUG-06 discovery gap: bounded VST3 bundle metadata now
reports optional vendor, version, and deduplicated class IDs through the Rust,
JSON-RPC, and TypeScript contracts without loading plugin code. A trailing-
comma-tolerant fixture regression, plugin-host/control tests (39/90), strict
Clippy, contracts typecheck/drift, and documentation validation pass.

## Native VST3 parameter descriptor evidence (2026-09-08)

The offline VST3 loader now enforces the worker's 256-entry descriptor ceiling,
validates finite normalized defaults, and emits bounded controller records with
parameter ID, ASCII-safe title, default, step count, and flags. The pinned SDK
acceptance passed after rebuilding with VS2026; the five mda matrix effects
emitted 5/13/8/4/7 descriptors, while the zero-parameter ChowMatrix case
remained valid. This is native offline discovery evidence only; mapping into
the Rust worker catalog and third-party realtime execution remain open.

## Worker descriptor wire regression (2026-09-08)

The opt-in worker fixture now has a `descriptors` mode that returns two valid
normalized parameter descriptors. The feature-gated subprocess regression
round-trips both entries and checks their IDs, titles, and bounds; the ordinary
fixture remains an explicit empty catalog. Plugin-host process tests (18 with
fixtures) and library tests (46) pass. This proves the typed wire transport,
not native third-party parameter mapping or realtime plugin execution.

The opt-in worker `latency` fixture now returns bounded same-rate updates; its
process regression observes 128 -> 192 -> 256 samples at 48 kHz and shuts down
cleanly. This strengthens dynamic-latency wire evidence without claiming a
third-party plugin measurement or graph compensation.

## All-features workspace requalification (2026-09-08)

The complete workspace regression passed with `cargo test --workspace
--all-features --quiet`, including 20 feature-enabled plugin-worker process
tests and the existing domain, control, engine, DSP, storage, transport, and
Windows-audio suites. This is portable/fixture evidence; native production
driver, signing, and realtime routing gates remain open.

The supervised worker regression now covers both fixture paths: a dynamic
latency response is accepted while the supervisor remains running, and the
two-entry descriptor catalog is returned through the supervised adapter before
clean shutdown.

## Control mutation-bucket retention bound (2026-09-08)

The in-memory mutation limiter now retains at most 256 distinct client buckets.
It evicts buckets idle for more than ten minutes and returns a retryable
one-second limit when all buckets are active, preventing client-ID churn from
growing control memory without weakening per-client burst/refill behavior.
Control tests (91), strict Clippy, formatting, diff checks, and documentation
validation pass. No audio or machine configuration was accessed.

## Recovery-retention directory bound (2026-09-08)

Closed an M07/SEC-12 filesystem-retention gap: recovery-backup pruning now
inspects at most 1,024 direct directory entries and fails closed before any
deletion when the directory exceeds that bound. The existing policy—retain
the ten newest daily backups, preserve pre-migration backups and unrelated
files—remains unchanged for valid directories. Storage tests (77), strict
Clippy, documentation validation, and diff checks pass. No audio or machine
configuration was accessed.

## Workspace requalification after retention hardening (2026-09-08)

`cargo test --workspace --all-features --quiet` passed from `67297d2f` across
the complete workspace, including domain (57), storage (77), control (92),
plugin-host (46 library/20 fixture-process), transport (18), and
Windows-audio (29) coverage plus doc-tests. No audio endpoint, driver, or
machine configuration was changed.

## CLI document-read bound (2026-09-08)

Closed an M01/SEC-12 adapter boundary: CLI API parameters and saved JSON plan
files now use a bounded reader capped at 4 MiB, while session create/import
documents use the existing 1 MiB session-document limit. The same limit is
applied to stdin API calls before JSON parsing, preventing oversized local
input from being allocated without bound. CLI coverage (26), strict Clippy,
formatting, and diff checks pass. No audio or machine configuration was
accessed.

## VST3 bundle binary enumeration bound (2026-09-08)

Closed an M06/SEC-12 plugin-scan gap: VST3 bundle resolution now stops after
the second regular file in `Contents/x86_64-win` and rejects the bundle as
invalid, since exactly one binary is required. It no longer collects an
arbitrarily large malformed bundle directory before rejecting it. Plugin-host
coverage (47), strict Clippy, formatting, and diff checks pass. No plugin was
executed and no audio or machine configuration was accessed.

## Client-enrollment cardinality bound (2026-09-08)

Durable and in-memory client enrollment paths now cap distinct identities at
256. SQLite inventory reads use a one-row overflow sentinel, and new writes
fail closed while updates to existing identities remain valid. Storage/control
tests (73/92), strict Clippy, formatting, diff checks, and documentation
validation pass. No audio or machine configuration was accessed.

Discovery also advertises the enrollment ceiling as
`system.describe.limits.maxClientEnrollments`, with a self-consistency
regression tied to the storage constant.

## Operation-journal cardinality bound (2026-09-08)

Durable idempotency outcomes now have a 4,096-entry ceiling after expiry
pruning. New keys fail closed at capacity, while existing keys continue to
replay without replacement, preserving idempotency semantics. Storage/control
tests (74/92) and strict Clippy pass; no audio or machine configuration was
accessed.

## Safe acceptance requalification after CLI/plugin bounds (2026-09-08)

The complete `tests/acceptance/safe-all.ps1` chain passed at `6db20704` with
exit code 0. It covered native toolchain/compile and read-only endpoint
inventory, disposable pinned SysVAD x64 qualification, workspace and M01/
M04/M05/M06/M07 acceptance, unsigned M08 artifacts, 158-ID traceability, and
documentation validation (51 Markdown files, 160 local links). Temporary
outputs were cleaned; driver installation/loading, signing-mode changes,
plugin/startup registration, and machine audio configuration were excluded.

The same ceiling is advertised as
`system.describe.limits.maxOperationJournalEntries`, with a self-consistency
regression tied to the storage constant.

## Windows SDK/WDK installation and SysVAD qualification (2026-09-08)

The requested Windows SDK is already installed through the native Visual
Studio/WDK toolchain; no additional SDK installer was run. Read-only
verification found SDK/WDK `10.0.28000.0`, MSVC `14.51.36231`, audio and kernel
headers, x64 libraries, and `signtool.exe`. The repository native compile gate
passed with that toolchain.

The authorized disposable SysVAD qualification cloned the pinned Microsoft
driver-samples revision, initialized its pinned WIL submodule, built the x64
sample and APO targets, generated the package/catalog, and passed normal
package/API validation. The checkout and all generated outputs were removed.
This closes the host toolchain/build prerequisite only; it does not establish
AudioRouter driver behavior, target-machine lifecycle, production signing,
installation, or clean uninstall evidence. No driver, plugin, startup
registration, or machine audio configuration changed.

The control-plane audit also confirmed that session-import, startup, and
virtual-device plan maps are already expiry-pruned and capped, while the
operation outcome and plugin inventory caches have independent bounds. No
redundant retention change was made.

## Global session cardinality bound (2026-09-08)

Closed an M01/SEC-12 resource-retention gap: the domain store and both direct
SQLite session-write paths now cap a backend at 128 session records. The
bound applies to new IDs, while replacing an existing ID remains valid; this
prevents empty sessions from bypassing the aggregate node/edge budgets.
Startup hydration uses the same domain bound and therefore fails closed on an
oversized persisted inventory. Discovery advertises
`system.describe.limits.maxSessionsGlobal`. Domain, storage, and control
regressions cover overflow, replacement, journal-atomic writes, and bounded
startup restoration. Domain/storage/control tests (57/76/92), strict Clippy,
full workspace all-features tests, formatting, diff checks, and documentation
validation pass. No audio or machine configuration was accessed.

## Safe acceptance requalification after SDK/WDK verification (2026-09-08)

The complete `tests/acceptance/safe-all.ps1` chain passed at `d9e20682` with
the installed VS2026/SDK/WDK toolchain. It covered native compile and
read-only endpoint inventory, disposable pinned SysVAD x64 compile/package/API
validation, the locked workspace and milestone acceptance suites, M06 SDK/
VST3 qualification, M07 headless checks, unsigned M08 artifact preparation,
158-ID traceability, and documentation validation (51 Markdown files, 160
local links). The run exited 0 and cleaned its temporary outputs. It excluded
driver installation/loading, test-signing mode changes, plugin/startup
registration, and machine audio configuration changes.

## Editor lifecycle policy groundwork (2026-09-08)

Closed a control-plane portion of M06/PLUG-04: `EditorLifecycle` models
open/close/failure and deliberate retry for an optional plugin editor while
holding the processing-generation token unchanged. Focused regressions cover
duplicate operations, failure recovery, and cleanup. This does not create a
native window, execute editor code, or close the Windows/UI-thread acceptance
gate. The next M06 slice is controlled plugin-worker failure and dynamic
latency evidence; native execution remains separately gated.

## Dynamic latency session boundary (2026-09-08)

Closed the M06/PLUG-03/PLUG-05 protocol-state gap: `WorkerSession` now retains
the latest validated latency report, accepts bounded sample-count updates at
the negotiated sample rate, and rejects sample-rate changes without replacing
the previous authoritative value. A handshake/session regression covers the
initial report, dynamic update, rejection, and state preservation. This is
control-plane evidence; actual plugin-added latency measurement and graph
compensation remain open.

The process adapter regression also fixed a failure-reporting hole: a worker's
structured latency failure is now propagated as `WorkerProcessError::Protocol`
instead of being mislabeled as an unexpected response. The nine-test process
worker suite and strict Clippy requalification are the acceptance evidence.

## Graph-plan retention bound (2026-09-08)

Closed an M01/SEC-12 memory-retention gap in `GraphStore`: expired plans are
pruned before new planning, and no more than 100 live graph plans are retained
at once. A typed `planLimitReached` error gives callers a retryable outcome
when all slots are active; restoring persisted plans uses the same bound.
Domain coverage is 55 tests with strict Clippy; no audio or machine
configuration was accessed.

## In-memory graph idempotency retention bound (2026-09-08)

Closed an M01/SEC-12 retention gap in the non-durable `GraphStore`: completed
graph commit results now use a FIFO ledger capped at 100 entries, matching the
existing bounded graph-history policy. Removing a session also removes its
ledger entries; storage-backed control retains durable replay through SQLite.
Domain coverage is 56 tests with strict Clippy; no audio or machine
configuration was accessed.

## Graph idempotency-key boundary (2026-09-08)

Closed an M01/SEC-12 lower-layer validation gap: direct `GraphStore` commits
now reject empty or over-128-byte idempotency keys before lookup or insertion,
matching the control and SQLite boundaries. Domain and control regressions,
workspace tests, and strict Clippy pass; no audio or machine configuration was
accessed.

## RMS-window allocation bound (2026-09-08)

Closed an M02/SEC-12 constructor gap: the public engine `RmsWindow` now
rejects capacities above 480,000 samples, matching the DSP meter's ten-second
maximum at the 48 kHz internal rate, before allocation. The boundary regression
covers zero and oversized capacities; engine coverage is 77 tests with strict
Clippy and no audio or machine configuration was accessed.

## Ephemeral-plan hydration bounds (2026-09-08)

Closed an M03/M07/SEC-12 persistence gap: startup and pending virtual-device
plan hydration now reads at most 101 rows and rejects an oversized inventory
explicitly. This prevents a corrupted database from expanding startup memory
before validation. Storage coverage is 72 tests with strict Clippy; no audio,
driver, or machine configuration was accessed.

## In-memory ephemeral-plan admission bounds (2026-09-08)

Closed the corresponding M01/M03/M07/SEC-12 control-plane gap: startup,
session-import, and virtual-device plan maps now prune expired entries before
admission and cap live entries at the shared 100-plan limit. A control
regression fills each map, verifies deterministic rejection at capacity, and
confirms an expired entry is reclaimed. Control coverage is 89 tests with
strict Clippy; no audio, driver, or machine configuration was accessed.

## Durable ephemeral-plan write bounds (2026-09-08)

Closed the remaining M01/M03/SEC-12 persistence bypass: direct SQLite saves
for startup and virtual-device plans now count live rows before insertion,
allow replacement of an existing ID, ignore expired rows, and reject a live
101st plan. Startup-plan hydration now excludes expired rows, matching virtual
device hydration and preventing stale records from consuming the look-ahead
bound. Storage coverage is 72 tests with strict Clippy; no audio, driver, or
machine configuration was accessed.

## Safe acceptance requalification (2026-09-08)

The complete `tests/acceptance/safe-all.ps1` chain passed at `c438a21` after
the pending-plan persistence fixes. It covered native toolchain discovery and
compile, read-only 34-endpoint format inventory, disposable pinned SysVAD x64
build/package/API validation, M01 CLI, M04 DSP/recording, M05 UI with its
temporary production build, M06 SDK/VST3, M07 headless, unsigned M08 release
preparation, traceability, and documentation. Temporary outputs/checkouts
were removed; no driver was installed or loaded, signing mode or registration
was changed, and no machine audio configuration was touched.

## Live adapter requalification (2026-09-08)

The explicitly authorized guarded live wrappers passed at `3179ad9`: the
unrouted adapter processed 14,880 capture frames and 14,848 scheduler frames
across 116 graph blocks, while the selected VB-Audio route processed 14,400
capture and 14,336 routed frames across 112 blocks. Both runs reported zero
xruns, overruns, deadline misses, and deadline lateness; stream stop/reset and
media/configuration rollback checks passed. This remains shared-mode adapter
evidence, not managed-driver callback, physical-latency, or production-routing
evidence.

## Event replay page-cursor hardening (2026-09-08)

Closed an M07/API-08 replay correctness gap: bounded `events.subscribe` pages
now return the last inspected event sequence when a page is full, rather than
the event-log tail. This lets a client resume at `nextSequence` without
skipping retained events that were outside the first page. The legacy domain
`since` API remains unchanged; control dispatch uses the page-aware API, and
both domain and control regressions cover a 501-event boundary. Control passes
88 tests with strict Clippy; the locked workspace suite passes. No audio or
machine configuration was accessed.

Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to
the managed endpoint-owned production scheduler after driver lifecycle exists.

## VS/WDK and Windows SDK requalification (2026-09-08)

The installed native toolchain is usable: Visual Studio Community 18.0
(MSVC 14.51.36231), Windows SDK/WDK 10.0.28000.0, and the required WDK
properties/tools were discovered read-only. The custom-output native WASAPI
probe compiled successfully and left no repository object. This validates the
SDK/build prerequisite, not driver installation, signing, or production
endpoint routing.

The M06 SDK installer provenance acceptance also passed using disposable Git
metadata and a rejected reparse-point destination. It made no SDK, plugin,
driver, or audio configuration changes.

## Safe acceptance after scheduler lifecycle fix (2026-09-08)

The elevated `tests/acceptance/safe-all.ps1` chain passed at `76e3852` after
the scheduler queue-invalidation change. It covered the native toolchain and
34-endpoint format inventory, disposable SysVAD x64 qualification, M01/M04/
M05/M06/M07, unsigned M08 release preparation, 158-ID traceability, and docs.
Temporary outputs/checkouts were removed; no driver was installed or loaded,
and no signing mode, plugin/startup registration, or machine audio setting was
changed.

## Live Rust adapter requalification (2026-09-08)

The guarded 300 ms Rust adapter smoke passed on the existing endpoints with
14,880 capture frames, 14,848 scheduler frames, 116 graph blocks, zero
xruns/overruns/deadline misses, and an unchanged media snapshot. The explicit
VB-Audio routed smoke also passed with 14,400 capture frames and 14,336
scheduler/routed frames, zero deadline misses/lateness, and unchanged defaults,
volume, mute, privacy, driver, signing, and startup state. Streams were
stopped/reset and temporary outputs removed. This is shared-mode adapter
evidence, not managed-driver callback, physical-latency, or production-driver
evidence.

## Virtual-bus persistence read bound (2026-09-08)

Closed an M03/SEC-12 storage gap: virtual-bus hydration now reads at most the
eight-bus domain capacity plus one look-ahead row and rejects an oversized
inventory explicitly. A corrupt database can no longer cause an unbounded
virtual-bus read before domain validation. Storage coverage is 70 tests with
strict Clippy; no driver, endpoint, or machine configuration was accessed.

## WASAPI teardown hardening (2026-09-08)

Hardened `SharedCapture`, `SharedRender`, and process-loopback teardown so
`Reset` is attempted even when `Stop` reports an error, while local started
state is cleared before the COM call. This preserves best-effort cleanup and
prevents repeated stop attempts against a failed client. Windows-audio tests
(29) and strict Clippy pass; guarded live adapter and routed runs also passed
with unchanged media state. No persistent audio configuration was changed.

## Scheduler lifecycle queue invalidation (2026-09-08)

Closed an M02/ARCH-04 generation-boundary gap in `RealtimeScheduler`: graph
publication, successful session activation, and deactivation now recycle both
queued input and output blocks. Pending input from a prior graph can no longer
be processed by a replacement graph after a lifecycle transition. Engine
coverage is 77 tests with strict Clippy; the full workspace gate remains the
next verification. No audio endpoint or machine configuration was accessed.

## UI inventory cursor consumption (2026-09-08)

Closed an M05/API parity gap in the live UI backend: recordings, sessions,
devices, and managed virtual devices now follow every bounded `nextCursor`
page instead of returning only the first page. The adapter accepts the legacy
array response, rejects malformed pages and non-advancing cursors, and caps
the number of pages at 10,000. UI tests (91), TypeScript typecheck,
and a temporary elevated Vite production build passed. The original build
attempt was blocked only by the host's locked `ui/dist`; no existing output
was removed and no audio or machine configuration changed.

## Safe acceptance requalification at UI-pagination tip (2026-09-08)

The complete `tests/acceptance/safe-all.ps1` chain passed at pushed tip
`b120b87`: M00 toolchain/native compile/format inventory, disposable pinned
SysVAD x64 qualification, M01 CLI, M04 DSP/recording, M05 UI (91 tests and
temporary production build), M06 SDK/VST3, M07 headless, unsigned M08
artifacts, 158-ID traceability, and documentation validation (51 Markdown
files, 160 local links). Temporary outputs/checkouts were removed. Driver
installation/loading, signing-mode changes, plugin/startup registration, and
machine audio configuration were excluded and unchanged.

## M06 fixture-gate inventory (2026-09-08)

A read-only inventory of the standard system and user VST3 locations found no
existing `.vst3` bundles beyond the repository-local mda fixture. The M06
acceptance requirement for three compatible x64 effects from at least two
vendors therefore remains unverified. No plugin was downloaded, installed,
registered, loaded, or executed; no compatibility claim was synthesized.

## M06 plugin-state inventory hardening (2026-09-08)

The SQLite plugin-state listing boundary now probes one row beyond its shared
500-item limit and rejects an oversized result explicitly, preventing an
unbounded legacy read while preserving per-state validation. Storage (69) and
control (87) tests, strict package Clippy, formatting, and diff checks passed.
No plugin was executed and no audio or machine configuration was accessed.

## M06 failed-worker cleanup hardening (2026-09-08)

Supervised worker heartbeat, processing, latency, and externally reported
failures now terminate the failed child immediately instead of retaining it
until an incidental drop. Shutdown also handles an already-terminated child
without writing to a closed pipe. Plugin-host (39) and worker-process (8)
tests, strict Clippy, formatting, and diff checks passed. This is process
containment evidence only; it does not claim third-party plugin execution or
full OS filesystem/network sandboxing.

## Current actionable handoff (2026-09-08)

The latest validated implementation head is `96b5ba1b` on `main`. The guarded
M00–M08 safe-chain evidence at that head is recorded above and in [M08 release
evidence](evidence/M08-release.md). The current head includes the Windows x64
VST2 modern/legacy worker boundary, negotiated-rate built-in graph activation,
and rate-aware adapter deadline reporting; the complete chain requalified
after those changes. Its event-replay cursor, Windows SDK/toolchain, and
SDK-installer provenance checks are recorded above.
The read-only M06 fixture inventory found no additional system/user VST3
bundles, so the loader matrix now covers six effects across two vendors. The
official ChowMatrix validator still has two fixture-specific failures, and
worker/editor containment remains explicitly open. Remaining release gates
include production-driver ownership/signing, native-shell HWND authorization,
independent rights-cleared plugin coverage, installer/clean-machine evidence,
physical latency, and manual UI acceptance.
Safe portable and
adapter work through the PCM16 quantum bridge, event-driven process-loopback,
bounded scheduler integration, explicit 44.1 kHz-to-48 kHz conversion,
scheduler deadline telemetry, and conservative p99.9 histogram bounds is
implemented and regression-tested. The full locked workspace suite, guarded
include/exclude process-loopback acceptance, and safe acceptance chain pass
with unchanged media state.

The latest acceptance-harness correction is now validated: a five-second
shared soak that previously exposed the invalid p99.9-versus-maximum check
passes after the fix, as does the routed wrapper. Histogram accounting,
sample counts, and totals remain enforced; no persistent audio configuration
changed.

Requalified the plugin-host containment boundary on 2026-09-08: 42 unit tests
plus doc-tests and 12 worker-process integration tests passed with strict
Clippy. Worker lifetime termination, handshake identity, quarantine/heartbeat,
bounded frames/deadlines, shared-memory guards, state integrity, scanner
limits, editor lifecycle policy, dynamic latency retention, and structured
latency-failure propagation remain covered. Actual third-party plugin
execution and full OS filesystem/network sandboxing remain open.

The worker protocol now also bounds failure-code payloads to 128 bytes at both
encode and decode validation, preventing an untrusted worker from turning an
error report into a large retained diagnostic. The focused protocol regression
and workspace validation remain required evidence for this boundary.

The feature-gated worker fixture now provides deterministic `crash`, `hang`,
and `invalid-output` modes. Thirteen process tests pass with the feature
enabled: crash reaping, bounded hang kill, malformed-output rejection, normal
IPC, and supervisor containment are all exercised. This closes the controlled
failure fixture portion of M06 evidence; it does not claim third-party plugin
loading or full OS filesystem/network sandboxing.

Added `PluginIdentity::verify_current` for the discovery-to-launch boundary.
It rechecks canonical root containment and the selected binary's format,
architecture, size, and SHA-256, rejecting replacement or outside-root content
without rebinding. Two regressions cover changed bytes and a changed grant;
this is launch authorization evidence, not plugin execution evidence.

Added bounded opaque state messages (`StateRestore`, `StateSave`, and `State`)
to the worker protocol. Assets are limited to 512 KiB and must pass SHA-256
integrity validation; the disposable worker round-trips one asset through the
process API. Plugin-host coverage is now 45 library tests, 12 ordinary worker
tests, and 16 feature-enabled worker tests. Vendor-specific VST3 state
serialization remains a native-host gate.

Both worker wrappers now expose version-aware restore helpers. They verify the
versioned asset locally before IPC and return a typed `StateError::VersionMismatch`
without killing or mutating a running worker; the process regression proves the
rejection path and subsequent worker usability.

Added typed `DescribeParameters`/`Parameters` worker messages with bounded
descriptor validation: at most 256 unique IDs, finite normalized ranges, and
128-byte titles. The disposable worker returns an explicit empty catalog;
native plugin parameter discovery remains open. Plugin-host coverage is now 46
library tests, 13 ordinary worker tests, and 17 feature-enabled worker tests.

Aligned the plugin-host state contract with the storage boundary by rejecting
version `0` during asset construction, restore verification, and worker-wire
validation. The existing version/integrity regression now covers this invalid
version at both layers.

`SupervisedWorkerProcess::spawn_verified` now composes that identity check with
supervised worker creation. A process regression copied a temporary executable
as a VST3 fixture, scanned it, launched through the verified path, processed one
frame, and cleaned the fixture; 12 ordinary worker-process tests and 44 library tests
passed. This closes the stale-scan launch seam, while native third-party VST3
execution and full OS sandboxing remain open.

The companion regressions prove a missing identity is rejected before process
creation and unavailable state fails closed. The ordinary process suite now has
13 passing tests, and the feature-enabled suite has 17; no child is spawned for
the missing-plugin case.

The supervised wrapper now also round-trips versioned opaque state and proves a
version mismatch leaves the processing worker running. This keeps state control
operations independent from processing-generation restart behavior.

Requalified M07 headless acceptance on 2026-09-08: 25 CLI tests, 2 MCP
interoperability tests, 87 control tests, 39 plugin-host tests, and 8
worker-process tests passed with doc-tests and strict Clippy. Durable plans,
backup/restore, recovery, privacy, authorization, and worker boundaries were
exercised without audio or machine-configuration access.

Requalified M05 UI acceptance on 2026-09-08: TypeScript typecheck, 14 Vitest
files with 91 tests, and a temporary three-file production build passed. The
portable UI surface remains validated; native shell packaging and manual
visual/accessibility acceptance remain open.

Requalified M06 SDK acceptance elevated after diagnosing an unprivileged
MSBuild FileTracker `E_ACCESSDENIED` in the existing build tree. The elevated
run passed 51 SDK self-tests, 1,598 validator tests, 68 classes, finite stereo
processing, five-parameter automation, and a 180-byte state round trip. It
made no global plugin or audio-configuration changes.

Added a source-level engine regression for the quantile-tail case that exposed
the adapter harness defect: p99.9 may be below a rare absolute maximum. The
engine suite now passes 76 tests plus doc-tests and strict Clippy, with the
quantile contract covered at its source.

The complete elevated safe acceptance chain was requalified at pushed tree
`9d6b267` on 2026-09-08 after M04 acceptance evidence was recorded. Native
compile/inventory, disposable pinned SysVAD x64 qualification, M01/M04/M05/
M06/M07, unsigned M08 preparation, and documentation validation passed; all
temporary outputs/checkouts were removed and machine audio state was unchanged.

Requalified M00 controlled process-tree attribution on 2026-09-08 with the
native live wrapper: a disposable child route captured 33,075 frames and
121,582 nonzero payload bytes over 750 ms, then stopped and released cleanly
with persistent audio configuration unchanged. PID reuse, physical latency,
managed-driver callback timing, and production isolation remain open.

The paired process-tree exclusion wrapper also passed on 2026-09-08 with
33,075 captured frames over 750 ms and unchanged persistent audio state. The
result validates exclusion-mode lifecycle only; quantitative isolation and
PID-reuse evidence remain open.

Requalified the native event-driven lifecycle on 2026-09-08 using the selected
VB-Audio pair: 24,480 capture frames and 28,320 silent render frames over
500 ms, with clean teardown and unchanged defaults, volume, mute, privacy,
driver, signing, and startup state. This remains shared-mode lifecycle
evidence, not managed-driver callback or latency proof.

Requalified the bounded digital impulse return path on 2026-09-08 using the
explicit VB-Audio pair: 97/100 impulse groups detected, zero p95 spacing error,
and 69.13 ms estimated digital onset. Temporary artifacts were cleaned up and
persistent audio state was unchanged. This is digital correlation only, not
calibrated acoustic latency or production-driver callback evidence.

Requalified the Rust asynchronous process-loopback bridge on 2026-09-08 in
both include and exclude modes for 500 ms. The runs produced 182 and 187
scheduler quanta respectively, with explicit 44.1 kHz-to-48 kHz conversion,
zero rejected packets, xruns, or buffer overruns/underruns, clean stream
teardown, and unchanged persistent media configuration. This remains user-mode
process-loopback evidence, not managed-driver callback or physical latency.

The complete Windows-audio crate regression target also passed on 2026-09-08:
29 tests plus doc-tests covering endpoint/format binding, snapshot diffs,
bounded recovery, process-loopback limits and telemetry, read-only application
inventory, and restart identity matching. No device configuration changed;
managed-driver callback timing and physical latency remain open.

The guarded routed adapter was also qualified for 2,000 ms on the explicit
VB-Audio pair: 96,480 captured frames, 96,384 scheduler frames, 95,904 routed
frames, 753 graph blocks, 32,768 ns processing p99.9, and zero deadline misses
or lateness. Stream cleanup and all endpoint/configuration snapshots passed;
this remains user-space adapter evidence, not managed-driver callback proof.

The matching shared capture/render adapter run also passed for 2,000 ms:
96,480 capture frames, 96,384 scheduler frames, 97,056 silent render frames,
753 graph blocks, 65,536 ns processing p99.9, and zero xruns, overruns,
deadline misses, or lateness. Stream teardown and media identity/state checks
passed; this remains user-space evidence, not managed-driver callback proof.

Fixed a real M02 acceptance-harness defect: shared and routed wrappers no
longer reject a valid p99.9 histogram merely because an absolute tail maximum
exceeds the quantile bucket. Totals and histogram accounting remain enforced.
PowerShell parsing, a five-second shared soak, and a two-second routed run
passed with zero xruns/overruns/deadlines and unchanged audio configuration.

Requalified the native tone/loopback signal path on 2026-09-08 using the
explicit VB-Audio pair: 216,970 nonzero capture payload bytes over a 1,000 ms
capture while the 1,500 ms render tone completed successfully. Streams were
stopped/reset and all persistent audio snapshots were unchanged. This is
digital propagation evidence only, not calibrated physical latency or
production-driver callback evidence.

The complete safe acceptance chain was requalified at `cbf52f4` on
2026-09-08 after backup write-path enforcement: native compile and 34-endpoint
read-only inventory, disposable pinned SysVAD x64 qualification, M01/M04/M05/
M06/M07 validation, unsigned M08 preparation, and documentation acceptance
(51 Markdown files, 160 local links) all passed. Temporary outputs/checkouts
were removed; no driver, signing mode, plugin/startup registration, or machine
audio configuration changed.

The safe chain was requalified again at the pushed tree `3d01a6e` on
2026-09-08 after the discovery self-consistency regression. Evidence is in
[M08 release evidence](evidence/M08-release.md); temporary outputs and
checkouts were removed and machine audio state was unchanged.

Completed safe slice M04/DSP-01/API-01 preset contract versioning: built-in
voice-chain and EQ catalog entries now carry an explicit version in the DSP
registry, control schemas/responses, TypeScript contract, CLI assertion, and
API reference. Verification and rollback are limited to portable catalog
metadata; no graph, audio, or machine state is touched.

The preset payload slice is also complete: `EqPreset` and `VoiceChainConfig`
retain the same version after DSP lookup/instantiation, with engine and DSP
struct-construction regressions. This remains portable metadata and does not
activate or mutate an audio graph.

Unsupported preset payload versions now fail closed in `ParametricEq` and
`VoiceChain` constructors before processor state is created, with focused
regressions for both paths.

M04 DSP/recording acceptance was requalified on 2026-09-08: 27 DSP tests and
30 recording tests passed, including signal transfer, finite-output repair,
long-duration pitch, WAV/FLAC, bounded queues, metadata, checkpoint/recovery,
worker failure, pause/split, and library behavior. Formatting, strict package
Clippy, and diff checks passed. This remains portable evidence; native realtime
recorder integration and W1 hardware timing are still open.

Removed the stale unused M04-unavailable processor value and corrected the UI
fixture that repeated its obsolete reason. Control tests (87), strict Clippy,
focused UI tests (4), TypeScript typecheck, and diff checks passed; no audio or
machine configuration was accessed.

Completed safe contract slice M06/M07/SEC-12 plugin-size discoverability:
the enforced 256 MiB plugin binary ceiling is exposed consistently by
the `plugins.scan`, `plugins.list`, and `plugins.inspect` output schemas.

Completed safe storage slice M06/SEC-12 plugin-state identity symmetry:
direct plugin-state writes and filtered reads enforce the same bounded plugin
identifier contract already used by state metadata consumers.

Completed safe storage slice M06/SEC-12 plugin-state read-boundary validation:
plugin-state list hydration now revalidates stored IDs, plugin identity, state
hashes, versions, absolute paths, size limits, and reparse-point ancestry
before exposing opaque metadata to plugin consumers. A corrupt-row regression
fails closed; storage/control tests (60/87), strict Clippy, formatting, and
diff checks pass. No plugin was executed and no audio or machine configuration
was accessed.

Completed safe storage slice M07/STATE-12 backup-budget alignment: the
configuration backup ceiling now matches the specification's documented
100 MiB budget, and newly created backup outputs are size-checked and removed
when oversized. The focused storage regression covers the write-path cleanup.

Completed safe storage slice M03/M07/SEC-12 virtual-device commit-boundary
validation: the durable virtual-bus commit helpers now validate plan IDs and
idempotency keys before changing desired state or writing journal rows. The
focused storage regression covers both invalid-key forms and the oversized
plan identifier. The same commit path now prunes expired journal outcomes
before insertion, with runtime-retention coverage.

Completed safe storage slice M01/SEC-12 enrollment read-boundary validation:
client-enrollment lookup and list operations now revalidate persisted client
IDs and roles, failing closed if a legacy or corrupted database row bypasses
the write-side checks. Storage/control tests and strict Clippy pass.

Completed safe storage slice M01/SEC-12 session read-boundary validation:
session lookup, history, and paged-list reads now apply the same domain and
1 MiB document validation used by writes, rejecting corrupted persisted
documents instead of returning them as authoritative state. Storage/control
tests and strict Clippy pass.

Completed safe storage slice M01/SEC-12 graph-plan integrity validation:
graph-plan writes and hydration now require the plan session ID to match the
candidate session, reject negative persisted revisions, and validate the
hydrated candidate and document budget before exposing a plan.

Completed safe storage slice M01/SEC-12 session identity hydration validation:
session lookup, history, and paged-list reads now verify the persisted SQLite
row key matches the deserialized session ID, preventing identity substitution
through a relocated or corrupted row.

Completed safe storage slice M04/REC-04/SEC-12 recording read-boundary
validation: recording list and paged-list hydration now reapply the shared
identity, format, channel/rate, and metadata checks used by writes, rejecting
corrupt persisted rows before they reach control or UI consumers.

Completed safe control slice M01/STATE-02/SEC-12 durable-startup failure
handling: storage-backed control initialization now has a fallible
`try_with_storage` path and the compatibility constructor fails closed instead
of silently replacing unreadable persisted sessions, plans, or privacy state
with defaults. Backend-epoch claiming is deferred until all persisted state
has hydrated successfully, so failed initialization performs no durable write.

Completed safe control slice M01/SEC-12 startup session completeness:
storage-backed initialization now restores all persisted sessions by walking
bounded stable-cursor pages instead of truncating at the former 128-row
bootstrap query. A 129-session regression covers the boundary.

Completed safe storage slice M07/SEC-12 journal read/write-boundary
validation: operation status and idempotency replay now revalidate operation
names, result-size bounds, and nonnegative persisted revisions before exposing
records. Direct journal writes enforce the same operation/result contract, and
a corrupt-row regression fails closed. Storage/control tests (61/87), strict
Clippy, formatting, and diff checks pass; no audio or machine configuration
was accessed.

Completed safe storage slice M04/REC-04/SEC-12 recording counter-boundary
validation: SQLite recording frame and file-byte counters now reject negative
read values and values outside SQLite's signed range on writes, preventing
unchecked signed-to-unsigned wrapping. Recording/storage/control tests
(10/62/87), strict Clippy, formatting, and diff checks pass; no recording file
or audio/machine configuration was accessed.

Completed safe storage slice M01/SEC-12 SQLite revision-boundary validation:
session revisions, graph-plan base revisions, and history cursors now reject
`u64` values that cannot be represented by SQLite's signed integer type before
query or mutation. A boundary regression covers all three paths; storage
(63), control (87), strict Clippy, formatting, and diff checks pass. No audio
or machine configuration was accessed.

Completed safe storage slice M06/SEC-12 plugin-state numeric read validation:
persisted plugin-state versions now use checked signed-to-unsigned decoding,
so negative SQLite values fail closed instead of wrapping to a valid-looking
version. The focused regression and strict storage Clippy pass; no plugin was
executed and no audio or machine configuration was accessed.

Completed safe storage slice M01/SEC-12 persisted session revision integrity:
session lookup, history, and stable-cursor list hydration now compare the
SQLite revision column with the validated document revision and reject
mismatches or negative values. Storage/control tests (65/87), strict Clippy,
formatting, and diff checks pass; no audio or machine configuration was
accessed.

## Plugin inventory cache bound (2026-09-08)

Closed an M06/M07/SEC-12 control-memory gap: the in-memory `plugins.scan`
inventory cache now retains at most 64 distinct scan roots in FIFO order,
while rescanning an existing root preserves its entry. The newest inventory
remains available through `plugins.list`, and eviction is isolated to the
non-durable cache; plugin files are still never loaded or executed by this
path. Control coverage is 90 tests with strict Clippy.

Completed safe storage slice M01/SEC-12 SQLite count conversion validation:
session and recovery count reads now reject negative SQLite results before
conversion to `usize`, preventing malformed persistence data from wrapping
into an unbounded count. The focused storage regression and strict Clippy pass;
no audio or machine configuration was accessed.

Completed safe storage slice M06/SEC-12 plugin-state size decoding validation:
persisted plugin-state `size_bytes` values now use checked signed-to-unsigned
decoding, so negative legacy values fail before metadata reaches plugin
consumers. The focused storage regression and strict Clippy pass; no plugin was
executed and no audio or machine configuration was accessed.

Completed safe storage slice M07/SEC-12 journal revision write validation:
durable journal writes now convert revisions to SQLite's signed integer type
before validation and insertion, explicitly rejecting values above
`i64::MAX`. The focused storage regression and strict Clippy pass; no audio or
machine configuration was accessed.

Completed safe storage slice M07/SEC-12 journal request-hash validation:
direct journal, transactional session, virtual-bus, and checked replay paths
now cap request-hash strings at 128 bytes while preserving legacy empty hashes.
Oversized values fail before SQLite mutation or lookup; focused storage/control
tests and strict Clippy pass with no audio or machine configuration accessed.

Requalified M06/SEC-07 native SDK and offline loader acceptance on 2026-09-08:
the pinned local SDK passed 51 self-tests and 1,598 official validator tests;
the x64 loader discovered 68 classes and verified finite stereo processing,
five-parameter automation, and a 180-byte state round trip. No plugin was
registered and no audio or machine configuration changed.

Requalified M08 unsigned release preparation on 2026-09-08: optimized locked
CLI/plugin-worker builds, UI production archive, Cargo/npm SBOMs, notices,
provenance, required-artifact checks, and SHA-256/byte-count verification all
passed in a disposable directory that was cleaned afterward. No installer,
driver, signing action, plugin registration, or audio configuration changed.

Corrected M05/UI capability wording: the library's Recorder entry now points
users to the implemented dedicated Recorder panel instead of claiming that M04
runtime integration is unavailable. A focused UI regression covers this
guidance; no backend, audio, or machine configuration changed.

Revalidated the full locked workspace at the current head on 2026-09-08: 466
unit/integration tests passed across CLI/MCP, control, domain, DSP, engine,
plugin-host/worker, protocol, recording, storage, transport, and Windows-audio
crates, with all doc-tests, strict workspace Clippy, formatting, and diff
checks passing. No driver, signing mode, plugin/startup registration, or
machine audio configuration changed.

Requalified M07 headless acceptance at the current tree: M01 CLI acceptance,
25 CLI tests, two MCP interoperability tests, 87 control tests, 39 plugin-host
tests, eight worker-process tests, strict Clippy, and formatting passed. No
audio device, driver, plugin registration, or machine configuration changed.

Requalified M01 CLI acceptance on 2026-09-08: offline discovery, graph/session
lifecycle, recording operations, validated bundle import/export, backup/restore,
and authorization checks passed with temporary cleanup. No user database,
audio endpoint, driver, or machine configuration was accessed.

Requalified the complete safe acceptance chain after the SDK check on
2026-09-08: native compile/inventory, disposable SysVAD x64 package/API
qualification, M01/M04/M05/M06/M07 acceptance, unsigned M08 preparation, and
documentation validation passed. Temporary outputs/checkouts were removed.
Production driver, signing, installer, clean-machine, physical-latency,
managed callback, and manual UI gates remain open.

Revalidated the locked workspace after the SDK/setup documentation update:
466 unit/integration tests, all workspace doc-tests, strict workspace Clippy
with `-D warnings`, and the documentation acceptance passed. No driver,
signing mode, plugin/startup registration, or machine audio configuration
changed.

Added and validated `tests/acceptance/m00-toolchain.ps1` for M00/ARCH-12:
it read-only verifies Visual Studio/MSVC, Windows SDK headers/libraries, WDK
build properties, `stampinf`, and the shared `28000` kit line. The verifier
passed on the installed host; the first full-chain rerun reached M08 and
stopped because release preparation correctly requires a clean Git tree while
this new acceptance script was uncommitted. No product or machine state was
changed.

The clean rerun then passed with the toolchain verifier included: native
compile/inventory, disposable SysVAD x64 package/API qualification, M01/M04/
M05/M06/M07 acceptance, unsigned M08 preparation, and documentation validation.
Temporary outputs/checkouts were removed; no driver, signing mode, plugin or
startup registration, or machine audio configuration changed.

Added and validated `tests/acceptance/m08-traceability.ps1`: its range-aware
parser extracts 158 normative requirement IDs from the specification and proves
each is represented in the delivery traceability table. It reports coverage
only; it does not promote unverified implementation, hardware, driver,
signing, installer, or usability gates.

The clean safe chain was requalified with both M00/M08 guards enabled:
toolchain compatibility, native compile/inventory, disposable SysVAD x64
qualification, M01/M04/M05/M06/M07 acceptance, unsigned M08 preparation, 158-ID
traceability, and documentation validation all passed. Temporary outputs and
checkouts were removed; native driver lifecycle, signing, installer, hardware,
physical latency, and manual UI gates remain open.

Completed M04/M07/SEC-09/SEC-12 direct recording-list hardening: storage now
uses the shared 500-item authority, probes one extra row, and fails explicitly
with a cursor-pagination error instead of collecting an unbounded legacy list.
Single-record lookup now queries by identity directly, so it remains usable for
larger libraries. Storage (68) and control (87) tests, strict Clippy, formatting,
and diff checks passed; rollback is isolated to the storage query boundary and
its regression.

The next actionable item is native callback deadline/period evidence only when
the production-style native scheduler owns an endpoint stream. That gate is
not satisfied by the current process-loopback diagnostic or portable rings.
Managed virtual-driver provisioning, production signing/Secure Boot/HVCI,
installer/uninstaller, physical latency, clean-machine/reboot identity, and
manual Discord/OBS/UI accessibility acceptance remain open. No driver,
signing mode, endpoint default, volume, mute, privacy, plugin registration, or
startup registration may be changed as part of the current safe work.

The checked-in native probe build script was revalidated on 2026-09-06 with
the installed Visual Studio Community 2026 MSVC and Windows SDK/WDK toolchain.
This verification compiled only; it did not execute the probe or touch audio
configuration. Generated build outputs were removed afterward.

Rechecked the SDK prerequisite on 2026-09-08 after the user reported that the
reference repository was not an installer: the machine already contains the
matching Windows SDK/WDK `10.0.28000.0` headers, libraries, binaries, and WDK
build properties. The official Microsoft `10.1.28000.2705` bootstrapper was
downloaded to the user temp directory, but its unattended setup remained
resident without adding a kit directory, so only the two processes launched by
this check were stopped and the temporary bootstrapper was removed. Existing
SDK/WDK state was not modified; no driver or audio configuration was touched.

## M00 execution log

### M00 working scope

- Objective: establish Windows feasibility evidence and driver/toolchain decisions before M01.
- Requirement IDs: CAP-01–08, ARCH-05/07/08, VDEV-02/09, NFR-01–03, and ENG-03/04.
- Completed in this pass: native machine/OS/toolchain/device inventory and evidence record.
- Remaining checklist: physical loopback latency; process restart/PID-reuse runtime evidence; managed-driver production integration/signing evaluation; live shell injection/manual UI acceptance. DEC-03/06/07 are now synchronized with the available evidence: the React/TypeScript/Vite stack is confirmed, the 48 kHz/128-frame graph baseline remains provisional pending physical timing, and project-owned SysVAD evaluation is selected while production driver/signing remains blocked. Endpoint enumeration, shared capture, process-loopback include/exclude data paths, and controlled process-tree tone attribution now have native evidence.
- Rollback: documentation-only changes can be reverted; no system state was changed.

## M01 continuation scope

- Objective: begin the headless domain/contracts foundation permitted while M00's Windows capture and managed-driver gates remain blocked.
- Requirement IDs: ARCH-01/03/06/10/12, GRAPH-01/02/03/05/06/07/08/09/12/13/14, API-01/02/03/04/05/06/07/08/09/10/11/12, AUTO-01/02/03/04/05/09/10/11/12, STATE-01â€“07/12, SEC-01â€“04/09/10/12, ENG-01/02/04.
- Ordered checklist: establish a pinned Rust workspace; define domain IDs/session/node/edge contracts; implement bounded graph validation with path-specific errors; add deterministic fake runtime and tests; add machine-readable schema/fixture foundation; document real-audio capability as unavailable until M02.
- Validation matrix: portable `cargo fmt`, `cargo check`, and `cargo test`; no M01 test is allowed to claim Windows audio, driver, process-loopback, or physical-latency evidence.
- Rollback: new portable crate/files can be reverted without touching Windows configuration, audio endpoints, drivers, or user data.

### 2026-09-05 — M01 domain foundation

- Added the pinned workspace and `crates/domain` portable crate. It contains opaque entity IDs, session/node/port/edge contracts, graph limits, direction/channel/matrix checks, dangling/duplicate/multiple-input checks, and cycle detection with field-path errors.
- Added `FakeRuntime` with prepare/start/stop lifecycle, idempotent start, generation identity, and failed-prepare behavior. It never opens audio devices and cannot satisfy M02 Windows evidence.
- Checks: `cargo fmt --all`, `cargo test -p audiorouter-domain` — 6 tests passed; `git diff --check` passed. A dependency-free implementation was used because crates.io access was unavailable in this environment; JSON/schema derives remain a subsequent M01 task when dependencies can be supplied reproducibly.
- Evidence boundary: these are portable domain tests only. They do not validate WASAPI, driver, process-loopback, real-time timing, or physical audio.
- Added the M01 node registry with stable type names/versions, realtime cost classes, and explicit availability. Graph-safe nodes are marked available; physical/application/loopback nodes report `requires M02 Windows audio adapters` rather than pretending to work. The root workspace explicitly excludes the standalone Windows probe so its independent lockfile remains valid.
- Checks: `cargo fmt --all -- --check`, `cargo test -p audiorouter-domain` — 7 tests passed; `cargo check --manifest-path tools/m00-wasapi-probe/Cargo.toml` passed; `git diff --check` passed.
- Added an in-memory `GraphStore` transaction foundation: complete-candidate validation, base-revision checks, plan IDs, atomic revision increment, stale-commit rejection, and idempotency-key replay. This is a portable M01 proof layer; SQLite, named-pipe authorization, and JSON-RPC remain subsequent slices.
- Checks: `cargo fmt --all`, `cargo test -p audiorouter-domain` — 8 tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Restored pinned `serde`/`serde_json` contract dependencies once the approved build path supplied the crates. Domain structs now serialize using camelCase API fields, and `tests/fixtures/valid-session.json` is a checked-in valid contract fixture.
- Checks: `cargo fmt --all`, `cargo test -p audiorouter-domain` — 10 tests passed including JSON round-trip and fixture validation; standalone WASAPI `cargo check` and `git diff --check` passed.
- Added authoritative method-discovery metadata for the initial API surface, including permission scopes and side-effect classes (`readOnly`, `planOnly`, `mutating`, `externalOperation`). This keeps CLI/MCP/UI adapters aligned with the backend contract.
- Checks: `cargo fmt --all`, `cargo test -p audiorouter-domain` — 11 tests passed; `git diff --check` passed.
- Added `crates/control`, a portable control-plane façade over the domain store. `system.describe`-style output includes protocol/schema versions, build, method permissions/side effects, node availability, and limits; session reads and graph plan/commit use the same authority.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 2 control tests and 11 domain tests passed; standalone WASAPI `cargo check` and `git diff --check` passed. No transport, Windows audio, driver, or durable SQLite behavior is claimed yet.
- Added `crates/protocol` with 4-byte little-endian length framing, a 4 MiB maximum frame, malformed-frame errors, and JSON-RPC request/response contracts. This is transport-independent framing; named-pipe ACLs remain Windows-only work.
- Checks: `cargo fmt --all`, `cargo test -p audiorouter-protocol` — 3 tests passed; `git diff --check` passed.
- Extended the protocol boundary with JSON-RPC message parsing: version/method validation, explicit notification detection, non-empty batches, and the 32-request maximum required by API-01/API-10/AUTO-02.
- Checks: `cargo fmt --all`, `cargo test -p audiorouter-protocol` — 5 tests passed; `git diff --check` passed.
- Extended `crates/control` with JSON-RPC dispatch over the shared authority: discovery/status/list reads, graph.plan/graph.commit parameter parsing, unknown-method and invalid-parameter errors, batch dispatch, and rejection of mutating notifications.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 4 control, 11 domain, and 5 protocol tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Added `crates/storage` with SQLite schema migration, session document persistence, and an idempotent operation journal. The schema records migration version, session revision/document, operation key/result/revision, and timestamps. Tests use only an in-memory database.
- Checks: `cargo fmt --all`, `cargo test -p audiorouter-storage` — 2 tests passed; no user database path was opened or modified.
- Added fake session lifecycle operations to `crates/control`: idempotent start while running, generation advancement only after stop/restart, explicit fake-runtime labeling, and stable missing-session/parameter errors. A test caught and fixed repeated-start generation churn.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 6 control, 11 domain, 5 protocol, and 2 storage tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Connected the control plane to SQLite through an explicit storage-backed constructor. Session inserts and graph commits now persist through the same control authority; the default in-memory constructor remains available for deterministic tests and no implicit user database path is opened.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 7 control, 11 domain, 5 protocol, and 2 storage tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Connected protocol framing to control dispatch with `decode_rpc_frame` and `dispatch_frame`; framed JSON-RPC requests now produce framed responses through one portable end-to-end path.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 8 control, 11 domain, 5 protocol, and 2 storage tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Added scoped `ClientGrant` authorization to `crates/control`, using the same method permission metadata exposed by discovery. Read-only access succeeds; graph/session mutations are denied before dispatch with stable permission errors.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 10 control, 11 domain, 5 protocol, and 2 storage tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Hardened graph plans with a five-minute default expiry and a testable TTL override. Expired plans fail before session mutation, while idempotency and revision checks remain unchanged.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 10 control, 12 domain, 5 protocol, and 2 storage tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Added `crates/cli`, an offline M01 command surface for `help`, `status`, `schema`, `devices list`, `apps list`, `nodes types`, and `api methods`, with `--json` output and human-readable output. It reports fake/unavailable audio explicitly and shares control-plane discovery rather than inventing device results.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 3 CLI, 10 control, 12 domain, 5 protocol, and 2 storage tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Added `tests/acceptance/m01-cli.ps1`, a checked-in PowerShell acceptance script for schema, status, device-list, and node-type discovery. It verifies offline M01 behavior and refuses success-shaped fake device results.
- Corrected JSON-RPC notification semantics in control dispatch: read-only notifications are consumed without response envelopes, while mutating notifications remain rejected. This prevents clients from receiving misleading success responses for notifications.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 3 CLI, 11 control, 12 domain, 5 protocol, and 2 storage tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Added the [M01 execution evidence report](evidence/M01-contracts.md), mapping implemented crates, commands, test counts, supported requirement slices, and explicit untested Windows/security/audio boundaries. The next task is the authorized Windows named-pipe/authentication boundary; no HTTP transport is planned.
- Added `crates/transport`, a Windows-native named-pipe prototype over the existing bounded protocol framing. It validates local pipe names, rejects remote clients, handles partial I/O, and flushes before disconnecting. The native round-trip test initially exposed a real disconnect race and now passes after the flush fix. Evidence is in [M01 native transport](evidence/M01-native-transport.md).
- Checks: `cargo test --workspace` — all 35 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. This is transport-only evidence: no audio streams, defaults, drivers, or system configuration were touched. The default pipe security descriptor is not production authentication.
- Connected the native pipe test harness to `ControlPlane::dispatch_frame`; a framed `system.describe` request now crosses the actual Windows pipe and returns the control-plane discovery contract. This remains a test harness, not a production daemon or authentication implementation.
- Checks: `cargo test --workspace` — all 36 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.
- Added a native `GetNamedPipeClientProcessId` peer-identity primitive and verified it in the Windows round-trip test. The API documents that a process ID alone is not authentication; token/SID validation and an explicit pipe ACL remain required before sensitive operations.
- Checks: `cargo test --workspace` — all 37 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.
- Added read-only same-user token validation to the native transport: the connected client PID is opened with limited query access, both user SIDs are read with `GetTokenInformation`, and `EqualSid` is used for comparison. The helper is deliberately not wired as production authorization until an explicit restrictive pipe ACL is added.
- Checks: `cargo test --workspace` — all 37 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.
- Added an owner-only SDDL security descriptor to named-pipe creation and enforced the same-user SID check before request reads. This closes the prototype’s unauthenticated default-descriptor gap; long-lived daemon integration and method-level grant enforcement remain next.
- Checks: native transport tests passed after correcting descriptor pointer cleanup; full workspace validation is recorded with this checkpoint. No audio endpoint or persistent Windows configuration was touched.
- Added a bounded `serve_connections` lifecycle helper and bounded client retry for transient `ERROR_PIPE_BUSY` while rotating pipe instances. The native test now serves two sequential authenticated clients and caught/fixed the instance-rotation race.
- Checks: `cargo test --workspace` — all 38 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.
- Added authorized framed dispatch to `crates/control` and wired the native pipe integration test through `ClientGrant::read_only`. A `graph.commit` request is rejected with `-32001` before parameter parsing or mutation, while `system.describe` remains available.
- Checks: `cargo test --workspace` — all 40 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.

### 2026-09-05 — WSL inventory

- Environment: WSL2 Linux kernel `6.6.87.2-microsoft-standard-WSL2`, `x86_64`.
- Portable tools visible in WSL: Rust `1.96.0`, Cargo `1.96.0`, Node `24.12.0`, npm `11.6.2`.
- Windows filesystem is mounted at `/mnt/c`; Windows PowerShell is present on disk.
- Attempted native Windows queries for OS/build, media devices, and compiler commands through `powershell.exe` and `cmd.exe`.
- Result: both interop attempts failed before command execution with `WSL (3 - ) ERROR: UtilBindVsockAnyPort:307: socket failed 1`.
- Evidence status: `blocked` for Windows OS build, endpoint inventory, SDK/WDK, hardware, WASAPI, process-loopback, virtual-driver, and latency checks. No Windows requirement is marked passed from this result.
- Safety: no drivers installed, no defaults changed, no audio captured, and no user files modified outside the project documentation.

This establishes that Codex can continue documentation and portable implementation from WSL, but M00's Windows gates require a working native Windows session (or a separately reachable Windows test machine). The WSL shell should not be treated as a replacement for that environment.

### 2026-09-05 — Native daemon entry point

- Added `serve_control_connections`, a reusable authenticated native entry point that owns a `ControlPlane` and applies an explicit `ClientGrant` to every framed request. Notification response suppression remains a documented follow-up for a production long-lived daemon.
- Checks: `cargo test --workspace` — all 40 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.
- Added optional-response pipe handling and `send_oneway` so JSON-RPC notifications are delivered, consumed, and disconnected without creating a response frame or blocking the client. `serve_control_connections` now uses this path.
- Checks: `cargo test --workspace` — all 41 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.
- Fixed batch response loss in the control-pipe adapter: all framed responses are now concatenated in order, and `round_trip_many` reads the expected response count. A native two-request batch test verifies both IDs survive the pipe.
- Checks: `cargo test --workspace` — all 42 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.

### 2026-09-05 — Mixed-batch authorization

- Added mixed-batch authorization coverage: permitted discovery and denied `graph.commit` responses remain ordered through control dispatch and the native pipe, with the denial produced before mutation.
- Checks: `cargo test --workspace` — all 43 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.
- Added native malformed-frame coverage: an oversized length header is rejected before control dispatch, and the temporary authenticated pipe closes without leaving a server handle behind.
- Checks: `cargo test --workspace` — all 44 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.
- Added explicit deny-by-default `ClientRole` mapping (`Observer`, `Editor`, `Operator`) to the control layer. Built-in roles never imply capture, recording, or device administration; sensitive scopes require an explicit grant.
- Checks: `cargo test --workspace` — all 45 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.
- Added `serve_control_connections_as_role`, wiring authenticated native connections to the explicit role policy without duplicating scope construction at transport call sites. The raw `ClientGrant` entry point remains available for custom policies.
- Checks: `cargo test --workspace` — all 45 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent Windows configuration was touched.
- Added SQLite `session_history` persistence with bounded revision reads, preserving prior session documents for recovery/undo foundations while keeping the latest-session lookup unchanged.
- Checks: `cargo test --workspace` — all 46 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No user database path was opened or modified; tests used only in-memory SQLite.
- Added bounded validated session import/export to storage. Imports reject oversized or domain-invalid documents before writing a session/history row; exports return the persisted canonical JSON document.
- Checks: `cargo test --workspace` — all 47 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. Tests used only in-memory SQLite; no user database path was opened.
- Made current-session and revision-history persistence atomic with one SQLite transaction, closing the partial-write window between those related tables.
- Checks: `cargo test --workspace` — all 47 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. Tests used only in-memory SQLite; no user database path was opened.
- Added SQLite online backup support through rusqlite’s backup API, with a live-database round-trip test using temporary project files and cleanup.
- Checks: `cargo test --workspace` — all 48 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No user database path was opened.
- Added explicit SQLite client enrollment/revocation records with constrained roles, auditable revoked state, and re-enrollment reset semantics. Control-plane integration remains the next authorization slice.
- Checks: `cargo test --workspace` — all 49 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. Tests used only in-memory or uniquely named temporary project databases.
- Connected control-plane enrollment APIs to durable storage and grant lookup: unknown/revoked clients receive no grant, enrolled roles map to explicit scopes, and re-enrollment clears revocation. Native PID-to-enrollment identity binding remains a follow-up.
- Checks: `cargo test --workspace` — all 51 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. Tests used only in-memory or uniquely named temporary project databases.

### 2026-09-05 — Native enrollment identity binding

- Bound native pipe authorization to the authenticated Windows user SID: `current_user_sid` resolves the durable enrollment grant, and an un-enrolled same-user client is rejected before dispatch. Fixed a real SID-buffer lifetime bug discovered by the native test.
- Checks: `cargo test --workspace` — all 52 unit tests and doc tests passed; `cargo fmt --all` and `git diff --check` passed. No audio endpoint or persistent machine configuration was touched.

### 2026-09-05 — Native capture cross-check and M01 durability

- Added a native MSVC WASAPI capture diagnostic using the newly installed Visual Studio Community 18.9.2/MSVC 14.51.36231 and Windows SDK 10.0.26100. It independently reproduced `E_INVALIDARG` from `IAudioClient::Initialize` on all 13 capture endpoints after `Activate`, `GetMixFormat`, and `IsFormatSupported` all succeeded. The harness never started or read audio and was removed after execution.
- Made normal storage-backed graph commits persist session history and the operation journal in one SQLite transaction. Added failure-stage rollback tests and backup destination policy tests for relative paths, missing parents, symlinks, and live-database targets.
- Checks: `cargo test --workspace` — 3 CLI, 16 control, 12 domain, 5 protocol, 8 storage, and 10 transport tests passed, plus all doc tests; `cargo fmt --all` and `git diff --check` passed. No system audio configuration was changed.

### 2026-09-05 — Native process-loopback activation

- Corrected the native harness to use an agile WRL `FtmBase` completion handler and to distinguish `GetActivateResult` from the callback method’s HRESULT. `ActivateAudioInterfaceAsync` for Explorer, `IAudioClient` query, shared-mode 44.1 kHz PCM initialization, and event-handle setup all returned `S_OK`.
- This is a real native activation/initialization pass but not an audio-data or latency pass: the harness did not call `Start`, `GetBuffer`, or read samples. The official sample solution was inspected and its build is blocked by its WIL NuGet dependency; the local dependency-free harness compiles with the installed MSVC/SDK.
- No default endpoint, volume, mute, privacy, or persistent audio configuration was changed. Generated executable/object files were removed after testing.
- The Rust scaffold was updated to call `GetActivateResult` and remains compile-validated, but its opt-in runtime still aborts in COM teardown with heap corruption. It is not enabled in the normal probe and is not counted as a runtime pass; the native WRL harness remains the safe reference path.

### 2026-09-05 — Per-user backend singleton

- Added a runtime-scoped `Local\AudioRouter-*` named mutex around the multi-connection backend server. A competing backend using the same pipe name now fails before creating pipe instances, while distinct pipe names remain independent.
- The Windows transport suite passed all 11 tests, including a same-user collision test and two sequential authenticated connections. The mutex is released on server shutdown and no audio or persistent machine configuration is touched.
- Added bounded eight-client concurrent named-pipe coverage; all clients received intact responses while the singleton server serialized authenticated connections. The transport suite now passes 12 tests.

### 2026-09-05 — Install matching Windows SDK and WDK

- Installed Microsoft Windows SDK `10.0.28000.2526` and Windows Driver Kit `10.1.28000.2526` through WinGet, matching the Visual Studio 2026 toolchain. Verified `km\wdm.h`, KMDF headers, and `km\x64\ntoskrnl.lib` are present.
- Downloaded Microsoft’s driver samples temporarily. The full SysVAD solution reached MIDL/compiler tasks but MSBuild’s file-tracker subprocess failed because this host shell exposes duplicate case variants of `PATH`; a direct MSVC kernel-mode compile of `EndpointsCommon\NewDelete.cpp` succeeded against the new WDK.
- No driver was installed, registered, loaded, signed, or used to alter audio configuration. Temporary sample sources and build outputs were removed.

### 2026-09-05 — Safe SQLite backup restore

- Added `Storage::restore_backup`, which requires absolute paths, a regular non-symlink source, a destination parent that already exists, a new destination file, a 100 MiB bound, and SQLite `integrity_check` success before restoring.
- The restore test verifies data round-trip and rejects a second restore over an existing destination. Storage coverage is now 9 tests; no live database or machine configuration is overwritten.

### 2026-09-05 — Persistent CLI import/export

- Added real `import <document-path> --database <absolute-path>` and `export <session-id> --database <absolute-path>` commands. Import delegates to the storage validator and export reads the persisted session; both support human and `--json` output.
- Added a fixture-backed round-trip test and retained the M01 CLI acceptance suite. CLI coverage is now 4 tests; no default database or machine configuration is created by the offline commands.

## Decisions and assumptions

- Required UI: React, TypeScript, Vite; proposed shell: Tauri/WebView2.
- Backend baseline: Rust, local versioned named-pipe API, per-user background process.
- Windows 11 x64 only; other OSes and ARM are future/out of scope.
- Managed virtual driver strategy/signing remains a required M00/M08 dependency.
- Built-in voice processing is required; VST3 compatibility does not imply legacy ReaPlugs support.
- Desktop routing uses an explicit virtual render source to avoid duplicate playback/self-capture.
- Numeric performance requirements are unmeasured targets pending Windows evidence.

## Specification verification

Before handing off the baseline, check local Markdown links, requirement-ID uniqueness/traceability, milestone prerequisite order, and whether all requested areas are assigned. Check for accidental claims of implementation or Windows test completion. Record the actual verification result below after execution.

Verification result (2026-09-05): passed documentation checks using a read-only Node.js filesystem validator in this workspace. It enumerated 30 Markdown files, found 174 unique numbered requirements and nine milestone files, verified local links and referenced heading anchors, parsed all three JSON examples, checked balanced code fences, and confirmed every requirement ID appears in the delivery traceability register. Requirement families have no numbering gaps. A manual consistency pass checked sequential prerequisites, v1/future scope, graph versus external-operation atomicity, protected voice failure behavior, and the reference route diagram.

The first validator also matched individual IDs in the traceability table as definitions; the final validator distinguishes actual requirement definitions from references and passes without duplicate definitions. No application tests, Windows audio measurements, driver builds, or hardware validation were run. No Git diff is available because the workspace is not an initialized repository.

Official Rogue Amoeba, Microsoft, Steinberg, Cockos, JSON-RPC, Tauri, and React Flow sources were consulted for relevant product/platform assumptions and are linked in the specification source register. Evolving MCP SDK/client details are explicitly deferred to implementation-time verification.

### 2026-09-05 — Native Windows inventory

- Evidence: [M00 Windows inventory](evidence/M00-windows-inventory-2026-09-05.md).
- Host: Windows 11 Home x64, build `26200`, `PATRICK5080`, CyberPowerPC GamingPC, ~31.3 GiB reported memory.
- Portable toolchain: Rust/Cargo `1.96.0`, Node `v22.22.3`, npm `10.9.8`.
- Hardware: Focusrite USB Audio, USB Digital Audio, PD200X Podcast Microphone, NVIDIA/Realtek audio; ATEM/Blackmagic devices are present but report `Unknown`.
- Existing virtual devices: VB-Audio Voicemeeter, VB-Audio Virtual Cable, and SteelSeries Sonar. No AudioRouter-managed driver is installed.
- Windows SDK `10.0.26100.0` is installed with `midl.exe`, `rc.exe`, and `signtool.exe` available by absolute path, but `cl.exe`, MSBuild, and CMake are not on PATH; Visual Studio and WDK remain unverified.
- WMI/PnP access was denied in the restricted shell and succeeded only with approved elevated read-only execution. No defaults, drivers, streams, or user files were changed.
- Evidence status: native machine inventory is now available. Capture/render, process-loopback, endpoint format/period, latency, driver bridge, and signing gates remain `not run` or `blocked`.

### 2026-09-05 — SDK/WDK follow-up

- Windows SDK `10.0.26100.0` and older SDK directories are present.
- `vswhere.exe -all -format json` returned an empty Visual Studio instance list.
- No WDK-specific markers were found under the Windows Kits tree; MSVC build tools remain unavailable on PATH.
- Consequence: the C++/driver probe cannot be built on this host until an authorized Visual Studio/WDK environment is available. Portable Rust work can continue, but this does not satisfy the native audio probe or driver gate.
- Evidence updated in [M00 Windows inventory](evidence/M00-windows-inventory-2026-09-05.md).

### 2026-09-05 — WASAPI probe scaffold

- Added a read-only Rust endpoint identity/state probe at [`tools/m00-wasapi-probe`](../../../tools/m00-wasapi-probe) with evidence notes in [M00 WASAPI probe](evidence/M00-wasapi-probe.md).
- The probe intentionally does not open streams, change defaults, install drivers, or write outside stdout.
- `cargo check`, `cargo build`, and `cargo run` pass; the probe reported 34 active endpoints, all with active state and endpoint IDs.
- The probe now also reports current mix format and device periods: most endpoints are 48 kHz/two-channel/32-bit with 2–3 ms minimum periods; Sonar endpoints include 96 kHz/eight-channel devices; the Focusrite render endpoint reports a 3 ms minimum period.
- It now performs non-mutating shared-mode `IsFormatSupported` queries for 44.1/48 kHz mono/stereo IEEE-float formats. No streams were initialized and no Windows configuration was changed.
- Format results: all 34 endpoints returned a closest match for 44.1 kHz mono/stereo; 48 kHz mono was exact on 1/34; 48 kHz stereo was exact on 29/34. `S_FALSE` closest-match results require negotiation rather than rejection.
- Shared-mode initialization test: 20/34 endpoints initialized with their current mix format and `AUTOCONVERTPCM|NOPERSIST`; 13 returned `E_INVALIDARG`, one returned `AUDCLNT_E_EXCLUSIVE_MODE_ONLY`. Successful buffer sizes were 1,056–2,112 frames. Clients were reset/released without being started.
- Render smoke test: 20 render endpoints started and stopped successfully with no submitted audio; one render endpoint failed initialization with `AUDCLNT_E_EXCLUSIVE_MODE_ONLY`. Capture clients were never started, so no microphone or desktop audio was captured.
- Capture isolation retry: capture clients were tested with no stream flags and a 100 ms shared buffer request; all 13 still returned `E_INVALIDARG`. This is a reproducible probe limitation/result, not a Windows capture feasibility conclusion.
- Capture baseline retry: the same no-flag capture initialization with zero buffer duration also returned `E_INVALIDARG` on all 13 endpoints. Microsoft’s `GetMixFormat` contract indicates the same-device shared-mode mix format should be accepted; capture-client diagnostics and exact format validation are next, not a conclusion that capture is unsupported.
- Capture duration retry: a 20 ms request matching Microsoft’s shared capture sample also returned `E_INVALIDARG` on all 13 capture endpoints. The failure is independent of the tested 100 ms, zero, and 20 ms durations; no capture stream was started.
- Fresh-client capture retry: initialization now uses a format allocated by the same client being initialized, no flags, and each endpoint’s measured minimum period. All 13 still return `E_INVALIDARG`. A representative descriptor is structurally consistent (`WAVE_FORMAT_EXTENSIBLE`, 48 kHz, 2 channels, 32-bit, block align 8, 384,000 bytes/sec, `cbSize=22`), so ordinary device contention is not the current leading explanation; full extensible subformat/channel-mask validation remains.
- Capture event retry: event-only shared initialization with a private event handle and the measured minimum period also returns `E_INVALIDARG` on all 13 endpoints. Audiosrv/AudioEndpointBuilder are running, microphone consent is allowed, and no AppPrivacy deny policy was found. The remaining issue is driver-specific/native capture compatibility, not an established device-busy condition.
- Capture duration/format isolation: a one-second/no-flag retry still returns `E_INVALIDARG` on all 13 capture endpoints. Three fresh-client format variants (raw `GetMixFormat`, copied full `WAVEFORMATEXTENSIBLE`, and constructed IEEE-float `WAVEFORMATEX`) all fail identically, including the 96 kHz mono endpoint. This rules out the earlier duration, event, and Rust format-copy hypotheses; it also makes ordinary shared-client contention an insufficient explanation. The detailed result is in [M00 WASAPI probe](evidence/M00-wasapi-probe.md).
- Endpoint loopback check: 20/21 render endpoints accepted a fresh `LOOPBACK|AUTOCONVERTPCM|NOPERSIST` initialization and were reset/released; the same one endpoint failed with `AUDCLNT_E_EXCLUSIVE_MODE_ONLY`. No loopback stream was started or read.
- Process-loopback research: Microsoft’s supported path is asynchronous `ActivateAudioInterfaceAsync` with `VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK` and `AUDIOCLIENT_ACTIVATION_PARAMS`, with one include/exclude target process tree and a Windows 10 build 20348 minimum. Host build 26200 meets the OS minimum; implementation and runtime evidence remain outstanding. SYSVAD is documented as a starting WDM sample, not a finished redistributable driver, and the host still lacks Visual Studio/WDK.
- Process-loopback Rust scaffold: the activation blob and completion-handler shape compile, but runtime invocation is disabled after the generated Rust COM result/interface handoff corrupted the probe before a trustworthy HRESULT could be collected. This is explicitly not process-loopback evidence; the next reliable implementation is the official native C++ sample in the missing Visual Studio/WDK environment.
- Driver decision: use a project-owned SysVAD-derived prototype for technical evaluation, while keeping production distribution blocked pending Visual Studio/WDK, isolated target-machine, package-signing, and Secure Boot/HVCI evidence. The decision and licensing/signing consequences are recorded in [M00 driver options](evidence/M00-driver-options.md); no driver source was downloaded or installed.
- Configuration safety: no defaults, volume, mute, exclusive-mode settings, or device properties changed; no restoration was required. `GetStreamLatency` before start returned zero and is not evidence of latency.
- This remains capability metadata only. Actual capture/render, loopback latency, process capture, and physical tone/impulse behavior remain unmeasured.

## Next authorized implementation task

Continue [M00](../../milestones/M00-feasibility.md) with the remaining
non-destructive evidence: calibrated physical latency and restart/PID-reuse
behavior. Native activation, capture/render lifecycle, process-loopback
include/exclude, event-driven, format, digital-loopback, and Rust adapter
data-path probes are already recorded. Bounded live testing is authorized only
with rollback and unchanged configuration; driver installation/loading,
default changes, signing-mode changes, and production-driver claims remain
outside this plan.

## M00 preparation checklist

- Preserve the verified Windows 11 x64 test environment and audio hardware inventory.
- Review current driver integration/signing options using the source register.
- Record toolchain/OS/device versions and supported capture probes. This is complete for the current host; preserve failed and unsupported results.
- Resolve DEC-03/06/07 with evidence before broad implementation.
- Preserve all failed/unsupported results and distinguish prototype from production-driver capability.

## Handoff rule

When work begins, add objective, requirement IDs, task checklist, changes, decisions, checks, environment, results, blockers, rollback, and next action here. After the milestone gate passes, archive the execution plan under `archived/` and link its evidence. Future ideas belong in `future/`, not this active task.

### 2026-09-05 — Bounded bundle staging

- Added storage-side v1 `.audiorouter` ZIP staging and import. The validator requires absolute non-symlink bundle/staging paths, caps compressed size at 100 MiB, expanded size at 250 MiB, entries at 1,000, and individual assets at 16 MiB.
- Rejected archive paths include absolute paths, `..` traversal, duplicates, symlinks, and executable extensions. Manifest format/schema and referenced graph/assets must be present before the staged graph is passed to the existing session validator.
- Extraction uses a unique child of the caller-owned staging directory and `create_new`, removes that child on rejection, and imports only after validation. No live database or path outside staging is written by the bundle boundary.
- Added tests for valid staging/import, traversal rejection with outside-path protection, and oversized-asset rejection. Added the pinned `zip` dependency and lockfile entries.
- Check: `cargo test -p audiorouter-storage` passed 12 tests plus doc tests. No Windows audio settings, defaults, streams, drivers, or SDK configuration were changed.

### 2026-09-05 — Native pipe concurrency stress

- Added a 32-client concurrent named-pipe stress test over the owner-only, same-user transport. Each client sends a framed JSON request concurrently; every response remains independently decodable and complete.
- The test initially caught only an assertion mismatch with the echo handler’s documented `{ok:true}` response shape; the transport itself remained intact. After correcting the assertion, all 13 transport tests passed.
- Check: `cargo test -p audiorouter-transport` passed 13 tests plus doc tests. No audio stream, default, driver, or machine configuration was touched.

### 2026-09-05 — Bundle asset integrity

- Extended the v1 bundle manifest asset form to accept either a path string or `{path, size, sha256}` metadata. Staged bytes are hashed with SHA-256 and optional declared size/hash values are checked before the session is committed.
- Added a mismatch test proving rejected bundles do not create an imported session. The `zip` and `sha2` dependencies are pinned through the workspace lockfile.
- Check: `cargo test -p audiorouter-storage` passed 13 tests plus doc tests. No audio streams, defaults, drivers, or machine configuration were changed.

### 2026-09-05 — Native WASAPI toolchain and activation correction

- Added [`tools/m00-native-wasapi-probe/build.ps1`](../../../tools/m00-native-wasapi-probe/build.ps1), which builds the existing C++ diagnostic against the installed VS2026 MSVC, Windows SDK, and WDK paths without changing global environment variables.
- Native capture cross-check: all 13 active capture endpoints returned success for activation, `GetMixFormat`, shared-mode `IsFormatSupported`, and non-starting `IAudioClient::Initialize` with `NOPERSIST`. This supersedes the earlier Rust-only all-`E_INVALIDARG` observation as a probe/binding issue, not a device-busy conclusion.
- Native process-loopback cross-check: Explorer process-tree activation, `IAudioClient` query, 44.1 kHz shared loopback/event/autoconvert initialization, and event-handle setup all returned `S_OK`. The client was reset/released; no stream was started or read.
- Evidence is in [M00 WASAPI probe](evidence/M00-wasapi-probe.md). The M00 gate remains open for actual process-tree data capture, render/capture data-path validation, latency, and driver lifecycle/signing evidence.

### 2026-09-05 — Native capture data path

- Extended the native diagnostic with an explicit `capture [endpoint-index] [milliseconds]` mode. It starts one selected shared capture client, counts packets/frames without retaining samples, then stops/resets/releases it.
- Run: `capture 0 200` returned successful activation, mix-format retrieval, initialization, capture service lookup, `Start`, ten packet reads totaling 4,800 frames, `Stop`, and `Reset`; process exit was zero.
- This is endpoint capture data-flow evidence only. Process-tree capture data, render/loopback data, latency, two-device synchronization, and driver lifecycle/signing remain open. No defaults, volumes, mutes, drivers, or persistent settings were changed, so no configuration restoration was required.
- Added an opt-in silent render data-path implementation that submits `AUDCLNT_BUFFERFLAGS_SILENT` buffers and cleans up deterministically. The newly rebuilt unsigned executable was blocked at runtime by Windows Application Control; no security-policy bypass was attempted. Render runtime evidence remains open.

### 2026-09-05 — Shared TypeScript contracts

- Added the pinned `contracts` package with strict TypeScript types for JSON-RPC requests/responses, sessions, nodes, edges, ports, permissions, side effects, discovery, and the currently implemented method set.
- Added a package lock and local `typescript@5.9.2`; `npm --prefix contracts run typecheck` passed. The package is transport-only and has no native/audio permissions.

### 2026-09-05 — CLI bundle round trip

- Added safe `export-bundle <session-id> --database <path> --output <path>` and `import-bundle <bundle-path> --database <path> --staging <directory>` commands. Export refuses existing destinations; import delegates to bounded staged validation and session validation.
- Extended `tests/acceptance/m01-cli.ps1` to import the checked-in fixture, export a bundle, import it into a separate database through an explicit staging directory, and verify the session identity. Temporary files are removed in a `finally` block.
- Focused checks: CLI (4) and storage (14) tests passed. No audio or machine configuration was changed.

### 2026-09-05 — Reusable Windows audio adapter groundwork

- Added `crates/windows-audio` to the workspace. Its read-only adapter owns COM setup/teardown and enumerates active capture/render endpoint IDs, directions, periods, and mix-format metadata with safe copying before COM memory release.
- `cargo test -p audiorouter-windows-audio` passed 2 tests, including live enumeration on this Windows host. This is M02 adapter groundwork, not a claim of live graph routing or realtime safety.
- No stream was initialized or started by the adapter; no defaults, volume, mute, driver, or persistent machine configuration was changed.
- Added `SharedCapture` to the adapter with exact endpoint selection, COM-owned format lifetime handling, shared-mode initialization, explicit start/stop/reset, and immediate packet-buffer release. A Windows test covers unknown-endpoint rejection without opening a stream; the native diagnostic remains the evidence for actual packet capture.
- Added the symmetric `SharedRender` lifecycle with exact endpoint selection, shared-mode initialization, explicit start/stop/reset, and silence-only buffer submission. Unknown capture/render endpoint tests pass without opening streams; the ordinary workspace test suite remains non-invasive.
- Wired `devices.list` through the read-only Windows adapter so control/CLI discovery now reports authoritative active endpoint IDs, direction, format, and periods instead of an invented empty list. Full audio status, app discovery, and graph routing remain unavailable until later M02 slices.
- Corrected `status.get` capability reporting to distinguish available device metadata discovery from unavailable realtime graph/routing; this avoids claiming the entire Windows adapter is missing.
- Added bounded read-only process discovery (`apps.list`) with PID and executable name only, excluding command lines and full paths. This supplies identities for future process-loopback binding but does not claim process-tree audio capture.
- Added identity-preserving endpoint snapshot diffing for added, removed, and changed metadata. It is a control-plane polling helper and never silently rebinds a missing endpoint; native IMMNotificationClient callbacks remain open.
- Upgraded `SharedCapture` and `SharedRender` to event-driven initialization with private RAII event handles, `SetEventHandle`, bounded waits, and `EVENTCALLBACK` flags. The normal tests remain non-invasive; actual stream lifecycle evidence stays in the opt-in native diagnostic.
- Added `EndpointNotificationSubscription` using WASAPI's `IMMNotificationClient`. The callback is allocation-free and nonblocking: it only sets an atomic dirty flag, while the owner performs a later read-only endpoint resnapshot. Registration/unregistration is COM- and RAII-scoped; it does not change defaults or open streams.

### 2026-09-05 — Preallocated realtime block core

- Added `crates/engine` with the M02 48 kHz planar-float32/128-frame representation and preallocated `AudioBlock` operations for clear, copy, gain, mix, and non-finite sanitization. Runtime operations reuse existing storage and do not allocate, lock, log, or perform I/O.
- Added deterministic tests for shape/quantum bounds, planar gain/mix behavior, NaN/Inf repair, and invalid-gain safety. This is portable engine groundwork; WASAPI callback scheduling, graph compilation, resampling, drift, and live routing remain open.
- Added explicit destination-major channel-matrix conversion for mono/stereo paths with no allocation, plus mono-to-stereo, stereo-to-mono, and invalid-matrix tests. Resampling, clock drift, and live graph scheduling remain open.
- Added an immutable prepared `RuntimeGraph` schedule for gain/mute stages with generation identity and post-stage finite-value sanitization. The seven engine tests now cover ordered stage execution; domain-session compilation and live generation publication remain open.
- Added bounded linear sample-rate conversion into preallocated output blocks with invalid-rate/shape rejection and a 48→24 kHz test. This does not claim cross-block clock-drift correction or hardware synchronization.
- Added a bounded FIFO-occupancy `DriftController` with configurable ppm clamp and adjusted resampling ratio, plus a ±100 ppm simulation test. This is deterministic control groundwork, not dual-device hardware drift evidence.
- Added `engine::compile_session` as an explicit domain-to-engine preparation seam. It validates the session, derives deterministic node order, prepares supported gain/mute stages with a generation, and rejects enabled edge routing until realtime buffer transfer is implemented; no endpoint is opened.
- Added `SharedCapture::next_packet_into`, a caller-owned bounded byte-buffer API that copies packet data, zero-fills WASAPI silent packets, and always releases the device buffer before returning. It requires explicit bytes-per-frame and reports undersized destinations; no sample storage is allocated by the adapter.
- Added `SharedRender::submit_bytes`, the symmetric caller-owned interleaved-byte submission boundary. It validates complete frames, caps writes to current device capacity, copies into WASAPI's buffer, and releases it without allocation or settings changes.
- Added an `ArcSwapOption`-backed `RuntimePublication` slot. Control code prepares and publishes immutable generations; readers retain the old graph safely during replacement, and deferred reference-count reclamation prevents torn graph ownership. No device or machine configuration is touched.
- Added optional `CallbackMetrics` instrumentation for processed quanta and repaired non-finite samples. `RuntimeGraph::process_instrumented` updates only relaxed atomics on the processing path; the metrics test confirms the counters without adding logging or allocation.
- Added a bounded per-frame `GainRamp` for de-clicked transitions, including finite-target sanitization and immediate mute/unmute support. The 13-test engine suite verifies exact ramp progression without device interaction.
- Added an atomic process-local `PrivacyMute` gate that clears each processed block while enabled and resumes normal processing when cleared. It does not alter Windows privacy permissions or other applications' microphone access.
- Added `RuntimeProcessor` to combine safe pre-activation silence, immutable graph publication, callback metrics, and the process-local privacy gate at one block boundary. Its 15-test engine suite verifies no-graph silence, generation activation, and emergency mute behavior.
- Corrected `RuntimeProcessor` privacy ordering so mute clears the block before graph stages execute; muted capture cannot reach processors, not merely the final output.
- Added `AudioBlockQueue::drain` for stop/reconnect cleanup; it discards pending blocks without replay and does not count intentional cleanup as an underrun.
- Added optional fixed-shape queue construction; mismatched blocks are rejected without entering the queue and counted separately as invalid blocks. Engine tests remain at 21 and strict engine Clippy passes.
- Added `AudioBlock::mix_mapped_from` for allocation-free destination-major matrix accumulation, preserving existing destination audio for explicit fan-out/mixer inputs. The engine suite now has 16 passing tests; full node scheduling remains open.
- Added allocation-free `AudioBlock::clamp_unit` and `peak_abs` primitives for output-boundary clipping counts and peak metering. Internal graph processing still retains headroom; the caller explicitly chooses when to clamp.
- Extended `CallbackMetrics` with caller-recorded clipping and xrun counters. The counters are atomic and deliberately do not infer hardware failures; the Windows scheduler will record those events when implemented.
- Added lock-free `BlockMeter` peak and clipping observation with reset semantics. It is a portable Meter-node primitive; per-node runtime wiring and external health publication remain open.
- Added fixed-capacity lock-free `AudioBlockQueue` storage using preallocated slots. Push/pop never wait or allocate after construction and expose full/empty conditions for explicit xrun policy; the engine suite now has 19 passing tests.
- Added atomic overrun/underrun counters to `AudioBlockQueue`; full pushes and empty pops remain nonblocking and are now observable as queue-health events.
- Added allocation-free per-channel peak/RMS and aggregate RMS primitives to `AudioBlock`, with finite-sample filtering and invalid-channel handling. Engine tests now total 20, and strict engine Clippy passes.
- Added preallocated rolling `RmsWindow` with bounded capacity, finite-input handling, and reset semantics for the specified RMS telemetry window. Engine tests now total 21 and strict engine Clippy passes.
- Wired `BlockMeter` into `RuntimeProcessor` so active processed blocks update peak and clipping health; pre-activation silence is not reported as active graph telemetry.
- Extended the native probe with opt-in `process-capture` data reads. A 500 ms run completed async process-loopback activation, event-driven `Start`/read/`Stop`/`Reset`, and read 50 packets/22,050 frames with 15,217 nonzero payload bytes; temporary binaries were removed and no system audio settings changed.
- Cleared strict workspace Clippy findings in the domain, engine, Windows adapter tests, and CLI iterator code; `cargo clippy --workspace --all-targets -- -D warnings` now passes.
- Added explicit `process-capture-exclude` support and verified the exclude-tree mode for 500 ms with the same successful 50-packet/22,050-frame read and cleanup. Controlled per-process tone attribution remains open.
- Added stable `AudioFailureKind` classification for invalid arguments, access denial, device-in-use, exclusive-only, invalidated-device, unsupported-format, service-unavailable, and buffer-constraint cases while retaining original HRESULTs.
- Extended read-only application discovery with optional Windows process creation timestamps, allowing future process-loopback bindings to verify PID plus creation time and reject PID reuse; command lines and full paths remain excluded.
- Added `bind_application`, which requires PID, executable name, and creation timestamp to match the observed process before a future loopback activation; the Windows identity test passes without opening an audio stream.
- Tightened `bind_application` so an unavailable creation timestamp is rejected rather than treated as a valid identity; PID/name alone can no longer authorize a future loopback binding.
- Changed control/API serialization of process creation timestamps to exact decimal strings, preventing JavaScript precision loss for Windows `FILETIME` values; control tests, contracts typecheck, and strict control Clippy pass.
- Corrected the failure taxonomy so undersized caller buffers report `BufferConstraint` rather than being conflated with malformed arguments; seven adapter tests and strict adapter Clippy pass.
- Added `EndpointMonitor` to combine the coalesced notification dirty flag with a control-thread endpoint resnapshot and explicit `EndpointChange` diff; no automatic rebinding is performed.
- Corrected the M02 engine evidence report's stale nine-test conclusion; it now distinguishes historical baseline coverage from the current 21-test engine state and remaining end-to-end routing gaps.
- Added explicit `RuntimePublication::clear`/`RuntimeProcessor::deactivate` lifecycle operations; new readers receive safe silence after stop while retained old generations remain valid for existing readers.
- Added `RuntimeProcessor::process_queued` for control/worker integration: it consumes fixed-shape queued blocks without waiting, safely silences empty input, and returns shape errors after clearing output. It is explicitly not callback-safe because owned block reclamation can deallocate; a reusable realtime pool/ring remains required for native scheduler wiring.
- Added `AudioBlockPool` with construction-time allocation of every fixed-shape block and explicit acquire/release operations. A correctly recycled ownership cycle performs no allocation or deallocation; the pool is groundwork for callback-owned rings and is not yet integrated with native scheduling.
- Added `AudioBlockRing`, pairing the fixed-shape pool with a bounded ready queue. Producer/consumer transfer, full/empty counters, and explicit recycle ownership are now tested; native callback integration and scheduler timing evidence remain open.
- Added `RuntimeProcessor::process_ring_once` for pooled worker-path transfer between input and output rings. It acquires destination-owned storage, copies/processes/recycles without allocation, and records output-pool starvation as an xrun; ring membership remains a caller invariant because shape checks cannot identify allocation provenance.
- Hardened pool recycling to clear every block before it becomes available again, preventing stale audio from crossing ownership cycles; the engine suite remains at 25 passing tests.
- Added a regression test for output-pool starvation: input is recycled, output remains empty, and exactly one xrun is recorded without fallback allocation.
- Added read-only `routes.inspect` domain/control support. It validates the desired session graph and returns enabled upstream node/edge provenance for a destination, with disabled-edge and missing-destination tests; running-resource paths remain separate until M02 activation exists.
- Extended `routes.inspect` paths with the validated destination-major channel matrices for every edge, preserving conversion visibility required by GRAPH-03/12; contracts typecheck remains green.
- Added bounded engine execution for validated single-path, same-channel edges using in-place channel matrices. A linear physical-input-to-output fixture now proves matrix application; fan-out, mixer convergence, device activation, and live scheduling remain unsupported.
- Added a fan-out regression fixture proving the engine rejects branching before execution until per-branch buffers and mixer/resource scheduling are implemented; engine coverage is now 28 passing tests.
- Added CLI parity for read-only `routes inspect <session-id> <destination-node> --database <path>`, including persisted-session loading and channel-map output. CLI tests, strict Clippy, and the M01 acceptance script pass.
- Added bounded read-only `graph.history` discovery/dispatch. In-memory history retains the newest 100 snapshots; durable control reads the existing SQLite history with a 1–500 request bound, and newest revisions are returned first.
- Added revision-checked `graph.undoPlan` as a PlanOnly method. It selects the prior bounded snapshot and routes it through normal plan validation; the resulting undo still requires ordinary `graph.commit`, and ambiguous/no-history cases are refused.
- Added portable bounded `EventLog` groundwork with backend epoch, monotonic sequence, resource revision, operation ID, 10,000-event retention, 1–500 replay limits, and explicit resync-required signaling for lost cursors. Control subscription wiring remains next.
- Wired `events.subscribe` into control dispatch. Session creation and graph commit now emit filtered state events; clients can replay by sequence and session, while invalid limits and lost cursors are explicit errors. Meter events remain excluded.
- Added explicit `nodes.describe` API/CLI parity over the authoritative node registry, preserving availability and realtime-cost metadata; the M01 acceptance script now checks description parity with `nodes.types`.
- Added persisted CLI `history <session-id> --database <path> [--limit N]`, enforcing the 1–500 bound and returning newest SQLite snapshots; the M01 acceptance script now verifies the imported fixture history.
- Enforced the promised 100-snapshot bound in the in-memory graph history store and added overflow coverage; durable SQLite retention remains governed by its existing bounded-history tests.
- Added persisted `session start`/`session stop` CLI commands with lazy database session loading. They exercise the deterministic fake runtime only; M02 real graph activation remains unavailable, and M01 acceptance now verifies both lifecycle transitions.
- Added read-only `sessions.get` control/API parity and `session get` CLI loading from the selected database. The acceptance script now verifies the persisted fixture before lifecycle operations; this remains configuration inspection, not live audio activation.
- Added bounded `sessions.list` storage/control/API parity and `session list --database <path> [--limit N]` CLI output, sorted by opaque ID and capped at 500; the M01 acceptance script now verifies the persisted resource list.
- Extended `graph.plan` responses with deterministic diff entries, affected physical-output names, warnings, and required GraphWrite scope metadata. Planning remains side-effect free and does not activate audio.
- Connected committed graph revisions to an already-running deterministic fake session: a successful new commit prepares/starts one new fake generation and reports activation, while stopped sessions report pending fake activation. Native resource activation remains M02 work.
- Made `graph.undoPlan` restart-safe by hydrating prior bounded snapshots from SQLite when the control process has no in-memory history; a fresh-control-plane regression verifies undo planning after restart.
- Bound in-memory idempotency keys to their original plan IDs: replaying the same key for the same plan remains safe, while reusing it for a different plan returns an explicit conflict. Durable cross-process request hashing and expiry remain open storage work.
- Added v1 bundle `requiredNodeTypes` compatibility metadata. Exports list the session's known node types deterministically, and imports reject unknown or unsupported type versions before session commit; legacy manifests without the optional field remain readable.
- Added a durable SQLite idempotency request-hash column with migration-safe initialization, checked replay lookup, and explicit same-key/different-hash conflict handling. The storage suite now has 16 passing tests; control-plane wiring and bounded expiry remain next.
- Integrated durable idempotency into `graph.commit`: the control plane hashes the planned session payload, replays a matching journal result before mutation after restart, and rejects a reused key with a different request hash. Domain/control/storage tests pass (19/22/16); bounded expiry remains open.
- Added 24-hour durable idempotency retention. Journal lookups and writes prune expired rows through an indexed timestamp, allowing bounded replay memory while preserving same-key conflict checks within the retention window; storage coverage is now 17 tests.
- Corrected durable replay identity to hash the commit plan ID and base revision, allowing a fresh control process to replay before in-memory plan lookup while still rejecting key reuse for another request. A restart regression now proves this path; affected coverage is 23 control, 19 domain, and 17 storage tests.
- Added explicit `events.subscribe` recovery results for expired cursors: clients receive `resyncRequired`, the current backend cursor, and a bounded session snapshot instead of an opaque request error. Control coverage is now 24 tests; transport subscriber lifetime remains open.
- Added bounded persistent named-pipe sessions with shared same-user authentication, fixed frame lifetimes, deterministic disconnect, and a control-plane wrapper. The Windows transport suite passes 14 tests, including two requests over one authenticated connection; no audio configuration is touched.
- Added a preallocated `MixerStage` engine primitive for bounded multi-input convergence through validated destination-major matrices. Processing clears/accumulates caller-owned blocks without allocation or device access, and rejects shape/count errors before output mutation; the engine suite now passes 30 tests.
- Added `compile_mixer_session` and `CompiledMixerGraph` for the narrow enabled topology of multiple sources converging on one mixer and feeding one output. Preparation validates ports/matrices; execution uses caller-owned scratch/output blocks, and the engine suite now passes 31 tests. Full device scheduling and arbitrary graph topology remain open.
- Hardened mixer compilation to reject unrelated enabled edges, duplicate source participation, and non-physical output destinations before preparation. The existing compiler/runtime coverage remains green at 31 tests with strict Clippy.
- Added the specified eight-input maximum to prepared mixer stages, with explicit `InputLimit` rejection and regression coverage; the engine suite now passes 32 tests with strict Clippy.
- Added bounded `CompiledFanoutGraph`/`compile_fanout_session` for one enabled source feeding two to eight physical outputs through independent channel matrices. Branches execute into caller-owned destinations while the generic single-block compiler continues to reject fan-out; engine coverage remains 32 tests.
- Added non-finite sanitization at prepared mixer and fan-out boundaries so direct compiled-stage callers cannot send NaN/Inf to sinks. Regression coverage is now 33 engine tests with strict Clippy.
- Added complete fan-out preflight validation for frame/channel/matrix shapes, preventing an early branch from being modified when a later branch is invalid. The engine suite remains green at 33 tests with strict Clippy.
- Applied the domain’s finite ±2 coefficient bound to prepared mixer matrices as well as graph validation; out-of-range and non-finite preparation is rejected, and the engine suite now passes 34 tests with strict Clippy.
- Added global session resource enforcement: aggregate nodes are capped at 128 and aggregate edges at 256, with replacement-safe accounting. Discovery and TypeScript contracts expose both global limits; domain/control tests and contract typecheck pass.
- Added control-plane graph-store checkpoints around session insertion and graph commits. If a subsequent SQLite write fails, the prior in-memory state is restored, preventing memory/durable divergence; affected control/domain tests remain green at 24/20.
- Added a legacy SQLite journal migration regression: pre-request-hash schemas upgrade with conservative empty hashes, preserve old results, and reject new-hash reuse. Storage coverage is now 18 tests with strict Clippy.
- Applied global node/edge budget checks transactionally during `GraphStore::restore_history`, closing the restart hydration bypass. Rejected oversized restored state rolls back fully; affected coverage is 21 domain and 24 control tests with strict Clippy.
- Enforced the specification's two-active-session cap in the fake/control lifecycle: starting an already-running session remains idempotent, while a third distinct running session is rejected. Discovery and TypeScript contracts expose `maxActiveSessions`; domain/control tests, strict Clippy, and contract typecheck pass.
- Added an opt-in native `process-attribution` probe mode. It launches a short-lived child copy of the probe, renders a deterministic 997 Hz tone, captures the child process tree through application loopback, and reports child exit/packet data while terminating and cleaning up bounded resources. The mode builds with the installed Visual Studio/Windows SDK toolchain but was not run because it intentionally emits an audible test signal; no machine audio configuration was changed.
- Hardened native process-loopback data validation: capture results now include accumulated 16-bit sample energy, while only controlled attribution requires nonzero payload. Ordinary read-data modes continue to accept valid silent packets so liveness and silence remain distinguishable. The updated probe builds successfully; runtime attribution remains intentionally unrun because it emits an audible test tone.
- Implemented the portable linear compiler's compatible bypass semantics: bypassed Gain, Mute, and Meter nodes preserve their surrounding dry path, while bypass on device-bound nodes remains rejected. Engine coverage is now 35 tests with strict Clippy; native scheduling and disabled-device semantics remain open.
- Implemented disabled-node semantics for the supported linear compiler: disabled physical/application/loopback sources, mixers, and outputs insert a silence boundary, while disabled compatible processors use dry bypass. Regressions cover disabled source and sink paths; the 35-test engine suite and strict Clippy remain green. Native scheduler recovery and recorder finalization remain open.
- Corrected `runtime.stopped` event publication to carry the session's current resource revision instead of hard-coded revision zero. A lifecycle regression verifies the event metadata; control tests remain green with strict Clippy.
- Hardened CLI list-command validation so invalid `nodes` subcommands return an actionable argument error instead of reaching a panic path. CLI tests and strict Clippy pass.
- Added per-client mutation rate limiting to authenticated control dispatch: a 40-request burst refills at 20 requests/second, returns a stable `rateLimited` code with `retryAfterMs`, and is keyed by the authenticated Windows client SID in transport wiring. End-to-end response coverage is included; control/transport tests pass (27/14) with strict Clippy. Meter subscription throttling remains open.
- Completed the event retention bound with monotonic 15-minute expiry in addition to the 10,000-event cap. Append and replay paths prune expired entries before cursor checks, preserving explicit resync for lost cursors; domain/control tests pass (22/27) with strict Clippy.
- Added stable ID cursor pagination to `sessions.list` across in-memory and SQLite-backed control paths, returning bounded `{items,nextCursor}` pages while preserving the internal snapshot helper. Durable/control/domain regressions and contract typecheck pass; CLI output remains backward-compatible.
- Added revision cursor pagination to `graph.history`, with bounded look-ahead so `nextCursor` is emitted only when another snapshot exists. In-memory and SQLite-backed history pages share newest-first ordering; control/storage/domain tests and contract typecheck pass.
- Added backend-owned `sessions.create` and `sessions.delete`: creation requires a validated stopped revision-0 session; deletion removes SQLite current/history rows and in-memory plans/runtime state, refuses running sessions, and emits a revisioned event. CLI delete and shared contracts are wired; affected tests and strict Clippy pass.
- Added `sessions.duplicate` across control, contracts, and CLI. It clones a validated source into a fresh revision-0 stopped session, supports an optional name, and rejects destination ID collisions; affected tests and strict Clippy pass.
- Added CLI parity for `sessions.create` via `session create <absolute-document> --database <path>`, delegating revision-0 and graph validation to control/storage. Temporary test databases/documents are cleaned up; affected tests and strict Clippy pass.
- Added stable `data.code` values to application JSON-RPC failures (`revisionConflict`, `planExpired`, `notFound`, `idempotencyConflict`, `permissionDenied`, and related categories), while retaining existing numeric codes and readable messages. Mutating notification handling now covers the complete session lifecycle; control tests and strict Clippy pass.
- Rebuilt and reran the native probe with the installed Visual Studio/WDK toolchain. Endpoint 0 captured 4,800 frames; process-loopback include and exclude each captured 8,820 frames with nonzero payload, and all streams stopped/reset successfully. Temporary binaries were removed; no machine audio configuration changed.
- Superseding evidence note: earlier M00/M02 entries that list process-loopback data as unmeasured are historical snapshots. The later native probe results establish include/exclude data-path reads; controlled per-process tone attribution, physical latency, and driver/signing remain unverified.
- Added JSON-Schema-style input and output schemas to every discovery method, covering required fields, bounded pagination values, and closed request objects. Rust discovery regression and TypeScript contract updates are included; schema validation remains client-side until a generated validator is selected.
- Verified the checked-in native WASAPI probe builds with the installed Visual Studio 2026/Windows SDK toolchain, and the Windows adapter’s non-invasive suite passes 8 tests. Temporary probe binaries were removed; no runtime audio probe, driver operation, or machine configuration change was performed.
- Added human-readable descriptions to every discovered API method and the shared TypeScript contract. Discovery now exposes method purpose alongside permission, side-effect class, and schemas; control tests and contract typecheck pass.
- Enforced the discovered closed request objects at the control boundary: non-object parameters and unknown fields now return JSON-RPC invalid-params errors instead of being silently ignored. The control suite is green at 32 tests with strict Clippy.
- Added read-only `system.handshake` protocol negotiation. Clients can request additive minor versions, receive the supported major/minor and persisted schema version, and are rejected before dispatch for unknown major versions; control/domain tests, strict Clippy, and contract typecheck pass.
- Hardened API-09 application errors with structured `fieldPath`, `resourceIds`, `retryable`, and `remediation` metadata; rate-limited errors retain `retryAfterMs`. Control/protocol tests, strict Clippy, and contract typecheck pass.
- Added the versioned TypeScript RPC client surface for all 20 implemented methods, including method-specific parameter/result mappings, request ID generation, undefined-parameter omission, and typed structured error propagation. Contracts typecheck passes.
- Extended offline discovery with the implemented state-event category list, explicit non-replayable meter policy, and authoritative 10,000-event/900-second retention bounds. Control/domain tests, strict Clippy, and contract typecheck pass.
- Added first-class `api call <method> [<params-json-file|->] [--database <path>]` CLI coverage using the shared JSON-RPC dispatcher, bounded 4 MiB parameter input, and JSON response envelopes. CLI compile/Clippy validation passes; runtime execution of the rebuilt test binary is blocked by the host Application Control policy (OS error 4551), while control/protocol tests pass.
- Added canonical `applications.list` discovery/API parity while retaining `apps.list` as a backward-compatible alias; both names use the same authoritative application inventory and return identical results. Control/domain tests, strict Clippy, and contract typecheck pass.
- Extended node discovery with built-in parameter schemas for Gain (`gainDb`, -60..24 dB) and Mute (`muted`), plus typed contract fields for node parameters. Compile-only validation, strict Clippy, and contract typecheck pass; rebuilt control test execution remains blocked by Windows Application Control (OS error 4551).
- Added checked-in golden JSON-RPC handshake request/response fixtures and a control regression that compares the real dispatcher response byte-for-byte after deserialization. Compile-only test validation and strict Clippy pass; runtime test execution remains blocked by Windows Application Control (OS error 4551).
- Aligned nullable optional cursor and duplicate-name parameters with their discovered schemas: explicit `null` is now treated as omitted, with a control regression covering both dispatch paths. Compile-only validation and strict Clippy pass; runtime execution remains blocked by Windows Application Control (OS error 4551).
- Added read-only `system.diagnostics` to the API registry, discovery, TypeScript client mapping, and control dispatch. It reports redacted backend/storage/audio capability state and event-log counters without filesystem paths or secrets; compile-only validation, strict Clippy, and contract typecheck pass.
- Exposed durable client enrollment management through `clients.list`, `clients.authorize`, and `clients.revoke`, with explicit `deviceAdministration` authorization for mutations and stable sorted SQLite listing. Storage tests (20), compile checks, strict Clippy, and contract typecheck pass.
- Added canonical plural `sessions.start`/`sessions.stop` aliases alongside the existing singular names, preserving identical session-control authorization, rate limiting, notification rejection, and fake-runtime lifecycle behavior. Compile checks, strict Clippy, and contract typecheck pass; rebuilt control execution remains blocked by Application Control.
- Added read-only `operations.get` backed by the durable SQLite operation journal. It returns completed operation status, method, revision, timestamp, and exact persisted result, while memory-only backends explicitly report non-durable unknown status; storage tests (20), compile checks, Clippy, and contract typecheck pass.
- Extended `operations.get` with bounded in-process outcome retention (100 graph commits), returning live results with `durable:false` while preserving SQLite-backed durable results across restart. Control compile checks, storage tests (20), strict Clippy, and contract typecheck pass.
- Corrected `operations.get` lookup precedence so configured SQLite journal results are returned as `durable:true` before consulting the in-process cache; memory-only backends retain their non-durable fallback. Control tests (41) and strict Clippy pass.
- Made `status.get` an authoritative dynamic snapshot: it now reports storage mode, loaded session count, active session IDs/count, and the event cursor alongside explicit audio availability. The TypeScript contract models the snapshot; compile checks, strict Clippy, and contract typecheck pass.
- Started M04 with a standalone `audiorouter-recording` WAV primitive: validated mono/stereo 44.1/48 kHz PCM16, PCM24, and float32 encoding, finite-value sanitization, optional deterministic TPDF dithering, frame/byte accounting, and seek-back RIFF finalization. Three in-memory tests and strict Clippy pass; recorder queues, pause/split/recovery, FLAC, and filesystem policy remain open.
- Added a fixed-capacity nonblocking `RecordingQueue` for caller-owned interleaved chunks, with returned-overflow ownership and explicit overrun counters. Four recording tests and strict Clippy pass; queue workers and encoder integration remain open.
- Added the M04 `RecorderController` state machine for arm/start/pause/resume/split/stop/fail, exact part frame boundaries, pause intervals, backward-clock rejection, and new-recording reset. Six recording tests and strict Clippy pass; file-worker integration and recovery remain open.
- Connected the M04 queue/state primitives to a caller-owned `WavRecorder` worker boundary. It drains bounded contiguous chunks, honors pause gaps through frame clocks, rejects discontinuities, and finalizes only after stop; seven recording tests and strict Clippy pass.
- Added M04 recording path policy: absolute local roots only, Windows-invalid/reserved component sanitization, canonical root containment, and exclusive `create_new` file creation. Eight recording tests and strict Clippy pass; reparse-point and full library/recycle integration remain open.
- Restricted recording path extensions to the supported `wav` and `flac` formats; unsupported extensions are rejected before file creation. Eight recording tests and strict Clippy pass; reparse-point and full library/recycle integration remain open.
- Rejected recording roots that are Windows reparse points (and Unix symlink roots in the portable regression), before canonicalization. The Windows recording suite remains green with strict Clippy; nested reparse-point and full library/recycle integration remain open.
- Added conservative WAV crash recovery: a partially finalized file can be repaired in place using its known format, incomplete trailing frames are truncated, and RIFF/data sizes are rebuilt without replacement or deletion. Nine recording tests and strict Clippy pass; crash journal integration remains open.
- Made recorder worker failures terminal: queue frame discontinuities and encoder/I/O errors move `WavRecorder` to `Failed`, prevent finalization, and preserve the caller-owned partial destination for recovery or quarantine. Ten recording tests and strict Clippy pass.
- Added strict WAV inspection metadata for library indexing: format, channels, sample rate, exact frame/data counts, and file size are validated without decoding or device access; malformed or truncated files return an explicit error. Eleven recording tests and strict Clippy pass.
- Added non-fatal recording inspection statuses for library consumers: existing valid WAVs return metadata, missing files return `Missing`, and malformed files return `Invalid` without aborting a listing. Twelve recording tests and strict Clippy pass.
- Added a pure-Rust `flac-io` 0.1.1 batch encoder boundary compatible with the workspace Rust 1.80 floor and MIT/Apache-2.0 licensing. `FlacBufferEncoder` supports mono/stereo 44.1/48 kHz FLAC16/24 conversion, finite repair, and decoded PCM16 round-trip verification; thirteen recording tests and strict Clippy pass. Streaming FLAC worker integration remains open.
- Added a root-scoped in-memory `RecordingLibrary`: registration validates containment, records present/missing/invalid status, supports refresh and session filtering, and removes metadata entries without deleting files. Fourteen recording tests and strict Clippy pass; durable library persistence, rename/metadata editing, preview, and recycle authorization remain open.
- Added bounded in-memory recording title/artist/comment metadata with control-character and 256-character validation. Metadata updates are separate from file removal and preserve the non-destructive library contract; fourteen recording tests and strict Clippy pass.
- Added durable SQLite recording-library rows with session/recorder/path, format/shape, frame/size, start/state/missing, and title/artist/comment fields. Save/list/update/remove-entry APIs are transactional at the row boundary; 21 storage tests and strict Clippy pass, including reopen persistence and non-destructive removal. Control/API integration, file tags, rename, preview, and recycle remain open.
- Added the standalone `audiorouter-dsp` biquad primitive for M04 parametric EQ groundwork: peaking, low/high shelf, low/high pass, and notch shapes; bounded sample-rate/frequency/Q/channel validation; allocation-free interleaved mono/stereo processing; finite-sample repair; reset and parameter updates. Four DSP tests and strict Clippy pass; graph node/API integration and full transfer-function vectors remain open.
- Extended `audiorouter-dsp` with a stereo-linked feed-forward compressor: threshold/ratio/knee/makeup bounds, attack/release envelope state, linked gain reduction, finite input/output repair, and allocation-free interleaved processing. Six DSP tests and strict Clippy pass; gate/limiter/delay, graph/API integration, and measured transfer vectors remain open.
- Extended `audiorouter-dsp` with a stereo-linked gate/downward expander: threshold, hysteresis, ratio, range, attack, hold, and release bounds; explicit open-state tracking; finite repair; and allocation-free processing. Eight DSP tests and strict Clippy pass; limiter/delay, graph/API integration, and measured transfer vectors remain open.
- Added a conservative sample-peak `PeakLimiter` and bounded allocation-once `DelayLine` to `audiorouter-dsp`. The limiter enforces a declared -12..0 dBFS sample ceiling without claiming true-peak/lookahead protection; the delay supports bounded 0..1000 ms changes, reset, finite repair, and interleaved mono/stereo processing. Ten DSP tests and strict Clippy pass; graph/API integration and measured vectors remain open.
- Exposed `Biquad::magnitude_db_at` from the same coefficients used for processing, providing a single-source response-curve calculation. Reference tests cover flat response, +6 dB peaking, deep notch attenuation, and Nyquist bounds; eleven DSP tests and strict Clippy pass.
- Added explainable fixed EQ presets to `audiorouter-dsp`: all-disabled `VoiceNeutral`, plus single-band Q8 notch presets at 50 Hz and 60 Hz. Preset bands reuse the validated `BiquadParams` schema and reject invalid sample rates; twelve DSP tests and strict Clippy pass.
- Added named `VoiceChainPresetId` configurations for `VoiceNeutral` and `VoiceGateAndCompression`, using the documented conservative gate/compressor/limiter defaults. Presets only prepare DSP state and never arm recording or enable monitoring; DSP tests and strict Clippy pass. See [M04 effects and recording evidence](evidence/M04-effects-recording.md).
- Added stable names and explanations for the two voice-chain presets, allowing clients to present their purpose before application without making the DSP layer mutate a session. DSP tests and strict Clippy pass.
- Published the voice-chain preset catalog through `system.describe` and the TypeScript discovery contract, keeping preset application behind explicit graph plan/commit. Control tests, contracts typecheck, and strict Clippy pass.
- Added read-only `presets.list` API and `presets list` CLI parity for the voice-chain catalog, with stable result schemas and no preset application side effect. Domain/control/CLI tests and strict Clippy are the validation targets.
- Added a fixed-capacity eight-band `ParametricEq` processor that prebuilds band state, executes enabled bands without allocation, supports preset construction, band replacement, reset, and invalid-index rejection. Thirteen DSP tests and strict Clippy pass; graph/API integration and graphic EQ remain open.
- Added the fixed ten-band `GraphicEq` processor with 31.5 Hz–16 kHz center frequencies, +/-18 dB gain bounds, Nyquist compatibility checks, reset, and allocation-free interleaved processing. Fourteen DSP tests and strict Clippy pass; graph/API integration and parameter smoothing remain open.
- Added allocation-free `Biquad::set_params_ramped` automation: target coefficients are prepared before processing, interpolated over an explicit frame count, and snapped exactly to the target at completion. Fifteen DSP tests and strict Clippy pass; graph/API integration and higher-level automation policy remain open.
- Extended `GraphicEq` with per-band `set_gain_db_ramped`, reusing the shared coefficient ramp for click-free gain automation while retaining fixed filter state. Fifteen DSP tests and strict Clippy pass; graph/API integration and higher-level automation policy remain open.
- Exposed compressor `gain_reduction_db()` metering from the effective linked detector path, with reset semantics and quiet/compressed reference coverage. Fifteen DSP tests and strict Clippy pass; broader per-channel RMS/peak telemetry and graph/API integration remain open.
- Added allocation-free `SignalMeter` telemetry to `audiorouter-dsp`: separate mono/stereo peak and RMS values, clipping count, finite -120 dBFS silence floor, and reset semantics. Sixteen DSP tests and strict Clippy pass; peak hold/window policy, graph/API integration, and live scheduler publication remain open.
- Added prepared `VoiceChain` integration over the built-in DSP primitives: EQ, gate, compressor, delay, sample-peak limiter, and signal meter execute in a declared order with construction-time state and no processing allocation. Seventeen DSP tests and strict Clippy pass; engine graph publication, parameter API, and live scheduling remain open.
- Added the engine-side `VoiceChainBlockProcessor`, which bridges fixed-shape planar `AudioBlock`s through construction-time interleaved scratch into the prepared DSP chain, rejects shape changes, and exposes reset/meter access. Engine coverage is 36 tests with strict Clippy; runtime graph publication and native scheduler integration remain open.
- Started M05 with a React/Vite/TypeScript UI shell consuming the local contracts package. It provides accessible keyboard-focusable session/node/library/recording surfaces, responsive layout, CSP, and explicit disconnected/read-only plus muted/unarmed startup states. `npm run build` passes and the production-only audit is clean; transport bridge, live API state, graph editing, and packaged Windows shell remain open.
- Added M05 local node selection and inspector behavior: node cards support mouse/Enter/Space selection, the selected node is announced with `aria-current`, and enable/bypass/mute controls remain visibly disabled with an actionable disconnected explanation. `npm run build` and full npm audit pass; backend mutations and live graph state remain open.
- Added `GraphicEq::magnitude_db_at`, aggregating the response of its exact cascaded biquads for a single processing/visualization source of truth. Flat and +18 dB band reference checks pass; sixteen DSP tests and strict Clippy remain green.
- Corrected storage-backed status counting to query the durable SQLite session table rather than only hydrated memory, and sorted active IDs for deterministic responses. SQLite tests (20), compile checks, strict Clippy, and contract typecheck pass.
- Added an M05 typed UI backend snapshot seam: live adapters consume the shared client for status, discovery, and session reads, while the default disconnected adapter is local-only and exposes no mutation surface. UI typecheck and production build pass; transport connection, live events, and graph editing remain open.
- Wired the disconnected snapshot into the React shell with guarded mount-time hydration, keeping local fixtures as the safe initial state while making the selected-node view backend-snapshot driven. UI typecheck and production build pass; connection lifecycle, live events, and graph editing remain open.
- Added a typed local session picker to the M05 preview. It switches session-shaped presentation state without invoking control mutations or audio/device operations; UI typecheck and production build pass.
- Added plan-only UI candidate helpers for node enabled/bypass flags with deterministic change descriptions and unknown-node rejection. They preserve the authoritative revision and leave validation/commit to the backend; UI typecheck and production build pass.
- Corrected the TypeScript event contract to model structured state events, bounded replay, and explicit resync snapshots, then exposed read-only `UiBackend.subscribe` support for disconnected and live adapters. Contracts/UI typecheck, production build, and high-severity audit pass.
- Added bounded `SnapshotCache` reconnect semantics for M05: preserve the last successful snapshot as stale on refresh failure, surface the error, and clear staleness only after a successful refresh. No edit queue or audio fallback is introduced; contracts/UI checks and audit remain green.
- Wired stale snapshot state into the React shell with an accessible status message, preserving the last known view during future reconnect failures. UI typecheck and production build pass.
- Added M05 Vitest fake-backend coverage: four tests cover disconnected snapshots/events, failed-refresh stale retention, revision-preserving drafts, and unknown-node rejection. UI tests, typecheck, production build, and high-severity audit pass.
- Added M05 reduced-motion and forced-colors CSS behavior with non-color selection outlines and a labeled session picker, improving keyboard/screen-reader presentation without native or audio effects. UI tests, typecheck, production build, and high-severity audit pass.
- Typed the M05 route explanation contract (`RouteInspection`/`RoutePath`) and exposed read-only route inspection through disconnected/live UI backends, preserving backend authority and avoiding invented paths. Contracts/UI tests, typecheck, production build, and audit pass.
- Added shared typed recording-library row/state models aligned with durable storage, including format/shape, metadata, missing state, and terminal failure. This exposes schema groundwork without renderer file operations; contracts/UI tests, typecheck, production build, and audit pass.
- Added backend-derived audio/storage/session status facts to the M05 visible status strip, retaining explicit disconnected preview values and avoiding inferred readiness. UI tests, typecheck, production build, and high-severity audit pass.
- Selected and downloaded the official Steinberg VST3 SDK with all submodules at pinned commit `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` into the ignored local cache; its checked-in license is MIT. VST3 x64 remains the explicit boundary, with VST2/x86 unsupported. SDK CMake example build is pending because Visual Studio 18 has MSVC/MSBuild but no CMake executable; no plugins or audio configuration were installed/changed.
- Added `audiorouter-plugin-host` portable non-executing inspection groundwork: canonical configured-root containment, VST3/VST2 classification, bounded PE/x64 validation, SHA-256 identity, and three-failure deliberate-retry quarantine policy. Three plugin-host tests and strict Clippy pass; worker process execution/IPC and plugin loading remain open.
- The post-plugin integration `cargo test --workspace` suite passes across all workspace crates (142 unit tests plus doc-test targets); plugin-host strict Clippy remains green. Native plugin execution, worker IPC, and CMake SDK example build remain open.
- Extended `audiorouter-plugin-host` with bounded finite `WorkerFrame` validation and monotonic deadline/sequence guards for future isolated workers. Four plugin-host tests and strict Clippy pass; shared-memory transport and plugin execution remain open.
- Added bounded explicit-root plugin directory enumeration to `audiorouter-plugin-host`, retaining invalid/unsupported candidates as visible results and refusing more than 256 candidates. Five plugin-host tests and strict Clippy pass; disposable process scanning and plugin execution remain open.
- Added explicit scanner cancellation and ten-second deadline controls, checked even for empty roots, to the plugin-host discovery boundary. Six plugin-host tests and strict Clippy pass; disposable process execution and plugin loading remain open.
- Added a regression for the 256-candidate scan budget: 257 candidates are rejected before inspection. Seven plugin-host tests and strict Clippy pass; disposable process execution and plugin loading remain open.
- Added explicit `WorkerFailurePolicy` protected-path semantics: worker failures silence protected paths and permit dry fallback only for explicitly unprotected paths. Eight plugin-host tests and strict Clippy pass; worker process execution remains open.
- Added bounded versioned `PluginStateAsset` handling with SHA-256 integrity verification before restore; empty, oversized, mismatched, and tampered state are rejected. Nine plugin-host tests and strict Clippy pass; durable asset storage and plugin-specific serialization remain open.
- Added durable SQLite `PluginStateRecord` storage with validated plugin/state hashes, version/path/size metadata, plugin filtering, and metadata-only removal. Storage tests (22) and strict Clippy pass, including persistence coverage; asset file I/O and plugin-specific serialization remain open.
- Corrected quarantine accounting to use a ten-minute rolling failure window with an injectable clock. The new test compiles and strict Clippy passes, but Windows Application Control blocks launching this generated test executable with OS error 4551; runtime verification remains pending without weakening policy.
- Added compile-validated `WorkerSupervisor` lifecycle policy: VST3/x64 eligibility, heartbeat timeout, failure/quarantine transition, and deliberate retry reset. Strict Clippy passes; runtime test launch remains blocked by Windows Application Control OS error 4551.
- Corrected plugin compatibility reporting so arbitrary `.dll` files remain `Unknown` instead of being falsely labeled VST2; only `.vst3` receives VST3 classification. Nine plugin-host tests and strict Clippy pass.
- Hardened durable plugin-state metadata to require absolute asset paths, preventing ambiguous later resolution. Storage tests (23) and strict Clippy pass.
- Added compile-validated fixed-capacity `BoundedFrameQueue` worker handoff with overflow ownership/counter and no waiting. Strict Clippy passes; runtime test launch remains blocked by Windows Application Control OS error 4551.
- Added compile-validated safe plugin state asset file I/O: canonical approved-root containment, safe IDs, exclusive creation, flush-to-disk, bounded size, and version/hash verification on read. Strict Clippy passes; plugin-host runtime launch remains blocked by Windows Application Control OS error 4551.
- Added explicit plugin capability classification: only VST3/x64 identities are supported; valid PE files with unknown/legacy formats remain unsupported. The new test compiles and strict Clippy passes; runtime launch remains blocked by OS error 4551.
- Added compile-validated bounded plugin parameter automation with finite 0..1 normalized values, 2048-frame offsets, fixed queue capacity, and overflow ownership/counters. Strict Clippy passes; plugin-host runtime launch remains blocked by OS error 4551.
- Repaired the ignored portable CMake 4.4.0 cache and successfully configured/built the pinned VST3 SDK with Visual Studio 18 2026 x64, MSVC 19.51, and Windows SDK 10.0.28000.0. SDK validator self-test: 51 passed; built VST3 validator: 94 passed. Upstream VSTGUI warnings and optional EXPAT/LIBJACK/AAX gaps are recorded; no system plugins or audio configuration were changed.
- Corrected plugin inspection for real VST3 bundles by resolving `Contents/x86_64-win` binaries and preserving bundle/binary identity paths. The new test compiles and strict Clippy passes; runtime plugin-host tests remain blocked by Windows Application Control OS error 4551.
- Ran the locally built `mda-vst3.vst3` sample through the official SDK validator: exit code 0, with 1,598 tests passed and 0 failed. The sample remained in the local build directory; no system plugin installation or machine audio configuration changed.
- Closed a real plugin-state integrity gap: `read_state_asset` now requires and compares the durable expected SHA-256, with regressions for wrong metadata and post-write tampering. The plugin-host suite passes all 16 tests and strict Clippy; no plugin was executed or installed by this slice.
- Hardened plugin-state writes to validate the asset's own digest before exclusive creation, rejecting caller-side mutation before corrupt bytes reach disk. The 16-test plugin-host suite and strict Clippy pass.
- Full validation checkpoint: `cargo test --workspace` passes across every Rust crate (including 16 plugin-host tests and native transport coverage); UI Vitest passes 4 tests, TypeScript typecheck passes, and the Vite production build passes. No audio endpoint or persistent machine configuration was changed.
- Added a bounded framed control protocol for the future disposable plugin worker (`Hello`, `Ready`, `Process`, `Processed`, `Shutdown`, `Failure`). Decoding revalidates worker frame and parameter limits; malformed/oversized inputs are rejected. Plugin-host tests now pass 18 tests with strict Clippy; process IPC and plugin execution remain open.
- Hardened worker `Hello` negotiation to require protocol version 1, a valid 64-hex plugin fingerprint, and mono/stereo channels; empty failure codes are rejected. Plugin-host tests now pass 19 tests with strict Clippy; native worker IPC and plugin execution remain open.
- Added a stateful `WorkerSession` gate requiring identity/channel handshake before activation and monotonic, non-expired frames until shutdown/failure. Plugin-host tests now pass 20 tests with strict Clippy; native process creation, OS isolation, and plugin execution remain open.
- Added bounded `WorkerLatency` reports and a `Latency` message with validated 8–192 kHz rates, ten-second maximum declarations, sample-to-millisecond conversion, and dynamic active-session updates. Plugin-host tests now pass 21 tests with strict Clippy; values are protocol evidence, not native measurements.
- Added typed UI `planGraph`/`commitGraph` backend operations. Live adapters route candidates and authoritative revisions through the shared API; disconnected mode rejects mutations explicitly. UI tests (4), typecheck, and production build pass; the rendered editor still needs mutation controls and conflict UX.
- Added the UI `applyGraphDraft` two-phase workflow: plan the candidate, verify the returned base revision, then commit with the caller's idempotency key. Revision mismatches stop before commit. UI tests now pass 6 cases with typecheck and production build green.
- Made the React shell accept an injected `UiBackend` and wired Reconnect to refresh its owned `SnapshotCache`, preserving stale state on refresh failure. UI tests (6) and production build pass; the default remains disconnected until a transport is supplied.
- Added the first real built-in pitch implementation using MIT-licensed `pitch_shift` 2.1.0's phase-vocoder API. `PitchShifter` enforces the required semitone/cent bounds, preserves offline frame count, reports 1,024-frame algorithmic latency, sanitizes non-finite input, and supports bypass. DSP tests now pass 19 cases with strict Clippy; realtime streaming and full 60-second/voice-quality acceptance remain open.
- Executed the dedicated 60-second pitch-duration check at both −12 and +12 semitones: the combined test completed in 34.6 seconds, and each run preserved 2,880,000 frames exactly while staying finite. Realtime streaming, speech quality, and native W2 latency measurements remain open.
- Added FLAC STREAMINFO inspection to the recording library. Valid `.flac` files now expose channels, sample rate, bit depth, frame count, and size; malformed files remain listable as invalid. Recording tests now pass 15 cases with strict Clippy; streaming FLAC worker integration remains open.
- Bounded the explicitly batch-only FLAC encoder to ten minutes of frames, returning `TooManyFrames` before temporary sample-buffer growth can exceed the declared limit. Recording tests (15) and strict Clippy pass; incremental FLAC output remains open.
- Added `BufferedFlacRecorder` to connect the bounded queue and lifecycle state to valid batch FLAC output, with contiguous-frame and terminal-failure handling. Recording tests now pass 16 cases with strict Clippy; true incremental FLAC output remains open.
- Added preparation-time engine latency compensation: branch spreads are converted to per-path sample delays and rejected above the declared 250 ms budget or outside 8–192 kHz. Engine tests now pass 37 cases with strict Clippy; runtime delay insertion and native latency measurement remain open.
- Added preallocated engine `FixedDelay` processing for the calculated compensation samples, with per-channel ring state, reset, shape checks, and no process-time allocation. Engine tests now pass 38 cases with strict Clippy; scheduler graph insertion and physical latency evidence remain open.
- Added exact-stream plugin-worker I/O helpers with pre-allocation size checks, fragmented-read handling, write flushing, and a chunked-reader regression. Plugin-host tests now pass 22 cases with strict Clippy; native worker process creation and shared-memory audio transport remain open.
- Added `audiorouter-plugin-worker`, a disposable process protocol fixture that negotiates identity/channels, validates and echoes process frames, reports latency, and shuts down cleanly. The process integration test and 22 library tests pass with strict Clippy; it does not load plugins or open audio devices.
- Added reusable `WorkerProcess` supervision around the disposable worker: bounded Hello/Ready negotiation, channel/sequence-checked frame exchange, latency forwarding, graceful shutdown, and kill-on-drop cleanup. The process integration now exercises this client; 22 library tests, integration tests, and strict Clippy pass. Plugin loading, OS sandbox policy, and shared-memory transport remain explicit blockers.
- Added a fixed, versioned `SharedAudioLayout` contract for the future worker mapping: bounded mono/stereo slots, sequence/deadline metadata, little-endian samples, and corruption/finite-value rejection. Plugin-host coverage is now 23 tests with strict Clippy; this defines the shared-memory payload format but does not claim OS mapping, synchronization, or plugin execution.
- Added file-backed `SharedAudioRegion` mapping over that layout, with explicit absolute paths, exclusive creation, reopen support, bounded map sizing, flush, and safe read/write operations. Plugin-host coverage is now 24 tests with strict Clippy; cross-process synchronization/ownership protocol and plugin execution remain open.
- Added an acquire/release epoch protocol to `SharedAudioRegion`: one writer reserves a slot, readers reject empty/busy slots, and readers detect a changed epoch instead of accepting a torn frame. Plugin-host coverage remains 24 tests with strict Clippy; worker integration and realtime transport wiring remain open.
- Hardened `SharedAudioRegion` against stale producers by requiring strictly increasing frame sequence numbers on every rewrite. The regression, 24 plugin-host tests, and strict Clippy pass; cross-process worker wiring and plugin execution remain open.
- Added caller-owned `SharedAudioLayout::read_into` and `SharedAudioRegion::read_into` decoding, returning validated sequence/deadline metadata without allocating an audio sample vector. The 24-test plugin-host suite, worker integration, and strict Clippy pass; realtime scheduling and plugin execution remain open.
- Added the M07 AUTO-04 CLI plan-file workflow: `graph plan` validates a candidate and writes a versioned local preview, `graph inspect` reads it without side effects, and `graph apply` reloads the current session revision, rejects stale plans, replans, and commits with an idempotency key. The CLI/control tests pass; no audio or machine configuration is changed.
- Added the AUTO-04 `node set` convenience command. It loads one session, changes one JSON scalar node parameter, hydrates the authoritative backend session, then uses the existing plan/commit path with an explicit idempotency key. CLI tests pass; no audio or machine configuration is changed.
- Added true `node set --dry-run` behavior: scalar input and the complete candidate are validated without creating a durable graph plan or changing the session revision; normal edits still require an idempotency key and use plan/commit.
- Added optional bounded cursor paging to `recordings.list`; unpaged callers retain the legacy array, while paged callers receive `{items,nextCursor}`. The UI normalizes either shape, and control/contracts/UI validation pass without reading audio files.
- Added `RuntimeProcessor::activate_session`, which prepares a supported session before publishing it and retains the previous generation when preparation fails. Engine coverage now includes the failed-activation rollback boundary; native device scheduling remains open.
- Exposed recording-list paging through the CLI with `recordings list --limit N [--cursor ID]`; existing unpaged CLI output remains unchanged and paged requests use the shared control dispatcher.
- Strengthened recording pagination coverage with a two-record control regression proving cursor advancement and terminal-page behavior; control clippy remains clean.
- Replaced generic recording discovery output schemas with explicit recording item fields, format/sample-rate/state enums, metadata bounds, and the paged-list envelope. Discovery assertions and contracts typecheck pass; no recording content is accessed.
- Re-ran the checked-in M01 CLI acceptance script plus M08 artifact-verifier and qualification-command regressions at the current tip; all passed with temporary state cleaned up and no audio configuration changes.
- Full locked workspace regression passed at the current tip: Rust unit/integration/doc tests, 23 UI tests, contracts/UI typechecks, and the Vite production build all succeeded; no audio endpoint or persistent machine setting was touched.
- Added UI regression coverage for normalizing the paged recording response into the existing row-list view model; UI behavior remains compatible with both API response forms.
- Updated the focused MCP `list_recordings` tool schema to advertise optional recording cursors and bounded limits, with regression assertions for the published fields; authorized dispatch behavior is unchanged.
- Moved paged recording reads behind a bounded SQLite query using the stable `(startTime, id)` ordering, avoiding loading the entire recording library before slicing. Storage/control tests and strict Clippy pass.
- Extended `session list` with optional stable `--cursor` pagination while preserving the legacy flat response when no cursor is supplied. The CLI page path uses the backend's authoritative `sessions.list` dispatcher and returns `{items,nextCursor}`; CLI/MCP tests pass without audio or machine changes.
- Extended the headless `history` command with optional revision cursors, returning the same bounded `{items,nextCursor}` envelope as `graph.history` while retaining the legacy flat response without a cursor. CLI validation and MCP interoperability remain green.
- Hardened M07 event reconnect semantics: `events.subscribe` accepts the caller's `backendEpoch` and returns a bounded snapshot with `resyncRequired` when the backend epoch differs, before replaying a stale cursor. Control tests now pass 42 cases with strict Clippy; native lifecycle restart evidence remains open.
- Hardened M07 recovery backups so `Storage::backup_to` refuses any existing destination, including regular files, preserving prior recovery copies against overwrite races. Storage tests now pass 24 cases with strict Clippy; retention scheduling and corrupted-database recovery UX remain open.
- Added the first M07 MCP adapter slice as `audiorouter mcp serve`: newline-delimited stdio JSON-RPC, pinned protocol negotiation (`2025-06-18`), focused read tools, schema-discoverable `call_api`, and enrolled-client authorization through the existing control dispatcher. CLI/control tests pass; resources, notifications, durable backend transport, and full MCP client interoperability remain open.
- Extended the M07 MCP adapter with three read-only resources: capabilities, redacted diagnostics, and headless workflow guidance. Resource discovery/read shares enrolled-client authorization and no audio/raw recording data is exposed. CLI tests and strict Clippy pass; live MCP client interoperability remains open.
- Added an optional M07 MCP `--pipe` proxy mode. Tool/resource API reads can now be forwarded as framed JSON-RPC to the authenticated local Windows named-pipe backend, while the existing local storage mode remains explicit. The adapter does not open audio devices or modify machine settings; static compilation and strict Clippy pass, but the rebuilt CLI test executable was blocked by Windows Application Control OS error 4551.
- Added M08 release-preparation tooling at `tools/release/prepare-artifacts.ps1`. It builds the CLI and plugin worker from locked inputs, emits Cargo metadata as an SBOM, generates SHA-256 artifact metadata, and explicitly marks output unsigned/not publication-ready with driver/signing/install blockers. Script syntax, formatting, and diff checks pass; the attempted release build was blocked by Windows Application Control OS error 4551 before artifacts were produced.
- Added the M08/M07 headless operator runbook at `docs/operations/headless-runbook.md`, covering verified CLI plan/apply, backup/restore, MCP stdio/pipe launch, stale-plan recovery, and explicit native-driver/signing boundaries. Documentation link/text checks and `git diff --check` pass.
- Added focused MCP `plan_graph_change`, `apply_graph_change`, and `control_session` tools, mapped directly to the authorized graph/lifecycle API with revision, idempotency, and role enforcement. CLI tests pass 7 cases with strict Clippy.
- Fixed live application alias parity by caching one bounded 100 ms process snapshot per control plane; `apps.list` and `applications.list` no longer perform two race-prone enumerations for adjacent requests. Control tests pass 44 cases with strict Clippy.
- Added typed read-only CLI conveniences `diagnostics` and `operation get`, both routed through the shared JSON-RPC dispatcher and documented in help. CLI coverage now passes 8 tests with strict Clippy.
- Tightened recording privacy permissions: `recordings.list` and `recordings.get` now require explicit `Record` scope because their metadata includes absolute paths; MCP descriptions state the same requirement. Read-only denial coverage passes with the existing 44 control and 7 CLI tests.
- Added read-only `recordings.list` to the shared API discovery/authorization/dispatch path, backed by persisted SQLite recording metadata and exposed as the MCP `list_recordings` tool. Control coverage is now 43 tests and CLI coverage 7 tests; strict Clippy passes. No audio files are read or changed by this slice.
- Added read-only `recordings.get` storage/API dispatch and the MCP `get_recording` tool, preserving stable recording identity and metadata without reading file contents. Targeted storage/control/CLI tests pass (24/43/7) with strict Clippy.
- Added authorized `recordings.setMetadata` mutation with explicit `Record` scope, bounded metadata validation, and path-preservation coverage. Targeted storage/control/CLI tests pass (24/44/7) with strict Clippy; recording content and file actions remain outside this slice.
- Added authorized `recordings.removeEntry` mutation and MCP `remove_recording_entry`; it removes only the SQLite library row, returns `fileAction: none`, and requires explicit `Record` scope. Targeted storage/control/CLI tests pass (24/44/7) with strict Clippy.
- Added CLI parity for the recording library: `recordings list`, `recordings get`, and `recordings remove-entry` use the shared control dispatcher, require an absolute SQLite database path, and explicitly preserve the underlying file on removal. The CLI recording regression, strict Clippy, formatting, and diff checks pass.
- Aligned graph-plan expiry with API-05: the default plan lifetime and discovered response metadata are now five minutes (300,000 ms), while short TTLs remain available for deterministic tests.
- Added durable graph-plan retention for SQLite-backed control planes. Uncommitted plans survive backend restart, are revalidated against the hydrated session/revision before commit, expire through bounded storage cleanup, and are removed after successful commit. Control coverage is now 45 tests; storage coverage remains 24, with strict Clippy and formatting green.
- Added `operations.cancel` API/CLI/MCP parity. Since the current backend exposes only completed operations, cancellation now returns an explicit `alreadyCompleted`/`cancelled: false` result without reversing graph or file side effects; unknown operation IDs remain actionable errors. Domain/control/CLI tests and strict Clippy pass.
- Hardened session deletion to remove durable graph plans for the deleted session in the same SQLite transaction as current and history rows. This prevents orphaned candidates from surviving resource deletion; storage and control regressions plus strict Clippy pass.
- Added non-destructive `recordings.preview` API/CLI/MCP parity over the existing WAV/FLAC header inspector. It reports present, missing, or invalid status plus format/frame metadata without decoding audio or modifying files; recording/control/CLI tests, strict Clippy, and full workspace validation pass.
- Added typed recording metadata editing parity: `recordings set-metadata` preserves unspecified fields before dispatching the authorized update, and MCP exposes `set_recording_metadata`. Path/audio content remains unchanged; control/CLI tests and strict Clippy pass.
- Added authorized recording rename parity through API, CLI, and MCP. Renames are restricted to existing regular WAV/FLAC files and their current canonical directory, refuse collisions, and update SQLite only after the filesystem operation succeeds. Storage/control/CLI tests and strict Clippy pass.
- Hardened API-05 graph commits with a strict nullable `acknowledgments` schema. Warning IDs are bounded and type-checked, and non-empty acknowledgments are rejected when the current plan exposes no warnings; they cannot grant permission or bypass stale-plan checks. Control/domain/CLI tests and strict Clippy pass.
- Added durable `safety.setPrivacyMute` API/CLI/MCP parity with explicit Capture authorization, restart persistence, status reporting, and privacy state events. The latch does not alter Windows privacy settings or open audio; control/storage/CLI tests and strict Clippy pass.
- Hardened privacy-mute startup recovery to fail closed when the durable latch cannot be read, and exposed the same muted/persistence state through diagnostics. Control/CLI tests, strict Clippy, formatting, and diff checks pass.
- Rebuilt the native WASAPI probe with the installed Visual Studio Community 2026, Windows SDK 10.0.28000.0, and WDK toolchain. A 200 ms shared capture on endpoint 0 completed start/read/stop/reset with 10 packets and 4,800 frames; process-loopback capture at 44.1 kHz completed asynchronously with 50 packets, 22,050 frames, and nonzero payload. Temporary binaries were removed; no defaults, volume, mute, privacy, driver, or persistent audio configuration changed.
- The same native probe completed a 200 ms silent render run on endpoint 0: shared initialization, start, 13,920 silent frames submitted, stop, and reset all succeeded. This validates the non-audible render lifecycle; the temporary binaries were removed immediately afterward.
- Completed the M05 editor mutation slice: connected UI sessions now expose enabled/bypass draft controls, discard, and the two-phase plan/commit action with surfaced backend conflicts/errors; disconnected startup remains read-only and all mutation controls stay disabled. UI tests (6), TypeScript typecheck, production build, and diff checks pass.
- Added M05 live event cursor handling: connected UI polling sends the known backend epoch and bounded replay cursor, refreshes authoritative snapshots on events or resync, and preserves stale-state errors on subscription failure. Disconnected mode performs no polling; UI tests now pass 7 cases with typecheck, production build, and diff checks green.
- Added M05 read-only route inspection to the rendered editor. It queries the authoritative backend for the selected destination, displays reachable paths or an explicit unavailable state, and never invents routes; UI tests (7), typecheck, production build, and diff checks pass.
- Added authorized `recordings.reveal` API/CLI/MCP parity. It resolves only a persisted recording path, reports missing files without launching anything, and on Windows asks Explorer to select an existing regular file; domain/control/CLI tests and strict Clippy pass.
- Added separately authorized `recordings.recycle` API/CLI/MCP parity with preview-by-default and explicit confirmation. Confirmed Windows actions use the OS Recycle Bin and mark the library row missing; missing files and unsupported platforms return explicit non-destructive results, with no permanent-delete fallback. Storage/domain/control/CLI tests and strict Clippy pass; control coverage is now 49 tests.
- Added honest read-only `startup.get` capability reporting across the API, CLI, and MCP surfaces. Sign-in startup registration remains explicitly unavailable in this build; no startup or machine configuration is changed.
- Added explicit headless `backup --database ... --output ...` and `restore --backup ... --database ...` commands over the validated SQLite recovery primitives. Both require absolute new destinations and restore checks source integrity; CLI tests and strict Clippy pass without opening audio.
- Wired persisted graph node parameters into domain validation and engine compilation: Gain now honors bounded `gainDb` values, Mute honors `muted`, unknown/invalid processor parameters are rejected, and absent parameters retain safe defaults. Domain/engine tests and strict Clippy pass; native scheduling remains open.
- Extended the M05 connected editor with typed Gain parameter drafts. The Gain dB control is disabled while disconnected, stays local until plan/commit, and contributes deterministic parameter paths to graph plans; UI tests (8), typecheck, production build, and diff checks pass.
- Added the connected editor's graph-level Mute parameter control (`muted`) with deterministic draft coverage. Privacy mute remains a separate disabled safety action; UI tests (9), typecheck, production build, and diff checks pass.
- Added a versioned recording-controller checkpoint/restore format for crash recovery. It serializes only validated lifecycle boundaries (never queued audio or file handles), rejects corrupt/inconsistent state, and passes 18 recording tests with strict Clippy; durable worker journal integration remains open.
- Added SQLite persistence for versioned recording checkpoints with atomic replace, typed restore validation, explicit corruption errors, and clear semantics. Storage coverage is now 26 tests with strict Clippy; integrating checkpoint writes into a live recorder worker remains open.
- Exposed authorized read-only `recordings.recovery` through API, CLI, and MCP. It returns validated checkpoint metadata or an explicit missing result without reading audio or touching files; control coverage is now 51 tests and CLI coverage 12 tests with strict Clippy.
- Hardened recording-entry removal to delete its durable recovery checkpoint in the same SQLite transaction, preventing orphaned recovery state while preserving the non-destructive file boundary; storage coverage remains 26 tests with strict Clippy.
- Connected live WAV and bounded FLAC recorder workers to the versioned recovery boundary: each drained chunk advances the controller checkpoint to its committed end frame, and both worker types expose snapshots without audio buffers or file handles. Recording coverage remains 18 tests with strict Clippy; durable persistence scheduling and true incremental FLAC remain open.
- Added an explicit read-only SQLite integrity check during storage open. Malformed or damaged databases now return `CorruptDatabase` before migrations run, preserving the source for an explicit backup/restore workflow; storage coverage is 27 tests and strict Clippy is green.
- Added bounded WAV RIFF `LIST/INFO` metadata finalization for title, artist, and comment, with control/size validation and a worker-facing `finish_with_metadata` seam. Existing no-metadata WAV output remains unchanged; recording coverage is now 20 tests with strict Clippy. FLAC tags and durable metadata scheduling remain open.
- Added bounded FLAC Vorbis-comment metadata finalization for title, artist, and comment, plus the corresponding buffered-worker seam. Default FLAC output remains unchanged and the 10-minute batch bound is retained; compilation and strict Clippy pass, while the rebuilt test executable is blocked by Windows Application Control OS error 4551.
- Added `createLiveBackendFromTransport` to the M05 UI, constructing the typed API client directly from the host-provided framed transport while preserving the disconnected safe default. UI tests (10), typecheck, production build, and diff checks pass; native WebView/pipe injection remains a host integration task.
- Hardened M08 release provenance: the preparation script now emits the complete locked Cargo dependency graph rather than `--no-deps`, and records the exact 40-character source revision in `release-manifest.json`. PowerShell parsing and full locked metadata validation pass; artifact build/signing/install gates remain open.
- Hardened M08 release input integrity: artifact preparation now refuses tracked or untracked working-tree changes before building, preventing release hashes and SBOMs from representing an uncommitted source state. PowerShell parsing and diff validation pass.
- Aligned the shared TypeScript contract maps with the implemented cancellation, recording library/recovery, privacy-mute, startup, and event-epoch APIs. Contracts typecheck; dependent UI typecheck, 10 tests, and production build pass.
- Connected the M05 recording panel to the authorized session-scoped `recordings.list` API. Connected views show persisted title/state/missing/path metadata; disconnected startup remains empty and non-mutating. UI coverage is now 11 tests with typecheck and production build green.
- Corrected M05 recording status UX so authorization/backend failures render as an explicit unavailable state instead of an incorrect zero-file result; genuinely empty and disconnected states remain distinct. UI typecheck, 11 tests, and production build pass.
- Added a connected-only read-only recording Preview action in M05, routed through `recordings.preview`; it reports format/status inspection without decoding or modifying files, and disconnected mode rejects it explicitly. UI typecheck, 11 tests, and production build pass.
- Added direct UI transport regression coverage for `recordings.preview`; the live backend forwards the recording ID to the authorized read-only method and preserves its result. UI coverage is now 12 tests with typecheck and production build passing.
- Integrated WAV RIFF metadata with the recording library: registration now reads bounded title/artist/comment INFO tags while skipping audio payloads, preserving valid files even when tags are malformed. Recording runtime coverage is now 21 tests with strict Clippy; FLAC metadata indexing remains open.
- Integrated FLAC Vorbis comments with recording-library registration. Bounded metadata parsing skips audio frames and ignores malformed values while preserving valid files; encode/inspect/register coverage passes in the 21-test recording suite with strict Clippy.
- Closed the remaining TypeScript graph-commit schema drift by adding nullable bounded warning acknowledgments, matching Rust discovery and validation. Contracts typecheck; dependent UI typecheck, 11 tests, and production build pass.
- Added an open-time bounded retention sweep for expired idempotency journals and graph plans, preventing stale recovery rows from accumulating while leaving sessions/files untouched. Storage coverage is now 28 tests with strict Clippy.
- Propagated corrupt-database failures through the control error envelope as stable non-retryable `corruptDatabase` responses with restore guidance. Control coverage is now 52 tests and strict Clippy is green.
- Added a caller-owned recorder checkpoint persistence hook for both WAV and bounded FLAC workers. The hook runs after each contiguous chunk advances the validated lifecycle boundary and fails the worker if persistence fails; recording coverage is now 23 tests with strict Clippy and formatting green. True incremental FLAC encoding and native realtime integration remain open.
- Added M05 connected UI recovery inspection, metadata editing, non-destructive library-entry removal, bounded metadata search, accessible graph list view, resource-driven session navigation, privacy-mute control, and explicit graph-warning acknowledgment. UI validation passes with 17 tests, TypeScript typecheck, and production build; disconnected mode remains non-mutating.
- Revalidated the full Rust workspace: all unit/integration/doc tests, strict workspace Clippy, and formatting checks pass. The checked-in offline M01 CLI acceptance script also passes with temporary data; no audio endpoint or machine configuration was changed.
- Added a real M07 MCP stdio interoperability regression that launches the CLI binary, enrolls a temporary observer client in temporary SQLite state, completes initialize, tools/list, and resources/list, and exits cleanly on EOF. The process test, CLI Clippy, formatting, and diff checks pass; native pipe interoperability remains separate.
- Expanded the MCP process regression to read the diagnostics resource and call the authorized read-only get_startup tool. The complete stdio exchange remains green and still uses only temporary local state; native pipe interoperability remains separate.
- Extended the MCP process regression with an observer graph-mutation attempt; the real stdio server returns an MCP tool error while the read-only calls remain successful. This proves authorization is preserved across the external process boundary.
- Revalidated the complete Rust workspace after the M06/M07 slices: all unit, integration, and doc-test targets pass, including 25 plugin-host unit tests, 4 worker process tests, 13 CLI unit tests, and the MCP stdio process test. Strict Clippy and formatting remain green; no audio endpoint or machine configuration was changed.
- Integrated the bounded rolling RMS/peak-hold meter into `VoiceChain` using the documented 300 ms RMS and 1 s peak-hold defaults while preserving the existing `MeterSnapshot` API. DSP tests, strict Clippy, and formatting pass; graph publication and native scheduling remain open.
- Added the SDK/native-toolchain setup runbook. It explains that the VST3 GitHub repository is a source SDK rather than an installer, records the pinned local checkout and validation paths, and documents the verified Windows SDK/WDK versions and no-audio-configuration safety boundary.
- Added SharedAudioTransport, a caller-owned paired input/output mapping over the existing bounded shared-memory slots, with allocation-free read_into access and a two-endpoint exchange regression. Plugin-host targets compile and strict Clippy passes; runtime test execution remains blocked by Windows Application Control OS error 4551.
- Hardened SharedAudioTransport creation with rollback when the output slot cannot be created, preventing an orphaned input mapping; the regression verifies cleanup. Plugin-host all-target compilation and strict Clippy pass.
- Extended the disposable worker protocol with bounded shared-frame messages, added SharedAudioTransport paths to WorkerProcess, and wired the worker fixture to read/write mapped input/output slots. The process-level shared-audio regression passes (2 tests), all plugin-host targets compile, and strict Clippy is green; plugin loading and OS sandboxing remain open.
- Re-ran the plugin-host library suite after the shared-worker changes: all 25 tests now execute and pass, including quarantine, state, framing, mapped transport, and worker-session coverage. The earlier Application Control runtime block is superseded for this crate; actual plugin loading and OS sandboxing remain open.
- Added the native M06 VST3 loader probe using the pinned SDK headers and installed VS/Windows SDK toolchain. It loaded the locally built mda-vst3 bundle, obtained GetPluginFactory, enumerated 68 classes, and cleanly released/unloaded the module; no processor, editor, audio device, or machine setting was touched.
- Extended the native VST3 loader probe to instantiate and initialize the first audio component, inspect its one-input/one-output bus layout, then terminate/release it and unload the module. This is synthetic non-audio component evidence; processing, editor behavior, worker integration, and sandbox enforcement remain open.
- Extended the native VST3 probe with bounded offline processing at 48 kHz/64 frames. The real component processed a stereo block and produced finite output for all samples before clean shutdown; no audio device or user audio was involved. Editor behavior, worker integration, and sandbox enforcement remain open.
- Extended the native VST3 probe to resolve and initialize the component controller; the real mda component exposed 5 parameters before clean controller shutdown. This is parameter-surface evidence without a native editor or user audio; parameter automation through AudioRouter and sandbox enforcement remain open.
- Extended the native VST3 probe to exercise all 5 controller parameters: read normalized values, set each to 0.5, validate finite/in-range results, and restore originals. The synthetic automation pass succeeded without an editor, audio device, or user audio.
- Extended the native VST3 probe with an in-memory component state round trip; the real mda component emitted and accepted a 180-byte opaque state payload. No state file, editor, audio device, or user audio was accessed.
- Hardened WorkerProcess shutdown with a bounded timeout and kill-and-reap fallback, while retaining the five-second default shutdown path. All 25 plugin-host unit tests, both worker process tests, strict Clippy, and formatting pass; blocking reads during processing and full OS sandbox policy remain open.
- Wired WorkerSession enforcement into the disposable worker with an explicit worker-side Hello transition. Runtime process tests now reject duplicate sequence frames through the worker (3 process tests pass, with 25 unit tests and strict Clippy); shared deadlines still require a common clock and OS sandboxing remains open.
- Replaced the worker loop's zero deadline tick with a cross-process Unix-epoch millisecond clock and added an expired-frame process regression. Worker validation now rejects both duplicate sequence and expired deadline frames; 25 unit tests and 4 process tests pass with strict Clippy.
- Added the M08 release qualification checklist at a stable operational path. It documents the verified unsigned artifact flow, backup/install/uninstall expectations, recovery boundaries, and explicit native driver/signing/clean-machine blockers without presenting the repository as an installer.
- Added the missing read-only `watch <session-id> --database <path> [--after N] [--limit N]` CLI workflow over `events.subscribe`. Cursor and 1–500 limit validation remain backend-authoritative; CLI coverage is now 13 tests with strict Clippy.
- Qualified the unsigned M08 release flow from clean revision 5258346cf4002367723f05efd11a9bf1692507c0: locked release binaries and SBOM were generated, manifest hashes/byte counts were verified, and the temporary output was removed. Signing, driver, installer, and clean-machine gates remain open; no audio configuration changed.
- Revalidated the native VST3 SDK probe with the installed Visual Studio Community 2026/MSVC and Windows SDK toolchain: the pinned mda bundle loaded, 68 classes enumerated, bounded offline stereo processing produced finite output, all 5 parameters round-tripped, and 180-byte component state restored. Generated files were removed; no audio or machine configuration changed.
- Added explicit M07 filesystem backup retention: Storage now preserves all pre-migration/unrelated files and retains the newest ten direct audiorouter-backup-*.sqlite files; the headless backup prune command exposes this explicit maintenance action. Storage and CLI tests plus strict Clippy pass; recordings are never pruned.
- Added a cross-crate M04 recovery regression wiring the live WAV worker's checkpoint hook to SQLite: two committed chunks persist and reload boundary frame 103 without samples/file handles. Storage coverage is now 30 tests with strict Clippy; true incremental FLAC and native realtime integration remain open.
- Added a real incremental FLAC path: StreamingFlacWriter emits bounded verbatim frames as chunks arrive and patches STREAMINFO totals at finish; StreamingFlacRecorder connects it to queue/lifecycle/checkpoint handling. Decoding and worker tests pass (25 recording tests) with strict Clippy; compression tuning, streaming-path metadata, and native realtime integration remain open.
- Extended the incremental FLAC path with bounded Vorbis comments for title, artist, and comment metadata; a file round-trip test verifies metadata and frame count. Recording coverage is now 26 tests with strict Clippy; compression tuning and native realtime integration remain open.
- Bounded plugin-worker response reads: WorkerProcess now drains child stdout on a reader thread and applies a five-second receive timeout, with a regression proving a silent reader returns promptly. Plugin-host coverage is now 26 tests plus 4 worker-process tests with strict Clippy; OS sandboxing and production plugin execution remain open.
- Added Windows worker lifetime containment: WorkerProcess now fail-closes when it cannot assign a child to a kill-on-close Job Object, and owns that handle through shutdown/drop. Native worker process tests and strict Clippy pass; filesystem/network sandbox policy and production plugin execution remain open.
- Revalidated the complete locked workspace after the latest M04/M06/M07 changes: all unit, integration, and doc tests pass across 14 CLI + MCP process, 52 control, 24 domain, 24 DSP, 38 engine, 26 plugin-host + 4 worker-process, 5 protocol, 26 recording, 30 storage, 14 transport, and 8 Windows-audio tests; strict Clippy, formatting, and diff checks are green. No audio configuration changed.
- Added conservative incremental-FLAC crash recovery: recover_streaming_flac_file validates metadata, scans complete bounded verbatim frames/CRCs, truncates an incomplete tail, and patches STREAMINFO counts. The temporary-file regression passes; recording coverage is now 27 tests with strict Clippy. Arbitrary third-party FLAC recovery remains unsupported.
- Hardened plugin state file boundaries: read/write now reject symlink roots and reads reject symlink assets before canonicalization, preventing approved-root symlink traversal. Plugin-host tests and strict Clippy pass; Windows reparse-point and full filesystem/network sandbox coverage remain open.
- Extended plugin state boundary checks to Windows reparse-point attributes (with symlink fallback on other platforms), so junctions and other tagged reparse roots/assets are rejected before canonicalization. Plugin-host tests, strict Clippy, formatting, and diff checks pass; nested reparse traversal and full filesystem/network sandbox policy remain open.
- Revalidated the locked workspace after the M04/M06 hardening: all unit, integration, worker-process, MCP, and doc tests pass, including 27 recording, 26 plugin-host, and 4 worker-process tests; workspace strict Clippy, formatting, and diff checks are green. No audio or machine configuration changed.
- Closed the nested plugin-state path traversal gap: reads now inspect each existing component between the requested asset and the approved canonical root for symlink/reparse metadata before opening. Plugin-host unit/worker tests and strict Clippy pass; full OS filesystem/network sandboxing remains open.
- Added a nested directory-link regression for plugin state reads; when the host permits link creation, an asset reached through the linked directory is rejected before opening. Plugin-host and worker-process tests plus strict Clippy pass; full OS filesystem/network sandboxing remains open.
- Closed a transport lifecycle gap: control-plane named-pipe connection and persistent-session servers now hold the same per-user singleton mutex as the generic server, preventing competing control backends on one pipe. The 14-test native transport suite and strict Clippy pass; unbounded daemon restart policy remains open.
- Hardened the connected UI event loop: it seeds the cursor from the authoritative snapshot and advances after event-triggered snapshot refresh succeeds, preserving retry behavior when the backend snapshot temporarily fails. UI tests (20), typecheck, and production build pass; live native transport and manual visual acceptance remain open.
- Restricted plugin state reads to the same direct-child layout used by state writes, in addition to root/asset reparse checks. This removes nested path ambiguity and closes the remaining state-file traversal surface; plugin-host/worker tests and strict Clippy pass. Full OS filesystem/network sandboxing for plugin execution remains open.
- Revalidated the complete locked workspace at the current tip after transport singleton and UI event-cursor changes: all unit, integration, worker-process, MCP, and doc tests pass, including 14 CLI + MCP process, 52 control, 24 domain, 24 DSP, 38 engine, 26 plugin-host + 4 worker-process, 5 protocol, 27 recording, 30 storage, 14 transport, and 8 Windows-audio tests; workspace strict Clippy, formatting, and diff checks are green.
- Tightened M04 checkpoint ordering: WAV and incremental-FLAC workers flush each encoded chunk before invoking the durable checkpoint hook, and flush errors fail the worker without claiming the boundary. Recording coverage (27 tests) and strict Clippy pass; OS-level sync and native realtime integration remain separate concerns.
- Added a flush-failure regression proving the WAV worker enters `Failed` and does not invoke checkpoint persistence when the destination flush fails. Recording coverage is now 28 tests with strict Clippy; OS-level sync and native realtime integration remain separate concerns.
- Hardened worker launch provenance: WorkerProcess now requires an absolute canonical regular executable and rejects symlink/reparse worker paths before spawning. Plugin-host coverage is now 27 unit tests plus 4 worker-process tests with strict Clippy; full OS sandboxing and actual plugin execution remain open.
- Closed worker launch cleanup holes: Job Object attachment and stdin/stdout extraction failures now explicitly kill and reap the spawned child, and normal Drop shares the same bounded cleanup helper. Plugin-host coverage (27 unit + 4 worker-process) and strict Clippy pass; full OS sandboxing and actual plugin execution remain open.
- Hardened M07 storage path boundaries with Windows reparse-point detection for backup destinations, restore sources, bundle inputs/staging roots, and recovery-retention directories. Storage coverage remains 30 tests with strict Clippy; nested deployment filesystem policy remains open.
- Closed dangling-link destination handling in backup/restore: destination presence is now checked with `symlink_metadata`, so broken symlinks cannot bypass `exists()` and be followed by SQLite. Storage tests (including conditional dangling-link coverage) and strict Clippy pass.
- Re-ran the native VST3 loader with the installed Visual Studio/Windows SDK toolchain against the pinned mda bundle: 68 classes loaded, real offline 64-frame stereo processing was finite, five parameters round-tripped, and 180-byte component state round-tripped. Generated files were removed; no audio endpoint or machine configuration changed.
- Extended M07 storage path hardening to parent directories: backup/restore reject reparse-point parents, and recording rename rejects reparse sources/parents before moving files. Storage coverage remains 30 tests with strict Clippy; broader deployment filesystem policy remains open.
- Hardened bundle export destinations too: export now rejects reparse-point parent directories and any existing path via `symlink_metadata`, including dangling links, before creating the archive. Storage coverage remains 30 tests with strict Clippy.
- Extended storage regression coverage to restore and bundle-export dangling destinations; both APIs now explicitly reject broken links when the host permits link creation. Storage coverage remains 30 tests with strict Clippy.
- Re-ran the read-only native WASAPI probe against all 13 active capture endpoints: each endpoint's own mix format passed shared `IsFormatSupported` and `IAudioClient::Initialize` with `NOPERSIST`. This confirms the prior `E_INVALIDARG` was request-format/flag-specific rather than endpoint contention; no stream was started/read and generated files were removed.
- Requalified the unsigned M08 release flow at the current clean revision with the installed VS2026 toolchain: optimized CLI and plugin-worker builds completed, the full locked Cargo SBOM and provenance manifest were generated, and the artifact verifier accepted all hashes and byte counts. Temporary output was removed; signing, driver, installer, and clean-machine gates remain explicitly open.
- Revalidated the complete locked workspace after the latest storage, WASAPI-evidence, and release checkpoints: all unit, integration, worker-process, MCP, and doc tests pass (14 CLI + MCP process, 52 control, 24 domain, 24 DSP, 38 engine, 27 plugin-host + 4 worker-process, 5 protocol, 28 recording, 30 storage, 14 transport, and 8 Windows-audio); strict workspace Clippy, formatting, and diff checks are green. No audio endpoint or machine configuration was changed.
- Added the portable STATE-10 crash-loop policy primitive: bounded crash timestamps expire after ten minutes, three recent crashes enter safe mode, and automatic recovery admits only previously running non-recording sessions. Domain coverage is now 27 tests with strict Clippy; supervisor persistence, restart orchestration, and OS lifecycle evidence remain open.
- Added bounded SQLite crash-marker persistence for the STATE-10 supervisor boundary: markers are retained only for the ten-minute window, capped to the three decisions needed by safe mode, reject unrepresentable timestamps, and can be explicitly cleared after a stable run. Storage coverage is now 32 tests with strict Clippy; supervisor wiring and automatic restart remain open.
- Corrected STATE-10 safe-mode semantics so three recent crashes latch safe mode until an explicit stable-run clear, even after crash timestamps expire. Domain coverage is now 28 tests with strict Clippy; supervisor wiring and automatic restart remain open.
- Revalidated the complete locked workspace after crash-policy and persistence changes: all unit, integration, worker-process, MCP, and doc tests passed, including 14 CLI + MCP process, 52 control, 28 domain, 24 DSP, 38 engine, 27 plugin-host + 4 worker-process, 5 protocol, 28 recording, 32 storage, 14 transport, and 8 Windows-audio tests; strict workspace Clippy, formatting, and diff checks are green.
- Extended M08 release preparation with a deterministic `THIRD-PARTY-NOTICES.txt` generated from the same locked Cargo metadata as the SBOM; the notice is included in manifest hashing and accepted by the artifact verifier. Signing, driver, installer, and clean-machine gates remain open.
- Requalified the updated M08 flow after adding dependency notices: optimized locked builds, SBOM, manifest, SHA-256/byte-count verification, and expected package-entry checks all passed; the temporary artifact directory was removed.
- Extended the release verifier regression to include `THIRD-PARTY-NOTICES.txt` alongside a binary artifact and to retain tamper rejection coverage; PowerShell parsing and the test harness pass.
- Added file-backed recovery-marker reopen coverage: crash markers survive reopening SQLite storage and are removed only by the explicit clear operation. Storage coverage is now 33 tests with strict Clippy; process-supervisor integration remains open.
- Bounded the portable crash tracker itself to the three markers needed by STATE-10 safe-mode decisions, preventing unbounded in-memory growth during a crash storm. Domain coverage is now 29 tests with strict Clippy; supervisor integration remains open.
- Persisted the STATE-10 safe-mode latch in SQLite when three recent crash markers are recorded, retained it across storage reopen, and cleared it only through the explicit recovery-clear operation. Storage coverage remains 33 tests with strict Clippy; process-supervisor restart wiring remains open.
- Exposed the persisted STATE-10 safe-mode latch through read-only `status.get` and `system.diagnostics`; a storage-backed control regression verifies both surfaces report the latched state without activating audio. Control coverage is now 53 tests with strict Clippy; supervisor restart wiring remains open.
- Evaluated the official Microsoft SysVAD sample with the installed VS/WDK: the full solution is blocked by missing WIL headers in APO projects and broken WDK x86 InfVerif/API-validator execution, but a scoped build of `EndpointsCommon` plus `TabletAudioSample.sys` succeeds. Outputs were disposable; local automatic test signing and validation skips are not production-signing evidence, and no driver was installed.
- Repeated the SysVAD evaluation with the sample's WIL dependency populated: all previously blocked APO projects and `TabletAudioSample.sys` compiled, while the full normal-validation build still fails at the installed WDK x86 `InfVerif.dll`/API-validator path. This isolates WIL as resolved and the WDK validation tools as the remaining host blocker; no driver was installed or machine audio configuration changed.
- Revalidated the M05 UI lane at the current revision: all 20 Vitest tests passed, TypeScript typecheck passed, the Vite production build completed, and diff checks remained clean. Manual visual/accessibility acceptance and live native transport injection remain open.
- Added a repository-local VST3 SDK installer/repair script: it pins the known-good Steinberg revision, initializes recursive submodules, verifies required headers, refuses accidental replacement without `-Force`, and documents the source-SDK installation boundary. It performs no global installation or audio configuration change.
- Made the VST3 native loader build portable across installed VS2026 updates: it now discovers MSVC through `vswhere`, selects the newest compatible Windows SDK with headers, and supports explicit repository-local SDK/output overrides instead of hardcoded machine version paths.
- Corrected recording-library refresh to reload bounded WAV/FLAC metadata from the current file along with its format status, preventing stale tags after an external replacement or recovery. Added a file-replacement regression; recording tests and strict Clippy remain the validation targets.
- Completed recovery retention maintenance: storage-open pruning now removes crash markers older than the ten-minute window while deliberately preserving the latched safe-mode setting. Reopen and retention regressions cover the boundary; storage tests and strict Clippy pass.
- Aligned Rust shared WASAPI clients with the native endpoint-format evidence: capture and render now initialize the exact `GetMixFormat()` using only event-callback plus `NOPERSIST`, removing unnecessary `AUTOCONVERTPCM` from an exact-format request that drivers may reject as `E_INVALIDARG`. No stream was started by this code change; Windows-audio compile/tests and strict Clippy are required validation.
- Corrected event-driven WASAPI initialization to pass zero buffer duration, as required when `AUDCLNT_STREAMFLAGS_EVENTCALLBACK` is selected; the public duration argument remains source-compatible but is intentionally ignored. This addresses the remaining request-shape cause of `E_INVALIDARG` without starting a stream or changing endpoint configuration.
- Added and ran a native read-only `event-capture-init` probe: endpoint 0 accepted its exact `GetMixFormat()` with `EVENTCALLBACK|NOPERSIST`, zero duration, and `SetEventHandle`, all without `Start` or packet reads. This directly validates the corrected Rust request shape; process attribution and physical latency remain open.
- Extended the event-driven native probe across all 13 active capture endpoints; every endpoint accepted exact-format `EVENTCALLBACK|NOPERSIST` initialization with zero duration and `SetEventHandle`, without starting or reading. This further rules out ordinary endpoint contention for the previous `E_INVALIDARG`; process attribution and physical latency remain open.
- Generalized the native event probe to active render endpoints: 17 of 18 accepted exact-format event initialization with zero duration, while one endpoint returned the correctly classified `AUDCLNT_E_DEVICE_IN_USE` (`0x8889000A`), not `E_INVALIDARG`. No endpoint was started or rendered to; process attribution and physical latency remain open.
- Attempted a Windows-gated Rust adapter regression against the first active capture endpoint; it still returned `E_INVALIDARG` at `IAudioClient::Initialize` despite the equivalent native C++ request succeeding. The temporary failing test and diagnostics were removed, and the discrepancy is retained as an explicit Rust COM-marshalling/adapter blocker rather than claimed solved.
- Follow-up isolation found that the proposed Rust live-open regression still returns `E_INVALIDARG` at `IAudioClient::Initialize`, even though the equivalent native C++ request succeeds on all 13 capture endpoints. The temporary failing test and diagnostics were removed; this is now an explicit Rust COM-marshalling/adapter blocker, and the native success must not be generalized to the Rust path until that discrepancy is resolved.
- Narrowed that discrepancy further: the Rust `IsFormatSupported` call returns `S_OK` for the same endpoint-owned `GetMixFormat()` pointer immediately before Rust `Initialize` returns `E_INVALIDARG`, with endpoint ID/order matching the native probe. This rules out endpoint lookup and basic format support; the remaining target is Rust COM/ABI initialization marshalling.
- Additional isolation passed a caller-owned copy of the complete 40-byte format and tested Rust initialization without event/conversion flags; both still returned `E_INVALIDARG`. This rules out format allocation, event-handle creation, and `AUTOCONVERTPCM` as the sole cause. Temporary diagnostics were removed; a generated-binding ABI fix or verified native shim remains required.
- A final stack-owned simple PCM/IEEE-float format control also returned Rust `E_INVALIDARG`, ruling out `WAVEFORMATEXTENSIBLE` as the determining factor. All temporary code was removed; the native C++ reference remains the only qualified live-initialization path on this host.
- A fresh Rust direct-open attempt using the endpoint ID without prior enumeration reproduced `E_INVALIDARG`, ruling out enumeration/reopen lifetime. The unresolved difference remains the Rust COM initialization boundary versus the successful native C++ call.
- A temporary direct Rust `IAudioClient` vtable call bypassed the generated method wrapper and reproduced `E_INVALIDARG`; all diagnostic code was removed. The remaining issue is Rust process/runtime ABI interaction, with a verified native C++ shim as the fallback boundary.
- Added prepared per-node Meter stages to `RuntimeGraph`: each stage owns a lock-free `BlockMeter`, observes its exact processing boundary without allocation or locking, and is readable while the graph snapshot is retained. Engine regression verifies distinct pre/post-gain peaks and clipping counts; native scheduler/API publication remains open.
- Added copyable `BlockMeterSnapshot` reads for prepared graph meters, separating the stable readout contract from atomic storage internals while retaining lock-free bounded access. Native scheduler/API transport publication remains open.
- Added meter lifecycle reset semantics: `RuntimeGraph::reset_meters` is lock-free, and `RuntimeProcessor::publish` clears reused graph readings at activation boundaries. Regression coverage prevents telemetry leaking across generations; native scheduler/API publication remains open.
- Exposed published per-node meter snapshots through `RuntimeProcessor::meter_snapshot`, with lifecycle coverage for active graphs, missing indices, and deactivation. The portable meter publication boundary is now complete; native scheduler/control API transport remains open.
- Added the supervisor-facing `CrashRecoveryTracker::decide_recovery` API, returning safe-mode state and eligible non-recording sessions from one bounded policy evaluation. Regression coverage prevents callers from combining inconsistent mode/session reads; process restart and native lifecycle wiring remain open.
- Added `Storage::recovery_status`, reading the recent crash count and durable safe-mode latch in one SQLite transaction. Storage regression verifies the consistent snapshot; supervisor process wiring and automatic route restart remain open.
- Wired the persisted recovery snapshot into `status.get` and `system.diagnostics`, exposing bounded `recentCrashes` alongside `safeMode` from one read. Control coverage verifies both fields; process supervision and automatic route restart remain open.
- Added Windows Job Object resource containment for plugin workers: one active process and a 512 MiB process-memory cap now accompany kill-on-close cleanup. The portable worker suite remains green; full filesystem/network sandboxing and production plugin execution remain open.
- Corrected the transport crate’s security boundary documentation to match the implemented owner-only named-pipe ACL and peer-SID check; method-level client grants remain enforced by control dispatch. No audio endpoint or machine configuration is involved.
- Added authorized recovery-latch clearing through `recovery.clearSafeMode` and the CLI command `recovery clear-safe-mode`; it clears only durable crash markers/latch state and never starts sessions or audio. Control/CLI parity tests remain the validation target.
- Connected the M05 UI to persisted recovery status and the authorized `recovery.clearSafeMode` operation. The UI shows safe-mode/crash-marker state, enables clearing only for a connected backend with an active latch, and keeps disconnected preview read-only; UI tests, typecheck, and production build pass. No audio or machine configuration changed.
- Added MCP `clear_recovery_safe_mode` parity with operator-scope authorization and read-only grant denial coverage. The tool routes through the shared control dispatcher and changes only persisted recovery state; audio/session restart remains unavailable.
- Corrected control mutation classification so `safety.setPrivacyMute` notifications are rejected and included in mutation rate limiting, matching its declared mutating side effect. Added regression coverage; this remains process-local and does not change Windows privacy settings.
- Audited the same classifier against API metadata and added missing `operations.cancel` and `recordings.rename` mutation/external-operation entries. Notifications are now rejected consistently for these methods and rate limiting applies; regression coverage passes.
- Replaced the duplicated mutation-name list with metadata-driven classification from `API_METHODS`; all non-read-only methods now share notification rejection and rate limiting automatically. Added a drift-prevention regression covering the complete method table.
- Hardened `WorkerSupervisor` against duplicate starts: a running worker now returns `AlreadyRunning` without resetting its heartbeat or creating a second represented instance. Plugin-host lifecycle coverage and strict Clippy remain green.
- Cleared inherited environment variables when launching plugin workers; worker configuration remains explicit through validated arguments and pipes, reducing backend credential/configuration exposure. Process Job Object limits remain active; full filesystem/network isolation and production plugin execution remain open.
- Tightened `WorkerProcess::process` response validation to require matching sequence, deadline, channel count, and sample length before accepting worker audio. Plugin-host process regressions and strict Clippy remain green.
- Added an explicit `WorkerSupervisor::record_failure` transition for immediate worker exits, protocol failures, and invalid-output reports; it shares the three-failure quarantine policy without auto-restarting processes. Plugin-host coverage now includes repeated immediate faults; native plugin execution and full sandboxing remain open.
- Updated the MCP stdio interoperability expectation for the now-published recovery-clear tool catalog (23 tools); the full locked workspace test and strict Clippy suite are green.
- Rebuilt the checked-in native WASAPI probe with the installed Visual Studio/WDK toolchain after the decision-register update; compilation succeeded and the generated executable/object were removed without running the probe or touching audio devices/configuration.
- Added a conservative process-restart resolver to the Windows audio adapter: persisted executable selectors rebind only to exactly one case-insensitive live match with a creation timestamp; missing, ambiguous, and identity-unavailable matches remain silent with typed errors. The regression is metadata-only and does not open audio; controlled tone attribution and native restart runtime evidence remain open.
- Aligned the shared status contract and UI with authoritative persisted privacy-mute state. Connected snapshots now drive the displayed latch, while disconnected preview remains explicitly muted in memory; UI tests, typecheck, and production build pass without changing Windows privacy or audio settings.
- Made PID-bound application verification use the same case-insensitive executable comparison as restart rebinding, matching Windows naming semantics while retaining the required creation timestamp check. The metadata-only Windows-audio regression passes; process-loopback runtime attribution remains open.
- Corrected the M08 release runbook to invoke the artifact verifier with its actual `-ManifestPath` parameter, and added a PowerShell documentation regression to prevent the obsolete `-ReleaseDirectory` command from returning an unusable workflow. No artifacts or machine configuration are changed.
- Hardened M08 artifact verification against symlink/reparse-point manifest, SBOM, notice, and payload paths, and require the generated third-party notice file. The verifier regression exercises tamper rejection and conditional reparse rejection without creating release artifacts outside a temporary fixture.
- Tightened the M08 verifier to require `THIRD-PARTY-NOTICES.txt` both on disk and in the manifest checksum list; added omission-rejection coverage to prevent a package from silently dropping dependency notices.
- Added `.github/workflows/ci.yml` with separate portable and Windows jobs: portable Rust/UI/contract checks run on Ubuntu, while Windows excludes hardware-dependent endpoint tests and still compiles the Windows audio adapter. The workflow performs no driver installation, audio activation, or machine configuration changes.
- Extended the Windows CI job to run the release artifact verifier and qualification-runbook regressions, keeping manifest path safety and documented command parity continuously checked.
- Reconciled milestone headers M04–M08 with the evidence: portable foundations are marked implemented where present, while native shell, lifecycle, plugin execution/sandboxing, driver/signing, installer, and clean-machine gates remain explicitly open.
- Added the missing incremental-FLAC recording-library refresh regression: external replacement now reindexes bounded Vorbis metadata just like WAV, while preserving the existing status/path safeguards. Recording tests and strict Clippy pass; native realtime integration remains open.
- Added read-only Windows audio-session discovery to `applications.list` using `IAudioSessionManager2` across active render and capture endpoints. Entries now expose observed activity, session counts, display names, and capture-session observations while retaining the existing PID/creation identity safeguards; no stream is opened or started, and unobserved protected/background capture remains explicitly unknown. Control and Windows-audio tests pass; process-loopback attribution remains open.
- Reconciled the streaming-FLAC milestone evidence with the existing implementation: bounded title/artist/comment Vorbis metadata is written by `StreamingFlacWriter::new_with_metadata` and verified by reopening the produced file. Compression tuning and native realtime integration remain open; metadata insertion is no longer an outstanding item.
- Hardened Windows application-session discovery to fail soft for inaccessible endpoints, aggregate sessions, and transient session removal, preserving the rest of the bounded application inventory. Windows-audio/control tests and strict Clippy pass; process-loopback attribution remains open.
- Wired the UI application inventory to refresh on reconnect and successful event-triggered snapshot refreshes, preventing audio activity/capture observations from remaining stale after backend changes. UI typecheck, 21 tests, and production build pass; live shell injection and manual acceptance remain open.
- Updated CLI/MCP application-list descriptions to identify the observed Windows audio-session fields exposed by the shared dispatcher. CLI tests (15), MCP stdio interoperability, control tests (54), and strict Clippy pass.
- Added the canonical `applications list` CLI alias alongside `apps list`; both return the same shared application/session snapshot, and help now advertises both forms. CLI/MCP process tests and strict Clippy pass.
- Strengthened CLI application discovery coverage to require the observed activity, capture-observation, session-count, and display-name fields for every returned application. CLI/MCP tests and strict Clippy pass.
- Fixed application discovery contract correctness found by the full workspace run: PID-0's Windows system pseudo-process is excluded, and the CLI regression no longer assumes two independent live snapshots are byte-identical. The locked workspace suite passes across all crates; no audio stream or configuration was touched.
- Made process application snapshots deterministic by sorting case-insensitively by executable and then PID. Added a live Windows ordering regression; Windows-audio/control tests and strict Clippy pass without opening streams.
- Extracted the application ordering policy into a pure adapter helper and added a synthetic case-insensitive/PID ordering regression in addition to the live Windows check. Windows-audio tests (12) and strict Clippy pass.
- Corrected UI lifecycle labels to derive Running/Stopped state from the authoritative `status.get.activeSessionIds` snapshot instead of always displaying Stopped. UI typecheck, 22 tests, and production build pass; native shell/manual acceptance remains open.
- Audited M04 compressor graph integration and preserved the explicit boundary: DSP compressor state is implemented and tested, but published `Arc` runtime graphs are immutable while compressor detector state is mutable. The attempted integration was removed after compile validation; a scheduler-owned mutable-state design is required before exposing a compressor node, avoiding unsafe aliasing or per-block state resets.
- Added explicit `devices.list` contracts and bounded read-only cursor pagination. Endpoint descriptors now advertise their direction, active state, mix format, and periods in both Rust discovery schemas and shared TypeScript types; invalid cursors/limits are rejected before enumeration, while the legacy unpaged array remains compatible. Control tests, strict Clippy, formatting, and contracts typecheck pass; enumeration remains metadata-only with no stream or machine configuration changes.
- Added the UI's read-only Windows endpoint inventory. The live adapter requests a bounded `devices.list` page, the disconnected adapter returns an empty safe state, and reconnect/event refreshes update the displayed direction, format, and period metadata. UI tests (24), typecheck, production build, and diff checks pass; no endpoint is opened or reconfigured.
- Revalidated the full locked Rust workspace after the device/UI slices: CLI/MCP, control, domain, DSP, engine, plugin-host/worker, protocol, recording, storage, transport, Windows-audio, and doc-test targets all passed. The 24-test UI suite, TypeScript typecheck, and production build also pass; no stream, driver, or machine audio configuration was changed.
- Requalified the unsigned M08 artifact flow at the current clean revision: optimized locked CLI/plugin-worker artifacts, Cargo SBOM, third-party notices, manifest hashes, and byte counts were generated and accepted by the verifier in a disposable directory. The directory was removed afterward; signing, driver, installer, and clean-machine gates remain open.
- Replaced the generic `nodes.types`/`nodes.describe` output schema with explicit descriptor and parameter schemas covering availability, realtime cost, and bounded processor metadata. Control tests (55), strict Clippy, formatting, and diff checks pass; this improves discovery contract correctness without opening audio.
- Replaced the generic `clients.list` output schema with an explicit enrollment record schema for client ID, role, and revocation state. Control tests (55), strict Clippy, contracts typecheck, formatting, and diff checks pass; no enrollment state was changed.
- Replaced the generic `status.get` output schema with an explicit typed snapshot schema covering audio/device capability, storage, sessions, privacy mute, recovery, and event cursors. Control tests (55), strict Clippy, contracts typecheck, formatting, and diff checks pass; the read-only status path remains configuration-safe.
- Replaced the generic `system.diagnostics` output schema with an explicit redacted diagnostics schema covering backend/storage identity, unavailable-audio reason, privacy/recovery state, and event-log counters. Control tests (55), strict Clippy, contracts typecheck, formatting, and diff checks pass; no diagnostic data or machine state was changed.
- Added explicit output schemas and shared contract types for `startup.get` and `recovery.clearSafeMode`, including their unavailable/cleared-state invariants. Control tests (55), strict Clippy, contracts typecheck, formatting, and diff checks pass; startup registration was not invoked and recovery state was not changed.
- Replaced generic pagination output schemas for `sessions.list` and `graph.history` with explicit page envelopes and session snapshot descriptors, including cursor, revision, node, port, edge, and matrix fields. Control tests (55), strict Clippy, contracts typecheck, formatting, and diff checks pass; no session or graph state was mutated.
- Corrected `events.subscribe` resync snapshots to use the advertised paginated `SessionListPage` shape instead of a bare session array; epoch-mismatch and cursor-expiry regressions now verify the page envelope. Control tests (55), strict Clippy, contracts typecheck, formatting, and diff checks pass.
- Added an explicit `routes.inspect` output schema for destination identity, reachability, route paths, edge IDs, and channel maps. Control tests (55), strict Clippy, contracts typecheck, formatting, and diff checks pass; route inspection remains read-only and does not activate audio.
- Added explicit output schemas for `graph.plan` and `graph.commit`, covering plan expiry/diff/warnings/scope metadata and commit revision/activation fields. Control tests (55), strict Clippy, contracts typecheck, formatting, and diff checks pass; graph planning/commit behavior was not broadened into native audio activation.
- Added explicit output schemas and shared result types for `operations.get` and `operations.cancel`, including completed/unknown operation variants and the non-cancelling completed-operation result. Control tests (55), strict Clippy, contracts typecheck, formatting, and diff checks pass; no operation was cancelled or mutated.
- Added the explicit full-session output schema to `sessions.get`, reusing the validated session descriptor used by paginated session/history responses and covering graph nodes, ports, edges, and channel matrices. Control discovery regression, strict Clippy, formatting, contracts typecheck, and diff checks remain required; the read-only lookup does not mutate session or audio state.
- Added explicit output schemas and shared TypeScript result types for protocol handshake, session create/duplicate/delete, session start/stop, undo planning, and privacy-mute results. The schemas encode the existing fake-runtime/stopped-state invariants and are discovery-tested without starting audio or changing machine privacy settings.
- Added explicit output schemas and shared TypeScript result types for client authorization/revocation and recording metadata/rename/remove/reveal operations. The schemas capture stable success and missing-file branches; discovery-only regressions verify representative fields without opening audio or invoking file/OS actions.
- Tightened recording recovery, preview, and recycle discovery contracts with explicit variant schemas and shared TypeScript result types. The variants distinguish validated checkpoints, WAV/FLAC metadata, missing/invalid files, preview-only recycle, successful recycle, and unavailable recycle outcomes; tests remain metadata-only and do not move files or read audio payloads.
- Replaced the generic `system.describe` output schema with an explicit discovery-document schema for protocol/version identity, method descriptors, node types, limits, and event retention. Control discovery regression, strict Clippy, formatting, contracts typecheck, and diff checks remain the validation target; this changes no runtime or machine state.
- Corrected the UI backend adapter and tests after the typed recording/privacy/session result contracts exposed stale `Record<string, unknown>` signatures and obsolete preview/recovery field access. UI typecheck, all 24 Vitest tests, and the production build pass; the fix is adapter-only and does not invoke audio or file actions.
- Removed the remaining generic UI/shared result seams for recovery clearing and graph-commit activation. `RecoveryClearResult` now encodes the cleared latch, and commit activation distinguishes pending from running fake-runtime generations; contracts/UI typechecks, UI tests, workspace check, and diff validation pass.
- Added typed `startSession`/`stopSession` methods to the UI backend adapter and a guarded lifecycle control. Connected mode forwards the existing authorized `session.start`/`session.stop` API; disconnected mode remains read-only and fail-closed. UI typecheck, 25 tests, production build, and diff checks pass; the current backend is still fake and opens no audio.
- Added typed recording recycle forwarding to the UI backend adapter, including the explicit confirmation boolean and preview-result contract. The disconnected adapter rejects it; a live-adapter regression verifies forwarding without invoking the file action. UI typecheck, 26 tests, contracts typecheck, and diff checks pass.
- Added `ControlPlane::record_runtime_crash`, integrating the bounded crash policy with SQLite persistence and returning one consistent recovery decision for a future supervisor. Memory and file-backed regressions cover restore eligibility and latched safe mode; control tests (57) and strict Clippy pass. Process creation, automatic restart, and audio activation remain intentionally outside this seam.
- Made recovery decision candidate ordering deterministic by sorting opaque session IDs by their stable string form. A two-session regression covers the ordering; control tests (58), strict Clippy, formatting, and diff checks pass.
- Fixed memory-backed recovery clear semantics: `recovery.clearSafeMode` now resets the in-memory crash tracker as well as durable markers; a regression confirms a cleared tracker returns to restore-eligible mode. Control tests (59), strict Clippy, formatting, and diff checks pass.
- Made the UI library honest and useful: available built-in processors (gain, mixer, mute, and meter) can be added to the local draft with deterministic IDs and valid ports/parameters, while device-bound input and recorder entries are disabled until their native/runtime capabilities exist. UI tests (28), typecheck, production build, and diff checks pass; additions still require backend planning and commit.
- Added keyboard/select-based topology editing to the UI: users can choose real output and input ports, preview a bounded channel matrix, and add a draft edge with duplicate and occupied-input guards. The authoritative backend still validates graph cycles and commits; UI tests (30), typecheck, production build, and diff checks pass.
- Added explicit draft-edge removal controls in the canvas view, preserving the list-view alternative and leaving committed topology untouched until plan/commit. UI tests (31), typecheck, production build, and diff checks pass.
- Added presentation-only canvas layout persistence: dragged node positions are bounded, stored per session in browser-local UI storage, restored on revisit, and never serialized into graph candidates. UI tests (33), typecheck, production build, and diff checks pass.
- Added bounded node renaming in the inspector. Names are trimmed and limited to 120 characters while node IDs, revisions, and edges remain unchanged until backend plan/commit; UI tests (34), typecheck, production build, and diff checks pass.
- Added explicit selected-node removal from the UI draft. Incident edges are removed together, the user must confirm, disconnected mode remains disabled, and authoritative revision/state are unchanged until plan/commit. UI tests (35), typecheck, production build, and diff checks pass.
- Added selected-node duplication to the UI draft. Copies receive deterministic IDs, cloned parameters/ports, and no implicit connections; the original graph and authoritative revision remain unchanged until plan/commit. UI tests (36), typecheck, production build, and diff checks pass.
- Added draft connection enable/disable controls in the canvas view. Toggling preserves edge identity and topology, is visible as text state, and remains draft-only until plan/commit. UI tests (37), typecheck, production build, and diff checks pass.
- Added bounded draft undo/redo history across node, edge, parameter, name, and session edits. Switching sessions or discarding resets history, new edits invalidate redo, and no history action calls the backend; UI tests (39), typecheck, production build, and diff checks pass.
- Added guided UI templates for Gaming + Discord, Processed Microphone, and Mix-minus conversation. Each loads an independent inspectable stopped draft with explicit edges and bounded channel maps, while preserving the current session identity/revision until plan/commit. UI tests (41), typecheck, production build, and diff checks pass.
- Added a canvas Reset layout control that clears only the selected session's browser-local presentation positions and restores automatic placement. UI tests (42), typecheck, production build, and diff checks pass; audio topology and machine state are unaffected.
- Added enabled-path highlighting in the canvas: selecting a node highlights its connected upstream/downstream component, dims unrelated nodes/edges, and excludes disabled edges. Pure graph-view regressions and UI validation pass at 44 tests; this is presentation-only.
- Added bounded searchable node-library filtering across labels, categories, and capability explanations. Search remains local; unavailable device/runtime entries stay visible when matched, while only connected mode can add supported processors. UI tests (46), typecheck, production build, and diff checks pass.
- Added connection enable/disable/remove parity to the keyboard-friendly graph list view, matching canvas controls and preserving connected-backend gating. UI tests (46), typecheck, production build, and diff checks pass.
- Added explicit dark, light, and high-contrast UI themes with bounded browser-local preference persistence and fail-closed invalid-value handling. Existing forced-colors and reduced-motion behavior remains active; UI tests (48), typecheck, production build, and diff checks pass.
- Added detailed canvas port labels for direction, role, and channel count, with a pure formatting regression. UI tests (49), typecheck, production build, and diff checks pass; canvas topology remains non-editable and backend-authoritative.
- Added matching port-role/channel summaries to the keyboard-friendly graph list, so canvas-independent inspection exposes the same endpoint information. UI tests (49), typecheck, production build, and diff checks pass.
- Added route-explanation status labels for enabled, bypassed, muted, and disabled nodes, with unknown IDs preserved visibly. UI tests (50), typecheck, production build, and diff checks pass; route inspection remains read-only.
- Added an explicit inspector reset for gain and mute processor parameters, restoring documented defaults in the local draft and preserving identity/topology/revision. UI tests (51), typecheck, production build, and diff checks pass.
- Added bounded session-name draft validation with trimming and actionable empty/overlong errors. Valid edits preserve the session identity/revision and use the existing plan/commit path; UI tests (52), typecheck, production build, and diff checks pass.
- Added a read-only guided setup readiness checklist for backend, audio capability, storage, endpoint inventory, and application observations, with explicit Discord/OBS external-selection guidance. UI tests (54), typecheck, production build, and diff checks pass; no setup action changes machine state.
- Corrected setup readiness to align with the published storage contract: both `memory` and `sqlite` are recognized, with memory explicitly labeled non-durable. UI tests (55), typecheck, production build, and diff checks pass.
- Re-ran the repository-local VST3 SDK setup and verified the pinned checkout at `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, recursive submodules, and required hosting headers. The source SDK remains local to `third_party/vst3sdk`; no global installation, system plugin, or audio configuration changed.
- Requalified the unsigned M08 artifact flow from clean revision `4aa534fcc781e3245f3d861270c108279c11cd39`: locked optimized CLI/plugin-worker artifacts, complete Cargo SBOM, third-party notices, provenance hashes/byte counts, verifier, and runbook-command regression all passed in a disposable directory. The output was removed; signing, driver, installer, and clean-machine gates remain open.
- Added `SupervisedWorkerProcess` to connect worker protocol success/failure to `WorkerSupervisor`: successful exchanges refresh bounded heartbeats, while spawn and protocol faults enter the existing failure/quarantine policy. Five worker process tests and 29 plugin-host unit tests pass with strict Clippy; automatic restart, plugin execution, and full OS sandboxing remain open.
- Hardened `SupervisedWorkerProcess` to fail closed after heartbeat timeout or worker failure, rejecting further processing until an outer supervisor deliberately replaces it. The timeout regression passes; plugin-host coverage is 29 unit tests plus 6 worker-process tests with strict Clippy. Automatic restart, plugin execution, and full OS sandboxing remain open.
- Extended the supervised worker bridge to shared-memory transport with `spawn_shared`; a mapped stereo frame now exercises supervised processing and heartbeat refresh before clean shutdown. Plugin-host coverage remains 29 unit tests plus 6 worker-process tests with strict Clippy; plugin loading, automatic restart, and full OS sandboxing remain open.
- Added process-exit supervision: `poll` detects an exited child, and outer adapters can report failures explicitly; both transitions fail closed and reject further processing. Plugin-host coverage is now 29 unit tests plus 7 worker-process tests with strict Clippy; automatic restart, plugin execution, and full OS sandboxing remain open.
- Added explicit supervisor-ledger handoff for replacement workers, preserving failure history across deliberate process generations so the third fault reaches quarantine instead of resetting policy. Plugin-host coverage is now 29 unit tests plus 8 worker-process tests with strict Clippy; automatic restart, plugin execution, and full OS sandboxing remain open.
- Added explicit `SupervisedWorkerProcess::restart` support for both pipe-only and shared-memory workers. A failed shared worker now carries its caller-owned mapped transport into the replacement, preserves the supervisor ledger, and remains fail-closed during replacement errors. The shared restart regression, plugin-host tests, strict Clippy, and full M07 headless acceptance pass; automatic route restart, plugin execution, and full OS sandboxing remain open.
- Added bounded `SupervisedWorkerProcess::poll_and_restart`: one poll-triggered replacement is attempted for a failed worker, quarantine remains authoritative, and no unbounded restart loop or route restoration is introduced. Plugin-host tests, strict Clippy, formatting, and the complete M07 headless acceptance pass.
- Revalidated the native M06 VST3 loader probe with Visual Studio Community 2026, MSVC 14.51.36231, and Windows SDK 10.0.28000.0 using the pinned local SDK headers. Compilation succeeded; the generated executable/object were removed and the probe was not executed, so plugin compatibility and runtime execution remain unclaimed.
- Ran the native M06 loader against the pinned SDK's local `mda-vst3.vst3` fixture. It loaded the factory, enumerated 68 classes, processed a bounded offline stereo block with finite output, verified five normalized parameter readbacks/automation, and round-tripped 180 bytes of component state. The loader and object were removed afterward; this is one local fixture only and does not satisfy the three-plugin/two-vendor or realtime acceptance gate.
- Revalidated the project-local CMake/SDK build with the installed VS2026 toolchain: the SDK built in Release, its self-test reported 51 passed, and the official VST3 validator reported 1,598 passed and 0 failed for the local `mda-vst3` fixture. The SDK remains source-local; no system plugin registration or audio configuration changed.
- Added `tests/acceptance/m06-vst3-sdk.ps1` to reproduce the pinned SDK verification: revision/header checks, local Release build, official validator, offline native loader, and generated-loader cleanup. It has no system installation or audio-configuration side effects.
- Fixed the Windows UI test invocation by using Vitest's `--configLoader runner`, avoiding the host's blocked `node_modules/.vite-temp` config bundle path. The full UI suite now passes 58 tests while TypeScript typecheck remains green.
- Applied the same Vite runner loader to the UI production build, eliminating the blocked bundled-config temp write. The build succeeds when directed to a disposable temporary output; the host still denies writes/deletion in the existing `ui/dist`, so that protected-output limitation remains explicitly recorded.
- Added and ran `tests/acceptance/m05-ui.ps1`, which executes UI typecheck, Vitest, and a disposable-output production build. The acceptance script passed with 58 tests and three generated build files, then removed its temporary output.
- Hardened plugin scanning with a per-candidate ten-second inspection window plus cancellation checks during bounded 64 KiB binary reads. This preserves fail-fast discovery behavior for large plugin files without loading or executing plugin code; plugin-host tests and strict Clippy pass. See [M07 automation and recovery evidence](evidence/M07-automation-recovery.md).
- Added the explicit read-only CLI entry point `plugins scan --directory <absolute-path>`, exposing bounded VST3 identity and compatibility results without downloading, loading, or executing plugin code. CLI help/validation and plugin-host checks pass; see [M07 automation and recovery evidence](evidence/M07-automation-recovery.md).
- Exposed the bounded plugin inspection path as shared read-only `plugins.scan` API metadata, schemas, Rust/TypeScript result contracts, and control dispatch. Invalid candidates remain visible without loading/executing code; focused control/CLI tests, strict Clippy, and UI typecheck pass.
- Separated plugin scanning into the explicit `pluginScan` permission scope required by SEC-03; built-in roles do not gain it implicitly, and authorization regressions pass alongside the shared scan tests.
- Extended plugin authorization coverage through dispatch: generic read-only access is denied before inspection, while an explicit `pluginScan` grant reaches the bounded scanner; the regression passes with strict Clippy.
- Added read-only `plugins.inspect` parity for one absolute plugin binary, returning typed identity or visible inspection errors through the shared control/API contract without module loading; focused control/CLI tests, strict Clippy, and UI typecheck pass.
- Routed the CLI plugin-scan convenience path through the shared authorized dispatcher and added `plugins inspect --path`; relative-path validation and invalid-candidate visibility regressions pass, preserving CLI/API parity.
- Tightened `plugins.inspect` discovery with a fully typed nullable identity schema and a regression for its seven identity fields; 66 control tests and strict Clippy pass.
- Added `tests/acceptance/m07-headless.ps1` to reproduce the headless control/CLI/plugin-host gate, including strict Clippy and M01 CLI acceptance; it performs no audio, driver, or machine-configuration actions.
- Revalidated the native M06 VST3 acceptance script with elevated MSBuild access: local Release SDK build, 51 SDK tests, validator 1,598/0, and offline loader all passed under Visual Studio Community 2026; generated outputs were cleaned and no audio configuration changed.
- Revalidated unsigned M08 artifact preparation from the current revision: optimized x64 CLI/worker binaries, locked SBOM, notices, checksums, and manifest verified in a disposable directory; the manifest correctly remains unsigned and not publication-ready, with release blockers retained.
- Added `tests/acceptance/m08-release.ps1` to reproduce unsigned artifact preparation/verification with overwrite refusal, blocker assertions, and guaranteed temporary-output cleanup; no installer, driver, signing, or audio action is performed.
- Ran the new M08 wrapper successfully from clean revision `ff2fd5e` with VS2026: optimized artifacts were prepared and verified, unsigned blockers were asserted, and temporary output cleanup completed.
- Hardened plugin discovery to reject a symlink/reparse-point directory selected as its scan root before enumeration, matching the existing canonical containment and reparse checks for plugin binaries. The regression passes in the plugin-host suite; no plugin code is loaded or executed.
- Re-ran `tests/acceptance/m07-headless.ps1` after the scan-root hardening: M01 CLI, MCP stdio, 66 control tests, 31 plugin-host tests, 8 worker-process tests, and strict Clippy passed without audio or machine configuration access.
- Re-ran `tests/acceptance/m08-release.ps1` after the scan-root hardening: optimized unsigned artifacts, SBOM, notices, checksums, and manifest verified in a disposable directory, with blocker assertions and cleanup passing.
- Re-ran `tests/acceptance/m06-vst3-sdk.ps1` from the current tip with elevated MSBuild access: the pinned local SDK built, 51 self-tests and 1,598/0 validator tests passed, and the offline loader passed. Outputs were cleaned; no plugin registration or audio configuration changed.
- Replaced arbitrary `HashMap` eviction in the bounded in-memory operation cache with explicit FIFO retention, preserving deterministic availability at the 100-entry cap. A control regression and strict Clippy pass; durable SQLite outcomes are unchanged.
- Re-ran `tests/acceptance/m07-headless.ps1` from clean revision `854c6b5`; M01 CLI, MCP stdio, 67 control tests, plugin-host/worker tests, and strict Clippy passed without audio or machine configuration access.
- Scoped authenticated graph and virtual-device idempotency records by client and method internally, preserving caller-visible operation IDs and preventing cross-client key collisions. Added a two-client regression; control coverage is 68 tests with strict Clippy.
- Revalidated the full locked Rust workspace at the current tip: 323 unit/integration tests, all doc-tests, workspace strict Clippy, formatting, and diff checks passed. Native audio, driver, signing, installer, and hardware gates remain explicitly unchanged and open.
- Added the M08 development quickstart and troubleshooting runbook, linked from the documentation map. It documents the repository-local VST3 SDK installation, safe acceptance commands, offline CLI/UI/MCP boundaries, known native `E_INVALIDARG` and MSBuild restrictions, and the exact unsigned-release limitations without changing machine configuration.
- Added development release notes for the current M08 qualification snapshot, documenting the tested Windows/toolchain boundary, 323-test/SDK acceptance results, known native/driver/signing/installer limitations, and configuration-safety conditions. The notes are explicitly not a release or publication claim.
- Added M08 privacy/permissions and plugin-compatibility operational guides, linked from the documentation map. They document implemented local grants, privacy limitations, bounded plugin inspection, verified VST3 fixture results, and the remaining full-sandbox/native-execution limits without expanding any machine or audio operation.
- Added a dependency-free documentation acceptance tool and PowerShell wrapper that validate all Markdown local links, heading anchors, and balanced code fences. It is read-only and provides a reproducible check for the documentation requirement without touching audio or machine configuration.
- Hardened the release verifier to reject unlisted files and nested directories beside the manifest, and added tamper coverage for package-content pollution. The release directory is now required to contain exactly the checksum-listed artifacts plus its manifest.
- Re-ran the complete M08 release-preparation wrapper from clean revision `1b5094e` after the exact-content hardening; optimized unsigned artifacts, SBOM, notices, checksums, and manifest verified successfully and temporary output was cleaned.
- Hardened release preparation to require an existing non-reparse output parent before creating a disposable artifact directory, preventing redirected output paths. The normal M08 wrapper remains the regression for the safe output flow.
- Added `tools/release/test-prepare-artifacts.ps1` and wired it into Windows CI to cover missing and reparse-point release output parents. The test is disposable and exercises path rejection before any build or artifact creation.
- Re-ran the complete M08 wrapper from clean revision `ddfa192` after the output-parent guard; artifact preparation, exact-content verification, unsigned blocker assertions, and cleanup passed.
- Added sanitized Rust/Cargo/target/profile provenance to the release manifest and required it during verification, with synthetic-fixture coverage. This makes the unsigned artifact output traceable to both source revision and build toolchain without exposing machine paths.
- Re-ran the complete M08 wrapper from clean revision `f59fe5c`; generated toolchain provenance, exact-content artifact verification, unsigned blocker assertions, and disposable cleanup all passed.
- Repaired two stale repository-owned M00 evidence links exposed by the new validator and made the validator ignore vendored SDK Markdown while recognizing explicit HTML anchors. `tests/acceptance/docs.ps1` now passes with 50 project Markdown files and 147 local links.
- Added the read-only documentation validator to the portable GitHub Actions job, so local links, anchors, and fenced-code balance are checked on every push and pull request without requiring SDK or audio access.
- Added shared UI draft constants and validation for the documented -60 to +24 dB gain range, with boundary/rejection regression coverage; the inspector HTML bound now matches +24. M05 UI acceptance passes with typecheck, 59 tests, and a disposable production build.
- Added CLI regression coverage for `presets list`, verifying the stable voice-chain IDs and explanatory metadata exposed to headless clients; the command remains read-only and does not apply a graph preset.
- Exposed the already-implemented 50 Hz and 60 Hz hum-notch EQ starting points alongside voice-neutral in `system.describe`, `presets.list`, the TypeScript discovery contract, and CLI output. Control/DSP/CLI tests and strict Clippy pass; preset discovery remains read-only and does not alter a graph or audio state.
- Added `tests/acceptance/m04-dsp-recording.ps1` to reproduce the portable M04 gate (format, DSP/recording tests, strict Clippy, and diff checks) without opening audio or changing machine configuration.
- Corrected in-memory `operations.get` metadata so cached virtual-device apply outcomes report `virtualDevices.apply` rather than the graph-commit operation. A control regression covers the operation name; all 64 control tests and strict Clippy pass.
- Added restart-backed coverage for durable `operations.get`: a completed virtual-device apply is reported with its stored `virtualDevices.apply` operation name after reopening SQLite, alongside the existing same-key replay test.
- Requalified the unsigned M08 preparation from revision `ff5d9e3c71d5f22c59a8b8d2af488b051f2c72dc`: optimized CLI/plugin-worker binaries, SBOM, notices, and manifest were generated and verified in a disposable directory; the manifest correctly reports `signed: false` and `publicationReady: false`.
- Hardened the native loader probe to fail when a factory has no compatible audio-effect class instead of returning false success. Rebuilt and reran the local fixture successfully with the same processing, automation, and state evidence; generated outputs were removed.
- Hardened the native loader's VST3 error handling: audio-bus activation and parameter write/restore return codes are now checked, so legitimate plugin argument rejection cannot be mistaken for success. The local fixture rebuilt and passed all checks; generated outputs were removed.
- Restricted native VST3 loading to the resolved regular file and its DLL directory/default safe search paths via `LoadLibraryExW`, reducing dependency-hijacking risk. The local fixture rebuilt and passed; generated outputs were removed and no machine configuration changed.
- Verified the restricted loader fails closed for a missing plugin target (`resolved plugin binary is not a regular file`, exit 1) before any module load. Probe outputs were removed afterward.
- Added a reproducible `--class-index` selector to the native VST3 probe and exact VST3 result-code diagnostics. The local mda fixture passes for Ambience (0), BeatBox (4), Combo (6), and Delay (12); Bandisto (2) and Limiter (32) reject processor activation with `0x80004001` (`E_NOTIMPL`), not `E_INVALIDARG`. This records plugin-specific incompatibility instead of misattributing it to the loader; outputs were removed.
- Completed selector negative-path validation: a controller index and an out-of-range index both return exit 1 with `factory exposes no compatible audio effect`; generated outputs were removed.
- Added `Virtual Render Source` and `Virtual Capture Sink` to the authoritative domain/TypeScript node contracts and read-only UI library. Both are explicitly unavailable until the M03 managed virtual driver exists; engine matching treats them as device-bound placeholders and never claims an active bus. Rust workspace tests (all targets), strict Clippy, contracts typecheck, and UI tests (56 via runner loader) pass. UI production output remains blocked by Windows `EPERM` while writing Vite temp/dist files; no driver, endpoint, or machine configuration changed.
- Improved Windows audio error diagnostics by including the stable numeric HRESULT in `AudioError` display text; the adapter regression confirms `0x80070057` is classified and rendered as invalid argument. This is metadata/error-path evidence only and does not open or start a stream.
- Added a portable `VirtualBusLease` ownership primitive with monotonic generations, competing-owner rejection, stale-release protection, and explicit force release. Domain coverage is now 32 tests with strict Clippy; this is lease-policy evidence only and does not create a driver bus, retain audio, or change machine state.
- Extended the M03 foundation with a bounded `VirtualBusRegistry`: up to eight trimmed, uniquely named stereo buses can be created, listed, renamed, enabled/disabled, leased, and deleted only when disabled and unowned. Domain coverage is now 34 tests; full locked workspace tests and strict Clippy pass. No driver endpoint or machine configuration is created.
- Exposed the managed virtual-bus desired-state inventory through read-only `virtualDevices.list` in the shared Rust/TypeScript API contracts. The cursor-compatible result includes availability, endpoint placeholders, and lease ownership without activating audio; control coverage is now 60 tests. Bus lifecycle mutations, persistence, native endpoint identity, and driver integration remain open.
- Added adapter parity for the read-only inventory: `virtual-devices list` is available in the CLI help/command surface and `list_virtual_devices` is available through the local MCP tool catalog. CLI coverage confirms the empty inventory is represented honestly; no endpoint is synthesized.
- Added UI backend parity for managed virtual-device inventory, including paged-response normalization and a live-client request regression. UI tests now pass 57 tests with typecheck; no UI action provisions or alters an endpoint.
- Added registry-level force-release cleanup for crashed virtual-bus owners while retaining lease-generation safety. Domain coverage now includes the integrated crash/reconnect path; driver bridge heartbeat and native endpoint recovery remain open.
- Added durable SQLite persistence for virtual-bus desired state and wired the storage-backed control constructor plus explicit control methods for create/rename/enable/disable/delete. Restart tests preserve stable IDs/names and enabled state while intentionally clearing runtime leases; native endpoint identity and driver lifecycle remain open.
- Added `virtualDevices.plan` and `virtualDevices.apply` lifecycle APIs for create, rename, enable/disable, and delete. Plans validate the bounded registry, apply is idempotent and persists desired state, and every result retains the explicit unavailable-driver warning; control coverage is now 62 tests. Native provisioning and endpoint activation remain blocked.
- Added typed UI backend methods for virtual-device planning and apply, with demo-backend fail-closed behavior and live-client request normalization. UI coverage is now 58 tests and typecheck passes; no UI path provisions an endpoint while the driver is unavailable.
- Added explicit MCP tools `plan_virtual_device` and `apply_virtual_device`, forwarding lifecycle requests through the authorized shared control dispatcher. MCP tool catalog/process tests pass with 26 tools; native provisioning remains unavailable and explicitly reported.
- Persisted virtual-device plan records with five-minute expiry and reload-on-restart behavior. Apply removes the durable plan after successful desired-state persistence; storage coverage is now 36 tests and control coverage 63 tests. Driver operations and production signing remain open.
- Made virtual-device apply persistence atomic: bus desired-state replacement and durable plan deletion now commit in one SQLite transaction, preventing replayable plans after a successful state write. Storage/control focused suites and strict Clippy pass.
- Extended virtual-device inventory metadata with explicit render/capture capability flags, required privilege, restart impact, and client-impact placeholders. The unavailable-driver state remains explicit; control and contracts checks pass.
- Added authorization regression coverage proving virtual-device plan/apply requires explicit `deviceAdministration`; all ordinary read/editor/operator grants are denied before dispatch. The test target compiled and strict Clippy passed, but Windows Application Control blocked execution with OS error 4551, so no runtime-pass claim is made.
- Connected virtual-device apply idempotency to the durable operation journal and atomically persisted bus state, plan deletion, and replay result. A restart regression now replays the completed result before plan lookup; control coverage is 64 tests and storage remains 36.
- Bound virtual-device apply journal entries to a deterministic request hash. Reusing an idempotency key with a different plan now returns the stable `idempotencyConflict` error instead of replaying an unrelated result; same-request replay remains valid across restart.
- Centralized bounded in-memory operation retention across graph and virtual-device outcomes, evicting virtual request hashes with their cached results so repeated lifecycle operations cannot grow process memory without limit.
- Added operation-specific Windows audio errors for shared capture/render `IAudioClient::Initialize` failures. Diagnostics now identify capture versus render plus the exact HRESULT while retaining stable classification; 12 Windows-audio tests and strict Clippy pass. This does not claim the unresolved Rust live-open discrepancy is fixed and does not start audio.
- Replaced the visual editor's placeholder node strip with a read-only React Flow canvas. Nodes remain selectable and route edges are visible with fit-view, pan/zoom controls, and a list-view alternative; the canvas cannot mutate graph topology or audio state. UI tests (26), typecheck, production build, and diff checks pass.
- Re-ran the complete `tests/acceptance/m07-headless.ps1` wrapper from clean revision `76b4d2b` after client/method-scoped idempotency hardening. M01 CLI acceptance, MCP stdio interoperability, 68 control tests, 31 plugin-host unit tests, 8 worker-process tests, and strict Clippy passed without audio-device, driver, or machine-configuration actions.
- Strengthened client/method-scoped idempotency coverage with a SQLite-backed regression: two authenticated clients may reuse the same human-readable key, and each client's durable `operations.get` resolves only its own result. Focused control tests and strict Clippy pass; no audio or machine state is involved.
- Revalidated the locked workspace at clean revision `5670dde`: 325 unit/integration tests, all doc-tests, strict workspace Clippy, formatting, documentation acceptance (50 Markdown files/147 local links), and M08 unsigned artifact preparation/verification passed. Temporary release output was removed; native audio, driver, signing, installer, and machine-configuration gates remain unchanged.
- Added a stable readable [API reference](../../operations/api-reference.md) covering all 47 discovered methods with permissions/side effects, the current node catalog, presets, generic CLI invocation, and MCP parity. It points to runtime `schema --json` for exact machine-readable contracts; documentation validation passes and unavailable native capabilities remain explicit.
- Added a redacted `diagnostics --output <absolute-path>` CLI export with exclusive destination creation, preserving the read-only backend snapshot and preventing accidental support-bundle overwrite. CLI diagnostics tests, strict Clippy, formatting, and documentation validation pass; no audio or machine configuration is changed.
- Re-ran `tests/acceptance/m07-headless.ps1` at clean revision `e1e72f9`: M01 CLI (21 tests), MCP stdio, control (68), plugin-host (31), worker-process (8), and strict Clippy passed without audio-device, driver, or machine-configuration actions.
- Hardened documentation validation to compare the readable API reference against the authoritative Rust `API_METHODS` table, rejecting missing, extra, or duplicate method rows. Direct and acceptance documentation checks pass with 51 Markdown files and 150 local links.
- Hardened shared CLI output creation to reject missing, non-directory, and symlink/reparse-point parents before exclusive writes. This protects redacted diagnostics and graph-plan files from redirected output paths; focused CLI tests and strict Clippy pass without machine-state access.
- Re-ran the complete M07 headless acceptance at clean revision `e7b5ccc`: M01 CLI (21), MCP stdio, control (68), plugin-host (31), worker-process (8), and strict Clippy passed without audio-device, driver, or machine-configuration actions.
- Hardened the shared CLI output writer to require absolute paths, flush and sync completed output, and remove partial files after write/flush failure. Diagnostics and graph-plan exports now fail closed against incomplete artifacts; focused CLI tests and strict Clippy pass.
- Re-ran the complete M07 headless acceptance at clean revision `fc05361`: M01 CLI (21), MCP stdio, control (68), plugin-host (31), worker-process (8), and strict Clippy passed after durable output-writer hardening.
- Added the explicit `diagnostics export --output <absolute-path>` CLI form, with validation requiring an output path while preserving the existing `diagnostics --output` compatibility form. CLI regression coverage exercises the named export command without touching audio or machine configuration.
- Re-ran the complete M07 headless acceptance at clean revision `c133b23`: M01 CLI (21), MCP stdio, control (68), plugin-host (31), worker-process (8), and strict Clippy passed with diagnostics export and graph-plan output protections enabled.
- Added direct output-writer regression coverage proving relative destinations and missing parents are rejected before file creation. The focused CLI test and strict Clippy pass; no machine or audio state is involved.
- Rebuilt the checked-in native WASAPI probe at the current tip with Visual Studio Community 2026, MSVC 14.51, and the installed Windows SDK/WDK. Compile-only validation succeeded; generated `main.exe`/`main.obj` were removed immediately, and the probe was not run.
- Added `tests/acceptance/m00-native-build.ps1`, a disposable compile-only acceptance wrapper for the native WASAPI probe. It refuses a pre-existing generated object, uses a unique temporary executable, and cleans both outputs in `finally`; it cannot start audio or install a driver.
- Re-ran the complete M07 headless acceptance at clean revision `e377adc`: M01 CLI (22), MCP stdio, control (68), plugin-host (31), worker-process (8), and strict Clippy passed with output-path safety coverage.
- Revalidated the repository-local VST3 SDK acceptance at clean revision `22da8a2` with Visual Studio Community 2026, MSVC 14.51.36231, and Windows SDK 10.0.28000.0: the pinned SDK built, 51 SDK tests and 1,598 official validator tests passed, and the offline loader passed. Generated loader outputs were removed; no system registration or audio configuration changed.
- Requalified the complete M08 release wrapper at clean revision `e5f240b`: optimized locked CLI/plugin-worker artifacts, complete SBOM, third-party notices, sanitized provenance, checksums, and exact-content verification passed in a disposable directory. Unsigned publication blockers were asserted and temporary output was removed; no installer, driver, signing, or audio configuration action occurred.
- Re-ran the complete M07 headless acceptance at clean revision `308a5c9`: M01 CLI (22), MCP stdio, control (68), plugin-host (31), worker-process (8), and strict Clippy passed. Temporary state was isolated and no audio device, driver, or machine configuration was accessed.
- Revalidated the locked workspace at clean revision `c689e9a`: all workspace all-target tests passed, including 22 CLI, 68 control, 35 domain, 25 DSP, 40 engine, 31 plugin-host, 8 worker-process, 5 protocol, 30 recording, 36 storage, 14 transport, and 12 Windows-audio tests; strict Clippy with `-D warnings` also passed. No audio stream was started and no machine configuration changed.
- Added a Windows-gated end-to-end MCP named-pipe interoperability regression: the CLI `mcp serve --pipe` proxy now forwards an actual `tools/call` through an authenticated local backend and preserves the backend response contract. Focused MCP tests, CLI Clippy, formatting, and the complete M07 headless wrapper pass; only temporary SQLite/named-pipe state is used.
- Revalidated the locked workspace at clean revision `1fcbbfa`: all workspace all-target tests passed, including 22 CLI, 68 control, 35 domain, 25 DSP, 40 engine, 31 plugin-host, 8 worker-process, 5 protocol, 30 recording, 36 storage, 14 transport, and 12 Windows-audio tests; strict workspace Clippy with `-D warnings` and formatting passed. No audio stream or machine configuration was touched.
- Requalified the M04 and M05 acceptance wrappers at clean revision `e34438f`: 25 DSP, 30 recording, and 59 UI tests passed with UI typecheck, disposable production output, formatting, and strict Clippy. Temporary outputs were removed; no audio, driver, or machine configuration changed.
- Re-ran the M00 native WASAPI probe compile acceptance at clean revision `904c115` with the installed Visual Studio Community 2026/MSVC and Windows SDK/WDK. The C++ probe compiled successfully and generated outputs were cleaned; it was not executed and no audio, driver, signing, or machine configuration action occurred.
- Added a recovery regression proving `poll_and_restart` leaves a healthy worker running and usable, rather than needlessly replacing it. Plugin-host tests, strict Clippy, formatting, and the complete M07 headless acceptance pass; no audio or machine configuration was accessed.
- Re-ran the repository-local VST3 SDK installer with a process-scoped PowerShell execution-policy bypass, confirming the pinned checkout and recursive submodules without changing the machine policy. The compile-only M06 acceptance then passed with VS2026/MSVC 14.51.36231 and Windows SDK 10.0.28000.0: 51 SDK self-tests, 1,598 official validator tests, and the offline loader all passed; generated outputs were cleaned and no system plugin or audio configuration was changed.
- Added process-level coverage proving `poll_and_restart` cannot bypass the STATE-10-style worker quarantine boundary: after the third recorded fault, it returns the preserved quarantine decision without spawning a replacement. All 8 worker-process tests and plugin-host strict Clippy pass.
- Promoted the worker quarantine decision to a typed `WorkerProcessError::Quarantined` result, so outer recovery code cannot confuse a policy latch with a transient protocol failure. The process-level quarantine regression, all 8 worker-process tests, and plugin-host strict Clippy pass.
- Requalified M07 headless acceptance at clean revision `8715875`: M01 CLI (22), MCP interoperability (2), control (68), plugin-host unit tests (31), worker-process tests (8), and strict Clippy passed. Requalified M08 unsigned release preparation as well; optimized artifacts, SBOM/provenance, notices, hashes, and verification passed in a disposable directory. Native audio, driver, signing, installer, and machine-configuration gates remain unchanged and open.
- Corrected the M03 milestone status to reflect the implemented portable virtual-bus registry, durable desired-state lifecycle, authorization, and CLI/MCP/UI parity while retaining native driver provisioning, endpoint identity, and live routing as open gates. Documentation validation remains required before committing this status correction.
- Added the disposable compile-only M00 native WASAPI probe to the Windows CI job. CI now checks the repository's native C++/Windows SDK build boundary on every Windows run while cleaning generated outputs and never starting audio or changing machine configuration.
- Made Windows endpoint snapshot diffs deterministic by sorting private copies by opaque endpoint ID, insulating lifecycle/event consumers from unstable OS enumeration order. Windows-audio coverage is now 13 tests with strict Clippy; this remains read-only metadata handling and does not open streams.
- Added stable retryability and remediation helpers to `AudioFailureKind`/`AudioError`, distinguishing transient device/service conditions from errors requiring configuration or permission changes. Windows-audio coverage remains 13 tests with strict Clippy and formatting; no stream is opened.
- Revalidated the Windows audio adapter after the deterministic-diff and failure-remediation changes: all 13 metadata/error-path tests and strict Clippy pass, with no endpoint stream opened or machine configuration changed.
- Extended the Windows audio error regression to exercise retryability and remediation through concrete `AudioError` HRESULT instances, not only enum values. The 13-test adapter suite, formatting, and strict Clippy remain green without opening a stream.
- Fixed a native adapter cleanup edge: capture/render setup now creates the event handle before requesting the COM-allocated mix format, so an event-creation failure cannot leak that format buffer. The Windows-audio tests, strict Clippy, formatting, and compile-only native probe acceptance pass; no stream was opened.
- Hardened capture packet copying after WASAPI `GetBuffer`: overflow or destination-size validation now releases the acquired packet before returning, preserving the buffer ownership invariant. Windows-audio tests, strict Clippy, formatting, and compile-only native probe acceptance pass without opening a stream.
- Added a typed `AudioError::hresult()` accessor and routed failure classification through it, preserving the original numeric HRESULT for machine-readable diagnostics without string parsing. Windows-audio tests, strict Clippy, formatting, and compile-only native probe acceptance pass with no stream opened.
- Preserved cleanup failures in capture packet validation: if `ReleaseBuffer` itself fails after `GetBuffer`, the original WASAPI HRESULT is returned instead of being masked by the validation error. Windows-audio tests, strict Clippy, formatting, and native compile-only acceptance pass without opening a stream.
- Corrected stale M04 evidence so the implemented and tested ten-band `GraphicEq` is no longer listed as missing; only its graph/API integration remains open. Documentation validation remains green.
- Requalified M04 at clean revision `914718a`: all 25 DSP tests, including 60-second pitch-duration extremes, all 30 recording tests, formatting, and strict Clippy passed. Native realtime graph integration remains open; no audio device or machine configuration was touched.
- Requalified M05 at clean revision `61a0bc1`: TypeScript typecheck, all 59 UI tests, and the disposable three-file production build passed. Manual visual/accessibility acceptance and native shell injection remain open; no audio, driver, or machine configuration changed.
- Revalidated the full locked Rust workspace at clean revision `02dc687`: all 325 unit/integration tests across every workspace target passed, including 40 engine, 25 DSP, 30 recording, 36 storage, 31 plugin-host, 8 worker-process, 14 transport, and 13 Windows-audio tests. Native stream execution, driver, signing, installer, and machine-configuration gates remain explicitly open.
- Hardened shared-render byte submission against truncating `usize`-to-`u32` frame-count conversion: oversized source buffers now return the typed invalid-frame-size error instead of wrapping. Formatting, the 13-test Windows-audio suite, strict Clippy, and compile-only M00 native acceptance pass; no stream or machine configuration was touched.
- Completed the API-08 event-category filter slice: `events.subscribe` now validates a bounded 1–32 item category list and combines it with the existing cursor/session filters; Rust and TypeScript contracts advertise the field, with a regression for selective replay and oversized filters. Full workspace library tests, contracts/UI typechecks and tests, strict Clippy, and documentation acceptance pass; launching the CLI binary target remains blocked by host Application Control error 4551.
- Exposed the API-08 category filter through the typed UI backend adapter, preserving the existing cursor/session call shape while forwarding optional category lists. UI typecheck and all 59 UI tests pass; the adapter remains transport-only and does not alter audio or machine state.
- Added repeatable bounded `watch --category NAME` CLI filtering over `events.subscribe`, including client-side length/count validation and usage documentation. CLI formatting, compilation, and strict Clippy pass; runtime launch of the freshly rebuilt test executable remains blocked by host Application Control error 4551, while the corresponding control filter is runtime-tested.
- Extended the headless runbook with a bounded state-event replay example using repeatable category filters, including cursor-resynchronization and no-machine-state side-effect boundaries. Documentation acceptance passes.
- Bounded authenticated persistent transport sessions to 500 request frames on both server and client helpers, rejecting oversized lifetimes before pipe/platform access. Transport coverage is now 15 tests with formatting and strict Clippy green; an unbounded production daemon lifecycle remains open.
- Added `round_trip_session_many` so authenticated persistent clients can exchange distinct bounded JSON-RPC requests on one connection; the Windows regression verifies request IDs remain paired in order. Transport coverage is now 16 tests with strict Clippy and formatting; production daemon shutdown/restart ownership remains open.
- Added a Windows-gated authenticated control-session regression proving one bounded persistent connection can perform a recovery-state mutation and then replay its category-filtered event using a distinct request. Transport coverage is now 17 tests with strict Clippy and formatting; production daemon lifecycle ownership remains open.
- Persisted the control-plane backend epoch in SQLite and advanced it atomically when a durable `ControlPlane` opens, so reconnecting clients can detect a real backend restart instead of seeing epoch `1` again. Storage coverage is now 37 tests and control coverage 70 tests; strict Clippy, formatting, and restart regressions pass. This uses only test/durable control state and does not access audio or machine configuration.
- Added a file-backed enrollment restart regression: a client authorized before shutdown is reloaded from SQLite by the next control instance and can pass the same scoped authorization boundary, while the operation remains configuration-only. Control coverage is now 71 tests with strict Clippy, formatting, and diff checks green.
- Requalified M08 unsigned release preparation at the current revision: optimized locked CLI/plugin-worker artifacts, SBOM/notices, manifest hashes, and byte counts were generated and verified in a disposable directory. The manifest correctly remains unsigned and not publication-ready; driver, signing, installer, and clean-machine gates remain open.
- Requalified M07 headless acceptance at the current revision: 22 CLI tests, MCP stdio and named-pipe interoperability tests, 71 control tests, 31 plugin-host tests, 8 worker-process tests, and strict Clippy all passed. The acceptance used temporary state only and did not access audio devices, drivers, or machine configuration.
- Requalified the M00 native WASAPI probe compile at the current revision with the installed Visual Studio Community 2026/MSVC and Windows SDK/WDK toolchain. Compilation passed; the probe was not executed and no audio stream, driver, signing mode, or machine configuration was touched.
- Added typed UI-backend forwarding for the existing explicit `recordings.rename` operation, preserving the backend's same-directory file-safety boundary. UI typecheck and all 60 UI tests pass; no recording file or machine state was changed.
- Added typed UI-backend forwarding for the existing explicit `recordings.reveal` operation, preserving the backend's missing-file and OS-action result contract. UI typecheck and all 61 UI tests pass; the adapter regression performs no file or Explorer action.
- Requalified the complete M05 UI acceptance wrapper at the current revision: TypeScript typecheck, all 61 Vitest tests, and a disposable three-file Vite production build passed. Temporary output was removed; manual visual/accessibility acceptance and native shell injection remain open, with no audio, driver, or machine configuration changes.
- Reinstalled and requalified the repository-local VST3 SDK at the current revision. The pinned checkout remained at `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`; the VS2026 Release build, 51 SDK self-tests, official validator (1,598 passed/0 failed), and offline native loader all passed. The first sandboxed build was denied by MSBuild file tracking, so the same acceptance was rerun with authorized native build access; no global SDK/plugin registration or audio configuration changed.
- Revalidated the locked Rust workspace at the current revision: all 325 tests across CLI/MCP, control, domain, DSP, engine, plugin host/worker, protocol, recording, storage, transport, and Windows-audio metadata targets passed; formatting and strict Clippy (`-D warnings`) also passed. No audio stream, driver, signing, installer, or machine configuration action occurred.
- Requalified M07 headless acceptance at the current revision: 22 CLI tests, MCP stdio/named-pipe interoperability, 71 control tests, 31 plugin-host tests, 8 worker-process tests, and strict Clippy passed. M08 unsigned artifact preparation also passed with optimized locked binaries, SBOM/notices, provenance, hashes, and blocker assertions; temporary output was removed and no installer, driver, signing, or audio configuration action occurred.
- Added a connected-only M05 recording action panel for the existing authorized rename, reveal, and recycle APIs. Rename remains backend-directory constrained, reveal reports missing files without launching an action, and recycle has separate preview/confirmation controls; disconnected mode disables all file actions. UI typecheck, all 61 tests, and the disposable production build pass; no recording file or machine configuration was changed.
- Extended the M03 managed virtual-device panel to cover create, rename, enable/disable, and delete desired-state operations. Every operation requires explicit plan/apply sequencing, shows the required `deviceAdministration` scope and unavailable-driver reason, and never synthesizes or activates a native endpoint. M05 UI typecheck, all 61 tests, and the disposable production build pass.
- Requalified the compile-only M00 native WASAPI probe at the current revision with the installed VS2026/MSVC and Windows SDK/WDK toolchain. `main.cpp` compiled and temporary outputs were cleaned; the probe was not executed and no audio stream, driver, signing mode, or machine configuration was touched.
- Hardened the portable engine compiler to reject multi-node sessions with no enabled connections, preventing disconnected processors from being applied serially to the same audio block. Added a regression; the engine suite now passes 41 tests with formatting and strict Clippy, without opening audio or changing machine state.
- Added a `virtualDevice.changed` event after a successful virtual-device desired-state apply and retained global events when `events.subscribe` uses a session filter, so other connected clients can refresh without receiving unrelated session events. Control tests and strict Clippy pass; no native endpoint or machine configuration is changed.
- Added session-scoped events for successful recording metadata edits, renames, library-entry removal, and confirmed recycle actions. Read-only preview/missing outcomes remain event-free; the 71-test control suite, formatting, and strict Clippy pass without changing test files or machine state.
- Updated `system.describe` to advertise the emitted `virtualDevice.changed` and recording mutation event categories, keeping discovery consistent with runtime replay. Control tests, formatting, and strict Clippy pass; no native endpoint or machine configuration is changed.
- Added optional idempotency keys to recording metadata, rename, library-entry removal, and confirmed recycle mutations. Keyed requests are scoped by client and method, hash-bound, durably journaled, replayable after restart, and reject same-key/different-payload conflicts; previews and missing-file responses remain side-effect free. Control tests (71) and strict Clippy pass; no audio endpoints or machine configuration were touched.
- Requalified the documentation, M01 CLI, and M04 DSP/recording acceptance wrappers after the recording idempotency and contract changes: documentation validation passed for 51 Markdown files and 150 local links; M01 CLI passed; M04 passed all 25 DSP and 30 recording tests plus formatting/Clippy. The wrappers used only disposable/test state and did not access audio devices or change machine configuration.
- Requalified M05, M06, M07, and M08: UI typecheck/61 tests/disposable build passed; the repository-local VST3 SDK built with VS2026 and passed 51 self-tests, official validator 1,598/0, and offline loader; M07 headless passed; and unsigned M08 artifacts verified. The first M06 attempt hit host FileTracker access denial, then passed with the authorized elevated build; no system plugin, driver, signing, installer, endpoint, or machine configuration was changed.
- Requalified M00 compile-only native probe plus M05–M08 wrappers at the current tip: M00 compiled, M05 UI passed, M06 passed with the local VS2026 SDK build, M07 headless passed, and M08 unsigned artifacts verified. The initial non-elevated M06 FileTracker denial was resolved by the authorized elevated repository-local build; all temporary outputs were cleaned and no audio stream, driver, signing mode, installer, or machine configuration was used.
- Added scoped, request-hash-bound durable idempotency to `sessions.start` and `sessions.stop`, retaining the existing no-key compatibility path. Keyed lifecycle retries replay exact results and cross-session key reuse is rejected; the focused control regression and strict Clippy pass without starting native audio or changing machine state.
- Extended the same keyed durable retry contract to `sessions.delete`: successful deletion outcomes replay before resource lookup, and keys are scoped/hash-checked through the shared operation journal. Control coverage is now 72 tests with strict Clippy; no audio or machine configuration is touched.
- Extended keyed durable retry support to `sessions.create` and `sessions.duplicate`, hashing the complete session/request identity before resource creation. Existing callers remain compatible when no key is supplied; control tests and strict Clippy pass without native audio or machine changes.
- Added scoped, hash-bound durable replay for `safety.setPrivacyMute`; a keyed retry now returns the original process-local result across control restart without duplicating the state event. The existing authorization/restart regression and strict Clippy pass; Windows privacy settings remain untouched.
- Added the same durable keyed replay path to `recovery.clearSafeMode`, including a regression proving repeated clears return the original result without duplicating the recovery event. Control tests and strict Clippy pass; this changes only AudioRouter recovery state.
- Extended scoped, hash-bound durable idempotency to client enrollment and revocation mutations, with operation lookup support for restart inspection. Control coverage remains green at 72 tests with strict Clippy; authorization data only is changed, never Windows account or audio configuration.
- Aligned the TypeScript request contracts and UI backend adapter with the keyed session, recovery, privacy, and enrollment mutation APIs. Contracts typecheck, UI typecheck, and all 61 UI tests pass; optional keys preserve existing callers and no native or machine state is touched.
- Added CLI support for `--idempotency-key` on privacy mute and recovery safe-mode clearing, forwarding keys through the shared control dispatcher while preserving existing invocations. The 22-test CLI suite and strict Clippy pass; only AudioRouter state is affected.
- Routed CLI session create, duplicate, start, stop, and delete through the shared JSON-RPC dispatcher and added optional `--idempotency-key` forwarding. The 22-test CLI suite and strict Clippy pass; session changes remain confined to the selected test/database state.
- Added keyed durable replay to `operations.cancel`, preserving its completed-operation/no-undo semantics while making retries return the original result and adding method-scoped operation lookup. The focused control test and strict Clippy pass; no runtime audio or machine state is changed.
- Exposed `operations.cancel` idempotency keys in the TypeScript contract and CLI (`--idempotency-key`); CLI tests, strict Clippy, and contracts typecheck pass. The root-level npm check was correctly redirected to `contracts`, since the repository has no root package manifest.
- Added a CLI regression that performs a keyed recording metadata mutation through two fresh control instances and verifies the second invocation replays the durable result. The focused CLI test and strict Clippy pass; no recording file or audio device is touched.
- Wired the UI’s user-triggered recording, session lifecycle, privacy-mute, and recovery actions to generate one idempotency key per action; recycle previews intentionally remain unkeyed until confirmation. M05 typecheck, all 61 UI tests, and the disposable production build pass without audio or machine configuration changes.
- Added the first real recorder API slice: `recorders.arm/start/pause/resume/split/stop` are now authoritative discovered methods with Record-scope authorization, bounded input/output schemas, and control dispatch backed by the existing frame-accurate `RecorderController`. A regression covers the complete lifecycle and rejects a second stop; 73 control tests, strict Clippy, formatting, contracts typecheck, and documentation validation pass. This is lifecycle/control-state evidence only: it does not open audio, create files, or claim realtime recording integration.
- Hardened recorder lifecycle mutations with client/method-scoped, request-hash-bound idempotency. Same-key retries replay the exact lifecycle result and a different frame/payload returns the stable conflict message; two focused regressions, strict Clippy, formatting, and diff checks pass. No audio stream or recording file is opened or created.
- Added MCP parity for the recorder lifecycle: six explicit tools expose arm/start/pause/resume/split/stop with frame and idempotency inputs, forwarding through the authorized shared dispatcher. MCP catalog and stdio/named-pipe interoperability tests plus strict CLI Clippy pass; no audio stream or recording file is opened.
- Added typed UI-backend parity for recorder arm/start/pause/resume/split/stop actions, forwarding explicit frame boundaries and optional idempotency keys while keeping disconnected startup fail-closed. UI typecheck and 62 tests pass; no audio stream or file action is performed.
- Added a connected-only UI recorder panel exposing arm/start/pause/resume/split/stop with explicit frame input, lifecycle status, retry keys, and fail-closed disconnected behavior. UI typecheck, 62 tests, and diff checks pass; this does not open audio or create recording files by itself.
- Wired recorder lifecycle checkpoints into the SQLite-backed control plane: first use reloads the validated checkpoint, each transition persists its boundaries, and a restart can continue the recorder state without opening audio or creating files. Output normalization now matches the documented lowercase/camel-case contract while preserving the versioned checkpoint format. Three recorder regressions and strict Clippy pass.
- Added `recorder.changed` state events for successful recorder lifecycle transitions, with session resource identity and revision, and advertised the category through discovery. Idempotent replays remain event-free; recorder, discovery, Clippy, and documentation checks pass without audio access.
- Added a `recorder <arm|start|pause|resume|split|stop>` CLI convenience surface with database-backed dispatch, explicit frame validation, optional idempotency keys, and help documentation. The constructor now hydrates persisted sessions and bounded history together; this fixed a history-cursor regression. CLI (22), control (75), strict Clippy, formatting, and diff checks pass.
- Added direct CLI regression coverage for database-backed recorder arm/start commands, including explicit frame and idempotency arguments. The disposable SQLite test passes alongside strict CLI Clippy, formatting, and diff checks; no audio or recording file is touched.
- Exposed the specified read-only `sessions.export` JSON-RPC method through the authoritative domain catalog, control schemas/dispatch, TypeScript contracts, and API reference. It returns the canonical persisted session without file, audio, or machine side effects; control (75), domain (35), Clippy, contracts, formatting, and documentation gates pass.
- Exposed `sessions.importPlan` and `sessions.importCommit` using validated stopped-session candidates, five-minute in-memory plans, duplicate-ID rejection, and hash-bound durable commit replay. Added authoritative Rust/TypeScript schemas and API reference entries; the focused import regression, strict Clippy, formatting, and diff checks pass. This API does not install drivers, activate audio, or write outside the selected backend database.
- Added UI backend parity for canonical session export and stopped import plan/commit, with typed result contracts, disconnected fail-closed behavior, and live transport forwarding. UI coverage is now 63 tests; UI/contracts typechecks and diff checks pass without file-picker, audio, or machine-state actions.
- Routed the CLI JSON `export` and `import` workflows through the shared control dispatcher: export uses `sessions.export`, while import validates with `sessions.importPlan` then commits with `sessions.importCommit` and an explicit/deterministic idempotency key. CLI coverage is now 23 tests; strict Clippy, formatting, history restart behavior, and documentation validation pass without audio or machine changes.
- Added explicit MCP session portability tools for export, import planning, and import commit, forwarding through the shared authorized dispatcher. The MCP catalog now contains 35 tools; catalog, stdio/named-pipe interoperability, strict Clippy, formatting, and diff checks pass without file or audio side effects.
- Reconciled stale M04 evidence wording with the implemented state: durable control-plane recorder checkpoints, flush-ordered WAV/incremental-FLAC worker hooks, streaming FLAC recovery, and bounded metadata are now covered. The evidence now retains native realtime, hardware timing, and production performance as explicit unclaimed gates; documentation validation passes.
- Added a real `plugins.list` inventory method backed by the last explicit bounded scan in the control plane. It returns an empty inventory before scanning, never loads plugin code or scans implicitly, and is exposed through the CLI and MCP with `pluginScan` authorization. Control/CLI/MCP tests, strict Clippy, contracts, formatting, and documentation validation pass.
- Added `plugins.retry` as an authenticated, idempotency-key-bound bounded inventory refresh. It explicitly rescans the selected directory after a prior scan issue, retains the result for `plugins.list`, and does not load plugin code or claim worker restart behavior; CLI/MCP parity and control regressions pass with strict Clippy and documentation validation.
- Updated the readable API reference to reflect the current 58-method catalog after session portability, recorder lifecycle, and plugin inventory/retry additions. This is documentation-only; the method-table/link validator passes and all native audio/driver/signing boundaries remain unchanged.
- Re-ran the repository-local VST3 SDK installer and complete M06 acceptance with the installed Visual Studio Community 2026 toolchain. The pinned checkout (`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`) and submodules were verified; the SDK built, 51 SDK self-tests and 1,598 official validator tests passed, and the offline loader passed. Generated build artifacts remain local/ignored; no system SDK, plugin registration, driver, audio stream, or machine configuration was changed.
- Added the portable M07 recovery-supervisor application boundary. After a runtime crash, the control plane stops all fake runtimes, restarts only eligible non-recording sessions, and leaves every session stopped in latched safe mode after three recent crashes; armed/recording/paused/stopping recorder sessions are excluded. Control tests (78), formatting, strict Clippy, and diff checks pass. Native process supervision, route restart, and audio-stream recovery remain explicitly open.
- Requalified the complete M07 headless acceptance at commit `5a58a13`: CLI (23), MCP stdio/named-pipe interoperability (2), control (78), plugin-host (31), worker-process (8), and strict Clippy passed. Temporary test state only; no audio device, driver, or machine configuration was accessed.
- Requalified the unsigned M08 release preparation at the current tip: optimized pinned artifacts, manifest, notices/SBOM, provenance, checksums, and verification passed in a disposable directory. The manifest remained explicitly unsigned and not publication-ready; temporary output was removed and no installer, driver, signing, audio endpoint, or machine configuration was used.
- Requalified the current-tip compile-only M00 native probe with Visual Studio Community 2026/MSVC and Windows SDK/WDK: `main.cpp` compiled and outputs were cleaned. The probe was not executed and no audio stream, driver, signing mode, or machine configuration was touched.
- Requalified the complete portable M04 acceptance: all 25 DSP tests and 30 recording tests passed, including 60-second pitch-duration extremes, with formatting and strict Clippy. No audio device or machine configuration was accessed.
- Requalified the complete M05 UI acceptance: TypeScript typecheck, all 63 Vitest tests, and the disposable three-file Vite production build passed. Temporary output was removed; manual visual/accessibility acceptance and native shell injection remain open.
- Requalified the full Rust workspace at the current tip: all workspace unit, integration, and doc tests passed (23 CLI, 2 MCP, 78 control, 35 domain, 25 DSP, 41 engine, 31 plugin-host, 8 worker-process, 5 protocol, 30 recording, 37 storage, 17 transport, and 13 Windows-audio tests), with workspace Clippy, formatting, and diff checks green. Tests used temporary/simulated state and did not alter audio configuration.
- Added `runtime.crashed` event publication to the portable recovery supervisor before it stops affected fake runtimes; eligible restorations continue to publish `runtime.started`. Control tests (78), strict Clippy, formatting, and diff checks pass. This remains fake-runtime observability and does not claim native route recovery.
- Strengthened the M07 recovery regression to verify event ordering for crash notifications and eligible restart events. The focused control test, strict Clippy, formatting, and diff checks pass.
- Corrected the M07 milestone and release checklist summaries to reflect that portable crash-recovery orchestration is implemented while sign-in background lifecycle and native process/audio restart remain open.
- Added durable SQLite recovery-orchestration coverage: the portable supervisor restores an eligible fake session before the crash threshold and preserves a latched safe-mode stop after the third crash. The focused control test, strict Clippy, formatting, and diff checks pass.
- Re-ran the complete control suite after the durable recovery regression: 79 tests and doc tests passed with strict Clippy, formatting, and diff checks; no native audio or machine configuration was touched.
- Requalified the complete M07 headless acceptance at commit `c4e8e73`: CLI (23), MCP stdio/named-pipe interoperability (2), control (79), plugin-host (31), worker-process (8), and strict Clippy passed. Temporary test state only; no audio device, driver, or machine configuration was accessed.
- Requalified M07 again at commit `7a027ba` after durable recovery coverage: CLI (23), MCP stdio/named-pipe interoperability (2), control (79), plugin-host (31), worker-process (8), and strict Clippy passed. Temporary test state only; no audio device, driver, or machine configuration was accessed.
- Removed the unused legacy UI virtual-device panel, leaving the connected lifecycle panel as the single authoritative surface. M05 typecheck, all 63 UI tests, and the disposable production build pass; manual visual/accessibility and native shell acceptance remain open.
- Completed UI mutation idempotency coverage: graph commit, virtual-device apply, recording-entry removal, and session create/duplicate/delete now all use the shared UUID-backed retry-key helper. M05 typecheck, all 63 UI tests, and the disposable production build pass; no audio, driver, recording file, or machine configuration was accessed.
- Reconciled stale M00 wording: the checked-in native process-loopback probe already covers asynchronous activation, include/exclude modes, and bounded data reads; only controlled per-process attribution and physical latency remain unclaimed. Documentation validation passes, with no audio or machine configuration changed by this documentation-only correction.
- Current-tip portable qualification completed: M04 passed 25 DSP and 30 recording tests; M05 passed TypeScript typecheck, 63 UI tests, and the disposable three-file production build; M07 passed CLI (23), MCP interoperability (2), control (79), plugin-host (31), worker-process (8), and strict Clippy; M08 unsigned artifact preparation and verification passed. Temporary outputs/state were cleaned, and no audio device, driver, signing, installer, or machine configuration was changed.
- Requalified M06 at the current tip with the repository-local pinned VST3 SDK and VS2026: SDK build, 51 self-tests, official validator (1,598 passed/0 failed), and offline loader passed. Generated artifacts were cleaned; no system plugin, audio stream, driver, or machine configuration was changed.
- Added UI adapter regressions asserting idempotency keys are forwarded for recording-entry removal and session create/duplicate/delete mutations. M05 typecheck, all 63 UI tests, and the disposable production build pass; no audio or machine configuration was accessed.
- Requalified the full locked Rust workspace at the current tip: 346 unit/integration tests passed across all targets (23 CLI, 2 MCP, 79 control, 35 domain, 25 DSP, 41 engine, 31 plugin-host, 8 worker-process, 5 protocol, 30 recording, 37 storage, 17 transport, and 13 Windows-audio), including the 60-second pitch boundary cases. This used temporary test state and did not access or change machine audio configuration.
- Hardened the UI idempotency-key fallback: it now prefers UUIDs, then browser cryptographic random values, and finally a timestamp/monotonic counter rather than Math.random. M05 typecheck, all 63 UI tests, and the disposable production build pass; no audio or machine configuration was accessed.
- Requalified full workspace strict Clippy at the current tip with `cargo clippy --workspace --all-targets --locked -- -D warnings`; all targets passed. No runtime audio, driver, signing, or machine configuration action occurred.
- Requalified the compile-only M00 native WASAPI probe at the current tip with the installed Visual Studio Community 2026/MSVC and Windows SDK/WDK toolchain. Temporary executable/object outputs were cleaned; the probe was not executed and no audio stream, driver, signing mode, or machine configuration was touched.
- Extracted the UI idempotency-key helper into a directly tested module. The M05 suite now passes 64 tests with typecheck and disposable production build, including uniqueness/format coverage; no audio or machine configuration was accessed.
- Hardened the repository-local VST3 SDK installer to reject reparse-point destinations and verify the pinned checkout's origin URL before reuse or force-update. This prevents an unrelated checkout from being treated as the SDK; no installed SDK or machine configuration was changed.
- Added a disposable installer provenance regression that rejects an existing Git checkout with the wrong SDK origin, and wired it into Windows CI. The test uses only temporary Git metadata and changes no SDK, plugin, driver, audio, or machine configuration.
- Hardened release verification to reject a reparse-point artifact root directory before resolving manifest contents. Release safety regressions and documentation validation pass; no installer, signing, driver, audio, or machine configuration action occurred.
- Corrected release-root reparse validation to inspect the requested manifest path and every lexical parent before canonicalization, and added a symlinked-root regression. Release safety tests pass; no installer, signing, driver, audio, or machine configuration action occurred.
- Hardened release preparation to reject reparse points anywhere in the existing output-parent chain, preventing redirected artifact creation through a grandparent junction/symlink. Release safety and documentation checks remain green; no installer, signing, driver, audio, or machine configuration action occurred.
- Hardened the SDK installer to reject reparse-point parents when creating a new checkout, and extended its disposable provenance regression to cover redirected parents. The installer acceptance and documentation checks pass without changing the installed SDK or machine configuration.
- Corrected SDK destination validation to allow ordinary missing parent directories while still auditing the nearest existing ancestor chain for reparse points. Installer provenance regressions and documentation checks pass.
- Requalified the complete M08 unsigned release-preparation wrapper after release path hardening: optimized artifacts, manifest, notices/SBOM, provenance, checksums, blocker assertions, and verification passed in a disposable directory, which was removed afterward.
- Validated the VST3 SDK installer from a clean disposable destination: it cloned the pinned revision, initialized all seven SDK submodules, verified the required hosting header, and removed the temporary checkout. The repository-local SDK and machine audio configuration were unchanged.
- Revalidated the full locked workspace at pushed revision `041e8c4`: 346 all-target tests passed, strict workspace Clippy with `-D warnings` passed, formatting and diff checks passed, and no audio device, driver, signing mode, installer, or machine audio configuration was accessed.
- Closed an M05 node-library parity gap by exposing the authoritative M02 `application-capture`, `endpoint-loopback`, and `physical-output` entries as read-only unavailable choices with the same adapter reason as `physical-input`. UI tests cover all four entries; no native route or machine state is touched.
- Improved M05 library accessibility and error clarity: unavailable node choices now expose their exact capability reason in visible text and accessible labels, while available choices retain concise labels. UI unit coverage validates both forms; no native route or machine state is touched.
- Clarified the SDK setup runbook with the exact recursive-submodule verification command and expected seven-submodule result, matching the clean first-run installation evidence. Documentation validation is required before committing this documentation-only update.
- Added a read-only `processors` catalog to `system.describe` and the TypeScript discovery contract for the seven implemented DSP primitives, including typed ranges/defaults and pitch latency. Every entry remains explicitly unavailable pending M04 graph/runtime integration; no processing or machine state changed.
- Connected the read-only processor catalog to the UI snapshot: the new DSP panel shows category, exact unavailable reason, and declared latency, and disables any implication of activation until graph/runtime integration exists. UI tests and typecheck cover the presentation formatter; no audio or machine state changed.
- Added the read-only `processors.list` API method, returning the same seven-entry DSP catalog as `system.describe` with observer-level permission and no side effects. Contract, dispatch, and adapter parity validation are required before committing.
- Requalified the pushed current revision `0385f08` with the safe acceptance sweep: compile-only M00 native build; M04 (25 DSP and 30 recording tests); M05 (64 UI tests, typecheck, disposable production build); M07 (23 CLI, 2 MCP interoperability, 79 control, 31 plugin-host, 8 worker-process tests, and strict Clippy); M08 unsigned artifact preparation/verification; and documentation validation (51 Markdown files, 150 local links). Temporary outputs were cleaned and no audio device, driver, signing mode, installer, or machine audio configuration was accessed.
- Added the current qualification snapshot to the release runbook, recording the green repository-local M04/M05/M06/M07/M08 gates, compile-only M00 toolchain boundary, and the local source-distributed VST3 SDK installation while retaining native routing, driver, signing, installer, and clean-machine blockers. Documentation validation is required before committing this handoff update.
- Requalified commit `82932c9` after adding `processors.list`: the full locked workspace passed, strict Clippy passed, contracts typecheck passed, documentation validation passed (51 Markdown files and 150 local links), and M07 headless acceptance passed. The release notes' stale 325-test total was corrected to 346. No audio device, driver, signing mode, installer, or machine configuration was accessed.
- Requalified the new head `822975b`: M05 UI acceptance passed TypeScript typecheck, 68 UI tests, and the temporary three-file production build; M08 unsigned artifact preparation and verification passed. Temporary outputs were removed; no installer, driver, signing mode, audio endpoint, or machine configuration was changed.
- Added direct CLI and MCP regression coverage for `processors.list`, confirming both adapters return the seven explicit unavailable DSP descriptors through their typed/read-only surfaces. Focused adapter tests, formatting, and documentation validation pass without audio or machine configuration changes.
- Corrected the current release checklist's stale M05 count from 64 to the currently verified 68 UI tests; historical evidence counts remain unchanged. Documentation validation passes and no runtime or machine state was changed.
- Revalidated the pinned repository-local VST3 SDK with the authorized VS2026 native build: SDK self-tests passed 51/51, the official validator passed 1,598/1,598, and the offline mda loader passed with 68 classes, finite stereo processing, five parameters, automation, and 180-byte state round-trip. Generated build outputs were cleaned; no system plugin registration, audio stream, driver, or machine configuration was changed.
- Requalified the M00 compile-only native WASAPI acceptance at the current head with the installed VS2026/MSVC and Windows SDK/WDK toolchain. `main.cpp` compiled successfully and temporary outputs were cleaned; the probe was not executed and no audio stream, driver, signing mode, or machine configuration was touched.
- Re-ran the repository-local VST3 SDK acceptance and M00 native compile gate at the current head using the authorized VS2026 toolchain. The SDK passed 51 self-tests, 1,598 official validator tests, and offline loader validation; the native WASAPI probe compiled successfully. Temporary outputs were cleaned. The Rust live-stream `E_INVALIDARG` discrepancy remains an unclosed format/request investigation and was not tested by opening user audio.
- Tightened the discovered processor JSON Schemas: `system.describe` and `processors.list` now validate availability status/reason, version/category, nonnegative latency, and typed parameter metadata instead of accepting generic objects. Control tests (79), strict Clippy, formatting, documentation validation, and diff checks pass without audio or machine changes.
- Revalidated the M06 SDK installer provenance acceptance at the current head. The disposable checkout/origin/submodule checks passed and cleanup completed; no installed SDK, plugin, driver, audio endpoint, or machine configuration was changed.
- Requalified M04 and M08 at the current head: all 25 DSP and 30 recording tests passed, including 60-second pitch-duration extremes, and unsigned artifact preparation/verification passed with cleanup. No audio device, driver, installer, signing mode, or machine configuration was accessed.
- Extended the UI's read-only DSP catalog to display typed parameter names, units, and bounded ranges from discovery, with empty-parameter handling and formatter tests. UI typecheck/tests/build and documentation validation pass without processor activation or audio state changes.
- Corrected status and diagnostics capability wording to distinguish implemented portable graph/DSP primitives from the still-unavailable native realtime scheduler and endpoint routing. Control tests, strict Clippy, formatting, and documentation validation are required; no audio or machine state changes.
- Added a control regression locking the corrected native-audio capability reason in `status.get`, preventing future wording from collapsing portable graph support and native endpoint availability. The focused control suite and strict Clippy pass without audio access.
- Added the portable `RealtimeScheduler` ownership boundary in the engine. It owns fixed-shape input/output rings, exposes nonblocking submit/receive operations, and delegates one bounded processing step to the published runtime graph; a regression verifies generation and sample output while preserving pooled-block ownership. Engine tests (42), strict Clippy, formatting, and diff checks pass; native endpoint activation remains outside this boundary.
- Added exact-shape, allocation-free planar/interleaved `f32` block conversion methods to the engine, with round-trip and mismatch regressions. The 43-test engine suite and strict Clippy pass; this is a format bridge for future WASAPI integration and does not open or route a live stream.
- Requalified the checked-in M04 acceptance wrapper at the current head: 25 DSP tests and 30 recording tests passed, including the 60-second pitch-duration extremes. The process-scoped PowerShell execution-policy bypass was not persisted; no audio device or machine configuration was accessed.
- Added the missing `startup.plan` and `startup.apply` contract surfaces. Planning validates the desired policy and returns an explicit unavailable registration plan; apply is idempotency-keyed but remains fail-closed before OS registration. Domain/control/TypeScript schemas and an 80-test control regression pass; native sign-in lifecycle remains open and no startup or machine configuration changed.
- Made startup plans durable in SQLite alongside their enabled flag and expiry. A control-plane restart regression verifies a planned startup policy remains addressable while apply still returns `state: unavailable` before OS registration. Control (81) and strict Clippy pass; the standalone storage test executable was blocked before launch by Windows Application Control error 4551, while storage behavior was exercised through the passing control restart test.
- Added CLI `startup plan --enabled|--disabled` and `startup apply <plan-id>` commands backed by an explicit SQLite path, so plan/apply works across separate CLI processes without touching OS startup registration. CLI/MCP process tests (24), strict Clippy, formatting, and documentation validation pass; registration remains unavailable and no machine configuration changed.
- Added focused MCP `plan_startup` and `apply_startup` tools mapped to the same authorized startup API methods, with session-control permission and explicit unavailable results. MCP regression coverage now validates the plan/apply flow and the 40-tool catalog; CLI/MCP tests, strict Clippy, and docs validation pass without startup or machine changes.
- Corrected the MCP stdio process acceptance fixture to expect the expanded 40-tool catalog. Both MCP interoperability tests now pass at the current head; no startup, audio, or machine configuration was accessed.
- Requalified M05 at the current head: TypeScript typecheck, all 69 UI tests, and the disposable three-file Vite production build passed. Temporary output was cleaned; manual visual/accessibility, native shell injection, startup registration, and live audio remain open.
- Requalified the checked-in M07 headless acceptance at the current head: M01 CLI (24 tests), MCP stdio/named-pipe interoperability (2), control (81), plugin-host (31), worker-process (8), and strict Clippy passed. Temporary test state only; no audio device, driver, or machine configuration was accessed.
- Requalified M08 unsigned release preparation at the current head: optimized locked artifacts, complete SBOM, third-party notices, sanitized provenance, checksums, manifest verification, and blocker assertions passed in a disposable directory. Temporary output was removed; signing, driver, installer, clean-machine, and native audio gates remain open.
- Requalified M06 at the current head with the authorized VS2026 toolchain: the pinned repository-local VST3 SDK built, 51 SDK self-tests and 1,598 official validator tests passed, and the offline loader verified 68 classes, finite processing, five-parameter automation, and a 180-byte state round-trip. Generated outputs were cleaned; no system plugin registration, audio stream, driver, or machine configuration was changed.
- Requalified the M01 CLI acceptance after the startup plan/apply additions; schema, status, discovery, and safe command behavior passed. The acceptance used temporary state only and did not access audio devices or machine configuration.
- Requalified the full locked Rust workspace at the current head: 352 unit/integration targets passed across all packages (24 CLI, 2 MCP, 81 control, 35 domain, 25 DSP, 43 engine, 31 plugin-host, 8 worker-process, 5 protocol, 30 recording, 38 storage, 17 transport, and 13 Windows-audio), with temporary test state only. Strict Clippy, formatting, and diff checks remain green; no audio endpoint, driver, or machine configuration was accessed.
- Requalified the full locked Rust workspace after the scheduler boundary: 346 unit/integration targets passed and workspace strict Clippy with `-D warnings` passed. The run used temporary test state only; no audio endpoint, driver, signing mode, installer, or machine configuration was accessed.
- Clarified the troubleshooting and release-note wording for the unresolved Rust WASAPI `E_INVALIDARG`: native C++ initializes the same endpoints, and the HRESULT is distinct from `AUDCLNT_E_DEVICE_IN_USE`; the Rust COM/ABI path remains blocked without claiming routing or changing device settings. Documentation validation passes.
- Requalified M05 at the current head after the startup API expansion: TypeScript typecheck, all 69 UI tests, and the disposable three-file Vite production build passed. Temporary output was cleaned; no audio device, driver, startup registration, or machine configuration was accessed. Manual visual/accessibility, native shell injection, startup registration, and live audio remain open.
- Added the portable M03 global graph-validation boundary. `VirtualBusRoute` models known render-to-bus and bus-to-capture session links; validation rejects missing sessions, conflicting writers, duplicate routes, and known cross-session cycles while allowing one writer to fan out to multiple consumers. Domain tests (38) and strict domain Clippy pass. Native endpoint identity, driver provisioning, and live routing remain open.
- Requalified M06 at the current head with the installed VS2026 toolchain and pinned local SDK: SDK build, 51 self-tests, 1,598 official validator tests, and the offline loader (68 classes, finite processing, five parameters/automation, 180-byte state) passed. Generated loader outputs were removed; no system plugin registration, audio stream, driver, or machine configuration was changed.
- Added CLI `virtual-devices plan` and `virtual-devices apply` commands with explicit operation-file/database paths and `deviceAdministration` authorization. A focused durable process-style regression passes (25 CLI tests), including honest `state: applied` plus unavailable endpoint capability. Driver provisioning and native endpoint creation remain open.
- Added `maxVirtualBuses` to the typed `system.describe` limits contract, sourced from the domain's declared capacity of eight and covered by control discovery regression. Control tests (81), strict Clippy, and documentation validation pass; the native driver still remains unavailable.
- Hardened global virtual-bus validation to enforce aggregate node/edge budgets and non-empty bus IDs in addition to cross-session cycle/writer checks. Domain tests (39) and strict domain Clippy pass; native endpoints and driver lifecycle remain open.
- Added control-plane capacity acceptance for all eight virtual buses and rejection of a ninth create plan. Control tests now pass 82 with strict Clippy; this remains desired-state validation and does not provision native endpoints.
- Requalified the complete M07 headless acceptance after the virtual-device CLI/discovery additions: M01 CLI (25 tests), MCP interoperability (2), control (82), plugin-host (31), worker-process (8), and strict Clippy passed. Temporary state only; no audio device, driver, or machine configuration was accessed.
- Added the portable `VirtualBusBridge` engine boundary with separate bounded render/capture rings, initialized silence, generation-safe ownership, queue clearing on deactivation, and explicit overflow drops. Engine tests now pass 45 with strict Clippy; native driver bridge and live endpoint routing remain open.
- Extended `VirtualBusBridge` with explicit one-to-many fan-out into caller-owned bounded destination rings. Each destination receives an independent copy when capacity exists; slow destinations are skipped without blocking others. Engine tests now pass 46 with strict Clippy; native driver integration remains open.
- Added allocation-free `VirtualBusBridge::receive_capture_into`, which fills a caller-owned block and explicitly returns silence on underrun or inactive state. The 46-test engine suite and strict Clippy pass; native driver integration remains open.
- Hardened `VirtualBusBridge` to sanitize non-finite samples to silence before single-output capture or one-to-many fan-out delivery. Engine tests remain green at 46 with strict Clippy; native driver integration remains open.
- Added a final ownership-generation check before bridge publication, so deactivation/replacement observed during processing drops the in-flight block instead of publishing it. Engine tests (46) and strict Clippy pass; the bridge remains portable and native integration is open.
- Made bridge activation generation advancement atomic with compare-exchange and verified stale activation does not deactivate a live generation. Engine tests (46) and strict Clippy pass; native driver ownership remains open.
- Added concurrent activation coverage proving one winner for a contested ownership generation and rejection of all stale contenders. Engine tests now pass 47 with strict Clippy; native driver ownership remains open.
- Added a bridge regression proving a capture shape mismatch returns an explicit error and consumes the queued block rather than retaining stale audio. Engine tests remain at 46 with strict Clippy; native integration remains open.
- Added CLI help/schema regressions for the virtual-device plan/apply commands and the discovered eight-bus limit. The 25-test CLI suite and strict Clippy pass; no audio or machine configuration was accessed.
- Re-ran the project-local VST3 SDK installer and complete M06 acceptance with the installed VS2026 toolchain. The pinned checkout and all seven submodules were verified; 51 SDK self-tests, 1,598 official validator tests, and offline loader checks passed. No global SDK/plugin registration, driver, audio stream, or machine audio configuration was changed.
- Serialized concurrent virtual-bus activation and deactivation around their control-plane drain/reactivate sequences, preventing stale contenders or shutdown from clearing/restarting ownership out of order. Engine tests now pass 48 with strict Clippy; native driver synchronization remains open.
- Requalified the full locked Rust workspace after the bridge ownership fixes: 363 unit/integration targets passed (25 CLI, 2 MCP, 82 control, 39 domain, 25 DSP, 48 engine, 31 plugin-host, 8 worker-process, 5 protocol, 30 recording, 38 storage, 17 transport, and 13 Windows-audio), with strict workspace Clippy, formatting, and diff checks green. Temporary/simulated state only; no audio endpoint, driver, or machine configuration was changed.
- Added a post-submit generation check to the virtual-bus capture bridge, closing the race where activation could drain just before an old-generation publication. Raced stale blocks are dropped and drained before replacement consumption; engine tests remain at 48 with strict Clippy, and native driver synchronization remains open.
- Added generation tags to pooled bridge blocks and a generation-filtered ring receive operation, so stale capture/fan-out blocks can be recycled at the consumer boundary. Engine tests now pass 49 with strict Clippy; native driver synchronization remains open.
- Hardened inactive bridge reads to drain and reject queued capture blocks before generation filtering, preserving fail-silent shutdown under a consumer/deactivation race. Engine tests remain at 49 with strict Clippy; native driver synchronization remains open.
- Requalified the full locked Rust workspace after generation-tagged bridge changes: 364 unit/integration targets passed (25 CLI, 2 MCP, 82 control, 39 domain, 25 DSP, 49 engine, 31 plugin-host, 8 worker-process, 5 protocol, 30 recording, 38 storage, 17 transport, and 13 Windows-audio), with strict workspace Clippy and diff checks green. Temporary/simulated state only; no audio endpoint, driver, or machine configuration was changed.
- Corrected scheduler output ownership tags so processed blocks carry the immutable graph generation that produced them, and added generation-filtered output receive support. Engine tests and strict Clippy cover the boundary; native endpoint scheduling remains open.
- Hardened scheduler output filtering so no-graph silence carries no stale generation and replaced-generation outputs are recycled before exposure. Engine tests now pass 50 with strict Clippy; native endpoint scheduling remains open.
- Corrected the drift controller with bounded integral correction and added an eight-hour-equivalent simulation for both ±100 ppm clock mismatches. Engine tests now pass 51 with strict Clippy; this is deterministic simulation evidence, not hardware drift validation.
- Added explicit drift-controller reset semantics for stream/reconnect boundaries, clearing learned correction without changing the nominal rate ratio. Engine tests now pass 52 with strict Clippy; native device recovery remains open.
- Added concurrent graph-publication coverage proving processing observes only complete generation-specific snapshots, never a torn graph/parameter pair. Engine tests now pass 53 with strict Clippy; native scheduler edits remain open.
- Hardened delay reconfiguration to clear old preallocated ring history before using a new delay length, preventing stale replay across schedule changes. Engine tests now pass 54 with strict Clippy; native scheduler reconfiguration remains open.
- Bounded prepared delay capacity to 48,000 frames (250 ms at 192 kHz), rejecting oversized requests before arithmetic or allocation. Engine tests and strict Clippy remain green; native latency measurement remains open.
- Requalified the complete safe acceptance sweep at the current head: compile-only M00 native build, M01 CLI, M04 DSP/recording, M05 UI, M06 SDK installer and VST3 SDK, M07 headless, M08 unsigned release preparation, and documentation validation all passed. Temporary outputs/state were cleaned; no audio stream, driver, signing, plugin registration, or machine audio configuration was changed.
- Requalified the full locked Rust workspace after the scheduler, drift, and delay-boundary changes: 369 unit/integration targets passed, with strict workspace Clippy, formatting, and diff checks green. Tests used temporary/simulated state only; no audio endpoint, driver, or machine configuration was changed.
- Hardened linear resampling to treat non-finite source samples as silence before interpolation, preventing invalid values at the format bridge. Engine tests now pass 55 with strict Clippy; native stream conversion remains open.
- Downloaded/verified the official Steinberg VST3 SDK through the repository-local installer at pinned revision `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, initialized all seven submodules, and reran M06 acceptance with VS2026: 51 SDK self-tests, 1,598 official validator tests, and offline loader validation (68 classes) passed. No system SDK/plugin registration, driver, audio stream, or machine configuration was changed.
- Requalified the full locked Rust workspace at the current head: 369 unit/integration targets passed (25 CLI, 2 MCP, 82 control, 39 domain, 25 DSP, 55 engine, 31 plugin-host, 8 worker-process, 5 protocol, 30 recording, 38 storage, 17 transport, and 13 Windows-audio), with strict workspace Clippy, formatting, and diff checks green. Tests used temporary/simulated state only; no audio endpoint, driver, or machine configuration was changed.
- Added a checked `EndpointInfo::bytes_per_frame` metadata contract for interleaved packet copies, rejecting zero or non-byte-aligned sample shapes before buffer arithmetic. Windows-audio tests and strict Clippy pass; no endpoint was opened and native stream integration remains open.
- Retried current-head full workspace execution after the Windows-audio test addition: the build completed, but Windows Application Control blocked launching the CLI test binary with OS error 4551 before any test ran. Workspace strict Clippy, formatting, diff checks, and the focused 14-test Windows-audio suite pass; the last fully executed workspace qualification remains 369 tests at the preceding head.
- Propagated the checked endpoint packet stride through `devices.list` as `format.bytesPerFrame`, with schema/TypeScript parity and fail-closed discovery on malformed metadata. No endpoint was opened; focused Windows-audio/control checks remain metadata-only.
- Validated the packet-stride discovery contract across adapters: Windows-audio (14), control (82), contracts TypeScript typecheck, and UI tests (69) all pass. `npm.cmd` was used because the host blocks `npm.ps1`; no execution-policy setting was persisted and no audio stream or machine configuration was changed.
- Closed the session schema compatibility gap: domain validation now rejects unknown session schema versions with a field-path error, and the control JSON Schema declares the supported v1 boundary. Domain/control tests and strict Clippy remain required; no audio or machine state is involved.
- Added explicit per-node `typeVersion` compatibility against the domain registry, with v1 contract/schema/TypeScript parity and path-specific rejection of unknown node versions. Legacy fixtures default to v1 during deserialization; no audio or machine state is involved.
- Corrected stale M01 evidence and tightened the TypeScript `Session` contract to the same supported schema version (`1`) enforced by the domain/control boundary. No runtime or machine state was changed.
- Added hostile JSON-RPC shape coverage for primitive payloads, missing/wrongly typed fields, array-valued parameters, and mixed-validity batches; the parser rejects every case without accepting a partial request. Protocol tests and strict Clippy remain portable-only.
- Hardened plugin inspection root containment to reject non-directory and reparse-point configured roots before canonical authorization, with regression coverage for a symlinked root. Plugin execution and full OS sandboxing remain open; no external plugin was run.
- Replaced Rust debug-formatted graph validation errors with stable readable messages containing field paths, and wired control/storage error responses to the shared formatter. Domain/control/storage checks remain portable-only.
- Extended plugin root validation through the full existing ancestor chain, rejecting configured paths redirected by a parent reparse point as well as reparse roots themselves. Plugin execution remains unavailable and no external plugin was run.
- Requalified the compile-only M00 native WASAPI probe with the installed VS2026/MSVC and Windows SDK/WDK toolchain. `main.cpp` compiled and temporary outputs were cleaned; the probe was not executed and no audio stream, driver, signing mode, or machine configuration was touched.
- Requalified the complete M08 unsigned release-preparation wrapper at the current head: optimized locked artifacts, manifest, notices/SBOM, provenance, checksums, blocker assertions, and verification passed in a disposable temporary directory, which was cleaned afterward. Driver, signing, installer, clean-machine, and native-audio gates remain open.
- Requalified the current M07 headless acceptance: M01 CLI (25 tests), MCP stdio/named-pipe interoperability (2), control (82), plugin-host (33), worker-process (8), and strict Clippy passed. Temporary test state only; no audio device, driver, or machine configuration was accessed.
- Requalified current portable acceptance at this head: M04 passed 25 DSP and 30 recording tests, including 60-second pitch-duration and recovery coverage; M05 passed TypeScript typecheck, 69 UI tests, and a disposable three-file Vite build; M06 passed the pinned local VST3 SDK self-tests (51), official validator (1,598), and offline loader checks (68 classes). Temporary outputs were cleaned; no audio device, driver, plugin registration, startup registration, or machine configuration was changed.
- Revalidated documentation acceptance after the current evidence updates: 51 Markdown files and 151 local links passed the repository validator. This changed documentation only and did not access audio, drivers, plugin registration, startup registration, or machine configuration.
- Added an explicit Rust `SharedCapture::open_polling` compatibility path using the native-qualified non-event shared-mode initialization and bounded packet polling. The event-driven request remains primary and retries only on the exact observed `E_INVALIDARG`; Windows-audio tests (14), strict Clippy, formatting, and diff checks pass. No live stream or machine audio configuration was accessed, so native Rust runtime qualification remains open.
- Made the normal Rust capture open fail-closed except for the specifically observed `E_INVALIDARG`: that exact error now retries once with a fresh native-compatible polling client, while busy-device, permission, and endpoint-loss errors remain surfaced unchanged. Focused Windows-audio tests and strict Clippy pass; no live stream or machine audio configuration was accessed.
- Re-ran the M00 compile-only native WASAPI acceptance after the capture compatibility change: `main.cpp` compiled with the installed VS2026/MSVC and Windows SDK/WDK, and temporary outputs were cleaned. The probe was not executed and no audio stream, driver, signing mode, or machine configuration was touched.
- Rebuilt the full Microsoft SysVAD x64 solution with WIL and normal package/API validation using the 64-bit VS2026 MSBuild host. Validation passed and produced the sample driver/package in a disposable checkout; the earlier failure was the 32-bit host selecting absent x86 validator components. Outputs were removed, and no driver was installed/loaded, test-signing mode enabled, or machine audio configuration changed. AudioRouter-specific adaptation, target-machine, lifecycle, and production-signing gates remain open.
- Added `tests/acceptance/m00-sysvad-build.ps1` to make the successful SysVAD validation reproducible. It requires a disposable temporary checkout, selects amd64 MSBuild, enforces normal package/API validation, verifies the driver/catalog outputs, and cleans generated x64 directories without installing or loading a driver.
- Executed the new SysVAD acceptance wrapper against fresh disposable SysVAD/WIL checkouts: full x64 Release compilation, package/API validation, and driver/catalog assertions passed; generated outputs and the source checkout were removed afterward.
- Documented the verified SysVAD/WIL wrapper procedure in the native toolchain setup guide, including its disposable-checkout, 64-bit MSBuild, validation, cleanup, and no-driver-install boundaries.
- Requalified the full locked Rust workspace at the current head: 378 unit/integration tests passed across CLI/MCP, control, domain, DSP, engine, plugin-host/worker, protocol, recording, storage, transport, and Windows-audio; workspace strict Clippy, formatting, and diff checks also passed. Tests used temporary/simulated state only; no audio stream, driver, or machine configuration was changed.
- Added pure regression coverage proving the capture compatibility retry accepts only `E_INVALIDARG` and does not retry `AUDCLNT_E_DEVICE_IN_USE` or unrelated HRESULTs. Windows-audio coverage is now 15 tests; strict Clippy and formatting pass, with no live stream or machine configuration access.
- Added stable machine-readable `errorCode` diagnostics to plugin scan/inspect results while preserving human-readable errors. Clients can now distinguish unsupported extensions, non-PE files, unsupported architectures, missing files, cancellation, deadlines, root-policy failures, size limits, and I/O failures without parsing Rust debug text. Control/plugin-host tests, contracts typecheck, strict Clippy, and documentation validation pass; no plugin code was loaded or executed.
- Hardened the repository-local VST3 SDK installer to verify all seven recursive submodules are initialized at their recorded commits before reporting readiness. The real installer and disposable provenance acceptance pass through the Git-for-Windows Bash path used by the host; no global SDK/plugin registration or audio configuration changed.
- Added a connected-only UI plugin-scan panel and backend adapter methods for explicit read-only VST3 discovery. Results show identity/compatibility or stable diagnostic codes, while disconnected mode fails closed; UI typecheck, 70 UI tests, contracts typecheck, and the disposable M05 build pass without loading plugin code or changing audio.
- Extended the UI discovery panel with explicit single-plugin inspection through `plugins.inspect`, including stable error-code display and disconnected fail-closed behavior. UI typecheck, 70 UI tests, and the disposable M05 build pass; no plugin code or audio was executed.
- Closed the plugin diagnostic vocabulary in the API: `errorCode` is now a ten-value enum including `null` for success in both the Rust discovery schema and TypeScript contract. Control/plugin-host tests, worker-process coverage, strict Clippy, contracts typecheck, and documentation validation pass.
- Promoted directory-level plugin scan failures to stable application error codes (`invalidRoot`, `tooManyCandidates`, `cancelled`, `deadlineExceeded`, and `io`) with retry/remediation metadata. The control suite now has 83 tests; plugin-host/worker tests, strict Clippy, contracts, formatting, and docs validation pass.
- Synchronized the current release qualification snapshot to the latest counts: 379 locked Rust tests, 70 UI tests, and 83 control tests. Historical evidence entries remain unchanged; documentation validation passes.
- Added UI parity for authenticated `plugins.retry`: the plugin panel now offers an explicit idempotency-key-bound retry for the selected directory, while remaining disconnected-safe and non-executing. UI typecheck, 70 tests, and the disposable production build pass.
- Added UI parity for `plugins.list`: the plugin panel can explicitly load the last backend scan for the entered directory without triggering a filesystem rescan. UI typecheck, 70 tests, and the disposable production build pass; disconnected mode remains fail-closed and no plugin code or audio was accessed.
- Closed the M07 session-inventory parity gap: the connected UI session picker now reads bounded `sessions.list` pages through the typed backend adapter, while disconnected preview sessions remain local fixtures. Paged results are normalized and capped at 500 items; UI typecheck, 71 tests, production build, and documentation validation pass without session mutation or audio/machine access.
- Requalified M07 after the session-inventory parity change: CLI (25), MCP interoperability (2), control (83), plugin-host (33), worker-process (8), M01 CLI acceptance, and strict Clippy all passed. Temporary state only; no audio device, driver, or machine configuration was accessed.
- Requalified the unsigned M08 release-preparation gate at the current head: optimized artifacts, manifest, notices/SBOM, provenance, checksums, and verification passed in a disposable temporary directory. Signing, driver, installer, clean-machine, and native-audio gates remain open; no machine configuration changed.
- Requalified the full locked Rust workspace at the current head: 379 unit/integration tests passed across CLI/MCP, control, domain, DSP, engine, plugin-host/worker, protocol, recording, storage, transport, and Windows-audio, with all doc-tests green. Temporary/simulated state only; no audio endpoint, driver, or machine configuration was accessed.
- Fixed UI session authority ordering: when `sessions.list` and the point-in-time `sessions.get` snapshot contain the same ID, the snapshot now wins; focused merge regressions cover stale inventory and local-created-session precedence. M05 typecheck, 73 UI tests, and the disposable production build pass without session mutation or audio/machine access.
- Hardened connected session inventory failure handling: a failed `sessions.list` now clears the connected inventory and surfaces an unavailable status instead of showing demo sessions as backend resources; disconnected preview fixtures remain intact. M05 typecheck, 73 UI tests, and the disposable production build pass without session or audio/machine mutation.
- Improved M06 plugin discovery workflow: each returned scan candidate now has an explicit path-selection action that feeds `plugins.inspect` without executing code; selection remains separate from inspection and disconnected mode stays disabled. M05 typecheck, 73 UI tests, and the disposable production build pass.
- Added the M07 startup lifecycle UI boundary: connected users can read startup capability, explicitly plan a desired sign-in policy, and apply the backend plan with an idempotency key; unavailable registration remains visible and fail-closed. Corrected the shared TypeScript method union for `startup.plan`/`startup.apply`; contracts typecheck, UI typecheck, 74 tests, and M05 acceptance pass without OS startup changes.
- Closed the processor-catalog API parity gap: the UI now requests `processors.list` directly, reports connected failures explicitly, and keeps the disconnected catalog fail-closed. Contracts typecheck, UI typecheck, 75 UI tests, and M05 acceptance pass without processor activation or audio/machine access.
- Added read-only UI parity for `presets.list`: connected views display authoritative voice-chain and EQ preset metadata with explicit failure handling, while local template loading remains a separate non-mutating draft action. Contracts typecheck, UI typecheck, 76 UI tests, and M05 acceptance pass without preset activation or audio/machine access.
- Requalified M07 after adding the startup UI boundary: CLI (25), MCP interoperability (2), control (83), plugin-host (33), worker-process (8), M01 CLI acceptance, and strict Clippy passed. Temporary state only; no startup registration, audio device, driver, or machine configuration was accessed.
- Requalified current portable/native-safe gates: M04 passed 25 DSP and 30 recording tests, and M00's native WASAPI probe compiled with VS2026/MSVC and the installed Windows SDK/WDK. No audio stream, driver, signing mode, or machine configuration was used.
- Re-ran the repository-local M06 SDK installer provenance acceptance at the current head. The pinned SDK checkout and all seven recursive submodules were verified in place; no global SDK/plugin registration or machine audio configuration changed.
- Re-ran the disposable M00 SysVAD acceptance at the current head using a fresh Microsoft Windows-driver-samples checkout, WIL submodule, and 64-bit VS2026 MSBuild. The full x64 Release solution passed normal package/API validation and the driver/catalog output assertions; the checkout and generated outputs were removed without installing/loading a driver or changing test-signing or machine audio state.
- Requalified the unsigned M08 release-preparation wrapper at the current head: optimized artifacts, manifest, notices/SBOM, provenance, checksums, and verification passed in a disposable directory, which was cleaned afterward. No installer, driver, signing, or machine audio configuration was used.
- Requalified M05 at the current head: TypeScript typecheck, 76 UI tests, and the disposable three-file Vite production build passed. No audio, driver, or machine configuration was changed.
- Requalified M07 at the current head: CLI (25), MCP interoperability (2), control (83), plugin-host (33), worker-process (8), M01 CLI acceptance, and strict Clippy passed. Temporary test state only; no audio device, driver, startup registration, or machine configuration was accessed.
- Requalified M06 at the current head with elevated native build-tool access after the non-elevated MSBuild FileTracker permission failure: the pinned SDK built, 51 SDK self-tests passed, the official validator reported 1,598 passed/0 failed, and the offline loader enumerated 68 classes and processed a finite stereo block with five parameters and 180-byte state. Repository-local artifacts only; no system plugin registration, audio stream, driver, or machine configuration changed.
- Closed a real M04 graph-integration slice for parametric EQ: `parametricEq@1` is now a validated domain node, available in discovery and the UI library, and compiled into a stateful per-channel peaking-EQ stage in the portable engine. The stage is prepared before processing, uses a nonblocking state boundary, fails closed to silence if state is unavailable, and has finite-output regression coverage. Native realtime scheduling and hardware timing remain open.
- Requalified M07 after the parametric-EQ discovery change and corrected one stale CLI assertion that assumed every processor was unavailable. Current head passes CLI (25), MCP interoperability (2), control (83), plugin-host (33), worker-process (8), M01 CLI acceptance, and strict Clippy; no audio device, driver, or machine configuration was accessed.
- Completed the next M04 graph-integration slice by adding validated `compressor@1` nodes and discovery/UI parity, and compiling stateful per-channel compressor stages in the portable engine. Dedicated regression coverage confirms sustained-level reduction and finite output; the existing DSP compressor remains finite and allocation-free. Native realtime scheduling and stereo hardware timing remain open.
- Added validated `gate@1` nodes with threshold/range/attack/release bounds, discovery/UI parity, and stateful per-channel portable engine stages. Native callback scheduling and hardware timing remain open.
- Added validated `limiter@1` nodes with the declared -12..0 dBFS ceiling, discovery/UI parity, and allocation-free per-channel sample-peak limiting in the portable engine. True-peak/lookahead behavior and native realtime scheduling remain open.
- Added validated `delay@1` nodes with bounded 0..1,000 ms configuration, discovery/UI parity, and preallocated per-channel delay-line stages in the portable engine. Native scheduler timing and hardware latency measurement remain open.
- Added validated `graphic-eq@1` nodes with ten bounded band-gain parameters, discovery/UI parity, and prebuilt per-channel fixed-band stages in the portable engine. Native callback timing and hardware response measurements remain open.
- Requalified M04 after the Graphic EQ graph integration: all 25 DSP and 30 recording tests passed, including the 60-second pitch-duration coverage; no audio device or machine configuration was accessed.
- Added a preallocated 128-frame streaming pitch-shifter boundary with caller-owned input/output buffers, finite-safe bypass handling, and state-retention coverage across successive blocks. DSP coverage is now 27 tests with strict Clippy; pitch remains unavailable to graph/API until reset/reconnect semantics and measured realtime quality/latency evidence are complete.
- Requalified the full locked workspace after the streaming pitch boundary: 385 Rust unit/integration tests and all doc-tests passed, workspace strict Clippy and formatting passed, and documentation acceptance reported 51 Markdown files with 151 local links. No audio endpoint, driver, signing mode, or machine configuration was accessed.
- Added validated `pitch@1` graph/API/UI support with bounded semitone/cents parameters and a per-channel preallocated streaming stage that accepts only the declared 128-frame quantum, failing closed on other shapes. Control-plane reset now rebuilds stream state for reconnects; native scheduler timing and measured realtime quality remain open.
- Requalified M01/M07 after pitch exposure: CLI (25), MCP interoperability (2), control (83), plugin-host (33), worker-process (8), and strict adapter checks passed; no audio device, driver, or machine configuration was accessed.
- Extended `routes.inspect` with authoritative `latencySamples` per upstream path. Portable inspection now accounts for the declared 1,024-sample pitch latency and configured delay at the 48 kHz graph baseline; native device/plugin timing remains open.
- Added `latencySamples` to node-type discovery so clients can distinguish estimated processor latency from measured native endpoint timing; `pitch@1` advertises its 1,024-sample algorithmic estimate.
- Requalified M05 at the current pitch/latency-enabled head: TypeScript typecheck, all 76 UI tests, and the disposable three-file Vite production build passed. No audio, driver, or machine configuration was accessed.
- Requalified unsigned M08 release preparation at the current head: optimized locked artifacts, manifest, SBOM/notices, provenance, checksums, and verification passed in a disposable directory. Signing, installer, driver, clean-machine, and native-audio gates remain open.
- Requalified M06 at the current head after the VS2026/WDK toolchain update: the repository-local pinned SDK passed build, 51 SDK self-tests, the official validator (1,598 passed/0 failed), and the offline loader (68 classes, finite stereo processing, five parameters, 180-byte state). No SDK/plugin was globally registered and no audio, driver, or machine configuration changed.
- Requalified the disposable M00 SysVAD build at the current head with VS2026/MSBuild and WDK 10.0.28000.0: the x64 Release solution passed driver/API validation, package signability, and catalog generation. The temporary Microsoft sample checkout was removed; no driver was installed, loaded, deployment-signed, or used to change machine audio state.
- Requalified the safe acceptance sweep at the current head: M00 native compile-only, M04 DSP/recording, M05 UI, M07 headless, M08 unsigned release preparation, and documentation validation all passed. Temporary outputs/state were cleaned; no audio stream, driver installation/loading, deployment signing, plugin registration, or machine configuration changed.
- Added UI parity for route latency: route inspection now renders each authoritative path's estimated latency in samples next to its edge count and channel map. Contracts/typecheck, all 76 UI tests, and the disposable M05 production build passed; measured native endpoint latency remains a separate open gate.
- Requalified the locked Rust workspace after the route-latency UI change: 386 unit/integration tests and all doc-tests passed, and workspace strict Clippy passed with `-D warnings`. No audio endpoint, driver, signing mode, plugin registration, or machine configuration was accessed.
- Centralized UI route-latency formatting with finite/non-negative fail-closed handling and regression coverage. UI typecheck, 77 UI tests, and diff validation passed; no audio, driver, plugin, or machine configuration was accessed.
- Hardened M06 SDK submodule commands against shell parsing of apostrophes in valid destination paths by using Bash positional arguments. Disposable installer provenance tests and the real pinned SDK installer/offline acceptance passed; no SDK, plugin, driver, audio, or machine configuration changed.
- Hardened the plugin worker launch boundary to reject executables reached through reparse-point ancestors, with platform-aware regression coverage. The worker remains non-audio and third-party plugin execution remains explicitly gated.
- Hardened shared-memory region creation/opening to reject reparse-point paths and parents, preventing IPC slot redirection; the plugin-host regression suite remains non-audio and third-party-code-free.
- Hardened shared-memory transport reopening to reject distinct path spellings and filesystem-identity aliases, including hard links, preventing input/output slot aliasing. Portable plugin-host coverage remains non-audio.
- Hardened recording rename validation to reject source or destination paths with reparse-point ancestors, closing a higher-level path-redirection gap while retaining same-directory and non-overwrite rules.
- Applied the same regular non-reparse file validation to recording preview, reveal, and confirmed recycle actions, preserving missing-file responses while preventing redirected files from being inspected or acted on.
- Removed raw storage/OS error debug serialization from the control boundary: filesystem/database failures now use safe categorized messages, while validation errors retain actionable text and corruption retains its stable non-retryable code. Added a regression proving private paths are not exposed.
- Extended reparse-ancestor protection to recovery backup, restore, and session-bundle export destinations and restore sources, preventing redirected persistence files from crossing their approved directory boundary.
- Extended persistence-boundary protection to plugin-state metadata and bundle import/source/staging paths, rejecting reparse ancestors before state is recorded or archive contents are staged.
- Extended reparse-ancestor protection to recovery-backup pruning, so the bounded deletion operation cannot target a redirected directory hierarchy.
- Consolidated the UI graph-view imports after route-latency integration; UI typecheck, 77 UI tests, and diff validation remain green.
- Extended Windows CI to invoke the checked-in documentation, M04 DSP/recording, M05 UI, M07 headless, and M08 unsigned-release acceptance wrappers. The first local M04 run exposed rustfmt drift in recent path/error-hardening edits; those files were formatted, and M04/M07 plus UI/docs acceptance passed. M08 was then intentionally deferred until the tree could be committed cleanly; no audio, driver, signing, plugin registration, or machine configuration was changed.
- Pushed the CI acceptance coverage as commit `993684c`; clean-tree M08 unsigned artifact preparation and verification then passed. The post-commit locked workspace passed 386 tests plus doc-tests, strict all-feature Clippy, formatting/diff checks, and contracts typecheck (via `npm.cmd` on this host). Native audio, driver, signing, plugin registration, and machine configuration remained untouched.
- Hardened the portable `DriftController` against invalid correction limits: limits are now finite, non-negative, and strictly below one million ppm so the adjusted resampling ratio cannot become zero or negative. Added rejection coverage; all 60 engine tests, engine doc-tests, strict engine Clippy, and diff checks pass. Hardware clock and dual-device evidence remain open.
- Added the missing ENG-01 contract drift gate: `contracts` now exposes `check:drift`, which compares the TypeScript `ImplementedMethod` union with the authoritative CLI-discovered catalog and validates protocol major 1. The check reports all 61 methods aligned; TypeScript typecheck, Node syntax validation, and diff checks pass. It is read-only and does not access audio or machine configuration.
- Completed UI processor-parameter parity for the portable built-in catalog: the connected inspector now renders numeric/boolean controls from the authoritative processor descriptors for every supported built-in, and reset restores each node to its declared draft defaults. The change remains draft-only until explicit backend planning/commit; 78 UI tests, TypeScript typecheck, and a disposable production build pass. No audio or machine configuration was accessed.
- Corrected the authoritative Parametric EQ processor catalog to include the engine/UI defaults of 1,000 Hz and Q=1, with control-schema regression assertions. Control tests (84), strict control Clippy, contracts typecheck, and the 61-method drift check pass; no audio or machine configuration was accessed.
- Extended the ENG-01 drift check to compare the 17 TypeScript `NodeKind` values, normalized to wire names, against the authoritative CLI node catalog in addition to the 61 API methods. The expanded check, contracts typecheck, Node syntax validation, documentation validation, and diff checks pass; no audio or machine configuration was accessed.
- Extended the contract drift gate to verify all 7 backend-advertised processors also have a UI library entry. The read-only check now covers 61 methods, 17 node kinds, and 7 processors; contracts typecheck, documentation validation, and syntax/diff checks pass.
- Hardened the processor drift check bidirectionally: duplicate UI entries and UI-only processor kinds now fail, while the built-in non-processor nodes remain explicitly allowed. The catalog check still passes for 7 processors, alongside contracts typecheck, documentation validation, and syntax/diff checks.
- Extended ENG-01 drift validation to require the TypeScript `MethodParams` and `MethodResult` maps to cover exactly the same 61 methods as `ImplementedMethod`. The check passes together with the 17 node-kind and 7 processor catalog comparisons, contracts typecheck, and documentation validation; the native E_INVALIDARG runtime discrepancy remains separately documented.
- Requalified the current head with the complete M07 and M08 safe wrappers: M01 CLI (25), MCP interoperability (2), control (84), plugin-host (35), worker-process (8), strict Clippy, and unsigned release artifact preparation/verification all passed. Temporary state was isolated and removed; no audio stream, driver, signing, plugin registration, startup registration, or machine configuration was changed.
- Added catalog-driven UI parameter validation: inspector edits now reject non-finite, out-of-range numeric values and wrong boolean types before entering the draft, while unknown fields remain subject to backend validation. Added focused validation coverage; 79 UI tests, TypeScript typecheck, and diff checks pass. Changes remain draft-only and no audio or machine configuration was accessed.
- Reinstalled and reverified the repository-local VST3 SDK with the VS2026 toolchain: the pinned checkout and all seven submodules are present, the official validator passed 1,598 tests with zero failures, and the offline loader passed. This source SDK remains local to `third_party/vst3sdk`; no global SDK/plugin registration, driver, audio stream, or machine configuration was changed.
- Updated the M00 native WASAPI probe to use the engine-selected shared-mode buffer duration (`0`) rather than imposing a one-second capture request. The standalone probe compiles cleanly; execution remains intentionally deferred because it would open live streams. This removes one probe variable but does not claim that buffer duration is the confirmed `E_INVALIDARG` cause.
- Extended the M00 probe's compile-only format diagnostics to preserve the extensible channel mask and subformat GUID for capture endpoints, using an explicit unaligned copy for the packed Windows structure. The native probe compiles cleanly; runtime execution and live format evidence remain deferred.
- Performed an authorized bounded native live-capture qualification across all 13 enumerated capture endpoints. Every endpoint initialized, started, delivered packets, stopped, and reset successfully; no `E_INVALIDARG` or `AUDCLNT_E_DEVICE_IN_USE` occurred. Event-driven capture initialization and event registration also returned `S_OK`, and the post-test media-device snapshot matched the pre-test snapshot. This qualifies the native reference path, rules out device ownership as a general cause of the Rust discrepancy, and leaves the Rust live-open boundary and measured realtime latency open.
- Ran the actual Rust WASAPI probe under the same authorization: all 13 capture endpoints reported success for the original, extensible, and float initialization variants after the checked-in event-first, exact-`E_INVALIDARG` polling fallback. This closes the prior Rust live-open discrepancy on this host; the fallback remains fail-closed for device-in-use and unrelated HRESULTs. One separate render endpoint remains correctly classified as `AUDCLNT_E_EXCLUSIVE_MODE_ONLY`. The final media-device check found ten present devices with no non-`OK` state; measured production realtime latency remains open.
- Qualified the native process-loopback data path with an authorized 500 ms read for the current test process, without generating a tone: asynchronous activation, 44.1 kHz PCM initialization, event registration, capture-service access, start, packet reads, stop, and reset succeeded; 50 packets/22,050 frames included nonzero data. The post-test media snapshot remained ten present devices, all `OK`. Deliberate process attribution and physical output latency remain open.
- Qualified controlled process-tree attribution with an authorized 500 ms child-tone run: the child exited successfully, loopback captured 50 packets/22,050 frames with nonzero sample energy and no silent packets, and the stream stopped/reset cleanly. The temporary child was gone afterward and all ten present media devices remained `OK`. Process restart/PID-reuse behavior and physical output latency remain open.
- Qualified a bounded silent shared-render lifecycle on an active endpoint: the 48 kHz extensible stream initialized, acquired its buffer/service, started, submitted 14,400 silent frames, stopped, and reset successfully. The post-test media snapshot remained ten present devices, all `OK`; no audible tone or persistent configuration change occurred. Process restart/PID-reuse behavior and physical output latency remain open.
- Extended the native probe with endpoint timing diagnostics and qualified them live: the capture endpoint reported a 10 ms default/3 ms minimum period and the silent render endpoint a 10 ms default/2 ms minimum period; both returned `GetStreamLatency=0` after start and completed stop/reset. These are API timing baselines, not physical acoustic latency; physical loopback and restart/PID-reuse evidence remain open.
- Requalified the safe acceptance chain at the current tip: M01 CLI (25), M04 DSP (27) and recording (30), M05 UI (79), M07 CLI (25), MCP interoperability (2), control (84), plugin-host (35), worker-process (8), M08 unsigned release preparation/verification, and documentation validation (51 Markdown files/151 local links) all passed. Temporary outputs/state were cleaned; no driver, signing, installer, or machine audio configuration was changed.
- Requalified the disposable Microsoft SysVAD/WIL sample with the installed VS2026/WDK toolchain and 64-bit MSBuild: the full x64 Release solution, normal package/API validation, signability checks, driver output, and catalog output all passed. The checkout and generated outputs were removed; no driver was installed/loaded, test-signing mode or machine audio configuration was changed. AudioRouter-specific adaptation, target-machine lifecycle, uninstall/recovery, and production signing remain open.
- Added a Windows-only process lifecycle regression: a real bounded helper is observed through the process inventory with PID, executable, and creation timestamp, successfully bound, terminated/reaped, and then rejected as a stale binding; a replacement helper identity is also observed. The 16-test Windows-audio suite passes. No PID reuse occurred during this run, so actual PID-reuse behavior remains explicitly unclaimed.
- Added an opt-in production Rust adapter smoke mode to the standalone WASAPI probe and qualified it live: `SharedCapture` read 51 packets/24,480 frames into caller-owned storage while `SharedRender` submitted 25,536 silent frames for 500 ms; both streams stopped/reset successfully. The final media snapshot remained ten present devices, all `OK`. This advances M02 adapter data-path evidence without claiming graph scheduling, audible routing, or physical latency.
- Added `tests/acceptance/m02-rust-adapter-live.ps1`, an explicit `-AllowLiveAudio` acceptance wrapper that snapshots present media-device identity/state, runs the bounded production Rust adapter smoke, requires positive capture/render frame counts, and fails if the snapshot changes. It is intentionally opt-in and excluded from ordinary CI; it does not alter defaults, volume, mute, privacy, drivers, or startup configuration.
- Executed the guarded M02 live wrapper for 200 ms: it passed with 21 capture packets/10,080 frames/80,640 bytes and 11,136 silent render frames, while the media-device identity/state snapshot remained identical. The wrapper's live switch remains required and no persistent audio configuration was changed.
- Tightened the production adapter smoke to exercise `SharedRender::submit_bytes` with a preallocated all-zero caller buffer and reran the guarded 200 ms acceptance: 21 capture packets/10,080 frames/80,640 bytes and 10,656 zero-valued render frames passed with an identical media-device snapshot. No audible signal or persistent audio configuration change occurred.
- Extended the opt-in smoke through the portable `RealtimeScheduler`: a guarded 200 ms run converted and processed 8,064 captured samples as fixed 128-frame blocks, while `SharedRender::submit_bytes` submitted 11,136 zero-valued frames. Both clients stopped/reset successfully and the media-device snapshot remained identical. This advances adapter-to-engine ownership evidence without claiming complete graph routing or physical latency.
- Tightened adapter-to-engine integration to carry partial WASAPI packets across boundaries in a fixed staging buffer. The latest guarded 300 ms run consumed 14,400 capture frames, processed 14,336 complete 128-frame scheduler frames, retained a bounded 64-frame shutdown remainder, and submitted 15,936 zero-valued render frames. Counts vary with scheduling during the bounded window; this validates packet-to-quantum adaptation without changing machine audio configuration.
- Published a prepared generation-1 graph with a 0.5x gain stage in the adapter-to-engine smoke. The guarded 300 ms run passed finite-output and generation checks with 32 capture packets/15,360 frames, 120 graph blocks/15,360 processed frames, no pending remainder, and 16,032 zero-valued render frames; the media-device identity/state snapshot remained identical. This qualifies graph activation at the adapter boundary, while audible routing and physical latency remain open.
- Extended the native `tone` probe with an explicit render endpoint index and tested the physical-loopback prerequisites read-only. The Focusrite analogue capture endpoint (local capture index 10) delivered 10 packets/4,800 frames and stopped/reset cleanly; the corresponding Focusrite render endpoint (local render index 5) correctly returned `AUDCLNT_E_DEVICE_IN_USE` (`0x8889000A`), so no tone was emitted and no latency claim was made. The media inventory stayed unchanged; an available output route is still required for the 1,000-impulse physical gate.
- Attributed the occupied Focusrite render endpoint with the new read-only `render-ownership 5` diagnostic: active PID 35536 is `voicemeeterpro.exe` (`C:\Program Files (x86)\VB\Voicemeeter\voicemeeterpro.exe`), alongside the system audio service. This confirms the `AUDCLNT_E_DEVICE_IN_USE` cause without terminating the process or changing Voicemeeter/system audio settings; an available output route remains required for physical latency testing.
- Hardened the ownership diagnostic to report each session's executable image path directly through `QueryFullProcessImageNameW`; the Focusrite attribution now reproduces the Voicemeeter path without a separate process-inspection command. The diagnostic remains metadata-only.
- Added the opt-in `m00-native-live.ps1` wrapper to reproduce the native bounded capture/render lifecycle qualification. It discovers directional endpoint counts, validates capture start/stop/reset, accepts only the known render ownership HRESULT, compares media identity/state before and after, and removes generated outputs; it is not part of ordinary CI.
- Requalified the current head with the safe wrappers after the native live additions: M01 CLI, M04 DSP/recording (27 DSP and 30 recording tests), M05 UI (79 tests, typecheck, disposable production build), M07 headless (25 CLI, 2 MCP, 84 control, 35 plugin-host, 8 worker-process tests plus strict Clippy), M08 unsigned release preparation/verification, and documentation validation (51 Markdown files/151 local links) all passed. Temporary outputs were cleaned; no persistent audio, driver, signing, installer, startup, or machine configuration changed.
- Reran the opt-in native live wrapper at the current head: 13 capture endpoints and 21 render endpoints were discovered, one render endpoint was classified as occupied, all bounded lifecycle checks passed, and the media snapshot was unchanged. Runtime endpoint counts are intentionally discovered rather than hard-coded; no persistent audio configuration changed.
- Made Windows application audio-session inventory deterministic by sorting process records and normalizing display-name order/deduplication before exposing the read-only UI/API snapshot. The Windows-audio suite now passes 17 tests with strict crate Clippy; session inventory remains observational and does not open streams or change machine state.
- Added read-only `renderSessionCount` to application audio-session inventory across the Rust adapter, control schema/response, TypeScript contract, CLI assertions, and UI application rows. The focused Rust suites, contracts drift check (61 methods/17 node kinds/7 processors), UI typecheck, and 79 UI tests pass; no session or machine setting is changed by inventory refresh.
- Exercised the production read-only `apps list --json` path on the current host: `voicemeeterpro.exe` was reported with 3 active audio sessions, including 1 render and 2 capture sessions. This confirms endpoint-ownership context reaches the CLI/API contract; the command opened no stream and changed no machine state.
- Requalified the full locked workspace after the audio-session inventory hardening: 392 unit/integration tests and all doc-tests passed, including the 17-test Windows-audio suite. No driver, signing, installer, startup, or persistent audio configuration action occurred.
- Re-ran strict all-feature workspace Clippy at the current head with `-D warnings`; every workspace target passed after the audio-session inventory change. No runtime audio, driver, signing, installer, startup, or machine configuration action occurred.
- Requalified the locked Rust workspace after the adapter and process-lifecycle changes: 392 unit/integration tests and all doc-tests passed, and strict workspace Clippy passed with `-D warnings`. This remains portable/adapter evidence; native graph scheduling, physical latency, driver, signing, and installer gates remain open.
- Improved the native probe's capture evidence by polling packet availability throughout the capture window and reporting silent packets/nonzero payload bytes; added read-only friendly-name endpoint inventory. Authorized digital loopback through the existing VB-Audio Virtual Cable passed: `CABLE Input` render wrote a 1,500 ms tone and submitted 76,800 frames while `CABLE Output` capture observed 99 packets/47,520 frames, zero silent packets, and 200,532 nonzero payload bytes. The media identity/state snapshot was unchanged and all temporary outputs were removed. This qualifies digital render-to-capture propagation, not physical acoustic latency.
- Added the opt-in `m00-native-loopback.ps1` wrapper for reproducible digital loopback acceptance. It resolves the VB-Audio endpoints by friendly name at runtime, runs bounded concurrent tone/capture streams, requires successful lifecycle plus nonzero capture payload, compares the media snapshot, and removes generated outputs. It is excluded from ordinary CI and cannot run without `-AllowLiveAudio`; physical acoustic latency, managed-driver integration/signing, and manual UI/shell acceptance remain open.
- Performed a bounded authorized USB signal-path smoke on the available `PD200X Speakers` render and `PD200X Microphone` capture endpoints after a read-only ownership check. Render wrote 37,920 tone frames; capture observed 100 packets/48,000 mono frames with 73,521 nonzero payload bytes; all lifecycle calls succeeded and the media snapshot was unchanged. This confirms a physical-path candidate only; ambient signal was not separated and no impulse timing/p95 latency was measured, so the 1,000-impulse acoustic gate remains open.
- Requalified the complete safe acceptance chain at the current head: M01 CLI, M04 DSP/recording (27/30 tests), M05 UI (79 tests, typecheck, disposable production build), M07 headless, and M08 unsigned release preparation/verification all passed. Temporary outputs/state were cleaned; physical latency, managed-driver integration/signing, installer, native shell injection, and manual UI gates remain open.
- Generalized `m00-native-loopback.ps1` to accept friendly-name endpoint pairs while retaining its explicit live-audio guard and snapshot/cleanup protections. Replayed both the VB-Audio digital cable and the available PD200X USB speaker/microphone pair successfully; the latter captured 73,552 nonzero payload bytes over 1,000 ms. This improves repeatability of signal-path smoke testing but does not claim the required 1,000-impulse acoustic latency distribution.
- Closed the UI host-injection implementation gap: `main.tsx` now accepts a preloaded `window.__AUDIO_ROUTER_HOST__` bridge containing the typed framed transport and session ID, validates the shape, selects the live backend only when valid, and otherwise retains the disconnected read-only preview. Added malformed-injection fail-closed coverage. Native shell wiring/production host ownership and manual visual acceptance remain runtime gates.
- Generalized the guarded native signal-path wrapper with explicit friendly-name render/capture parameters. The wrapper passed again for both the VB-Audio cable and PD200X USB pair, including bounded lifecycle, nonzero-payload, media-snapshot, and cleanup checks; the physical impulse-latency gate remains separate and open.
- Corrected the Windows-audio crate contract documentation to distinguish read-only discovery from the explicit bounded `SharedCapture`/`SharedRender` stream clients now covered by live smoke evidence. The focused 17-test Windows-audio suite, doc-tests, strict Clippy, formatting, and documentation validation pass.
- Added native impulse-train and raw capture-file diagnostics plus the opt-in `m00-native-impulse.ps1` analyzer. The harness correlates deterministic 10 ms impulses, reports p95 inter-impulse spacing error and an onset estimate, snapshots media-device identity/state, and cleans temporary files; its estimate is explicitly not promoted to the calibrated acoustic latency gate.
- Added a bounded `WebView2RpcTransport` for the UI host boundary. It posts typed JSON-RPC request envelopes, correlates only matching response IDs, ignores unrelated page messages, caps pending requests, times out stale requests, and rejects sends after disposal. UI coverage now passes 83 tests with typecheck and production build; native host ownership and WebView2 runtime/manual acceptance remain open.
- Wired `main.tsx` to automatically select the bounded WebView2 transport when `chrome.webview` and a nonempty `__AUDIO_ROUTER_SESSION_ID__` are supplied, while prioritizing the explicit host bridge and retaining the disconnected fallback. UI coverage now passes 84 tests; native host origin/permission ownership and manual visual acceptance remain runtime gates.
- Ran the new impulse analyzer on the existing VB-Audio cable: 996/1,000 groups were detected with 0-frame p95 spacing error and a 76.92 ms estimated onset. The same 1,000-impulse run on the PD200X pair detected 0 groups; a follow-up threshold diagnostic captured 95,520 frames but only a 2.15e-6 maximum absolute sample and no samples above 0.001. The physical return path is therefore not measurable in the current setup, and the calibrated acoustic latency gate remains failed/unqualified rather than threshold-adjusted.
- Hardened the bounded WebView2 RPC transport to accept only unambiguous JSON-RPC success/error responses with finite numeric error codes and string messages; malformed host messages cannot resolve UI requests. M05 acceptance passed with 85 UI tests, typecheck, and a disposable production build. M07 headless, M08 unsigned artifact preparation, documentation validation, and diff checks also passed; no audio, driver, signing, plugin registration, or machine configuration changed.
- Added runtime validation for outbound WebView2 JSON-RPC requests: only version 2.0 requests with bounded method names and finite safe IDs are posted to the native bridge. Invalid requests fail locally; M05 acceptance passed with 86 UI tests, typecheck, and a disposable production build. No audio, driver, signing, plugin registration, or machine configuration changed.
- Extended M08 release preparation to build the locked UI into an isolated temporary directory and package `audiorouter-ui.zip` alongside the CLI and plugin worker, with the same manifest hash/size and exact-content verification. The temporary UI output is cleaned on all paths; M08 acceptance passed with unsigned driver/signing, installer, and clean-machine blockers retained. No UI was installed and no audio or machine configuration changed.
- Strengthened M08 acceptance to require exactly one `audiorouter-ui.zip` manifest artifact and verify its `index.html` entry through the archive API. The release wrapper, path-safety/verifier regressions, and documentation checks pass with no installation or machine configuration changes.
- Corrected the M08 archive assertion to explicitly load the framework compression assembly on PowerShell hosts where `ZipFile` is not preloaded; the clean-tree acceptance now exercises the UI archive check portably.
- Re-ran M08 from the clean pushed head after the archive portability fix: the UI bundle built, `audiorouter-ui.zip` contained `index.html`, hashes and exact-content checks passed, and all temporary output was removed. Unsigned driver/signing, installer, and clean-machine blockers remain unchanged.
- Extended M08 provenance with the validated UI `package-lock.json`, copied as `sbom.npm.package-lock.json` and included in manifest hashing/exact-content checks. npm's generated SBOM command is blocked by the local contracts dependency tree, so this is documented honestly as lockfile provenance rather than a generated npm SBOM.
- Added a repository-local deterministic CycloneDX generator from the pinned UI lockfile, producing `sbom.npm.json` alongside the source `sbom.npm.package-lock.json`; M08 now carries machine-independent npm dependency provenance without depending on the incomplete installed local dependency tree.
- Strengthened M08 acceptance to structurally validate the generated CycloneDX 1.5 npm SBOM and require both npm provenance artifacts in the manifest, beyond generic checksum verification.
- Replaced PowerShell lockfile parsing with Node JSON validation after the clean M08 run exposed a parser incompatibility with valid npm package keys; the release remains validated by the same Node toolchain used to build the UI.
- Added the opt-in `adapter-route` native Rust smoke: compatible explicitly selected capture/render endpoints now carry captured frames through the generation-1 gain graph into the production `SharedRender` client. The guarded VB-Audio run passed with 24,480 captured, 24,448 scheduled, and 24,448 routed frames, and an unchanged media snapshot. The checked-in wrapper is live-audio guarded and cleans temporary native artifacts; physical latency, arbitrary format conversion, native lifecycle recovery, drivers, signing, and machine configuration remain untouched.
- Added a Windows CI compile check for the standalone native Rust WASAPI probe, including the adapter-route path; live stream execution remains explicitly opt-in and outside CI.
- Hardened adapter-route ownership so processed scheduler blocks are recycled even when render serialization or submission fails. The standalone probe compile check and guarded VB-Audio route acceptance passed again with 24,480 captured, 24,448 scheduled, and 24,448 routed frames; no machine audio configuration changed.
- Replaced adapter-route's single-block render tail with a bounded 64-block carry queue, preserving partial `SharedRender::submit_bytes` results and failing closed on queue exhaustion instead of silently dropping processed audio. The route remains allocation-bounded and caller-owned; a 1-second stress run correctly exposed the prior bound without changing machine audio configuration.
- Corrected adapter-route endpoint selection to pass opaque IDs from the friendly-name inventory into Rust, eliminating dependence on cross-enumerator ordering. The live wrapper and compile checks retain explicit endpoint selection and snapshot/cleanup safeguards.
- Added two standalone-probe regressions proving adapter-route selection honors opaque endpoint IDs and directional filtering; probe tests, strict Clippy, and locked compile checks pass.
- Added bounded mono/stereo channel conversion and fixed-quantum linear sample-rate conversion to adapter-route, using preallocated graph blocks; cross-block clock-drift correction and hardware synchronization remain separate open gates. Three mapping tests, strict Clippy/compile checks, and a guarded VB-Audio regression passed; no machine configuration changed.
- Clarified the adapter-route resampling boundary: differing endpoint rates use the existing preallocated linear resampler at the fixed 128-frame graph quantum, while cross-block fractional phase and FIFO-driven `DriftController` feedback remain native scheduler work.
- Requalified clean-tree M08 after the Node lockfile-validation fix: CLI/worker/UI artifacts, npm lockfile provenance, archive contents, hashes, exact-content verification, unsigned blockers, and cleanup all passed.
- Requalified the complete safe acceptance chain after the SDK verification: M01 CLI, M04 DSP/recording (27/30 tests), M05 UI (86 tests, typecheck, disposable production build), M07 headless (25 CLI, 2 MCP, 84 control, 35 plugin-host, 8 worker-process tests plus strict Clippy), M08 unsigned release preparation/verification, and documentation validation (51 Markdown files/151 local links) all passed. Temporary state was cleaned; no driver, plugin registration, installer, signing, audio, or machine configuration action occurred.
- Reran the authorized bounded VB-Audio digital impulse loopback at the current head: 997/1,000 impulse groups were detected, p95 spacing error was 0 frames, and estimated digital onset was 63.73 ms. Media identity/state and cleanup checks passed. This strengthens digital signal-correlation evidence only; calibrated physical acoustic latency remains unqualified and no machine audio configuration changed.
- Requalified the locked Rust workspace at the current head: 393 unit/integration tests and all doc-tests passed, followed by strict all-target/all-feature Clippy with `-D warnings`. This is portable/native-adapter qualification only; no driver, signing, installer, plugin registration, audio stream configuration, or machine audio setting changed.
- Reran the authorized bounded native WASAPI lifecycle acceptance at the current head: 13 capture endpoints and 21 render endpoints were discovered, one render endpoint was classified as occupied, and the 100 ms capture/silent-render lifecycle passed. Media inventory and cleanup checks passed; defaults, volume, mute, privacy, drivers, signing, and startup configuration were unchanged.
- Replayed the authorized 1,000-impulse analyzer on the PD200X speaker/microphone pair; 0/1,000 impulse groups were detected against the 900-group acceptance threshold. The wrapper's media-state and cleanup boundaries remained active, so the physical acoustic-latency gate stays explicitly unqualified rather than being threshold-adjusted.
- Re-downloaded/repaired the pinned project-local VST3 SDK checkout at revision `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` and reran M06 acceptance with the installed VS2026 toolchain. The SDK self-tests (51), official validator suite (1,598), offline loader, and cleanup checks passed; this remains source-local SDK setup with no system plugin registration, driver, audio, or machine configuration change.
- Requalified the disposable Microsoft SysVAD/WIL checkout at the current head with 64-bit VS2026 MSBuild and WDK 10.0.28000.0: the full x64 Release solution, normal package/API validation, signability, driver/APO/INF outputs, and `sysvad.cat` generation passed. The checkout and generated outputs were removed; the local automatic test signature is not production signing, and no driver was installed/loaded or machine audio configuration changed.
- Reran the authorized 1-second Rust adapter-route smoke over the explicitly selected VB-Audio endpoints: 48,960 capture frames, 48,896 fixed-quantum scheduler frames, and 48,384 routed frames passed through the generation-1 graph. Endpoint/media snapshot and cleanup checks passed; no defaults, volume, mute, privacy, drivers, signing, startup, or other machine audio configuration changed.
- Added a read-only `resolve_endpoint_binding` seam that resolves only an exact opaque endpoint ID with the expected direction and returns explicit `Missing`/`DirectionChanged` results otherwise. Its regression, 18-test Windows-audio suite, strict Clippy, and doc-tests pass; native stream re-open/rebind and device recovery remain open.
- Extended endpoint binding resolution to require the persisted mix format as well as opaque ID and direction; changed sample rate, channel count, bit depth, or format tag now returns `FormatChanged` for deliberate renegotiation. The Windows-audio suite passes 19 tests with strict Clippy/doc-tests; native stream recovery remains open.
- Exposed format-aware binding resolution through `EndpointMonitor::resolve_binding`, so control code can evaluate the latest notification-driven snapshot without opening a stream or selecting a substitute endpoint. The 19-test Windows-audio suite and strict Clippy/doc-tests remain green.
- Added `tools/m00-sysvad/qualify.ps1`, a reproducible disposable SysVAD/WIL qualification helper that downloads the reference checkout, invokes the existing 64-bit validation wrapper, and removes the exact temporary checkout by default. The helper is explicitly non-installing and does not enable test signing or change machine audio state.
- Executed the new SysVAD helper end-to-end: repository clone, WIL initialization, x64 package/API validation, signability/output checks, and exact disposable-checkout cleanup all passed with exit code 0. No driver installation/loading, test-signing change, or machine audio configuration action occurred.
- Pinned the disposable SysVAD helper to Microsoft `Windows-driver-samples` commit `197ba2156a60e2b76fcd4820bae594223e91a1e9` and verified its WIL gitlink `3c00e7f1d8cf9930bbb8e5be3ef0df65c84e8928`; the exact-commit fetch/checkout, full helper qualification, and cleanup passed. Reference qualification is now reproducible instead of tracking a moving branch.
- Extended persisted endpoint format identity with the extensible channel mask and subformat GUID, and reject either change during snapshot-only binding resolution. The 20-test Windows-audio suite, strict Clippy, and documentation checks remain green; native stream recovery and deliberate renegotiation remain open.
- Added `EndpointMonitor::refresh_changes` as an explicit forced, read-only resnapshot path for recovery when notification delivery is coalesced or missed. It clears pending notification state, reports deterministic endpoint changes, and never opens a stream or selects a substitute; focused tests, Clippy, formatting, and docs validation pass.
- Requalified the authorized native bounded lifecycle after extensible-format enumeration changes: 13 capture endpoints and 21 render endpoints were discovered, one render endpoint remained occupied, and all capture/silent-render start/stop/reset checks passed. Media state and cleanup checks passed; no defaults, volume, mute, privacy, driver, signing, startup, or other persistent audio configuration changed.
- Added fail-closed `SharedCapture::open_bound` and `SharedRender::open_bound` APIs. They validate the latest monitored endpoint ID, direction, and full mix-format identity before COM activation, returning a boxed structured binding error for stale bindings; focused adapter/probe tests, strict Clippy, formatting, and docs validation pass.
- Added scheduler-owned `activate_session` and `deactivate` lifecycle wrappers, so graph publication and safe silence after deactivation are available at the same bounded queue boundary used by realtime processing. Engine regression coverage, full workspace tests, strict Clippy, and docs validation pass; native device scheduling remains open.
- Added allocation-free `RealtimeScheduler::telemetry`, combining bounded ring overrun/underrun counters, processor repair/xrun counters, processed quanta, and active generation for diagnostics-thread publication. Engine regression coverage and strict Clippy pass; external API publication and native callback scheduling remain open.
- Wired the opt-in Rust adapter-route probe through `EndpointMonitor` and `open_bound` for both streams, and reused `EndpointInfo::bytes_per_frame` for packet sizing. The guarded live route now exercises the same fail-closed binding and frame-stride checks used by recovery callers.
- Added scheduler telemetry assertions to the guarded adapter-route probe: every processed graph block must be accounted for in the active generation, with zero input/output overruns and zero XRuns. The one-second authorized route passed these checks in addition to its frame-count and media-snapshot checks.
- Added an explicit IEEE-float32 format predicate, including extensible subformat identity, and made the adapter route reject 32-bit integer PCM before decoding bytes as `f32`. Format regression coverage, probe tests, strict Clippy, and the guarded live route pass.
- Requalified the safe acceptance chain at the current head: M01 CLI, M04 DSP/recording, M05 UI (86 tests), M07 headless/MCP, M08 unsigned release artifacts, documentation, and the sequential M06 SDK qualification all passed. A concurrent M06 attempt was rejected as an MSBuild file-tracker access conflict and was successfully cleared by the sequential rerun; the exact generated SDK build tree was removed afterward.
- Added `open_refreshed_bound` for capture and render, forcing a read-only endpoint resnapshot immediately before fail-closed binding validation. The adapter smoke/route path now uses this stronger recovery precondition; topology races after validation remain surfaced by WASAPI.
- Added `AudioError::binding_resolution`, a typed accessor for missing, direction-changed, or format-changed bound-open failures, so recovery/control callers need not parse diagnostic text. The Windows-audio regression and strict Clippy checks pass.
- Added explicit `replace_with_refreshed_bound` recovery operations for capture and render. They stop and release the existing client before refreshing metadata and reopening the exact verified binding, with no substitute selection or overlapping clients; compilation, focused tests, and strict Clippy pass.
- Added a ratio-controlled resampling entry point and connected the adapter route's differing-rate path to bounded FIFO occupancy feedback from `DriftController`. The nominal rate conversion remains unchanged when rates match; correction stays within the configured ±100 ppm bound. Cross-block fractional phase, larger input buffering, and hardware clock qualification remain open.
- Added a preallocated `StreamingResampler` with bounded planar FIFO storage, persistent fractional phase, finite-sample repair, explicit short-result underflow/overflow signaling, and reset semantics. The adapter route now uses it for differing capture/render rates, retaining source samples across graph quanta; native callback timing and hardware clock qualification remain open.
- Corrected streaming-resampler underflow handling so an incomplete destination quantum is discarded without consuming FIFO samples or advancing fractional phase. Added a regression for ownership preservation; engine tests (62), strict Clippy, and probe checks remain green.
- Made streaming-resampler source admission transactional as well: an over-capacity source block is rejected with zero frames accepted and no partial FIFO mutation. The overflow regression and focused engine/probe validation pass.
- Requalified the authorized one-second Rust adapter route after the streaming-resampler correction: 48,960 capture frames, 48,896 scheduler frames, and 47,968 routed frames passed through the selected VB-Audio endpoints with unchanged media state and cleanup.
- Wired the Rust adapter route's bounded render carry drain to `SharedRender::wait_for_data`, so event-driven render availability gates submission instead of spinning on opportunistic polling. The probe compiles and passes strict Clippy; a new live rerun was blocked before execution by host Application Control error 4551, so no stream was opened.
- Corrected differing-rate drift tuning to target the midpoint of the 1,024-frame streaming FIFO (512 frames); the prior 8,192-frame target permanently saturated correction. Added centered-target regression coverage; engine tests (63), strict Clippy, and probe checks pass.
- Extended adapter-route diagnostics with bounded `resampler_queued_frames` and `drift_correction_ppm` fields, and fail-closed assertions for FIFO capacity and the configured ±100 ppm correction limit. Probe tests and strict Clippy pass; live execution remains subject to the host Application Control block.
- Requalified M07 at the current head: CLI (25), MCP interoperability (2), control (84), plugin-host (35), worker-process (8), and the headless acceptance wrapper all passed. This was configuration-only; no audio device, driver, startup registration, or machine setting was changed.
- Requalified M08 at the current head: optimized CLI/plugin-worker/UI artifacts, npm lockfile and CycloneDX provenance, archive contents/hashes, unsigned blocker assertions, and disposable cleanup all passed. No installer, driver, signing, or audio configuration action occurred.
- Fixed the M06 acceptance wrapper's clean-checkout defect: it now configures the ignored VST3 build tree with the installed Visual Studio 18 2026 x64 generator and disables SDK plugin-link creation before building. A clean rerun passed 51 SDK self-tests, 1,598 official validator tests, and offline loader checks; the generated build tree was removed afterward.
- Extended the native WASAPI probe with an opt-in event-driven capture-data path that keeps the event registration live through packet reads and cleanup. On the selected endpoint, the authorized 500 ms run completed 50 packets/24,000 frames with successful start/stop/reset; the endpoint was silent, so this is event lifecycle/data-path evidence rather than signal propagation or latency evidence.
- Extended the native WASAPI probe with an opt-in silent event-driven render path. On render endpoint 0, the authorized 500 ms run registered the event, started, submitted 28,320 silent frames, stopped, and reset successfully; no tone, default change, or persistent machine configuration was involved.
- Added `tests/acceptance/m00-native-event-live.ps1`, a guarded repeatable event-path acceptance wrapper. The authorized VB-Audio run resolved endpoints by friendly name, captured 24,480 frames and submitted 28,800 silent render frames over 500 ms, verified both event lifecycles and unchanged media inventory, and removed temporary outputs.
- Requalified controlled process-tree attribution from the current native probe: a disposable child tone exited cleanly, the include-tree loopback captured 49 packets/21,609 frames with nonzero payload and zero silent packets, and capture stopped/reset successfully. Temporary artifacts were removed; this remains process-loopback evidence, not physical latency or production route supervision.
- Requalified the native process-loopback exclusion mode against the current test process: asynchronous activation, event registration, capture start, 50 packets/22,050 frames with nonzero payload, and stop/reset all passed. Temporary artifacts were removed; this does not claim arbitrary multi-process exclusion or PID-reuse behavior.
- Added `tests/acceptance/m00-native-process-live.ps1`, a guarded repeatable controlled-attribution wrapper. Its authorized 500 ms run verified disposable child exit, include-tree activation/lifecycle, 22,050 captured frames with 78,114 nonzero bytes, unchanged media inventory, and exact temporary cleanup.
- Requalified the portable release chain after the clean M06 acceptance fix: M01 CLI, M04 DSP/recording (27 DSP and 30 recording tests), M05 UI (86 tests, typecheck, temporary production build), M07 headless, and M08 unsigned artifact preparation/verification all passed. Temporary outputs/state were cleaned; no driver, plugin registration, installer, signing, startup, audio, or machine configuration changed.
- Replayed the authorized Rust adapter route after the event-gated render and bounded-resampler changes: the existing VB-Audio endpoints delivered 24,000 capture frames, 23,936 scheduler frames, and 23,456 routed frames. Endpoint/media identity and state were unchanged, and the temporary executable was removed. This advances existing-rate live route evidence; differing-rate hardware, invalidation recovery, and physical latency remain open.
- Added a separate read-only native `inventory-formats` diagnostic and used it to inspect all 21 render and 13 capture endpoints. The host exposes a real 48 kHz/96 kHz pair (CABLE capture at 48 kHz; Sonar Aux render at 96 kHz), but the guarded differing-rate route retry was blocked before inventory by Windows Application Control when launching its freshly generated temporary executable. No stream opened and no machine configuration changed.
- Hardened the Rust route acceptance build to place its object file beside the per-run temporary executable and clean both exact paths, preventing a prior repository `main.obj` from stranding later runs. The native probe compiled successfully; runtime launch remains subject to the host policy above.
- Added explicit capture/render endpoint-ID parameters to the guarded Rust route acceptance, allowing exact persisted bindings to be tested without launching the temporary native inventory executable. The authorized 96 kHz mono Digital Audio Interface to 48 kHz stereo CABLE route passed through the wrapper with 48,000 capture and 23,936 scheduler/routed frames, zero XRuns/overruns, bounded FIFO occupancy, and unchanged media state.
- Requalified the repository-local SDK installer and pinned disposable SysVAD/WDK helper at the current head. SDK provenance passed; SysVAD's full x64 VS2026/WDK solution, package/API validation, signability, driver/APO/INF outputs, and catalog generation passed, then the checkout and outputs were removed. The local automatic test signature is not production signing; no driver was installed or loaded and no audio configuration changed.
- Added and passed `tests/acceptance/m00-native-format-inventory.ps1`: the read-only native `inventory-formats` command queried all 34 active endpoints, validated successful `GetMixFormat` results, compared unchanged media state, and cleaned its temporary executable/object. The host currently exposes 48 kHz mono/stereo, 96 kHz mono, and 96 kHz eight-channel formats; no stream or machine configuration was changed.
- Bounded injected UI/native session identities to 128 characters for both the direct host bridge and WebView2 session path, with fail-closed regression coverage. UI typecheck and all 86 Vitest tests pass; no native host or machine configuration was accessed.
- Added an opt-in exact-origin allowlist to the WebView2 response boundary; mismatched or missing origins are ignored before JSON-RPC correlation, and the allowlist itself is bounded. UI typecheck and all 87 Vitest tests pass; native shell integration remains open and no machine configuration was accessed.
- Wired the normal WebView2 startup path to pass `window.location.origin` into that allowlist, so the live UI rejects responses from another page origin by default. UI typecheck and all 87 Vitest tests pass; native shell packaging/manual acceptance remains open and no machine configuration was accessed.
- Requalified the complete `safe-all.ps1` chain at commit `e597840`: native compile and 34-endpoint format inventory, pinned SysVAD x64 package/signability validation, M01 CLI, M04 DSP/recording, M05 UI (87 tests), M06 SDK, M07 headless, M08 unsigned release artifacts, and documentation all passed. Temporary outputs were removed; no driver installation/loading, signing-mode change, plugin/startup registration, or machine audio configuration occurred.
- Next M06 task: tighten worker parameter-event validation so each event offset is within the accompanying frame's actual quantum, for both inline and shared-memory messages. Requirement scope: PLUG-02/03 and SEC-07/12 protocol bounds. Verification: focused plugin-host unit tests plus workspace formatting/tests/Clippy; rollback: revert the isolated validation/test/docs change without touching native plugins or machine state.
- Completed the worker parameter-event bound: inline and shared-memory messages now reject offsets at or beyond their actual frame count, while retaining global count/value/finite bounds. The plugin-host suite passes 36 unit tests plus 8 worker-process tests, doc-tests, formatting, strict Clippy, and diff checks. No plugin code, audio device, or machine configuration was accessed.
- Requalified the authorized live M02 paths at the current head: bounded native lifecycle passed across 13 capture/21 render endpoints; VB-Audio digital loopback passed with nonzero capture payload; and the explicit Rust adapter route passed with 48,000 capture frames and 23,936 scheduled/routed frames. Each wrapper verified cleanup and unchanged media state; physical acoustic latency, production driver, signing, and native shell gates remain open.
- Next M06 task: make `encode_worker_message` validate the complete worker message before serialization, so locally generated invalid protocol frames cannot cross the worker boundary. Requirement scope: PLUG-02/03 and SEC-07/12. Verification: plugin-host/workspace tests, formatting, strict Clippy, docs, and diff checks; rollback is isolated to the encoder/tests and touches no plugin or machine state.
- Completed outbound worker-message validation: `encode_worker_message` now applies the same protocol, frame, parameter, latency, and failure-code checks as decoding before serializing. Negative handshake/failure cases now prove invalid messages are rejected at the sender boundary. Plugin-host tests (36 unit, 8 process), doc-tests, formatting, strict Clippy, and diff checks pass; no plugin or machine state was accessed.
- Hardened native extensible-format diagnostics to print the complete subtype GUID instead of only `Data1`; the read-only format acceptance passed again for all 34 endpoints with full GUID, channel-mask, and valid-bit output. No stream or machine configuration was changed.
- Added bounded `open_refreshed_bound_with_retry` recovery APIs for capture and render. Recovery refreshes and revalidates the exact endpoint binding on each attempt, retries only transient device-in-use/invalidation/service failures, caps attempts/backoff, and never selects a substitute or sleeps on the realtime path. The 22-test Windows-audio suite, formatting, and strict Clippy pass.
- Added bounded `replace_with_refreshed_bound_with_retry` APIs for capture and render, composing stop/reset, release, fresh exact-binding validation, and transient retry into one recovery operation. Focused Windows-audio tests, doc-tests, formatting, and strict Clippy pass; native invalidation fault injection remains a separate host/device gate.
- Extracted the shared bounded recovery retry policy and added deterministic coverage for transient retry/success, immediate non-retryable `E_INVALIDARG` failure, and the five-attempt cap. The Windows-audio suite now passes 25 tests with doc-tests, formatting, and strict Clippy.
- Requalified the full locked Rust workspace after the recovery API: 393 unit/integration tests, all doc-tests, strict all-target/all-feature Clippy with `-D warnings`, and diff checks passed. No audio, driver, signing, installer, startup, or machine configuration action occurred.
- Added `tests/acceptance/safe-all.ps1` as a sequential aggregate for the safe plan stages. From clean commit `4c6e617`, the complete chain passed: native compile and format inventory, pinned SysVAD x64 build/package/signability, M01 CLI, M04 DSP/recording, M05 UI, M06 SDK provenance and VST3 SDK self-tests/validator, M07 headless, M08 unsigned release preparation/verification, and documentation. Temporary outputs were removed; no driver installation/loading, signing-mode change, plugin/startup registration, or machine audio configuration occurred.
- Next M04 task: make stereo stateful processor stages fail closed when their right-channel state is unavailable, rather than processing the left channel and leaking a dry right channel. Requirement scope: ARCH-09, SEC-06, DSP-01/02/03/05/07/08. Verification: engine regression plus workspace tests/Clippy/format/docs; rollback is isolated to the portable engine and tests.
- Completed stereo state containment: Parametric EQ, compressor, gate, delay, and Graphic EQ now clear both channels whenever required right-channel state is absent or cannot be acquired. Engine tests pass 64 cases with strict Clippy, and M04 DSP/recording acceptance plus documentation validation pass; native callback scheduling and hardware timing remain open.
- Next M05 task: reject the opaque WebView2 origin value `null` and other invalid origin configuration before enabling the live transport. Requirement scope: SEC-05/12 and UI shell trust boundary. Verification: UI tests/typecheck/build plus documentation and diff checks; rollback is isolated to the host transport and tests.
- Completed WebView2 origin hardening: the allowlist now rejects the opaque `null` origin and invalid bounds, and the startup path falls back to disconnected mode instead of enabling a meaningless origin gate. UI typecheck, 87 tests, M05 acceptance, documentation, and diff checks pass; native shell packaging/manual acceptance remains open.
- Next M06/SEC-12 task: cap worker frame and parameter queue allocation at the existing bounded protocol limits, so oversized constructor capacities cannot trigger unbounded memory reservation. Verification: plugin-host regressions, process tests, workspace Clippy/format/docs; rollback is isolated to queue construction and tests.
- Completed M06/SEC-12 queue allocation hardening: bounded frame and parameter queue constructors clamp capacity before allocation, with `usize::MAX` regression coverage. Plugin-host library tests and strict Clippy pass; worker subprocess tests are blocked by Application Control policy error 4551. No machine or audio configuration changed.
- Next M02/SEC-12 task: reject oversized realtime engine queue capacities before constructing lock-free storage, so public ring/pool constructors cannot reserve unbounded memory. Requirement scope: ARCH-04/09 and SEC-12. Verification: engine boundary regressions, workspace tests/Clippy/format/docs; rollback is isolated to the engine capacity guard and tests.
- Completed M02/SEC-12 queue allocation hardening: engine queues and pools reject capacities above 2,048 before allocation, with zero, over-limit, and `usize::MAX` regressions. Engine tests (65), doc-tests, formatting, and strict Clippy pass; native callback scheduling and hardware timing remain open.
- Next M04/SEC-12 task: reject oversized recording-queue capacities before constructing lock-free storage, so a public recording boundary cannot reserve unbounded memory. Requirement scope: ARCH-04, SEC-09/12, and M04 recording backpressure. Verification: recording regressions, workspace tests/Clippy/format/docs; rollback is isolated to the recording queue guard and tests.
- Completed M04/SEC-12 recording queue allocation hardening: capacities above 2,048 are rejected before allocation, with zero, over-limit, and `usize::MAX` regressions. Recording tests (30), doc-tests, formatting, and strict Clippy pass; native realtime recorder integration remains open.
- Next M06/SEC-09/12 task: bound plugin-state restore reads after opening the file, so a file growth or replacement race cannot make `read_to_end` allocate beyond the 16 MiB state limit. Verification: plugin-host state regressions, workspace tests/Clippy/format/docs; rollback is isolated to the bounded restore reader and tests.
- Completed M06/SEC-09/12 bounded state restore: plugin-state reads are limited to 16 MiB plus one byte after opening, with oversized-file regression coverage. Plugin-host library tests (38), strict Clippy, formatting, diff checks, and documentation acceptance pass; native plugin execution remains open.
- Next M06/SEC-09/12 task: enforce plugin-state size invariants during restore verification as well as construction, because the public asset fields can otherwise bypass the constructor bound before persistence. Verification: plugin-host state regressions, workspace tests/Clippy/format/docs; rollback is isolated to state verification and tests.
- Completed M06/SEC-09/12 state invariant enforcement: restore verification rejects empty and oversized directly constructed assets before persistence, with correctly hashed oversized regression coverage. Plugin-host tests (39), strict Clippy, formatting, diff checks, and documentation acceptance pass; native plugin execution remains open.
- Next M07/SEC-06/12 task: make the authoritative state event log discard meter-prefixed categories before sequence allocation, matching the documented non-replayable meter policy and preventing accidental telemetry retention. Verification: domain event-log regression, workspace tests/Clippy/format/docs; rollback is isolated to event classification and tests.
- Completed M07/SEC-06/12 event-log privacy hardening: `meter` and `meter.*` categories are discarded before sequence allocation, with replay/retention regression coverage. Domain tests (44), doc-tests, formatting, strict Clippy, and documentation acceptance pass; native telemetry scheduling remains open.
- Requalified the full elevated safe acceptance chain at the current head: read-only native inventory (31 endpoints), disposable SysVAD x64 qualification, M01/M04/M05/M06/M07, unsigned M08 artifact verification, and documentation all passed. Temporary outputs were removed; live audio, driver installation, signing-mode changes, plugin/startup registration, and machine audio configuration remained excluded.
- Next M04/SEC-09/12 task: reject oversized caller-owned recording chunks before queue insertion, so a bounded queue cannot contain an individually unbounded sample allocation. Verification: recording queue regressions, workspace tests/Clippy/format/docs; rollback is isolated to chunk admission and counters.
- Completed M04/SEC-09/12 recording chunk admission hardening: chunks above 4,096 interleaved samples are returned before insertion and counted separately from queue overruns. Recording tests (30), doc-tests, formatting, strict Clippy, and documentation acceptance pass; native realtime recorder integration remains open.
- Requalified the latest pushed head through the elevated sequential safe acceptance chain: native read-only format inventory (34 endpoints), disposable SysVAD x64 qualification, M01/M04/M05/M06/M07, unsigned M08 artifacts, and documentation all passed. Temporary outputs were removed; live audio, driver installation, signing-mode changes, plugin/startup registration, and machine audio configuration remained excluded.
- Next M07/SEC-12 task: enforce shared depth and string-size budgets for decoded JSON method parameters before method-specific dispatch, complementing the framed byte bound against hostile nested values. Verification: control-plane regression, workspace tests/Clippy/format/docs; rollback is isolated to the parameter budget walker and tests.
- Completed M07/SEC-12 control value hardening: decoded method parameters now reject nesting deeper than 32 levels and individual strings/object keys over 4,096 bytes before schema dispatch. The focused control suite passes 85 tests with strict Clippy and formatting; workspace and documentation gates are recorded below. Rollback is isolated to the shared validator and regression.
- Next M07/SEC-12 task: cap the total decoded JSON value count in method parameters, complementing the frame, depth, and string bounds against large shallow arrays/objects. Verification: control regression, workspace tests/Clippy/format/docs; rollback is isolated to the shared value-budget counter and tests.
- Completed M07/SEC-12 total value hardening: method parameters now reject more than 8,192 decoded JSON values in addition to the framed, depth, and string/key limits. The control suite passes 85 tests with strict Clippy, formatting, and diff checks; workspace and documentation gates will be recorded with this change.
- Requalified the guarded M02 Rust adapter route at the current head using the existing named VB-Audio endpoints: 24,000 capture frames and 23,936 scheduler/routed frames passed. The read-only native format inventory also passed for all 34 endpoints. Exact endpoint/media cleanup checks passed; defaults, volume, mute, privacy, drivers, signing, startup, and other machine audio configuration were unchanged.
- Requalified `tests/acceptance/safe-all.ps1` at the current head: native compile and 34-endpoint format inventory, disposable SysVAD x64 qualification, M01/M04/M05/M06/M07, unsigned M08 release preparation, and documentation all passed. Temporary outputs/checkouts were removed; driver installation, signing-mode changes, plugin/startup registration, and machine audio configuration remained excluded.
- Requalified the aggregate safe chain again at pushed head `15499db`: native compile and format inventory, pinned disposable SysVAD x64 build/package/API checks, M01/M04/M05/M06/M07, unsigned M08 artifacts, and documentation passed. Temporary outputs/checkouts were removed; no driver installation, signing-mode change, plugin/startup registration, or machine audio configuration occurred.
- Next M04/M07/SEC-09/12 task: bound recorder checkpoint part and pause collections during restore, so public/JSON recovery inputs cannot carry unbounded lifecycle metadata. Verification: recording regressions, workspace tests/Clippy/format/docs; rollback is isolated to checkpoint limits and tests.
- Completed M04/M07/SEC-09/12 checkpoint hardening: recorder restore now rejects more than 4,096 parts or pause intervals before lifecycle validation. Recording tests (30), doc-tests, formatting, and strict Clippy pass; native realtime recorder integration remains open.
- Next M04/M07/SEC-09/12 task: reject oversized recorder checkpoint JSON before deserialization, complementing the bounded lifecycle collections against allocation-heavy recovery inputs. Verification: recording regression, workspace tests/Clippy/format/docs; rollback is isolated to the restore input guard and test.
- Completed M04/M07/SEC-09/12 checkpoint input hardening: JSON recovery documents above 1 MiB are rejected before deserialization, alongside the 4,096-part and 4,096-pause limits. Recording tests (30), doc-tests, formatting, and strict Clippy pass; native realtime recorder integration remains open.
- Next M07/API-01/SEC-12 task: expose the enforced control JSON depth, string/key, and total-value budgets through `system.describe` limits so clients can discover the actual request boundary. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to discovery fields and tests.
- Completed M07/API-01/SEC-12 discovery alignment: `system.describe` now advertises `maxControlValueDepth`, `maxControlStringBytes`, and `maxControlValueCount`, with regression assertions matching enforcement. Control tests (85), full workspace tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Completed M07/API-01/SEC-12 discovery alignment: `system.describe` now advertises the enforced control limits (`maxControlValueDepth`, `maxControlStringBytes`, and `maxControlValueCount`) and the discovery regression asserts their values. Control tests (85), formatting, and strict Clippy pass; workspace/docs gates are recorded with this checkpoint.
- Next M01/M07/API-02/SEC-12 task: bound JSON-RPC method-name bytes during protocol validation, so unknown requests cannot spend the full frame budget on an unbounded method identifier. Verification: protocol regression, workspace tests/Clippy/format/docs; rollback is isolated to the method-name validator and test.
- Completed M01/M07/API-02/SEC-12 method-name hardening: protocol validation now rejects method identifiers over 128 UTF-8 bytes, and `system.describe` advertises `maxMethodNameBytes` alongside the control JSON budgets. Protocol/control tests (6/85), workspace tests, strict Clippy, formatting, diff checks, and docs acceptance pass.
- Completed stereo fail-closed processing: Parametric EQ, compressor, gate, delay, and graphic EQ now clear the entire block when the right-channel state is absent or unavailable, preventing a partially processed stereo path. Engine tests now pass 64 cases with strict engine Clippy, formatting, and diff checks; native scheduling remains open.
- Next M01/M07/API-02/SEC-12 task: bound JSON-RPC request-ID payloads during protocol validation, so correlation identifiers cannot consume the full frame budget as arbitrary nested JSON. Verification: protocol/control regressions, workspace tests/Clippy/format/docs; rollback is isolated to the request-ID validator and discovery field.
- Completed M01/M07/API-02/SEC-12 request-ID hardening: protocol validation now rejects request IDs whose encoded JSON exceeds 128 bytes, and `system.describe` advertises `maxRequestIdBytes`. Protocol/control regressions, workspace tests, strict Clippy, formatting, diff checks, and docs acceptance pass; no audio or machine configuration changed.
- Next M01/M07/SEC-04/SEC-12 task: apply JSON-RPC request validation before authorized method lookup, so malformed known-method requests cannot receive an authorization response carrying unvalidated request metadata. Verification: authorized control regression, workspace tests/Clippy/format/docs; rollback is isolated to dispatch ordering and the regression.
- Completed M01/M07/SEC-04/SEC-12 request validation ordering: authorized dispatch now validates JSON-RPC requests before method lookup, permission checks, and mutation rate limiting, so malformed known-method requests receive the standard `-32600` Invalid Request response. Control tests (86), full workspace tests, strict Clippy, formatting, diff checks, and docs acceptance pass; no audio or machine configuration changed.
- Requalified the aggregate safe acceptance chain at pushed head `4d4c40c`: native compile and read-only 34-endpoint format inventory, disposable SysVAD x64 qualification, M01 CLI, M04 DSP/recording, M05 UI (87 tests and temporary production build), M06 SDK (51 self-tests and validator), M07 headless, M08 unsigned artifacts, and documentation all passed. Temporary outputs/checkouts were removed; driver installation, signing-mode changes, plugin/startup registration, live audio, and machine audio configuration remained excluded.
- Next M01/M07/API-02/SEC-12 task: restrict JSON-RPC request IDs to scalar string/number/null values during protocol validation, so object/array/boolean correlation metadata cannot cross the control boundary. Verification: protocol regression, workspace tests/Clippy/format/docs; rollback is isolated to request-ID type validation and tests.
- Completed M01/M07/API-02/SEC-12 request-ID type hardening: protocol validation accepts only scalar string/number/null identifiers and retains the 128-byte encoded bound; hostile object, array, and boolean IDs are rejected. Protocol/control tests, workspace tests, strict Clippy, formatting, diff checks, and docs acceptance pass; no audio or machine configuration changed.
- Next M01/GRAPH-01/SEC-12 task: bound graph entity-ID byte lengths during domain validation, so oversized session/node/edge/virtual-route identifiers cannot expand graph indexes or error payloads without limit. Verification: domain regression, workspace tests/Clippy/format/docs; rollback is isolated to the domain ID validator and tests.
- Completed M01/GRAPH-01/SEC-12 entity-ID hardening: domain validation now bounds session, node, edge, endpoint-reference, and virtual-route IDs to 128 UTF-8 bytes before graph indexing. Domain tests (45), full workspace tests, strict Clippy, formatting, diff checks, and docs acceptance pass; no audio or machine configuration changed.
- Next M03/SEC-12 task: apply the same 128-byte entity-ID bound at the managed virtual-bus registry insertion boundary, so desired-state bus IDs cannot bypass graph validation and remain unbounded in registry storage. Verification: domain regression, workspace tests/Clippy/format/docs; rollback is isolated to the registry error/check and tests.
- Next M03/SEC-12 task: bound virtual-bus lease-owner identifiers before storing ownership state, so an oversized authenticated owner cannot bypass the domain identity budget through the lease path. Verification: domain regression, workspace tests/Clippy/format/docs; rollback is isolated to lease validation/error mapping and tests.
- Completed M03/SEC-12 lease-owner hardening: virtual-bus lease acquisition rejects owner IDs over 128 UTF-8 bytes before storing ownership, with explicit `OwnerTooLong` mapping through the registry. Domain tests, full workspace tests, strict Clippy, formatting, diff checks, and docs acceptance pass; no audio or machine configuration changed.
- Next M07/API-08/SEC-12 task: bound retained state-event metadata before insertion, so oversized categories, operation IDs, or session IDs cannot expand the event log beyond its count/age limits. Verification: domain event-log regression, workspace tests/Clippy/format/docs; rollback is isolated to event metadata admission and tests.
- Completed M07/API-08/SEC-12 event metadata hardening: retained events now reject category and operation-ID values over 128 bytes and session IDs over the 128-byte entity bound before sequence allocation. Domain event-log tests, full workspace tests, strict Clippy, formatting, diff checks, and docs acceptance pass; no audio or machine configuration changed.
- Next M01/GRAPH-03/SEC-12 task: bound the number of parameters on each graph node during domain validation, so direct session imports cannot bypass control JSON budgets with oversized parameter maps. Verification: domain regression, workspace tests/Clippy/format/docs; rollback is isolated to the node parameter-count guard and test.
- Completed M01/GRAPH-03/SEC-12 node parameter-map hardening: domain validation now rejects more than 32 parameter entries per node before processing the map. Domain tests, full workspace tests, strict Clippy, formatting, diff checks, and docs acceptance pass; no audio or machine configuration changed.
- Next M01/GRAPH-03/SEC-12 task: bound graph node parameter-name bytes during domain validation, so direct session imports cannot expand validation paths or parameter metadata beyond the established 128-byte identity budget. Verification: domain regression, workspace tests/Clippy/format/docs; rollback is isolated to the parameter-name guard and test.
- Completed M01/GRAPH-03/SEC-12 parameter-name hardening: direct session validation rejects parameter names over 128 UTF-8 bytes with bounded index-based diagnostics, without echoing hostile names. Domain tests (48), full workspace tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/GRAPH-01/GRAPH-03/SEC-12 task: bound graph display names, port names, and edge port references before graph lookup/indexing, so direct session imports cannot retain oversized labels or echo them through missing-port errors. Verification: domain regression, workspace tests/Clippy/format/docs; rollback is isolated to graph text guards and tests.
- Completed M01/GRAPH-01/GRAPH-03/SEC-12 graph text hardening: node display names are limited to 256 bytes and port names/references to 128 bytes before lookup/indexing, with bounded diagnostics. Domain tests (49), full workspace tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/GRAPH-01/GRAPH-03/SEC-12 task: bound each node's port-definition collection and reject duplicate port names before edge lookup, so direct session imports cannot expand graph work or create ambiguous port resolution. Verification: domain regressions, workspace tests/Clippy/format/docs; rollback is isolated to the port-count/uniqueness guards and tests.
- Completed M01/GRAPH-01/GRAPH-03/SEC-12 port-definition hardening: node port collections are limited to 16 entries and duplicate names are rejected before edge lookup, with bounded validation traversal. Domain tests (50), full workspace tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/GRAPH-03/SEC-12 task: reject edge coefficient vectors above the four coefficients possible for mono/stereo ports before node lookup, so malformed dangling-edge imports cannot retain oversized matrices outside the graph budget. Verification: domain regression, workspace tests/Clippy/format/docs; rollback is isolated to the matrix-size guard and test.
- Completed M01/GRAPH-03/SEC-12 matrix hardening: edge coefficient vectors are limited to four entries before node/port lookup, including dangling-edge imports. Domain tests (51), full workspace tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/GRAPH-01/SEC-12 task: apply the bounded display-name policy to session names during direct validation, so imported session labels cannot bypass the graph text budget. Verification: domain regression, workspace tests/Clippy/format/docs; rollback is isolated to the session-name guard and test.
- Completed M01/GRAPH-01/SEC-12 session-label hardening: session display names now use the 256-byte UTF-8 bound. Domain tests (52), full workspace tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/API-01/GRAPH-03/SEC-12 task: advertise all enforced graph text, port, and channel-matrix limits through `system.describe` and the TypeScript contract, so clients can discover the actual validation boundary. Verification: discovery regression, contracts typecheck, workspace tests/Clippy/format/docs; rollback is isolated to discovery fields and contract declarations.
- Completed M01/API-01/GRAPH-03/SEC-12 discovery alignment: `system.describe` and the TypeScript contract now advertise display-name, port-name, ports-per-node, and channel-matrix bounds, with values sourced from domain constants. Control tests (86), full workspace tests, strict Clippy, contracts typecheck/drift, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/API-01/GRAPH-01/GRAPH-03/SEC-12 task: align the discovered session JSON schema with enforced ID, text, collection, parameter-map, port, and matrix bounds, so clients get field-level constraints in addition to the shared limits object. Verification: control discovery regressions, workspace tests/Clippy/format/docs; rollback is isolated to schema metadata and assertions.
- Completed M01/API-01/GRAPH-01/GRAPH-03/SEC-12 schema alignment: discovered session schemas now declare the enforced ID/text, node/edge, port, parameter-map, and matrix bounds, with discovery regressions covering representative fields. Control tests, full workspace tests, strict Clippy, contracts typecheck/drift, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/API-01/GRAPH-01/GRAPH-03/SEC-12 task: replace unconstrained session placeholders in create/import input schemas with the shared bounded session schema, so clients receive the same field-level contract before submission. Verification: control discovery regressions, workspace tests/Clippy/format/docs; rollback is isolated to input-schema reuse and assertions.
- Completed M01/API-01/GRAPH-01/GRAPH-03/SEC-12 input-schema alignment: `sessions.create` and `sessions.importPlan` now reuse the bounded session schema instead of advertising unconstrained objects. Control discovery regressions, full workspace tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/API-01/GRAPH-03 task: correct the discovered session port schema from eight channels to the enforced mono/stereo range, so clients cannot plan a schema-valid graph that domain validation must reject. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to the channel maximum and assertion.
- Completed M01/API-01/GRAPH-03 schema correction: discovered session port definitions now cap channels at 2, matching the mono/stereo domain validator. Control discovery regression, full workspace tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/API-01/GRAPH-03/SEC-12 task: expose the bounded node parameter-name policy in the discovered session schema, alongside the existing parameter-count limit, so clients can validate parameter maps before submission. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to schema metadata and assertion.
- Completed M01/API-01/GRAPH-03/SEC-12 parameter-schema alignment: discovered session schemas now advertise both the 32-property parameter-map limit and 128-byte parameter-name limit. Control discovery regression, full workspace tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/API-01/GRAPH-03 task: expose the channel-matrix coefficient range in the discovered session schema, so clients see the enforced `[-2, 2]` bound in addition to the four-entry shape limit. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to matrix item metadata and assertion.
- Completed M01/API-01/GRAPH-03 matrix-schema alignment: discovered matrix items now advertise the enforced `[-2, 2]` coefficient range and four-entry maximum. Control discovery regression, full workspace tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Requalified the authorized M00 process-attribution wrapper at the current head: a disposable child exited cleanly, process-loopback capture completed with 21,609 frames and 76,370 nonzero payload bytes over 500 ms, and the media-device identity/state snapshot was unchanged. Exact temporary executable/object cleanup passed; this remains controlled process-tree evidence and does not claim PID-reuse behavior or physical latency.
- Requalified the authorized M02 Rust adapter route at the current head over the existing VB-Audio endpoints: 24,480 capture frames and 24,448 scheduler/routed frames passed through the bounded route over 500 ms. Endpoint/media identity and state remained unchanged and exact temporary cleanup passed; this advances digital adapter evidence without claiming physical acoustic latency, production-driver integration, or clock qualification.
- Requalified the authorized M00 digital loopback wrapper at the current head: the existing VB-Audio cable carried the bounded test tone to capture with 219,572 nonzero payload bytes during 1,000 ms capture/1,500 ms tone windows. The wrapper verified endpoint/media identity and state, exact temporary cleanup, and unchanged defaults, volume, mute, privacy, drivers, signing, and startup configuration; physical acoustic latency remains separate and unqualified.
- Requalified the aggregate safe acceptance chain at pushed head `0961c07`: native compile and read-only 34-endpoint format inventory, pinned disposable SysVAD x64 build/package/API validation, M01 CLI, M04 DSP/recording, M05 UI (87 tests and disposable production build), M06 SDK, M07 headless, unsigned M08 release preparation, and documentation all passed. Temporary outputs/checkouts were removed; driver installation, signing-mode changes, plugin/startup registration, live audio, and machine audio configuration remained excluded.
- Requalified the authorized M02 Rust adapter route at current head `0ffbc4f`: the existing VB-Audio endpoints delivered 24,480 capture frames, 24,448 scheduler frames, and 24,448 routed frames over 500 ms. Endpoint/media identity and cleanup checks passed; defaults, volume, mute, privacy, drivers, signing, and startup configuration were unchanged. This strengthens digital existing-endpoint evidence without claiming production-driver integration, differing-rate hardware qualification, or physical acoustic latency.
- Attempted the authorized differing-rate M02 route using a read-only inventory-selected 96 kHz render endpoint and 48 kHz capture endpoint. The render endpoint exposes 8 channels, so the adapter's intentional mono/stereo guard rejected it with `InvalidFrameSize` before stream activation; the wrapper cleaned up and media state remained unchanged. A valid differing-rate mono/stereo pair is not currently available, so hardware clock/resampler qualification remains open.
- Requalified the aggregate safe acceptance chain at pushed head `f87ad6e`: native compile and read-only 34-endpoint format inventory, pinned disposable SysVAD x64 build/package/API validation, M01 CLI, M04 DSP/recording (27 DSP and 30 recording tests), M05 UI (87 tests and disposable production build), M06 SDK (51 self-tests and validator), M07 headless, unsigned M08 release preparation, and documentation all passed. Temporary outputs/checkouts were removed; driver installation, signing-mode changes, plugin/startup registration, live audio, and machine audio configuration remained excluded.
- Reinstalled/repaired the repository-local pinned Steinberg VST3 SDK at revision `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` and reran dedicated M06 acceptance: 51 SDK self-tests, 1,598 official validator tests, and offline loader validation (68 classes, finite stereo processing, five parameters/automation, 180-byte state) passed. The installed Visual Studio Community 2026 host resolved Windows SDK `10.0.28000.0`; no global SDK/plugin registration, driver, audio stream, or machine configuration changed.
- Next M02/M07/SEC-12 task: bound Windows audio-session display-name count and UTF-8 size before exposing application inventory, so OS-provided session metadata cannot expand control responses without limit. Verification: portable Windows-audio regressions, schema parity, workspace tests/Clippy/format/docs; rollback is isolated to metadata admission and schema assertions.
- Completed M02/M07/SEC-12 application-inventory hardening: Windows audio-session discovery now retains at most 64 display names per process, rejects names over 256 UTF-8 bytes, uses saturating session counters, and advertises matching bounds in `applications.list`. Windows-audio tests (26), control tests (86), full workspace checks, strict Clippy, contracts drift/typecheck, formatting, diff checks, and documentation acceptance pass; discovery remained read-only with no audio or machine configuration changes.
- Next M02/M07/API-01/SEC-12 task: advertise the existing 4,096-process enumeration ceiling in `applications.list`, so the read-only response cardinality is explicit and clients can size bounded inventories. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to the schema bound and assertion.
- Completed M02/M07/API-01/SEC-12 application-cardinality contract alignment: the 4,096-process Windows discovery ceiling is now a shared constant, enforced by enumeration, and advertised as `applications.list` `maxItems` with discovery regression coverage. Control and Windows-audio tests, targeted Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Requalified M01 CLI acceptance after the application-inventory contract changes: Windows process/application discovery, schema, status, and offline command checks passed. The discovery path remained read-only; no audio stream, endpoint selection, default, volume, mute, privacy, driver, or startup configuration changed.
- Next M07/API-08/SEC-12 task: align the `events.subscribe` output schema with its existing 500-event/500-session resync bounds and validated event metadata limits, so replay consumers can discover the complete bounded response contract. Verification: control discovery regression, control/workspace tests/Clippy/format/docs; rollback is isolated to the replay constant, schema metadata, and assertions.
- Completed M07/API-08/SEC-12 event replay contract alignment: `events.subscribe` now centralizes its 500-item limit and advertises bounded event/session arrays plus category, operation-ID, and session-ID lengths in discovery. Control tests (86), targeted strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Requalified the complete safe acceptance chain at pushed head `e48efb2`: native compile and read-only 34-endpoint format inventory, pinned disposable SysVAD x64 build/package/API validation, M01 CLI, M04 DSP/recording (27 DSP and 30 recording tests), M05 UI (87 tests and disposable production build), M06 SDK installer/VST3 validation, M07 headless, unsigned M08 release preparation, and documentation all passed. Temporary outputs/checkouts were removed; driver installation, signing-mode changes, plugin/startup registration, live audio, and machine audio configuration remained excluded.
- Next M01/M04/M07/API-01/SEC-12 task: align paged `sessions.list`, `graph.history`, and `recordings.list` output schemas with their existing 500/100/500 item limits, so clients can discover each response cardinality. Verification: control/storage/recording discovery regressions, workspace tests/Clippy/format/docs; rollback is isolated to shared page constants, schema metadata, and assertions.
- Completed M01/M04/M07/API-01/SEC-12 pagination contract alignment: `sessions.list`, `graph.history`, and paged `recordings.list` now advertise their enforced 500/100/500 item bounds, with shared constants used by input/runtime validation and discovery regressions. Control tests, recording tests, targeted strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M03/M07/API-01/SEC-12 task: align the paged `virtualDevices.list` response schema with its existing 500-item request/runtime bound, so managed-bus inventory consumers can discover the complete bounded read-only contract. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to the virtual-device page constant, schema metadata, and assertions.
- Completed M03/M07/API-01/SEC-12 virtual-device pagination alignment: `virtualDevices.list` now uses a shared 500-item bound for input validation, runtime paging, and discovered output schema, with control discovery regression coverage. Control tests (86), targeted strict Clippy, formatting, diff checks, and documentation acceptance pass; managed driver and endpoint lifecycle remain explicitly unavailable.
- Next M01/M07/API-01/SEC-12 task: expose the existing two-session and 128-byte identity limits on `status.get.activeSessionIds`, so status consumers can discover the bounded active-runtime identity contract. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to status schema metadata and assertions.
- Completed M01/M07/API-01/SEC-12 status contract alignment: `status.get.activeSessionIds` now advertises the enforced two-item maximum and 128-byte identity bound, with discovery regression coverage. Control tests (86), targeted strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/M02/API-01/GRAPH-01/GRAPH-03/SEC-12 task: replace the unconstrained `graph.plan.candidate` input placeholder with the shared bounded session schema, so graph planning clients receive the same limits enforced by candidate validation. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to input-schema reuse and assertions.
- Completed M01/M02/API-01/GRAPH-01/GRAPH-03/SEC-12 graph-plan contract alignment: `graph.plan.candidate` now reuses the bounded session schema, with discovery regression coverage confirming the node limit. Control tests (86), targeted strict Clippy, formatting, diff checks, and documentation acceptance pass; native graph activation remains open.
- Next M01/M02/API-01/GRAPH-03/SEC-12 task: advertise the three-entry maximum of `graph.plan.diff`, matching the fixed name/nodes/edges diff construction so plan consumers can size the response contract. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to the diff constant, schema metadata, and assertion.
- Completed M01/M02/API-01/GRAPH-03/SEC-12 graph-plan diff contract alignment: the `graph.plan.diff` output schema now advertises its fixed three-entry maximum, with control discovery regression coverage. Control tests (86), targeted strict Clippy, formatting, diff checks, and documentation acceptance pass; native graph activation remains open.
- Next M01/M02/API-01/GRAPH-01/SEC-12 task: bound `graph.plan.affectedDestinations` in discovery to the validated 64-node graph and 256-byte display-name limits, so plan responses cannot advertise an unbounded destination list. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to output schema metadata and assertions.
- Completed M01/M02/API-01/GRAPH-01/SEC-12 affected-destination contract alignment: `graph.plan.affectedDestinations` now advertises the 64-entry and 256-byte bounds inherited from validated graph nodes, with discovery regression coverage. Control tests (86), targeted strict Clippy, formatting, diff checks, and documentation acceptance pass; native graph activation remains open.
- Next M04/M07/API-01/SEC-12 task: expose the existing 4,096-part and 4,096-pause recorder checkpoint limits in the `recordings.recovery` output schema, so recovery clients can discover the bounded lifecycle metadata contract. Verification: recording/control discovery regressions, workspace tests/Clippy/format/docs; rollback is isolated to public recording constants, schema metadata, and assertions.
- Completed M04/M07/API-01/SEC-12 recovery-schema alignment: the `recordings.recovery` schema now advertises the enforced 4,096-part and 4,096-pause checkpoint bounds through shared recording constants. Recording tests (30), control tests (86), targeted strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M04/M07/API-01/SEC-12 task: expose the existing 4,096-part and 4,096-pause limits on recorder transition responses, matching the checkpoint/recovery bounds so all recorder lifecycle outputs are bounded and discoverable. Verification: control/recording regressions, workspace tests/Clippy/format/docs; rollback is isolated to recorder output schema metadata and assertions.
- Completed M04/M07/API-01/SEC-12 recorder-transition schema alignment: recorder lifecycle responses now advertise the enforced 4,096-part and 4,096-pause checkpoint bounds through shared recording constants. Control tests (86), recording tests (30), targeted strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/API-01/SEC-12 task: expose the validated 128-byte session identity limit on remaining session-scoped input schemas, so clients can discover the same identifier bound used by domain validation before dispatch. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to input-schema metadata and assertions.
- Completed M01/API-01/SEC-12 session-input schema alignment: deletion, lifecycle, duplication, route-inspection, and graph-history inputs now advertise the validated 128-byte session identity bound. Control tests (86), strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/API-01/SEC-12 task: expose the same validated 128-byte session identity limit on session lifecycle, deletion, and graph-commit responses, so output consumers receive a bounded public contract. Verification: control discovery regression, workspace tests/Clippy/format/docs; rollback is isolated to output-schema metadata and assertions.
- Completed M01/API-01/SEC-12 session-output schema alignment: session deletion/start/stop and graph-commit responses now advertise the validated 128-byte session identity maximum, with discovery regression coverage. Control tests (86), strict Clippy, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/M02/API-01/GRAPH-12/SEC-12 task: determine and bound `routes.inspect` path cardinality and nested path fields against the graph inspection implementation, preserving explicit completeness semantics if a safe response ceiling is required. Verification: domain/control regressions, workspace tests/Clippy/format/docs; rollback is isolated to route-inspection bounds and evidence.
- Completed M01/M02/API-01/GRAPH-12/SEC-12 route-field schema alignment: `routes.inspect` now advertises validated destination/entity lengths, per-path node/edge cardinalities, channel-map cardinality, four-coefficient matrices, and coefficient range limits. Control tests (86), strict Clippy, formatting, diff checks, and documentation acceptance pass; path-list cardinality remains open pending an explicit completeness policy.
- Next M02/API-01/GRAPH-12/SEC-12 task: add an explicit completeness policy for `routes.inspect` path enumeration before imposing a path-count ceiling, since the current result promises complete provenance and a silent truncation would be incorrect. Verification: domain/control regression, contract/UI parity, workspace tests/Clippy/format/docs; rollback is isolated to route result semantics and consumers.
- Completed M02/API-01/GRAPH-12/SEC-12 route completeness policy: route inspection now caps path enumeration at 500 entries, reports `complete: false` when the cap is exceeded, and updates the API contract/UI to distinguish partial provenance from complete results. Domain/control tests, contracts/UI typechecks, strict Clippy, formatting, diff checks, and documentation acceptance pass; native route activation remains open.
- Next M06/M07/API-01/SEC-12 task: advertise the existing 256-candidate plugin scan ceiling in `plugins.scan/list/retry` output schemas, so bounded read-only plugin inventories are discoverable before plugin execution remains deliberately isolated. Verification: plugin-host/control regressions, workspace tests/Clippy/format/docs; rollback is isolated to schema metadata and discovery assertions.
- Completed M06/M07/API-01/SEC-12 plugin-inventory contract alignment: `plugins.scan/list/retry` now advertise the existing 256-candidate scan ceiling, with control discovery regression coverage. Plugin-host/control tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no plugin registration or machine audio configuration changed.
- Next M01/M04/M07/API-01/SEC-12 task: advertise fixed node-type, voice-preset, EQ-preset, and node-parameter collection bounds in discovery schemas, matching the authoritative registries and domain parameter limit. Verification: control/domain/DSP regressions, contracts typecheck/drift, workspace tests/Clippy/format/docs; rollback is isolated to catalog schema metadata and assertions.
- Completed M01/M04/M07/API-01/SEC-12 catalog-schema alignment: node-type, voice-chain, EQ-preset, processor, and node/processor-parameter collections now advertise their authoritative cardinality bounds, with discovery regression coverage. Control/domain/DSP tests, strict Clippy, contract checks, formatting, diff checks, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/API-01/SEC-12 task: align client-enrollment identifier schemas with the bounded local identity contract, after confirming the persistence and authorization paths enforce the same limit. Verification: control/storage regressions, workspace tests/Clippy/format/docs; rollback is isolated to client identity validation/schema metadata and tests.
- Completed M01/API-01/SEC-12 client-enrollment identity alignment: enrollment and revocation reject IDs over the validated 128-byte limit, and client-management input/output schemas advertise that bound. Control (86) and storage (39) tests, strict Clippy, formatting, diff checks, contract typecheck, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/M07/API-01/SEC-12 task: align client-enrollment list cardinality and client-ID fields in discovery with the persistence boundary, without imposing an arbitrary enrollment cap unless storage/runtime policy provides one. Verification: control/storage discovery regressions, workspace tests/Clippy/format/docs; rollback is isolated to contract metadata or an explicitly justified storage limit.
- Completed M01/M07/API-01/SEC-12 client-enrollment cardinality audit: no storage/runtime enrollment-count ceiling exists, so the plan preserves an unbounded list contract while enforcing the shared 128-byte client-ID field bound. No arbitrary enrollment cap was introduced; valid persistence and authorization behavior remains covered by control/storage tests.
- Next M01/M04/API-01/SEC-12 task: close the legacy unpaged `recordings.list` allocation gap by using the bounded 500-record page query and requiring cursor pagination when more records exist. Verification: control/storage regressions, workspace tests/Clippy/format/docs; rollback is isolated to list dispatch behavior and regression coverage.
- Completed M01/M04/API-01/SEC-12 unpaged recording-list hardening: the legacy array response now uses the bounded 500-record query and requires cursor pagination when additional records exist, avoiding unbounded loading or silent truncation. Control/storage tests, strict Clippy, formatting, diff checks, contract typecheck, and documentation acceptance pass; no audio or machine configuration changed.
- Next M01/M02/API-01/SEC-12 task: audit the legacy unpaged `devices.list` array branch against the 500-record contract and native endpoint enumeration behavior, preserving complete inventory semantics without changing endpoint state. Verification: control/windows-audio regressions, read-only native inventory check, workspace tests/Clippy/format/docs; rollback is isolated to list response handling.
- Completed M01/M02/API-01/SEC-12 unpaged device-list hardening: the compatibility array response advertises the 500-endpoint ceiling and now requires cursor pagination when a native inventory exceeds it, preserving complete inventory semantics without opening streams or changing endpoint state. Control/windows-audio tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; no machine audio configuration changed.
- Next M01/M02/API-01/SEC-12 task: audit legacy unpaged `virtualDevices.list` and other compatibility-array responses for the same bounded-completeness behavior, using authoritative inventory limits and avoiding arbitrary truncation.
- Completed M01/M02/API-01/SEC-12 compatibility-array audit: the unpaged `virtualDevices.list` response now advertises the authoritative eight-bus inventory ceiling; its runtime list already cannot exceed that domain limit, so no truncation behavior was added. Control/domain tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; managed driver activation remains unavailable.
- Next M01/API-01/SEC-12 task: audit remaining unpaged compatibility arrays for authoritative cardinality limits and document any intentionally unbounded persistence-backed responses rather than adding arbitrary caps.
- Completed M01/API-01/SEC-12 compatibility-array audit: unpaged device, virtual-device, and recording responses now advertise and enforce their authoritative bounds; persistence-backed client lists remain intentionally unbounded because no storage policy exists. Control/domain/Windows-audio/storage regressions, strict Clippy, formatting, diff checks, and docs acceptance pass.
- Next M01/M04/API-01/SEC-12 task: align recording item identity and path field schemas with the bounded identifiers and metadata limits enforced by storage/recording validation, covering all recording read and mutation responses.
- Completed M01/M04/API-01/SEC-12 recording identity alignment: storage now rejects recording, session, and recorder identifiers over 128 bytes, and the shared recording item schema advertises those limits. Control/storage tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; recording path policy remains intentionally unspecified.
- Next M01/M04/API-01/SEC-12 task: propagate the 128-byte recording-ID bound to every recording recovery, preview, metadata, rename, reveal, recycle, and removal input/output schema, preserving the storage boundary across all lifecycle methods.
- Completed M01/M04/API-01/SEC-12 recording lifecycle schema alignment: all recording recovery, preview, metadata, rename, reveal, recycle, and removal schemas now advertise the storage-enforced 128-byte recording-ID maximum. Control/storage tests, strict Clippy, formatting, diff checks, and documentation acceptance pass; path length remains separately unspecified.
- Completed M01/M04/API-01/SEC-12 recording path-policy audit: storage and recording policy enforce absolute/root/reparse-point and file-action safety, but define no path-length ceiling; path fields remain intentionally unbounded in discovery until a platform support policy is approved. No arbitrary compatibility cap was introduced.
- Next M00-M08/API-01/SEC-12 task: requalify the complete safe acceptance chain at the current pushed head and refresh milestone evidence, preserving all native driver, signing, installer, hardware, and manual-acceptance blockers.
- Completed M00-M08/API-01/SEC-12 safe-chain requalification at pushed head `8fd56c1`: native compile, read-only 34-endpoint inventory, disposable SysVAD x64 qualification, M01 CLI, M04 DSP/recording, M05 UI, M06 SDK/VST3, M07 headless, unsigned M08 release preparation, and documentation all passed. Driver installation/loading, signing-mode changes, live audio, plugin/startup registration, installer, hardware, and manual UI/accessibility gates remain open and were not attempted.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M02/ENG-01/ENG-03/ARCH-07 PCM boundary slice: `AudioBlock` now decodes caller-owned interleaved PCM16 into planar float32 and encodes planar float32 back to bounded interleaved PCM16 without allocation, with explicit shape validation, non-finite silence, and endpoint clamp behavior. Engine tests (66), strict engine Clippy, formatting, and diff checks pass; native scheduling and physical/driver gates remain open.
- Next M02/ENG-01/ENG-03/ARCH-07 task: connect the bounded PCM16 bridge to a preallocated block-quantum adapter harness, proving packet accumulation/splitting into 128-frame engine blocks without allocation or unbounded buffering. Verification: engine/Windows adapter regression and guarded live evidence only if required; rollback is isolated to the new adapter seam and tests.
- Completed M02/ENG-01/ENG-03/ARCH-07 quantum adapter slice: `Pcm16QuantumAdapter` now stages split interleaved PCM16 packets in a fixed one-quantum buffer, applies backpressure when full, and decodes exact 128-frame mono/stereo blocks into preallocated engine storage. Engine tests (67), strict engine Clippy, formatting, and diff checks pass; native scheduler/device integration remains open.
- Next M02/ENG-01/ENG-03/ARCH-07 task: exercise the fixed quantum adapter with real process-loopback packet boundaries through a guarded Windows harness, retaining the existing media/configuration snapshot and cleanup checks.
- Completed M02/ENG-01/ENG-03/ARCH-07 guarded packet-boundary evidence: the Rust process-loopback include and exclude runs each delivered 10,584 PCM16 frames across 24 packets and emitted 82 exact 128-frame engine quanta. The harness passed stream stop/reset and unchanged media-device state checks; no persistent audio configuration changed. Physical latency, native scheduler integration, and production-driver gates remain open.
- Next M02/ENG-01/ENG-03/ARCH-07 task: integrate the bounded capture-to-quantum adapter with the existing realtime scheduler rings, preserving generation filtering and explicit overflow/underrun accounting before any route claim is expanded.
- Completed M02/ENG-01/ENG-03/ARCH-07 scheduler-ring integration: the guarded Rust process-loopback include and exclude runs each converted 10,584 frames into 82 exact quanta, submitted them through the bounded scheduler rings, processed generation 1, and recycled matching outputs. Stop/reset and media-state checks passed; no persistent audio configuration changed. Explicit overflow/underrun stress behavior and native output routing remain separate work.
- Next M02/ENG-01/ENG-03/ARCH-07 task: add deterministic scheduler overflow/underrun regression coverage for the capture adapter, proving input backpressure and output starvation are counted and fail closed without blocking or allocation.
- Completed M02/ENG-01/ENG-03/ARCH-07 scheduler pressure regression: a one-block scheduler now has coverage proving input-ring overflow is rejected immediately and counted, while held output ownership causes output starvation to fail closed and increment the xrun counter. The focused engine suite (68), strict Clippy, formatting, and diff checks pass.
- Next M02/ENG-01/ENG-03/ARCH-07 task: add a bounded scheduler adapter helper that drains stale-generation output explicitly during graph publication, then cover replacement while capture blocks are in flight.
- Completed M02/ENG-01/ENG-03/ARCH-07 generation replacement slice: `RealtimeScheduler::publish` now recycles queued output ownership at the control-plane publication boundary, while generation-filtered receive remains the race-safe guard for in-flight old blocks. Replacement and ownership tests pass; the guarded live process-loopback scheduler run still passes both modes with 82 generation-1 quanta and unchanged media state. Engine tests (69), tool check, strict Clippy, formatting, and diff checks pass.
- Next M02/M03/ARCH-05/ARCH-07 task: qualify event-driven scheduler wakeup and bounded period adaptation against the existing endpoint event handles, without claiming realtime timing until a native callback timing measurement exists.
- Completed M02/M03/ARCH-05/ARCH-07 event-wakeup slice: `ProcessLoopbackCapture::wait_for_data` now waits on the WASAPI event handle with bounded timeout/error handling, and the scheduler probe uses that wakeup path instead of polling sleeps. Guarded include/exclude runs delivered 11,025 frames across 25 packets and processed 86 generation-1 quanta each; media state and persistent audio configuration were unchanged. This does not claim callback deadline or physical-latency compliance.
- Next M02/M03/ARCH-05/ARCH-07 task: add bounded period/packet telemetry to the event-driven adapter, recording observed packet-frame ranges and timeout counts without logging or allocating on the callback path.
- Completed M02/M03/ARCH-05/ARCH-07 packet telemetry slice: process-loopback delivery now exposes bounded atomic snapshots for waits, timeouts, packets, frames, packet-frame minimum/maximum, and silent packets. Guarded include/exclude runs each observed 25 packets/11,025 frames with 441-frame packets, 39–40 waits, and 14–15 timeouts; no silent packets or persistent audio configuration changes occurred. This records observations but does not claim deadline compliance.
- Next M02/M03/ARCH-05/ARCH-07 task: add bounded period adaptation policy tests that map observed endpoint packet sizes to whole engine quanta and fail closed on unsupported or excessive periods.
- Completed M02/M03/ARCH-05/ARCH-07 rejection telemetry: process-loopback diagnostics now include a saturating `rejected_packets` count for periods rejected after WASAPI ownership release. Guarded include/exclude runs reported zero rejected packets with 441-frame periods and 82/86 scheduler quanta; Windows-audio tests (28), strict Clippy, tool compilation, formatting, and diff checks pass. No persistent audio configuration changed.
- Next M02/M03/ARCH-05 task: add bounded telemetry snapshot tests for counter monotonicity and zero-state semantics, then keep callback deadline and physical-latency claims gated on native scheduler evidence.
- Completed M02/M03/API-01 telemetry invariant slice: zero-state telemetry is explicitly all-zero, and counter increments saturate at `u64::MAX` rather than wrapping. Windows-audio tests (29), strict Clippy, formatting, and diff checks pass; no audio stream or machine configuration was changed.
- Next M02/M03/API-01 task: audit the adapter telemetry field bounds against the diagnostics/schema budget and document the control-plane `not activated` distinction in the API reference.
- Completed M02/M03/API-01/ARCH-05 adapter-status audit: process-loopback telemetry remains available through the shared Windows adapter snapshot, while control-plane diagnostics correctly continues to report `nativeAdapter: not activated` because no native adapter session is owned by the control plane. This preserves an honest unavailable state and avoids fabricating telemetry; no machine audio configuration changed.
- Next M02/M03/ARCH-05 task: measure callback-period timing and deadline behavior only after the native realtime scheduler owns the endpoint stream; the current portable/event evidence cannot close that gate.
- Completed M02/M03/ARCH-05/ARCH-07 shared period-admission policy: `Pcm16QuantumAdapter::push_packet` now independently rejects empty or over-4,096-frame packets before staging, matching the Windows adapter bound; valid packets remain split into exact 128-frame quanta. Engine (69) and Windows-audio (28) tests, strict workspace Clippy, formatting, tool compilation, and guarded live include/exclude runs passed. Both live modes observed 441-frame packets and no persistent audio configuration change.
- Next M02/M03/ARCH-05/ARCH-07 task: verify bounded period adaptation across a synthetic range of packet sizes around quantum boundaries, including partial carry and exact-boundary behavior, before attempting any differing-rate hardware claim.
- Completed M02/M03/ARCH-05/ARCH-07 synthetic period matrix: engine coverage now verifies 127+1 carry, exact 128-frame packets, and a maximum 4,096-frame packet drained in bounded quanta without staging growth. Engine tests (70), strict Clippy, formatting, and diff checks pass; no hardware or machine audio configuration was touched.
- Next M02/M03/ARCH-07 task: connect packet-period telemetry to the read-only diagnostics contract without fabricating an active native stream when the control plane has no adapter session.
- Completed M02/M03/ARCH-05/ARCH-07 packet-period policy: the process-loopback adapter now rejects zero or over-4,096-frame packets after releasing WASAPI ownership, before caller-buffer copying; valid packets remain splittable into fixed 128-frame quanta. Policy regression, Windows-audio tests (28), strict Clippy, formatting, tool compilation, and guarded live include/exclude runs passed. Live packets were 441 frames; no persistent audio configuration changed.
- Next M02/M03/ARCH-05/ARCH-07 task: add explicit bounded event-wait and packet-period diagnostics to the shared adapter status surface, keeping timing claims separate from host observations.
- Completed M02/M03/ARCH-05/ARCH-07 adapter status-surface slice: bounded event-wait, packet count/frame totals, min/max packet period, and silent-packet diagnostics are exposed through `ProcessLoopbackCapture::telemetry`; the 4,096-frame policy is enforced before copying. Focused Windows tests (28), strict Clippy, full workspace tests, formatting, documentation, and guarded live evidence pass with no persistent audio configuration change.
- Next M02/M03/ARCH-05/ARCH-07 task: add bounded period adaptation policy tests that map observed endpoint packet sizes to whole engine quanta and fail closed on unsupported or excessive periods.
- Revalidated the full locked workspace at pushed head `2ec7701` after the scheduler pressure changes: all unit tests and doc-tests passed, including engine (68), Windows-audio (27), control (86), CLI (25), recording, storage, transport, plugin-host/worker, and supporting crates. No machine audio configuration changed.
- In progress M02/CAP-03/CAP-04/CAP-07/ARCH-07 task: expose bounded Rust process-loopback capture in the Windows adapter, using the supported asynchronous activation path and explicit include/exclude tree mode. The API must preserve process identity inputs, use exact shared-mode initialization, expose only caller-owned packet copies, and stop/reset without changing endpoint defaults. Verification will include focused Windows adapter tests, a guarded live process-loopback run, workspace checks, and documentation; rollback is isolated to the adapter API and tests.
- Completed M02/CAP-03/CAP-04/CAP-07/ARCH-07 process-loopback adapter slice: `ProcessLoopbackCapture` now uses asynchronous `ActivateAudioInterfaceAsync`, explicit include/exclude target-tree mode, the supported 44.1 kHz stereo PCM/event-callback initialization shape, caller-owned packet copies, bounded activation timeout, and stop/reset cleanup. The guarded Rust acceptance passed both modes with 24 packets/10,584 frames each after fixing PROPVARIANT ownership, COM thread-boundary handling, and process-loopback format requirements. Focused Windows tests, the full locked workspace test suite, strict all-target/all-feature Clippy, formatting, and documentation checks pass; no persistent audio configuration changed.
- Completed M03/M01/API-01/VDEV-02/SEC-12 virtual-device identity alignment: managed virtual-device plan inputs and list outputs now advertise the domain-enforced 128-byte entity-ID bound. Control discovery regression, workspace tests, strict Clippy, formatting, diff checks, and documentation validation pass; no driver or machine audio configuration changed.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M02/M01/API-01/GRAPH-12/SEC-12 route-output schema alignment: `routes.inspect.paths` now advertises the implemented 500-path ceiling, matching `complete: false` truncation semantics and discovery regression coverage. Control tests, strict Clippy, formatting, diff checks, and documentation validation pass; native route activation remains open.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M03/M01/API-01/VDEV-02/SEC-12 virtual-bus name-bound alignment: the 120-Unicode-character name limit is now a domain authority and is reused by validation, virtual-device input/output schemas, and `system.describe` limits. Domain/control tests, strict Clippy, formatting, diff checks, and documentation validation pass; no driver or machine audio configuration changed.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M01/API-01/SEC-12 discovery-schema audit: `system.describe` now advertises bounded methods, node types, processors, and preset collections using the same authoritative registries used to build the response. Control tests (86), strict Clippy, formatting, diff checks, and documentation validation pass; no audio or machine configuration changed.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M04/M05/API-01/SEC-12 recording metadata boundary alignment: the live UI backend adapter rejects title, artist, and comment values above the storage-aligned 256 Unicode-character limit before dispatch, with regression coverage. UI tests/typecheck/build and documentation validation pass; no audio or machine configuration changed.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00-M08/API-01/SEC-12 safe-chain requalification at pushed head `0e28f3a`: native compile, read-only 34-endpoint inventory, disposable SysVAD x64 qualification, full workspace tests, M01 CLI, M04 DSP/recording, M05 UI, M06 SDK/VST3, M07 headless, unsigned M08 release preparation, contracts, and documentation all passed. Driver installation/loading, signing-mode changes, live audio, plugin/startup registration, installer, hardware, and manual UI/accessibility gates remain open and were not attempted.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00/CAP-01/CAP-02/ARCH-07/NFR-01 live lifecycle evidence: the authorized bounded native check exercised 13 shared capture endpoints and 21 render endpoints for 250 ms, handled one occupied render endpoint, and stopped/reset every client successfully. The probe used silent render buffers and reported defaults, volume, mute, privacy, drivers, signing, and startup configuration unchanged; physical latency, process attribution, and production driver gates remain open.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00/CAP-01/CAP-02/ARCH-07/NFR-01 event-driven lifecycle evidence: the authorized 250 ms event-mode check passed on the existing VB-Audio render/capture endpoints, reporting 16,800 submitted render frames and 12,480 capture frames. It used silent render only and stopped/reset its clients; defaults, volume, mute, privacy, drivers, signing, and startup configuration were unchanged.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00/CAP-01/CAP-02/CAP-05/ARCH-07/NFR-01 signal-path evidence: the authorized 250 ms tone/capture check on the existing VB-Audio virtual cable captured 6,056 nonzero payload bytes and passed bounded start/stop/reset cleanup. The harness verified media-device identity/state rollback and reported defaults, volume, mute, privacy, drivers, signing, and startup configuration unchanged; physical latency and production-driver gates remain open.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00/CAP-03/CAP-04/ARCH-07/NFR-01 process-attribution evidence: the authorized 250 ms process-scoped loopback check captured 10,584 frames and 32,907 nonzero bytes from a disposable child/process tree, then completed cleanup with no persistent audio configuration change. Physical latency, cross-process exclusion, and production-driver gates remain open.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M02/ENG-01/ENG-03/ENG-04/CAP-01/CAP-02 Rust-adapter live evidence: the authorized 250 ms production Rust adapter run passed with 26 capture packets, 12,480 capture frames, 97 generation-1 graph blocks, 13,536 render frames, zero scheduler XRuns, and deterministic stream stop/reset cleanup. It used zero-valued render buffers; routed signal remained false, and endpoint-specific initialization/physical-latency gates remain separately open.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M02/ENG-01/ENG-03/ENG-04/CAP-01/CAP-02 route activation evidence: the authorized 100 ms Rust adapter route run on the existing virtual cable passed with 4,800 capture frames, 4,736 scheduler frames, and 4,736 routed frames. Streams stopped/reset successfully and persistent audio configuration was unchanged; physical latency and production-driver gates remain open.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00/CAP-05/NFR-01 impulse-correlation evidence: 50 expected impulses on the existing VB-Audio virtual cable produced 51 detected groups, p95 spacing error of 471 frames, and estimated onset of 26.27 ms. This remains software signal-correlation evidence only; calibrated physical p95 latency and production-driver gates remain open.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M03/M01/API-01/VDEV-02/SEC-12 virtual-device operation schema alignment: `virtualDevices.plan` input and plan/apply outputs now reuse one fixed operation schema with bounded action, identity, name, and enabled fields. Control tests (86), strict Clippy, formatting, diff checks, and documentation validation pass; driver activation remains open.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M01/M07/API-01/SEC-12 event discovery alignment: the fixed 15-category state-event registry is now shared by `system.describe` response construction and its output schema, which advertises the exact category bound. Control tests (86), strict Clippy, formatting, diff checks, and documentation validation pass; no audio or machine configuration changed.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M01/M07/API-01/SEC-12 plan-metadata bounds: startup, graph, and virtual-device plan schemas now advertise their one-entry required-scope/warning arrays through shared control constants, matching current fail-closed/validated response construction. Control tests, strict Clippy, formatting, diff checks, and documentation validation pass; no audio or machine configuration changed.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00/CAP-06/CAP-11 process-identity regression evidence: the Windows process-binding test passed and proved a restarted helper process cannot inherit a stale executable/creation-time binding. The test launches only disposable bounded helpers, performs no audio access, and does not replace full reboot, service-restart, or multi-user transition evidence.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00/CAP-07 process-loopback exclusion-mode evidence: the controlled native harness now exercises the supported exclude-tree activation with a disposable child, verifies capture start/stop/reset and child cleanup, and checks the media-device identity/state snapshot before and after. This validates the native mode and lifecycle only; it does not claim a cross-process rejection threshold, reboot behavior, or production-driver capability. No persistent audio configuration changed.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00-M08/API-01/SEC-12 safe-chain requalification at pushed head `19f86e8`: native compile and 34-endpoint read-only format inventory, disposable pinned SysVAD x64 qualification, full locked workspace tests, M01 CLI, M04 DSP/recording, M05 UI, M06 SDK/VST3, M07 headless, unsigned M08 release preparation, and documentation all passed. Temporary outputs/checkouts were removed; driver installation/loading, signing-mode changes, plugin/startup registration, installer, hardware, and manual UI/accessibility gates remain open and were not attempted.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00-M08/API-01/SEC-12 safe-chain requalification at pushed head `f741be5` (2026-09-07): native compile and 34-endpoint read-only format inventory, disposable pinned SysVAD x64 qualification, locked workspace tests, M01 CLI, M04 DSP/recording, M05 UI (typecheck, 88 tests, temporary production build), M06 SDK/VST3 validation, M07 headless checks, unsigned M08 release preparation, and documentation validation all passed. Temporary outputs/checkouts were removed. No driver was installed or loaded, signing mode was unchanged, and no plugin/startup registration or machine audio configuration was changed.
- Next M00/M02/M03/M08 task: maintain the explicit native qualification backlog and investigate only safe, non-mutating evidence improvements until production driver/signing/installer authority and hardware/manual acceptance are available.
- Completed M00-M08/API-01/SEC-12 safe-chain requalification at pushed head `419c368` on 2026-09-08: native compile and 34-endpoint read-only inventory, disposable pinned SysVAD x64 qualification, workspace checks, M01/M04/M05/M06/M07 validation, unsigned M08 preparation, and documentation validation all passed. Temporary outputs/checkouts were removed; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00-M08/API-01/SEC-12 safe-chain requalification after `f0d1b03` on 2026-09-08: elevated native compile and 34-endpoint read-only inventory, disposable pinned SysVAD x64 qualification, M01/M04/M05/M06/M07 validation, unsigned M08 preparation, and documentation validation (51 Markdown files/158 local links) all passed. Temporary outputs/checkouts were removed; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M02/ARCH-05/ENG-03 raw timing-distribution propagation on 2026-09-08: adapter and routed acceptance output now preserves all 32 fixed histogram buckets, and both guarded 300 ms checks require exactly 32 entries. The routed run reported 116 samples, 1,134,000 ns total, 28,400 ns maximum, and unchanged media state. This remains adapter/event-loop evidence, not native callback deadline evidence.
- Next M02/M03/ARCH-05 task: connect the bounded telemetry to a production-style native scheduler callback when that scheduler owns an endpoint stream, then measure callback period/deadline distributions without changing the user's configured defaults.

- Completed M02/ARCH-05/ENG-03 histogram-content validation on 2026-09-08: both guarded adapter acceptance wrappers now verify sequential bucket labels, numeric counts, exactly 32 entries, and a count sum equal to reported samples. Both 300 ms checks passed; the routed run reported 108 complete samples and unchanged media state.
- Next M02/M03/ARCH-05 task: connect the bounded telemetry to a production-style native scheduler callback when that scheduler owns an endpoint stream, then measure callback period/deadline distributions without changing the user's configured defaults.

- Completed M02/ARCH-05/ENG-03 route evidence-summary propagation on 2026-09-08: the guarded routed-adapter wrapper now emits the validated graph-block, timing-total, timing-maximum, and histogram-sample values in its final acceptance line. The 300 ms run passed with 14,400 capture frames, 112 graph blocks, 14,336 scheduled frames, 13,920 routed frames, and unchanged media state.
- Next M02/M03/ARCH-05 task: connect the bounded telemetry to a production-style native scheduler callback when that scheduler owns an endpoint stream, then measure callback period/deadline distributions without changing the user's configured defaults.

- Completed M02/ARCH-05/ENG-03 route telemetry completeness hardening on 2026-09-08: the guarded routed-adapter acceptance now requires histogram samples to equal reported graph blocks, in addition to validating timing bounds and nonzero routed frames. The 300 ms route acceptance passed with 13,920 capture, 13,824 scheduler, and 13,824 routed frames; media state was unchanged.
- Next M02/M03/ARCH-05 task: connect the bounded telemetry to a production-style native scheduler callback when that scheduler owns an endpoint stream, then measure callback period/deadline distributions without changing the user's configured defaults.

- Completed M02/ARCH-05/ENG-03 callback instrumentation on 2026-09-08: the runtime processor now records saturating total and maximum monotonic processing duration in nanoseconds through allocation-free atomic counters, and scheduler telemetry exposes the values for off-thread diagnostics. Engine tests (70), locked workspace tests/doc-tests, strict Clippy, formatting, and documentation validation passed. This is instrumentation readiness, not native callback deadline evidence.
- Next M02/M03/ARCH-05 task: connect this instrumentation to a production-style native scheduler callback when that scheduler owns an endpoint stream, then measure callback period/deadline distributions without changing the user's configured defaults.

- Completed M02/ARCH-05/ENG-03 bounded callback histogram on 2026-09-08: runtime processing duration now has a fixed 32-bucket nanosecond histogram in addition to saturating total/maximum counters, allowing future off-thread percentile calculation without retaining per-callback samples. Engine tests (70), workspace compilation, strict Clippy, formatting, diff checks, and documentation validation passed. This remains portable instrumentation readiness, not native callback deadline evidence.
- Next M02/M03/ARCH-05 task: connect the bounded telemetry to a production-style native scheduler callback when that scheduler owns an endpoint stream, then measure callback period/deadline distributions without changing the user's configured defaults.

- Completed M02/ARCH-05/ENG-03 adapter telemetry propagation on 2026-09-08: `adapter_smoke` now validates and reports processing-time total, maximum, and histogram sample count, and the guarded route acceptance requires the same bounded timing fields. Probe check and both guarded 300 ms live adapter/route checks passed; the adapter run reported 116 histogram samples for 116 processed quanta, zero scheduler xruns/overruns, and unchanged media state. This remains event-loop/adapter evidence, not production native callback deadline evidence.
- Next M02/M03/ARCH-05 task: connect the bounded telemetry to a production-style native scheduler callback when that scheduler owns an endpoint stream, then measure callback period/deadline distributions without changing the user's configured defaults.

- Completed M02/ARCH-05/ENG-03 timing-test portability correction on 2026-09-08: timing regressions now validate accounting invariants without assuming that the platform clock has nonzero nanosecond resolution. Engine tests (71), strict Clippy, formatting, and diff checks passed. This preserves the native callback-deadline gate as unclaimed.
- Next M02/M03/ARCH-05 task: connect the bounded telemetry to a production-style native scheduler callback when that scheduler owns an endpoint stream, then measure callback period/deadline distributions without changing the user's configured defaults.

- Completed M02/ARCH-05/ENG-03 timing regression coverage on 2026-09-08: an inactive runtime now has explicit regression coverage proving that its silence path records one bounded timing-histogram observation without incrementing processed-quanta counts. Engine tests (71), strict Clippy, formatting, diff checks, and workspace compilation passed. This does not establish native callback deadline evidence.
- Next M02/M03/ARCH-05 task: connect the bounded telemetry to a production-style native scheduler callback when that scheduler owns an endpoint stream, then measure callback period/deadline distributions without changing the user's configured defaults.

- Completed M00-M08/API-01/SEC-12 safe-chain requalification after `b181406` on 2026-09-08: elevated native compile and 34-endpoint read-only inventory, disposable pinned SysVAD x64 qualification, M01/M04/M05/M06/M07 validation, unsigned M08 preparation, and documentation validation (51 Markdown files/158 local links) all passed. Temporary outputs/checkouts were removed; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00/CAP-03/CAP-04/CAP-06/QUAL-01 native process-attribution recheck on 2026-09-08: the bounded 500 ms native acceptance captured 21,609 frames and 77,823 nonzero bytes from a disposable child/process tree, then completed cleanup with unchanged media state. This remains controlled process-tree evidence, not full exclusion, reboot/PID-reuse, or physical-latency evidence.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00/CAP-05/QUAL-01 maximum digital impulse qualification on 2026-09-08: the 2,000-impulse VB-Audio run detected 1,997 groups with zero p95 spacing error and estimated onset of 77.77 ms; temporary artifacts and endpoint state were cleaned up. This remains digital timing evidence, not calibrated physical latency or callback-deadline evidence.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00/CAP-06/CAP-07 native process-loopback exclusion recheck on 2026-09-08: the bounded 250 ms native acceptance captured 11,025 frames while excluding a disposable child tree, then completed child/stream cleanup with unchanged media state. This validates mode/lifecycle only, not a full cross-process rejection threshold or physical latency.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00/CAP-01/CAP-02/ARCH-05/NFR-01 native endpoint sweep on 2026-09-08: all 13 active capture and 21 active render endpoints were exercised for bounded start/stop/reset behavior; one occupied render endpoint was handled as an expected ownership case. Silent buffers and media/configuration snapshots confirmed no persistent changes. This remains lifecycle evidence, not production-driver, callback-deadline, or physical-latency evidence.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00/CAP-05/QUAL-01 digital signal-path recheck on 2026-09-08: the bounded native tone/loopback acceptance produced 213,082 nonzero capture bytes from the explicitly selected VB-Audio cable, with lifecycle cleanup and unchanged machine state. This is digital transfer evidence only, not calibrated physical latency or managed-driver/application compatibility evidence.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00/CAP-05/QUAL-01/NFR-01 digital impulse recheck on 2026-09-08: the authorized 1,000-impulse VB-Audio cable run detected 997 groups with p95 spacing error of 0 frames and estimated onset of 67.27 ms; cleanup and unchanged machine state were preserved. This remains digital correlation evidence and does not satisfy calibrated physical latency or callback deadline requirements.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00/CAP-01/CAP-02/ARCH-05 native event-lifecycle recheck on 2026-09-08: the bounded 300 ms event acceptance passed on the explicitly selected VB-Audio endpoints with 14,400 capture and 19,200 silent render frames; initialize/event/start/stop/reset succeeded and media state was unchanged. This remains shared-mode lifecycle evidence, not callback-deadline or physical-latency evidence.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00/M02 differing-rate prerequisite recheck on 2026-09-08: read-only inventory confirmed the 96 kHz mono capture and 48 kHz stereo render pair used by the guarded route qualification. No endpoint state or defaults changed; the resulting 2,000 ms route evidence is recorded under M02. Native production-driver, callback-deadline, physical-latency, signing, installer, and manual UI gates remain open.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M02/ARCH-05/ARCH-07 maximum-duration differing-rate stability evidence on 2026-09-08: the guarded 2,000 ms 96 kHz mono to 48 kHz stereo route passed with 191,040 capture, 95,488 scheduler, and 95,488 routed frames; detailed counters observed 20 queued resampler frames, bounded -100 ppm correction, and zero scheduler xruns/overruns. Cleanup and unchanged machine state were verified. This remains short-duration shared-mode evidence, not independent-clock lock or physical-latency evidence.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M02/CAP-01/CAP-02/ENG-03/ARCH-05/ARCH-07 differing-rate route evidence on 2026-09-08: the guarded endpoint-ID-selected adapter route passed from the current 96 kHz mono capture endpoint to a 48 kHz stereo render endpoint, processing 28,800 capture frames into 14,336 scheduler/routed frames with zero scheduler xruns/overruns and unchanged machine state. Drift correction reached its declared -100 ppm bound; this does not claim long-term clock lock, physical latency, managed-driver lifecycle, or application compatibility.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M02/CAP-01/CAP-02/ENG-01/ENG-03/ARCH-05/ARCH-07 native adapter qualification on 2026-09-08: the bounded shared capture/render scheduler smoke passed with 15,360 capture/scheduler frames, 120 generation-1 quanta, zero scheduler xruns/overruns, and unchanged media state. The explicitly selected VB-Audio virtual-cable route also passed with 13,920 capture and 13,824 routed frames, with endpoint/default/volume/mute/privacy/driver/signing/startup state unchanged. This remains adapter/route evidence, not managed-driver lifecycle, physical-latency, or callback-deadline evidence.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00-M08 workspace regression after `99f4eac` on 2026-09-08: locked workspace tests passed across control (86), domain (53), DSP (27), engine (70), plugin host/worker (39/8), protocol (6), recording (30), storage (40), transport (17), and Windows audio (29), with doc tests passing. Native production-driver, signing, installer, physical-latency, and manual UI gates remain open.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M02/ARCH-05/ARCH-07 scheduler telemetry accounting on 2026-09-08: control-boundary output draining now uses a non-counting queue pop, so intentional generation replacement does not produce a false underrun. The regression, engine tests (70), probe check, formatting, and guarded include/exclude live acceptance passed; both modes reported zero scheduler xruns and input/output overrun/underrun counters with unchanged media state.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M00-M08/API-01/SEC-12 safe-chain requalification after `1264fa4` on 2026-09-08: native compile and 34-endpoint read-only inventory, disposable pinned SysVAD x64 qualification, locked workspace checks, M01/M04/M05/M06/M07 validation, unsigned M08 preparation, and documentation validation all passed. Temporary outputs/checkouts were removed; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M02/ARCH-07 engine-rate bridge on 2026-09-08: the process-loopback probe now converts its explicit 44.1 kHz PCM16 source domain to the fixed 48 kHz planar-f32 engine domain through the bounded phase-preserving `StreamingResampler`. Drain scheduling permits zero or multiple engine quanta per source quantum, preventing bounded FIFO growth from a one-to-one period assumption. Focused engine tests (70), probe check, formatting, and guarded include/exclude live acceptance passed; each live mode captured 12,789 source frames, emitted 13,696 engine frames/107 quanta, rejected zero packets, and preserved the media snapshot. No persistent audio configuration changed.
- Next M02/M03/ARCH-05 task: retain native callback deadline, physical latency, production-driver, signing, installer, and manual UI/accessibility gates as explicit release blockers while advancing only independently testable adapter and control-plane work.

- Completed M02/ARCH-05 native scheduler ownership audit on 2026-09-08: `SharedCapture` and `SharedRender` remain explicit user-mode WASAPI clients, and the process-loopback route remains a diagnostic adapter. Their event wakes and packet timing cannot satisfy the production-style endpoint-owned callback deadline gate. No code path was promoted or relabeled as realtime evidence; production driver ownership, callback timing, signing, installer, hardware, and manual UI gates remain open.
- Next M02/M03/ARCH-05 task: implement and qualify the endpoint-owned native scheduler only after the managed virtual-driver boundary exists; until then, advance portable/control-plane work only where it does not weaken that gate.

- Completed M02/ARCH-05/ENG-03 portable deadline telemetry slice on 2026-09-08: `RealtimeScheduler::process_once_with_deadline` now records missed completed quantum deadlines and saturating total/maximum lateness through allocation-free atomics. A regression confirms deadline accounting does not alter generation output or processing behavior; engine tests (72), strict Clippy, formatting, and diff checks pass. This is callback integration readiness, not native deadline evidence.
- Next M02/M03/ARCH-05 task: connect the deadline API to an endpoint-owned native scheduler callback and measure its period/deadline distribution once the managed driver boundary exists.

- Completed M02/ARCH-05/ENG-03 shared-mode scheduler deadline qualification on 2026-09-08: the authorized guarded adapter and explicitly selected VB-Audio route exercised endpoint-owned `SharedCapture`/`SharedRender` streams through `process_once_with_deadline`; adapter/routed runs processed 120/112 quanta with zero xruns and zero deadline misses/lateness, and media snapshots were unchanged. This is native shared-mode adapter evidence, not managed-driver or production callback compliance.
- Next M02/M03/ARCH-05 task: connect the same deadline boundary to the managed endpoint-owned production scheduler after driver lifecycle exists, then measure period/deadline distributions under the release hardware matrix.

- Completed M02/ARCH-05/ENG-03 bounded deadline-lateness distribution slice on 2026-09-08: scheduler telemetry now retains a fixed 32-bucket lateness histogram, and both guarded adapter acceptance paths validate its shape and sample count against deadline misses. The follow-up 300 ms adapter/route runs reported zero misses and zero lateness with unchanged media state; this remains shared-mode adapter evidence, not production callback compliance.
- Next M02/M03/ARCH-05 task: connect the bounded lateness distribution to the managed endpoint-owned production scheduler after driver lifecycle exists and collect release-hardware p99.9 evidence.

- Completed M02/ARCH-05 deadline-boundary correction on 2026-09-08: an exactly-on-time scheduler completion is no longer classified as a miss; only strictly positive lateness enters the miss counters and histogram. Engine tests (72), strict Clippy, formatting, diff checks, and documentation validation pass.
- Next M02/M03/ARCH-05 task: connect the corrected bounded lateness distribution to the managed endpoint-owned production scheduler after driver lifecycle exists and collect release-hardware p99.9 evidence.

- Completed M02/API-01/ARCH-05 telemetry documentation alignment on 2026-09-08: the API reference now distinguishes probe-only processing/deadline distributions from the control-plane `nativeAdapter: not activated` state. Documentation validation passed with 51 Markdown files and 158 local links; no runtime or machine configuration changed.
- Next M02/M03/ARCH-05 task: connect the corrected bounded lateness distribution to the managed endpoint-owned production scheduler after driver lifecycle exists and collect release-hardware p99.9 evidence.

- Completed M02/ARCH-05 deterministic deadline-boundary regression on 2026-09-08: the zero-lateness path is tested directly, confirming no miss, lateness total/maximum, or histogram sample is recorded for an exactly-on-time completion. Engine tests (73), strict Clippy, formatting, diff checks, and documentation validation pass.
- Next M02/M03/ARCH-05 task: connect the corrected bounded lateness distribution to the managed endpoint-owned production scheduler after driver lifecycle exists and collect release-hardware p99.9 evidence.

- Completed M00-M08/API-01/SEC-12 safe-chain requalification after `d0461a8` on 2026-09-08: native compile and 34-endpoint read-only inventory, disposable pinned SysVAD x64 qualification, full workspace checks, M01/M04/M05/M06/M07 validation, unsigned M08 preparation, and documentation validation (51 Markdown files/158 local links) all passed. Temporary outputs/checkouts were removed; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: connect the bounded lateness distribution to the managed endpoint-owned production scheduler after driver lifecycle exists and collect release-hardware p99.9 evidence.

- Completed M00-M08/API-01/SEC-12 safe-chain requalification after `29951c3` on 2026-09-08: native compile and 34-endpoint read-only inventory, disposable pinned SysVAD x64 qualification, full workspace checks, M01/M04/M05/M06/M07 validation, unsigned M08 preparation, and documentation validation (51 Markdown files/158 local links) all passed. Temporary outputs/checkouts were removed; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: connect the deadline API to the managed endpoint-owned production scheduler after driver lifecycle exists, then measure period/deadline distributions under the release hardware matrix.

- Completed M00-M08/API-01/SEC-12 safe-chain requalification after `801bd50` on 2026-09-08: native compile and 34-endpoint read-only inventory, disposable pinned SysVAD x64 qualification, full workspace tests/Clippy, M01/M04/M05/M06/M07 validation, unsigned M08 preparation, and documentation validation (51 Markdown files/158 local links) all passed. Temporary outputs/checkouts were removed; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: connect the deadline API to an endpoint-owned native scheduler callback and measure its period/deadline distribution once the managed driver boundary exists.

- Completed M00-M08/API-01/SEC-12 safe-chain requalification after `95fab8d` on 2026-09-08: native compile and 34-endpoint read-only inventory, disposable pinned SysVAD x64 qualification with the installed VS/WDK toolchain, full workspace checks, M01/M04/M05/M06/M07 validation, unsigned M08 preparation, and documentation validation (51 Markdown files/158 local links) all passed. Temporary outputs/checkouts were removed; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: connect the bounded percentile telemetry to the managed endpoint-owned production scheduler after driver lifecycle exists and collect release-hardware p99.9 evidence.

- Completed M02/API-01/ARCH-05 bounded percentile extraction on 2026-09-08: fixed histogram telemetry now exposes conservative p99.9 upper bounds, the adapter reports them, and both guarded live acceptance paths validate them against maxima. Engine tests (74), strict Clippy, formatting, tool checking, PowerShell parsing, live adapter/route acceptance, diff checks, and documentation validation pass; media state remained unchanged.
- Next M02/M03/ARCH-05 task: connect the bounded percentile telemetry to the managed endpoint-owned production scheduler after driver lifecycle exists and collect release-hardware p99.9 evidence.

- Completed M02/API-01/ARCH-05 histogram counter-overflow hardening on 2026-09-08: percentile rank extraction now saturates histogram totals, with a regression for a saturated snapshot. Engine tests (75), strict Clippy, formatting, and diff checks pass; no machine audio configuration changed.
- Next M02/M03/ARCH-05 task: connect the hardened bounded percentile telemetry to the managed endpoint-owned production scheduler after driver lifecycle exists and collect release-hardware p99.9 evidence.

- Completed M00-M08/API-01/SEC-12 safe-chain requalification after `0162438` on 2026-09-08: native compile and 34-endpoint read-only inventory, disposable pinned SysVAD x64 qualification, full workspace checks, M01/M04/M05/M06/M07 validation, unsigned M08 preparation, and documentation validation (51 Markdown files/158 local links) all passed. Temporary outputs/checkouts were removed; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: connect the hardened bounded percentile telemetry to the managed endpoint-owned production scheduler after driver lifecycle exists and collect release-hardware p99.9 evidence.

- Completed M00/CAP-04/QUAL-01 digital loopback requalification on 2026-09-08: the guarded existing VB-Audio render-to-capture path produced 209,982 nonzero payload bytes over a 1,000 ms capture window, stopped/reset cleanly, and preserved the media-device snapshot with exact temporary cleanup. This is digital propagation evidence only; physical acoustic latency, managed-driver ownership, signing, and installer gates remain open.
- Next M00/M02/M03/ARCH-05 task: retain the digital loopback result while pursuing physical acoustic latency only with suitable hardware and managed endpoint-owned callback evidence after a production driver exists.

- Completed M00/CAP-01/CAP-02/NFR-01 endpoint lifecycle requalification on 2026-09-08: the current native sweep passed 13 capture and 21 render endpoints for bounded start/stop/reset behavior, correctly recognizing one occupied render endpoint, with unchanged media/configuration state.
- Next M00/M02/M03/ARCH-05 task: retain the lifecycle and digital-loopback evidence while pursuing physical acoustic latency only with suitable hardware and managed endpoint-owned callback evidence after a production driver exists.

- Completed M00/CAP-03/CAP-04/QUAL-01 USB signal-path requalification on 2026-09-08: the current PD200X speaker/microphone pair produced 129,225 nonzero capture bytes through the bounded selected-endpoint path, with exact cleanup and unchanged media/configuration state. Ambient input was not separated, so calibrated acoustic latency remains open.
- Next M00/M02/M03/ARCH-05 task: retain the USB signal-path smoke as physical-path evidence while pursuing calibrated acoustic latency and managed endpoint-owned callback evidence after a production driver exists.

- Completed M00/CAP-03/NFR-01 negative acoustic-correlation attempt on 2026-09-08: the explicitly selected PD200X pair detected 0/1,000 impulse groups, so the acceptance threshold correctly failed; cleanup and unchanged configuration checks passed. The threshold remains intact and calibrated physical latency is still unqualified.
- Next M00/M02/M03/ARCH-05 task: retain the negative acoustic result and pursue calibrated latency only after the physical return path is measurable and the managed endpoint-owned callback exists.

- Completed M00/SEC-12 disposable native-build isolation on 2026-09-08: custom-output probe builds now place implicit objects beside the temporary executable, and compile acceptance asserts repository isolation plus cleanup. Native compile, PowerShell parsing, documentation validation, and diff checks pass; no audio or machine state changed.
- Next M00/M02/M03/ARCH-05 task: retain isolated disposable builds while pursuing calibrated latency only after the physical return path is measurable and the managed endpoint-owned callback exists.

- Completed M07/SEC-12 transport lifecycle hardening on 2026-09-08: the multi-response named-pipe client now rejects zero or over-500 response counts before endpoint/pipe access, preventing an unbounded or non-completing request from holding a session. Native transport tests (18), strict transport Clippy, formatting, and diff checks pass; no audio or machine configuration changed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M08/SEC-12 release-manifest completeness hardening on 2026-09-08: artifact verification and acceptance now require both release executables, the UI archive, cargo/npm SBOMs, the npm lock snapshot, and third-party notices, in addition to checksum/list validation. PowerShell parsing, unsigned release acceptance, and diff checks pass; no signing, installer, driver, audio, or machine configuration changed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Requalified M08/SEC-12 unsigned release preparation after `ca4e8a4` on 2026-09-08: optimized CLI/plugin-worker artifacts, UI production archive, SBOMs, notices, required-artifact manifest validation, and checksum verification all passed in a disposable temporary directory, which was cleaned afterward. No signing, installer, driver, audio, or machine configuration changed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Requalified the M00-M08 safe acceptance chain after `f86f870` on 2026-09-08: native compile and 34-endpoint read-only inventory, disposable pinned SysVAD x64 qualification, workspace checks, M01/M04/M05/M06/M07 validation, unsigned M08 preparation, and documentation validation (51 Markdown files/158 local links) all passed. Temporary outputs/checkouts were removed; driver installation/loading, signing-mode changes, plugin/startup registration, and machine audio configuration were not performed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M03/VDEV-05/VDEV-10/SEC-12 virtual-bus ownership safety on 2026-09-08: disabling a leased virtual bus now fails with `Owned` until the lease is explicitly released, matching the existing deletion guard and preventing an implicit invalidation of an active writer. Domain tests (53), strict Clippy, formatting, and diff checks pass; no driver, endpoint, or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M04/REC-04/SEC-12 recording-checkpoint identity hardening on 2026-09-08: direct storage checkpoint save/load/clear operations now enforce the shared 128-byte recording ID bound, preventing a lower-level caller from bypassing recording identity limits. Storage tests (41), strict Clippy, formatting, and diff checks pass; no audio endpoint or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M04/REC-04/SEC-12 recording metadata persistence hardening on 2026-09-08: direct storage row writes now enforce the shared 256-character, no-control-character title/artist/comment contract, preventing invalid metadata from bypassing the recording library and API validators. Storage tests (42), strict Clippy, formatting, and diff checks pass; no audio endpoint or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/M04/SEC-12 direct session persistence hardening on 2026-09-08: `Storage::save_session`, its atomic journal variant, and graph-plan candidate persistence now validate domain sessions and enforce the existing 1 MiB serialized-document limit before opening a transaction. Storage tests (45), strict Clippy, formatting, and diff checks pass; no audio endpoint or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/SEC-12 direct enrollment persistence hardening on 2026-09-08: SQLite enrollment writes now enforce nonempty bounded client IDs and the supported role set, with stable control error mapping. Storage/control tests (46/86), strict Clippy, formatting, and diff checks pass; no audio endpoint or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/SEC-12 durable plan identity hardening on 2026-09-08: graph, startup, and virtual-device plan writes now enforce the shared 128-byte identity ceiling before SQLite mutation. Storage/control tests (47/86), strict Clippy, formatting, and diff checks pass; no audio endpoint or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M06/SEC-12 plugin-state identity hardening on 2026-09-08: direct SQLite plugin-state writes now enforce the shared 128-byte record-ID ceiling, closing a lower-layer metadata bypass. Storage tests (48), strict Clippy, formatting, and diff checks pass; no plugin was executed and no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M07/SEC-12 durable idempotency-key hardening on 2026-09-08: every SQLite journal read/write path rejects empty or over-128-byte keys, and all public control schemas advertise the same bound. Storage/control tests (49/86), strict Clippy, formatting, and diff checks pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Requalified the guarded M00–M08 safe acceptance chain at `719f0a8` on 2026-09-08 after the storage/API boundary changes: native compile/inventory, disposable SysVAD x64 package/API qualification, portable milestone checks, and documentation validation completed with temporary cleanup. No driver installation/loading, signing-mode change, registration, or machine audio configuration occurred; production driver/signing, installer, physical-latency, and manual UI gates remain open.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/SEC-12 enrollment read-boundary hardening on 2026-09-08: SQLite client-enrollment lookup and revoke now enforce the same nonempty 128-byte identity contract as writes. Storage/control tests (49/86), strict Clippy, formatting, and diff checks pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M04/SEC-12 recording-library identity hardening on 2026-09-08: lookup, paging, metadata, rename, missing-state, and removal storage APIs now enforce the shared 128-byte recording-ID bound. Storage tests (50), strict Clippy, formatting, and diff checks pass; no recording file, audio endpoint, or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M06/SEC-12 plugin-state removal identity hardening on 2026-09-08: plugin-state deletion now enforces the same 128-byte record-ID limit as persistence. Storage tests (50), strict Clippy, formatting, and diff checks pass; no plugin was executed and no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/SEC-12 session read-boundary hardening on 2026-09-08: session lookup, history, export, deletion, and stable-cursor storage APIs now enforce the shared 128-byte identity limit. Storage tests (51), strict Clippy, formatting, and diff checks pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/SEC-12 durable plan read-boundary hardening on 2026-09-08: graph, startup, and virtual-device plan load/delete APIs now enforce the shared 128-byte identity bound and revalidate loaded IDs. Storage tests (51), strict Clippy, formatting, and diff checks pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/SEC-12 session pagination hardening on 2026-09-08: SQLite history and session-list APIs now enforce bounded limits of 101 look-ahead history rows and 500 session-page rows. Storage/control tests (52/86), strict Clippy, formatting, and diff checks pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Requalified M02/ARCH-05 shared-mode Rust adapter behavior on 2026-09-08: the guarded 300 ms live smoke processed 15,360 capture/scheduler frames and 15,456 silent render frames with zero xruns/deadline misses and a 32,768 ns processing p99.9 upper bound; streams stopped/reset and media-device state was unchanged. This is not managed-driver callback or physical-latency evidence.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Requalified M02/ARCH-05 shared-mode routed Rust adapter behavior on 2026-09-08: the guarded 300 ms run routed 13,824 frames from the explicitly selected VB-Audio pair with 112 graph blocks, a 32,768 ns processing p99.9 upper bound, and zero xruns/deadline misses/lateness; all stream and configuration rollback checks passed. This is not managed-driver callback or physical-latency evidence.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M00/SEC-12 acceptance-wrapper cleanup propagation on 2026-09-08: all six custom-output native live wrappers now remove adjacent implicit objects, and the 21-script parse check plus bounded VB-Audio loopback passed with no temporary or repository artifacts left behind. No audio configuration changed.
- Next M00/M02/M03/ARCH-05 task: retain isolated wrapper cleanup while pursuing calibrated latency only after the physical return path is measurable and the managed endpoint-owned callback exists.

- Completed M00/CAP-06/CAP-07/SEC-12 process-wrapper requalification on 2026-09-08: guarded attribution and exclusion runs passed with 10,584/11,025 captured frames, full temporary-object cleanup, and unchanged persistent audio configuration. This remains controlled process-loopback evidence, not full isolation or PID-reuse evidence.
- Next M00/M02/M03/ARCH-05 task: retain the process evidence while pursuing calibrated latency and managed endpoint-owned callback evidence after the production driver boundary exists.

- Completed M00/CAP-01/CAP-02/ARCH-07 event-wrapper requalification on 2026-09-08: selected VB-Audio endpoints completed the bounded event lifecycle with 12,480 capture and 16,320 silent render frames; temporary and repository objects were absent afterward and settings were unchanged.
- Next M00/M02/M03/ARCH-05 task: retain event lifecycle evidence while pursuing calibrated latency and managed endpoint-owned callback evidence after the production driver boundary exists.

- Completed M00/CAP-05/QUAL-01 impulse-wrapper requalification on 2026-09-08: the 50-impulse VB-Audio run detected 45 groups with zero p95 spacing error and an estimated 82.54 ms digital onset; raw/log/object cleanup and unchanged configuration checks passed. This remains digital correlation evidence, not calibrated acoustic latency.
- Next M00/M02/M03/ARCH-05 task: retain digital timing evidence while pursuing calibrated latency and managed endpoint-owned callback evidence after the production driver boundary exists.

- Completed M02/API-01/ARCH-05 route-telemetry reporting alignment on 2026-09-08: the guarded 300 ms route acceptance now prints the validated processing and deadline-lateness p99.9 upper bounds in its summary. It passed with 14,400 captured and 13,856 routed frames, 32,768 ns processing p99.9 bound, zero deadline misses, and unchanged endpoint/configuration state.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M00-M08 workspace regression at `8335d9a` on 2026-09-08: `cargo test --workspace`, strict workspace Clippy, formatting/diff checks, and documentation validation passed. Windows-audio tests exercised read-only identity/lifecycle behavior; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M00/M08 disposable-artifact audit on 2026-09-08: two stale, named SysVAD reference checkouts were found in the user temp directory, verified to be outside the repository and not installed drivers, and removed. The repository probe directory contains only checked-in source/build scripts; no audio or machine configuration changed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/API-01/SEC-12 public identifier schema alignment on 2026-09-08: control discovery now advertises authoritative 128-byte entity bounds for session, plan, node, recorder, event, and cursor inputs, plus the durable idempotency-key bound for operation lookup/cancel. Control/storage tests (86/52), strict Clippy, formatting, and documentation validation pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/API-01/SEC-12 public output identifier schema alignment on 2026-09-08: returned plan IDs, operation IDs, and session-list cursors now advertise the same authoritative bounds as persistence; discovery regressions cover representative response method groups. Control/storage tests (86/52), strict Clippy, formatting, diff validation, and documentation validation pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/API-01/SEC-12 nested event-snapshot cursor alignment on 2026-09-08: resynchronization snapshots now bound their nested session cursor consistently with the top-level session-list contract, with discovery regression coverage. Control/storage tests (86/52), strict Clippy, formatting, diff validation, and documentation validation pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/API-01/SEC-12 graph-history cursor boundary on 2026-09-08: decimal `u64` revision cursors now reject values above the exact 20-byte representation limit before parsing, and input/output discovery plus `system.describe` expose the same bound. Control/storage tests (86/52), strict Clippy, formatting, diff validation, and documentation validation pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M01/API-01 discovery self-consistency regression on 2026-09-08: required top-level, limits, and event fields declared by the `system.describe` output schema are now checked against the actual response, preventing future self-schema drift. Control/storage tests (86/52), strict Clippy, formatting, diff validation, and documentation validation pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Corrected M01/API-01/SEC-12 discovery self-schema for the graph-history cursor boundary on 2026-09-08: `system.describe` now declares `maxRevisionCursorBytes` in both its limits properties and required list, matching the returned payload. Focused control/storage tests, strict Clippy, formatting, diff checks, and documentation validation pass; no audio or machine configuration was accessed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M00-M08 workspace regression at `03c58d9` on 2026-09-08: locked workspace tests, strict workspace Clippy, formatting/diff checks, and documentation acceptance passed after plugin-state read-boundary hardening. No driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M00-M08 safe acceptance at `09494d7` on 2026-09-08: elevated native compile, read-only 34-endpoint format inventory, disposable pinned SysVAD x64 qualification, M01/M04/M05/M06/M07 acceptance, unsigned M08 preparation, and documentation validation passed. Temporary outputs/checkouts were removed; no driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M02/ARCH-05/ENG-03 live adapter requalification at `acf1454` on 2026-09-08: the bounded shared-mode smoke processed 24,480 capture frames through 191 graph blocks and 24,448 scheduler frames, with zero xruns/overruns or deadline misses and a 65,536 ns processing p99.9 upper bound. Streams stopped/reset and the media snapshot was unchanged. This is shared-mode adapter evidence, not managed-driver callback qualification.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Completed M02/CAP-01/CAP-02/ENG-03/ARCH-05 routed adapter requalification at `a4b72ec` on 2026-09-08: the bounded VB-Audio route processed 24,000 captured and 23,936 routed frames across 187 graph blocks, with zero xruns, deadline misses, and deadline lateness and a 32,768 ns processing p99.9 upper bound. Streams stopped/reset; endpoint and configuration snapshots were unchanged. This is shared-mode diagnostic evidence, not managed-driver callback qualification.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Requalified M06/SEC-07 offline VST3 SDK/loader acceptance at `656fdb3` on 2026-09-08: the local pinned SDK passed 51 self-tests and 1,598 validator tests; the x64 loader discovered 68 classes and verified finite stereo processing, five parameters/automation, and a 180-byte state round trip. No plugin was globally registered or external plugin executed; OS sandbox, multi-vendor compatibility, and native production execution remain open.
- Next M06 task: obtain and qualify the required independent x64 VST3 fixture matrix and worker/editor containment evidence without weakening the plugin sandbox boundary.

- Requalified the locked workspace at `b571e24` on 2026-09-08 after journal hardening: all workspace unit/integration tests and doc-tests passed, and strict workspace Clippy passed with `-D warnings`. No driver, signing mode, plugin/startup registration, or machine audio configuration changed.
- Next M02/M03/ARCH-05 task: connect the complete bounded telemetry report to the managed endpoint-owned production scheduler after driver lifecycle exists.

- Requalified the complete locked workspace after the native VST2 invalid-output, layout, crash, hang, legacy-entry-point, state, and editor-containment changes on 2026-09-08: `cargo test --workspace --locked` passed all workspace unit/integration tests and doc-tests; strict workspace Clippy passed with `-D warnings`; formatting, diff checks, and documentation validation passed (51 Markdown files, 160 local links). This is portable/repository evidence plus previously recorded opt-in native worker probes; it does not close the native editor authorization, independent-rights, production-driver, signing, installer, or clean-machine gates. No driver, plugin registration, audio stream, or machine audio configuration was changed.
- Next M06 task: integrate an explicitly authorized control-plane/UI parent-window token with the worker-owned editor path, or record that integration as externally blocked; continue VST2 rights and independent-fixture qualification before any user-facing release claim.

- Completed a bounded M06/PLUG-01/PLUG-02/PLUG-03/PLUG-05 compatibility probe on 2026-09-08 using the explicitly selected installed `pitchproof-x64.dll` at `C:\\Program Files\\Common Files\\VST3\\Pitchproof\\pitchproof-x64.dll`. Read-only inspection classified the 1,077,760-byte binary as x64 VST2 (`SHA-256` `1974a3033b53ae72da5f419a9f37056d44c1610591bfd11a615ace0c448cf050`), and the disposable worker load/process acceptance passed. This is independent installed-binary evidence for bounded processing only; rights, state/editor/latency matrix coverage, and release qualification remain open. The test used the original file in place, restored `AUDIOROUTER_VST2_FIXTURE`, and changed no audio or machine configuration. The repeatable wrapper is `tests/acceptance/m06-vst2-installed.ps1`.
- Next M06 task: use additional independent installed fixtures when the user provides them, while implementing only an explicitly authorized native UI parent-window integration and retaining the VST2 release gate.

- Completed an M06/PLUG-01 negative-control inspection on 2026-09-08: the sibling installed `pitchproof.dll` was rejected read-only as `unsupportedArchitecture` (x86), while no plugin code was loaded. Together with the recorded x64 `pitchproof-x64.dll` worker pass, this confirms the current VST2 boundary does not bridge or execute x86 binaries. No audio or machine configuration changed.
- Next M06 task: retain the x86 rejection as compatibility evidence and continue only with x64 fixtures and the explicitly authorized native-editor integration prerequisite.

- Completed the installed Pitchproof native-editor containment probe on 2026-09-08: the dedicated Windows editor thread and supervised worker tests both passed their five-second bounds, with the editor failing closed and the worker terminated/reaped. The plugin did not return from `effEditOpen`, so this is not successful editor-window compatibility; it confirms the same third-party editor limitation seen in ReaPlugs without hanging the host. The installed-fixture wrapper now repeats processing plus both editor checks and restores `AUDIOROUTER_VST2_FIXTURE`.
- Next M06 task: keep native editor controls gated until an authorized UI parent-window path and a compatible editor fixture are available; continue rights and x64 compatibility qualification.

- Completed M00-M08 guarded safe acceptance on 2026-09-08 after the installed VST2 editor-containment update: VS/WDK discovery, native compile, read-only 34-endpoint inventory, disposable SysVAD x64 package/API validation, M01/M04/M05 (91 UI tests), M06 SDK/VST3, M07 headless, unsigned M08 artifacts, 159 normative mappings, and documentation validation (51 files, 161 links) passed. Temporary outputs/checkouts were removed; no driver, signing mode, registration, stream, or machine audio configuration changed. Production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M06 task: keep native editor controls gated until the control-plane/UI parent-window authorization and an editor-compatible fixture are available; continue the VST2 rights and x64 compatibility matrix.

- Implemented the bounded M06/PLUG-04/SEC-05/SEC-07 editor-parent authorization boundary: `EditorOpen` now carries an opaque bounded token and expected owner PID, the worker validates the wire fields, and the Windows editor thread verifies the live HWND owner before calling native VST2 code. `EditorParentAuthorizationIssuer` derives the token from a control-plane-held key, HWND, and owner PID; portable tests cover malformed authorization, issuer binding, and wire round-trip. The native editor acceptance rejects a mismatched owner PID before exercising the plugin, then confirms the existing five-second timeout containment. Native-shell HWND ownership and authenticated key storage/issuance are still not integrated, so editor controls remain gated.
- Next M06 task: connect the issuer to the authenticated control plane/native shell once a real HWND owner exists; do not expose raw parent handles from the preview WebView.

- Requalified M00-M08 portable consumers after the editor authorization wire change on 2026-09-08: `cargo test --workspace --locked` passed all workspace unit/integration tests and doc-tests, including plugin-host (56 unit, 13 ordinary worker-process tests); strict workspace Clippy, formatting, documentation validation (51 files, 161 links), and diff checks also passed. The installed Pitchproof wrapper passed processing, owner-mismatch rejection, editor timeout containment, and supervised worker reaping. No driver, plugin registration, audio stream, or machine audio configuration changed.
- Next M06 task: implement the authenticated control-plane/native-shell token issuer and real HWND ownership path when the native shell exists; retain the current fail-closed editor gate until then.

- Requalified the guarded M00-M08 `safe-all.ps1` chain at `1e1ba0cb` on 2026-09-08 after the editor authorization change: toolchain/native compile, read-only 34-endpoint inventory, disposable SysVAD x64 package/API checks, M01/M04/M05 (91 UI tests), M06 SDK/VST3, M07 headless, unsigned M08 artifacts, 159 traceability mappings, and documentation validation (51 files, 161 links) passed. Temporary outputs/checkouts were removed; no driver, signing mode, registration, stream, or machine audio configuration changed. Production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M06 task: implement the authenticated native-shell/control-plane token issuer and real HWND ownership path when available; keep the preview WebView unable to expose raw editor parent handles.

- Completed the portable portion of M06/PLUG-04/SEC-05/SEC-07 on 2026-09-08: `EditorParentAuthorizationIssuer` now creates a bounded authorization from the control-plane-held key, parent HWND, and expected owner PID; malformed authorization and wire round-trip tests pass. The locked workspace regression passed all unit/integration/doc tests and strict Clippy; the installed Pitchproof processing/editor-containment wrapper also passed. This establishes the issuer seam only: authenticated key storage, native-shell HWND creation, and control-plane transport integration remain open. No driver, plugin registration, audio stream, or machine audio configuration changed.
- Next M06 task: integrate the issuer with the authenticated control plane and a real native-shell HWND owner when that shell exists; retain the fail-closed editor gate and do not expose raw WebView handles.

- Requalified the guarded M00-M08 `safe-all.ps1` chain after commit `7a7aa423` on 2026-09-08: VS/WDK discovery, native compile, read-only 34-endpoint inventory, disposable SysVAD x64 package/API qualification, M01/M04/M05, repository-local VST3 SDK, M07 headless, unsigned M08 artifacts, 159 traceability mappings, and documentation validation (51 files, 161 links) passed. Temporary checkouts/builds were removed; no driver, signing mode, plugin registration, startup registration, stream, or machine audio configuration changed. Production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Expanded and requalified guarded `safe-all.ps1` at the clean commit on 2026-09-08: the repository-owned VST2 fixture stage now builds and verifies both `VSTPluginMain` and legacy `main`, chunk-state restoration, non-finite output rejection, crash containment, and hang reaping. The complete chain passed through M08 release preparation, traceability, and documentation validation. The fixture wrapper restored `AUDIOROUTER_VST2_FIXTURE`; generated DLLs remain ignored local test artifacts. No driver, signing mode, plugin registration, stream, or machine audio configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Corrected the M05 plugin-scan UI wording on 2026-09-08: the discovery panel now names both VST3 and gated VST2 support and its empty state includes all DLL candidates, matching the backend identity contract without implying execution or release qualification. UI typecheck and all 91 Vitest tests passed; no audio or machine configuration was accessed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Synchronized the M06/PLUG-07 delivery documentation on 2026-09-08: the milestone, delivery risk register, and development release notes now distinguish the verified Windows x64 VST2 worker/ABI fixture boundary from release qualification, rights, editor, x86, redistribution, and third-party compatibility gates. Documentation validation remains required; no code, plugin registration, audio stream, or machine configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Completed a read-only Windows plugin-location inventory on 2026-09-08: no additional VST3 binaries were found in the standard machine/user locations checked. The available candidates remain the six copied local ReaPlugs VST2 effects plus installed Pitchproof x64 VST2; its x86 sibling remains rejected. No binary was copied, registered, loaded, or executed by this inventory, and no audio or machine configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Requalified the Windows identity/lifecycle regression on 2026-09-08 with `cargo test -p audiorouter-windows-audio --locked -- --nocapture`: 30 unit tests and doc-tests passed, including observed application identity binding, stale-process rejection, read-only inventory, endpoint lifecycle, and the narrowly scoped `E_INVALIDARG` retry policy. No audio stream or machine configuration changed.
- Next M00/M02 task: perform calibrated physical latency and an actual documented PID-reuse observation only when the required timestamped physical setup/process condition exists; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.

- Requalified the complete guarded M00–M08 `safe-all.ps1` chain at pushed commit `d253bb47` on 2026-09-08: VS/WDK discovery and native compile, read-only 31-endpoint format inventory, disposable SysVAD x64 package/API qualification, M01/M04/M05, pinned VST3 SDK and validator, native VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159 traceability mappings, and documentation validation (51 files/161 local links) all passed. Temporary outputs/checkouts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred. Production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Requalified the longer guarded native impulse correlation on 2026-09-08: `m00-native-impulse.ps1 -AllowLiveAudio -ImpulseCount 1000` detected 998 of 1,000 groups across the explicitly named VB-Audio pair, with p95 spacing error of 0 frames and estimated onset of 52.88 ms. Stream lifecycle, temporary cleanup, and media-state preservation passed. This is stronger digital correlation evidence only; the onset is not calibrated acoustic latency and does not close managed-driver or physical-latency gates.
- Next M00/M02 task: perform calibrated physical latency only with a validated timestamped physical setup; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.
- Requalified the hardened impulse harness at 1,000 impulses on 2026-09-08: all 1,000 groups were detected across the explicitly named VB-Audio pair, with zero p95 spacing error, estimated onset 59.63 ms, and successful exit codes from both directly owned native children. Stream teardown, temporary cleanup, and media-state preservation passed. This remains digital correlation only; calibrated acoustic latency and managed-driver gates remain open.
- Next M00/M02 task: perform calibrated physical latency only with a validated timestamped physical setup; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.
- Hardened and requalified the M00 endpoint-loopback harness on 2026-09-08: concurrent capture/tone children are now directly owned `.NET Process` instances with reliable exit-code checks, bounded output collection, and kill/reap cleanup. The 500 ms capture/800 ms tone run passed with 75,432 nonzero payload bytes; stream teardown and media-state preservation passed, with no persistent audio configuration change.
- Next M00/M02 task: perform calibrated physical latency only with a validated timestamped physical setup; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.
- Fixed the M00 impulse harness process-ownership gap on 2026-09-08: redirected capture and impulse children are now launched through directly owned `.NET Process` instances, preserving reliable exit codes, bounded stdout/stderr collection, and kill/reap cleanup on failure. The live 100-impulse requalification passed with 95 detected groups and zero p95 spacing error; media state and audio configuration remained unchanged.
- Next M00/M02 task: perform calibrated physical latency only with a validated timestamped physical setup; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.
- Investigated an M00 impulse-wrapper hardening experiment on 2026-09-08: adding redirected-child exit-code assertions exposed a PowerShell `Start-Process` handle limitation on this host (`ExitCode` was blank, then the .NET property reported that the process was not started by the object). The experiment was fully reverted; the existing lifecycle/payload/media-state checks remain unchanged and no unvalidated harness behavior was committed. A future exit-code check must use a separately validated process-launch primitive.
- Next M00/M02 task: perform calibrated physical latency only with a validated timestamped physical setup; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.
- Requalified the optional native VST2 matrix at pushed head `2905572a` on 2026-09-08: all six ignored local ReaPlugs effects passed verified worker processing at 44.1, 48, and 96 kHz (18 combinations), and the installed Pitchproof x64 VST2 binary passed the same three rates plus dedicated and supervised editor-containment tests. Its SHA-256 remained `1974a3033b53ae72da5f419a9f37056d44c1610591bfd11a615ace0c448cf050`; environment variables were restored, with no copy, registration, stream, or machine configuration change. This remains compatibility evidence and does not close rights, editor success, or release gates.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Closed an M02/M04/DSP-08 observability gap on 2026-09-08: every prepared `RuntimeGraph` now retains the bounded sample rate used to initialize its stateful stages, while the compatibility constructor remains explicitly 48 kHz. `RuntimeProcessor::active_sample_rate_hz` exposes the immutable published value to control/diagnostics callers without callback synchronization. Engine tests (81), strict engine Clippy, formatting, and diff checks passed; no audio endpoint or machine configuration was accessed.
- Next M02/M03 task: connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists; retain the native-driver blocker and portable 48 kHz compatibility default.
- Added fresh M00/M02 digital-correlation evidence on 2026-09-08: guarded `m00-native-impulse.ps1 -AllowLiveAudio -ImpulseCount 100` detected 97 of 100 impulses across the explicitly named VB-Audio virtual-cable render/capture pair, with p95 spacing error of 0 frames and estimated onset of 72.21 ms. Stream teardown, temporary cleanup, and media-state preservation passed. This is bounded signal correlation only; it is not calibrated acoustic p95 latency or managed-driver callback evidence.
- Next M00/M02 task: perform calibrated physical latency only with a validated timestamped physical setup; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.
- Requalified guarded M00 native lifecycle and attribution evidence on 2026-09-08: `m00-native-live.ps1 -AllowLiveAudio -DurationMilliseconds 100` passed all 13 capture and 18 render endpoints (one occupied render was correctly reported); `m00-native-event-live.ps1 -AllowLiveAudio -DurationMilliseconds 200` passed the selected VB-Audio event-driven pair with 10,080 capture and 14,400 render frames; and `m00-native-process-live.ps1 -AllowLiveAudio -DurationMilliseconds 500` passed controlled process-tree attribution with 21,609 capture frames and 77,823 nonzero bytes. All wrappers stopped/reset streams and verified unchanged media state; no defaults, volume, mute, privacy, driver, signing, startup, or other persistent audio configuration changed. These results do not close managed-driver, physical-latency, or actual PID-reuse gates.
- Next M00/M02 task: perform calibrated physical latency and an actual documented PID-reuse observation only when the required timestamped physical setup/process condition exists; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.
- Hardened and requalified the M02 Rust adapter live wrapper on 2026-09-08: it now requires capture/render rates, the 128-frame quantum, and a deadline equal to the ceiling of `quantum_frames * 1e9 / capture_rate_hz`. The 250 ms live smoke passed at 48 kHz capture/render with a 2,666,667 ns deadline, 12,480 capture frames, 97 graph blocks, and zero deadline misses/XRuns/queue overruns. Streams stopped/reset and media state was unchanged; this remains shared-mode adapter evidence, not managed-driver or physical-latency evidence.
- Next M02/M03 task: connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists; retain the native-driver and physical-latency blockers.
- Extended the M02 scheduler integration seam on 2026-09-08: `RealtimeScheduler::active_sample_rate_hz` now exposes the immutable negotiated rate directly to a future endpoint adapter, with regressions for the unactivated, active 44.1 kHz, and deactivated states. Engine tests (81), strict engine Clippy, formatting, and diff checks passed; no audio endpoint or machine configuration was accessed.
- Next M02/M03 task: connect scheduler activation and rate metadata to the managed endpoint-owned callback after the production driver boundary exists; retain the native-driver and physical-latency blockers.
- Requalified the complete guarded M00–M08 `safe-all.ps1` chain at pushed head `3e679a2f` on 2026-09-08: VS/WDK discovery and native compile, read-only 31-endpoint format inventory, disposable SysVAD x64 qualification, M01/M04/M05 (91 UI tests), pinned VST3 SDK/matrix, modern/legacy/fault VST2 fixtures, M07, unsigned M08 artifacts, 159 traceability mappings, and documentation validation (51 Markdown files/161 local links) passed with exit code 0. Temporary outputs/checkouts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred. Production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M02/M03 task: connect scheduler activation and rate metadata to the managed endpoint-owned callback after the production driver boundary exists; retain the native-driver and physical-latency blockers.
- Requalified the hardened differing-rate M02 route on 2026-09-08 at 250 ms using the documented 96 kHz capture to 48 kHz render pair: 23,040 capture frames, 90 graph blocks, 11,520 scheduler frames, and 11,424 routed frames. The wrapper verified a 1,333,334 ns deadline, 65,536 ns p99.9 processing bound, zero deadline misses/lateness, and zero histogram/accounting faults; teardown and media-state rollback passed. This remains shared-mode adapter evidence, not managed-driver callback, independent-clock, or physical-latency qualification.
- Next M02/M03 task: connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists; retain the native-driver and physical-latency blockers.
- Requalified the guarded Rust process-loopback adapter on 2026-09-08 at 250 ms: include mode converted 10,584 source frames to 11,392 engine frames across 89 scheduler blocks; exclude mode converted 11,025 source frames to 11,904 engine frames across 93 blocks. Both 44.1 kHz-to-48 kHz runs reported zero rejected packets, XRuns, input/output overruns, and underruns, with media state unchanged after teardown. This remains asynchronous process-loopback evidence, not managed-driver routing or physical-latency evidence.
- Next M00/M02 task: perform calibrated physical latency and an actual documented PID-reuse observation only when the required timestamped physical setup/process condition exists; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.
- Requalified the read-only M00 native format inventory on 2026-09-08: 31 endpoints passed activation/GetMixFormat inspection, exposing 48 kHz 32-bit extensible mono/stereo, 96 kHz 32-bit mono, and 96 kHz 32-bit eight-channel formats. Temporary executable cleanup and media-state comparison passed; no audio stream, driver, default, or machine configuration action occurred. This confirms endpoint metadata only and does not claim production routing or arbitrary format support.
- Next M00/M02 task: perform calibrated physical latency and an actual documented PID-reuse observation only when the required timestamped physical setup/process condition exists; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.
- Requalified guarded endpoint-loopback and exclusion evidence on 2026-09-08: `m00-native-loopback.ps1 -AllowLiveAudio -CaptureDurationMilliseconds 500 -ToneDurationMilliseconds 800` passed on the explicitly named VB-Audio pair with 72,054 nonzero payload bytes, and `m00-native-process-exclude-live.ps1 -AllowLiveAudio -DurationMilliseconds 500` passed with 22,050 capture frames and the disposable child excluded. Streams were cleaned up and media state remained unchanged. These checks do not establish full cross-process isolation, calibrated physical latency, or managed-driver behavior.
- Next M00/M02 task: perform calibrated physical latency and an actual documented PID-reuse observation only when the required timestamped physical setup/process condition exists; otherwise connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists.
- Requalified downstream consumers after `d25cdf3d` on 2026-09-08: `cargo test --workspace --locked` passed all workspace unit/integration tests and doc-tests, strict workspace Clippy passed with `-D warnings`, and formatting/diff checks passed. The graph metadata addition preserved existing compatibility constructors and introduced no endpoint, driver, plugin, or machine-configuration side effects.
- Next M02/M03 task: connect the rate-aware scheduler metadata and activation seam to the managed endpoint-owned callback after the production driver boundary exists; retain the native-driver blocker and portable 48 kHz compatibility default.
- Requalified the guarded M00–M08 `safe-all.ps1` chain at pushed head `fe7ba8eb` on 2026-09-08: VS/WDK discovery and native compile, read-only endpoint inventory, disposable SysVAD x64 qualification, M01/M04/M05, pinned VST3 checks, modern/legacy/fault VST2 fixtures, M07, unsigned M08 artifacts, 159 traceability mappings, and documentation validation (51 Markdown files/161 local links) all passed. Temporary outputs/checkouts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred. Production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Performed a read-only installed-plugin inventory on 2026-09-08 while investigating the remaining M06 independent-fixture gate: the available paths contained the already-qualified x64/x86 Pitchproof pair and ReaPlugs VST2 DLLs, but no additional x64 VST3 binary or second-vendor fixture. No plugin was loaded, copied, registered, or executed, and no machine configuration changed; the second-vendor VST3 gate remains externally dependent on a supplied fixture.
- Next M06 task: qualify a supplied rights-cleared second-vendor x64 VST3 fixture (or an additional rights-cleared VST2 fixture) through the existing worker matrix; otherwise integrate the authenticated native-shell HWND owner when available.
- Fixed M02/M04/DSP-08 sample-rate propagation on 2026-09-08: `compile_session_at_sample_rate` and `RuntimeProcessor::activate_session_at_sample_rate` now initialize parametric/graphic EQ, compressor, gate, delay, and pitch stages from the negotiated bounded 8–192 kHz rate instead of silently hard-coding 48 kHz. The existing compile API preserves its 48 kHz compatibility default. Engine tests (80), strict engine Clippy, formatting, and diff checks passed; no audio device or machine configuration was accessed.
- Next M02/M04 task: route the new endpoint-rate graph activation API through the managed native scheduler when that production boundary exists; retain the portable 48 kHz compatibility default and native-driver blocker.
- Extended the portable M02 scheduler seam on 2026-09-08: `RealtimeScheduler::activate_session_at_sample_rate` now forwards negotiated-rate graph preparation and recycles rings only after successful publication, preserving the prior generation on invalid-rate failure. Engine tests (81), strict engine Clippy, formatting, and diff checks passed; native endpoint ownership and driver lifecycle remain open.
- Next M02/M03 task: connect this scheduler activation seam to the managed endpoint-owned callback after the production driver boundary exists; retain the portable rate validation and fail-closed replacement behavior.
- Fixed M02/ARCH-05 adapter deadline-rate propagation on 2026-09-08: the native `adapter-route` probe now derives its 128-frame scheduler deadline from the selected capture endpoint's negotiated sample rate instead of a hard-coded 48 kHz period. The standalone probe tests (4) and strict Clippy passed; the live route was not run in this slice, and no machine audio configuration changed.
- Next M02/M03 task: requalify the guarded adapter-route at a selected non-48 kHz endpoint when available, then connect the rate-aware scheduler to the managed endpoint-owned callback after driver lifecycle exists.
- Requalified the corrected M02/ARCH-05 rate-aware adapter route on 2026-09-08 using the documented 96 kHz mono capture to 48 kHz stereo render pair: 48,000 capture frames, 187 graph blocks, 23,936 scheduler frames, and 23,904 routed frames; processing p99.9 was 32,768 ns with zero deadline misses/lateness and zero scheduler overruns/XRuns. Stream teardown, media-state comparison, and temporary cleanup passed. This remains shared-mode evidence, not managed-driver callback, independent-clock, or physical-latency qualification.
- Next M02/M03 task: connect the rate-aware scheduler to the managed endpoint-owned callback after driver lifecycle exists; retain the existing guarded differing-rate route as repeatable user-mode evidence.
- Hardened M02/API-01 route evidence reporting on 2026-09-08: the adapter summary now exposes capture/render rates, the 128-frame quantum, and computed graph deadline; the guarded wrapper parses and verifies those fields. The authorized 500 ms 96 kHz-to-48 kHz route passed with a 1,333,334 ns deadline, 48,000 capture frames, 23,936 scheduler/routed frames, zero deadline misses/lateness, and 16,384 ns processing p99.9. State rollback and cleanup passed; no persistent audio configuration changed.
- Next M02/M03 task: connect the rate-aware scheduler to the managed endpoint-owned callback after driver lifecycle exists; retain the guarded runtime rate/deadline assertions.
- Corrected the future-plan index on 2026-09-08: the VST2/ReaPlugs row now describes the implemented gated M06 x64 audio-effect boundary and its remaining rights, editor, independent-coverage, maintenance, and release gates, rather than listing legacy hosting as unstarted future work. Documentation validation remains required; no machine configuration changed.
- Next M06 task: qualify a supplied rights-cleared second-vendor x64 VST3 fixture (or an additional rights-cleared VST2 fixture) through the existing worker matrix; otherwise integrate the authenticated native-shell HWND owner when available.
- Corrected the documentation index on 2026-09-08 so its compatibility-snapshot description includes the gated x64 VST2 observations alongside the VST3 fixture and remaining limits. This aligns the user-facing map with the M06/PLUG-07 evidence; docs validation remains the verification gate and no machine configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Requalified the complete guarded M00–M08 `safe-all.ps1` chain at pushed commit `8ae4e4d7` on 2026-09-08 with the installed VS/WDK toolchain: read-only 34-endpoint inventory, disposable SysVAD x64 package/API qualification, M01/M04/M05, pinned VST3 SDK and 1,598 validator tests, native VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159 traceability mappings, and documentation validation (51 files/161 local links) passed. Temporary outputs/checkouts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred. Production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Synchronized M06/PLUG-03/PLUG-07 compatibility documentation on 2026-09-08: the guide now records the exact observed VST2 processing matrix (six local ReaPlugs effects plus installed Pitchproof at 44.1, 48, and 96 kHz) and explicitly limits that evidence to compatibility observations, excluding rights, editor, and release qualification. Documentation validation passed (51 files/161 local links); no machine configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Requalified ENG-01 contract parity on 2026-09-08 after the VST2 UI wording change: contracts typecheck and the drift checker passed with 61 methods, 17 node kinds, and 7 processors matching the UI/CLI catalogs. The plugin path placeholder now names both `.vst3` and `.dll` candidates. No audio or machine configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Audited the M06/PLUG-04 VST2 editor lifecycle on 2026-09-08: open and close commands both use the bounded worker response deadline; the dedicated UI thread does not join during drop, and a third-party call that does not return remains contained within the disposable worker that the supervisor can reap. No lifecycle defect was found and no native editor call was added. This does not qualify editor-window compatibility.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Completed M06/PLUG-07 compatibility hardening on 2026-09-08: each loaded VST2 instance now gives the legacy host callback its negotiated sample rate and block size, keeping host queries consistent with `effSetSampleRate` and `effSetBlockSize` instead of always reporting 48 kHz/128 frames. A Windows regression covers the callback context; plugin-host tests (57), worker-process tests (13), strict Clippy, formatting, and diff checks pass. No plugin registration, audio stream, or machine configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Requalified the workspace after `2f63fb3a` on 2026-09-08: `cargo test --workspace --locked` passed all workspace unit/integration tests and doc-tests, and strict workspace Clippy passed with `-D warnings`. The VST2 negotiated-format regression remains covered; no driver, plugin registration, audio stream, or machine audio configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Requalified the native M06 VST2 fixture path after `2f63fb3a` on 2026-09-08: the repository-owned x64 DLLs passed modern `VSTPluginMain` and legacy `main` load/process and chunk-state checks; non-finite output was rejected, and crash/hang fixtures were contained and reaped. The wrapper restored `AUDIOROUTER_VST2_FIXTURE` and reported no plugin registration or audio/configuration changes. This remains repository-fixture evidence, not independent rights or release qualification.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Implemented M06/PLUG-03 format propagation on 2026-09-08: supervised and direct worker launches now offer bounded 8–192 kHz sample-rate selection, pass it explicitly as `--sample-rate`, and use it for VST2 processing setup; existing launch APIs retain the 48 kHz default. The opt-in native VST2 worker acceptance now exercises 44.1 kHz, while malformed rates fail before process creation. Portable plugin-host tests (57), worker-process tests (13), strict Clippy, formatting, and diff checks pass; no audio or machine configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Requalified the complete workspace after the sample-rate propagation changes on 2026-09-08: `cargo test --workspace --locked` passed all workspace unit/integration tests and doc-tests, and strict workspace Clippy passed with `-D warnings`. No driver, plugin registration, audio stream, or machine audio configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Requalified the complete guarded M00–M08 `safe-all.ps1` chain after `36ed1d7f` on 2026-09-08: elevated read-only native inventory (34 endpoints), disposable SysVAD x64 qualification, M01/M04/M05, pinned VST3 SDK, native VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifact verification, 159 traceability mappings, and documentation validation (51 files/161 local links) all passed. Temporary outputs/checkouts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Requalified the six ignored local ReaPlugs VST2 fixtures at the explicit 44.1 kHz worker format on 2026-09-08: ReaComp, ReaDelay, ReaEQ, ReaFIR, ReaGate, and ReaXcomp all passed bounded load/process acceptance through disposable workers. The wrapper restored `AUDIOROUTER_VST2_FIXTURE`; no plugin registration, audio stream, or machine configuration changed. These are local compatibility fixtures only and do not resolve rights or release qualification.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Requalified the explicitly selected installed Pitchproof x64 VST2 binary at the explicit 44.1 kHz worker format on 2026-09-08: processing passed, the dedicated editor thread bounded the non-returning native editor call, and the supervised worker was terminated/reaped within its deadline. SHA-256 remained `1974a3033b53ae72da5f419a9f37056d44c1610591bfd11a615ace0c448cf050`; the original file was used in place, with no copy, registration, audio stream, or machine configuration change. Editor compatibility remains unqualified and rights remain unresolved.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.

- Requalified the complete guarded M00–M08 `safe-all.ps1` chain at the current pushed head on 2026-09-08: elevated read-only native inventory (34 endpoints), disposable SysVAD x64 qualification, M01/M04/M05, pinned VST3 SDK, VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159 traceability mappings, and documentation validation (51 files/161 local links) passed. Temporary outputs/checkouts were removed; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Completed M06/PLUG-03/SEC-12 worker-format contract hardening on 2026-09-08: the 8–192 kHz worker sample-rate range is now defined once and reused by launch, command-line, and latency validation, with lower/upper boundary and rejection regressions. Plugin-host unit (57), worker-process (13), formatting, and strict Clippy checks pass; no plugin registration, audio stream, or machine configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Requalified the complete guarded M00–M08 `safe-all.ps1` chain at commit `a353a990` on 2026-09-08: VS/WDK discovery, native compile, read-only 34-endpoint inventory, disposable SysVAD x64 package/API qualification, M01/M04/M05, pinned VST3 SDK and matrix, modern/legacy VST2 and fault fixtures, M07, unsigned M08 artifacts, 159 traceability mappings, and documentation validation (51 files/161 local links) passed. Temporary outputs/checkouts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred. Production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Completed M06/PLUG-03/SEC-12 launch-validation regression on 2026-09-08: feature-enabled worker-process coverage now proves both below-minimum and above-maximum sample rates are rejected before process creation, complementing the latency-boundary checks. The focused suite passed 21 tests with 6 expected native-fixture tests ignored; strict feature Clippy, formatting, and diff checks passed. No plugin registration, audio stream, or machine configuration changed.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Requalified the six ignored local ReaPlugs VST2 effects at pushed head `8b417906` on 2026-09-08 after worker launch-boundary regression changes: ReaComp, ReaDelay, ReaEQ, ReaFIR, ReaGate, and ReaXcomp each passed the verified native x64 worker load/process acceptance. The wrapper restored `AUDIOROUTER_VST2_FIXTURE`; no plugin registration, audio stream, or machine configuration changed. These remain local compatibility fixtures and do not close rights or release gates.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Requalified the explicitly selected installed Pitchproof x64 VST2 binary at pushed head `fe0fe10c` on 2026-09-08: SHA-256 remained `1974a3033b53ae72da5f419a9f37056d44c1610591bfd11a615ace0c448cf050`; bounded worker processing passed, the dedicated editor thread contained its non-returning native editor call, and the supervised worker was terminated/reaped. The original file was used in place, the environment variable was restored, and no copy, registration, audio stream, or machine configuration change occurred. Editor compatibility, rights, and release qualification remain open.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Expanded M06/PLUG-03/PLUG-07 compatibility evidence on 2026-09-08: the six ignored local ReaPlugs x64 VST2 effects each passed verified worker load/process acceptance at 44.1, 48, and 96 kHz (18 combinations total). The wrapper now selects each bounded rate explicitly and restores both `AUDIOROUTER_VST2_FIXTURE` and `AUDIOROUTER_VST2_SAMPLE_RATE`; no plugin registration, audio stream, or machine configuration changed. This remains local fixture evidence and does not close rights, editor, or release gates.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Expanded M06/PLUG-03/PLUG-07 independent-fixture evidence on 2026-09-08: the installed Pitchproof x64 VST2 binary (`SHA-256` `1974a3033b53ae72da5f419a9f37056d44c1610591bfd11a615ace0c448cf050`) passed verified worker processing at 44.1, 48, and 96 kHz, followed by dedicated and supervised editor-containment checks. Both environment variables were restored; the original binary was used in place with no copy, registration, audio stream, or machine configuration change. Editor compatibility, rights, and release qualification remain open.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Requalified the complete guarded M00–M08 `safe-all.ps1` chain at commit `74e67d6d` on 2026-09-08: VS/WDK discovery, native compile, read-only 34-endpoint inventory, disposable SysVAD x64 package/API qualification, M01/M04/M05, VST3 SDK/matrix, VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, traceability, and documentation passed. Temporary outputs/checkouts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred.
- Completed the post-chain warning cleanup on 2026-09-08: feature-gated VST2 sample-rate test imports no longer warn in ordinary builds. Plugin-host tests passed 57 unit/13 ordinary worker and 21 feature-enabled worker tests (6 expected native-fixture tests ignored), with strict Clippy, formatting, and diff checks passing. Native production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Requalified the complete guarded M00–M08 `safe-all.ps1` chain at commit `74e67d6d` on 2026-09-08 after the expanded installed-fixture work: VS/WDK discovery, native compile, read-only 34-endpoint inventory, disposable SysVAD x64 package/API qualification, M01/M04/M05, VST3 SDK/matrix, VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159 traceability mappings, and documentation validation passed. Temporary outputs/checkouts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred.
- Completed the resulting feature-import warning cleanup on 2026-09-08: ordinary and feature-enabled plugin-host checks are warning-free after the VST2 multi-rate test imports were gated correctly. Plugin-host tests passed 57 unit, 13 ordinary worker, and 21 feature-enabled worker tests (6 expected native-fixture tests ignored), with strict Clippy, formatting, and diff checks passing.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Requalified the complete guarded M00–M08 `safe-all.ps1` chain at the current pushed history on 2026-09-08: VS/WDK discovery, native compile, read-only 34-endpoint inventory, disposable SysVAD x64 package/API qualification, M01/M04/M05 (91 UI tests), VST3 SDK/matrix, VST2 modern/legacy/fault fixtures, M07, unsigned M08 artifacts, 159 traceability mappings, and documentation validation (51 files/161 local links) passed. Temporary outputs/checkouts were cleaned; no driver installation/loading, signing-mode change, plugin/startup registration, stream, or machine audio configuration occurred. Production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M06 task: obtain an additional rights-cleared independent x64 VST2/VST3 fixture or integrate the issuer with an authenticated native shell once a real HWND owner exists; retain fail-closed editor controls and the gated VST2 release boundary.
- Completed M06/PLUG-06/PLUG-07 fixture availability audit on 2026-09-08: a read-only scan of the available Windows plugin directories found only the six qualified ReaPlugs effects, standalone/MIDI/JS assets, and the already-qualified installed Pitchproof x64 VST2 binary plus its rejected x86 sibling. No second-vendor rights-cleared x64 audio-effect fixture is currently available. No plugin was copied, registered, loaded, or modified; no audio or machine configuration changed.
- Next M06 task: qualify a user-supplied rights-cleared independent x64 VST2/VST3 fixture through the existing worker matrix, or integrate the authenticated native-shell HWND owner when available; retain the VST2 rights/editor/release gates.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at pushed head `e8b8f8cd` on 2026-09-08: VS2026/WDK toolchain and native compile, read-only 31-endpoint format inventory, disposable pinned SysVAD x64 package/API/signability qualification, M01/M04/M05, pinned VST3 SDK/validator and matrix, repository VST2 modern/legacy/fault fixtures, M07, unsigned M08 preparation, 159 normative traceability mappings, and documentation validation (51 Markdown files/161 local links) passed. Temporary checkouts/artifacts were removed. No driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine audio configuration occurred; production driver/signing, installer, clean-machine, physical-latency, and manual UI gates remain open.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture or integrate the authenticated native-shell HWND owner when available; retain the gated VST2 rights/editor/release boundary.
- Requalified the all-features locked workspace at pushed head `479a7ad7` on 2026-09-08: 466 unit/integration tests and all doc-tests passed, including the feature-enabled worker suite (21 tests, 6 expected native-fixture skips) and Windows-audio identity/lifecycle coverage (32 tests). No driver, plugin/startup registration, audio stream, signing-mode, or machine audio configuration action occurred.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture or integrate the authenticated native-shell HWND owner when available; retain the gated VST2 rights/editor/release boundary.
- Requalified strict all-features linting on 2026-09-08 at pushed head `07ea9f90`: `cargo clippy --workspace --all-features --locked --all-targets -- -D warnings` passed, covering the VST2 feature and all target binaries without warnings. No runtime, plugin, driver, or machine configuration action occurred.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture or integrate the authenticated native-shell HWND owner when available; retain the gated VST2 rights/editor/release boundary.
- Completed M05/CAP-05/CAP-06 UI identity presentation on 2026-09-08: added a read-only verified executable-path panel consuming the authoritative nullable `executablePath` field, with explicit empty-state and no binding/mutation controls. UI typecheck, 15 test files/93 tests, temporary production build, and diff checks passed; no audio, process, plugin, driver, or machine configuration changed.
- Next M05/M06 task: retain verified identity as display-only until the authenticated native shell and managed process-capture owner are integrated; continue independent plugin qualification and native gates.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at pushed head `f6b394e2` on 2026-09-08 after the verified application identity UI panel: M00 toolchain/native compile and read-only 31-endpoint inventory, disposable pinned SysVAD x64 package/API/signability qualification, M01/M04, M05 UI (typecheck, 15 files/93 tests, temporary production build), M06 SDK/VST3 and VST2 fixtures, M07, unsigned M08 preparation, 159 normative traceability mappings, and documentation validation (51 Markdown files/161 local links) passed. Temporary outputs/checkouts were removed; no driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine audio configuration occurred. Production/native release gates remain open.
- Next M05/M06 task: retain the read-only identity presentation until authenticated native-shell/process-capture integration exists; qualify an independent rights-cleared plugin when supplied and continue the remaining native release gates.
- Hardened guarded acceptance cleanup on 2026-09-09: `safe-all.ps1` snapshots existing temp children and removes only newly created, directly validated `audiorouter-*` children in `finally`, including failure paths. It avoids deleting pre-existing temp artifacts and preserves M08 clean-tree validation.
- Requalified the complete guarded M00-M08 chain at pushed `f9dfb955` on 2026-09-09 after the cleanup hardening: every M00-M08 step passed, including the M08 clean-tree release gate, 159 traceability IDs, and documentation validation (51 Markdown files/163 local links). The runner reported cleanup of 13 run-owned temp children; post-run audit found zero matching `audiorouter-*` temp children and zero MSBuild/compiler processes. No driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine configuration occurred.
- Next M05/M06 task: perform manual Windows Narrator/scaling/packaged-shell acceptance when the authenticated native shell is available, or qualify a supplied rights-cleared independent x64 VST2/VST3 effect; retain all native production gates.
- Fixed M00-M08 acceptance orchestration on 2026-09-09: `safe-all.ps1` no longer treats a stale nonzero `$LASTEXITCODE` from an expected-negative child test as a step failure. The corrected runner was pushed at `3cdf97c6`; an independently captured rerun then completed every step through M08 with a terminal `Safe acceptance chain passed.` result.
- Requalified the complete guarded M00-M08 chain at pushed `3cdf97c6` on 2026-09-09: VS2026/WDK discovery and native compile, read-only 31-endpoint inventory, disposable SysVAD x64 compile/package/API/signability, M01 CLI, M04 DSP/recording, M05 UI (98 tests and temporary production build), pinned VST3 SDK/AGain/mda matrix, native VST3 worker, repository VST2 modern/legacy/fault fixtures, M07 headless, unsigned M08 artifacts, 159 traceability IDs, and documentation validation (51 Markdown files/163 local links) all passed. Logs were captured outside the repository so the M08 clean-tree gate was meaningful. No driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine configuration occurred.
- Cleaned 1,219 repository-owned `audiorouter-*` direct children from the OS temp root after the run; a follow-up scan found zero matching direct children and no native build/compiler processes remained. The first cleanup validation emitted nonfatal null-parent warnings for file targets but still removed only validated direct children; the result was verified.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture or integrate the authenticated native-shell HWND owner when available; retain the gated VST2 rights/editor/release boundary.
- Corrected active-plan checkpoint metadata on 2026-09-09: the latest clean full-chain evidence is `3cdf97c6`, and the currently pushed branch is `09723e28`; later commits record the acceptance evidence and cleanup. Documentation validation and the all-features plugin-host suite remain green.
- Completed the modal focus-containment follow-up on 2026-09-09: the keyboard connection dialog now cycles Tab/Shift+Tab within its controls, handles Escape, and restores the invoking control after successful completion or dismissal. UI typecheck, 15 test files/98 tests, and diff checks passed; no audio or machine configuration changed.
- Completed M05/UI-11 keyboard connection accessibility hardening on 2026-09-09: added a real keyboard connection dialog using the existing draft/validation path, with labelled source/destination selectors, Escape dismissal, initial focus, and focus restoration to the opener. UI typecheck, 15 test files/98 tests, temporary production build, and diff checks passed. The ordinary `ui` build also reached Vite but could not replace a locked existing `ui/dist` asset (`EPERM`); the identical build succeeded in `.tmp-ui-build`. No audio, plugin, driver, or machine configuration changed.
- Completed M05/UI-11 executable accessibility coverage on 2026-09-09: added a jsdom-backed React test environment and focused `App` integration tests for keyboard-dialog initial focus, Tab/Shift+Tab containment, Escape dismissal, focus restoration, validation retention, and successful draft insertion. UI typecheck, 16 test files/100 tests, and a temporary production build (3 files) passed; the new packages are dev-only and `npm audit` reported zero vulnerabilities. No audio or machine configuration changed.
- Next M05 task: perform the manual Windows Narrator, scaling, and packaged-shell acceptance when the authenticated native shell is available; retain the native production gates.
- Completed M05/UI-02/UI-03 canvas connection editing on 2026-09-09: React Flow now exposes visible, named input/output handles and routes drag connections through the same backend draft validation used by the keyboard dialog and port selectors. Disconnected previews remain fail-closed; invalid or incomplete connections stay visible as actionable status. UI typecheck, 16 test files/101 tests including named-handle coverage, temporary production build (3 files), and diff checks passed. No audio or machine configuration changed.
- Next M05 task: perform manual canvas, Narrator, scaling, and packaged-shell acceptance when the authenticated native shell is available; retain native production and independent-plugin gates.
- Closed the remaining portable UI-02 selection gap on 2026-09-09: the canvas now enables React Flow marquee and modified-click multi-selection while keeping transient selection state inside the canvas and the inspector's primary node/application draft state authoritative. A first controlled-selection attempt caused a React Flow/jsdom nested-update loop and was removed; the final ownership boundary passes UI typecheck and all 101 UI tests. No audio or machine configuration changed.
- Next M05 task: manually verify multi-selection, keyboard/list parity, Narrator announcements, scaling, and packaged-shell behavior when the authenticated native shell is available; retain native production and independent-plugin gates.
- Completed the remaining portable UI-02 layout action on 2026-09-09: the canvas now offers a deterministic `Tidy layout` operation that persists presentation-only positions independently from graph/audio state; reset layout remains available separately. A connected UI regression verifies the persisted arrangement. Focused UI typecheck and accessibility tests (4/4) passed; no audio or machine configuration changed.
- Next M05 task: manually verify tidy/multi-selection, keyboard/list parity, Narrator announcements, scaling, and packaged-shell behavior when the authenticated native shell is available; retain native production and independent-plugin gates.
- Closed a portable UI-03 topology gap on 2026-09-09: draft helpers now provide explicit previewable insert-mixer and remove-and-reconnect operations. Mixer removal is fail-closed unless exactly one incoming and one outgoing path exist; no operation changes the authoritative revision or bypasses backend plan/commit validation. Focused typecheck and draft-connection tests (6/6) passed; no audio or machine configuration changed.
- Next M05 task: expose these topology helpers through the connected editor with manual validation of preview/rejection behavior; retain native-shell, production-driver, and independent-plugin gates.
- Corrected mixer insertion channel preservation on 2026-09-09: inserted mixer ports now match the original source width (mono or stereo), and the original destination matrix is retained, preventing an implicit mono/stereo topology change. Stereo-to-mono and mono-path regressions plus the full UI suite (16 files/105 tests) and typecheck pass. No audio or machine configuration changed.
- Added a custom-matrix regression for mixer insertion on 2026-09-09: the downstream preview edge retains a caller-authored channel map instead of replacing it with an identity map. UI typecheck and the full suite (16 files/106 tests) pass; no audio or machine configuration changed.
- Corrected remove-and-reconnect matrix handling on 2026-09-09: the draft helper now composes incoming and outgoing channel maps instead of regenerating a generic map, preserving arbitrary routing coefficients through a preview round trip. Focused topology tests (8/8) and typecheck pass; no audio or machine configuration changed.
- Hardened mixer reconnection validation on 2026-09-09: malformed incoming or outgoing matrix dimensions now fail closed instead of silently treating missing coefficients as zero. The focused topology suite (9/9) and typecheck pass; no audio or machine configuration changed.
- Exposed UI-03 topology previews on 2026-09-09: the extracted draft connection list now offers Insert mixer and Remove and reconnect actions, routed through the existing App draft mutation boundary and fail-closed while disconnected. Component coverage verifies both action dispatches; UI typecheck and the full suite (16 files/108 tests) pass. No audio or machine configuration changed.
- Added connected App integration coverage on 2026-09-09: the test now creates a draft edge through the keyboard connection dialog, inserts a mixer from the rendered list, and removes/reconnects it through the rendered topology action. The focused suite (6/6), full UI suite (16 files/109 tests), and typecheck pass; no audio or machine configuration changed.
- Improved topology action accessibility on 2026-09-09: repeated Insert mixer and Remove and reconnect controls now expose route- or node-specific accessible names, while retaining concise visible labels. UI typecheck and all 109 tests pass; no audio or machine configuration changed.
- Closed the disconnected topology-control gap on 2026-09-09: extracted connection-list mutation buttons now consume a backend connection context and are disabled while offline, while App handlers retain fail-closed guards. The connected App/provider and disconnected presentation boundaries are covered by the UI tests; typecheck and all 110 tests pass. No audio or machine configuration changed.
- Closed UI-11/UI-03 list-view parity on 2026-09-09: structured graph-list view now exposes the same contextual Insert mixer and Remove and reconnect actions as canvas view, with backend-context disabled state. List-view action coverage, typecheck, and the full UI suite (16 files/111 tests) pass; no audio or machine configuration changed.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at pushed head `66333850` on 2026-09-09 after the backend-context and list-view safety changes. Toolchain/native compile, read-only endpoint inventory, disposable SysVAD qualification, M01/M04/M05, current UI production build (210 modules), pinned VST3/native workers, repository VST2 modern/legacy/state/fault fixtures, M07 (67 plugin-host and 13 worker-process tests), unsigned M08 artifacts, 159 traceability IDs, and documentation validation (51 Markdown files/163 local links) passed. Cleanup removed 13 run-owned temporary children; no driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine configuration occurred.
- Next M05 task: manually verify topology preview/rejection messaging, keyboard/list parity, Narrator, scaling, and packaged-shell behavior when the authenticated native shell is available; retain native production and independent-plugin gates.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at pushed head `d634006d` on 2026-09-09 after list-view topology parity. Toolchain/native compile, read-only endpoint inventory, disposable SysVAD qualification, M01/M04/M05, current UI production build (210 modules), pinned VST3/native workers, repository VST2 modern/legacy/state/fault fixtures, M07 (67 plugin-host and 13 worker-process tests), unsigned M08 artifacts, 159 traceability IDs, and documentation validation (51 Markdown files/163 local links) passed. Cleanup removed 13 run-owned temporary children; no driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine configuration occurred.
- Follow-up correction on 2026-09-09: the dialog now remains open when validation rejects an incomplete/duplicate connection, so the user can correct the selection without losing context; successful draft insertion closes it and restores focus. UI typecheck, 15 test files/98 tests, and diff checks passed.
- Rechecked M06/PLUG-07 legacy boundary on 2026-09-09: the contained VST2 adapter still recognizes only the established `VSTPluginMain` and legacy `main` exports after absolute-path/x64 worker launch, validates the `AEffect` header before lifecycle calls, and keeps VST2 on the single-stream `processReplacing` contract. Repository fixtures cover both exports; x86, instruments/MIDI, scripting, redistribution, and auxiliary-bus flattening remain explicitly rejected/out of scope.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 effect or integrate the authenticated native-shell HWND owner; retain the gated VST2 rights/editor/release boundary.

- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at pushed head `861d4b51` on 2026-09-09: VS2026/MSVC/Windows SDK/WDK discovery and native compile, read-only 31-endpoint format inventory, disposable pinned SysVAD x64 compile/package/API/signability qualification, M01 CLI, M04 DSP/recording, M05 UI, pinned VST3 SDK/validator and loader matrix, repository modern/legacy/fault VST2 fixtures, M07 headless checks, unsigned M08 artifacts, 159 normative traceability mappings, and documentation validation (51 Markdown files/163 local links) passed. The first guarded attempt exposed strict-Clippy regressions in the newly added multi-bus fixture test; those were corrected in `861d4b51` before this successful rerun. Temporary checkouts/artifacts were cleaned. No driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine audio configuration occurred. Production driver/signing, installer, clean-machine, physical-latency, manual UI, and rights-cleared independent-plugin gates remain open.
- Next M06/PLUG-03 task: qualify a rights-cleared effect that genuinely exposes auxiliary buses, then connect it to the bounded worker/graph staging contract; retain VST2 single-stream support and the explicit one-input/one-output VST3 side-chain rejection until that evidence exists.
- Requalified dedicated M07 headless acceptance on 2026-09-09: 26 CLI tests, 2 MCP interoperability tests, 97 control tests, 57 plugin-host tests, 13 worker-process tests, and strict Clippy passed. The run created 13 project-named temporary SQLite fixtures; they were removed only after direct-child temp-root validation, and a follow-up scan found zero `audiorouter-*` matches. No audio, driver, startup registration, signing, or persistent machine configuration changed.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture or integrate the authenticated native-shell HWND owner when available; retain the gated VST2 rights/editor/release boundary.
- Rechecked common Windows VST2/VST3 directories on 2026-09-08 with a read-only recursive inventory: 11 DLL candidates were present, consisting of the known ReaPlugs standalone/MIDI/utility assets and the already-qualified Pitchproof x64/x86 pair. No additional x64 VST3 bundle or second-vendor x64 audio-effect fixture was available. No plugin was loaded, copied, registered, or modified, and no audio or machine configuration changed; the independent-fixture gate remains externally blocked.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture through the existing worker matrix, or integrate the authenticated native-shell HWND owner when available; retain the gated VST2 rights/editor/release boundary.
- Rechecked the configured Windows plugin locations read-only on 2026-09-09: no new x64 VST3 bundle or independent audio-effect vendor is available. The inventory still contains the previously known Pitchproof DLL pair and local ReaPlugs VST2 assets only; no plugin was copied, loaded, registered, or modified, and no audio or machine configuration changed. The independent-fixture gate remains externally blocked.
- Completed M00/CAP-01/API-09 audio diagnostic hardening on 2026-09-09: Windows audio failures returned through `devices.list` now preserve a stable category, unsigned HRESULT, retryability, and remediation instead of collapsing into generic `invalidRequest`; focused Windows-audio (33) and control (98) tests plus strict Clippy passed. The `E_INVALIDARG` event/polling boundary and distinct device-in-use classification remain unchanged; no stream or machine configuration changed.
- Next M00/M02 task: retain structured audio diagnostics while connecting the adapter to the managed endpoint-owned callback after the production driver boundary exists.
- Requalified the guarded native endpoint contention boundary on 2026-09-09 with `m00-native-live.ps1 -AllowLiveAudio -DurationMilliseconds 100`: all 13 capture endpoints and 18 render endpoints completed bounded lifecycle checks; one occupied render endpoint followed the exact `0x8889000A` device-in-use branch and cleanup succeeded. Media-device identity and persistent audio settings were unchanged, and temporary probe artifacts were removed. This confirms the contention path without weakening the separate invalid-argument contract; production callback and physical-latency gates remain open.
- Next M00/M02 task: retain this evidence while connecting structured diagnostics to the managed endpoint-owned callback after the production driver boundary exists.
- Extended M00/CAP-01/CAP-05/API-09 audio diagnostics on 2026-09-09: `apps.list` now preserves the same stable Windows audio failure category, HRESULT, retryability, and remediation contract for process and audio-session discovery instead of returning a generic invalid request. Focused Windows-audio (33) and control (98) tests, strict Clippy, formatting, and diff checks passed; no stream or machine configuration changed.
- Next M00/M02 task: retain structured discovery diagnostics while connecting the adapter to the managed endpoint-owned callback after the production driver boundary exists.
- Completed M01/API-09 contract parity on 2026-09-09: the shared TypeScript `ApplicationErrorData` now exposes the optional unsigned Windows `hresult` emitted by structured audio failures, while retaining compatibility for errors without an OS HRESULT. Contracts typecheck, drift validation (61 methods/17 node kinds/7 processors), UI typecheck, and 93 UI tests passed; no runtime or machine configuration changed.
- Next M05/M00 task: consume structured error data in user-facing connected views where native shell integration is available, while retaining the disconnected fail-closed UI and native audio blockers.
- Requalified the locked all-features workspace after M00/CAP-01/API-09 audio diagnostic hardening on 2026-09-09: 466 unit/integration tests and all workspace doc-tests passed; strict all-target Clippy, formatting, and diff checks passed. The run left 36 direct `audiorouter-*` temporary test databases, which were removed after temp-root validation; a follow-up scan found zero matches. No driver, plugin/startup registration, audio stream, signing-mode, or persistent machine configuration action occurred.
- Next M00/M02 task: retain structured audio diagnostics while connecting the adapter to the managed endpoint-owned callback after the production driver boundary exists.
- Requalified the complete guarded `tests/acceptance/safe-all.ps1` chain at pushed head `16aed8a1` on 2026-09-09: VS2026/WDK toolchain and native compile, read-only 31-endpoint inventory, disposable pinned SysVAD x64 package/API/signability qualification, M01/M04/M05, pinned VST3 SDK/validator and VST2 modern/legacy/fault fixtures, M07, unsigned M08 preparation, 159 normative traceability mappings, and documentation validation (51 Markdown files/161 local links) passed. The 13 direct `audiorouter-*` temporary test databases left by the run were removed and a follow-up scan found zero matches; no driver installation/loading, signing-mode change, plugin/startup registration, audio stream, or persistent machine audio configuration occurred.
- Next M06 task: qualify a supplied rights-cleared independent x64 VST2/VST3 fixture through the existing worker matrix, or integrate the authenticated native-shell HWND owner when available; retain the gated VST2 rights/editor/release boundary.
- Completed M08 test-artifact hygiene on 2026-09-08: after the guarded acceptance run, removed exactly 6,690 direct children of `%TEMP%` whose names matched the repository-owned `audiorouter-*` prefix (SQLite test databases and disposable plugin/recording/CLI fixtures). The target paths were validated under the temp root before removal; a follow-up scan found zero matching direct children. Repository files, local plugin binaries, drivers, audio streams, and persistent machine configuration were not touched.
- Next M05/M06 task: retain the read-only identity presentation until authenticated native-shell/process-capture integration exists; qualify an independent rights-cleared plugin when supplied and continue the remaining native release gates.
