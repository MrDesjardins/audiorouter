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
- This is identity/state evidence only. Formats, periods, shared-mode capture/render, loopback latency, process capture, and physical tone/impulse behavior remain unmeasured.

## Next authorized implementation task

Continue [M00](../../milestones/M00-feasibility.md) using an authorized Windows build environment: add and run a minimal Windows WASAPI endpoint/format/period and process-loopback probe; identify an authorized physical loopback setup; and evaluate the managed-driver path. This host's SDK-only state is insufficient for the C++/driver probe. Keep the WSL inventory as supporting context only. Do not install a driver, change Windows defaults, or claim a feasibility gate until the native evidence is attached.

## M00 preparation checklist

- Identify an available Windows 11 x64 test environment and audio hardware. WSL interop is currently blocked as recorded above.
- Review current driver integration/signing options using the source register.
- Record toolchain/OS/device versions and supported capture probes.
- Resolve DEC-03/06/07 with evidence before broad implementation.
- Preserve all failed/unsupported results and distinguish prototype from production-driver capability.

## Handoff rule

When work begins, add objective, requirement IDs, task checklist, changes, decisions, checks, environment, results, blockers, rollback, and next action here. After the milestone gate passes, archive the execution plan under `archived/` and link its evidence. Future ideas belong in `future/`, not this active task.
