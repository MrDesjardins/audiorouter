# M02 realtime engine core groundwork

## 2026-09-05 — Preallocated audio blocks

Added `crates/engine` with the M02 internal representation constants: 48 kHz planar float32 audio, a maximum two channels, and a 128-frame processing quantum. `AudioBlock` allocates only during preparation and reuses its channel-major storage for clear, copy, gain, mix, explicit mono/stereo channel matrices, bounded linear sample-rate conversion, finite-value sanitization, and shape checks. `DriftController` applies bounded FIFO-occupancy correction in ppm. `RuntimeGraph` holds an immutable prepared gain/mute schedule and applies it without allocating; `RuntimeGeneration` provides an opaque generation identity for later publication/reclamation logic.

The engine now exposes `compile_session`, which validates a domain session and prepares the supported processing-only subset in deterministic topological order. Enabled edge routing is rejected explicitly until the scheduler can transfer blocks between nodes; the compiler therefore cannot silently claim that physical or application audio is routed. The compiler test confirms generation propagation and mute execution without opening any device.

`RuntimePublication` adds the immutable-generation handoff: a fully prepared graph is atomically published, new readers see the replacement, and existing readers may finish on the prior generation before deferred reclamation. The replacement test confirms both generations remain valid across publication.

Optional `CallbackMetrics` records processed quanta and repaired sample counts using relaxed atomics. Instrumented processing remains bounded and free of locks, logging, I/O, and allocation; twelve engine tests pass.

`GainRamp` provides a preallocated per-frame transition for de-clicked gain and privacy-mute changes. It clamps invalid targets to silence, supports immediate changes, and reaches its target exactly at the configured boundary. The engine suite now contains 13 passing tests.

`PrivacyMute` adds an atomic process-local silence gate checked at block boundaries. The test verifies mute and unmute behavior; persistent latching, restart recovery, and actual Windows microphone privacy behavior remain outside this portable engine slice. The engine suite now contains 14 passing tests.

`RuntimeProcessor` integrates the prepared publication slot, safe silence before activation, callback metrics, and privacy mute into one bounded processing boundary. The integration test confirms generation reporting and silence policy; physical endpoint scheduling and durable privacy-latch recovery remain open. The engine suite now contains 15 passing tests.

Privacy mute is applied before the published graph runs, preventing muted physical-capture samples from reaching processor stages. This remains process-local and does not disable other applications' direct microphone access.

`AudioBlockQueue::drain` now provides explicit stop/reconnect cleanup so pending blocks are discarded rather than replayed into a new generation. The queue test confirms intentional draining leaves underrun counters unchanged; 20 engine tests and strict engine Clippy pass.

`AudioBlockQueue::new_for_shape` can enforce a fixed channel/frame shape at the queue boundary. Mismatched blocks are returned immediately and counted as invalid rather than being processed; the engine suite contains 21 passing tests and strict engine Clippy passes.

`AudioBlock::mix_mapped_from` adds destination-major matrix accumulation for explicit fan-out and mixer inputs without overwriting existing destination samples or allocating. Its mono-to-stereo accumulation test passes; node-level scheduling and mixer parameter semantics remain open. The engine suite now contains 16 passing tests.

`AudioBlock::clamp_unit` and `peak_abs` provide explicit output-boundary clipping and peak-meter primitives. Clipping is counted while non-finite values become silence, and internal processing is not implicitly clamped so mixer headroom is preserved. The engine suite now contains 17 passing tests.

`CallbackMetrics` now exposes atomic clipping and xrun counters in addition to processed-quantum and repaired-sample counts. Tests verify caller recording; hardware xrun detection and latency evidence remain Windows scheduler work.

`BlockMeter` adds lock-free peak and over-range clipping observation with reset semantics for a future Meter node. Its test passes without allocation, logging, or device access; node-level wiring and health API publication remain open. The engine suite now contains 18 passing tests.

`AudioBlockQueue` adds a fixed-capacity lock-free queue for preallocated `AudioBlock` values. It explicitly reports full/empty states rather than blocking or dropping silently; queue sizing and recorder-specific overflow policy remain scheduler/recorder work. The engine suite now contains 19 passing tests.

Queue full/empty events now increment atomic overrun/underrun counters while still returning immediately. The queue test verifies both counters; correlation with native device callbacks and external health publication remain open.

`MixerStage` now prepares a bounded set of finite destination-major matrices off the realtime boundary and converges multiple caller-owned input blocks into a preallocated destination. It validates input count and shapes before clearing or mutating output. Two regressions cover successful two-input convergence and rejection without output mutation; the engine suite now contains 30 passing tests. Full domain graph compilation and native scheduling remain open.

