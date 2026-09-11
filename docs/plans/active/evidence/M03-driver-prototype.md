# M03 AudioRouter virtual-driver prototype evidence

Date: 2026-09-10
Environment: Windows x64, Visual Studio 18.9.1 (Community 2026), WDK
10.0.28000.0, PowerShell, repository `main`
Command:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\drivers\audiorouter-virtual\build.ps1 -Configuration Release -Platform x64 -KeepOutput
```

## Result

The AudioRouter-owned derivative built successfully through Utilities, Filters,
Main, Package, and Inc. The WDK signability/catalog step reported zero errors
and zero warnings and produced:

- `drivers/audiorouter-virtual/Source/Main/x64/Release/AudioRouterVirtual.sys`
- `drivers/audiorouter-virtual/Source/Main/x64/Release/AudioRouterVirtual.inf`
- `drivers/audiorouter-virtual/x64/Release/package/audioroutervirtual.cat`

The build passed `SignMode=Off`, `SkipPackageVerification=true`, and
`ApiValidator_Enable=false`. The WDK therefore did not test-sign the binary or
claim Universal-driver API qualification. Catalog generation is not production
signing evidence.

## Scope and remaining gates

This is source/build evidence for M03 VDEV-01..08 and SEC-08 only. The source
currently exposes the derivative of the upstream sample's one render and one
capture endpoint families. It does not yet implement AudioRouter bus count,
ownership lease, broker/bridge IPC, graph-to-driver callback transport, managed
create/rename/enable/disable/delete lifecycle, uninstall/restore, production
signing, or clean-machine installation. VDEV-09 and M08 signing/release gates
remain open.

No INF installation, service start, device registration, boot-policy change,
test-signing change, audio-default change, stream open, or persistent machine
configuration change occurred. The generated build directories are ignored and
disposable.

## Bridge groundwork

`audiorouter-engine` now provides `VirtualBusBridgeSet`, a bounded eight-slot
control-plane collection. It creates bridges lazily, exposes `Arc` handles for
the realtime path, and deactivates/drains a bridge before removal. The focused
`cargo test -p audiorouter-engine --locked` run passed all 96 tests. This does
not claim native shared-memory transport or driver callback integration.

Managed control operations now synchronize the bounded bridge collection using
stable bus IDs. Create provisions lazily, disable deactivates after durable
state succeeds, delete removes the bridge and restores it on storage failure,
and planned apply compensates bridge changes when its journal write fails. The
focused `cargo test -p audiorouter-control --locked` run passed all 106 tests.

## Mapped bridge region

`audiorouter-windows-audio` now provides `NativeBridgeRegion`, a bounded
explicit-path memory-mapped slot using the protocol block header and an aligned
seqlock word. It validates path ancestry, shape, generation, sequence, exact
payload length, finite samples, and torn reads before returning data to a caller
buffer. `cargo test -p audiorouter-windows-audio --locked` passed all 43 tests;
strict package Clippy passed. The mapping is not connected to a driver IOCTL,
and no device handle or endpoint was opened.

`NativeBridgeSession` now owns the negotiated hello and mapped region together.
It rejects an invalid bus identity before creating the backing file, binds writes
and reads to the negotiated generation and PCM shape, and assigns monotonic
sequences without allocating audio buffers. The focused Windows-audio suite
passed 45 tests after this addition, with strict package Clippy and formatting
checks passing. This remains broker-side evidence: the sample driver still has
no control-device IOCTL, and no device handle, endpoint, or driver was opened.

The driver IOCTL is intentionally not added as an unsecured named device. A
real implementation must define the security descriptor, broker ownership,
PnP/remove cleanup, bounded request validation, and mapping lifetime together;
until then, the driver build remains a safe non-installing prototype.

The prototype now includes `Source/Inc/bridgeio.h`, which defines the bounded
versioned open/close/heartbeat IOCTL numbers and fixed request/block layouts.
Its pure request validator rejects incompatible protocol versions, empty or
odd-sized bus IDs, unsupported PCM shape/rate, zero generation, and invalid
lease values before any future dispatch can touch a mapping or stream. An
elevated WDK rebuild compiled the header into the x64 driver and again reported
zero signability errors/warnings. No control device is registered and no
IOCTL was sent to the system.

The driver now contains the first secured control-device scaffold in
`Source/Main/adapter.cpp`. `IoCreateDeviceSecure` uses the explicit
`D:P(A;;GA;;;SY)(A;;GA;;;BA)` ACL, and create/close/device-control dispatch plus
symbolic-link cleanup are defined. Open and heartbeat requests are validated
against the bounded ABI. The x64 WDK rebuild passed with
`wdmsec.lib`, zero signability errors/warnings, and catalog generation. The
control device was not registered or loaded; this is compile evidence, not live
driver evidence.

`NativeBridgeSession` now enforces the negotiated lease: reads and writes fail
after expiry, and a late heartbeat cannot revive the old generation. The expiry
regression uses a forward synthetic `Instant`, avoiding monotonic-clock
underflow. The focused Windows-audio suite passed 46 tests and strict Clippy
passed. The kernel now owns one exact-identity lease: open claims it, heartbeat
refreshes it, close releases it, and expired ownership is reclaimed. Shared
memory and endpoint audio are still separate gates.

The user-mode Windows adapter now contains an explicit
`NativeBridgeControlClient` matching the driver's fixed request layout and
IOCTL numbers. It validates the hello before encoding it, bounds the UTF-16 bus
ID, opens the secured device only when requested, and closes the handle via
RAII. The layout regression is included in the 48-test Windows-audio suite.
The client has not been run against a loaded driver because installation and
loading remain outside this non-mutating validation scope; no live audio-path
claim is made.

`NativeBridgeController` now composes the explicit control client and mapped
session: it claims the kernel lease before block publication, forwards
heartbeats, closes the lease before flushing, and retries release on drop/error
paths. This is a user-mode lifecycle integration boundary. The kernel still
does not receive the file mapping, so live shared-memory/audio transport and
driver-loaded validation remain open.

The kernel mapping boundary is now implemented for mapped opens: a nonzero
section handle is checked for the negotiated PCM size, referenced with
`UserMode` access, mapped into system space, and released on contention, close,
expiry replacement, and unload. Lease-only opens remain unmapped by design.
The updated WDK build passed with zero signability errors/warnings. No mapped
open was sent to a loaded driver, so live kernel/audio transport evidence is
still outstanding.

Mapped opens now also verify the actual system-space view size returned by the
kernel mapping API and unwind a truncated view before accepting the lease. The
updated non-installing WDK build passed with zero signability errors/warnings;
the focused Windows-audio suite passed 48 tests.

The fixed bridge ABI now includes an optional section handle and mapping byte
count. Kernel and user-mode validators reject half-specified or undersized
mapping descriptors, while the client exposes an explicit mapped-open method.
The current lease-only path remains valid for control testing; no zero handle is
interpreted as mapped audio. The updated elevated WDK build passed with zero
signability errors/warnings and catalog generation.

The controller now retains the section handle for the complete lease and sends
the same handle/size pair on mapped heartbeat and close, preventing a mapped
lease from being accidentally downgraded to a lease-only request. The focused
Windows-audio suite passed 48 tests. End-to-end execution remains gated on a
deliberately loaded driver.

The temporary section regression maps the section handle and observes generation
bytes written through the broker region view, proving the two views share the
expected file-backed bytes. It performs no device or audio access.

## Native bridge contract

`audiorouter-protocol` now defines a versioned `AudioBridgeHello` and bounded
`AudioBridgeBlockHeader`. Validation rejects incompatible protocol majors,
overlong bus IDs, zero generations, unsupported rates/channels/quantum sizes,
invalid lease durations, and payload lengths that do not exactly match the
declared f32 block shape. `cargo test -p audiorouter-protocol --locked` passed
all 8 tests. Native shared-memory/IOCTL execution remains a separate gate.

## Failed attempts and fixes

The first build attempt failed because forcing `OutDir` and `IntDir` into one
temporary directory invalidated the sample projects' relative library paths;
it also hit a WDK `InfVerif.dll` x86-load exception. The build script now keeps
project-relative outputs, uses a separate log directory, disables the known
local verifier/API-validation hooks for this build-only qualification, and
leaves installation/signing explicit future gates.
