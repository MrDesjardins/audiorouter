# Cable latency, bit-exactness and isolation diagnostic (VCAB-25/20/26 preparation)

Measures how long an impulse takes from **AudioRouter Cable A Input** to
**AudioRouter Cable B Output** inside the AR-DriverTest VM, through the
driver's bridge and a route between the cables. It needs the test-signed
driver already installed in the VM (see the [direct audio runbook](virtual-cable-direct-audio.md))
and asks for no manual playback.

`-Route` picks what sits between the cables:

- `proxy` (default): a diagnostic pass-through relay that copies blocks.
- `engine`: the AudioRouter engine. The tool compiles a cable-only session
  (Cable A render source → Cable B capture sink, no tools) with the engine
  compiler and runs it on the engine's realtime scheduler, re-blocking the
  480- or 128-frame bridge blocks into 128-frame engine quanta
  (`CableRouteProcessor`). This is the product's audio processing, but not
  yet its backend lifecycle (starting and stopping a session, lease
  supervision, the app). It supports 1–2 channels, the engine's width.

**What it is not:** it is not the full product's latency, not the VB-Cable
comparison VCAB-25 also requires, and not qualification. Inside a VirtualBox VM
that runs through the Windows Hypervisor Platform, late timer delivery adds
delay and jitter that bare metal does not have
([trace analysis](../plans/active/evidence/2026-10-10-m03-render-commit-validity.md#traced-300-second-run-2026-10-10-failed-cause-attributed)),
so treat the numbers as an upper bound until measured under native VT-x
([session runbook](virtual-cable-native-vtx-session.md)) or on bare metal.

## How it works

1. `m03_bridge_tone --passthrough` (or `--engine`) opens the Cable A
   render-source and Cable B capture-sink leases and republishes every
   Cable A block into Cable B through three preallocated relay buffers
   (counted: forwarded, silence before audio arrives, dropped when the relay
   backs up). With `--engine`, each block first passes through the engine;
   the tool reports the quanta it processed and any it replaced with
   silence because the engine produced no output (expected: zero).
2. The native probe (`m00-probe.exe cable-impulse`) plays 1,000 impulses at a
   10 ms cadence into Cable A Input and records Cable B Output. Impulse *k*
   has the exact float amplitude (1 + k mod 16) / 32, so each arrival is
   matched to the impulse that produced it even when some are lost (the older
   `impulse-loopback` mode pairs by arrival order and suits only a lossless
   physical loopback). Render time comes from the render stream's
   `IAudioClock` anchored at the end of the run; capture time from each
   packet's QPC timestamp.
3. Two configurations run back to back: a 480-frame relay with default engine
   periods (VCAB-25 target p95 ≤ 40 ms) and a 128-frame relay with
   low-latency periods (target p95 ≤ 20 ms). Jitter target: p99 − p1 ≤ 2 ms.
   Any lost or corrupted impulse fails the targets.
4. A third pass-through run (480 frames) checks bit-exactness:
   `m00-probe.exe cable-bitexact` plays 10 s of seeded float noise (values on
   the 2^-24 grid, |x| ≤ 0.5 so the render limiter never acts, distinct per
   channel) then 0.5 s of silence; the capture is aligned on an exact
   32-frame match and every sample is compared, and the silence must be
   exact +0.0. This is VCAB-20 at the endpoints' float32 mix format through
   the chosen route, not at other rates or channel counts. The engine maps
   −0.0 to +0.0 by design (its channel matrix sums from +0.0); the probe's
   noise contains no −0.0, and `crates/engine/tests/cable_route.rs` pins
   that single deviation.
5. Isolation: the tone tool runs in its normal mode (a tone on Cable B
   Output, Cable A Input consumed). Nothing routes into Cable A Output, so
   `m00-probe.exe cable-isolation` records it for 5 s while noise plays into
   Cable A Input, then into Cable B Input; every sample must be exact +0.0
   (VCAB-26 asks for ≤ −140 dBFS; the peak is reported in dBFS).

## Optional: quiet the guest first

Traces show VM audio losses only during guest background bursts (Windows
Update, Defender, background tasks). The bundle includes a reversible,
guest-only script. After copying the bundle (first line of the block below),
in Administrator PowerShell inside the VM:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-latency-r8\quiet-guest.ps1' -Apply
```

It records the current values, pauses Windows Update for 7 days, disables
Automatic Maintenance, adds Defender exclusions for `C:\ar` and the test
tools, updates Defender signatures now, then waits (up to 15 minutes) for 60
quiet seconds. `-Status` shows the settings; `-Revert` restores exactly what
it recorded. It refuses to run outside AR-DriverTest.

## Run it (inside the VM)

Close Media Player and turn Cable B Listen off. In **Administrator PowerShell
inside AR-DriverTest**, with the driver installed:

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261010-cable-latency-r8' 'C:\ar\diagnostics-20261010-cable-latency-r8' /E /R:1 /W:1 /XF latency-*.zip | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-latency-r8\run-cable-latency.ps1' -Route engine
    if ($LASTEXITCODE -ne 0) { throw 'Latency diagnostic failed. Evidence was preserved; send the output.' }
}
```

It takes about a minute and a half. It prints one line per latency configuration (p50,
p95, jitter, lost, corrupted), the relay statistics and the driver counters,
one bit-exact line (compared frames, mismatched samples, non-zero silence),
two isolation lines (frames, non-zero samples), then
copies `latency-<run>.zip` to the same folder on `Z:` (host:
`C:\VMs\ar-share\diagnostics-20261010-cable-latency-r8`). Send the output. A
missed target is a measurement, not a crash; do not rerun it unchanged.

## Host-side checks

`tests/acceptance/m03-cable-latency.ps1` runs the runner's real orchestration
with fake process boundaries (pass, slow, lost impulses, status failure,
ambiguous endpoints, early pass-through exit, probe failure, hung
pass-through), the real static probe's `cable-impulse-selftest` and the
host-refusal guard, and the engine route's arguments and recorded scope.
`cargo test -p audiorouter-windows-audio --example m03_bridge_tone` covers
the relay (order, counted silence, bounded drop of the oldest block, an
end-to-end mapped-slot pass-through) and an engine-route ramp that must
arrive continuous and unchanged. `cargo test -p audiorouter-windows-audio
--lib cable_route` covers re-blocking at 16–4,096 frames and fail-closed
silence; `cargo test -p audiorouter-engine --test cable_route` covers the
compiled session itself.

## Repeated starts and the VB-Cable comparison (VCAB-25 step 2)

One run gives one end-to-end level, and in this VM that level depends on how
full the endpoint buffers were when the streams started, or after a VM
stall (2026-10-10 runs r5–r8: 25–57 ms on the same path). VCAB-25 also asks
for "never worse than VB-Cable on the same PC by more than 5 ms".
`run-cable-latency-starts.ps1` therefore opens fresh streams for every
measurement and runs each period mode `-Starts` times (default 5, 400
impulses each):

- `-Route engine`: Cable A Input → bridge → AudioRouter engine → Cable B
  Output, a new engine route process per measurement.
- `-Route vbcable`: VB-Cable's own CABLE Input → CABLE Output. This has no
  route in between, so the comparison favours VB-Cable.

The probe reports latency levels in impulse order. A level lasts while each
impulse stays within 1 ms of the level's first impulse, and the first level
lasting 50 impulses is the start's steady level. A VM stall shows as a
second segment instead of jitter. The runner prints the steady level of
every start, the median per mode and how many starts shifted mid-run.

Each impulse is dated from render-clock readings (stream position and QPC)
taken after every render buffer, not from one end-of-run reference: with a
single reference, a mid-run render stall misdated every earlier impulse
(r9: an impossible −3.9 ms VB-Cable level). Each reading implies the time
stream frame 0 would have played; an impulse uses the median of those
offsets within ±0.1 s of it, because VB-Cable's position wobbles by ±3 ms
and snaps back (r10). The probe reports only lasting steps over 2 ms as
render stalls or skips (`cable_render_discontinuities`) and also prints the
old single-reference median (`cable_end_anchor_p50_ms`). Every measurement
saves `<name>-raw.csv` (clock readings and arrivals) so the analysis can be
rerun on the host with `m00-probe.exe cable-impulse-reanalyze <file>`. A start
is **clean** when nothing was lost or dropped and the render stream never
stalled; the comparison uses the median of the clean starts' p50 values,
because VB-Cable's latency alternates by about 1 ms within a level.

### Install VB-Cable inside the VM only

Never on the host for this test. Optional first: in VirtualBox, take a
snapshot of AR-DriverTest (for example `03-before-vbcable`) so VB-Cable can
be removed by restoring it.

1. **Host PC:** download the VB-CABLE driver pack (zip) from
   <https://vb-audio.com/Cable/> and save it in `C:\VMs\ar-share\`.
2. **Inside the VM:** copy the zip from `Z:\` to the desktop, extract it,
   right-click `VBCABLE_Setup_x64.exe` → **Run as administrator** →
   **Install Driver**, then restart the VM.
3. **Inside the VM:** in Sound settings, check that **CABLE Input** and
   **CABLE Output** appear. Leave VB-Cable's settings at their defaults.

### Run it (inside the VM)

Administrator PowerShell, AudioRouter driver installed, Media Player
closed, Cable B Listen off. AudioRouter engine route first (no VB-Cable
needed):

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261010-cable-latency-r11' 'C:\ar\diagnostics-20261010-cable-latency-r11' /E /R:1 /W:1 /XF latency-*.zip starts-*.zip | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-latency-r11\run-cable-latency-starts.ps1' -Route engine
    if ($LASTEXITCODE -ne 0) { throw 'Repeated-start run failed. Evidence was preserved; send the output.' }
}
```

After VB-Cable is installed and the VM restarted:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-latency-r11\run-cable-latency-starts.ps1' -Route vbcable
```

The engine run takes about 3 minutes and the VB-Cable run about 1.5. Each
copies `starts-<route>-<run>.zip` to the bundle folder on `Z:`. Host-side
checks: `tests/acceptance/m03-cable-latency-starts.ps1` (fake process
boundaries, the probe's steady-level self-test, host refusal).

## Short route/submission trace after r12

The r12 loss classifier used nominal impulse times, which drifted about
4.8 seconds from the saved render QPC clock over the hour. The updated runner
uses explicit QPC event/snapshot times. This fixes attribution; losses and
counters remain visible. A short trace also measures the relationship between
submitted payload frames and Windows device positions, and records impulses
at route input/output with relay queue depth, silence and drop counts.
Route trace times are observation times, not DMA presentation timestamps.
The new raw `submit` rows retain clock readings before/after the padding query
so its uncertainty can be checked. Original latency outputs remain unchanged
until the extra delay is located; neither a QPC correlation nor this trace
closes the native qualification gates.

Prepared destination: `C:\VMs\ar-share\diagnostics-20261010-cable-stage-trace-r13`.
Use Administrator PowerShell **inside AR-DriverTest**, Media Player closed,
Cable B Listen off. This measures three fresh starts per period mode, taking
about two minutes, and preserves a ZIP in the matching shared folder.

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261010-cable-stage-trace-r13' 'C:\ar\diagnostics-20261010-cable-stage-trace-r13' /E /R:1 /W:1 /XF latency-*.zip starts-*.zip continuity-*.zip | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-stage-trace-r13\run-cable-latency-starts.ps1' -Route engine -Starts 3 -Impulses 400 -TraceRoute
    if ($LASTEXITCODE -ne 0) { throw 'Stage trace failed. Evidence was preserved; send the output.' }
}
```

The opt-in route trace is limited to short stereo 48 kHz runs with 128/480
frame blocks and uses preallocated queues. Trace overflow, missing data, raw
file write failure, or missing QPC observations fail the diagnostic. CSV output
is flushed only after the workers and leases stop. No new driver installation
or host configuration change is needed for this bundle.

## Capture reader scheduling comparison after r13

r13 preserved every observed input impulse through sink publication, but
39 impulses disappeared downstream. The diagnostic's own render/capture
threads were not registered with MMCSS. The corrected probe registers both
workers under Pro Audio, initializes their COM apartments and waits for
readiness before starting either stream. Registration/API failure invalidates
the run; no fallback silently substitutes a different scheduling condition.

With `-TraceRoute`, raw CSV also contains
`capture,device_position,frames,flags,packet_qpc_100ns,read_qpc_100ns,wake_qpc_100ns`.
The packet timestamp dates its first sample; read/wake times are observations.
Keep timestamp-error flags visible and exclude such timestamps from lag
calculations. Compare packet position gaps, read/wake intervals, batches
drained per wake and `read − packet timestamp − packet duration` near missing
impulses. These narrow reader/engine/driver delivery timing; they do not
independently timestamp the driver's GetReadPacket call.

New destination: `C:\VMs\ar-share\diagnostics-20261010-cable-capture-reader-r14`.
Run only after the agent verifies its manifest. Use Administrator PowerShell
**inside AR-DriverTest**, with Media Player closed and Cable B Listen off.
About two minutes; no driver reinstall. Rollback: use the retained r13 bundle.

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261010-cable-capture-reader-r14' 'C:\ar\diagnostics-20261010-cable-capture-reader-r14' /E /R:1 /W:1 /XF latency-*.zip starts-*.zip continuity-*.zip | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-capture-reader-r14\run-cable-latency-starts.ps1' -Route engine -Starts 3 -Impulses 400 -TraceRoute
    if ($LASTEXITCODE -ne 0) { throw 'Capture trace failed. Evidence was preserved; send the output.' }
}
```

All earlier losses and qualification failures remain recorded. MMCSS reduces
a diagnostic scheduling risk; guest evidence must establish whether it
changes losses. Buffer sizes, audio periods, SYS and thresholds are unchanged.

## Focused capture scheduling trace after r14

The r14 reader registration succeeded but did not solve loss. The offline
128-frame model shows that a small delay before Windows fetches a completed
packet can skip it even with a prompt client reader. The next run records
the already guest-proven minimal scheduling/DPC/ISR profile alongside one
10-second impulse train through the low-latency engine route. No default-mode
repeat, driver reinstall, buffer increase or host setting change.

Use Administrator PowerShell **inside AR-DriverTest**, Media Player closed,
Cable B Listen off, and no other WPR recording running. Bundle:
`C:\VMs\ar-share\diagnostics-20261010-cable-scheduling-r15`.
Allow about two minutes including trace saving. Busy/unknown recorder status
is refused; the wrapper saves/stops only its unique owned recorder instance.
The final `cable-scheduling-*.zip` contains scheduling ETL, recorder logs,
redirected measurement output and that run's nested measurement ZIP.

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261010-cable-scheduling-r15' 'C:\ar\diagnostics-20261010-cable-scheduling-r15' /E /R:1 /W:1 /XF latency-*.zip starts-*.zip continuity-*.zip cable-scheduling-*.zip | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-scheduling-r15\run-cable-scheduling.ps1'
    if ($LASTEXITCODE -ne 0) { throw 'Scheduling trace failed. Evidence preserved; send the output.' }
}
```

Host review must verify trace coverage/lost events first, then correlate
normalized QPC capture gaps with worker IDs, audiodg thread readiness and
dispatch, and DPC activity. Scheduler evidence distinguishes waiting before
readiness from waiting after readiness; it does not directly record every
GetReadPacket invocation or prove the driver notified on time. Trace overhead
and VirtualBox NEM remain limitations. Rollback: stop only the owned recording
if save failed, and use r14. Retain all prior failed qualification evidence.

## Driver notification/fetch comparison after r15 — r16

r15 proved that capture waits occur before thread readiness, but did not
directly timestamp the driver notification or Windows packet fetch. r16
adds opt-in fixed-size diagnostic records; it contains no audio repair.
The wrapper runs one 1,000-impulse disabled baseline, then one identical
enabled run on the same diagnostic build. It writes the kernel CSV after
the child stops and rejects drops/exhaustion. No WPR runs in this comparison;
compare the two r16 observations first, with r15 as prior context.

Use Administrator PowerShell **inside AR-DriverTest**, with Media Player
closed and Cable B Listen off. This block upgrades the guest driver to the
test-signed diagnostic version 0.1.1, then records about one minute of short
comparisons. It must never run on the host. If install requests a restart
from the helper, restart the guest normally and run only the last measurement
command. Other install errors stop the block; send its output.

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261010-cable-driver-timing-r16' 'C:\ar\diagnostics-20261010-cable-driver-timing-r16' /E /R:1 /W:1 /XF driver-timing-*.zip starts-engine-*.zip | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-driver-timing-r16\vm-checks.ps1' -Step install
    if ($LASTEXITCODE -ne 0) { throw 'Install did not complete; stop and send its output (restart guest if requested).' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-driver-timing-r16\run-cable-driver-timing.ps1'
    if ($LASTEXITCODE -ne 0) { throw 'Comparison failed; evidence was preserved. Send the output.' }
}
```

Send the output; the current archive is copied to the matching Z: directory.
It contains both measurement ZIPs, raw kernel timestamps and failure evidence.
Kernel CSV has raw QPC ticks; the printed frequency is authoritative. It
contains metadata only. See the [event contract](../../tools/m03-timing-trace/README.md).
The helper mirrors the VM marker guard and owns one process job and trace
handle; no global recorder cancellation, host settings or driver operation.

Rollback inside the guest: use the retained
`C:\ar\repair-20261010-slot-provenance\vm-checks.ps1 -Step install` to
restore 0.1.0/492d8ca8; follow any guest restart request. All long/native,
Verifier, signing and release gates remain open. Do not repeat the hour run
until a meaningful repair is supported and passes a short comparison.

## Eight-cable sustained continuity (VCAB-24 step 3 deviation run)

This one-hour guest run checks four concurrent AudioRouter engine routes
(Cable A→B, C→D, E→F and G→H) while sending a separately identity-coded
impulse train through each pair. It attributes losses or driver counter
increases to a VM-wide scheduling stall only when all four route processes
report a nearby gap. It is a **VirtualBox NEM deviation run**, not VCAB-24
qualification or a release gate: NEM scheduling can stall the whole guest.
No host driver is installed or loaded, and the run does not change host
settings.

### Run it (inside the VM)

Prerequisites: the test-signed AudioRouter driver is installed in AR-DriverTest;
the VM is booted normally; use **Administrator PowerShell inside the VM**.
Close Media Player and leave Cable B Listen off. Run this single block:

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261010-cable-continuity-r12' 'C:\ar\diagnostics-20261010-cable-continuity-r12' /E /R:1 /W:1 /XF continuity-*.zip | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-continuity-r12\run-cable-continuity.ps1' -Minutes 60
    if ($LASTEXITCODE -ne 0) { throw 'Continuity run failed. Evidence was preserved; send the output.' }
}
```

The run takes about 62 minutes. It enables eight cables temporarily when
needed, restores the original cable count in `finally`, and copies only this
run's `continuity-<id>.zip` to
`Z:\diagnostics-20261010-cable-continuity-r12`. The archive includes the
four probe reports, raw impulse timings, route progress/counters and verdict.
Send the printed output and the archive. A clean result is indicative only;
it does not establish native VT-x/bare-metal continuity.

Host PC checks: `tests/acceptance/m03-cable-continuity.ps1` covers progress
parsing, four-route stall attribution, cable-count restoration, watchdogs,
the VM guard and probe self-test without starting audio or opening the driver.
