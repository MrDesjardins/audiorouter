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

## Local candidate — 2026-10-30

Windows 11, Node 22.22.3, Rust 1.96.0, source commit `46bb64e7` (clean worktree
`.worktrees/release-0.0.25`, no release tag). `prepare-artifacts.ps1 -AppOnly`
exited 0 (static CRT, Stream Deck typecheck and 33 tests passed, NSIS built);
`verify-artifacts.ps1` on its manifest exited 0 (`includesDriver: false`,
`publicationReady: false`). Candidate directory (local, not a release):
`target/release-0.0.25-candidate` in the main checkout. Installer
`AudioRouter_0.0.25_x64-setup.exe` SHA-256
`c8355e28b9b089a0b04641e6c1c482da60b3361d138af5eec44ed313ce84e47d`, 22,386,264
bytes; Stream Deck plugin SHA-256
`4ae4c03827d4f03f22032b61a9ebfcdf823c9c78ed846b99c979d965587fdf30`.

Independent checks: a byte scan of the shell, CLI, VST2 plugin worker and VST3
worker found no `VCRUNTIME`/`MSVCP` import names; the generated NSIS script
installs all 107 `sound-library` files (library.json, NOTICES.md, 105 sounds);
the embedded UI bundle contains the Soundboard. `fresh_install_shell` against
the built shell, with no developer variables and no other AudioRouter running,
passed: backend answered 71 ms after launch, one consent prompt, then prepare
proceeded to the device choice.

Not established: the installer was not executed (an install would replace the
owner's per-user installation), so clean-machine, upgrade/uninstall, standard
account, missing-WebView2, physical audio, live MCP and Soundboard playback from
the packaged build remain open. The final assets must be rebuilt from the tagged
main commit; this candidate's hashes will not carry over.
Next: owner decides on the PR (branch is not pushed).

## Hosted checks — 2026-10-30

PR #93 at `a6e0bedf`: all required checks passed (Windows build, tests and
release preparation 27m57s; Edge UI tests; CodeQL for all languages;
dependency advisories; documentation; Stream Deck). The only check annotations
are GitHub notices that the `ubuntu-latest` label moves to Ubuntu 26. They name
no failure, and changing the runner image is a CI change that does not belong
in a release-preparation PR, so it stays; pinning the label is a follow-up.
`main` gained #92 (a docs-only virtual-cable plan update) during review and
was merged into the branch so the release contains all of main.
