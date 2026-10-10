# Packet-clock repair: next VM test

This procedure tests the repairs in the [review record](../plans/active/evidence/2026-10-09-m03-packet-clock-review.md).
Use only the **AR-DriverTest VM**. The host shared folder is
`C:\VMs\ar-share`; inside the VM it is `Z:\`. Keep WSL/Hyper-V enabled.
The zero-error gate is unchanged. This bounded test does not replace the
one-hour/eight-cable or hardware qualification gates.

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
