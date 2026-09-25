# 15 — Delivery sequence, traceability, decisions, and sources

Milestone ownership: all milestones. This is the stable release map; execution status lives in the [active plan](../plans/active/current.md).

## Active scope decision (2026-09-19, supersedes 2026-09-17)

The delivery track is VB-Cable/Voicemeeter-first, permanently: the
project-owned kernel driver, PortCls integration, and production signing are
set aside by explicit user cost decision (a signing credential is not
funded), not deferred pending a future budget. Existing VB-Cable/Voicemeeter
and physical WASAPI endpoints are the project's complete, supported
virtual-routing strategy for v1 — not an interim substitute for a managed
bus. Completing "everything else" in this delivery map means every
non-driver, non-signed-release requirement across M00–M08: routing, DSP,
recording, automation, recovery, CLI/MCP parity, and the visual editor. It
does not include VDEV-01/03/09, SEC-08, or M08's signed-installer/
clean-machine gates, which remain normative only for a future funded signed
track and are explicitly excluded from this project's v1 completion target.
See [the set-aside driver plan](../plans/future/M03-driver-signing.md).

## Milestone sequence

| Milestone | Outcome | Prerequisites | Release gate |
| --- | --- | --- | --- |
| [M00](../milestones/M00-feasibility.md) | Windows probes, driver strategy, measured technical baseline | Specification | Capture/output proven; credible managed-driver/signing path |
| [M01](../milestones/M01-contracts.md) | Backend contracts, domain/store, CLI, fake engine | M00 decisions | Headless transactions/discovery and invariant tests |
| [M02](../milestones/M02-audio-engine.md) | Physical and application audio engine | M01 | Actual Windows sound, realtime/clock/feedback evidence |
| [M03](../milestones/M03-virtual-routing.md) | Managed virtual buses and primary routes | M02 + M00 driver outcome | Discord/OBS isolation and endpoint lifecycle |
| [M04](../milestones/M04-effects-recording.md) | Built-in voice processing and recording | M03 | Functional alpha from CLI, DSP/file tests |
| [M05](../milestones/M05-visual-editor.md) | Accessible React visual application | M04 | Primary workflow entirely from UI, API parity |
| [M06](../milestones/M06-plugins-pitch.md) | Isolated VST3 and pitch shift | M05 | Worker containment, state/latency/compatibility |
| [M07](../milestones/M07-automation-recovery.md) | Full MCP, background lifecycle, recovery/security | M06 | Concurrent clients, restart, privacy, migration |
| [M08](../milestones/M08-release.md) | Signed, installable, documented Windows v1 | M07 | All v1 requirements and release evidence |

Default execution is sequential and each milestone can be requested independently. Do not ask one LLM to generate the whole application in one step. Split a milestone into bounded active-plan tasks when necessary, retaining its gate. Earlier functionality must remain usable while later work is added. No calendar estimates are committed before M00 resolves the driver risk.

## Requirement traceability

The ranges below include every normative ID; each milestone must attach evidence to its owned IDs in its execution report. “Final” means regression/release evidence, not postponing the initial implementation.

| Requirement IDs | Primary delivery | Final scenarios/evidence |
| --- | --- | --- |
| PROD-01–08 | M00/M01 and each feature milestone | M08 scope/platform/usability review |
| ARCH-01–03, ARCH-10, ARCH-12 | M01 | UC-07/10, dependency/adapter review |
| ARCH-04–09, ARCH-11 | M02 | UC-01/06, timing/drift/soak |
| GRAPH-01–09, GRAPH-12–13 | M01/M02 | UC-02/05/07, compiler/property tests |
| GRAPH-10, GRAPH-14 | M04/M06 | UC-02/06, compensation/failure tests |
| GRAPH-11 | M02/M03 | UC-01/04/05, global cycle checks |
| CAP-01–08 | M00/M02 | UC-03, Windows device/app matrix |
| CAP-09–10 | M03/M05 | UC-01, duplicate-playback validation |
| CAP-11–12 | M02/M07 | UC-03/06, OS transitions |
| VDEV-01–08, VDEV-10–12 | M03/M07 | UC-01/05/09/10, driver lifecycle |
| VDEV-09 | M00 strategy; M08 shipping | Secure Boot/HVCI installation |
| DSP-01–05, DSP-07–09 | M04 | UC-02, transfer-function vectors |
| DSP-06, PLUG-01–06 | M06 | UC-02/06, pitch/worker/format evidence |
| REC-01–08, REC-10–12 | M04/M05 | UC-08, frame/file/path tests |
| PLUG-07 | M06 extension/M08 | Native x64 VST2 adapter, worker containment, rights, compatibility, and failure evidence |
| REC-09 | M04/M07 | UC-08, crash/disk-failure recovery |
| UI-01–15 | M05/M07 | UC-01–10, keyboard/Narrator/usability |
| API-01–07, API-09–12 | M01, each feature extension | UC-07/10, protocol/parity tests |
| API-08 | M01 baseline; M07 hardening | Reconnect/backpressure/resync |
| AUTO-01–05 | M01, each feature extension | UC-10, executable PowerShell fixture |
| AUTO-06–12 | M07 | UC-07/10, MCP permission/parity |
| AUTO-13–14 | M05/M07 | Assistant setup/activity, bounded privacy-safe diagnostics |
| STATE-01–07, STATE-12 | M01, M07 hardening | Import/migration/corruption fixtures |
| STATE-08–11 | M07 | UC-06/09, startup/sign-out/recovery |
| SEC-01–06, SEC-10, SEC-12 | M01, M07 audit | Scope, pipe, shell, abuse cases |
| SEC-07 | M06 | Plugin containment evidence |
| SEC-08 | M03/M08 | Driver boundary/signing review |
| SEC-09 | M04/M07 | Path and bundle attacks |
| SEC-11 | M08 | Signed update/rollback |
| NFR-01–16 | Gates in [14](14-quality.md) | M08 measured reports |
| QUAL-01–06 | M02–M06 as relevant | M08 signal/recording regression |
| ENG-01–05 | Every milestone | M08 contract/docs/build evidence |

