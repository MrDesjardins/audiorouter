# M06 — Isolated VST3 effects and pitch shift

Status: not started. Prerequisite: M05. Outcome: extensible effects and pitch processing with explicit compatibility, latency, and failure containment.

## Read first

[Architecture](../spec/03-architecture.md), [graph](../spec/04-graph.md), [processing](../spec/07-processing.md), [interface](../spec/09-interface.md), [security](../spec/13-security.md), and [quality](../spec/14-quality.md).

## Ordered implementation

1. Select/pin a supported VST3 SDK/hosting boundary and pitch algorithm/library using recorded license, maintenance, audio-quality, and latency criteria. Verify actual dependencies' terms; do not assume historical SDK terms still apply.
2. Implement disposable scan workers, binary identity/version inspection, directory grants, deadlines, cancellation, and quarantine. Add unsupported-format diagnostics for legacy ReaPlugs/VST2/x86 fixtures where available.
3. Implement per-instance workers with bounded shared-memory audio transport, sequence/deadline checks, channel negotiation, parameter automation, latency reporting, and no synchronous realtime waits.
4. Implement opaque state save/restore, missing-plugin placeholders, generic controls and optional worker-owned native editors. State load failures follow protected-path policy.
5. Implement built-in time-preserving pitch shift and expose semitone/cents parameters, warmup, latency, bypass, and defaults. Preserve independent low-latency monitoring branches.
6. Extend graph compensation/failure propagation, UI/CLI/API schemas, import assets, diagnostics, and W2 workload. Probe plugin CPU overload, hang, crash, NaN, dynamic latency, and editor lifecycle.

## Acceptance gate

DSP-06 and PLUG-01–06; GRAPH-10/14 extended; SEC-07; W2 latency/containment evidence. At least three available x64 VST3 effects from at least two vendors/categories pass parameter/state/editor tests, with exact versions recorded. Include a controlled failure plugin and one dynamic-latency fixture; fixtures need not be commercial products.

An unavailable test plugin is not simulated and labeled compatible. The user's exact ReaPlugs compatibility is stated from binary inspection if supplied; built-in EQ/gate/compression remain available independently. Plugin and pitch latency appears in route inspection. Worker crash does not crash the backend or unrelated branches, and protected voice audio never silently becomes dry after a fault.

## Verification

Check pitch/duration tolerances and speech quality; buffer/deadline and quarantine behavior; worker handles/memory over repeated instances; save/reload fidelity; scanner/editor crashes; 8-hour W2 soak; client parity for every parameter. Measure actual worker-added latency instead of reusing in-process estimates.

## Boundaries and rollback

No VST2/x86 bridge, downloaded plugins, instruments, arbitrary plugin scripts, or unbounded in-process fallbacks. If process sandbox restrictions break a plugin, state the restriction/compatibility result before weakening privileges. Keep crash containment even where full network/filesystem sandboxing is impractical.

## Handoff

Provide tested/unsupported plugin matrix, license notices, worker protocol, state compatibility policy, pitch algorithm/latency evidence, and failure recovery instructions. Update future backlog for legacy plugin requests without promising unverified rights/support.

Suggested request: “Implement M06's isolated VST3 hosting and built-in pitch shift, prove latency/failure behavior, and document actual plugin compatibility including legacy-format limitations.”
