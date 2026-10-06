# Plan: sign the Windows release and publish it on GitHub

**Decision (2026-10-03, user): parked.** Keep publishing unsigned
prereleases while the project builds the public track record SignPath
Foundation expects, then apply. No purchase or enrollment now.

**Current status (2026-10-03): blocked on the user's signing decision.**
Unsigned prereleases 0.0.2–0.0.7 are public on GitHub with their tags, and
the release tooling (`prepare-artifacts.ps1`, `verify-artifacts.ps1`, the
repaired `create-draft-release.ps1`) works end to end; the "tags only local"
and "no releases" statements below are history. Nothing is signed. The next
step needs the user, not an agent: choose a provider (SignPath Foundation's
free OSS route, which requires applying with the maintainer's identity, or a
paid certificate) and enroll. Tasks 1–3 (sign a test file, add `-Sign`,
verify signatures) can start once a credential exists. Driver signing stays
set aside ([M03](../future/M03-driver-signing.md)); this plan is only about
the app and installer.

Status: preflight started; signing decisions pending (2026-10-01). It is authorized by the user to be
executed by another agent. Read this whole file, `AGENTS.md`,
`docs/plans/active/current.md`, `docs/operations/distribution.md` and
`docs/spec/15-delivery.md` (M08 gates) before acting.

## Objective

## Execution preflight (2026-10-01)

### SignPath feasibility follow-up

Interim publication explicitly requested by the user: publish a ZIP containing
the working USB package. Located `G:/audiorouter0_0_2`; all 12 files matched
`target/releases/v0.0.2` by SHA-256. Both original and extracted archive pass
`verify-artifacts.ps1`. Exact shell fresh-install consent regression passes
1/1 on Windows with no developer device-access variable; no real audio opens.
Published [unsigned prerelease v0.0.2](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.2)
via existing annotated tag push and `gh release create --verify-tag --prerelease
--notes-file`. ZIP and ZIP checksum are attached. Notes preserve incomplete
M08/signing gates; no claim of a signed or fully qualified release. Archive
SHA-256: `50ec091b6b6e29c4d3403abbbbe7aca67f84abf63efe84d3d77b3cfff592dc7d`.
Release files/notes and extraction verification are retained under
`target/releases/github-v0.0.2`. This supersedes the no-public-release
observation below; SignPath acceptance and other prerequisites remain pending.

Interim publication explicitly requested by the user: publish a ZIP containing
the working USB package. Located `G:/audiorouter0_0_2`; all 12 files matched
`target/releases/v0.0.2` by SHA-256. Both original and extracted archive pass
`verify-artifacts.ps1`. Exact shell fresh-install consent regression passes
1/1 on Windows with no developer device-access variable; no real audio opens.
Published [unsigned prerelease v0.0.2](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.2)
via existing annotated tag push and `gh release create --verify-tag --prerelease
--notes-file`. ZIP and ZIP checksum are attached. Notes preserve incomplete
M08/signing gates; no claim of a signed or fully qualified release. Archive
SHA-256: `50ec091b6b6e29c4d3403abbbbe7aca67f84abf63efe84d3d77b3cfff592dc7d`.
Release files/notes and extraction verification are retained under
`target/releases/github-v0.0.2`. This supersedes the no-public-release
observation below; SignPath acceptance and other prerequisites remain pending.

The user closed AudioRouter and selected SignPath if feasible and reasonably
simple. A fresh process check found no shell running. GitHub CLI read-only
checks confirmed PUBLIC visibility, null `licenseInfo` and no GitHub releases.
Cargo metadata declares MIT; the user approved adding a root MIT LICENSE with
Patrick Desjardins as copyright holder. The license is local, not pushed.

