# Active plan

Updated: 2026-09-05.

## Current state

Specification baseline created. Application implementation has not started. The workspace initially contained empty `.agents`, `.codex`, and `.git` directories; `git status` reported that it was not a Git repository. Do not invent a branch/commit or initialize Git unless that becomes part of an authorized implementation workflow.

The current deliverable is documentation only: product requirements, domain/architecture, feature contracts, non-functional targets, nine sequential milestone definitions, and agent lifecycle instructions. Read the [documentation index](../../README.md) and [delivery map](../../spec/15-delivery.md).

M00 feasibility work has now started with a read-only environment inventory from WSL2. The inventory is evidence about the development shell only; it is not Windows audio evidence.

## M00 execution log

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

## Next authorized implementation task

Continue [M00](../../milestones/M00-feasibility.md) on native Windows 11 x64: collect the exact OS/build, compiler/SDK/WDK, endpoint and hardware manifest; run the capture/render and process-loopback probes; and evaluate the managed-driver path. Keep the WSL inventory as supporting context only. Do not install a driver, change Windows defaults, or claim a feasibility gate until the native evidence is attached.

## M00 preparation checklist

- Identify an available Windows 11 x64 test environment and audio hardware. WSL interop is currently blocked as recorded above.
- Review current driver integration/signing options using the source register.
- Record toolchain/OS/device versions and supported capture probes.
- Resolve DEC-03/06/07 with evidence before broad implementation.
- Preserve all failed/unsupported results and distinguish prototype from production-driver capability.

## Handoff rule

When work begins, add objective, requirement IDs, task checklist, changes, decisions, checks, environment, results, blockers, rollback, and next action here. After the milestone gate passes, archive the execution plan under `archived/` and link its evidence. Future ideas belong in `future/`, not this active task.
