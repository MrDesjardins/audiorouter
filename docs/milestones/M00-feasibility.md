# M00 — Windows feasibility and architecture decisions

Status: evidence collected; feasibility gate remains open. Prerequisite: specification baseline. Outcome: evidence-backed implementation choices before a large application is generated.

## Read first

Read [product](../spec/01-product.md), [architecture](../spec/03-architecture.md), [capture](../spec/05-windows-capture.md), [virtual devices](../spec/06-virtual-devices.md), [quality](../spec/14-quality.md), and the [decision/risk register](../spec/15-delivery.md). Follow [AGENTS.md](../../AGENTS.md).

## Deliverables and ordered tasks

1. Inventory Windows test availability, compiler/SDK/WDK requirements, physical audio hardware, and any existing virtual driver. Record exact versions and what is missing. Select reference machines and a reproducible tone/impulse harness. Do not imply the current non-Windows workspace validates audio.
2. Build a minimal Windows-only capture/render probe: enumerate endpoints, select an exact mic/output, stream shared-mode audio, query formats/periods, and measure loopback latency. Test 44.1/48 kHz boundaries and two independent devices. No full visual application is needed.
3. Probe process-tree capture with two tone-producing apps, a browser, and Discord where available. Test restart/PID reuse, include/exclude mode, capture with ordinary playback, and what happens when the Windows audio session is muted. Record protected/unsupported cases.
4. Prototype the virtual data path and bus lifecycle using the prospective driver or a clearly labeled interim cable. Demonstrate external render → engine and engine → external capture; enumerate stable identities, no-backend silence, and simultaneous consumers. Identify the exact integration interface and privilege boundary.
5. Select vendor versus project driver with a concrete decision: rights to redistribute, source/maintenance ownership, eight-bus feasibility, endpoint rename/restart semantics, driver bridge, Secure Boot/HVCI, signing steps, and unresolved credentials/costs. A sample-driver link alone does not pass this task.
6. Evaluate Rust Windows bindings and minimal Tauri/WebView2 packaging; confirm a small C++ boundary is sufficient where needed. Pin toolchain candidates and dependency/license inventory. Revisit 128-frame quantum using measurements.
7. Update DEC-03/06/07 and any disproved assumptions in specs. Produce the M00 execution report and next milestone plan.

## Acceptance gate

CAP-01–08 and ARCH-05/07/08 have prototype evidence adequate to implement, with supported/unsupported behavior distinguished. NFR-01–03 methods and baseline measurements exist. VDEV-02 has a feasible managed-driver design and credible production-signing route; actual production credentials/signature can remain an explicit M08 dependency. The golden UC-01 topology is shown to avoid recapturing its headphone output.

A third-party cable can demonstrate audio flow but cannot satisfy managed provisioning feasibility by itself. If virtual driver creation/signing is not credible, mark that gate blocked and present a concrete decision to the user. Independent domain work may proceed only with this limitation stated; no full-product completion claim is allowed.

## Verification and artifacts

Keep probe source, commands, raw timings, machine manifests, capability results, and architectural decisions in the execution evidence. Record permission denied, unavailable device, no-render-stream silence, and restart cases as well as successful sound. Use native Windows output for exact errors when needed, following the RTK policy.

## Boundaries and rollback

No production driver installation on the user's primary PC, purchases, signing-account enrollment, cloud deployment, or undocumented audio hooks are implied. Use existing authorized test resources; request missing external actions only after concrete findings. Test driver changes need a recovery image/restore/uninstall procedure. Do not change Windows defaults without recording and restoring their exact prior selections.

## Handoff to M01

Archive the completed execution plan with measured results. List selected APIs, stack/toolchain versions, endpoint strategy, resource/latency assumptions, and unresolved release dependencies. M01 must consume these choices rather than repeat the investigation.

Suggested implementation request: “Execute M00, produce Windows feasibility evidence and decisions, update the active plan and specifications, and clearly identify any hardware or signing dependency that prevents the gate from passing.”
