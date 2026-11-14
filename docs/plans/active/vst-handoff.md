# VST work: handoff for the next agent (2026-11-14)

Read this page first, then [the plan](vst-full-compatibility.md) (decisions,
phase definitions, evidence so far) and
[the instrument design](vst-midi-instruments-design.md). `AGENTS.md` rules
apply in full; the ones that bit this work are repeated under "Rules" below.

## Where the work is

- Worktree: `C:\Users\miste\.t3\worktrees\audiorouter\feature-vst-plugin-expe-f8a7ad7c0183990c`
- Branch: `feature/vst-plugin-experience`, 34 commits ahead of `main`, head
  `dc8e3fa5b` plus the commit that adds this page. Working tree clean.
- **Nothing is pushed. There is no PR.** Both need the owner's explicit approval.
- The main checkout `C:\code\audiorouter` is on another branch (`fix/resilience`)
  owned by another session. Do not work there.

## Owner decisions that stand

- "All VST2 and VST3": effects **and** instruments with MIDI (D1), 32-bit too
  (D2), spec already revised (D3), test corpus is free and demo plugins (D4).
- Commit per phase, stage files by name, no Claude attribution.
- **Never run Blue Cat plugins** (demo popup). The census skips them unless
  `AUDIOROUTER_CENSUS_RUN_DEMOS=1`; list a folder before pointing any loader
  at it. `C:\Program Files\VSTPlugins` contains Blue Cat PatchWork.
- The owner is often gaming or on a call on this PC. Nothing may pop up or
  play through the speakers without asking: no shell launch, no editor
  windows, no audible route. Heavy builds were approved ("go now") but ask
  again if the owner says they are on a call.

## What is done

| Phase | State | Proof |
| --- | --- | --- |
| P1 instrument detection | VST2 at load (`effFlagsIsSynth`), VST3 from `moduleinfo.json`, learned refusals saved to `%LOCALAPPDATA%\AudioRouter\learned-instruments.txt` | unit tests |
| P8a 32-bit | x86 Rust and C++ workers, routing by bitness, packaging scripts, CI installs `i686-pc-windows-msvc` | real 32-bit ReaPlugs, Pitchproof, Dragonfly, Zam |
| P0 census | `plugin_census` tool, two corpus runs (about 75 plugin builds) | reports in the plan |
| P8b instruments and MIDI | `NodeKind::Instrument`, both workers, `midi.devices`, `midi.sendEvents`, MCP `list_midi_devices` and `play_notes`, hardware MIDI input (winmm), UI picker group and Properties keyboard, docs | VST2 test synth and Surge XT (VST3) play through `ControlPlane::dispatch`; browser test in three themes |
| P2 (part) | host callback opcodes fixed and filled; `effStartProcess`/`effStopProcess`; `ioChanged` suspends and resumes | pinned-opcode unit tests |
| P3 (part) | `ARST2` state envelope with parameter snapshot fallback | round trip, and the fallback itself with Dragonfly Plate Reverb (`vst3_state_fallback`) |
| P6 (part) | editor opens on a stopped session | Pitchproof, Dragonfly through dispatch |

## What is open, in the order to do it

Each item says what "done" needs. Items marked **owner** cannot be finished
without the owner; prepare everything else and then ask once, with one list.

### A. Must fix before any PR (small, no audio, no popups)

