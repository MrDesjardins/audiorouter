# M03 — Managed virtual buses and reference routing

Status: VB-Cable-first existing-device routing, native endpoint identity, and
multi-input/many-output lifecycle are implemented and guarded-qualified.
AudioRouter-owned virtual-driver provisioning, PortCls transport, production
signing, and clean-machine installation are deferred to the future driver
track. Prerequisites for the current track: M02 and an installed supported
VB-Cable/physical/Voicemeeter endpoint. Outcome: supported applications and
tools can use existing endpoint routes without requiring AudioRouter to create
a kernel device.

## Read first

[Workflows](../spec/02-workflows.md), [graph](../spec/04-graph.md), [capture](../spec/05-windows-capture.md), [virtual devices](../spec/06-virtual-devices.md), [security](../spec/13-security.md), and [quality](../spec/14-quality.md).

## Current VB-Cable-first track

The active delivery scope uses existing VB-Cable, Voicemeeter, physical
WASAPI, and other already-installed virtual endpoints as external I/O. Current
evidence covers endpoint identity binding, application capture,
multi-input/many-output routing, recorder/tool taps, lifecycle, cleanup, and
bounded failure behavior. These routes must not change defaults, volume, mute,
privacy, startup registration, or persistent machine audio configuration. The
managed virtual-device steps below remain a future signed-driver track and are
not required for current non-driver completion.

## Ordered implementation

1. **Deferred driver track:** implement/integrate the selected driver package
   and scoped installer/broker only after an authorized signing and Windows
   qualification environment is available. Specify/version the bounded data
   bridge and ownership lease, with a tested uninstall/restore path.
2. Implement bus inventory/create/rename/enable/disable/delete plans and operations. Expose real endpoint IDs, capabilities, privilege requirements, client impacts, and restart requirements through API/CLI.
3. Implement and qualify existing-device render/capture source and sink
   adapters, initialized silence, ownership reset, multiple consumers, and
   explicit pass-through templates without creating a kernel device.
4. Extend global topology validation across sessions, bus boundaries, endpoint loopback, and known external application selections. Reject proven cycles and conflicting capture writers.
5. Create CLI fixtures for current VB-Cable/Voicemeeter/physical workflows,
   including desktop, voice-chat, game-recording, explicit mixers, and
   headphones. Document Windows/Discord/OBS device-selection steps and
   duplicate-audio checks.
6. Test endpoint identity, backend crash, user-mode cleanup, route disable/
   delete while referenced, and simultaneous clients. Add redacted diagnostics
   for endpoint/bridge mismatch. Reboot identity and cross-user managed-bridge
   tests remain future signed-driver prerequisites.

## Acceptance gate

For the current VB-Cable-first track, acceptance is limited to GRAPH-11,
CAP-09/10, existing-endpoint identity and lifecycle, multi-input/many-output
routing, tool/recorder taps, and bounded security/cleanup behavior covered by
the M02, M03, and M07 evidence. Existing VB-Cable or another installed
third-party endpoint is valid external I/O and must not be described as an
AudioRouter-managed bus. The managed VDEV and signing language below belongs
to the deferred future driver track.

VDEV-01–08/10–12 functionality; GRAPH-11 global validation; CAP-09/10 route policy; SEC-08 initial bridge review; NFR-02/10/16 and QUAL-01 have evidence. UC-01 passes routing isolation before effects are added. Discord receives mic only; Game Recording receives desktop only by default; headphones get one desktop copy.

The future managed-driver profile must manage at least three buses and
demonstrate the declared eight-bus capacity. A test-signed build may pass
that development gate only on an identified test system; VDEV-09 production
signing remains an explicitly open M08 gate. This condition does not apply to
the current VB-Cable-first profile, where installed third-party endpoints are
supported external I/O and no managed lifecycle is claimed.

## Verification

Capture distinct tone fixtures from Discord-selected and OBS-selected endpoints; retain signal analysis and app versions. Kill the backend and measure time to silence. Reboot and compare endpoint identities. Render into a bus without a running graph and verify bounded memory/no stale playback. Verify another Windows user cannot seize an active owner's bridge or read retained buffered audio.

## Boundaries and rollback

No assumption that naming an endpoint forces an app to select it. No disabling Windows driver security on an ordinary user's machine. Driver install/update is a separate side-effect operation with exact package IDs and rollback; graph undo does not uninstall a driver.

## Handoff

Publish working API/CLI recipes, endpoint/channel naming conventions, existing
device selection and cleanup guidance, and feedback/duplicate detection
limitations. Keep driver version protocol and signed-release dependencies in
the future driver track. M04 adds effects/recorders to the proven primary
route.

The historical managed-driver request below is superseded for the current
delivery track. Current work verifies existing VB-Cable/Voicemeeter and
physical endpoint routing without AudioRouter-owned driver installation;
managed virtual buses remain deferred until signed-driver prerequisites exist.

Suggested request: “Implement M03's managed virtual buses and verify the separate Discord/desktop/headphone routing on Windows, with explicit development-versus-production driver status.”
