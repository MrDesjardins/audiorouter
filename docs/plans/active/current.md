# Active plan — public release preparation

Updated: 2026-10-02. Status: v0.0.3 Joe prerelease published; M08 qualification open.

## Objective and scope

Prepare a reviewable Windows 11 x64 public-release candidate with truthful
evidence, clear troubleshooting, and a recoverable installer. Requirements:
M08's approved non-driver scope, DIST-01–08, ENG-01–05, CAP-01/13,
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
| Public package | Unsigned v0.0.3 prerelease published for Joe; tagged clean source, exact-exe fresh-install and downloaded hashes verified | [Release evidence](evidence/2026-10-02-joe-prerelease-0.0.3.md); v0.0.2 retained for rollback |
| Current build | Current UI, discovery fix, clearer logs, instance recovery and log-folder controls | [Release evidence](evidence/2026-10-02-joe-prerelease-0.0.3.md); native recovery/Explorer actions need attended review |
| Usability | Network direction diagram, prominent install instructions, video placeholders; canvas list switch removed, keyboard controls under Advanced | [147 UI tests and 9 production Edge cases](evidence/2026-10-02-usability-followups.md); attended accessibility remains open |
| Joe's input list | Old log showed nine 0xE000020B failures; inventory now skips only disappeared endpoints and logs the failed stage | Verify on Joe's PC; exact original endpoint/stage was absent from the old log |
| Routing / recording / LAN | Existing Windows evidence and user-reported two-PC use; multi-path recorder repair validated by user | [Continuity](evidence/2026-09-26-audio-continuity.md), [network](evidence/2026-10-01-network-qualification.md), archive; refresh candidate hardware/soak evidence |
| External app control | Existing REST supports active-session node names/IDs, flags and parameters | [Examples](../../operations/local-http-api.md); [request-builder proposal](../future/external-app-integrations.md) is future work |
| Signing | User prefers SignPath if feasible; MIT license approved; enrollment/acceptance/hosted signing uncompleted | [Signing plan](release-signing-and-publication.md); unsigned first release remains allowed under DEC-17 |

Latest executable:
`C:\code\audiorouter\target\releases\v0.0.3\audiorouter-shell.exe`.
Keep the CLI and plugin worker beside it. Installer in the same folder:
`AudioRouter_0.0.3_x64-setup.exe`. Version/build identity `v0.0.3`.

## Decisions in force

- No VST implementation changes for these requests. “Plugin system” means
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
| Installer (DIST-01–08) | Standard-user install, WebView2 present/absent, upgrade, uninstall/state retention and rollback; old v0.0.2 fresh-install pass does not qualify this build |
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
See [UX evidence](evidence/2026-10-02-instance-recovery-and-log-access.md) for
build, screenshots and native-action limitations. Earlier discovery/control/log
regressions are in [diagnostics evidence](evidence/2026-10-02-diagnostics-review-build.md).
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
[Joe release evidence](evidence/2026-10-02-joe-prerelease-0.0.3.md).

**Exact next task:** Joe tests the v0.0.3 primary-input list; collect logs and
reproduction time if it fails. Then attended instance recovery/log-folder review
and remaining M08 qualification. Full M08 is not complete.
