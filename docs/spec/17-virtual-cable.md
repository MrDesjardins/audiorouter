# 17 — AudioRouter virtual cable (technical design)

Milestone ownership: M03 driver track (DEC-18). Execution order, work
packages and evidence: [virtual cable plan](../plans/active/virtual-cable.md).
Testing: [virtual cable testing](../operations/virtual-cable-testing.md).
Signing: [virtual cable signing](../operations/virtual-cable-signing.md).

This file refines [06 Virtual devices](06-virtual-devices.md) for the first
shipped AudioRouter cable. VDEV-01–12 and SEC-08 still apply; where this file
is narrower (for example "first N of 8" cables instead of freely created buses), the
difference is listed in "Deviations" and must be accepted by the user before
release. `VCAB-xx` identifiers are requirement IDs for traceability.

## 1. Goal and non-goals

**Design rule: sign once.** Anything that may need to change after release
must be changeable **without a new driver build**, because each driver build
needs a new Microsoft signature and a valid EV certificate
([signing](../operations/virtual-cable-signing.md)). So the signed driver
already supports the maximum we may want (8 cables, 8 channels, 44.1/48/96
kHz, low-latency periods), reads its tunable values from the registry, and
speaks an extensible protocol; AudioRouter (signed separately) decides what
to use. A driver change is then needed only for a bug or a Windows change.

**Quality rule.** The driver never changes the audio: no gain, no
resampling, no dither, no effects. With matching formats a signal passes
bit-exact; everything else is measured against §11.

**Goal.** A user installs AudioRouter, clicks one button, approves one Windows
prompt, and gets AudioRouter-owned virtual audio devices that games, Discord,
OBS and browsers can select. No VB-Cable download, no reboot in the normal
case, no Secure Boot change.

**Non-goals for the first release:** more than eight cables, more than eight
channels per cable, sample rates other than 44.1/48/96 kHz, exclusive mode, ASIO, Windows Server, ARM64 *release* (it keeps
building), Windows 10, distribution through Windows Update, a cable that
passes audio when AudioRouter is not running.

## 2. Product behavior

- **VCAB-01 — Up to eight cables, two enabled by default.** The driver
  supports **Cable A to Cable H** (`AR_MAX_CABLES = 8`, a compile-time
  constant; raising it later to at most 16 needs only the tables below and a
  new qualification). Each cable has one playback endpoint (apps play into
  it) and one recording endpoint (apps record from it), so 8 cables = 16
  Windows endpoints. Only **enabled** cables appear in Windows. The user
  picks how many (1–8, default 2) in Setup; enabled cables are always the
  first N (A, A–B, A–C, …) so that a cable's identity never moves. Changing
  the number needs one UAC prompt and briefly restarts the virtual device
  (all cables pause for about a second). Lowering the number removes the
  highest cables; apps set to them fall back to their own default and the UI
  warns first. Development and stage A tests use 2 cables; check A15
  qualifies all 8 before signing.
- **VCAB-02 — Names.** Windows endpoint names, chosen to read like VB-Cable so
  existing guides map over (the same pattern continues for Cable C–H,
  `busId` `cable-c` … `cable-h`):

  | Endpoint | Windows name | AudioRouter node | Typical use |
  | --- | --- | --- | --- |
  | Cable A playback | `AudioRouter Cable A Input` | Virtual Render Source, `busId = "cable-a"` | Game or browser output |
  | Cable A recording | `AudioRouter Cable A Output` | Virtual Capture Sink, `busId = "cable-a"` | OBS or recorder input |
  | Cable B playback | `AudioRouter Cable B Input` | Virtual Render Source, `busId = "cable-b"` | Music or second app |
  | Cable B recording | `AudioRouter Cable B Output` | Virtual Capture Sink, `busId = "cable-b"` | Discord microphone |

  Device description: `AudioRouter Virtual Cable`. Provider and manufacturer:
  `AudioRouter Project`. These replace the prototype names
  `AudioRouter - Desktop In` / `AudioRouter - Voice Chat`.
- **VCAB-03 — Audio goes through AudioRouter.** A cable's Input and Output are
  **not** connected inside the driver. Audio played into `Cable A Input`
  reaches `Cable A Output` only if an AudioRouter session routes it there
  (for example Virtual Render Source → Virtual Capture Sink, with or without
  tools in between). This is deliberate: it keeps every route visible on the
  canvas (VDEV-06) and lets AudioRouter process it. The UI states it, and the
  first-run guide offers a ready "pass-through" session.
- **VCAB-04 — Silence without AudioRouter.** With no active AudioRouter
  writer, a recording endpoint delivers digital silence and a playback
  endpoint accepts and discards audio at real-time pace, with no memory
  growth (VDEV-04). After an AudioRouter crash, the recording side is silent
  within 500 ms (VDEV-12) and never replays old audio.
- **VCAB-05 — Persistence.** Once installed, the enabled cables' endpoints exist at every
  boot whether AudioRouter runs or not, and keep their Windows endpoint IDs
  across reboots, AudioRouter updates and driver updates (VDEV-03). Apps that
  selected them keep their selection.
- **VCAB-06 — Coexistence.** VB-Cable, Voicemeeter and physical devices keep
  working. A session may mix AudioRouter cables and VB-Cable endpoints.
- **VCAB-07 — No default-device changes.** Installing, updating or removing
  the cable never sets a Windows default device on purpose. Windows itself may
  make a newly added endpoint the default when no default exists; the
  installer records the default playback and recording devices before
  installing and, if they changed, tells the user and offers to open Sound
  settings. It never changes defaults silently (see 9.4).
- **VCAB-08 — Optional.** The cable is never required. Skipping it keeps the
  current existing-endpoint workflow.

## 3. Audio format

