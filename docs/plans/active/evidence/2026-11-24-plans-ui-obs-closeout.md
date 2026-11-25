# OBS/Discord and UI quality close-out (2026-11-24)

Branch `plans/ui-obs`, base `02ca8dc9d`; code/test commit `d7228d849`.
Environment: Windows host, Node 22.22.3, npm 10.9.8, Rust/Cargo 1.96.0,
Microsoft Edge through Playwright (`channel: msedge`), free port 4186. `npm ci`
was run in `ui/` and `contracts/`. No AudioRouter shell, OBS or Discord process
was launched; no device, driver, database or owner profile was touched.

## What was done

- Added a one-shot workspace-event fixture to the disposable route harness
  (`__routeFixtureWorkspaceEvents`) so a browser test can raise a real
  `devices.bindingInvalidated` event on a live route.
- Added `ui/e2e/device-loss-message.pw.ts`: three themes, a live route, the
  lost-device message is an assertive red alert, lasts past 1.5 s, and the top
  bar, tabs and canvas keep their exact bounds. A narrow 640x690 case proves the
  message covers neither a tab nor a top-bar button and the page does not
  scroll sideways.
- Added `ui/e2e/narrow-200-percent.pw.ts`: a 640x360 CSS viewport with
  `deviceScaleFactor: 2` (a 1280x720 window at 200 % zoom) in three themes. No
  horizontal document overflow, no navigation control outside the window
  sideways, no control's own text clipped, the canvas renderer is non-zero and
  the Properties panel stays reachable by scrolling. This is browser geometry,
  not attended Windows display scaling.
- Added `ui/e2e/obs-card.pw.ts` plus `__routeFixtureObsStatus` / OBS action
  stubs in the harness: the Apps tab's OBS card against a simulated live OBS
  with the raw-microphone problem and CABLE-A voice path. Three themes at
  1440x1000: table, device text, problem, "Fix in OBS" button, a repeated poll
  and opening/cancelling the fix dialog leave the card, summary and table at the
  same document offsets; 640x690 adds no sideways scrolling.
- Strengthened `ui/e2e/native-select-keyboard.pw.ts`: after Escape and Tab the
  picker keeps the chosen device, focus lands on "Refresh available outputs",
  and no commit/start/plan call reached the backend. (Native popup provisional
  cancellation is a physical-key Windows check; see the gates.)

No product defect was reproduced by these paths. The first narrow-window run
failed only because the assertion demanded that the stacked workbench sit above
the fold; the corrected assertion checks horizontal containment and scrolling,
which is how the documented narrow layout works. The first OBS-card run failed
because the disconnected fixture had no OBS action stubs; the harness now
provides them.

## Commands and results

| Command | Result |
| --- | --- |
| `cargo test -p audiorouter-control obs::tests --locked -- --test-threads=1` | 25 passed, 0 failed (fake OBS v5 server, saved file, auth, apply/restore, bounds, shutdown) |
| `cargo test -p audiorouter-windows-audio audio_endpoints_merge_repeats_and_are_bounded --locked` | 1 passed |
| `cargo test -p audiorouter-control --lib --locked -- --test-threads=1` | 509 passed, 0 failed, 29 ignored |
| `cargo test -p audiorouter-cli --lib --locked -- --test-threads=1` | 71 passed, 0 failed, 1 ignored; includes the default-MCP reachability test |
| `cargo clippy -p audiorouter-control -p audiorouter-cli -p audiorouter-domain --all-targets --locked -- -D warnings` | clean |
| `cargo fmt --all -- --check`; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | both clean |
| `npm --prefix ui test -- src/appUsage.test.ts src/obsUsage.test.ts src/ObsCard.test.tsx src/useObsStatus.test.tsx` | 69 passed |
| `npm --prefix ui test` | 136 files, 1,191 tests passed |
| `npx playwright test --config=playwright.config.ts device-loss-message.pw.ts narrow-200-percent.pw.ts native-select-keyboard.pw.ts obs-card.pw.ts --workers=1` | 21 passed, 0 failed (50 s) |
| `npx playwright test ... quit-keeps-edits.pw.ts device-used-by.pw.ts --workers=1` | 27 passed with the above set earlier |
| `npm --prefix ui run typecheck:all` | clean |
| `npm --prefix ui run format`; `format:check`; `lint` (`--max-warnings 0`) | clean |
| `npm --prefix contracts run check:drift` | passed: 156 methods, 38 kinds, 20 processors, 24 event categories |
| `node tools/site/check-tool-guide-property-coverage.mjs` | 39 guide kinds, 258 properties covered |
| `node tools/site/check-tool-guide-videos.mjs` | 39 tools, 204 videos, 54,492,108 bytes, all within format/duration/size |
| `node tools/docs/validate.mjs` | 304 Markdown files, 1,695 local links |
| `git diff --check` | clean |

