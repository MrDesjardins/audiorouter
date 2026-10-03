# Active plan — post-0.0.7 maintenance

Updated 2026-10-03. v0.0.7 is published as an unsigned prerelease; see
[0.0.7 evidence](evidence/2026-10-03-release-0.0.7.md).

## Objective and scope

Support users of the published releases and close the remaining qualification
gates. Preserve sessions, local API credentials, device formats and published
assets. Requirements DSP-19, GRAPH-08/14/15, UI-05/08/11/12/13/17, AUTO-15,
ARCH-04, SEC-01/10 and DIST-01–08 stay traceable through the release evidence.

## Where things stand

- [v0.0.7](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.7)
  (tag `ac29df97`): Rust workspace 979/0, shell 44, UI 458, full development
  Edge 202/0, exact-package fresh install and published-asset verification
  pass. Unsigned.
- Completed work of 2026-10-03 (0.0.6/0.0.7) is recorded in the
  [execution record](../archived/2026-10-03-releases-0.0.6-0.0.7.md). Earlier
  history is indexed in [archived plans](../archived/README.md).
- Other active plan: [app/installer signing](release-signing-and-publication.md),
  blocked on the user's provider choice and enrollment.

## Decisions (2026-10-03, user)

- Signing: keep publishing unsigned prereleases while the project builds the
  public track record SignPath Foundation expects; apply later. The signing
  plan stays parked.
- Authorized now, in this order after items 1–2: Siege compressor
  simulation (item 3); Surround to headphones for 44.1/96 kHz 5.1/7.1 devices
  by resampling; the API request builder from the
  [external app integrations plan](../future/external-app-integrations.md);
  spatial speaker mode and distance/room controls from the
  [spatial audio plan](../future/spatial-audio.md). Each gets its own section
  here with requirement IDs, steps, validation and rollback before coding.

## Open work

Agent work (no hardware or credentials needed):

1. Done 2026-10-03: the intermittent `live-inspector-regressions` autosave
   test (reproduced 9/40 with 8 workers) was a test defect. Clicking the node
   card centre landed on its inline threshold slider, a real edit
   (`thresholdDb` −0.5) that autosave saved 400 ms later; under load "Learn
   again" came after that save, giving two plans. The test now selects the
   node by its title (as other tests do): 160/160 under the same load.
2. Done 2026-10-03: the busy-device message names the VB-Cable "… 16ch" twin
   of the same cable when the last device list contains it (matched on the
   full "(VB-Audio …)" cable name, so CABLE and CABLE-A stay apart).
   `backend.test.ts` regression added.
3. [Siege footstep EQ](../future/siege-footstep-eq.md): simulate the
   compressor on the reference takes and tune after the user's match feedback.

Needs the user (attended, hardware or decisions):

4. Attended confirmation in the shell: drag-to-canvas, device picker on the
   friend's PC, named busy-device message.
5. Native/attended qualification: real-match Duck release, native Quit
   process exit, combined quiet-tone continuity, live path change, spatial
   listening.
6. M08: clean-machine install/upgrade/uninstall as a standard user, missing
   WebView2, hardware/endurance, accessibility (Narrator), and signing.

Exact next task: item 1, then item 2; release 0.0.8 only when a user-facing
change exists. Do not disturb the user's audio or close their apps without
asking.

## Risks, evidence and rollback

Stats.cc's undocumented feed can change; unknown state releases Duck.
Independent clock drift and queue latency remain limits. Keep the previous
installer and compatible configuration/recording backups for rollback; no
migration or driver install. Published assets are immutable; repairs use a new
version.
