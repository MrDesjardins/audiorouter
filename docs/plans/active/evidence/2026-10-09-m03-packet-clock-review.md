# M03 packet-clock and capture catch-up review — 2026-10-09

## Scope and evidence

User request: find code defects before the next VM test. Requirements:
VCAB-24/25/27/28, VDEV-12. Previous guest archive:
`C:\VMs\ar-share\evidence-20261009-175425.zip`, SHA256
`477BD491D3C8BD84B364A4337F581EEA5A5D23739B61E5557E09867317033CAB`.
The 300-second run lost audio during four bursts and measured 163 ms worker
gaps. The VM did not sleep. The earlier conclusion that no code fix was
established was premature: reviewing the packet clock exposes independent
defects. These do not prove the cause of VM descheduling or explain every
lost frame. WSL/Hyper-V and host drivers/settings are unchanged.

## Findings and repairs

1. **Completed packets diverged from DMA.** UpdatePosition advances by full
   elapsed QPC time, but TimerNotifyRT incremented the packet count only once
   and repaid overshoot on later timer ticks. At a 10 ms packet interval, a
   163 ms delay means sixteen completed packets, not one. GetPacketCount could
   also advance position without updating its packet result. Derive completed
   packets from the absolute DMA byte position; retain the last notified count
   separately. One event resynchronizes PortCls without replaying callbacks.
   GetReadPacket and SetWritePacket refresh the same clock before snapshots.
   This follows the [Microsoft packet-count contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/portcls/nf-portcls-iminiportwavertoutputstream-getpacketcount).
2. **Capture catch-up repeated overwritten DMA laps.** WriteBytes looped over
   all elapsed bytes under the position spinlock. A long delay made callback
   work proportional to pause duration. Keep only the surviving physical DMA
   lap, requiring at most two segments across wrap, while advancing the full
   logical position. Missed historical capture frames remain counted; queued
   good audio is not consumed for the discarded laps. Existing render catch-up
   already discards old laps and counts them. Thus render overrun is not proof
   of FIFO exhaustion alone.
3. **Capture timestamp was one packet late.** The previous multiplication
   identified the end of the latest completed packet. Use `(completed - 1)`
   to locate its first sample, as required by the
   [Microsoft capture contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/portcls/nf-portcls-iminiportwavertinputstream-getreadpacket).
   Before the first completion return DEVICE_NOT_READY explicitly.
4. **STOP retained fractional clock state.** Reset both time/byte remainders
   and notification progress with DMA position. PAUSE retains progress and
   RUN establishes a fresh QPC anchor, excluding time spent paused.
5. **Long-uptime timestamp conversion overflowed.** Multiplication of absolute
   100 ns time by QPC frequency before division rejects otherwise valid
   timestamps after about 51 hours at 10 MHz. Split seconds/remainder before
   scaling and check both intermediate ranges and the final sum. This is an
   independent long-uptime defect, not a demonstrated cause of the recent run.
6. **Loss snapshots lacked matching worker timing.** The tone tool now prints
   maximum capture/render pump gaps for each progress interval as well as
   lifetime peaks. Workers only update fixed atomic counters; reporting stays
   on the control thread. These are diagnostic observation windows, not proof
   that driver and user-mode callbacks were delayed identically.

No queue enlargement, sample-storage allocation, ABI change, acceptance
relaxation or silent recovery was introduced. Signed packet count saturates
instead of overflowing. A stopped VM can still lose overwritten audio.

## Host verification

Environment: Windows x64, Rust 1.96.0, installed MSVC/WDK. Build processes
select the x64 host compiler through process-local PATH/PreferredToolArchitecture;
no Visual Studio repair or persistent environment change.

| Check | Command | Result |
| --- | --- | --- |
| Production arithmetic/bridge regressions | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File drivers/audiorouter-virtual/tests/build-tests.ps1` | 578 checks pass; `target/driver-unit-release/tests.log` |
| WDK x64 compile/catalog/source guards | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1 -Platform x64` | pass; build-only |
| WDK ARM64 compile/catalog/source guards | same command with `-Platform ARM64` | pass; build-only |
| Tone worker/recording regressions | `cargo test --locked -p audiorouter-windows-audio --example m03_bridge_tone` | 15 pass |
| Workspace Clippy | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass; cache hard-link fallback warnings only |
| Shell Clippy | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings` | pass; same cache warning |
| Formatting | both workspace and shell `cargo fmt`, then both `-- --check` commands | pass |
| Documentation | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/docs.ps1` | 138 Markdown files / 746 local links pass |
| Guest wrapper syntax | Windows PowerShell `Parser.ParseFile` | pass; no VM step run |
| Guest wrapper host guard | invoke `tools/vm/run-packet-clock-review.ps1` on the host | expected refusal / exit 1 before any mutation |

New C++ checks exercise 163 ms and 895 s delays, partial/first packets,
ULONG packet-number wrap, first-sample location, zero buffer/configuration,
and bounded traversal through maximum ULONG displacement/position wrap.
The follow-up additionally covers 100-day 10/50 MHz timestamps, sub-tick
fractions, representable maximum output and intermediate/final overflow.
They test production helpers, not a running kernel or DPC latency. Source
guards additionally require correct callback wiring and STOP reset.
The new Rust check confirms interval resets preserve the lifetime peak.
Local Jev remains disabled by the user's earlier request.

## Next test and rollback

Final candidate prepared and verified:

- Host bundle: `C:\VMs\ar-share\repair-20261009-packet-clock-r2`.
- Source commit: `97b393d5e25cd0e114bf0a8078627b14ab97b13b`, clean tree;
  driver built at `2026-10-10T01:19:22.9790918Z` (2026-10-09 local).
- Driver SHA256:
  `23F741CD5D4D455840898D0A614E39BFA588B96BE8CB2C1CF5E7AD5CAF0D34B3`.
- `prepare-vm-share.ps1` completed all six stages; package verification:
  33 checks pass. All 30 manifest file hashes independently verified.
- Test certificate thumbprint unchanged:
  `FF6876FBE50A74DC0B69077C11DC28A1A9FAAD40`. The host intentionally does
  not trust this test root; signature verification records that expected
  condition. No host certificate trust or driver installation was performed.
- The earlier `repair-20261009-packet-clock` bundle is superseded; use r2.

