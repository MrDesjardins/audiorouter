# Virtual cable: guest sign-in probe on AR-DriverTest (2026-11-25)

Scope: one sign-in probe only, under the manager's limited VM slot. No Stage A
block, no driver install or load, no measurement, no password change, no snapshot
created, overwritten or deleted. Host was not idle (about 28-38 % non-VM CPU and
WSL running), so no block was run. Plan: [virtual cable](../virtual-cable.md);
[review fixes](../virtual-cable-review-fixes.md).

## What was checked

- `VBoxManage list runningvms`: empty. `AR-DriverTest` was `poweroff` with the
  current snapshot `02-test-signing-ready` (UUID
  `69db618a-42e4-4cf4-8428-403aed970c29`), 8192 MB, 4 vCPUs.
- `VBoxManage startvm AR-DriverTest --type headless`: started.
- Guest Additions are installed and running: `/VirtualBox/GuestAdd/Version`
  `7.2.20`, Windows 11.
- Unattended sign-in with already-configured means: **it does not work.**
  `/VirtualBox/GuestInfo/OS/LoggedInUsers` was `0` at 31 s and again at every
  60 s check for five minutes (1, 2, 3, 4, 5 min); `LoggedInUsersList` stayed
  unset. A screenshot was captured
  (`C:\VMs\ar-share\evidence-20261125-signin-probe.png`) but not interpreted
  here; the guest property is the decisive datum: no interactive user session
  existed.
- Guestcontrol is available for credential-free queries: `guestcontrol list
  sessions` returns "No active guest sessions found". `guestcontrol run`/`stat`
  need a guest user name and password; none is known, and guessing or changing
  the password was out of scope.

## Reading

The snapshot's `artest` password expired after the snapshot date; the
2026-11-22 run recorded that every restore of `02-test-signing-ready` prompts for
a password change at first sign-in. Auto-logon needs a valid stored password, so
it cannot bootstrap this restore, and the runner's `-AutoLogon` prompt itself
needs a signed-in operator to type and validate the password. Therefore a person
must sign in once per restore (set a run-only guest password, as on
2026-11-22), after which `-AutoLogon` covers the planned restarts of that run.
`guestcontrol` would need credentials configured for the guest, which is an
owner decision, not a guess.

Consequence for the gates: the round-2 VM block (HG-1 to HG-4) is agent-run only
after that first sign-in; until then it is a human step. No driver or
measurement evidence was produced or claimed by this probe.

## Cleanup (verified)

`controlvm AR-DriverTest poweroff`, then `snapshot AR-DriverTest restore
02-test-signing-ready` (progress 0-100 %, snapshot UUID reported). Read-only
verification afterwards: `VMState="poweroff"`,
`CurrentSnapshotName="02-test-signing-ready"`, `CurrentSnapshotUUID` unchanged
(`69db618a-...`), snapshot tree unchanged, `list runningvms` empty. Only this
probe's own boot changes were discarded; no snapshot was taken, changed or
deleted, and the account was not modified.

## Not run

Any Stage A block, driver install/load, Driver Verifier, tone, pool, latency or
other measurement; `wsl --shutdown` or any change to the owner's apps; any
snapshot edit.