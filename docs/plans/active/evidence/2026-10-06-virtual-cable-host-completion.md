# Virtual cable host completion and VM readiness — 2026-10-06

Host-only work requested by the user ("verify everything around the driver,
add more tests, complete the specs, document the VM setup in detail"). No
driver was installed, loaded or trusted on this PC; the helper and the VM
tools were never run elevated here. Nothing below is VM, endpoint, audio or
sound-quality evidence.

## Verification findings and fixes

| # | Finding | Fix | Commit |
| --- | --- | --- | --- |
| 1 | The VM fuzzer still spoke protocol 1.0 (176-byte OPEN, 32-byte header): every OPEN was refused, so a Verifier run would never reach lease/mapping/teardown code, and new refusal codes counted as unexpected | 240-byte FLOAT64 request, 128-byte header, extension/minor/cable/prefix/QUERY fuzzing, coverage guard (fails when no OPEN or QUERY ever succeeds) | `2f24f66b` |
| 2 | VM runner defaulted to the removed prototype endpoint names | Default `-EndpointProfile Cables` | `2f24f66b` |
| 3 | Windows composes endpoint names as `<description> (<interface name>)` (seen on this host: `CABLE Output (VB-Audio Virtual Cable)`); exact-name matching would fail A3 | Runner, helper and inventory accept the exact or composed form and record the actual names | `dc8569dd` |
| 4 | 17 §5.4/§5.5 not implemented: low-latency packet constraints, MinPeriodFrames/DefaultPeriodFrames/MaxLeaseMs | Implemented (DEVPKEY_KsAudio_PacketSize_Constraints2 per wave interface; registry reads; lease cap); decisions for names, default period and stale silence in 17 §5.5 | `dc8569dd` |
| 5 | WP-07 helper missing | `crates/driver-helper` (SetupAPI, WinTrust catalog verification, MMDevice names), runner `-Helper` | `83276370` |
| 6 | A wall-clock producer against the single-block capture-sink slot drifts against the driver's QPC clock (and Windows' 15.6 ms default sleep), producing underruns/gaps that would look like driver bugs | Driver acknowledges consumed capture-sink blocks in `ReaderSequence`; tone tool paces on that acknowledgement at 1 ms timer resolution and snapshots counters when the final block is taken | this commit |
| 7 | The development PC is Windows 11 Home: no Hyper-V for the planned VM | VirtualBox-based [VM guide](../../../operations/virtual-cable-vm-guide.md), `tools/vm/prepare-vm-share.ps1`, `tools/vm/vm-checks.ps1` | this commit |
| 8 | VM tools imported `VCRUNTIME140.dll`, absent on a clean VM | Prepare script builds them with `+crt-static` (verified: no import) | this commit |

## Checks run (Windows 11 Home 26300, WDK 10.0.28000.0)

- Driver host unit tests (`drivers/audiorouter-virtual/tests/build-tests.ps1`): 215 passed.
- `tests/acceptance/m03-driver-build.ps1` x64 and ARM64: passed (new guards: packet constraints wired and set before filter install, registry config applied, lease cap, capture-sink acknowledgement).
- `tests/acceptance/m03-driver-vm-guards.ps1`: 28 checks passed; real runner and `vm-checks.ps1` refuse on the host.
- `cargo test -p audiorouter-driver-helper`: 25 passed (fake-platform command flows, real repository INF identity).
- `cargo test -p audiorouter-windows-audio`: 118 passed, 1 ignored; examples `m03_bridge_tone` (4) and `m03_cable_inventory` (2) passed.
- `m03_cable_inventory --match CABLE` on this host (read-only, VB-Cable): engine periods default = min = 480 frames, 15/60 formats, composed names; exit 1 as designed.
- `tools/vm/prepare-vm-share.ps1` end to end into a scratch folder: test-signed package (DriverVer 0.1.0.0 = package.json driverVersion, signed = test), package integrity checks, four static-CRT tools, scripts, manifest.
- Documentation check: 129 files, 683 links.

## Still open

All stage A VM checks (see the guide's sessions 1–5), helper `update`/
`repair` with a second package version, WP-08 to WP-15. Independent fresh-
context review of the kernel changes since WP-04 (QUERY, counters, view
initialization, registry configuration, capture-sink acknowledgement) is
still required by WP-04's acceptance and has not been done.
