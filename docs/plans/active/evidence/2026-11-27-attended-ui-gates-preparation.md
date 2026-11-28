# Attended UI gate preparation, 2026-11-27

## Scope and authority

Owner requested the remaining screen, keyboard, scaling, Narrator, tray and
live-language gates for the frontend/app qualification. This record prepares and
independently checks them on a fresh build. It does not claim an attended pass.
No driver work, PR, publication, live owner audio, OS scaling change, sleep or
Narrator session was performed. Physical-key F2 was explicitly skipped by the
owner and is not repeated.

Branch `t3/frontend-accessibility-qualification` at
`c27f6cfd196783a3e4ed87f4dee76b8dc378d1c8`, based on the integration of
`improve/ui-accessibility-app-qualification` with `origin/main` (A5-A9 included).
Evidence root:
`target/attended-ui-qualification-2026-11-27/`.

## Environment and fresh build

Windows 11 Home 26H2 build 26300, Node 22.22.3, pinned Rust 1.96.0, WebView2
158.0.4370.45. At audit time: display scale 100% (`GetDpiForSystem` and
`AppliedDPI` both 96), Windows app theme dark (`AppsUseLightTheme=0`), Narrator
installed with a saved profile but not running, and no AudioRouter shell process.
The owner's database and WebView profile were not opened or changed.

`tools/build-desktop.ps1` completed with exit 0 (`build.log`). The UI bundle is
newer than the last UI source change and the shell is newer than `ui/dist`:

- `ui/dist/index.html`: 2026-11-27T20:57:45-08:00
- `audiorouter-shell.exe`: 2026-11-27T20:59:17-08:00

| Binary | Bytes | SHA-256 |
| --- | --- | --- |
| `audiorouter-shell.exe` | 27,774,976 | `020389EC90249A9A4D426BECC2683C9F63FB88479C2D152E435F151B00D83CC3` |
| `audiorouter-cli.exe` | 17,942,528 | `5B008F69B11F04CB5DFA95673A58C7D63427BA667B8E9C9786CD3B18761864EA` |
| `audiorouter-plugin-worker.exe` | 1,289,728 | `A92F0C8233055E24D7D14DCAF71F9F85781D97028FC5D94332B5E001B25DCDEB` |
| `audiorouter-vst3-worker.exe` | 503,808 | `041E305D671C0AFC2553453B3994EEC163A91835FDD02ED78E9B69C19BB3E9C3` |
| `audiorouter-plugin-worker-x86.exe` | 1,052,672 | `E4E157366B6BBC177A6AD43C206C7529E3A61FE4BDD5FF63ED67A02E5E3DFACE` |
| `audiorouter-vst3-worker-x86.exe` | 417,792 | `1997C9BBD9B2821478FEEDDE37AE2589D3477E94609BC4F1B219138B719FDAFB` |

This is a local unsigned shell build, not the recorded 0.0.32 installer. The
installer at `target/ui-app-qualification/artifacts/AudioRouter_0.0.32_x64-setup.exe`
(SHA-256 `5c76b8a4...f8014`) predates the A5-A9 merge and was not used for this
attendance preparation. No installer, release or publication claim is made.

## Prior evidence audit

| Gate | Existing evidence | What it does not cover |
| --- | --- | --- |
| F1 unplug/sleep | Owner unplug/replug pass on the 2026-11-08 attended exe (red message, sound returned). 2026-11-27 PD200X capture-only exact-endpoint loss/recovery passed through Windows notifications (`devices.changed`, `bindingInvalidated`, `runtime.stopped/started/recovered`), graph unchanged. Red, non-expiring lost-device alert regression in `App.accessibility.test.tsx`. | No current desktop-UI unplug, no actual sleep/resume, no audible check. Prior UI pass predates A5-A9. |
| F2 cancel device list | Owner skipped the physical-key test. Current build reproduces the known CDP result: Escape retained a changed draft value and Tab did not leave the picker; the backend session stayed unchanged. | Do not repeat without a new owner request. |
| F3 Windows scaling | Owner reported 150% and 200% pass on the 2026-11-07 Round 28 exe. Current 27-case viewport matrix passes. | No actual OS scaling on this build, no narrow half-screen 200% window with a playing route, no current French/Chinese scaled visual check. |
| F4 Narrator | Owner's 2026-11-07 Narrator tab pass (names read, Tab order sound) on an older exe. | No Narrator run on this build, no translated announcement check, no lost-device alert announcement. |
| F5 tray/native dialogs | Tray items owner-confirmed on the 2026-11-07 short menu. Shell-owned text moved into the catalogs on 2026-11-24. | No current tray or native "already running" dialog inspection; restart/no-English-flash and single-instance behavior unrun. |
| F6 language during audio | New requirement. Packaged shell persisted English, French and Simplified Chinese to the native settings file, reloaded each language, and left the backend graph unchanged while the route was stopped. | No live route, no audible continuity, no switch during actual playback. |

