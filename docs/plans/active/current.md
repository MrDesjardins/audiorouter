# Active plan — post-0.0.8 maintenance

Updated 2026-10-04. v0.0.12 published as an unsigned prerelease; see [0.0.12 evidence](evidence/2026-10-04-release-0.0.12.md).

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

## Public website (user request, 2026-10-05)

Objective: present the free/open-source Windows product, current tools,
integrations, and beginner setup at `audiorouter.org` without implying that
unreleased driver work is available. Website lives in `site/`; deployment is
prepared by `.github/workflows/pages.yml` and awaits Pages/DNS configuration.

- Current backend contract: 121 methods. The public website catalog is
  generated from public HTTP methods plus convenience aliases; generated
  assets currently contain 106 public backend methods and 111 HTTP operations.
  Local Swagger remains authoritative for the installed version and schemas.
  Catalog generation is `node site/generate-catalogs.mjs`.
- Available-node catalog: 32 currently available node kinds. Descriptions
  and API routes derive from repository contracts/code, while local Swagger
  remains authoritative for request/response schemas.
- MCP setup distinguishes ChatGPT's Secure MCP Tunnel route from Cowork's
  local desktop-plugin route. AudioRouter does not yet ship a Cowork extension
  package; this is stated on the site. Client instructions link to provider
  documentation because availability and UI change over time.
- Performance wording is limited to measured evidence: ~31 MB shell working
  set with a live route after editor close; six WebView processes/~302 MB
  released. The separate 4 MB reading was an idle fresh tray launch before
  device consent, not an active audio session. Source:
  [0.0.10 release evidence](evidence/2026-10-04-release-0.0.10.md).
- Rollback: remove the site folder and Pages workflow. Next action: review at
  `http://localhost:3000`, then enable GitHub Actions Pages and configure
  domain DNS when ready.

### Website presentation follow-up (2026-10-05)

Objective: improve the tools visual, show current published activity, and make
Stream Deck extension steps discoverable. No backend or app behavior changes.

- Tool catalog icons now mirror the app's tool glyphs (with its Duck SVG),
  wrapped in SVG for consistent vector rendering. The routing illustration
  joins ports exactly and animates signal comets; reduced-motion preferences
  hide the comets.
- Added `site/releases.html`, which reads published release notes from the
  public GitHub Releases API so the page updates as releases are published.
  Added a Stream Deck setup showcase: install the release companion, enter
  local API connection details, and assign supported key actions.
- Validation: catalog generator reports 106 public API methods, 111 HTTP
  operations, 32 node kinds; inspect the page locally in a browser for layout,
  icon alignment, SVG motion, release-feed rendering, and reduced-motion.
  No automated tests requested or run. GitHub API access is required to show
  the release list; the page provides a GitHub Releases fallback.
- Rollback: revert the website-only changes in `site/` and this note. Next:
  visual browser review, then GitHub Pages configuration when ready.

### Website homepage review (2026-10-05, user request)

- Added a homepage "real setup" slot: `assets/setup.jpg` poster, then a muted
  looping `assets/setup.mp4` that loads after page load and near the viewport,
  pauses off-screen, and is skipped for reduced motion or data saver. The slot
  shows a placeholder on localhost and is hidden in production until the files
  exist. Recipe in `site/README.md`. The media files are still to come from
  the user.
- Added use-case cards (gamers/streamers/podcasts) and a three-step "first
  route" section that replaces the old bottom CTA. Fixed `.cta .heading`
  losing its centered margin (it affected `siege.html`). The hero caption no
  longer overflows at phone width.
- Verified in Edge/Playwright at 1366×900 and 390×844/Pixel 7: no horizontal
  overflow. With sample media, the poster and video have identical boxes, the
  video plays muted and looped after scrolling, and pauses off-screen. Also
  checked the missing-media placeholder (localhost), hiding on a non-local
  host, and the reduced-motion case (poster only). The site has a single dark
  theme.
- Rollback: revert the `site/` changes and this note.

## Decision (2026-10-05, user): GPL-3.0-only with commercial licensing (DEC-19)

The user asked to stop others from profiting from AudioRouter while staying
open source and keeping the option to earn money later. CC BY-NC was
considered and rejected: it is not an open-source license and Creative Commons
advises against it for software. Chosen: `GPL-3.0-only` for versions after
0.0.13, with commercial licenses offered by the sole copyright holder
(Patrick Desjardins, author of every commit). Releases up to 0.0.13 stay MIT.

