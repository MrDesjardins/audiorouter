# VST plans audit (2026-11-24)

Audit of the eight active VST plans against the tree at branch `plans/vst`,
HEAD `02ca8dc9d`, based on `origin/main` `53e1de80e` (the PR #164 VST work and
the #187 portable-hardening overnight are already in that base). Nothing here
was pushed or turned into a PR.

Purpose: decide which plan items still need an agent, which are human or
external gates, and record the proof for each "already done" claim so the plan
files can be archived. Only documentation was changed by this audit; no product
code was edited because the audit found no remaining agent-doable software gap.

## Environment and branch

- Worktree `C:\Users\miste\.t3\worktrees\audiorouter\plans-vst-e4ae210d35fbf57c`
  (branch `plans/vst`), Windows, pinned Rust toolchain, `CARGO_BUILD_JOBS=4`.
- Base `53e1de80e` already contains: PQ #164 merge (`35ecd8e51`), the portable
  hardening work listed in #187 (`crates/cli/tests/security_boundaries.rs`
  stderr tails, `tests/acceptance/find-cmake.ps1`, the two-sub-ID shell test,
  the shell window-procedure tests), and the 2026-11-24 VST plan records.
- Local commits above the base are documentation/UI/i18n only; the one VST
  change is `a0f388981` "Skip VST editor lifecycle on headless hosted runners".
  It is on this branch and still needs a hosted run to confirm.
- `third_party/vst3sdk` is absent in this worktree, so the native VST3 and
  SDK-fixture tests were not rebuilt or rerun here. They are marked below.