`compile_mixer_session` now prepares a narrow domain-to-engine topology with two or more enabled sources, one mixer, and one output edge. `CompiledMixerGraph::process` routes source blocks through preallocated mixer scratch and output blocks without allocation at the processing boundary. A compiler/runtime regression proves two-source convergence and generation propagation; the engine suite contains 31 passing tests with strict Clippy. Fan-out, arbitrary multi-stage scheduling, endpoint resources, and native timing remain open.

The mixer compiler now rejects enabled edges outside that exact topology, duplicate source participation, and destinations that are not physical outputs. These checks prevent silently ignoring graph branches during preparation; the 31-test engine suite and strict Clippy remain green.

`MixerStage` now enforces the specification's maximum of eight converging inputs during preparation and returns an explicit `InputLimit` error above that bound. The regression suite now contains 32 passing engine tests with strict Clippy.

`compile_fanout_session` and `CompiledFanoutGraph` now support the narrow bounded topology of one source to two through eight physical outputs. Each branch owns an independent validated channel matrix and caller-provided destination block; the existing generic single-block compiler still rejects fan-out rather than serializing it. The two-sink regression now proves both branches execute, with 32 engine tests and strict Clippy green.

Prepared mixer and fan-out stages now sanitize non-finite samples at their output boundaries, including direct stage use outside `RuntimeGraph`. A regression covers NaN/Inf from both paths; the engine suite contains 33 passing tests with strict Clippy.

`CompiledFanoutGraph::process` now preflights every destination's frame count, channel count, and matrix shape before mutating any branch. A regression verifies that a later invalid destination leaves an earlier destination unchanged; the 33-test engine suite and strict Clippy remain green.

Prepared mixer construction now applies the domain contract's finite coefficient range of -2 through +2, rejecting NaN/Inf and out-of-range values before runtime use. The engine suite contains 34 passing tests with strict Clippy.

`AudioBlock` now exposes per-channel sample peak, per-channel RMS, and aggregate RMS without allocation. Non-finite samples are excluded from meter calculations and invalid channels return `None`; the engine suite contains 20 passing tests and strict engine Clippy passes.

`RmsWindow` adds a preallocated rolling RMS window with explicit capacity and reset behavior. It treats non-finite input as silence and performs no allocation while pushing blocks; the engine suite contains 21 passing tests and strict engine Clippy passes.

`RuntimeProcessor` now observes active processed blocks through `BlockMeter`, exposing peak and clipping health without changing the no-graph silence behavior. The integration test verifies the meter sees processed output; per-node API publication remains open.

`RuntimeGraph` now prepares lock-free `BlockMeter` instances for enabled Meter
stages and exposes them through the retained graph snapshot. Meter stages
observe the block at their exact processing boundary without allocating or
locking; a two-boundary regression verifies distinct pre- and post-gain peaks
and clipping counts. Native scheduler/API transport publication remains open.

The graph now also exposes a copyable `BlockMeterSnapshot`, so control-facing
code need not depend on the atomic meter storage representation. Snapshot reads
remain lock-free and bounded; transport publication and reset authorization
remain outside this portable engine slice.

Graph meters now expose explicit reset semantics, and `RuntimeProcessor::publish`
clears a graph's meter history at each activation boundary. This prevents
readings from a prior generation being reported after reuse; the reset remains
process-local until a live scheduler/API authorizes its publication.

## 2026-09-06 — Graph processor parameter integration

The domain `Node` contract now carries a backward-compatible parameter map.
Validation accepts only bounded `gainDb` (−60 to +24 dB) for Gain and boolean `muted` for Mute,
rejecting unknown or invalid processor parameters. `compile_session` consumes
those values when preparing runtime stages, defaulting Gain to 0 dB and Mute to
muted when omitted. Domain and engine tests cover validation and a compiled
−6 dB Gain path; this remains portable preparation, not native scheduling.

The original nine-test baseline is retained in the history above; the current engine suite contains 21 deterministic tests. The portable crate now covers preallocated blocks, explicit mapping/accumulation, bounded queues, rolling and instantaneous metering, resampling/drift primitives, de-click ramps, privacy gating, domain preparation, immutable publication, and callback instrumentation. It still does not claim end-to-end node buffer scheduling, physical endpoint routing, latency evidence, or driver behavior.

