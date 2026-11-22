# Release qualification, candidate v0.0.31

Execution record for [the plan](../release-qualification.md). Started 2026-11-19.
Status: in progress; RQ-04/05 guest runs blocked on the guest credential.

## Candidate reconciliation (RQ-01)

The plan's inventory step asked to reconcile the intended release source with
the VST handoff before freezing anything. That reconciliation changed the
candidate.

- The VST handoff declared `release-candidate-2026-11-19` (`work/vst-compatibility`,
  source `39e5e32ff`, installer `54f40f62…2204`, version 0.0.30). It supersedes
  the `18b` candidate (`d6c400c7b`, installer `d76278cb…814f`) that the earlier
  lifecycle runs qualified, because the source gained the plain refused-preset
  message and the sidechain output-queue fix (`14aa4af12`).
- That VST candidate then fell behind its own branch: `0585e71b1` (refuse a
  network `shellPath` on the local HTTP API) landed after the build, so its
  bytes no longer match a clean tree.
- Independently, `release/0.0.31` was merged to `main` as PR #167 (`a4e92bb05`)
  and **v0.0.31 was already published** as an unsigned pre-release at
  2026-11-20T05:27Z. Its published manifest reports version 0.0.31, app-only,
  source `a4e92bb05482a16e3eef6ffefaf5ad9083b7e467`, installer
  `b94e52764b31fc4c237c688c49d1b1fc80e3de9799dc09e90b5093e077543a92`.

Decision: qualify the **published v0.0.31 bytes**. They are the artifact users
can download, they include the VST work plus `0585e71b1`, the localization
merge and the 0.0.31 version metadata, and they are the only candidate whose
bytes are frozen by publication. The 0.0.30 candidates are superseded
pre-publication builds and are not qualified here. Do not re-label their
lifecycle results as evidence for 0.0.31.

The manifest's own `blockers` list matches this workstream: unsigned disclosure,
standard-user install/upgrade/repair/uninstall and clean-machine acceptance,
and endpoint-specific acceptance.

## Environment

- Host: Windows 11 Home, build 26300; VirtualBox 7.2.20r175154.
- Checkout: `release/0.0.31` at `73b4cfefff272a42d16dd8337afa85900f9c0196`;
  untracked `target-preview/`, `target-preview2/`, `ui/tsconfig.tsbuildinfo`
  are unrelated and untouched.
- Disposable VM `AR-ReleaseQualification-20261116`,
  `87c31665-2022-49a2-a182-1a1ebb5e631c`, powered off, `audio="none"`,
  `vrde="off"`. `AR-DriverTest` (`c889da39-…`) stays excluded.
- Baselines: runtime-present `lifecycle-offline-20261118`
  (`45d20e3c-8066-4896-9e28-a206f9d88723`); runtime-absent
  `lifecycle-webview-absent-v10-20261119`
  (`6b16514e-0f8c-4699-9585-9253468a492e`, the current snapshot).
- Previous-version pair: v0.0.30 (source `6950c05e`, installer
  `34143713873aa69b1443547ecbcee424e4de710b8c314b4d1f04aeb9f6a89b6a`), app-only.

## Case results so far

### RQ-01 freeze the candidate: passed

Downloaded all 15 published v0.0.31 assets to
`%TEMP%/ar-rq01-v0031`. The installer hash matches the manifest exactly
(`b94e5276…3a92`), and `tools/release/verify-artifacts.ps1` returned exit 0:
"Verified unsigned artifacts". Manifest agreement: version 0.0.31,
`releaseTag` v0.0.31, x64, `includesDriver: false`, Rust 1.96.0,
`signed: false`, `publicationReady: false` with three explicit blockers.
No driver payload in the app-only profile. Publication itself was a human
action already taken; this record does not perform or endorse it.

### RQ-02 bounded orchestration: harness changed; host parts rerun, guest timeout open

The harness gained the RQ-05 bootstrap-retry work (`bda697cfd`) after the three
clean passes cited in the earlier handoff, so those passes do not cover the
current revision and the timeout case must be repeated.

- `tests/acceptance/m08-lifecycle-process.ps1`: passed, exit 0 (evidence
  `%TEMP%/ar-process-3f79a726-86fa-4248-bbfe-215dd57707ea`).
