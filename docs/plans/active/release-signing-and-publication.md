# Plan: sign the Windows release and publish it on GitHub

Status: not started (written 2026-10-01). It is authorized by the user to be
executed by another agent. Read this whole file, `AGENTS.md`,
`docs/plans/active/current.md`, `docs/operations/distribution.md` and
`docs/spec/15-delivery.md` (M08 gates) before acting.

## Objective

Publish a **signed** AudioRouter release on GitHub
(`https://github.com/MrDesjardins/audiorouter`) that a user can download and
install without an "unknown publisher" warning:
- the app, its companion executables and the installer carry a valid,
  timestamped Authenticode signature;
- the release has release notes, checksums, a provenance manifest, SBOMs and
  third-party notices;
- the release passes the fresh-install gate.

## Where things stand

- **Releases:**
  - **0.0.1** (`959d1c63`, tag `v0.0.1`) and **0.0.2** (`3f5da9fb`, tag
    `v0.0.2`) were built **unsigned** with `tools/release/prepare-artifacts.ps1`
    and verified with `verify-artifacts.ps1`.
  - 0.0.2 was installed and used on the user's second computer; Network
    Send/Receive works between the two PCs.
  - `main` is pushed. The tags `v0.0.1`/`v0.0.2` exist **only locally**
    (not pushed).
- **Release tooling (`tools/release/`):**
  - `prepare-artifacts.ps1`:
    - requires a clean tree;
    - builds the CLI, plugin worker, UI and shell, then the per-user NSIS
      installer via `ui/node_modules/.bin/tauri.cmd build --no-sign --ci
      --bundles nsis --config src-tauri/tauri.release.conf.json`;
    - writes `SHA256SUMS.txt`, SBOMs, notices and `release-manifest.json`
      with `signed: false`, `publicationReady: false` and explicit
      `blockers`.
  - `verify-artifacts.ps1` validates that manifest. It currently **throws if
    `signed` is not false** (line ~56).
  - `create-draft-release.ps1 -Tag vX.Y.Z -NotesPath … -OutputDirectory …`:
    - rebuilds from the exact tag;
    - verifies the artifacts;
    - creates a GitHub **draft** with `gh release create --verify-tag
      --draft` (the tag must already be pushed).
  - `.github/workflows/manual-release.yml` is the same flow as a
    manually dispatched workflow on `windows-latest`.
- **Bundle:**
  - `src-tauri/tauri.release.conf.json` bundles
    `target/release/audiorouter-cli.exe` and `audiorouter-plugin-worker.exe`
    as resources;
  - `currentUser` NSIS install;
  - WebView2 via the download bootstrapper.
  - `src-tauri/tauri.conf.json` has `"windows": []`: no signing
    configuration.
- **Release gate (AGENTS.md, validated lesson 2026-10-01):** before handing
  off any installer, run the fresh-install test on the exact built exe:
  `AUDIOROUTER_SHELL_EXE=<exe> cargo test -p audiorouter-transport --test fresh_install_shell -- --ignored --nocapture`.
  - No other AudioRouter may be running; ask the user to close theirs.
  - Never set `AUDIOROUTER_ALLOW_DEVICE_ADMIN`.

## Decisions the user must make first (ask; do not assume)

1. **Signing certificate source.** Verify current terms, prices and
   eligibility at the time of execution; these change.
   - **Microsoft Azure Artifact Signing** (formerly "Trusted Signing"):
     - pay-per-month cloud signing;
     - integrates with `signtool` through the Microsoft dlib;
     - identity validation is required, and eligibility for individuals
       depends on country.
     - Likely the cheapest managed option if the user qualifies.
   - **OV code-signing certificate from a CA** (DigiCert, Sectigo, SSL.com,
     Certum, …):
     - since 2023 the private key must live on a hardware token or a cloud
       HSM (for example SSL.com eSigner, DigiCert KeyLocker);
     - yearly fee.
   - **EV certificate:** more expensive. It no longer gives instant
     SmartScreen reputation, so it is not required.
   - **SignPath Foundation (free for open source):** only if the repository
     is public and OSI-licensed, and the user accepts their review process.

   Microsoft SmartScreen reputation builds per certificate and file over time
   with any of these. Early downloads may still show a warning; disclose that
   in the release notes.
2. **Publisher name** shown in Windows. It is the certificate's subject (the
   user's legal or business name).
