# M06 native VST3 worker

This Windows-only worker is the production-facing single-stream VST3 process
owner used by the supervised Rust worker client. It loads one verified x64
VST3 bundle in a separate process, sends the existing length-prefixed JSON
`Hello`/`Ready` handshake, accepts bounded `Process` frames and parameter
events, calls `IAudioProcessor::process`, and returns `Processed` frames. It
uses restricted DLL search paths and never opens an audio device.

Build it from the repository root after the pinned SDK fixture has been built:

    .\tools\m06-vst3-worker\build.ps1

The generated executable and object file are ignored and must not be committed.
The worker accepts either one mono/stereo input and output bus through
`Process`, or a bounded auxiliary-bus layout through `HelloBuses` and
`ProcessBuses` (up to four buses and eight aggregate channels per direction).
Bus counts and per-bus channel counts must match the plugin exactly; auxiliary
buses are never flattened into the main stream. Native process buffers and the
framed protocol are currently worker-thread implementation details, not proof
of realtime graph scheduling or physical-latency performance.
