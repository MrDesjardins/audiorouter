# Optional native VT-x session for sustained cable tests (and how to undo it)

This is an **optional, user-run** procedure for the development PC. Nothing
here is done by an agent, and the test-signed driver still runs **only inside
the AR-DriverTest VM**. Use it only for the long continuity runs (VCAB-24) that
cannot pass while VirtualBox runs through the Windows Hypervisor Platform.

## Why

With WSL 2 kept on, Windows starts its hypervisor at boot. VirtualBox then
cannot use VT-x directly and falls back to the Windows Hypervisor Platform
("NEM"; `VBox.log`: `HM: HMR3Init: Attempting fall back to NEM: VT-x is not
available`). Traces from 2026-10-09/10 show two NEM effects that break
real-time audio in the guest: periods in which the whole guest records no
event at all, and late timer delivery to every audio thread at once while the
guest is busy ([analysis](../plans/active/evidence/2026-10-10-m03-render-commit-validity.md#traced-300-second-run-2026-10-10-failed-cause-attributed)).
Starting Windows once **without** its hypervisor lets VirtualBox use VT-x.

## What changes during the session, and what does not

State checked read-only on 2026-10-10: hypervisor present; Virtualization-
Based Security running; Memory Integrity (HVCI) configured and running, not
UEFI-locked; Credential Guard not configured; WSL default version 2 (Ubuntu).

| While the hypervisor is off | After you restore it |
| --- | --- |
| WSL 2 distributions (Ubuntu) **cannot start**. Docker Desktop/WSL-based tools stop working. | WSL 2 works again |
| Memory Integrity and Virtualization-Based Security are **not running**; Windows Security may warn about it | Both run again (they stay configured on) |
| VirtualBox uses VT-x natively | VirtualBox uses NEM again |

Nothing is uninstalled: the WSL, Virtual Machine Platform and Windows
Hypervisor Platform features stay installed; only one boot setting changes.
Secure Boot, test signing and the driver on this PC are **not** touched.

## Before you start (once)

1. Make sure WSL work is saved and closed: `wsl --shutdown` in a terminal.
2. **BitLocker:** an agent could not read its state without administrator
   rights. In **Administrator PowerShell on the host**, run
   `manage-bde -status C:`. If *Protection Status* is *Protection On*, make
   sure you can see your recovery key at <https://aka.ms/myrecoverykey> (or
   have it printed) before changing any boot setting.
3. Record the current boot entry so the restore can be verified
   (Administrator PowerShell, **host**):

   ```powershell
   New-Item -ItemType Directory -Force 'C:\VMs\ar-share\host-state' | Out-Null
   bcdedit /enum '{current}' | Out-File 'C:\VMs\ar-share\host-state\bcd-before.txt' -Encoding utf8
   Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard |
       Format-List VirtualizationBasedSecurityStatus,SecurityServicesConfigured,SecurityServicesRunning |
       Out-File 'C:\VMs\ar-share\host-state\vbs-before.txt' -Encoding utf8
   ```

   `bcd-before.txt` should show `hypervisorlaunchtype    Auto` (or no such
   line, which also means the default *Auto*). If it shows anything else,
   stop and note it: restore to that value instead of *Auto* below.

## Turn the hypervisor off for the test session

Administrator PowerShell on the **host**:

```powershell
# Only if BitLocker protection is on: suspend it for exactly one restart.
Suspend-BitLocker -MountPoint 'C:' -RebootCount 1
bcdedit /set hypervisorlaunchtype off
```

Restart the PC. Then confirm (normal PowerShell is enough):

```powershell
(Get-CimInstance Win32_ComputerSystem).HypervisorPresent   # expect False
```

Start AR-DriverTest. On the host, its new log must show VT-x and no NEM:

```powershell
Select-String 'C:\VMs\AR-DriverTest\Logs\VBox.log' -Pattern 'HMR3Init|NEM' | Select-Object -First 5
```

Expect a line like `HM: HMR3Init: VT-x w/ nested paging ...` and no
`fall back to NEM`. If NEM still appears, do not run tests; restore below.
Then run only the long tests the agent gives you.

## Restore the hypervisor (bring WSL and Memory Integrity back)

Administrator PowerShell on the **host**:

```powershell
# Only if BitLocker protection is on: suspend it for exactly one restart.
Suspend-BitLocker -MountPoint 'C:' -RebootCount 1
bcdedit /set hypervisorlaunchtype auto
```

Restart the PC, then verify:

```powershell
(Get-CimInstance Win32_ComputerSystem).HypervisorPresent   # expect True
Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard |
    Format-List VirtualizationBasedSecurityStatus,SecurityServicesRunning   # expect 2 and {2}
wsl -l -v                                                    # Ubuntu, VERSION 2
wsl -d Ubuntu -e uname -r                                    # prints a kernel version
```

Optionally compare `bcdedit /enum '{current}'` (Administrator) with
`bcd-before.txt`: the `hypervisorlaunchtype` line should match again. Windows
Security → Device security → Core isolation should show Memory integrity
**On**. BitLocker protection resumes by itself after the counted restart
(`manage-bde -status C:` shows *Protection On*).

If anything does not come back, run the restore block again and restart;
it is the same single setting. VirtualBox returns to NEM automatically.