- `tests/acceptance/m08-lifecycle-guards.ps1`: passed, exit 0.
- `tests/acceptance/m08-lifecycle-recording.ps1`: passed, exit 0 (evidence
  `%TEMP%/ar-wav-check-08362f16-…`).
- Injected marker timeout, three consecutive clean passes and graceful
  shutdown on the current harness: **not run** (needs the guest credential).

### RQ-03 refusal guards: host-side cases passed on v0.0.31

Each refusal stopped before `VBoxManage` ran; the VM stayed `poweroff` with its
snapshots intact, and each evidence directory holds a `result.json` with no
steps.

| Case | Result |
| --- | --- |
| Excluded driver VM `c889da39-…` | refused: "The driver test VM is excluded from installer qualification" (`ar-rq03-v0031-wrongvm`) |
| Wrong installer SHA-256 | refused: "Host installer hash mismatch" (`ar-rq03-v0031-wronghash2`) |
| Wrong snapshot UUID | refused: "Snapshot UUID does not belong to this VM" (`ar-rq03-v0031-wrongsnap`) |
| Administrator account, unloaded hive, existing staging/app | synthetic only (`m08-lifecycle-guards.ps1`); real-guest cases not run |

### RQ-04 lifecycle on v0.0.31: not run

The published bytes differ from every lifecycle run so far (`18b`, `39e5e32ff`),
so all six transitions and the production recording must be re-established on
this package. Blocked on the guest credential.

### RQ-05 interactive setup, recovery and first run: not run

Unchanged from the handoff: interactive unsigned-trust observation, endpoint
guidance, default-device and volume checks, and fully offline installed
operation still need a person at a GUI console. The runtime-absent bootstrap
case retains its earlier **failed** attribution.

## Blocker

`arqual4`'s password was deleted with the earlier temporary credential files.
Two secret requests to the operator timed out. Without the file the wrapper
cannot sign the standard user in at the console, so RQ-02's guest timeout,
RQ-03's guest-side refusals, RQ-04 and guest RQ-05 cannot start. Everything
credential-independent above is done.

## Next action

Provide the `arqual4` guest password, then run in order on the frozen v0.0.31
bytes: (1) `-WebViewBaseline Present -ProductionRecording` on
`lifecycle-offline-20261118` for RQ-04 plus RQ-02's clean passes;
(2) `-InjectGuestControlTimeout` for RQ-02's marker case; (3) the
administrator/hive/staging refusal states for RQ-03; (4) a GUI-console pass
with an observer for RQ-05's interactive cases.

## Automated run preflight, 2026-11-21

The disposable VM was powered off with host audio disabled. The retained
v0.0.31 installer still matched its recorded SHA-256. Windows blocked
`Get-FileHash` on the retained v0.0.30 rollback installer. Microsoft Defender
recorded threat ID `2147927547` (`Trojan:Win32/Suschil!rfn`), action success,
and an active threat status. The cause of the detection is unverified.
The harness checks both installer hashes before boot, including for the
injected timeout case. No guest run or installer execution was attempted.
Keep the protection in place. Resolve the detection or verify a separate,
compatible previous-version package before retrying the automated lifecycle.

Follow-up on the same day: `Get-MpThreatDetection` recorded successful action
after the PowerShell read. The installer was no longer in the download
directory. Antivirus and real-time protection were enabled with signature
version `1.463.404.0`. The GitHub release asset digest from `gh api` matches the
retained manifest's published SHA-256 for source `6950c05e`. The flagged local
bytes could not be rehashed, and matching release metadata cannot settle the
detection. External analysis has not been requested.

### Alternate rollback fixture and VM sign-in

The retained published v0.0.29 installer SHA-256 matched the GitHub release
digest `f2c43d115f04fdef00f55ef96862eba41d581464db24785cde5f69755ba209d8`.
This older fixture can exercise the lifecycle independently of the immediate
v0.0.30 rollback case. The first wrapper invocation failed at PowerShell
parameter binding before boot. The corrected invocation restored the verified
runtime-present snapshot from poweroff and reached Windows. Guest Additions
reported 7.2.20, but guest control returned `VERR_UNRESOLVED_ERROR` before
sign-in. The console initially showed an expired-password prompt for `artest`.
After canceling it, a console sign-in attempt for `arqual4` displayed the
Windows account lockout message. No further sign-in input was sent.
The timeout marker and lifecycle were not reached. Attempt evidence is under
`%TEMP%/ar-rq-v0031-v0029-a0b659b849334eeb92e396d27fdca11a`.
Next action: recover `arqual4` in the disposable guest and verify one successful
standard-user sign-in before re-enabling unattended authentication attempts.

