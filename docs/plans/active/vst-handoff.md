# VST work: handoff for the next agent (2026-11-14)

> **Superseded (status note, 2026-11-24).** This handoff is a historical
> snapshot. The VST branch merged to main in PR #164 (`35ecd8e51`). P2, P3, P5
> and P6 software work is implemented and tested on main. The current state
> and the remaining owner-only checks are in the reconciled table of the
> [completion work plan](vst-completion-work.md) and in
> [VST owner checks](vst-owner-checks.md): editor focus and DPI by eye,
> listening, sidechain ducking by ear, hardware MIDI, PolyMAX and signing.
> The worktree paths, branch heads and "not all work is complete" statements
> below no longer describe the repository.

Read this page first, then [the plan](vst-full-compatibility.md) (decisions,
phase definitions, evidence so far), the
[completion work plan](vst-completion-work.md) (ordered remaining tasks) and
[the instrument design](vst-midi-instruments-design.md). `AGENTS.md` rules
apply in full; the ones that bit this work are repeated under "Rules" below.

## Where the work is

Latest completion audit (2026-11-14): **not all work is complete**. P2
programs/shell/preset import, P3 units/programs/multi-class qualification,
P5 sidechain graph execution, P6 editor qualification and P9 release gates
remain open. The metadata groundwork and real VST3/VST2 `DescribeBuses` worker paths
are now implemented and tested; see the checkpoint in the
[completion work plan](vst-completion-work.md). These do not finish P5.
The checkout has uncommitted review and discovery work; the older clean-tree
and head statements below are historical. The requested
`C:/code/audiorouter/docs/plans/active/vst-full-compatibility.md` does not
exist in the main checkout; the plan is in this VST worktree. Do not copy it
into the unrelated resilience branch to make that path exist.

- Worktree: `C:\Users\miste\.t3\worktrees\audiorouter\feature-vst-plugin-expe-f8a7ad7c0183990c`
- Branch: `feature/vst-plugin-experience`, 45 commits ahead of `main`, head
  `2483c6010` plus the commit that adds this line. Working tree clean.
- The release build is at `src-tauri/target/release` (shell, CLI and the four
  workers; the x86 pair is beside them), SHA-256 values in
  `target/build-desktop2.out.log`. That exe passed `fresh_install_shell`
  (backend answered 62 ms after launch; a raw caller was denied device
  administration).
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
| P0 census | `plugin_census` tool, three corpus runs (about 75 plugin builds each; run 3 is 343 files, 190 effects and 4 instruments); the wide results are published as [plugin-census.md](../../operations/plugin-census.md) | reports in the plan |
| P8b instruments and MIDI | `NodeKind::Instrument`, both workers, `midi.devices`, `midi.sendEvents`, MCP `list_midi_devices` and `play_notes`, hardware MIDI input (winmm), UI picker group and Properties keyboard, docs | VST2 test synth and Surge XT (VST3) play through `ControlPlane::dispatch`; browser test in three themes |
| P2 (part) | host callback opcodes fixed and filled; `effStartProcess`/`effStopProcess`; `ioChanged` suspends and resumes | pinned-opcode unit tests |
| P3 (part) | `ARST2` state envelope with parameter snapshot fallback | round trip, and the fallback itself with Dragonfly Plate Reverb (`vst3_state_fallback`) |
| P6 (part) | editor opens on a stopped session | Pitchproof, Dragonfly through dispatch |

## Review session (2026-11-14, later)

- Full local gate passed (all 11 steps exit 0, `target/validate.log`), and a
  wide census ran with the corpus listed in the plan: 172 of 190 effects clean,
  Dexed, RipplerX and Surge XT play a note, PolyMAX silent (UA Connect signed
  out), Blue Cat skipped. The published page is
  [plugin-census.md](../../operations/plugin-census.md). Recorded limitations:
  ReaControlMIDI crashes its own worker when restoring its state (contained),
  ProM and ZamAutoSat reject their state, Kars and Nekobi are recognised as
  instruments at load rather than at scan.
- A test left uncompilable by the previous session was moved into `mod tests`
  in `vst2.rs`; the refusal reason for the old `process` call is now its own
  sentence.
- The release build (`tools/build-desktop.ps1`, with the six workers) and the
  `fresh_install_shell` run are redone in this session. The built exe is the
  one to try; nothing is pushed and there is still no PR.
- Still open, unchanged: sidechain (E/P5), resizable editors and keyboard/DPI
  work (F/P6). The owner accepted the visible occupied-input Mixer proposal on
  2026-11-14; it stays. VST2 sidechain is in scope only for pins whose metadata
  explicitly labels them sidechain/key, never by extra-input count.

