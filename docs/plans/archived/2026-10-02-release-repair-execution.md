# Active plan â€” public release preparation

Updated: 2026-10-02. Status: v0.0.3 Joe prerelease published; M08 qualification open.

## Objective and scope

Spatial audio slice (2026-10-02, user approved; supersedes the "no
implementation" status of [spatial-audio](../future/spatial-audio.md)):
"Surround to headphones" for game audio. Decisions (user): capture a real
7.1/5.1 stream and render it binaurally with a measured HRTF; MIT Media Lab
KEMAR diffuse-field set (Gardner & Martin 1994; free use, citation required).
Contract: a Physical Input gains `spatialMode` (`off` | `headphones`). In
`headphones` mode its capture endpoint must be 6 or 8 channels, 48 kHz float32;
the capture adapter renders each speaker channel at its standard position
(C 0Â°, FL/FR Â±30Â°, SL/SR Â±90Â°, BL/BR Â±150Â°, LFE to both ears) and delivers
stereo, so the node port and the rest of the graph stay stereo
(MAX_CHANNELS unchanged). Static positions, no head tracking, no room model,
no speaker mode in this slice. Requirements: CAP-01/06, GRAPH-04, PROC
(07-processing), UI-05/11, ARCH-04. Realtime: preallocated time-domain
convolution, no allocation/logging/locks in the packet path; target <10% of a
core in release. Failure: an endpoint whose channel count/format does not match
fails Prepare with a specific message; no silent stereo fallback. Bypass/disabled
behaviour follows existing Physical Input rules.
Tasks: (1) convert KEMAR HRIRs to a 48 kHz table with a checked-in generator and
NOTICE citation; (2) `BinauralRenderer` in `audiorouter-dsp` with unit tests
(left/right/front/back energy, LFE, silence, allocation-free process);
(3) windows-audio capture wrapper turning 6/8-channel packets into stereo;
(4) control binding/validation and prepare plumbing; (5) domain parameter
schema, UI inspector toggle and setup guidance, three-theme screenshots;
(6) spec/docs; (7) Windows qualification with a generated 8-channel tone on a
VB-Cable 16ch endpoint, then the user's attended Siege listening test.
User setup (not automated; changes Windows devices): set Siege's output to a
7.1-configured endpoint whose capture side is 8 channels. Rollback: remove the
parameter and capture wrapper; sessions without `spatialMode` are unchanged.
Decision (2026-10-02, evidence): VB-Cable's recording side cannot be
multichannel in shared mode, so surround inputs may name a 5.1/7.1 render
endpoint captured by loopback. Implemented tasks 1â€“6 plus an automatic route
restart for refused live changes (user report: Mixer save silenced audio until
Stop/Play). See [spatial evidence](../active/evidence/2026-10-02-spatial-audio.md).
Review build `target/reviews/spatial-audio-20261002/`. Next: user closes the
running app, sets CABLE-B Input to 7.1, launches the review build, enables
Surround to headphones on Siege game with the CABLE-B Input loopback, and runs
the listening check; also qualify a path-changing save while playing.

User explicitly requests an experimental executable to test while dropout repair
continues (2026-10-02), and prioritizes a release soon. Hand off the already-built
optimized inspector/connected-Mixer review with its matching worker, identity,
hashes and a short known-issue note. This authorizes local user testing despite
the failed native gate; it does not waive release qualification or resume
publication. Preserve this test copy while subsequent repairs/builds proceed.
Continue portable investigation of service/request timing and render queues
while the user runs it; live cable checks require a later normal app closure.

Publication PAUSED at user's explicit request (2026-10-02). Local candidate
tag v0.0.4 and unsigned artifacts exist, but no tag/source push, GitHub draft
or publication occurred. Workspace Rust suite completed successfully; all
other source checks above passed. Do not publish until the following issues
are resolved and the final source/artifacts requalified.

Current repair objective: (1) isolate tool-specific property controls on node
selection (UI-05/11/12); user reports Attack/Release and signal displays on all
tools: Advanced EQ, Voice Compressor, Voice Gate and Meter. Inspect selection, discovery and
editor state, reproduce with browser transitions, add focused regression and
visually verify dark/light/high-contrast before building a new review.
(2) Support Patrick Main Session (Native) revision 214's connected Mixers
(GRAPH-01/02/04/14/15, CAP-13): read-only history shows Mic + Siege mixer-2
fed by microphone meter-1 and Siege + Discord mixer-1. The original compiler
rejected components with more than one Mixer; Stop/Play alone could not solve it. Preserve
the saved graph, microphone-only Cable-A branch, game/call Outplayed branch and
explicit combined headphone branch. Read compiler/runtime contracts, record a
bounded execution design before implementation, add exact-topology portable
and Windows continuity regressions. No VST change or microphone substitution.
Rollback retains prior binaries and reverts owning UI/compiler/runtime slice;
never rewrite the user's session without explicit instruction. Next action:
reproduce inspector complaint and design connected-Mixer execution.

Implemented: identity/footer sibling key collision removed; editor hooks made
unconditional; connected Mixers compile into a bounded cache without duplicate
capture/processing. UI typecheck/build, 443 UI tests and 21 production Edge cases
pass, with EQ/Meter visuals inspected in all three themes. Engine/control suites
pass (155 + 220 library cases, 33 deterministic + 7 route cases); exact saved
revision 214 compile-only check passes with 3 sources/3 outputs/1 component.
First 30-second two-Mixer Windows check has clean reference/routed/Recorder
signals (zero glitches/silent runs), but concurrent suite load caused service
late gaps. User closed reopened old review PID 20904 normally; final checks
failed: debug repeats had 2 routed glitches each with clean references/Recorder;
single-Mixer baseline also failed (42 routed glitches). First optimized run was
inconclusive because reference itself dropped a packet; optimized repeat failed
with clean reference/Recorder and a 76-sample routed silence. Native continuity
is NOT qualified. No user process was terminated. Unpublished
review: `target/reviews/connected-mixers-20261002/audiorouter-shell.exe`.
See [repair evidence](../active/evidence/2026-10-02-inspector-and-connected-mixers.md).
Next: correlate render ring/device occupancy and control-service stalls, isolate
output delivery/recovery from recorder/control work, repair the owning layer and
repeat clean-reference continuity. Do not publish or claim audio qualification.
Diagnostic step (user closed apps again): add an internal, optional bounded
channel of numeric service-pass timings (wait, dispatch, audio service). The
production entry point remains unchanged and disabled by default; the live
fixture drains the channel off-thread and correlates slow passes with output
underruns and recorded discontinuities. No logging, allocation or blocking send
in the processing callback; no public RPC, session, token or device changes.
Qualify the probe with focused tests, then run optimized 47 Hz two-Mixer
continuity. Use measured phases to choose the owning repair; do not increase
latency or discard failed samples as a substitute for diagnosis. Rollback removes
the optional observer and fixture reporting. Preserve the delivered test exe.
Measured cause of service stalls: recorder drains account for 14.9â€“16.8 ms
of 15.1â€“17.1 ms service passes; control requests stayed below 3.5 ms. The
clean-reference probe failed with 32 routed glitches while the Recorder file
remained continuous. Repair REC-01/08 and ARCH-04/09 by assigning factory-made
file encoders their own bounded worker thread. Keep the existing pooled tap,
frame reader and lifecycle barriers; routine service drains only collect a
bounded progress/error notification and never wait for disk/encoding. Preserve
pause, split, finalization, failure-prefix and library metadata semantics. Add a
deliberately blocked writer regression and rerun recording/control plus native
continuity. No buffering/latency increase or file format migration. Rollback
removes the threaded factory adapter, retaining old review and user data.
User now requests publication after the repairs qualify and supplied a two-minute
YouTube overview; README Videos links it. Release hold remains until qualification.
Combined release scope confirmed by user: include spatial audio. Review found
automatic route restart must retain the caller's DeviceAdministration/consent
and SessionControl authority; a GraphWrite-only caller must leave the existing
route running and receive restartRequired. Gate internal re-preparation using
the current authorized-dispatch scope, and regress consent/grant decisions.
Final checks cover the combined source, including spatial format rejection;
real 7.1 continuity/listening remains a separately declared preview limitation.
The user's CABLE-B playback endpoint is now 7.1/48 kHz. Preserve that setting.
Extend the opt-in quiet fixture to generate a front-left 8-channel tone, read
its exact render-loopback reference, prepare spatialMode=headphones and route
stereo through the existing two Mixers to the first cable. This qualifies
end-to-end spatial preparation/processing without private audio or format changes.
For ordinary stereo checks use the already-present stereo CABLE-B In 16ch
playback ID. Record each exact endpoint and preserve failed format attempts.
User review of updated inspectors and real topology follows that repair;
keep release publication paused and supersede old 0.0.4 assets before release.

Inspector reproduction confirmed in optimized Edge in all three themes: selecting
EQ after Compressor left a Compressor live view mounted although the heading and
ID changed. NodeIdentity and the live editor were siblings with identical React
keys. Remove the redundant outer identity key (its inner footer already resets
on ID changes), keep parameter-editor hooks unconditional, and regress repeated
Compressor/EQ/Meter/Gate selections during telemetry refresh. The first key-only
repair passes 3/3; final combined checks and visual review follow.

Connected-Mixer design: retain existing single-convergence compilation; components
with several authored convergence nodes use a bounded topological cache. Each
source is captured once, each processor runs once per quantum, and each Mixer
sums explicitly mapped cached parent blocks with its own input volumes. Reuse
the current nonblocking DSP buffer gate and output rings/taps. Allocate caches
only at compile time, bound source/input/output counts, retain node telemetry and
generator transport identity, and clear readiness each quantum so an unavailable
cache cannot replay old audio. Disabled/bypassed Mixers and sinks are silent;
processors retain defined dry bypass. Do not flatten matrices across processors
or Mixers, or rewire the user's data. Verify three sources/two Mixers/three sinks,
branch isolation, volume edits, flags, busy buffers and stateful execution count;
then qualify Windows tone continuity before handing off audio changes. Source or
output inventory changes still require Stop/Play; parameter-only changes retain
prepared streams. Rollback reverts this compiler/cache extension and UI fix.

Release 0.0.4 (user requested 2026-10-02): package the current audio continuity
repairs, persistent/revealable API token, node/session ID copy controls and
Stats.cc external example (30% non-action/100% action, Discord 100%). M08
artifact subset; CAP-06/11, HTTP-03, UI-05/11/12, API-09, AUTO-15, SEC-01/10.
Keep the established unsigned prerelease status and disclose open M08 gates.
Tasks: inspect scoped diff/secrets; bump app/workspace metadata; run locked
Rust/shell/UI/contracts/example and production-browser checks; commit/tag clean
inputs; build matching executables/NSIS/SBOM/manifest/checksums; run fresh-install
on exact packaged exe after user closes PID 41912; create and publish GitHub
prerelease, then download/hash verify. Existing user data/settings are retained;
DPAPI token remains local and is excluded from packages. Include portable
integration sources with no private local config or node_modules. Rollback:
retain v0.0.3 plus compatible config/recordings; older builds regenerate API
tokens, so integrations need updating after rollback. No driver/VST changes.

Stats.cc volume follow-up (2026-10-02): user reports integration working and
requests 30% outside playable action, 100% during action. AUTO-15, HTTP-04/06,
GRAPH-08. Update example and ignored local configuration, phase fixtures and
README; preserve Discord/fail-safe/shutdown 100%. Run the required focused
example regressions. Rollback restores quietPercent to 50. Restart the script
to load configuration; do not stop the user's running process. Keep application
capture at internal 48 kHz; a user rate selector would require conversion and
separate qualification and is not implemented by this follow-up.
Implemented: example and user's ignored local configuration use quietPercent
30, actionPercent/discordPercent/restorePercent 100. README and phase/service
fixtures updated. `npm.cmd test` on Windows: 11/11 passed. User reports the
integration was working in Siege; new 30% policy takes effect after normal
script restart. No app rebuild is required for this configuration change.

Siege crackling repair (2026-10-02): CAP-06/11, NFR-02, GRAPH-14, QUAL-05.
Direct Siege speaker output is clean; Cable-B routed output crackles even with
EQ bypass and minimized UI. Live counters confirmed delivery gaps. Two defects
are repaired: full process inventory stalled the audio service every second;
application capture fed 44.1 kHz frames into a 48 kHz Mixer without resampling.
Exact bound-process checks replace routine inventory, preserving verified restart
matching; process capture now requests graph-rate PCM16 stereo. User closed
integration, AudioRouter and Siege normally for qualification. No global device
formats, stored graph, microphone binding or VST implementation changed.

Two 30-second mixed-input native runs pass with clean references and zero routed
glitches/silent runs/service late gaps. Plain-route repeat also passes (first
attempt inconclusive because its reference dropped a packet). Reproduction,
failed experiments, commands and rollback are in the
[Siege continuity evidence](../active/evidence/2026-10-02-siege-crackling-liveness.md).
Serial checks passed: Windows adapter 108, control 220 (8 ignored), shell 44
(1 ignored); documentation validation and diff whitespace checks passed.
Review: `target/reviews/siege-audio-20261002/audiorouter-shell.exe`, with hash
and build evidence in the linked report. Next: user confirms the same Siege +
Discord topology during gameplay. Installer/M08 and
hardware endurance gates remain open. Rollback retains the previous review exe
and reverts the liveness probe/fast paths and process-capture format correction.

Token visibility correction (2026-10-02, user approved): HTTP-03, UI-11/12.
Reproduction: generating a token while stopped saves it, but the token controls
are nested under the running API URL and cannot reveal it. Show the saved token
in the API tab independently of listener state, including immediately after
replacement; provide Copy and optional Hide. Keep persistence and the user's
new credential unchanged. Verify stopped/running/replacement/copy/fallback,
three-theme appearance, shell/UI checks, then build a separate review executable.
Rollback reverts this visibility slice only; it does not replace the saved token.
Implemented: API tab shows/copies token while stopped and after replacement.
Checks: 3 component tests, typecheck/build, 44 shell tests (1 ignored), 3 optimized
Edge theme cases and screenshot review passed. Updated review executable:
`C:\code\audiorouter\target\reviews\token-visibility-20261002\audiorouter-shell.exe`.
The previous review build is running; it was not stopped or overwritten.
Next action: close it normally and review this build, then resume Stats.cc setup.

Node-ID Properties slice (2026-10-02, user approved): UI-05/11/12 and API-09.
Also show Session ID in the Session tab with identical copy/fallback behavior.
Add a shared footer to every selected-node inspector with a muted Node ID
caption, selectable full ID and keyboard-accessible copy icon. Tooltip explains
API integrations; show brief Copied status and manual-copy guidance on failure.
Preserve all Stats.cc example changes. Verify copy success/failure, selection
changes/long IDs, production browser appearance in all three themes at 1280Ã—720,
type checking and docs. Rollback removes the footer/component/style only; no
graph or backend contract changes. Implemented; copy/fallback unit checks and
production Edge checks passed in all three themes. See the
[identity/token evidence](../active/evidence/2026-10-02-identity-and-persistent-token.md).

Persistent HTTP token slice (2026-10-02, user approved): HTTP-03 and SEC-01/10.
User requests configuration of the Stats.cc example with the supplied current
token, retained until explicit regeneration. Store the token encrypted with
current-user Windows DPAPI outside source control; example reads it privately.
API remains stopped at app launch. Stop/restart reuses the saved token; explicit
Generate new token replaces it and invalidates old credentials. Fail closed on
corrupt/decrypt-failed storage; no silent replacement. Add storage/restart/rotation
and UI checks, update HTTP/security docs, inspect three themes. Preserve user's
running app and all example changes. Rollback removes persistence/rotation UI;
old builds regenerate tokens on start, so integrations must be reconfigured.
Implemented and qualified by 44 shell tests (1 ignored), 7 focused UI tests,
3 production Edge theme cases and real read-only HTTP inspection. Review build:
`C:\code\audiorouter\target\reviews\identity-token-20261002\audiorouter-shell.exe`.
Close the old app normally before launching it; no fresh-install/installer gate
is claimed for this unpublished review build.

Current authorized task: [Stats.cc Siege integration](../active/stats-cc-siege-integration.md).
Investigate its local state data, then implement an independent example service
under `examples/integrations/` controlling only the intended mixer input levels.
Release qualification remains open; this task does not reopen VST implementation.

Prepare a reviewable Windows 11 x64 public-release candidate with truthful
evidence, clear troubleshooting, and a recoverable installer. Requirements:
M08's approved non-driver scope, DIST-01â€“08, ENG-01â€“05, CAP-01/13,
GRAPH-15/16, UI-03/04/11/12/13, API-09 and SEC-10/13.
The user authorizes unsigned v0.0.3 and GitHub prerelease publication for Joe.
Signing enrollment and full public-release qualification remain separate work.

Completed work and detailed experiments are in the
[execution archive](../archived/2026-10-02-pre-release-execution-log.md).
Read [M08](../../milestones/M08-release.md) and the
[release checklist](../../operations/release-qualification.md) before release work.

## Where things stand

| Area | Current state | Evidence / next check |
| --- | --- | --- |
| Public package | Unsigned v0.0.3 prerelease published for Joe; tagged clean source, exact-exe fresh-install and downloaded hashes verified | [Release evidence](../active/evidence/2026-10-02-joe-prerelease-0.0.3.md); v0.0.2 retained for rollback |
| Current build | Current UI, discovery fix, clearer logs, instance recovery and log-folder controls | [Release evidence](../active/evidence/2026-10-02-joe-prerelease-0.0.3.md); native recovery/Explorer actions need attended review |
| Usability | Network direction diagram, prominent install instructions, video placeholders; canvas list switch removed, keyboard controls under Advanced | [147 UI tests and 9 production Edge cases](../active/evidence/2026-10-02-usability-followups.md); attended accessibility remains open |
| Joe's input list | Old log showed nine 0xE000020B failures; inventory now skips only disappeared endpoints and logs the failed stage | Verify on Joe's PC; exact original endpoint/stage was absent from the old log |
| Routing / recording / LAN | Existing Windows evidence and user-reported two-PC use; multi-path recorder repair validated by user | [Continuity](../active/evidence/2026-09-26-audio-continuity.md), [network](../active/evidence/2026-10-01-network-qualification.md), archive; refresh candidate hardware/soak evidence |
| External app control | Existing REST supports active-session node names/IDs, flags and parameters | [Examples](../../operations/local-http-api.md); [request-builder proposal](../future/external-app-integrations.md) is future work |
| Signing | User prefers SignPath if feasible; MIT license approved; enrollment/acceptance/hosted signing uncompleted | [Signing plan](../active/release-signing-and-publication.md); unsigned first release remains allowed under DEC-17 |

Latest executable:
`C:\code\audiorouter\target\releases\v0.0.3\audiorouter-shell.exe`.
Keep the CLI and plugin worker beside it. Installer in the same folder:
`AudioRouter_0.0.3_x64-setup.exe`. Version/build identity `v0.0.3`.

## Decisions in force

- No VST implementation changes for these requests. â€œPlugin systemâ€ means
  external feature/control integration; the request-builder proposal is not
  approved implementation. Game menu/match volume changes use Volume or Mixer;
  Duck is audio-level triggered.
- AudioRouter-owned drivers remain excluded (DEC-16). Per-user installer and
  install/recovery gates are in scope (DEC-17); unsigned status must be disclosed.
- First Play asks for device consent; developer variables cannot qualify a release.
- Network audio is user-directed, unencrypted LAN UDP (GRAPH-16, SEC-13).
- HyperFrames video work is discontinued. The 21 local skills and their lockfile
  were removed by user request; old video artifacts are retained outside this plan.
- Preserve unrelated changes, the user's database and running app. Do not launch
  a second desktop shell or stop a user-owned process.

## Ordered release tasks

Joe prerelease (2026-10-02): user explicitly requests a GitHub release for
testing these fixes. Prepare unsigned v0.0.3 in an isolated clean release
checkout; preserve unrelated video/signing work and the running app. Run current
Rust/shell/UI/contracts/docs and production UX checks; build installer plus
matching companions, provenance/SBOM/hashes; run fresh-install on the exact
packaged shell after the user closes their app. Publish as a prerelease with
open M08/native-action gates and rollback disclosed, then download and verify
the uploaded package. No signing enrollment or full M08 completion is implied.
Rollback retains v0.0.2 and uses a new repair version rather than replacing assets.

Current implementation slice (2026-10-02): UI-13, API-09, SEC-01/10 and ENG-05.
User requests understandable duplicate-instance recovery and convenient log access.
Detect same-user AudioRouter shells before database/backend startup; offer an
explicit force-close-and-continue action with audio/recording consequences, never
terminate arbitrary processes. Distinguish an already-prepared in-process worker.
Add desktop-only log-folder open/copy controls in Logs. Verify process targeting,
cancel/retry behavior, UI error handling, three themes and fresh embedded assets.
Real termination of the user's running app is not authorized as an agent test.
Rollback: remove this shell/UI slice; stored session formats are unchanged.
Implemented: early native instance recovery with explicit force-close and Cancel,
truthful worker-preparation guidance, Logs open/copy controls and fallback.
Checks: 68 UI tests, 43 shell tests plus final 2 process-policy/inventory checks,
and 3 optimized Edge theme cases pass. Native force-close and Explorer actions
remain attended checks; no user-owned process was stopped. See latest UX evidence.

1. Review this build and verify Joe's input list. Collect shell/backend/discovery
   logs with reproduction time if it fails; document affected versions.
2. Audit the M08 requirement/evidence matrix. Carry forward unresolved gates
   below and reconcile stale capability/docs claims. Choose the release version
   and signing approach before packaging.
3. Run current full Rust, shell, UI, contract and production-browser suites from
   clean pinned inputs. Resolve failures; rebuild UI before shell. Record commands,
   Windows environment and results in dated evidence.
4. Build the matching release executables/installer, hashes, manifest, notices and
   SBOMs. Run exact-exe fresh-install qualification with no other app or developer
   access variables. Then standard-user clean-machine install/upgrade/uninstall.
5. Complete supported hardware, recording, accessibility, performance/security
   gates or record explicitly approved scope decisions. Prepare release notes,
   recovery instructions and known issues. Present exact assets for publication.

## Validation matrix and blockers

| Gate | Required evidence / current limitation |
| --- | --- |
| Exact packaged exe first run | Passed 1/1 on v0.0.3: fresh DB, default pipe, no developer grants, no other shell; no real audio opened |
| Installer (DIST-01â€“08) | Standard-user install, WebView2 present/absent, upgrade, uninstall/state retention and rollback; old v0.0.2 fresh-install pass does not qualify this build |
| Automated candidate checks | Workspace/shell/UI/contracts/docs and 12 production UX cases passed. Full dev E2E not rerun: 2026-10-01 retained eight failures and eight skips; revisit original evidence before classifying failures |
| Audio / recording / performance | Supported device/app matrix, long-duration/endurance, parsed recording output, dropout/continuity and resource distributions; Windows hardware required |
| Accessibility / first-time usability | Narrator, keyboard, 200% scaling and M08 first-time-user scenarios; screenshots/unit tests do not close these |
| Security / plugins / contracts | CAP-13 and GRAPH-15 evidence reconciliation, plugin rights/containment/multi-vendor matrix, contract drift, retention bounds; no new VST implementation authorized |
| Logs (SEC-10) | Typed/redacted diagnostics verified; aggregate diagnostic retention is currently three 5 MiB current/previous pairs, so full 20 MiB audit-budget compliance remains open |
| Signing | Credentials/provider acceptance absent; do not represent an unsigned candidate as signed |

Known operating risks to review: independent-device clock drift lacks correction;
queues can retain latency after stalls; virtual-cable output dropouts were observed
on earlier builds; plugin-reported latency is not fully reflected in path timing.
These are historical findings requiring candidate verification, not new passes.

## Verification, rollback and next action

Latest UX results: 68 focused UI tests, 43 shell tests (1 ignored) and 3 production
Edge theme cases passed; final process-policy/inventory checks passed 2/2.
See [UX evidence](../active/evidence/2026-10-02-instance-recovery-and-log-access.md) for
build, screenshots and native-action limitations. Earlier discovery/control/log
regressions are in [diagnostics evidence](../active/evidence/2026-10-02-diagnostics-review-build.md).
Documentation/link and diff checks pass.

Rollback: retain the published v0.0.2 package and a compatible database backup;
this review slice adds no migration. Use a new release version for repairs;
do not overwrite published assets. Archive this plan only when its scope is
complete, keeping evidence and unresolved limitations accessible.

Candidate source checks on Windows: full workspace Rust suite passed (native,
hardware and opt-in cases remain ignored), 43 shell tests passed with 1 ignored,
436 UI tests and type checking passed, contract type checking/drift passed,
and 12 production Edge cases passed across dark/light/high-contrast themes.
Dependency lock diffs contain workspace version changes only. A broken release
notes heading link was repaired before packaging. See the
[Joe release evidence](../active/evidence/2026-10-02-joe-prerelease-0.0.3.md).

**Exact next task:** attended setup and validation of the
[Stats.cc integration example](../active/stats-cc-siege-integration.md): prep 30%, action
100%, Discord always 100%. Implementation and 11 fixture checks are complete;
Private example configuration and read-only authenticated HTTP inspection passed.
Stats.cc feed and real gameplay are pending. Joe's enumeration
and remaining M08 qualification remain separate open tasks.

