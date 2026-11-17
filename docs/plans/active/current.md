# Active plan — current priorities (2026-11-16)

## Where things stand

`main` is at `8f42bf8de` (PR #151, global-only keyboard shortcuts). The November 6–16 work
streams are merged: release-readiness repairs (#155–#159), the three-day code
review (#158), device-binding recovery (#154), theme-editor preview (#153), the
VST plugin experience (#152), live-parameter autosave (#150), the resilience
implementation (#147), the high/medium audit repairs (#142), CI speed (#143),
update-check hardening (#140), plugin reliability (#139), the audio-latency
repair (#137) and the virtual-cable review fixes (#144/#145).

Their execution plans are archived. The 2026-11-16 consolidation moved the
previous active plan's chronological history into
[Active-plan history](../archived/2026-11-16-active-plan-history.md) and left
this short priorities page behind.

No feature below is release-ready. The app-only candidate is 0.0.30, unsigned
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
gate are recorded in [the verification plan](issue-149-session-verification.md).

## Current priorities

Ordered. Each item is authorized work unless it names an owner decision.

1. **Release qualification (DIST-02/08, NFR-16, M08).** Finish the automated VM
   installer-lifecycle harness: make guest control repeatable with retained
   structured logs, snapshot the prepared baseline, then cover registration,
   real recording retention, repair and missing-WebView2. Keep the intermittent
   VirtualBox guest-control stalls out of any CI claim until a repeatable
   timeout/recovery path is qualified. Runner and usage:
   [release qualification](../../operations/release-qualification.md).
2. **Full VST2/VST3 compatibility (PLUG-01…07, GRAPH-14, M06/M07).** Remaining
   phases P2 (programs/shell/preset import), P3 (units/programs/multi-class
   qualification), P5 (sidechain graph execution), P6 (editor qualification)
   and the P9 release gate, plus real editor/DPI/pumped-parent, owner
   endpoints, i686 hosted CI and signing. Start at the
   [VST handoff](vst-handoff.md), then the
   [completion work plan](vst-completion-work.md) and
   [full compatibility](vst-full-compatibility.md).
3. **Virtual cable (DEC-18, VCAB-24/25/27/28/30).** After the 2026-11-10 review
   fixes, the next package is RC 0.2.5 and Stage A restarts on it; then Stage C
   and the owner-run signing track. Plans: [virtual cable](virtual-cable.md),
   [review fixes](virtual-cable-review-fixes.md) and the
   [agent handoff](virtual-cable-agent-handoff.md). Do not install the
   test-signed driver on the development PC.
4. **UI quality review (UI-*).** Continue the rounds and close the open attended
   evidence: Narrator, real Windows scaling, live-route device-loss messaging,
   a first-time-user walk-through and refreshed guide clips. See
   [UI quality review](ui-quality-review.md).
5. **Application quality roadmap.** Q3.2 hosted qualification, then Q4–Q6; see
   [application quality](application-quality.md).
6. **Audio-latency residual owner gates (ARCH-08, CAP-04, NFR-03).** The owner's
   listening test, a real-session `audioHealth` log and live cable continuity.
7. **Code-review-fixes residual qualification.** Isolated VB-Cable
   continuity/service-gap runs, large-library service timing and
   latency-changing commercial-plugin continuity; see
   [code-review fixes](code-review-fixes.md).
8. **MCP setup UX (AUTO-06) and attended OBS/Discord acceptance.** See
   [MCP setup UX](mcp-setup-ux.md) and [OBS/Discord](obs-discord.md).
9. **Maintenance backlog.** Refresh the
   [code-scanning remediation](code-scanning-remediation.md) alert counts from
   the current CodeQL run. The low-severity findings of the
   [2026-11-11 bug audit](../archived/2026-11-11-bug-audit.md) and resilience
   RES-10/11 remain unfixed, and RES-6 still needs representative
   large-import/export timings
   ([resilience implementation](../archived/2026-11-13-resilience-implementation.md)).

## Open owner gates

These need hardware, an owner decision or a fresh hosted run. They are not
blocked on implementation, and mocks or portable tests must not stand in for
them.

- Installer install/upgrade/rollback/uninstall and data retention on a clean
  standard-user Windows system, WebView2 absence, and the signed artifact
  ([release signing](release-signing-and-publication.md), parked on the owner's
  provider/EV-certificate decision).
- Real MIDI unplug/replug and duplicate names; sidechain plugin failure on a
  protected voice path; sleep and device recovery on live endpoints.
- Attended UI checks: Narrator, Windows 150 %/200 % scaling, physical-key
  dropdown cancellation, and a first-time-user walk-through.
- Endurance and load-controlled UDP continuity; i686 hosted CI.

## Active plan index

Execution plans that still hold open work:

- [Application quality](application-quality.md) — Q3.2 hosted, then Q4–Q6.
- [Code review fixes](code-review-fixes.md) — residual continuity and
  service-timing qualification.
- [Code-scanning remediation](code-scanning-remediation.md) — current-head
  CodeQL refresh and alert dispositions.
- [MCP setup and connection UX](mcp-setup-ux.md).
- [OBS and Discord](obs-discord.md) — copied-profile and attended acceptance.
- [Release signing and publication](release-signing-and-publication.md) —
  parked on owner/provider signing decisions.
- [UI quality review](ui-quality-review.md).
- [Virtual cable](virtual-cable.md),
  [virtual-cable review fixes](virtual-cable-review-fixes.md) and the
  [agent handoff](virtual-cable-agent-handoff.md).
- VST: [full compatibility](vst-full-compatibility.md),
  [completion work](vst-completion-work.md), [handoff](vst-handoff.md),
  [MIDI/instrument design](vst-midi-instruments-design.md),
  [issue #136 experience](issue-136-vst-experience.md) and the
  [VST2 chain matrix](issue-136-vst2-chain-matrix.md).

The [future-plan index](../future/README.md) is deferred backlog, not an
implicit implementation queue.

## Recently archived (2026-11-16)

Completed implementations moved to `docs/plans/archived/` by the consolidation.
Each carries an archive banner, and its residual gates are listed above.

- [Active-plan history](../archived/2026-11-16-active-plan-history.md) — the
  previous current.md logs from 2026-09 to 2026-11-16.
- [Resilience implementation](../archived/2026-11-13-resilience-implementation.md)
  and [resilience audit](../archived/2026-11-13-resilience-audit.md) (PR #147).
- [Code review repairs](../archived/2026-11-15-code-review-repairs.md) (PR #158).
- [Device binding recovery](../archived/2026-11-15-device-binding-recovery.md)
  (PR #154).
- [Live parameter autosave](../archived/2026-11-14-live-parameter-autosave.md)
  (PR #150).
- [Audit repairs](../archived/2026-11-12-audit-repairs.md) and the
  [bug audit](../archived/2026-11-11-bug-audit.md) (PR #142).
- [CI speed](../archived/2026-11-12-ci-speed.md) (PR #143).
- [Plugin reliability](../archived/2026-11-11-plugin-reliability.md) (PR #139).
- [Update-check hardening](../archived/2026-11-11-update-check-hardening.md)
  (PR #140).
- [Audio latency](../archived/2026-11-11-audio-latency.md) (PR #137).
- [CI quality repair](../archived/2026-11-09-ci-quality-repair.md) (PR #134).

## Next action

### Global keybinding creation UX (owner request, 2026-11-16)

Follow-up deletion UX (owner, 2026-11-16): the bulk minus icon was unclear and
single deletion required the bulk-name-edit flow. Requirements UI-10/11/17/29.
Plan: use a trash icon with a labelled bulk action and selection count; expose
Delete keybinding inside every saved binding; share named destructive
confirmation and exact-ID removal; preserve unrelated bindings on cancel or
save failure and restore focus after success. Add component and real-Settings
browser regressions (single, cancellation, bulk, failure), inspect all themes,
grade/iterate, then rebuild all desktop companions and smoke the packaged app.
Prerequisites: installed Windows build tools and browser dependencies; no audio
or driver access needed. Rollback: revert UI/tests; storage/API unchanged.
Result: saved bindings expose Delete keybinding with a trash-can icon; Manage
has a labelled trash-can bulk action with a fixed-width selection count.
Single confirmation names the binding and chord, bulk confirmation names the
count, and both are destructive dialogs. Cancel saves nothing. Successful
deletion announces completion and restores Add focus. A delayed failure
restores both bindings and the previous selection (regression explicitly waits
for the optimistic removal before rejecting the write).

Review iteration: initial visuals showed cramped Manage icon/text and little
bulk guidance; added spacing, guidance and specific deletion-success feedback.
Final agent self-review 8.5/10: visible single action, consistent trash icon,
clear scope/confirmation, keyboard focus, stable count and recoverable failure.
This is not a participant study. Reviewed single/bulk screenshots in dark,
light and high contrast at `target/keybinding-ux/delete-{single,bulk}-*.png`.

Windows checks: typecheck, lint, format:check and production build pass; full
UI suite passes 1,126 tests; focused suites pass 19. Six Edge scenarios pass
creation plus single/cancel/bulk deletion, exact persisted IDs, focus return,
and unchanged bulk-button bounds as selection changes, in all themes.
Docs acceptance passes (278 files/1,551 links), complete diff inspected and
diff check clean. Build retains the main-chunk size advisory and Playwright's
NO_COLOR/FORCE_COLOR warning documented below; no new application errors.

Fresh build: `tools/build-desktop.ps1` passes for shell, CLI, x64/x86 VST2/VST3
workers. Shell `src-tauri/target/release/audiorouter-shell.exe` SHA-256
`E02EC3CF1DEA0C70F454729CD79F3DD6568330237F97019965139E9C08ED7317`.
Packaged smoke passes (`target/keybinding-ux/delete-packaged-smoke/result.json`):
isolated Windows/WebView2 launch, native bridge, save/restart and graceful quit.
Deletion itself was exercised through the browser Settings UI with simulated
shell saves; live hook/audio and Narrator remain unrun. No owner data changed.
Owner approved the test build and explicitly requested a new PR. Fetched
origin: latest main is still `a981d3489`, already an ancestor of this work.
Branch `fix/global-keybinding-delete` contains the deletion follow-up and
depends on the still-open creation-form PR #161. Target that branch for a
focused stacked PR; merge #161 first. Reviewed its existing check annotations:
all ten are GitHub notices about ubuntu-latest migrating to Ubuntu 26, with
no source diagnostics; retain the notice rather than changing runner policy
in this UI change. No source changes since the passing checks/build above.
Next action: open the authorized deletion PR, then owner tests single/bulk
deletion in the rebuilt desktop app; retain the broader M05 attended gates.

Objective: configure name, shortcut and action together before activating a
binding (UI-10, UI-11, UI-17, UI-29; M05). Existing shell commands and storage
remain the authority; no API or audio behavior change is planned.

Prerequisites: Windows UI dependencies are installed. Browser shell invocation
can be simulated for form checks; this does not qualify the Windows hook.
Preserve unrelated untracked target-preview directories and tsbuildinfo.

Tasks: expose Add directly; open a local draft with all fields visible; require
a captured shortcut and valid action target; save once or cancel without writes;
keep failed drafts; show saved names, keys and actions together. Add regression
coverage, typecheck immediately, format/lint/build, and visually inspect dark,
light and high contrast. Grade clarity, efficiency, feedback, accessibility and
visual consistency; iterate until at least 8/10 and record evidence here.

Risks: shortcut capture must suspend existing hooks and restore them; cancel
must never save a partial binding. Rollback: revert the UI/test changes; saved
bindings retain the existing format.

Implementation and review: Add opens an expanded unsaved draft with name,
action and shortcut; Save validates the name/chord/target and writes once.
Cancel writes nothing, rejected saves retain the draft, duplicate chords warn
before saving, and saved names show key/action. Empty soundboard choices guide
the user to add the tool. No contract or shell-command changes were required.

UX iteration: first visual review scored 7.5/10 (keyboard exit and focus return
were weak). Added Tab to cancel capture, focus return after Save/Cancel, reserved
capture-help space and removed the redundant plus button. Final self-review:
clarity 9, efficiency 9, feedback/recovery 9, keyboard accessibility 8, visual
consistency 8; average 8.6/10, reported conservatively as 8.5. This is an agent
review, not a first-time-user study or Narrator qualification.

Windows verification (2026-11-16): `npm --prefix ui run typecheck:all`, lint,
format:check and production build pass; the full unit suite passed 1,123 tests
before the final keyboard/help refinement, and the final focused suites pass
17 tests. `npx playwright test --config=playwright.config.ts
global-keybinds.pw.ts --workers=1` from ui passes 3 Edge scenarios: real Settings
entry, form, capture, Tab cancellation, stable Save position, Save/Cancel focus,
one save and collapsed summary in all three themes. Screenshots reviewed:
`target/keybinding-ux/draft-{dark,light,high-contrast}.png` and matching saved
screenshots. Initial browser runs timed out on exact label matching for a
select (the label includes option text); using its combobox role fixed the
test locator. The component regression also caught optimistic-save duplicate
React keys; the rendered draft now excludes its optimistic saved copy.

Docs acceptance passes (278 Markdown files, 1,551 links). Production build
reports a 645 kB main-chunk advisory; kept visible here because splitting the
main application bundle is unrelated to this form change. Playwright reports
the runner's NO_COLOR/FORCE_COLOR conflict, with no application console errors.
Owner follow-up explicitly authorized a test build followed by a new PR.
Branch: `feat/global-keybinding-creation`, based on main `a981d3489`.
`tools/build-desktop.ps1` passed: release shell, matching CLI and x64/x86 VST2
and VST3 workers. Shell: `src-tauri/target/release/audiorouter-shell.exe`,
SHA-256 `BCC887131B254514F73B6E7BC4366515F3F99A360BCEF4131B7722B1035D5F6A`;
its UTC timestamp (2026-11-17 01:41:55) is newer than ui/dist (01:41:27).
`node tools/quality/packaged-shell-smoke.mjs --shell
src-tauri/target/release/audiorouter-shell.exe --output
target/keybinding-ux/packaged-smoke --timeout-ms 60000` passed on Windows 11
build 26300 with WebView2 157.0.4341.46, Medium/non-admin. Evidence:
`target/keybinding-ux/packaged-smoke/result.json`; fresh-profile launch,
native bridge, save/restart and graceful quit passed with zero uncaught UI
errors. Disposable database and WebView profile left owner settings unchanged.
No live Windows hook/audio exercised. Next task: owner test of the rebuilt
desktop shortcut creation/capture/activation and Narrator check; the broader
M05 attended gates stay open.

Start with priority 1 or 2 above. Nothing on this page authorizes a pull
request, a release, a driver install or a change to the owner's live audio or
saved data; see [AGENTS.md](../../../AGENTS.md) for the approval and evidence
rules.