## Review pass 2 (2026-11-14, later)

Code review found and fixed MIDI stuck-note paths: queue overflow could evict a
note-off after clearing its held bit; panic did not lift sustain/sostenuto;
both workers evicted the oldest MIDI event on backlog; `midi.sendEvents` did
not use the shared optional-session/exact-name resolver; and `midi.devices`
reported a device connected without an open connection or released notes on
unplug. The queue now prioritizes releases and recovery controls, invalidates
scheduled note sequences on panic/overflow, and the workers reject overflow.
New regressions cover these cases. Full `cargo test --workspace` now passes
(including the updated VST2 queue integration test), as do both format checks,
both Clippy runs, contract drift and docs validation. Real debug-worker checks
opened/closed Dragonfly Plate Reverb's editor in a stopped session and played
Surge XT midi through the instrument bridge. The rebuilt release package is at
`src-tauri/target/release` (shell `6ADE13D9...3A306`); `fresh_install_shell` is
blocked because the owner's main-checkout shell is running, and it was not
stopped.

The plan is still **not complete**: P5 graph/runtime sidechain, P6 resizable
editors/keyboard/DPI, P2 programs/shell/`.fxp`/`.fxb`, P3 units/programs/
multi-class/Steinberg validation, and P9 installer/compatibility/live-ear gates
remain unimplemented or unqualified. Do not describe this branch as completing
"all VST2 and VST3" yet.

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

- Done 2026-11-14 (plan, "P3 step 2"): `IComponentHandler2`, restart flags
  (re-activation for latency, I/O change and reload; bus layouts are not
  re-negotiated), and content scale was already there.
- Open: `IUnitInfo`/program lists (read only), context menu stub, re-negotiating
  buses after `kIoChanged`.
- Multi-class modules: list every audio effect class at scan, load by class ID
  (the identity already carries `classIds`). Surge XT Effects and Zam are
  single class; check a multi-class bundle from the corpus.
- 64-bit sample size: confirm `kSample32` is always set up and a plugin that
  only does `kSample64` is refused with a reason.
- Steinberg validator: run it against the corpus as extra evidence. Remember
  the validated lesson: validator success is not AudioRouter evidence.
- Done 2026-11-14 without running plugin code (plan, "P1 follow-up"): the scan
  reads the `Instrument|…` category string from the binary. The idea below is
  no longer needed. Original note: a worker "describe" mode that reads the class category
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

## State at the end of the 2026-11-14 session (supersedes older "open" lists above)

Commits on `feature/vst-plugin-experience` since the handoff was written:
DPF instrument classification, VST3 `canProcessSampleSize`, read-only VST3
program lists, `plugins.addSidechainPort` / `plugins.removeSidechainPort`
(API, MCP, Properties UI), the sidechain connection-rule fix (no implicit
sidechain connection), editor focus/DPI forwarding in the shell window, the
64-bit-only and two-class VST3 fixtures with real-worker tests, the
compatibility status section and draft release notes. Nothing is pushed and no
PR exists.

Still open, in the order to take them:

| Item | What is missing | Needs |
| --- | --- | --- |
| VSTC-04 | headed browser scenario and three-theme screenshots of the Sidechain section; guide/property-manifest entries; a visible text label for the `sidechain` handle (the canvas already renders a handle named `<node> sidechain input`, no visible label); real AGain run (`add_sidechain_port_adds_the_real_again_auxiliary_input_once_and_undoes`) | AGain build (CMake) |
| VSTC-08 | VST3 unit hierarchy and a context-menu stub | design decision on read API |
| VSTC-09 | Steinberg validator run | validator source/binary and CMake |
| VSTC-10 | real-editor checks of focus and DPI, VST2 refit on DPI change, pumped-parent harness, attended pass | owner on the built exe |
| VSTC-11 | installer build, `fresh_install_shell` (close shell PID 55108 first), live-ear check, hosted i686 CI and annotations, signed installer, checksums | owner and hosted CI |
| Owner | hardware MIDI keyboard, PolyMAX UA Connect sign-in, approval to push and open a PR | owner |

Fixture environment variables are documented in the header of each test file
(`vst3_programs.rs`, `vst3_sample_format.rs`, `vst3_multi_class.rs`).

## Continuation update (2026-11-15)

Supersedes the VSTC-04/VSTC-08/VSTC-09/VSTC-10/VSTC-11 checklist above where
it differs:

- VSTC-04: the real AGain dispatcher test passed after its expected auxiliary
  channel count was corrected to the fixture's actual mono aux bus. The canvas
  now shows a visible `Sidechain input` label. New headed Playwright coverage
  passed in dark, light and high-contrast, with card and Properties screenshots
  under `%TEMP%\audiorouter-playwright-results`. The Properties section's grid
  spacing was fixed after screenshot inspection. The occupied-port replace/keep
  flow and guide/property-manifest/video coverage are complete. The default MCP
  all-method reachability test passed for the registered add/remove tools.
- VSTC-08: unit hierarchy, per-unit read-only programs and the safe context-menu
  stub are implemented. UTF16Name reports five units through the worker and
  `ControlPlane::dispatch`; see the dated evidence below.
- VSTC-09: pinned SDK mda and AGain builds passed Steinberg validation (94
  passed, 0 failed each) using CMake 4.4.4 and the same configure/build args as
  the acceptance wrapper. The wrapper itself still expects ignored local
  CMake 4.4.0 and did not run end-to-end. AGain's aux port discovery test is
  separate from audible graph/runtime sidechain proof.
- VSTC-10: no real-editor focus/DPI, VST2 refit, pumped-parent, or attended
  check has been run.
- VSTC-11: release desktop build, unsigned app-only NSIS install/uninstall and
  `fresh_install_shell` passed. Signing, hosted i686 CI and live-ear checks
  remain open.

The latest local continuation completed guide coverage, explicit occupied-key
replacement, VST3 unit metadata UI and the safe context-menu stub. The unsigned
app-only installer installed and uninstalled in a disposable directory, and
the installed shell passed `fresh_install_shell`. Detailed command results,
warnings and artifact hash are in `vst-completion-work.md`.

Still open before release qualification: exercise a real VST3 that reports
units through the live dispatcher (worker metadata is now verified with
Steinberg's UTF16Name sample, which reports five units);
run the pumped-parent/editor resize and attended focus/DPI checks; qualify live
audio on owner endpoints; run hosted i686 CI and read all annotations; and use
the signed installer. No push or PR is authorized. Next local action: inspect
the complete diff, rerun final format/checks after documentation updates, and
commit each completed work item separately. Then hand the branch and exact
remaining gates to the owner for review/hosted qualification.

## Final local review handoff (2026-11-15)

This handoff supersedes older statements above about incomplete guide coverage,
missing unit-bearing plugins and an unbuilt installer. The ordered local work
is implemented and committed in `36e79804f` (VSTC-08 metadata/guide work),
`b492ce0cd` (VSTC-04 replacement flow) and `60601f4a6` (real VST3 dispatcher
regression); this plan update records the final verification. The real
UTF16Name VST3 now loads through the native worker's factory
metadata fallback and returns five units. Both the opt-in `vst3_programs.rs`
worker test and `plugins_programs_dispatches_real_vst3_unit_hierarchy` through
`ControlPlane::dispatch` pass. The mda SDK bundle also passes the generic
worker metadata check.

The sidechain replace/keep dialog, one-edge replacement, guide recording and
coverage manifest are complete. All guide checks pass. The unsigned app-only
NSIS package was installed in an isolated temp folder, all four workers were
verified, `fresh_install_shell` passed, and the package was uninstalled. A
temporary isolated WebView profile remains under
`%TEMP%\audiorouter-nsis-smoke-issue136`; the app is not installed in the
owner's normal location. The installer is not signed and does not include a
driver.

Final local checks: 125 plugin-host library tests; the real UTF16Name and mda
worker tests; native worker self-test; UI suite 1,075 tests and the 9/9 headed
sidechain/unit UI scenarios across dark, light and high-contrast; workspace and
Tauri Clippy; Rust formatting; UI format/lint/typecheck/build; contract drift
(156 API methods / 38 node kinds); default MCP reachability; docs and guide
checks; app-only release guard and installer smoke. The UI build retains the
main-chunk advisory above 500 kB. Stream Deck dependency audit and upstream
Rollup/Zod warnings are recorded in `vst-completion-work.md`.

Do not create or push a PR: explicit approval is still required. The next
reviewer should inspect the complete branch diff and these remaining gates:
pumped-parent editor resize and close; attended VST2/VST3 focus and DPI checks;
live-ear behavior on owner endpoints; hosted i686 CI with every annotation
reviewed; and a signed installer. No hardware MIDI, PolyMAX sign-in,
reporter-specific plugin, or real-audio sidechain ducking qualification is
claimed here.