### Fresh disposable guest

The owner does not have credentials for the earlier test accounts and supplied
Windows installation media. VirtualBox detected Windows 11 x64, build
`26300.9457`. A separate guest `AR-ReleaseQualification-Fresh-20261121`
(`074bd5af-a936-4e25-a2c7-d8fbd632b631`) uses a new VDI on the C: SSD,
EFI, TPM 2.0, NAT and no host audio. VirtualBox started unattended Windows
11 Home setup with Guest Additions from the original `E:\ISO\Win11.iso`.
No temporary ISO copy was created. Setup start is not evidence of a usable
standard-user snapshot or of an AudioRouter test pass.

### Standard-user guest ready; v0.0.31 installer quarantined by Defender (2026-11-21)

The disposable guest `AR-ReleaseQualification-Fresh-20261121` now has the
administrator `arvmadmin` and the standard user `arqualfresh` (member of Users
only). A guest-control command as `arqualfresh` worked and a console sign-in
worked. Clean snapshot `clean-standard-user-20261121`
(`d739d434-199b-411b-a9b3-7ef373f57e3c`) was taken with the desktop running and
no AudioRouter installed. Do not restart the guest after restoring it: a
pending Windows update stalled a restart at "Restarting" and needed a power-off.
Take no live snapshot with a signed-in session; one hung at 0% and the VM had to
be killed. Restore, lock (Win+L), choose `arqualfresh` with Tab/Down/Enter and
type the password to reach the signed-in state the harness requires.

First lifecycle attempt, v0.0.31 (installer `b94e5276…3a92`, verified before
copy) with v0.0.29 as the previous version, `-ProductionRecording
-WebViewBaseline Present`, evidence `%TEMP%/ar-rq-v0031-fresh-b867fdf`:
**failed at `fresh-install`**, setup exit 2 after 3.5 s. WebView2 157.0.4341.60
was present before and after.

Cause: Microsoft Defender in the guest (signatures 1.463.406.0) quarantined the
copied installer at 16:41:59 as `Trojan:Win32/Wacatac.H!ml` (threat ID
2147814523, severity 5, action succeeded); `candidate.exe` was gone from the
staging folder while `previous.exe` (v0.0.29) remained. This is a
machine-learning heuristic on an unsigned NSIS installer, not a signature match,
and is the same detection class seen on the host for the v0.0.30 installer
(`Trojan:Win32/Suschil!rfn`, 2026-11-21). The bytes are the published release
asset, hash-verified. It is a user-facing finding for the unsigned release (DIST
unsigned-trust blocker), not evidence about the installer's behaviour.

Not done: an exclusion for the staging folder in the guest failed ("not enough
permissions" over guest control, tamper protection likely). No Defender setting
was changed. RQ-04 on v0.0.31 therefore remains not run.
Options: (a) with the owner at the guest console, add the exclusion or turn off
real-time protection in this disposable guest only, record it as a deviation,
and rerun; (b) submit the installer to Microsoft for analysis (outward action,
needs owner approval); (c) code-sign the installer.

### RQ-04 lifecycle on v0.0.31: passed with a recorded Defender deviation (2026-11-21)

Owner chose option 1 (exclusion in the disposable guest). From the signed-in
`arqualfresh` console I ran `Add-MpPreference -ExclusionPath
C:\Users\arqualfresh\ar-release-qualification` in an elevated PowerShell
(UAC entered as `arvmadmin`); it took effect. Real-time protection stayed on for
everything else. **Deviation:** the installers were not scanned in this run, so
this result says nothing about the Defender detection above.

Run: `tools/vm/installer-lifecycle.ps1 -ProductionRecording -WebViewBaseline
Present`, VM `074bd5af-…`, standard user `arqualfresh`, candidate v0.0.31
(`b94e5276…3a92`), previous v0.0.29 (`f2c43d11…09d8`), both hash-verified before
copy. Restored from `clean-standard-user-20261121`; Windows 11 build 26300,
WebView2 157.0.4341.60 before and after. Evidence `%TEMP%/ar-rq-v0031-fresh-c5221f9`.
Result `passed`; every step passed: preflight, webview-baseline, fresh-install,
webview-after-setup, seed-data, repair, rollback to 0.0.29, upgrade, uninstall,
reinstall. The session and the recording from the installed engine were retained
through the transitions (the harness checks both).

Still open: RQ-02 injected guest-control timeout on this harness revision, RQ-03
real-guest refusals, RQ-05 interactive/GUI cases (trust prompt, shortcuts, first
run, endpoint defaults), the v0.0.30 rollback fixture (blocked by the host
Defender detection), a second supported Windows build, and the Defender
detection itself (needs signing or a Microsoft submission).

### RQ-02 injected guest-control timeout: passed on the current harness, partial method (2026-11-21)

Ran the injected-timeout logic of `installer-lifecycle-unattended.ps1` three
times in a row against the running fresh guest as `arqualfresh`, using the
harness's own `Invoke-LifecycleProcess` (current revision, including the
RQ-05 bootstrap-retry work). Each run: warm-up, then a guest command that wrote
`AR_QUAL_TIMEOUT_REACHED` to a marker and hung on `ping -n 20`. Results: every
run timed out at 12.0-12.1 s (bound 12 s), the marker was reached in the guest
each time, and no `VBoxManage guestcontrol` process survived on the host.
Evidence `%TEMP%/ar-rq02-timeout-f244b9c6`.

