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
