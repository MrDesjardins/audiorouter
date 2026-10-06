The latest release is [0.0.14](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.14),
an unsigned experimental prerelease. Download `AudioRouter_0.0.14_x64-setup.exe`
from its assets. Every release from 0.0.3 on was built from a clean tag
with `tools/release/`, passed the exact-package fresh-install check, and had
its published assets downloaded and verified; see the
[release notes](release-notes.md) and the evidence files under
`docs/plans/active/evidence/` (for example
[0.0.7](../plans/active/evidence/2026-10-03-release-0.0.7.md)). Signing,
clean-machine install/upgrade/uninstall and missing-WebView2 checks remain
open.

## What a release contains (from 0.0.14)

One download installs everything. `AudioRouter_<version>_x64-setup.exe` is a
per-user NSIS installer containing:

| Part | Where it lives after install |
| --- | --- |
| Desktop app: window, tray, embedded UI and the audio/control backend (one process, `audiorouter-shell.exe`) | install folder |
| `audiorouter-cli.exe` (CLI and MCP server) | install folder (resource) |
| `audiorouter-plugin-worker.exe` (isolated VST host) | install folder (resource) |
| Stream Deck plugin (`com.mrdesjardins.audiorouter.streamDeckPlugin`) | install folder (resource); **API → Install Stream Deck plugin** hands it to the Stream Deck app |

The release page carries only:

| Asset | Who needs it |
| --- | --- |
| `AudioRouter_<version>_x64-setup.exe` | everyone |
| `com.mrdesjardins.audiorouter.streamDeckPlugin` | a Stream Deck on another PC that controls AudioRouter over the network (the same file is inside the installer) |
| `SHA256SUMS.txt`, `release-manifest.json` | verifying downloads and provenance |
| `sbom.cargo.json`, `sbom.npm.json`, `sbom.npm.package-lock.json`, `sbom.streamdeck.json`, `sbom.streamdeck.package-lock.json`, `THIRD-PARTY-NOTICES.txt` | license and supply-chain review |

Up to 0.0.13 the page also listed `audiorouter-shell.exe`,
`audiorouter-cli.exe`, `audiorouter-plugin-worker.exe`, `audiorouter-ui.zip`,
`run-vb-cable-desktop.ps1` and `audiorouter-examples.zip`. They were copies
of what the installer already holds (the UI is embedded in the shell), a
developer launch script, and sources available in the repository, and they
made testers unsure which file to take. `tools/release/create-draft-release.ps1`
now opens every release's notes with a "Which file do I download?" section.
No portable (no-install) package is published; the installer is per-user
and needs no administrator rights.

The paragraphs below are kept as history.

[0.0.3](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.3) is published
as an unsigned test prerelease for Joe's device-discovery
failure, clearer logging, instance recovery and usability fixes. Use its
[release notes](release-notes.md)
for changes and open qualification gates. Download `AudioRouter_0.0.3_x64-setup.exe`
directly or extract the full ZIP. The exact packaged shell passed fresh-install
consent regression; downloaded installer/ZIP hashes match the tested artifacts.
See [release evidence](../plans/active/evidence/2026-10-02-joe-prerelease-0.0.3.md).

Update 2026-10-01: [0.0.2 unsigned Windows preview](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.2)
is published as a ZIP of the maintainer's working USB package, including setup,
executables, provenance, checksums, SBOMs and notices. Extract and run
`AudioRouter_0.0.2_x64-setup.exe`. The maintainer reports successful use on two
PCs. The exact shell passes fresh-install consent regression; clean-machine,
upgrade/uninstall and missing-WebView2 gates remain open. Windows may warn
because it is unsigned. SignPath signing is under investigation.

The following 2026-09-28 status is historical.

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
product cannot promise a virtual microphone on a clean machine until the
AudioRouter cable planned under DEC-18 ships as a separate signed package.

The intended official release remains maintainer-driven. A maintainer will
explicitly run a release script or manually dispatch a GitHub Actions
workflow. It will build and verify assets and create a GitHub **draft** release
for human review. Publishing the release remains a separate manual action.
No push-triggered publication or automatic updater is planned. Each release
candidate should include a setup package, release notes, checksums, a
provenance manifest, SBOMs, and third-party notices. Drafts and source ZIPs
are not equivalent to an installed application.

The implementation and runbook are in [Windows distribution planning](../plans/archived/2026-10-03-windows-distribution.md)
and `tools/release/`. The checked-in workflow requires a reviewed existing
`vMAJOR.MINOR.PATCH` tag matching all app version files, validates its ancestry
against the default branch, runs acceptance checks, and creates a verified
GitHub **draft**. Maintainers inspect and publish it manually.

The user selected an unsigned first app release. Release notes and the
manifest must state that the app and installer are unsigned; Windows trust
prompts are expected. App signing is separate from driver signing. The
AudioRouter-owned driver is planned under DEC-18 as a separate signed
package; current releases do not include it.
Standard-user install, upgrade/rollback, uninstall/data retention,
clean-machine and missing-WebView2 qualification remain open M08 gates, so do
not present the current bundle as a supported consumer release.

The localhost API embeds Swagger UI 5.33.0 assets from its official npm
distribution. License, NOTICE and bundled dependency notices are retained in
`src-tauri/http-assets/` and available offline from the documentation page's
license link (`/licenses/swagger-ui.txt`). Include this vendored dependency in
the release rights/SBOM review; the UI npm dependency tree alone does not cover it.
