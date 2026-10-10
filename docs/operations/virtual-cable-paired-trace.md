# Paired scheduling diagnosis: recorder probes

Purpose: distinguish late guest worker readiness from host VirtualBox thread
execution delay. This is preparation for VCAB-24/27 analysis, not continuity
qualification. It changes no driver/audio binaries, VM configuration, power,
security or WSL/Hyper-V settings. Recorder probes launch no audio test.

## Current next step: paired two-second probe

The host probe is reviewed: its archive hash matches, independent decoding
finds 717,343 events with zero lost, and the offline switch reader matches
232,406 context switches / 141,391 readiness events with zero rejected.
Start/save succeed, the intended profile matches, and WPR is idle afterward.
Follow the paired steps below with audio still off. Review both traces and
clock brackets before any audio phase.

## Completed two-second host probe

The agent cannot start WPR because its Windows token is not an administrator.
The script refuses before recording. Sandbox permission does not grant Windows
administrator rights. The user ran the following command in Administrator
PowerShell on the main PC; real host start/save is now verified. It is retained
as reproduction of the completed step and need not be repeated:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\VMs\ar-share\diagnostics-20261010-paired-probe\run-paired-scheduling-host.ps1' -Phase HostProbe
```

Result: **HostProbe passed. No audio test was run.** Measurement was two
seconds; save took 10.498 s. Archive identity and decoding are in the review
record. No host audio/driver operation occurred.

Host recording contains system process/image names and scheduling/DPC/ISR
metadata, not microphone/speaker audio. Evidence stays locally under the
bundle; do not publish raw ETL/CSV or commit them. A unique owned recorder
instance is stopped/saved in `finally`. Existing/unknown recorder state is
refused; no global cancellation occurs. Ctrl+C permits cleanup while this
PowerShell process remains alive; forcibly closing/killing it can prevent
cleanup. Preserve files if stop/cancel fails and send the exact output.

## Paired probe: follow these steps now

The host probe has passed review. These steps launch no tone/audio/driver
executable. Leave
the VM running with `Z:` connected; audio loop/listener remain off.

First, inside the VM, open **Administrator Windows PowerShell** and copy:

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261010-paired-probe' 'C:\ar\diagnostics-20261010-paired-probe' /E /R:1 /W:1 /XD paired-runs /XF paired-*.zip
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
}
```

The exclusions prevent host traces/earlier results from being copied into
the guest. There is no reinstall or snapshot restore.

On the **main PC**, in **Administrator Windows PowerShell**:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\VMs\ar-share\diagnostics-20261010-paired-probe\run-paired-scheduling-host.ps1' -Phase PairProbe
```

When it says **Ready for the guest probe**, within two minutes paste into the
**VM's Administrator Windows PowerShell**:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-paired-probe\run-paired-scheduling-guest.ps1'
```

Both scripts finish automatically. Send both outputs. Host recording begins
only after the guest joins; the guest then records two seconds while the host
records concurrently. The host waits at most 45 seconds for guest start/save;
a slow save fails safely and keeps both sets of evidence. No five-minute test
is available in these scripts. The first paired trace and measured clock
uncertainty must be reviewed before preparing an audio phase.

## Protocol and evidence

- Bundle hashes are verified on both sides. Host invocation in AR-DriverTest
  is refused; guest invocation requires that name and a copied `C:\ar\*`
  bundle. Both require administrator privileges and free space.
- One atomically published offer and unique run ID coordinate the pair.
  A second/stale offer is refused; no existing protocol messages are
  overwritten. The host removes only its own offer. Timeout/peer failure
  markers propagate; unique run folders and temporary ETLs remain preserved.
- Eight shared-file request/reply clock brackets before and eight after
  measurement record host UTC/QPC, guest UTC/QPC and QPC frequencies. The
  guest timestamp lies inside the host round trip. Offset bounds are
  `guest UTC - host end` through `guest UTC - host start`; no symmetric
  network/disk latency assumption. Polling and IO are outside audio leases.
  Record guest UTC around its QPC read and reject host wall-clock jumps.
- Clock brackets can be too wide or inconsistent for 28–35-ms attribution.
  Inspect the smallest round trips, interval overlap, before/after changes
  and ETL clock headers; do not infer sub-millisecond synchronization.
- Host starts with at least 4 GB free. During the paired trace, sampled
  safeguards stop the callback at less than 2 GB free or greater than 1 GB
  evidence. This is a stop threshold, not an exact size cap; save can add
  data. Peer waits/recorder commands are bounded; stop/save uses 120 seconds.
- Host saves `paired-<id>-host.zip`; guest saves/copies
  `paired-<id>-guest.zip` to the host bundle. Only the current probe is zipped,
  excluding earlier audio history. Protocol messages remain under
  `paired-runs\<id>`; `host\before-clock.csv` / `after-clock.csv` are in the
  host ZIP. Capture VirtualBox process IDs/names/start times without command
  lines; ETL includes thread/process lifetime to resolve identity/reuse.

Rollback: omit these optional scripts and keep previous artifacts. No driver,
clock, VM or Windows setting needs undoing. Failed continuity remains open.

Recorder lifecycle uses the minimal profile and Microsoft's documented
[custom recording profiles](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/authoring-recording-profiles)
and [WPR commands](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/wpr-command-line-options).
Local checks and real-recorder limitations are in the
[review record](../plans/active/evidence/2026-10-09-m03-packet-clock-review.md).