`RuntimePublication::clear` and `RuntimeProcessor::deactivate` now provide explicit stop behavior: future readers receive no active graph and are silenced, while existing retained snapshots remain safe until released. The lifecycle test passes without device access.

`RuntimeProcessor::process_queued` connects the bounded block queue to processing for control/worker use: it consumes one block without waiting and clears output on empty queues or shape errors. Because popping an owned block may reclaim its backing allocation on drop, it is explicitly not a realtime callback API; a reusable block pool/ring is still required for native scheduler wiring.

`AudioBlockPool` now allocates all fixed-shape block storage during construction and exposes nonblocking acquire/release operations. The pool test verifies capacity, recycling, exhaustion, and rejection of a mismatched shape. This makes the ownership requirement concrete for a future callback ring, but no native scheduler uses the pool yet and callers must preserve the recycle path to avoid deallocation on the realtime thread.

`AudioBlockRing` pairs that pool with a fixed-shape ready queue. A producer acquires and submits a block, a consumer receives it, and the consumer explicitly recycles it; full submission returns the block and increments an overrun counter, while empty receive increments an underrun counter. This is a portable ownership primitive, not evidence of native callback timing, endpoint routing, or physical latency.

`RuntimeProcessor::process_ring_once` now demonstrates pooled transfer from an input ring through the prepared graph into destination-owned output storage. Input and output blocks are recycled independently; output-pool starvation is recorded as an xrun. The API intentionally documents pool membership as a caller invariant: fixed shape alone cannot prove that an externally supplied block came from the preallocated pool.

The compiler now accepts a narrow linear topology: every enabled node has at most one incoming and outgoing edge, all participating ports have the same mono/stereo channel count, and each edge matrix is applied in place before the destination node stage. A 0.5 mono route test passes. Branches, mixer fan-in, disabled-node semantics, endpoint resources, and native scheduling remain explicitly outside this subset.

The negative topology coverage includes a valid source-to-two-sinks fan-out and verifies explicit `UnsupportedTopology` rejection. This prevents a branch from being incorrectly serialized through one block; branch buffers and fan-out scheduling remain required for full GRAPH-02 behavior.

Pool release now clears the block before returning it to the free set. The test verifies that samples written before release are silent on the next acquire, closing a stale-audio ownership hazard without adding work to the allocation path.

The ring-processing tests also cover output-pool starvation: the input block returns to its free pool, no output is published, and one xrun is recorded. No fallback allocation or silent ownership loss is used.

The linear compiler now honors bypass for compatible processing nodes (`Gain`, `Mute`, and `Meter`) by preserving the surrounding channel-matrix path and omitting the processor stage. A regression proves a bypassed gain passes dry samples unchanged; bypass on device-bound nodes remains rejected. The engine suite passes 35 tests with strict Clippy.

Disabled device-bound nodes in the supported linear subset now insert an explicit silence stage, while disabled compatible processors retain their defined dry bypass. The regression covers both disabled source and disabled sink paths and confirms zero output without allocation or device access; the 35-test engine suite and strict Clippy remain green. Native endpoint lifecycle and disabled-recorder finalization are still open.

`RuntimeProcessor::meter_snapshot` now reads a prepared node meter from the
currently published immutable graph snapshot, returning `None` when the graph
is inactive or the index was not prepared. The processor lifecycle regression
covers publication, per-node readout, missing indices, and deactivation. This
completes the portable publication boundary; native scheduler/control API
transport and physical endpoint telemetry remain open.
The engine now provides preparation-time `calculate_latency_compensation`.
It computes destination-major delay samples from cumulative path latency,
validates an 8--192 kHz graph rate, and rejects branch spreads beyond the
declared 250 ms extra-compensation budget. A 37-test engine suite and strict
Clippy pass. This calculates a plan only; runtime delay insertion and native
latency measurement remain open.

`FixedDelay` now applies those prepared sample delays through a preallocated
per-channel ring directly on `AudioBlock`, with reset and shape/bound checks.
The engine suite has 38 passing tests with strict Clippy clean. This proves the
portable delay stage; scheduler graph insertion and physical latency evidence
remain open.

The compiler now fails closed for a multi-node session with no enabled edges.
Without this guard, disconnected processing nodes could be interpreted as one
serial block pipeline. A regression covers the typed `UnsupportedTopology`
result; the engine suite passes 41 tests. This is portable graph-safety
evidence only, and native scheduling/routing remain open.

