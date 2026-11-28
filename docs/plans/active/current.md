# Active plan — current priorities (2026-11-24)

## Where things stand

On 2026-11-24 the owner asked to finish the active plans and archive them. Every
plan whose remaining work needs a person moved to `docs/plans/archived/`; those
gates, including **failed** checks, are in one file:
[human-gated tasks](human-gated-tasks.md). Archiving passed none of them. The
previous version of this page is
[kept as history](../archived/2026-11-24-active-plan-history.md), and older logs
are in [the November 16 history](../archived/2026-11-16-active-plan-history.md).

No feature below is release-ready. The previous app-only candidate is 0.0.32,
unsigned, experimental and unpublished; the virtual-cable driver (RC 0.2.5) is a
separate track whose Stage A is **not passed**. A fresh 0.0.33 app-only candidate
is integrated and locally qualified (unsigned, unpublished) in
[next app-only candidate](app-candidate-integration.md); it is not release-ready.

This change also fixed a defect: after AudioRouter restarts, repeating an old Play
or recording request with the same idempotency key no longer answers "running" for
audio that stopped. It is refused and starts nothing (API-07). The 0.0.32 bytes
predate the fix; the 0.0.33 candidate includes it.

## Current priorities

Owner requested the next non-driver work on 2026-11-27. Backend A5-A9 baseline
investigation is in [recovery-state consistency](recovery-state-consistency.md);
the owner approved all five repairs, now implemented and locally verified
([evidence](evidence/2026-11-27-recovery-state-consistency.md)) and merged in #199.
One attended PD200X capture unplug/recovery cycle passed; broader hardware gates
remain open.

Owner-selected work (2026-11-27): the frontend spacing/error port, accessibility
checks and fresh app-only build qualification are in
[frontend and app qualification](ui-app-qualification.md); #201, #203, #205 and
#206 are merged. Driver work is excluded. The owner's answers to A1–A4 and
A12–A18 are in [owner decisions 2026-11-27](owner-decisions-2026-11-27.md).

1. **Owner gates.** Take any task from [human-gated tasks](human-gated-tasks.md).
   The first virtual-cable task is D1 (sign in to AR-DriverTest on a quiet PC).
2. **CABLE-A continuity (ARCH-08, CAP-04, NFR-03).** Failed 3 of 6 runs on
   2026-11-24 with unproven isolation. An agent repeats it once the owner gives a
   quiet window (task C1). Plan: [code-review fixes](code-review-fixes.md).
3. **API-07 and the 0.0.33 candidate.** The fresh local app-only build includes
   the fix, recording opt-in, `sessions.rename`, the A3 bus-list tools, the
   frontend port and its aa95a6394 spacing fix, recovery repairs, site wording and
   A4 output widening. The 7.1 continuity evidence is merged in #206. It is
   updated to current `main`, qualified with isolated profiles and
   unsigned/unpublished; publication remains an owner decision (E1).
4. **Frontend port and qualification.** The port from `improve/ui-spacing-errors`
   (`d61bfb7d4`) is adapted to current localization and is on `main` through #200
   and #205; the 0.0.33 candidate branch also contains it, with
   browser/native/installer evidence in the [execution plan](ui-app-qualification.md).
   No merge of the candidate PR is authorized. Attended accessibility and the
   deferred soak remain open.
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
- [Next app-only candidate integration](app-candidate-integration.md)
- [One driver, signed once](driver-sign-once.md): cables, performance and stability before signing (accepted 2026-11-25, freeze 2027-01-24);
  its [W0 inventory](driver-irreversibility-inventory.md) is approved (driver edits still wait for D2), hours are collected with the [driver soak](../../operations/virtual-cable-soak.md)
- [Code-review fixes](code-review-fixes.md)
- Evidence for active and archived plans stays in `evidence/`, because many records
  link to it.

The [future-plan index](../future/README.md) is deferred backlog, not an implicit
queue. Archived plans: [archive index](../archived/README.md).

## Next action

Hand the locally qualified 0.0.33 app-only candidate to the owner for attended
testing, or take another owner gate. The installer VM lifecycle, attended
audio/device checks, clean-Defender/MOTW, second-PC and signing remain open; the
7.1 continuity gate passed in #206, while attended 7.1 Play and the application
worker live path remain open, and the frontend thread owns attended
accessibility. The eight-hour soak is deferred by the owner. Nothing on
this page authorizes a merge, a release, a driver install or a change to
the owner's live audio or saved data; see [AGENTS.md](../../../AGENTS.md).