## Checks run (all passed)

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0, no warnings |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | exit 0 after `npm --prefix ui ci` + `npm --prefix ui run build` created the git-ignored `ui/dist` (Vite's existing >500 kB chunk advisory and the react-compiler module-directive warning only) |
| `cargo test --workspace` | exit 0. Key suites: control lib 509 passed / 29 ignored; plugin-host lib 130 passed; engine lib 189 passed / 1 ignored; CLI lib 71 passed / 1 ignored; transport 43 passed / 1 ignored; windows-audio 166 passed / 4 ignored; storage 114 passed / 1 ignored |
| `cargo test -p audiorouter-engine --test source_combinations --test tool_combinations` | 6/6 and 6/6 passed (tool runs include `sidechain_handoff_never_allocates_for_ready_late_or_full_queues`) |
| `node tools/docs/validate.mjs` | passed, 303 Markdown files / 1,695 links |
| `git diff --check` | exit 0 |

### Fixture-only opt-in VST2 tests rerun on this tree

Fixtures built with `cargo build --locked --manifest-path
tools/vst2-test-instrument/Cargo.toml --target-dir target/vst-shell --features
shell-fixture` (and the same with `target/vst-bus --features bus-fixture`), and
the worker with `cargo build --locked -p audiorouter-plugin-host --bin
audiorouter-plugin-worker`. Tests used the debug worker and those DLLs.

| Test | Result |
| --- | --- |
| `shell_scan_and_saved_route_bind_the_exact_sub_id` | passed |
| `two_saved_shell_sub_ids_each_restore_their_own_state` | passed |
| `backend_compiles_the_verified_vst2_labelled_key_route` | passed |
| `discovered_sidechain_port_plans_commit_replay_and_undo_atomically` | passed |
| `a_shell_fixture_lists_three_named_sub_plugins` | passed |
| `real_worker_keeps_exact_shell_sub_id_and_refuses_unknown_selection` | passed |
| `the_worker_protocol_lists_and_selects_real_programs` | passed |
| `vst2_bus_discovery` (all three: deterministic gain, labelled pins, real worker describe) | passed |

## VS-01 row-to-code proof

The reconciled table in `vst-completion-work.md` is accurate. Where a row says
"exercised (N)", the test exists at the path below; the "(N)" ones that need the
SDK or a third-party plugin were not rerun in this worktree.

| Item | Code/test location (proof) | Local rerun |
| --- | --- | --- |
| VSTC-01 sidechain domain/transactions | `crates/control/src/graph_tests.rs:9`; `crates/control/src/plugins_tests.rs:257`, `:166` | plan/sidechain/connection tests passed |
| VSTC-02 bus discovery/multi-bus | `crates/plugin-host/tests/vst3_bus_discovery.rs`, `vst2_bus_discovery.rs`; `crates/plugin-host/src/bus_discovery.rs` | VST2 tests passed; VST3 needs SDK (not rerun) |
| VSTC-03 engine sidechain | `crates/engine/src/sidechain.rs`, `connected_mixers.rs`; combination suites | 6/6 + 6/6 passed |
| VSTC-04 control/API/MCP/UI | `plugins.addSidechainPort`/`removeSidechainPort`, default MCP tools, `ui/e2e/plugin-sidechain.pw.ts` | control/UI suites pass; browser scenario not rerun here |
| VSTC-05/07 programs/presets | `crates/plugin-host/src/vst2_preset.rs`; `vst2_shell_discovery.rs:176` | real-worker program/protocol test passed with the bus fixture |
| VSTC-06 shell sub-ID | `vst2_shell_discovery.rs:107`; `plugins_tests.rs:636` | both passed |
| VSTC-08 VST3 units/programs | `crates/plugin-host/tests/vst3_programs.rs`; `plugins_tests.rs:359` | needs SDK fixture (not rerun) |
| VSTC-09 multi-class/sample size | `vst3_multi_class.rs:59`, `vst3_sample_format.rs:13`, `vst3_bus_discovery.rs:43` | needs SDK fixtures (not rerun) |
| VSTC-10 editors | `crates/plugin-host/tests/worker_process.rs:3649`; `src-tauri/src/plugin_editor_windows.rs:503`, `:532` | shell window tests in workspace run pass; real editor needs SDK (not rerun) |
| VSTC-11 release gate | runner + census records in `vst-completion-work.md` | not rerun (needs SDK/DPF download, and the live slot was not this thread's) |

## Residual caveats checked

- **Two shell sub-IDs in one shared worker.** `start_plugin_bridges` records the
  selected shell sub-ID as `verified.metadata.class_ids` (`crates/control/src/plugins.rs:743-755`)
  and chain eligibility requires `identity.metadata.class_ids.is_empty()`
  (`plugins.rs:796-812`). A shell sub-ID node therefore never joins a shared
  VST2 chain worker; the unexercised "two sub-IDs in one worker" case is
  prevented by the production grouping rule, not left untested.
- **VST3 output parameter changes.** The worker supplies an output-parameter
  list on every process call but clears it without forwarding
  (`tools/m06-vst3-worker/main.cpp:1607-1610`, `1670-1673`, `1725-1728`,
  comment at `1760`). This is a documented conformance limitation, not a plan
  gate; plugin-originated parameter changes do not update the host model.
- **VST2 scan-time instrument classification.** A VST2 instrument is still
  learned at its first load refusal (`effFlagsIsSynth` is only readable after
  the plugin entry point runs), so it can be listed as an effect until then.
  Documented P1 limit; no scan-time code path exists that would not run plugin
  code in the app process.
- **Hosted editor skip.** The third hosted run (2026-11-24) failed because AGain
  created no window on the new headless image; `a0f388981` skips both editor
  lifecycle entries when `GITHUB_ACTIONS=true`. Confirming it needs a hosted
  run.

## Not run here, and why

- Native VST3 worker builds and every SDK-fixture test (AGain, mda, UTF16Name,
  sample64only, twoclasses, editor lifecycle, validator): no
  `third_party/vst3sdk` in this worktree and the full runner would download it;
  also kept out of the way while another thread held the live-audio slot.
- The census over installed plugins: rule says not to run it over installed
  commercial plugins; Blue Cat never run.
- Live audio, shell launch, device/driver changes, owner database/profile: not
  touched.
- Hosted `vst-nightly.yml` run and annotations: hosted/owner gate.
- All by-eye, by-ear, MIDI-hardware, vendor sign-in and signing gates: human
  gates listed in the final audit report for the manager's
  `human-gated-tasks.md`.

## Agent-doable live VST route check (run 2026-11-24 with the audio slot)

This is the one live check an agent can run without ears: a real VST2 effect
inside the production backend's prepared route, driven by the Test Signal (no
microphone), rendered and captured entirely on VB-Audio virtual cables, with
the existing glitch detector deciding the result.

Pre-flight: `audiorouter-shell` was not running; CPU load was recorded before
and after each run (8-41 % across runs, with VirtualBox running); the render
endpoints were the main VB-Cable and Cable B, and Cable A - the owner's default
console/multimedia/communications microphone - was never opened. `devices list`
showed CABLE-A Output has the `console,multimedia,communications` roles and
both other cables have no default roles. No microphone, speaker, headphone,
physical endpoint, default format or volume was touched.

Non-audio preparation (no stream opened):

- Built the C VST2 state fixture with
  `powershell -NoProfile -ExecutionPolicy Bypass -File
  tests/acceptance/m06-vst2-state-fixture.ps1 -OutputDirectory <repo>\target\vst-live-fixture -BuildOnly`.
- Created a disposable database `<repo>\target\vst-live-check\live.sqlite`,
  scanned that folder through `audiorouter-cli api call plugins.scan <params>
  --database <db>` (persists the scan inventory), then created the stopped
  session `vst-live-check` containing one Plugin node `vst-live-effect`
  (`audiorouter-vst2-state-fixture.dll`, SHA-256
  `2fd4a4c14fb58fcd5fe77e18b12836a074d3974df3675f55a25f1e7d5f505476`, stereo
  in/out, no state asset) with `audiorouter-cli session create <doc>
  --database <db> --idempotency-key vst-live-create-1`. `session get` returned
  the node unchanged.

Live command actually used (Cable A deliberately not touched; the external
reference tone went to Cable B and the route result went to the main
VB-Cable, because Cable B's input is 8-channel and the endpoint route requires
a stereo render endpoint):

```powershell
$root = (Get-Location).Path
$env:AUDIOROUTER_LIVE_CONTINUITY = '1'
$env:AUDIOROUTER_CONTINUITY_SOURCE = 'testSignal'      # no microphone
$env:AUDIOROUTER_CONTINUITY_SECONDS = '10'
$env:AUDIOROUTER_CONTINUITY_CHAIN = 'gain'
$env:AUDIOROUTER_CONTINUITY_PLUGIN_DB = "$root\target\vst-live-check\live.sqlite"
$env:AUDIOROUTER_CONTINUITY_PLUGIN_SESSION = 'patrick-main-session'  # disposable copy
$env:AUDIOROUTER_CONTINUITY_PLUGINS = 'vst-live-effect'
$env:AUDIOROUTER_PLUGIN_WORKER_PATH = "$root\target\release\audiorouter-plugin-worker.exe"
$env:AUDIOROUTER_CONTINUITY_TONE_RENDER_ID = '{0.0.0.00000000}.{7ebf5c2d-9a8e-4056-8dc9-277cb59ec3e8}'      # CABLE-B Input (8ch reference sink)
$env:AUDIOROUTER_CONTINUITY_ROUTE_CAPTURE_ID = '{0.0.1.00000000}.{65195e1b-3ee4-4e81-b1f6-a95d243fa1ce}'    # CABLE-B Output (reference recorder)
$env:AUDIOROUTER_CONTINUITY_ROUTE_RENDER_ID = '{0.0.0.00000000}.{71f96f14-94c1-4189-b9b9-df8c960db8f2}'      # CABLE Input (route render)
$env:AUDIOROUTER_CONTINUITY_RESULT_CAPTURE_ID = '{0.0.1.00000000}.{06268191-5f8c-42ed-827e-d3c7a19637ed}'  # CABLE Output (result recorder)
$env:CARGO_BUILD_JOBS = '4'
cargo test -p audiorouter-transport --test live_audio_continuity live::live_backend_service_keeps_a_routed_tone_continuous -- --ignored --exact --nocapture
```

(The test prints hard-coded labels "reference (CABLE Output)" and "routed
result (CABLE-B Output)"; with this mapping the reference recorder actually
read CABLE-B Output and the result recorder read CABLE Output.)

Results (commit `02ca8dc9d`, release VST2 worker built with
`cargo build --locked --release -p audiorouter-plugin-host --bin
audiorouter-plugin-worker`):

- **Release worker, 2 of 2 runs passed.** Plugin `vst-live-effect` state
  `running`, `failureCount` 0, `outputMisses` 2 (startup only), delay
  5.33 ms; routed result peak 0.025 (fixture gain 0.5), **0 glitches**, 0
  silent runs, 0 WASAPI flags in both; reference also 0 glitches. CPU 23 %->30 %
  and 41 %->8 %. Exit 0.
- **Debug worker, 2 runs failed** the same check with 2 startup glitches and
  one 128-sample silent run at 1.88 s and 0.13 s into measurement (residual
  0.98x and 0.52x), plugin still `running`, `failureCount` 0, `outputMisses`
  2. The variable position shows a slow debug-build cold start delivering its
  first audio after the test's 1 s settle, not a steady-state discontinuity;
  the shipped release worker has no glitch. Recorded as a debug-build
  limitation, not a product failure.
- Pass criteria met by the release-worker runs: exit 0, `active: true`,
  result 0 glitches. This covers a live VST2 effect route on virtual cables
  only. It does not cover VST3 (no SDK worker in this worktree), 32-bit, a
  physical endpoint, or listening.