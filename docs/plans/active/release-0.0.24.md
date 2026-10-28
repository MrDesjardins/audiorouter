# Release 0.0.24 preparation — 2026-10-27

Objective: prepare the app-only unsigned experimental 0.0.24 prerelease from
main after the owner's successful long-sleep test. Requirements: CAP-02/03/11/12/13,
STATE-03/11, SEC-06/10, NFR-10/11, DIST-01–08 and M08 release evidence.

Scope: version bump by package name; release notes covering sleep recovery,
diagnostic privacy, and toolchain maintenance; no driver or storage migration.
The owner confirms the 0.0.23 development build from `26de4a4f` worked after
long sleep. This is one attended user result; it does not establish 100-cycle
endurance, device-missing behavior or other native route/recording cases.
The privacy fix merged later as #82 and was not part of that test executable.

Prerequisites: clean Windows release checkout and tag on main, pinned Rust and
Node, VST3 SDK and verified tunnel-runtime inputs, NSIS, GitHub credentials;
packaged-shell validation requires the production control pipe to be free.
The owner's test shell is currently running; don't stop or replace it.

Ordered tasks: update package versions and notes; verify only AudioRouter
packages changed and run focused lock/format/docs checks; commit, get release
changes onto main through the repository's PR approval rule; tag the reviewed
main commit; build and verify app-only artifacts; qualify packaged startup and
download checks; publish the prerelease when authorized and gates are met.
Rollback: retain the 0.0.23 installer and configuration backups; revert the
release preparation commit if not published. Do not overwrite published assets.

Validation: record the exact commit, tag, environment, command, hashes, local
and hosted checks; mark hardware, signing, clean-install, and accessibility
limits separately. Release notes: [0.0.24 notes](evidence/2026-10-27-release-0.0.24-notes.md).
Next: validate the version bump and present the release branch for PR approval.

## Local preparation checks — 2026-10-27

Windows 11, Node 22.22.3: clean npm installs for UI, contracts and Stream
Deck; UI TypeScript 7, lint, format, 743 tests and production build passed;
contracts typecheck and drift (131 methods, 36 node kinds, 20 processors and
22 event categories) passed; Stream Deck typecheck, 26 tests and build passed.
Both Rust format checks, documentation validation (203 files, 1103 links),
`git diff --check`, and locked offline Cargo metadata for workspace and shell
passed. The three Cargo lockfiles changed only `audiorouter-*` package
versions (12, 12 and 10 respectively); 13 application manifests and both npm
projects were updated by name. The probe's separate Cargo metadata command
failed because Cargo resolves the outer worktree's workspace; its lockfile
was inspected but the probe was not built. Vite still warns about the 608 kB
application chunk; Rollup warns about two zod PURE comments and one zod
circular dependency. These third-party/tooling warnings are documented in the
0.0.23/PR #81 evidence and do not indicate a release-specific change.

The owner confirmed a long sleep recovered on the `26de4a4f` test build. The
0.0.24 security fix is newer than that test build; its own shell tests, strict
Clippy, Windows CI, Edge and CodeQL passed in PR #82. Packaged-shell startup,
artifact checks, download checks and publication are pending. The owner is
using the test shell; do not close it or occupy its production pipe.

## Owner test candidate — 2026-10-27

Prepared app-only candidate from source commit `e32c06993ee3789a701a82962784d8268ae1de2f`
with the locked release script and verified it using `verify-artifacts.ps1`.
The manifest records version 0.0.24, no release tag, `includesDriver: false`,
and `publicationReady: false`. Candidate installer and manifest are in
`target/release-0.0.24-candidate` in the repository's main checkout; this is a
local test artifact, not the final tagged release. A separate executable folder
`target/local-test-0.0.24-e32c0699` contains the shell, matching CLI, VST2 and
VST3 workers, verified tunnel runtime and Stream Deck plugin. The CLI copy's
SHA-256 matched the just-built CLI; its `schema --json` output parsed. The shell
contains UI asset `index-C9kR5eod.js`. Stream Deck packaging/validation and its
26 tests passed. No live MCP enrollment, tunnel connection, Stream Deck button,
or packaged UI launch was attempted while the owner had an app running.

The 0.0.24 candidate build's Stream Deck `npm ci` reported three development
advisories: `deepmerge` 4.3.1 via Rollup node-resolve (low) and `fast-uri`
3.1.8 via the Elgato CLI/Ajv (moderate). `npm audit --omit=dev` returned zero
vulnerabilities. The pinned dev tools handle release-build inputs, not the
shipped production dependency graph. Do not silently suppress these findings:
review a compatible fixed dependency for a follow-up; the existing grouping
policy prevents automatic patch proposals. Vite's chunk and zod Rollup
warnings are recorded above. Hosted branch CI run 40609255113 is in progress.
Next: owner tests the executable and MCP/Stream Deck; retain this branch without
a PR while that test request is in effect. Then obtain explicit PR approval,
merge, tag main, rebuild/verify final assets, and complete the release gates.
