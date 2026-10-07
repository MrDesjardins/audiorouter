# AudioRouter development quickstart

Use Tools → **Group** to add a named, rounded visual background. Properties
edits its color, caption font size (18 px by default), and opacity (5% by
default, with a 1–100% integer slider). Drag its caption or background to move
it and resize its border. Groups stay beneath audio nodes/wires and save automatically in this
PC's per-session canvas layout. They do not change audio or require Save/Stop;
session-file exports currently carry the audio graph, not these annotations.

The new **API** tab starts optional localhost HTTP control and opens its bundled
Swagger page. See [HTTP setup and examples](local-http-api.md).

Bass & Treble now includes the main voice frequencies: bass defaults to
500 Hz and treble to 1500 Hz, with gain from −12 to +12 dB. In Properties,
raise **Bass frequency** or lower **Treble frequency** to affect more of
your voice. A warm/dark starting point is Bass +12, Treble −12; reverse
them for a thinner, brighter voice. Saved nonzero settings sound stronger
than earlier builds. Set the frequencies to 120 and 6000 Hz to restore
the original tuning. Zero gain remains neutral.

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
cargo build --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target
cargo build --manifest-path Cargo.toml -p audiorouter-plugin-host --bin audiorouter-plugin-worker --target-dir src-tauri/target
```

The shell and matching plugin worker are written beside each other in
`src-tauri\target\debug\` as `audiorouter-shell.exe` and
`audiorouter-plugin-worker.exe`. Keep both executables together; building the
shell alone leaves plugin nodes unable to start. The shell also explains the
expected worker locations and repair steps if it is missing.
This is compile-only evidence; it does not install a driver, register startup,
open an audio stream, or alter machine audio configuration. The release flow
rebuilds the UI automatically before its optimized shell build.

For optimized canvas startup/connection regressions without opening audio
devices, run `npm.cmd run e2e:production-canvas --prefix ui`. It checks saved
lines and actual drag creation, including recovery from a missed first
connector measurement in all three themes. This supplements native first-launch
manual testing; browser fixtures do not prove the native startup timing.

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
  **Save route**; the backend validates it and saves it automatically when there
  are no warnings. Review and confirm any warnings. Restart the session if it was
already running. The same path is used for EQ, gate, compressor, limiter, and
other built-in processors added from the canvas.

An ordinary input port accepts one incoming connection. If a Physical output
already receives another source, dragging Test Signal to that input shows the
current source and offers **Replace input connection**. This changes only the
draft; **Undo** restores the old edge, and **Save route** persists the chosen
route. To combine both sources in the graph, add a visible Mixer and connect
them through it rather than stacking two edges on the output port. Native
activation of a combined route still requires its own validation. The current
single-endpoint engine supports one stereo Physical Input and one stereo Test
Signal feeding the same Mixer and one Physical Output. Other source mixes may
still return an unsupported-route message before audio starts.

While the session plays, **Enabled** and **Bypass** on an already prepared
tool apply immediately. No separate Save or Stop is needed. Other unsaved
names, settings or connections stay in your draft. A new source that was not
prepared still requires saving and preparing the route before it can play.

For separate voice, game and recording feeds, keep these routes distinct:

- Microphone → voice effects → CABLE-A Input; Discord microphone = CABLE-A Output.
- Siege playback → CABLE-B Input; CABLE-B Output → Siege EQ → Scarlett.
- Discord capture + Siege EQ → Mixer → a **different** recording cable's Input;
  select that cable's Output in your recording application.

Do not send Mixer output back to CABLE-B Input when CABLE-B Output already
feeds Siege EQ. That makes an endless return through EQ and Mixer, even if the
output node is named “Outplayed”. AudioRouter checks known VB-Cable returns and
names the offending source/output in a Save warning and before starting.
You can acknowledge the warning to keep the selected configuration; playback
remains blocked while that known return is connected. Keep the recording mix out
of CABLE-A too, so callers receive only processed microphone audio.

Discord's Mic Test plays your selected microphone through Discord's output.
Application capture includes that test playback along with other Discord
sounds; it cannot separate other people's speech. If Scarlett is Discord's
speaker output, capture keeps that original playback. Send the capture to the
separate recording feed rather than Scarlett again to avoid duplicate playback.
See [Discord Mic Testing](https://support.discord.com/hc/en-us/articles/360020641332-Mic-Testing)
and [Microsoft process-loopback capture](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/).

The **Input device** tool accepts a Windows capture endpoint, whether it is a
microphone or an installed virtual capture bus. The **Output device** tool
accepts a Windows playback endpoint. Existing virtual devices are chosen in
these device pickers. To route system sound whose Windows default playback
device is **Voicemeeter Input**, enable **B1** on the Voicemeeter strip
receiving that sound. In AudioRouter, select **Voicemeeter Out B1** as the
Input device capture endpoint, connect it to the Output device, and select
the intended speakers (for example, Focusrite) as the output endpoint.
Voicemeeter Input itself is a playback endpoint, so it does not appear in the
AudioRouter capture picker. AudioRouter does not change Voicemeeter's B1
switch or the Windows default device. This B1 path requires the Voicemeeter
mixer to run: the virtual driver exposes the endpoints, while Voicemeeter
produces the B1 mix. To route system sound without running Voicemeeter, set
Windows playback to **CABLE Input**, then select **CABLE Output** as the
AudioRouter capture endpoint and the intended speakers as its output.

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

### One session with several independent paths

A session can hold several separate paths that run together, for example a
microphone through voice plugins to a virtual cable, and a game cable through
an EQ to the headphones. Connected Mixers can combine explicit branches in
stages, with tools between them and separate outputs before or after each mix.
For example, game + Discord can feed a recording output and a second Mixer
that adds the microphone for headphones; the microphone-only cable stays
separate. Each shared tool runs once and each Mixer keeps its input volumes.
A source's final tool before
a Mixer may also feed direct outputs: its processed signal is reused before
Mixer input gain, without the other mixed sources. Paths never mix unless you
add a Mixer. Choose each Input device and Output device node's device in its
**Properties**; the choice is saved on that node. The same output device may be
used by several paths. **Save** the session, then press **Play**: every path
starts and stops together. Up to 8 sources and 8 outputs are supported in one
session. A mono microphone connected to a stereo Input device node is copied to
both channels.

Tools without an enabled input connection, and any downstream nodes fed only
by those tools, show a warning and are ignored during playback. They stay on
the canvas and in the saved session; other connected routes can still play.
Reconnect an input and save to include that chain again.

To connect Advanced EQ to a Mixer, start dragging at the **EQ's blue sending
dot**, then drop on the **Mixer's orange receiving dot**, which appears during
the drag. Every source and tool follows that same direction. Then drag from
the **Mixer's blue dot** to the **Physical Output's orange dot**. An EQ output
can feed Scarlett directly and also feed a Mixer that combines it with Discord
for another output. Adding the Mixer branch preserves the Scarlett connection.
If an ordinary receiving input is occupied, the notice offers **Replace input
connection**; **Undo** restores its previous source, and **×** keeps its existing
connection. Mixers accept multiple sources explicitly.

While it plays, the **Timing** tab shows, for each output, how long the sound
spends at each step in travel order: how long a source's audio waits before it
is picked up, the delay each tool adds (a plugin's worker queue included), and
how much audio is queued ahead of the output device. The longest bar marks the
slowest step. Timing is measured for Mixer and multi-path routes.
Long timing lists scroll inside the sidebar; opening Timing keeps the canvas
in place. Warning and action notices stay above the workspace with their
buttons accessible even when both are present.

### When Play says audio is not prepared

**Backend ready** confirms the control connection. To play a route:

1. Select each Input Device and Output Device node and choose its device in
   **Properties**. AudioRouter never chooses a microphone automatically.
   Existing physical/VB-Cable endpoints do not need the AudioRouter virtual
   driver. A Test Signal or Audio File route needs no input device: once it is
   saved with its output chosen, Play runs it without opening any microphone.
   (**Advanced → Troubleshooting: manual device binding** remains for device
   diagnostics, loopback tests, and as a fallback for a route whose nodes have
   no device.)
2. Press **Play** in the header to activate the visible canvas route. Unsaved
   changes to a route with an input device are previewed without saving; a
   route with several paths, or generated sound only, plays its saved version.
   Test Signal and Audio File stay stopped until played on their nodes.
   **Save** when you want to keep the route.
3. Press **Play** on a Test Signal or Audio File node to start that source.
   Its **Stop** button stops only that source; header **Stop** stops the route.
   If an endpoint is unavailable or occupied, resolve that named endpoint's
   error and retry.

The first time you press **Play** on a computer, AudioRouter asks **Allow
AudioRouter to use your audio devices?**. Choose **Allow and play**; it does
not ask again. **Setup → Audio device access** withdraws or gives it later.
Only the AudioRouter window can give this permission; CLI and MCP clients
never receive it. The older developer variable
`AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` still works but is no longer needed.

New built-in processing nodes use stereo ports to match the Test Signal,
microphone and physical-output defaults. A mono microphone capture is copied
to both stereo graph channels at the WASAPI boundary, so a single-channel
device such as the PD200X can feed stereo processors and a stereo VB-Cable
output. The graph remains stereo after that explicit conversion. Older saved
routes containing a stereo-to-mono connection may still be rejected at Start;
rebuild those paths with matching channel counts and save them. An endpoint
already owned by another application reports a device-in-use error; select
another exact output or release that endpoint before preparing again.

Background refreshes keep local edits. If another client commits a newer
revision, the editor preserves the draft and reports a conflict. Copy any edits
you want to keep before choosing **Discard edits** to load the saved revision.

### Route into Voicemeeter or another existing tool

On a machine with Voicemeeter or VB-Cable already installed, use its existing
Windows endpoints as the tool boundary. Select an existing virtual render
endpoint (for example `CABLE Input` or `Voicemeeter Input`) as an AudioRouter
output, and select the matching virtual capture endpoint (for example `CABLE
Output` or `Voicemeeter Out B1`) in the receiving tool. Physical microphone
and desktop captures can then be connected to that output through the same
validated graph. In the editor, drag the capture into the **Existing virtual
output** shelf destination (or a physical output), select the exact render
endpoint in Endpoint binding, then use **Save route**, prepare the exact
endpoints, and start the session.

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

In the UI, add a **Test Signal** source (Tools → Inputs), connect it to an
output, save, and press Play; destination meters and connection activity show
the signal. VoiceMeeter may remain open; stop or close it only if it owns the
exact endpoint AudioRouter must prepare, and treat `deviceInUse` as an
ownership diagnostic rather than changing defaults.

### Stream audio to another computer (Network Send / Network Receive)

Use this to send audio from one PC to another on the same local network, for
example game audio from a gaming PC to a streaming PC. AudioRouter must run on
both computers.

1. On each PC, find its IP address: run `ipconfig` and read the **IPv4
   Address** (for example `192.168.1.20`).
2. On the **sending** PC, add **Network Send** (Tools → Outputs → Network) and
   connect your source to it (for example the game's capture or a Mixer). In
   its Properties, enter the *receiving* PC's IP address. Keep the default port
   47800 unless another program uses it. Save and press Play.
3. On the **receiving** PC, add **Network Receive** (Tools → Inputs → Network)
   and connect it to an output: speakers, or a virtual cable that OBS
   captures. In its Properties, enter the *sending* PC's IP address and the
   same port. Save and press Play. The first time, Windows Firewall may ask
   whether AudioRouter may use the network; allow **private networks**.
4. The receive node's Properties show `Receiving · … packets · … ms buffered`.
   "Waiting for audio" means nothing is arriving. Check both IP addresses, the
   port, that both sessions are playing, and the firewall. Packets from any
   other address are ignored and counted.
5. If AudioRouter audio arrives from a different address than the one you
   entered (for example the sending PC also has Wi-Fi and Ethernet), the
   receive node names that address and offers **Use 192.168.x.y**. One click
   switches to it while playing.
6. Pair the two computers (recommended). On one PC, select the node and
   choose **Generate** under **Pairing key**, then **Copy**. Enter the same
   key in the **Pairing key** of the node on the other PC, and save both.
   The key is 16–128 characters; a generated one is 24 random characters.
   While the key is blank, the node shows "Not paired": the receiving PC
   then plays any AudioRouter audio that appears to come from the sender's
   address, which another device on the network can fake.

With a pairing key, every packet carries a tag made with that key, and the
receiving PC plays only packets with the right tag that it has not played
before. If the keys differ, or only one PC has a key, nothing is played and
the receive node says which: for example "Waiting for audio · 40 packets had
no pairing key" with "The sending computer has no pairing key. Copy this key
into its Network Send." A key change applies while playing. Both PCs need an
AudioRouter version with pairing; an older version cannot send to, or
receive from, a paired node.

The sending PC does not need a speaker or headphone output in its session:
**Microphone → (tools) → Network Send** alone is a complete route.

Address, port and buffer changes apply while audio plays; there is no need to
Stop and Play again. If nothing arrives on the receiving PC and the node does
not name another address, the receiving PC's Windows Firewall is the usual
cause. Open *Windows Security → Firewall & network protection → Allow an app
through firewall* and make sure AudioRouter is allowed on **Private**
networks, and that your network is set to Private, not Public.

The stream is uncompressed 48 kHz float audio (about 3 Mbit/s for stereo).
Pairing stops other devices from injecting or replaying audio, but the audio
is **not encrypted**: anyone on the same network can still listen to it. Use
it only on a network you trust. The pairing key is saved in the session (and
in an exported session bundle), never in AudioRouter's logs or diagnostics;
treat a shared bundle like the key itself. On Wi-Fi, raise
**Buffer (ms)** on the receiver (for example to 80) if you hear gaps; the
buffer is the added delay.

### Check audio quality (crackling)

Audio continuity is measured, not judged from builds. With VB-Cable installed,
the following plays a quiet tone into `CABLE Input`, routes `CABLE Output`
through AudioRouter to `CABLE-B Input`, records `CABLE-B Output`, and counts
every discontinuity sample by sample. Nothing reaches speakers unless
"Listen to this device" is enabled on a cable.

```powershell
:AUDIOROUTER_LIVE_CONTINUITY = "1"
cargo test -p audiorouter-transport --test live_audio_continuity -- --ignored --nocapture
```

Options such as processor chains, the single-endpoint worker, real plugin
nodes from a database copy, Test Signal and network routes are listed in the
[continuity evidence](../plans/active/evidence/2026-09-26-audio-continuity.md#method).
A passing run reports `0 glitches` for both the reference and the routed
result.

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

In the connected UI, open the Tools tab: its "Plugins (VST2/VST3)" group lists
every supported plugin from remembered scans (scan results persist across
restarts). Use **Scan standard folders** for Windows' usual VST3/VST2 folders,
or **Scan another folder…** (the canvas shelf's “Plugin (VST2/VST3)” picker) for
any other folder. Click a plugin to add it, or insert it on a connection, then
select its node and edit the worker-described parameters in Properties. Scans
read metadata only; plugin code runs only in an isolated worker process, bound
to the exact rescanned path and SHA-256. VST2 binaries such as ReaPlugs do not
publish VST3 class IDs, so a neutral authoring placeholder is used.

While the route plays, **Open plugin editor** (VST2, desktop app) shows the
vendor's own window for the very instance that processes the audio, so
meters and analysers (for example ReaFIR's noise profile) see the live
signal and every edit is heard immediately. Your settings are kept
automatically: the backend captures each plugin's state when its editor
closes, when you press **Stop**, and on tray **Quit**, and the next **Play**
restores it. Session files carry these latest settings. **Save plugin
settings** captures them immediately (`plugins.saveState`) without stopping.
VST2 plugins without chunk support (for example ReaComp and ReaGate) are
saved as a bank of their parameter values. VST3 editor windows are not
supported yet; VST3 plugins are configured through their parameters.

Scanning and plugin actions require the `pluginScan` permission. The desktop
shell includes this scope; other clients need an explicitly granted scope.
Backend authorization remains authoritative for every client.

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
backend. Use the Gate, Advanced EQ, Compressor, or Limiter action on any
connected draft path to insert that processor between the existing nodes; the
insertion and its parameters are real draft changes, not canvas-only
decoration. The Presets panel can also expand EQ presets and the voice-chain
presets into ordinary draft nodes. A voice chain is inserted into the first
compatible connected path when one exists; otherwise its nodes remain
unconnected for explicit user wiring. Draft changes are not committed until an
authorized plan/apply flow.
In Advanced EQ properties, use Add point or double-click the logarithmic graph
to create a filter. Drag a point to set frequency and applicable gain, then
choose Peaking/Band (bell-shaped boost/cut), Low/High shelf, Low/High pass, Band pass, All pass, or Notch
and set Q precisely. Band pass keeps a frequency band with unity gain at its
center; All pass changes phase without changing level (a flat 0 dB curve).
Both use frequency and Q, with no gain control.
Use Select point to edit a point without dragging it. Undo/Redo beside Save
and Play (Ctrl+Z/Ctrl+Y) restore tool edits, including after Save or live
autosave. Save a restored stopped route to keep it; live parameter changes
follow the ordinary automatic application path.
Remove point disables that band. The backend bounds the node to sixteen bands;
older saved `parametricEq@1` nodes use the same editor and retain their sound.

**FIR Filter Hz** removes steady noise per frequency, like ReaFIR's gate mode.
While the route plays, its Properties show the live spectrum of the incoming
sound. Press **Learn noise** and let only the unwanted noise play (fan, hiss,
room tone): the tool remembers the loudest level each of 64 frequency bands
reaches. **Stop and keep** stores that profile. From then on, a band whose
level does not rise **Threshold above noise** (default 3 dB) over the learned
noise is turned down by **Reduction** (default 40 dB). The orange line on the
graph is that threshold. Audio passes unchanged while learning, and the tool
adds about 21 ms of delay. Use **Denoise** instead for gentler, subtractive
noise reduction.

To back up a session or move it to another PC, use **Session → Session file**.
**Save to file…** writes the saved session (every node and its settings, plus
the imported audio and saved plugin settings it uses) to one `.audiorouter`
file; save pending edits first. **Open file…** adds a file as a new stopped
session and never replaces one: a session ID already present gets an
"(imported)" copy. Plugins themselves must be installed on the other PC, and
each Input/Output node's device should be checked there. The desktop app uses
the Windows Save/Open dialogs.

The **Setup** tab ("Set up this PC") holds app-wide items only: service and
device status, the audio devices Windows offers, guidance for other apps, and
start at sign-in. Connecting nodes without dragging (keyboard use) is under
**Advanced → Connect nodes without dragging**.
For keyboard node selection and topology edits, use **Advanced → Keyboard
graph controls**. The canvas no longer has a separate List view switch.

The **Advanced → JSON graph transfer** panel exports the selected stopped
configuration as a local `.audiorouter.json` file for scripts. Import first validates the file through the
backend, then requires the separate Commit stopped import action; imported
sessions remain stopped and require endpoint rebinding/review. The transfer
does not include credentials, grants, recordings, plugin binaries, or machine
authorization. The same versioned `.audiorouter` ZIP bundle used by Session
file is also available headlessly through the `export-bundle` and
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

## Browser feature regression

With UI dependencies and Microsoft Edge installed, run from the repository root:

```powershell
npm.cmd --prefix ui run e2e
npm.cmd --prefix ui run e2e -- --repeat-each=2
```

The suite builds a test-only Rust backend, uses isolated temporary databases
and generated audio, and owns a Vite server on port 4186. Keep that port free.
It does not open the desktop shell, native capture or physical monitoring.
Failures retain Playwright evidence under `ui/test-results`; theme screenshots
are under `target/feature-confidence-visual`. See the
[coverage and limits](../plans/active/evidence/2026-09-26-feature-confidence.md)
before interpreting a green browser result as audio qualification.
## Recording with one click

Put a **Recorder** where you want to capture audio, for example after a Meter
that also feeds your headphones. The route plays whether or not you are
recording. While it plays, press **Record** on the node (or in Properties) and
press **Stop** to save. Stopping playback also saves an open recording.

In Properties, choose the **File format** (WAV 24-bit for editing, FLAC for
smaller lossless files, MP3 to share). Turn on **Record automatically when Play
starts** for every-session capture. Set **New file every** a number of minutes
to split long sessions (WAV splits without a gap; FLAC and MP3 start a new file).
Before your first recording, choose a **Recording folder**. It appears under
the Recorder's Properties and at the top of the Recording tab, with a suggested
folder (`Music\AudioRouter Recordings`). Press **Use this folder** and it is
created for you; **Change folder** picks another one later. Only local folders
are accepted (no network shares or links), and only the AudioRouter window can
change the folder; the localhost API and MCP cannot. Until a folder is chosen,
Record explains where to set it. Scripts, StreamDeck
buttons and the MCP use `recorders.startRecording` and `recorders.stopRecording`
with the session and node IDs.

## Ducking other audio while you talk

Add **Duck** on the line you want lowered (for example game audio) and choose
**Triggered by** in Properties (usually your microphone). While the trigger is
louder than the trigger level, this audio goes down by the set amount. It comes
back after Hold and Release. Drag the orange line in its live view, or use the
suggested level calculated from your voice and room noise. A dashed violet
line on the canvas shows which node triggers it.

### Turning the game down outside Siege rounds

Set the Duck's **Trigger** to **Siege round (Stats.cc)** and tick the phases
that should be quieter (menu, preparation, between rounds). With Stats.cc
running and its game feed enabled once (`npm run setup:stats` in
`examples\integrations\stats-cc-siege`, then restart Stats.cc), AudioRouter
connects to Stats.cc on this PC while you play: the game goes down by the
Duck's amount outside rounds and back to full volume when a round starts. No
script or API token is needed. If Stats.cc closes or reports nothing, the game
returns to full volume, and the Duck's status line says why. Phase changes are
not saved as session edits.

## Surround game audio on headphones

A game sends 5.1/7.1 only to a playback device that Windows reports as 5.1/7.1.
In **Sound settings → More sound settings → Playback**, select the device the
game plays to (for example **CABLE-B Input**), choose **Configure**, pick
**7.1 Surround** and finish the wizard; the device must stay at 48 kHz. Restart
the game so it opens 7.1. In AudioRouter, select the game's input node, set
**Spatial audio** to **Surround to headphones**, then choose that playback
device from the **Loopback** entries of the capture list, and press Play. Each
speaker is placed around your head with a measured dummy head (MIT KEMAR); the
rest of the route stays stereo. A 2-channel device is refused with a message
naming the node. Changing the mode while playing needs Stop and Play.

## Properties status and timing

Add **Meter** between tools from a connection's insert menu, or connect its
input and output yourself. It passes samples unchanged. An older input-only
Meter offers **Add pass-through output** in Properties; save the upgraded draft.
Properties has larger channel meters: RMS average level, current sample peak,
held maximum, headroom and per-channel clipped sample time/share. **Reset peak
& clipping** clears that Meter's backend statistics without Save or Stop.
Statistics restart on graph preparation/replacement. Sample peaks do not
measure inter-sample true peak or LUFS; these meters are not loudness compliance
meters. Clipped time is the sum of over-full-scale samples per channel, rather
than wall-clock duration of an uninterrupted clipping event.
While playing, each connection shows its audio. Moving lights travel toward the receiving node, and the colour follows the level: teal when quiet, gold for speech, orange when loud, red near clipping. Glow and the number of lights grow with intensity. Lock a visual group in Properties (**Lock position and size**) so it cannot be dragged or resized by accident.
The thin recent-peak line holds for 1.2 s and then falls, so speech peaks stay
readable; the numbers always show the exact current values.

### Tuning a Gate, Compressor or Limiter by voice

Select the tool while the route plays and talk normally. **Live response**
shows the tool's curve (sound in across, sound out up), with a moving dot where
your voice is now, In/Out level meters and the gain reduction. The 8-second
history below draws your level, the result and the threshold line. Drag the
round handles on the curve or the orange line in the history; exact values
stay editable in the fields underneath.

- **Gate**: the history marks *room noise* and *voice* from your last 8 seconds.
  Set the threshold between them (the suggestion button picks a third of the way
  up). The strip under the history is green while the gate is open and red
  where it turns the room down. If word endings are cut, lower the threshold or
  raise Hold.
- **Compressor**: around 3–6 dB of reduction on loud words sounds natural; the
  suggestion takes about 6 dB off your voice peaks at the current ratio.
- **How one word is shaped** sketches attack, hold and release on a sample
  word, and updates as you change them.

Graphic EQ, Bass & Treble, Dehum, Pitch, Delay, Volume/Gain, Input Switch,
Declick and Speech Denoise also have a visual editor with quick choices above
their exact settings. EQ curves come from the same DSP code that processes the
audio.

**Arrange** puts every input on the left and every output on the right, with
tools in between in signal order. Long connections get their own room, chains
line up straight, and there is about one card of space between columns. It
also resets custom connector sides. **Undo arrange** appears for 15 seconds
and restores the previous positions. Audio nodes move; visual groups keep
their own positions. Arranging never changes connections or audio. Layout is
kept locally for each session.

To place a tool exactly, drag its card from the **Tools** tab (or from the
canvas's own tool list) onto the canvas. While you drag, the tool's real card
with its default settings follows the pointer; release to drop it there. The
view stays where it is so you can connect it straight away. Clicking a card
still adds the tool at the next free spot.

Spectral Gate **Learn again** replaces its previous noise curve. **Add noise**
keeps that curve and raises bands where additional noise is louder. Play only
unwanted noise while learning, then **Stop and keep**. The combined profile is
stored with the node. Live autosave attempts each changed draft once; after a
conflict, review the refreshed session and deliberately Save or Revert edits.

Properties shows Off, Bypass, Ready, Active, Disconnected or Failed beside the
node name. Ready means enabled while stopped; Active does not prove incoming
sound. Unsaved changes are identified separately. Off/bypassed tools explain
why processing readings are unavailable, and noise learning is disabled until
the tool is active. Use Timing during playback for delay and processing
measurements; tool descriptions omit approximate fixed latency claims.