**Deviation:** not the unattended wrapper itself. It requires a powered-off VM
and a cold boot with console sign-in, and this guest's cold boot is blocked by a
pending Windows update that hangs at "Restarting". The wrapper's restore, start,
sign-in and graceful-shutdown steps were therefore not exercised on this guest.
Closing that needs a powered-off baseline with `arqualfresh` as the default
sign-in account (finish the update once and snapshot while off).

### RQ-02 and RQ-04 through the unattended wrapper (2026-11-21, evening)

The guest was brought to a powered-off baseline: the pending Windows update was
applied (a restart from the clean snapshot completed after roughly 25 minutes
this time), `arqualfresh` signed in once so it is the default account, the
staging-folder Defender exclusion was re-added from an elevated shell, the guest
was shut down gracefully, and snapshot `offline-standard-user-20261121`
(`1dae1ead-1a27-4fc8-8e8c-f68c70d254f7`) was taken while powered off. This
snapshot replaces the deviation noted under RQ-02 above for the wrapper steps.
The Defender exclusion deviation still applies to every run below.

`tools/vm/installer-lifecycle-unattended.ps1` with `-RestoreDisposableSnapshot
-ProductionRecording -WebViewBaseline Present`, candidate v0.0.31 (`b94e5276…`),
previous v0.0.29 (`f2c43d11…`):

- **Injected guest-control timeout** (`-InjectGuestControlTimeout`), evidence
  `%TEMP%/ar-rq-v0031-unatt-4d951d`: restore, start, console sign-in, guest-ready
  passed; the hung command was cut off at 12 s ("timed out after 12 seconds"),
  the guest marker was reached (`reached: true`), and the guest shut down
  gracefully. The run is `failed` by design, as the harness intends.
- **Full lifecycle, three consecutive passes** without injection, evidence
  `%TEMP%/ar-rq-v0031-unatt-full1-af9b78`, `…-full2-4680a9`, `…-full3-ffe921`:
  each restored the snapshot, cold-booted headless, signed in at the console,
  ran every lifecycle step (install, seed, repair, rollback, upgrade, uninstall,
  reinstall) and shut down gracefully; all three exited 0 with status `passed`.

RQ-02's bounded-orchestration case is therefore closed on the current harness
for this guest. RQ-04 on v0.0.31 now has four passing runs (one attended-style
run through `installer-lifecycle.ps1` and three through the wrapper), all with
the exclusion deviation. Still open: RQ-03 real-guest refusals, RQ-05
interactive cases, the v0.0.30 rollback fixture, a second Windows build, and the
Defender detection.

### RQ-03 real-guest refusal cases: passed (2026-11-21)

Each case restored `offline-standard-user-20261121`, cold-booted headless, set up
one bad precondition and ran `installer-lifecycle.ps1` with the v0.0.31 and
v0.0.29 installers (hashes verified). Every run refused, and the protected state
was identical before and after. Evidence: `%TEMP%/ar-rq03-guest-5ac790/*`
(first three) and `%TEMP%/ar-rq03-admin-ok-792f3b`.

