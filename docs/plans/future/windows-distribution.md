# Windows distribution and manual GitHub release plan

Status: planning requested 2026-09-28; implementation is not authorized by
this plan. The user asked for an easy integrated install and a manual official
release process. This plan defines a reviewable route from the current
unsigned artifact preparation to a per-user Windows installer and GitHub
draft releases. The driver and its production signing remain on ice under
DEC-16. Authenticode signing of the desktop app/installer is a separate,
unresolved choice; this plan does not authorize a signing purchase/account
setup or automatic publication.

## Outcome and boundaries

The target is one x64 Windows installer that installs AudioRouter and its
runtime companions, creates the app's local state automatically, and guides a
first-time user through selecting supported audio endpoints. A user should
not need to build the UI/backend, edit environment variables, create a
database, or manually place the CLI/plugin worker. The app remains offline
after installation, consistent with PROD-06.

The supported audio boundary remains existing VB-Cable, Voicemeeter, physical
WASAPI, and other compatible installed endpoints. The AudioRouter installer
must not install/remove third-party audio drivers or silently change Windows
default devices. It can detect missing endpoints and provide clear guidance;
it cannot provide a virtual microphone without changing the existing
no-owned-driver decision (DEC-16). Therefore “install and it works” means the
app installs cleanly and its onboarding leads a user with a supported endpoint
to a working route. It cannot promise an AudioRouter-created endpoint on a
clean machine under current scope.

Initial install should be per-user and not require elevation. Preserve the
existing `%LOCALAPPDATA%\AudioRouter\state.sqlite` state location. Setup,
upgrade, repair, and uninstall must preserve user sessions and recordings by
default; uninstall must leave external audio drivers/endpoints alone. No
automatic updater is planned. No plugins are redistributed; the app reports
missing or incompatible plugins and lets the user install them separately.

## Proposed package and first-run experience

Deliver a setup EXE. Decide separately whether the shell and installer receive
Authenticode signatures; if unsigned, label the package clearly and measure
the Windows trust/reputation experience. The package
contains the desktop shell, the adjacent CLI and plugin-worker executables
needed by integrations and plugin hosting, required app resources, and
versioned notices. The CLI/worker lookup path must be verified after install;
the current artifact script puts all three executables together, which is a
useful starting layout. The installer is the primary user download; a ZIP may
remain as a diagnostic/portable option, not as the normal setup path.

On first launch, create or migrate app state without requiring manual paths,
discover available endpoints, explain which endpoint is needed for the
selected workflow, and offer a guided route/template. Show unavailable or
unsupported capabilities before Play. Do not start audio, switch system
defaults, or bind a protected microphone without deliberate user action.
Machine-specific endpoint IDs must be revalidated and explicitly rebound
after importing a session on another PC. Keep setup distinct from endpoint
vendor installation: if no supported endpoint exists, give the user a clear
supported-device route and stop short of promising a working virtual
microphone.

WebView2 behavior needs a tested policy. Prefer the Windows-provided runtime
when present and a small bootstrapper when an internet connection is
available; decide whether an offline installer is necessary before
implementation. Do not silently make the normal installer a roughly 127 MB
offline runtime bundle without a stated offline-install requirement.

## Manual release mechanism

Use a version tag `vX.Y.Z` and require it to match the Tauri app version,
Cargo shell version, and release manifest. The existing `0.1.0` values are
aligned in two files, but the release process should validate all version
owners and fail before building on a mismatch. A release must use a clean,
reviewed commit and a reproducible Windows x64 toolchain.

The maintainer explicitly starts a local PowerShell release script or a
GitHub Actions `workflow_dispatch` workflow. A manual workflow is useful for
a clean hosted build; it must not run on pushes or pull requests. It builds,
runs the declared release checks, stages artifacts, verifies hashes and
provenance, then creates a **draft GitHub Release** attached to the selected
tag. The operator reviews the draft, notes, assets, and checksums, then
publishes it manually. There is no auto-publish and no automatic updater in
this track. A script may implement the draft/upload step after the release
workflow is approved.

Each draft should contain the setup EXE, optional ZIP/debug CLI package,
release notes, SHA-256 checksums, `release-manifest.json`, SBOMs, and
third-party notices. The manifest identifies version, source commit, target,
toolchain, artifact hashes, and signing status. Generated GitHub source
archives are not the application installer. The release job should use
minimal `contents: write` permission only for the release-creation job, pin
third-party Actions by full commit SHA, and avoid exposing release credentials
to pull-request builds.

The current `tools/release/prepare-artifacts.ps1` is a useful unsigned
foundation, but its native build command omits `--features custom-protocol`
even though the repo's validated release lesson requires that feature for a
plain Cargo shell release build. It also declares all output
`publicationReady = false`, has signing/installer blockers, and has no draft
upload. Before this foundation can produce a release candidate, reconcile
those contracts with the newly requested app installer scope; do not weaken
the signing or driver gates by inference.

## Scope and requirement reconciliation

