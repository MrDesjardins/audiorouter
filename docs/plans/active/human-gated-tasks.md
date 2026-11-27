# Human-gated tasks

Created 2026-11-24 from the plans archived that day. Every task here needs a
person: a decision, money, ears, eyes, hardware, an account or another person.
Agent-only work is not listed. Each task can be done on its own unless **Needs**
names another one.

## How to use this page

1. Pick a task. Read **Needs** first; it says what must be ready.
2. Do the steps. Where an agent can run the commands once you have done your
   part, the task says **Agent can run**.
3. Record the result in the task's **Result** line: date, build (SHA-256 of the
   exe or commit), pass or fail, and a link to the evidence file in
   `docs/plans/active/evidence/`. Do not put private audio or passwords there.
4. A failed check stays failed until a later run passes. Do not delete it.
5. When a task passes, move its row to [Finished](#finished) with the evidence
   link. When all of a plan's tasks are finished, its archived record needs no
   further change.

Results that are **FAILED today** are marked so. Nothing on this page has passed.

### Common preparation

- **Build:** `powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-desktop.ps1`,
  then check that `src-tauri/target/release/audiorouter-shell.exe` is newer than
  `ui/dist/index.html` and note `Get-FileHash` of the exe.
- **Disposable profile** (keeps your own AudioRouter data untouched). Close every
  AudioRouter first; `Get-Process audiorouter-shell*` must print nothing. Then:

  ```powershell
  $t = Join-Path $env:TEMP ('ar-check-' + (Get-Date -Format yyyyMMdd-HHmmss))
  New-Item -ItemType Directory -Force $t | Out-Null
  $env:LOCALAPPDATA = "$t\local"
  $env:AUDIOROUTER_DATABASE = "$t\state.sqlite"
  $env:WEBVIEW2_USER_DATA_FOLDER = "$t\webview"
  & .\src-tauri\target\release\audiorouter-shell.exe
  ```

  Use a new PowerShell window so the variables end with it. Do not set
  `AUDIOROUTER_ALLOW_DEVICE_ADMIN` for a release check.
- **After a check:** search the log under that folder (`logs\backend.jsonl`) for
  `"outcome":"error"`.
- **Never run a Blue Cat plugin unattended** (its demo dialog blocks).

### Contents

| Group | Tasks | What you need |
| --- | --- | --- |
| [A. Decisions](#a-decisions) | A1–A18 | Only an answer |
| [B. Purchases and outside companies](#b-purchases-and-outside-companies) | B1–B4 | Money, account or reply |
| [C. Quiet-PC runs](#c-quiet-pc-runs) | C1–C3 | A quiet time window |
| [D. Virtual machine](#d-virtual-machine-virtual-cable) | D1–D6 | AR-DriverTest, one sign-in |
| [E. Release and installer checks](#e-release-and-installer-checks) | E1–E6 | GUI guest, second PC |
| [F. Screen, keyboard and Narrator](#f-screen-keyboard-and-narrator) | F1–F7 | Eyes, about 2 hours |
| [G. Listening, plugins and MIDI](#g-listening-plugins-and-midi) | G1–G12 | Ears, plugins, hardware |
| [H. OBS, Discord and assistants](#h-obs-discord-and-assistants) | H1–H7 | Your real apps |
| [I. Other people](#i-other-people) | I1–I4 | Participants, reviewers, reporters |
| [J. Driver after signing](#j-driver-after-signing) | J1–J2 | B1 finished |

## A. Decisions

Answer in one line; an agent can then do the work. "Default" is what happens if
you do not answer.

| ID | Question | Options and recommendation | Requirements | Source |
| --- | --- | --- | --- | --- |
| A1 | May an assistant start and stop recordings? | (a) per-client opt-in, off by default; (b) window only. Default: (b), as today. (a) needs a decision record in `docs/spec/13-security.md` and `11-automation.md` | R2-1a, AUTO-10/18, SEC-02 | [API and MCP](../archived/2026-11-24-api-mcp-quality.md) |
| A2 | May an assistant rename a session? | (a) add `sessions.rename` with a default tool; (b) window only. Default: (b) | R2-4, AUTO-06/18 | [API and MCP](../archived/2026-11-24-api-mcp-quality.md) |
| A3 | Keep the virtual bus lists as expert-only tools? | An agent decided yes: no assistant role can set up buses. Say no to reverse it | R2-4 | [API and MCP](../archived/2026-11-24-api-mcp-quality.md) |
| A4 | Support an 8-channel CABLE-B Input? | (a) keep stereo only and record the CABLE-B runs as not applicable; (b) support it, as its own change with layout tests. Today the worker refuses it; you use 7.1 there on purpose | ARCH-08, NFR-03 | [Code review fixes](code-review-fixes.md) |
| A5 | What should a route show when its device recovery was dropped by an edit, a recording start or a failed plugin? | Today it still says running. Recommended: mark it stopped with the reason, never auto-restart. Alternative: keep the status, say "press Stop, then Play" | RES-3, STATE-10 | [Application quality](../archived/2026-11-24-application-quality.md), `a_graph_commit_during_device_recovery_cancels_the_stale_recovery` |
| A6 | What should `sessions.start` do on a route that lost its device? | Today it succeeds and stays silent. Recommended: refuse with `nodeNotReady` ("stop, then play"). Alternative: recover automatically | RES-3, CAP-11 | `a_start_on_a_device_lost_route_cancels_its_recovery_and_starts_nothing` |
| A7 | Two sleeps without a resume: keep the first list of stopped routes? | Today the second sleep empties it. Recommended: merge the lists, still never auto-start | STATE-11 | `a_second_sleep_forgets_the_first_sleeps_resume_list` |
| A8 | Should the resume list skip routes whose plugin failed? | Today only crash recovery skips them. Recommended: skip in both, with a message | RES-3 | [Application quality](../archived/2026-11-24-application-quality.md) |
| A9 | Does an armed recorder block sleep? | Today two checks disagree. Recommended: one "armed or recording" rule, and never re-arm after recovery | CAP-11 | `sleep_with_a_recording_changes_nothing_and_resume_after_it_restarts_nothing` |
| A10 | Coverage floors per module? | **Needs** at least 14 nightly rows over 7 commits, collected automatically. Then pick floors from [the baselines](evidence/2026-11-24-q5-coverage-baselines.md) | ENG-02/03/04 | [Application quality](../archived/2026-11-24-application-quality.md) Q5.2 |
| A11 | Gate CI on start-up and UI-ready timings? | **Needs** enough nightly rows to know runner noise. Default: report only | NFR-14 | Q5.6 |
| A12 | Change the plugin-worker restart policy? | Default: keep it. A change needs its own tests | RES-10 | [Active plan](current.md) |
| A13 | Build the deferred "Record clips" and "Hear game and Discord" goals, and OBS separate tracks/toast? | Default: not built | UI-26 | [OBS and Discord](../archived/2026-11-24-obs-discord.md) |
| A14 | Public site and quickstart still say "Physical input/output"; the app says "input device/output device". Change the site? | Recommended: yes; an agent then updates the site and guides | UI wording | [UI quality](../archived/2026-11-24-ui-quality-review.md) |
| A15 | Leave Fun Voice Changer preset names in English? | Default: yes (accepted limitation). Otherwise an agent adds a translation map | L10N-01/05 | [Localization](../archived/2026-11-24-ui-localization.md) |
| A16 | Delete or restore the keyboard connection dialog that nothing opens? | Default: leave as it is | UI-11 | [Active plan](current.md) |
| A17 | Future work: promote the signed updater (key custody, feed host, channels) or the UAD inventory? | Default: both stay in `future/` | SEC-11, PLUG | [Updater](../future/auto-update-tauri.md), [UAD](../future/uad-plugin-support.md) |
| A18 | Allow more than 16,384 edits in 24 hours (automation)? | Above it new edits are refused until old records expire. Default: keep. A larger journal costs disk and time on every edit | API-07/11 | [Application quality](../archived/2026-11-24-application-quality.md) |

**Result:** none recorded.

## B. Purchases and outside companies

### B1. Signing certificate, identity and key storage

- **Status: waiting for OSSign (your decision, 2026-11-25).** It can sign the app
  and attestation-sign the driver, but needs six months of project history. Apply
  again on or after **2027-03-05**; buy and enroll nothing meanwhile. App
  signatures would carry an OSSign backing company's name. Acceptance is not
  promised. See the [reply, rules and application
  checklist](../future/M03-driver-signing.md#ossign-reply-2026-11-25-ticket-991393).
  Everything from **Steps** on is the fallback if OSSign declines.
- **Why you:** legal identity and money; the key cannot be copied later.
- **Needs:** confirmed eligibility. The driver part also needs D2–D4 clean.
- **Steps:** 1. Send Microsoft the written question in Part 4 of the
  [driver signing guide](../../operations/virtual-cable-signing.md)
  ("testing purposes only" wording). App signing steps are in the
  [signing plan](../archived/2026-11-24-release-signing-and-publication.md). 2. Choose the provider (SSL.com sole
  proprietor EV, business EV, or another). 3. Choose a cloud signer or a
  token. 4. Buy and validate. 5. **Agent can run** the rest: sign a throwaway
  CLI copy with `signtool sign /fd SHA256 /tr <ts> /td SHA256 <key> <exe>`,
  then `signtool verify /pa /v` and `Get-AuthenticodeSignature`.
- **Pass:** the copy is Valid with a timestamp, and the signed regression cases pass.
- **Driver:** submit exactly the frozen 0.2.5 files (or a new RC if your
  D3 decision needs one). There is no 0.2.5 PDB, so submit without symbols.
- **Record:** provider, subject, thumbprint, expiry, submission IDs, signed hashes.
- **IDs:** DIST-05/06/07, ENG-04/05, WP-13, M08. **Result:** open.

### B2. Microsoft Defender verdict for v0.0.31

- **Status: FAILED today.** Defender quarantines the unsigned v0.0.31 installer
  (`Trojan:Win32/Wacatac.H!ml`). Submission `eb70e424-324e-430b-a096-6e05574d16bb`
  was filed under the wrong category (PUA) by a form error.
- **Steps:** 1. Check the submission. 2. If the category is wrong, file a
  correction that quotes the ID; do not resubmit. 3. After a verdict, restore the
  release guest with **no exclusion**, update definitions. 4. **Agent can run** the
  unattended lifecycle and `Get-MpThreatDetection`.
- **Pass:** no quarantine, and a clean lifecycle without an exclusion. Also covers
  the clean 0.0.32 run: the guest's existing exclusions could not be read, so
  the 0.0.32 lifecycle is not clean-Defender evidence.
- **Record:** verdict and date, definitions, run ID, exclusion list.
- **IDs:** DIST-07/08, RQ-04. **Result:** open.

### B3. Compatible previous package for rollback

- **Why you:** the local v0.0.30 file was quarantined, so its bytes cannot be rehashed.
- **Steps:** download the published v0.0.30 again and compare `Get-FileHash` with
  the published digest (starts `34143713`). If it is quarantined, approve v0.0.29
  (`f2c43d11…`, already verified) as the previous package.
- **Pass:** a matching hash, or your recorded choice. **Agent can run** the rollback.
- **IDs:** Q6.4. **Result:** open.

### B4. ChatGPT connector

- **Needs:** a ChatGPT account with connector support; AudioRouter running.
- **Steps:** 1. Settings → AI assistants (MCP) → ChatGPT, start the tunnel.
  2. Add the AudioRouter connector in ChatGPT. 3. Ask it to call `get_recipes`
  and `list_sessions`. 4. Ask it to call `play`. 5. Read the MCP activity view.
- **Pass:** the reads work; `play` is refused with the explanation; the activity
  view shows the calls.
- **IDs:** AUTO-06/16, DEC-20/21. Source: [MCP setup](../archived/2026-11-24-mcp-setup-ux.md).
  **Result:** open.

## C. Quiet-PC runs

You give a time window; **an agent can run** all commands.

### C1. CABLE-A continuity

- **Status: FAILED today, 3 of 6 runs** (2026-11-24; gain 2/3, ten-tool chain 1/3).
  The cables were not proven free: Discord and Zoom were open, and CABLE-A Output
  is your default communications microphone. The cause is **not known**.
- **You do:** close Discord, Zoom, games and AudioRouter, and say the cables are
  free for 15 minutes. Nobody may be listening to CABLE-A.
- **Agent runs:**

  ```powershell
  $env:CARGO_BUILD_JOBS=4
  cargo test -p audiorouter-transport --test live_audio_continuity --release --locked --no-run
  $env:AUDIOROUTER_LIVE_CONTINUITY='1'; $env:AUDIOROUTER_CONTINUITY_SECONDS='20'
  $env:AUDIOROUTER_CONTINUITY_TONE_RENDER_ID='{0.0.0.00000000}.{71f96f14-94c1-4189-b9b9-df8c960db8f2}'
  $env:AUDIOROUTER_CONTINUITY_ROUTE_CAPTURE_ID='{0.0.1.00000000}.{06268191-5f8c-42ed-827e-d3c7a19637ed}'
  $env:AUDIOROUTER_CONTINUITY_ROUTE_RENDER_ID='{0.0.0.00000000}.{03498a2a-ef6a-4dd4-b9ba-72e22f567d7a}'
  $env:AUDIOROUTER_CONTINUITY_RESULT_CAPTURE_ID='{0.0.1.00000000}.{ef18e30a-69c9-4299-bae2-1a2452fb8e4f}'
  $env:AUDIOROUTER_CONTINUITY_CHAIN='gain'
  cargo test -p audiorouter-transport --test live_audio_continuity --release --locked -- live::live_backend_service_keeps_a_routed_tone_continuous --ignored --exact --nocapture
  ```

  Three times with `gain`, then three times with
  `gain,parametricEq,graphicEq,bassTreble,dehum,compressor,gate,limiter,delay,meter`.
- **Pass:** 3 of 3 for each chain; zero routed glitches and silent runs on a clean
  reference. A repeatable glitch is a defect to trace.
- **IDs:** ARCH-08, CAP-04, NFR-03. Source: [Code review fixes](code-review-fixes.md),
  [record](evidence/2026-11-24-code-review-fixes-residual.md). **Result:** failed.

### C2. Live fault cases and recovery

- **Needs:** four named endpoints nobody uses, and your presence for unplugging.
- **Steps:** on a disposable route, do privacy mute, kill a plugin worker on a
  protected microphone path (also a sidechain plugin), remove and return an
  endpoint, sleep and resume, and a UDP network route under controlled load.
- **Pass:** protected paths go silent until you press Stop, then Play; nothing
  starts by itself; Windows defaults and volumes are unchanged; UDP is continuous.
- **IDs:** QUAL-01–06, NFR-04/09/10/11, RES-3, CAP-11. Source: Q4.6/Q4.7. **Result:** open.

### C3. Eight-hour soak, memory and your real library

- **Needs:** AudioRouter closed, eight hours, the reference PC.
- **Agent can run:** `powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\quality\run-app-soak.ps1 -Hours 8 -AutoPlay -Shell <release exe> -Output <new folder>`
  (it uses a copy of your database and plays its selected session). Also time
  Play, export and import with your real library (RES-6).
- **Pass:** shell alive; private bytes < 5 MiB/h; WebView < 20 MiB/h; handles
  < 50/h; thread spread ≤ 8; both logs keep ≥ 24 h.
- **IDs:** NFR-04/05/06/14/15, RES-6, Q5.4/Q5.5. **Result:** open.

## D. Virtual machine (virtual cable)

Driver RC 0.2.5 (`7c88895c2`). Test-signed drivers never go on this PC. Guide:
[VM guide](../../operations/virtual-cable-vm-guide.md). Bundle:
`C:\VMs\ar-share\rc-0.2.5-r7` (manifest SHA-256 starts `3D40FB0C`).

### D1. First VM task: sign in to AR-DriverTest

- **Why you:** the guest's `artest` password has expired, so every restore of
  snapshot `02-test-signing-ready` stops at a password change. An agent must not
  set it.
- **Steps:** 1. Close games and heavy apps; run `wsl --shutdown` yourself.
  2. `powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\round2-host-preflight.ps1`
  must exit 0. 3. Start AR-DriverTest, sign in, set a run-only password.
  4. Optional decision: allow **one new snapshot** after that, so this is needed
  only once. 5. Tell the agent the VM is ready and for how long the PC is quiet.
- **Pass:** you are at the guest desktop and the preflight is still green.
- **Result:** done 2026-11-25 by the agent, with the owner's authorization. Changing the
  expired password failed with "Access is denied" both at sign-in and through ADSI, so the
  guest was booted with its clock set back 36 days, signed in with the old password, and the
  new password was set from an admin terminal. The password no longer expires, and the
  maximum password age is unlimited. Snapshot `04-signed-in` (powered off, 8 GB, clock
  settings restored) has autologon and an elevated task `ArAdmin` that runs
  `C:\ar\admin-job.ps1`. The password is in `C:\VMs\artest-password.txt` on the PC.

### D2. Freeze reproduction (Verifier)

- **Status: PASSED 2026-11-26, one caveat.** 5 of 5 boots passed, no freeze, no bugcheck, on
  a quiet host (other-process CPU mean 2.2 %), but with WSL on (T3 Code restarts it).
  Minute-long VM stalls under the fuzz recovered by themselves. Details:
  [driver sign-once plan](driver-sign-once.md), "What happened (2026-11-25/26)".
  Earlier: freezes on 2026-11-21/22; the 2026-11-22 repeat was aborted because the PC was
  overloaded.
- **Agent can run after D1**, in an Administrator PowerShell in the guest:
  `robocopy Z:\rc-0.2.5-r7 C:\ar\rc-0.2.5-r7 /MIR`, then in that folder
  `powershell -NoProfile -ExecutionPolicy Bypass -File .\run-stage-a.ps1 -Session round2 -Blocks repro -FuzzSeconds 300 -ReproBoots 5 -AutoLogon`.
- **If it freezes over 10 minutes:** keep `round2-state.json`, `Z:\host-load.csv`
  and `VBox.log`, note the boot, then `VBoxManage controlvm AR-DriverTest reset`.
- **Pass:** 5 boots, every fuzz passes, no freeze, `host load` line INFO "host idle".
  A freeze on an idle host is a driver defect and blocks signing.
- **IDs:** SEC-08, VCAB-24, VCAB-30 (A11). **Result:** passed 2026-11-26 with WSL on, accepted
  by the owner the same day: WSL stays on for VM runs (T3 Code uses it), so the
  preflight's WSL check is an expected caveat, not a stop. VCAB-24 stays open (NEM stalls).

### D3. Frame-clock A/B, tagged pool, Verifier and live update

- **Status: A/B FAILED** 2026-11-21 (median 960 → 2692 frames) but the PC was loaded.
  **Live update/remove FAILED** 2026-11-22 (helper exit 5; the route survived).
  **Needs** D2 clean and a fresh restore plus sign-in.
- **Agent can run:** `.\run-stage-a.ps1 -Session round2 -Blocks ab,pool,verifier,live -PoolMinutes 120 -AutoLogon`
  (optionally `-Blocks pause -WithHostPause`).
- **Pass:** both A/B shapes pass (≥ 4 valid runs, median within +480 frames/+1 packet
  and 1.5×); both fuzz rounds pass; no stall over 30 s; helper exit 0 for update and remove.
- **Your decisions after it:** accept 0.2.5 or revert the frame clock as a new RC
  above 0.2.6 (which restarts Stage A); accept "restart required" under a
  playing route, or ask for a new RC. No driver source change before you choose.
- **IDs:** VCAB-05/24/25/30, VDEV-09, SEC-08, NFR-02. **Result:** failed, needs a clean run.
- **2026-11-27 run (WSL on):** A/B **FAILED again on a quiet host** (steady shape: RC3 median
  underrun 0 against 0.2.5 1436 frames; restart shape passes), with a confound: the 0.2.5
  phases that needed a restart ran right after a reboot. The first launch's pool, Verifier
  and live blocks were lost to a watcher defect (no freeze). Relaunch: **pool PASS** (60 min,
  0 MiB/h); **Verifier not judged** (NEM stalls, one reset by the agent, then a runner bug,
  now fixed); **live update/remove: helper exit 5 again**, the route survived, no bugcheck.
  Details: [driver sign-once plan](driver-sign-once.md), "What happened (2026-11-27 night, D3)".
  Your decisions above still stand; the Verifier rounds need their own long window.
- **2026-11-27 afternoon:** Verifier alone: not judged (a 30-minute NEM stall in fuzz round 1,
  no dump). A/B repeat with both drivers restarted alike (`-AbSameStart`): 2 of 6 phases, steady
  runs similar for both, no verdict. Stopped by **restart hangs** with both drivers: Windows
  finished shutting down and VirtualBox did not reset it. See the plan.

### D4. 24-hour tagged pool

- **Decide:** accept the 60-minute pass plus Stage C, or run 24 h with
  `-Blocks pool -PoolMinutes 1440` (**agent can run** after D1; the PC must stay
  quiet for a day).
- **Pass:** MNWR/MNTR/SyAu/AUDR growth ≤ 0.25 MiB/h. **IDs:** VCAB-28. **Result:** open.

### D5. VB-Cable ratio in the guest

- **Agent can run after D1:** install VB-Cable in the disposable guest only, then
  `.\run-stage-a.ps1 -Session extras`. Deferring it to J1 is a valid decision.
- **Pass:** the ratio is recorded (absolute VM latency means nothing). **IDs:** VCAB-25.
  **Result:** open.

### D6. Discord and OBS see the cable (A12)

- **Needs:** D1, a Discord account you allow in the guest.
- **Steps:** `vm-checks.ps1 -Step install`; in Discord choose input
  `AudioRouter Cable B Output`, in OBS add Audio Input Capture on it; run
  `.\tools\m03_bridge_tone.exe --seconds 60 --out C:\ar\a12-render.wav`.
- **Pass:** both meters move. **IDs:** VDEV-11, VCAB-29. **Result:** open.

## E. Release and installer checks

The unattended lifecycle, offline recording after reinstall and the missing-WebView2
online retry passed in the VM on the 0.0.32 bytes. **The missing-WebView2 run as a
whole is FAILED by design:** offline setup exits 2 for a reason not yet identified.

### E1. Publish 0.0.32, or wait

- **Know first:** the 0.0.32 bytes contain a restart bug: after AudioRouter
  restarts, a script that repeats an old Play request with the **same key** is told
  "running" while nothing plays. The window is not affected. The fix is on `main`
  after the plan-archive PR and ships in the next build. No tag, draft or
  release exists.
- **Choose:** publish 0.0.32 with a known-issue note, build 0.0.33 with the fix, or
  wait for signing (B1). Also decide on an opt-in preview group.
- **Steps to publish:** approve the tag; **agent can run** the rebuild with
  `-ReleaseTag v0.0.32` and the draft; you publish; compare the downloaded hash.
- **IDs:** DIST-05/06/07, Q6.5. Source: [0.0.32](../archived/2026-11-24-release-0.0.32.md). **Result:** open.

### E2. Downloaded file, trust prompt and first run

- **Needs:** a GUI VirtualBox guest, a standard user, the installer downloaded by
  the guest's browser (a copied file has no Mark-of-the-Web). The qualification
  guest accounts' passwords are yours.
- **Steps:** check the hash; double-click; copy the prompts word for word; check
  shortcuts; first run and the device question; uninstall with the default choices.
- **Pass:** prompts recorded, no silent elevation, no device change, user data kept.
- **IDs:** DIST-03/07/08, RQ-05, UI-10. **Result:** open.

### E3. Real devices, defaults and volumes

- **Steps:** record defaults and volumes; install; try no device, physical only and
  virtual devices, refuse and allow access; record again; uninstall; record again.
- **Pass:** clear guidance, no substituted microphone, no difference before/after.
- **IDs:** DIST-03, NFR-16, RQ-05. **Result:** open.

### E4. Second Windows build and a reference PC

- Repeat the lifecycle and E2/E3 there. **Pass:** same results.
- **IDs:** DIST-08, M08. **Result:** open.

### E5. Database and recordings on a reference PC

- **Steps:** close the app; copy `state.sqlite`, `state.backups` and recordings to a
  disposable place; install the previous package; Restore on the recovery screen;
  open sessions, play imported audio and a recording; compare WAV hashes.
- **Pass:** everything opens, hashes match, nothing else deleted. **IDs:** Q6.4. **Result:** open.

### E6. Signed installer

- **Needs:** B1. Sign shell, CLI, workers and installer; verify; install on a clean
  machine; run `fresh_install_shell` with `AUDIOROUTER_SHELL_EXE` set.
- **IDs:** VSTC-11, M08. **Result:** open.

## F. Screen, keyboard and Narrator

One build and one disposable profile serve all of these.

### F1. Device unplugged while playing, and sleep

Route mic → headphones, Play, speak, unplug one; then replug, Stop, Play. Then sleep
and resume the PC on a running route. **Pass:** a lasting red alert names the
device, nothing moves, sound returns only after Stop + Play.
**IDs:** UI-04/05/08/17. **Result:** open.

### F2. Cancel a device list with real keys

Properties → Output device: Alt+Down, ArrowDown, **Escape** keeps the saved value;
open again, move, **Tab** leaves without applying; Enter then Save works.
**IDs:** UI-11. **Result:** open.

### F3. Windows scaling: 150 %, 200 % and a narrow 200 % window

At 150 %, 200 % and 200 % with the window about half the screen, check the top bar,
tabs, canvas, Properties, Settings and a playing route in dark, light and high
contrast, and in English, French and Chinese. **Pass:** nothing clipped, no sideways
page scroll, no movement when values change. **IDs:** UI-02/12/17/36, L10N-07. **Result:** open.

### F4. Narrator

Win+Ctrl+Enter. The "Mic to Discord" goal, Apps OBS card, the lost-device alert, tabs
with arrows/Home/End, Settings → API invalid port; then repeat the main screens in
French or Chinese. **Pass:** everything is announced, in the chosen language, and
focus is never lost. **IDs:** UI-02/11, L10N-07. **Result:** open.

### F5. Tray and native dialogs in each language

Settings → Appearance → Language, English → Français → 简体中文: check every tray item and
status line; restart (no English flash); start a copy of the exe from another folder
and check the "already running" dialog, Cancel, then force close once. **Pass:** all in
the chosen language, one backend only. **IDs:** L10N-01/02/03/05/07, UI-10. **Result:** open.

### F6. Language switch during audio

Play a disposable route with real devices and switch languages. **Pass:** the sound and
route are unchanged; no backend error. **IDs:** L10N-03. **Result:** open.

### F7. MCP page look

Settings → AI assistants (MCP): three paths, exact client ID, role, not-authorized and
revoked states, in three themes. **Pass:** correct text, no movement while switching.
**IDs:** AUTO-06/13, UI-17/19. **Result:** open.

## G. Listening, plugins and MIDI

### G1. Plugin editors by eye

AGain, TDR Nova and Surge XT Effects with Test Signal → output: type in a field, Tab
and Escape, resize, Windows scale 100/150/200 %, move between monitors, close and
reopen three times while playing. **Pass:** keys reach the plugin, nothing cropped,
audio continues. **IDs:** VSTC-10, P6, UI-08, PLUG-06. **Result:** open.

### G2. Listen to a VST3 and a 32-bit VST2

Mic or Test Signal → TDR Nova → headphones for one minute; the same with a 32-bit VST2;
compare with bypass; use three frameworks. **Pass:** no clicks, dropouts or added delay.
Note: on 2026-11-24 the fixture route passed 2/2 with the release worker and **FAILED
2/2 with the debug worker** (slow start). **IDs:** VSTC-11, PLUG-03/05. **Result:** open.

### G3. Sidechain ducking by ear

Music → a VST3 compressor with an auxiliary input → headphones; add the sidechain port,
connect the mic, enable key mode, talk, stop, remove the port. **Pass:** speech lowers the
music, silence restores it, removal keeps playing. **IDs:** VSTC-03, P5, GRAPH-14. **Result:** open.

### G4. MIDI keyboard

Instrument (Surge XT or Dexed) with a USB keyboard: notes, sustain, the on-screen keys by
mouse and Enter/Space; change node while a note is held; unplug and replug while playing;
two keyboards with the same name if you have them. **Pass:** no stuck note, no crash,
other audio continues; write down whether notes resume. **IDs:** D1, P8b, UI-08/11. **Result:** open.

### G5. Instrument in the built app

Add Instrument, pick Surge XT, play, open and close its editor, route to headphones.
**IDs:** P8b step 9. **Result:** open.

### G6. PolyMAX sign-in

Note the silence before signing in to UA Connect; sign in; Play a note. **Pass:** audible
after, silent before. **IDs:** D4, P8b. **Result:** open.

### G7. Blue Cat PatchWork, attended only

**Be present, not in a game or call; never unattended.** Set
`AUDIOROUTER_CHAIN_SHOW_EDITORS=1` and `AUDIOROUTER_CHAIN_FILTER=BC`, then
`cargo test -p audiorouter-plugin-host --all-features --test vst2_chain_matrix -- --ignored --nocapture`.
Watch each window, including the single-plugin case; write down the demo dialog.
**Pass:** a window for every member, audio continues, no crash. **IDs:** #136 R2/R3, PLUG-05/06. **Result:** open.

### G8. Licensed plugin that changes latency (not Blue Cat)

You name the plugin and sign in; **agent can run** the C1 command in a quiet window with
`AUDIOROUTER_CONTINUITY_PLUGIN_DB`, `_PLUGIN_SESSION`, `_PLUGINS` (a copy of your session)
and `AUDIOROUTER_PLUGIN_WORKER_PATH`, once active and once bypassed. **Pass:** latency
reported, continuous, audibly different from bypass. **IDs:** PLUG-05/07. **Result:** open.

### G9. Audible Instrument guide video

Record the Properties panel alone in 9:16 with a real instrument; play it in Firefox;
update the manifest. **IDs:** UI-08, AGENTS website rule. **Result:** open.

### G10. Files and plugins you could supply

Give any of these and **an agent can test it**: a third-party VST2 shell that lists its
plugins (VSTC-06, PLUG-02); a vendor-made `.fxp`/`.fxb` (VSTC-05/07); a multichannel-only
VST3 and one that sends `kIoChanged` (P3/P4). **Result:** open.

### G11. Voice chain by ear

Real mic → Denoise → Gate → Compressor → headphones. **Pass:** the processed voice is
audible and clean. **IDs:** UI-08 R3/R7. **Result:** open.

### G12. Mute and Fun Voice Changer while recording

Play and record a disposable route; toggle Mute and Fun Voice Changer quickly. **Pass:** no
gap in the recording. **IDs:** GRAPH-08/09. **Result:** open.

## H. OBS, Discord and assistants

| ID | Do | Pass | IDs |
| --- | --- | --- | --- |
| H1 | Copy `%APPDATA%\obs-studio` first. OBS → Tools → WebSocket Server on; Apps → OBS → Connect. Fix in OBS with one change unticked; Restore; change a source and apply again; switch scene collection in the preview | Only ticked changes; exact restore; the changed source is refused; the preview cancels; no password shown | SEC-15, UI-26, CAP-15, DEC-22 |
| H2 | From an enrolled Observer assistant, call `get_obs_status` | Facts match OBS; no password returned | SEC-15, AUTO-06 |
| H3 | Apps page against your real Discord and Windows Sound, in and out of a call; chips, tooltip, "Used by" footer | Names match, no false warning, nothing moves | UI-17/26, CAP-15 |
| H4 | Discord Voice & Video: microphone is the cable AudioRouter feeds; Noise Suppression None, AGC and Echo off; talk with a strong effect | Card says Ready; the processed voice is heard | UI-26, CAP-15 |
| H5 | With Discord, Siege and Outplayed running: `audiorouter-cli.exe --json applications list`, compare with Volume mixer, join/leave a call, repeat | IDs, directions and active state match | CAP-15, UI-26 |
| H6 | Your real profile: Settings → AI assistants (MCP); change no grant; one read call from your assistant | Client ID and role are correct; the read works | AUTO-06/13, SEC-14 |
| H7 | Disposable profile: enroll each role, one default tool each, revoke a live client | Refusals match the role; the revoked client loses access | AUTO-06/18 |

**Result:** none recorded. Sources: [OBS and Discord](../archived/2026-11-24-obs-discord.md),
[MCP setup](../archived/2026-11-24-mcp-setup-ux.md).

## I. Other people

### I1. Five first-time users

Follow [the script](evidence/2026-11-24-ui-quality-participant-study.md) word for word.
**Pass:** at least 4 of 5 finish within ten minutes; all 5 identify what Discord receives.
**IDs:** M08, UI-03/09/10/11/28. **Result:** open.

### I2. Fluent French and Simplified Chinese review

A fluent reader marks every row in [the review queue](evidence/2026-11-24-fluent-review-queue.md)
accept, correct or open; **an agent can** apply corrections and run `i18n:check`. Safety,
permission and delete wording matters most. **IDs:** L10N-04/06/08. **Result:** open.

### I3. Independent UI reviewer

Score the categories in the [UI review](../archived/2026-11-24-ui-quality-review.md); a 10
needs someone who did not build it. **Result:** open.

### I4. Reporters

- **#149:** on the reporter's PC with 0.0.30 or later and Clear, ERA6 and Fun Voice
  Changer: play and record both outputs, toggle Mute and the voice changer quickly, edit
  during a save, open and close editors. **Pass:** no gap, no new worker, no conflict lock.
  IDs: GRAPH-08/09, UI-07, STATE-02/05/06, PLUG-01/03/04.
- **#136: BLOCKED.** Ask for plugin names, versions, scaling and a support bundle. IDs: I136-R1/R2.

## J. Driver after signing

### J1. Stage C on this PC

**Needs** B1, a restore point and the Safe Mode removal path. Run the Stage C matrix in the
[virtual cable plan](../archived/2026-11-24-virtual-cable.md): 1 h with 8 cables and load;
latency p95 ≤ 20/40 ms and VB-Cable ratio ≤ +5 ms; DPC < 20 µs average, < 200 µs max;
96 kHz sweep within 0.05 dB; the microphone route. **IDs:** VCAB-22/24/25/27/28/30, NFR-02. **Result:** open.

### J2. Secure Boot and Memory Integrity (Stage B)

Only you change boot and security settings. With the signed package and both on, install
and play. **Pass:** loads and plays, no HVCI block. **IDs:** SEC-08. **Result:** open.

## Finished

None yet.
