# Virtual cable WP-05 host implementation evidence (2026-10-05)

This is an in-progress host record, not WP-05 completion or VM qualification.

## Implemented on the host

- Added a canonical A-H cable list and deterministic INF generator. It emits
  80 interface registrations, stable cable-specific filter names, and the
  default `CableCount=2`; acceptance rejects malformed cable lists.
- Added a bounded hardware-key `CableCount` reader (default 2; accepted range
  1-8), and installation iterates only the first N render/capture pairs.
- Added double-precision bridge payload validation and PCM16, PCM24-in-32,
  PCM32 and float32 conversion helpers. Portable regressions include a PCM32
  sample that cannot survive float32, full-scale values, shape bounds, and
  every required rate/channel/format combination (60 total).
- Expanded the WaveRT direction enum to eight stable render/capture pairs and
  wired cable-specific pair names to cable-only WaveRT descriptors. The
  render and capture descriptors advertise exact supported channel layouts,
  rates and encodings, with 48 kHz float32 stereo as the default.
- Replaced the cable pairs' speaker/microphone-array topology tables with
  minimal direct speaker/line-in topologies; they insert no volume/mute or
  microphone-array nodes/properties.

## Checks run

- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-inf-gen.ps1` — passed; 80 interfaces, deterministic output, malformed list rejected.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1 -Platform x64` — passed; WDK build, InfVerif/signability and catalog checks. No driver install/load.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1 -Platform ARM64` — passed; same scope. No driver install/load.
- `powershell -NoProfile -ExecutionPolicy Bypass -File drivers/audiorouter-virtual/tests/build-tests.ps1` — passed; 132 bridge, conversion and format-metadata checks, including finite float32 bit-exact conversion without unit clamping, PCM rounding boundaries, and rejection of the old protocol minor.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tools/m03-bridge-fuzz/build.ps1` — passed; VM-only fuzzer built and not run.
- `git diff --check` — passed after whitespace cleanup.

## Remaining work and limits

The bridge endpoint identity does not yet carry a bus index into the lease
table: Cable B-H are not operational bridge buses until WP-06 maps all 16
directional leases and updates the Rust wire format. QPC position,
high-resolution notification timing, stream cleanup, and removal of unused
sample tone/file-writing code remain outstanding. WDK builds validate static
descriptors but do not prove Windows endpoint enumeration, format negotiation,
or playback.

The staged driver declares protocol 1.1 and rejects 1.0 requests because the
current payload width is double precision. WP-06 must finish the negotiated
header/open extension and Rust client before any integration; this intermediate
version intentionally has no compatible operational client.

Do not install or load this intermediate package, even in the VM, until
WP-06 bridge ABI integration is ready and the VM gate is deliberately resumed.
WP-03/04 VM ownership, Verifier, fuzz, endpoint, audio-continuity and signing
gates remain open. Host evidence does not satisfy them.