- Changed: `LICENSE` (standard GPL-3.0 text from GitHub's license API, since
  gnu.org was unreachable), workspace and shell `Cargo.toml` license fields,
  README License section, new `CONTRIBUTING.md` (contributors grant the
  copyright holder a right to relicense, which dual licensing needs), and
  website wording ("GPL-3.0 open source", footer).
- Compatibility checked: shipped Rust dependencies are MIT/Apache-2.0/BSD/
  ISC/Zlib/Unicode/MPL-2.0 or LGPL-3.0 (LAME), all GPL-3.0 compatible; VST2
  uses AudioRouter's own ABI declarations, not Steinberg's VST2 SDK; the VST3
  SDK is MIT. The MS-PL driver stays a separate program under its own license.
- Not legal advice; a lawyer should review the commercial-license terms
  before any are sold. Rollback: restore the MIT `LICENSE` and fields.
- Next: the next release ships under GPL-3.0; no code change is needed.

## Decision (2026-10-05, user): ship AudioRouter's own virtual cable (DEC-18)

AudioRouter will ship its own signed virtual cable so users do not need
VB-Cable; VB-Cable/Voicemeeter stay supported. Lowest-cost signing
(attestation, EV certificate bought only for driver-release windows).
Recorded in [15-delivery DEC-18](../../spec/15-delivery.md); full plan,
costs and phases in the [driver track](../future/M03-driver-signing.md).

- Requirements: VDEV-01–12, SEC-08, NFR-16, DIST-01/03/07 (amended).
- Prerequisites for phase 1: a Hyper-V VM (Windows 11, Gen2) or a spare PC,
  created by the user; test signing only inside it. Nothing is installed or
  changed on the daily workstation.
- **Execution plan with work packages WP-00…WP-15:**
  [virtual cable plan](virtual-cable.md). Design: [17 Virtual
  cable](../../spec/17-virtual-cable.md). Signing runbook:
  [virtual cable signing](../../operations/virtual-cable-signing.md).
  WP-01 host baseline passed; WP-02 package tooling complete (2026-10-05),
  with [build/signing evidence](evidence/2026-10-05-virtual-cable-wp02.md).
  No driver has been loaded; VM evidence is pending.
- Test procedure (stages A–E, VM to main PC): [virtual cable
  testing](../../operations/virtual-cable-testing.md).
- WP-03 smoke tooling is prepared and WP-04 host bridge hardening is reviewed;
  their VM gates (install/remove, Verifier/fuzz, second-user denial) remain
  pending. WP-05 host endpoint, precision and format work is in progress; see
  its execution record and evidence in the [virtual cable plan](virtual-cable.md).
  Current WP-05 cable descriptors compile for x64/ARM64 and pass portable
  INF/conversion checks, but remain non-loadable until timing/cleanup work,
  WP-06 bridge/Rust compatibility and the VM gates are complete. Host builds
  are not runtime or sound-quality evidence.
- Needs a separate user go-ahead: buying the EV certificate (phase 5) and
  the first public driver release.
- Cables: up to 8 (A–H), user enables 1–8, default 2; develop and test with
  2, qualify all 8 before signing (user, 2026-10-05).
- Open choices: see WP-00 D2–D6 in the [virtual cable plan](virtual-cable.md).
- Rollback: the driver is a separate optional package; releases without it
  keep today's VB-Cable workflow. Revert DEC-18 doc changes to return to
  DEC-16.

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

### Code review P2/P3 follow-ups (2026-10-07, user request)

