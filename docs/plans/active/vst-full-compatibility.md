# Full VST2 and VST3 compatibility

Status: **approved 2026-11-13, in progress.** Owner request: "support ALL VST2
and 3".

## Decision record (owner, 2026-11-13)

- D1 instruments and MIDI: **in scope**, "we need both to be working".
- D2 32-bit plugins: **in scope**.
- D3 spec: **revisit**. Done in this change: PLUG-01, PLUG-07, DEC-08, the
  deferred list in `01-product.md` and the traceability rows. The rights,
  containment, failure and evidence clauses stay.
- D4 corpus: free and demo plugins plus plugins named in user reports; paid
  only with a licence. The assistant may download and install free plugins.
- Working rules for this effort: commit per phase on branches, stage files by
  name; no PR, push or release without a separate approval; demo plugins with
  popups (Blue Cat) never run while the owner plays or is on a call; census
  editor windows stay hidden; no plugin binary enters git; no heavy build or
  plugin run while the owner says "docs/code only".

## Order (revised: tightest constraint first)

1. Commit #136 work (done, `268fa0146`).
2. **P1** Detect and classify instruments (VST2 `effFlagsIsSynth`; refuse with a
   reason until MIDI ships). Existing refusal tests and the
   `unsupportedInstrument` contract flip deliberately.
3. **P8a** 32-bit: first audit the worker protocol for pointer-width types, then
   x86 workers, architecture routing, build and installer.
4. **P0** Census (needs the corpus; its failures order P2/P3).
5. **P8b** MIDI source node and instruments (largest piece; full AUTO-18).
6. P2, P3, P4, P5, P6, P7, P9 as below.

This work spans many sessions; each phase reports its own results and what was
not run.

## Progress

| Phase | State |
| --- | --- |
| #136 fixes + chain matrix | committed `268fa0146` |
| Spec revision (D3) | done in the same change as this page |
| P1 instrument detection | load-time refusal done (`effFlagsIsSynth`, unit tests pass, Clippy clean); not run against the real BC Synth (demo popup); scan-time classification via a per-fingerprint cache and the VST3 worker unchanged |
| P8a 32-bit | done for VST2 (exercised with real 32-bit ReaPlugs); VST3 x86 worker built but untested; see P8a results |
| everything else | not started |

