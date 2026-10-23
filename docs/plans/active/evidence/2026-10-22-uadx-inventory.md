# UADx inventory (UAD plan U1) — 2026-10-22

Owner installed UA Connect with its free UADx titles on the development PC
(Windows 11 Home 10.0.26300). Plan: [UAD plugin support](../../future/uad-plugin-support.md).
Repository at `1be9ce3b` plus uncommitted documentation. AudioRouter was
running; the owner approved the small native builds. No plugin was
registered, modified or copied; no audio device was opened.

## What UA Connect installed

Nine x64 VST3 bundles in `C:\Program Files\Common Files\VST3` (found by
**Scan standard folders**), each ~11 MB, each with `Contents\x86_64-win`,
`Contents\Resources\manifest.json` and a bundled `Resources\WebView2`
runtime (the editors are web views). Matching AAX bundles went to
`C:\Program Files\Common Files\Avid\Audio\Plug-Ins` and LUNA components to
`C:\Program Files\Common Files\Universal Audio\Plug-Ins`. No VST2 builds.

| Bundle | Class name | `is_synth` (manifest) | State bytes |
| --- | --- | --- | --- |
| `uaudio_610_a.vst3` | UADx 610-A Preamp and EQ | false | 2941 |
| `uaudio_610_b.vst3` | UADx 610-B Preamp and EQ | false | 3205 |
| `uaudio_century_channel_strip.vst3` | UADx Century Tube Channel Strip | false | 3733 |
| `uaudio_polymax.vst3` | UADx PolyMAX Synth | **true** | 9694 |
| `uaudio_pure_plate.vst3` | UADx Pure Plate Reverb | false | 2813 |
| `uaudio_showtime_amp.vst3` | UADx Showtime 64 Tube Amp | false | 2203 |
| `uaudio_teletronix_la-2a_tc.vst3` | UADx LA-2A Tube Compressor | false | 1093 |
| `uaudio_ua_1176.vst3` | UADx 1176 FET Compressor | false | 2677 |
| `uaudio_verve_essentials.vst3` | UADx Vibe Analog Machines Essentials | false | 1194 |

## Offline loader probe

Command (per bundle, 60 s limit): `tools\m06-vst3-loader\build.ps1`, then
`m06-vst3-loader.exe <bundle>`. Build output was removed afterwards.

All nine: exit 0 in about 1 s; two factory classes (one `Audio Module
Class`, one separate `Component Controller Class`); 1 input and 1 output
audio bus; a 64-frame offline block processed with finite output; component
state saved and restored. No sign-in prompt or licence failure appeared at
load. Two observations:

1. **Every plugin reported `parameters=0`.** The loader does not connect the
   component and controller through `IConnectionPoint`, so this may be a
   probe limitation; the production worker does connect them but could not
   be checked (finding 2). Open question for the next run.
2. **Each plugin writes a text banner to stdout** (`Timers:`,
   `General@info`, `Errors@warn`, ... `UAPW@info`, `HBGF@info`).

PolyMAX is a synth but presents an ordinary audio-effect class with an
audio input bus, so a class-category check alone would accept it.

## Native VST3 worker (C++) on the 1176

`tests\acceptance\m06-vst3-worker.ps1 -FixturePath
'C:\Program Files\Common Files\VST3\uaudio_ua_1176.vst3' -SingleStreamOnly`
failed (exit 101):

    native VST3 parameter description response:
    Message(TooLarge { length: 538970637, maximum: 1048576 })

`538970637` is `0x2020200D`: a carriage return and spaces from the banner,
read as a frame-length header. The worker uses stdout as its binary protocol
pipe (`write_frame`, `tools/m06-vst3-worker/main.cpp`), so any plugin that
prints to stdout corrupts the protocol.

## Shipped plugin worker on a VST3

`crates/control/src/plugins.rs` starts `audiorouter-plugin-worker.exe`
(`plugin_worker_path`) for every plugin node, VST3 included, and that
worker loads plugins only through `Vst2Library::load`. Running the release
worker (`src-tauri\target\release\audiorouter-plugin-worker.exe`,
2026-10-16 build; `target\release` copy 2026-10-20 is the same size) on the
1176 module with `--plugin-sha256 <hash> --channels 2 --sample-rate 48000
--plugin-path <module>` printed:

    plugin worker stopped: VST2 load failed: MissingEntryPoint

So in the current app a VST3 node passes scanning and can be added, but its
worker cannot start. The native VST3 worker is exercised only by the M06
acceptance scripts. This applies to every VST3 plugin, not only UADx.

## Consequences

- The UAD plan gains two blocking items before U2: ship and launch a VST3
  worker from the app (U0), and protect the protocol pipe from plugin stdout
  output (U0b). Recorded in the plan as G0 and G11.
