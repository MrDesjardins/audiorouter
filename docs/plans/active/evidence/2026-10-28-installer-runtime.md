# Installer runtime qualification — 2026-10-28

On Windows 11, a linked clone of AR-DriverTest snapshot 02-test-signing-ready
was used. It is a test-configured VM, not clean-machine qualification. Under a
medium-integrity token with administrator membership disabled, published
0.0.23 installed and 0.0.24 upgraded with exit code 0. Both published CLIs
exited 0xC0000135 before opening a database. The guest lacked VCRUNTIME140.dll
and VCRUNTIME140_1.dll; PE import inspection found VCRUNTIME140.dll in the
published CLI and both DLLs in the shell. A running process alone is not shell
readiness evidence. On downgrade to 0.0.23 and silent uninstall, registration
was removed and a test marker survived. This does not yet prove real database
or recording retention or compatibility.

A separate `RUSTFLAGS=-C target-feature=+crt-static` release build of CLI,
plugin worker and shell imports no VCRUNTIME or MSVCP DLL. In the same VM,
the CLI ran `--json session list --database <isolated path>` with exit 0 and
created a real empty database. The shell started with an isolated database and
its one-shot native bridge probe fired. This is binary-level evidence, not a
packaged installer result or normal multi-request shell qualification.

The release preparation script now sets static CRT for its Rust builds and
rejects actual packaged shell/CLI/VST2/VST3-worker imports of the VC runtime.
The PE guard's bounded synthetic tests accept kernel32 and reject VCRUNTIME140;
manual qualification also rejected the published CLI and accepted the static
CLI and worker. Pending: build and execute the exact new installer, exercise
real database and recording retention across install/upgrade/uninstall, check
full native bridge and actual Apps-tab polling. Published 0.0.24 remains
unchanged. No physical audio, device defaults, signing or user state were changed.

## Candidate lifecycle and Apps measurement

Candidate from clean source `34e872b66ad038623ae78b907d2cdbc5bd1cfd2b`:
`prepare-artifacts.ps1 -AppOnly` and `verify-artifacts.ps1` passed. Installer
SHA-256 `88de32eb102612bc33229a3057fb0ca99c83d00220a3a454a56e0e3d33869065`;
manifest has no release tag, no driver and `publicationReady: false`. The
published 0.0.23 installer SHA-256 is
`015b95a6ed0b50b0e56182647fde728f13169e3ab660836b04f72245f676204b`.
This candidate is not a published 0.0.24 replacement.

In the test-ready disposable VM, a CLI rebuilt with static CRT from the exact
0.0.23 tag `11737991135112c7085f6b45a88770f36287fd04` created a session in
its real SQLite database. The published 0.0.23 CLI itself was unusable without
VC runtime. The candidate silently upgraded 0.0.23, reported 0.0.24, and its
installed CLI read that session. Its installed shell hit the one-shot native
bridge probe. Candidate uninstall preserved the database, and reinstall read
the session. Downgrading to the published 0.0.23 installer preserved the session,
which the separately rebuilt 0.0.23 CLI could read. Uninstalling 0.0.23 also
preserved it. A generated PCM WAV in the guest temporary directory had the same
SHA-256 at each stage. These are silent per-user operations under a medium,
non-elevated token whose account is a member of Administrators (deny-only in
the token), not a separate standard account; the test-configured VM is not a
clean-machine or trust-prompt test. The old published CLI remained broken.

On the Windows host, an isolated UI shell from the static build used a separate
backend, private named pipe, temporary database and desktop-equivalent grant;
no route was started or device changed. With the Apps tab open and host Discord
running, 20 additional bridge `apps.list` calls at 600 ms intervals measured
9.1 ms minimum, 9.7 ms median, 10.2 ms p95 and 10.4 ms maximum, observing 455
process entries and Discord. This is client-side request duration during UI
polling, not a comparison against a controlled old implementation or a live
Discord voice-call/OBS setup. OBS was not running. The first measurement attempt
closed an empty preflight connection and ended the test backend; a second lacked
a seeded session. Both timed out; the corrected private fixture passed. Scripts
and screenshots are scratch evidence in this worktree's `target/`, not user
session data. No claim of real audio or OBS qualification is made.

The UI delayed-read regression failed on main (one request instead of the
follow-up) and passed after the fix; focused tests also cover a rejected read,
backend replacement and unmount. UI lint, format, typecheck and 746 tests passed
before the additional focused failure test; that focused suite then passed all
14 tests. Release test suite and docs validation passed. Vite chunk warning and
Stream Deck build warnings/advisories remain as recorded for 0.0.24. The
published 0.0.24 artifacts remain immutable; ship a new release after review.

## Fresh-profile and browser checks (2026-10-28, continued)

Installed candidate in the same test-ready VM, normal fresh profile and no
one-shot probe: `fresh_install_shell` passed (1 test). The backend answered
`status.get` 642 ms after launch, and after the one-time device-access consent
`prepare` answered "choose the device for Physical input in its Properties",
the expected not-yet-configured reply. UI: 747 tests, lint with zero warnings
and `format:check` passed. Edge browser spec `e2e/application-sources.pw.ts`
passed 3 of 3 (closed-app source, app group without graph changes in all
themes, picker readable and stable in every theme); a leftover Vite server on
port 4186 from the interrupted earlier run was stopped first. Still not
established: a clean machine, a true standard account, an attended Discord/OBS
call, and physical audio.
