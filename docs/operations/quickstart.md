# AudioRouter development quickstart

This repository contains a tested control-plane and UI foundation, not a
releasable audio router yet. Native end-to-end routing, managed virtual
devices, the installer, and production signing remain unavailable. The steps
below are safe, offline development checks: they do not change Windows audio
defaults, volume, mute, privacy settings, drivers, or endpoint state.

## 1. Prepare the toolchain

Use a Windows 11 x64 machine with the repository's Rust toolchain and Visual
Studio C++ tools. The native SDK/WDK dependency versions and the repository-
local VST3 SDK checkout are documented in [SDK setup](sdk-setup.md).

To download or repair the pinned VST3 SDK checkout:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\m06-vst3-sdk\install.ps1
```

This downloads source into `third_party\vst3sdk`; it is not a system-wide
installation and does not register plugins.

## 2. Run the safe acceptance checks

From the repository root:

```powershell
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m07-headless.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m08-release.ps1
Push-Location contracts
npm.cmd run typecheck
npm.cmd run check:drift
Pop-Location
```

The M07 check exercises the portable control, CLI, MCP, and plugin-worker
boundaries. The M08 check creates and removes a disposable unsigned artifact
directory. Neither check opens an audio stream or installs a driver.
The contracts checks verify TypeScript type safety and catalog parity; they use
only the local CLI schema and do not access audio or machine configuration.

To run the complete non-live acceptance chain in milestone order, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\safe-all.ps1
```

This includes native compile and read-only format checks, disposable SysVAD
reference qualification, M01/M04/M05/M06/M07/M08 acceptance, and documentation
validation. It deliberately excludes all live-audio wrappers.

For an explicitly authorized native adapter smoke on a Windows host, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\\tests\\acceptance\\m02-rust-adapter-live.ps1 -AllowLiveAudio -DurationMilliseconds 200
```

This opens bounded capture and render streams, copies capture data into
caller-owned memory, submits zero-valued render buffers, stops/resets both
streams, and verifies the media-device identity/state snapshot is unchanged.
It is intentionally opt-in and must not be treated as physical latency or
graph-routing evidence.

For an explicitly selected compatible digital cable pair, the route smoke
passes captured frames through the generation-1 graph into the render client:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\\tests\\acceptance\\m02-rust-adapter-route-live.ps1 -AllowLiveAudio -DurationMilliseconds 500
```

This is opt-in live testing only; it does not change defaults, volume, mute,
privacy, drivers, signing, or startup configuration.

To inspect the active endpoint mix formats without opening an audio stream, run
the read-only acceptance:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m00-native-format-inventory.ps1
```

The route wrapper also accepts `-CaptureEndpointId` and `-RenderEndpointId`
together. Use the opaque IDs from the native inventory when testing a
differing-rate pair; this bypasses friendly-name resolution while preserving
the adapter's exact ID, direction, and format checks.

For the repository-local VST3 SDK build, validator, and offline fixture loader,
run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst3-sdk.ps1
```

MSBuild FileTracker may require an elevated shell on managed Windows hosts.
Elevation is used only for the disposable build; it does not authorize driver
installation or audio changes.

## 3. Explore the offline control surface

The CLI exposes honest capability discovery and configuration-only operations:

```powershell
cargo run -p audiorouter-cli -- help
cargo run -p audiorouter-cli -- status --json
cargo run -p audiorouter-cli -- nodes types --json
cargo run -p audiorouter-cli -- presets list --json
```

The status output may report audio and virtual-device capabilities as
`unavailable`. That is expected until the native graph and managed driver are
implemented and qualified; it is not a failed installation.

## 4. Use the UI and MCP safely

The UI can inspect the disconnected/demo state, edit a local draft, inspect
routes, and display device/application/recording metadata when connected to a
backend. Draft changes are not committed until an authorized plan/apply flow.
The [headless runbook](headless-runbook.md) documents the local MCP stdio
adapter, optional authenticated named-pipe proxy, backups, imports, exports,
and recovery boundaries.

Do not point a client at a production-looking audio workflow yet. Do not use a
third-party virtual cable as if it were an AudioRouter-managed endpoint.

## Troubleshooting

- Earlier Rust WASAPI initialization runs returned `E_INVALIDARG` on the
  event-driven request. The current adapter retries only that exact HRESULT
  with its qualified bounded polling request; the live smoke now succeeds on
  all tested capture endpoints. `E_INVALIDARG` remains distinct from
  `AUDCLNT_E_DEVICE_IN_USE`, and ordinary tests do not work around either
  error by changing device settings.
- The backend may report audio as unavailable even though portable graph and
  DSP tests pass; the remaining unavailable capability is the native realtime
  scheduler and endpoint routing integration.
- MSBuild FileTracker access errors are host/tool-process restrictions. Retry
  the same SDK acceptance command in an approved elevated build shell; do not
  disable Windows security features.
- A release manifest that says `signed: false` or `publicationReady: false` is
  correct for this development repository. Signing, installer creation, driver
  packaging, and clean-machine qualification are still release gates.
- If a command reports a missing or revoked grant, enroll the client through
  the authorized local control workflow. A generic read grant must not be
  widened to recording, device administration, or plugin scanning.

For release, backup, restore, and uninstall expectations, see [release
qualification](release-qualification.md). For the exact implemented versus
blocked evidence, see the [active plan](../plans/active/current.md).
