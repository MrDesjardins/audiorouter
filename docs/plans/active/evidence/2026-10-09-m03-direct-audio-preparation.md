# M03 automatic direct audio preparation — 2026-10-09

## Reproduction and scope

The paired 30-second run on the clean `97b393d5` test driver passes native
counters and deadline checks, yet the user hears continuous hiss mixed with
the beep through Cable B's listener. Hiss stops when the test stops. The
user's Media Player → VM speakers comparison is clean. No direct Cable B WAV
existed, so the earlier Cable A WAV cannot identify that defect.

Requirements: VCAB-12/20/24/29 and VDEV-12. This change diagnoses signal shape
and controls test lifecycle; it changes neither driver nor acceptance gates.
WSL/Hyper-V, host security, power and VM settings are preserved. No Audacity,
VS installation/repair, test certificate or driver installation is involved.

## Implementation

- [User-mode entry point](../../../../crates/windows-audio/examples/m03_direct_audio.rs):
  `record` refuses any machine other than AR-DriverTest and requires a new
  empty output directory canonicalized beneath C:\ar. `analyze` is offline.
- [Stream worker](../../../../crates/windows-audio/examples/m03_direct_audio/mod.rs):
  one exact active render Cable A Input and capture Cable B Output, retained
  IDs, stereo float32/48 kHz guard, metadata rechecked after client opening.
  No default or microphone fallback; existing safe WASAPI clients are reused.
  Generates 440/660 Hz by submitted-frame index, preserving phase on partial
  submissions. Preallocates sample/packet storage before Start. No periodic
  file/console I/O; packet copies release WASAPI buffers before storage.
  Saves samples, flags, device/QPC positions and maximum pump gap after Stop.
  Controller and worker have independent bounded lifetimes; readiness is
  published atomically from the control thread.
- [Offline analyzer](../../../../crates/windows-audio/examples/m03_direct_audio/signal.rs):
  bounded WAV read, strict header/shape/finite validation, sine fit with
  arbitrary phase plus DC; report worst residual, amplitude and phase jump
  across windows. Expected frequencies are 440/660 Hz for A and 997/47 Hz
  for B. Requires 29.8–30.2 seconds of active signal, preserves full raw files,
  documents the two boundary quanta excluded from fitting. Reports explicitly
  say `qualification: false`; triage is not bit-exact/THD+N qualification.
- [Guest wrapper](../../../../tools/vm/run-direct-audio.ps1): manifest/admin/VM
  checks, installed status first, one owned source/recorder, readiness guard,
  unchanged native tone for 30 seconds with 90-second process watchdog,
  bounded offline analyses of both WAVs, in-signal packet flag checks,
  current-run-only ZIP/copy on success or failure, owned-process cleanup.
- [Copy-only preparer](../../../../tools/vm/prepare-direct-audio-update.ps1): clean
  source commit, fresh built helper, complete verified base entries only,
  independent new manifest. It executes no audio/driver binary.

## Windows verification

Environment: Windows host, pinned Rust 1.96.0; no live audio endpoint opened.
Native guard checks execute record mode only to verify refusal before its
audio API calls. All orchestration/audio cases otherwise use fake boundaries.

