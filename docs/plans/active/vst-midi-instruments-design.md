# MIDI input and instrument plugins: design (phase P8b)

Status: software steps 1-8 built and tested (2026-11-13/14); hardware MIDI and attended shell checks open. Part of the
[full VST compatibility plan](vst-full-compatibility.md); owner decision D1
(2026-11-13): instruments and MIDI are in scope. The rules in AGENTS.md (every
source works in every layout, REST and MCP kept current, guides for new tools)
decide the order.

## Progress and evidence

| Step | Result | Evidence |
| --- | --- | --- |
| 1 Wire | Done. `MidiEvent`, one-way `WorkerMessage::Midi` (not a field of the process messages: the real plugin paths are `Process` and `ProcessBuses`, and a separate message needs no change to either) | unit tests `midi_*` in `crates/plugin-host/src/lib.rs`, commit `3675f3d4d` |
| 2 VST2 worker | Done. `--instrument`, `Vst2Role`, `effProcessEvents` (opcode 25) with the public `VstEvents`/`VstMidiEvent` layout, 0-input instruments, per-segment delivery | `crates/plugin-host/tests/vst2_instrument.rs` against `tools/vst2-test-instrument` (a dependency-free test synth): silence without MIDI, sound only after the note-on sample, silence after note-off, velocity scales level, mono route works, a bad offset ends the worker, an instrument loaded as an effect is refused with its own reason; commit `ef6058d2c` |
| 3 VST3 worker | Done. `--instrument`, `IEventList`, event bus activation, `IMidiMapping` for pitch bend/CC/pressure, instrument class selection | opt-in `tests/vst3_instrument.rs`: real **Surge XT** peak 0 without a note and 0.276 while a note is held; Dragonfly (an effect) is refused with "effect, not an instrument"; commit `52fcf224b` |
| 4 Bridge | Done. `MidiQueue` (bounded, held-note tracking, CC 120/123 become explicit note-offs, `release_all_notes`), `PluginRuntimeBridge::start_instrument`, `SupervisedWorkerProcess::spawn_instrument_verified` and restart | bridge tests with the real worker: silent until a note, audible while held, silent after the panic release, healthy with no MIDI; commit `5c421b43e` |

Not run: the **x86** VST3 worker compiled with the instrument code but no
instrument was played through it; the VST2 instrument was only the test synth
(no third-party VST2 instrument was available: Cardinal is 1.1 GB and the
others offer no VST2 build); the packaged workers in
`src-tauri/target/release` predate this code and are rebuilt by
`tools/build-desktop.ps1`.

Known gaps carried to steps 5-6: the refusal wording of the effect workers
still says "no MIDI input yet" and must point to the Instrument tool when it
exists; MIDI timing is quantised to the start of the next block (about the
pipeline depth, a few ms), which is recorded here as the accepted behaviour.

## What the owner gets

A user adds an **Instrument** tool, picks a VST2 or VST3 instrument and a MIDI
keyboard, connects it like any other source (to an output, a Mixer, several
outputs), and plays. An on-screen keyboard and an API call can play notes too,
so it works without hardware, from Stream Deck and from an assistant.

## Constraints from the code (2026-11-13 survey)

- A `Plugin` node processes through `RealtimePluginProcessor::process(&self,
  &mut AudioBlock)`: a bounded hand-off to a worker process; the callback never
  waits (PLUG-03). An instrument can use the same processor with an ignored
  silent input and replace the block with the worker's output.
- Sources (Test Signal, Audio File, Soundboard) are classified in several
  hand-written lists (`connected_mixers.rs:82`, `engine/lib.rs:7896`, `8695`,
  `8749`, `8816`). The AGENTS rule demands an exhaustive `match`.
- The worker protocol is JSON over a pipe plus shared audio buffers
  (`WorkerMessage`). Sample-accurate parameter events already ride along;
  MIDI can use the same idea.