**P3-1 unsafe audit.** Every `unsafe` block and impl now has a
`// SAFETY:` comment with its invariants (pointer and handle validity,
owner and lifetime, buffer lengths, COM thread). Before: 188 missing
(windows-audio 98, transport 27, plugin-host 19, engine allocator test 4,
shell 26, `tools/m00-wasapi-probe` 14); after: 0. The lints
`clippy::undocumented_unsafe_blocks` and `clippy::missing_safety_doc` are
`deny` in `[workspace.lints.clippy]` (each crate has
`[lints] workspace = true`) and in `[lints.clippy]` of `src-tauri` and the
probe, so all targets, tests and examples included, are covered.
Soundness fixes: `SharedCapture::next_packet_into` and
`SharedRender::submit_bytes` reject `bytes_per_frame` above the stream's
`nBlockAlign` (a larger value read or wrote past the WASAPI buffer);
`created_callback` clones its sender before sending, so `create` may free
the context while `send` runs; `token_user_sid_string` reads `TOKEN_USER`
with `read_unaligned` from its byte buffer; the OS-transition and plug-in
editor windows no longer free their boxed context in `WM_NCDESTROY`, which
could double-free when `CreateWindowExW` fails after `WM_NCCREATE`; the
probe's `raw_capture_initialize` no longer calls `CoUninitialize` before
its interfaces are released.
Open: `NativeBridgeRegion` and `SharedAudioRegion` write through pointers
taken from `MmapMut`'s shared slice (`map.as_ptr()`), which Rust's aliasing
model does not allow; move them to `memmap2::MmapRaw`. The `enumerate_*`
helpers return a `windows::core::Error` (which may hold COM error info)
after `CoUninitialize`.
Local evidence (Linux container, pinned 1.96.0, `x86_64-pc-windows-gnu`):
workspace and shell Clippy with `-D warnings` clean; probe Clippy clean;
`cargo fmt` checks clean; engine, storage, domain, dsp, recording,
protocol and plugin-host tests pass. Windows CI and hardware runs not done
here; the Jev rule check was not run (no tool checkout or key).
Rollback: revert the commit.

### Code review P0 and P1 fixes (2026-10-07, user request)

The user asked to fix every P0 and P1 item in the
[code review](../future/code-review-2026-10-07.md). Commits on
`claude/zealous-archimedes-fz1lbv`: `f8440b20` (P0-3, P1-1), `2cae3569`
(P0-1), `72aa0731` (P0-2), `69ab047c` (P1-3), `0fa970f0` (P1-5),
`8477b97a` (P1-6), `9b1ac2ea` (P1-2) and `e3621dcc` (P1-4).
Decisions: benchmarks use a small `harness = false` bench instead of
Criterion, so no new dependencies; the MCP default tool list is a
hand-picked set of 38 task-level tools (`DEFAULT_MCP_TOOLS`), and every
tool a recipe names must be in it (unit test); the database runs in WAL
mode, so a copy of a running database must include `-wal`
(spec 12, AGENTS.md lesson updated).
Local evidence (Linux container): Windows-target Clippy (mingw) clean for
the transport, control, CLI and windows-audio crates; storage (97) and
engine tests pass, including `processing_never_allocates_for_any_tool_or_layout`;
485 Vitest tests; 146 harness-only Playwright tests in Chromium; error
panels checked in the dark, light and high-contrast themes. Bench, release
build on Linux: tools 23–93 µs per 128-frame quantum (0.9–3.5 % of the
budget); a 32-tool route 694 µs (26 %). Gain alone costs 23 µs, so most of
that is per-route overhead, which is a P2-7 follow-up.
Windows evidence: CI on the branch (see the next action). Transport,
control and CLI tests run only there.
Remaining: P0-1 still runs plug-in worker start/re-hash at Play and
bundle export/import inline. Rollback: revert the individual commits;
the WAL switch is undone by `PRAGMA journal_mode = DELETE`.
Next action: green Windows CI on the branch, then a pull request if the
user asks for one.

### CI back to green on Windows (2026-10-06, user request)

