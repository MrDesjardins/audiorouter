# Active-plan history: 2026-11-16 to 2026-11-24

> **Archived 2026-11-24 — the former `active/current.md`, kept as history.** Open gates are in
> [human-gated tasks](../active/human-gated-tasks.md); the current priorities are in the
> [active plan](../active/current.md).

## Where things stand

Owner test-build refresh (2026-11-22): rebuilt local `main`
`ba34371ad190776193a630626bc27a117372b755` with
`powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-desktop.ps1`.
Release shell, matching CLI, x64/x86 VST2/VST3 workers, UI typecheck and UI
production build passed on Windows. Shell at
`src-tauri/target/release/audiorouter-shell.exe` has SHA-256
`F39C926411367131532573B0F0ACD0E868D03CDFAADF53099935370538B621C7`
and is newer than `ui/dist/index.html`. Packaged smoke passed using
`node tools/quality/packaged-shell-smoke.mjs --shell src-tauri/target/release/audiorouter-shell.exe --output target/local-test-build-2026-11-22 --timeout-ms 60000`:
isolated database/WebView profile, native bridge, save/restart and graceful
quit, with zero uncaught UI errors. Evidence:
`target/local-test-build-2026-11-22/result.json`. Build warnings: chunks above
500 kB and bundler handling of the React runtime's `use no memo` directive;
retained without dependency or code changes in this build-only task. No live
audio or plugin processing was exercised. No source changes or release/driver
qualification claimed. Next: owner tests this executable with their routes.