- The VST2 header check refuses `effFlagsIsSynth` (phase P1) and allows no
  instrument with zero audio inputs; the VST3 worker refuses instruments at
  load.

## Design

1. **Node kind.** `NodeKind::Instrument`: a source (no input port; one output
   port, mono or stereo). Parameters: `path`, `format`, `classId`,
   `fingerprint`, `stateId`, `pluginParameter:N` (same as `Plugin`), and
   `midiInput` = a MIDI device id, `"keyboard"` (the on-screen keyboard and API
   only) or `"all"`. Classified in every source list by an exhaustive match; it
   is added to `source_combinations.rs` (alone, fan-out, in a Mixer, in
   connected Mixers).
2. **Realtime path.** The instrument stage reuses the plugin processor trait:
   the engine hands it a silent block; the processor drains a lock-free MIDI
   queue (fixed capacity, no allocation) into the frame it posts to the worker
   and replaces the block with the worker's output (same late-result rule as
   effects: silence, never stale audio).
3. **MIDI wire.** New one-way message `WorkerMessage::MidiEvents { events }`,
   sent before the `Process*` message of the block they belong to; each event is
   `{ sampleOffset < 2048, status, data1, data2 }`, bounded count per block.
   The worker keeps them until the next process call. The VST3 worker's tiny
   JSON reader gets a matching field.
4. **VST2 worker.** Opcode `effProcessEvents` (25) with a `VstEvents` block of
   `VstMidiEvent`s (32 bytes each, same layout 32- and 64-bit) before
   `processReplacing`; instruments may declare 0 inputs; `audioMasterWantMidi`
   already answers 1. Header validation gains an instrument mode chosen by the
   node kind, not by guessing.
5. **VST3 worker.** An `IEventList` in `ProcessData.inputEvents` with
   `kNoteOnEvent`/`kNoteOffEvent` (plus pitch bend, CC through `IMidiMapping`
   parameter changes); activate the event input bus and zero audio input buses;
   `IParameterChanges` for CC and pitch bend as the SDK recommends.
6. **MIDI input.** A `MidiInputHub` in the control crate using `winmm`
   (`midiInGetNumDevs`, `midiInOpen` with a callback) feeding one bounded queue
   per instrument; the callback only pushes. Device lists are read on request,
   hot-unplug is reported (never silently switched, as for microphones).
7. **API (AUTO-18).** New methods: `midi.devices` (read), `midi.sendEvents`
   (note on/off, control change, pitch bend, all-notes-off for a node; used by the
   on-screen keyboard, Stream Deck and assistants), and the Instrument kind in
   `list_tool_kinds` with `node_parameter_schema`. Authorization: `midi.sendEvents`
   needs session control (Operator); `midi.devices` is read-only. Default MCP
   tools: `list_midi_devices`, `play_notes`; a `get_recipes` entry
   ("play an instrument").
8. **UI.** Add-tool entry for Instrument in the library; Properties: plugin
   picker (as Plugin), MIDI device select, an on-screen keyboard (a row of keys
   plus octave shift; reserved size, no layout shift), panic button. Guide
   videos and the property coverage manifest per the website rule.
9. **Failure policy.** A failed instrument worker plays silence until recovery
   (as for effects); stuck notes are released on worker restart, route stop and
   device removal (all-notes-off).

## Order of work (each step ends with tests and a commit)

1. Worker protocol: `MidiEvent`, `MidiEvents` message, validation, round-trip
   tests (host side only).
