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
