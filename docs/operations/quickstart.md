# AudioRouter development quickstart

This repository contains a tested control plane, UI, and VB-Cable-first
existing-device routing path, but it is not a releasable Windows installer
yet. AudioRouter-owned managed virtual devices, the installer, and production
signing remain unavailable. The steps below are safe development checks: they
do not change Windows audio defaults, volume, mute, privacy settings, drivers,
or endpoint state.

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

To build the native Tauri shell and its embedded current UI without launching
or packaging it, run from the repository root:

```powershell
npm.cmd run build --prefix ui
cargo build --manifest-path src-tauri/Cargo.toml
```

The shell executable is written to `src-tauri\target\debug\audiorouter-shell.exe`.
This is compile-only evidence; it does not install a driver, register startup,
open an audio stream, or alter machine audio configuration. The release flow
rebuilds the UI automatically before its optimized shell build.

For a human-testable VB-Cable desktop run, use the disposable launcher after
building the CLI and shell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\run-vb-cable-desktop.ps1
```

Add `-Build` to build the UI and shell first. The launcher performs a read-only
inventory, refuses ambiguous or missing VB-Cable endpoints, creates a database
under `%TEMP%`, and grants device administration only to that temporary
process. In the UI, select the exact capture/render endpoints, prepare them,
plan/commit the graph, and start the session. Use the tray **Quit and stop
audio** action when finished; the launcher removes its database and restores
the caller's environment. It does not change Windows defaults, endpoint
volume/mute, driver state, or startup registration.

The fresh desktop session starts with a neutral 0 dB Gain node between the
selected input and output. Adjusting that node is a real graph change: use
**Plan changes**, commit the validated plan, and restart the session if it was
already running. The same path is used for EQ, gate, compressor, limiter, and
other built-in processors added from the canvas.

The defaults are `CABLE Output (VB-Audio Virtual Cable)` capture and `CABLE
Input (VB-Audio Virtual Cable)` render for deliberate loopback testing. To use
the normal VB-Cable-capture-to-physical-output route, pass one exact active
render endpoint ID selected from `devices list --json`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\run-vb-cable-desktop.ps1 -RenderEndpointId '{0.0.0.00000000}.{endpoint-guid}'
```

The override refuses missing, inactive, ambiguous, or non-render IDs. An
occupied endpoint remains an explicit backend `deviceInUse` failure; the
launcher never substitutes another output.

Both directions can be selected explicitly when qualifying another deliberate
pair:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\run-vb-cable-desktop.ps1 -CaptureEndpointId '{0.0.1.00000000}.{capture-guid}' -RenderEndpointId '{0.0.0.00000000}.{render-guid}'
```

Each ID must match exactly one active endpoint of its requested direction.

### Route into Voicemeeter or another existing tool

On a machine with Voicemeeter or VB-Cable already installed, use its existing
Windows endpoints as the tool boundary. Select an existing virtual render
endpoint (for example `CABLE Input` or `Voicemeeter Input`) as an AudioRouter
output, and select the matching virtual capture endpoint (for example `CABLE
Output` or `Voicemeeter Out B1`) in the receiving tool. Physical microphone
and desktop captures can then be connected to that output through the same
validated graph. In the editor, drag the capture into the **Existing virtual
output** shelf destination (or a physical output), select the exact render
endpoint in Endpoint binding, then use **Plan changes**,
commit the draft, prepare the exact endpoints, and start the session.

This workflow uses the installed third-party virtual driver and does not
provision an AudioRouter-owned kernel endpoint. It is the supported
current-machine VB-Cable-first path. AudioRouter-owned virtual-bus
provisioning remains deferred to the separately signed managed-driver profile;
that deferral does not prevent routing through explicitly selected existing
VB-Cable, Voicemeeter, physical WASAPI, or other installed virtual endpoints.

For an explicit application-capture qualification, use the guarded wrapper
with the process identity returned by `apps list --json`. Pass the executable
basename to `-Executable` and the verified full path separately to
`-ApplicationPath`; the backend compares both fields independently:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m02-control-application-live.ps1 `
  -AllowLiveAudio -ProcessId 34568 -Executable 'voicemeeterpro.exe' `
  -CreationTime100ns 134336595287373468 `
  -ApplicationPath 'C:\Program Files (x86)\VB\Voicemeeter\voicemeeterpro.exe'
