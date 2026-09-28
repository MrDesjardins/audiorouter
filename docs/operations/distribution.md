# Distribution status and intended release path

Status as of 2026-09-28: AudioRouter has no supported end-user installer or
published GitHub release. The current scripts prepare unsigned qualification
artifacts; they do not install the app or publish a release. Use the
[development quickstart](quickstart.md) for the current source-based setup.
The detailed proposed target is in the
[Windows distribution plan](../plans/future/windows-distribution.md).

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

The installer and release workflow remain plans rather than operational
instructions until M08 scope, signing expectations, WebView2 policy, and
Windows install evidence are approved and implemented. The existing unsigned
artifact flow must not be described as a supported consumer release.
