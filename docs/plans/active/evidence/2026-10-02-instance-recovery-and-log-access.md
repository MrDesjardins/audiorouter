# Instance recovery and log access — 2026-10-02

Requirements: UI-13, API-09, SEC-01/05/10, ENG-05. User authorizes clearer
existing-instance UX, a button to kill the old app and convenient log navigation.
Windows x64, Edge, installed Rust/Node. No VST implementation changed.

## Implementation and decisions

- Desktop ownership is claimed before storage, enrollment, backend supervision
  or RPC forwarding. An existing same-user/same-Windows-session AudioRouter
  shell triggers the Windows startup dialog: “There is already an instance
  running. Please close it.” Cancel leaves the existing app untouched.
- “Force close old instance and continue” explicitly says audio stops and
  unsaved changes/unfinished recordings can be lost. Cancel is the default.
  Owned process handles pin the verified identity during the dialog; no PID
  comes from the UI. Other users, other app/worker names and this process are
  ineligible. Another Windows session requires manual closure there. Access
  denial/timeout gives recovery instructions instead of a success claim.
- A named desktop mutex prevents simultaneous updated shells starting a
  backend; process inventory also recognizes older shells without that mutex.
  External-pipe test shells retain their explicit separate lifecycle.
- Already-attached audio workers within one backend get preparation/retry
  guidance. That error alone does not imply another Windows process.
- Logs has a prominent support card with Open logs folder, Copy folder path,
  selectable read-only path, current/previous file names and reproduction-time
  instructions. Explorer receives only the fixed app-owned log path; commands
  accept no external path. Clipboard/Explorer failure has manual fallback.
  Browser previews explain that local files require the desktop app.
- Startup check outcome is recorded as a bounded `shell.instanceCheck` event;
  arbitrary process names/paths or native error text are excluded.

Native UI uses the existing Tauri common-controls-v6 application manifest;
[Microsoft TaskDialog documentation](https://learn.microsoft.com/en-us/windows/win32/api/commctrl/nf-commctrl-taskdialog)
describes the system dialog used here. It appears before a WebView exists and
uses Windows appearance/accessibility; the Logs card follows the app themes.

## Verification

| Command / evidence | Result |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 43 passed, 1 attended HTTP fixture ignored |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked instance_windows` after session-boundary change | 2 passed; read-only native inventory, identity exclusion policy |
| `npx.cmd tsc --noEmit -p .` (ui) | Passed |
| `npx.cmd vitest run src/LogFilesPanel.test.tsx src/backend.test.ts --configLoader runner` | 68 passed; open/copy, failure fallback, browser preview, worker wording |
| `npm.cmd run build` (ui) | Passed, chunk-size advisory |
| Production harness build and `npx.cmd playwright test --config playwright.usability.config.ts --grep 'logs folder controls'` | 3 passed in Edge; open/copy dispatch, path, field style and viewport bounds |

1280×720 screenshots visually inspected after final support-card spacing:
[dark](2026-10-02-instance-ux/dark-logs.png),
[light](2026-10-02-instance-ux/light-logs.png),
[high contrast](2026-10-02-instance-ux/high-contrast-logs.png).
Field captions are stacked; global 9 px field styling remains intact.

Initial compile caught duplicate log command insertion and a Windows function
namespace error; both were corrected before passing checks. Initial UI tests
used unavailable jest-dom matchers; standard DOM assertions pass on rerun.

## Build and limitations

The isolated review folder is rebuilt with identity
`instance-ux-20261002-review`, still package version 0.0.2 and uncommitted source.
Folder: `C:\code\audiorouter\target\instance-ux-20261002\release`.
It is separate from the earlier diagnostics-only review folder.
Build commands passed with `AUDIOROUTER_BUILD_ID=instance-ux-20261002-review`:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --release --locked --features custom-protocol --target-dir target/instance-ux-20261002
cargo build --release --locked -p audiorouter-cli -p audiorouter-plugin-host --bin audiorouter-cli --bin audiorouter-plugin-worker --target-dir target/instance-ux-20261002
```

All executables pass the repository's x64 PE validator. Shell timestamp
19:15:29 UTC is newer than the rebuilt UI. Binary inspection confirms build ID,
current `index-CvjdloOm.js` / `index-Cx-j8Cuv.css` assets and common-controls
manifest. The source tree remains dirty; no release commit is invented.

| Executable | Bytes | SHA-256 |
| --- | --- | --- |
| audiorouter-shell.exe | 23178240 | B9AAFB209EF5A403F5FA58C6B001EA0CA2871CC1BA9C64C1C59B4C4C2F3FA5FE |
| audiorouter-cli.exe | 13540352 | 0D9ACB0B83FF73AFD41FD5E6348ED42CD3C2A1E4D665AE29205736F90CF6329C |
| audiorouter-plugin-worker.exe | 968192 | 10C16188FE7957CEF540A67630685F40E4DEB4261CE6CAF73851C7E981406128 |

Documentation validation: 89 Markdown files, 477 local links; diff check passed.
No new tag, installer or publication is implied. The existing user shell
PID 58604 was initially preserved; the user then opened the earlier review
as PID 74480. Rebuilding its path failed with OS error 5 because it was in use,
so the final build moved to the new folder. No second production shell was launched and no
user process was terminated. Actual force-close/continue, native dialog
appearance and Explorer launching need attended verification; policy tests
and browser invoke mocks are not evidence of those actions.

Fresh-install and native hardware gates remain open. Rollback uses the
previous installation; storage schema is unchanged. Next: review startup
recovery on a disposable app instance, open Logs in the native shell, then
resume M08 release reconciliation.
