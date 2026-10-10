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

## Direct r2 recordings and reporting repair (2026-10-09)

User reports beep plus substantial continuous static during the automatic
test. Archive `direct-4586cab5c56d429c8835b3c0eaa3008c.zip` was read from
`C:\VMs\ar-share\diagnostics-20261010-direct-audio-r2`; SHA-256 matches the
user's `7F47E3C08F46DF6A3945C60F5731E79CC9FDEE94E530A3F2240BF30A2B554AA4`.
Unique local review directory: `target\direct-review-4586cab5`. Nothing in
the guest or original bundle was changed during review.

### Measured outcome

- Startup, installed status and recorder exit pass. Metadata confirms stereo
  IEEE float32 at 48,000 Hz, exact Cable A Input and Cable B Output IDs.
- Native process exits 0; the check wrapper correctly fails the counter
  gate: 12,912 capture underrun and 12,912 render overrun frames (269 ms
  each). Leases stop at 30,003 ms; there is no watchdog timeout.
- Native capture/render maximum pump gaps: 288,446 / 288,263 us. Independent
  WASAPI source/recorder gap: 288,257 us. Control gap: 291,265 us. Progress
  first shows the counter burst between native seconds 26 and 27. Correlated
  stalls do not identify host descheduling, a guest wait or a driver lock.
- Cable A WAV: 1,427,040 frames / 29.73 seconds. Direct Cable B: 1,497,600
  frames / 31.2 seconds including pre/post-lease silence; active interval
  spans 29.73 seconds. Both fitted intervals are 29.71 seconds after the
  same two boundary quanta are excluded. Duration failure is genuine.
- Cable B has one in-signal discontinuity flag at file frame 1,324,800
  (27.6 seconds), with a 13,440-frame device-position gap. One earlier
  480-frame position gap and the initial discontinuity flag occur before
  the generated tone begins. Raw packet flags/positions remain preserved.

### Waveform evidence versus audible hiss

Independent Python/NumPy least-squares fits of 100-ms windows find median
residual RMS about 3.6–3.7e-9 in Cable B's 997/47-Hz channels, with amplitude
0.25. A sine recurrence check across every adjacent sample triple finds
only the start/stop boundaries and the single in-signal break at frame
1,324,800. This is float32 rounding-level residual between the loss event,
not continuous broadband noise in the directly captured samples.

Cable A has breaks at frames 577,776 / 577,920 / 578,400 (12.037 / 12.040 /
12.050 seconds), plus several breaks during recovery near 26.44–26.48
seconds. Before the later pause, bridge counters remain zero despite that
earlier source-path discontinuity. A zero bridge counter is therefore not
sufficient waveform continuity evidence.

The data narrows continuous hiss to further investigation of Windows Listen,
speaker rendering and VirtualBox/host playback. It does not prove which one
is defective or exclude another concurrent capture stream. The known 48-kHz
float recording does not justify a speculative 44.1/48-kHz setting change.
Do not label the whole run clean: it has measured loss and phase breaks.

| Original artifact | SHA-256 |
| --- | --- |
| Cable A render-source.wav | `DEDFD3E45F430253B56B3FCAC68CC174262E91D49F3382CDFB8F3D8DB2AF72B5` |
| Direct cable-b-output.wav | `6A6127DE457D25CEA2AF877BA0FA861D6E1DF9E02630173F9700C11BC568749B` |

### Owning diagnostic defect fixed

The old offline analyzer returned at the duration gate before fitting samples,
so both reports contained only "fewer than 29.8 seconds". This also omitted
the fit bounds used by the wrapper's in-signal packet check. The source
repair retains duration result/error, fit bounds, per-channel metrics and
per-window phase/residual details even when duration fails. The 29.8–30.2 s,
noise, amplitude and phase bounds are unchanged; malformed, nonfinite or
unfittable files still fail explicitly.

Verification on Windows host, Rust 1.96.0, no endpoint or driver opened:

