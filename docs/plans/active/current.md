# Active plan — v0.0.5 release preparation

Updated: 2026-10-02. v0.0.4 is published; v0.0.5 preparation authorized.

## Objective and scope

Current request: prepare v0.0.5 (existing three-part version format), commit and
push all tonight's changes. Include built-in Siege-round Duck, connected-Mixer
Duck bypass repair, 1,800 tool combinations, Duck controls/icon/widget, Advanced
EQ input spectrum and window Quit. Requirements DSP-19, GRAPH-08/14/15,
UI-05/11/12/13, AUTO-15, ARCH-04, SEC-01/10 and DIST-01–08.
Steps: review diff/evidence/secrets; bump metadata; run locked source checks;
commit clean inputs; build unsigned matching installer/artifacts; exact-exe
fresh-install after normal app closure; push source/tag and publish prerelease;
download and verify hashes. Current shell PID 61752 and RainbowSix PID 49376
must remain untouched. Native/attended and wider M08 limits stay disclosed.
Rollback retains v0.0.4 and compatible backups. No VST or user-data changes.

Prepare and publish the user-requested unsigned Windows x64 prerelease with
inspector selection fixes, connected Mixers, recording service isolation,
spatial audio preview, persistent/copyable API credentials, node/session IDs,
Stats.cc Siege integration and the supplied two-minute README video.
No VST implementation changes. Preserve user sessions, tokens, endpoint formats
and the stable experimental executable. Previous v0.0.4 assets are superseded.

Requirements: GRAPH-01/02/04/14/15, CAP-06/11/13/14, REC-01/08,
ARCH-04/09, UI-05/11/12, HTTP-03, API-09, AUTO-15, SEC-01/10 and DIST-01–08.
Contracts: graph/spec04, capture/spec05, recording/spec08, interface/spec09,
local HTTP/spec16; release gates in [M08](../../milestones/M08-release.md).

## Current slice: Siege round trigger for Duck (2026-10-02, user approved)

