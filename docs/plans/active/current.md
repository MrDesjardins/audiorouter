# Active plan — v0.0.4 release qualification

Updated: 2026-10-02. Publication held pending final native checks.

## Objective and scope

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

Exact next action: finish combined source checks and prepare new local artifacts
while the user plays; final native/fresh-install checks wait for normal closure.
Prior detailed decisions/results are retained in the
[superseded execution record](../archived/2026-10-02-release-repair-execution.md).
