# M07 — MCP parity, persistence, and recovery hardening

Status: MCP, persistence, and recovery foundation implemented; background lifecycle and native restart acceptance remain open. Prerequisite: M06. Outcome: external assistants and people can safely share configuration, and approved sessions survive ordinary lifecycle events.

## Read first

[API](../spec/10-api.md), [automation](../spec/11-automation.md), [persistence](../spec/12-persistence.md), [security](../spec/13-security.md), [quality](../spec/14-quality.md), and UC-06/07/09/10 in [workflows](../spec/02-workflows.md).

## Ordered implementation

1. Audit API/CLI coverage against every implemented UI action. Close missing method/schema/permission/error gaps. Generate method/node references and execute all published command examples.
2. Pin the current supported MCP SDK/protocol from official documentation. Implement stdio server, capability/schema resources, focused tools, validated generic dispatch, permission enrollment, cancellation, and operation-status handling.
3. Harden multi-client conflict/undo/idempotency, event replay/resync/epochs, bounded subscriptions, backpressure, and reconnect UI. Test simultaneous UI/CLI/MCP edits.
4. Implement opt-in sign-in startup, tray-visible background state, recorder-autostart policy, persistent privacy mute, graceful stop/sign-out, sleep/resume, and single-owner virtual bridge recovery.
5. Complete crash journal, backups/migrations, missing-plugin/device restoration, operation recovery, recorder partial-file recovery, and safe mode after repeated crashes.
6. Audit pipe/shell/grants/file/plugin/driver trust boundaries. Fuzz hostile inputs and prove read-only/denied clients cannot invoke write effects through generic dispatch, import, or undo.
7. Produce user-facing headless/MCP setup docs using generic stdio launch configuration, plus tested-client-specific instructions verified against official client documentation at that time.

## Acceptance gate

AUTO-06–12 and full AUTO-01–05 parity; API-01–12 hardening; STATE-08–12 and earlier durability requirements; CAP-11/12 lifecycle; SEC-01–12 relevant integrated tests; NFR-07/09–11/14–16 lifecycle portions. UC-06/07/09/10 pass with recorded outcomes.

The same action from UI/CLI/MCP produces the same canonical graph and errors. A revoked MCP client cannot record or edit. Another client's gain edit is not overwritten by an assistant's stale plan. Closing all clients leaves approved sessions running. Reboot/sign-in restores only authorized sessions, recorders remain governed by separate consent, and privacy mute remains latched. Driver endpoints are silent while the backend is unavailable.

## Verification

Run three-client concurrency, lost replies, repeated idempotency keys, server restart/event-cursor loss, slow subscribers, schema upgrade/downgrade, corrupted storage copy, crash-loop safe mode, process/app restart, and cross-user cases. Run the 24-hour W1/8-hour W2 lifecycle tests and 100 reconnect/suspend cycles under the agreed protocol. Do not claim Windows evidence from a simulated clock alone.

## Boundaries and rollback

No cloud model integration, provider account storage, remote HTTP/MCP control, arbitrary shell execution, or recording audio sent to LLM tools. A stdio MCP client runs locally within enrolled permissions. Keep migrations/backups and startup registration reversible; never silently restore expired/revoked grants.

## Handoff

Deliver the API parity checklist, tested MCP client/protocol versions, migration/recovery runbooks, security results, endurance data, and unresolved release risks. M08 should package and verify a complete implementation, not discover missing core features.

Suggested request: “Implement M07's MCP adapter and complete API parity, concurrency, background startup, crash recovery, and permission tests; prove headless and UI workflows agree.”