| Case | Setup | Result |
| --- | --- | --- |
| Existing staging directory | `ar-release-qualification\marker.txt` created first | refused at `guest-staging` (exit 35 from the staging check); marker SHA-256 and listing unchanged; no AudioRouter directory created |
| Existing app directory | `%LOCALAPPDATA%\AudioRouter\marker.txt` created first | refused: "Guest profile is not fresh: AudioRouter directory already exists"; marker hash and listing unchanged |
| Administrator account | harness run as `arvmadmin` signed in at the console | refused: "Guest account must not belong to Administrators" (guest exit 33); no AudioRouter directory created |
| User not signed in (profile hive not loaded) | cold boot, no console sign-in | refused on the host before any copy: "Guest user must already be signed into the desktop before guest control loads a profile" |

Notes: a first administrator attempt added `arqualfresh` to Administrators over
guest control, which was denied (system error 5), so that attempt ran as a
standard account and is not counted. The sign-in order on the picker follows the
last user (`arqualfresh` first, `arvmadmin` second). The unloaded-hive refusal
here is the host's signed-in check; the guest-side registry-hive assertion
itself stays covered by the synthetic guard test only.

Open after this: RQ-05 interactive cases, the v0.0.30 rollback fixture, a
second Windows build, the Defender detection (signing or Microsoft
submission). The Defender exclusion deviation does not affect refusal cases.

### RQ-05 attended-style interactive pass on the guest console (2026-11-21)

Driven through the VirtualBox console with keystrokes and screenshots, as the
standard user `arqualfresh`, on the restored `offline-standard-user-20261121`
baseline (Windows 11 build 26300 with the update applied, WebView2 present, no
audio devices, NAT). Screenshots: `evidence/2026-11-21-rq05-screens/`. The
installer was copied in (so there is no Mark-of-the-Web) and sat in the
Defender-excluded staging folder.

Observed, v0.0.31 installer `b94e5276…3a92`:

- Setup opened from Run with **no UAC prompt and no SmartScreen warning**. The
  absence of SmartScreen is an artifact of copying; the real downloaded-file
  unsigned-publisher prompt was **not observed** and remains open.
- Wizard: Welcome, per-user location `%LOCALAPPDATA%\AudioRouter` (74 MB),
  install in about 10 s, "Setup was completed successfully", finish page with
  "Run AudioRouter" and "Create desktop shortcut" both ticked.
- Both shortcuts exist and target `audiorouter-shell.exe`: Desktop and Start
  menu. Uninstall registry key written under HKCU (Publisher shows as lowercase
  `audiorouter`: cosmetic).
- First run opened "AudioRouter 0.0.31" with a "What do you want to do?"
  chooser (four starting routes plus two more). It states nothing plays until
  Play is pressed, the mic stays out of the headphones and recording stays off.
  Dismissed with Esc: default route Input device, Gain, Output device; with no
  audio devices the input and output cards show "NO DEVICE" and nothing is
  substituted. Header shows "Audio stopped".
- Uninstall wizard: "Delete the application data" unticked by default. With the
  app still open it prompted "AudioRouter is running! Click OK to kill it"
  (blunt wording; polish item). After OK: "Uninstall was completed
  successfully". Residue check: app files, both shortcuts and the HKCU uninstall
  entry removed, no Run key, no AudioRouter process; only user data remained
  (`state.sqlite`, backups, `shell-settings.json`, logs), as intended by the
  default.

Not covered here: pressing Play with real endpoints and the default-device and
volume checks (the VM has no audio devices and host audio stays disabled); the
downloaded-file unsigned trust prompt; fully offline installed operation.
Those need the host or a person with the real flow, so RQ-05 stays **partial**.
Polish items to consider: uninstall running-app wording; lowercase publisher.

### RQ-05 real-PC audio checks, isolated run (2026-11-21, evening)

