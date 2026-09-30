# Distribution status and intended release path

Status as of 2026-09-28: the repository builds an unsigned per-user NSIS
installer and includes a manually dispatched GitHub workflow that creates a
draft release. No release has been published or qualified on a clean machine;
the installer remains an M08 candidate. Use the [development quickstart](quickstart.md)
for source-based setup and [M08 evidence](../plans/active/evidence/M08-release.md)
for qualification status.

The requested direction is a per-user Windows 11 x64 setup package that
installs the desktop app and its CLI/plugin worker companions, initializes
local app state, and guides the user through supported endpoint discovery.
AudioRouter currently routes through existing VB-Cable, Voicemeeter, physical
WASAPI, or compatible installed endpoints. The installer will not provision
an AudioRouter-owned virtual device under the current scope, install or remove
third-party drivers, or silently change Windows default audio devices. A
machine without a supported endpoint must receive clear setup guidance; the
product cannot promise a virtual microphone on a clean machine under DEC-16.

The intended official release remains maintainer-driven. A maintainer will
explicitly run a release script or manually dispatch a GitHub Actions
workflow. It will build and verify assets and create a GitHub **draft** release
for human review. Publishing the release remains a separate manual action.
No push-triggered publication or automatic updater is planned. Each release
candidate should include a setup package, release notes, checksums, a
provenance manifest, SBOMs, and third-party notices. Drafts and source ZIPs
are not equivalent to an installed application.

The implementation and runbook are in [Windows distribution planning](../plans/future/windows-distribution.md)
and `tools/release/`. The checked-in workflow requires a reviewed existing
`vMAJOR.MINOR.PATCH` tag matching all app version files, validates its ancestry
against the default branch, runs acceptance checks, and creates a verified
GitHub **draft**. Maintainers inspect and publish it manually.

The user selected an unsigned first app release. Release notes and the
manifest must state that the app and installer are unsigned; Windows trust
prompts are expected. App signing is separate from driver signing. The
AudioRouter-owned driver and driver signing remain excluded under DEC-16.
Standard-user install, upgrade/rollback, uninstall/data retention,
clean-machine and missing-WebView2 qualification remain open M08 gates, so do
not present the current bundle as a supported consumer release.

The localhost API embeds Swagger UI 5.33.0 assets from its official npm
distribution. License, NOTICE and bundled dependency notices are retained in
`src-tauri/http-assets/` and available offline from the documentation page's
license link (`/licenses/swagger-ui.txt`). Include this vendored dependency in
the release rights/SBOM review; the UI npm dependency tree alone does not cover it.