- The user guide and public pages that call VST3 supported overstate the
  current app. The guide is corrected; the site claims
  (`site/index.html`, `site/tools.html`, `site/compare.html`) need the
  owner's decision (fix the app first or soften the wording). The owner
  chose to fix the app and keep the wording (see the addendum below).
- Instruments must be filtered by VST3 sub-category (or the manifest), not
  by class category (G12).

## U0 results in the app worker (2026-10-22)

After the owner's instruction to make VST3, VST2 and UAD work, U0 was
implemented (see the [UAD plan](../../future/uad-plugin-support.md), package U0).
Three host defects stood between the worker and processed UADx audio:

1. **stdout banner (G11).** Fixed by moving the protocol to duplicated
   handles and pointing stdout at `NUL` before the plugin loads.
2. **Parameter count.** The UADx modules expose more parameters than the
   bounded descriptor contract allows (mostly hidden internals). The worker now
   lists only automatable, visible, writable, non-program-change parameters
   (at most 256): 1176 = 8 (Input, Output, Attack, Release, Ratio, Meter,
   Power, Master Bypass).
3. **Pass-through output (G13).** With the first two fixed, every UADx
   output equalled its input. An 8-second warm-up and a Windows environment
   allowlist (G14, needed anyway since `env_clear` removed `LOCALAPPDATA`
   and `ProgramData`) did not change it. UA's own log
   (`%LOCALAPPDATA%\Universal Audio\Logs\UAPW_*.txt`) showed
   `mixer.plugin.invalid_host_signature` and an "Error mode" fallback.
   Throwaway PowerShell probes reproduced it outside the job object: the
   only trigger was loading the bundle through the verbatim `\?\C:\...`
   path that `fs::canonicalize` returns. The host now passes the plain
   path when it fits in `MAX_PATH` (`loadable_plugin_path`). Job limits,
   `CREATE_NO_WINDOW` and the environment allowlist were all compatible.

Then all eight effects processed. Command (Git Bash, release worker):

    AUDIOROUTER_VST3_FIXTURE=<bundle> \
    AUDIOROUTER_VST3_NATIVE_WORKER=src-tauri/target/release/audiorouter-vst3-worker.exe \
    AUDIOROUTER_VST3_CHANNELS=1|2 \
    cargo test -p audiorouter-plugin-host --features test-fixtures \
      --test worker_process native_vst3_fixture_signal_report -- --ignored --nocapture

Results (48 kHz; mono and stereo both changed the 220 Hz input and stayed
finite): latency 1176, 610-A, 610-B and Century 86 samples; LA-2A 87;
Showtime 34; Pure Plate 31; Vibe Analog Machines Essentials 714. PolyMAX is
refused: "VST3 plugin is an instrument; AudioRouter hosts audio effects
only". The control-plane test `installed_vst3_plugins_start_in_the_native_worker`
(scan, then `plugins.parameters` for each `uaudio_` module, with
`AUDIOROUTER_VST3_WORKER_PATH` set) passed.

Regressions run on the same tree: plugin-host and control unit tests, the
`worker_process` suite with `test-fixtures` (35 passed), the AGain
`m06-vst3-worker.ps1` acceptance (exit 0), the ReaPlugs VST2 acceptance
(6 fixtures at 44.1, 48 and 96 kHz), and the release self-tests
(`test-prepare-artifacts`, `test-app-only-release`, `test-verify-artifacts`).

Not yet done: Play on a real route in the shell, a signed-out UA Connect
run, VST3 editors, and feeding the reported latency into path latency.

## First attended Play and state restore (2026-10-22)

The owner's first Play of mic → `uaudio_ua_1176` → headphones failed with
"native graph rejected: UnsupportedTopology" (backend log: four
`session.start` errors -32602 after `nativeEndpoints.rebind`). The route
was mono microphone, mono plugin, stereo headphones. The single-endpoint
compiler accepts only one channel width per chain; the multi-path compiler
accepts the route. The UI now sends routes with width-changing connections
to the multi-path worker (`needsNativePaths`).

Qualifying that route then exposed a second defect: Stop saves the plugin
state and the next Play failed with `plugin state restore failed:
StateUnsupported`. A temporary diagnostic showed `VST3 component setState
failed with VST3 result 0x1`. The worker's `IBStream::read` returned
`kResultFalse` on a short read at the end of the data; the SDK's
`MemoryStream` (and the U1 loader, whose round trip passed) return
`kResultOk`. After the fix all eight effects restore their state into the
same and a fresh instance, mono and stereo (`native_vst3_fixture_signal_report`),
and `live_plugin_route_plays_again_after_stop` ran Play, Stop, Play on a
copy of the owner's database with the plugin `running` both times.
