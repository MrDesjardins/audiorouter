# Frontend and app-only qualification, 2026-11-27

## Scope and environment

Owner selected the spacing/error port, accessibility checks and fresh app-only
qualification. No driver work, PR, tag or publication. The owner explicitly
deferred the eight-hour soak. The owner closed the live app before native checks.

Windows 11 Home build 26300, Node 22.22.3, pinned Rust 1.96.0, WebView2
158.0.4370.45. Branch: `improve/ui-accessibility-app-qualification`, based on
`2f24ad4364eb1a22bbc88f5cb51a0c33070f37b0`. Build/test evidence is under
`C:/code/audiorouter/target/ui-app-qualification/`. Builds are local qualification
bytes at version 0.0.32, not the earlier 0.0.32 candidate and not a release.

## Changes

- Ported `ea4398e38` and `d61bfb7d4` onto current code without restoring obsolete
  plan history or overwriting localization, error severity or forced-Quit handling.
- Dialogs scroll inside the window. Panels and button/toggle rows retain gaps;
  chart labels avoid collisions. No audio behavior, processing setting or API changed.
- Backend failures are readable sentences with recovery guidance; shell string
  rejections use the same formatter. Technical codes remain in diagnostics.
- English/French/Chinese catalogs cover the new messages, with runtime translation
  on language switches. Corrected the old port's claim that all 32-bit plugins are
  unsupported and its incomplete explanation of idempotency conflicts after restart.
- Unknown error codes cannot select inherited JavaScript object properties.
- Added three-language/theme/viewport layout qualification and the original
  dialog/panel measured-spacing audits.

## Build-hook defect and repair

The first NSIS invocation exited successfully but printed the PowerShell hook's
program instead of running npm (`installer-build.log`). `ui/dist/index.html`
remained at 23:21:13 UTC despite the later build. That run's executable timestamp
therefore did not establish fresh embedded UI; its initial smoke result is retained
but superseded for final artifact qualification.

Replaced the inline hooks with `tools/run-ui.ps1`. The first repair used the wrong
working directory and failed explicitly (`installer-rebuild.log`); the actual
Tauri frontend directory is `ui/`. Corrected the relative path and made the
regression execute from that directory. Four cases cover build/dev, success and
exit 7, using a fixture root with spaces. The shared `release-tests` inventory now
runs them. `installer-hook-fixed.log` shows npm/typecheck/Vite actually running
and updating the UI before Rust embedding. No old installer is offered as the
final candidate.

## Verification

- `npm --prefix ui run typecheck:all`, `lint`, `format`, `format:check`,
  `i18n:generate` and `i18n:check`: passed. 3 catalogs, 3,803 messages.
- `npm --prefix ui test`: **1,210 passed in 137 files** in the final complete run
  (`ui-unit-complete.log`). Earlier failures were changed capitalization/punctuation
  expectations and a doubled Quit full stop, repaired with regressions.
- `npm --prefix contracts run check:drift`: passed; 156 methods, 38 node kinds,
  20 processors, 24 event categories. No API/contract change in this work.
- Compiled Edge checks via `playwright.ci.config.ts --workers=1`: **108 passed,
  3 skipped**, zero flaky/unexpected cases (`browser-results.json`). Includes
  dialog/panel spacing, every tool's Properties at two widths, native-select
  keyboard paths, Cowork roles/revocation, language switching and combined UX.
  The skips are development-only `en-XA` cases absent from production bundles.
- `scaled-layout-matrix.pw.ts`: **27 passed**, zero skips/flakes
  (`scaled-results.json`): three shipped languages, three themes, CSS viewports
  960x666, 720x500, 480x500, horizontal overflow and Home/End tab navigation.
  This is a viewport equivalent, not actual Windows display scaling evidence.
- Focused `audio-tools.pw.ts` desktop-workspace/MCP case: **1 passed**
  (`mcp-results.json`); verifies three setup paths, IDs, role states and stable
  panel geometry, retaining three-theme screenshots under `mcp-ui/`.
- Expanded final panel audit: every tool's Properties in all three themes at
  1500x950 and dark at 1024x700, plus sidebar/Settings panels. Chart screenshots
  retained under `panel-theme-ui/`; inspected Compressor in the three themes.
  **8 passed**, zero skips/flakes (`panel-theme-results.json`).
- The initial development-server browser run had **84 passed, 1 timeout** while
  waiting for the command palette after navigation. Source/catalog regeneration
  occurred during that run. Cause not established; retained trace in the initial
  Playwright temp output. Compiled, fixed-input run above passed that same case.
  No product workaround or timeout increase was introduced.