All 500+ `AudioRouter CI` runs through 2026-09-07 failed; the workflow was
later made manual-only. Causes: (1) the Linux job built the whole workspace,
but `windows` 0.62 (`windows-future`) does not compile on Linux and
`windows-audio`, `control`, `transport` and `cli` depend on it; (2)
`cargo fmt --check` and strict Clippy had never passed (1,019 findings were
the generated KEMAR table); (3) the Windows job's
`backup_never_overwrites_an_existing_recovery_copy` failure (2026-09-07).
Decision: CI runs on Windows only (the product is Windows-only), with Rust
1.96.0 and Node 22.14.0 pinned like `manual-release.yml`, on push to
`main`, pull requests and manual dispatch. It adds shell fmt/Clippy/tests
and drops the m04/m05/m07 wrappers, whose steps it already runs once.
Code changes: rustfmt (workspace and shell), Clippy fixes and documented
allows, `rust-version` 1.85, non-Windows worker-spawn stubs, Unix-safe test
cleanup, and a quoted vitest exclude (sh expanded `e2e/**`).
Local evidence: Clippy `-D warnings` clean for the workspace and shell
with `--target x86_64-pc-windows-gnu` (mingw cross-check, Linux); portable
crate tests and 480 UI tests pass on Linux. Windows evidence: CI
[run 37417530916](https://github.com/MrDesjardins/audiorouter/actions/runs/37417530916)
(2026-10-06, `474ddcb`) passed every step, including workspace and shell
tests, and the unsigned installer build with the bundled Stream Deck plugin
plus verification. Three more fixes came from earlier runs: the M00 probe's
stale `Cargo.lock` (re-seeded from the workspace lock), `m01-cli.ps1
-AllowNoAudioEndpoints` for runners without audio devices (attended runs
stay strict), and an explicit `exit 0` in `m06-sdk-installer.ps1`, whose
last check is an expected native failure. Next: open a pull request so CI
runs on `main`; the release-packaging items above still need attended
Windows checks (install, Stream Deck button, three themes).

### Release packaging: one download (2026-10-06, user request)

User found the 0.0.13 release page confusing (16 assets) and asked for one
package containing the app, backend, frontend and Stream Deck plugin.
Finding: the installer already held the shell (UI embedded, backend in
process), CLI and plugin worker; the loose `.exe`s, `audiorouter-ui.zip`,
`run-vb-cable-desktop.ps1` and examples ZIP were duplicates or developer
files. Changes (requirement area M08 distribution, HTTP/Stream Deck):

1. `tauri.release.conf.json` bundles the packed Stream Deck plugin;
   `prepare-artifacts.ps1` and `m08-installer-smoke.ps1` pack it before NSIS.
2. Shell command `install_streamdeck_plugin` opens the bundled file with
   `ShellExecuteW` (Stream Deck asks to confirm); API tab gains
   `StreamDeckPluginPanel` with a reserved status line (UI-17).
3. Published assets: installer, `.streamDeckPlugin`, `SHA256SUMS.txt`,
   `release-manifest.json`, SBOMs, notices. `verify-artifacts.ps1`, its test
   (which still lacked the 0.0.12 Stream Deck entries) and `m08-release.ps1`
   updated; `m08-release.ps1` now rejects loose copies of bundled files.
4. `create-draft-release.ps1` prepends a "Which file do I download?" section.

Checks run (Linux container, 2026-10-06): `vitest` for
`StreamDeckPluginPanel`/`ApiPanel` pass; UI typecheck clean;
`npm run pack` in `tools/streamdeck` produced a package with
`com.mrdesjardins.audiorouter.sdPlugin/manifest.json`.
Not run (need Windows): shell compile, PowerShell release scripts and
tests, NSIS build, installing the plugin from the installed app, three-theme
screenshots of the API tab. Next: on Windows run `test-verify-artifacts.ps1`,
`m08-installer-smoke.ps1`, `m08-release.ps1`, then install the 0.0.14
candidate and press **Install Stream Deck plugin** with and without the
Stream Deck app. Rollback: revert the commit; earlier releases are unchanged.

### Defect 2026-10-05: backend dead after resume ("Backend refresh failed")

Reproduction (v0.0.12 release shell, PID 17188): sign-in start at
2026-10-04 19:05 with `patrick-main-native` running; lock + sleep at 19:20
(only `lock` was journaled); resume 2026-10-05 16:30:01, when the backend
still ran endpoint inventory (`discovery.jsonl`, `getDevicePeriod`
0x88890008). Opening the window at 19:39 showed "Unable to load your saved
session: Backend refresh failed", and every shell RPC was `transportError`.
The `\\.\pipe\audiorouter-control` pipe no longer existed, the process's live
threads totalled ~1.5 min CPU of 2 h 23 min (the 1 ms service thread had
exited), and `recovery_crashes` was empty, so the supervisor's error path
never ran: the control thread **panicked**. A panic bypassed the
`BackendSupervisor` loop, and release builds have no console, so the panic
location was lost.

Fix: `serve_control_connections_forever_observed` catches a control-plane
panic, wakes and joins its I/O thread (which otherwise keeps the only pipe
instance; max instances is 1), and returns an error. The shell supervisor
also wraps each attempt in `catch_unwind`, so a panic is counted and
restarted like any other failure (safe mode after repeated failures). A
panic hook writes a `shell.panic` record (thread name, file and line; no
message text) to `shell.jsonl`. Regressions:
`stopped_control_plane_releases_the_pipe_for_a_restarted_server` (fails with
the wake disabled) and `shell_panic_log_keeps_location_and_thread_but_no_message`.
Checks 2026-10-05: `cargo test -p audiorouter-transport --lib` 26/26,
`cargo test --manifest-path src-tauri/Cargo.toml` 61 passed, 1 ignored.

Released in 0.0.13 (2026-10-05): see the
[execution record](../archived/2026-10-05-release-0.0.13.md) and
[release evidence](evidence/2026-10-05-release-0.0.13.md).

Still open (next action): the panic site itself is unknown. After the next
occurrence, read the `shell.panic` record, fix the owning code and qualify a
sleep/resume with a native route running. A restarted backend leaves routes
stopped until Play.

### Release 0.0.12 — completed 2026-10-04

Published unsigned prerelease with the Stream Deck plugin; all code pushed to
main, all 16 public assets downloaded and verified. See the
[execution record](../archived/2026-10-04-release-0.0.12.md) and
[release evidence](evidence/2026-10-04-release-0.0.12.md).
Next: attended Stream Deck Record/session-switch qualification; broader M08
gates remain open. Intermittent pipe stress timeout and existing formatting
differences remain recorded limitations.

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

Exact next task: attended Stream Deck Record/session-switch qualification,
then remaining M08 gates. Do not disturb the user's audio or close their
apps without asking.

## Risks, evidence and rollback

Stats.cc's undocumented feed can change; unknown state releases Duck.
Independent clock drift and queue latency remain limits. Keep the previous
installer and compatible configuration/recording backups for rollback; no
migration or driver install. Published assets are immutable; repairs use a new
version.

### Website follow-up (2026-10-05, user request)

Objective: make the homepage signal animation follow real input-to-output paths,
show product media, and explain the Stream Deck and Siege workflows in detail.

- Reworked `site/assets/hero-route.svg` so signal dots trace the microphone and
desktop-audio paths through processing and to their destinations. Corrected
`site/assets/branching.svg` so the Mixer-to-monitor line reaches the monitor
port. Comets honor reduced-motion preferences.
- Homepage now displays a real Duck inspector screenshot captured from project
UI evidence and embeds the video requested by the user. Added dedicated
`site/stream-deck.html` and `site/siege.html` pages; the Stream Deck page
explains starting the integration, first-time local API connection, and all
seven key actions.
- Siege page uses the user-approved documented setup: anonymized Duck settings
from the screenshot (−20 dB in menu/preparation, action full level, between
rounds unchecked) plus the recorded EQ/compressor analysis. It does not claim
to represent a live active session; the running app could not return a session
snapshot through its local control pipe. Feed setup/security caveats are shown.
- Static checks: `node --check` passed for the site JavaScript and catalog
generator; homepage references and animated SVG features are present; local
server returned HTTP 200. Browser visual review was not available. No
automated tests requested or run.
- Rollback: revert website-only changes in `site/` and this note. Next: local
browser review at `http://localhost:3000`, then Pages/DNS setup when ready.

### GitHub Pages deployment handoff (2026-10-05)

- Workflow `.github/workflows/pages.yml` deploys `site/` on pushes to `main`;
  `site/CNAME` contains `audiorouter.org`. `site/README.md` records GitHub
  Pages and registrar setup.
- Not published: `gh auth status` reports the saved GitHub token is invalid, and
  `git push --dry-run origin main` could not connect to GitHub. The local
  `main` has eight commits ahead of `origin/main`, all separate driver work;
  pushing it would publish that work along with the site.
- Exact next step: configure Pages and DNS as described in `site/README.md`;
  restore GitHub access and publish the website changes on an isolated branch/PR
  before merge to `main`. Then verify the Pages deployment and enable HTTPS.
- **Superseded 2026-10-05 (release 0.0.14):** GitHub access works; Pages is
  live at https://audiorouter.org with HTTPS enforced, and every push to
  `main` deploys `site/` successfully. The site reads the newest release
  from the GitHub API; see the
  [0.0.14 evidence](evidence/2026-10-05-release-0.0.14.md).
