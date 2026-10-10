# Automatic direct audio diagnosis

**Current state:** the r2 run has finished and its recordings have been
reviewed. Do not repeat the procedure below or extend it. Direct Cable B
samples fit the expected tones between one real loss event; they do not
contain the reported continuous hiss. All audio workers also paused about
288 ms, and both recordings are short. The listening/playback path and the
shared pause need separate investigation. See the
[waveform review](../plans/active/evidence/2026-10-09-m03-direct-audio-preparation.md#direct-r2-recordings-and-reporting-repair-2026-10-09).
The procedure below is retained for reproducibility, not a new retry request.

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

Next task: review both direct waveforms; repair the measured owning layer,
then qualify that same build with bounded short/sustained and specified gates.