Owner chose an isolated run (no install on the owner's PC) and closed their
AudioRouter first. The v0.0.31 files were taken from a throwaway-VM install
(`candidate.exe /S` in the guest, `copyfrom` to `%TEMP%/ar-v0031-files`, 120 files)
and `audiorouter-shell.exe` was launched from there with
`LOCALAPPDATA=%TEMP%/ar-iso-run/local`, `WEBVIEW2_USER_DATA_FOLDER` pointing into
the same folder and a localhost-only debug port to drive the UI. No
`AUDIOROUTER_*` variables (so the product's own first-run consent was exercised).
The owner's `%LOCALAPPDATA%\AudioRouter` was not touched (state WAL last written
17:47:02, before the launch at 17:49:39). Screenshots 10-17 in
`evidence/2026-11-21-rq05-screens/`; audio state before/after in the same folder.

Observed on the host with real endpoints (Windows 11 build 26300):

- First run: same "What do you want to do?" chooser; default route shows
  "NO DEVICE" on the input and output until chosen; Properties says "No device is
  chosen for this node yet" (no fallback to a remembered or default device).
- The output picker listed every real endpoint (3 VB-Cables, 3 monitors, Realtek,
  Focusrite, PD200X). The CABLE endpoints were avoided: the owner's voice, game
  and recording routing uses all three. A monitor endpoint (DELL U3223QE) carried
  the test.
- Dragging a second source onto an occupied output auto-added a Mixer with the
  message "Added a Mixer so Neutral gain and Test Signal 1 can share Output
  device."
- **Privacy:** pressing Play with an Input device that has no device chosen was
  refused ("No audio started. This route also needs an input device…", with a
  "Go to Input device" action). No microphone was substituted.
- **Consent:** the next Play showed "Allow AudioRouter to use your audio devices?"
  (once per computer); the backend log holds the matching `permissionDenied` until
  "Allow and play". That is the only error line in 43 backend log entries.
- With Test Signal (-18 dB) to Mixer to Output device (DELL), Play gave "Audio
  running"; pressing Play tone (a 1 s pulse) read peak -18.0 dB at all three
  stages (RMS about -21 dB, as a sine at that peak should read) and then returned
  to silence. Stop returned to "Audio stopped"; Quit needs a second click
  ("Click again to quit"). No audiorouter process or WebView2 process from the
  run remained.
- **Defaults and volumes unchanged:** the owner's default playback (Speakers,
  Focusrite), default recording (CABLE-A Output, all three roles), and every
  endpoint's volume (1.000, unmuted) were identical before, during playback and
  after quitting (diff empty each time).

Not covered: whether sound actually emerged from the DELL monitor (not
measured; engine meters only), long-run stability, real microphone paths,
playing through the owner's cables, fully offline installed operation, and the
downloaded-file unsigned prompt. This run used extracted files, so it says
nothing about the installer on the owner's own account. RQ-05 stays
**partial**, now with a passing real-endpoint check.

### Defender false-positive submission and signing options (2026-11-21, late)

SignPath Foundation told the owner it cannot sign this project, so the
2026-10-03 free-route plan is closed. Remaining routes: Microsoft Artifact
Signing (individual onboarding is reported limited to the US and Canada; owner's
country not yet given), a paid OV certificate, or staying unsigned. Artifact
Signing does not cover the planned kernel driver: Microsoft is the only signer of
kernel-mode drivers and attestation signing through Partner Center requires an
EV certificate (a 2026-04 Microsoft Q&A answer says Trusted Signing cannot stand
in for it; community answer, confirm with Microsoft support). The M03 plan's own
estimate of $300-450 per driver release window still applies to the driver.

The owner submitted the installer to Microsoft Security Intelligence as a
software developer. Submission ID
`eb70e424-324e-430b-a096-6e05574d16bb`
(https://www.microsoft.com/en-us/wdsi/submission/eb70e424-324e-430b-a096-6e05574d16bb).
File `AudioRouter_0.0.31_x64-setup.exe`, SHA-256 `b94e5276…3a92`, detection
`Trojan:Win32/Wacatac.H!ml`, definitions 1.463.406.0. The form showed an error on
submit but the submission exists. **Caveat:** the form recorded the category as
"Incorrectly detected as PUA" rather than "incorrectly detected as
malware/malicious" (typing the detection name under the PUA option switched it).
The verdict is pending; the correct category is malware false positive. If
Microsoft's reply treats the category as wrong, file a correction that refers to
this ID. Do not resubmit meanwhile.

Next: when a determination arrives, restore `offline-standard-user-20261121`,
remove the guest's Defender exclusion by not adding it, update definitions, and
rerun the lifecycle to see whether the verdict clears. That run removes the
Defender deviation from the RQ-04 results.