- `cargo test -p audiorouter-windows-audio --example m03_direct_audio`:
  10 pass, including shortened recordings with noise/phase breaks, clean
  shortened failure, and whole-second loss/replay failure.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-direct-audio.ps1`:
  117 checks pass; short-duration plus packet failure still evaluates packet
  flags. Evidence: `target\direct-audio-tests-8674085fe5504699aeb1cadc5cb6d702`.
- `tools/vm/build-direct-audio.ps1`: static release build and PE import gate
  pass. Both original WAVs analyzed offline with the repaired executable:
  exit 1 on each, duration false, waveform/phase metrics retained. Reports:
  `target\direct-review-4586cab5\repaired-metrics-a.json` and `repaired-metrics-b.json`.
- Root/shell formatting checks and both workspace/shell Clippy pass. No
  unrelated Rust reformatting. Documentation: 141 Markdown files / 771 local
  links pass; diff checks pass. Incremental hard-link fallback warnings only.

Rollback: revert the analyzer/reporting regression only. Driver, native tone,
host settings, WSL and all guest bundles are unchanged. No new share bundle
or guest retry is needed to review these existing recordings. Next task:
isolate the downstream listening path and attribute the shared stall; hold
long/stall runs and all unresolved VCAB-12/20/24/29 and VDEV-12 gates.

## Saved-file replay and supported speaker formats (2026-10-09)

User confirms hiss while replaying the saved Cable B WAV through Media
Player → Speakers. This does not require the live bridge process. User
reports Speakers at PCM16/44.1 kHz, with only 16/22.05/44.1 kHz available;
Cable B is stereo 32-bit/48 kHz. The proposed 48-kHz speaker setting is
unavailable and is withdrawn. Preserve the supported setting and the cable.
Windows distinguishes the float engine mix from a PCM device format
([Microsoft device formats](https://learn.microsoft.com/en-us/windows/win32/coreaudio/device-formats));
the depth difference alone is not proof of a defect. Neither the rate
conversion nor the WAV container is yet proved responsible for the hiss.

Additional offline comparison: Cable B file seconds 2–7 versus independently
generated 997/47-Hz float32 samples, aligned to the first generated frame
(55,728). Maximum absolute difference 1.4901161e-8; zero mismatches among
480,000 quantized PCM16 samples. Original bytes remain untouched. This
confirms that this section carries the intended tones, not added broadband
noise. It does not make the full recording or driver pass.

[Offline reference generator](../../../../tools/vm/prepare-playback-reference.py)
uses only the Python standard library, creates new files and opens no audio
endpoint. Produces stereo PCM16 references at 44.1 and 48 kHz: 17 seconds,
amplitude 0.25, both 997/47-Hz tones for five seconds, 997-Hz only for five,
47-Hz only for five, one-second silent separators and 20-ms boundary fades.
Each file has a standard PCM WAV header and checksummed metadata.

Host verification:

- `python tests/acceptance/test_playback_reference.py`: 3 tests pass for both
  rates, WAV headers, frame counts, amplitude, frequency/sample values within
  0.5 PCM16 step, boundary silence, unsupported-rate rejection and no
  overwriting. The initial temporary directory check hit sandbox access denial;
  it now uses an explicitly checked directory under workspace target.
- `python tools/vm/prepare-playback-reference.py --destination target/playback-reference-20261009`:
  files generated offline. Independent hashing of the copied share files
  matches the metadata. No audio playback or driver operation by the agent.

Published new folder: `C:\VMs\ar-share\playback-reference-20261009`.

| Reference | SHA-256 |
| --- | --- |
| reference-44100.wav | `C9652D3C629C45FE7BAC8AB0C332A6668F7BD584E3A55323BA0CE9F54842D6A7` |
| reference-48000.wav | `D227E7F42268B2F390D919BFE683275B15511AB7BEA79B050967E1CAC559578C` |

Next: user plays only the 44.1-kHz reference through the same VM Speakers
and reports which section has hiss. It is a supported-format playback
comparison, not an AudioRouter driver run. Review before the 48-kHz file or
any live/long/stall test. Rollback is to stop playback; WSL, host configuration,
driver binaries and all prior bundles remain unchanged.

### Received reference result and read-only VM review

User reports static in **all three** sections of the independent 44.1-kHz
PCM16 reference. Rechecked the published share file, not just the generator:
SHA-256 matches `C9652D3C629C45FE7BAC8AB0C332A6668F7BD584E3A55323BA0CE9F54842D6A7`;
header is stereo PCM16/44.1 kHz, uncompressed, 749,700 frames. Peak is 8,192.
793,800 interior samples match independent sine equations with maximum error
0.499984803 PCM16 step; both silent separators contain only zero samples.
No file regeneration, driver operation or host audio playback occurred.

Read-only sources: `C:\VMs\AR-DriverTest\AR-DriverTest.vbox` and current
`Logs\VBox.log` (opened 2026-10-10T01:21:10.416351800Z). Configuration:
HDA, Windows Audio (`HostAudioWas`), output enabled; default output identified
as Speakers (Focusrite USB Audio). VirtualBox reports version 7.2.20 r175154.
Scheduling-hint warnings print implausible durations; retain them as log
observations, not evidence of an actual pause of that duration. No log entry
reviewed identifies the cause of continuous hiss.

Inference: the hiss reproduces with supported-format audio that was not
generated or captured through AudioRouter. The still-enabled concurrent
Cable B Listen path is not yet excluded. Next is the guest-only listener-off
reference comparison in the runbook, including the silent gaps. No host
settings, WSL, driver build or qualification criteria change. The real loss
and source phase breaks remain failed gates regardless of this comparison.

### Listener disabled: static reduced but remains

After the guest listener-off comparison, user reports "a lot less" static,
still audible especially in the first and final parts. This supports an effect
of the concurrent Listen setting on perceived noise; it does not identify
the remaining cause. The first/final reference sections contain 47 Hz, while
the middle contains only 997 Hz. No new recording or objective noise measure
exists for this playback result. Asked whether the residual is continuous
through those sections or brief at boundaries, and whether the silent gaps
and middle section are clean. Keep Listen off and all bridge/long tests held
pending that clarification. No host, VM configuration, format or driver change.