`main` is at `ba34371ad` (2026-11-23, after #170–#180). The November 6–16 work
streams are merged: release-readiness repairs (#155–#159), the three-day code
review (#158), device-binding recovery (#154), theme-editor preview (#153), the
VST plugin experience (#152), live-parameter autosave (#150), the resilience
implementation (#147), the high/medium audit repairs (#142), CI speed (#143),
update-check hardening (#140), plugin reliability (#139), the audio-latency
repair (#137) and the virtual-cable review fixes (#144/#145).

Their execution plans are archived. The 2026-11-16 consolidation moved the
previous active plan's chronological history into
[Active-plan history](2026-11-16-active-plan-history.md) and left
this short priorities page behind.

No feature below is release-ready. The app-only candidate is 0.0.32, unsigned
and experimental; the driver track is separate. Requirement IDs live in the
linked plans, not here.

## Latest main update: global-only keyboard shortcuts

PR #151 implements the owner's UI-10 decision: Windows-wide global keybinds
are the only audio shortcut mechanism. Local shortcut settings, defaults and
the `audiorouter.ui.shortcuts` preference were removed. Global keybinds offer
session start/stop, privacy mute, push-to-talk and soundboard slots; duplicate
enabled chords are reported. This supersedes the 2026-11-07 default-binding
decision. Requirements: UI-10, UI-11, UI-29.

The merged work records UI typecheck/lint/format, 1,119 unit tests, production
build, 127 shell tests, strict Clippy, Rust formatting, docs validation and 17
focused Edge scenarios passing. No attended Windows hook or audio run was
claimed. Rollback: revert that change; saved global keybinds and shell settings
are unaffected, and the old local-shortcut preference is ignored.

Issue #149 supplied-session verification and its remaining exact-plugin live
gate are recorded in [the verification plan](2026-11-24-issue-149-session-verification.md).

## Current priorities

Ordered. Each item is authorized work unless it names an owner decision.

Delegation briefs prepared 2026-11-18: [app-only release qualification](2026-11-24-delegate-release-qualification.md)
and [VST2/VST3 compatibility](2026-11-24-delegate-vst-compatibility.md). These define
separate agent scopes, ordered tasks, evidence and coordination; no execution
or additional publication authority is implied. The VST brief requires
reconciling the later handoff against older open-work summaries first.

1. **Release qualification (DIST-02/08, NFR-16, M08).** Finish the automated VM
   installer-lifecycle harness: make guest control repeatable with retained
   structured logs, snapshot the prepared baseline, then cover registration,
   real recording retention, repair and missing-WebView2. Keep the intermittent
   VirtualBox guest-control stalls out of any CI claim until a repeatable
   timeout/recovery path is qualified. Runner and usage:
   [release qualification](../../operations/release-qualification.md).
   The [2026-11-19 execution plan](2026-11-24-release-qualification.md) reconciles the
   completed timeout/recording runs and identifies the remaining interactive,
   WebView2 diagnostic, first-run and exact-candidate acceptance work. Use it
   before the older summary above. Execution found that v0.0.31 is already
   published and is the frozen candidate. **Status 2026-11-21:** a new fresh
   guest with a standard user exists; RQ-02 (timeout, three consecutive wrapper
   passes) and RQ-03 (real-guest refusals) pass, RQ-04 (full lifecycle on v0.0.31)
   passes with a recorded Defender-exclusion deviation, and RQ-05 is partial
   (guest interactive pass and an isolated real-endpoint audio run pass; the
   downloaded-file trust prompt and offline operation are still open). Defender
   quarantines the unsigned v0.0.31 installer in a fresh guest
   (`Trojan:Win32/Wacatac.H!ml`); a Microsoft false-positive submission is
   pending and signing is decided in
   [release signing](2026-11-24-release-signing-and-publication.md)
   ([record](../active/evidence/2026-11-19-release-qualification-v0.0.31.md)).
2. **Full VST2/VST3 compatibility (PLUG-01…07, GRAPH-14, M06/M07).** Remaining
   phases P2 (programs/shell/preset import), P3 (units/programs/multi-class
   qualification), P5 (sidechain graph execution), P6 (editor qualification)
   and the P9 release gate, plus real editor/DPI/pumped-parent, owner
   endpoints, i686 hosted CI and signing. Start at the
   [VST handoff](2026-11-24-vst-handoff.md), then the
   [completion work plan](2026-11-24-vst-completion-work.md) and
   [full compatibility](2026-11-24-vst-full-compatibility.md).
3. **Virtual cable (DEC-18, VCAB-24/25/27/28/30).** RC 0.2.5 is frozen
   (commit `7c88895c2`, no driver change since) and Stage A ran on it on
   2026-11-11/12, but **is not passed**: the 2026-11-21
   [gap analysis](../active/evidence/2026-11-21-virtual-cable-stage-a-0.2.5-gap-analysis.md)
   lists what blocks it (frame-clock revert gate, unexplained render underruns,
   unload with a live route, the post-fuzz guest freeze, tagged pool, A15
   unexplained events). Next: a guest A/B wrapper, then one owner VM block; a
   worse A/B means a new RC above 0.2.6. Then Stage C and the owner-run signing
   track. Plans: [virtual cable](2026-11-24-virtual-cable.md),
   [review fixes](2026-11-24-virtual-cable-review-fixes.md) and the
   [agent handoff](2026-11-24-virtual-cable-agent-handoff.md). Do not install the
   test-signed driver on the development PC.
4. **UI quality review (UI-*).** Narrator and real Windows 150 %/200 % scaling
   are re-confirmed on the packaged 0.0.32 shell (Round 30, 2026-11-24, owner
   attended); a first-time-user rehearsal led to a new “Mic to Discord” goal
   (no game step), but the real participant study is open. Still open: live-route device-loss messaging on
   the rebuilt exe, physical-key dropdown cancellation, a narrow 200 % window,
   the M08 participant study and refreshed guide clips. See
   [UI quality review](2026-11-24-ui-quality-review.md).
5. **Application quality roadmap.** Q3.2 is complete (2026-11-23). Next: the
   two CI-flake repair branches, then Q4–Q6; see
   [application quality](../active/application-quality.md).
6. **Audio-latency residual owner gates (ARCH-08, CAP-04, NFR-03).** The
   real-session `audioHealth` log is analysed (55.8 h after the repair,
   [evidence](../active/evidence/2026-11-23-audio-health-after-latency-repair.md)).
   The owner's listening test was done across recent evening sessions and
   chose to keep the 10 ms `voice-to-cable-a` cushion floor (2026-11-24), so
   the pump constants are unchanged. The automated multi-input continuity runs
   passed on CABLE/CABLE-A; a long session through the owner's own app is not
   separately claimed.
7. **Code-review-fixes residual qualification.** Isolated VB-Cable
   continuity/service-gap runs, large-library service timing and
   latency-changing commercial-plugin continuity; see
   [code-review fixes](../active/code-review-fixes.md).
8. **MCP setup UX (AUTO-06) and attended OBS/Discord acceptance.** See
   [MCP setup UX](2026-11-24-mcp-setup-ux.md) and [OBS/Discord](2026-11-24-obs-discord.md).
9. **Maintenance backlog.** #187 fixed the
   [2026-11-11 bug audit](2026-11-11-bug-audit.md) low-severity
   items (DSP-4, SES-4, UI-1; REC-2 not reproduced; HTTP-1/UI-2 already fixed)
   and resilience RES-11a/RES-11b, and added synthetic large-bundle service
   timings for RES-6. Checked 2026-11-24 at `53e1de80e`: hosted CI and CodeQL
   are green with no check annotations and **zero open CodeQL alerts**, so the
   [code-scanning remediation](2026-11-24-code-scanning-remediation.md) refresh is current.
   Still open: RES-10 plugin-worker restart stays an owner decision, and RES-6
   needs representative real library/plugin workload timings beyond the
   synthetic fixtures
   ([resilience implementation](2026-11-13-resilience-implementation.md)).

## Open owner gates

These need hardware, an owner decision or a fresh hosted run. They are not
blocked on implementation, and mocks or portable tests must not stand in for
them.

- Installer install/upgrade/rollback/uninstall and data retention on a clean
  standard-user Windows system, WebView2 absence, and the signed artifact
  ([release signing](2026-11-24-release-signing-and-publication.md), parked on the owner's
  provider/EV-certificate decision).
- Real MIDI unplug/replug and duplicate names; sidechain plugin failure on a
  protected voice path; sleep and device recovery on live endpoints.
- Attended UI checks: Narrator, Windows 150 %/200 % scaling, physical-key
  dropdown cancellation, and a first-time-user walk-through.
- Endurance and load-controlled UDP continuity; i686 hosted CI.

## Active plan index

[Post-game execution](2026-11-24-post-game-execution.md) covers priorities 2–10 from the
November 24 ranked roadmap. Start after the owner says the game is finished
and asks to execute it. The installer qualification workstream has its own plan.

Owner-tested local build and next prerelease (2026-11-22):
[0.0.32 candidate preparation](2026-11-24-release-0.0.32.md). Build in an isolated
worktree while the owner's app remains running; publication is a separate
human action. No driver or signing scope change.

Owner-requested UI-only code review (2026-11-22):
[review findings and evidence](2026-11-24-ui-code-review.md).
Reviewed latest main `c6f59c195`; seven reproduced findings, including Sound
Check stop recovery and standalone Instrument playback. All seven now have
repairs: 1,165 UI tests, three-theme Edge checks, engine combination suites and
Firefox guide playback pass. The fresh built shell ran on an isolated external
backend; normal owned-backend save/restart smoke remains open while the owner's
app holds the default pipe and executable. See the record for its exact scope.

Owner-requested UI language support (2026-11-19):
[plan](2026-11-24-ui-localization.md). Implemented: central per-language JSON catalogs
with validation/generation for TypeScript keys and native Rust text, live
Language selection in Settings → Appearance, localized top bar, workbench
navigation, Settings/onboarding/dialog surfaces, and the native tray
(static labels plus status/recording/playback text) following the choice.
English is the only shipped language. Remaining work is the canvas/Properties
and tool catalogs, structured backend error mapping, a real reviewed second
language, and three-theme/Narrator/packaged evidence; see the plan’s execution
record. No release or PR is authorized by the plan.

Owner-requested REST API and MCP quality review (2026-11-21):
[findings](2026-11-24-api-mcp-quality.md). 14 ranked findings (a network preset path
reachable through MCP, non-retry-safe session creation, Rust debug text in
error messages, wrong tool annotations, protocol conformance). All 14 were
fixed in PR #170. A second review (2026-11-22, same page) found 13 default
MCP tools that no MCP role can use, a REST path that revokes the window's own
enrollment, and `sessions.start` reporting a silent route as running. The
owner chose R2-1 option (b); the repairs merged in #171 and #174. Open owner
questions: whether assistants may record, and session rename for assistants.

Execution plans that still hold open work:

- [Application quality](../active/application-quality.md) — Q3.2 hosted, then Q4–Q6.
- [Code review fixes](../active/code-review-fixes.md) — residual continuity and
  service-timing qualification.
- [Code-scanning remediation](2026-11-24-code-scanning-remediation.md) — current-head
  CodeQL refresh and alert dispositions.
- [MCP setup and connection UX](2026-11-24-mcp-setup-ux.md).
- [OBS and Discord](2026-11-24-obs-discord.md) — copied-profile and attended acceptance.
- [Release signing and publication](2026-11-24-release-signing-and-publication.md) —
  parked on owner/provider signing decisions.
- [UI quality review](2026-11-24-ui-quality-review.md).
- [Virtual cable](2026-11-24-virtual-cable.md),
  [virtual-cable review fixes](2026-11-24-virtual-cable-review-fixes.md) and the
  [agent handoff](2026-11-24-virtual-cable-agent-handoff.md).
- VST: [full compatibility](2026-11-24-vst-full-compatibility.md),
  [completion work](2026-11-24-vst-completion-work.md), [handoff](2026-11-24-vst-handoff.md),
  [MIDI/instrument design](2026-11-24-vst-midi-instruments-design.md),
  [issue #136 experience](2026-11-24-issue-136-vst-experience.md) and the
  [VST2 chain matrix](2026-11-24-issue-136-vst2-chain-matrix.md).

The [future-plan index](../future/README.md) is deferred backlog, not an
implicit implementation queue.

## Recently archived (2026-11-16)

Completed implementations moved to `docs/plans/archived/` by the consolidation.
Each carries an archive banner, and its residual gates are listed above.

- [Active-plan history](2026-11-16-active-plan-history.md) — the
  previous current.md logs from 2026-09 to 2026-11-16.
- [Resilience implementation](2026-11-13-resilience-implementation.md)
  and [resilience audit](2026-11-13-resilience-audit.md) (PR #147).
- [Code review repairs](2026-11-15-code-review-repairs.md) (PR #158).
- [Device binding recovery](2026-11-15-device-binding-recovery.md)
  (PR #154).
- [Live parameter autosave](2026-11-14-live-parameter-autosave.md)
  (PR #150).
- [Audit repairs](2026-11-12-audit-repairs.md) and the
  [bug audit](2026-11-11-bug-audit.md) (PR #142).
- [CI speed](2026-11-12-ci-speed.md) (PR #143).
- [Plugin reliability](2026-11-11-plugin-reliability.md) (PR #139).
- [Update-check hardening](2026-11-11-update-check-hardening.md)
  (PR #140).
- [Audio latency](2026-11-11-audio-latency.md) (PR #137).
- [CI quality repair](2026-11-09-ci-quality-repair.md) (PR #134).

## Next action

### Application quality and audio latency gates (owner request, 2026-11-23)

Owner asked to work on priorities 5 and 6. Done so far:

- Q3.2 is complete on hosted runners (nightly packaged smoke and its guard
  qualification pass; see [application quality](../active/application-quality.md)).
- `main` CI was red after #178/#180. Two of the three failing tests were real
  defects, each fixed on its own branch with a regression that fails without
  the fix: `fix/keybind-selection-after-failed-delete` (UI, a failed bulk
  delete lost the selection) and `fix/pipe-guard-claim-race` (transport, a
  client could take the backend's guard pipe before it was claimed, so the
  backend never started). The third (`LogFilesPanel`, one coverage timeout)
  did not reproduce and is left unchanged.
- The real-session health log is analysed:
  [2026-11-23 evidence](../active/evidence/2026-11-23-audio-health-after-latency-repair.md).
  Output queues fell from about 50 ms to 20 ms median, full-ring drops from
  about 14,000 to 0, and gaps outside stalls from 14–18 to 0.1–0.4 an hour.

Next:

1. Verify the consolidated 0.0.32 candidate in PR #181. The owner approved
   including both repair branches and `docs/quality-latency-2026-11-23` in
   that PR; separate PRs for these branches are no longer needed. Follow
   [the release plan](2026-11-24-release-0.0.32.md) for combined checks and exact artifacts.
2. Owner, tonight: stop the route and free the cables, then the agent runs
   `live_backend_service_keeps_a_routed_tone_continuous` for the multi-input
   worker on the owner's cables.
3. Done: the owner's listening test across recent evening sessions; the
   `voice-to-cable-a` cushion floor is decided (keep 10 ms, 2026-11-24), so no
   pump constant changed and the remaining gaps stay as measured.
4. Then Q4.2 restart interleaving and Q5.1–Q5.2 metrics, as listed in
   [application quality](../active/application-quality.md).

### Overnight 2026-11-24 work and its follow-ups

All of this is local only: no branch was pushed and no PR was opened. Details
are in the evidence files, not repeated here.

- Integration: `integration/overnight-2026-11-24` at `84bfad559` contains every
  2026-11-24 slice (Q4.1 contract tests, Q4.2 fault interleavings, Q4.3 fuzzing
  and worker-frame hardening, the low-audit, resilience, VST, L10N and UI-audit
  branches) and the independent review's PLAN-1 repair. Records:
  [overnight integration](../active/evidence/2026-11-24-overnight-integration.md) and
  [review](../active/evidence/2026-11-24-integration-review.md). Q4.2's restart
  interleaving is now done; Q5.1–Q5.2 remain.
- `fix/instrument-app-usage` (`4f3b4e8f8`): the OBS and app-usage checks use
  the exhaustive source classification, so an Instrument is named and a
  disabled one no longer counts as passing sound.
- `fix/rust-small-defects` (new, from `84bfad559` + the Instrument fix):
  closes the three Q4.1 owner items without changing any role policy.
  Optional `minLength: 1` strings now refuse `""`; `probe_pipe` errors keep
  their reason instead of reading as "app not running"; MCP `create_session`
  and `start_from_goal` refuse an Editor before creating a session instead of
  creating one it cannot open. Record:
  [rust small defects](../active/evidence/2026-11-24-rust-small-defects.md).
  Workspace and shell tests, Clippy, both fmt checks and the full Edge
  Playwright suite (862 passed, 16 skipped) pass; not run: built exe, hosted
  CI.
- Not started, still owner decisions: the signed updater's key custody and
  feed host, assistant recording (R2-1a), `rename_session` for assistants
  (R2-4), RES-10 plugin worker restart, and the unreachable keyboard
  connection dialog (delete or restore).
- PR #187 is the first hosted run. Its first pass failed only on `rustfmt`
  (one misindented doc comment) and CodeQL's test-only `cleartext logging` /
  `hard-coded key` heuristics; both are fixed on the branch, with the timing
  harness now writing rows only to its CSV file.

Start with priority 1 or 2 above. Nothing on this page authorizes a pull
request, a release, a driver install or a change to the owner's live audio or
saved data; see [AGENTS.md](../../../AGENTS.md) for the approval and evidence
rules.
