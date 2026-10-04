# AudioRouter release notes

## 0.0.9 — 2026-10-04 (unsigned experimental preview)

- Input Device **Channels**: send a stereo input as mono to both ears. Use
  "left channel only (input 1)" or "right channel only (input 2)" for a
  single microphone on a stereo audio interface, which was otherwise heard in
  one ear; "mix both channels" averages them. It applies while playing and is
  saved with the node; the default stays stereo.
- Much lower memory use while audio plays: the window no longer re-renders
  the whole app 20 times a second for meter updates. On a measured session
  the WebView process settled at 440–470 MB instead of swinging between 600
  and 960 MB, and creates about a third less garbage.

Unsigned experimental prerelease; Windows may warn. No data migration or
device-format change; sessions without the new setting behave as before.
Keep 0.0.8 and a configuration backup for rollback. Mono input passes
sample-level tests but has not been tried on a stereo interface by ear.
Real-match Duck, native Quit, combined continuity and full M08
install/hardware/accessibility/signing qualification remain open.

## 0.0.8 — 2026-10-03 (unsigned experimental preview)

- The window title and header show the version. Once a day the app checks
  GitHub's public release list and, when a newer version exists, shows
  "<version> available" next to it; click it to open the release page. Turn it
  off in Setup → New versions. Nothing about you or your audio is sent.
- Devices at 44.1, 88.2, 96 or 192 kHz work in multi-path sessions (inputs,
  outputs, endpoint loopback and Surround to headphones): Windows resamples
  them to AudioRouter's 48 kHz. Previously such a device was refused, for
  example a 96 kHz 7.1 SteelSeries Sonar channel or a DAC set to 44.1 kHz.
- The "device in use" message also names a VB-Cable's "In 16ch" twin when an
  application playing to it is what blocks the cable.
- Surround to **speakers**: a 5.1/7.1 input can be rendered for two speakers in
  front of you, with crosstalk cancellation so each ear mostly hears its own
  channel (sit centred). New **Room** slider for both surround modes adds a
  small room; higher sounds further away. Change either, then Stop and Play.
- API tab: **Build a request** generates the exact REST request (JSON, curl or
  PowerShell) for a tool setting, including Mixer input volumes by source name
  and a Duck's trigger, and can send it once to try it.
