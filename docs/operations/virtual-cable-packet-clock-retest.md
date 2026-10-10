# Packet-clock repair: next VM test

This procedure tests the repairs in the [review record](../plans/active/evidence/2026-10-09-m03-packet-clock-review.md).
Use only the **AR-DriverTest VM**. The host shared folder is
`C:\VMs\ar-share`; inside the VM it is `Z:\`. Keep WSL/Hyper-V enabled.
The zero-error gate is unchanged. This bounded test does not replace the
one-hour/eight-cable or hardware qualification gates.

## Next step: check recorder compatibility for two seconds

The five-minute test now finishes on time, but all three servicing threads
show a delay around 310–317 ms when audio is lost. The next useful evidence
is a scheduling trace from **inside the VM**. The first tracing attempt failed
before audio started: WPR rejected GeneralProfile.Light.File with 0x80070032.
The revised collector uses a smaller scheduling profile. Its syntax is checked;
actual guest recording must pass this short probe first. Driver/audio bytes stay
unchanged; no reinstall, snapshot restore or WSL/Hyper-V change is needed.

This probe launches no tone test. Leave your sound settings as they are. Paste
into **Administrator PowerShell inside AR-DriverTest**:

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261009-scheduling-trace-r2' 'C:\ar\diagnostics-20261009-scheduling-trace-r2' /E /R:1 /W:1
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261009-scheduling-trace-r2\run-packet-clock-review.ps1' -Phase TraceProbe
}
```

Recording takes two seconds, followed by start/save processing (stop watchdog
up to two minutes) and a small ZIP copy. Earlier audio files are excluded.
Send the final output, including any exact startup error. Do not start another
audio test until the probe archive has been reviewed. This compatibility
candidate does not establish why the original broad profile was unsupported.
Do not cancel an existing WPR recording.

The probe ZIP includes `scheduling.etl`, recorder command logs/status, the exact
`used-profile.wprp`, `trace-summary.json` and `probe-result.json`. A nonempty
saved file alone does not prove that all required events are present; inspect
the trace before proceeding. Tracing adds overhead; neither a pass nor a
failure alone closes the sustained gate.
The collector stops/saves its own uniquely named recording before collection.
If stop/cancel fails or the guest crashes, preserve the VM files and send the
output. Do not run a longer or intentional-stall test.

After the probe is reviewed, keep the Media Player loop routed to **Cable A
Input** and **Cable B Output → Listen → Speakers** enabled. The subsequent
five-minute diagnostic command (do not run yet) is:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261009-scheduling-trace-r2\run-packet-clock-review.ps1' -Phase Tone -ToneSeconds 300 -TraceScheduling
```

The tone watchdog is six minutes; trace save and ZIP collection follow.
The tone ZIP includes `scheduling\scheduling.etl` and native timing reports.

## Current harness update after the five-minute failure

The r2 driver remains installed. Its streaming-output test program is
superseded by **repair-20261009-bounded-tone**. The update reuses exactly the
same verified driver; this step requires no snapshot restore or reinstall.
Earlier audio losses remain unresolved. This run checks the reporting repair
and gathers timing evidence; it does not guarantee a continuity pass.

Keep the existing Media Player loop routed to **Cable A Input**, and
**Cable B Output → Listen → Speakers** enabled. Inside the VM, open
**Administrator PowerShell** and paste:

```powershell
& {
    robocopy.exe 'Z:\repair-20261009-bounded-tone' 'C:\ar\repair-20261009-bounded-tone' /E /R:1 /W:1
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\repair-20261009-bounded-tone\run-packet-clock-review.ps1' -Phase Tone -ToneSeconds 30
    if ($LASTEXITCODE -ne 0) { throw 'Tone failed; evidence was copied. Send the output.' }
}
```

The measurement is quiet while it runs. Native output goes to
`tone-stdout.txt` / `tone-stderr.txt`; interval snapshots are saved in
`render-source.progress.txt` after audio stops. `tone-process.json` records
exit code and elapsed time. The 30-second process watchdog is 90 seconds,
followed by separate evidence collection/copy time. A timeout fails with exit
124. A paused VM can delay execution of its watchdog; this is not a host
watchdog. Send the final output before any longer run. Do not use the old r2
tone command below.

If the short updated run is reviewed and passes, the next command is:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\repair-20261009-bounded-tone\run-packet-clock-review.ps1' -Phase Tone -ToneSeconds 300
```

Its process watchdog is 360 seconds plus termination/collection time. Zero
counters remain mandatory. No stall or longer qualification is authorized by
this procedure.

## Original r2 setup (historical commands)

The steps below record the original preparation and passing short test. For
current tone testing use the update above. After a fresh snapshot restore,
substitute the bounded-tone folder in the preparation command as well.

## 1. Restore the clean snapshot

Shut down the VM. In VirtualBox, restore your saved clean test-signing
snapshot (the one with test mode enabled and no AudioRouter driver).
Start the VM normally. Open **PowerShell as Administrator inside the VM**.

## 2. Copy and prepare the new bundle

Paste this whole block into that PowerShell window:

```powershell
& {
    robocopy.exe 'Z:\repair-20261009-packet-clock-r2' 'C:\ar\repair-20261009-packet-clock-r2' /E /R:1 /W:1
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\repair-20261009-packet-clock-r2\run-packet-clock-review.ps1' -Phase Prepare
    if ($LASTEXITCODE -ne 0) { throw 'Preparation failed. Send the output; stop here.' }
    Copy-Item -LiteralPath 'Z:\loop-test-60s.wav' -Destination 'C:\ar\loop-test-60s.wav'
}
```

This verifies every bundle hash, runs preflight and smoke, then installs the
driver and checks all four active endpoints/formats. It collects evidence and
copies the ZIP to `Z:\`. Continue only when **Prepare passed** appears.

## 3. Keep both audio directions running

1. Open `C:\ar\loop-test-60s.wav` in Media Player. Enable **Repeat/loop**.
2. In Windows **Settings → System → Sound → Volume mixer**, set Media
   Player's **Output device** to **AudioRouter Cable A Input**. Start playback
   and leave it playing throughout both tests. Check the Cable A Input bar
   moves in the Sound panel.
3. Open `mmsys.cpl` → **Recording → AudioRouter Cable B Output → Properties →
   Listen**. Tick **Listen to this device**, select **Speakers (High Definition
   Audio Device)** under playback, then Apply/OK. Leave this enabled.

Start the loop **before** the test command. No playback-pause workaround is
needed for this candidate. Keep the VM running and avoid changing sound
settings during measurement.

## 4. Run 30 seconds

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\repair-20261009-packet-clock-r2\run-packet-clock-review.ps1' -Phase Tone -ToneSeconds 30
```

Wait until it ends. Continue only if **Tone passed** appears and every error
counter is zero. If it fails, stop and send the output; its evidence has
already been collected and copied to the host shared folder.

## 5. Run five minutes

Keep the audio loop and listener running. Paste:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\repair-20261009-packet-clock-r2\run-packet-clock-review.ps1' -Phase Tone -ToneSeconds 300
```

Wait approximately five minutes plus collection time. Send the final output
and the **Evidence copied to shared folder** filename. The ZIP includes the
per-interval worker gaps. Do not run an intentional stall or longer test yet.
If the VM crashes, preserve its new dump and power it off; the script cannot
collect after a guest crash.
