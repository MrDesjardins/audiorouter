# Active plan — post-0.0.8 maintenance

Updated 2026-10-04. v0.0.11 published as an unsigned prerelease; see [0.0.11 evidence](evidence/2026-10-04-release-0.0.11.md).

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

### Release 0.0.12 (2026-10-04, user authorized)

Objective: publish the Stream Deck changes as an unsigned experimental
prerelease and push all committed code to main. Requirements: HTTP-07/09,
AUTO-15, UI-17, DIST-01–08; Stream Deck phase 0 and subsequent user-requested
actions are recorded in the linked Stream Deck plan.

Ordered tasks: bump AudioRouter manifests by package name; integrate the
validated packed Stream Deck plugin into release assets and checksums; run
locked workspace/shell tests, UI typecheck/unit/Edge suites, plugin
typecheck/tests/pack, contracts and documentation checks; commit named paths,
tag clean source, push main/tag; build and verify draft assets; run the exact
packaged fresh-install check; publish prerelease and verify downloaded assets.

Prerequisites: Windows/MSVC, existing locked dependencies, GitHub credentials;
the user's running shell must close before rebuilding/first-run validation.
No hardware audio test or Stream Deck device operation is part of packaging.
Evidence: `evidence/2026-10-04-release-0.0.12.md` (created at handoff), logs under
`target/release-012-*`. Hardware, sign-in, install/upgrade/uninstall, missing
WebView2, accessibility and signing gates remain open. Rollback: retain
0.0.11 and compatible configuration/recording backups; never replace published
assets. Next action: release validation and package preparation.

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