- `cargo test -p audiorouter-control --lib recorder_play_tests --locked`:
  **10 passed**, through `ControlPlane::dispatch`, including old-key replay after
  restart (`api07-rerun.log`). The first shell wrapper reported exit 1 from native
  stderr despite Cargo's 10-pass result; rerun preserved Cargo's actual exit 0.
- `cargo test --manifest-path src-tauri/Cargo.toml database_recovery --locked`:
  **2 passed**, corruption preservation, permission reset and invalid backup refusal
  (`database-recovery.log`). These are fixtures, not a reference-PC rollback.
- `node tools/quality/verify.mjs --checks release-tests --keep-going`: passed,
  including artifact/path-safety, app-only profile, lifecycle host guards,
  recording verifier and four hook regressions (`release-checks.log`). Expected
  injected recorder-cleanup warnings belong to the fault fixtures.
- `node --test tools/quality/policy.test.mjs tools/quality/verify.test.mjs`: passed.
- Stream Deck typecheck, **35 tests**, build, validation and pack: passed.
- Complete desktop build includes shell, CLI and x64/x86 VST2/VST3 workers.
  App-only NSIS uses static CRT flags for Rust binaries; PE architecture and
  portable-runtime import checks pass for all six executables. Installer unsigned.
- Native release-shell smoke uses a disposable database and WebView profile,
  Medium integrity/non-admin, denied device access, no audio and graceful cleanup.
  It verifies bridge, canvas, save/reload/restart and no uncaught UI errors.
  Results before the hook fix remain separate from `hook-fixed-packaged-smoke/`.
  Final-byte result: `installer-final-smoke/result.json`, shell SHA-256
  `64f82213d6377f58e260cadf7dba400ad6efec2d9a624ef58a20d6b48eb69e5f`.
  Fresh-profile UI ready 1,002 ms; restart 1,352 ms. These are individual samples,
  not OS-cold measurements or p95 qualification. No error outcomes in its backend
  log, no uncaught UI errors, both owned shell processes exited 0.
- Shared fast profile: docs, policy, Rust formatting, contracts typecheck,
  UI formatting/lint/typecheck and Stream Deck typecheck passed (`fast-checks.log`).
- Manual visual inspection of the compiled browser's Appearance in dark/high
  contrast and workspace in light showed themed fields, visible focus and panel
  spacing. Browser preview closed afterwards.

## Warnings retained with reasons

Vite reports 604 kB application and 956 kB catalog/shared-message chunks. This
work adds translated errors but does not redesign catalog loading/chunking;
the startup smoke supplies measured local baselines, not a responsiveness gate.
Rolldown's `use no memo` warning concerns a third-party runtime directive; no
React compiler is configured here. Neither warning is hidden or promoted to a
pass. Review chunk loading in a separate performance slice before release.

Development React Flow warns about hidden attribution; license entitlement was
not verified in this task. NO_COLOR/FORCE_COLOR warnings are runner environment
configuration. Stream Deck Rollup reports a misplaced third-party Zod purity
annotation and strips it; typecheck/tests/package validation passed. No lint
warnings remain in changed UI source.

## Human and environment boundaries

- F1 device unplug/sleep and F6 language switch during real audio: not run.
- F2 real Edge keyboard paths pass; full attended native dropdown acceptance
  remains open. F3 actual Windows 150/200% scaling and F4 Narrator are not replaced
  by viewport or accessibility-tree checks. F5 native tray/dialog language and
  single-instance checks remain open. French/Chinese fluent review is unchanged.
- E2 trust/MOTW, E3 actual devices/defaults/volumes, E4 a second supported Windows
  build, E5 reference-PC package/database/recording rollback and E6 signing remain
  open. The local installer is not publication-ready or clean-checkout provenance.
- The eight-hour real-route soak is deferred by the owner, not passed.
- App-release VM UUID `87c31665-2022-49a2-a182-1a1ebb5e631c`, 4 CPUs/8 GiB,
  host audio disabled. Powered-off state preserved as snapshot
  `18bfcb33-e4fd-432d-a92c-293fbed2ff3e` before baseline restoration. Initial
  baseline boots to artest's expired-password prompt. No password was printed.
  Existing host credential file authenticates arqual4 through guest-control,
  but no signed-in arqual4 desktop was initially available.
  Owner first deferred VM work, then allowed it again without supplying a password;
  the owner then signed in arqual4. Windows computer-use helper failed its pipe
  connection twice and again after reset. No credential or security setting changed.

## Final artifact and VM lifecycle

