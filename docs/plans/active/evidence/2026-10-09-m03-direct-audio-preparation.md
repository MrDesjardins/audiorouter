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

### Clarification and speaker-loopback preparation

User describes about three distinct noises in the early/final portions;
the middle was not perfect but quieter. Approximate times are not a measured
click count. This supports intermittent crackles in addition to the earlier
hiss report; neither their exact timing nor silent-gap quality is known.
Keep Listen off. Do not infer a low-frequency, conversion or driver cause.

New diagnostic mode `speaker-record` selects only the exact active guest
Speakers render endpoint and opens its shared loopback capture. It opens no
render, microphone or cable stream. Preserves native stereo 44.1-kHz PCM16
or float32, rejects all other rates/layouts/encodings, and rechecks metadata
after open. Preallocated samples/packet storage, 30-second monotonic duration,
separate readiness/controller and process deadlines, file writes after Stop.
The wrapper verifies the reference checksum and recorder readiness before
opening the reference in the existing file player. It retains numeric/hex
child exits, raw WAV and packet/position/gap metadata and archives only this
run. `CaptureCompleted` and `qualification: false` do not assert clean audio.

Boundary: loopback is the guest rendering endpoint mix, not the host or
acoustic speaker output ([Microsoft loopback recording](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)).
Other guest audio could contaminate the mix; stop unrelated playback. No
default/mic fallback, driver install/rebuild, format change, WSL/Hyper-V or
host setting change. A separate checksummed bundle contains no driver files.

Windows host verification, Rust 1.96.0, all audio/process GUI boundaries mocked
or guarded before endpoint access:

- `cargo test -p audiorouter-windows-audio --example m03_direct_audio`: 12 pass,
  including strict speaker identity/rate/encoding rejection and PCM16/float32
  WAV header/stride/storage checks.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-speaker-loopback.ps1`:
  82 checks pass. Covers valid PCM/float readiness, every missing readiness
  field, early loader exit, bad readiness, native failure, playback failure,
  missing WAV, readiness timeout, process timeout, owned cleanup and actual
  host guard. Initial fixture expected unquoted mode; corrected to match
  production Windows argv quoting. Evidence:
  `target\speaker-tests-ab7fb171feb94a00b4e876771b021415`.
- Existing `tests/acceptance/m03-direct-audio.ps1`: 117 checks pass; evidence
  `target\direct-audio-tests-3c2080f9220444dca7085d1ca2f65cc8`.
- Static release build/import gate pass; no Visual C++ redistributable import.
  Workspace and shell formatting/Clippy pass, with incremental hard-link
  fallback warnings only. No driver or live host audio was opened.

Real guest capture is pending. Next: one reference recording from the new
speaker-only bundle, then offline waveform/packet review. Long/stall/driver
qualification gates remain open; all prior bundles stay immutable.

Published after a clean source commit `72766a66dab28830644aca6f330270b6b802a375`:
`C:\VMs\ar-share\diagnostics-20261009-speaker-loopback`. Prepared
2026-10-10 05:47:54Z. Independent reread verifies all five manifest entries;
no driver binaries or certificates are included. Final wrapper rerun remains
82/82; evidence `target\speaker-tests-70cabb42e15349639510dafc7815eb09`.
Docs: 141 Markdown files / 773 local links pass; diff checks pass.

| Published artifact | SHA-256 |
| --- | --- |
| MANIFEST.txt | `7F899E6AC49C211E32F7EB61E0F2E01738A692ED3D1BFAEA1481860D18B769FC` |
| m03_direct_audio.exe | `4A3A9D20B6DA94BA5790130755C7BE071FC5D06FBACE215E78A02A96E2BC05C2` |
| run-speaker-loopback.ps1 | `78A12CEE4CE5CD0F971F242381DDEA63B17A21965D4D0147163C3C743205A83E` |

The reference hash is unchanged. Actual guest speaker capture is still
pending; this package publication runs no audio and changes no host settings.

## Guest speaker recording review (2026-10-09)

User operated the bounded capture inside AR-DriverTest and heard crackles
at reference seconds **0–5 and 12–16**. The agent opened no host audio endpoint
and made no machine/audio/driver setting change.

Received archive:
`C:\VMs\ar-share\diagnostics-20261009-speaker-loopback\speaker-loopback-0e7000c055ba41d392abb3a902097fca.zip`.
SHA-256 matches the user's output:
`3D1CE8550A60E9D511627DC8765C0F31D5BA13973650DB0546FB1C5E358CF8A3`.
Raw `capture\speakers.wav` SHA-256:
`45E62F47FD86B13FEF3D30D064A1519EEEA81CE3E16CD4857CB9D822CA4DD1E6`.
Reference SHA-256 remains
`C9652D3C629C45FE7BAC8AB0C332A6668F7BD584E3A55323BA0CE9F54842D6A7`.

Observed endpoint: Speakers (High Definition Audio Device), opaque ID
`{0.0.0.00000000}.{15bef167-ae9c-4c78-814b-843520a938d8}`. Recorder startup
and final exit are zero, no timeout/error. WAV is native stereo IEEE float32,
44,100 Hz, 1,320,256 frames (29.937778 s); recorder elapsed 30.0053374 s.
All samples finite; peak 0.25 in both channels. Maximum pump gap 15,324 us.

Offline exact comparison: read the reference as little-endian signed PCM16,
expand each sample to float32 by dividing by 32,768; read the recording as
little-endian float32. Offset is 64 recorded frames. Compare both channels
without resampling, amplitude adjustment, sine fitting or tolerance:

```python
reference = pcm16.astype(numpy.float32) / 32768
delta = recorded[64:64 + len(reference)] - reference
first_different_frame = numpy.flatnonzero(numpy.any(delta != 0, axis=1))[0]
# 749260; every preceding stereo frame is identical.
```

| Reference interval | Mismatching channel samples | Maximum absolute difference |
| --- | --- | --- |
| 0–5 s (997/47 Hz; reported noisy) | 0 | 0 |
| 5–6 s (silence) | 0 | 0 |
| 6–11 s (997/997 Hz) | 0 | 0 |
| 11–12 s (silence) | 0 | 0 |
| 12–16 s (47/47 Hz; reported noisy) | 0 | 0 |

Exact prefix: 749,260 frames, 16.990022676 s. The final 440 frames of the
first reference differ around the playback transition; tone sections
continue afterward. Do not treat the whole 30-second capture as one clean
17-second playback. An exploratory RMS-span sine fit was discarded because
adjacent repeated tone sections merge into spans with different frequencies;
the evidence above is a direct sample comparison, not that invalid fit.

Packet metadata retains 2,947 packets of 448 frames, 35 discontinuity flags
(including the first packet), and 34 position gaps totaling 15,232 frames.
Do not infer actual missing samples from these flags/position jumps alone:
they coexist with the bit-exact prefix. They remain unresolved timing
observations; no physical-clock or whole-recording continuity claim is made.

Offline review command (bundled Python with NumPy, Windows host, no audio):
`python target/speaker-review-0e7000c0/exact_review.py`, exit 0.
The safely extracted source archive, reproducible scratch script and
`offline-inspection.json` remain under that ignored target directory; raw
audio is not committed. No new test or build was run for this review.

Inference: the reported crackles in the compared intervals are introduced
after the guest loopback capture point, not present in those captured mix
samples. Loopback is not host/acoustic capture
([Microsoft loopback recording](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)).
Guest HDA/device rendering, VirtualBox transport/backend and host playback
remain candidates; this recording does not identify which is responsible.
The separate direct-r2 bridge loss (12,912 frames in each direction), Cable A
phase breaks and sustained-continuity gate remain failed.

Read-only VBoxManage machine query failed at COM initialization with
E_ACCESSDENIED; existing XML/log evidence remains the configuration source.
Next: review installed-version playback/backend behavior before preparing a
single reversible comparison. No new run, settings change, driver rebuild,
host audio operation or WSL/Hyper-V change is requested by this finding.
Rollback: documentation only; all bundles and installed driver are unchanged.

Review verification: `node tools/docs/validate.mjs` passes 141 Markdown files
and 775 local links; `git diff --check` passes. Code tests/builds are not
rerun for this documentation-only change.

## Installed VirtualBox backend selection reviewed (2026-10-09)

Installed `VBoxManage --version`: `7.2.20r175154`, exit 0. Its COM-dependent
help/state queries still fail with E_ACCESSDENIED in the agent environment;
no failed query is treated as state confirmation. Read-only current machine
XML: HDA, `useDefault=true`, stored driver WAS, input/output enabled.
Machine/global XML have no `VBoxInternal2/Audio/WindowsDrv` override.
Existing startup log names HostAudioWas.

Downloaded the exact official
[VirtualBox 7.2.20 source](https://download.virtualbox.org/virtualbox/7.2.20/VirtualBox-7.2.20.tar.bz2)
for offline inspection. SHA-256 matches
[Oracle's checksums](https://download.virtualbox.org/virtualbox/7.2.20/SHA256SUMS):
`5c2138213b72f36c129b92c2c267f2a40e9c98513f4c86a584327f09f9be706d`.
Read only four named regular files into checked workspace target paths;
no archive code was built or executed. Offline scratch:
`target\vbox-playback-review-20261009`.

`ConsoleImplConfigCommon.cpp:3755–3787`: DirectSound selection normally
falls through to HostAudioWas on this Windows platform. Machine override
`VBoxInternal2/Audio/WindowsDrv=dsound` prevents that substitution when
DirectSound is selected, yielding DSoundAudio. Setting only the override
while leaving default/WAS selected would not switch the implementation.
`VBoxManageModifyVM.cpp:2783–2806` confirms `--audio-driver dsound` and
`--audio-driver default`. The DirectSound implementation is present in the
source archive; its actual startup must still be verified in the new VM log.

Prepared one comparison, with exact host/guest steps and rollback in the
[direct audio runbook](../../../operations/virtual-cable-direct-audio.md#completed-virtualbox-playback-backend-comparison).
Host block checks powered-off state, expected baseline and absent overrides,
saves original XML, changes only the VM backend and override, and handles
second-command failure by removing the override. User boots normally, then
agent reads current log before any bounded reference playback/capture.
Rollback restores default selection and removes the machine override with
VM off. No raw XML restore, global override, controller/format/driver, host
default endpoint, WSL/Hyper-V or security change.

Verification: all seven PowerShell blocks in the runbook parse without syntax
errors through `System.Management.Automation.Language.Parser`; none was
executed. Baseline XML attribute access was checked read-only.
`node tools/docs/validate.mjs`: 141 Markdown files / 777 local links pass.
`git diff --check` passes. No code build/test or DirectSound audio run.

This is a playback-path hypothesis comparison, not a proven fix or driver
qualification. DirectSound still uses Windows audio services. All measured
bridge-loss/phase-break/sustained-continuity failures remain open. Next:
user shuts down guest Windows normally before the guarded host block.

User completed the powered-off host block: original XML saved to
`C:\VMs\ar-share\vbox-audio-before-336a68538bfa437aa8167432a0de8729.xml`;
both setting commands succeeded. Agent read-only XML inspection confirms HDA,
DirectSound selection and machine override `dsound`. No VirtualBoxVM process
is running; existing log ends with powered-off shutdown of the earlier WAS
session. This confirms configuration only, not actual DirectSound startup.
Next: user boots normally; read the new log before any playback/capture.

Fresh startup read successfully: log opened `2026-10-10T06:04:57.145334800Z`,
Driver and DriverName both DSoundAudio. User started normally without snapshot
restore. This verifies backend selection for the bounded comparison; it does
not establish playback quality. Next: user runs the unchanged speaker-only
recorder/reference, with Media Player routed to guest Speakers, Repeat/Listen
off and prior formats/volume retained; send current archive and audible report.

## DirectSound capture result (2026-10-09)

Archive `C:\VMs\ar-share\diagnostics-20261009-speaker-loopback\speaker-loopback-98d3cd98893d4095b97504508cf77ee6.zip`
SHA-256 matches user output:
`18C4850E40647A111E3C11B6AC505ECE230FE0FC31E1D8B9BCDEF66EABE15EDA`.
WAV SHA-256:
`3A12823775918AAAE48268021B403132A3C86A055A562B1FB10FADEB86D10E7C`.
DirectSound startup is confirmed in the current VBox.log, including
`Audio: Initializing DirectSound audio driver` and its device enumeration.

Startup exit 0, 0.0748672 s, no timeout. Recorder exits 1 with
`speaker capture storage exhausted or packet shape changed`; result has
CaptureCompleted=false and Qualification=false. Saved metadata:

- Elapsed 25.1114944 s; 1,543,360 stereo float32 frames at declared 44,100 Hz
  = 34.996825397 nominal sample seconds. Next 448-frame packet exceeds
  preallocated 35-second storage. Do not increase storage to hide this.
- 3,445 saved packets, all 448 frames; packet metadata cap is 60,000.
  Device-position increments match preceding packet frame counts throughout.
  Flags: one startup discontinuity, 342 silent packets, 3,102 zero flags.
- Maximum pump gap 8,482 us. First packet elapsed 410,577 us; last packet
  elapsed 25,097,960 us. Packet QPC timestamps span 25.2071347 s for
  1,542,912 position frames. Sample progress, packet QPC and worker elapsed
  disagree; this does not identify which virtual clock/path is responsible.
- Samples finite, peak 0.25 in each channel. Same 64-frame reference offset
  and same 749,260-frame exact prefix (16.990022676 s) as earlier recording.
  Final reference transition differs; no whole-recording pass is claimed.

Offline command: `python target/speaker-review-98d3cd98/wave_review.py`, using
bundled Python/NumPy, exits 0. It reads the checksummed archive in memory,
validates the WAV header, compares PCM16-expanded reference bytes exactly
and saves `offline-inspection.json` under that ignored workspace directory.
No recorded audio is committed or played on the host. SharedCapture source
review confirms each successful packet is copied once and released, with
frame count/stride tied to the initialized format; no packet-copy duplication
defect was established by this inspection.

User: silent gaps are perfectly quiet; scratching during capture, then clean
sound afterward (approximately 30 s). The recording stopped at the measured
25.1114944 s above. Treat the capture-on versus capture-off observation as a
useful hypothesis, not proof that MMCSS, 1-ms polling/timer resolution or a
specific driver caused the audible effect. The original WAS recording and
pre-capture listening observations also remain in the history.

Next: one full 17-second playback on DirectSound with the recorder stopped,
same reference/route/volume/formats and Listen/Repeat off. This checks all
sections in the capture-off condition; no new recording/collect or bridge
test. Keep prior driver loss/phase-break/continuity failures unresolved.
Rollback is the documented powered-off default-backend restoration.

Documentation verification: all 10 PowerShell blocks parse without execution;
141 Markdown files / 778 local links and `git diff --check` pass. No code
build/test or native audio operation was run by the agent for this review.

## DirectSound playback with capture off and clock review (2026-10-10)

User completed playback with the recorder stopped and reports inconsistent
scratching, about six to eight scratches on some plays, with more scratching
as playback continues. No new recording accompanies this report. Supersede
the provisional capture-only explanation above: the clean post-capture tail
did not establish stable playback. Both tested backends reproduce audible
problems; DirectSound is not a proven improvement. Increasing noise by ear
does not measure clock drift or establish a code defect.

Read-only review of the current DirectSound boot log finds:

- At log elapsed `00:02:49.080835`, a 59-second guest heartbeat gap.
- At `00:10:59.758904`, virtual-clock catch-up gives up with
  `248 755 386 920 ns` lag; the next line reports a 249-second heartbeat gap.
- No aligned playback marker or audible-event timestamp exists for this
  listening comparison. These events cannot be assigned to individual
  scratches. They do not prove that either the guest or host slept.

The reviewed log was preserved in ignored workspace storage at
`target/vbox-playback-review-20261009/VBox-DirectSound-observation.log`,
SHA-256 `BA0D2364FF417D00C7CAB551C3990CD2452A9895A159F45DA9A7D23CF30EF45C`.
Host System power-event queries for this boot interval fail with
`Attempted to perform an unauthorized operation`; no power-state conclusion
is drawn from the unavailable events.

Further read-only inspection uses the previously checksummed official
[VirtualBox 7.2.20 source archive](https://download.virtualbox.org/virtualbox/7.2.20/VirtualBox-7.2.20.tar.bz2).
Only named regular source files were copied into ignored workspace storage;
no VirtualBox code was compiled or executed:

| File | SHA-256 |
| --- | --- |
| DevHda.cpp | `7e74ee9ff6678e971806e283d1fbf3409c756256602a2e7dd3bf20fcf9f4cc12` |
| DevHdaStream.cpp | `5348daa69bf24c8b0ae463f385ce2eda80a7b4c220242688e33d0e832babf34e` |
| TMAllVirtual.cpp | `3209ff31f2612aa1aa2e95c2aedc9a7f13cd5adc2d48d808c249d467d35d3f7d` |
| DrvAudio.cpp | `8c16e256421f54ce5740c1ed74a6c97d6f5e761e9004894fef684eea8cef3562` |
| AudioMixer.cpp | `4a0a9caf3a44e23b6dc1d0137c159fbbcab75949c15d9400a87bf26faebbdb95` |

`DevHda.cpp:5149–5164` creates HDA timers on `TMCLOCK_VIRTUAL_SYNC`.
`TMAllVirtual.cpp:440–469` reduces that clock's offset during catch-up.
`DrvAudio.cpp:1212–1215` explicitly accounts for guest DMA delays and
subsequent acceleration during clock recovery. Both audio backends share
this device/clock path. `AudioMixer.cpp:53–65` describes guest/host pacing
independence; `DrvHostAudioDSound.cpp:2091–2096` can reposition its software
write cursor without a normal release-log warning. Absence of an underrun
message in VBox.log therefore cannot qualify playback.

Inference: virtual-device clock recovery is a supported candidate for the
sample/elapsed-time disagreement and uneven playback. The active catch-up
percentage and its overlap with the recording/listening intervals are not
recorded, so attribution remains open. No justified AudioRouter driver patch
has been identified by this source review. Do not modify VM timer settings,
disable Hyper-V/WSL, increase buffers/storage or relax acceptance criteria.

Decision and next action: end the unsuccessful DirectSound comparison. Stop
playback; user shuts down guest Windows normally and uses the existing guarded
default-backend rollback. Rollback is not yet performed. Review clock recovery
against saved paired timing data before another candidate/test. VCAB-12/20/24/29
and VDEV-12 measured loss, phase breaks and qualification gates remain open.

Verification for this documentation-only update: `node tools/docs/validate.mjs`
passes (141 Markdown files / 781 local links); all 10 runbook PowerShell blocks
parse without execution; `git diff --check` passes. No code build, new audio
test, driver installation or VM setting change was performed by the agent.

## Rollback verified and shutdown statistics reviewed (2026-10-10)

User shut down guest Windows normally and reports the guarded rollback
succeeded. Agent read-only XML inspection confirms HDA, `useDefault=true`,
stored driver WAS and zero machine backend overrides. No VirtualBoxVM or
VBoxHeadless process is running; VBox.log ends in PoweredOff. No snapshot,
guest format, driver, host security, power or WSL change was made.

Closed-session log preserved at ignored workspace path
`target/vbox-playback-review-20261009/VBox-DirectSound-shutdown.log`, SHA-256
`695DFED2A994083F7BC844CCB44C9F1A3806D84B42315618B83323855CBC74B1`.
Its shutdown statistics provide additional evidence:

- Stream4: 44,100 Hz, 4-byte frames, 1,792-byte current DMA period;
  `DMASkippedPendingBCIS=11`. This is the first output stream, following
  the four input streams (`DevHda.cpp:478–483`, `DevHdaStream.cpp:905–928`).
- `DevHdaStream.cpp:1366–1391` skips DMA when the prior buffer-completion
  interrupt remains pending. Its comment identifies the pacing mismatch:
  the host backend continues consuming while DMA pauses. These counters
  confirm skipped virtual-HDA work, not merely a hypothetical playback path.
- Stream0 has 3,101 input DMA underruns and 1,864,280 bytes of inserted silence.
  The source registers that underrun counter only for input
  (`DevHda.cpp:5313–5325`). Do not mislabel this as speaker corruption.
- Synchronous virtual-clock current offset is 253,171,005,741 ns and given-up
  offset is 248,755,386,920 ns. These are final aggregate values, not the
  catch-up percentage during the short recording or the duration of a scratch.
- The session later logs `HostSuspend` at elapsed `00:23:56.065542` and
  `HostResume` at `10:39:22.767589`, with host default-output changes around
  resume. This later overnight suspension does not prove that earlier audio
  comparisons slept or identify their cause. No host power setting was changed.

Limit: skipped-transfer counters cover the whole boot session and have no
individual timestamps. They cannot quantify the comparison's audible events
or replace digital continuity evidence. Earlier AudioRouter bridge loss and
phase breaks remain distinct unresolved defects. The owning virtual HDA
interrupt/clock recovery path is now a more specific investigation target;
no AudioRouter code defect or qualified driver fix is claimed.

Next engineering task: inspect that path against retained timing data before
preparing a candidate. Keep the VM off; no new playback or audio test is
requested. The original backend selection is restored, not declared clean.