- **VCAB-10 — Engine side.** The precision-preserving bridge carries interleaved float64, 1–8
  channels, at the endpoint's current sample rate. `AR_BRIDGE_MAX_CHANNELS`
  becomes 8 (protocol 1.1); the lease request already carries `Channels` and
  `SampleRateHz`. User decision 2026-10-05: preserve VCAB-21's PCM32 ≤1 LSB
  target by increasing precision, rather than accepting float32 loss. Float32
  has only 24 significant bits and cannot represent arbitrary PCM32 samples.
  The new client requires negotiated float64 capability; no silent float32
  fallback may qualify as the high-precision path.
- **VCAB-11 — Windows side.** Each endpoint offers:
  - sample rates **44 100, 48 000 and 96 000 Hz**;
  - channels **1, 2, 4, 6 (5.1) and 8 (7.1)** with the standard
    `KSAUDIO_SPEAKER_*` channel masks (eight channels use
    `KSAUDIO_SPEAKER_7POINT1_SURROUND`, 0x63F: front L/R, center, LFE,
    back L/R, side L/R; not the obsolete wide 7.1 mask 0xFF);
  - formats **IEEE float 32-bit**, PCM 16-bit, PCM 24-bit in a 32-bit
    container, PCM 32-bit.

  **Default format: 48 kHz, IEEE float 32-bit, stereo.** Windows' shared
  engine mixes in float, so a float device format means no quantization
  anywhere between an app and AudioRouter. Users can choose another default
  in Sound settings → device properties → Advanced; the driver accepts any
  combination above. The prototype offers only 16-bit render and 32-bit
  capture at 48 kHz stereo; extend the data ranges and the bridge conversion
  (`IsBridgePcmFormat`, `ReadBridgePcmSample`, `WriteBridgePcmSample` in
  `minwavertstream.cpp`).
- **VCAB-12 — Conversions.** Float32 ↔ float32: copy, bit-exact (no clamp,
  only NaN/Inf replaced by 0 and counted). Float32 → float64 → float32 is
  exact for finite float32 input, including signed zero and subnormals.
  Integer → float64: exact for PCM16/24/32
  (`x / 2^(bits-1)`). Float → integer: multiply by `2^(bits-1)`, round to
  nearest, clamp to the integer range, no dither (dither belongs in
  AudioRouter tools, not the kernel). **No resampling in the driver:** the
  driver runs each stream at its own rate; when an endpoint rate differs
  from the route's graph rate, AudioRouter resamples in user
  mode to the §11 targets.

## 4. Architecture

```text
 Game / browser                                      Discord / OBS
     │ plays into                                         ▲ records from
     ▼                                                    │
 [Cable A Input endpoint]                     [Cable B Output endpoint]
     │ render DMA (driver)                                ▲ capture DMA (driver)
     ▼                                                    │
 lease (cable-a, RENDER_SOURCE)              lease (cable-b, CAPTURE_SINK)
     │ shared section, seqlock                            ▲ shared section
     ▼                                                    │
 NativeBridge*Worker  ──►  AudioRouter graph (tools)  ──► NativeBridge*Worker
 (user mode, standard user, audiorouter-shell / backend)
```

Components:

| Component | Location | Runs as | Role |
| --- | --- | --- | --- |
| Kernel driver `audioroutervirtual.sys` | `drivers/audiorouter-virtual/` | Kernel (PortCls WaveRT miniport) | Exposes 4 endpoints and the bridge control device |
| Driver package | build output: `.inf`, `.sys`, `.cat` | — | Microsoft-signed for release |
| Driver helper `audiorouter-driver-helper.exe` | new crate `crates/driver-helper` | Elevated (UAC), short-lived | Install, update, repair, remove; nothing else |
| Bridge client | `crates/windows-audio/src/lib.rs` (`NativeBridgeControlClient`, `NativeBridge*Worker`) | Standard user | Opens the control device, maps sections, heartbeats |
| Control plane | `crates/control/src/lib.rs` | Standard user | Cable status, buses, route lifecycle |
| Engine | `crates/engine/src/lib.rs` | Standard user | Virtual Render Source / Capture Sink nodes |
| Shell | `src-tauri/src/main.rs` | Standard user | Launches the helper with UAC; UI commands |
| UI | `ui/src/` | WebView | First-run choice, Setup panel, library nodes |
| Installer | `src-tauri/tauri.release.conf.json` (NSIS, per user) | Standard user | Ships driver package + helper as resources; never installs the driver itself |

## 5. Kernel driver changes

The current driver is a Microsoft SysVAD "Simple Audio Sample" derivative
(MS-PL) with one render pair (`SpeakerMiniports`) and one capture pair
(`MicArray1Miniports`) in `Source/Filters/minipairs.h`, a bridge control
device in `Source/Main/adapter.cpp` and bridge helpers in
`Source/Inc/bridgeio.h`.

### 5.1 Endpoints (VCAB-01/02)

- Replace the two minipairs with `2 × AR_MAX_CABLES` = 16, generated by a
  macro per cable index (A–H): `CableARenderMiniports`,
  `CableACaptureMiniports`, …, `CableHCaptureMiniports`. Each has its own
  topology/wave filter names, for example `TopologyCableARender` /
  `WaveCableARender`, matching `KSNAME_*` strings, interface sections and
  `AddInterface` lines in the INF (16 render + capture sections; generate
  the INF fragment with a small script, `tools/m03-inf-gen`, so the 8 cables
  cannot drift apart).
- Extend the endpoint enum in `Source/Inc/definitions.h` (today
  `eSpeakerDevice`, `eMicArrayDevice1`, …) and add a table mapping each
  endpoint to `(busIndex 0–7, direction)`: `cable-a` = 0 … `cable-h` = 7;
  render endpoints publish `AR_BRIDGE_DIRECTION_RENDER_SOURCE`, capture
  endpoints read `AR_BRIDGE_DIRECTION_CAPTURE_SINK`.
