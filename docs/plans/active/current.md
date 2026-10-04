# Active plan — post-0.0.7 maintenance (0.0.8 in preparation)

Updated 2026-10-03. v0.0.8 release in preparation (user-requested); see [0.0.8 evidence](evidence/2026-10-03-release-0.0.8.md). v0.0.7 is published.

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
3. Done 2026-10-03 (simulation): [Siege footstep EQ](../future/siege-footstep-eq.md) compressor
   settings simulated with the engine DSP; Balanced/Maximum proposed, awaiting the user's choice by ear.

4a. Done 2026-10-03: 44.1/96 kHz devices (CAP-14, multi-path inputs, outputs,
   endpoint loopback, surround). New opt-in constructors
   (`SharedCapture::open_loopback_at_rate`, `open_bound_at_rate_with_retry`,
   `SharedRender::open_with_headroom_at_rate`) request the endpoint's own
   layout at 48 kHz with `AUTOCONVERTPCM | SRC_DEFAULT_QUALITY`; the
   multi-path engine uses them and accepts float32 at 8–192 kHz. Existing
   constructors and the single-path bridge are unchanged. Unit test for the
   format change; opt-in live test on the user's 96 kHz 7.1 "SteelSeries
   Sonar – Media" (silence only): 48,022 frames/s converted, 96,067
   unconverted. Control 228 and windows-audio 110 tests pass. Not yet:
   continuity harness or attended listening through a 96 kHz device.
   Rollback: revert the constructors' use in control.

4b. Done 2026-10-03: API request builder (HTTP-08, external app integrations
   plan). API tab section builds `POST /api/v1/nodes/set` (follow/pin session,
   tool, Enabled/Bypass/parameter, Mixer inputs by upstream name, Duck trigger
   node by name) with catalog validation, JSON/curl/PowerShell and a token
   placeholder; Send applies once via `nodes.set`. Catalog adds Duck `trigger`,
   `keyNodeId` (`"reference": "node"`) and phase booleans; the inspector's
   generic editor filters them (the Duck editor owns them). `e2e_backend`
   allows `nodes.catalog`/`nodes.set` (graph edits, no devices). Evidence:
   `requestBuilder.test.ts` (5), `api-request-builder.pw.ts` (3 themes with a
   real Send; Duck-inspector check fails without the filter); UI 464, control
   228. Open: overrides, scenes/batches, per-integration permissions.

4c. Done 2026-10-03: spatial speaker mode and room (CAP-14, spatial plan).
   `BinauralRenderer::with_options` adds RACE crosstalk cancellation
   (`spatialMode: "speakers"`) and a four-line FDN room
   (`spatialRoomPercent` 0–100); defaults stay bit-identical. Domain accepts
   both; the running capture keeps its options and a change reports restart
   required. UI adds the speakers mode and a Room slider (three themes
   reviewed). Objective tests: 1 kHz ear separation 4.4 → 18.8 dB in a
   simulated speaker-to-ear path; room tail present and −51 dB from
   50–150 ms to 400–500 ms; bounded on full-scale 7.1 noise. Not yet: attended
   listening on speakers or with the room.

4d. Done 2026-10-03 (user request): version in the window title and header,
   and an optional daily new-version check (UI-18; PROD-06 amended with this
   one user-authorized network read). UI reads the public GitHub releases list
   (CSP `connect-src` adds only `https://api.github.com`), caches it a day,
   compares `vX.Y.Z` tags including prereleases, and shows "<version>
   available" on the header line; `open_release_page` opens only this
   repository's validated release URL. Setup → New versions turns it off.
   Tests: `updateCheck.test.ts`, shell `release_page_opens_only_this_repository_for_numeric_tags`,
   `update-notice.pw.ts` (three themes, injected list, zero GitHub requests).

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
