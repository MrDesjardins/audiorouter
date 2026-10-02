# Joe prerelease 0.0.3 — 2026-10-02

## Scope and prerequisites

User explicitly authorizes a GitHub release for Joe to test the discovery,
diagnostics and UI fixes. Windows 11 x64; unsigned prerelease, existing endpoints,
no driver and no VST implementation changes. Requirements: CAP-01/13, ENG-05,
UI-03/04/13, API-09, SEC-10 and M08 artifact/provenance subset. Full M08 remains
open. Release source is the isolated `release/0.0.3` checkout; unrelated video
and signing-plan edits remain in the original workspace.

Joe's old shell log contained nine `0xE000020B` device-instance failures.
It did not identify the failed endpoint or inventory stage. Discovery now skips
only disappeared endpoints; verify the original symptom on Joe's PC.

## Source verification

Commands run in the isolated checkout on Windows with locked dependencies:

- `cargo test --workspace --locked --target-dir C:/code/audiorouter/target`:
  exit 0, all non-ignored workspace tests passed. Opt-in hardware, private-audio
  and attended fixtures remain ignored; this is not a new hardware qualification.
- `cargo test --manifest-path src-tauri/Cargo.toml --locked --target-dir C:/code/audiorouter/src-tauri/target`:
  43 passed, 1 attended HTTP/browser fixture ignored.
- UI `npm run typecheck` and `npm test`: passed, 436 tests in 43 files.
- Contracts `npm run typecheck` and `npm run check:drift`: passed; 121 methods,
  34 node kinds, 18 processors and 22 event categories agree.
- Production canvas Vite build and `playwright test --config playwright.usability.config.ts`:
  12 Edge cases passed: Arrange/undo, drag/drop, network diagrams and Logs
  controls in dark, light and high-contrast themes.
- npm lock diffs: only application/workspace version changed to 0.0.3;
  dependency versions retained. Documentation initially found a broken release
  notes anchor; replaced it with the stable page link before packaging.

## Packaging and publication

Pending: clean source commit/tag, matching shell/CLI/worker and NSIS installer,
artifact verifier, exact packaged-exe fresh-install, GitHub publication and
download/hash verification. Build identity will be `v0.0.3`.

## Limitations and rollback

Native force-close and Explorer actions require attended review. Standard-user
clean-machine/WebView2/upgrade/uninstall, accessibility, hardware soak and the
aggregate diagnostic retention budget remain open. Existing clock-drift and
virtual-cable dropout findings remain unresolved qualification risks.
Keep v0.0.2 and a compatible configuration/recordings backup. Do not overwrite
published assets or delete user data to roll back. Next task after publication:
Joe tests his primary-input list and sends current/previous shell, backend and
discovery JSONL files with reproduction time if it fails.