- **Enabled count.** At `StartDevice` the driver reads the DWORD
  `CableCount` from the device's hardware registry key (`HKR`, written by
  the helper; missing or out of range → 2) and registers subdevices only for
  cables `0 … CableCount-1`. Because each cable always uses the same filter
  names, its Windows endpoint ID stays the same when the count changes or
  the driver updates. The INF sets the default with
  `HKR,,CableCount,0x00010001,2` in the `.HW` section.
- Topology: render = speaker topology without jack detection; capture = a
  simple line-in style topology (no mic-array geometry, no
  `MicArray1CustomName`). Set `KSNODETYPE` so Windows shows a generic
  speaker / line-in icon rather than "microphone array".
- Remove sample features not needed: tone generator
  (`Source/Utilities/ToneGenerator.*`), the diagnostic file writer
  (`savedata.*`) and its registry switches, audio modules/effects examples,
  and any keyword-spotter or offload remnants. They must not be reachable
  from a realtime path.

### 5.2 Bridge protocol 1.1 (multi-cable)

- Lease table: `AR_BRIDGE_LEASE_SLOTS` = `AR_MAX_CABLES` × 2 directions =
  16, statically allocated. Index = `busIndex * 2 + (direction - 1)`.
- `AR_BRIDGE_OPEN_REQUEST.BusId` selects the cable: exactly `cable-a` …
  `cable-h` (UTF-16, no terminator inside `BusIdBytes`). Unknown value →
  `STATUS_OBJECT_NAME_NOT_FOUND`; a known but not enabled cable →
  `STATUS_DEVICE_NOT_CONNECTED`.
- Every successful OPEN must use a nonzero generation strictly greater than
  the last successful generation for that direction during the current driver
  load. The driver retains the high-water mark across CLOSE/expiry so a stream
  that missed the inactive interval can detect lease turnover and reset its
  block sequence. A reused, decreasing, or exhausted generation is rejected
  with `STATUS_INVALID_PARAMETER`; the client chooses a monotonically
  increasing generation for each lease. Restarting the driver resets the
  high-water mark.
- `AR_BRIDGE_PROTOCOL_MINOR` = 1. The 176-byte open request stays as the
  fixed prefix; the `C_ASSERT`s stay.
- **Extensible open request.** Protocol 1.1 accepts either the 176-byte
  request or the 176-byte request followed by `AR_BRIDGE_OPEN_EXTENSION {
  ULONG ExtensionBytes; ULONG Flags; ULONG Reserved[14] }` (64 bytes). The
  driver rejects unknown `Flags` bits and non-zero `Reserved` with
  `STATUS_NOT_SUPPORTED` (never ignores them silently), so a newer
  AudioRouter can detect an older driver and fall back. Future options use
  new flag bits and reserved words; the size never shrinks.
- **Capabilities.** QUERY (below) returns a `Capabilities` bit mask:
  `MULTICHANNEL` (8 ch), `RATES_44_48_96`, `LOW_LATENCY_PERIODS`,
  `STREAM_COUNTERS`, `CONFIG_FROM_REGISTRY`. AudioRouter uses only what the
  installed driver reports.
- **Shared section header 128 bytes** (was 32): state/seqlock (8), block
  header (24), stream counters (written only by the driver, read by user
  mode, no IOCTL): `UnderrunFrames`, `OverrunFrames`, `SequenceGaps`,
  `NonFiniteSamples`, `FormatMismatches`, `LastDevicePosition`,
  `LastQpcTime` (all `ULONGLONG`, monotonic), then reserved bytes to 128.
  Payload offset becomes 128. Max payload: 8 ch × 4096 frames × 8 bytes
  (float64, see the precision extension) = 256 KiB per lease, in the
  user-mode section (never kernel pool). The
  header change is allowed in a minor version only because protocol 1.0 was
  never released; after the first release, layout changes need a new major
  version. Version rule: user mode sends its major/minor; the driver
  accepts major 1 with minor ≤ its own; a major mismatch →
  `STATUS_REVISION_MISMATCH`, which the Rust client maps to a clear
  "AudioRouter cable driver is too old/new; repair it from Setup" error
  (VDEV-12).
- **Precision extension (2026-10-05).** OPEN extension flag `FLOAT64` (bit 0)
  selects 8-byte samples; QUERY capability `SAMPLE_FLOAT64` reports support.
  Without that flag the legacy float32 transport can be selected only
  explicitly, never as an automatic replacement for a precision-preserving
  route. Shared header includes `SampleBytes` (4 or 8) in the previously
  reserved area; readers require the negotiated size before payload access.
  The maximum float64 payload is 8 × 4096 × 8 = 256 KiB per lease. Exact
  length validation uses negotiated sample size and overflow-safe arithmetic.
  These changes are before the first published protocol; no public 1.0 client
  compatibility is claimed. The old 32-byte header is not accepted as a 1.1 view.
- **As implemented (2026-10-05, host-built, not yet VM-qualified).** The first
  driver implements only float64 transport: OPEN must be the 240-byte form with
  `FLOAT64` set; a 176-byte OPEN returns `STATUS_NOT_SUPPORTED` (HEARTBEAT and
  CLOSE may use either form; the extension is never part of the lease
  identity). A later driver may add other transports through new flag bits.
  Shared header offsets: state 0, block header 8, counters 32–87,
  `SampleBytes` 88 (driver-written at OPEN, 8), `ReaderSequence` 96 (the
  consumer's acknowledgement of the last block it took: user mode writes it
  for a `RENDER_SOURCE` lease, and the driver counts `OverrunFrames` when it
  replaces a block that was not acknowledged; **the driver writes it for a
  `CAPTURE_SINK` lease** when its capture callback takes a block, so the
  user-mode producer publishes the next block right after the
  acknowledgement and runs on the endpoint's QPC clock instead of its own
  timer: flow control for the single-block slot, added 2026-10-06 after
  analysing wall-clock pacing against VCAB-24; the driver never reads the
  value back for a capture sink), reserved
  to 128. OPEN zeroes bytes 32–127 before the lease becomes visible. Counter
  units: `UnderrunFrames` frames of silence while a usable capture-sink lease
  had no newer block; `SequenceGaps` skipped block sequences;
  `NonFiniteSamples` every NaN/Inf sample in a rejected block (the whole block
  is refused and replaced by silence); `FormatMismatches` callbacks in which an
  active lease's rate or channel count differed from the endpoint stream (that
  lease receives or produces silence, never wrong-speed audio);
  `LastDevicePosition` in frames and `LastQpcTime` in QPC ticks. QUERY reports
  only implemented capabilities; `MinPeriodFrames`/`DefaultPeriodFrames` are 0
  until `LOW_LATENCY_PERIODS` is implemented, and `DriverVersion` is the
  four-part version `build.ps1 -Version` stamps (0.0.0.0 for unversioned
  developer builds). IOCTL codes are `METHOD_BUFFERED`: OPEN `0x0022E000`,
  CLOSE `0x0022E004`, HEARTBEAT `0x0022E008`, QUERY `0x0022600C`; both the C
  header and the Rust client pin these literal values in tests.
