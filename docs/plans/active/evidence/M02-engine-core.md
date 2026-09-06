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

`AudioBlock` now exposes per-channel sample peak, per-channel RMS, and aggregate RMS without allocation. Non-finite samples are excluded from meter calculations and invalid channels return `None`; the engine suite contains 20 passing tests and strict engine Clippy passes.

`RmsWindow` adds a preallocated rolling RMS window with explicit capacity and reset behavior. It treats non-finite input as silence and performs no allocation while pushing blocks; the engine suite contains 21 passing tests and strict engine Clippy passes.

`RuntimeProcessor` now observes active processed blocks through `BlockMeter`, exposing peak and clipping health without changing the no-graph silence behavior. The integration test verifies the meter sees processed output; per-node API publication remains open.

The original nine-test baseline is retained in the history above; the current engine suite contains 21 deterministic tests. The portable crate now covers preallocated blocks, explicit mapping/accumulation, bounded queues, rolling and instantaneous metering, resampling/drift primitives, de-click ramps, privacy gating, domain preparation, immutable publication, and callback instrumentation. It still does not claim end-to-end node buffer scheduling, physical endpoint routing, latency evidence, or driver behavior.

`RuntimePublication::clear` and `RuntimeProcessor::deactivate` now provide explicit stop behavior: future readers receive no active graph and are silenced, while existing retained snapshots remain safe until released. The lifecycle test passes without device access.

`RuntimeProcessor::process_queued` connects the bounded block queue to processing: it consumes one block without waiting, clears output on empty queues or shape errors, and processes only caller-owned preallocated storage. The integration test passes; native capture/render scheduler wiring remains open.
