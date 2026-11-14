# Full VST2 and VST3 compatibility

Status: **approved 2026-11-13, in progress.** Owner request: "support ALL VST2
and 3".

**Taking over this work? Start with [the handoff page](vst-handoff.md):** what
is done, what is open in order, how to check, and what needs the owner.

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
| P8a 32-bit | done: VST2 and VST3 exercised with real 32-bit plugins (ReaPlugs, Pitchproof, Dragonfly); release exe/installer not rebuilt |
| P0 census | tool written; first corpus run (Dragonfly, ReaPlugs, Pitchproof, UADx) all pass; wider corpus pending |
| P8b instruments and MIDI | engine, control plane, API and MCP, scan flip, UI picker and Properties keyboard committed on this branch; see [the design](vst-midi-instruments-design.md). Not yet run with a real hardware keyboard, a rebuilt exe, or recorded guides |
| everything else | not started (see the open list below) |

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

## P0 census, first results (2026-11-13)

Tool: `plugin_census` in `crates/control/src/plugin_census_tests.rs` (opt-in,
Windows). It walks folders of VST2 DLLs and VST3 bundles, 64-bit and 32-bit,
and drives each supported effect through the dispatcher (`plugins.scan`,
`plugins.parameters`) and the real worker for its format and bitness: start,
100 blocks, save state, restore, restore into a fresh worker, at mono and
stereo. Nothing is asserted, so one bad plugin does not hide the others; the
report is JSON plus a markdown table. It opens no audio device or editor.

Corpus so far (all free): Dragonfly Reverb 3.2.10 (Early, Hall, Plate, Room as
VST2 and VST3, 64-bit and 32-bit), ReaPlugs 2.36 (64-bit installed, 32-bit in a
temp folder), Pitchproof (x64 and x86 VST2), UADx free titles (VST3 x64).
Not run: Blue Cat (demo dialog), the rest of the planned corpus (Melda,
TDR, Valhalla, Voxengo, Airwindows, Surge XT, Chow).

Result: all 16 Dragonfly builds (4 effects x VST2/VST3 x x64/x86), the UADx
effects, Pitchproof x64 and x86 and the ReaPlugs started, processed, saved
state and restored at mono and stereo. Notes: ReaComp, ReaDelay, ReaEQ,
ReaGate and ReaXcomp are reported "pass-through" because their default
settings do not change a quiet tone (not a defect); PolyMAX is an instrument
(refused).

Defect found by the census: 32-bit VST3 bundles keep their binary in
`Contents/x86-win`, which both inspection (`resolve_binary_path`) and the VST3
worker (`resolve_binary`) did not look in ("Missing"). Fixed in both;
regression `inspects_a_32_bit_vst3_bundle_under_x86_win_contents`; verified by
loading the four 32-bit Dragonfly VST3 effects in the 32-bit VST3 worker.

### Census run 2 (2026-11-13): wider free corpus