4e. Done 2026-10-03 (user request, a friend's stereo interface): mono input
   (CAP-03). Input Device `channelMode` stereo/mono/left/right, validated in
   the domain and folded by `fold_input_channel_modes` into the input's
   outgoing matrices at every engine compile entry (after width
   harmonization), so all workers and live saves get it; no realtime code
   change; skipped while surround renders. Inspector "Channels" select with
   reserved guidance height (UI-17). Tests: engine
   `input_channel_mode_sends_one_interface_input_to_both_ears` (samples
   through the compiled graph), domain validation, `InputChannelsField.test.tsx`,
   `input-channels.pw.ts` (three themes, no shift, no commit). Workspace
   control 228, engine 158, domain 72, UI 469 pass. Not yet: attended
   listening on a stereo interface. Rollback: revert the fold calls; saved
   `channelMode` values then stop validating and must be removed.

5. Done 2026-10-04 (user request): desktop UI 800–1000 MB during playback.
   Not a leak: garbage. Measured on the user's database with the release
   shell and `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`
   (CDP, read-only): renderer 65 MB stopped, 600–960 MB after 6 min of
   playback; JS heap 100.7 MB used / 73k listeners before a forced GC,
   13.5 MB / 1.5k after, and the renderer fell to 92 MB. Allocation 237 MB
   per 5 s. Cause: the 50 ms `system.diagnostics` refresh set App state
   although only `nodeTelemetry` changes between ticks, re-rendering about
   5,200 elements (4,000 of them always-hidden legacy panels) 20×/s. Fix:
   `ui/src/liveTelemetry.tsx`; a meter-only refresh goes to a store that
   only the canvas, inspector and Timing views subscribe to; any other
   change still updates App state. After: allocation 149 MB per 5 s,
   renderer plateau 440–470 MB over 5 min of playback. Tests:
   `liveTelemetry.test.tsx`, UI 471, Edge 212/212. Remaining: the canvas
   rebuilds every node card as new JSX each tick (React Flow re-diffs all
   nodes); memoized custom node types would cut most of the rest but touch
   the measured-size/selection/drag regressions, so it is proposed, not
   done. The always-hidden legacy panels could also be unmounted (tests
   query some of them). Rollback: revert the `LiveTelemetry` wrappers and
   the refresh change in `App.tsx`.

6. Done 2026-10-04 (implementation below; results): the canvas element is
   built once per App render and `LiveCanvasContext` feeds node visuals,
   connection lights and Duck links. Edge harness (12 nodes, 8 live links,
   meters changing every tick): card renders over ~40 ticks 0 (was every
   tick), allocation 312 → 186 MB per 5 s (dev build). Real shell, user's
   database (23 nodes, 19 meters, playing): 61 MB per 5 s (0.0.9: 149,
   0.0.8: 237); live JS heap 13.2 MB; a forced GC took the renderer from
   ~770 MB to 165 MB. The process still climbs because V8 defers full GC
   while allocation pressure is low and RAM is free: uncollected garbage,
   not a leak. Tests: `canvas-live-performance.pw.ts` (0 card renders,
   lights and meters move, Stop clears lights at once, three themes), UI
   471, Edge 216 (213 + 3 theme cases) / production 27. Possible next step:
   cap the WebView2 V8 heap (`--js-flags=--max-old-space-size`) so garbage
   is collected sooner; needs a measured trial.
   Item 7 below follows up on the remaining process growth.
   Original plan (in progress until the results above):
   canvas performance
   without regressions (UI-17, M05 canvas lessons). Cause: every telemetry
   tick re-renders `SessionFlowCanvas`, which rebuilds each node's
   `data.label` JSX (handles, header, ports, `NodeVisual`) and edge data and
   hands React Flow new `nodes`/`edges` arrays, so every card re-renders and
   React Flow re-adopts every node.
   Steps: (1) baseline in the Edge harness with telemetry changing every
   refresh: allocation per 5 s and card render count; (2) build the canvas
   element once per App render (not inside the per-tick render prop) and
   pass the slow diagnostics plus `telemetryStore`; (3) a canvas live
   provider merges store telemetry (same basis rule as `LiveTelemetry`) and
   owns the 160 ms freshness flag; only `NodeVisual`, the edge components and
   the Duck trigger edge read it through context, so the canvas, its node
   array and React Flow's store change only on real state changes;
   (4) measured sizes, selection, drag positions and edge sides stay where
   they are.
   Validation: new Edge test with changing telemetry asserting meters and
   edge signal classes update while card renders stay flat, plus an
   allocation budget; full Edge suite (canvas, drag, measured-size,
   inspector, connection tests), UI unit tests, three-theme check of a
   playing canvas; then a CDP measurement in the release shell on the
   user's database. Rollback: revert the canvas/App commit; 0.0.9 is the
   released baseline.

7. Done 2026-10-04 (user request): cap the WebView2 V8 heap. Trial on the
   user's database with the canvas build, playing (19 meters), cap passed
   via `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`; V8 reported a 352 MB heap
   limit. Over the same ~6 min of playback the renderer levelled at
   415–430 MB (uncapped: climbing to 750–770 MB); JS heap 158 MB used of
   352. CPU profile over 5 s: garbage collector 1.0% of main-thread
   samples; the renderer used ~1.1 cores, mostly native rendering
   ("(program)" 61%, script 0.54 s of 5 s), i.e. painting the animated SVG
   connections, unrelated to the cap. Shipped as `WEBVIEW_BROWSER_ARGS` in
   `src-tauri/src/main.rs` (wry's default `--disable-features=...` kept,
   since setting arguments replaces it); shell test
   `webview_arguments_keep_wry_defaults_and_cap_the_js_heap`. Risk: a live
   heap above 256 MB would crash the page (live heap is ~13 MB). Rollback:
   remove the `.additional_browser_args` call. Over the full 15 min capped
   run the renderer stayed at 340–450 MB with periodic full-GC drops. The
   built shell, launched with no WebView2 variable set, starts its page with
   the cap and the wry defaults and no debugging port. Open: the rendering CPU of
   the animated connections deserves its own measurement.

8. Done 2026-10-04, released in 0.0.10 (user request): free the WebView when the editor
   is closed to the tray. A hidden window keeps the page (340–450 MB), GPU
   (~150 MB) and WebView2 browser processes and its timers alive; audio is
   owned by the shell's backend, not the page. Decision (user): close
   destroys the window when the page reports no unsaved route edits, and
   only hides it when there are some, so nothing is lost; tray Open
   recreates it (fresh page, same database and running session).
   Steps: UI reports unsaved state to the shell (`set_ui_unsaved`); the
   shell's CloseRequested and tray Close choose hide or destroy; the app
   no longer exits when its last window closes (only Quit exits); tray
   Open/Quit/privacy work with no window. Validation: shell unit tests for
   the decision and exit rule; UI test for the report; attended check:
   close → WebView2 processes exit, shell and audio keep running, pipe
   answers; tray Open → UI back with audio live; unsaved edit → close
   hides, edit still there on Open; Quit still finalizes and exits.
   Rollback: revert the commit (hide-only behavior).
   Extended (user request, same day): tray Play and Stop audio (the
   window's saved-route Play: `nativePaths.prepare` then `session.start`,
   via `src-tauri/src/tray_playback.rs`); autoplay when AudioRouter starts
   (`shell-settings.json` beside the database, Advanced → When AudioRouter
   starts); sign-in registration now `"<exe>" --tray`, starting with no
   window; the selected session persists in the backend
   (`control_settings.activeSessionId`) so a restart plays the right route.
   Decision: autoplay is a shell launch preference stored by the shell, like
   the Run registration; CLI/MCP do not launch the shell.

9. In progress 2026-10-04 (Joe's report: Network Send/Receive between two
   computers silent): his logs (laptop sender, desktop receiver, both
   0.0.9) show both sessions playing at the same time (18:44:32–18:47:09
   and 18:49:45–18:50:36 UTC) with no failed call, but contain no network
   detail. Added `network.jsonl` (control `network_log.rs`): socket
   start/failure with Windows error code, summaries every 5 s for the first
   minute then 30 s (packets, losses, rejected senders, the sender's real
   source address, the receiver's address toward the sender), a final
   summary on stop, and plain-language hints (nothing arrived → firewall or
   address; audio from another address → set it; 10054 → receiver port
   closed). Logs tab asks for network.jsonl from both computers. Tests:
   network_log unit tests, real UDP loopback asserting summaries, socket
   tests. Released in 0.0.10. Also (after 0.0.11): the window names each
   computer's real address while playing (receive telemetry `thisAddress`,
   send `localAddress`/`lastErrorCode`; inspector help and diagram caption;
   "receiving computer is not listening" for 10054), asserted over real
   UDP and in `network-receive-hint.pw.ts` (three themes reviewed). Next:
   Joe retries on the next release and sends both folders.

12. Done 2026-10-04 (user decision): Setup → Animated connections, On
   (default) / When AudioRouter is in focus / Off, remembered per computer
   (`audiorouter.ui.flow-animation`). Measured in the Edge harness (12 nodes,
   8 live connections): main thread 53% with comets, 18% without (script
   13% → 10%); the travelling comets are about two thirds of the canvas
   work. Off and an unfocused window reuse the reduced-motion rendering
   (colour, glow, meters and arrows keep updating; comets stop) through
   `FlowMotionProvider`, so focus changes re-render only the connection
   layers. Tests: `FlowAnimationSetting.test.tsx` (storage, choices, comets
   on/off/blur/focus), `canvas-live-performance.pw.ts` three themes (switch,
   blur pause, focus resume, remembered after reload; label fit reviewed).
   UI 481, Edge 222.

13. Done 2026-10-04 (user request, list A): (A1) a second launch of the same
   program shows the running instance's window (per-user event
   `Local\AudioRouter.ShowWindow.<SID>`; another build keeps the close-it
   dialog), `577f1526`, unit-tested, live check pending with the release;
   (A2) always-hidden legacy panels no longer render (harness 1,110 → 597
   elements), 70 App tests moved to the visible tabs, 5 tests of unreachable
   UI removed, the recording library surfaced in the Recording tab
   (`recording-library.pw.ts`, three themes), `3e5cad65`; device-list and
   session-list errors were shown only in the removed panels and are now
   unread state (follow-up: show them in Setup and Session); (A3) future
   index corrected (denoise/dehum/declick, Duck, FIR, Time Shift, Input
   Switch exist) and the [Stream Deck plan](../future/stream-deck.md) added;
   spec 16 HTTP-07 now states the backend remembers the selected session.