## Initial decision register

These decisions are the proposed baseline for implementation. Evidence may change a provisional decision through a documented active-plan decision and synchronized specification update.

| ID | Decision | State and rationale |
| --- | --- | --- |
| DEC-01 | Windows 11 x64 only | Scope baseline; ARM support deferred explicitly |
| DEC-02 | Rust domain/control/engine; narrow Windows FFI | Baseline; maintainability and ownership boundaries |
| DEC-03 | React + TypeScript + Vite, Tauri/WebView2 shell | React/TypeScript/Vite is implemented and validated in M05; the native shell choice remains provisional until live transport injection and manual acceptance |
| DEC-04 | Shared JSON-RPC application API over restricted named pipe | Baseline; local headless/GUI parity without exposed TCP |
| DEC-05 | Per-user background engine, sign-in startup | Baseline; separate user capture from privileged driver lifecycle |
| DEC-06 | 48 kHz float32, initial 128-frame quantum | Retained as the graph baseline; native endpoint formats and process-loopback evidence exist, but physical latency/quantum measurements are still required before making the quantum final |
| DEC-07 | Managed persistent driver, not user-mode-only virtual microphones | Project-owned SysVAD-derived prototype selected for technical evaluation; production package, adaptation, target-machine validation, and signing remain release blockers |
| DEC-08 | VST3 x64 plus built-ins; gated legacy VST2 x64 extension | Baseline release path remains VST3 plus built-ins; the explicitly approved PLUG-07 extension adds contained x64 VST2 effects only after ABI, rights, worker, editor, compatibility, and release evidence is complete |
| DEC-09 | Explicit virtual desktop render route; capture-only app sources | Baseline; automatic capture-and-mute not assumed |
| DEC-10 | Protected voice paths silence on effect failure | Baseline; deliberate user bypass is a separate action |
| DEC-11 | SQLite + versioned portable bundle | Baseline; journal/import safety tested before recovery claims |
| DEC-12 | Mono/stereo, eight buses, sequential milestones | Scope baseline to prioritize common routing workflows |
| DEC-13 | Legacy VST2 is an explicitly approved, gated M06 extension | User approved planning on 2026-09-08; the original V1 baseline remains VST3 plus built-ins, while actual ABI, rights, worker, and compatibility evidence remain required |
| DEC-14 | NFR-01 wired physical loopback latency target revised from ≤30 ms to ≤250 ms p95 | User-approved 2026-09-21 after calibrated measurement (see [14-quality.md](14-quality.md) NFR-01 and [M00 WASAPI probe evidence](../plans/active/evidence/M00-wasapi-probe.md)): a real reference device (Focusrite Scarlett Solo) measured p95 ≈ 185.5–185.9 ms in WASAPI shared mode — AudioRouter's production sharing model, not exclusive/ASIO — across three reproducible runs, after an initial calibration bug was found and fixed. Buffer size and Windows 10+ `IAudioClient3` low-latency shared-mode tuning were both tested and ruled out as a fix on this device (its low-latency engine-period floor is a fixed 10 ms). The original ≤30 ms figure was an unvalidated aspirational target. The revised target is only validated on one device; it is not yet known whether other supported interfaces perform better or worse, and closing the gap further (e.g. via exclusive mode) remains unexplored future work, not authorized by this decision |
| DEC-15 | NFR-02 mic-to-virtual-capture latency target revised from ≤40 ms to ≤160 ms p95 | User-approved 2026-09-21 after calibrated measurement through the real engine route (see [14-quality.md](14-quality.md) NFR-02 and [M00 WASAPI probe evidence](../plans/active/evidence/M00-wasapi-probe.md)): the same reference device measured p95 of 97.5/102.9/115.5/110.9 ms across four runs (physical mic capture → real `PhysicalInput → Gain → Recorder → PhysicalOutput` graph/scheduler → virtual render → virtual capture read), each run's own internal spread under ~13 ms with more run-to-run variance across separate engine-route launches than NFR-01 showed. The revised ≤160 ms target follows the same DEC-14 methodology: roughly 35% headroom over the worst observed run (115.461 ms). Also validated on one device only; not yet known whether this cross-run variance narrows on repeated measurement or with a different mic/driver |

