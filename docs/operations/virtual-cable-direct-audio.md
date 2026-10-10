# Automatic direct audio diagnosis

**Current state:** the r2 run has finished and its recordings have been
reviewed. Do not repeat the procedure below or extend it. Direct Cable B
samples fit the expected tones between one real loss event; they do not
contain the reported continuous hiss. All audio workers also paused about
288 ms, and both recordings are short. The listening/playback path and the
shared pause need separate investigation. See the
[waveform review](../plans/active/evidence/2026-10-09-m03-direct-audio-preparation.md#direct-r2-recordings-and-reporting-repair-2026-10-09).
The procedure below is retained for reproducibility, not a new retry request.

## Current next step: supported-format speaker reference

**Result received:** all three sections have continuous static. The independently
synthesized file matches the supported speaker format and contains the intended
tones. That reproduces hiss without AudioRouter-generated samples; it does not
identify the faulty playback component or resolve the driver's measured loss.

**Listener-off result:** user reports much less static, with several distinct
crackles in the early/final portions. The middle is not perfectly clean either.
Times are approximate; silence quality is unconfirmed. This does not identify
a 47-Hz or sample-rate defect. Keep Listen off.

### Completed guest speaker recording: review before another test

The returned `speaker-loopback-0e7000c055ba41d392abb3a902097fca.zip`
matches the user's checksum. After a 64-frame alignment, **749,260 stereo
frames (16.990023 seconds) match the PCM16 reference exactly** when expanded
to float32. Both reported noisy sections, 0–5 and 12–16 seconds, have zero
sample differences. The two silent separators also match exactly.

This places the reported audible crackles downstream of the guest loopback
capture point for those sections. It does not identify whether guest HDA,
VirtualBox's audio backend or the host playback path introduces them. The
last 440 frames of the first reference differ, and further tone sections
continue afterward; the full 30-second recording is not declared clean.
Packet position gaps/discontinuity flags remain in the evidence and do not
by themselves establish missing audio: the matching sections include them.
See the [speaker recording evidence](../plans/active/evidence/2026-10-09-m03-direct-audio-preparation.md#guest-speaker-recording-review-2026-10-09).

**Next:** leave Listen off and stop playback after this completed comparison.
Review the current VirtualBox playback configuration/backend before preparing
one reversible comparison. No new audio run or configuration change is
requested here. Preserve WSL/Hyper-V, installed driver and speaker formats.
The independent bridge loss and sustained-continuity gates remain failed.

The following command is retained to reproduce the completed recording;
do not repeat it unchanged.

A new user-mode bundle records the VM's exact Speakers endpoint for 30 seconds
and opens the same 17-second reference after the recorder is ready. It starts
no AudioRouter cable/bridge stream and changes no audio/VM settings. Native
44.1-kHz stereo PCM16 or float32 mix is retained without resampling.
WASAPI loopback observes the rendering endpoint mix; it does not record the
host/hardware/acoustic output ([Microsoft loopback recording](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)).

Inside **AR-DriverTest**:

1. Keep Cable B Output **Listen to this device unchecked**. Stop test scripts
   and other playback. Turn Media Player's Repeat off for this one reference.
2. Keep Media Player's output **Speakers (High Definition Audio Device)**,
   speaker setting 16-bit/44.1 kHz, and cable settings unchanged.
3. Paste this block in the VM's PowerShell window:

   ```powershell
   & {
       robocopy.exe 'Z:\diagnostics-20261009-speaker-loopback' 'C:\ar\diagnostics-20261009-speaker-loopback' /E /R:1 /W:1 /XF speaker-loopback-*.zip
       if ($LASTEXITCODE -ge 8) { throw 'Bundle copy failed. Stop here.' }
       powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261009-speaker-loopback\run-speaker-loopback.ps1'
       if ($LASTEXITCODE -ne 0) { throw 'Speaker capture failed; evidence was preserved. Send the output.' }
   }
   ```

4. The reference opens automatically. Listen once and wait for the final result
   (normally under one minute). Send the output and whether crackles occurred.
   Only this run's WAV, packet/timing metadata and process logs are zipped to
   `Z:\diagnostics-20261009-speaker-loopback\speaker-loopback-<run>.zip`.

Capture completion is **not** an audio-quality pass. Review the raw recording
before another cable or long test. Another guest app's audio would contaminate
this mix, so stop unrelated playback. No microphone or host endpoint is opened.
The helper refuses the host before enumeration and has fixed native/control/
parent deadlines. Rollback: stop reference playback; the wrapper stops only
its owned recorder. All previous bundles and driver binaries remain unchanged.

### Prepared next step: VirtualBox playback backend comparison

The installed version is **7.2.20r175154**. Its official source confirms that
selecting DirectSound alone can silently select Windows Audio instead. A
VM-specific override is needed for a real comparison. Verified against the
[Oracle source archive and checksums](https://download.virtualbox.org/virtualbox/7.2.20/),
`ConsoleImplConfigCommon.cpp:3755–3787` and
`VBoxManageModifyVM.cpp:2783–2806`. Source archive SHA-256:
`5c2138213b72f36c129b92c2c267f2a40e9c98513f4c86a584327f09f9be706d`.

This compares two VirtualBox playback implementations. It keeps HDA, the
guest driver/formats and the host output unchanged. DirectSound still uses
Windows audio services; a difference cannot prove the exact defective layer
or resolve the separate bridge stalls. No driver or long test runs here.

**Step 1 — inside the VM:** stop playback and use Windows **Start → Power →
Shut down**. Wait until VirtualBox says **Powered Off**. Do not save state
or restore a snapshot. Keep Listen off.

**Step 2 — on the main PC**, in PowerShell under the same Windows user who
opened VirtualBox, paste this complete block. It refuses running/saved VMs
or changed baseline settings, and saves the original XML for reference.
Never restore by copying XML over VirtualBox's live configuration.

```powershell
& {
    $ErrorActionPreference = 'Stop'
    $vbox = 'C:\Program Files\Oracle\VirtualBox\VBoxManage.exe'
    $machinePath = 'C:\VMs\AR-DriverTest\AR-DriverTest.vbox'
    $info = & $vbox showvminfo 'AR-DriverTest' --machinereadable
    if ($LASTEXITCODE -ne 0) { throw 'Could not read VM state; nothing changed.' }
    if ($info -notcontains 'VMState="poweroff"') { throw 'Shut down guest Windows normally first; nothing changed.' }
    [xml]$machine = Get-Content -LiteralPath $machinePath -Raw
    [xml]$globalConfig = Get-Content -LiteralPath "$env:USERPROFILE\.VirtualBox\VirtualBox.xml" -Raw
    $audio = $machine.VirtualBox.Machine.Hardware.AudioAdapter
    if ($machine.VirtualBox.Machine.name -ne 'AR-DriverTest' -or $audio.useDefault -ne 'true' -or $audio.driver -ne 'WAS') {
        throw 'Audio baseline differs; stop and send the output. Nothing changed.'
    }
    $selector = "//*[local-name()='ExtraDataItem' and @name='VBoxInternal2/Audio/WindowsDrv']"
    if ($machine.SelectNodes($selector).Count -ne 0 -or $globalConfig.SelectNodes($selector).Count -ne 0) {
        throw 'An audio override already exists; nothing changed.'
    }
    $backup = 'C:\VMs\ar-share\vbox-audio-before-' + [Guid]::NewGuid().ToString('N') + '.xml'
    Copy-Item -LiteralPath $machinePath -Destination $backup
    Write-Host "Original configuration saved: $backup"
    & $vbox setextradata 'AR-DriverTest' 'VBoxInternal2/Audio/WindowsDrv' 'dsound'
    if ($LASTEXITCODE -ne 0) { throw 'Override command failed; stop and send the output.' }
    & $vbox modifyvm 'AR-DriverTest' --audio-driver dsound
    if ($LASTEXITCODE -ne 0) {
        & $vbox setextradata 'AR-DriverTest' 'VBoxInternal2/Audio/WindowsDrv'
        if ($LASTEXITCODE -ne 0) { throw 'Backend and cleanup commands failed; keep VM off and send the output.' }
        throw 'Backend command failed; override removed. Keep VM off and send the output.'
    }
    Write-Host 'DirectSound comparison configured. Start the VM normally and send this output before playing audio.'
}
```

**Step 3 — start the VM normally**, then send Step 2's output. The agent
checks the current startup log for `DSoundAudio` before giving the playback
command. Do not rerun the speaker capture or a cable tone yet. An unreadable
log or wrong backend blocks the comparison, not independent offline work.

**Rollback — main PC, only with the VM Powered Off:**

```powershell
& {
    $ErrorActionPreference = 'Stop'
    $vbox = 'C:\Program Files\Oracle\VirtualBox\VBoxManage.exe'
    $info = & $vbox showvminfo 'AR-DriverTest' --machinereadable
    if ($LASTEXITCODE -ne 0) { throw 'Could not read VM state; nothing changed.' }
    if ($info -notcontains 'VMState="poweroff"') { throw 'Shut down guest Windows normally first.' }
    & $vbox modifyvm 'AR-DriverTest' --audio-driver default
    if ($LASTEXITCODE -ne 0) { throw 'Backend restore failed; keep VM off and send the output.' }
    & $vbox setextradata 'AR-DriverTest' 'VBoxInternal2/Audio/WindowsDrv'
    if ($LASTEXITCODE -ne 0) { throw 'Override removal failed; keep VM off and send the output.' }
    [xml]$machine = Get-Content -LiteralPath 'C:\VMs\AR-DriverTest\AR-DriverTest.vbox' -Raw
    if ($machine.VirtualBox.Machine.Hardware.AudioAdapter.useDefault -ne 'true' -or
        $machine.SelectNodes("//*[local-name()='ExtraDataItem' and @name='VBoxInternal2/Audio/WindowsDrv']").Count -ne 0) {
        throw 'Rollback verification failed; keep VM off and send the output.'
    }
    Write-Host 'Original default Windows Audio selection restored. Start the VM normally.'
}
```

Preparation is source/XML/log review and offline PowerShell parsing only.
The agent has not applied these commands or tested DirectSound playback.

### Completed comparison inside the VM: disable the concurrent listener

1. Stop Media Player playback and leave all tone scripts stopped.
2. Open the Sound control panel:

   ```powershell
   Start-Process control.exe -ArgumentList 'mmsys.cpl'
   ```

3. **Recording → AudioRouter Cable B Output → Properties → Listen**.
   Clear **Listen to this device**, then click **Apply → OK**. If it is already
   clear, leave it clear. Keep the current speaker and cable formats.
4. Confirm Media Player's output is **Speakers (High Definition Audio Device)**.
   Play the existing reference:

   ```powershell
   Invoke-Item -LiteralPath 'C:\ar\speaker-reference-44100.wav'
   ```

5. Report whether static remains in the three tone sections and in the one-second
   silent gaps. Leave Listen off until this result is reviewed. This is one
   17-second playback comparison; no bridge or long test runs.

Rollback: recheck **Listen to this device** after the comparison if it was
previously enabled. This guest-only setting removes a concurrent capture-to-speaker
client; it does not alter host audio, VirtualBox, Hyper-V/WSL or the driver.

### Completed reference preparation and playback

User also hears hiss while replaying the saved WAV through Media Player →
Speakers. Speakers only offers 16-bit 16/22.05/44.1 kHz. Keep Speakers at
**16 bit, 44,100 Hz** and Cable B at **32 bit, 48,000 Hz**. Do not force an
unsupported speaker format or run another bridge tone. The bit-depth
difference alone is not a defect: the Windows audio engine can process float
samples while a device uses PCM ([Microsoft device formats](https://learn.microsoft.com/en-us/windows/win32/coreaudio/device-formats)).

The new reference file is synthesized offline, stereo PCM16 at 44.1 kHz,
matching the supported speaker format. It carries no driver recording,
dropout or sample-rate conversion from 48 kHz. Keep Media Player's output
set to **Speakers (High Definition Audio Device)**, then paste inside the VM:

```powershell
& {
    $source = 'Z:\playback-reference-20261009\reference-44100.wav'
    $target = 'C:\ar\speaker-reference-44100.wav'
    $hash = 'C9652D3C629C45FE7BAC8AB0C332A6668F7BD584E3A55323BA0CE9F54842D6A7'
    if (Test-Path -LiteralPath $target) {
        if ((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash -ne $hash) {
            throw 'An unrelated file already uses the target name. Stop here.'
        }
    } else {
        Copy-Item -LiteralPath $source -Destination $target
    }
    if ((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash -ne $hash) {
        throw 'Reference copy failed verification. Stop here.'
    }
    Invoke-Item -LiteralPath $target
}
```

Playback lasts **17 seconds**. There is one second of silence between parts:

1. **0–5 s:** high tone on the left, low tone on the right (997/47 Hz).
2. **6–11 s:** high tone on both channels (997 Hz).
3. **12–17 s:** low tone on both channels (47 Hz).

User reports continuous static in all three parts. Do not repeat this completed
comparison unchanged. The separately prepared 48-kHz reference remains held;
the completed listener and speaker-recording reviews are above. No continuity
gate has passed through this playback observation.

## Previous direct-recording procedure

Use the **r2 bundle** below. The first helper was accidentally built with a
dynamic Visual C++ runtime dependency and exited before readiness in the
guest. The repaired helper includes that runtime; packaging checks its PE
imports and the wrapper checks executable startup before any audio test.

Current task: explain Cable B's reported continuous hiss before another long
run. Media Player routed directly to VM speakers sounded clean to the user.
That observation narrows the reproduction but does not identify the cause.
The earlier Cable A recording did not record Cable B Output.

This update adds a user-mode generator/recorder to the installed driver test.
It generates 440/660 Hz on **Cable A Input**, records **Cable B Output**
directly while the bridge sends 997/47 Hz, and analyzes both recordings.
No manual playback, Audacity, reinstall, snapshot restore or reboot is needed.
The record mode refuses to open endpoints outside AR-DriverTest.

## 1. Prepare inside the VM

1. **Stop playback in Media Player.** Leave it stopped throughout this test.
2. Leave the existing **Cable B Output → Listen → Speakers** setting as it is.
   This lets the same listening path reproduce the reported hiss while the
   probe records Cable B directly. No volume/default-device changes are made.
3. Open **Windows PowerShell as Administrator inside AR-DriverTest**.

## 2. Copy and run inside the VM

Paste this entire block into that VM PowerShell window:

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261010-direct-audio-r2' 'C:\ar\diagnostics-20261010-direct-audio-r2' /E /R:1 /W:1 /XF direct-*.zip
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }

    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-direct-audio-r2\run-direct-audio.ps1'
    if ($LASTEXITCODE -ne 0) { throw 'Diagnostic failed. Evidence was preserved; send the output.' }
}
```

The audio portion runs for 30 seconds. Status, saving and analysis add time;
normally the whole command takes about one minute. It has readiness, native
process, recorder and analysis deadlines. Do not start another audio test.
The wrapper stops only processes it created and archives only the current
recordings/reports, avoiding the large historical evidence collection.

## 3. Send the result

Send the final PowerShell output and say whether the listening path still had
hiss. Both WAVs and reports are copied automatically into:

```text
Z:\diagnostics-20261010-direct-audio-r2\direct-<run>.zip
```

On the main PC, the same file is under:

```text
C:\VMs\ar-share\diagnostics-20261010-direct-audio-r2\
```

Do not repeat a failing run or start a longer/stall run. Review the saved
samples first: a noisy Cable B WAV identifies a digital-path failure; clean
digital samples with audible hiss require reviewing the listener/playback
path. Neither result alone identifies the exact defective function.

## Diagnostic contract and limits

- Only one exact active Cable A Input / Cable B Output name is accepted;
  opaque IDs are retained. No default or microphone fallback. Stereo IEEE
  float32 at **48,000 samples/second** is required and metadata is rechecked
  after opening. **47 Hz** is a generated tone frequency, not the sample rate.
- Samples/packet metadata are preallocated before stream start. The service
  loop performs no file/console I/O. WAV and JSON writes happen after Stop.
  Storage and audio lifetime are bounded to 100 seconds; normal stop follows
  the bridge's 30-second run. The existing driver buffers are unchanged.
- Preserve raw samples, packet flags, device/QPC positions, pump gaps, native
  exit/counter reports and endpoint identity. In-signal discontinuity or
  timestamp-error flags fail the diagnostic.
- Offline sine fitting allows unknown starting phase, checks both channels
  for noise/distortion/DC/amplitude/phase jumps, and requires 29.8–30.2
  seconds of active audio. Pre/post-lease silence is not fitted. One 10-ms
  boundary quantum on each side is excluded from the fit; the full WAV and
  those packets remain available for startup/teardown review.
  The source analyzer now retains duration failure and per-window waveform
  metrics together, so a short recording cannot hide distortion or phase
  breaks. A duration failure still fails the command. Existing r2 artifacts
  retain the previous analyzer; review can use the repaired analyzer offline.
- Triage limits: residual RMS ≤ 0.00001, residual peak ≤ 0.0001, DC ≤ 0.00001,
  amplitude 0.24–0.26, inter-window phase jump ≤ 0.001 radians. These are
  diagnostic limits, **not** replacements for VCAB-20 bit-exact conversion,
  THD+N, VCAB-24 one-hour/eight-cable continuity, or latency/signing gates.
  A diagnostic pass is marked `qualification: false` in its reports.
- Nothing changes host audio, WSL/Hyper-V, security, VM or power settings.
  The driver package and native bridge-tone binary are copied unchanged.
  Rollback is to stop this test and use the previous bundle.

Next task: investigate playback after the guest loopback boundary and the
separate shared scheduling pause; repair only the established owning layer,
then qualify that same build with bounded short/sustained and specified gates.