User finds Stats.cc + Node script + AudioRouter too much to run. Replace the
script for this use: Duck gains `trigger` = `level` (existing, `keyNodeId`) or
`siegeRound`. In `siegeRound` mode the backend reads Stats.cc's existing local
feed (`ws://127.0.0.1:17892`, client only, loopback only) and the Duck lowers
by its own Amount with its Attack/Release during the phases the user ticks
(`duckMenu`, `duckPrep`, `duckBetweenRounds`; all default on) and releases in
`action`. Phase changes are runtime-only (no saved revision). Fail-safe: no feed,
unknown or malformed state releases the Duck (full volume). Phase rules match
`examples/integrations/stats-cc-siege/core.mjs` `phaseState`. Requirements:
AUTO-15, GRAPH-08, PROC Duck (07-processing), UI-05/11, ARCH-04, SEC-01.
Realtime: the Duck reads one atomic; no locks, I/O or allocation in the audio
path. Network: one bounded client thread with backoff, started only while a
playing route has a `siegeRound` Duck; raw snapshots never logged (they contain
player data). New dependency: `tungstenite` (no TLS features). Stats.cc's feed is
an undocumented vendor interface (inspected 1.8.1) and binds all interfaces
without authentication; AudioRouter never opens or changes it beyond an
explicit, consented "Enable Stats.cc feed" file write (same rules as the
example's `setup-stats.mjs`).
Tasks: (1) engine/DSP external engage for Duck + tests; (2) phase rules + feed
client + status in control/diagnostics + tests (fake WebSocket server);
(3) enable-feed RPC with consent; (4) UI Trigger choice, phase checkboxes,
status, three themes; (5) spec/docs; (6) live check against the user's running
Stats.cc (read-only), then attended match. The Node example stays until the
built-in version is verified. Rollback: remove `trigger`/phase parameters and
the client; level-triggered Ducks are unchanged.
Implemented tasks 1, 2, 4, 5 (DSP-19); task 3 (feed-enable button) deferred:
the status line points to the existing setup script and the user's feed is
already enabled. Live read-only connect to the user's Stats.cc passed. See
[Duck round evidence](evidence/2026-10-02-duck-siege-round.md). Review build
`target/reviews/duck-siege-round-20261002/`. Next: user closes the running
review app normally, launches this build, sets the game Duck's Trigger to
Siege round, and checks menu/prep ducking, action release and release when
Stats.cc closes. Not committed or published.
Follow-ups (2026-10-02, user requests): Duck bypass between Mixers repaired;
1 800-combination tool suite; duck icon; window Quit (two-step confirm,
`quit_app` → `system.quit`). See
[combination/icon/quit evidence](evidence/2026-10-02-tool-combinations-duck-icon-quit.md).
Latest review build `target/reviews/duck-quit-combos-20261002/` supersedes the
two Duck review builds. Next: attended check of window Quit and the Siege
round Duck in a match.
Advanced EQ live spectrum (2026-10-02, user request): display-only analyzer of
the incoming sound behind the curve. See
[EQ spectrum evidence](evidence/2026-10-02-eq-live-spectrum.md).
Duck amount slider and canvas duck while ducking (user request); see
[Duck widget evidence](evidence/2026-10-02-duck-widget-slider.md). Latest
review build `target/reviews/duck-widget-20261002/` includes all changes above.

## Where things stand

v0.0.3 remains published. Inspector and exact saved revision-214 graph fixes are
implemented. Measured recorder drains stalled audio service for 15–17 ms;
factory file encoders now use a bounded dedicated thread. A corrected 30-second
47 Hz run passed clean reference, routed signal and parsed recording with zero
glitches. This predates the final combined spatial/permission changes and does
not qualify the final package.

Combined checks: UI 445 tests/typecheck/build, shell 44 (1 ignored), contracts
121 methods/34 nodes/18 processors/22 events, example 11, production Edge 24
cases pass. Spatial and inspector screenshots reviewed in all three themes.
Full locked workspace rerun passes; an initial authority regression used a
nonexistent RPC method, now corrected. Final control rerun passes 224 tests
(8 ignored), including the stopped-activation response. The dropped-tail pool
regression and release path-safety checks pass. Documentation: 97 files/514 links.

Final native attempts: stereo output rejects the user's now-7.1 endpoint;
an alternate playback pin reports DeviceInUse. The 8-channel loopback fixture
prepares successfully, but reference contains gameplay audio and cannot qualify
continuity. Exact contaminated temporary recording removed; no private audio
retained. No global formats changed and no user process terminated.

## Decisions and prerequisites

- User explicitly approves including spatial audio and asks to prepare while
  still playing. Leave AudioRouter PID 58360 and RainbowSix PID 49376 alone.
- Before final live checks, user must quit AudioRouter, Siege and integration
  and keep them closed through qualification. No second desktop shell.
- Surround preview uses measured MIT KEMAR HRTFs; Physical Input accepts 6/8
  channel 48 kHz float capture or render loopback and delivers stereo.
- Automatic restart requires session-control and device/consent authority.
  Recording or insufficient authority keeps the existing route, reporting
  restartRequired. Failed restart must report actual stopped runtime state.
- File writers own their bounded queues off the service thread. Lifecycle calls
  retain durable completion barriers; overflow reports a failed playable prefix.
- Unsigned prerelease authority is explicit; full M08 signing, installation,
  hardware, endurance and accessibility gates remain open and disclosed.

## Ordered tasks

1. Finish source/regression checks and inspect diff/secrets; record failures.
2. Commit clean combined inputs and build a new installer, matching shell/CLI/
   worker, public examples ZIP, SBOM, manifest and hashes. Preserve old artifacts.
3. Once package exists, obtain the closed-app window. Run clean 47 Hz spatial
   two-Mixer and baseline continuity with recording; qualify live path changes
   and recording-preserved restart refusal. Keep original user graph untouched.
4. Run fresh-install test on the exact packaged shell, new database/default pipe,
   no developer grants and no other instance. Verify embedded UI and artifacts.
5. Only after repair gates pass, replace the unpublished local tag, push approved
   source/tag, publish v0.0.4 prerelease and verify downloaded hashes.
6. Archive this execution plan after publication; retain open M08 gates.

## Validation matrix

| Gate | Evidence/status |
| --- | --- |
| Inspector and connected graph | [Repair evidence](evidence/2026-10-02-inspector-and-connected-mixers.md); portable topology and three-theme checks pass |
| Spatial preview | [Spatial evidence](evidence/2026-10-02-spatial-audio.md); DSP/format/UI checks pass, final clean native and attended listening pending |
| Earlier Siege/API changes | [Continuity](evidence/2026-10-02-siege-crackling-liveness.md), [credentials/IDs](evidence/2026-10-02-identity-and-persistent-token.md) |
| Final source/package | Current combined logs under ignored target; full Rust run/build/provenance pending |
| Exact exe first run | Required for newly packaged shell; earlier v0.0.3 pass does not qualify it |
| Full M08 | Clean-machine install/upgrade/uninstall, missing WebView2, Narrator/scaling, soak/hardware and signing remain open |

## Risks, rollback and next action

Independent clock drift and bounded queue latency remain known limits. Spatial
listening, real-user topology and automatic topology restart require attended
validation. Do not publish reproducible dropouts or claim complete M08 evidence.
Keep published v0.0.3 and compatible backups for rollback; older tokens need
integration reconfiguration. No schema migration or driver install in this slice.

Package prepared and verified from source `240721b639d19da25805b1606a20ff43b41b4ecb`:
`target/releases/v0.0.4-fixed/AudioRouter_0.0.4_x64-setup.exe`, matching shell,
CLI/worker, public examples ZIP, UI, SBOM, notices, hashes and manifest.
Examples ZIP inspected: no private configuration, token file or node_modules.
Exact packaged fresh-install check passes 1/1: disposable database, default pipe,
no developer access variable, consent denied then allowed, no audio device opened.
Log: `target/release-004-fixed-fresh-install.log`. AudioRouter was closed; Siege
PID 49376 remained. User is starting another match. Leave it alone; final quiet
native continuity and live path-change checks wait for normal game closure.
Decision: user explicitly requests "Deploy the package and then just stop" while
playing. Publish the verified package as an unsigned experimental prerelease,
disclosing final combined native/path-change/spatial listening checks pending.
This does not complete M08 or claim a fully qualified audio release. Stop after
publication and uploaded artifact verification; do not run live cable checks.
Published: [v0.0.4](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.4),
not draft, explicitly prerelease. Tag points to packaged source `240721b6`.
Downloaded all uploaded assets into `target/releases/v0.0.4-downloaded` and
artifact hash/provenance verification passes. No live check during gameplay.
Exact next task, only when the user resumes: remaining quiet native continuity,
live path-change and spatial listening qualification. Full M08 remains open.
Prior detailed decisions/results are retained in the
[superseded execution record](../archived/2026-10-02-release-repair-execution.md).