## Risk and dependency register

| Risk | Consequence | Required mitigation / owner gate |
| --- | --- | --- |
| Driver redistribution/signing unavailable | Cannot ship integrated virtual endpoints | M00 select credible path; M08 blocks release until signed package exists |
| Driver bugs/security weakness | System instability or cross-user audio leak | Dedicated test systems, small interface, verifier/security tests, signed rollback; M03/M08 |
| Process capture misses protected/complex apps | Capture scope differs from expectation | Capability errors and virtual-output alternative; M00/M02/M08 |
| Desktop loopback captures own monitor | Feedback or duplicate audio | Explicit desktop bus plus global topology validation; M03 |
| Existing ReaPlugs may be legacy VST2 with incomplete rights/editor evidence | User's exact plugin chain may not migrate directly or be release-qualified | Built-ins plus gated x64 VST2 worker qualification; explicit rights, editor, compatibility, and release gates; M06/M08 |
| Device clocks drift | Growing latency, clicks, underruns | Bounded asynchronous resampling and 8-hour tests; M02 |
| Plugin worker latency/instability | Delayed or interrupted voice | Per-instance containment, measured latency, protected failure policy; M06 |
| Unavailable Windows hardware here | Cannot verify platform requirements | Record blocked test evidence; never substitute Linux mocks; M00 onward |
| LLM/client concurrent edits | Lost user change or unintended route | Backend revisions, preview, scopes, idempotency; M01/M07 |
| Installer/update changes endpoint IDs | Discord/OBS selections break | Identity-preserving migration and explicit reselection flow; M08 |
| Scope expands to all reference-product features | Delays core application | Future backlog and explicit scope decisions; every milestone |

External prerequisites that agents cannot invent include a Windows test machine, appropriate physical audio equipment, driver source/redistribution rights, signing organization credentials, and installed test plugin/app binaries. Gather concrete findings before requesting any purchase, account action, or scope decision. This specification authorizes none of those external actions by itself.

## Source register

Consulted 2026-09-05. References inform Windows constraints and product inspiration; the requirement text is an original proposed design. User-supplied competitor release numbers/dates are not prerequisites and are not repeated as verified facts. Recheck evolving SDK, OS support, protocol, and signing details when implementing their gates.

| Source | Use and limitation |
| --- | --- |
| [Audio Hijack product](https://rogueamoeba.com/audiohijack/) | Inspiration: visual capture/processing/recording sessions; not a Windows capability guarantee |
| [Loopback product](https://rogueamoeba.com/loopback/) | Inspiration: named virtual devices/routing/monitoring; not Windows driver design |
| [Microsoft application-loopback sample](https://github.com/microsoft/Windows-classic-samples/blob/main/Samples/ApplicationLoopback/README.md) | Process-tree scope and Windows API feasibility |
| [Microsoft process-loopback modes](https://learn.microsoft.com/en-us/windows/win32/api/audioclientactivationparams/ne-audioclientactivationparams-process_loopback_mode) | Include/exclude one target tree |
| [Microsoft endpoint loopback](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording) | Endpoint mix, protected-content and session-scope caveats |
| [Microsoft low-latency audio](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/low-latency-audio) | Query supported periods; hardware-dependent latency |
| [Microsoft SysVAD sample](https://github.com/microsoft/Windows-driver-samples/tree/main/audio/sysvad) | WDK virtual audio reference; not production cable functionality |
| [Microsoft driver signing offerings](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings) | Shipping gate and signing route to revalidate |
| [Microsoft driver code-signing requirements](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-reqs) | Account/certificate submission dependencies |
| [Microsoft pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights) | Explicit local IPC security boundary |
| [Steinberg VST3 licensing FAQ](https://steinbergmedia.github.io/vst3_dev_portal/pages/FAQ/Licensing.html) | SDK license and VST2 distinction; pin actual dependency license |
| [Cockos ReaPlugs](https://www.reaper.fm/reaplugs/) | Existing-user plugin context; inspect actual binary format |
| [JSON-RPC 2.0](https://www.jsonrpc.org/specification) | Request/response protocol; app methods/framing are project decisions |
| [Tauri architecture](https://v2.tauri.app/concept/architecture/) | Candidate Windows desktop shell boundary |
| [React Flow documentation](https://reactflow.dev/learn) | Candidate visual graph implementation; backend remains authoritative |

MCP's evolving transport/SDK documentation must be pinned and verified during M07. No product-specific Claude/Codex setup format is frozen here; provide generic stdio server launch instructions and validate supported clients against their official documentation at implementation time.

## Definition of done

A milestone is complete when all scoped requirements have implementation and evidence, applicable Windows tests pass, docs/contracts match behavior, and the active plan contains a reproducible handoff. A release additionally needs M08 signing/install/update/rollback, hardware performance, privacy/security, and usability gates. A plan, generated UI, successful compile, or mocked demo alone is not completion evidence.
