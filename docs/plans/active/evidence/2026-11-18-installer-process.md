# Windows installer process evidence

Source checkout started at `33cdf58d4f577258e9d372c58ac5d7d3230f30f4`.
The runner changes were local. Host Windows build 26300.9974, VirtualBox 7.2.20.
The disposable release VM and snapshot UUIDs, artifact SHA-256 values, and
remaining acceptance cases are recorded in the [execution plan](../delegate-release-qualification.md).

The injected test verified spaces and a trailing backslash in an argument,
expected and fatal exit 23, a missing executable, a timeout with exit confirmed
for the root and its known child, survival of an unrelated process, and
successful execution after timeout. Evidence directory
`%TEMP%/ar-process-74c99f76-6823-4382-a606-95bf795af328`.
This run also verified single-process timeout cleanup and that filtered
command output was removed from the evidence directory.

Historical attempt `%TEMP%/ar-rq02-historical-a71cc8bc-4a19-46b1-81f0-e26676a7c4e8`
used 0.0.30 from `20503e066` and 0.0.29. Restore, start, guest readiness,
profile lookup, staging, four copies, silent install, session import, repair,
rollback, upgrade, uninstall, reinstall and graceful shutdown passed. Guest
lifecycle took 38.3 s. The retention fixture was a generated WAV. This is one
historical lifecycle result for DIST-01/02/08 and NFR-16. Current candidate,
production recording, WebView2 failure and recovery, and interactive trust
qualification remain open.

A second historical attempt,
`%TEMP%/ar-rq02-historical-b25cb901-9cdd-4017-baea-bc46536d9c78`,
passed and shut down cleanly. It also checked that all five candidate
companions were present, removed the x64 plugin worker before repair, and
confirmed it was restored with the original SHA-256. The guest lifecycle
took 36.2 s. Both historical installer hashes match their retained app-only
manifests. Previous source was `f047279a9037b3a35b4856b5666c2949c2c8dc76`.

Third historical attempt
`%TEMP%/ar-rq02-historical-c8bad6f9-287a-42f3-acb4-95dae411efde`
passed with the same harness as the second. All three historical runs passed,
with the last two exercising the companion repair case. The earlier
intermittent hang was not observed in these attempts. Three consecutive passes
on the same harness and injected timeout recovery inside the VM workflow are
still open at that point.

Fourth historical attempt
`%TEMP%/ar-rq02-historical-d950f616-ab0b-4bdd-9720-c0dd9b7d7b9a`
passed and shut down. It closed three consecutive passes on the same
harness revision and historical installer pair.

End-to-end timeout experiments:

- `%TEMP%/ar-rq02-historical-c9b8c6ef-a084-429e-8a6c-b936f67de04c`:
  PowerShell probe exited 1 after 1685 ms. Shutdown passed.
- `%TEMP%/ar-rq02-historical-d95feba7-7d07-4952-a2e0-23c72d836acb`:
  direct PING probe exited 1 after 1295 ms without a marker.
- `%TEMP%/ar-rq02-historical-936e89ef-422e-40b5-822e-c2a2744a30fc`:
  cmd probe timed out after 6101 ms and VBoxManage exit was confirmed.
  The marker was unconfirmed and ACPI shutdown did not reach poweroff.
  Preflight then refused the running VM (attempt `b33434f1`). The guest
  remains running for inspection with no logged-in-users property.
  A marker, graceful shutdown, and clean recovery remain unverified.

Invalid installer SHA-256 and the excluded driver VM UUID both returned
exit 1 before launching VBoxManage. Historical 0.0.30 passed
`tools/release/verify-artifacts.ps1` with its retained manifest.

The new WAV check accepted a known tone and rejected silent and truncated
fixtures (`%TEMP%/ar-wav-check-b2c28b7c-61b8-4905-a97c-331929df1253`).
The installed recorder route and present/absent WebView2 guards need VM
execution. No verified runtime-absent baseline is registered. Interactive
trust and offline bootstrap recovery remain open.

Synthetic follow-up checks passed on Windows PowerShell:

- `tests/acceptance/m08-lifecycle-guards.ps1` exercised snapshot UUID parsing,
  existing staging preservation with a real local cmd process, guest profile
  refusal decisions, fixture runtime registry rows, fragmented RPC responses,
  RPC error and correlation checks, cancellation, best-effort cleanup, and
  virtual-link operation ordering with injected disconnect/reconnect callbacks.
- `tests/acceptance/m08-lifecycle-recording.ps1` verified 48 kHz mono PCM16,
  duration, 440 Hz tone and level bounds, and rejected silence, wrong
  frequency, wrong rate, short duration and truncation.
