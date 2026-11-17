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

Start with priority 1 or 2 above. Nothing on this page authorizes a pull
request, a release, a driver install or a change to the owner's live audio or
saved data; see [AGENTS.md](../../../AGENTS.md) for the approval and evidence
rules.
