# 06 — Persistent virtual audio devices

Milestone ownership: M00 driver decision/prototype; M03 endpoint functionality; M07 ownership/recovery; M08 production signing, installation, and uninstall.

## Endpoint terminology

A virtual bus has a user-facing name and may expose two Windows endpoints:

| Windows endpoint | External application action | Graph node |
| --- | --- | --- |
| Render/playback: `AudioRouter — Desktop In` | Game/browser selects it as output | Virtual Render Source reads app audio |
| Capture/recording: `AudioRouter — Voice Chat` | Discord selects it as microphone | Virtual Capture Sink writes processed audio |

A bus may expose both sides. They are not implicitly connected. A pass-through template adds an explicit source-to-sink route. AudioRouter's driver bridge is distinct from the public endpoints: the engine reads the render stream and supplies the capture stream through a bounded native interface, not by opening a capture endpoint and pretending it can write to it.

Persistent endpoint identity and continuous processed audio are separate properties. Driver presence keeps endpoints enumerated; a running authorized backend supplies their live audio. Silence is the safe default when no owner exists.

## Requirements

- **VDEV-01 — Provisioning.** Create, rename, list, enable, disable, and delete up to eight named buses per installation. v1 buses are stereo endpoints; mono graph streams map explicitly to/from them. Capability discovery returns limits, endpoint IDs, required privilege, affected clients, and any restart/reopen requirement. Do not promise arbitrary dynamic endpoint creation until the driver prototype proves it.
- **VDEV-02 — Driver gate.** M00 shall choose a maintained redistributable driver with documented integration rights, or a project-owned driver based on an appropriate WDK design. Record maintenance ownership, cost/credentials dependencies, security review, supported channels, dynamic provisioning, data bridge, and production signing route. A sample such as SysVAD is a starting reference, not a finished cable driver. [Microsoft SysVAD](https://github.com/microsoft/Windows-driver-samples/tree/main/audio/sysvad).
- **VDEV-03 — Identity.** Bus UUID and Windows endpoint identities survive backend/editor restarts, reboot, normal upgrades, and rename whenever supported. If Windows requires re-enumeration for a change, preview identity impact and require stopped affected routes. Preserve a migration mapping and guide app reselection; never silently assign old identity to unrelated content.
- **VDEV-04 — Persistence.** Enabled endpoints remain enumerable when the editor/backend is closed. With no live writer, capture returns initialized silence, never stale ring-buffer audio. Render accepts/drains or safely drops incoming frames without unbounded memory growth. Driver buffers are reset on owner/session changes.
- **VDEV-05 — Enable versus mute.** Device disable makes the endpoint unavailable to consumers and may invalidate open streams. Sink mute keeps the endpoint available while delivering silence. Node disable does not uninstall or disable the Windows device. The API and UI shall expose these as different operations and show affected external clients.
- **VDEV-06 — Pass-through.** App render audio can be routed to one or multiple capture sinks, physical monitors, processors, and recorders. Bus-level gain is represented in the backend graph; monitoring gain is independent. No invisible monitor or pass-through shall exist outside topology inspection.
- **VDEV-07 — Ownership.** Enforce one backend writer lease per capture endpoint; multiple receiving apps may read it through normal Windows shared audio. Cross-session contributions require an explicit mixer or designated owning session. A second writer gets `resourceConflict`, never an implicit mix. Cross-user ownership is exclusive and data buffers are cleared before transfer.
- **VDEV-08 — Safe control.** Driver install/update/remove uses a scoped elevated component; normal stream routing does not require administrator rights. Provisioning shall return an operation ID and compensating actions if partially completed. Do not bundle driver provisioning into graph atomic-commit claims.
- **VDEV-09 — Production installation.** Public builds require a production-signed driver that works with Secure Boot and Memory Integrity on supported Windows 11 systems. Test-signing is limited to an identified developer test environment. Driver signing/access is an explicit delivery dependency, not something an LLM can manufacture. Recheck current Microsoft requirements before shipping. [Driver signing offerings](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings).
- **VDEV-10 — Deletion.** Preview session references and active clients; refuse deletion while owned/running unless an explicit stop-and-delete plan identifies affected paths. Remove only AudioRouter-owned endpoints/packages. Keep user recordings. Deleting a bus and recreating the same label creates a new identity unless restoring an actual saved bus record through supported migration.
- **VDEV-11 — Compatibility.** Prove use as input in Discord and OBS and output from a browser/game via Windows/app settings. Test simultaneous consumers and apps restricted to mono/stereo. External app noise suppression/AGC may alter audio; document recommended app configuration and test both default and disabled processing.
- **VDEV-12 — Recovery.** Backend/bridge/driver version negotiation fails clearly on incompatible versions. A heartbeat/lease timeout forces silence within the [quality budget](14-quality.md). Restart/reconnect never replays old frames. Repeated bridge failure disables only affected routes and emits diagnostics.

## Interim and final delivery

An already installed third-party virtual cable may support M00 experiments and M02 demos. Its channels, naming, license, and separate installation must be stated. It does not satisfy VDEV-01, VDEV-03, or VDEV-09. M03 must demonstrate AudioRouter-managed bus lifecycle with the selected driver; M08 must demonstrate the distributable signed package. If rights, signing, or driver feasibility cannot be secured, record the blocked gate and propose a concrete revised product scope to the user.

## Test cases

Create three buses; rename one while stopped; reopen Discord; reboot and compare endpoint IDs. Read the capture endpoint before backend startup and verify digital silence. Kill the backend during a tone and confirm silence within 500 ms. Feed render audio with no readers for one hour without growth. Open two recording clients concurrently. Switch users and prove no previous user's buffered audio appears. Attempt cross-session and virtual nesting cycles. Upgrade and roll back the driver without removing unrelated audio devices.
