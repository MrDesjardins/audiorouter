# Active plan — current priorities (2026-11-24)

## Where things stand

On 2026-11-24 the owner asked to finish the active plans and archive them. Every
plan whose remaining work needs a person moved to `docs/plans/archived/`; those
gates, including **failed** checks, are in one file:
[human-gated tasks](human-gated-tasks.md). Archiving passed none of them. The
previous version of this page is
[kept as history](../archived/2026-11-24-active-plan-history.md), and older logs
are in [the November 16 history](../archived/2026-11-16-active-plan-history.md).

No feature below is release-ready. The app-only candidate is 0.0.32, unsigned,
experimental and unpublished; the virtual-cable driver (RC 0.2.5) is a separate
track whose Stage A is **not passed**.

This change also fixed a defect: after AudioRouter restarts, repeating an old Play
or recording request with the same idempotency key no longer answers "running" for
audio that stopped. It is refused and starts nothing (API-07). The 0.0.32 bytes
predate the fix.

## Current priorities

Owner requested the next non-driver work on 2026-11-27. Backend A5-A9 baseline
investigation is in [recovery-state consistency](recovery-state-consistency.md);
the owner approved all five repairs, now implemented and locally verified
([evidence](evidence/2026-11-27-recovery-state-consistency.md)). One attended
PD200X capture unplug/recovery cycle passed; broader hardware gates remain open.

Owner-selected work (2026-11-27): frontend spacing/error port, accessibility
checks and fresh app-only build qualification are executing in
[frontend and app qualification](ui-app-qualification.md). Driver work is excluded.
The owner's answers to A1, A2, A4 and A12–A18 are in
[owner decisions 2026-11-27](owner-decisions-2026-11-27.md).

1. **Owner gates.** Take any task from [human-gated tasks](human-gated-tasks.md).
   The first virtual-cable task is D1 (sign in to AR-DriverTest on a quiet PC).
2. **CABLE-A continuity (ARCH-08, CAP-04, NFR-03).** Failed 3 of 6 runs on
   2026-11-24 with unproven isolation. An agent repeats it once the owner gives a
   quiet window (task C1). Plan: [code-review fixes](code-review-fixes.md).
3. **Ship the API-07 fix.** Included in the fresh local app-only qualification
   build; publication remains an owner decision (E1).
4. **Frontend port and qualification.** The owner approved the port from
   `improve/ui-spacing-errors` (`d61bfb7d4`). It is adapted to current localization
   on `improve/ui-accessibility-app-qualification`, with browser/native/installer
   evidence in the [execution plan](ui-app-qualification.md). It is not merged;
   no PR is authorized. Attended accessibility and the deferred soak remain open.
5. **Nightly upkeep.** Read each failed nightly from its retained evidence; coverage
   rows collect for the A10 and A11 decisions.

6. **Signing: waiting for OSSign until 2027-03-05** (owner decision 2026-11-25, no
   purchase meanwhile): task B1 in [human-gated tasks](human-gated-tasks.md).

## Open owner gates

All owner, hardware, purchase, listening and VM gates are in
[human-gated tasks](human-gated-tasks.md). Mocks and portable tests must not stand
in for them.

## Active plan index

- [Human-gated tasks](human-gated-tasks.md)
- [Frontend and app-only qualification](ui-app-qualification.md)
- [One driver, signed once](driver-sign-once.md): cables, performance and stability before signing (accepted 2026-11-25, freeze 2027-01-24);
  its [W0 inventory](driver-irreversibility-inventory.md) is approved (driver edits still wait for D2), hours are collected with the [driver soak](../../operations/virtual-cable-soak.md)
- [Code-review fixes](code-review-fixes.md)
- Evidence for active and archived plans stays in `evidence/`, because many records
  link to it.

The [future-plan index](../future/README.md) is deferred backlog, not an implicit
queue. Archived plans: [archive index](../archived/README.md).

## Next action

Finish the attended frontend gates from the qualification plan when available,
or take another owner gate. The eight-hour soak is deferred by the owner. Nothing on
this page authorizes a pull request, a release, a driver install or a change to the
owner's live audio or saved data; see [AGENTS.md](../../../AGENTS.md).
