# Virtual cable test VM: first-time guide

This is the step-by-step guide for testing the AudioRouter virtual cable
driver in a virtual machine (VM) when you have never done it before. It
covers stage A of the [test procedure](virtual-cable-testing.md): a
test-signed driver in a disposable Windows 11 VM. The design is in
[17 Virtual cable](../spec/17-virtual-cable.md); the work packages are in the
[virtual cable plan](../plans/active/virtual-cable.md).

**What you get out of it:** proof that the driver installs, shows its
endpoints, carries audio both ways, survives hostile input, and uninstalls
cleanly, before any money is spent on Microsoft signing.

**What it never touches:** your main PC's drivers, audio devices, Secure
Boot, Memory Integrity and test-signing settings. Everything risky happens
inside the VM, and every test starts from a saved snapshot, so a crash or a
bad driver costs you one click to undo. The scripts refuse to run on any
computer that is not the test VM.

**Time:** about 2 hours the first time (most of it is Windows installing
and updating), then 15–60 minutes per test session.

## Contents

- [Part 0: Words used in this guide](#part-0-words-used-in-this-guide)
- [Part 1: Prepare your PC (once)](#part-1-prepare-your-pc-once)
- [Part 2: Create the VM (once)](#part-2-create-the-vm-once)
- [Part 3: Finish Windows in the VM (once)](#part-3-finish-windows-in-the-vm-once)
- [Part 4: Turn the VM into a driver test machine (once)](#part-4-turn-the-vm-into-a-driver-test-machine-once)
- [Part 5: Test sessions](#part-5-test-sessions)
- [Part 6: Sending the results](#part-6-sending-the-results)
- [Part 7: When something goes wrong](#part-7-when-something-goes-wrong)
- [Part 8: Updating to a new driver build](#part-8-updating-to-a-new-driver-build)
- [Part 9: Removing everything](#part-9-removing-everything)
- [Appendix: Hyper-V instead of VirtualBox](#appendix-hyper-v-instead-of-virtualbox)

## Part 0: Words used in this guide

| Word | Meaning |
| --- | --- |
| Host | Your real PC (Windows 11 Home on the development PC). |
| VM / guest | The virtual Windows 11 PC that runs in a window on the host. |
| Snapshot | A saved state of the VM. Restoring one undoes everything done after it, including a broken driver or a crash. |
| Secure Boot | A firmware feature that only lets Microsoft-signed drivers load. It is switched **off in the VM only**. |
| Test signing / Test Mode | A Windows mode that also loads drivers signed with a developer certificate. Shown as "Test Mode" in the bottom-right corner. **VM only.** |
| Test certificate | The developer certificate that signs the test driver. Only its public part (`AudioRouterTest.cer`) goes into the VM. |
| Helper | `audiorouter-driver-helper.exe`, the tool that installs, updates and removes the driver (spec 17 §6). |
| Evidence | The folder of logs and results each test writes in the VM (`C:\ar\evidence`). |

## Part 1: Prepare your PC (once)

### 1.1 Why VirtualBox

The development PC runs **Windows 11 Home**. Home does not include
Hyper-V, Microsoft's built-in VM software that the older instructions
assumed. Use **Oracle VirtualBox** instead: it is free, runs on Home, and
supports Windows 11 guests (TPM 2.0, Secure Boot on/off per VM). If you
upgrade to Windows 11 Pro later, Hyper-V also works; see the
[appendix](#appendix-hyper-v-instead-of-virtualbox).

### 1.2 Turn on "Windows Hypervisor Platform"

The PC has Memory Integrity (Core isolation) turned on. That is good and
stays on. It means Windows itself owns the CPU's virtualization feature,
and VirtualBox must go through Windows to use it. Enable that bridge:

1. Press **Win + R**, type `optionalfeatures`, press Enter.
2. Tick **Windows Hypervisor Platform**. Leave everything else as it is.
3. Click **OK** and restart the PC when asked.

This adds a Windows component. It does not change Secure Boot, Memory
Integrity, test signing, drivers or audio. VirtualBox will show a small
green turtle icon in the VM window's status bar, meaning it runs through
Windows' hypervisor. That is expected; the VM is a bit slower, which does
not matter for these tests.

To check virtualization is on in the firmware: Task Manager → Performance →
CPU → "Virtualization: Enabled". It is on by default on this PC's Intel
Core Ultra 9.

### 1.3 Install VirtualBox

1. Go to <https://www.virtualbox.org/wiki/Downloads> and download
   **VirtualBox 7.1 (or newer) platform package for Windows hosts**.
2. Run the installer with the default options. It installs network and USB
   drivers for VirtualBox itself and asks for permission once; accept.
3. You do not need the "Extension Pack".

### 1.4 Download Windows 11

1. Go to <https://www.microsoft.com/software-download/windows11>.
2. Under **Download Windows 11 Disk Image (ISO) for x64 devices**, choose
   **Windows 11 (multi-edition ISO for x64 devices)**, then your language,
   then **64-bit Download**.
3. Save it as `E:\ISO\Win11.iso` (about 6–7 GB).

The VM does not need a product key or activation for these tests; an
unactivated Windows only shows a watermark and limits personalization.

### 1.5 Folders

Create two folders on the D: drive (it has the most free space):

- `E:\VMs` — VirtualBox will store the VM's virtual disk here (up to 80 GB).
- `C:\VMs\ar-share` — the exchange folder between the PC and the VM. **Leave it
  empty**; Part 4 fills it.

## Part 2: Create the VM (once)

Open **Oracle VirtualBox** and click **New**.

**Name and operating system**

| Field | Value |
| --- | --- |
| Name | `AR-DriverTest` |
| Folder | `E:\VMs` |
| ISO Image | `E:\ISO\Win11.iso` |
| Type / Version | detected automatically: Microsoft Windows, Windows 11 (64-bit) |
| Proceed with Unattended Installation | **tick it** (VirtualBox installs Windows for you). VirtualBox 7.0 shows the opposite box, "Skip Unattended Installation": leave that one unticked. |

**Unattended guest OS install setup** (next page)

| Field | Value |
| --- | --- |
| Username | `artest` |
| Password | 123123123 |
| Hostname | `AR-DriverTest` (the test scripts check this exact name) |
| Domain name | leave as proposed |
| Product key | leave empty |
| Edition / Image | **Windows 11 Pro** |
| Install Guest Additions | **ticked** (needed for the shared folder and clipboard) |

**Hardware**

| Field | Value |
| --- | --- |
| Base Memory | 8192 MB |
| Processors | 4 |
| Enable EFI | ticked (required by Windows 11) |

**Virtual hard disk**: Create a new disk, **80 GB**, leave "Pre-allocate
Full Size" unticked.

Click **Finish**. Before starting it, open the VM's **Settings** (Expert
mode, top of the settings window) and check:

- **System → Motherboard**: TPM **v2.0**, **Enable Secure Boot ticked** for
  now (Windows 11 setup expects it; you switch it off in Part 4).
- **Audio**: Enable Audio ticked, Host Audio Driver **Default**, Audio
  Controller **Intel HD Audio**, **Enable Audio Output ticked**, **Enable
  Audio Input unticked**. The VM never needs your microphone, and with input
  off it cannot hear it.
- **Network**: NAT (default). The VM needs the internet for Windows Update.

Click **OK**, then **Start**.

## Part 3: Finish Windows in the VM (once)

1. **Let it install.** The unattended setup installs Windows, signs in as
   `artest` and installs the Guest Additions. It reboots several times and
   takes 20–40 minutes. Do not type anything unless a window asks you to.
   If setup stops at a question (for example region or keyboard), answer
   it; if it insists on a Microsoft account, see Part 7.
2. **Eject the install disk.** In the VM window: **Devices → Optical
   Drives → Remove disk from virtual drive** (if VirtualBox asks, choose
   **Force Unmount**). Hover over the CD icon in the status bar at the
   bottom: it must say the drive is empty. Do not pick `Win11.iso` in that
   menu: that inserts it. While the install disk stays in, every update
   restart boots the installer instead of Windows, which hangs the VM or
   leaves it unable to start.
3. **Check the computer name.** In the VM: Start → Settings → System →
   About. "Device name" must be `AR-DriverTest`. If it is not, click
   **Rename this PC**, type `AR-DriverTest`, restart.
4. **Guest Additions.** If the VirtualBox menu **Devices → Shared Clipboard**
   is greyed out or Part 4's shared folder does not appear, install them by
   hand: **Devices → Insert Guest Additions CD image…**, then in the VM open
   File Explorer → the CD drive → run `VBoxWindowsAdditions.exe` with the
   defaults, restart. Eject that CD afterwards as in step 2.
5. **Clipboard.** **Devices → Shared Clipboard → Bidirectional**, so you can
   paste the commands from this guide into the VM.
6. **Windows Update.** In the VM: Settings → Windows Update → **Check for
   updates**; install everything and restart until no update is left.
   This can take up to an hour once and avoids updates interrupting tests.
   A feature update shows a blue **"Installing Windows 11 — xx% complete"**
   screen and restarts two or three times; it can sit on one percentage
   for 5–15 minutes. Leave it alone while the disk icon at the bottom of
   the VirtualBox window flickers. The driver tests do not need a feature
   update: if one hangs (see Part 7), click **Cancel** on that screen
   instead of retrying. After the updates, check once more.
   Then click **Pause updates → 5 weeks** (the longest choice), so no new
   update starts during a test or right after you restore a snapshot.
7. **Shut the VM down** (Start → Power → Shut down).
8. **Snapshot 1.** In VirtualBox, select `AR-DriverTest`, click the menu
   icon (☰) next to it → **Snapshots** → **Take**. Name it
   `01-clean-windows`.

Take every snapshot with the VM **shut down**. A snapshot of a running VM
also saves its memory; on this PC (VirtualBox through Windows Hypervisor
Platform) that froze the VM for over 15 minutes and failed.

## Part 4: Turn the VM into a driver test machine (once)

### 4.1 Build the test files on the PC

On the PC, open **PowerShell** (no administrator needed) in the repository
and run:

```powershell
cd C:\code\audiorouter
powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\prepare-vm-share.ps1 -Share C:\VMs\ar-share
```

It takes 5–10 minutes and ends with `Ready: C:\VMs\ar-share`. It builds and
places:

| In `C:\VMs\ar-share` | What it is |
| --- | --- |
| `driver\` | The test-signed driver package (`.inf`, `.sys`, `.cat`), `package.json`, logs, and `AudioRouterTest.cer` (public certificate only) |
| `tools\audiorouter-driver-helper.exe` | The installer helper (debug build: the only build that accepts a test-signed driver, and only when a test explicitly allows it) |
| `tools\m03_cable_inventory.exe` | Read-only report: endpoint names, formats, low-latency period |
| `tools\m03_bridge_tone.exe` | Sends test tones through the cables and records what comes back |
| `tools\m03-bridge-fuzz.exe` | Sends random and hostile requests to the driver for 30 minutes |
| `repo\` | The test scripts |
| `vm-checks.ps1` | The one script you run in the VM, step by step |
| `MANIFEST.txt` | Version, git commit and SHA-256 of every file |

Nothing is installed or trusted on the PC by this step. The private key of
the test certificate stays in your Windows certificate store on the PC.

### 4.2 Connect the shared folder

1. With the VM **shut down**, open its **Settings → Shared Folders**.
2. Click the folder icon with **+**:
   - Folder Path: `C:\VMs\ar-share`
   - Folder Name: `ar-share`
   - **Read-only: unticked** (the VM copies its results back here)
   - **Auto-mount: ticked**, Mount point: `Z:`
3. Click **OK** twice.

### 4.3 Switch Secure Boot off (VM only)

Still in the VM's **Settings → System → Motherboard**: **untick "Enable
Secure Boot"**. If VirtualBox asks whether to reset the Secure Boot keys,
answer **No**. Click **OK**.

This is a setting of the VM's virtual firmware. Your PC's Secure Boot is not
affected.

If Windows installed updates (for example a feature update) after you took
Snapshot 1, finish them first and pause updates as in Part 3 step 6. Then,
with the VM shut down, take a new snapshot, `01b-updated-secure-boot-off`.
Otherwise restoring Snapshot 1 brings the old Windows back and the updates
run again. You can then delete `01-clean-windows` to save disk space.

### 4.4 Copy the files and enable Test Mode

Start the VM and sign in. Open **PowerShell as administrator** in the VM
(Start → type `powershell` → right-click **Windows PowerShell** → **Run as
administrator** → Yes). Paste these lines one block at a time:

```powershell
# 1. Copy the test files from the shared folder to the VM's own disk.
#    (Driver files must not be installed from a network path.)
robocopy Z:\ C:\ar /E
Get-ChildItem C:\ar -Recurse | Unblock-File
```

`robocopy` prints a summary; any exit code below 8 means success.

```powershell
# 2. Confirm Secure Boot is off in this VM. Must print False.
Confirm-SecureBootUEFI
```

If it prints `True`, shut down and repeat 4.3.

```powershell
# 3. Turn on Test Mode (VM only).
bcdedit /set testsigning on

# 4. Trust only the PUBLIC test certificate, only in this VM.
Import-Certificate -FilePath C:\ar\driver\AudioRouterTest.cer -CertStoreLocation Cert:\LocalMachine\Root
Import-Certificate -FilePath C:\ar\driver\AudioRouterTest.cer -CertStoreLocation Cert:\LocalMachine\TrustedPublisher
```

If the `TrustedPublisher` import returns **Access is denied** even in an
Administrator PowerShell window, use this command instead for that store:

```powershell
certutil -addstore -f TrustedPublisher C:\ar\driver\AudioRouterTest.cer
```

```powershell
# 5. Restart so Test Mode takes effect.
Restart-Computer
```

Windows asks once whether to install the certificate in Root: answer
**Yes**. After the restart, the bottom-right corner of the desktop shows
**Test Mode** with the Windows build number.

### 4.5 Preflight check

Open PowerShell **as administrator** again and run:

```powershell
cd C:\ar
powershell -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step preflight
```

Every line must be `[PASS]`, ending with `preflight: PASS`. The checks are
read-only: Test Mode on, Secure Boot off, certificate trusted in both
stores, all files present, Windows Audio running, no AudioRouter driver
installed yet, files on the local disk.

### 4.6 Snapshot 2

Shut the VM down. In VirtualBox: Snapshots → **Take** →
`02-test-signing-ready`.

**Every test session starts from this snapshot and ends by going back to
it.** To restore: shut the VM down, open Snapshots, select
`02-test-signing-ready`, click **Restore**, and untick "Create a snapshot of
the current machine state". The command-line equivalent (PC, normal
PowerShell):

```powershell
& "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe" snapshot AR-DriverTest restore 02-test-signing-ready
```

## Part 5: Test sessions

Each session starts from the snapshot named in that session: restore it,
start the VM, sign in, and open **PowerShell as administrator**. In every
PowerShell window used for a session, first change to the shared folder:

```powershell
Set-Location C:\ar
```

The step names in this guide (such as `preflight`, `install`, and `smoke`) are
not commands by themselves. Each session below gives the complete command for
each script step. Copy the command for the step you are on exactly as shown.

For example, run preflight with:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step preflight
```

Run steps one at a time and read the result before continuing. If a step
reports `FAIL`, stop that session and follow its recovery instructions; do not
continue to an install, smoke test, or the next session.

### Copy-and-run commands for each session

For every session below, run `Set-Location C:\ar` first in an Administrator
PowerShell window. Then copy the commands for that session exactly as written,
running one line at a time and waiting for it to finish before running the
next. Do not type only the step name. Do not continue after a failed check.

#### Session 1 — installation smoke test

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step preflight
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step smoke
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step collect
```

#### Session 2 — audio and cable checks

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step preflight
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step install
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step status
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step tone
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step tone-stall
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step tone-8ch
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step collect
```

#### Session 3 — cable lifecycle

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step preflight
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step install
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step cables
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step rename
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step remove
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step collect
```

#### Session 4 — Driver Verifier and fuzzing

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step preflight
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step install
```

After those commands, enable Driver Verifier using the instructions below and
restart when instructed. After signing in and opening Administrator
PowerShell again, run:

```powershell
Set-Location C:\ar
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step fuzz
```

Then disable Driver Verifier using the instructions below and restart when
instructed. After signing in and opening Administrator PowerShell again, run:

```powershell
Set-Location C:\ar
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step collect
```

#### Session 5 — access control

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step preflight
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step install
```

Continue with the separate account and user-switching instructions below. Run
the named tone check with its full command at the indicated user account, then
collect evidence with:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step collect
```

### If preflight fails on test mode or Secure Boot

Do not run `install` while `preflight` has any failures. If it reports
`Test mode on` or `Secure Boot off`, shut down Windows completely. In the
VirtualBox VM settings, open **System > Motherboard** and disable **Secure
Boot** while preserving the VM's existing EFI setting. Start the VM, open an
Administrator PowerShell window, then run:

```powershell
Set-Location C:\ar
bcdedit /set testsigning on
Restart-Computer
```

After Windows restarts, open a new Administrator PowerShell window and check:

```powershell
Confirm-SecureBootUEFI
bcdedit /enum '{current}'
```

`Confirm-SecureBootUEFI` must report `False`, and the current boot entry must
show `testsigning Yes`. If either condition is not met, stop and correct the
VM boot configuration before continuing. Rerun the complete `preflight`
command above and proceed only when it passes. Refresh snapshot
`02-test-signing-ready` while the VM is powered off so later sessions restore
these settings.

Each step prints `[PASS]`/`[FAIL]` lines and a final verdict, and saves
everything under `C:\ar\evidence\<time>-<step>\`. A `FAIL` is not your
mistake: it is a result. Keep going with the remaining steps of that
session unless the step says the driver is not installed.

If the VM shows a blue screen during a test, let it restart, sign in, run
`collect` (it includes the crash dump), and send it. That is exactly what
the VM is for.

### Session 1 — Install and uninstall (do it twice) · 10 min · checks A1, A2, A3, A14

| Step | What happens | Pass means |
| --- | --- | --- |
| `preflight` | Read-only checks | VM still ready |
| `smoke` | Records a baseline (drivers, devices, default devices), installs with the helper, waits for the four endpoints, uninstalls, compares with the baseline | Package and one AudioRouter device installed; **AudioRouter Cable A/B Input/Output** appear and are healthy; after removal the PC is exactly as before |
| `collect` | Zips the evidence | — |

Copy the zip: `Copy-Item C:\ar\evidence-*.zip Z:\`. Restore snapshot 2 and
run the session a second time (the procedure requires two clean runs).

#### Retry the endpoint-naming check (2026-10-08)

The latest clean package is staged at
`repair-20261008-speaker-endpoints-clean`. The previous package made the capture
names correct but used the `KSNODETYPE_ANALOG_CONNECTOR` render-pin category;
Windows hid those render endpoints by default. This build uses the speaker
category with the unique per-cable bridge-pin names. Restore snapshot
`02-test-signing-ready` with the VM powered off, start the VM, and open
**Administrator PowerShell**. Copy and run this command; it overlays the
package into `C:\ar`, verifies every manifest hash, then performs preflight,
smoke and evidence collection:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File Z:\repair-20261008-speaker-endpoints-clean\retry-smoke.ps1
```

The package uses the already-trusted test certificate. If smoke fails, its
runner evidence includes `endpoint-names-on-install-failure.json`, captured
before cleanup. Send the resulting zip so the actual Windows endpoint names
remain available for diagnosis. Do not continue to tone or Verifier unless
smoke ends `PASS` and the VM stays running. If Windows restarts, start the VM
again, run `collect`, then copy the resulting zip to `Z:`. After one clean
smoke run, restore `02-test-signing-ready` and run the same command once
more; two clean runs are required before moving on.

### Session 2 — Audio through the cables · 30 min · checks A4 (tool), A5, A6, VCAB-11, VCAB-25

1. `preflight`, then `install` (installs and keeps the driver; also checks
   that a second install is a no-op).
2. `status` — helper status plus the endpoint inventory: every endpoint
   must accept all 60 formats (44.1/48/96 kHz × 1/2/4/6/8 channels × float,
   16-, 24- and 32-bit) and offer a minimum shared period of 128 frames
   (2.7 ms) or less. For comparison, VB-Cable on the PC reports 480 frames.
3. **Set up listening** (once per session, in the VM):
   - Start → type `mmsys.cpl` → Enter (the classic Sound panel).
   - **Recording** tab → **AudioRouter Cable B Output** → Properties →
     **Listen** tab → tick **Listen to this device**, and set **Playback
     through this device** to the VM's own speakers ("Speakers (High
     Definition Audio Device)"), never an AudioRouter cable → OK. You will now
     hear whatever arrives on Cable B through your PC's speakers. Listening
     also keeps Cable B's recording side running, which the tone test needs
     for its glitch counters.
4. `tone` — runs for 10 minutes. While it runs:
   - You should hear a steady high tone (997 Hz) on the left and a low hum
     (47 Hz) on the right. Clicks, gaps or a changing pitch are findings.
   - In the Sound panel **Playback** tab, right-click **AudioRouter Cable A
     Input** → **Test**, a few times: the chimes go into Cable A and are
     recorded into `render-source.wav` in the evidence.
   - At the end the step prints the stream counters. Pass: all of
     underrun, overrun, gaps, non-finite and format mismatches are 0. (The
     tool paces itself on the driver's acknowledgements, so a clean run
     really should be 0; anything else is a finding.)
5. `tone-stall` — the tool pauses for half a second on purpose. Pass: the
   counters are **above** 0 (the driver noticed the gap).
6. `tone-8ch` — the same at 96 kHz with 8 channels.
7. **Silence check (A6):** after the tone steps, with nothing running, the
   Listen output must be completely silent.
8. Optional: open **Sound Recorder** (Start → Sound Recorder), choose
   **AudioRouter Cable B Output** as the microphone, start `tone` again and
   record 30 seconds; save it into `C:\ar\evidence`. If Sound Recorder sees
   no microphone, allow it in Settings → Privacy & security → Microphone.
9. `collect`, copy the zip to `Z:\`.

### Session 3 — Cable count and names · 15 min · checks A15 (count), VCAB-02/05

1. `preflight`, `install`.
2. `cables` — switches to 8 cables (16 endpoints, all formats), back to 2,
   and checks Cable A and B kept their endpoint IDs.
3. `rename` — renames cable 2 to "Discord" and checks its endpoint IDs did
   not change. In the Sound panel the endpoints now show "Discord".
4. `remove` — uninstalls with the helper.
5. `collect`, copy the zip.

### Session 4 — Driver Verifier and hostile input · 50 min · checks A10, A11 (fuzz)

1. `preflight`, `install`.
2. `verifier-on`, then `Restart-Computer`. Sign in again, open PowerShell
   as administrator, `cd C:\ar`.
3. `fuzz` — 30 minutes of random, malformed and racing requests while
   Windows' Driver Verifier watches the driver. Pass: no blue screen, no
   unexpected result, and the run opened real leases (the tool refuses to
   pass if it only ever got rejections).
4. `verifier-off`, then `Restart-Computer`.
5. `collect`, copy the zip.

A blue screen here is a real driver bug and the most valuable result this
session can produce: after the restart run `collect` and send it.

### Session 5 — A second Windows user (A11) · 15 min

This proves one Windows user cannot take over a cable another user is using.

1. `preflight`, `install`.
2. Create a second account (administrator PowerShell):
   `net user artest2 Test-Pass-2 /add`
3. In a **normal** (not administrator) PowerShell as `artest`, start a long
   tone: `C:\ar\tools\m03_bridge_tone.exe --seconds 600 --out C:\ar\evidence\user1.wav`
4. Switch user (Start → your account picture → **artest2**; password
   `Test-Pass-2`). In a normal PowerShell as artest2 run:
   `C:\ar\tools\m03_bridge_tone.exe --seconds 10 --out $env:TEMP\user2.wav`
5. Pass: it stops with **"AccessDenied: This cable is in use by another
   Windows user."** Copy that text into a file `C:\ar\evidence\a11.txt`
   (switch back to artest, administrator PowerShell, then `collect`).

## Part 6: Sending the results

After `collect` and `Copy-Item C:\ar\evidence-*.zip Z:\`, the zip is in
`C:\VMs\ar-share` on your PC. Tell the developer agent which session it was and
the zip's name (for example "Session 2, `C:\VMs\ar-share\evidence-20261007-201500.zip`").
The agent unpacks it, checks every number, records it in
`docs/plans/active/evidence/`, and fixes what failed. The evidence contains
device names, endpoint IDs and test tones only, never a microphone
recording.

## Part 7: When something goes wrong

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| VirtualBox: "VT-x is not available" or "VERR_NEM_…" | Windows Hypervisor Platform not enabled | Part 1.2, then restart the PC |
| Windows setup: "This PC can't run Windows 11" | TPM/EFI off or too little memory | VM Settings → System: EFI ticked, TPM v2.0, 8192 MB |
| Setup insists on a Microsoft account | Recent Windows 11 builds | Sign in with a Microsoft account (fine for a test VM), or at the network screen press Shift+F10 and type `start ms-cxh:localonly` |
| No `Z:` drive in the VM | Guest Additions missing or folder not auto-mounted | Part 3 step 4; check Settings → Shared Folders; or use `\\vboxsvr\ar-share` |
| `bcdedit` says the value is protected by Secure Boot policy | Secure Boot still on in the VM | Shut down, Part 4.3, start again |
| No "Test Mode" text after restart | `bcdedit` did not apply | Run `bcdedit /enum {current}`: `testsigning Yes` must be listed |
| `preflight` FAIL on the certificate | Certificate not imported in both stores | Repeat Part 4.4 step 4 |
| A script says "Refusing driver smoke test outside AR-DriverTest" | Wrong computer name | Rename the VM to `AR-DriverTest` (Part 3 step 3), or create the marker file `C:\ar\IS_TEST_VM` **inside the VM only** |
| Helper exit code 2 | Package not trusted or not test-enabled | Run `preflight`; never copy the helper from another build than the share |
| Device Manager shows the device with Code 52 | Signature not accepted | Test Mode off or certificate missing; see the two rows above |
| Device Manager shows Code 10 or the endpoints never appear | Driver failed to start | Run `collect` and send it; it contains `setupapi.dev.log` |
| Blue screen | Driver bug | Let it restart, run `collect`, send the zip |
| Stuck on a black **"Restarting"** screen; the spinner dots do not move and the disk icon stays dark for several minutes | Windows hung while restarting (seen after an update) | **Machine → Reset**. Windows resumes or rolls back the update on the next start; let it finish |
| Popup "It looks like you started an upgrade and booted from installation media" | The install disk is still in the VM's DVD drive | **Never click No** (it starts a clean install that erases Windows). Eject the disk (Part 3 step 2), check the drive is empty, then click **Yes** |
| Firmware menu after "Boot failure"; **Windows Boot Manager** returns straight to the menu | An upgrade was started from the install disk and the disk is now gone | Do not repair. Restore the last snapshot, eject the disk before starting, untick Secure Boot again if that snapshot predates 4.3, then cancel the update if it resumes |
| **"Installing Windows 11 — xx%"** does not move | A feature update installing; slow in a VM | Wait while the disk icon flickers (up to 15 minutes per percentage). Only if it stays put for 30+ minutes with a dark disk icon: **Machine → Reset**, or restore the last snapshot |
| VM very slow, turtle icon | Running through Windows' hypervisor | Expected; give the VM 4 CPUs and 8 GB, close other heavy apps |

When in doubt: restore snapshot 2 and start the session again. Nothing in
the VM can harm the PC.

## Part 8: Updating to a new driver build

When the developer agent changes the driver, refresh the files without
redoing Parts 2–4:

1. On the PC, empty `C:\VMs\ar-share` (keep your evidence zips somewhere else
   first) and run `tools\vm\prepare-vm-share.ps1 -Share C:\VMs\ar-share` again.
2. Restore snapshot 2, start the VM, and in administrator PowerShell:
   `robocopy Z:\ C:\ar /MIR /XF evidence-*.zip` then
   `Get-ChildItem C:\ar -Recurse | Unblock-File`.
3. Run `preflight`. If the test certificate changed (rare: only if the PC's
   test certificate was recreated), repeat the two `Import-Certificate`
   lines from Part 4.4, restart, and take a new snapshot 2.
4. Optional: shut down and take a new snapshot (for example
   `03-build-YYYYMMDD`) so later sessions start with the new files.

## Part 9: Removing everything

- **The VM:** VirtualBox → right-click `AR-DriverTest` → **Remove** →
  **Delete all files**.
- **VirtualBox:** Settings → Apps → Installed apps → Oracle VirtualBox →
  Uninstall.
- **Windows Hypervisor Platform:** `optionalfeatures`, untick it, restart
  (only if nothing else on the PC uses it, for example Android emulators).
- **The test certificate on the PC** (optional): it lives in your user
  certificate store (`certmgr.msc` → Personal → Certificates →
  "AudioRouter Test Driver"); it was never trusted on the PC.

## Appendix: Hyper-V instead of VirtualBox

Windows 11 **Pro** includes Hyper-V. If the PC is upgraded to Pro, the
original commands in the [test procedure](virtual-cable-testing.md#one-time-host-setup-main-pc-administrator-powershell)
apply (Generation 2 VM, `Set-VMFirmware -EnableSecureBoot Off`, checkpoints
instead of snapshots, `Copy-VMFile` instead of a shared folder). Parts 4.4–9
of this guide are the same inside the VM. Use one VM product at a time.