## Independent qualification on this build

All commands ran from the repository root unless noted. No endpoint was opened
and no audio was started.

- `npm --prefix ui run i18n:check`: 3 catalogs, 3,803 messages, generated
  native text current. A direct key-set audit found 47 `messages.native.*` keys
  in each of `en`, `fr` and `zh-Hans`, with no key identical to English.
- `npm --prefix ui run typecheck:all`, `lint` and `format:check`: passed, zero
  lint warnings.
- Focused Vitest set (`App.accessibility`, `UiOptionsWorkbench.language`,
  `backendErrors`, `actionMessage`, `useWorkspaceEvents`, `preferences`):
  **130 passed** in 6 files.
- `npm --prefix contracts run typecheck`: passed.
- `scaled-layout-matrix.pw.ts` on Edge, one worker: **27 passed** (English,
  French, Simplified Chinese; dark, light, high contrast; 960x666, 720x500,
  480x500 viewport equivalents; no horizontal overflow; Home/End tab focus).
  This is a CSS viewport equivalent, not Windows display scaling (`scaled.log`).
- Packaged shell baseline smoke with a disposable database/profile:
  **passed** (`smoke-baseline/result.json`). Standard-user token (integrity
  0x2000, not administrator), cold UI ready 961 ms, warm ready 1,521 ms, save
  76 ms, no uncaught UI errors, both owned processes exited 0.
- Packaged accessibility smoke, opt-in mode (`smoke-accessibility/result.json`):
  **failed only the known F2 CDP probes**. Independent checks in the same run
  passed: explicit Save persisted one of **9 real output endpoints**; English,
  French and Simplified Chinese were written to the native settings file,
  survived reload, and left the route identical; the route stayed stopped and
  device access denied. This is not tray, Narrator, scaling or live-audio
  evidence.
- Rust focused suites: `native_language_matches...` **1 passed**; `tray_`
  **19 passed**; `os_transition` **8 passed**; `instance_` **3 passed**;
  `-p audiorouter-control --lib device_loss` **20 passed**, including simulated
  sleep/resume and recorder-arm cases. These exercise logic, not physical
  devices or OS power transitions.
- Static trace of the live-language path: `App.changeLanguage` updates React
  state, the local preference and i18n, then calls only the shell
  `setUiLanguage` command. It does not call a session read, graph commit, Play
  or Stop. This is code-path evidence for F6, not audible continuity.
- `node tools/docs/validate.mjs`: passed before this record was added.

## Prepared attended session

One fresh shell and one disposable profile serve every step. Estimated owner time
is 50-70 minutes. Skip F2. Stop and report at the first reproduced defect; do not
edit the build or the owner's profile during the session.

### Preconditions

1. The owner closes AudioRouter and confirms
   `Get-Process AudioRouter,audiorouter-shell` shows nothing.
2. A microphone and headphones are connected. Discord and OBS are not needed.
3. Record the starting Windows scale (currently 100%), app theme (currently
   dark), interface language and Narrator state. Restore them afterwards.

### Launch with isolated state

Run in one PowerShell window and keep it open until the app quits. This leaves
the owner's `%LOCALAPPDATA%\AudioRouter` database, logs and WebView profile
untouched. It deliberately does not set `AUDIOROUTER_ALLOW_DEVICE_ADMIN`; the
owner answers the normal device-access prompt in the window.

```powershell
$repo = 'C:\Users\miste\.t3\worktrees\audiorouter\t3-frontend-accessibili-2e6825432b19c00d'
$out  = Join-Path $repo 'target\attended-ui-qualification-2026-11-27\owner-session'
$shell = Join-Path $repo 'src-tauri\target\release\audiorouter-shell.exe'
New-Item -ItemType Directory -Force $out, "$out\localappdata", "$out\appdata", "$out\webview", "$out\shots", "$out\copy" | Out-Null
$env:AUDIOROUTER_DATABASE = Join-Path $out 'state.sqlite'
$env:LOCALAPPDATA = Join-Path $out 'localappdata'
$env:APPDATA = Join-Path $out 'appdata'
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $out 'webview'
Remove-Item Env:AUDIOROUTER_ALLOW_DEVICE_ADMIN -ErrorAction SilentlyContinue
Remove-Item Env:AUDIOROUTER_CONTROL_PIPE -ErrorAction SilentlyContinue
Get-FileHash $shell -Algorithm SHA256
Start-Process -FilePath $shell
```

Expected shell SHA-256:
`020389EC90249A9A4D426BECC2683C9F63FB88479C2D152E435F151B00D83CC3`.

### 1. F4 Narrator, English (about 15 minutes)

