# Driver soak: start, pause, stop

Collects clean hours for the virtual cable driver in the AR-DriverTest VM. The PC gets
no test driver. Run from the repository in any terminal on the PC:

| Command | What it does |
| --- | --- |
| `tools\vm\soak.cmd start -Until 15:00` | Start or resume; pauses by itself at 15:00 |
| `tools\vm\soak.cmd pause` | **I need the PC now.** Freezes the VM, then saves it to disk and frees its 8 GB |
| `tools\vm\soak.cmd status` | Clean hours, failures, what was not counted |
| `tools\vm\soak.cmd stop` | End the campaign and shut the VM down |

Options for `start`: `-Routes 2` (default, lightest) or `4`; `-Minutes 20` per segment;
`-AllowWsl`.

## First time only

1. Finish [D1](../plans/active/human-gated-tasks.md): sign in to the VM and set its password.
   Take no snapshot and restore none while a campaign is running.
2. Close games, browsers and AI apps; run `wsl --shutdown`. `start` needs about 14.5 GB
   free (8 GB for the VM, 6 GB that must stay free) and refuses otherwise.
3. `tools\vm\soak.cmd start -Until 15:00`
4. In the VM, PowerShell as administrator:
   `powershell -NoProfile -ExecutionPolicy Bypass -File Z:\rc-0.2.5-soak\soak-guest.ps1`.
   If it asks for a restart, restart the VM and run it again. At "Ready", leave it open.

Later starts need only step 3: a saved VM resumes with the runner still open. After `stop`,
a PC restart while running, or a VM crash, repeat step 4. **Pause before you shut down or
restart the PC.**

## What counts

A segment adds its delivered audio time only if it is clean, the PC was measured idle
throughout, the guest kept real time, the loaded driver is the expected one, and it was not
paused. The interrupted segment is dropped, so a pause costs at most 20 minutes. Losses in
a guest-wide stall and busy-PC segments are `invalid`; corrupted audio, unexplained losses
and crash dumps are `fail`; anything the test could not finish is `review`; a silent VM is
an `incident` and is never reset automatically. With any failure, review or incident, or
above 20 % invalid, no reliability claim is printed. Hours are kept per driver image and
load; a new driver starts at zero, and drivers before 0.3.0 are marked REHEARSAL.

The PC stays awake while the campaign runs (no power setting is changed); closing a laptop
lid or a Windows Update restart still interrupts it. Data: `C:\VMs\ar-share\soak`.

With two routes a stall needs only two processes to coincide, so more events become
`invalid` than with four. The 20 % rule is the guard.

## First night: check before trusting the hours

The scripts are tested on the PC with a fake VM; these points need the real one.

1. `status` shows "heard" under 30 s ago.
2. After about 25 minutes one segment is recorded for driver `D2C26DD92C00`, with a reason.
3. `pause` in the middle of a segment, then `start`: that segment must say `paused`, not
   `fail`. If not, the share served an old file after the restore; stop and tell the agent.
4. The PC is fully usable after `pause` (the VirtualBox window is gone).
5. If every segment is invalid, failed or in review, stop: the VM cannot produce these hours.

## For agents

Rules: `soak-support.ps1`. Guest loop: `soak-guest.ps1`. Bundle:
`prepare-soak-bundle.ps1` (`-AllowDirty` is for rehearsals). Tests, no VM needed:
`soak.tests.ps1` and `soak-guest.tests.ps1`.