10. Done 2026-10-04 (user report on 0.0.10): Advanced → Start at sign-in
   showed "Native registration: unavailable" and "unavailable in this
   host" in the desktop app. Cause: `createInitialBackend` took the
   host-bridge branch (the shell injects `__AUDIO_ROUTER_HOST__`) and
   dropped the shell's `startup_register`/`startup_status`, present only on
   the Tauri-core branch; never worked in the app. Fix passes them whenever
   the Tauri core exists (`c6e9b2d8`); `host.test.ts` covers bridge plus
   core and fails on the old code. Live check 2026-10-04 in the user's
   rebuilt shell: Native registration unregistered → Plan → Apply →
   "registered"; HKCU Run AudioRouter = `"<repo>\src-tauri\target\release\audiorouter-shell.exe" --tray`;
   shell-settings.json autoPlay true. Not yet: an actual sign-in. Note:
   the Run value names this exe; switching to an installed build needs
   Disabled applied from this build first (ownership check). Release
   0.0.11 pending.

11. Done 2026-10-04 (user request, option A of the
   [hardware control panel plan](../future/hardware-control-panel.md)):
   local-network HTTP listener (HTTP-09; spec 16 decision, 13-security,
   10-api, 09-interface amended). API tab "Who can connect": This PC only
   (default) or one of this PC's private/link-local addresses
   (`src-tauri/src/lan_addresses.rs`, `GetAdaptersAddresses`, connected
   non-loopback adapters). `HttpApi::start_on` keeps the loopback listener
   and adds one on exactly that address and port; refuses non-private
   addresses; network peers must be private/link-local; Host must be one
   listener's exact `address:port` and a sent Origin must match it. Same
   token, grant, desktop-only methods and rate budget. Not remembered across
   launches; regeneration keeps the address. Tests: shell
   `network_listener_accepts_only_private_addresses_hosts_and_peers`,
   `network_listener_serves_its_address_beside_loopback` (real bind on this
   PC's 10.0.0.73: 200 with its Host, 403 with a foreign Host, OpenAPI lists
   both servers), shell 58/0; `ApiPanel.test.tsx` network case, UI 477;
   `api-panel.pw.ts` three themes, screenshots reviewed (dark, light, high
   contrast). Not yet: a request from a second device (the Pi) and the
   Windows Firewall prompt in the release shell. Rollback: "This PC only";
   revert the commit to restore loopback-only.

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