3. **Version** to publish. Recommended: **0.0.3**, a new signed build;
   0.0.2's unsigned binaries cannot be reused, because signing changes the
   files. Mark it as a GitHub **pre-release** while the version is 0.0.x.
4. **Visibility:** whether the GitHub repository and release are public, and
   whether to push the local tags `v0.0.1`/`v0.0.2` as history (unsigned) or
   keep them local.
5. **Where signing runs:** locally on this PC (simplest with a token or
   cloud HSM) or in the GitHub workflow (needs repository secrets or Azure
   OIDC). Recommended: **locally first**, then the workflow later.

## Implementation tasks (in order)

1. **Get the certificate and the signing tool working by hand.**
   - Install the Windows SDK `signtool.exe` (already present with VS/Windows
     SDK 10.0.28000) and the provider's client (Azure Artifact Signing dlib,
     eSigner CKA, KeyLocker, or the token driver).
   - Sign a throwaway copy of `target/releases/v0.0.2/audiorouter-cli.exe`
     with SHA-256 and an RFC 3161 timestamp:
     `signtool sign /fd SHA256 /tr <provider timestamp URL> /td SHA256 <provider-specific key options> <file>`.
   - Verify it with `signtool verify /pa /v <file>` and
     `Get-AuthenticodeSignature <file>`: the status must be `Valid` and a
     timestamp must be present.
   - Record the exact command shape (without secrets) in this plan.
2. **Add a signing step to `prepare-artifacts.ps1`.** Keep the unsigned mode
   as the default, so unsigned builds remain possible.
   - Add a `-Sign` switch plus parameters or env vars for the signing
     command; never hard-code credentials.
   - Sign `target/release/audiorouter-cli.exe` and
     `audiorouter-plugin-worker.exe` **after cargo builds them and before
     Tauri bundles them**. They are bundle resources; Tauri does not sign
     resources.
   - Let Tauri sign the shell exe and the NSIS installer/uninstaller:
     - drop `--no-sign` in `-Sign` mode;
     - configure `bundle.windows.signCommand` in a release-only config
       (for example `src-tauri/tauri.release.signed.conf.json`) that calls
       the same `signtool sign … %1` command. Check the current Tauri 2 docs
       for `signCommand`.
   - Alternatively, sign the built installer after bundling. The installer
     exe must be signed in any case.
   - Write `signed: true` and the signer subject, thumbprint and timestamp
     authority into `release-manifest.json`. Remove the "intentionally
     unsigned" blocker; keep any blocker that is still true.
   - Checksums are computed **after** signing.
3. **Teach `verify-artifacts.ps1` to verify signatures.**
   - When `signed` is true, require for every `.exe` (setup, shell, cli,
     worker):
     - `Get-AuthenticodeSignature` status `Valid`;
     - the expected signer subject or thumbprint from the manifest;
     - a timestamp (`TimeStamperCertificate` not null).
   - Also verify the copies inside the installed folder (task 6).
   - Keep the unsigned rules unchanged when `signed` is false.
   - Add cases to `tools/release/test-verify-artifacts.ps1` for: signed
     valid, signed but tampered (hash mismatch), wrong signer, missing
     timestamp, and unsigned.
4. **Release notes.** Add a `0.0.3` section to
   `docs/operations/release-notes.md`:
   - signed, with the publisher name;
   - how to verify (right-click → Properties → Digital Signatures, or
     `Get-AuthenticodeSignature`);
   - SmartScreen reputation may still warn on the first downloads;
   - the remaining known issues, carried over from 0.0.1/0.0.2.

   Update `docs/operations/distribution.md`: no longer "unsigned first
   release".
5. **Version, tests and tag.**
   - Bump to 0.0.3 in every manifest and lockfile. 0.0.2 used: every
     `crates/*/Cargo.toml` and `src-tauri/Cargo.toml`,
     `src-tauri/tauri.conf.json`, `ui/package.json` and
     `contracts/package.json`, then `cargo update -w --offline`,
     `cargo update -p audiorouter-shell --offline` in `src-tauri`, and
     `npm install --package-lock-only --ignore-scripts --offline` in `ui`
     and `contracts`.
   - Check the lockfile diffs show only AudioRouter versions.
   - Run the full checks and record the results:
     - `cargo test --workspace`;
     - `cargo test --manifest-path src-tauri/Cargo.toml`;
     - in `ui`: `npx tsc --noEmit -p .` and `npx vitest run src`;
     - in `contracts`: `npm run check:drift`;
     - both Playwright configs. Rebuild `target/canvas-production-ui`
       with `npx vite build --configLoader runner -c
       vite.canvas-production.config.ts` before the production config.
       Known pre-existing dev e2e failures: the Undo/Bypass locator
       ambiguity (media-and-routing, session-workflows, audio-tools),
       live-controls, and undo-eq-points ×3. Do not report others as known.
   - Commit, then create an annotated tag `v0.0.3`.
