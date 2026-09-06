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
```

The M07 check exercises the portable control, CLI, MCP, and plugin-worker
boundaries. The M08 check creates and removes a disposable unsigned artifact
directory. Neither check opens an audio stream or installs a driver.

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

- `E_INVALIDARG` from the Rust WASAPI initialization path is a known native
  interop blocker. The native C++ reference path succeeds farther; ordinary
  tests keep audio unavailable and do not work around this by changing device
  settings.
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
