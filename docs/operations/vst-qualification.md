# VST qualification runner

`tools/vst-qualification/run.ps1` reproduces AudioRouter's real-worker VST2
and VST3 tests on any Windows machine with Visual Studio's C++ tools, CMake,
Git and the pinned Rust toolchain (with the `i686-pc-windows-msvc` target).
It needs nothing installed in the system plugin folders. The nightly workflow
`.github/workflows/vst-nightly.yml` runs the same script on GitHub's Windows
runner every night and on demand; it never runs on pull requests. GitHub runs
scheduled workflows, and shows the Run workflow button, only for workflow
files on the default branch, so the nightly run starts once the file is on
`main`. Until then, run the script locally.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/vst-qualification/run.ps1 -Clean
```

## What it builds

All of it goes into `target/vst-nightly` (or `-OutputDirectory`):

- the four plugin workers, as `tools/build-desktop.ps1` builds them: VST2
  64-bit and 32-bit (cargo, release) and VST3 64-bit and 32-bit
  (`tools/m06-vst3-worker/build.ps1 -Release`);
- the Steinberg VST3 SDK at its pinned revision (fetched by
  `tools/m06-vst3-sdk/install.ps1` when `third_party/vst3sdk` is missing), and
  its sample plugins: AGain, mda (64-bit and 32-bit) and UTF16Name;
- the repository's own test plugins: the VST2 state, legacy-main, bank,
  non-finite, crash and hang fixtures (`tests/fixtures/vst2-state-fixture.c`),
  the VST2 test instrument and its key-pin and shell variants
  (`tools/vst2-test-instrument`), and the VST3 Sample64Only and TwoClasses
  fixtures;
- unless `-SkipThirdParty` is given, the open-source DPF-Plugins v1.7 release
  (ISC licence), 64-bit and 32-bit, downloaded from GitHub and checked against
  a pinned SHA-256. No third-party binary is ever committed.

## What it checks

- `--self-test-host` of both VST3 workers, and Steinberg's validator on AGain
  and mda (supplemental only: the validator does not exercise AudioRouter's
  worker).
- About 45 opt-in tests, each run by its exact name. A test passes only when
  cargo reports exactly one passed test, so a renamed or filtered-out test is
  a failure, never a silent pass. They cover:
  - VST2: processing, chunk/program/bank state, non-finite output, contained
    crash and hang, shared chains, key-pin discovery and execution, shell
    sub-plugins by exact ID, programs;
  - VST3: processing, multi-bus (sidechain) and the asynchronous bus bridge,
    bus identity per class, class selection without substitution, instruments
    (64-bit and 32-bit mda JX10), unit and program metadata, 64-bit-only
    refusal, state fallback, DPF stateless restore (64-bit and 32-bit), and a
    hidden-window editor lifecycle (open, resize, close three times while
    audio flows) for a fixed-size and a resizable editor;
  - the control plane through `ControlPlane::dispatch`: bus discovery,
    sidechain port add/remove with undo, a saved sidechain route prepared and
    compiled for VST3 and VST2, a shell route saved and reloaded through
    SQLite, VST3 unit metadata, and an instrument node playing MIDI.
- A census over the same plugins (both formats, both bitnesses, effects and
  instruments), with state saved and restored in the same and a fresh worker.

## Results

`report.json` holds the source commit (and whether the tree was dirty), the
SDK revision, the SHA-256 of every worker and test plugin, and one entry per
test with its status, time and log path under `logs/`. The census writes
`census.json`. The workflow uploads the report, the census and the logs; the
built plugins are not kept. The census JSON holds full paths, so do not
publish one from a personal machine; `docs/operations/plugin-census.md` is
generated from an owner-run census, not from this runner.

Known results that are not failures of this runner: DPF Kars and Nekobi
VST3 32-bit stay silent with a held note (a defect in that DPF build; see
`plugin-census.md`), so the census lists them and the runner only plays the
64-bit Kars. On GitHub's runner the DPF ProM editor entry is reported as
`skipped` with its reason (and a notice annotation): the runner has no GPU
driver, and ProM's OpenGL editor ended its worker there with `0xC0150011`, an
activation-context fault inside the plugin, while AGain's editor passed. It
runs, and passes, on a desktop PC. A skipped entry is not a failure and is not
counted in the "of N passed" total.

## What it cannot check

These stay attended or owner checks and are recorded as such in the plan:

- plugin editors looked at by eye: keyboard focus, 100/150/200 percent
  scaling and moving between monitors;
- live audio on real endpoints, listening, and real-audio sidechain ducking;
- licensed plugins (UADx, PolyMAX) and any vendor sign-in;
- hardware MIDI, unplug and replug;
- the owner's own installed plugin collection (run the census for that);
- Blue Cat plugins are never run.

Options: `-SkipBuild` reuses an earlier build in the same folder, `-Only
<regex>` runs a subset (and skips the census), `-SdkDirectory` and `-Cmake`
point at an existing SDK checkout or CMake.