Screenshots from the browser runs are in Playwright's per-test output folder
(`testInfo.outputPath`), not in `docs/`.

## OBS and Discord plan status

D1, O1, D2 and O2 are implemented on this branch's base. The portable evidence
above now covers the fake OBS server, the saved scene-collection reader,
apply/restore refusal and resume paths, bounded queues and files, the Apps OBS
card in three themes, and the Discord/app usage modules. The API methods are
reachable by the default MCP tools (CLI lib test). No part of this evidence is a
real OBS connection, a real Discord configuration or a real device walk; those
remain human gates.

## UI quality review status

The remaining items listed in the plan were split:

- **Portable, now covered:** lost-device messaging on a live route (red, lasting,
  no shift, narrow placement); a narrow window at 200 % logical size in three
  themes; Escape/Tab cancellation of the native device list (browser semantics).
- **Still human:** Narrator, the real Windows display-scaling session (the
  owner's Round 30 table already records 100/150/200 %; a new narrow 200 %
  window on the physical PC was not done), physical-key native popup
  cancellation, the five-participant study, and a live device unplug on a
  rebuilt exe.
- **Guide clips:** all 258 properties are recorded and 204 videos pass the
  format/size/duration checks. PR #176 (2026-11-22) refreshed the five overviews
  affected by the seven findings; since then commit #187 changed message
  plumbing and the Network address reserved slot, not a Properties control, so
  no Properties guide needs re-recording on `main`. The unmerged
  `improve/ui-spacing-errors` branch changes chart label spacing and would need
  its own guide refresh if it is ever merged.
- A participant script and observation sheet are in
  `docs/plans/active/evidence/2026-11-24-ui-quality-participant-study.md`.

## Local branches inspected read-only (not merged)

- `improve/ui-spacing-errors` (2 commits, `ea4398e38`, `d61bfb7d4`, 2026-11-10):
  **holds valuable work that never reached main.** It adds the measured
  `crampedLayout` audit, `ui/e2e/dialog-spacing.pw.ts` (13 dialogs at two sizes,
  three themes) and `ui/e2e/panel-spacing.pw.ts` (every tab, Settings page and
  tool panel), fixes the spacing defects those found, and makes every UI error a
  plain sentence through `formatUiError` across 19 call sites plus the desktop
  shell's plain-string rejection. `git ls-tree origin/main` has neither spacing
  spec. It cannot be merged blindly: since 2026-11-10 `App.tsx`, `styles.css`
  and `backend.ts` diverged heavily (including the L10N-06 structured-message
  change), so the error work overlaps and needs a careful port and re-review.
- `improve/ui-quality-review-iteration4` (2 commits, `b1ee0ef12`, `33a7a17b5`):
  **nothing unique remains.** Main already has a stronger teardown fix
  (`context.unroute(matches, serveBundledAsset)` plus waiting for in-flight
  handlers in `ui/e2e/browser-test.ts`) and the 5 s/15 s lazy-Status waits in
  `App.sessionSelection.test.tsx`.

## Limitations

- No attended run, no Narrator, no real Windows scaling change, no physical
  keyboard/dropdown popup, no real OBS/Discord, no device unplug, no packaged
  shell, no participant.
- The 200 % test proves CSS geometry under a narrow logical viewport and 2x
  device scale; it does not prove font rasterization on the owner's display.
- The OBS card fixture feeds a simulated `obs.status`; the real OBS thread and
  WebSocket client were exercised only by the Rust fake-server tests.