The identified candidate includes verified catalog, SYS, INF and manifest.
Run only in AR-DriverTest after restoring the clean
test-signing snapshot. Follow the [retest procedure](../../../operations/virtual-cable-packet-clock-retest.md):
preflight/smoke/install/status, 30-second tone, then 300 seconds if the short
run passes. Evidence is collected and copied even after an ordinary failure.
No new VM run, Verifier, stall injection, one-hour continuity, DPC measurement,
or hardware latency result is claimed here. VCAB-24/25/27/28 stay open.
Rollback: power off and restore the same clean snapshot; keep earlier bundles
and evidence. Reverting the source commit restores previous code if needed.

## Guest preparation evidence — 2026-10-09 18:23 local

User ran Prepare in AR-DriverTest and copied
`C:\VMs\ar-share\evidence-20261009-182328.zip`. Host inspection verified
SHA256 `3EE56FD05D809207FBD8A2BBCF17BAC0D5C37B67572E19CF50BF157CE40AB210`
and the archived JSON summaries. Exact r2 source `97b393d5`, clean build:
preflight 15/15, smoke A1/A2/A3/A14, retained install 3/3 (four endpoints,
idempotency), status 2/2, collect 2/2 all pass. Smoke left unrelated devices
and default roles unchanged and restored its baseline. The subsequent install
keeps the driver for tone testing. All four active endpoints accept 60 formats
and report 128-frame minimum period. This is not hardware latency evidence.
Next: keep this guest session running, loop Media Player into Cable A Input,
listen to Cable B Output through the VM speakers, and run the 30-second tone
phase. Sustained continuity remains pending.

## Guest 30-second tone evidence — 2026-10-09 18:30 local

User ran the r2 Tone phase for 30 seconds. Archive
`C:\VMs\ar-share\evidence-20261009-183059.zip`, SHA256
`27BB85E54EA9F211E3E6DD41B444BF15D07E444068BDB92CC43FA72BBF92629C`,
was independently verified on the host. Archived tone summary passes all
four checks: tool exit 0, complete final reports, all five error counters
zero in both directions, recording written. Harness sequence gaps: zero.
Render recording: exactly 3,000 blocks / 1,440,000 frames / 30 seconds,
48 kHz stereo float32; RIFF/WAVE header and data/file lengths verified.
Maximum worker gaps: capture 7,523 us / render 7,467 us; WAV append 2,425 us,
control heartbeat 183 us, report 356 us, lease close 57 us. These are harness
timings, not kernel DPC or hardware latency measurements. No waveform
discontinuity analysis or long-run qualification is claimed from this check.
Next: keep the same loop/listener active and run the bounded 300-second Tone
phase. Preserve zero-counter and zero-sequence-gap acceptance.

## Failed five-minute attempt and reporting repair — 2026-10-09

Archive `C:\VMs\ar-share\evidence-20261009-191454.zip`, independently verified
SHA256 `B576C55519311EF2E7DC00D89AF5B2304C0F6077B0E90801D500C4D2AC1C30C5`.
Requested 300 seconds; final tool exit 1. Capture underrun 296,496 frames;
render overrun 344,640 frames; other driver error counters and harness sequence
gaps zero. The final run is invalid as a five-minute qualification.

Two distinct findings:

1. At progress **56,162 ms**, capture/render interval gaps are **53,439 /
   53,385 us**, and counters first rise to **1,584 / 2,016** frames. Loss
   therefore predates the later report hang. These observations correlate
   delayed workers and loss but do not identify the scheduler/kernel cause.
2. Last emitted progress is **227,675 ms**. The measured maximum synchronous
   progress output call is **2,310,481,790 us** (38m30s). Active control printed
   into a PowerShell pipeline that relayed output to the console. While that
   call blocked, the control thread could not heartbeat or observe completion;
   workers kept supplying/recording until lease shutdown. The WAV contains
   **121,487,040 frames / 2,530.98 seconds**. Its RIFF/WAVE header, 48-kHz
   stereo float32 layout and data/file lengths (971,896,320 / 971,896,364 bytes)
   were checked on the host. No waveform continuity result is claimed.

The console/environment trigger of blocking is unknown. The reported
HRESULT `0x80070016` maps in the existing client to `LeaseNotActive`, consistent
with missing heartbeats; that is an inference, not a separately traced expiry.
No new long whole-VM pause was established from this report.

Repair: control buffers at most 3,661 once-per-second progress snapshots,
enforces elapsed duration independently of worker completion, deactivates
leases and joins workers before writing `render-source.progress.txt` or console
output. Reports include control-loop interval/lifetime gaps and lease stop
time. The guest runner redirects native stdout/stderr to separate evidence
files, records process duration/exit status, and applies duration + intentional
stall + 60 seconds as a watchdog. Timeout kills only its owned child, records
exit 124 and fails the run; it never turns partial counters into a pass.
Windows PowerShell 5.1 needs the process handle retained to preserve the exit
code; the host regression caught this and the implementation was corrected.

The driver, ABI, buffer sizes, host security and Hyper-V/WSL are unchanged.
Sustained VCAB-24 continuity remains failed; VCAB-25/27/28 remain open. No new
VM run should use the old streaming-output harness.

Host verification of reporting repair (Windows x64, Rust 1.96.0; no audio or
driver opened):

| Check | Command | Result |
| --- | --- | --- |
| Tone regressions / production source guard | `cargo test --locked -p audiorouter-windows-audio --example m03_bridge_tone` | 17 pass; bounded collector and no-I/O/deadline wiring included |
| Process runner | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-vm-process.ps1` | 11 pass; `target/vm-process-tests-37d41a4680e94aa8b5f593bc99af7a7c` |
| Clippy | workspace and shell all-targets/all-features, locked, `-D warnings` | both pass; incremental hard-link cache warnings only |
| Formatting | both workspace/shell format and `-- --check` | pass |
| PowerShell syntax | `Parser.ParseFile` on changed runner/support/update/test scripts | pass |
| Documentation | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/docs.ps1` | 138 Markdown / 747 local links pass |
| Diff whitespace | `git diff --check` | pass |

The process regressions use a fake PowerShell child: preserve exit 7 and stderr,
round-trip spaces/quotes/trailing backslash, drain 10,001 lines without console
forwarding, kill a sleeping child after a one-second timeout, verify that it
exited, and retain partial output. They do not exercise a real bridge lease or
prove scheduler continuity. Local Jev remains disabled by user request.

Build a separate `repair-20261009-bounded-tone` bundle with
`tools/vm/prepare-tone-report-update.ps1`, using the verified r2 bundle as its
base. Rebuild only the static-runtime tone example; verify each original hash
before copying and generate a new manifest. Preserve identical driver bytes
and the installed guest driver. Next guest task: only the updated 30-second
Tone phase, then review its reports before considering a longer run.

