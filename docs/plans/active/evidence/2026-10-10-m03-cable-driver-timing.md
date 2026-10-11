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

## Next action and rollback

User runs the [one short guest block](../../../operations/virtual-cable-latency.md#driver-notificationfetch-comparison-after-r15-r16)
inside Administrator PowerShell in AR-DriverTest, Media Player closed and
Cable B Listen off. No guest execution occurred during host preparation.
Review raw records/drop counters and both measurements before choosing a
repair or requesting duration. Rollback inside the guest uses retained
492d8ca8/0.1.0 package from `C:\ar\repair-20261010-slot-provenance` with its
existing install step. No host trust/security/audio/WSL change occurred.