- New IOCTL `IOCTL_AUDIOROUTER_BRIDGE_QUERY` (function 0x803, METHOD_BUFFERED,
  read access) returns `AR_BRIDGE_DRIVER_INFO { ProtocolMajor, ProtocolMinor,
  DriverVersion (4×USHORT), CableCount (enabled), MaxCables, MaxChannels,
  Capabilities, SupportedRates bit mask, MinPeriodFrames, DefaultPeriodFrames,
  Reserved[16] }` with `C_ASSERT` on its size. Used by status and repair.
- Rust mirrors: `crates/windows-audio/src/lib.rs` constants
  `IOCTL_AUDIOROUTER_BRIDGE_*` and the request encoder; add a unit test that
  checks every offset against the C header values.

### 5.3 Security fixes (SEC-08) — required before any signing

1. **Double-fetch in `AudioRouterCopyBridgeBlock` (bridgeio.h).** The header
   in the user-writable shared view is validated, then `Frames` and
   `Channels` are read again to size the copy. A hostile or buggy writer can
   enlarge them between the two reads, making the kernel read past the end of
   the mapped view (bug check). Fix: copy the 24-byte header into a local
   variable once (volatile read), validate the local copy, and use only the
   local values for the size checks and loops. Also copy each sample once
   (read → check finite → store) instead of a validate pass plus a copy pass.
   The existing seqlock retry (`stateBefore` check) stays.
2. **Access for a standard user (NFR-16).** The control device SDDL
   `D:P(A;;GA;;;SY)(A;;GA;;;BA)` lets only SYSTEM and Administrators open it,
   but AudioRouter runs as a standard user. Change to
   `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)` (interactive users read/write)
   **and** enforce ownership in the driver:
   - store the opener's session ID (`IoGetRequestorSessionId`) and
     `FILE_OBJECT` in the lease at OPEN;
   - HEARTBEAT and CLOSE from another file object → `STATUS_ACCESS_DENIED`;
   - an OPEN for a lease held by another live file object → `STATUS_SHARING_VIOLATION`
     (maps to API `resourceConflict`, VDEV-07);
   - an OPEN from a different session while a lease is active →
     `STATUS_ACCESS_DENIED`; after release, buffers are zeroed before reuse
     (cross-user isolation, VDEV-07).
   Keep "lease follows the control handle" (validated lesson 2026-09-11).
3. **Section validation.** Reference the section with
   `ObReferenceObjectByHandle(..., *MmSectionObjectType, UserMode, ...)` (the
   object type must be checked, today it is `NULL`), require the mapped size
   to equal the required size (header + one quantum), and reject a second
   OPEN that reuses the same section for both directions.
4. **IOCTL input.** Check `InputBufferLength == sizeof(request)` exactly and
   `OutputBufferLength` for QUERY; reject anything else with
   `STATUS_INVALID_PARAMETER` before reading.
5. **INF device security.** The PnP device `Security` value grants World
   read/write/execute (sample default for audio stacks). Keep it for the
   audio stack, but the bridge control device must use only the SDDL in
   item 2.
6. Lessons from 2026 Voicemeeter CVEs (CVE-2026-23762–23764): never map
   kernel non-paged pool into user space, never honor special file
   attributes on open, and fuzz every IOCTL.

### 5.4 Real-time rules and performance in the driver

- WaveRT position/DMA callbacks run at DISPATCH_LEVEL: no allocation, no
  paged memory, no waits, no logging beyond `DPF` compiled out in Release.
- **Clock.** Stream position is computed from `KeQueryPerformanceCounter`
  and the stream's sample rate (exact integer arithmetic on 64-bit values,
  no accumulated rounding), never from counting timer ticks. All AudioRouter
  cables therefore share one clock (QPC), which lets the engine skip drift
  correction between two cables (7.3).
- **Timer.** Notification events use a high-resolution timer
  (`ExAllocateTimer` with `EX_TIMER_HIGH_RESOLUTION`) at the stream's period;
  no timer runs for a stopped or closed stream, so idle cables cost nothing.
- **Low latency.** Advertise packet-size constraints
  (`KSAUDIO_PACKETSIZE_CONSTRAINTS2`; take the exact property and pin
  wiring from Microsoft's low-latency audio documentation and the WDK SysVAD
  sample, and verify with `IAudioClient3::GetSharedModeEnginePeriod`) so
  Windows'
  `IAudioClient3` offers shared-mode periods down to **128 frames (2.67 ms
  at 48 kHz)**, default 480 frames (10 ms). AudioRouter opens its side with
  the smallest period the engine quantum allows. Lesson 2026-09-21: a
  physical device may have a fixed 10 ms floor; our own driver must not.
- Underrun (no newer sequence): write silence for the missing frames and
  increment `UnderrunFrames`. Overrun on render (user mode not reading):
  overwrite the oldest unread block, increment `OverrunFrames`, never queue
  without bound.