1. **Done 2026-11-14 (after this page was first written).** The Instrument
   tool now has guide content (`site/guide-content/routing.json`,
   `tutorials.json`), a generated page `site/tools/instrument.html`, an in-app
   guide link (UI and shell allowlists), a silent Properties tour recorded
   headlessly from the route harness (`instrument-overview.mp4`: picks a MIDI
   device, presses C4, E4, G4, Stop all notes), and four manifest rows exempted
   with a reason (a prepared recording cannot stand in for an installed
   instrument). `check-tool-guide-catalog`, `check-tool-guide-property-coverage`
   (only the older `plugin.recovery` row is pending, as on `main`),
   `check-tool-guide-videos`, `site/check-guides.mjs`, the site node tests and
   the headless Firefox playback check for `GUIDE_KIND=instrument` pass. Not
   done: an audible recording with a real instrument (**owner**). To rerun the
   recorder in a fresh worktree, create `target/tool-guide-tools/render-report.json`
   and `target/tool-guide-properties/render-report.json` as `[]`, start the
   harness (`npm --prefix ui run dev -- --host 127.0.0.1 --port 4186 --strictPort`),
   then `GUIDE_KIND=instrument node tools/site/record-tool-guide-overviews.mjs`
   and `GUIDE_KIND=instrument node tools/site/build-tool-guide-review.mjs`,
   then `site/generate-catalogs.mjs`, `generate-social.mjs`, `generate-csp.mjs`
   (the last two put back headers the guide generator drops). Also fixed: the
   plugin window hint promised "sliders below" on an Instrument, which has none
   (test in `InstrumentNodeEditor.test.tsx`). Original note:
   `node tools/site/check-tool-guide-catalog.mjs`
   fails: "Discovery snapshot is stale: crates/control/src/catalog.rs". Run
   `node tools/site/export-tool-guide-catalog.mjs`, then add the Instrument
   tool to `tools/site/tool-guide-families.json` and
   `tool-guide-property-manifest.json` (`midiInput`, the on-screen keyboard,
   "Stop all notes"), and run `check-tool-guide-property-coverage.mjs`,
   `site/generate-catalogs.mjs`, `site/check-guides.mjs`. A property may be
   recorded or carry a written exemption; say which.
2. **Done 2026-11-14:** `crates/plugin-host/tests/vst3_state_fallback.rs`, run
   with Dragonfly Plate Reverb (see the plan, P3 step 1). Original note: Add a "reject restore" switch to a test
   path so a plugin with parameters refuses its state, and assert the
   parameters come back. Simplest: an environment variable read only by the
   VST3 worker in debug builds, or a small fixture VST3. Without this the
   feature is written, not exercised.
3. **Rebuild the release and x86 workers** (`tools/build-desktop.ps1`) and
   rerun `fresh_install_shell` with `AUDIOROUTER_SHELL_EXE`. The binaries in
   `src-tauri/target/release` (00:15 to 00:18 on 2026-11-14) predate the P2
   and P3 changes.
4. Full local gate (commands under "How to check").

### B. P2 remainder: VST2 plugin-side opcodes

File: `crates/plugin-host/src/vst2.rs` (worker: `src/bin/audiorouter-plugin-worker.rs`).
One opcode per commit, each with a recording-dispatcher test that pins the
literal number (pattern: `start_and_stop_process_bracket_the_mains_switch`,
`host_opcodes_match_the_public_aeffectx_numbering`), then rerun the census.
The opcode numbers below are from memory: check each against the public
`aeffectx.h` before use (two host opcodes were wrong for months).

- Done 2026-11-14: `effCanDo` (51), used to refuse an instrument that says no
  to both MIDI questions. `effSetBypass` is deliberately not used (decision in
  the plan, P2 step 3).
- Programs: `effGetProgram`/`effSetProgram` (3, 2), `effGetProgramNameIndexed`
  (29). Needs an API surface (list and select a preset): full AUTO-18 if added.
  If the owner does not want presets yet, record that as a decision.
- Shell plugins: `effGetPlugCategory` (35) = `kPlugCategShell` (10), enumerate
  with `effShellGetNextPlugin` (70), load by sub-ID through
  `audioMasterCurrentId`. Scan must list each sub-plugin without running
  plugin code in the app process (do it in the worker). No free shell plugin
  is in the corpus; write a fixture DLL like the test synth.
- `.fxp`/`.fxb` import: parse the header (`CcnK`, `FxCk`/`FPCh`/`FxBk`/`FBCh`),
  bounds-checked, then `effSetChunk` or parameters. Needs API + MCP + UI.
- Old `process` fallback for plugins without `canReplacing`: today refused.
  Either implement (accumulating call into a zeroed buffer) or keep the
  refusal with a plain reason; no such plugin was seen in the census.