[Foundation terms](https://signpath.org/terms.html) require OSS licensing,
an already released project, a code signing policy, MFA, manual approval and
verifiable builds. Acceptance includes discretionary reputation review. The
certificate publisher is SignPath Foundation, not the maintainer's name.
[GitHub integration](https://docs.signpath.io/trusted-build-systems/github)
requires uploaded workflow artifacts and GitHub-hosted runners for OSS builds.
Tasks 1/2/6 therefore need a provider-specific CI design after acceptance;
local SignTool signing is not the Foundation route.

Prepared application details: AudioRouter; maintainer MrDesjardins; repository
https://github.com/MrDesjardins/audiorouter; Windows 11 x64 visual routing,
DSP, local recording, user-installed plugins, CLI/MCP and user-directed LAN
streaming. Proposed signed files are shell, CLI, plugin worker and NSIS
installer/uninstaller. Only local unsigned 0.0.1/0.0.2 releases exist. Do not
claim public release history or completed clean-machine qualification.

Next ordered actions:
1. Review dependency rights and prepare a truthful code signing policy with
   maintainer signing roles and privacy statement. Confirm MFA separately.
2. Resolve the existing-release prerequisite with SignPath and the user.
   Any unsigned bootstrap publication requires explicit authorization and
   cannot waive M08 gates. Version/history choices remain pending.
3. With explicit enrollment authorization, apply at
   https://signpath.org/apply using the project details above and the user's
   contact information entered privately. No application has been submitted.
4. After acceptance, configure provider-approved hosted CI and artifact
   handling, including executable metadata and NSIS uninstaller signatures.
   Sign companions before packaging; verify all returned signatures and
   regenerate checksums from signed bytes. Resume tasks 3–9 and retain all
   M08 gates. Never store the provider token in source control.

No build or signature validation was run for this provider. Scope remains
DIST-05/06/07/08 and ENG-04/05. Rollback is local documentation/LICENSE removal
before publication; preserve the unsigned tooling until signing is qualified.

- Scope: M08, DIST-05/06/07/08 and ENG-04/05; signed artifacts,
  provenance, fresh-install and standard-user installation evidence.
- Initial working tree was clean at `2181bf59`; local tags are `v0.0.1`
  and `v0.0.2`. No remote mutation was performed.
- Windows SDK x64 SignTool is installed in both 10.0.26100.0 and
  10.0.28000.0. Provider credentials and identity validation are unconfirmed.
- A user shell is running (PID 48904, default release executable).
  Do not rebuild that executable or run the fresh-install test until the
  user closes it; the process was left untouched.
- Inspected preparation, verifier and verifier regression scripts. Preparation
  is unsigned-only; verification rejects signed manifests. Signing integration
  remains unimplemented until task 1 establishes the selected provider.
- Requested provider, legal publisher/country, version, visibility, historical
  tags and local/CI decisions. No defaults have been accepted yet.
- Current provider references: [Microsoft signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
  lists Artifact Signing at approximately USD 9.99/month and individual
  availability in the US/Canada; [SignPath terms](https://signpath.org/terms.html)
  describe eligibility for free open-source signing. Recheck actual enrollment
  terms before purchase; no purchase or enrollment has been performed.
- Verification: read-only Git/process/tool inventory only. No build, signature,
  fresh-install or clean-machine pass is claimed.
- Next action: resolve the required decisions, then sign and verify a throwaway
  CLI copy (task 1). Follow tasks 2–9 in order. Existing rollback applies;
  preflight changed documentation only.

## Release outcome

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
    `target/release/audiorouter-cli.exe`, `audiorouter-plugin-worker.exe`
    and (from 0.0.14) the packed Stream Deck plugin
    `tools/streamdeck/dist/com.mrdesjardins.audiorouter.streamDeckPlugin`
    as resources, so `npm run pack --prefix tools/streamdeck` must run
    before the NSIS build (`prepare-artifacts.ps1` and the installer smoke
    do);
  - published assets (0.0.14 on): installer, Stream Deck plugin, checksums,
    manifest, SBOMs and notices only; see
    [distribution](../../operations/distribution.md#what-a-release-contains-from-0014);
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