- The copy per callback is one bounded loop over the frames due; no
  per-sample function calls through pointers; conversion functions are
  `__forceinline` per format.

### 5.5 Configuration without re-signing (registry)

The driver reads these values at `StartDevice` from its hardware key (`HKR`,
written only by the elevated helper, §6). Missing or out-of-range values use
the compiled default; every value is range-checked before use. A change
takes effect when the helper restarts the device (about 1 s pause).

| Value | Type | Range | Default | Purpose |
| --- | --- | --- | --- | --- |
| `CableCount` | DWORD | 1–8 | 2 | Enabled cables (VCAB-01) |
| `Cables\<n>\Name` | REG_SZ, ≤ 48 chars | printable, no control chars | `Cable A` … `Cable H` | Display name; the driver sets the endpoint names `AudioRouter <Name> Input` / `Output` (VCAB-02) |
| `MinPeriodFrames` | DWORD | 64–480 | 128 | Smallest shared-mode period offered |
| `DefaultPeriodFrames` | DWORD | 128–960 | 480 | Default period |
| `MaxLeaseMs` | DWORD | 500–60 000 | 60 000 | Upper bound for lease requests |
| `SilenceOnStaleMs` | DWORD | 20–500 | 100 | Capture side outputs silence when no new block arrived for this long (VDEV-12 budget 500 ms) |

**As implemented (2026-10-06, host-built; VM verification pending):**

- `CableCount`, `MinPeriodFrames`, `DefaultPeriodFrames` and `MaxLeaseMs`
  are read at `StartDevice` (REG_DWORD only; anything else uses the
  default) and reported by QUERY with `CONFIG_FROM_REGISTRY`. A default
  period below the minimum is raised to the minimum.
- `MinPeriodFrames` becomes `KSAUDIO_PACKETSIZE_CONSTRAINTS2.
  MinPacketPeriodInHns` (frames at 48 kHz → 100 ns units, 128 → 26 666) on
  every cable wave filter's `KSCATEGORY_AUDIO` interface through
  `DEVPKEY_KsAudio_PacketSize_Constraints2`, set before the interface is
  enabled (QUERY reports `LOW_LATENCY_PERIODS`). Whether Windows then offers
  128-frame periods is measured in the VM with
  `IAudioClient3::GetSharedModeEnginePeriod` (A-check in the testing
  procedure); it is not claimed from the build.
- `DefaultPeriodFrames` is **reported only**: Windows, not the driver, owns
  the default shared-mode engine period (10 ms). AudioRouter uses the value
  as its own preferred period when it opens a cable.
- `MaxLeaseMs` caps OPEN/HEARTBEAT/CLOSE `LeaseMs`
  (`STATUS_INVALID_PARAMETER` above it).
- `SilenceOnStaleMs` is **not read**: the requirement holds by construction.
  Every bridge block is consumed at most once and never replayed; with no
  newer block the capture endpoint writes silence in the same callback (and
  counts `UnderrunFrames`). After the producer stops, at most the one
  unread quantum (≤ 4096 frames, ≤ 93 ms at 44.1 kHz) can still play, well
  inside VDEV-12's 500 ms.
- **Names (decision):** the driver does not read `Cables\<n>\Name`. Windows
  hardcodes `Speakers` for speaker endpoints, so render bridge pins use the
  standard analog-connector category and a cable-specific pin `Name` GUID.
  Capture bridge pins use cable-specific category GUIDs. The INF registers
  these GUID/name pairs in the device software key as
  `AudioRouter Cable <letter> Input` or `Output`. The helper identifies a
  cable from that stable description, with a composed-name fallback for older
  test packages, and counts only active endpoints whose actual data flow
  matches the cable direction.
  Custom user names are written by the
  elevated helper as the endpoint's `PKEY_Device_FriendlyName`, preserving
  the driver-provided `PKEY_Device_DeviceDesc` category identity and endpoint
  ID; for example
  `AudioRouter Discord Output (AudioRouter Virtual Cable)`. The driver
  interface names remain stable across user renames and driver updates. The
  root PnP device description is `AudioRouter Virtual Cable`.
- **Observed VM defect (2026-10-08):** the first category-based package
  created four healthy endpoints and correctly named both capture endpoints,
  but Windows still returned `Speakers (AudioRouter Virtual Cable)` for both
  render endpoints. The speaker label is fixed, and the helper could not
  identify the two render cables. The follow-up uses analog-connector render
  pins with cable-specific pin `Name` GUIDs. The revised names remain
  unqualified until a new VM install confirms all four cable/direction names.

### 5.6 INF and package

- `DriverVer` set by the build from the AudioRouter version
  (`MM/DD/YYYY, X.Y.Z.0`); it must increase with every driver release.
- Target `NT$ARCH$.10.0...22000` (Windows 11). Keep it, but add a test that
  the package installs on 23H2, 24H2 and 25H2 (stage B). The issue reported
  on another project (Code 52 on most builds) came from a Microsoft
  signature limited to one build; when submitting, select **all** Windows 11
  versions in Partner Center.
- Hardware ID: keep `ROOT\AudioRouterVirtual` (root-enumerated device, created
  by the helper). Remove the `SWD\AudioRouterVirtual` match and the
  Software-Device provisioning path for the first release (see 8).
- Remove the `[AUDIOROUTERVIRTUAL_SA.NT.Wdf]` section unless the driver links
  KMDF (PortCls miniports normally do not; verify with the build).
- `PnpLockDown = 1` stays. Pass `InfVerif /h /rulever 25h2` with no errors
  (required for Partner Center submissions).
- Ship `LICENSE-MS-PL.txt` beside the package and list the driver in
  third-party notices.

## 6. Driver helper (elevated)

New crate `crates/driver-helper`, binary `audiorouter-driver-helper.exe`,
embedded manifest `requireAdministrator`. It is the **only** component that
changes the driver store or creates the device (VDEV-08).

