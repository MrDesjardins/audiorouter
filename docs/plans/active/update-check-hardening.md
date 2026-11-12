# Update-check hardening

Status: implementation and local validation complete on branch
`improve/update-check` in the isolated worktree
`C:\code\audiorouter-update-check`. Owner approved opening a pull request on
2026-11-11. The branch `fix/virtual-cable-review` was not modified.

## Objective

Make UI-18's new-version notice reliable for a long-running tray app and honest
about what it is doing: recheck while the app runs, expose a real status and a
Check now action, survive offline/rate-limited checks with bounded retries,
look only at the newest releases, add a stable/prerelease channel choice, and
forget cached data when the user turns checking off.

## Requirements

- UI-18: version in the title/header, daily check, opt-out, channel choice,
  release-page opening, no identifier or audio sent, no layout shift.
- PROD-06: the check remains the only network request the app makes on its own.
- SEC-11: no download or install is added; the check stays read-only.
- UI-17: the status, actions and controls reserve their space in all themes.

## Decisions

- 2026-11-11 (owner): normally check once a day; on a failed check retry with
  exponential backoff for up to 12 hours, then skip to the next day.
- 2026-11-11 (owner): look at the newest releases first; the first 20 are
  enough (the newest one alone would also be acceptable). The implementation
  sorts by publication date, takes the first 20, then picks the highest
  `vX.Y.Z` among them.
- 2026-11-11 (owner): a release must have the matching
  `AudioRouter_<version>_x64-setup.exe` asset before it can produce a notice.
- 2026-11-11 (owner): Settings gets a real Check now action and status; the
  controls must be usable and visually verified in dark, light and
  high-contrast.
- 2026-11-11 (owner): add a stable/prerelease channel preference.
- 2026-11-11 (owner): turning checking off clears the cached result; a cached
  time in the future is clamped rather than trusted.
- A true Tauri auto-updater is future work, recorded in
  [auto-update-tauri.md](../future/auto-update-tauri.md); it is not authorized
  or implemented by this plan.

## Ordered tasks

1. Rework `ui/src/updateCheck.ts`: sorted newest-first page, installer-asset
   check, channel option, retry window, persistent failure state, focus/
   visibility/online rechecks, 24-hour recheck while running, cache clearing
   and clock clamping.
2. Add the no-network fetcher for unit tests and automated browsers, so the
   feature state stays consistent without contacting GitHub.
3. Rework `ui/src/UpdateNotice.tsx`: reserved three-line status, honest
   offline/rate-limited/exhausted copy, Check now, release action and the
   channel select; keep the header notice unchanged for users.
4. Wire the channel and state through `ui/src/App.tsx` and update the Settings
   search keywords.
5. Add model, hook, cache and component tests; update the Playwright notice
   test to cover Check now, channel filtering, cache clearing and the three
   themes with zero GitHub requests.
6. Update UI-18, PROD-06 and the release-note/distribution docs; add the future
   Tauri auto-update plan and its index row.

## Validation matrix

| Check | Environment | Result |
| --- | --- | --- |
| Unit: versions, sort, installer, channels, page limit | Vitest/jsdom | Pass |
| Unit: retry plan, 12-hour window, resume | Vitest/jsdom | Pass |
| Hook: fresh cache, stale cache, 24-hour timer | Vitest/jsdom fake timers | Pass |
| Hook: failure backoff, success reset, online, focus | Vitest/jsdom fake timers | Pass |
| Hook: check now, channel change, offline, saved retry, off/on races | Vitest/jsdom fake timers | Pass |
| Component: every status, buttons, select, labels | Vitest/jsdom | Pass |
| Browser: header, panel, Check now, channel, off + reload | Playwright/Edge, injected list | Pass, zero GitHub requests |
| Browser: offline status and retry copy | Playwright/Edge, offline page | Pass |
| Themes, contrast and target size | Playwright dark/light/high-contrast | Pass |
| Full UI suite | `npm --prefix ui test` | 115 files, 1039 tests pass |
| Typecheck, lint, format | `npm --prefix ui run ...` | Pass |
| Documentation links | `tests/acceptance/docs.ps1` | 258 files, 1471 links pass |

Evidence (2026-11-11, Windows / Node 22.22.3 / Edge):

- `npm run test` in `ui/`: 115 files, 1039 tests passed.
- `npm run typecheck:all`, `npm run lint`, `npm run format`: clean.
- `playwright test --config=playwright.config.ts --workers=1 e2e/update-notice.pw.ts e2e/settings-controls.pw.ts e2e/text-contrast.pw.ts e2e/target-size.pw.ts`:
  22 passed. The update scenarios cover dark, light and high contrast, Check
  now, the channel change, turning off and reloading, and an offline failure.
- Screenshots: `%TEMP%\audiorouter-playwright-results\update-notice.pw.ts-*\`
  and `target/ui-quality-review/round-25/updates-*.png`.
- PR screenshots (dark theme, injected 0.0.28 release): [Settings page](evidence/2026-11-11-update-check-settings-dark.png)
  and [New versions panel](evidence/2026-11-11-update-check-panel-dark.png).
- One earlier parallel run ended with a Playwright
  `browserContext.close: ENOENT ... recording.trace` artifact error; the same
  scenario passed when rerun alone and in the final single-worker run. It did
  not reproduce as an assertion or product failure.

Not run here: a packaged-shell launch, a real network failure against GitHub
from the app, and a visual inspection of the PNG screenshots by this agent
(image input is unsupported). The automated contrast, target-size and
layout-shift checks all pass. The check remains a UI-only feature; no backend,
driver or audio path changes.

## Risks and rollback

- Timer-heavy hook: guard every async result with mount/enabled refs and keep
  one timer for the next attempt; tests use fake timers.
- Retry policy could hammer GitHub: base delay 1 minute, exponential, capped
  by the 12-hour window; a successful check resets it.
- Cache format change: old entries without channel/failure fields still load
  as prerelease with no failures; malformed entries are ignored.
- Layout shift: the status has a fixed height, both actions are always present
  and stacked, and the release action label is constant.
- Rollback: revert this branch; the feature is isolated to the UI, docs and
  tests. No migration, API, contract or audio change is involved.

## Next action

The changes are committed on `improve/update-check` and the pull request is
open for review. Merge only after the required checks pass and the owner
approves the merge.