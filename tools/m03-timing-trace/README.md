# Bounded driver timing diagnostic

Host: static build and `m03-timing-trace.exe selftest` only. Capture refuses
any computer except AR-DriverTest. Never install or open this driver on the host.

Build the driver with `build.ps1 -TimingDiagnostics`; default builds exclude
all trace storage, fields, calls and the IOCTL. The diagnostic build contains
8 MiB of fixed nonpaged records. This is observation storage, not an audio
buffer. It does not change periods, timers, packet selection or counters.

Private diagnostic IOCTL `0x0022E010` uses METHOD_BUFFERED, 16-byte requests
and exactly 8,232-byte zeroed responses (128 records at most). START/STOP/READ
require enabled SeLoadDriverPrivilege plus file-object ownership. The helper
enables the privilege in its own elevated guest process. No audio samples,
kernel addresses or user pointers are exported. Cleanup/unload stops, drains
and scrubs records; only the owning control handle may read them.

Writers atomically enter an open gate with at most 16 CAS attempts, reserve
one non-overwriting slot with at most 16 CAS attempts, copy scalars and leave.
STOP closes admission and waits at PASSIVE_LEVEL for admitted writers only.
Epoch changes and pre-START QPC rejection prevent old callback entries from
contaminating a new session. Capacity and epoch growth saturate; exhaustion
does not change the audio path. The response's `Dropped` combines session
reservation failures with **lifetime-cumulative admission failures**, which
are never reset or frozen by STOP. Any drop/exhaustion invalidates capture.
Export follows the audio child and owned process-tree shutdown.

Every QPC field is raw ticks; use the response's QpcFrequency, never assume
10 MHz. CSV sequence is storage reservation order, not timestamp order;
sort by QPC and stream ID for timing analysis. Events:

| Event | Meaning | Extra |
|---|---|---|
| 1 | Timer entry QPC before position lock; pre-update snapshot under lock | Last notified packet count |
| 2 | Timer UpdatePosition completed, snapshot under lock | QPC used to advance DMA |
| 3 | One notification event has been signalled | Last notified count |
| 4 | GetReadPacket entry before lock; pre-update snapshot under lock | Lock-acquired QPC |
| 5 | GetReadPacket return, post-update snapshot | Entry QPC, for call pairing |
| 6 | Stream enters RUN | Rate in high 32 bits; packet bytes in low 32 bits |

Event 6 LinearBytes is block alignment and PacketNumber is notifications per
buffer. Other events' LinearBytes is the DMA byte position. Event 4 is emitted
only after valid parameter/state checks; early failures have return events
only. Timer advances are instrumented, but other position-query progress
is not individually traced. GetReadPacket's before/after snapshots still show
its own progress. Notification QPC is immediately after KeSetEvent, not the
consumer's wake or sample-presentation time. Repeated notifications can
coalesce. An older DMA slot may already be reused; these records never
authorize replay. Instrumentation overhead and NEM remain measurement limits.

Microsoft documents the privilege check at PASSIVE_LEVEL and the extended
timer callback at DISPATCH_LEVEL:
[SeSinglePrivilegeCheck](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddk/nf-ntddk-sesingleprivilegecheck),
[EXT_CALLBACK](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nc-wdm-ext_callback).
