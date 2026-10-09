# Testing the AudioRouter virtual cable (DEC-18)

How the AudioRouter-owned virtual cable is tested from the first build to a
public release, without risking the maintainer's daily PC. What is built:
[17 Virtual cable](../spec/17-virtual-cable.md). Work packages (WP-xx):
[virtual cable plan](../plans/active/virtual-cable.md). Signing:
[virtual cable signing](virtual-cable-signing.md). Costs and background:
[driver track](../plans/future/M03-driver-signing.md). Requirements:
VCAB-01–12, VDEV-01–12, SEC-08, NFR-02, NFR-16.

## The test ladder

| Stage | Where | Driver signature | Costs money? | Proves |
| --- | --- | --- | --- | --- |
| A | Test VM, Secure Boot **off**, test signing **on** | Developer test certificate | No | The driver works: devices, audio both ways, silence, recovery, uninstall |
| B | Fresh VM, Secure Boot **on**, Memory Integrity **on** | Microsoft (attestation) | Yes, the EV certificate (buy only after A passes) | Windows accepts the real signed package like on a user's PC |
| C | Main PC | Microsoft (same package as B) | No | Real hardware, Discord/OBS/games, anti-cheat, a week of daily use |
| D | 2–3 beta testers' PCs | Microsoft | No | Other hardware and Windows builds |
| E | Public release | Microsoft | No | — |

Why the main PC cannot be used before stage B: Windows 11 with Secure Boot
loads only Microsoft-signed kernel drivers. Loading a test-signed driver
needs test-signing mode, which needs Secure Boot off. That weakens the PC's
security (SEC-08 forbids it on a daily machine), and anti-cheat systems
such as BattlEye (Rainbow Six Siege) and Vanguard refuse to run in test
mode. The VM takes that role instead.

A product point the tests confirm: the cable carries audio **through the
AudioRouter engine** (playback side → engine → recording side). With
AudioRouter stopped, the recording side delivers silence (VDEV-04). It is not
a stand-alone cable like VB-Cable.

## Stage A — test-signed driver in a VM (free)

**First time? Follow the [VM guide](virtual-cable-vm-guide.md).** It is the
step-by-step version of this stage for someone who has never set up a VM,
uses **VirtualBox** (the development PC runs Windows 11 Home, which has no
Hyper-V), builds every test file with `tools/vm/prepare-vm-share.ps1`, and
runs each check inside the VM with `tools/vm/vm-checks.ps1 -Step <name>`.
The Hyper-V commands below apply only to Windows 11 Pro/Enterprise hosts.

### One-time host setup (main PC, administrator PowerShell)

Requires Windows 11 Pro/Enterprise (Hyper-V) and a Windows 11 ISO. Nothing below
changes the host's audio, drivers, Secure Boot or test-signing state.

```powershell
# Enable Hyper-V (reboot once).
Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All

# Create the VM (Generation 2, TPM, 4 CPUs, 8 GB, 80 GB disk).
$vm = 'AR-DriverTest'
New-VM -Name $vm -Generation 2 -MemoryStartupBytes 8GB `
  -NewVHDPath "$env:PUBLIC\Documents\Hyper-V\$vm.vhdx" -NewVHDSizeBytes 80GB `
  -SwitchName 'Default Switch'
Set-VMProcessor -VMName $vm -Count 4
Set-VMKeyProtector -VMName $vm -NewLocalKeyProtector
Enable-VMTPM -VMName $vm
Add-VMDvdDrive -VMName $vm -Path 'C:\ISO\Win11.iso'
Set-VMFirmware -VMName $vm -FirstBootDevice (Get-VMDvdDrive -VMName $vm)
Set-VM -Name $vm -CheckpointType Standard
Enable-VMIntegrationService -VMName $vm -Name 'Guest Service Interface'
```

Install Windows in the VM, apply Windows updates, then shut it down and
create the first checkpoint:

```powershell
Checkpoint-VM -Name $vm -SnapshotName '01-clean-windows'
```

### Prepare the VM for test signing (inside the VM only)

```powershell
# Host: turn Secure Boot off for this VM only.
Set-VMFirmware -VMName $vm -EnableSecureBoot Off
```

```powershell
# VM, administrator PowerShell:
bcdedit /set testsigning on
# Trust only the PUBLIC part of the developer test certificate, in the VM:
Import-Certificate -FilePath C:\ar\driver\AudioRouterTest.cer -CertStoreLocation Cert:\LocalMachine\Root
Import-Certificate -FilePath C:\ar\driver\AudioRouterTest.cer -CertStoreLocation Cert:\LocalMachine\TrustedPublisher
Restart-Computer
```

Then on the host: `Checkpoint-VM -Name $vm -SnapshotName '02-test-signing-ready'`.
Every test run starts from this checkpoint and ends by restoring it:

```powershell
Restore-VMCheckpoint -VMName $vm -Name '02-test-signing-ready' -Confirm:$false
```

### Build on the host, copy into the VM

```powershell
# Host, x64 Developer PowerShell; choose a fresh output folder:
.\drivers\audiorouter-virtual\build.ps1 -Configuration Release -Platform x64 -Version 0.1.0 -KeepOutput -Output C:\ar\driver-pkg
.\drivers\audiorouter-virtual\sign-test.ps1 -Package C:\ar\driver-pkg
.\tests\acceptance\m03-driver-package.ps1 -Package C:\ar\driver-pkg
# Copy the driver package (with AudioRouterTest.cer), the AudioRouter build
# and the test scripts into the VM:
Copy-VMFile -Name $vm -SourcePath <package folder or zip> -DestinationPath C:\ar\driver\ -CreateFullPath -FileSource Host
```

The private key of the test certificate never leaves the host; only the
signed package and the `.cer` go into the VM.
`build.ps1 -TestSign` combines build and test signing in one command. Signing
is offline without a timestamp by default; see the
[package workflow](../../drivers/audiorouter-virtual/README.md#versioned-test-package-wp-02)
for certificate selection and optional timestamping. Build/sign the package
before preparing the VM certificate trust above. Host verification records an
untrusted test root as expected and never imports that root on the host.

### Stage A checks (inside the VM)

WP-03 runner (copy the repository scripts and `drivers/audiorouter-virtual`
alongside the test package into the VM):

```powershell
# VM only, administrator PowerShell. Identity marker is needed if the Windows
# computer name is not AR-DriverTest; never create this marker on the host.
New-Item -ItemType File C:\ar\IS_TEST_VM -Force
.\tests\acceptance\m03-driver-vm.ps1 -Package C:\ar\driver
# If A2 reports that no root device exists, restore the checkpoint and retry
# with the signed x64 WDK devcon.exe copied into the VM:
.\tests\acceptance\m03-driver-vm.ps1 -Package C:\ar\driver -CreateRootDevice -Devcon C:\ar\tools\devcon.exe
```

The default profile expects the prototype's two endpoints; after WP-05 use
`-EndpointProfile Cables` for Cable A/B's four endpoints. `-Cli <absolute exe>`
adds CLI inventory if supplied. The runner stages a unique package beneath the
developer wrapper's allowed driver tree, records ownership state, removes only
the exact matching root instance and owned package in `finally`, and compares
the full baseline. It records six actual default endpoint IDs through read-only
MMDevice COM; `Win32_SoundDevice` inventory alone cannot prove Windows defaults.
Failures retain `summary.json` and raw logs; redact IDs/paths before sharing.
`-KeepInstalled` skips A14 only after successful A1–A3 and is not a full smoke
pass. Restore the checkpoint after every run, including failures.

WP-06 bridge tone tool (A4 tool precursor, A5, A6; before the engine
integration). Build on the host, copy the exe into the VM with the package:

```powershell
# Host:
cargo build --release -p audiorouter-windows-audio --example m03_bridge_tone
# -> target\release\examples\m03_bridge_tone.exe
# VM, driver installed (WP-03 runner with -KeepInstalled), normal user:
.\m03_bridge_tone.exe --seconds 600 --out C:\ar\evidence\render-source.wav
.\m03_bridge_tone.exe --seconds 60 --channels 8 --rate 96000 --out C:\ar\evidence\render-8ch-96k.wav
.\m03_bridge_tone.exe --seconds 30 --stall-ms 500 --out C:\ar\evidence\render-stall.wav
```

The tool first prints the driver's QUERY report and refuses an incompatible
driver (exit 2). It then writes 997 Hz (even channels) and 47 Hz (odd
channels) at −12 dBFS into the `cable-b` capture sink and records the
`cable-a` render source to an IEEE-float WAV (`--wav64` for float64). While
it runs, record **AudioRouter Cable B Output** with a recorder (for example
Audacity or `ffmpeg -f dshow`) and play a known file into **AudioRouter Cable
A Input**. Start playback while the tone command is running; do not wait for
the 30-second or 10-minute command to finish. Zero recorded render blocks
means no playback reached Cable A Input during that run. At the end it prints
both leases' stream counters: a clean run
must end with zero underrun, overrun, gap, non-finite and format-mismatch
counts; the `--stall-ms` run must show them rising. Setting a cable's
Windows format (Sound settings → Advanced) to another rate than `--rate`
must give silence and a rising `format_mismatches`, never wrong-speed audio.
Run `--capture-bus cable-a --render-bus cable-b` too, for both directions on
both cables. It never opens a microphone.

Host-safe checks: `tests/acceptance/m03-driver-vm-guards.ps1` tests only the pure
guard/comparison functions and read-only default enumeration. The real runner
refuses an unidentified host before querying even its boot configuration.

Each check records its output under `C:\ar\evidence\` and is copied to
`docs/plans/active/evidence/` (redacted) when it passes. A1–A3 and A14 are
automated by `tests/acceptance/m03-driver-vm.ps1` (WP-03); the "From WP"
column says when each check becomes possible.

Stage A runs with the default **2 cables**. Expected endpoint names
(VCAB-02): `AudioRouter Cable A Input`, `AudioRouter Cable A Output`,
`AudioRouter Cable B Input`, `AudioRouter Cable B Output`. Check A15 then
qualifies all 8 cables (`… Cable C …` to `… Cable H …`). Before WP-05 the prototype shows
`AudioRouter - Desktop In` and `AudioRouter - Voice Chat`.

| Check | From WP |
| --- | --- |
| A1–A3, A14 | WP-03 (prototype), WP-05 (final names) |
| A4 (tool), A5, A6 | WP-06 with the `m03_bridge_tone` example (above); through AudioRouter after WP-09 |
| A7 | WP-09 |
| A8, A9 | WP-05 |
| A10, A11 | WP-04 |
| A12 | WP-09 |
| A13 | WP-07 |
| A15 | WP-07 (count), WP-09 (audio) |
| A16 | WP-09b |

| # | Check | How | Pass when |
| --- | --- | --- | --- |
| A1 | Baseline | Save `pnputil /enum-drivers`, `Get-PnpDevice -Class Media,AudioEndpoint -PresentOnly`, AudioRouter `devices list` | Files saved |
| A2 | Install | Before WP-07: `manage.ps1 -Install -Preview`, then `-Install -AllowDriverInstall` (+ root device, see WP-03). After WP-07: `audiorouter-driver-helper.exe install --package <dir>` (debug build, `AUDIOROUTER_ALLOW_TEST_DRIVER=1`) | Package in driver store; recorded `oem*.inf`; one `ROOT\AudioRouterVirtual` device |
| A3 | Devices appear | Sound settings, `Get-PnpDevice -Class AudioEndpoint`, `audiorouter-cli virtual-cable status --json` (WP-08) | The four expected names, `Status = OK`; nothing else changed (compare A1) |
| A4 | Audio both ways | AudioRouter pass-through session `Cable A Input → Cable A Output`; `tests/acceptance/m00-native-impulse-loopback.ps1 -AllowLiveAudio -RenderFriendlyName 'AudioRouter Cable A Input' -CaptureFriendlyName 'AudioRouter Cable A Output'`; repeat for Cable B and both at once | All impulses detected; p95 latency ≤ 160 ms (NFR-02); crosstalk between A and B below −90 dBFS; also run against VB-Cable in the same VM for comparison |
| A5 | Clean signal | Tone through the pass-through at unity gain, compare input and output | Nulls within the [quality tolerance](../spec/14-quality.md) |
| A6 | Silence without owner | Stop AudioRouter; record the recording side | Digital silence (VDEV-04) |
| A7 | Crash recovery | Kill the backend during a tone | Silence within 500 ms; no old audio replayed on restart (VDEV-12) |
| A8 | No leak | Play into the cable for 1 hour with nothing reading; watch Performance Monitor `Memory\Pool Nonpaged Bytes` | Flat (VDEV-04) |
| A9 | Reboot | Note endpoint IDs, reboot, compare | Same IDs, devices still present (VDEV-03) |
| A10 | Driver Verifier | `verifier /standard /driver audioroutervirtual.sys`, reboot, repeat A4–A7, then `verifier /reset` | No crash, no verifier stop |
| A11 | Security | Bridge IOCTL fuzzer; a second Windows user tries to open the bridge | Rejected cleanly, no crash (SEC-08, VDEV-07) |
| A12 | Apps | Discord and OBS in the VM select the cable | Both see it and receive the tone (VDEV-11) |
| A13 | Upgrade | Install version N, then N+1 over it | Devices keep their IDs; sessions still bind |
| A14 | Uninstall | Helper `remove` (or `manage.ps1 -Uninstall -AllowDriverInstall` before WP-07); compare with A1 | Matches baseline: no `oem*.inf` from `AudioRouter Project`, no `ROOT\AudioRouterVirtual` or `SWD\AudioRouter*` devices |

| A15 | All 8 cables | Helper `set-cables --count 8`; run 8 pass-through sessions (Cable X Input → Cable X Output) at once with distinct tones, 30 min; then `--count 2` | 16 endpoints present; every cable carries its own tone, crosstalk below −90 dBFS, zero glitches; backend CPU recorded; after `--count 2` Cable A/B keep their endpoint IDs and C–H are gone |

| A16 | Sound quality (VM, indicative) | `tests/acceptance/m03-cable-quality.ps1` (WP-09b), all cable-only cases | Bit-exact float path at 44.1/48/96 kHz × 1/2/8 ch (VCAB-20); integer formats ≤ 1 LSB (VCAB-21); resampler targets (VCAB-22); crosstalk ≤ −140 dBFS (VCAB-26); counters zero. Latency and CPU recorded but judged only in stage C |

Stage A passes when A1–A16 pass on the same build. Only then is the EV
certificate bought (user go-ahead).

## Stage B — Microsoft-signed package in a clean VM

1. Buy the EV certificate, register in Partner Center, submit the CAB,
   download the Microsoft-signed package
   ([virtual cable signing](virtual-cable-signing.md), plan WP-13).
   Repeat stage B on Windows 11 23H2, 24H2 and 25H2 VMs (one checkpoint
   each) to catch a signature limited to one build (Code 52).
2. Restore `01-clean-windows` (test signing never enabled), then on the host:
   `Set-VMFirmware -VMName $vm -EnableSecureBoot On -SecureBootTemplate MicrosoftWindows`
   and `Set-VMProcessor -VMName $vm -ExposeVirtualizationExtensions $true`
   (needed for Memory Integrity inside the VM). In the VM turn on
   Settings → Device security → Core isolation → Memory integrity.
3. Install through the real product path (AudioRouter first run or Setup →
   install the cable → Windows elevation prompt).
4. Check: `pnputil /enum-drivers` shows `Signer Name: Microsoft Windows
   Hardware Compatibility Publisher`; Event Viewer
   `Microsoft-Windows-CodeIntegrity/Operational` has no block for
   `audioroutervirtual.sys`; repeat A3, A4, A6, A9, A14.

Stage B passes when the signed package installs and works with Secure Boot
and Memory Integrity on, and uninstalls cleanly.

## Stage C — the main PC

Only the stage B package, never a test-signed build, and test signing stays
off.

**Before installing**

1. Create a restore point:
   `Checkpoint-Computer -Description 'Before AudioRouter cable' -RestorePointType MODIFY_SETTINGS`
   (System Protection must be on for C:).
2. Save the same baseline as A1, plus the default playback/recording devices
   (screenshot of Sound settings).
3. Keep VB-Cable installed; both can coexist.

**Test**

1. Install via AudioRouter Setup; approve the elevation prompt.
2. Repeat A3, A4 (compare with VB-Cable on the same PC), A6, A9.
3. Real use: Siege output = `AudioRouter Cable A Input` → AudioRouter
   (footstep chain) → headphones; microphone chain → `AudioRouter Cable B
   Output` selected as Discord's microphone; OBS records `Cable A Output`
   from a pass-through. Launch Siege with the driver loaded to confirm
   BattlEye accepts it. Check whether Windows changed a default device
   (VCAB-07).
4. Use it daily for a week; note dropouts, crackles, wake-from-sleep and
   device-change behavior.
4a. **Binding sound-quality and performance run** (17 §11, VCAB-20–30),
   with the WP-09b harness on this PC:
   - all A16 cases again (real hardware);
   - drift path: Cable A Input → AudioRouter → Focusrite output, a loopback
     cable from the Focusrite output to its input, captured back into
     AudioRouter → Cable A Output; 1 hour, THD+N and continuity (VCAB-23);
   - 8 cables at once for 1 hour with distinct tones, plus a game or CPU
     stress at 80 % and one USB device unplug/replug (VCAB-24/30);
   - latency with low-latency and default periods, and the same test on
     VB-Cable (VCAB-25);
   - `wpr -start CPU -start DPC_ISR` during the 8-cable run; driver DPC
     average and maximum, backend CPU (VCAB-27); non-paged pool for 24 hours
     of streaming (VCAB-28).
   Record every value with its target. A miss needs a fix or a written user
   decision before release.
5. Uninstall, compare with the baseline, reinstall.

**If something goes wrong**

- AudioRouter Setup → Remove AudioRouter cable, or
  `pnputil /delete-driver <oemNN.inf> /uninstall /force` (the name is in the
  install state file and in `pnputil /enum-drivers`).
- Restore point from step 1.
- An audio driver is not needed to boot, so Safe Mode can always remove it.

## Stage D — beta testers

A GitHub prerelease with the signed driver for 2–3 people on different
hardware (for example a laptop with built-in audio and a USB interface).
Each runs A3/A4 from the AudioRouter UI and reports diagnostics. Fixes that
change the driver go back through A → B with the same certificate.

## Release gate

Stages A–D recorded in `docs/plans/active/evidence/` with build version,
Windows build, and results; release notes state what was tested and the
known limits.
