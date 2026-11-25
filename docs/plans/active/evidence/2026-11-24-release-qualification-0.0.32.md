# Release qualification, 0.0.32 bytes and remaining human gates (2026-11-24)

Local agent session. No push, PR, tag, draft, release or publication action was
performed. The owner authorized VM use for this session, capped at four guest
CPUs, with minimal Defender exclusions and a requirement to restore the prepared
baseline afterwards.

## Identity

| Item | Value |
| --- | --- |
| Candidate | `AudioRouter_0.0.32_x64-setup.exe`, 24,472,180 bytes, SHA-256 `50bcdfc1f16e7ce4c8a04a6a73387c709b0d148d9aa00481cc3c58a86db7ceed` |
| Candidate source | `0af71ca715333ae6c08c6e58c809f8b1f7bf514e` (consolidated 0.0.32, [release plan](../release-0.0.32.md)); no tag exists |
| Candidate path | `C:\code\audiorouter\target\releases\0.0.32-consolidated-preserved-0af71ca71\artifacts\` |
| Previous | 0.0.29, SHA-256 `f2c43d115f04fdef00f55ef96862eba41d581464db24785cde5f69755ba209d8`, 23,083,014 bytes |
| VM | `AR-ReleaseQualification-20261116`, UUID `87c31665-2022-49a2-a182-1a1ebb5e631c`, 4 CPUs, 8 GiB, audio `none`, guest Windows 11 |
| Baseline snapshot | `lifecycle-offline-20261118`, UUID `45d20e3c-8066-4896-9e28-a206f9d88723` |
| Safety snapshot taken first | `pre-agent-rq-20261124`, UUID `ff45d732-0ab1-4761-8859-4f453cc41934` |
| Account | `arqual4`, standard user; host-only credential file `%TEMP%\ar-vm-qualification-password.txt` |
| Host | Windows 11 build 26300, VirtualBox 7.2.20 |

The guest was **not** the fresh 2026-11-21 guest. That guest's standard-user
password is not on the host, so its console cannot be signed in (human gate
below). The credential that exists belongs to `arqual4` on the older release
guest, which is the guest the 2026-11-18 lifecycle evidence used.

Deviation: previous version is 0.0.29, not 0.0.31. Microsoft Defender on the
host blocks reads of the retained v0.0.31 installer
(`C:\Users\miste\AppData\Local\Temp\ar-rq01-v0031\AudioRouter_0.0.31_x64-setup.exe`,
threat `2147927549`, detected 2026-11-24 18:22). Its published digest
`b94e52764b31fc4c237c688c49d1b1fc80e3de9799dc09e90b5093e077543a92` matches
`SHA256SUMS.txt` in that folder and the GitHub release asset. No Defender
exclusion was added: adding one needs elevation, and elevation was not worked
around. Rollback therefore exercised 0.0.29, the same previous fixture used for
the v0.0.31 runs.

## Defender state and exclusions

- No exclusion was added on the host or in the guest during this session.
- Host exclusion list could not be read (`Get-MpPreference` needs elevation);
  guest exclusion list could not be read as a standard user. No exclusion was
  added, so nothing was removed afterwards.
- The candidate 0.0.32 installer was **not** quarantined in the guest in the
  lifecycle run. The v0.0.31 installer remains blocked on the host (above), so a
  clean-Defender result for v0.0.31 bytes is still open.
- The guest baseline is known to have had a staging-folder exclusion added
  during the 2026-11-21 owner-attended session. Whether that exclusion survives
  in `lifecycle-offline-20261118` could not be verified without elevation. The
  0.0.32 run below is therefore not claimed as a no-exclusion Defender result.

## RQ-04: full lifecycle on 0.0.32 (passed, these bytes only)

Ran `tools/vm/installer-lifecycle.ps1` with `-ProductionRecording
-WebViewBaseline Present`, after manually signing `arqual4` in at the guest
console (see the sign-in gate). Output:
`%TEMP%\ar-rq-20261124-lifecycle-ba393b5eed604d3d9521426a8ebd2de3`.

Every step passed: `preflight` (784 ms), `webview-baseline`, `fresh-install`
(6,781 ms), `webview-after-setup`, `seed-data` (12,182 ms, production
recording), `repair` (8,952 ms), `rollback` to 0.0.29 (6,547 ms), `upgrade`
(5,627 ms), `uninstall` (2,385 ms), `reinstall` (5,197 ms). WebView2 was
`157.0.4341.46` before and after.

Production recording through the installed desktop bridge and production
Recorder: 440.0 Hz, stereo, 48 kHz, 97,152 frames, RMS 0.0890, SHA-256
`8592C312624D0FBF8BAC94F562A1FBB4AD833AFF88064915E8407634253DE011`, retained
through repair, rollback, upgrade, uninstall and reinstall. The imported
`session-fixture` was readable through the installed CLI after every step.

This is the first installer-lifecycle result for the merged 0.0.32 candidate. It
does not transfer to a tagged or rebuilt 0.0.32, and it does not close the
published v0.0.31 case.

## RQ-02: injected guest timeout and marker (passed logic, manual method)

The unattended wrapper itself could not be used: restoring
`lifecycle-offline-20261118` cold-boots to a lock screen whose Winlogon
`DefaultUserName` is `artest` with an expired password
(`hasDefaultPassword: false`), so the wrapper's wake-plus-password sequence
types into `artest` and fails three times. Evidence of that failed wrapper
attempt: `%TEMP%\ar-rq-20261124-timeout-f8616933071747bfa3d91e8a9f2995db`
(`console sign-in did not complete`, graceful shutdown passed, no guest change).
OCR of the console showed `artest - The passwords you entered did not match` and
then artest's expired-password form.

With `arqual4` signed in manually (Escape from the expired-password form,
then the arqual4 tile), the harness's own `Invoke-LifecycleProcess` ran the
injected case: warm-up passed, the hung guest command was cut off at **12.0 s**
(bound 12 s), the guest marker `AR_QUAL_TIMEOUT_REACHED` was reached
(`marker: true`), and no `VBoxManage` process survived. Output:
`%TEMP%\ar-rq-20261124-timeout-manual-d55e9790c8cc4503ae06f8e0d4bf7a1c`.

Deviation: this is the documented manual method from 2026-11-21, not the
wrapper's restore/start/sign-in/shutdown path. The wrapper's new result field
`timeoutMarkerReached` was added and is covered by a host regression, but its
cold-boot path needs a baseline the wrapper can sign in to.

## RQ-05: installed offline operation (partial in the first session; passed in the follow-up)

Guest network was disconnected with `VBoxManage controlvm setlinkstate1 off`
(verified `cableconnected1="off"`). Script result:
`%TEMP%\ar-rq-20261124-offline-85c928db69fc4e02a89f4a5cb9286ad5.json`.

Passed offline:

- no DNS resolution and no TCP reachability;
- the installed folder holds all six executables and reports version `0.0.32`;
- no shell process was running before or after (no automatic startup audio);
- `audiorouter-cli.exe status --json`, `schema --json`, `session list` (3
  sessions) and `session get session-fixture` (id and name checked);
- `api call system.describe` through the CLI (protocol major 1);
- MCP stdio `server/discover` with protocol metadata `2026-07-28` through an
  enrolled client from `clients.list`, answered with the modern result envelope;
- `Win32_SoundDevice` was empty before and after: this guest has no endpoints,
  so default-device and volume checks have nothing to compare and are **not**
  claimed.

Failed: the production-recording step could not start the retained
`qualification-recorder` session. `sessions.play` returned `state: running`, but
`sessions.summary` immediately reported `playing: false`, and
`audioSources.transport` returned `-32602` "start the prepared session before
controlling its audio source". The same failure reproduced with the guest
network **reconnected**, so it is not an offline-specific defect. The lifecycle
run above recorded successfully *before* uninstall/reinstall; the failure
appears in the post-reinstall state when the helper drives the RPC directly.
The retained session still parses and the CLI reads it. Likely cause is that the
fresh-install path prepared the route implicitly while a later launch needs the
UI Play/Prepare step; this is recorded as an open finding, not a product verdict,
and no product code was changed. Consequence: the offline **generated
recording** sub-case is not qualified; it needs either the UI Play action (a
person) or a harness step that prepares the retained route.

**Corrected by the follow-up below:** the cause was a replayed idempotency
result, not route preparation. The harness and the control plane were fixed,
and offline operation (including the recording) then passed on the same bytes.

## Cleanup

- Guest NAT link reconnected before shutdown.
- Guest shut down with ACPI power button, reached `poweroff`, then
  `snapshot restore lifecycle-offline-20261118`.
- Both release guests are powered off at their prepared baselines:
  `AR-ReleaseQualification-Fresh-20261121` is at `offline-standard-user-20261121`
  (4 CPUs), the older guest is at `lifecycle-offline-20261118`.
- New snapshots `pre-agent-rq-20261124` remain on both VMs; no existing snapshot
  or VM was deleted or overwritten.
- No `VBoxManage`, `VBoxHeadless` or `VirtualBoxVM` process remained.

## Not run

- Interactive downloaded-file unsigned-publisher prompt, shortcuts and first-run
  usability on 0.0.32, endpoint vendor guidance with real endpoints, and
  default-device/volume comparison on a machine with devices.
- Absent-WebView2 baseline on the fresh guest (the absent snapshot is on the
  older guest, whose credential is missing; the fresh guest needs its admin
  password to remove the runtime).
- Defender verdict for v0.0.31 (Microsoft submission pending) and any signed
  artifact.
- A second supported Windows build, real audio listening, endurance and the
  broader M08 gates.
- Hosted CI for this branch (not pushed).

## Follow-up: post-reinstall recorder failure, absent runtime, wrapper sign-in (2026-11-24, second VM session)

### Root cause: a replayed idempotency result outlived its runtime

The earlier failure was not offline-specific and not a recording bug. The
recorder helper called `sessions.play` with the **fixed** key
`qualification-play`. Idempotency results are journaled to the database
(`operation_journal`) and survive a control-plane restart, while the runtime map
does not. On every later shell launch the same request replayed the journaled
"running" result without starting anything: `sessions.summary` reported
`playing:false` and `audioSources.transport` refused. The first lifecycle run
worked only because that key had not been used yet.

Reproduced on the host through `ControlPlane::dispatch` with a file database and
the real `tests/fixtures/installer-recorder-session.json`: play, restart the
control plane, replay the same request returned the cached `running` while the
summary said `playing:false`. The ten-case suite in
`crates/control/src/recorder_play_tests.rs` encodes the accepted rules; nine of
its cases failed on the re-executing revision (`42538818e`) as recorded in the
product bullet below.

Two defects, both fixed:

- **Product (owning layer, revised 2026-11-24 after review):** a journaled
  runtime outcome must not be re-executed. Within one backend lifetime a
  same-key retry now replays the recorded result verbatim, even after the route
  was stopped or started under another key. A result found only in storage came
  from an earlier lifetime (`lifetime_operations` records every key this process
  journaled; startup never reloads rows into the in-memory cache), so a
  play/start/toggle claiming `running`, a recorder claiming `recording`, or a
  mute claiming a different latch is refused with `idempotencyConflict` and
  "AudioRouter restarted since this request …"; the journal row stays and a new
  request is required. Stop outcomes replay only while nothing plays, and a
  recorder stop only while no recording runs. `runtime_replay_is_current`,
  `forget_operation_outcome` and `Storage::delete_journal_result` were removed:
  re-execution was the wrong fix (it could start a stopped microphone again).
  Ten dispatcher tests cover Stop-then-retry, both toggle directions, the
  recorded generation, restart refusal with a fresh-key success, recorder start,
  and stop replay/refusal. Before the revision, nine of the ten failed on
  `42538818e` for the expected wrong reasons: the retry re-executed (a hit
  "native worker is already attached"; b1 returned a fresh stop result; b2 tried
  to start the route and errored; c returned generation 2; restart, recorder
  start and stop-while-playing returned success instead of a refusal). Only the
  stop-replay-while-idle case already passed, because it performed no side
  effect.

  Startup constructs `operation_outcomes` empty and never reloads the journal,
  so a memory hit is exactly this lifetime; `lifetime_operations` keeps the
  origin fact for keys the bounded cache evicted. The rule covers
  `sessions.start`/`session.start`, `sessions.play`, `sessions.togglePlay`,
  `sessions.stop`, `recorders.create`, `recorders.arm`/`start`/`pause`/`resume`,
  `recorders.startRecording`/`stopRecording`, `recorders.stop` and
  `safety.setPrivacyMute`/`togglePrivacyMute`. Audit findings not changed:
  `audioSources.transport`, `timeShift.transport`, `soundboard.*` and
  `midi.sendEvents` take no idempotency key, so no replay exists;
  `sessions.active.set`, `system.quit`, `system.osTransition`, `plugins.retry`,
  `obs.connect`/`disconnect`/`apply`/`restore`, `recorders.split` and
  `startup.apply` are journaled but replaying performs no audio side effect and
  their results do not claim a route is playing or a recording is running. Those
  outcomes can still be stale (an OBS connection or an active selection); they
  are recorded for a later audit rather than changed here.
- **Harness:** the recorder helper used fixed idempotency keys for play, record,
  finalize and stop. It now uses one GUID per recording attempt so a later
  launch is a new intent, and `m08-lifecycle-recording.ps1` guards that no fixed
  `qualification-*` key returns. This alone makes the qualification path correct;
  the shipped 0.0.32 bytes do not contain the product fix, so the guest proof
  below exercises the harness fix, not the new binary.

Checks: `cargo test -p audiorouter-control` (519 passed, 29 ignored),
`cargo test -p audiorouter-storage`, workspace and shell strict Clippy, both
format checks, `node tools/quality/verify.mjs --checks release-tests` (8 scripts,
27.9 s), `node tools/docs/validate.mjs` (304 files/1,705 links), `git diff
--check`.

### Absent-WebView2 bootstrap on 0.0.32 (failed by design, same as v0.0.31)

First attempt through the unattended wrapper on
`lifecycle-webview-absent-v10-20261119` (UUID `6b16514e-0f8c-4699-9585-9253468a492e`)
failed at console sign-in: the screen showed **arqual4** with "The password is
incorrect". `guestcontrol` with the same credential succeeded, so the account
was not the problem; the password field was receiving stray characters (a wake
Space and an earlier attempt's text). After manually dismissing the expired
form and clearing the field (Ctrl+A, Delete), the standard user signed in.

Then, with the user signed in, `installer-lifecycle.ps1 -ProductionRecording
-WebViewBaseline Absent -OfflineBootstrapRetry` on the same snapshot:

- offline setup exit 2, duration 2.75 s; no registration, files, database, shell
  or runtime left behind (`unattributed-installer-failure`, retry eligible);
- after the NAT link returned, the same installer bootstrapped WebView2
  `158.0.4370.33` and passed the whole lifecycle (install 79.7 s because of the
  bootstrap download, repair, rollback to 0.0.29, upgrade, uninstall, reinstall);
- production recording 440 Hz stereo 48 kHz, 96,768 frames, RMS 0.0890,
  SHA-256 `63A6A849FBB77157FA509B4F99A4544F1E25847D8DB4D0AFFCBE92CF229FE96F`,
  retained through the transitions.

Overall status is `failed` by design because the silent NSIS offline exit has no
verified bootstrap-specific diagnostic, exactly the v0.0.31 attribution gap; no
new conclusion about the cause is claimed. Evidence
`%TEMP%\ar-rq-20261124-absent-inner-34a6ffd05c0f4196bf6ac7513c1fe98d`.

### Offline installed operation after reinstall: passed

After that lifecycle, with the guest network disconnected, the offline check ran
again with the fixed recorder helper and passed all steps: no DNS/TCP, six
installed executables at 0.0.32, CLI `status`/`schema`/`session list`/`session
get`, `system.describe`, MCP stdio `server/discover`, no auto-started shell, no
endpoint changes (the guest has none), and a production recording through the
installed bridge: 440 Hz stereo 48 kHz, 97,152 frames, RMS 0.0890, WAV SHA-256
`6289270CB77C7E0C9A6706D51756DFC0614779C7EFE42B0846A2BA0670D240F4`.
Evidence `%TEMP%\ar-rq-20261124-offline-fixed-be29bbd739ae44c3ae59c90dd2020eb6.json`.
This closes the RQ-05 offline-operation sub-case for the 0.0.32 bytes with the
fixed harness.

### Wrapper cold-boot sign-in: fixed and proved

The wrapper now, on retry only, dismisses a blocking expired-password dialog
(Enter, then Escape to the account picker) and clears any characters left in the
password field (Ctrl+A, Delete) before typing. No guest account, password,
policy or protection is changed. A wrapper run on `lifecycle-offline-20261118`
with `-InjectGuestControlTimeout` recorded: `sign-in-reset`, successful
`console-sign-in`, `guest-ready`, a 12 s bounded timeout, `timeoutMarkerReached:
true`, graceful shutdown; status `failed` by design for the injection. Evidence
`%TEMP%\ar-rq-20261124-wrapperretry-702646bc7aa84aac8ae4707fffa0d9cb`. On the
absent snapshot the same reset/clear also produced a manual sign-in.

### Cleanup

Both release guests were powered off, the older guest restored to
`lifecycle-offline-20261118`, the fresh guest stayed at
`offline-standard-user-20261121`; 4 CPUs, no VBox processes, no Defender
exclusions added. `pre-agent-rq-20261124` snapshots are retained.

### Still open after this session

Interactive downloaded-file trust prompt and first-run on 0.0.32; real endpoint
states and default-device/volume comparison; a second Windows build; the
Defender verdict and any signing; the fresh guest's credential (optional while
the older guest is the qualification baseline); the absent-runtime offline
attribution. The product idempotency fix needs the next build to reach an
installer; 0.0.32's shipped bytes behave correctly through the fixed harness and
the UI path (which prepares before start).