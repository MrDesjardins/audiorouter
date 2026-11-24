# Low-severity audit repairs: DSP-3, DSP-5, DSP-6, REC-3 — 2026-11-24

Source: [2026-11-11 bug audit](../../archived/2026-11-11-bug-audit.md).
Branch: `fix/audit-low-dsp-recording` (from `origin/main` 0fb41911b, release 0.0.32).
Requirement areas: DSP quality, recording (REC), Time Shift tool.
Environment: Windows 11, Rust 1.96.0 pinned toolchain, `CARGO_BUILD_JOBS=6`,
debug builds only. Done unattended overnight; no app, driver or release build
was started.

## Per finding

### DSP-3 — infinite sample latches the interleaved compressor (fixed)

- **Still present on main:** yes. `Compressor::process_interleaved` and
  `Gate::process_interleaved` computed the detector peak from raw `sample.abs()`;
  only the output path sanitized. `process_planar_linked` already sanitized.
- **Regression tests** (`crates/dsp/src/lib.rs`):
  `interleaved_compressor_recovers_after_infinite_samples` (`+inf`, `-inf`, NaN;
  mono and stereo; a -20 dBFS sine on the next block must keep its energy and
  `gain_reduction_db` must be finite and small) and
  `interleaved_gate_treats_infinite_samples_as_silence`.
- **Failing before the fix:** both failed. Compressor: `compressor stayed
  silent after inf (1 ch): 0 vs 5.0126996` (output energy 0). Gate: `inf must
  not open the gate as if it were loud signal`. (The gate did not latch, since
  an open gate has target gain 0 dB, but a bad sample opened it and started the
  hold timer, which is the same root cause.)
- **Fix:** both interleaved detectors take the peak from `finite_or_zero(sample)`;
  the compressor envelope is clamped to [-120, 120] dB (`ENVELOPE_FLOOR_DB`,
  `ENVELOPE_CEILING_DB`) so it stays finite. Planar paths unchanged.
- **Files:** `crates/dsp/src/lib.rs`.

### DSP-5 — biquad ramp counts samples, not frames (fixed)

- **Still present on main:** yes, `advance_ramp()` ran once per sample.
- **Regression test:** `stereo_biquad_ramp_counts_frames_and_keeps_channels_identical`
  — a stereo biquad with a 64-frame ramp, equal L/R input, must give equal
  channels, match a mono biquad with the same ramp, and finish exactly after 64
  frames (`ramp_remaining == 0`, coefficients equal to the target).
- **Failing before the fix:** `channels diverged inside the ramp` (left
  0.13610294, right 0.13570055).
- **Fix:** `Biquad::process_interleaved` advances the ramp only on channel 0 of
  each frame. The `set_params_ramped` doc comment now says frames.
- **Files:** `crates/dsp/src/lib.rs`.

### DSP-6 — Time Shift pause cuts to silence (fixed)

- **Still present on main:** yes, `read_channel` filled zeros as soon as
  `paused` was set.
- **Regression test:** `pause_fades_out_instead_of_clicking`
  (`crates/dsp/src/timeshift.rs`): a 100 Hz sine at 48 kHz through 128-frame
  blocks, paused on a non-zero part of the waveform; the largest sample step
  across live blocks, the pause block and the following silent block must stay
  within twice the sine's own maximum step, the pause block must still carry
  audio, and later blocks are exactly zero.
- **Failing before the fix:** `the pause block must still carry the fading
  audio` (the whole pause block was zero).
- **Fix:** new `pause_fade` flag, set in `begin_block` on the block where
  `paused` turns true. That block reads the delayed audio (the delay has not
  moved yet) with a linear fade that reaches exactly 0.0 on its last frame;
  later paused blocks are silent. The fade spans the pause block, which is 128
  frames (2.7 ms at 48 kHz) in the engine. Behaviour for resume, jump and live
  is unchanged.
- **Two existing tests asserted the old hard cut** and were updated to the new
  contract (pause block fades to zero, every later block is silent):
  `time_shift_buffer_survives_recompilation_and_obeys_transport`
  (`crates/engine/src/lib.rs`) and
  `time_shift_transport_returns_exact_recorded_audio_after_jump_fades`
  (`crates/engine/tests/deterministic_audio.rs`).
- **Files:** `crates/dsp/src/timeshift.rs`, the two engine tests above.

### REC-3 — WAV writer omits the RIFF pad byte before INFO (fixed)

- **Still present on main:** yes, `WavWriter::finish_with_metadata` appended
  `LIST` directly after the data chunk.
- **Regression test:** `wav_metadata_survives_an_odd_sized_data_chunk`
  (`crates/recording/src/lib.rs`): three 24-bit mono frames (9 data bytes)
  with title and artist; checks the pad byte position, that the RIFF size
  equals file length minus 8, that `inspect_wav_file` still reports 3 frames
  and 9 data bytes, and that `read_wav_metadata` returns both tags.
- **Failing before the fix:** the byte after the data was `INFO`-body text
  (`[73, 83, 84, 42]` instead of `LIST`), so the tags were lost on reload.
- **Fix:** write one zero byte after an odd-sized data chunk when an INFO chunk
  follows, and count it in the RIFF size. Files with no metadata are unchanged
  (no trailing pad), so the existing size checks (`recover_wav_file`, the
  44 + data layout) behave as before. Real recordings use whole 128-frame
  quanta, so this stays latent in the app; the test builds the odd case by hand.
- **Files:** `crates/recording/src/lib.rs`.

## Commands run

All from the worktree, `CARGO_BUILD_JOBS=6`.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean (after dropping one `mut` in the new Time Shift test) |
| `node tools/docs/validate.mjs` | passed (296 files, 1642 links) |
| `cargo test -p audiorouter-dsp` | 87 + 2 passed |
| `cargo test -p audiorouter-recording` | 49 passed |
| `cargo test -p audiorouter-control` | 453 passed (24 ignored) |
| `cargo test -p audiorouter-storage` | 113 passed |
| `cargo test -p audiorouter-engine` (all targets, includes `tool_combinations` 6 and `source_combinations` 6) | all passed, incl. `deterministic_audio` 33 |

## Not run

- `cargo clippy --manifest-path src-tauri/Cargo.toml ...`: the worktree has no
  `ui/dist`, so the Tauri build script fails (`frontendDist ... doesn't exist`).
  No `src-tauri` code was changed; only its formatting check ran.
- The hardware sine-continuity harness
  `live_backend_service_keeps_a_routed_tone_continuous`, and any listening test
  of the Time Shift pause, the EQ ramp or the compressor. Those need audio
  devices and the owner's PC. The Time Shift fade is verified numerically only.
- `cargo test` for `audiorouter-transport`, `-cli` and `-windows-audio` (they
  depend on the touched crates but have no changed code or expectations).
- No release build, no launch of the app, no installer.

## Notes

- Fixes are deliberately in the owning crates (`dsp`, `recording`); no API,
  contract or MCP surface changed, so no schema, `API_METHODS` or site
  catalog updates were needed.
- The DSP-3 envelope clamp only applies to the interleaved compressor; the
  planar path already sanitizes and is left as it was.