2. VST2 worker: instrument mode, `effProcessEvents`; test with a real VST2
   instrument (a free one such as Cardinal's or Surge-class VST2 build).
3. VST3 worker: event bus and `IEventList`; test with Surge XT and RipplerX
   (installed in the census corpus).
4. Runtime bridge: instrument processor, queue, stuck-note release; test with a
   real worker and generated notes (output must be audible and finite).
5. Domain + engine: `NodeKind::Instrument`, exhaustive classification, source
   combination suite, tool combination suite.
6. Control/API/MCP: methods, authorization, schemas, tools, recipes, drift
   check, reachability test, `mcp_tool_call` test.
7. MIDI hardware input (`winmm`), device list, unplug handling.
8. UI, on-screen keyboard, guide videos, site pages; Firefox playback check.
9. Run the built exe with a real instrument and the keyboard; a real MIDI
   device only if the owner has one (none is assumed).

## Risks and open questions

- No MIDI hardware is known on this PC; steps 1-6 need none, step 7 can only
  be unit-tested with a fake device source and checked for "no devices" on the
  real machine.
- A fully general VST3 event/parameter mapping (MPE, expression) is larger than
  notes, CC and pitch bend; those three are the target.
- Instruments differ in latency; plugin latency already feeds the route delay
  display.
- Guide videos need recording tooling and Firefox; they come last.

## Evidence (2026-11-14)

Steps 1-8 are committed on `feature/vst-plugin-experience`. What was run:

| Check | Result |
| --- | --- |
| `an_instrument_node_starts_a_worker_and_plays_midi` with the VST2 test synth | passes: silent, then audible after `midi.sendEvents` through `ControlPlane::dispatch` (node named, not identified), silent after `allNotesOff` |
| Same test with **Surge XT.vst3** (free, JUCE, real third-party VST3 instrument) through the C++ VST3 worker | passes; the release tail takes about a second, so the test now waits for silence instead of one fixed window |
| Same test with licensed UADx PolyMAX (VST3) | loads, accepts notes (`heldNotes` 1), **output stays exactly 0**. The plugin's own log shows `license valid: true` but UA Connect `eState_NotLoggedIn`; treated as the UA sign-in rule recorded for UADx, not as a MIDI defect, since Surge XT plays through the same path. Not retried signed in |
| `tool_combinations.rs`, `source_combinations.rs` | pass with Instrument |
| UI: typecheck, lint, format, 1065 unit tests (including the keyboard editor: exact `midi.sendEvents` requests, release on unmount, disabled until Play, unplugged device kept) | pass |
| Workspace Clippy `-D warnings` and `src-tauri` Clippy, both `cargo fmt --check` | clean |

Not done yet (kept on the plan): a real hardware MIDI keyboard (none on this
PC), the built desktop exe and packaged workers (they predate this code), the
Instrument card on the canvas and its three-theme screenshots, guide video and
site page, and the property coverage manifest entry.

## Built-exe check and scan limitation (2026-11-14)

- `tools/build-desktop.ps1` (release) finished with exit 0; the shell exe is
  newer than `ui/dist`, and `ui/dist` holds the Instrument editor strings.
- The built `audiorouter-cli api call` (in-process control plane, disposable
  database) answers `midi.devices` with an empty list on this PC (no MIDI
  hardware) and scans the Surge XT folder.
- **Finding:** the scan reports Surge XT as `instrument: false`. The bundle has
  no `moduleinfo.json`, and a scan never runs plugin code, so only a worker's
  refusal reveals it. Until that has happened once, Surge XT is listed with the
  effects; adding it as an effect then fails with "is an instrument; add it with
  the Instrument tool", and from then on it is listed under Instruments.
  Fix made: the learned fingerprints were kept only in memory, so every launch
  forgot them; they are now saved in `%LOCALAPPDATA%\AudioRouter\learned-instruments.txt`
  (`AUDIOROUTER_LEARNED_INSTRUMENTS_FILE` overrides), with a regression test for
  the read/append/ignore-junk round trip.
- Still open: classify a VST3 without metadata before its first failed Play
  (needs a worker "describe" mode that reads the class category without
  creating the plugin).
- Not done: the shell window was not launched with an instrument (the
  owner is using the PC; audio would play through speakers) and the shell-side
  editor window for an instrument is untested.