`RealtimeScheduler` now owns fixed-shape input and output rings around the
published `RuntimeProcessor`. Its acquire/submit and receive/recycle operations
are nonblocking, and `process_once` delegates one bounded graph step without
opening an audio endpoint or allocating at the processing boundary. A regression
verifies generation publication, processed samples, and pooled ownership; the
engine suite passes 42 tests with strict Clippy clean. This is the portable
ownership/scheduling boundary only: native WASAPI activation, endpoint routing,
hardware timing, and driver behavior remain open.

The engine also exposes exact-shape, allocation-free conversion between its
planar `f32` blocks and interleaved `f32` buffers. Round-trip and mismatch
regressions pass, providing the format bridge required before a future WASAPI
adapter can feed the scheduler; endpoint sample-format negotiation and live
stream integration remain open.

## Scheduler output generation propagation (2026-09-07)

Processed scheduler outputs now carry the generation of the immutable graph
that produced them, rather than inheriting an unclaimed input tag. The
scheduler also exposes a generation-filtered output receive operation that
recycles older outputs at the boundary. A regression verifies the propagated
generation; the engine suite and strict Clippy pass. Native endpoint
scheduling and live routing remain open.

The scheduler now clears the ownership tag on silent no-graph outputs and
filters replaced-generation outputs before exposing them. Regression coverage
verifies both behaviors; the engine suite passes 50 tests with strict Clippy.
Native endpoint scheduling and live routing remain open.

Portable route inspection now includes accumulated path latency. It sums the
declared 1,024-sample pitch boundary and configured delay (using the 48 kHz
graph baseline) into each returned path, while native device/plugin latency
measurement remains unclaimed.

## Sustained dual-clock drift simulation (2026-09-07)

The drift controller now includes bounded integral correction in addition to
FIFO-error feedback. A deterministic eight-hour-equivalent simulation at both
`-100 ppm` and `+100 ppm` keeps occupancy away from underflow/overflow and
keeps correction within the configured ±100 ppm bound. The regression exposed
and corrected the proportional-only controller's long-run FIFO drift. This is
simulation evidence only; hardware clock behavior and native scheduling remain
open.

## Bounded queue construction (2026-09-07)

The realtime engine now rejects queue, ring, pool, and scheduler capacities
above `MAX_AUDIO_QUEUE_BLOCKS` (2,048) before constructing lock-free storage.
This closes the public constructor's unbounded-allocation path while retaining
the existing nonblocking behavior for valid capacities. A regression covers
zero, over-limit, and `usize::MAX` requests; the engine suite passed 65 tests,
doc-tests, and strict Clippy. Native callback scheduling remains open.

## Concurrent graph publication (2026-09-07)

The runtime processor now has a regression that publishes two complete graph
generations concurrently with repeated processing. Every observed result must
be a matching generation/gain pair, proving immutable snapshot publication
does not expose a torn graph. This is portable publication evidence only; live
 native scheduler edits remain open.

## Delay reconfiguration safety (2026-09-07)

`FixedDelay::set_delay_frames` now clears the preallocated delay history when
the schedule changes, preventing samples from the old delay configuration from
being replayed after a reconfiguration. A regression verifies the changed
delay starts with silence; the engine suite passes 54 tests with strict
Clippy. Native scheduler reconfiguration remains open.

`FixedDelay` now rejects capacities above 48,000 frames, the declared maximum
250 ms compensation window at 192 kHz, before arithmetic or allocation. This
closes an oversized/overflowing preparation input; native latency measurement
remains open.

## Resampler finite-source safety (2026-09-07)

`AudioBlock::resample_linear_from` now treats non-finite source samples as
silence before interpolation, preventing invalid values from propagating
through the format bridge. Regression coverage verifies finite zero output;
the engine suite passes 55 tests with strict Clippy. Native stream conversion
remains open.

## Endpoint packet-stride contract (2026-09-07)

`EndpointInfo::bytes_per_frame` now derives the interleaved packet stride with
checked arithmetic and rejects zero-channel, zero-bit, or non-byte-aligned
metadata. This gives `SharedCapture::next_packet_into` and
`SharedRender::submit_bytes` a single validated metadata boundary without
opening a stream; the Windows endpoint/runtime gate remains open.

The validated stride is now also included as `format.bytesPerFrame` in the
read-only `devices.list` contract, and malformed metadata fails closed during
discovery. The control and contract schemas are kept in parity; no endpoint
stream is opened.

The drift controller now exposes an explicit reset for stream/reconnect
boundaries, clearing learned integral correction while preserving the nominal
rate ratio and configured bounds. A regression verifies that a new stream does
not inherit prior correction; native device recovery remains open.

