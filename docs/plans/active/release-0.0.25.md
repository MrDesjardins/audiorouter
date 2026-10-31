# Release 0.0.25 preparation — 2026-10-30

Objective: prepare the app-only unsigned experimental 0.0.25 prerelease from
everything on main (owner request: all of main goes into the next release).
Requirements: DIST-01–08, M08 release evidence; contents cover the Soundboard
(UI-25), CAP/ENG source-in-every-layout rules, and the installer runtime fix.

Scope: version bump by package name from 0.0.24; release notes for PRs #85–#89
(panel resize and homepage demos, UI/Stream Deck dev-dependency advisories,
virtual cable assurance tooling with no driver change, static-CRT installer and
Apps inventory follow-up, Soundboard with bundled library and Stream Deck Sound
key). Base: main at `b044ec7a` (merge of #89), 55 commits after `v0.0.24`.
No driver or storage migration.

Prerequisites: clean Windows release checkout and tag on main, pinned Rust 1.96.0
and Node, VST3 SDK and verified tunnel-runtime inputs, NSIS, GitHub credentials;
packaged-shell validation needs the production control pipe free (the owner's
shell may be running: do not stop it).

Ordered tasks: (1) bump versions and notes (done on branch `release/0.0.25`);
(2) focused lock/format/docs checks; (3) owner approves a PR, merge; (4) tag the
reviewed main commit; (5) build and verify app-only artifacts with
`tools/release/prepare-artifacts.ps1 -AppOnly` and `verify-artifacts.ps1`,
confirming the installer contains `sound-library` (107 files) and no shell, CLI
or worker imports `VCRUNTIME140.dll`; (6) `fresh_install_shell` against the built
exe with no developer variables; (7) draft release, download and re-verify
assets; (8) publish only on authorization.
Rollback: retain the 0.0.24 installer and configuration backups; revert the
preparation commit if not published; never overwrite published assets.

Known open items carried into the notes: Soundboard drag and drop (SND-19),
category expansion in random sets, library titles pending friendlier names, Playwright e2e and dark/light/
high-contrast screenshots for the Soundboard UI not run on its branch, clean
machine and standard-account install, accessibility, hardware, signing.

Release notes: [0.0.25 notes](evidence/2026-10-30-release-0.0.25-notes.md).
The owner tested the Stream Deck Sound key on hardware on 2026-10-30 and all
modes work (attended result on the owner's build; not re-run on the final installer).
Next: build the candidate and ask for PR approval.