1. Press **Win+Ctrl+Enter** to start Narrator. Keep it running through step 6.
2. On the fresh goal picker, listen for the heading and the "Mic to Discord"
   goal; open Status and the **Apps** view; listen for the OBS card.
3. Tab to the main tabs and use Left/Right, Home and End. Confirm each tab name
   is announced and focus is never lost.
4. Settings > API: type `99999` in Port. Confirm Narrator reads the valid range
   and that leaving the field restores the old port.
5. Record exact announcements that are wrong or missing.

Pass: every listed item is announced with a useful name, focus stays logical,
and nothing in step 4 is silent.

### 2. F5 tray and native dialogs (about 15 minutes)

1. Settings > Appearance > Language: switch **English -> Français -> 简体中文**.
   After each switch, right-click the tray icon and read every item and status
   line. Expect the chosen language, with no English remnant.
2. Quit from the tray, then in the same terminal run
   `Start-Process -FilePath $shell` and watch startup. Expect no English flash
   before the chosen language.
3. In the same terminal, run
   `Copy-Item $shell "$out\copy\audiorouter-shell.exe" -Force` and then
   `Start-Process "$out\copy\audiorouter-shell.exe"` while the first instance
   runs. Expect the "already running" dialog in the chosen language. Press
   **Cancel**; the first instance must keep running.
4. Start the copy once more and choose **Force close old instance and continue**
   once. Confirm exactly one backend/window remains.
5. Quit the copy, relaunch the original with `Start-Process -FilePath $shell`,
   leave the app in English, and use the original for the following steps.

Pass: tray, startup and the single-instance dialog all use the chosen language;
only one backend runs; Cancel changes nothing.

### 3. F3 Windows scaling (about 20 minutes)

1. Windows Settings > System > Display > Scale: set **150%**. Return to
   AudioRouter. Check the top bar, tabs, canvas, Properties, Settings and the
   Appearance page in dark, light and high contrast.
2. Set **200%** and repeat. Resize the window to about half the screen width at
   200% and repeat. Confirm there is no sideways page scrollbar and no clipped
   text or controls.
3. In French and Simplified Chinese at 200% narrow, check the same surfaces.
4. With a route playing (or when returning here after F6), verify no control
   moves when live values change.
5. Restore Windows scale to **100%**.

Pass: nothing clipped, no sideways page scroll, no movement when values change,
in all three themes and languages.

### 4. F1 device unplug and sleep (about 10 minutes)

1. Build a disposable microphone -> headphones route and press Play. Allow the
   device-access prompt when it appears.
2. Speak, then unplug the output (or the microphone). Expect a red alert that
   names the device and stays; the route must not select a substitute device.
3. Reconnect the same device. Current approved RES-3 policy restarts the exact
   saved route automatically for up to two minutes; if it does not, press Stop
   and then Play. Record whether it recovered automatically.
4. Let Narrator read the alert; record its wording and language.
5. With the route playing, sleep and resume the PC. Record whether audio
   resumes, stops, or offers recovery, and whether any alert appears. Do not
   treat a simulated failure-interleaving test as this result.

Pass: no silent microphone substitution, loss is named on screen in the chosen
language, the saved route is unchanged, and sound returns only for the same
exact devices.

### 5. F6 language switch during audio (about 5 minutes)

1. With a disposable route audibly playing, open Settings > Appearance.
2. Switch **English -> Français -> 简体中文 -> English** while listening and
   watching the route.
3. Record any gap, click, route change, or error message.

Pass: sound and route are unchanged across every switch and no backend error is
shown. A stopped-route reload is not this result.

### Evidence capture

- Save screenshots to `$out\shots\` (for example `f3-200-fr-dark.png`,
  `f5-tray-zh.png`, `f1-alert-fr.png`).
- After quitting, collect the isolated logs:
  `Get-ChildItem "$out\localappdata\AudioRouter\logs"` and search
  `backend.jsonl` and `shell.jsonl` for `"outcome":"error"`.
- Record the final Windows scale, theme, language and Narrator state, and restore
  the starting values.

## Gate status

F1, F3, F4, F5 and F6 remain **open**. The independent checks above reduce risk
but do not replace actual Windows scaling, Narrator speech, tray/dialog
observation, physical unplug/sleep or live-audio listening. F2 remains skipped by
owner decision and was not re-run beyond the existing CDP diagnostic.

The A5-A9 recovery policy allows bounded automatic restart of the exact saved
endpoint, so F1 must record whether recovery was automatic; "sound returns only
after Stop + Play" is only the fallback after the two-minute window.

## Next action

Owner attendance is required for one 50-70 minute quiet window using the launch
block above. After the session, record the per-gate outcome and screenshots in
this file, restore the Windows scale/theme/language/Narrator state, and only then
decide whether any reproduced defect needs a regression and three-theme fix.