- Nine touched lifecycle PowerShell scripts parsed. These are host-side
  checks. The sick release VM was preserved and received no further changes.

The offline classification regression covers zero and nonzero exits with a
missing runtime, registration and app-file leftovers, an initialized
database, a registered runtime and a usable leftover shell. A clean nonzero
exit permits a diagnostic same-guest retry. The current installer interface
supplies no verified bootstrap-specific failure signal, so that outcome
remains unattributed even when the online retry passes. The test also
round-trips simultaneous setup and reconnect failures through JSON.

## Preserved guest recovery and baseline defect (2026-11-18)

- The stuck guest from attempt `936e89ef` was in Windows Recovery (Automatic
  Repair failed), so guest control and ACPI shutdown could not work. Choosing
  "Turn off your PC" in its console menu reached `poweroff` without a forced
  power-off.
- Timeout attempt `%TEMP%/ar-rq02-timeout-0b602f46-56a3-4415-9895-f71fa06310d2`
  restored `lifecycle-ready-20261116` and shut down gracefully, but every
  guest-control session failed at once (exit 1 after about 1.3 s, not a
  hang). The guest log shows `Failed to accept session N: VERR_NOT_FOUND`, so
  the saved-state baseline's guest-control service is stale after restore.
  Rebooting the guest from inside restored guest control. This is a baseline
  defect, not a harness timeout, and it is why that run could not reach the
  marker. The earlier three passes used the same snapshot, so the defect is
  intermittent or appeared later.
- That baseline also had a second interactive user (`artest`, expired password
  at every boot). The guest holds leftover accounts `arqual`, `arqual2`,
  `arqual3` and `arrelease`.
- A live snapshot of the cold-booted guest (`lifecycle-ready-20261118`) writes
  about 1 GB of memory in 10 minutes while the owner is using the host. The
  guest clock lagged by minutes (VirtualBox `TM: Giving up catch-up attempt`) and
  VBoxManage calls on that VM blocked behind the snapshot. Host contention is a
  plausible cause of the intermittent restore hangs. Prefer an offline
  baseline, and run VM cases when the host is quiet.
- Mistake recorded for honesty: I sent `controlvm poweroff` to the disposable
  guest once while meaning to snapshot it. Only that guest was affected.

## Final harness runs (2026-11-18, evening)

Baseline `lifecycle-offline-20261118`; wrapper signs in at the console. Timeout
case `ar-rq02-timeout-327fe687-1eda-4d12-83dc-5e667c0a5b6c` reached its marker
and shut down gracefully. Clean passes with `-ProductionRecording`:
`ar-rq02-clean-627dd10c-2354-46f1-9980-a689e4f7336e`, `-3d3a3b62-fa33-450d-ad83-8ce5ca7401cc`,
`-eb7f7c7b-508e-4649-bd85-ea70589164a5`. Recorder output: 440 Hz, stereo, 48 kHz,
RMS 0.089; the same SHA-256 after repair, rollback, upgrade, uninstall and
reinstall. A run where the 180 s guest deadline expired (`ar-rq02-clean-3920a3e8`)
failed honestly with VERR_TIMEOUT and motivated the 400 s deadline. Combined
candidate: `ar-rq06-combined-8aec0750` and `-7db05993` passed. Wrong snapshot
refusal: `ar-rq03-wrongsnap-1049c80e`. An all-zero recording from the first
installed-recorder run showed that a Test Signal must be played before recording.

## RQ-05 baseline attempt (2026-11-19)

- Probe in the guest: WebView2 157.0.4341.46 is machine-wide
  (`HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients`, plus 154.0.4258.62 on
  disk). Removing it needs an administrator. The only administrator, `artest`
  (documented in the VM guide), is refused by guest control as "restricted"
  (expired password), and `arqual4` is a standard user by design.
- Resetting it at the console was tried: a lock via `LockWorkStation` worked, but
  keyboard-only account selection opened the power menu instead. VirtualBox
  headless has no mouse control, so the attempt was stopped. No password or
  account was changed.
- After that the guest ignored two ACPI power-button requests for about 25
  minutes while locked. With the user's approval I terminated only that guest's
  VBoxHeadless (PID 53220, matched through its `VBox.log`), restored
  `lifecycle-offline-20261118` and confirmed `poweroff`. `discardstate` reported
  no saved state. No other VM or snapshot was touched.
- RQ-05 stays open. Ways forward: set a known `artest` password by an interactive
  console session (GUI VirtualBox, with a mouse) and take a WebView2-absent
  snapshot (`setup.exe --uninstall --msedgewebview --system-level --force-uninstall`),
  or build a fresh guest from a Windows ISO.