Local installer (0.0.32, unsigned, app only):
`target/ui-app-qualification/artifacts/AudioRouter_0.0.32_x64-setup.exe`, SHA-256
`5c76b8a4f11897395a5dfe437645215e6ee3de3f7bc600280297d24da1bf8014`.
It includes the freshly rebuilt UI, six architecture/runtime-checked executables,
the rebuilt/validated Stream Deck package and verified pinned tunnel runtime.
Locked Cargo/UI/Stream Deck metadata, notices and checksums accompany it. It is a
local qualification candidate, not a tagged clean-checkout release or publication.

After the owner signed in, `tools/vm/installer-lifecycle.ps1 -ProductionRecording
-WebViewBaseline Present` passed on this exact installer and the hash-verified
retained 0.0.29 fixture (`f2c43d115f04fdef00f55ef96862eba41d581464db24785cde5f69755ba209d8`).
Evidence: `vm-lifecycle/result.json`, `steps.jsonl`, bounded stdout/stderr and
`vm-lifecycle.log`. Guest work took about 50 seconds. Passed: preflight,
WebView baseline, fresh install, WebView after setup, seed data, repair, rollback,
upgrade, uninstall and reinstall. WebView2 stayed at 158.0.4370.45.

The installed production Recorder generated a 440.0025 Hz stereo 48 kHz file,
96,768 frames, RMS 0.089016; SHA-256
`af865686541768f0355ab746cee7c470dfdbfcec6fe4abd0270beff7c74bebb0`
survived the lifecycle unchanged. This is synthetic production-engine audio,
not the owner's microphone or a device-latency test. User data was retained.
The release VM was then shut down and restored to its pre-session snapshot
`18bfcb33-e4fd-432d-a92c-293fbed2ff3e`, confirmed powered off. Driver VM untouched.

Deviation: previous fixture is 0.0.29, as in the prior qualification, not 0.0.30/31.
The known baseline's possible Defender exclusion remains unverified; no exclusion
was added or removed. This is not clean-Defender, trust-prompt/MOTW, missing-WebView2,
second-Windows-build or reference-PC recovery evidence. Those gates stay open.

## Native accessibility continuation after owner "Go"

Extended the guarded packaged smoke with opt-in `--accessibility true` and an
attended physical-key mode, `--manual-keyboard true`. This changes qualification
tooling only; the installer and executable bytes above remain unchanged.
Windows desktop control again failed with "native pipe is unavailable".

`target/ui-app-accessibility-native-diagnostics/result.json` is **failed**, not
a passed F2 gate: CDP Alt+Down/ArrowDown/Escape retained a changed unsaved value,
and CDP Tab did not move focus off the picker. In both cases the real backend
session remained unchanged. The cause is not established; physical keys must
distinguish native-popup automation limitations from a product defect. No guessed
fix was made to the app's device picker.

Independent checks in that same run passed: explicit Save persisted an exact
real output endpoint through the backend; English, French and Simplified Chinese
were written to the actual native settings file, survived page reload, and kept
the backend route identical while switching among three themes. The route
stayed stopped and device access denied. Process restart/save and cleanup also
completed. This is not live-audio switching, native tray, Narrator or an OS-scaling
pass. Screenshots are retained in that output folder.

Runner syntax checks and **81 policy/verification/opt-in-validation tests passed**
(`target/ui-app-qualification/accessibility-runner-tests.log`). Three new CLI
regressions ensure invalid manual/accessibility flags fail before shell access.
The attended run uses `target/ui-app-accessibility-attended/` and requires explicit
owner confirmation at each physical-key step; no confirmation is inferred.
The Escape phase received no confirmation within three minutes, so the attended
run is **failed/incomplete**, with graceful owned-process exit and no cleanup
failure. No physical keyboard pass is claimed. `manual-escape-ready.png` and
`result.json` preserve the starting screen and timeout outcome.
The owner subsequently chose **Skip physical keyboard test**; do not repeat that
attended step without a new request. The ordinary smoke still passes with no
accessibility flags (`target/ui-app-accessibility-baseline/result.json`).
Shared `release-tests` passed again with all seven hook/opt-in CLI cases, artifact
checks and VM-free lifecycle/recording guards (`accessibility-release-checks.log`).
Documentation validation and the complete diff check passed. All owned shell
processes are closed; no installed-app or installer bytes were changed.

## Next action

Attend F1-F6 on the fresh app, then perform clean-Defender/MOTW and second-PC
qualification. Keep the source branch for owner testing/review. Run no eight-hour
soak now; no PR or publication is authorized.