Prepared candidate: `C:\VMs\ar-share\repair-20261009-bounded-tone`, static
runtime Release tool built from clean source
`658be117d8446703e15b9bba865c3a086eb8e892` at `2026-10-10 02:25:54Z`.
Tone executable SHA256:
`A0393BCEA4DBA6277DEED02977C393CBFA1DE5AAE909658E054C0D7550157CBD`.
All 30 manifest hashes independently pass; all 16 driver package files are
byte-identical to verified r2 (SYS SHA256 remains `23F741CD5D4D455840898D0A614E39BFA588B96BE8CB2C1CF5E7AD5CAF0D34B3`).
Invoking the copied VM script on the host gives its expected identity refusal
before any step runs. No driver rebuild, host trust/security change or VM
execution occurred. The next command is in the updated retest procedure.

## Guest bounded-tone 30-second verification — 2026-10-09 19:28 local

Archive `C:\VMs\ar-share\evidence-20261009-192823.zip`, independently verified
SHA256 `CBE551D5EB47E86A314AB479658DB925D9B562C0A14AE4268FB9DE4598D5650F`.
User ran the verified bounded-tone bundle against the unchanged r2 driver.
Status 2/2, tone 4/4 and collect 2/2 pass. Both final driver reports have all
five error counters zero; harness sequence gaps zero.

Leases deactivated at **30,001 ms**, whole child process **30.6044054 seconds**,
exit 0, no watchdog timeout. All 29 deferred interval snapshots are present.
Maximum capture/render worker gaps **5,615 / 5,536 us**; control-loop gap
**8,915 us**; heartbeat **77 us**, snapshot **162 us**, close **31 us**.
The recording contains 2,999 blocks / 1,439,520 frames (**29.99 seconds**);
RIFF/WAVE, 48-kHz stereo float32, data bytes 11,516,160 and file length
11,516,204 were independently checked. This is not waveform continuity,
kernel DPC or hardware latency evidence.

The reporting/deadline repair worked in this short guest run. The earlier
53-ms worker/audio-loss burst remains unresolved. Next: the updated bounded
300-second Tone command in the same guest session, with loop/listener kept
active. Review its interval/control timing and counters; no longer or stall
qualification yet. Sustained VCAB-24 and VCAB-25/27/28 remain open.

## Guest bounded-tone five-minute failure — 2026-10-09 19:35 local

Archive `C:\VMs\ar-share\evidence-20261009-193519.zip`, independently verified
SHA256 `F77E1359EC22E368420A58BDC04ABD8A26ABEB2A1E35E141A49554DCBB4E6D2B`.
Native tool exit 0, no watchdog timeout; leases stop at **300,003 ms** and
the child finishes in **300.1456619 seconds**. The reporting/deadline repair
holds for five minutes. All 299 deferred snapshots are present.

The zero-error gate fails: capture underrun **16,704** frames (348 ms), render
overrun **17,520** frames (365 ms). Other driver error counters and harness
sequence gaps remain zero. Recording: 29,963 blocks / 14,382,240 frames,
**299.63 seconds**, 370 ms short of the requested duration, close to the render
loss plus startup boundary. This relationship is evidence of actual missing
audio; no waveform or exact per-burst frame attribution is claimed.

| Progress snapshot | Capture worker gap | Render worker gap | Control loop gap | New capture underrun / render overrun |
| --- | --- | --- | --- | --- |
| 122.349 s | 310.442 ms | 310.289 ms | 317.342 ms | 16,560 / 17,232 frames |
| 184.541 s | 23.117 ms | 23.222 ms | 28.291 ms | 144 / 288 frames |

Maximum heartbeat call 372 us, snapshot 141 us, close 482 us, WAV append
30,998 us. Audio loops do mapped memory/tone arithmetic and short sleeps;
they do no file, console or native control call and share no control lock.
All three independently serviced loops are delayed in the same observation
windows. This supports a broader execution delay, but does not distinguish
guest scheduling, guest DPC/ISR, VirtualBox or host scheduling. It does not
prove a whole-VM suspension or a specific defective driver. Host System event
query at 19:31:50–19:33:25 local returned zero events. VBox.log has no trace
that resolves the thread scheduling question. No host setting was changed.

### Next diagnostic: scheduler trace

An opt-in `-TraceScheduling` mode surrounds only the bounded, redirected
native tone process with WPR `GeneralProfile.Light` in file mode. Its
context-switch/ready-thread/DPC/interrupt events are documented by Microsoft:
[General profile](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/recording-for-basic-system-diagnosis),
[command options and named instances](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/wpr-command-line-options).
Trace overhead makes the result diagnostic, not release qualification.

The VM identity guard precedes tracing. Require an available GeneralProfile,
known idle default recorder, and 2 GB guest free space. Use a fresh unique WPR
instance name for start/status/stop/cancel, always as the last argument.
Never cancel another recording. Save the owned trace before result reporting
or ZIP collection, including after native tone failure. A failed/timed-out
start prevents tone and cancels only that unique potentially partial instance.
Stop/save failure is a diagnostic failure; retain command logs and report a
failed cancellation explicitly. The callback performs only the owned child's
bounded, redirected run: no console output while recording/audio is active.
The duration + 60-second tone watchdog stays in place; WPR stop has a
120-second limit. A VM crash/pause can still prevent guest-side cleanup;
preserve the guest evidence if that occurs.

No driver, buffer, ABI, WSL/Hyper-V or host recording change. Host WPR was only
queried for status/profiles; no host recording was started. The collector's
fake-recorder ownership/lifecycle checks do not establish real ETW validity.
Next: prepare and verify a separate scheduling-trace bundle, then run one
bounded 300-second diagnostic with unchanged loop/listener settings.

Host verification of collector (no real recording or audio process launched):

- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-vm-scheduling-trace.ps1`:
  **77 checks pass**, evidence
  `target/vm-trace-tests-825a5a090f164ec6a224aa7dd5061b61`. Fake recorder cases
  cover successful save, failed native callback, existing/unknown recorder,
  missing profile, failed/timed-out startup, failed stop/cancel, failed status
  query and missing ETL. Integration invokes the actual `Invoke-ToneTool`
  against a fake native-process boundary, verifying argument forwarding,
  native exit preservation and exit 125 for failed trace save after native
  exit 0. No test recording or driver was started on the host.
- Existing owned-child process regressions: **11 checks pass**, evidence
  `target/vm-process-tests-960b5fc03f7041c8b663132ec7aa38e2`.
- Changed scripts parse; tracing wrapper refuses host invocation before any
  recording; `git diff --check` passes. Documentation acceptance: 138 Markdown
  files / 747 local links. Local Jev remains disabled by user request.
- Host `wpr -status` / `-profiles` only queried existing status/capabilities;
  `-exportprofile GeneralProfile.Light ... -filemode` saved a profile definition
  under `target/wpr-profile-review`, without starting a trace. Its actual XML
  contains CSwitch, ReadyThread, DPC, Interrupt and ThreadPriority keywords.
  This does not establish that guest recording/start/save works.

Prepare `diagnostics-20261009-scheduling-trace` separately from the retained
bounded-tone bundle. Preserve identical driver and audio executable bytes;
only the opt-in tracing scripts change. Follow the first section of the
updated retest procedure; one 300-second diagnostic, then inspect the trace.

Prepared/verified artifact:
`C:\VMs\ar-share\diagnostics-20261009-scheduling-trace`, clean source
`bf03cf26b4391f89f0395b52bf705ba7b79403e7`, built at
`2026-10-10 02:50:22Z`. All **31** manifest hashes independently verified.
All 16 driver files are identical to the bounded-tone/r2 candidate; audio
executable remains SHA256
`A0393BCEA4DBA6277DEED02977C393CBFA1DE5AAE909658E054C0D7550157CBD`.
Only script/reporting support differs. Copied wrapper refuses host tracing
before any recording or VM step. No guest diagnostic run is claimed yet.

## Guest recorder startup failure — 2026-10-09 19:53 local

Archive `C:\VMs\ar-share\evidence-20261009-195302.zip` independently verified:
SHA256 `7B481B854285D656364C44D7E8F03CD669042B4A52936E9FD3DBB1A5B3A0EABE`.
Status 2/2 passes; WPR 10.0.26100 CoreSystem lists GeneralProfile and reports
idle. Start rejects **GeneralProfile.Light.File**, error **0x80070032**, exit
-2147024846, elapsed **0.1689724 seconds**. Exact text: “The request is not
supported.” No native tone process ran. The owned cancellation returns
0xc5583000, “There are no trace profiles running,” in 0.0877787 seconds.
Trace summary Started/Saved false, RunFailed true, CleanupFailed false.

The failure is recorder compatibility, not new audio continuity evidence.
It does not isolate the unsupported keyword/provider/option. Host export and
fake lifecycle tests proved syntax/ownership, not guest recorder startup.

### Compatibility repair and short probe

Use checked-in `AudioRouterScheduling.wprp` with only ProcessThread, Loader,
CSwitch, ReadyThread, ThreadPriority, DPC and Interrupt. Exclude broad
GeneralProfile user providers, sampling, capture-state callbacks and other
system keywords. This narrows the requested features; it is a candidate
compatibility repair, not a proven cause of the original error. Microsoft's
[profile authoring](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/authoring-recording-profiles)
and [system provider](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/systemprovider)
describe this custom profile mechanism.

Preserve unique named ownership and existing-recorder refusal. Validate the
actual WPRP, copy it into evidence, record its hash, and surface recorder exit
and exact text to the user. No silent retry with global recorder ownership.
New TraceProbe phase exercises that same start/save path for two seconds,
launches no audio executable, and archives only its own small evidence folder.
Only after real guest start/save and event content review should the next
five-minute audio diagnostic run. Driver/audio bytes and host settings stay
unchanged; sustained VCAB-24 remains failed/open.

Host verification (Windows PowerShell 5.1; no real host recording/audio run):

- `tests/acceptance/m03-vm-scheduling-trace.ps1`: **128** fake-recorder,
  ownership, cleanup, custom-profile, probe and tone integration assertions
  pass. Evidence `target/vm-trace-tests-efb1bc39bf9346a5ae76492822f5233c`.
  Failed startup plus failed cancellation is now surfaced as cleanup failure;
  the observed explicit no-profiles code is distinguished from other errors.
- `tests/acceptance/m03-vm-process.ps1`: **11** process checks pass;
  `target/vm-process-tests-1c7dd45010cb49df98813b3ee2d9f77e`.
- Real host WPR **read-only** `-profiles <custom.wprp>` and
  `-profiledetails <custom.wprp>!AudioRouterScheduling.Light -filemode` both
  exit 0 and enumerate exactly the seven requested system keywords.
  This validates parsing/configuration, not actual guest start/save/events.
- Five changed scripts parse; wrapper TraceProbe refuses host invocation
  before any operation. Documentation acceptance: 138 Markdown / 747 links.
  `git diff --check` clean; no Rust/UI change; local Jev disabled by user.

Next: prepare/verify the separate `diagnostics-20261009-scheduling-trace-r2`
script bundle, then a two-second guest TraceProbe only. Review its saved event
content before a five-minute diagnostic. No audio fix or trace success claimed.

Prepared artifact: `C:\VMs\ar-share\diagnostics-20261009-scheduling-trace-r2`,
clean source `ee2048a6f73f223a7bd9e366a48ba6aac3a69a6d`, built
`2026-10-10 03:00:52Z`. All **32** manifest hashes independently pass. All
16 driver files match the previous bundle; tone executable still SHA256
`A0393BCEA4DBA6277DEED02977C393CBFA1DE5AAE909658E054C0D7550157CBD`.
Copied wrapper refuses host TraceProbe and copied WPRP parses with read-only
WPR query. Next: the guest TraceProbe command in the retest procedure; send
output and review the small copied archive before any audio test.

## Guest minimal-profile probe passes — 2026-10-09 20:03 local

Archive `C:\VMs\ar-share\evidence-20261009-200256-trace-probe-196bd5e0.zip`
independently matches SHA256
`A3E50C9558066CE4F928341768E8338DA0C0A8A248328F8430949AA588E7019B`.
All 32 bundle files verify in the guest. WPR start exit 0 in 0.1570748 s;
stop/save exit 0 in 2.0198765 s; no timeout/cleanup failure. The exact profile
hash is `CB4DE1288C25F900705D28D6E13A589221661B4BA916783936B0624F0548433C`,
matching the checked-in/copied candidate. Saved ETL length **13,631,488 bytes**.
Final collector status shows all seven requested keywords enabled, zero dropped
events and zero collector events lost. No tone executable/lease was launched.

Host inspection: extracted only ETL/profile into ignored
`target/trace-probe-review-20261009-200256`; `tracerpt.exe scheduling.etl -o
events.csv -of CSV -summary summary.txt -report report.xml -y` completes with
exit 0. Initial sandbox attempt could not access the WMI metadata service;
an approved read-only execution decoded the saved file without changing
services/settings or starting a host recording. The report identifies
AR-DRIVERTEST, build 26300, 4 processors and 8,173 MB memory. It processes
**96,771 events**, zero lost, two-second collection. Observed event groups:

- 4,548 ReadyThread; 726 DPC; 50 TimerDPC; 522 ISR records.
- 7,835 Thread / opcode 36 / version 5 records. Opcode 36 is the documented
  [context-switch event](https://learn.microsoft.com/en-us/windows/win32/etw/cswitch).
  The host decoder leaves the newer version-5 payload unnamed; its full
  payload interpretation and timing attribution have not yet been qualified.
- Process/thread lifetime, image load/rundown and thread priority records.

This proves the narrowed profile can start/save in this guest and carries the
required event families; it does not isolate the original GeneralProfile
unsupported feature or explain sustained audio loss. Named instance and file
mode work with this profile. Next: one bounded 300-second traced Tone using
the same r2 diagnostic bundle, installed driver and loop/listener. Inspect
audio timings/loss alongside the scheduler trace; no longer/stall run yet.
VCAB-24 sustained continuity and other hardware/signing gates remain open.

## Five-minute guest scheduling analysis — 2026-10-09 20:11 local

Archive `C:\VMs\ar-share\evidence-20261009-201122.zip` independently matches
SHA256 `16A48B2590608BEBC512213FCE092784BF9DCB231E280F28A487C162E01763C2`.
Affected driver source remains clean `97b393d5e25cd0e114bf0a8078627b14ab97b13b`;
native tone SHA256 remains
`A0393BCEA4DBA6277DEED02977C393CBFA1DE5AAE909658E054C0D7550157CBD`.
No new driver/audio repair or guest test was performed during this review.

### Process, recording and counters

- Native PID 3712 exits 0 after **300.1581863 s**, no watchdog timeout.
  Leases stop at 300,001 ms. Acceptance still fails: capture underrun
  **7,056 frames / 147 ms**, render overrun **14,832 frames / 309 ms**.
  Other driver counters and harness render sequence gaps are zero.
- Maximum worker gaps: capture **35,662 us**, render **35,857 us**;
  control gap **40,879 us**. Maximum control heartbeat 383 us, snapshot
  174 us, lease close 30 us. WAV append 13,852 us; finish 590 us. The writer
  runs on a separate thread; these values do not explain the coincident
  before-ready worker waits by themselves.
- Header inspection confirms playable-format RIFF/WAVE, float tag 3,
  stereo, 48,000 Hz, 32-bit, data 115,077,120 bytes, file 115,077,164 bytes.
  Recorded **14,384,640 frames / 299.68 s**, about 320 ms short. That is close
  to the 309-ms render loss plus boundary/startup time, but is not waveform
  continuity qualification. Private audio is excluded from Git.
- Counter increases occur near the 2,007-ms snapshot and again at snapshots
  237,705–246,721 ms. Snapshots are approximately one second apart and do
  not identify the exact instant each counter changed.

### Trace completeness and decoder verification

Minimal profile hash remains
`CB4DE1288C25F900705D28D6E13A589221661B4BA916783936B0624F0548433C`.
Owned instance `AudioRouterTone-bb4eb94657314166aed304976bf05ba4` starts in
0.1617771 s and saves in 2.7998765 s, both exit 0. No recorder timeout or
cleanup failure. ETL is **138,412,032 bytes**. Collector and independent
`tracerpt` report **zero lost events**. Decoding processes **3,420,594 events**:
1,717,691 context switches, 1,232,133 ReadyThread, 172,553 DPC, 67,904 ISR,
1,463 TimerDPC, plus process/thread/image/priority records.

Analysis extracts only this run into ignored
`target/trace-tone-review-20261009-200614`. Host Windows SDK `tracerpt` command:

```powershell
tracerpt.exe target\trace-tone-review-20261009-200614\scheduling.etl -o target\trace-tone-review-20261009-200614\events.csv -of CSV -summary target\trace-tone-review-20261009-200614\summary.txt -report target\trace-tone-review-20261009-200614\report.xml -y
```

Exit 0. Approved read-only metadata access is needed in this sandbox; no
host recording/service/setting change occurs. Decoded CSV is about 1.25 GB
and remains ignored. Host tracerpt leaves version-5 switch payload fields
unnamed. `-lr` with CSV was rejected and is not a verified workaround.

New [offline reader](../../../../tools/m03-scheduler-trace/README.md) uses the
checked common prefix from Microsoft's
[PerfView CSwitchTraceData parser](https://github.com/microsoft/perfview/blob/main/src/TraceEvent/Parsers/KernelTraceEventParser.cs).
Version-2 24-byte and version-5 40-byte records share these fields. Appended
fields are ignored, short/null/old switch payloads rejected. It never creates
or controls tracing sessions or accesses the driver. Verification on the host:

- Existing MSVC 14.51.36231 x64, SDK 10.0.26100.0, `/W4 /WX /O2 /MT` build
  passes with no warnings; no tool installation or repair.
- `target\m03-scheduler-trace.exe --self-test`: eight decoder checks pass.
- Saved ETL → converted-time CSV and `--raw` QPC CSV: each reports
  `ProcessTrace=0 CloseTrace=0 CSwitch=1717691 ReadyThread=1232133 Rejected=0 EventsLost=0`.
  Both counts independently match tracerpt. Existing output path is refused
  with exit 3. These qualify this parser on this trace, not the audio gate.

### Worker readiness and kernel activity

Thread names/lifetime identify main TID 1356, capture 8184, render 10848,
recording writer 1696, all belonging to native PID 3712. Match each switch-out
to its subsequent first readiness and switch-in; discard stale readiness at
switch-out and clear it at switch-in. Compare pre-readiness wait with runnable
delay. Raw QPC is 10 MHz. Approximate lease origin 62,997,286,114 ticks derives
from the final timestamp minus 300,001 ms, with sub-millisecond rounding.

Longest coincident worker wait, approximately 237.311 s after lease origin:

| Thread | Off CPU | Before ready | Ready to running | Running priority |
| --- | --- | --- | --- | --- |
| Capture | 35.6225 ms | 35.5923 ms | 30.2 us | 24 |
| Render | 35.7156 ms | 35.5546 ms | 161 us | 24 |
| Control | 40.8726 ms | 39.9214 ms | 951.2 us | 8 |
| WAV writer | 35.9641 ms | 35.5539 ms | 410.2 us | 8 |

The old state is Waiting (5), wait reason UserRequest (6). At the first loss
around 1.089 s, capture/render spend 28.7233/28.7429 ms before readiness,
then run within 142/213 us, also at priority 24. Across the complete run the
maximum ready-to-run delays are 2.6803/2.6736 ms; do not confuse these with
the much smaller runnable delays at the longest worker waits.

Source requests a 1-ms sleep in idle audio-worker loops, 5 ms in control,
1 ms in writer. The pinned
[Rust 1.96.0 Windows thread implementation](https://github.com/rust-lang/rust/blob/1.96.0/library/std/src/sys/thread/windows.rs)
uses a high-resolution waitable timer when available, creating/setting/waiting
and closing it per sleep; it can fall back to Sleep. This trace does not prove
fallback use. A timer-resolution/occluded-console explanation, or a reusable
timer as the repair, remains unverified.

There are two live-event gaps spanning all four guest processors in the loss
windows: **28.443 ms** near 1.060–1.089 s and **35.2318 ms** near
237.276–237.311 s. No context switch, ready, DPC, ISR or TimerDPC event is
recorded in those intervals. These events do not sample all execution, so
absence alone does not prove a suspended VM. Later 23–27-ms worker waits
occur while other guest events continue; one whole-VM pause does not explain
all observations. A third 10.778-ms gap occurs after leases end.

The longest individual decoded DPC is **2.7535 ms** and ISR **2.3169 ms**,
mapped by loaded-image address to dxgkrnl.sys; longest TimerDPC 96.5 us.
No individual decoded callback accounts for a 35-ms wait. There are no full
stacks/symbol attribution, and cumulative effects remain possible. In
particular, the driver's ExAllocateTimer callback can appear under a kernel
timer wrapper; absence of an address in its own image does not prove its
timer path uninvolved.

Read-only host System events for 20:06:00–20:11:35 contain no entries. Current
VirtualBox log confirms the previously observed NEM execution mode. A later
four-second GuestHeartbeat flatline is outside this run, and the NAT MTU
warning is after the loss burst; neither establishes this run's cause.
No evidence requires disabling WSL/Hyper-V or changing host power/security.

### Decision and next task

The strongest finding is **late readiness / timer wake delivery**, rather
than long runnable-queue delay at the largest loss, or the repaired synchronous
console block. It does not distinguish guest timer behavior from host/VirtualBox
execution delay and does not prove the driver fault-free. VCAB-24 sustained
continuity remains failed; VCAB-25/27/28 and hardware/signing gates remain open.

Stop repeating unchanged guest-only tone tests. Prepare paired host/guest
timing diagnostics with explicit clock alignment, owned recorder lifecycle,
bounded size/duration, privacy limits, and a short recorder probe before a new
attended audio run. Do not enlarge buffers, suppress/reset loss counters,
change driver timer semantics or host features on this correlation alone.
No new driver or VM bundle is produced. Rollback removes the offline reader
only. Preserve this archive and all earlier failed experiments.

Final host checks: rebuilt the offline reader with `/W4 /WX`, eight checks
pass, and the final executable again decodes the full saved ETL in raw mode
with matching counts and zero rejected/lost. Documentation acceptance passes
138 Markdown files / 749 local links; `git diff --check` clean. No Rust/UI
changes, so their formatting/Clippy suites were not rerun. Local Jev remains
disabled by the user's repository instruction. Diagnostic source and reviewed
findings are committed; raw traces/CSVs/audio and scratch analysis stay ignored.

## Paired recorder/clock probes prepared — 2026-10-10

User asks for the next step after stopping loop playback/listening. Prepare
recorder compatibility and clock alignment before another audio run. New
`run-paired-scheduling-host.ps1` supports HostProbe / PairProbe;
`run-paired-scheduling-guest.ps1` supports only the no-audio paired probe.
Neither launches any driver/audio/VM executable. Existing minimal WPRP and
owned-recorder cleanup are reused; driver/audio bytes are copied unchanged.

Protocol: atomic, non-overwriting messages and a unique pair ID; one offer
refuses concurrent/stale pairs, cleanup removes only the matching offer.
Bounded peer waits, identity checks and peer-failure markers. Eight UTC/QPC
request/reply brackets before and eight after the two-second guest trace keep
whole round-trip offset bounds. No symmetric-latency assumption, no clock
adjustment. Host wall-clock jumps are rejected; guest UTC around QPC read is
recorded. Alignment quality still requires real cross-machine evidence. Host
recording waits until guest join, then surrounds guest recording. A 45-second
guest measurement deadline can reject a slow save; failed evidence remains.
Host storage stop thresholds are sampled (1 GB evidence / 2 GB remaining,
4 GB required at startup); they are not a hard file-size cap. Only current
probe evidence is zipped; previous large audio history is excluded.

Host checks (Windows PowerShell 5.1, no real recorder):

- `tests/acceptance/m03-paired-trace.ps1`: **52** checks pass; atomic publish,
  no-overwrite, identity, timeout/peer errors, size guard, sixteen real
  same-machine request/reply brackets, syntax and production callback under
  recorder parameter scope. Evidence
  `target/paired-protocol-220f7907a9fe413bbc295439c8feeab5`.
  First sandbox job couldn't use its process IPC; the approved local fake
  checks succeeded unsandboxed. Same-machine brackets do not qualify guest
  clock alignment. Callback review caught and corrected caller `$run` /
  `$directory` collisions with recorder parameters before handoff.
- `tests/acceptance/m03-vm-scheduling-trace.ps1`: **128** existing fake
  recorder/lifecycle/integration checks pass. Evidence
  `target/vm-trace-tests-351fee58709445f19192abc4b996ac7e`.
- Copy-only preparation verifies all base hashes and candidate manifest;
  no build/tool repair, certificate/driver operation or audio run.
- Real candidate HostProbe invocation refuses at the administrator guard:
  the agent's Windows token is not Administrator, even with sandbox approval.
  **No real host recording started.** WPR read-only status had reported idle;
  availability/listing is not start/save evidence. Administrator PowerShell
  on the main PC is the next required user action, not another VM tone run.
- Documentation acceptance: **139 Markdown / 753 local links**, diff check
  clean. No Rust/UI changes; their suites not rerun. Local Jev disabled by
  user instruction. Raw host traces can contain process/image metadata;
  preserve locally and exclude from source control.

Next: publish/verify the copy-only `diagnostics-20261010-paired-probe` bundle.
User runs **HostProbe only**, then review ETL coverage/loss. Only after that
review should PairProbe run on both machines with audio still off. Do not
add a 300-second audio phase before reviewing real pair clocks/recordings.
The [step-by-step runbook](../../../operations/virtual-cable-paired-trace.md)
keeps host/guest commands distinct. VCAB-24/27 and other gates remain open.
Rollback omits the optional probe scripts; there are no setting changes.

Published copy-only artifact:
`C:\VMs\ar-share\diagnostics-20261010-paired-probe`, clean source
`88f6e808d5f802cd73a1b794f418dc15002eab1a`, prepared
`2026-10-10 03:44:25Z`. All **35** published manifest hashes independently
verify; all **32** base files, including all 16 driver files and tone executable,
are byte-identical to the verified scheduling-trace-r2 bundle. Three new probe
scripts are the only bundle additions. MANIFEST SHA256
`2852AEA2A4DA59F43F74869FF45BC4B692A432B62E6C696862838D7F8C140386`.
No dependency build/repair or Windows/VM setting changed. Next user step remains
HostProbe in Administrator PowerShell on the main PC; no paired/audio success
is claimed. Real host recording was not bypassed after the administrator refusal.

## Host recorder probe reviewed — 2026-10-09 20:45 local

User ran HostProbe in Administrator PowerShell on the main PC. Archive
`C:\VMs\ar-share\diagnostics-20261010-paired-probe\paired-6ac9b6ced68f41dc89c99d1861642218-host.zip`
independently matches SHA256
`89ACCA58C665D234621A8A20955F2FDC0FB0BE471E993767B38D29B4AEC9316A`.
Run result Passed true; trace Started/Saved true, no run/cleanup failure.
Start exit 0 in **0.8701411 s**, stop/save exit 0 in **10.4983717 s**; no
timeout. Profile hash matches
`CB4DE1288C25F900705D28D6E13A589221661B4BA916783936B0624F0548433C`.
ETL length **72,351,744 bytes**. Collector reports the seven intended keywords,
zero dropped and zero events lost. No audio or driver operation was launched.

Extracted only ETL/profile/result/summary to ignored
`target/host-probe-review-6ac9b6ce`. Approved read-only host metadata decoding:
`tracerpt.exe scheduling.etl -o events.csv -of CSV -summary summary.txt -report
report.xml -y` exits 0; **717,343 events**, **zero lost**, three-second trace
span surrounding the two-second callback. Observed **232,406** version-5
context switches, **141,391** ReadyThread, **25,390** DPC, **8,019** ISR,
**7,286** ISR-MSI, **428** TimerDPC, plus process/thread/image/priority records
and recorder metadata/rundown. The file contains useful event families;
do not treat total event count as exclusively the minimal live keywords.

`target\m03-scheduler-trace.exe scheduling.etl switches-qpc.csv --raw` on this
saved host ETL reports `ProcessTrace=0 CloseTrace=0 CSwitch=232406
ReadyThread=141391 Rejected=0 EventsLost=0`, matching independent counts.
Read-only `wpr.exe -status` reports **WPR is not recording** after the user
run. This verifies real host recorder compatibility and event coverage,
not host/guest clock alignment or audio continuity. Raw trace stays local.

Next: one paired two-second probe, loop/listener still off. Copy the guest
bundle first; start host PairProbe, then guest join within two minutes. Review
both saved traces, before/after clock brackets and their uncertainty before
preparing a 300-second audio phase. No new bundle or driver change needed.

## Paired probe reviewed; short audio phase prepared — 2026-10-10

Pair `02d977908cf14e5cbfd31f33eb5103ae` is reviewed from the local share.
Both ZIP SHA256 values independently match:

- Host: `8E4EC210BCF60B9BAEC0185DF3469B5ADF11FCE930171EDCF150706E29A63428`.
- Guest: `A51355595B883CCAD31BEB75BFF58D2F8D4006604C6DA4537E98CB06C9DF449E`.

Both results Passed true, Started/Saved true, no run/cleanup failures. Host
start/save exit 0 in 0.2425024/9.4529072 s; guest in 0.1603831/2.12579 s.
Both profile hashes match
`CB4DE1288C25F900705D28D6E13A589221661B4BA916783936B0624F0548433C`.
Host ETL 84,934,656 bytes; guest 12,582,912. Intended collector keywords,
zero dropped/lost. No audio was launched.

Selected entries extracted into ignored `target/pair-probe-review-02d97790`.
Read-only `tracerpt.exe scheduling.etl -o events.csv -of CSV -summary
summary.txt -report report.xml -y` exits 0 for both files: 1,090,583 host
events, 94,301 guest, zero lost. Offline `target\m03-scheduler-trace.exe
scheduling.etl switches-qpc.csv --raw` and converted mode both match
independent context-switch/readiness counts:

| Trace | Context switches | ReadyThread | Rejected | Lost |
| --- | ---: | ---: | ---: | ---: |
| Host | 459,738 | 267,316 | 0 | 0 |
| Guest | 6,855 | 3,921 | 0 | 0 |

Both headers identify QPC clock type and 10,000,000-Hz frequency. Raw first/last
event spans are 5.9997261 s host and 2.3507886 s guest; converted spans match.
Guest wall-clock header/report duration differs from raw QPC elapsed time.

Clock finding: guest UTC is about 19.7 s behind host before, 21.1 s afterward.
Tightest before/after round trips are 30.6668/30.0645 ms. Between their guest
samples UTC advances 3.8900716 s while QPC advances 5.3119743 s: a
1.4219027-s discrepancy, much larger than round-trip uncertainty. UTC offset
intervals are incompatible even within stages. UTC-only alignment is invalid;
this does not establish the cause of audio loss. Do not adjust either clock.

Normalized QPC offset bounds in ms are
`(guestQpc/guestFrequency - hostEndQpc/hostFrequency)*1000` through
`(guestQpc/guestFrequency - hostStartQpc/hostFrequency)*1000`. All sixteen
samples have a compatible interval `[-88295933.49521, -88295911.0389] ms`,
width **22.45631 ms**. Absolute offset reflects different counter origins.
This is short-probe compatibility, not sub-millisecond synchronization or
proof of a stable offset over 300 s. Use raw ETW QPC, check before/after
overlap again and retain uncertainty for every new run.

Implementation (VCAB-24/27 diagnostic; VDEV-12 ownership): support exports
normalized QPC bounds. Explicit matching Tone phases coordinate a bounded
30/300-second guest run using unchanged traced VM checks/native watchdog and
zero-counter gate. Guest status before joining is read-only. Failed audio
acceptance still gets after-clock samples and both trace saves, then both
scripts report failure. Only the uniquely identified new guest tone directory
is archived. Large host ETL stays separate from small metadata ZIP. Host
budgets: 16 GB initial/8 GB reserve/6 GB sampled evidence stop; save can add
bytes. No host audio/driver call, build, repair, install, VM control or setting
change occurred.

Host Windows PowerShell 5.1 checks (fake native/recorder boundaries):

- `tests/acceptance/m03-paired-trace.ps1`: **88** checks pass, evidence
  `target/paired-protocol-2d7aa87675f74fb18f59bd4a28e2917e`. Actual production
  callbacks under recorder parameter scope, both tone durations, process
  watchdog/arguments, failed status/acceptance, current evidence only,
  phase/duration/identity refusal, sixteen same-machine clock brackets.
  Same-machine checks do not qualify cross-machine alignment or real Tone.
- `tests/acceptance/m03-vm-scheduling-trace.ps1`: **128** fake lifecycle checks
  pass, evidence `target/vm-trace-tests-03ecc5c063dc4c0bb921b8472d6d9467`.
- Documentation acceptance passes **139 Markdown / 754 local links**; syntax
  and diff checks pass. No Rust/UI changes; their suites not rerun.
  Local Jev disabled by user repository instruction.

Next: commit/push and prepare/verify separate copy-only
`diagnostics-20261010-paired-tone` from the 32-file base. User runs **30 seconds
only** with loop/listener already running; review both traces and clock quality
before any longer run. [Current steps](../../../operations/virtual-cable-paired-trace.md)
separate host and guest. No paired Tone has run yet. Continuity, latency,
hardware, signing and other unqualified gates stay open. Rollback: keep the
probe-only bundle and omit optional paired Tone scripts.

Published artifact: `C:\VMs\ar-share\diagnostics-20261010-paired-tone`, clean
source `565c9e6df3058e536a1be22f641e04580e8bc0a4`, prepared
`2026-10-10 04:34:19Z`. Independent verification matches all **35** hashes,
all **32** base files and the three scripts against committed source. Manifest
SHA256 `1A1B766640C38CC8244B4F4B5C33E68C436BE32C116EF00097B45980547BAA3D`.
All 16 driver files and tone executable remain byte-identical to the retained
base. Host C: has 711.9 GB free at verification; scripts recheck at invocation.
Preparation was copy-only, no dependency repair/build or real recorder/audio
run. Source commit pushed to main. Next user step: current runbook's 30-second
paired Tone with loop/listener running; send both outputs before a longer run.

## Paired 30 seconds reviewed; audible hiss blocks longer tests — 2026-10-09

Run `2efe2c4cb0ae466a8536fbe49ed2ac74`, guest tone
`20261009-213832-tone`. Local share ZIP hashes independently verified:

- Guest `7A321BDADBB03E695FE31733B43EED4992CB1A2D18C21306688C372B8F5FB2F4`
  matches user output.
- Host metadata `877D5BDF91460B6B48A91AC335F61B2148D17A977553142896C0B68DB6AFF2EC`.
- Separate host ETL `7043B0A37E69E46EB1728800E1DCAB851D6F01F374FA4A022A0D0F7DCFB8E3FA`.
- Guest ETL `DA332E4BC6B7B84CBC873A9880B6FDE47E2DC8575DC53445E6D96F75CB02EB43`.

Both coordination results pass, guest tone exit 0. Native process elapsed
30.1754341 s, no timeout; leases deactivated at 30,004 ms. All ten final
driver counters and harness render sequence gaps zero. Capture/render maximum
pump gaps 8.168/7.911 ms, control gap 9.396 ms. Recorded 3,000 blocks and
1,440,000 frames. Guest status reports four endpoints, 60/60 formats each.
WAV SHA256 `62B05A78185585033966A43FDC36C25AF40C52B3FED41909F5C6623FCC19C321`:
RIFF sizes/chunks consistent, stereo IEEE float32/48 kHz, exactly 30 s,
no non-finite samples, peaks 0.25. These checks do not prove audible quality.

Both minimal profiles match prior hash. Host start/save exit 0 in
0.3515851/9.6653798 s; guest in 0.1678924/1.9215072 s. ETL sizes
202,375,168/26,214,400 bytes. Offline extraction/reports stay ignored under
`target/pair-tone-review-2efe2c4c`. Approved read-only
`tracerpt.exe <saved.etl> -o NUL -of CSV -summary <summary.txt> -report
<report.xml> -y` exits 0 for both, without retaining full process metadata CSV.
The existing offline reader with `--raw` also exits 0 and matches independent
scheduling counts:

| Trace | Total events | Context switches | ReadyThread | Lost/rejected |
| --- | ---: | ---: | ---: | ---: |
| Host | 4,154,879 | 2,219,737 | 1,312,290 | 0 |
| Guest | 418,391 | 176,757 | 112,951 | 0 |

Raw scheduling spans: host 35.0228132 s, guest 30.5091121 s. Sixteen normalized
QPC brackets intersect at `[-88295932.4433, -88295909.3556] ms`, width
**23.0877 ms**. Guest span maps wholly within host span under these bounds.
Tightest before/after RTT 30.1616/31.4937 ms. UTC offset bounds change from
`[247.1254, 277.287] ms` to `[-520.6932, -489.1995] ms`; UTC-only alignment
remains invalid. QPC compatibility applies to this short run, not a future
300-second measurement. Read-only `wpr.exe -status`: not recording afterward.

User reports **continuous static/hiss** mixed with audible high-pitched beep,
stopping when the script finished. Guest Audacity is not installed. Treat
audio quality as unresolved despite counter PASS. Current WAV records
Media Player → Cable A Input → render bridge; the complained-of path is
tone bridge → Cable B Output → Listen → emulated speakers → host playback.
There is no direct Cable B recording here. Source read covers absolute-phase
997/47-Hz generator, acknowledged publication, bounded capture queue and
integer/float endpoint conversion; no specific owning defect is demonstrated
by these counters or the Cable A recording. Do not label hiss an expected
tone or blame VirtualBox/host without waveform evidence.

Exploratory Cable A sine fit sees a startup phase difference; a global fit
does not prove steady noise. It is not evidence about Cable B and is not
used to close any quality gate. Raw audio/analysis remain private and ignored.
No new recording/test, dependency installation, driver/audio code or settings
changed during this offline review.

Next task (VCAB-24/27 diagnostics): prepare a bounded exact-endpoint Cable B
capture using existing Windows capture code, preserving negotiated format,
packet flags/positions and a directly recorded WAV for reference comparison.
No default/microphone fallback. Hold longer/stall runs until that signal is
reviewed and the owning failure is identified. Existing bundles stay retained;
WSL/Hyper-V, latency/counter/hardware/signing gates remain unchanged.
