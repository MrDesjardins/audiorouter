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

- Added `Storage::restore_backup`, which requires absolute paths, a regular non-symlink source, a destination parent that already exists, a new destination file, a 64 MiB bound, and SQLite `integrity_check` success before restoring.
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

Continue [M00](../../milestones/M00-feasibility.md) in an authorized Visual Studio/WDK environment: build/run the official native process-loopback activation path and a native endpoint capture client, then perform initialization/format checks without starting or reading audio. In parallel, evaluate the managed-driver prototype. No driver install, default change, or capture stream start is authorized by this plan; do not claim the feasibility gate until native evidence is attached.

## M00 preparation checklist

- Identify an available Windows 11 x64 test environment and audio hardware. WSL interop is currently blocked as recorded above.
- Review current driver integration/signing options using the source register.
- Record toolchain/OS/device versions and supported capture probes.
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
- Added `AudioBlock::mix_mapped_from` for allocation-free destination-major matrix accumulation, preserving existing destination audio for explicit fan-out/mixer inputs. The engine suite now has 16 passing tests; full node scheduling remains open.
- Added allocation-free `AudioBlock::clamp_unit` and `peak_abs` primitives for output-boundary clipping counts and peak metering. Internal graph processing still retains headroom; the caller explicitly chooses when to clamp.
- Extended `CallbackMetrics` with caller-recorded clipping and xrun counters. The counters are atomic and deliberately do not infer hardware failures; the Windows scheduler will record those events when implemented.
- Added lock-free `BlockMeter` peak and clipping observation with reset semantics. It is a portable Meter-node primitive; per-node runtime wiring and external health publication remain open.
- Added fixed-capacity lock-free `AudioBlockQueue` storage using preallocated slots. Push/pop never wait or allocate after construction and expose full/empty conditions for explicit xrun policy; the engine suite now has 19 passing tests.
- Added atomic overrun/underrun counters to `AudioBlockQueue`; full pushes and empty pops remain nonblocking and are now observable as queue-health events.
- Added allocation-free per-channel peak/RMS and aggregate RMS primitives to `AudioBlock`, with finite-sample filtering and invalid-channel handling. Engine tests now total 20, and strict engine Clippy passes.
- Added preallocated rolling `RmsWindow` with bounded capacity, finite-input handling, and reset semantics for the specified RMS telemetry window. Engine tests now total 21 and strict engine Clippy passes.
- Extended the native probe with opt-in `process-capture` data reads. A 500 ms run completed async process-loopback activation, event-driven `Start`/read/`Stop`/`Reset`, and read 50 packets/22,050 frames with 15,217 nonzero payload bytes; temporary binaries were removed and no system audio settings changed.
- Cleared strict workspace Clippy findings in the domain, engine, Windows adapter tests, and CLI iterator code; `cargo clippy --workspace --all-targets -- -D warnings` now passes.
- Added explicit `process-capture-exclude` support and verified the exclude-tree mode for 500 ms with the same successful 50-packet/22,050-frame read and cleanup. Controlled per-process tone attribution remains open.
- Added stable `AudioFailureKind` classification for invalid arguments, access denial, device-in-use, exclusive-only, invalidated-device, unsupported-format, service-unavailable, and buffer-constraint cases while retaining original HRESULTs.
- Extended read-only application discovery with optional Windows process creation timestamps, allowing future process-loopback bindings to verify PID plus creation time and reject PID reuse; command lines and full paths remain excluded.
- Added `bind_application`, which requires PID, executable name, and creation timestamp to match the observed process before a future loopback activation; the Windows identity test passes without opening an audio stream.
- Tightened `bind_application` so an unavailable creation timestamp is rejected rather than treated as a valid identity; PID/name alone can no longer authorize a future loopback binding.
- Corrected the failure taxonomy so undersized caller buffers report `BufferConstraint` rather than being conflated with malformed arguments; seven adapter tests and strict adapter Clippy pass.
