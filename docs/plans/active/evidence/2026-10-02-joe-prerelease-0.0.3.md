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

Published [v0.0.3](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.3)
as an unsigned prerelease (`isDraft=false`, `isPrerelease=true`). Source commit
`349bdc8256d5eb035190739b7127a487e1bebb05`, annotated tag `v0.0.3`, branch
`release/0.0.3`. Tag was pushed before publication and remains immutable.

Clean tagged checkout command, with `AUDIOROUTER_BUILD_ID=v0.0.3`:
`tools/release/prepare-artifacts.ps1 -OutputDirectory C:/code/audiorouter/target/releases/v0.0.3 -ReleaseTag v0.0.3`.
Passed; 12 files include per-user NSIS setup, matching executables, UI ZIP,
checksums, manifest, Cargo/npm SBOMs and notices. Artifact verifier passed.
Manifest deliberately retains `signed=false`, `publicationReady=false` and
qualification blockers; publication is the user-authorized test prerelease.
Rust 1.96.0 / Cargo 1.96.0, x86_64-pc-windows-msvc, optimized build.

Binary inspection found current `index-CvjdloOm.js`, `index-Cx-j8Cuv.css`,
build identity and common-controls manifest. Shell time 19:31:09 UTC is newer
than UI index 19:28:56 UTC. Working tree was clean for build provenance.

Exact packaged executable fresh-install check passed 1/1:
`AUDIOROUTER_SHELL_EXE=C:/code/audiorouter/target/releases/v0.0.3/audiorouter-shell.exe cargo test -p audiorouter-transport --test fresh_install_shell --locked --target-dir C:/code/audiorouter/target -- --ignored --nocapture`.
No other shell, default pipe, fresh disposable database, no developer device
grant. Before consent preparation was denied; after consent it reached device
selection ("choose the device for Physical input in its Properties"). Recording
folder setup was authorized on first launch. No real device was opened; this
does not qualify installer execution or Joe's actual discovery reproduction.
Only the test's own child process was terminated during its cleanup.

| Artifact | SHA-256 |
| --- | --- |
| audiorouter-shell.exe | 627e1498445faedafca8f54c5040a93cefce90876a8060247337004d8124d524 |
| audiorouter-cli.exe | 3cc34efe667b50a4cefb99b235b5bdc18446f5fcb5ca2f9ce9e27be566c26f23 |
| audiorouter-plugin-worker.exe | d386ce66b7687f65f32dc1a6323eeb6b18f088ad3da6071ddc63d1cdf242b7bc |
| AudioRouter_0.0.3_x64-setup.exe | afaecc53e4578f34fe760b87f0ccc402b24289c15b0075b7c1f8377cbb301bf7 |
| AudioRouter_0.0.3_windows-x64.zip | 664daa1e86f38117b8a66c8926d1d42e44601d53a6792aefc4745eba6453cee3 |

Published assets: direct installer, full ZIP, ZIP checksum, per-artifact checksums
and manifest. Downloaded ZIP/installer using `gh release download`; hashes matched
local assets and GitHub's reported digests. Expanded downloaded ZIP and reran
`verify-artifacts.ps1`: passed. Output lives under
`C:/code/audiorouter/target/releases/github-v0.0.3/downloaded`.
Initial ZIP creation was denied by sandbox filesystem access; elevated retry
succeeded without modifying source or existing packages.

## Limitations and rollback

Native force-close and Explorer actions require attended review. Standard-user
clean-machine/WebView2/upgrade/uninstall, accessibility, hardware soak and the
aggregate diagnostic retention budget remain open. Existing clock-drift and
virtual-cable dropout findings remain unresolved qualification risks.
Keep v0.0.2 and a compatible configuration/recordings backup. Do not overwrite
published assets or delete user data to roll back. Next task after publication:
Joe tests his primary-input list and sends current/previous shell, backend and
discovery JSONL files with reproduction time if it fails.