### C. P3 remainder: VST3 worker

File: `tools/m06-vst3-worker/main.cpp`, build with
`tools/m06-vst3-worker/build.ps1 -Output <path>` (add `-Arch x86` for 32-bit).

- Restart flags in `IComponentHandler::restartComponent`: `kLatencyChanged`,
  `kIoChanged` (deactivate, re-read buses, reactivate), `kParamValuesChanged`
  (re-read cache), `kReloadComponent`.
- `IComponentHandler2` (`setDirty`, `requestOpenEditor`, group edits),
  `IUnitInfo`/program lists (read only), context menu stub.
- `IPlugViewContentScaleSupport`: send the monitor scale before `attached`.
- Multi-class modules: list every audio effect class at scan, load by class ID
  (the identity already carries `classIds`). Surge XT Effects and Zam are
  single class; check a multi-class bundle from the corpus.
- 64-bit sample size: confirm `kSample32` is always set up and a plugin that
  only does `kSample64` is refused with a reason.
- Steinberg validator: run it against the corpus as extra evidence. Remember
  the validated lesson: validator success is not AudioRouter evidence.
- P1 follow-up: a worker "describe" mode that reads the class category
  (`Instrument`) from the factory without creating the component, so a VST3
  without `moduleinfo.json` (Surge XT) is listed as an instrument from the
  first scan. This runs plugin code during a scan, so it must be in the
  isolated worker with a timeout, never in the app process.

### D. P4 layouts (mostly verification)

VST2 already adapts mono and stereo (`channel_layouts_are_mappable`,
`spread_graph_to_plugin_inputs`). Do: the same for VST3 (bus arrangement
negotiation with `setBusArrangements`, fall back to what the plugin offers),
a plain refusal reason for multichannel-only plugins, and
`crates/engine/tests/tool_combinations.rs` still green.

### E. P5 sidechain (largest open design item)

- Domain: a second input port (`sidechain`) on plugin nodes whose plugin
  reports an auxiliary input bus (VST3) or more than two inputs (VST2).
  Saved sessions without the port must still load (no migration that edits
  user files; treat a missing port as "not connected").
- Engine: the route compiler feeds that port; unconnected means silence.
  `tool_combinations.rs` must cover the plugin with and without the port in
  every Mixer layout (owner rule: no hand-kept kind lists, exhaustive matches).
- Workers: the VST3 worker has a test-only auxiliary-bus gate; make it real.
  Shared audio transport needs a second input region; keep it bitness-neutral.
- UI: connecting a second source to the sidechain port must not insert a
  Mixer (UI-03). Three themes, no layout shift.
- AUTO-18 in full: schema in `node_parameter_schema`, recipe, MCP reachability
  test, regenerated API reference and site catalogs.
- Write the design in the plan before code, as was done for P8b.

### F. P6 remainder: editors

User-resizable VST3 editors (`IPlugView::canResize`, `checkSizeConstraint`,
`onSize`), keyboard focus into the child window, monitor and DPI moves, vendor
dialogs owned by the editor window. Tests need a pumped parent window and
audio flowing (validated lesson 2026-11-12). The by-eye part is **owner**.

### G. P7 shared VST3 worker

Optional by the plan: "only if the census shows latency or memory complaints".
None has been seen. Record that decision with the census numbers instead of
building it, unless the owner asks.

### H. P9 release gate

- Widen the census corpus (free): Melda free bundle, TDR, Valhalla free,
  Voxengo free, Airwindows, Analog Obsession, Chow, Kilohearts free. Download
  each into its own new folder under
  `C:\Users\miste\AppData\Local\Temp\vstcorpus`; never commit a binary.
- Generate the compatibility page from the census JSON
  (`docs/operations/plugin-compatibility.md` plus a site page), with pass rate
  per release and known issues (ZamAutoSat state, PolyMAX needing UA Connect
  signed in, Surge XT listed as an effect until first refusal).
- Fresh-install check on the rebuilt exe.

### I. Needs the owner (ask once, together)

