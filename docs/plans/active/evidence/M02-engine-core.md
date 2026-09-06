# M02 realtime engine core groundwork

## 2026-09-05 — Preallocated audio blocks

Added `crates/engine` with the M02 internal representation constants: 48 kHz planar float32 audio, a maximum two channels, and a 128-frame processing quantum. `AudioBlock` allocates only during preparation and reuses its channel-major storage for clear, copy, gain, mix, explicit mono/stereo channel matrices, bounded linear sample-rate conversion, finite-value sanitization, and shape checks. `DriftController` applies bounded FIFO-occupancy correction in ppm. `RuntimeGraph` holds an immutable prepared gain/mute schedule and applies it without allocating; `RuntimeGeneration` provides an opaque generation identity for later publication/reclamation logic.

The engine now exposes `compile_session`, which validates a domain session and prepares the supported processing-only subset in deterministic topological order. Enabled edge routing is rejected explicitly until the scheduler can transfer blocks between nodes; the compiler therefore cannot silently claim that physical or application audio is routed. The compiler test confirms generation propagation and mute execution without opening any device.

`RuntimePublication` adds the immutable-generation handoff: a fully prepared graph is atomically published, new readers see the replacement, and existing readers may finish on the prior generation before deferred reclamation. The replacement test confirms both generations remain valid across publication.

Optional `CallbackMetrics` records processed quanta and repaired sample counts using relaxed atomics. Instrumented processing remains bounded and free of locks, logging, I/O, and allocation; twelve engine tests pass.

`GainRamp` provides a preallocated per-frame transition for de-clicked gain and privacy-mute changes. It clamps invalid targets to silence, supports immediate changes, and reaches its target exactly at the configured boundary. The engine suite now contains 13 passing tests.

`PrivacyMute` adds an atomic process-local silence gate checked at block boundaries. The test verifies mute and unmute behavior; persistent latching, restart recovery, and actual Windows microphone privacy behavior remain outside this portable engine slice. The engine suite now contains 14 passing tests.

`RuntimeProcessor` integrates the prepared publication slot, safe silence before activation, callback metrics, and privacy mute into one bounded processing boundary. The integration test confirms generation reporting and silence policy; physical endpoint scheduling and durable privacy-latch recovery remain open. The engine suite now contains 15 passing tests.

`AudioBlock::mix_mapped_from` adds destination-major matrix accumulation for explicit fan-out and mixer inputs without overwriting existing destination samples or allocating. Its mono-to-stereo accumulation test passes; node-level scheduling and mixer parameter semantics remain open. The engine suite now contains 16 passing tests.

The nine deterministic unit tests cover gain/mix, shape and bound rejection, NaN/Inf repair, non-finite gain safety, mono/stereo conversion, invalid matrix rejection, linear rate conversion, bounded drift correction, and ordered runtime stages. No Windows API, stream, driver, filesystem, or control-plane operation is performed by this crate. WASAPI event callbacks, graph compilation from domain sessions, cross-block continuity, and live generation publication remain unimplemented.
