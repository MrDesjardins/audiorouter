# VST owner checks (open, 2026-11-18)

These checks close the gates the automated runner cannot reach
([VST qualification](../../operations/vst-qualification.md#what-it-cannot-check)).
They need the owner's screen, ears, hardware or sign-in, so no agent runs
them unasked. Use the release candidate in
[the evidence record](evidence/2026-11-18-vst-release-candidate.md) or a
debug build of `work/vst-compatibility`. Before you start, close any other
AudioRouter: only one desktop shell may run.

Write each result (pass, fail with what you saw, or skipped) under the check,
with the date and the build you used.

## 1. Editors by eye (VSTC editor gate)

Plugins: AGain (fixed size), TDR Nova, Surge XT Effects (resizable).

1. Add each plugin to a route with a Test Signal into an output, press Play,
   and open its editor from Properties.
2. Focus: click a text or number field inside the editor and type. The keys
   must reach the plugin, not the AudioRouter window behind it. Press Tab and
   Escape; AudioRouter must not react.
3. Scaling: in Windows Settings > Display, set 100 %, then 150 %, then 200 %
   with the editor open, and again with it closed then reopened. The editor
   must stay readable, not cropped, and its window must fit the plugin.
4. Monitors (if two are connected): drag the editor window to the other
   monitor and back, with different scaling on each.
5. Resize the two resizable editors by their window edge; the plugin must
   follow without flicker loops. Close and reopen each editor three times while
   audio plays; audio must not stop.

The same open, resize and close cycle can be watched without the app:

```powershell
$env:AUDIOROUTER_VST3_FIXTURE = 'C:\Program Files\Common Files\VST3\Surge Synth Team\Surge XT Effects.vst3'
$env:AUDIOROUTER_VST3_NATIVE_WORKER = (Resolve-Path target\vst-nightly\workers\audiorouter-vst3-worker.exe)
$env:AUDIOROUTER_VST3_EDITOR_VISIBLE = '1'
cargo test -p audiorouter-plugin-host --features test-fixtures --test worker_process `
  native_vst3_editor_survives_repeated_open_resize_and_close -- --ignored --exact
```

(Adjust both paths to the plugin and to a worker built by
`tools/vst-qualification/run.ps1`.)

## 2. Listening on a live route

1. Microphone > a VST3 effect (for example TDR Nova) > headphones. Talk for a
   minute: no clicks, dropouts or added delay you can hear compared with the
   plugin bypassed.
2. Same with a 32-bit VST2 plugin (any DPF `-vst.dll` from the 32-bit
   package) to hear the 32-bit worker.

## 3. Sidechain ducking with real audio

1. Route music (an application or an audio file) into a compressor plugin
   that has a sidechain input (a VST3 that `list_plugin_buses` shows with an
   auxiliary input), then to headphones.
2. In the plugin's Properties, add the sidechain port, and connect the
   microphone to the input named `sidechain`.
3. Enable the plugin's sidechain (key) mode in its editor. Talking must lower
   the music; silence must let it come back. Remove the port afterwards and
   check the route still plays.

## 4. Hardware MIDI

1. Plug a USB MIDI keyboard in, add an Instrument tool (for example Surge XT
   or Dexed), choose the keyboard as its MIDI input, press Play, and play
   notes and the sustain pedal.
2. Unplug the keyboard while it plays, then plug it back in. Write down what
   AudioRouter shows and whether notes work again after replugging, or only
   after choosing the keyboard again. The app must not crash and other audio
   on the route must keep playing; this behaviour has not been tested with
   hardware yet.

## 5. PolyMAX sign-in

Sign in to UA Connect, then add UADx PolyMAX as an Instrument and play a
note. Before sign-in it loads and stays silent (census result); after
sign-in it must play.

## 6. Nightly workflow on GitHub

Only after the branch is pushed with the owner's approval: start
`VST nightly qualification` from the Actions tab (Run workflow), wait for it,
download the `vst-qualification` artifact, and read every annotation on the
run (`gh api repos/<owner>/<repo>/check-runs/<id>/annotations`). The first
hosted run is the evidence that the runner works on a clean machine; it has
only run on the owner's PC so far (48 of 48 on 2026-11-18).