- Run the built shell with an instrument: add Surge XT, press keys, open its
  editor window; and three plugins of different frameworks on a live route by
  ear. Plays through the owner's speakers.
- Record the Instrument guide video (real Properties panel, portrait 9:16,
  production audio) and run the Firefox playback check.
- A hardware MIDI keyboard (none on this PC): the winmm input path is only
  unit-tested with a fake source.
- PolyMAX with UA Connect signed in (output is exactly 0 when signed out).
- CI: the i686 target step in `ci.yml` and `manual-release.yml` has never run
  on GitHub; it needs a push, then read every check annotation.
- Approval to push and to open the PR.

## How to check

Run from the worktree. Opt-in tests are `#[ignore]`; pass `-- --ignored <name>`.

```
cargo fmt --all -- --check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test -p audiorouter-engine --test source_combinations --test tool_combinations
npm --prefix contracts run check:drift
npm --prefix ui run format && npm --prefix ui run lint && npm --prefix ui test
node tools/site/check-tool-guide-catalog.mjs
```

Real-plugin tests (set the worker paths to `target/debug/*.exe` first:
`AUDIOROUTER_PLUGIN_WORKER_PATH`, `AUDIOROUTER_VST3_WORKER_PATH`, and the
`_X86_PATH` pair for 32-bit):

| Test | Variables |
| --- | --- |
| `plugin_census` (`crates/control/src/plugin_census_tests.rs`) | `AUDIOROUTER_CENSUS_DIRS`, `AUDIOROUTER_CENSUS_SKIP` (`,` or `;`), `AUDIOROUTER_CENSUS_REPORT` |
| `an_instrument_node_starts_a_worker_and_plays_midi` | `AUDIOROUTER_TEST_INSTRUMENT` or `AUDIOROUTER_TEST_VST2_INSTRUMENT` |
| `the_editor_opens_while_the_session_is_stopped` | `AUDIOROUTER_TEST_PLUGIN` (opens a window: ask first) |
| `vst3_state_fallback` (`crates/plugin-host/tests`) | `AUDIOROUTER_TEST_VST3_WORKER`, `AUDIOROUTER_TEST_VST3_EFFECT` (a plugin that refuses a damaged state, e.g. Dragonfly) |
| `vst2_chain_matrix` (`crates/plugin-host/tests`) | `AUDIOROUTER_CHAIN_WORKER`, `AUDIOROUTER_CHAIN_FILTER`, `AUDIOROUTER_CHAIN_SKIP` |
| `ui/e2e/instrument-editor.pw.ts` | none (route harness) |

Corpus on this PC: `C:\Users\miste\AppData\Local\Temp\vstcorpus` (Surge XT,
RipplerX, Zam, Dragonfly 64 and 32-bit). A temp folder can be cleaned by
Windows; download again if it is gone.

## Rules that already cost time here

- Write code with Edit/Write. Shell heredocs, `sed` and `node -e` strings
  collapse `\\` in Windows paths and TS strings; this happened four times.
- Never chain `git commit` after a check with `;`. A commit went in with
  Clippy failing. Use `&&`, and read the check output first.
- Python is not installed; use Node or PowerShell for scripts.
- The census skip list once used the wrong separator and ran Blue Cat. Check
  what a loader will open before running it.
- A release tail is not a defect: Surge XT takes about a second to fall
  silent after all notes off; tests wait for silence.
- Plugin output of exactly 0 with no error usually means a licence state.
  Read the vendor's log (`%LOCALAPPDATA%\Universal Audio\Logs`) before
  changing host code.
- Rebuild `target/debug/examples/e2e_backend.exe` before real-backend browser
  tests, and add new method families to its allowlist.
- Update the plan after each step with what ran and what did not. Do not say
  a phase is done when only its code is written.

## Suggested first session

A1 (catalog and manifest), A2 (fallback test), then B `effCanDo` and
`effSetBypass`, then C restart flags and content scale, rerunning the census
after each worker change. Write the P5 design last in that session and give
the owner the single "needs the owner" list from section I.