Commands (all print one JSON result to stdout and also write it to the path
given by `--result <file>`, because an elevated process cannot be read
through a pipe by its non-elevated parent when started with `runas`):

| Command | Action |
| --- | --- |
| `status` | Read-only: package in driver store (version), device present, endpoints present, protocol via QUERY. Does not need elevation; the backend implements the same logic directly. |
| `install --package <dir>` | Verify the package (9.1); record current default devices; add the package to the driver store (`DiInstallDriverW` or `SetupCopyOEMInfW` + install); if no `ROOT\AudioRouterVirtual` device exists, create one (`SetupDiCreateDeviceInfoW` + `SPDRP_HARDWAREID` + `SetupDiCallClassInstaller(DIF_REGISTERDEVICE)` + `UpdateDriverForPlugAndPlayDevicesW`); wait (bounded, 30 s) for the enabled cables' endpoints (default 2 cables = 4 endpoints); report defaults that changed. |
| `update --package <dir>` | Same as install when the device exists: newer `DriverVer` is applied with `UpdateDriverForPlugAndPlayDevicesW`; the device instance and endpoint IDs stay. Report if Windows requires a restart. |
| `repair --package <dir>` | Reinstall the same version; recreate the device if missing. |
| `set-cables --count <1-8>` | Write `CableCount` to the device's hardware key, then restart the device (`SetupDiCallClassInstaller(DIF_PROPERTYCHANGE)` with `DICS_PROPCHANGE`); wait (bounded, 30 s) until exactly the first N cables' endpoints are present. Reports a pending restart if Windows cannot restart the device while it is in use. |
| `configure --name <n>=<text> --param <Value>=<n>` | Write cable names and §5.5 parameters (validated against the same ranges), then restart the device like `set-cables`. |
| `remove` | Remove the `ROOT\AudioRouterVirtual` device (`DiUninstallDevice`), then the package (`DiUninstallDriverW` / `pnputil` equivalent) for **this** package only (exact `oem*.inf` from state). Never touches other drivers. |

Rules:
- Idempotent; a second `install` of the same version is a no-op success.
- State file: `%ProgramData%\AudioRouter\driver\state.json`
  (`{ schema, packageVersion, oemInf, deviceInstanceId, installedAt }`),
  written atomically. Log: `%ProgramData%\AudioRouter\driver\helper.log`,
  bounded (1 MB, one rotation), no audio, no user paths beyond the package
  dir.
- Arguments are validated; `--package` must be the AudioRouter install
  directory's `driver\` folder (resolved from the helper's own path), never
  an arbitrary path.
- Exit codes: 0 success, 1 invalid request, 2 verification failed, 3 Windows
  refused (HRESULT in JSON), 4 timeout, 5 restart required (success but
  pending).
- Rollback inside `install`: if the device cannot be created after the
  package was added, remove the package again and report both results
  (pattern already used by `drivers/audiorouter-virtual/manage.ps1`).
- `manage.ps1` stays as the developer/VM tool; the helper is the product
  path. Both must produce the same end state.

## 7. AudioRouter integration

### 7.1 Status detection (backend, standard user)

`crates/windows-audio`: new `virtual_cable_status()` returning
`{ state: "notInstalled" | "installed" | "needsRepair" | "needsRestart" |
"incompatible", driverVersion, protocol, cables: [{ busId, renderEndpointId,
captureEndpointId, present }] }`, from SetupAPI/CfgMgr (device + driver
version), MMDevice enumeration (endpoints by device instance, not by
friendly name, because users can rename endpoints in Sound settings) and
`IOCTL_AUDIOROUTER_BRIDGE_QUERY`. Cached for 2 s; refreshed on device-change
notifications that the backend already watches.

### 7.2 API (spec 10) and CLI/MCP

- New read method `virtualCable.status` (scope: `sessionRead` or the lowest
  read scope that `devices.list` uses), mirrored by `GET /api/v1/virtualCable/status`
  and `audiorouter-cli virtual-cable status`.
- Install/update/repair/remove are **not** backend RPC methods, because
  they need UAC and the backend must not elevate. They are shell commands
  (`virtual_cable_install`, `_update`, `_repair`, `_remove`) that start the
  helper with `ShellExecuteExW` verb `runas`, wait for it, read the result
  file, then ask the backend to refresh status. The CLI gets
  `virtual-cable install|remove` that does the same (shows UAC). MCP and the
  HTTP API expose status only: an AI client or a LAN device can never trigger
  a driver install.
- Shell command `virtual_cable_set_count` (and CLI `virtual-cable set-count
  <n>`) runs the helper `set-cables` elevated; before lowering the count it
  lists sessions that use the cables to be removed and requires
  confirmation (VDEV-10).
- `virtualDevices.list` returns the enabled buses `cable-a` … with
  `availability` from the status, and `maxBuses: 8`. `virtualDevices.provision` and
  `virtualDevices.remove` return `notSupported` with reason
  `fixedCablesInstalledByPackage`, are marked deprecated in the schema, and
  are removed at the next API major version (spec 10 versioning rules).
- `nodes.catalog`: Virtual Render Source / Virtual Capture Sink become
  available when the status is `installed`; their `busId` parameter is an
  enum of enabled cables with display names "Cable A" … "Cable H".

### 7.3 Engine

- Virtual Render Source (`NodeKind::VirtualRenderSource`, param `busId`)
  compiles to a path whose capture side is a `NativeBridgeInputWorker` lease
  `(busId, RENDER_SOURCE)`; Virtual Capture Sink compiles to a path whose
  render side is a `NativeBridgeOutputWorker` lease `(busId, CAPTURE_SINK)`.
  Follow the way physical endpoint paths are compiled in the multi-path
  engine (search `open_bound_at_rate_with_retry` and
  `fold_input_channel_modes` in `crates/control` and `crates/engine`).
