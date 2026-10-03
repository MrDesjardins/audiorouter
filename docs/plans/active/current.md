# Active plan — post-0.0.5 maintenance

Updated 2026-10-02. User-requested unsigned experimental prerelease published.

## Objective and scope

Track support and remaining qualification after v0.0.5. No new implementation
is authorized by this plan. Preserve sessions, local API credentials, device
formats and published assets; no VST changes. Requirements DSP-19, GRAPH-08/14/15,
UI-05/11/12/13, AUTO-15, ARCH-04, SEC-01/10, DIST-01–08 remain traceable through
[release evidence](evidence/2026-10-02-release-0.0.5.md).

## Where things stand

[v0.0.5](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.5) is
public, explicitly unsigned/experimental/prerelease. Source and tag point to
6e8148fcfcb6b4b85c47d3e007d8149b17ca2ae0; documentation updates are pushed separately.
Locked workspace, shell, 453 UI, contracts/example and 40 production Edge checks
pass. Exact packaged first-run passes 1/1 after user app closure. Public assets
were downloaded and all hashes/provenance verified. No live cable test or device
format change in the publication turn.

## Remaining qualification and next action

Real-match Duck/failure release, native Quit process exit, combined quiet-tone
continuity, live path-change and spatial listening remain open. Full M08 clean-
machine install/upgrade/uninstall, WebView2 absence, hardware/endurance,
accessibility and signing remain open and disclosed. These are not implied by
portable tests or this experimental publication.

Exact next task: user testing/support; on a new request, plan attended/native
qualification with a quiet reference and closed competing apps. Do not run new
implementation or disturb the user's audio autonomously after this release.

## Risks, evidence and rollback

Stats.cc's undocumented feed can change; unknown state releases Duck. Independent
clock drift/queue latency remain limits. Keep v0.0.4 and compatible configuration/
recording backups for rollback; no migration or driver install. Published assets
are immutable; repairs use a new version.

Completed publication history: [execution record](../archived/2026-10-02-release-0.0.5-published.md).
