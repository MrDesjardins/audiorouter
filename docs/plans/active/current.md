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
([evidence](evidence/2026-11-27-recovery-state-consistency.md)). Physical-device
qualification remains a separate short attended gate. Prior frontend work is kept
on `improve/ui-accessibility-app-qualification` and is not part of this branch.

1. **Owner gates.** Take any task from [human-gated tasks](human-gated-tasks.md).
   The first virtual-cable task is D1 (sign in to AR-DriverTest on a quiet PC).
2. **CABLE-A continuity (ARCH-08, CAP-04, NFR-03).** Failed 3 of 6 runs on
   2026-11-24 with unproven isolation. An agent repeats it once the owner gives a
   quiet window (task C1). Plan: [code-review fixes](code-review-fixes.md).
3. **Ship the API-07 fix** in the next build; publication is an owner decision (E1).
4. **Unmerged UI work.** `improve/ui-spacing-errors` (`d61bfb7d4`) holds spacing
   audits and plain-language error work that never reached `main`. Port it as its
   own change if the owner agrees.
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
- [One driver, signed once](driver-sign-once.md): cables, performance and stability before signing (accepted 2026-11-25, freeze 2027-01-24);
  its [W0 inventory](driver-irreversibility-inventory.md) is approved (driver edits still wait for D2), hours are collected with the [driver soak](../../operations/virtual-cable-soak.md)
- [Code-review fixes](code-review-fixes.md)
- Evidence for active and archived plans stays in `evidence/`, because many records
  link to it.

The [future-plan index](../future/README.md) is deferred backlog, not an implicit
queue. Archived plans: [archive index](../archived/README.md).

## Next action

Wait for an owner gate, or do priority 2 or 4 when the owner allows it. Nothing on
this page authorizes a pull request, a release, a driver install or a change to the
owner's live audio or saved data; see [AGENTS.md](../../../AGENTS.md).
