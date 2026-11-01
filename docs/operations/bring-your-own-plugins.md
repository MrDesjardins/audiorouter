# Bring your own plugins (VST3, VST2 and UAD)

This guide shows how to use effects you already own inside an AudioRouter
route: where to install them, how AudioRouter finds them, how to add one to
a route, and what to do when one does not work. The last part covers
Universal Audio (UAD) plugins.

AudioRouter never downloads, installs, registers or redistributes plugins.
You install and license them yourself, exactly as you would for a DAW.

## What can be loaded

| Plugin | Works? |
| --- | --- |
| 64-bit (x64) **VST3** audio effect (`.vst3`) | Yes. VST3 effects run in their own VST3 worker. Mono and stereo effects both work on mono and stereo paths. |
| 64-bit (x64) **VST2** audio effect (`.dll`) | Yes, through a gated adapter. |
| UADx and UAD Spark (UA Connect) | Yes; see the [UAD section](#universal-audio-uad-plugins). |
| 32-bit (x86) VST2 or VST3 | No. Install the 64-bit version. |
| Instruments and synths (MIDI) | No. AudioRouter has no MIDI input yet; Play says the plugin is an instrument. |
| AAX (Pro Tools), Audio Units (macOS), CLAP | No. Many vendors include VST2 or VST3 builds in the same installer. |
| Stand-alone apps and "Console" style mixers | Not as a plugin. Route their audio devices instead (see the [UAD section](#apollo-console-unison-and-console-inserts)). |

Tested plugins are listed in the
[compatibility snapshot](plugin-compatibility.md). VST3 support in the app
dates from 2026-10-22; earlier builds could add VST3 plugins but not start
them (Play reported `VST2 load failed: MissingEntryPoint`).

A plugin that loads in another host may still not work here: each plugin runs
in its own separate process, with a fixed block size, and with a protected
voice rule (below). Results for tested plugins are in the
[compatibility snapshot](plugin-compatibility.md).

## 1. Install the plugin where AudioRouter looks

Run the vendor's installer and choose the 64-bit VST3 (or VST2) option.
If it asks for a folder, keep the default. **Scan standard folders**
searches these folders and every folder inside them:

- `C:\Program Files\Common Files\VST3` (the standard VST3 location)
- `C:\Program Files\Common Files\VST2`
- `C:\Program Files\VSTPlugins`
- `C:\Program Files\Steinberg\VSTPlugins`

A plugin anywhere else (a portable folder, a second drive) still works; use
**Scan another folder…** in step 2.

Authorize the plugin with the vendor's own tool (iLok License Manager,
UA Connect, a serial number in the plugin window and so on) before you use it
in AudioRouter. Some plugins check their licence only when they open their
own window; see [troubleshooting](#a-plugin-is-silent-or-shows-failed).

## 2. Find it in AudioRouter

1. Open AudioRouter. In the tool library, find **Plugins (VST2/VST3)**.
2. Click **Scan standard folders**. The message says how many supported
   plugins were found.
3. If your plugin is elsewhere, click **Scan another folder…** and choose the
   folder that contains it.

Scanning reads each file's details (format, 64-bit or not, vendor, a
fingerprint). It does **not** run the plugin, so a scan cannot crash
AudioRouter. Supported plugins then appear in the Plugins list with a **V3**
or **V2** badge; hover one to see its vendor and folder.

Nothing appears? See [the plugin is not in the list](#the-plugin-is-not-in-the-list).

## 3. Add it to a route

1. Click the plugin in the Plugins list, or drag it onto the canvas where
   you want it. A new block named after the plugin file appears.
2. Connect it like any built-in tool: drag from the source's output (for
   example your microphone) to the plugin's input, and from the plugin's
   output to the next tool or the destination. You can also insert it into
   an existing connection.
3. Press **Play**. The plugin starts in its own worker process.
4. Select the block. The inspector shows the plugin's own controls as
   sliders, with the ranges the plugin reports. Changes apply live. Hidden
   and read-only internals are left out (up to 256 controls are shown).
5. **Open plugin editor** opens the plugin's own window (VST3 and VST2)
   while the route plays. Use the window, the sliders or both: the latest
   change wins. (The sliders do not follow knobs you turn in the window.)
6. Your settings are kept automatically when you close the editor or press
   Stop. **Save plugin settings** keeps them immediately.

A stereo-only effect also works on a mono path: AudioRouter feeds your mono
signal to both of its inputs and mixes its two outputs back to mono. Run a
stereo path through it to keep a reverb's stereo width.

The inspector's **Isolated worker health** shows whether the plugin is
running. **Loaded plugins** lists every plugin in the current session.

## 4. Check that it works

- Watch the meters before and after the plugin block: audio should arrive
  and leave it.
- Bypass the block and listen for the difference.
- The Properties panel shows the delay each tool adds, including the
  plugin's worker queue and the delay the plugin reports for itself. If you
  switch on a setting that changes that delay (lookahead or oversampling),
  the figure updates within about half a second while the route plays.
  Other branches of a Mixer are not delayed to match.
- If the plugin returns your audio unchanged for a couple of seconds,
  **Isolated worker health** says so. That is normal for an effect at
  neutral settings, but it is also how an unlicensed plugin behaves; see
  [troubleshooting](#a-plugin-is-silent-or-shows-failed). On a microphone
  path the plugin is then silenced (below).

### The protected voice rule

If a plugin on your **microphone** path crashes, hangs, outputs invalid
samples or is not authorized, that path goes **silent**. AudioRouter never
sends your unprocessed voice instead, so a broken noise gate can't suddenly
broadcast your room. The block and **Loaded plugins** show the failure. Fix
the cause, then press **Stop** and **Play** to start the plugin again.
AudioRouter does not keep restarting a failing plugin on its own.

An unlicensed plugin often doesn't fail at all: it passes your voice
through untouched. On a path fed by a recording device, AudioRouter treats
that as a failure too. After about two seconds of clearly audible, unchanged
audio, the plugin's output goes silent. Those first two seconds do reach the
output, because AudioRouter cannot tell sooner. The **Voice protection**
card in the plugin's Properties turns red and offers two ways to get your
voice back:

1. **Expected an effect?** Sign in to the vendor's app, then press
   **Retry**. If the plugin still passes the audio through, it stays
   silent; press **Stop** and **Play** so it loads again.
2. **Meant to leave your voice unchanged?** (an EQ kept flat, a meter
   plugin) Turn on **Allow unchanged audio**. The plugin reloads, with a
   short gap if the route is playing, and is no longer muted for this.

Going back to a version from before 2026-10-31? Turn **Allow unchanged
audio** off and save first. Older versions don't know that setting: one
saved route that still has it can stop them from listing your sessions at
all. Older undo history that includes it may still fail to open there.

## Troubleshooting

### The plugin is not in the list

| What you see | Meaning and fix |
| --- | --- |
| Not found at all | It is installed in another folder. Use **Scan another folder…**, or reinstall into `C:\Program Files\Common Files\VST3`. |
| `unsupportedArchitecture` | It is a 32-bit build. Install the 64-bit version. |
| `unsupportedFormat` / `unsupportedExtension` | It is not a VST2 or VST3 effect (for example AAX, CLAP or a stand-alone app). Look for the VST2 or VST3 build. |
| A block stops starting after a plugin update | The file changed, so its fingerprint no longer matches the block. Scan again, then add the plugin again and reconnect it. |

The advanced **Plugin scan** panel (**Advanced** tab) shows every candidate
file, including rejected ones with their reason. **Inspect path** checks one
file.

### A plugin is silent or shows "failed"

1. **Licence.** Open the vendor's licence tool, make sure the plugin is
   authorized on this PC, then press Stop and Play. Demo or unauthorized
   plugins often output silence or noise, or pass your audio through
   unchanged (the inspector then says "Your audio comes out of this plugin
   unchanged").
2. **Hardware.** Plugins that need hardware (UAD-2, some dongles) do not
   process without it.
3. **Channels.** Mono and stereo effects are adapted automatically. A
   plugin whose main input or output is neither mono nor stereo (surround)
   refuses to start; the error names its channel counts.
4. **Missing worker.** "Plugin worker executable ... was not found" means
   the app is incomplete: VST2 plugins need `audiorouter-plugin-worker.exe`
   and VST3 plugins `audiorouter-vst3-worker.exe` beside the app. Reinstall
   AudioRouter; developers build both beside the shell (see the
   [quickstart](quickstart.md)).
5. **Logs.** Open **Logs** for the exact error.

### The sound is late or out of sync

Every plugin adds its worker's queue delay, and many add their own
processing delay ("lookahead", oversampling, linear-phase EQ): the UADx
compressors and preamps about 86 samples (1.8 ms at 48 kHz), Vibe Analog
Machines 714 samples (15 ms). AudioRouter adds the delay a plugin reports
to the delay it shows for that block (read when the route starts and again
when you close the plugin's window). If timing between branches matters,
keep a delayed effect on one branch only.

### Report a result

Please report what worked and what did not, so the
[compatibility snapshot](plugin-compatibility.md#reporting-a-plugin-result)
can list it: plugin name, vendor, exact version, VST2 or VST3, what you
tried, and the error text from **Logs**. Never attach the plugin file itself
or licence files.

## Universal Audio (UAD) plugins

UA sells several kinds of products under the UAD name, and they work
differently. Find yours below. The
[UAD support plan](../plans/future/uad-plugin-support.md) lists what is
done and what is still planned.

### UADx native and UAD Spark plugins (no UA hardware)

These run on your computer's processor and **work in AudioRouter**. UA
Connect installs them as VST3 (plus AAX and LUNA builds, which AudioRouter
does not use) in `C:\Program Files\Common Files\VST3`, with names starting
`uaudio_`.

1. Install them with **UA Connect** and sign in, so they are authorized.
2. In AudioRouter, **Scan standard folders**. They appear with a **V3**
   badge, named like `uaudio_ua_1176`.
3. Add, connect and Play as above, on a mono or stereo path.
4. Adjust them with the inspector sliders (Input, Output, Attack, Release,
   Ratio and so on). Their own windows are not supported yet.

Tested on 2026-10-22 with the eight free UADx effects: 1176 FET
Compressor, LA-2A Tube Compressor, 610-A and 610-B Preamp and EQ, Century
Tube Channel Strip, Showtime 64 Tube Amp, Pure Plate Reverb and Vibe Analog
Machines Essentials ([results](plugin-compatibility.md#2026-10-22-uadx-free-titles)).
Each one processes audio in mono and stereo.

If a UADx plugin passes your audio through unchanged (the inspector says so
after a couple of seconds of sound), it is not authorized on this PC: open UA Connect, check that you are signed in and the title is
licensed (a Spark subscription must be active), then press Retry, or Stop
and Play. On a microphone path the plugin is silent until then (see
[the protected voice rule](#the-protected-voice-rule)).

**UADx PolyMAX Synth** is an instrument. AudioRouter has no MIDI input, so
a scan lists it as `unsupportedInstrument` (in the **Advanced** Plugin scan
panel) and leaves it out of the Plugins list. A list remembered from before
this check still shows it until you scan again; Play refuses it either way.

### UAD-2 "Powered Plug-Ins" (need Apollo or UAD-2 hardware)

These run on the DSP chips inside an Apollo interface or a UAD-2 Satellite
or PCIe card, so the hardware must be connected and its UA software running.
UA's installer usually puts them in a `Powered Plug-Ins` folder inside one of
the standard folders, which **Scan standard folders** covers; if not, use
**Scan another folder…**. Both the VST3 and the VST2 builds can be added.

UAD-2 plugins have **not been qualified** with AudioRouter yet: no test
hardware was available. They use the same UA plugin wrapper and licensing as
UADx, which works, but they also need the DSP hardware. They add a delay
that depends on the block size, and
they stop processing when the DSP is full or the device is disconnected (a
voice path then goes silent, as described above). If you own UAD-2 hardware
and try them, please [report your result](#report-a-result).

### Apollo Console: Unison and Console inserts

Unison preamp emulations and Console inserts run inside the Apollo itself,
before Windows receives the audio. They are not plugins a host can load.
Use them this way instead:

1. Set up the Unison preamp or inserts in UA's Console app as usual.
2. In AudioRouter, use the Apollo's Windows input as the source block (your
   microphone), and route on from there with AudioRouter's tools.
3. Avoid processing the same signal twice: if Console already compresses your
   voice, don't add a second compressor by accident.

### Not supported

- **AAX** builds (Pro Tools only). Install the VST2 or VST3 build instead.
- **UAD instruments and synths** (no MIDI input in AudioRouter yet).
- **LUNA** extensions, which are part of UA's LUNA software.