Added: FILT-R, TIME-12, RipplerX (tiagolr, JUCE), Zam plugins 4.5 (DPF, 64-bit
and 32-bit, VST2 and VST3), Surge XT nightly (VST3 plugins). 160 files or
bundles walked (80 are LV2 helper DLLs or instruments, counted as "not a
supported effect"); about 75 plugin builds ran through the real workers.

- Passed everything: FILT-R, TIME-12, all Zam effects in all four
  variants (VST2/VST3 x x64/x86), Surge XT Effects.
- **ZamAutoSat.vst3** (x64 and x86): saves 17 bytes of state, but the plugin
  rejects its own state on restore, so the host reports `StateUnsupported`
  (documented capability limitation; the route keeps running). Plugin quirk,
  not a host crash; keep recorded.
- **Surge XT.vst3** is an instrument but the scan lists it
  `supportedVst3X64`; the worker then refuses it at load with the plain
  message. Same scan-time classification gap as the VST2 case (P1 follow-up).
  `RipplerX.vst3` is correctly classified `unsupportedInstrument` at scan.
- Still unproven: editors for VST3, sidechain, host callbacks (P2/P3), anything
  needing MIDI (waits for P8b).

## Where this stands (end of 2026-11-13 session)

Done and committed: decision record and spec revision; instrument refusal for
VST2 at load; 32-bit VST2 and VST3 hosting (workers, detection, packaging
scripts), exercised with real 32-bit plugins; census tool plus two corpus
runs. Not started: P8b MIDI source and instrument nodes (the largest piece),
P2/P3 conformance gaps (opcodes and interfaces listed above), P4 layouts, P5
sidechain, P7 shared VST3 worker and P9 release gate. Added later the same day
(see the sections below): P2 step 1 (VST2 host callback), P6 step 1 (editor
before Play) and the packaging check (next section). The design for P8b is in
[vst-midi-instruments-design.md](vst-midi-instruments-design.md).

## P2 step 1 (2026-11-13): VST2 host callback

- **Defect:** the host answered `getSampleRate`/`getBlockSize` on opcodes 10 and
  11 (`tempoAt`, `getNumAutomatableParameters`); the real numbers are 16 and
  17 (public `aeffectx.h`, checked against LMMS's copy). A plugin that asks the
  host for its sample rate (common in JUCE and iPlug VST2) received 0.
  Fixed; `host_opcodes_match_the_public_aeffectx_numbering` pins the
  literals.
- Added the answers a conforming host gives: vendor and product string
  ("AudioRouter"), `canDo` (yes: sizeWindow, acceptIOChanges, startStopProcess,
  supplyIdle, sendVstTimeInfo; no: receive events, offline, file selectors,
  shell category; unknown stays 0), language, pin connected, automation state,
  replace-or-accumulate, latency queries, update display, begin/end edit,
  need idle, `ioChanged` (sets `io_changed` on the context for the worker to
  read) and `wantMidi`.
- Verified: unit tests; census of the VST2 plugins in `C:\Program Files\VSTPlugins`
  (ReaPlugs, Pitchproof), Dragonfly and 32-bit ReaPlugs: no change in results.
- P2 step 2 (2026-11-14): `effStartProcess`/`effStopProcess` bracket the mains switch (setup, format change, close) and `ioChanged` now suspends and resumes the plugin once per request (`apply_io_change`), both pinned by a recording-dispatcher test.
- P2 step 3 (2026-11-14): `effCanDo` (51). The host asks an instrument
  `receiveVstEvents` and `receiveVstMidiEvent` when it loads; it refuses the
  plugin ("reports that it does not accept MIDI notes") only when both answers
  are an explicit no, because most plugins answer "don't know" to features they
  support. Unit test with a table-driven dispatcher pins the opcode and the
  rule; the real VST2 instrument worker test still passes (the test synth
  answers "don't know").
- **Decision, `effSetBypass` (44): not used.** Node bypass is done by the host:
  a bypassed plugin is not processed at all, which works for every plugin and
  cannot be defeated by a plugin that ignores the request. Asking the plugin to
  bypass itself would put the protected-voice rule in the plugin's hands.
- Open in P2: programs, shell plugins, `.fxp/.fxb`, old `process` fallback.

## P6 step 1 (2026-11-13): editor before Play

- `plugins.openEditor` on a stopped session starts that one plugin's worker
  without a route (`start_editor_only_plugin`, held in `editor_only_plugins`),
  opens its editor, and `plugins.closeEditor` captures the state and ends the
  worker; starting the route releases any such worker first; a failed open
  releases it too. No new API method, so no schema or MCP change.
- The UI's "Open plugin window" no longer waits for Play (still needs the
  desktop app, which owns the window).
- Exercised through `ControlPlane::dispatch` with real workers and a
  message-pumping parent window: Pitchproof (VST2) and Dragonfly Plate Reverb
  (VST3), `the_editor_opens_while_the_session_is_stopped` (opt-in).
- Not run: the desktop shell's own window by eye; the shell's
  `open_plugin_editor` command path with the stopped session (the shell code
  has no running check, per grep, but it was not launched).
- Still open in P6: user-resizable editors, keyboard focus, monitor/DPI moves,
  vendor dialogs owned by the editor window.

## Packaging check with the x86 workers (2026-11-13)

- `tools/build-desktop.ps1` (release, with the UI build) finished with exit 0
  and verified six binaries in `src-tauri/target/release`: shell, CLI,
  `audiorouter-plugin-worker`, `audiorouter-vst3-worker` and the two `-x86`
  workers. SHA-256 values are in the script output (shell `F1CD195B...`,
  x86 Rust worker `B2FD867E...`, x86 VST3 worker `F1E8236F...`).
- The shell exe (23:20) is newer than `ui/dist/index.html` (23:19), and the
  embedded bundle contains `supportedVst3X86`.
- `fresh_install_shell` (`AUDIOROUTER_SHELL_EXE` = that exe, `--ignored`; new
  database, no developer variables, no other shell running): passed. The backend
  answered `status.get` 134 ms after launch; a raw caller is refused
  `DeviceAdministration`, as intended.
- Not run: the installer build (`prepare-artifacts.ps1`) and its hook test; CI
  still needs the i686 target installed (`rust-toolchain.toml` lists it, the
  workflows were not changed or run); the shell was not driven by eye.

## State at end of 2026-11-14 session

Committed on `feature/vst-plugin-experience` (nothing pushed, no PR): all of
P8b except the items below. Evidence is in
[vst-midi-instruments-design.md](vst-midi-instruments-design.md). Workspace
tests, both Clippy runs, both `cargo fmt --check`, the contract drift check,
UI typecheck, lint, format and 1065 unit tests, and the Instrument browser test
in three themes all pass. P4 note: the VST2 worker already adapts mono/stereo
graphs to plugins of other widths (`channel_layouts_are_mappable`,
`spread_graph_to_plugin_inputs`), so P4 is mostly VST3 verification and the
multichannel refusal reason, not new code.

Open, in suggested order:

1. P8b: guide video, site page, property coverage manifest and Firefox check for
   the Instrument tool (AGENTS.md "Static website writing"); run the shell exe
   by eye with an instrument and the editor window; a real MIDI keyboard.
2. P1 follow-up: recognise a VST3 without `moduleinfo.json` before its first
   failed Play (needs a worker describe mode; weigh against "a scan never runs
   plugin code").
3. P2 remainder (plugin-side VST2 opcodes: bypass, programs, `effCanDo`, shell
   plugins, `.fxp/.fxb`, act on `io_changed`, old `process` fallback).
4. P3 VST3 conformance (`IComponentHandler2`, units and programs, content
   scale, restart flags, parameter-snapshot state fallback; Steinberg validator).
5. P5 sidechain, P6 remainder (resizable editors, focus, DPI), P7 shared VST3
   worker (only if the census shows a need), P9 release gate and compatibility
   page, wider census corpus. (CI i686 target: added to `ci.yml` release job and `manual-release.yml` on 2026-11-14; not yet seen running on GitHub.)
6. Before a PR: read the check annotations and ask the owner for approval.

## P3 step 1 (2026-11-14): VST3 parameter-snapshot state fallback

- The VST3 worker's saved state is now envelope `ARST2`: the old component and
  controller sections plus the controller's writable parameter values. `ARST1`
  states still restore. If a plugin rejects its own component/controller state,
  the worker puts the snapshot values back (controller `setParamNormalized`,
  and a parameter change at the start of the next block) instead of reporting
  `StateUnsupported`.
- Exercised: census round trip (save, restore, restore into a fresh worker) with
  Dragonfly Plate Reverb and Surge XT Effects: ok, same as before.
- **Fallback exercised (2026-11-14, later):** `crates/plugin-host/tests/vst3_state_fallback.rs`
  (opt-in) sets a parameter on Dragonfly Plate Reverb (0.8 to 0.125), saves,
  then gives a fresh worker a state whose component section is damaged. With no
  values beside it the worker reports the state-unsupported limitation; with
  the saved values the restore succeeds and the parameter reads 0.125 again.
  Passed with the debug worker. The same test fails its "really refuses" step on
  Surge XT Effects, because that JUCE plugin accepts a damaged state without an
  error; use a plugin that refuses (DPF plugins do). ZamAutoSat still reports
  `restore failed`: it has no parameters to fall back on.

## P3 step 2 (2026-11-14): component handler

- The VST3 worker's handler now also answers `IComponentHandler2` (`setDirty`,
  group edits: accepted; `requestOpenEditor`: declined, the host opens editors
  only when the user asks).
- `restartComponent`: `kIoChanged` and `kReloadComponent` now get the same
  off-and-on re-activation as `kLatencyChanged`. `kParamValuesChanged` needs no
  action (the worker keeps no copy of the values). Limit: bus counts and layouts
  are fixed when the worker starts, so a plugin that changes them keeps the
  layout it was given; a full reload (terminate, initialise, restore state) is
  not done.
- Already present, found while reading: `IPlugViewContentScaleSupport` is sent
  the window's DPI scale when the editor is attached.
- Checked: `audiorouter-vst3-worker.exe --self-test-host` (new assertions for
  the second interface and each restart flag); census of Dragonfly (VST2 and
  VST3) and Surge XT Effects with the rebuilt debug worker: same results as
  before. Not checked: a real plugin that sends `kIoChanged` (none in the corpus
  is known to); the x86 and release workers are not rebuilt.
- Open in P3: units and program lists, multi-class modules beyond the first
  class check, 64-bit-only plugins, the Steinberg validator run.

## P1 follow-up (2026-11-14): VST3 instruments without metadata

- A VST3 bundle that lists no audio classes in `moduleinfo.json` (or has no such
  file) is now classified from the class category string in its binary: a string
  that starts with `Instrument|` and a sub-category, such as `Instrument|Synth`.
  No plugin code runs. The bare word "Instrument" is ignored (JUCE and UA effects
  contain it) and so is `Fx|Instrument`. A class list in the metadata is still
  trusted over the search.
- Measured on this PC: Surge XT, RipplerX and PolyMAX contain the string; 30
  effects (Dragonfly, Zam, Surge XT Effects, FILT-R, TIME-12, eight UADx) do not.
- Checked: unit test `a_vst3_without_metadata_is_classified_from_its_category_string`;
  the real files through `inspect_binary` in `vst3_instrument.rs` (Surge XT is an
  instrument, Surge XT Effects is not, and both still load in their own role).
  So Surge XT is listed under Instruments from the first scan.
- Limits: a module with both an effect class and an instrument class is listed
  as an instrument; a VST2 instrument is still only learned at its first load.
- Also repaired: two test lines in `vst2.rs` held raw NUL bytes instead of `\0`
  (a shell heredoc had swallowed the backslash); same meaning, now plain text.