| Check | Command | Result |
| --- | --- | --- |
| Signal/format regressions | `cargo test -p audiorouter-windows-audio --example m03_direct_audio` | 9 passed |
| Script orchestration/host guards | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-direct-audio.ps1` | 75 checks passed |
| User-mode build | `cargo build --release -p audiorouter-windows-audio --example m03_direct_audio` | passed |
| Workspace Clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | passed |
| Shell Clippy | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | passed |
| Formatting | root and shell `cargo fmt`, both `-- --check` commands | passed; no unrelated Rust changes |
| Documentation | `node tools/docs/validate.mjs` | 141 Markdown files / 765 local links passed |
| Diff | `git diff --check` | passed |

Regressions reject noise, clipping, swapped channels, dropped/repeated quanta,
whole-second loss that aliases integer-frequency tones, nonfinite/silent/short
audio, PCM versus IEEE float, 44.1 versus 48 kHz and malformed WAV sizes.
Script checks cover status, recorder, native tone, signal and packet failures,
an exception in the native boundary, retaining current evidence and closing
only the owned recorder. The initial compile required an explicit controller
`Result` type; this was corrected before the passing build/tests.

PowerShell evidence: `target\direct-audio-tests-4c721425d7ea420da12ea4d136ee9671`.
Cargo reported incremental-cache hard-link fallback warnings; the compiler
and Clippy completed successfully. Jev remains disabled by the user's prior
decision. Full live suites, first real direct recording, longer continuity,
latency, eight-cable, Verifier, signing and physical-host gates are not run
by this preparation and remain open.

## Handoff and rollback

Published copy-only bundle: `C:\VMs\ar-share\diagnostics-20261010-direct-audio`,
prepared at `2026-10-10 05:06:11Z` from clean pushed source
`c0bc9595d6f90259bb013f62bd39a82b4c1a18ef`. Independent read-only hashing
verified all 37 entries and confirmed all 35 base files byte-identical.
Only the new helper and wrapper are added. No base driver or tone binary is
rebuilt/replaced, and no host endpoint is opened by preparing the share.

| Artifact | SHA-256 |
| --- | --- |
| MANIFEST.txt | `08CAF415683CD83C0B073D92052A8919168EC23D791E752EB26C63971F278549` |
| m03_direct_audio.exe | `C93F329ED5A4ECB2085CBA50FEEC3524893DF6D844A61546C564C6DA8C970FFD` |
| Unchanged audioroutervirtual.sys | `23F741CD5D4D455840898D0A614E39BFA588B96BE8CB2C1CF5E7AD5CAF0D34B3` |

Use the [copy/paste guest procedure](../../../operations/virtual-cable-direct-audio.md).
Stop Media Player so its signal cannot mix with the generated reference;
keep the existing listener for simultaneous listening-path reproduction.
Review both WAVs before a driver change or another long test. A clean direct
recording with audible hiss points to further listener/playback investigation;
a noisy recording requires tracing the digital path. Neither alone proves a
particular function is responsible. Rollback stops this owned diagnostic and
returns to the prior bundle without changing the installed driver.

## First guest attempt and startup repair (2026-10-09)

User ran the published diagnostic. Archive
`direct-4a281d278cde43ada173fb07d2506aea.zip` was opened read-only from the
host share; its SHA-256 matches
`CA129D76FCA71BD8A42125FA3B73DA5E7ED9A27E162641990CFF61673052E0AD`.
Installed status and all endpoint formats pass. The recorder exits before
readiness; stdout/stderr are zero bytes, capture directory empty, and the
wrapper never starts the bridge tone. There is no new signal measurement.

Confirmed preparation defect: the original helper was built with ordinary
`cargo build --release` and imports `VCRUNTIME140.dll` plus dynamic CRT API
sets. The established VM tool build uses `-C target-feature=+crt-static`.
The working bridge tone imports no Visual C++ runtime DLL. Local startup on
the development PC did not demonstrate portability to the clean guest.
The old wrapper omitted the recorder's pre-readiness exit code, so this
archive cannot prove the exact guest loader status or missing DLL.

Repair:

- [Build helper](../../../../tools/vm/build-direct-audio.ps1) restores the
  caller's build flags after building with `+crt-static` under a separate
  `target\vm-direct-audio` tree. Nothing is installed or repaired.
- [PE import gate](../../../../tools/vm/portable-tool-support.ps1) reads bounded
  PE32+ sections/imports without loading the executable; packaging rejects
  Visual C++ redistributable imports and malformed files. The preparer uses
  this static artifact rather than the ordinary release output.
- New `startup-check` opens no audio endpoint. The wrapper verifies it before
  status/audio and preserves its process report. Early recorder exits now
  retain numeric and unsigned hex codes in `recorder-process.json`.
- Expanded script checks simulate loader failure `0xC0000135` with empty
  stderr, early worker exit, and validate malformed PE rejection. They reject
  the actual previous dynamic helper and accept/launch the actual static one
  in offline mode. No host audio/driver operation occurs.

Verification on Windows: static release build succeeds; its PE parser and
independent `dumpbin /dependents` agree on system-only imports, with no
VCRUNTIME/MSVCP/CONCRT DLL. Nine Rust regressions and 103 script checks pass;
root/shell formatting and workspace Clippy pass. Script evidence:
`target\direct-audio-tests-0e2040ffb17848b684b6462838942d48`.
Jev remains disabled. The fixed executable's first guest run is pending;
hiss and sustained continuity are still unresolved. The old bundle/archive
are preserved; a new r2 bundle uses the original paired-tone base and copies
all driver files unchanged. Use the updated direct-audio procedure above.

Published r2 at `2026-10-10 05:12:54Z` from clean pushed source
`0e854d7e69077f0230bd9fc02c0f60e4dbb86f86`:
`C:\VMs\ar-share\diagnostics-20261010-direct-audio-r2`.
Independent hashes verify all 37 entries and all 35 unchanged base files.

| R2 artifact | SHA-256 |
| --- | --- |
| MANIFEST.txt | `B3B80D1A89DDB810994AB5FEB15194AB7F17799B29B307D29814343ACAA78897` |
| Static m03_direct_audio.exe | `95E8E4117991F6657DF7205D96660CA807402084D13883FA889E5A43EFE1C9A2` |
| Unchanged driver SYS | `23F741CD5D4D455840898D0A614E39BFA588B96BE8CB2C1CF5E7AD5CAF0D34B3` |

Documentation validation after this repair: 141 Markdown files, 768 local
links pass; diff checks pass. Next action is the bounded automatic guest
diagnostic using r2, with Media Player stopped and the existing listener
retained. Neither the packaging repair nor its host checks clears the hiss
or sustained continuity gate.
