# 04 — Domain model and graph semantics

Milestone ownership: M01 model/compiler contracts; M02 live graph activation; M03 virtual topology; M04/M06 processor semantics.

## Entities

| Entity | Persistent fields | Runtime fields |
| --- | --- | --- |
| Session | UUID, name, schema version, revision, nodes, edges, safety/startup policies | run state, active revision, graph generation, health |
| Node | UUID, type/version, name, enabled, bypass policy, parameters, port definitions | resolved device/plugin, health, latency, meter snapshot |
| Edge | UUID, source/destination node and port, channel matrix, enabled | compiled route, effective latency |
| Device binding | endpoint ID or default-role selector, expected direction, fallback policy | resolved endpoint ID, availability, negotiated format |
| App binding | verified executable/package identity, instance policy, include/exclude mode | PID + creation time, process tree, capture state |
| Virtual bus | UUID, display name, driver endpoint identities, channel count, enabled | owner lease, clients, health, restart requirement |
| Recording | UUID, session/node IDs, file references, format, start frame | bytes, duration, gaps, state, recovery result |
| Preset | UUID, schema/type version, parameter values or subgraph | compatibility findings on import |

IDs are opaque strings, independent of names, array position, PID, or screen coordinates. Timestamps use UTC ISO 8601 for human records and monotonic/sample-clock values for audio ordering. Store gain in dB, time in milliseconds or frames with explicit units, frequency in Hz, and ratios without ambiguous percentage notation.

## Requirements

- **GRAPH-01 — Directed topology.** Audio flows output-port → input-port through a directed acyclic graph. Sidechains are named ports and included in dependency validation. Reject self-links, cycles, dangling references, incompatible directions, and unsupported channel counts with path-specific errors.
- **GRAPH-02 — Explicit mixing.** Fan-out is allowed. An ordinary processor input accepts exactly one edge; summation requires an explicit Mixer node with named inputs, each accepting one edge. The UI may propose inserting a mixer, but must submit that insertion as part of the visible transaction. Duplicate identical edges are rejected.
- **GRAPH-03 — Channels.** v1 supports mono and stereo node ports and explicit matrix coefficients per edge. Mono duplication to stereo is `[1,1]`; stereo-to-mono defaults to `0.5L + 0.5R`. Conversion is shown in connection details. No unannounced channel drop, swap, normalization, or automatic gain change. Matrix coefficients shall be finite and in [-2,2].
- **GRAPH-04 — Mixing gain.** Mixer inputs expose gain, mute, and stereo balance; master gain is separate. Summation is float32 with headroom. Meters flag peaks above 0 dBFS. Templates include a visible final limiter where appropriate; do not hide a nonlinear limiter in every mixer. Boundary conversion clamps unsupported over-range samples as a final safeguard and counts clipping.
- **GRAPH-05 — Disabled nodes.** `enabled=false` preserves saved parameters/edges but produces silence for sources; suppresses writes/playback for sinks; and invokes defined bypass for processors. A disabled mixer produces silence. A disabled recorder stops and finalizes its current file. Display the effective semantic label, not just a generic power icon.
- **GRAPH-06 — Bypass versus mute.** A compatible one-input processor bypass passes dry audio with its declared compensation behavior; a mute passes zero frames and preserves gain. Multichannel/sidechain/plugin types declare their bypass map. If bypass is undefined, reject it with guidance to mute or rewire. Bypassing a processor deliberately is distinct from processor failure.
- **GRAPH-07 — Removal.** Removing a node removes its incident edges in the same transaction; it never invents a reconnect. Offer a separate “remove and reconnect” operation only if the backend can derive one unambiguous compatible mapping and preview it. Deleting an active recorder requires the operation to finalize its file before retiring it; deletion does not delete files.
- **GRAPH-08 — Parameter changes.** Validate types, units, enum members, finite numbers, and ranges in the backend. Smooth gain/filter changes over a type-defined ramp, default 10 ms where applicable. Changes needing rebuild advertise `requiresRecompile`; those needing stop advertise `requiresStop`. Do not accept a value and silently use a different one.
- **GRAPH-09 — Transactions.** Multi-edit changes validate and activate all-or-nothing for the graph. Transactions carry a base revision, idempotency key, canonical diff, and consequences. Compilation failure produces no new graph revision. External side effects use separate operations as defined in [10](10-api.md).
- **GRAPH-10 — Latency compensation.** Calculate cumulative per-path latency and align branches before they converge in a mixer, bounded to 250 ms extra compensation in v1. Reject over-budget alignment with a suggested graph change. Independent output paths do not have to share the largest delay; a low-latency monitor path may omit pitch/lookahead. Dynamic plugin latency changes require recompilation before use.
- **GRAPH-11 — Feedback.** Validate the combined graph across running sessions and managed virtual endpoints, including explicit render-to-capture paths. Reject known software cycles, including loopback of an endpoint to which the graph renders. Default-device rebinding reruns this check. Unknown external apps and acoustic routes cannot be proved safe; show specific risk with an acknowledgment tied to the plan revision, never a false “feedback-proof” claim.
- **GRAPH-12 — Inspection.** The backend shall provide upstream/downstream paths, source provenance, route reachability, channel maps, accumulated latency, conversions, inactive reasons, and effective failure policy for each destination. UI and AI explanations derive from these results, not visual positions.
- **GRAPH-13 — Limits.** Enforce finite per-session and global budgets from [14](14-quality.md) before allocating realtime resources. Reject oversize imports/edits with requested and available limits. Disabled graphs still count against persistence/import limits, but not active DSP CPU budgets.
- **GRAPH-14 — Protected paths.** Virtual microphone sinks are protected by default. When a required upstream processor is missing/faulted, their dependent contribution becomes silence, including any dry path used internally for bypass compensation. User-selected bypass is allowed and visible. Each destination may explicitly choose dry fallback for compatible failed processors; store this choice and show it in preview.

## Node registry minimum

M02: Physical Input, Application Capture, Endpoint Loopback, Physical Output, Mixer, Gain, Mute, Meter. M03: Virtual Render Source, Virtual Capture Sink. M04: Parametric EQ, Graphic EQ, Gate/Expander, Compressor, Limiter, Delay, Recorder. M06: Pitch Shift and VST3 Effect. Template/group/preset are authoring concepts that expand to regular nodes, not opaque code execution.

Each registry entry declares ports, display metadata, version, JSON parameter schema, defaults, units, ranges, bypass/failure behavior, estimated latency, dynamic-latency capability, realtime cost class, and availability reason. A recorder is a sink branch; insert it by splitting the edge, so pause/removal never interrupts downstream processing.

## State transitions

Session: `stopped → preparing → running → stopping → stopped`, with `degraded` for recoverable route failures and `failed` for inability to run. A failure carries affected nodes and recoverability. Starting is idempotent; stopping drains/finalizes sinks subject to timeouts. Runtime `degraded` does not mutate saved topology.

Node: desired enable/bypass/mute fields are separate from observed `ready`, `unavailable`, `faulted`, or `warmingUp`. Status must distinguish intentional silence from absent frames. Metering a bypassed node reports effective output, with optional pre-effect input meter.

## Acceptance examples

A mic split into three branches opens one capture stream. A cycle introduced by a virtual bus is rejected even if edges belong to different sessions. Disabling an EQ passes dry; disabling a mixer silences it. A transaction removing an EQ and adding a compatible replacement appears as one new revision, never an intermediate broken graph. Concurrent changes from base revision N result in one commit and one conflict. Imported missing plugins remain visible and silence protected outputs until resolved or deliberately bypassed.
