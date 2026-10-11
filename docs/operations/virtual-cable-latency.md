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
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-latency-r7\quiet-guest.ps1' -Apply
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
    robocopy.exe 'Z:\diagnostics-20261010-cable-latency-r7' 'C:\ar\diagnostics-20261010-cable-latency-r7' /E /R:1 /W:1 /XF latency-*.zip | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-cable-latency-r7\run-cable-latency.ps1' -Route engine
    if ($LASTEXITCODE -ne 0) { throw 'Latency diagnostic failed. Evidence was preserved; send the output.' }
}
```

It takes about a minute and a half. It prints one line per latency configuration (p50,
p95, jitter, lost, corrupted), the relay statistics and the driver counters,
one bit-exact line (compared frames, mismatched samples, non-zero silence),
two isolation lines (frames, non-zero samples), then
copies `latency-<run>.zip` to the same folder on `Z:` (host:
`C:\VMs\ar-share\diagnostics-20261010-cable-latency-r7`). Send the output. A
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