Related: [issue #136 plan](issue-136-vst-experience.md),
[chain matrix](issue-136-vst2-chain-matrix.md). Requirements touched:
PLUG-01..07, DEC-08, GRAPH-14, UI-03/08, AUTO-18; milestones M06/M08.

## What "all" can mean

No host runs every plugin: some need a 32-bit process, MIDI, a vendor shell,
or a specific DAW. The goal proposed here is measurable instead:

- every x64 VST2 and VST3 **audio effect** that loads in a reference host
  (REAPER) also loads, runs, shows its editor, and saves and restores in
  AudioRouter, or is refused before Add with a plain reason;
- no plugin can crash, hang or silence the app (already the worker rule);
- a published list says what was tested and with what result.

Instruments (need MIDI) and 32-bit plugins are separate, larger scope
decisions (D1, D2 below). Today's spec excludes both (PLUG-01).

## What is missing today (from the code, 2026-11-13)

| Area | Today | Gap |
| --- | --- | --- |
| VST2 host callback (`vst2.rs` `host_callback`) | Answers version, time info, process level, vendor version, sample rate, block size, size window | No `canDo`, `ioChanged`, `automate`/`beginEdit`/`endEdit`, `updateDisplay`, vendor/product string, `getDirectory`, `wantMidi`, `idle`. Plugins that ask and get 0 can misbehave or refuse features |
| VST2 calls to the plugin | open/close, rate, block size, mains, param name, editor, chunks | No `effCanDo`, `effSetBypass`, programs, `effGetPlugCategory` (shell plugins such as Waves), speaker arrangement, `effProcessEvents`, `effGetEffectName`, `effStartProcess`/`effStopProcess`, `.fxp`/`.fxb` import |
| VST2 header rules | Needs `canReplacing`, 1..64 inputs | A plugin with 0 inputs (generator) or only the old `process` call is rejected; `effFlagsIsSynth` is not read, so an instrument is listed as a supported effect |
| VST3 worker (`tools/m06-vst3-worker/main.cpp`) | Component + controller, parameters, state, latency, `IPlugFrame` resize, one class per module | No event list (MIDI), no `IComponentHandler2`/`IUnitInfo`/programs/context menu, no content-scale interface, no 64-bit sample mode, class choice in a multi-effect module unverified, some plugins report state unsupported |
| Sidechain | Worker has an auxiliary-bus test gate | Not wired to routes or the UI; connecting a second source adds a Mixer (UI-03) |
| Chains | Up to 8 VST2 in one worker | VST3 is one worker each (works, costs latency and memory) |
| Editors | Open only while the route plays; fixed-size host window fitted to the plugin | No editor on a stopped session; user resize of resizable VST3 editors, keyboard focus and per-monitor moves unverified |
| Channel layouts | Mono or stereo nodes; node width must match | Mono-only, stereo-only and multichannel plugins need automatic adaptation |
| Architecture | x64 only | x86 refused |
| Evidence | ~20 plugins tested by hand over two months | No broad, repeatable census |

## Decisions needed before work starts

- **D1 Instruments and MIDI.** In or out? In means a MIDI input source, MIDI
  routing in the graph, and event delivery in both workers: a new product
  area (spec change to PROD scope and PLUG-01, new node kinds, REST/MCP,
  guides). Recommendation: out for now; refuse instruments clearly (phase 1).
- **D2 32-bit plugins.** In or out? In means 32-bit builds of both workers
  and their signing and packaging. Recommendation: out until the census shows
  users have them.
- **D3 Spec change.** PLUG-01/PLUG-07 and DEC-08 call VST2 a "gated
  extension" and exclude instruments and x86. Approving this plan means
  rewriting them; that needs the owner's explicit decision record.
- **D4 Corpus.** Which plugins must pass? Proposal: the free and demo
  plugins listed in phase 0, plus anything named in user reports. Paid
  plugins only when the owner or a user has a licence.

## Phases

Each phase ends with the matrix rerun, a regression test per defect, the
compatibility page updated, and a build the owner can try.

**Phase 0 - Census (measure first).** Extend `vst2_chain_matrix.rs` into a
format-neutral plugin census that also covers VST3 and scans a folder: load,
layouts offered, audio, parameter, state, editor, close, restore, per plugin,
unattended (editor host hidden, vendor dialogs detected and reported, not
clicked). Install a corpus of free plugins with different frameworks (JUCE,
iPlug2, VSTGUI, DPF, Blue Cat, Melda free bundle, TDR, Valhalla free, Kilohearts
free, Voxengo free, Airwindows, Analog Obsession, ReaPlugs, UADx). Output: one
table of failures grouped by cause. This sets the order of everything after.

**Phase 1 - Honest refusal.** Read `effFlagsIsSynth`; refuse instruments,
0-output plugins and unsupported layouts before Add with a plain message, in
the picker, REST and MCP. Remember a load-time refusal per file fingerprint
so the list shows it next time.

**Phase 2 - VST2 host conformance.** Fill the callback and dispatcher gaps in
the table above: `canDo` answers on both sides, `ioChanged` (re-read
layout and latency, PLUG-05), automation notifications to the parameter
cache, display update, vendor/product strings, bypass, programs, start/stop
process, old `process` fallback, shell plugins (one DLL, many effects: scan
lists each, load by sub-ID), `.fxp`/`.fxb` import. Each opcode gets a fixture
test with the literal opcode value pinned.

**Phase 3 - VST3 host conformance.** Multi-class modules (pick by class ID,
list each effect), `IComponentHandler2`, unit and program lists, context menu
stub, content-scale support, restart flags (`kIoChanged`, `kParamValuesChanged`,
`kReloadComponent`), single-component plugins, state fallback for plugins that
report state unsupported (parameter snapshot), 64-bit sample request refused
correctly, single-file `.vst3` DLLs. Run Steinberg's validator plus the census.

**Phase 4 - Layouts.** Negotiate what the plugin offers instead of requiring a
match: mono plugin on a stereo node (dual mono or mid), stereo plugin on a mono
node (duplicate and sum), multichannel refused with a reason. Backend owns the
rule; `tool_combinations.rs` covers every layout.

**Phase 5 - Sidechain.** A second input port on plugin nodes whose plugin
declares an auxiliary bus (VST3) or more than two inputs (VST2); the route
compiler feeds it; the UI connects to that port instead of adding a Mixer.
REST/MCP parameters and recipe. (The #136 design item.)

**Phase 6 - Editors.** Editor on a stopped session (worker started for
configuration only, no audio device), user-resizable editors, keyboard focus,
monitor and DPI moves, vendor dialogs owned by the editor window. Shell run by
eye in three themes is not needed; real plugins on the built exe are.

**Phase 7 - Shared VST3 worker (optional).** Chain VST3 effects in one worker
as VST2 does, only if the census shows latency or memory complaints.

**Phase 8 - Only if D1/D2 are approved.** MIDI source + instruments; 32-bit
workers.

**Phase 9 - Release gate.** Census pass rate recorded per release; compatibility
page generated from the census; fresh-install check; live route by ear for
three plugins of different frameworks; known issues published.

## Validation

- Census and matrix: Windows host, real workers, no audio device.
- Fixture plugins for each opcode and interface (unit level).
- `source_combinations.rs` / `tool_combinations.rs` for new ports and layouts.
- `ControlPlane::dispatch` and `mcp_tool_call` for each API change; contract drift check.
- Built exe on a live route before each handoff; sine-continuity harness for worker changes.

## Risks

- Vendor licence checks and demo dialogs block unattended runs (seen with
  Blue Cat and UA); the census must report, not click.
- Each added callback answer can change a plugin's behavior; change one
  answer at a time and rerun the census.
- Third-party binaries stay user-owned; none are committed or redistributed.
- Sidechain and layouts change the graph contract and saved sessions
  (migration needed).

## Rollback

Each phase is its own branch and PR; reverting one leaves the others. The
census is test-only code.

## Next action

Owner answers D1-D4. Then start phase 0 on a new branch.

## P8a audit: is the worker protocol bitness-neutral? (2026-11-13, read-only)

Findings from the code; nothing built or run.

- **Audio transport is neutral.** `SharedAudioTransport` is a file-backed
  `memmap2` region with a fixed byte layout (`u16/u32` fields, `f32` samples, no
  pointers or `usize` on the wire). A 32-bit worker can map the same files.
- **Control messages are neutral.** Frames are `u32`-length-prefixed
  little-endian messages (`lib.rs:2532`, `2556`) over the worker's pipes.
- **ABI declarations are already correct for x86.** VST2 callbacks are
  `extern "C"` (cdecl, what VST2 uses on x86) and every Win32 import is
  `extern "system"` (stdcall on x86). `AEffect` uses pointer-sized Rust types,
  so recompiling for `i686-pc-windows-msvc` gives the 32-bit layout.
- **Host editor handle.** The editor parent is an HWND carried as `usize`; a
  64-bit shell's top-level HWND is a 32-bit-safe value, so a 32-bit worker can
  parent to it (Windows guarantees this for HWNDs). Check with a test anyway.
- **Everything that refuses x86 today** (the places to change):
  `inspect_plugin` returns `UnsupportedArchitecture` (`lib.rs:610`);
  `PluginIdentity::compatibility` requires `X64` (`lib.rs:419-428`);
  `verify_current` and the chain check repeat the x64 test (`lib.rs:1464`,
  `1475`); the worker file names are chosen in `control/src/plugins.rs:24-27`
  with no architecture; `instance_windows.rs:356` lists the workers by name;
  `PluginCompatibility` has no x86 value in the API schema
  (`api_output_schema.rs:1162`, `2423`), so adding `supportedVst2X86`/
  `supportedVst3X86` needs contract drift handling.
- **Toolchain.** Only `x86_64-pc-windows-msvc` is installed; Visual Studio 18
  has the x86 libraries. `rustup target add i686-pc-windows-msvc` is needed for
  the pinned 1.96.0, and the C++ VST3 worker needs an x86 CMake build of
  `tools/m06-vst3-worker`. A toolchain target is an install, not a version bump.
- **Chains.** A chain worker is one process, so members must share a bitness;
  the grouping rule splits by architecture.

Next steps for P8a, in order: (1) add the x86 values to the compatibility enum,
schema and UI badge text; (2) route `inspect_plugin` and the worker-file choice
by architecture, with a fake worker name in tests; (3) split chain grouping by
bitness; (4) install the i686 target and build the x86 Rust worker; (5) build the
x86 VST3 worker; (6) update `build-desktop.ps1`, the release workflow, the
installer and `instance_windows.rs`; (7) run a real 32-bit plugin (needs one
installed; the census corpus should include a 32-bit VST2 and VST3).

## P8a results (2026-11-13): 32-bit plugins

Done and exercised on this PC (Windows 11, release x86 worker built with the
pinned 1.96.0 toolchain plus the `i686-pc-windows-msvc` target, now listed in
`rust-toolchain.toml`):

- `PluginCompatibility` gains `SupportedVst2X86` / `SupportedVst3X86`
  (`supportedVst2X86` / `supportedVst3X86` in the API schema, `contracts`, and
  the UI's `SUPPORTED_PLUGIN_COMPATIBILITY`). Contract drift check passes.
- Defect found by the first real 32-bit run: the PE export walk only
  understood PE32+ headers, so every 32-bit VST2 DLL was `Unknown` ->
  unsupported. Fixed with a PE32 branch; regression
  `classifies_a_32_bit_vst2_dll_from_its_pe32_exports`.
- Defect found by the first 32-bit link: `GetWindowLongPtrW` is not exported
  by 32-bit user32. Wrapped in `window_long` (cfg by pointer width).
- Worker choice by bitness: `PluginWorkerKind::for_identity`; files
  `audiorouter-plugin-worker-x86.exe` and `audiorouter-vst3-worker-x86.exe`
  (override variables `AUDIOROUTER_PLUGIN_WORKER_X86_PATH`,
  `AUDIOROUTER_VST3_WORKER_X86_PATH`). A chain never mixes bitness: x86 VST2
  plugins each get their own 32-bit worker.
- Packaging: `tools/build-desktop.ps1`, `tools/release/prepare-artifacts.ps1`,
  both Tauri resource lists, `tools/m06-vst3-worker/build.ps1 -Arch x86`; the
  packaging fixture tests pass.
- Evidence: the ReaPlugs 2.36 **32-bit** build (free, installed to a temp
  folder; PE machine 0x14c) through `vst2_chain_matrix` with
  `AUDIOROUTER_CHAIN_WORKER` pointing at the x86 worker: 6 plugins (ReaComp,
  ReaEQ, ReaGate, ReaDelay, ReaXcomp, ReaFIR), 22 combinations of one to three
  plugins, mono and stereo, 44 runs: start, audio, configure, state, editor
  open/close with audio running, restore all passed; problems: none (227 s).
  The dispatcher path (`plugins.scan`, `plugins.parameters`) also ran with the
  x86 worker.

Not done / not proven:

- No 32-bit **VST3** plugin was available, so the x86 VST3 worker is built
  (PE machine 0x14c, 390 KB) but has never loaded a plugin.
- The release exe was not rebuilt or run with these changes; the NSIS bundle
  was not built; CI has to install the i686 target (the toolchain file now
  asks for it).
- A shell-level run by eye (editor of a 32-bit plugin inside the real window)
  is open.
- 32-bit VST2 chains in one worker work in the test but production groups
  only x64 plugins; each x86 plugin runs in its own worker.
- Instrument classification for x86 VST3/UA bundles uses the same metadata
  path as x64.