6. **Build, sign and verify locally.**
   - Close every AudioRouter process first (ask the user).
   - Run `prepare-artifacts.ps1 -OutputDirectory target/releases/v0.0.3 -ReleaseTag v0.0.3 -Sign …`,
     then `verify-artifacts.ps1 -ManifestPath target/releases/v0.0.3/release-manifest.json`.
   - **Fresh-install gate** on `target/releases/v0.0.3/audiorouter-shell.exe`
     (command above) must pass.
   - Check that `src-tauri/target/release/audiorouter-shell.exe` has the same
     SHA-256 as the artifact copy (the installer was built from it).
7. **Clean-machine acceptance (M08)**, with the user's consent. Use Windows
   Sandbox or a fresh VM, as a **standard** (non-admin) user:
   - install the signed installer and record what SmartScreen shows;
   - confirm the installed exe and companions are signed
     (`Get-AuthenticodeSignature` on the install folder);
   - first launch; first Play shows "Allow AudioRouter to use your audio
     devices?"; Allow, and Play works;
   - choose a recording folder and record a short take; the WAV plays;
   - upgrade from 0.0.2 over the top, keeping
     `%LOCALAPPDATA%\AudioRouter`;
   - uninstall, recording what remains;
   - with WebView2 absent, if the VM allows it: the bootstrapper installs
     it.

   Record the results in a new evidence file
   `docs/plans/active/evidence/<date>-signed-release-0.0.3.md`. Any failure
   blocks publication. Fix it, bump the patch version and repeat.
8. **Publish**: outward-facing, so ask the user before each step.
   - Push the tag: `git push origin v0.0.3`; the user decides about older
     tags.
   - Create the draft. Either:
     - locally: `tools/release/create-draft-release.ps1 -Tag v0.0.3
       -NotesPath <0.0.3 notes extract> -OutputDirectory
       target/releases/draft-v0.0.3`, which needs `gh auth login` and the
       `-Sign` plumbing from task 2; or
     - run `manual-release.yml` once signing works in CI.

     Its manifest check expects `publicationReady: false` at draft time;
     keep that, because publishing is a separate manual step.
   - The user reviews the draft (assets, checksums, notes, signer) on GitHub.
   - Publish as a **pre-release**:
     `gh release edit v0.0.3 --draft=false --prerelease`.
   - Download the published installer once and check its SHA-256 against
     `SHA256SUMS.txt` and its signature.
9. **Close out** per AGENTS.md:
   - update `current.md` and this plan with the results, commands and
     evidence links;
   - add a validated lesson if a signing pitfall was found;
   - commit and push the docs (ask first).

## Rules for the executing agent

- Never commit certificates, keys, passwords, PINs, tokens or `.pfx` files.
  Keep them in the provider's store, the hardware token, environment
  variables or GitHub secrets.
- Outward-facing actions need the user's explicit go-ahead each time: tag
  pushes, GitHub drafts or releases, publishing, buying or enrolling a
  certificate.
- Do not report unrun checks as passing. Do not weaken
  `verify-artifacts.ps1` to make a build pass.
- Use the Edit/Write tools for text containing Windows paths or backslashes
  (AGENTS.md lesson), not shell heredocs.
- Only one AudioRouter may run at a time. Check `audiorouter-shell` processes
  before builds and launches, and ask the user to close theirs.

## Rollback

- **Unpublish:** a published release can be returned to draft
  (`gh release edit v0.0.3 --draft`) or deleted. The user's installs keep
  working, and 0.0.2 stays available locally.
- **A signing problem after publication** (wrong subject, revoked
  certificate): unpublish, fix, and release 0.0.4. Never replace assets of a
  published version in place.
- **Script changes** are additive (`-Sign`); the unsigned path stays
  available and tested.