```

The wrapper requires an explicit identity, defaults only to one exact active
CABLE Input render endpoint, snapshots media devices before and after, runs two
bounded worker cycles, restores process environment values, and never changes
endpoint defaults, volume, mute, or the selected process.

The same `run-vb-cable-desktop.ps1` file is included beside the executables in
the prepared unsigned release directory, so an extracted development artifact
can be started without a repository checkout.

### Verify a signal without using a microphone

For a bounded, explicitly authorized raw signal-path check, run an elevated
PowerShell session from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m00-native-loopback.ps1 -AllowLiveAudio
```

The harness generates a tone on `CABLE Input (VB-Audio Virtual Cable)`,
captures `CABLE Output (VB-Audio Virtual Cable)`, and requires nonzero captured
payload. It snapshots media-device state before and after and removes its
temporary probe artifacts. This verifies the existing-device path only; it
does not prove AudioRouter graph activation, UI meters, or an attended
microphone route.

For the intended UI workflow, the next M05 slice is a graph-native Test Signal
node with destination meters. Until that node is implemented, use the raw
smoke above or play a known tone from an already-running application through a
deliberately selected existing endpoint. VoiceMeeter may remain open; stop or
close it only if it owns the exact endpoint AudioRouter must prepare, and treat
`deviceInUse` as an ownership diagnostic rather than changing defaults.

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
reference qualification, M01/M04/M05/M06/M07/M08 acceptance (including the
disposable unsigned NSIS installer smoke), the repository-
owned x64 VST2 state/legacy-entry-point fixture, and documentation validation.
It deliberately excludes all live-audio wrappers and third-party plugin
fixtures.

The repository-owned VST2 fixture can also be qualified directly. It compiles
ignored x64 DLLs for the modern `VSTPluginMain` and legacy `main` exports, then
checks chunk state plus contained invalid-output, crash, and hang behavior:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst2-state-fixture.ps1
```

This does not register a plugin or alter audio configuration. User-installed
VST2 fixtures remain opt-in and are not redistributed.

For an explicitly authorized native adapter smoke on a Windows host, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\\tests\\acceptance\\m02-rust-adapter-live.ps1 -AllowLiveAudio -DurationMilliseconds 200
```

This opens bounded capture and render streams, copies capture data into
caller-owned memory, submits zero-valued render buffers, stops/resets both
streams, and verifies the media-device identity/state snapshot is unchanged.
Run it from an elevated PowerShell session; the harness refuses before its
PnP snapshot otherwise. It is intentionally opt-in and must not be treated as
physical latency or graph-routing evidence.

For an explicitly selected compatible digital cable pair, the route smoke
passes captured frames through the generation-1 graph into the render client:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\\tests\\acceptance\\m02-rust-adapter-route-live.ps1 -AllowLiveAudio -DurationMilliseconds 500
```

This is opt-in live testing only; it does not change defaults, volume, mute,
privacy, drivers, signing, or startup configuration.

For an explicitly selected installed x64 VST2 effect on the guarded
mic-to-VB-Cable route, use the M02 NFR-02 wrapper with an absolute plugin path:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m02-nfr02-mic-virtual-capture.ps1 `
  -AllowLiveAudio `
  -PluginPath 'C:\Program Files\VSTPlugins\ReaPlugs\reaeq-standalone.dll'
```

