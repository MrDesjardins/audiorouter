# Frontend port, accessibility and app-only qualification

## Objective and authority

Owner requested priorities 2, 5 and 6 on 2026-11-27: port the unmerged
`improve/ui-spacing-errors` work, perform accessibility qualification, and
prepare and qualify a fresh app-only build including API-07. Driver work,
publication and opening a PR are outside this execution.

## Requirements and prerequisites

M05/M08: UI-02/04/05/08/10/11/12/17/19/36, L10N-01/02/03/05/07,
AUTO-06/13, API-07, DIST-02/03/06/07/08, NFR-04/05/06/14/15 and Q6.4.
Read M05, M08, the active plan, F1-F7 and C3/E1-E6 in human-gated tasks,
the interface and quality specifications and qualification/localization runbooks.

Initial environment: Windows PowerShell, main `2f24ad436`; new branch
`improve/ui-accessibility-app-qualification`. Preserve unrelated untracked
`target-preview/`, `target-preview2/`, `ui/tsconfig.tsbuildinfo`.
AudioRouter PID 13116 was running at inventory. Do not stop it or use its
devices/profile. A quiet window is requested for real-app/soak qualification.

Owner clarification: no eight-hour soak now. Owner can close AudioRouter for a
short test window; endurance remains deferred. No driver work.

## Decisions and ordered tasks

1. Inspect and port `ea4398e38` and `d61bfb7d4`. Retain current localization
   and newer behavior; do not restore the obsolete active-plan history file.
2. Immediately typecheck changes, then format/lint and run relevant component
   and Edge tests. Adapt old tests to current UI only where behavior warrants.
3. Check dialogs/panels in three themes, keyboard paths and translated scaled
   layouts. Browser viewport checks are not Windows scaling or Narrator evidence.
4. Verify API-07 through its real dispatcher regression and build shell plus
   all worker/CLI companions. Record hashes, timestamps and unsigned status.
5. Run available app-only packaging/recovery regressions. Launch the built shell
   with an isolated database and WebView profile only when the live app is closed.
   Run the eight-hour real-route soak only with free devices and owner availability.
6. Inspect the complete diff; update current priorities, results and exact next
   actions. No PR, tag, draft or publication without separate approval.

## Validation matrix and evidence

- Port: typecheck, UI lint/format, relevant unit tests and spacing browser tests.
- F2/F7: real browser keyboard/MCP states and three-theme screenshots where possible.
- F3: translated viewport/layout checks; actual Windows 150/200% remains distinct.
- F1/F4/F5/F6: attended device/sleep, Narrator, native tray/dialog and live-language
  checks require the fresh shell and appropriate device/desktop availability.
- Build: pinned toolchain, complete companions, embedded UI timestamp, SHA-256.
- API-07: dispatcher restart/idempotency regression, not a handler-only check.
- Release: app-only artifact/recovery/installer checks as available; no clean-machine
  or second-PC claim without those environments. Signing waits for OSSign.
- C3: eight hours of the real built app, thresholds from the existing soak runner;
  a short synthetic run cannot pass endurance.

Results: [2026-11-27 qualification record](evidence/2026-11-27-ui-app-qualification.md).
The spacing/error port, automated browser checks, native app smoke, app-only
installer build and signed-in release-VM lifecycle passed. Fixed the Tauri
hook that printed its program instead of rebuilding the embedded UI; four
command-path regressions now run in release checks. No source/tool API changed.

The final VM lifecycle used the actual new installer and a production-engine
recording, preserving its hash through repair, rollback, upgrade, uninstall and
reinstall. The guest was shut down and restored to its preserved snapshot.
Attended F1-F6, clean-Defender/MOTW/second-PC checks, signing and reference-PC
rollback remain open. The eight-hour soak was not run, per the owner.

## Risks and rollback

Old error text can overwrite translations or describe outdated plugin support.
Port at the owning layer and test the current catalogs. Roll back only this
branch's changes; leave the source branch and user files intact. Disposable
profiles keep qualification away from saved sessions; never change live defaults.
Installer trust, second-PC, physical unplug, sleep and human listening/accessibility
gates cannot be replaced with simulated evidence.

## Next action

Owner can test the local build on this branch. Next M05 task: attend F1-F6
(device/sleep, native keyboard, Windows scaling, Narrator, tray/dialog languages,
live language switching). Next M08 task: clean-Defender and MOTW/second-PC
qualification of the recorded artifact; publication and a PR need separate
approval. Do not start the deferred eight-hour soak.
