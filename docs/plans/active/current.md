# Active plan

Updated: 2026-09-05.

## Current state

Specification baseline created. Application implementation has not started. The workspace initially contained empty `.agents`, `.codex`, and `.git` directories; `git status` reported that it was not a Git repository. Do not invent a branch/commit or initialize Git unless that becomes part of an authorized implementation workflow.

The current deliverable is documentation only: product requirements, domain/architecture, feature contracts, non-functional targets, nine sequential milestone definitions, and agent lifecycle instructions. Read the [documentation index](../../README.md) and [delivery map](../../spec/15-delivery.md).

M00 feasibility work has now started with a read-only environment inventory from WSL2. The inventory is evidence about the development shell only; it is not Windows audio evidence.

## M00 execution log

### M00 working scope

- Objective: establish Windows feasibility evidence and driver/toolchain decisions before M01.
- Requirement IDs: CAP-01–08, ARCH-05/07/08, VDEV-02/09, NFR-01–03, and ENG-03/04.
- Completed in this pass: native machine/OS/toolchain/device inventory and evidence record.
- Remaining checklist: endpoint format/period enumeration; shared-mode capture/render; physical loopback latency; process-tree include/exclude and restart/PID reuse; controlled tone harness; managed-driver integration/signing evaluation; DEC-03/06/07 decision update.
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
- Hardened graph plans with a 30-second default expiry and a testable TTL override. Expired plans fail before session mutation, while idempotency and revision checks remain unchanged.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 10 control, 12 domain, 5 protocol, and 2 storage tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Added `crates/cli`, an offline M01 command surface for `help`, `status`, `schema`, `devices list`, `apps list`, `nodes types`, and `api methods`, with `--json` output and human-readable output. It reports fake/unavailable audio explicitly and shares control-plane discovery rather than inventing device results.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 3 CLI, 10 control, 12 domain, 5 protocol, and 2 storage tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Added `tests/acceptance/m01-cli.ps1`, a checked-in PowerShell acceptance script for schema, status, device-list, and node-type discovery. It verifies offline M01 behavior and refuses success-shaped fake device results.
- Corrected JSON-RPC notification semantics in control dispatch: read-only notifications are consumed without response envelopes, while mutating notifications remain rejected. This prevents clients from receiving misleading success responses for notifications.
- Checks: `cargo fmt --all`, `cargo test --workspace` — 3 CLI, 11 control, 12 domain, 5 protocol, and 2 storage tests passed; standalone WASAPI `cargo check` and `git diff --check` passed.
- Added the [M01 execution evidence report](evidence/M01-contracts.md), mapping implemented crates, commands, test counts, supported requirement slices, and explicit untested Windows/security/audio boundaries. The next task is the authorized Windows named-pipe/authentication boundary; no HTTP transport is planned.

### 2026-09-05 — WSL inventory

- Environment: WSL2 Linux kernel `6.6.87.2-microsoft-standard-WSL2`, `x86_64`.
- Portable tools visible in WSL: Rust `1.96.0`, Cargo `1.96.0`, Node `24.12.0`, npm `11.6.2`.
- Windows filesystem is mounted at `/mnt/c`; Windows PowerShell is present on disk.
- Attempted native Windows queries for OS/build, media devices, and compiler commands through `powershell.exe` and `cmd.exe`.
- Result: both interop attempts failed before command execution with `WSL (3 - ) ERROR: UtilBindVsockAnyPort:307: socket failed 1`.
- Evidence status: `blocked` for Windows OS build, endpoint inventory, SDK/WDK, hardware, WASAPI, process-loopback, virtual-driver, and latency checks. No Windows requirement is marked passed from this result.
- Safety: no drivers installed, no defaults changed, no audio captured, and no user files modified outside the project documentation.

This establishes that Codex can continue documentation and portable implementation from WSL, but M00's Windows gates require a working native Windows session (or a separately reachable Windows test machine). The WSL shell should not be treated as a replacement for that environment.

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

Continue [M00](../../milestones/M00-feasibility.md) in an authorized Visual Studio/WDK environment: build/run the official native process-loopback activation path and a native endpoint capture client, then perform initialization/format checks without starting or reading audio. In parallel, evaluate the managed-driver prototype. No driver install, default change, or capture stream start is authorized by this plan; do not claim the feasibility gate until native evidence is attached.

## M00 preparation checklist

- Identify an available Windows 11 x64 test environment and audio hardware. WSL interop is currently blocked as recorded above.
- Review current driver integration/signing options using the source register.
- Record toolchain/OS/device versions and supported capture probes.
- Resolve DEC-03/06/07 with evidence before broad implementation.
- Preserve all failed/unsupported results and distinguish prototype from production-driver capability.

## Handoff rule

When work begins, add objective, requirement IDs, task checklist, changes, decisions, checks, environment, results, blockers, rollback, and next action here. After the milestone gate passes, archive the execution plan under `archived/` and link its evidence. Future ideas belong in `future/`, not this active task.