In the connected UI, open the canvas shelf's “Plugin (VST2/VST3)” picker,
explicitly scan the plugin's folder, add or insert the verified result, select
its node, then edit the worker-described parameter controls in the inspector.
Plan and commit the graph before starting the session; plugin nodes begin as
stopped placeholders. VST2 binaries such as ReaPlugs do not publish VST3 class
IDs, so the picker uses a neutral authoring placeholder while execution remains
bound to the exact rescanned path and SHA-256. The vendor's native editor window
is not wired into this build; generic parameter controls are the supported
configuration UI.

To exercise one non-default ReaEQ setting in the guarded live route, use its
worker-reported `1-Gain` parameter (ID 1) at normalized value 0.75:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m02-nfr02-mic-virtual-capture.ps1 `
  -AllowLiveAudio `
  -RouteDurationMilliseconds 12000 `
  -PluginPath 'C:\Program Files\VSTPlugins\ReaPlugs\reaeq-standalone.dll' `
  -PluginParameterId 1 `
  -PluginParameterValue 0.75
```

Run in elevated PowerShell so the before/after media snapshots work, and only
with the physical loopback connected. This scans the selected DLL's containing
directory, inserts that exact verified plugin into the temporary production
graph, and requires 500 paired signals, p95 no greater than 160 ms, a running
plugin worker with zero reported failures, an unchanged binary fingerprint,
and unchanged media-device state. It restores the caller's
`AUDIOROUTER_PLUGIN_WORKER_PATH`, removes temporary route/recording artifacts,
and does not change Windows defaults, endpoint volume/mute, privacy, driver,
signing, or startup configuration. ReaEQ remains a user-installed fixture and
is not copied or redistributed.

For the guarded multi-input/many-output acceptance, open an elevated PowerShell
session and provide the explicit live-audio switch. The harness refuses before
endpoint inventory when the process is not elevated:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m02-multi-input-native-live.ps1 -AllowLiveAudio
```

The selector chooses exact active stereo 48 kHz endpoints; supply
`-CaptureEndpointIds` and `-RenderEndpointIds` with `|`-separated opaque IDs
when deterministic endpoint selection is required. The run is bounded and
process-scoped, and does not change persistent audio configuration.

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

The status output reports the native graph and the explicitly discoverable
existing endpoints available on the current machine. AudioRouter-owned
virtual-device provisioning may still report `unavailable` because it belongs
to the deferred managed-driver profile; that is not a failed installation and
does not disable existing-device routing.

## 4. Use the UI and MCP safely

The UI can inspect the disconnected/demo state, edit a local draft, inspect
routes, and display device/application/recording metadata when connected to a
backend. Use the Gate, Parametric EQ, Compressor, or Limiter action on any
connected draft path to insert that processor between the existing nodes; the
insertion and its parameters are real draft changes, not canvas-only
decoration. The Presets panel can also expand EQ presets and the voice-chain
presets into ordinary draft nodes. A voice chain is inserted into the first
compatible connected path when one exists; otherwise its nodes remain
unconnected for explicit user wiring. Draft changes are not committed until an
authorized plan/apply flow.
The Session transfer panel exports the selected stopped configuration as a
local `.audiorouter.json` file. Import first validates the file through the
backend, then requires the separate Commit stopped import action; imported
sessions remain stopped and require endpoint rebinding/review. The transfer
does not include credentials, grants, recordings, plugin binaries, or machine
authorization. The UI JSON transfer is separate from the versioned
`.audiorouter` ZIP bundle exposed by the headless `export-bundle` and
`import-bundle` commands.
If another client changes the session before commit, the UI reports the typed
revision conflict and structured remediation, refreshes the authoritative
session, and clears the stale warning/commit state. Review the refreshed draft
before planning again; it never retries the old plan automatically.
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
- The backend may report AudioRouter-owned managed-device capability as
  unavailable even though existing-device routing, portable graph, and DSP
  tests pass; the remaining unavailable capability is the signed managed
  driver profile, not the VB-Cable-first endpoint boundary.
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
