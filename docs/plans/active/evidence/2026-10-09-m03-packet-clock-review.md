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
