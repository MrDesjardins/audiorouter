# r16 — bounded driver notification/fetch timing

## Objective and prerequisites

User authorized implementation, timing models, independent WP-04 review,
candidate packaging and one short guest command after the
[r15 investigation](2026-10-10-m03-cable-latency-preparation.md#r15-returned-focused-scheduling-review-2026-10-10).
Requirements: SEC-08, VDEV-07/12, VCAB-20/24/25/28. This candidate diagnoses
timing; it contains no speculative continuity repair and cannot qualify
VCAB-24/25. Host is build/offline only. Loading/audio happens only in
AR-DriverTest, by the user. Preserve host security, WSL/Hyper-V and audio.

## Design and implementation

Opt-in `build.ps1 -TimingDiagnostics` sets AR_TIMING_DIAGNOSTICS. Normal
builds omit trace storage, stream fields, calls and diagnostic dispatch.
The diagnostic contains 131,072 fixed 64-byte records (8 MiB resident)
with raw QPC, monotonic stream ID, bus/direction, packet count, linear byte
position, status and event-specific scalars. No audio samples or pointers.
Records identify timer entry, timer progress, notification, fetch entry/
return and RUN format. The [contract](../../../../tools/m03-timing-trace/README.md)
defines snapshot times and known observation gaps. Audio periods, buffering,
packet selection, counters and timer lifecycle are unchanged.

Writers acquire one atomic gate admission using at most 16 CAS attempts,
then a unique append-only slot using at most 16 CAS attempts. Capacity
saturates. STOP closes admission, waits for admitted writers only at
PASSIVE_LEVEL and allows bounded export after drain. START advances a
saturating epoch and rejects entries dated before START. Cleanup and unload
close/drain/scrub. No new callback allocation, wait, file I/O or logging.

Private diagnostic IOCTL 0x0022E010 uses METHOD_BUFFERED, exact 16-byte
request and 8,232-byte response, reserved/action/offset validation, enabled
SeLoadDriverPrivilege and owning FILE_OBJECT. Request is copied locally
before response overwrite; outputs/unused records are zeroed. Export
contains at most 128 records and occurs after the audio child stops.
The helper enables privilege in its own elevated guest process, checks both
AR-DriverTest and C:\ar\IS_TEST_VM before opening the device, and owns one
suspended child assigned to a kill-on-close job with a 120-second watchdog.

`Dropped` combines per-session reservation failures with never-reset,
lifetime-cumulative admission failures. The latter is not frozen by STOP.
Any nonzero count or exhaustion invalidates the diagnostic conservatively.
The runner compares one disabled baseline with one enabled, identical
1,000-impulse low-latency route and archives both current measurement ZIPs,
raw kernel CSV and failure logs. No WPR or global cancellation. Wrapper
outer watchdog is 150 seconds; normal comparison is about one minute.

## Timing model and prior route loss

`tools/vm/model-cable-capture-timing.py` checks exact/late packet boundaries,
delayed timer versus delayed fetch (same packet, different notification
history), timely OS fetch versus delayed client read, coalesced notifications,
lock acquisition delay, and partially reused DMA slots. Fraction-based
microsecond sweep passes. Both illustrative 264-frame and measured r15
280-frame client rings hold two complete 128-frame packets, not three.
Returning an old overwritten slot remains unsupported.

Separate r15 impulse 720 review: observed input sequence 2742 at QPC
115205308851 contains code 1 at frame 64. Before publication, relay depth
reaches 3; dropped counter changes 2→3 at capture sequence 2809, QPC
115205370503. `relay_forward` discards oldest when fixed buffers are in use.
This correlates the one input/publication loss with a relay drop, not an
engine DSP corruption. It does not identify why the producer/consumer timing
diverged. No relay policy or buffer change was made.

## Independent WP-04 review

A fresh-context preliminary reviewer found two admission/accounting races
during development: a failed non-admitted writer could increment a reset
session counter; an unconditional ExchangeAdd admission could corrupt a
reopened epoch. Both alternatives were removed. The final design uses
open-state CAS admission plus an independent lifetime-cumulative admission
counter, atomic throughout and never reset. These decisions are reflected
in concurrency/stop/restart tests and the diagnostic contract.

User requested GPT-6 Luna High. A second fresh-context reviewer with that
model/effort inspected the final diff and found **no blocking source defect**.
Follow-up confirmed shared validator, epoch saturation and VM-marker
hardening introduce no blocker. Reviewed kernel user-memory path:
`HandleTiming` (only new consumer/producer of SystemBuffer);
`BridgeControlDeviceControl` serialized dispatch; `BridgeControlCreateClose`
→ `ReleaseLeasesOwnedByFileObject` → `ReleaseTiming` owner cleanup;
`DeleteBridgeControlDevice` unload scrub. Existing bridge validation unchanged.
Reviewed publication/lifetime: `AudioRouterTimingRecord`, buffer admission/
reservation/close/reset/open, `RecordTiming`, `GetReadPacket`, `TimerNotifyRT`,
`SetState`, stream ID assignment, compile-time exclusion. Reviewed helper
privilege, VM guard, owned child/job watchdog and post-stop chunk validation,
wrapper evidence handling and clean-source package preparation. Reviewers
ran no audio or driver installation and did not claim runtime evidence.

Microsoft sources used to check IRQL/privilege contracts:
[SeSinglePrivilegeCheck](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddk/nf-ntddk-sesingleprivilegecheck)
(PASSIVE_LEVEL, current subject),
[EXT_CALLBACK](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nc-wdm-ext_callback)
(DISPATCH_LEVEL), and
[timer deletion](https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/deleting-a-system-allocated-timer-object)
(existing wait/join before stream destruction).

## Host verification and limitations

Windows host, WDK 10.0.28000.0, MSVC 14.51.36231; no driver load/audio.

| Command/check | Result | Evidence |
|---|---|---|
| `drivers/audiorouter-virtual/tests/build-tests.ps1` | 738 checks pass | `target/driver-unit-release/tests.log` |
| `tools/m03-timing-trace/build.ps1 -StaticRuntime` | /O2 /W4 /WX, static build passes | `target/m03-timing-trace/m03-timing-trace.exe` |
| Timing helper `selftest` | ABI/request bounds, 8 concurrent writers, exhaustion, epoch saturation, old-QPC rejection, 100 stop/drain cycles pass | Host stdout, same executable |
| Direct helper attempt on host | Expected exit 1 before device open | `Guest-only: AR-DriverTest required. No device opened.` |
| `tests/acceptance/m03-cable-driver-timing.ps1` | 58 fake-boundary checks pass | `target/driver-timing-tests-f600d723da6e49a0b2d24800bd1c416c` |
| `tests/acceptance/m03-cable-latency-starts.ps1` | Existing suite passes, no audio | `target/cable-latency-starts-tests-b45506bfdbfa4ce0a837183afc86d3ad` |
| Fraction timing model | All assertions/microsecond sweep pass | `tools/vm/model-cable-capture-timing.py` |
| Normal x64 WDK build | Pass, no sign/install | `target/r16-driver-normal/build.log` |
| Diagnostic x64 WDK build 0.1.1 | Pass, no sign/install | `target/r16-driver-final-check/build.log` |
| PE storage inspection | .data virtual bytes normal 21,652 vs diagnostic 8,410,356 | `target/r16-pe-review.py` |

Initial diagnostic build failed on omitted declarations in the sample target
headers; a documented SeSinglePrivilegeCheck export declaration and literal
privilege LUID resolve it. Another attempt hit mismatched MSPDB140 DLL for
HostX86 linker tools. Setting PreferredToolArchitecture=x64 for this build
selects the installed matching tools and builds pass; no persistent system
change or unrelated process termination. The epoch-saturation test initially
poisoned later fixture state; it now runs last and all cases pass.

PE storage confirms normal-build trace allocation is absent; it is not binary
equivalence or timing evidence. Diagnostic SYS is about 8 MiB larger.
Instrumentation overhead, sample-time inference, uninstrumented position
query progress and VirtualBox NEM remain limits. Only the returned guest
disabled/enabled comparison can measure observer impact and notification/
fetch delays. Verifier/fuzz, native/bare-metal continuity, memory, signing
and release gates are not waived. No hour run yet.

## Clean candidate and immutable bundle — ready

Reviewed source commit: `8679e6e7fa7e7af4ef0f3ef9b4019c3df2c8c3cb`.
Clean x64 Release test-signed diagnostic package 0.1.1/0.1.1.0:
`target/r16-driver-8679e6e7`, built 2026-10-11 06:03:50 UTC.
`tests/acceptance/m03-driver-package.ps1 -Package target/r16-driver-8679e6e7`
passes 33 checks, evidence `target/driver-package-test-f9ea361275584f03995c05ff69f02c3b`.
Signature verification identifies the expected test signer; the host does
not trust that test root (expected trust-chain failure), and no trust was added.

| Artifact | SHA256 |
|---|---|
| SYS | `CE19FE2545CAD9E820F47CBCC3221C30F64ED3882231CB2C94B132C54B7457AD` |
| CAT | `F090EAD188F3A1272937A7794BF2E3B3A23BBDC7514A67032D1F4F12968A5AFC` |
| INF | `67BEBDFB08E3F218D9AF8F31858F45D7E46859A920A53D0624B523BC6AD078BB` |
| CER | `92A1CBDDEA789EABFF5B0D0A1FD5ECFA7C655433B1A9C122FB8F3EEA7C550E4D` |
| Bundle MANIFEST | `DA12FC85D4EB977952B3B28BB7C574E5E9E700701EED82B0AAA6F32559B09DD8` |

Test signer thumbprint `FF6876FBE50A74DC0B69077C11DC28A1A9FAAD40`;
CER is byte-identical to the retained slot-provenance bundle already trusted
by the guest. No additional guest trust import is needed.

Prepared `C:\VMs\ar-share\diagnostics-20261010-cable-driver-timing-r16`
using `prepare-cable-latency-update.ps1`, retained slot-provenance base and
`-TimingDriverPackage target/r16-driver-8679e6e7`. Static helper selftests
and PE import checks pass; every one of the 41 manifest entries was
independently recomputed with Get-FileHash and matches. Bundle is immutable.
The returned r12 result reports `CablesBefore=2`, `CableCountRestored=true`,
consistent with the existing install wrapper's four-endpoint check.

## First guest attempt (2026-10-10) — install passed, comparison not started

User-run in AR-DriverTest. `vm-checks.ps1 -Step install` passed 3 checks
(helper exit 0, four AudioRouter endpoints, idempotent second install),
evidence `C:\ar\evidence\20261010-231326-install`. The earlier exit-5
report did not recur, so no guest restart was needed.

`run-cable-driver-timing.ps1` then threw `Run only inside AR-DriverTest
from C:\ar.` at its first line, twice, before creating any evidence. The
driver was not exercised. Unlike the earlier run scripts, which check only
the computer name and the `C:\ar\*` bundle path, this wrapper also requires
the `C:\ar\IS_TEST_VM` marker. The documented guest setup creates that
marker only when the computer name is different
([testing guide](../../../operations/virtual-cable-testing.md)). The r15
scripts passed the name and path checks in this guest, so the missing marker
is the likely cause. The guard stays unchanged. Next: inside the guest only,
confirm the name, create the marker if it is absent, then run the comparison.
No timing data exists yet.

## Second guest attempt (2026-10-10) — invalid, guest-wide pause

The user created the marker in the guest and ran the comparison. Archive:
`Z:\diagnostics-20261010-cable-driver-timing-r16\driver-timing-e24cc619429a4511bc14bc6658b3b794.zip`
(inner measurement archives `starts-engine-acb1d737…` disabled,
`starts-engine-afca290b…` enabled). The wrapper failed as designed:
`Passed=false`, enabled child exit 1.

| | Disabled baseline | Enabled diagnostic | r15 (reference) |
|---|---|---|---|
| Route | exit 0, 20 s, 4,150 quanta, 0 silent | exit 1 after ~7.7 s, 294 quanta | — |
| Lost impulses | 61/1,000 | 521 (run cut short) | 84 |
| p95 / jitter | 19.314 / 8.296 ms | 19.616 / 12.863 ms (20 segments) | 19.076 / 7.390 ms |
| Max pump/control gap | 23.8 ms | 4,786 ms | — |
| Packet writes accepted/late/overrun | 3,832 / 187 / 43 | 255 / 24 / 6 | — |

Timing records: 23,377, `dropped_including_lifetime_admission=0`,
`exhausted=0`; the record set itself is complete.

Cause of the enabled failure: at about 2.9 s into the route, the capture worker,
render worker and control loop all stopped for 4.77 s. The kernel records
show the same gap at the driver's own timer (event 1) and contain no record
from any stream between QPC 3283865561 and 3331580903. An earlier 338 ms gap
of the same kind occurs at QPC 3280174455. The timing helper only waits on
its child during capture, and record writers are bounded CAS operations,
so neither explains a pause of the whole guest, including kernel timers.
This matches the validated NEM lesson (all-CPU guest silences). The pause
exceeded the bridge heartbeat, the driver expired the lease (designed
behavior) and the next bridge call returned `STATUS_INVALID_DEVICE_STATE`,
which user mode reported as `0x80070016` ("The device does not recognize the
command") in `start1-low-latency-tone-stderr.txt`.

Conclusion: the enabled run is invalid for comparison; no repair can be
chosen from it, and the instrumentation's effect on continuity remains
unmeasured. The disabled baseline is valid and consistent with r15. The
records before the pause are too short and include a 338 ms pause, so they
were not analysed further. Next: quiet the guest (`quiet-guest.ps1
-Status`, `-Apply` if not active, which waits 900 s to settle), then repeat
the same comparison once. If a guest-wide pause recurs while quieted,
add the scheduler trace instead of repeating.

**Correction (same day):** `quiet-guest.ps1 -Status` showed the guest was
already quieted during this run (Windows Update paused until
2026-10-18T01:19:20Z, maintenance disabled, Defender exclusions 1/1 paths and
4/4 processes); `-Apply` refused because it was already applied. So the
pause happened in a quieted guest. Earlier traced runs found whole-guest
silences of at most 19.1 ms inside a run and 163.7 ms anywhere in an ETL.
This run had 338 ms and 4,770 ms, and only in the enabled half. One run
cannot attribute that. A VM suspension and something in the diagnostic
build that holds every CPU both remain possible, so the earlier "not caused
by the instrumentation" reasoning is withdrawn until a trace decides it.
Next: one run of the same comparison inside the bundle's existing
`Invoke-DriverVmSchedulingRun` (WPR `AudioRouterScheduling.Light`, all files
covered by the bundle MANIFEST). No new bundle is needed. If ETW shows CPUs
active (DPC/ISR or a spinning thread) during a pause, investigate the record
path; if every CPU is silent including clock interrupts, treat it as a host
suspension.

Guest block (Administrator PowerShell, AR-DriverTest only, no reinstall):

```powershell
& {
    $ErrorActionPreference = 'Stop'
    if ($env:COMPUTERNAME -ine 'AR-DriverTest') { throw 'Run only inside AR-DriverTest.' }
    # This console only; the guest's machine policy is unchanged.
    Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
    $b = 'C:\ar\diagnostics-20261010-cable-driver-timing-r16'
    . "$b\repo\tests\acceptance\m03-driver-vm-support.ps1"
    . "$b\vm-scheduling-trace.ps1"
    $e = Join-Path 'C:\ar\evidence' ('driver-timing-traced-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $e | Out-Null
    $ps = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
    try {
        Invoke-DriverVmSchedulingRun -Recorder "$env:SystemRoot\System32\wpr.exe" -Directory "$e\trace" -ProfilePath "$b\AudioRouterScheduling.wprp" -Run {
            $r = Invoke-DriverVmProcess -Executable $ps -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',"$b\run-cable-driver-timing.ps1") -Stdout "$e\comparison.txt" -Stderr "$e\comparison-stderr.txt" -TimeoutSeconds 330
            $r | ConvertTo-Json | Set-Content -LiteralPath "$e\comparison-process.json"
        }
    } finally {
        Compress-Archive -LiteralPath $e -DestinationPath "$e.zip"
        Copy-Item -LiteralPath "$e.zip" -Destination "Z:\diagnostics-20261010-cable-driver-timing-r16\"
        Get-Content -LiteralPath "$e\comparison.txt", "$e\comparison-stderr.txt" -ErrorAction SilentlyContinue
        Write-Host "Copied to Z:\diagnostics-20261010-cable-driver-timing-r16\$(Split-Path -Leaf $e).zip"
    }
}
```

## Third guest attempt (2026-10-10) — valid, traced; capture skip identified

User-run with the console-only execution policy and WPR wrapper above.
Archives on the share: `driver-timing-traced-76ee37c5842248f1b37491b315d2015b.zip`
(ETL plus wrapper logs) containing comparison
`driver-timing-0e9a16913a984dedbe492a0f629e8311.zip` (`Passed=true`, both
children exit 0, about 22 s each). WPR: 648,094 events, 0 lost, 47.0 s.
Timing records 41,403, dropped 0, exhausted 0. The guide's IS_TEST_VM marker now exists in the guest.

| | Disabled baseline | Enabled diagnostic |
|---|---|---|
| Route | exit 0, 4,148 quanta, 0 silent | exit 0, 4,145 quanta, 0 silent |
| Lost impulses | 70 | 41 |
| p50 / p95 / jitter | 16.771 / 19.099 / 11.398 ms | 18.277 / 19.727 / 10.284 ms |
| Probe capture dropped frames | 27,136 | 15,232 (119 flagged packets) |
| Render discontinuities | 95 (148.8 ms) | 73 (62.1 ms) |
| Packet writes accepted/late/overrun | 3,852 / 166 / 52 | 3,978 / 105 / 38 |
| Max control loop gap | 24.3 ms | 24.4 ms |

Instrumentation did not worsen continuity in this pair. The 4.77 s pause did
not recur.

**Whole-guest silences** (`m03-scheduler-trace --silences`, 5 ms): 192 in
the ETL, but all except four lie between the measurements while
PowerShell starts processes and the 1 ms timer resolution is not active. Inside the enabled
probe window (ETL 26.2–37.4 s) there is one, 13.5 ms at probe stop.
`--activity` (1 s buckets, audiodg/m03_bridge_tone/m00-probe): steady
audio-thread maximum wait before readiness 4.6–6.1 ms, ready-to-running
at most 1.65 ms, CPUs 0–7 % busy. This run's losses are not VM
pauses or CPU contention.

**Kernel timing** (`tools/vm/review-driver-timing.py`, enabled run):

| Capture stream 192 (Cable B, probe) | min | p50 | p95 | p99 | max |
|---|---|---|---|---|---|
| Driver timer interval | 0.911 | 1.625 | 2.597 | 2.700 | 4.064 ms |
| Notification after true packet boundary | 0.002 | 1.217 | 2.332 | 2.636 | 3.282 ms |
| Notification → GetReadPacket return | — | 0.133 | 0.248 | 0.353 | 0.888 ms |
| True boundary → fetch return | 0.001 | 1.322 | 2.403 | 2.694 | 3.120 ms |

Position is exactly `floor(ms since RUN) × 384 B` for every update (0
deviations), so a 128-frame (1,024 B, 2.667 ms) packet is detected only at
the next whole-millisecond step taken by a timer that fires every 1.6 ms
median. Windows fetches promptly after each notification. The 102
non-success fetch returns are `STATUS_DEVICE_NOT_READY` polls.

**Capture loss mechanism.** The probe's 119 flagged capture packets
(15,232 frames = 119 × 128) equal exactly the 119 packet numbers the
driver skipped in `GetReadPacket`. That routine, unchanged from the
Microsoft sample, always returns the *newest* completed packet
(`packetCounter − 1`) and hardcodes `MoreData = FALSE`. With two packets
per WaveRT buffer, packet p stays intact until the DMA writer
(`WriteBytes`, under the position lock) reaches the start of packet p + 2.
Of the 119 skipped packets, **115 were intact** at the fetch that skipped
them, all with exactly 0 bytes left. Only 4 had begun reuse, by 128–384 B. No returned packet had begun reuse.
Example, packet 55: boundary 149.33 ms; the 149.874 ms timer update's
floored position 57,216 B stays below the boundary at 57,344 B; next timer at 151.983 ms
notifies; Windows fetches at 152.047 ms, its own update reaches 58,368 B
(start of packet 57), the counter reads 57, and packet 56 is returned
while intact packet 55 is dropped. Earlier invalid run: of 1,992
skips, about 1,800 fall in the 4.77 s pause; the rest show the same
pattern, plus late fetches.

This explains the capture side of the probe's loss. It does not cover the
render-side underruns, late packet writes or the r15 8 ms probe waits.
Those remain separate.

## Repair r17 — return the oldest intact unread capture packet (planned)

Requirement IDs: VCAB-24/25/28, VDEV-07. Scope: `GetReadPacket` only
(capture packet selection, timestamp and `MoreData`). Timer cadence,
periods, the ms-quantized position, buffering, bridge, render path and
counters are unchanged.

Rule: let `next = lastRead + 1` (wrapping 32-bit packet numbers),
`newest = packetCounter − 1` and N = notifications per buffer. If no new
packet, `STATUS_DEVICE_NOT_READY` as now. Return `next` when its slot is
intact (`linear ≤ (next + N) × packetSize`, so no byte of packet next + N
is written). Its timestamp is the QPC of its own first frame, and
`MoreData = (next < newest)`. In general return the oldest intact packet
`q = min(newest, max(next, ceil(linear / packetSize) − N))`. When `next`'s
slot was really reused, that skips only the overwritten packets, and in the
worst case it is today's `newest`. Use 64-bit linear-packet arithmetic
so packet-number wrap stays correct.

Known residual risk (sharpened by review): with two whole-packet slots,
**every** rescued older packet is returned with exactly 0 bytes of
margin. An older packet is intact only at `linear == (q + 2) × packetSize`,
the same point at which the counter completes. The next whole-millisecond
update, from a timer DPC on any CPU, writes 384 B of packet q + 2 into its
slot. Windows copies after return, outside the driver lock, so a copy still
in progress then is partly overwritten without a flag. The reviewer
estimates the chance per rescued packet at copy latency divided by 1 ms,
plausibly a few percent and worse under VM timer delays. No margin is
possible without losing the rescue. It is still a better trade than a
certain 2.667 ms flagged drop. The guest comparison must count corrupted
impulses as well as lost ones; a unit sweep pins the 0-byte property.

Validation: host timing model cases (on-time, late-by-one at 0 bytes,
partly reused, multiple behind, wrap, MoreData chain, startup) in
`model-cable-capture-timing.py` and driver unit checks for the selection
helper; normal and diagnostic WDK builds; independent WP-04 review of the
diff; clean test-signed diagnostic package 0.1.2; short guest comparison
with the r16 wrapper. Pass criteria: intact-slot skips 0 in the timing
CSV, probe flagged capture packets reduced by at least 80 % versus this run
(119), corrupted impulses still 0, and no new failures. Rollback: reinstall the r16
0.1.1 bundle or the retained 492d8ca8/0.1.0 package with its install
step. No hour-long run until the short comparison passes.

### r17 implementation and host verification (2026-10-10, uncommitted)

`AudioRouterSelectReadPacket` (`Source/Inc/streamtiming.h`) implements the
rule as constant-work 64-bit arithmetic. `GetReadPacket` uses it for packet,
timestamp and `MoreData`, and still records `m_ulLastOsReadPacket` as the
32-bit number returned. Lock usage, the diagnostics hooks and all other
paths are unchanged. `tools/vm/review-driver-timing.py` reproduces the
r16 tables above from a saved CSV.

| Command/check | Result | Evidence |
|---|---|---|
| `drivers/audiorouter-virtual/tests/build-tests.ps1` | 27,537 checks pass (new `readSelectionChecks`: r16 packet-55 case, MoreData follow-up, partial reuse, long pause, 32-bit wrap incl. wire 0xffffffff, one-slot buffer, invalid geometry, frame-granular brute-force sweep) | console output |
| `model-cable-capture-timing.py` | Passes, including `select()` cases and a microsecond sweep: every returned packet intact; only overwritten packets skipped | console output |
| `build.ps1 -Version 0.1.2` (normal, unsigned) | Pass, 0 warnings | `target/r17-build-normal.log`, `target/r17-driver-normal` |
| `build.ps1 -Version 0.1.2 -TimingDiagnostics` (unsigned) | Pass | `target/r17-build-diagnostic.log`, `target/r17-driver-diagnostic` |

Host only: no signing, installation or audio.

### r17 independent WP-04 review (2026-10-10)

User-requested ("You do it"). A fresh-context Claude review agent with
read-only scope reran the driver tests (27,537 pass) and the model. Verdict:
**no blocking defect**. It confirmed:
- `AllocateBufferWithNotification` (`minwavertstream.cpp:458-460`) rounds
  the buffer to `nBlockAlign × N` with N ∈ {1, 2}, so writer and OS slot
  mapping agree.
- `WriteBytes` runs before the linear position advances, and
  `AudioRouterSurvivingDmaWindow` writes only the last lap, so the
  intactness test is exact.
- `ULONG_MAX` and the counter reset together only at construction and STOP;
  PAUSE/RUN keep both. Wrap is correct.
- No new reachable timestamp failure return.
- `MoreData` matches the `IMiniportWaveRTInputStream::GetReadPacket`
  contract; later notifications become the existing
  `STATUS_DEVICE_NOT_READY` path.
- No new race, allocation or blocking.
- The diagnostic lambda records the returned packet.

Non-blocking findings and resolution:
1. Residual risk understated. Wording corrected above, and a sweep now
   asserts every `MoreData` return has exactly 0 bytes of margin. A
   zero-margin flag in the timing record was not added: the record format is
   reviewed r16 ABI, and `review-driver-timing.py` can derive the margin
   from the existing linear position.
2. Test gaps. Added an N = 1 sweep, the 0-byte assertion and a
   last-read-ahead case. No host test covers the inline `GetReadPacket`
   timestamp arithmetic; the reviewer found it correct by inspection
   (deferred).
3. A last-read number ahead of the counter (unreachable) replayed packet 0.
   It now resynchronizes to the newest packet; tested.
4. The timestamp ignores a sub-frame carry (informational, unchanged).

After the fixes: `build-tests.ps1` reports 88,167 checks passed, and the
normal 0.1.2 build passes (`target/r17-build-normal2.log`).

### r17 clean candidate and immutable bundle — ready (2026-10-11)

Source commit `70e0d82972c1aa0685ca60c88231b77ed7567023`. The user's
unrelated uncommitted release-0.0.15 evidence edit was stashed only while the
clean-tree packaging steps ran and then restored unchanged.

| Artifact / check | Result |
|---|---|
| `build.ps1 -Version 0.1.2 -TimingDiagnostics -TestSign` | `target/r17-driver-70e0d829`, package.json clean/test/timing, 0.1.2.0; the script's trailing exit code 1 is the expected host trust-chain verification (log complete) |
| Signer | `FF6876FBE50A74DC0B69077C11DC28A1A9FAAD40`; CER SHA256 `92A1CBDD…E4D` byte-identical to the guest-trusted retained bundle; no host trust added |
| `tests/acceptance/m03-driver-package.ps1` | 33 checks pass, `target/driver-package-test-8e9fc5ed06424ed09f89f6737b7be2e4` |
| SYS SHA256 | `C78DD35F33AACB48C86C448297DE3823336823F12578582FDDC0EC3C1EC9AC56` (bundle copy identical) |
| `prepare-cable-latency-update.ps1` (base `repair-20261010-slot-provenance`) | `C:\VMs\ar-share\diagnostics-20261011-cable-capture-read-r17`; timing tool selftest passes; `target/r17-bundle.log` |
| Bundle MANIFEST | SHA256 `5A163711510831DEE0E6DE12059DA6944F462DD6E29DC5B7425A4A37B2D18620`; all 41 entries recomputed, 0 mismatches |

The bundle's `run-cable-driver-timing.ps1` compares r17 with recording
off and r17 with recording on. The r16 run above is the pre-repair reference.
Next: the user runs the guest block in the operations reply (copy, install,
traced comparison). Rollback: reinstall the r16 bundle
(`diagnostics-20261010-cable-driver-timing-r16`, 0.1.1) or the retained
0.1.0 package with `vm-checks.ps1 -Step install`.

First r17 guest attempt (2026-10-10): `vm-checks.ps1 -Step install` failed
2 of 3 checks: the helper returned exit 5, and fewer than four endpoints
were reported. The second install was an idempotent no-op. Evidence:
`C:\ar\evidence\20261010-234557-install`. Exit 5 is the helper's
`RestartRequired` (`crates/driver-helper/src/lib.rs:58`): the 0.1.2 update
is staged, but 0.1.1 was loaded, so endpoints return only after a guest
restart. No comparison ran. Next: restart the guest only, then
`-Step status` and the traced comparison without reinstalling. Confirm
`driver_version: [0, 1, 2, 0]` in the tone output before using results.

Second r17 guest attempt (2026-10-10): **invalid for r17, because the old
driver was still running.**
- `-Step status` passed (`driver=0.1.2.0`, four endpoints). Its evidence
  folder is `20261010-234646-status`, only 49 s after the install, and the
  status step reports the installed package version, not the loaded image.
- The traced comparison passed: `driver-timing-traced-cd15ffbe4d394a16a3849d1b8e125f1f.zip`,
  inner `driver-timing-47c6798658394129ab10861f981d46ab.zip`; WPR lost
  0 events; timing records 41,181, dropped 0, exhausted 0.
- Both tone outputs report bridge `driver_version: [0, 1, 1, 0]`. The build
  stamps this from `-Version` (`build.ps1:84-87`), so the loaded image was
  r16 0.1.1.
- The kernel records confirm it: 165 skipped capture packets, 127 of them
  intact, i.e. the r16 newest-packet selection.

As an extra r16 repeat it reproduces the mechanism: the probe flagged 166
capture packets (21,120 frames) against 165 driver skips. Recording off/on:
lost 41/51, p95 18.109/18.496 ms, jitter 7.744/9.048 ms, corrupted 0/0.
Notification lateness p50 1.226 ms (max 3.287 ms); fetch after
notification p99 0.624 ms. These are not r17 results.

Harness gap: neither `-Step status` nor the timing wrapper checks that the
bridge-reported driver version matches the bundle's package. Until a wrapper
guard is added, the guest block must require a boot time later than the
install, and the review must check the tone output's `driver_version`.

## r17 guest result (2026-10-10) — valid; repair works in the kernel but fails the gate

The guest restart was verified: boot 23:51:18, after the 23:45:57 install;
status `20261010-235224-status`. Archive
`driver-timing-traced-ac0b0fb85f4d4273940f2ed3d78d6d7e.zip`, inner
`driver-timing-3ce8f411395d44958e4ecbb42af912b9.zip` (`Passed=true`). Both
tone outputs report bridge `driver_version: [0, 1, 2, 0]`. WPR lost 0
events. Timing records 39,751, dropped 0, exhausted 0.

| | r17 recording off | r17 recording on | r16 on (reference) |
|---|---|---|---|
| Lost impulses / corrupted | 36 / 0 | 54 / 0 | 41 / 0 |
| p50 / p95 / jitter | 17.293 / 18.827 / 20.647 ms | 16.386 / 18.302 / 5.804 ms | 18.277 / 19.727 / 10.284 ms |
| Probe flagged capture packets | 76 (25,472 dropped frames) | 118 (15,232) | 119 |
| Render discontinuities | 62 (371.7 ms) | 99 (110.7 ms) | 73 (62.1 ms) |
| Max control loop gap | 349.3 ms | 87.8 ms | 24.4 ms |

Kernel review (`review-driver-timing.py`, recording on, capture stream 188):
- **0 intact-slot skips** (r16: 115) and no returned packet with reuse
  begun, so the selection rule behaves as designed.
- Total skips are still **119**, now all partly reused: 117 by less than
  one packet, maximum 1,280 B.
- 150 returns set `MoreData`. The next fetch followed within 0.2 ms only
  14 times. 136 times it came only after the next driver notification.

So the Windows audio engine reads one packet per notification wake-up here
and mostly does not act on `MoreData`. Once a fetch rescues an older packet,
the engine remains one packet behind. With two slots, that packet's slot is
reached by the writer at the very boundary that triggers the next
notification, so its next fetch finds it partly overwritten. The loss moves
by one packet instead of disappearing.

Gate: criterion 1 (0 intact skips) **met**; criterion 2 (≥ 80 % fewer
flagged capture packets) **not met** (118 vs 119); criterion 3 (0
corrupted) met. Large run-to-run variation, for example 349 ms and 88 ms
control-loop gaps in this pair, also means single runs cannot resolve small
effects. r17 is not harmful in this data, but it is not a repair on its
own. No hour run.

Next candidate (r18, not implemented): when `GetReadPacket` returns with
`MoreData`, also signal the stream's registered notification event(s), so
the engine wakes again at once and catches up. The current behaviour is
14/150 immediate follow-ups. An extra signal at most produces one
`STATUS_DEVICE_NOT_READY` poll. It needs the notification-list
synchronisation used by the timer DPC, a model of OS wake/fetch behaviour
first, and review. Alternative or complementary: reduce notification
lateness (p50 1.287 ms) with frame-granular positions and boundary-aligned
timer due times.

## Repair r18 — re-signal the notification event on MoreData (2026-10-11)

User approved r18 after the r17 result. Requirement IDs: VCAB-24/25/28,
VDEV-07.

**Scope.** When `GetReadPacket` returns with `MoreData`, after updating
`m_ulLastOsReadPacket`, it takes `m_PositionSpinLock`. If the stream is in
RUN, it calls `KeSetEvent` on every registered notification event, the same
walk `TimerNotifyRT` does. Diagnostic builds add one `AR_TIMING_RESIGNAL`
(7) record per call. `m_llLastNotifiedPacketCounter`, the timer, periods,
buffers, the bridge and the render path are unchanged.

**Model** (`simulate()` in `model-cable-capture-timing.py`). The engine does
one fetch per wake, follows `MoreData` 14/150 of the time and fetches
0.15 ms after a notification; the timer is jittered. Skipped packets per
20 s, seeds 1/17/99:

| Policy | Seed 1 | Seed 17 | Seed 99 |
|---|---|---|---|
| r16 newest | 465 | 417 | 402 |
| r17 oldest intact | 355 | 308 | 305 |
| r18 with prompt re-signal | 175 | 139 | 138 |
| r18 with 1–2 ms re-signal response | 175 | 139 | 138 |

A response of up to about 2 ms is equivalent, because the unread packet
stays intact for one packet period. Every rescued packet is returned at
exactly 0 margin (264–290 per run), so the r17 residual risk applies to
more packets. The model's timer is harsher than the guest's (in r16, 115 of
119 real skips were intact), so it does not predict the guest percentage.

**Independent WP-04 review** (fresh-context Claude agent, read-only, same
process as r17). Verdict: **no blocking defect**. It confirmed:
- The re-signal is bounded: it needs a newer completed packet, and each call
  advances the last-read number.
- GetReadPacket is capture-only, and end-of-stream handling is render-only.
- PAUSE is excluded by the RUN check.
- Reusing `oldIrql` is correct.
- The exporter passes event 7 through unchanged.
- An extra wake costs at most one `STATUS_DEVICE_NOT_READY` poll.

Findings:
1. **Pre-existing, not fixed.** Register/Unregister change
   `m_NotificationList` without the position lock, and the list is now
   walked from two places. A client that unregisters while a fetch is in
   flight could make the walk read freed memory, which bugchecks. The audio
   engine stops before unregistering, and PortCls cannot close the pin while
   a property call is in flight. Proper fix (separate change): insert and
   remove under the lock in non-paged code, and free after release.
2. Fixed: one diagnostic record per call; README updated.
3. Fixed: the model now covers a late re-signal response (1–2 ms) and the
   engine's occasional own `MoreData` follow-up (14/150); assertions added.
4. Fixed: removed an assertion that was always true; added zero-margin
   assertions.
5. Not added: a host test for the inline re-signal condition, which is just
   `MoreData` plus RUN. The guest event-7 count checks it.
6. Fixed: the review tool's docstring now states which driver versions it
   expects.

**Host verification after the fixes.** `build-tests.ps1`: 88,167 checks
pass. `tests/acceptance/m03-cable-driver-timing.ps1`: 58 checks pass. Model
passes. Normal and diagnostic `build.ps1 -Version 0.1.3`: 0 warnings
(`target/r18-build-normal2.log`, `target/r18-build-diagnostic2.log`). No
signing, installation or audio on the host.

**Guest pass criteria, unchanged from r17:**
- 0 intact-slot skips;
- probe flagged capture packets at least 80 % below 119;
- 0 corrupted impulses.

Also check that event 7 records appear, that `MoreData` follow-ups mostly
arrive within 0.2 ms, and that the bridge reports
`driver_version: [0, 1, 3, 0]`. The guest must restart after the install;
the block checks boot time. Rollback: the r17 bundle (0.1.2) or the r16
bundle (0.1.1) via `vm-checks.ps1 -Step install`, then restart the guest.

**r18 candidate and bundle — ready.**
- Built from source commit `240b9a14486d2a3a6482dff699fbf04674398e27`.
  The user's release-0.0.15 evidence edit was stashed only during packaging
  and restored unchanged.
- `target/r18-driver-240b9a14`: version 0.1.3/0.1.3.0, built from a clean
  tree, test-signed, timing diagnostics included.
- Signer `FF6876FB…AD40`. The CER SHA256 `92A1CBDD…E4D` is unchanged and
  already trusted by the guest; no trust was added on the host.
- SYS SHA256 `4073348B13EE4EC563947928DCACDF74E470ADCFB0855E91C920F5884717C2C2`;
  the bundle copy is identical.
- Package checks: 33 pass.
- Bundle `C:\VMs\ar-share\diagnostics-20261011-cable-resignal-r18`, from
  base `repair-20261010-slot-provenance`. MANIFEST SHA256
  `D475611AEB99EEF97B78FC1490ECC021B4363F9F702B6FBD9A1E70AE4BC92C9E`;
  41/41 entries recomputed, 0 mismatches.

### r18 first guest run (2026-10-11) — invalid, whole-guest freeze attributed

- r18 installed as `20261011-000445-install`; the guest booted at 00:05:07,
  after it. Status `20261011-000603-status` reports `driver=0.1.3.0`, and
  the bridge reports `driver_version: [0, 1, 3, 0]`.
- Archive `driver-timing-traced-077f0fe6ab574ec89ffb5fcafa29e3dc.zip`,
  inner `driver-timing-6c2eda1a710d4d43a96f8b2e0eaec26b.zip`.
- The recording-off half failed: route exit 1 with `0x80070016` after
  every route thread stalled about 3.46 s; leases were deactivated at
  11,487 ms. The recording-on half did not run. Same failure pattern as the
  first r16 attempt.
- WPR: 324,897 events, 0 lost. `--silences`: a **3,451.1 ms interval with
  no event of any kind on any of the four CPUs** (QPC 675719125–710230565);
  each CPU's longest gap is 3.45–3.46 s.
- `--activity`: `svchost.exe` PID 1588 rose to 53 % at 10 s and filled one
  CPU through the freeze. It was also busy right after the install, at the
  start of the trace. Process rundown: `svchost.exe -k
  LocalSystemNetworkRestricted -p -s SysMain`, Windows' memory-prefetch
  service, formerly Superfetch.

A total absence of DPCs, interrupts and switches means the guest was not
executing: it was suspended beneath the guest OS. Neither the driver nor
timing recording can do that, and recording was off here. Both multi-second
freezes so far came shortly after an install or boot, and the valid r16 run
long after. Working hypothesis: SysMain's memory-management bursts are
pathologically slow under VirtualBox NEM, which stalls the whole VM. One
trace supports this; it is not yet proven. `quiet-guest.ps1` does not touch
SysMain.

Next: stop and disable SysMain in the guest only (reversible; the previous
start type is recorded in `C:\ar\sysmain-state.txt`), then rerun the r18
comparison without reinstalling. No host change.

### r18 second guest run (2026-10-11) — valid; works as designed, gate partly met

SysMain was stopped and disabled in the guest (saved start mode `Auto`, in
`C:\ar\sysmain-state.txt`), with a 30 s settle. Archive
`driver-timing-traced-dbf6da42d2f345988609169b2642f8bd.zip`, inner
`driver-timing-eb3e0f202db2462c80124454ce6be4d5.zip` (`Passed=true`). Both
halves report bridge `driver_version: [0, 1, 3, 0]`. WPR: 644,182 events,
0 lost; longest all-CPU silence 30.1 ms (4 ≥ 20 ms), so no multi-second
freeze. Timing records 40,042, dropped 0, exhausted 0.

| | r18 recording off | r18 recording on | r17 on | r16 on |
|---|---|---|---|---|
| Lost / corrupted impulses | 29 / 0 | 28 / 0 | 54 / 0 | 41 / 0 |
| p50 / p95 / jitter | 18.308 / 19.688 / 7.463 ms | 17.654 / 19.035 / 5.726 ms | 16.386 / 18.302 / 5.804 ms | 18.277 / 19.727 / 10.284 ms |
| Probe flagged capture packets | 61 | **51** | 118 | 119 |
| Render discontinuities | 118 (224.3 ms) | 85 (105.9 ms) | 99 | 73 |
| Max control loop gap | 10.6 ms | 21.7 ms | 87.8 ms | 24.4 ms |

Kernel review (capture stream 190, recording on):
- 0 intact skips. Total skips **50** (r17 119, r16 119), all partly reused
  by less than one packet (max 768 B).
- `MoreData` returns 143, re-signals (event 7) 143, next fetch within
  0.2 ms **141** (r17 14/150), after a boundary notification 0. The
  re-signal works.
- Notification lateness p50 1.218 / p99 2.689 / max 3.302 ms. True boundary
  → fetch max 3.248 ms.

**Gate:**
- criterion 1 (0 intact skips): met;
- criterion 3 (0 corrupted): met, despite 143 zero-margin rescues;
- criterion 2 (≥ 80 % fewer flagged capture packets): **not met**, 57 %
  (119 → 51); the target is ≤ 23.

Lost impulses also fell (28/29 against 36–70 in earlier runs). Caveat:
SysMain changed in this run, and run-to-run variance is large, so one pair
cannot separate r18's effect from the quieter guest. The 50 remaining
skips are fetches that came after the writer had reached the slot, i.e.
late *first* notifications. r17/r18 cannot recover those; only earlier
notification can.

Next candidate (r19, not implemented): cut notification lateness. Today the
position advances in whole-millisecond steps (`floor(ms) × 384 B`) and the
1 ms timer fires every 1.8 ms at median, so a completed packet is reported
1.2 ms late at median and up to 3.3 ms late.
- Frame-granular position: removes up to 1 ms of the floor.
- Timer due times aligned to packet boundaries (one-shot at the next
  boundary rather than a free-running 1 ms period): reduces the cadence
  component.
Both touch the timer and the position math shared with render, so the
model, render-path regression and review need more care than r17/r18.

## Next action and rollback

User runs the [one short guest block](../../../operations/virtual-cable-latency.md#driver-notificationfetch-comparison-after-r15-r16)
inside Administrator PowerShell in AR-DriverTest, Media Player closed and
Cable B Listen off. No guest execution occurred during host preparation.
Review raw records/drop counters and both measurements before choosing a
repair or requesting duration. Rollback inside the guest uses retained
492d8ca8/0.1.0 package from `C:\ar\repair-20261010-slot-provenance` with its
existing install step. No host trust/security/audio/WSL change occurred.