- **Clock domains.** All AudioRouter cables share the driver's QPC clock
  (5.4). A route whose inputs and outputs are all AudioRouter cables at the
  same rate runs **without drift correction or resampling** and must be
  bit-exact (VCAB-20). A route that also uses a physical device or VB-Cable
  crosses clock domains and uses the existing multi-path drift handling;
  its quality is measured by VCAB-23. Required test: Cable A Input →
  Focusrite output for 10 minutes with zero glitches in the sine-continuity
  harness (validated lesson 2026-09-26).
- **Rate and precision (user decision 2026-10-05).** A cable-only route
  with matching endpoint rates selects a float64 runtime at that rate
  (44.1/48/96 kHz), preserving samples without rate conversion. Mixed-clock
  or mixed-rate routes use the 48 kHz graph and a high-quality user-mode
  resampler where needed (VCAB-22). This resolves the former contradictory
  instructions to resample every 44.1/96 kHz cable and also pass those same
  rates bit-exact. The high-precision engine owns prepared float64 buffers,
  matrices and unity/gain/mix stages; no allocation occurs in callbacks.
  A legacy float32 DSP/plugin boundary must be explicit in the compiled plan
  and capability reporting; it cannot masquerade as a high-precision unity
  path. VCAB-20/21 qualification uses no tools and unity gain; processing
  nodes have their own declared numeric behavior. Existing physical/VB-Cable
  pipelines remain supported during the separate cable-runtime integration.
- **Channels.** The lease carries the endpoint's channel count; stereo
  graphs get an explicit channel matrix (downmix/upmix) shown on the canvas,
  never a hidden conversion (VDEV-06). Multichannel graphs (for example OBS
  multitrack) pass all channels.
- **Counters.** The bridge workers read the shared-header counters every
  telemetry tick and publish them in `system.diagnostics` and the Timing
  view; any increase during qualification is a failure.
- The backend's 1 ms service loop pumps the bridge workers (validated
  lesson: never pump from a UI timer) and heartbeats leases at
  `LeaseMs / 4` (default `LeaseMs` = 2000).
- Protected voice paths: if a bridge fails on a protected path, output
  silence (DEC-10); never fall back to another device.
- Add both node kinds to `crates/engine/tests/tool_combinations.rs` as
  sources/sinks so bypass and connected-Mixer layouts compile (validated
  lesson 2026-10-02).

### 7.4 UI

- **First-run guide** (`ui/src/FirstRunGuide.tsx`): new step "Virtual
  cables" with two buttons: "Install AudioRouter cables (recommended)" and
  "I use VB-Cable / Voicemeeter". Text explains: one Windows prompt, no
  restart normally, the cables carry sound only while AudioRouter runs.
  Replace the sentence "AudioRouter does not install virtual audio drivers."
- **Setup → Virtual cables panel**: status pill (reserved width, UI-17),
  driver version, a **Number of cables** select (1–8; changing it shows the
  UAC note, and lowering it lists affected sessions first), an editable
  **name per cable** (applied with one UAC prompt and a short device
  restart), each endpoint's current format (rate, channels, sample type)
  with a hint when it is not 48 kHz float, live counters (underruns,
  overruns) in reserved slots, buttons Install / Update /
  Repair / Remove (only the valid ones enabled, others disabled, no layout
  shift), and a "Create pass-through session" action.
- **Library** (`ui/src/library.ts`): `virtual-render-source` and
  `virtual-capture-sink` lose their `unavailableReason` when installed; when
  not installed, the reason becomes "Install AudioRouter cables in Setup, or
  use Input device with a virtual cable such as CABLE Output."
- **Templates:** "Game → Discord" (Cable A Input → processing → headphones;
  mic chain → Cable B Output) and "Pass-through" (Cable A Input → Cable A
  Output).
- All new controls use the app field style and are checked in dark, light
  and high-contrast themes (AGENTS.md UI conventions).

### 7.5 Installer and uninstall

- `src-tauri/tauri.release.conf.json` resources add
  `driver/audioroutervirtual.inf`, `.sys`, `.cat`, `LICENSE-MS-PL.txt` and
  `audiorouter-driver-helper.exe`. `installMode` stays `currentUser`
  (DIST-01). The NSIS installer never installs the driver.
- App update with a newer driver in the package: after the app starts, the
  Setup panel shows "Update available for AudioRouter cables" and the user
  clicks Update (one UAC prompt). Never update silently.
- App uninstall: an NSIS uninstall hook (Tauri 2 `installerHooks`,
  `NSIS_HOOK_PREUNINSTALL`) asks "Also remove AudioRouter virtual cables?"
  (default **No**, because other apps may be set to them). Yes → run the
  helper `remove` elevated. Sessions and recordings are kept either way
  (DIST-02).
- Release artifacts: the signed driver files are part of the app installer
  and are also listed with SHA-256 in the release manifest (ENG-05).

## 8. Deviations from 06 (need the user's acceptance before release)

| Requirement | 06 says | First release | Reason |
| --- | --- | --- | --- |
| VDEV-01 | Create/rename/delete up to 8 named buses | Up to 8 cables, enabled as "first N" (1–8); renaming changes only AudioRouter's label, not the Windows name | One root device with static endpoints is far simpler and safer than dynamic per-bus devices; free deletion of a middle cable would need per-bus devices (Software Device seam, `crates/windows-audio/src/software_device.rs`) |
| VDEV-10 | Delete a bus | Lower the count (removes the highest cables) or remove the whole package | Follows from "first N" |

## 9. Safety details

### 9.1 Package verification before install

The helper verifies with `WinVerifyTrust` that `audioroutervirtual.cat` is
signed by Microsoft (`Microsoft Windows Hardware Compatibility Publisher`)
and that the INF/SYS hashes match the catalog. Developer builds may accept
the WDK test certificate only when the helper is a debug build **and**
`AUDIOROUTER_ALLOW_TEST_DRIVER=1` is set; release builds never accept it.