## Streaming resampler FIFO (2026-09-07)

`StreamingResampler` now owns a fixed planar FIFO and fractional phase across
source-block calls. It rejects shape and ratio errors before mutation, repairs
non-finite source samples to silence, returns short production explicitly on
underflow, and resets without allocation. Regression coverage verifies samples
continue across source-block boundaries and bounded underflow/shape behavior;
the adapter route is wired to this boundary for differing device rates.

Underflow handling was tightened after route integration review: a partial
destination quantum is now returned as zero with the destination silenced, and
the FIFO/phase remain unchanged for the next source push. This prevents the
fixed-quantum adapter from consuming audio that it cannot publish; the engine
suite passes 62 tests with strict Clippy.

Source admission is transactional under overflow too: when the bounded FIFO
cannot accept the whole source block, `push` returns zero and preserves queued
frames rather than copying a partial block. This keeps caller retry/abort
decisions from inheriting hidden source data; focused engine and probe checks
remain green.

## 2026-09-07 — Full workspace qualification

The current head passed the locked full Rust workspace qualification with 393
unit/integration tests and all doc-tests, followed by strict all-target,
all-feature Clippy with `-D warnings`. The result validates the portable engine
and its cross-crate consumers; native endpoint scheduling, driver lifecycle,
signing, and installer gates remain separate.

## Scheduler-owned graph lifecycle

`RealtimeScheduler` now exposes `activate_session` and `deactivate` wrappers
around its processor publication boundary. Deactivation is tested to consume a
queued block as explicit silence, while invalid activation continues to retain
the previous prepared generation. This keeps graph lifecycle ownership beside
the bounded input/output rings without opening devices; native endpoint
scheduling and stream recovery remain separate gates.

`RealtimeScheduler::telemetry` now provides a point-in-time, allocation-free
snapshot of input/output queue pressure, repaired samples, xruns, processed
quanta, and the active graph generation. Counters remain atomic and monotonic;
the diagnostics caller can diff snapshots without adding work to the realtime
processing boundary. Control/API publication and native callback timing remain
open.

## Bounded application-session metadata (2026-09-07)

Windows audio-session discovery now retains at most 64 display names per
process, rejects names over 256 UTF-8 bytes, and saturates session counters
before the read-only inventory is exposed. The `applications.list` schema
advertises the same bounds (and the fixed 260-character executable limit).
Windows-audio tests (26), control discovery tests (86), full workspace tests,
strict Clippy, and documentation validation pass. Discovery remains read-only:
it opens no audio stream, changes no endpoint, and does not claim process
loopback activation or native realtime routing.

The application inventory itself also advertises and uses a shared 4,096-entry
ceiling. This keeps the read-only process discovery response bounded alongside
the per-process audio-session metadata limits; no stream or endpoint state is
modified.

The shared control event-replay schema is bounded independently of native
audio scheduling; this change does not open endpoints or alter realtime state.

## Route provenance completeness policy (2026-09-07)

Route inspection now enumerates at most 500 paths. It probes one additional
path to distinguish an exact-limit result from truncation, then returns
`complete: false` when more provenance exists. The UI exposes that result as
partial, so consumers do not mistake a bounded response for complete
provenance. Domain/control tests, contracts/UI typechecks, strict Clippy, and
documentation validation pass; native route activation remains open.

## Route inspection field bounds (2026-09-07)

The `routes.inspect` response now advertises the graph-backed bounds for
destination and path entity IDs, per-path nodes and edges, channel-map rows,
and four-coefficient channel matrices with the validated coefficient range.
Control discovery tests (86), strict Clippy, formatting, diff checks, and
documentation validation pass. The path-list count remains intentionally
unbounded until the complete-provenance versus truncation policy is specified;
native route activation remains open.
## Route-output cardinality contract (2026-09-07)

The `routes.inspect` output schema now advertises `MAX_ROUTE_PATHS` (500) for
its `paths` array. This matches the graph inspector's bounded enumeration and
its explicit `complete: false` result when more provenance exists. Control
discovery regression coverage verifies the contract; native route activation
and live audio remain open.

## Quantile-tail regression (2026-09-08)

Added an engine regression proving that a p99.9 histogram upper bound may be
below a rare absolute maximum: 999 samples in bucket 13 and one sample in
bucket 31 return the bucket-13 bound for p99.9. The engine suite passed 76
tests plus doc-tests, strict Clippy, formatting, and diff checks. This anchors
the quantile semantics used by the M02 acceptance wrappers; native callback
timing remains open.
