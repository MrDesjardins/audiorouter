# M03 — Managed virtual buses and reference routing

Status: portable desired-state and lifecycle foundation implemented; managed virtual-driver provisioning, native endpoint identity, and live routing remain open. Prerequisites: M02 and accepted M00 driver strategy. Outcome: Discord and a game recorder can select independent AudioRouter inputs.

## Read first

[Workflows](../spec/02-workflows.md), [graph](../spec/04-graph.md), [capture](../spec/05-windows-capture.md), [virtual devices](../spec/06-virtual-devices.md), [security](../spec/13-security.md), and [quality](../spec/14-quality.md).

## Ordered implementation

1. Implement/integrate the selected driver package and scoped installer/broker using an authorized Windows test environment. Specify/version the bounded data bridge and ownership lease. Retain a tested uninstall/restore path.
2. Implement bus inventory/create/rename/enable/disable/delete plans and operations. Expose real endpoint IDs, capabilities, privilege requirements, client impacts, and restart requirements through API/CLI.
3. Implement Virtual Render Source and Virtual Capture Sink, initialized silence, ownership reset, multiple external consumers, and explicit pass-through templates.
4. Extend global topology validation across sessions, bus boundaries, endpoint loopback, and known external application selections. Reject proven cycles and conflicting capture writers.
5. Create CLI fixtures for UC-01/04/05 using Desktop In, Voice Chat, Game Recording, explicit mixers, and headphones. Document Windows/Discord/OBS device-selection steps and duplicate-audio checks.
6. Test reboot identity, UI/backend absence, backend crash, user switching, bus disable/delete while referenced, and simultaneous clients. Add redacted diagnostics for driver/bridge mismatch.

## Acceptance gate

VDEV-01–08/10–12 functionality; GRAPH-11 global validation; CAP-09/10 route policy; SEC-08 initial bridge review; NFR-02/10/16 and QUAL-01 have evidence. UC-01 passes routing isolation before effects are added. Discord receives mic only; Game Recording receives desktop only by default; headphones get one desktop copy.

AudioRouter manages at least three buses and demonstrates the declared eight-bus capacity. A test-signed build may pass the development milestone only on an identified test system; VDEV-09 production signing remains an explicitly open M08 gate. A manually installed cable with no managed lifecycle cannot pass this milestone.

## Verification

Capture distinct tone fixtures from Discord-selected and OBS-selected endpoints; retain signal analysis and app versions. Kill the backend and measure time to silence. Reboot and compare endpoint identities. Render into a bus without a running graph and verify bounded memory/no stale playback. Verify another Windows user cannot seize an active owner's bridge or read retained buffered audio.

## Boundaries and rollback

No assumption that naming an endpoint forces an app to select it. No disabling Windows driver security on an ordinary user's machine. Driver install/update is a separate side-effect operation with exact package IDs and rollback; graph undo does not uninstall a driver.

## Handoff

Publish working API/CLI recipes, endpoint/channel naming conventions, driver version protocol, signed-release dependencies, and feedback/duplicate detection limitations. M04 adds effects/recorders to the proven primary route.

Suggested request: “Implement M03's managed virtual buses and verify the separate Discord/desktop/headphone routing on Windows, with explicit development-versus-production driver status.”