Relevant existing requirements include PROD-01, PROD-03, PROD-04, PROD-06,
PROD-07; SEC-11 (currently future-only); ENG-04/05; and the M08 release
contract. Existing v1 excludes production driver signing, installer, and
clean-machine gates by DEC-16. The 2026-09-28 user request reopens **planning** for
app installation and GitHub draft-release mechanics. Before implementation,
revise the M08 contract and traceability with an explicit user-confirmed
decision about which installer and clean-machine requirements return to v1.
DEC-16 remains in force for the owned driver and its production signing.
App/installer Authenticode signing is undecided and must be evaluated as its
own cost/trust choice. App packaging can be explored without driver work.
Whether a release is official is determined by the approved release process
and manual publication; signing affects publisher identity and the trust
experience, not whether a human-published GitHub release is official.

No new requirement IDs are assigned in this planning document. If promoted,
add durable acceptance IDs for installer install/upgrade/repair/uninstall,
first-run endpoint setup, release provenance, draft creation, and rollback;
map each to PROD/SEC/ENG and M08 in `docs/spec/15-delivery.md`.

## Implementation sequence after scope approval

1. Reconcile DEC-16, M08, PROD release wording, security requirements, and
   requirement traceability. Decide whether app/installer Authenticode signing
   is required for v1, how unsigned trust prompts are handled if it is not,
   and who owns signing credentials; this plan does not authorize purchase or
   account changes. Driver signing remains excluded.
2. Choose Tauri NSIS or WiX packaging after confirming per-user install,
   upgrade, uninstall, shortcuts, WebView2 bootstrap, and resource lookup
   behavior. Favor per-user NSIS unless Windows qualification demonstrates a
   concrete reason to use MSI.
3. Make release metadata and version validation authoritative. Fix the
   custom-protocol build invocation, include shell/CLI/worker/resources, and
   validate the installed worker/CLI paths.
4. Implement a manual build-and-verify workflow or script that emits a draft
   release only. Make the final publish an explicit human action.
5. Implement the onboarding and state-preservation behavior needed to make
   first run understandable. Keep endpoint selection explicit and never
   change OS defaults or start protected capture without consent.
6. Validate install, upgrade, repair, rollback, uninstall, clean-machine
   setup, offline routing after install, signatures, and recovery on Windows
   test machines. Record the tested OS image, WebView2 state, endpoint state,
   account/elevation level, and artifact hashes.
7. Update quickstart, installation, troubleshooting, release notes, and
   operator instructions from observed behavior. Do not claim support beyond
   the tested endpoint and Windows matrix.

## Acceptance matrix

| Scenario | Required evidence |
| --- | --- |
| Standard-user install on supported Windows 11 x64 | Install, launch, shortcut, and no unexpected elevation; clean-machine log |
| WebView2 present/missing and online/offline | Correct detected-runtime behavior and useful failure/recovery instructions |
| Existing state upgrade and repair | Sessions, endpoint choices, and recordings preserved; migration evidence |
| Uninstall with keep/remove choices | App files removed; user data follows explicit choice; third-party endpoints/drivers untouched |
| First run with supported endpoint / no endpoint | Guided discovery and clear capability state; no hidden audio start or OS default changes |
| CLI and plugin worker after installation | MCP/CLI setup and plugin load resolve packaged adjacent resources |
| Manual release from clean commit | Version/tag check, required tests, manifest, SBOM, notices, hashes, provenance, draft asset upload |
| Release rejection or rollback | Draft can be discarded; prior installer remains discoverable; documented data compatibility/recovery path |
| Signature and reputation state | Authenticode verification and first-run trust UX measured on a clean Windows account |

Linux tests, mock installers, and successful artifact creation do not satisfy
these Windows installation gates.

## Risks, rollback, and next decision

The largest product limitation is the external endpoint prerequisite. The
largest delivery risks are unsigned-download trust warnings, WebView2
availability, resource lookup after installation, endpoint identity migration,
and accidentally implying that a draft is a published release. Keep the
current unsigned artifact flow reversible while the installer is qualified;
do not replace the known-good development/test route until the installed
route passes the full acceptance matrix. Each installer migration must retain
a restorable copy of user state before changing its schema.

Next task after this planning handoff: explicitly promote a bounded
Windows-app installer and manual GitHub draft-release track into M08, or keep
it future-only. At that gate decide app signing, WebView2 offline support,
and whether the existing external endpoint prerequisite is acceptable. Driver
signing is not part of this decision.

## External references

- [GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
  attach assets and release notes to tags; source archives are separate from
  app packages.
- [Manually running a GitHub Actions workflow](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)
  documents the `workflow_dispatch` operator path.
- [GitHub workflow permissions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)
  documents scoped `GITHUB_TOKEN` permissions.
- [Tauri Windows installers](https://v2.tauri.app/distribute/windows-installer/)
  documents NSIS/WiX and WebView2 distribution choices. Recheck current
  version-specific behavior at implementation time.
