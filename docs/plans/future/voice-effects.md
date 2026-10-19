# Fun Voice Changer tool

**Status:** promoted 2026-10-18 ("lets build it now"); implemented, owner listening pending. Progress and deviations: [active plan](../active/current.md). Original note: implementation starts when the
owner says so. At that point, promote this plan by the procedure in
[README](README.md#promoting-a-future-plan): spec DSP-20 and an
active-plan entry.

**Owner decisions (2026-10-18):**
- One new tool, **Fun Voice Changer**, with many presets people can tweak.
- Existing tools are unchanged, Pitch included.
- The first preset set below is approved.
- About 25 ms of extra delay on the Discord voice path is acceptable.

**Defaults this plan chooses (the owner can override them before coding):**
- Mono processing, with the same voice sent to both channels.
- A fixed 1,024-sample delay (21.3 ms).
- The preset recipes in the table below.
- A new graph edit for every preset switch, so each switch is one undo step.

## 1. User outcome

Users add live, playful voice effects to the microphone for Discord, OBS and
recordings. They pick a preset (Pitch Correct, Deep, Robot, Radio and
others) and then adjust its sliders. Typical route:
Microphone → (Denoise) → Fun Voice Changer → cable → Discord.

## 2. Facts from the current code (read 2026-10-18)

| # | Fact | Where | Consequence for this tool |
| --- | --- | --- | --- |
| F1 | The graph runs at a fixed 48 kHz in 128-frame blocks. | `INTERNAL_SAMPLE_RATE_HZ` (`crates/engine/src/lib.rs:12`); Pitch requires exactly 128 frames (`:9910`). | Fixed buffer sizes and no rate handling. A block that is not 128 frames is cleared, as Pitch does. |
| F2 | The registry holds one fixed `latency_samples` per node kind. It is shown in the catalog, `describe()` and the graph plan's `latencySamples`. **No live branch compensation uses it:** `calculate_latency_compensation` is called only by tests. | `crates/domain/src/lib.rs` (registry), `crates/engine/src/lib.rs:146` | The tool must declare one constant: 1,024 samples. It must also really be constant. A delay that changed per preset would make the audio jump in time on every preset switch, and other branches are not realigned. |
| F3 | A parameter change while audio plays is auto-saved after a **400 ms pause** (`App.tsx`, `isParameterOnlyChange`). It goes through `graph.plan` and `graph.commit`, recompiles the whole graph, and swaps it in with `replace_graph` / `replace_paths` on the control thread. DSP state is rebuilt. Only Input Switch position, Duck gain and Time Shift buffers carry over. | `ui/src/App.tsx:1268-1295`, `crates/engine/src/lib.rs:4377` | (a) If the state were rebuilt, every slider pause would drop about 21 ms of audio. The tool therefore keeps its state in a per-node registry like Time Shift's (`time_shift_state_for`, `:3309`). (b) Recompiling runs on the thread that also pumps audio (lesson 2026-09-26), so compiling this stage must be cheap and must not allocate the large buffers again. (c) Users hear a change only after they pause dragging. That is existing behavior for every tool, not something to fix here. |
| F4 | A **bypassed** node is skipped at compile time; edge matrices carry the dry signal. A disabled node is left out too. | `crates/engine/src/lib.rs:5937` | Bypass removes the 21 ms delay, so toggling it jumps the audio by 21 ms, as Pitch and Denoise do today. With kept state, turning bypass off would replay audio buffered before the bypass. The registry must **reset the state** when the node comes back after being absent (§5.3). Mix 0 % is the smooth way to hear the dry voice; the guide says so. |
| F5 | The dry-bypass allow-lists of single-in/single-out tools are at `:5764`, `:5789`, `:7195` and `:8219`. | `crates/engine/src/lib.rs` | Add the new kind to every list; the 2026-10-02 lesson covers what happens if one is missed. |
| F6 | `independent_path_sessions` splits a session into weakly connected components, so a node belongs to one path. A chain session ID is `<session>::<label>` (`owning_session_id`). | `crates/engine/src/lib.rs:7761`, `:3303` | The registry key is (owning session, node ID). Still to verify in the code (task T2.4): a node is compiled into **at most one stage per path**, even after the fan-out. If it can appear on two branches, each branch needs its own state; one shared state would process two inputs per pump. |
| F7 | Stages hold DSP in `RealtimeDsp<T>` (busy flag). Pitch clears the block when `try_with` fails or processing errors. | `:2698`, `:9909-9945` | Same rule: no output is silence, which meets the protected-voice rule. |
| F8 | Tool ports are created with 2 channels (`ui/src/draft.ts`, for example `spectralGate`). A mono mic is upmixed by the edge channel matrix. | `ui/src/draft.ts:157` | The input is normally the same voice on both channels. |
| F9 | Per-node telemetry is assembled in `native_workers.rs` for both worker kinds. | `crates/control/src/native_workers.rs:1062`, `:1243` | Add `voiceChanger` (note, cents, voiced) in both places. |
| F10 | Choice parameters are validated as fixed string sets (for example Duck `trigger`, `domain/src/lib.rs:1892`). The limit is 96 parameters per node. | `crates/domain/src/lib.rs` | `preset`, `key`, `scale` and `modType` follow the same pattern. There are 36 parameters (§4). |
| F11 | `describe()` publishes presets from `audiorouter_dsp` enums (`voiceChains`, `eq`): ID, version, name, description. | `crates/control/src/catalog.rs:25-55` | Add `presets.voiceChanger`. Each entry also carries its **full parameter values** and its basic-view slider list. The UI never keeps its own copy. |
| F12 | `crates/dsp` depends on `microfft` with the `size-1024` feature only. | `crates/dsp/Cargo.toml` | Pitch detection needs a 2,048-point FFT, so add the `size-2048` feature. It is additive, and the existing tools are unchanged. |
| F13 | No denormal handling exists in `crates/dsp` or the engine. | grep, 2026-10-18 | Feedback paths (reverb, flanger, one-pole smoothers, filters) must snap values below 1e-20 to zero explicitly. DSP-08 requires this. |
| F14 | The site guides play **JavaScript illustrations** made from synthetic fixtures (`site/assets/guide-signal.js`). They do not run the backend DSP. Pitch, for example, resynthesizes its bell at the shifted frequency. | `site/assets/guide-signal.js:17-50`, `:338` | The guide needs a JavaScript illustration of a synthetic voice in which f0, formant scaling and the simple effects can be adjusted. The guide must say it is an illustration. The Firefox check applies. |
| F15 | The owner's live route (health record, 2026-10-18) has about 41–46 ms of output queue and about 11–12 ms of input pickup. | `backend.jsonl` audioHealth | Expected mic-to-Discord delay: about 55 ms now, about 76 ms with this tool, about 98 ms with Denoise as well. Measure it (§9); do not claim these figures. |

## 3. Signal chain

The tool works in mono. It sums the input to (L+R)/2, processes it, and
writes the result to both output channels.

- **Why:** this is a voice tool. Processing the mic twice would double the
  CPU for identical channels.
- **Consequence:** a stereo source (music) placed on it becomes mono. The
  library note and the guide say so.
- **Alternative (owner decision):** process L and R separately, at twice the
  CPU.

The stages run in a fixed order. Stages 1 and 10 are always on; each of the
others has an enable flag, and a disabled stage costs nothing.

1. **Input gate.** Soft downward expander with threshold `gateDb`, 3 dB
   hysteresis, 5 ms attack and 120 ms release. Its default is −90 dBFS
   (effectively off).
2. **Analysis.** Runs only when stage 3, 4 or 6 needs it.
   - **Pitch:** McLeod pitch method (normalized square difference) on a
     1,024-sample window every 256 samples (5.3 ms), with the
     autocorrelation computed by a 2,048-point FFT. It covers 70 Hz to
     1 kHz.
   - **Voicing:** voiced when the clarity peak is ≥ 0.75 and the level is
     above the gate; unvoiced below 0.6 (hysteresis).
   - **Octave check:** an estimate more than 0.8 octave away from the
     running median of the last 5 frames is rejected for up to 3 frames.
   - **Epochs:** pitch marks are placed by picking the waveform peak within
     ±P/2 of the previous mark plus one period.
   - **LPC:** order 16, by autocorrelation and Levinson–Durbin, every 256
     samples. It is used by Whisper (stage 6).
3. **Pitch correction.**
   - **Target note:** the nearest note of `scale` in `key`, with ±30 cents
     of hysteresis at note boundaries so the output does not warble between
     two notes.
   - **Glide:** the target ratio follows a one-pole glide with time constant
     `retuneMs`. At 0 ms the ratio steps within one analysis hop.
   - **Humanize:** after a note has been held 250 ms, the retune time is
     multiplied by (1 + 4·humanize), so held notes keep some natural
     vibrato.
   - **Monotone scale:** locks the voice to `key` at the octave nearest the
     voice, for the Robot preset.
4. **Pitch and formant shift (TD-PSOLA).**
   - **Grains:** Hann-windowed, two analysis periods long, centered on the
     epochs.
   - **Pitch:** synthesis marks are spaced by the analysis period divided by
     (correction ratio × 2^(semitones/12 + cents/1200)).
   - **Formant:** each grain is resampled by 2^(−formant/12) with linear
     interpolation, which keeps the cost bounded.
   - **Unvoiced input:** grains use a fixed 5 ms pseudo-period, so only the
     formant shift applies, and noise has no pitch.
   - **Voiced/unvoiced change:** 10 ms crossfade.
   - **Delay budget:** the largest period (70 Hz) is 686 samples, so a grain
     needs 686 samples of look-ahead. 1,024 − 48 (limiter) leaves 976
     samples, which covers it.
5. **Character.**
   - **Ring modulator:** `ringHz` 20–2,000 Hz, `ringDepthPercent`.
   - **Bit crusher:** `crushBits` 4–16, `crushDownsample` 1–12 (sample and
     hold).
   - **Saturation:** tanh with `drivePercent`, 0–36 dB of pre-gain and
     automatic −drive/2 makeup.
6. **Whisper.** The LPC residual is replaced by white noise of the same
   energy and filtered back through the LPC envelope. `whisperPercent`
   blends this with the current signal.
7. **Tone.**
   - Low cut 20–2,000 Hz (2nd order).
   - High cut 1–20 kHz (2nd order).
   - Peak 200–8,000 Hz, ±12 dB, Q 1.0.
   - Built on the existing biquad code, with coefficients interpolated per
     block (DSP-08).
8. **Modulation.**
   - **Chorus:** 15 ms base delay, depth up to 8 ms, no feedback.
   - **Flanger:** 2 ms base delay, depth up to 2 ms, feedback ±90 %.
   - **Controls:** `modRateHz` 0.05–5 Hz and `modDepthPercent`.
   - **Delay line:** fractional, preallocated to 30 ms.
9. **Space.**
   - **Reverb:** an 8-line feedback delay network (the same type as the
     spatial room).
   - **Lines:** the longest is 100 ms, preallocated.
   - **Controls:** `spaceSizePercent` (scales the line lengths),
     `spaceDecayMs` (RT60, 200–4,000 ms) and `spaceLevelPercent`.
10. **Output.**
    - **Mix:** dry/wet `mixPercent`, with the dry signal delayed by exactly
      the tool's delay.
    - **Gain:** `outputDb` −24…+12 dB.
    - **Safety limiter:** −1 dBFS sample peak, 48-sample look-ahead and
      50 ms release. It is always on.

**Delay accounting:** analysis and PSOLA use 976 samples and the limiter 48,
for 1,024 in total. Stages 5–9 are causal and add none. A preset that
leaves the pitch stages off delays its input through a matching line, so
the delay is the same for every setting.

## 4. Parameters (`funVoiceChanger`, 36)

| Name | Type | Range / choices | Default (neutral) |
| --- | --- | --- | --- |
| `preset` | string | preset IDs, `custom` | `custom` |
| `gateDb` | number | −90…−20 dBFS | −90 |
| `correctEnabled` | boolean | | false |
| `key` | string | `c` `cs` `d` `ds` `e` `f` `fs` `g` `gs` `a` `as` `b` | `c` |
| `scale` | string | `chromatic` `major` `minor` `majorPentatonic` `minorPentatonic` `blues` `monotone` | `chromatic` |
| `retuneMs` | number | 0–400 ms | 50 |
| `humanizePercent` | number | 0–100 % | 0 |
| `shiftEnabled` | boolean | | false |
| `pitchSemitones` | number | −12…+12 | 0 |
| `pitchCents` | number | −100…+100 | 0 |
| `formantSemitones` | number | −12…+12 | 0 |
| `ringEnabled` | boolean | | false |
| `ringHz` | number | 20–2,000 Hz | 50 |
| `ringDepthPercent` | number | 0–100 % | 100 |
| `crushEnabled` | boolean | | false |
| `crushBits` | number (integer) | 4–16 | 16 |
| `crushDownsample` | number (integer) | 1–12 | 1 |
| `driveEnabled` | boolean | | false |
| `drivePercent` | number | 0–100 % | 0 |
| `whisperPercent` | number | 0–100 % (0 = off) | 0 |
| `toneEnabled` | boolean | | false |
| `lowCutHz` | number | 20–2,000 Hz | 20 |
| `highCutHz` | number | 1,000–20,000 Hz | 20,000 |
| `peakHz` | number | 200–8,000 Hz | 1,000 |
| `peakDb` | number | −12…+12 dB | 0 |
| `modEnabled` | boolean | | false |
| `modType` | string | `chorus` `flanger` | `chorus` |
| `modRateHz` | number | 0.05–5 Hz | 0.5 |
| `modDepthPercent` | number | 0–100 % | 50 |
| `modFeedbackPercent` | number | −90…+90 % (flanger only) | 0 |
| `spaceEnabled` | boolean | | false |
| `spaceSizePercent` | number | 0–100 % | 50 |
| `spaceDecayMs` | number | 200–4,000 ms | 1,000 |
| `spaceLevelPercent` | number | 0–100 % | 30 |
| `mixPercent` / `outputDb` | number | 0–100 % / −24…+12 dB | 100 / 0 |

(`mixPercent` and `outputDb` count as two parameters, for 36 in total.)

Validation rules (domain):
- Unknown keys and wrong types are rejected.
- Integers are required where the table says integer.
- `preset` must be a known ID or `custom`.
- A missing parameter takes its neutral default. A node with every
  parameter missing passes the input through unchanged, apart from the
  delay.

## 5. Runtime design

### 5.1 State and parameter handoff

```
ProcessingStage::FunVoiceChanger {
    state: Arc<VoiceChangerState>,   // shared through the registry
    target: VoiceChangerParams,      // Copy struct, fixed at compile time
}
```

- **State:** `VoiceChangerState` holds a
  `RealtimeDsp<audiorouter_dsp::voice_changer::VoiceChanger>`, the telemetry
  atomics (f0 bits, cents, voiced, note) and `reset_pending: AtomicBool`.
- **Each block:** the stage passes `&target` to `process`. The DSP compares
  it with its current parameters and moves toward it:
  - continuous values glide over 30 ms (frequencies on a log scale);
  - booleans, choices, `crushBits` and `modType` crossfade the stage's
    output over 30 ms;
  - a preset change is simply many targets changing at once.
- **No atomics are needed for parameters:** the stage is immutable, and a
  new graph brings a new `target`.
- **Swap:** the control thread swaps graphs between pumps. The old graph's
  `Arc` is dropped there, never on the audio path.

### 5.2 Registry

- **Shape:** `Mutex<HashMap<(owning session, node id), Weak<VoiceChangerState>>>`.
  It is locked only at compile time and pruned of dead entries on each
  compile, as Time Shift does.
- **First compile:** allocates every buffer: about 64 KiB of analysis and
  PSOLA buffers, 30 ms of modulation delay and the FDN lines, boxed.
- **Later compiles:** reuse the state, and allocate nothing large.
- **Stop:** drops the graph and its `Arc`, so the next Play starts fresh.

### 5.3 Reset after absence (F4)

The registry entry records the graph generation and pump sequence at which
the state last processed a block. If a recompile finds the node present
again after a graph in which it was bypassed or disabled, it sets
`reset_pending`. On its next block the DSP then clears its buffers and fades
in over 10 ms. This prevents stale audio after un-bypass.

The simpler rule is acceptable: also reset on any recompile where the
previous graph did not contain the node. Either rule is tested by T2.5.

### 5.4 Realtime rules

- `process` never allocates, locks, logs or panics. Indexing is
  bounds-checked through fixed-size arrays, or the loop is written so it
  cannot panic.
- Any non-finite value inside the DSP resets that stage and outputs silence
  for the rest of the block. A counter is exposed in telemetry.
- Values below 1e-20 snap to zero in every feedback path (F13).

### 5.5 Cost estimate (to be measured)

- **Analysis:** 187.5 frames per second × (one 2,048-point FFT forward and
  inverse), about 0.15 Mflop per frame, or about 30 Mflop/s.
- **PSOLA, the effects, the 16-pole LPC and the 8-line FDN:** about
  10 Mflop/s.
- **Total:** under 2 % of one core, measured with `benches/realtime.rs`
  with every stage on. The registry entry also sets cost class `"high"`.

### 5.6 Telemetry

- **Content:** `voiceChanger: { voiced, f0Hz, note, cents, nonFinite }`,
  read at the 20 Hz diagnostics rate from atomics.
- **When pitch stages are off:** `voiced: false` and `f0Hz: null`, never
  NaN.

## 6. Presets (approved set; values are first drafts)

| ID | Name | Recipe (non-neutral values) | Basic sliders |
| --- | --- | --- | --- |
| `pitchCorrectSubtle` | Pitch Correct (subtle) | correct, chromatic, retune 80, humanize 40, gate −55 | key, scale, retune, humanize |
| `pitchCorrectHard` | Pitch Correct (hard) | correct, chromatic, retune 0, humanize 0, gate −55 | key, scale, retune, mix |
| `deep` | Deep | shift −5 / formant −3 | pitch, formant, mix |
| `chipmunk` | Chipmunk | shift +7 / formant +5 | pitch, formant, mix |
| `monster` | Monster | shift −8 / formant −5, drive 40, space 30/800/20 | pitch, formant, drive, space level |
| `child` | Child | shift +4 / formant +3, tone low cut 150 | pitch, formant |
| `robot` | Robot | correct monotone retune 0, ring 50 Hz 70 %, crush 10 bits, flanger 0.2 Hz / 40 % / 50 % | ring Hz, ring depth, crush bits |
| `alien` | Alien | shift +3, ring 300 Hz 40 %, chorus 1.5 Hz / 60 % | pitch, ring Hz, mod depth |
| `radio` | Radio | tone 300–3,000 Hz, peak 1.5 kHz +4 dB, drive 30 | low cut, high cut, drive |
| `megaphone` | Megaphone | tone 500–4,000 Hz, peak 2 kHz +6 dB, drive 70 | drive, peak dB |
| `telephone` | Telephone | tone 300–3,400 Hz, crush 12 bits ↓6 (8 kHz) | high cut, crush bits |
| `whisper` | Whisper | whisper 100, tone 200–8,000 Hz, gate −50 | whisper, high cut |
| `cave` | Cave | space size 90, decay 2,500 ms, level 50 | size, decay, level |
| `custom` | Custom | neutral (all off) | none; every section open |

Preset rules:
- **Applying** a preset writes every parameter, not only the
  non-neutral ones, in one graph edit. That is one undo step.
- **"(edited)"** is computed in the UI by comparing every parameter with the
  published values, with a tolerance of 1e-6. The rule is: show the node's
  own saved value.
- **Reset to preset** is the same write as applying it.
- **Loudness:** each preset is levelled so that a test voice's RMS is
  within ±3 dB of neutral (test T1.10). Switching presets must never blast
  Discord.
- **Versioning:** a preset carries a version. A later retune bumps the
  version but never rewrites saved nodes.

## 7. Inspector (`ui/src/FunVoiceChangerEditor.tsx`)

- **Header:** a preset select listing all 14, an "(edited)" badge and
  **Reset to preset**. Its width is fixed so the badge does not move the
  layout.
- **Basic view:** the preset's sliders, taken from the published list.
- **Advanced view:**
  - one collapsible section per stage, each with its enable switch;
  - collapse state is per viewer, in `localStorage` with try/catch;
  - Custom opens every section.
- **Tuner strip:** note, cents and a voiced dot, at a reserved height,
  shown as "—" when the pitch stages are off or audio is stopped (UI-17).
- **Scale and key:** two selects, stacked, with captions above.
- **Styling:** app field styles only, no CSS specific to this panel.
  Screenshots in the dark, light and high-contrast themes.
- **Notes in the editor:**
  - "Output is mono."
  - "Adds 21 ms delay."
  - "Bypass jumps the timing; use Mix 0 % to compare smoothly."

## 8. Files to change (taken from how Spectral Gate is wired)

| Area | Files |
| --- | --- |
| DSP | new `crates/dsp/src/voice_changer.rs` (analysis, PSOLA, stages, limiter, the `VoiceChangerPresetId` enum with values); `crates/dsp/src/lib.rs` (export); `crates/dsp/Cargo.toml` (`size-2048`) |
| Domain | `crates/domain/src/lib.rs`: `NodeKind::FunVoiceChanger` and `ALL` (34 → 35); `type_name` `fun-voice-changer`; availability, cost `"high"`, `VOICE_CHANGER_LATENCY_SAMPLES = 1_024`; validation |
| Engine | `crates/engine/src/lib.rs`: stage, compile arm, registry, reset rule, both process paths (`:9517`, `:9650` siblings), four allow-lists (F5), a telemetry accessor |
| Control | `catalog.rs` (schema, catalog entry `effect`, `presets.voiceChanger`); `api_output_schema.rs` (the preset entry shape); `simple.rs` (label and description); `cable_routes.rs` (tool list); `native_workers.rs` (telemetry, both workers) |
| CLI / contracts | `crates/cli/src/lib.rs` tool list; `contracts/src/index.ts` kind union and telemetry type |
| UI | `draft.ts` (kind, defaults, 2-channel ports, list); `library.ts` (entry, notes of 180 characters or fewer); `parameterText.ts` (change summaries); `PropertiesPanel.tsx`; `SessionFlowCanvas.tsx` (class and label); `Workbench.tsx` (icon); `DraftConnectionList.tsx`; `ToolGuideLink.tsx`; the new editor and its test |
| Shell | `src-tauri/src/tool_guides.rs` slug `fun-voice-changer` |
| Site | `site/guide-content/processing.json`; a `guide-signal.js` voice-changer illustration (F14); regenerate with `site/generate-guides.mjs` (tool page, index, manifest, `tool-catalog.json`), then run `site/check-guides.mjs` |
| Docs | DSP-20 in `docs/spec/07-processing.md`; `docs/operations/quickstart.md` recipe; `docs/operations/api-reference.md`; the active-plan entry; a README row link to the disposition |

## 9. Tasks and tests

**T1 — DSP core** (`crates/dsp`, offline). The fixture is a synthetic voice:

- a glottal-pulse train (Rosenberg) through three formant resonators, F1/F2/F3
  for /a/, /i/ and /u/;
- f0 steps at 90, 150, 220 and 400 Hz, plus a glide from 110 to 440 Hz;
- vibrato of 5 Hz at ±30 cents;
- white-noise "s" bursts, −60 dBFS room noise, silence, and keyboard-like
  clicks.

Tests:

1. **Detection:** within 1 % on the steady vowels, no octave error across
   the fixture, and unvoiced or silent input is flagged unvoiced within
   20 ms.
2. **Correction:** within 10 cents of the scale note after 3 × the retune
   time; retune 0 steps within 256 samples; no warble at a note boundary
   (a voice ±20 cents around a boundary stays on one note).
3. **Shift:** pitch ratio within 10 cents for −12…+12. With formant 0, the
   F1/F2 peaks (from an LPC envelope) move less than 5 %. Formant +5 moves
   them by 2^(5/12) ± 5 %.
4. **Duration:** 60 s in gives 60 s out, ±0.1 %.
5. **Delay:** an impulse comes out at exactly 1,024 samples for neutral and
   for every preset.
6. **Neutral passthrough:** the output equals the input delayed by 1,024
   samples, to within −120 dB.
7. **Robustness:** NaN, Inf, denormals, full-scale squares, DC and silence
   never give non-finite output; the limiter peak stays ≤ −1 dBFS, also
   with drive 100 % and a full-scale input.
8. **Glides:** a preset change mid-vowel gives no sample step larger than
   the largest step of the steady signal × 2.
9. **Reset:** after `reset_pending`, there is no audio from before the
   reset.
10. **Loudness:** each preset's RMS on the voice fixture is within ±3 dB of
    neutral.
11. **Allocation:** `process` makes no heap allocation (counting allocator
    test, as other DSP tests do if available; otherwise a code review
    item).

**T2 — Engine**

1. Add the kind to `tests/tool_combinations.rs` `TOOLS` (every flag pattern,
   Mixer and connected-Mixer layouts, live replacement, dry mix), plus
   `tool_routes.rs`, `voice_tools.rs`, `deterministic_audio.rs` and
   `benches/realtime.rs` (all stages on).
2. A recompile mid-tone with a slider change gives continuous output (no
   21 ms gap).
3. A preset switch on a running route is click-free (T1.8 through the
   engine).
4. Verify F6: one stage per node per path, including after fan-out.
5. Bypass on, then off: no stale audio.
6. Stop/Play gives fresh state, and the registry entry is freed (`Weak`
   dead).
7. Privacy mute silences the output, including the reverb tail.

**T3 — Control:** validation (ranges, enums, integers, unknown keys); every
published preset validates; preset values in `describe()` match the DSP
enum; registry latency equals 1,024 equals the catalog value; an edited
preset survives save, reload and export/import unchanged; telemetry appears
in both worker kinds.

**T4 — UI**
- Editor tests: preset apply as one edit; "(edited)" on change; Reset;
  tuner placeholder; Custom opens every section.
- Edge e2e: add the tool, apply a preset, edit, reset, undo.
- Layout-shift sample of the tuner strip during simulated telemetry; it must
  give one value.
- Three-theme screenshots.

**T5 — Site:** the illustration fixture and controls, `check-guides.mjs`,
and the Firefox audio sweep (`tools/site/firefox-guide-audio.mjs`).

**T6 — Live**
- **Continuity:** the sine-continuity harness on the VB-Cable route with
  neutral Custom, with zero glitches.
- **Delay:** measure mic-to-cable delay with and without the tool.
- **CPU:** check the audioHealth record with the tool running (no new late
  gaps or underruns).

**T7 — Checks:** fmt and Clippy (workspace and `src-tauri`), UI format and
lint, all test suites.

**T8 — Attended (owner):** each preset through their microphone to Discord,
heard by a friend or through a second account. A sung Pitch Correct test.
Is the delay acceptable in conversation? Tune the preset values, then
re-run T1.10.

## 10. Possible issues and how the plan handles them

| Issue | Effect | Handling |
| --- | --- | --- |
| **Discord's own processing.** Krisp noise suppression, echo cancellation and automatic gain control. | It can strip Whisper and the noise in effects, gate Robot's buzz, and pump levels. | Guide and quickstart: set Discord's noise suppression to None (or Standard) and turn off its AGC for the cable input. Recommend AudioRouter Denoise **before** the voice changer instead. |
| **Noisy mic into pitch detection.** | Octave jumps and warble, and the effect sounds broken. | Input gate, voicing hysteresis and octave rejection. Recommend Denoise first. Test early on the owner's recorded voice (T1 can take a WAV). |
| **PSOLA artifacts at large shifts.** | Buzzy, "metallic" sound at ±12. | Presets stay within ±8, and the artifacts are acceptable for "fun". Disclose them in the guide. |
| **Delay stacking.** | About 76 ms from mic to Discord with the tool, about 98 ms with Denoise as well (F15 estimates). | Measure (T6). Disclose "+21 ms" in the inspector. The owner approved up to 25 ms for this tool. |
| **Self-monitoring.** | Hearing your own changed voice 50+ ms late makes speaking awkward. | Guide: monitor only briefly to check the sound, not while talking. |
| **Bypass jumps timing** (F4). | A 21 ms skip or repeat when toggled. | Reset rule (§5.3). The guide recommends Mix 0 % for comparing. Bypass behaves as it does for Pitch today. |
| **Live slider feedback only after a 400 ms pause** (F3). | Users hear a change only after they stop dragging. | Existing behavior for every tool. Kept state makes each apply seamless. Real-time dragging is out of scope. |
| **Recompile cost on the audio thread** (F3). | Late audio passes if compile allocates. | The registry makes recompiles allocate nothing large. Watch `lateGaps` in T6. |
| **Loudness jumps** (ring modulator, drive, reverb). | Blasting Discord, or a preset that is too quiet. | Per-preset levelling (T1.10), drive makeup and the always-on limiter. |
| **Feedback loop** with speakers and mic. | Howling, made worse by reverb and drive. | The limiter caps the level. Guide: use headphones. |
| **Stereo collapse.** | Music placed on the tool becomes mono. | Disclosed. It is a voice tool. Owner can choose L/R processing instead. |
| **Identity.** | Users may think the effect hides who they are. | Never call it anonymization. The guide says voices can still be recognized. |
| **Downgrade.** | Sessions with the new kind fail validation in 0.0.18 and older. | Release notes. The same concern as `duckAction`. |
| **IP and naming.** | "Auto-Tune" is an Antares trademark. | Use "Pitch Correct" and no trademarked preset names. The Antares pitch-correction patent (US 5,973,252, filed 1998) and the TD-PSOLA patents are believed expired. Confirm this in the licence review before release. Write the code in-house; no GPL. |
| **Denormals** (F13). | CPU spikes in reverb or flanger tails in silence. | Explicit flushing; tested with long silence after a tone (T1.7). |
| **The same node on two branches** (F6). | Shared state processed twice, corrupting the audio. | Verify in T2.4. If it happens, key the state by branch. |

## 11. Order of work

1. T1, the DSP core. Gate: T1 green on fixtures and a quick render of the
   owner's recorded voice (a WAV, with the owner's consent; not committed).
2. Domain, engine, registry, control and telemetry (T2, T3).
3. UI (T4), then site and docs (T5).
4. T6 and T7, then a review build in `target/reviews/…` (the owner's shell
   is never stopped; one shell at a time).
5. T8 attended tuning, then the final checks and the spec/plan updates.
   Commit only when the owner asks.

## 12. Rollback

The tool is a new `NodeKind`, and no existing tool changes. The only shared
change is the additive `microfft` feature. Reverting removes the kind.
Sessions saved with it would then fail validation, so a revert ships either
a migration that removes or bypasses such nodes, or a release-notes warning.

## 13. Out of scope

- AI or model-based voice conversion.
- MIDI-driven target notes and harmonies.
- Real-time drag feedback.
- Separate L/R processing (unless the owner chooses it).
- Changes to the existing Pitch tool.
- Trademarked names.
