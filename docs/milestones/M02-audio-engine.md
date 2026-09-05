# M02 — Realtime engine and Windows capture/output

Status: not started. Prerequisite: M01. Outcome: real mic/app/endpoint audio routed through a backend-owned graph without a UI.

## Read first

[Architecture](../spec/03-architecture.md), [graph](../spec/04-graph.md), [capture](../spec/05-windows-capture.md), [API](../spec/10-api.md), and [quality](../spec/14-quality.md), plus M00 measurements.

## Ordered implementation

1. Implement endpoint enumeration/notifications, bindings, COM lifetimes, permissions, event-driven capture/render, and shared-mode period negotiation in the Windows adapter.
2. Implement preallocated audio buffers, bounded event/data queues, generation publication, deferred resource reclamation, and callback instrumentation. Keep async control and storage outside the audio deadline.
3. Implement internal 48 kHz audio with channel conversion, resampling, clock/timeline selection, bounded drift correction, and output fan-out. Use agreed M00 processing quantum.
4. Add Physical Input/Output, Application Capture, Endpoint Loopback, Mixer, Gain/Mute, and Meter nodes. Add process identity rebinding and explicit include/exclude capability reporting.
5. Connect plan/commit to resource preparation and runtime activation. Retain old graph on failure. Add de-click ramps, privacy mute, health events, and API/CLI diagnostics.
6. Validate endpoint recapture feedback, capture deduplication, resource ownership, topology limits, unplug/default-change behavior, and stopped-app silence.

## Acceptance gate

ARCH-04–09/11; live GRAPH-01–09/11–13; CAP-01–08/11 basic recovery; NFR-01/03/04/09/13 baseline; QUAL-01/02/04/05 have real and simulated evidence as applicable. Windows include/exclude behavior matches the advertised source scope.

Route a microphone to two physical outputs and a selected app to one output using CLI. Show channel mapping and levels. Run 8-hour dual-clock tests and simulated ±100 ppm mismatch. Reconfigure gain/topology under a steady signal; no torn graph or unexplained discontinuity occurs. Repeated source branches open a single compatible capture stream. Missing pinned mic never falls back to another microphone. Attempted render-loopback feedback is rejected.

## Verification

Publish raw physical latency distributions, callback timing, xrun/queue/drift counters, OS/device manifest, capture correctness results, and failure traces. Explain any reference-budget miss. Deterministic tests validate compiler/signal math; Windows hardware tests validate actual audio. Exercise device invalidation and Windows microphone denial.

## Boundaries and recovery

No new virtual-driver implementation belongs in the realtime callback. Interim cable experiments remain labeled as such. Do not tune the user's global Windows audio settings invisibly. Stop/release all test streams and restore any explicitly changed device defaults after experiments.

## Handoff

Document adapter ownership/unsafe invariants, supported formats/periods, measured delay budget, resource limits, and remaining device-specific limitations. M03 can then add managed buses without rewriting the engine/control contract.

Suggested request: “Implement M02 and prove real Windows mic/app routing, bounded clock correction, atomic live edits, and measured latency from the existing CLI.”