- Test and tooling: the intermittent autosave test was a test defect (it
  clicked a node's inline slider) and is fixed.

Unsigned experimental prerelease; Windows may warn. No data migration or
device-format change; keep 0.0.7 and a configuration backup for rollback.
The new-version check reads api.github.com once a day (Setup → New versions
turns it off). Speaker mode, Room and 44.1/96 kHz conversion pass objective
and live-device tests but have not been judged by ear yet. Real-match Duck,
native Quit, combined continuity and full M08 install/hardware/accessibility/
signing qualification remain open.

## 0.0.7 — 2026-10-03 (unsigned experimental preview)

- "Device in use" now names the exact device another application holds
  (for example "CABLE Input … is in use by another application") and says
  how to stop apps from locking it (Windows Sound → Properties → Advanced →
  untick exclusive control). Previously the message named only the node type.
- The full browser test suite passes again. Seven tests that failed since
  0.0.5 targeted the wrong element (several circles per EQ point, the Session
  tab's Undo beside the toolbar Undo, the Bypass badge beside its checkbox)
  or described the pre-0f32e9a7 live-flag behaviour; the live-flag test now
  also proves an unsaved added tool is not saved with a live Bypass.
- Browser tests no longer overwrite committed evidence screenshots.
- The draft-release script no longer stops under Windows PowerShell 5.1 when
  the release does not exist yet.

Unsigned experimental prerelease; Windows may warn. No data migration or
device-format change. One autosave timing test failed once and passed 20/20
on rerun; under investigation. Real-match Duck, native Quit, combined continuity,
spatial listening and full M08 install/hardware/accessibility/signing
qualification remain open.

## 0.0.6 — 2026-10-03 (unsigned experimental preview)

- Fix "No audio started. Choose the device for …" after choosing the device:
  a device node without a saved device showed a device remembered from
  elsewhere, so choosing it did nothing. The picker now starts empty and
  says the node has no device yet. (Workaround on older versions: pick another
  device, then the correct one, Save, Play.)
- Fix dragging tools onto the canvas in the desktop app: the drop preview
  shows and dropping adds the tool.
- Compressor, Gate and Duck threshold suggestions no longer make the panel
  jump: the suggestion keeps a fixed place and its last measurement.
- The compressor suggestion stays close to your voice peaks instead of
  dropping toward room noise at gentle ratios, and suggests turning the
  microphone up when your voice is quiet.
- The Gate suggestion keeps its closing point clear of room noise (it accounts
  for Hysteresis) and says where the gate closes.

Unsigned experimental prerelease; Windows may warn. No data migration or
device-format change; keep 0.0.5 and a configuration backup for rollback.
Seven browser-harness tests (Undo/EQ history, enable/bypass label, two live
draft checks) fail identically on 0.0.5 and are test defects under review,
not new regressions (fixed in 0.0.7). Real-match Duck, native Quit, combined continuity,
spatial listening and full M08 install/hardware/accessibility/signing
qualification remain open.

## 0.0.5 — 2026-10-02 (unsigned experimental preview)

- Duck can follow Siege rounds directly through Stats.cc; choose menu,
  preparation and between-round phases to quiet. Action or unavailable state
  restores full volume. Existing feed setup is required; no Node service or
  REST token is needed for this built-in mode.
- Fix bypassing a Duck between connected Mixers. The regression suite exercises
  1,800 tool/graph/flag combinations and live path replacement.
- Advanced EQ displays the incoming sound spectrum behind its response curve.
- Duck amount uses a slider; the canvas shows a duck while ducking, respecting
  reduced-motion preferences.
- The desktop top bar has a confirmed Quit action using backend finalization.

Unsigned prerelease: real-match Duck behavior, native Quit/process exit, final
combined continuity/spatial listening and wider M08 install/hardware/accessibility
qualification remain open. Unknown feed state releases ducking; vendor protocol
changes can break detection. Stats.cc setup remains explicit and its optional
1.8.1 feed binds all interfaces without authentication; keep inbound access blocked.
Close the old integration and AudioRouter before upgrading. Keep v0.0.4 and a
compatible configuration/recording backup for rollback; credentials stay local.

## 0.0.4 — 2026-10-02 (unsigned preview)

- Tool inspectors switch correctly between EQ, dynamics and Meter controls.
- Connected Mixers support the separate microphone, game/Discord and combined
  headphone branches without processing the same source twice.
- File recording runs on a dedicated bounded worker so encoder/disk stalls do
  not block routine audio service. Recording overflow preserves a playable
  prefix and reports failure instead of silently completing a truncated take.
- Surround to headphones is an experimental Physical Input option for 5.1/7.1
  48 kHz float endpoints, including playback loopback, using measured KEMAR HRTFs.
  End-to-end continuity and attended spatial listening qualification are pending.
- Refused live multi-input graph changes can restart the route when device and
  session permissions allow. During recording, or without those permissions,
  the existing route continues and the API reports `restartRequired`.
- README includes the two-minute overview video. The examples archive contains
  public integration sources only; configure credentials locally.

Publication scope: user explicitly requested this experimental prerelease before
the final combined quiet-audio checks. Source suites, artifact hashes and the
exact packaged fresh-install check pass. The recorder repair passed a bounded
clean-reference run before the final combined spatial changes; final combined
native continuity, live topology restart and attended spatial listening remain
pending. This is not completed M08 qualification.
- Fixes sustained breakup when a Mixer combines physical game audio and
  application capture: application capture now uses the graph's 48 kHz rate.
- Routine application liveness checks avoid a full Windows process inventory
  on the audio service thread. Restart matching retains verified identity rules.
- Two 30-second physical-plus-application Mixer checks and a plain physical
  route check pass with clean reference tones and zero routed glitches. This
  is bounded workstation evidence; hardware soak and wider compatibility
  qualification remain open.
- API credentials persist encrypted for the current Windows user. Stop/start
  reuses the token; Generate new token explicitly replaces it. The API tab
  shows the saved token with Copy and Hide even while the listener is stopped.
  The listener remains stopped when AudioRouter opens.
- Node Properties and the Session tab show subtle selectable IDs with Copy,
  making targets easier to find for external applications.
- The independent Stats.cc Siege example uses the REST API to set Siege's
  Mixer input to 30% during menu/selection/preparation/results and 100% during
  playable action. Discord stays 100%; failure/shutdown restores 100/100.
  Node.js 22 or newer and separately installed Stats.cc are required. See the
  [example instructions](../../examples/integrations/stats-cc-siege/README.md).

Upgrade: close the integration with `quit`, then close AudioRouter normally
before running setup. Sessions and recordings are retained; no driver or global
Windows audio setting changes. The first API use establishes a persisted token;
integrations using an older temporary token may need its replacement. API tokens
are local Windows-user credentials and are not transferable with exported sessions.
Keep v0.0.3 plus a compatible configuration/recording backup for rollback; that
older app generates temporary tokens again, requiring integration reconfiguration.

The app/installer are unsigned. Clean-machine, WebView2 absent, standard-user
upgrade/uninstall, accessibility, hardware endurance and full M08 qualification
remain open. Stats.cc's optional state feed is unauthenticated and binds all
interfaces in version 1.8.1: run the explicit setup command and keep its port
blocked for inbound network access. Its application code is not patched.

## 0.0.3 — 2026-10-02 (Joe's test release, unsigned)

This prerelease targets an empty device list reported with Windows error
`0xE000020B` (missing device instance). One disappeared endpoint previously
aborted discovery of the entire list. Discovery now skips only disappeared
endpoints and preserves permission/service/format errors. The old log does not
identify the exact endpoint; validation on Joe's PC remains the purpose of this build.

- Clearer diagnostic files include failing operation, category, hexadecimal
  Windows error, build/version and device counts. **Logs → Open logs folder**
  and **Copy folder path** make support files easy to find.
- A second desktop launch explains that another instance is running before
  opening storage or connecting to its backend. Cancel leaves it untouched;
  **Force close old instance and continue** explicitly warns about interrupted
  audio, unsaved changes and unfinished recordings.
- Network Properties shows audio in/out direction and paired address/port.
- Canvas List view is removed; keyboard graph controls remain under Advanced.
  Arrange and precise tool placement pass the production-browser regressions.
- README installation instructions are prominent and video links have placeholders.
- REST examples document active-session tool selection, enabled/bypass flags,
  and dynamic Volume/Mixer/Duck settings. No VST implementation changed.

Windows 11 x64, per-user installer, existing audio devices; no audio driver is
installed or Windows default device changed. Close AudioRouter normally before
upgrading and back up configuration/recordings. Data remains in
`%LOCALAPPDATA%\AudioRouter`; recordings remain in the chosen folder. The
previous [v0.0.2 prerelease](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.2)
remains available. Uninstall/data-retention and clean-machine upgrade behavior
are not fully qualified, so do not delete user data during rollback.

The app and installer are **unsigned**; Windows may show publisher/trust prompts.
This is a test prerelease, not completed M08 qualification. Native force-close
UX, Explorer launching, clean-machine/WebView2/upgrade/uninstall, accessibility,
hardware soak and aggregate audit retention remain open. Earlier virtual-cable
dropouts and clock-drift limitations still apply. A failing test is not presumed
to be a harmless locator issue without evidence.

If input discovery still fails, use Logs to send shell/backend/discovery JSONL
files (and previous files if present), reproduction steps and approximate time.

## 0.0.2 — 2026-10-01 (test release, unsigned)

Fixes Play on a fresh install. In 0.0.1, pressing Play on a newly installed
computer showed **Permission denied … device administration**, and only a
developer environment variable worked around it.

- **The first Play asks once:** "Allow AudioRouter to use your audio
  devices?". Choose **Allow and play**; it is remembered on that computer.
  **Setup → Audio device access** withdraws it.
- **Who can give it:** only the AudioRouter window. The localhost API refuses
  to grant it (403). CLI and MCP clients never receive it. After you allow
  it, StreamDeck/REST `sessions.play` works too.
- **The first launch of a fresh install** now gets the same permissions as
  every later launch; for example, it can choose a recording folder.
- **The developer variable:** `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` is no
  longer needed. If you set it on your second computer for 0.0.1, you may
  remove it:
  `[Environment]::SetEnvironmentVariable("AUDIOROUTER_ALLOW_DEVICE_ADMIN", $null, "User")`.

Verified by launching the built app as a fresh install (new database, no
developer variables): Play is refused before the question and authorized
after **Allow**. The same check fails on 0.0.1. The 0.0.1 known issues below
still apply.

## 0.0.1 — 2026-10-01 (first test release, unsigned)

A test build for installing on a second computer. It is **not signed**:
Windows SmartScreen will warn ("Windows protected your PC"); choose *More
info → Run anyway* only for an installer you built or received from the
maintainer. The installer is per-user (no administrator rights) and does not
install audio drivers. Use existing devices or VB-Cable/Voicemeeter.

Highlights since the development snapshots:
- **Network Send / Network Receive:**
  - stream audio between computers on a local network;
  - a sending PC needs no local output;
  - address/port/buffer edits apply while playing;
  - the receiver names the address audio really comes from and offers a
    one-click fix
  ([qualification](../plans/active/evidence/2026-10-01-network-qualification.md)).
- **One-click recording:**
  - Record/Stop on the Recorder node;
  - optional start with Play;
  - optional new file every N minutes;
  - a recording folder chosen in the app.

  A take that loses audio still leaves a playable file and says why.
- **Live tool visuals and editors** (Compressor, Gate, Duck, EQs, Meter, …),
  an animated signal flow, the **Duck** tool and **Input Switch**.
- **Canvas:**
  - **Arrange** lays the graph out, inputs to outputs;
  - drag tools from the panel with a live preview;
  - visual groups can be locked.
- **Automation:**
  - task-shaped REST endpoints for StreamDeck and scripts (`nodes.toggle`,
    `sessions.togglePlay`, …);
  - MCP tools for configuring AudioRouter through an AI assistant.

Fixed in this release:
- One-click recordings on multi-path sessions produced unplayable files.
- A session whose only outputs are Network Send or Recorder nodes played
  nothing.
- Live network edits reported "applied" but kept the old address.
- Stopping a recorder could fail with `FrameWentBackwards`.

Known issues:
- **Output dropouts:** occasional ~10–25 ms dropouts on virtual-cable outputs
  (about one per 30 s on the development PC). The previous build shows the
  same, so this is not new. It is under investigation.
- **Two-computer use is untested:** network streaming is qualified on one PC
  (loopback, LAN address, two backends), not yet between two PCs or over
  Wi-Fi.
- **Firewall:** the receiving PC's Windows Firewall must allow AudioRouter on
  **Private** networks (see the quickstart).
- **Browser tests:** a few known test failures (Undo/Bypass locator
  ambiguity, EQ point undo, live-controls message) are open in the UI test
  suite. They are test-locator issues, not product failures, and remain to be
  fixed.
- **Release gates:** clean-machine, upgrade/uninstall and missing-WebView2
  qualification (M08) have not been run yet.

Data lives in `%LOCALAPPDATA%\AudioRouter` (sessions, settings, logs);
recordings go to the folder you choose. Uninstall behaviour is not qualified
yet, so back up that folder if you need it.

## Canvas startup recovery — 2026-09-30

The canvas now performs bounded recovery when initial connector geometry is
missing or invalid, allowing saved lines and drag connections to initialize
without a refresh. Optimized cold-start regressions check existing lines and
real dragging in all themes. Saved sessions/layouts need no migration. The
reported native timing still needs first-launch confirmation on the rebuilt
shell; see [evidence](../plans/active/evidence/2026-09-30-canvas-cold-start.md).

## Tool naming and connected-pair qualification — 2026-09-30

"FIR Filter Hz" is now displayed as **Spectral Gate** to distinguish learned
frequency gating from **FIR Filter** impulse-response convolution. New nodes
use the clearer default name; existing saved node names are preserved. API
identifiers and parameters are unchanged. Deterministic qualification now
checks ordered pairs of built-in tools and independent combined-response,
delay and nonlinear processing-order expectations.

## Dehum correction — 2026-09-30

Dehum now uses finite-depth notches with constant harmonic bandwidth, preserving
wanted audio between hum harmonics at high Amount. Existing saved settings stay
valid; no migration is needed. The original 1003 Hz preservation regression and
additional stereo/frequency sweeps pass. The optimized desktop executable was
rebuilt for manual testing on 2026-09-30. See the
[qualification evidence](../plans/active/evidence/2026-09-30-dehum-preservation.md).

This document describes the current development snapshot (2026-09-17). It is not a signed
release and must not be presented as an installable Windows audio product.

## Current scope boundary — 2026-09-17

This development track relies on an existing VB-Cable pair, Voicemeeter, and
physical WASAPI endpoints for user-mode routing and tool integration. The
AudioRouter-owned kernel driver is deliberately deferred because production
signing and trusted installation requirements are not currently available.
Driver, PortCls, signed-package, and clean-machine gates remain open and are
not implied by the VB-Cable evidence below.

## Scope and platform

- Target: Windows 11 x64.
- Portable control, storage, DSP, recording, CLI, UI, MCP, and plugin-worker
  foundations are implemented and covered by automated tests. The UI has 257
  passing tests and includes a visual graph editor, backend-bound built-in
  processor/preset editing, explicit endpoint binding, and route provenance.
- The repository-local Steinberg VST3 SDK is pinned and verified at
  `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`.
- Native builds use Visual Studio Community 2026/MSVC 14.51.36231 and Windows
  SDK 10.0.28000.0; the installed WDK is 10.1.28000.2526.

## Current requalification refresh — 2026-09-17

The current tree requalification supersedes stale counts in older snapshot
paragraphs: M05 passes typecheck, 19 UI test files/257 tests, and a disposable
production build; M07 passes 36 CLI, 3 MCP, 177 control tests with 3 guarded
live tests ignored, 70 plugin-host, and 13 worker-process tests; and the full
locked workspace passes its package suites and doc-tests. The explicit
VB-Cable/Focusrite-to-VB-Cable/DELL multi-route also passed with 47,040
captured and 47,232 rendered frames. These are development evidence only. M08 artifact
preparation is currently blocked by the required clean Git worktree, and no
driver, signing, accessibility, physical-latency, sandbox, or publication
claim is implied.
The disposable unsigned NSIS smoke was rerun in the elevated build context,
produced a 4,888,262-byte x64 bundle, verified manifest integrity, and
removed the bundle afterward. This strengthens packaging evidence only; the
clean-tree artifact flow still refuses the current dirty checkout.

The guarded application-capture lifecycle also passed for the observed
Voicemeeter and Zoom process identities in both include and exclude modes,
including same-process worker restart and unchanged media-device state. The
refreshed Windows shell suite passed all 29 tests for notification mapping,
bounded delivery, safe-mode recovery, startup ownership, and tray authority.
These results do not promote real OS power-transition delivery, native reopen,
or universal process-loopback compatibility.
The latest exact-endpoint native run also toggled privacy mute during active
processing and verified the control state transitions, while the current Zoom
process passed exclude-mode capture with worker restart. No defaults, volume,
mute, privacy, or persistent audio configuration was changed by these checks.

## Verified in this qualification snapshot (2026-09-16)

The guarded integrated acceptance chain passed at commit `1e9c60ce`, including
the x64/ARM64 source gates, 234 UI tests, unsigned artifact/NSIS smoke,
traceability, and documentation validation. The generated installer was
4,760,908 bytes and was removed after the disposable smoke test.

- The locked Rust workspace passes its current package tests and doc-tests,
  formatting, and strict Clippy; the guarded native tests remain explicitly
  opt-in.
- M07 headless acceptance passes 36 CLI tests, 3 MCP interoperability tests,
  173 control tests (2 guarded live tests intentionally ignored), 70
  plugin-host tests, 13 worker-process tests, and 26 shell tests. The shell
  supervisor persists crash markers and keeps a stopped control plane
  available in safe mode.
- M08 disposable artifact preparation creates and verifies unsigned x64 CLI,
  native-shell, and plugin-worker artifacts, SBOM metadata, notices, checksums,
  and a manifest, validates bounded PE headers for x64 executables, then removes
  the temporary output.
- A transient Tauri 2.11.4 NSIS bundler smoke also succeeds with `--no-sign`;
  the generated installer is removed and is not a production or installability
  qualification result.
- VST3 SDK acceptance passes 51 SDK self-tests, 1,598 official validator tests
  with 0 failures, and the offline native mda fixture loader.
- A Windows-only, gated native x64 VST2 adapter is verified with repository-owned
  `VSTPluginMain` and legacy `main` fixtures, including chunk-state restoration,
  invalid-output rejection, crash containment, and hang reaping. This does not
  grant redistribution rights or make the VST2 extension release-qualified.
- Windows endpoint and application discovery failures preserve stable audio
  categories, unsigned HRESULTs, retryability, and remediation guidance;
  `AUDCLNT_E_DEVICE_IN_USE` remains distinct from `E_INVALIDARG`.
- The native WASAPI probes qualify shared capture across 13 endpoints,
  process-loopback include/exclude and controlled attribution, silent render
  lifecycle, and endpoint timing baselines. The guarded production Rust adapter
  smoke also passes bounded capture plus zero-valued `submit_bytes` render
  submission while preserving the media-device snapshot.
- The guarded control-owned native VB-Cable route qualifies exact capture and
  render endpoint binding, processor-bearing graph activation, bounded pump
  delivery, and clean start/stop. The latest 500 ms run observed 23,520
  captured frames, 23,424 rendered frames, and 183 processed quanta. It did
  not change defaults, volume, mute, privacy, drivers, or persistent audio
  settings.
- Shell-owned backend recovery persists crash markers and keeps a stopped
  control plane available after the safe-mode threshold, allowing an
  authorized operator to inspect and clear the latch without reopening native
  endpoints.
- The attended Tauri shell transport acceptance reaches the WebView's native
  RPC command and authenticated backend through a disposable pipe/database;
  manual visual accessibility and scaling review remains separate.

## Known limitations

- Rust capture retries the exact observed event-callback `E_INVALIDARG` with a
  fresh native-compatible polling client; busy-device and permission failures
  remain distinct and fail closed. The guarded live adapter smoke qualifies
  bounded adapter capture/render lifecycle, but is not evidence of complete
  AudioRouter graph routing.
- Production callback scheduling, physical acoustic latency, clock drift, and
  hardware/endurance qualification are incomplete. The current VB-Cable pump
  is a guarded transitional control-plane delivery path, not callback timing
  evidence.
- The managed virtual-audio driver is not included, installed, signed, or
  registered. Virtual-device lifecycle remains an honest unavailable
  capability.
- There is no production installer, signed package, upgrade/rollback proof,
  clean-machine qualification, or Secure Boot/Memory Integrity driver result.
- Plugin discovery and worker protocol protections are implemented, but full
  filesystem/network OS sandboxing, arbitrary plugin execution, and a broad
  third-party compatibility matrix remain open. The VST2 editor, rights, and
  release-qualification gates also remain open.
- Sign-in startup registration is verified at the native helper boundary, but
  an attended rollback run, manual accessibility, scaling, and first-time-user
  qualification remain open.

The native startup helper's HKCU Run boundary was requalified on 2026-09-14
with an opt-in enable/disable round trip. The test temporarily created only
the current user's `AudioRouter` value, verified ownership, removed it, and
confirmed the value was absent afterward. The native helper is qualified; an
attended shell/accessibility and clean-machine startup review is still
required before release claims.

The same guarded test was reattempted on 2026-09-17 in the current host and
was denied by the host's HKCU registry policy (`WIN32_ERROR(5)`) before any
write. A read-only query confirmed that the `AudioRouter` value was absent;
this is an environment validation blocker, not evidence of a successful current
round trip.

## Safety and recovery

The acceptance commands are configuration-safe: they do not change default
devices, volume, mute, privacy settings, drivers, or endpoint state. See the
[development quickstart](quickstart.md), [headless runbook](headless-runbook.md),
and [release qualification checklist](release-qualification.md) for commands,
backup/restore expectations, and recovery boundaries.

Do not install a third-party virtual cable and describe it as an AudioRouter
driver. Do not treat an unsigned manifest as publication-ready. Production
signing, driver installation, and any audio-stream experiment require separate
authorization and the appropriate isolated test environment.
