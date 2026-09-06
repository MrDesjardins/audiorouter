# M02 realtime engine core groundwork

## 2026-09-05 — Preallocated audio blocks

Added `crates/engine` with the M02 internal representation constants: 48 kHz planar float32 audio, a maximum two channels, and a 128-frame processing quantum. `AudioBlock` allocates only during preparation and reuses its channel-major storage for clear, copy, gain, mix, explicit mono/stereo channel matrices, finite-value sanitization, and shape checks. `RuntimeGeneration` provides an opaque generation identity for later publication/reclamation logic.

The six deterministic unit tests cover gain/mix, shape and bound rejection, NaN/Inf repair, non-finite gain safety, mono/stereo conversion, and invalid matrix rejection. No Windows API, stream, driver, filesystem, or control-plane operation is performed by this crate. WASAPI event callbacks, graph scheduling, resampling, drift correction, and live generation publication remain unimplemented.