### 9.2 What must never happen

No test-signing, Secure Boot or HVCI change; no change of Windows default
devices or volumes; no removal of drivers that are not this package; no
driver install without a visible UAC prompt started by a user click; no
driver work from MCP, the HTTP API or LAN clients.

### 9.3 Telemetry

None (PROD-06). Diagnostics stay local in the helper log and
`system.diagnostics`.

### 9.4 Default device risk

Windows may pick a new endpoint as default if the user had none of that kind
(common on PCs without a microphone). The helper reports this; the UI tells
the user and links to Sound settings. Stage C/D tests record whether it
happens.

## 10. Known risks

| Risk | Mitigation |
| --- | --- |
| Microsoft ends attestation for public drivers | Plan B: WHCP/HLK submission ([driver track](../plans/future/M03-driver-signing.md)) |
| Anti-cheat refuses the driver | Stage C tests Siege/BattlEye; no known reason for a signed audio driver to be blocked |
| Kernel bug crashes PCs | Small driver, Driver Verifier, fuzzing, VM-only development, beta stage D |
| Clock drift glitches | Per-cable clock domains plus continuity harness |
| Users expect a stand-alone cable | VCAB-03 text in first run and Setup; pass-through template |

## 11. Performance and sound quality

Targets below are acceptance criteria. Numbers marked *target* follow the
DEC-14 method: measure first; if a target is missed, optimize or record an
explicit user decision with the measured values before release. VM
measurements are indicative only; **the binding measurements are taken on
real hardware in stage C** ([testing](../operations/virtual-cable-testing.md)).

### 11.1 Requirements

- **VCAB-20 — Bit-exact path.** Float32 at the same rate end to end, cable
  → AudioRouter (no tools, unity gain) → cable: output equals input sample
  for sample after alignment, at 44.1/48/96 kHz and 1/2/8 channels. Also
  bit-exact: silence in → digital zero out (no DC, no dither, no noise).
- **VCAB-21 — Integer formats.** With a 16/24/32-bit device format and a
  matching-rate unity route without processing tools, the
  round-trip error is at most 1 LSB of that format; full-scale input does
  not wrap (clamped).
- **VCAB-22 — Resampling (endpoint rate ≠ 48 kHz).** Passband 20 Hz–20 kHz
  within ±0.05 dB; aliasing/imaging products ≤ −110 dBFS; THD+N at 997 Hz,
  −1 dBFS ≤ −110 dB; group delay constant (linear phase) or documented.
  *Target.*
- **VCAB-23 — Drift correction (route with a physical device).** THD+N at
  997 Hz ≤ −100 dB; no discontinuity in the sine-continuity harness over
  1 hour; no buffer under/overrun counter increase. *Target.*
- **VCAB-24 — No glitches.** Zero discontinuities (47 Hz and 997 Hz
  continuity harness) and zero counter increases over 1 hour with **8 cables
  active at once**, each carrying its own tone, plus normal desktop use.
- **VCAB-25 — Latency.** Cable → AudioRouter pass-through → cable, impulse
  method (`m00-native-impulse-loopback.ps1`): p95 ≤ **20 ms** with low-latency
  periods, ≤ 40 ms with default periods; never worse than VB-Cable on the
  same PC by more than 5 ms. Jitter (p99 − p1) ≤ 2 ms. *Target.* NFR-02
  still applies to routes with a physical microphone.
- **VCAB-26 — Isolation.** Crosstalk between any two cables ≤ −140 dBFS
  (digital paths; effectively none).
- **VCAB-27 — CPU.** Driver: no timers or DPCs for idle cables; per active
  stream DPC average < 20 µs, maximum < 200 µs (Windows Performance Recorder
  on the reference PC). AudioRouter backend: 8 active pass-through cables
  without tools ≤ 3 % of one core on the reference PC (spec 14). *Target.*
- **VCAB-28 — Memory.** Driver non-paged pool constant after the device
  starts (no growth over 24 hours of streaming); no per-stream allocation in
  callbacks.
- **VCAB-29 — No hidden processing.** The driver ships no APOs (so Windows
  "audio enhancements" have nothing to apply), no driver-side volume,
  endpoint volume defaults to 100 %, and the topology exposes no AGC or
  noise-suppression nodes. Guidance in the UI: set Windows
  "Communications → Do nothing" so Discord on a cable does not duck other
  audio, and turn off Discord's own processing when AudioRouter already
  processes the voice (VDEV-11).
- **VCAB-30 — Stable under load.** VCAB-24 still holds with a CPU stress
  load (e.g. a game or `cpu-stress` at 80 % on all cores) and while other
  devices are plugged/unplugged.

### 11.2 How it is measured

A new Rust tool `tools/m03-cable-quality` (Windows-only, WASAPI shared mode)
plays generated signals into one endpoint and records another, then writes
`metrics.json` and WAV files under `target/cable-quality/<case>/`, like
`tests/acceptance/deterministic-audio.ps1`:

| Signal | Measures |
| --- | --- |
| Pseudo-random float noise (fixed seed) | VCAB-20 bit-exact compare after alignment |
| Silence | Zero output, no DC |
| 997 Hz at −1 dBFS | THD+N, VCAB-21/22/23 |
| Log sweep 10 Hz–Nyquist | Frequency response, VCAB-22 |
| 47 Hz and 997 Hz long tones | Continuity harness, VCAB-23/24 |
| Impulse train | Latency and jitter, VCAB-25 |
| Different tone per cable | Crosstalk, VCAB-26 |

CPU and DPC: Windows Performance Recorder (`wpr -start CPU -start
DPC_ISR`, then `wpr -stop`) analysed with Windows Performance Analyzer;
backend CPU from `Get-Process` sampling. Memory: Performance Monitor
`Memory\Pool Nonpaged Bytes` and `poolmon` tag for the driver's pool tag.
