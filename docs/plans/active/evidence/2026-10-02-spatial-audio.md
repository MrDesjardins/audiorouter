# Surround to headphones (CAP-14) and automatic route restart

Date: 2026-10-02. Windows 11 x64, MSVC, Node 22, Edge Playwright harness.
User decisions: real 5.1/7.1 capture rendered binaurally; measured MIT KEMAR
diffuse-field HRTF (free use with citation, Gardner & Martin 1994).

## Implementation

- `tools/hrtf/generate-kemar-table.mjs` converts KEMAR `diffuse.zip`
  `elev0/H0e{000,030,090,150}a.wav` (44.1 kHz, 128 taps) to the generated
  `crates/dsp/src/kemar_hrir.rs` (48 kHz, 140 taps, frontal unit energy).
  Release `THIRD-PARTY-NOTICES.txt` gains a bundled-data citation.
- `audiorouter_dsp::binaural::BinauralRenderer`: per-speaker time-domain FIR,
  layout from the Windows channel mask (5.1 `0x3F`/`0x60F`, 7.1 `0x63F`),
  LFE −6 dB to both ears, preallocated, allocation-free rendering.
- `windows-audio`: `BinauralPacketConverter`/`BinauralCapture` turn 6/8-channel
  packets into stereo; `SharedCapture::open_loopback` captures a render
  endpoint (polled). VB-Cable's recording side stays 2-channel in shared mode
  ([VB-Audio forum](https://forum.vb-audio.com/viewtopic.php?t=1754)), so the
  supported route is loopback of the cable's playback side set to 7.1.
- Control: `spatialMode` (`off`|`headphones`) on Physical Input; prepare
  validates 48 kHz float, 6/8 channels and mask with node-named messages;
  surround loopback reaching the same render endpoint is feedback; mode change
  while playing reports restart required. UI: Spatial audio field and
  Loopback device choices.

## Automatic restart defect (found during the Mixer attempt)

Reproduction (user session revision 298/300, review build
`review-connected-mixers-20261002`): adding a Mixer that joins the game and
voice paths while playing was saved as `restartRequired`; the commit had
already advanced the runtime generation, `multi_input_worker_serves` no longer
matched, and the service stopped pumping. Live meters stayed frozen at
identical values for >2 s and the user heard nothing until Stop/Play.
Repair: a refused live change on the multi-path worker now restarts the route
(Stop, detach, prepare, Play) and reports `restarted`; while a recording runs
the old route keeps serving the new generation and `restartRequired` remains.
Not yet qualified live: needs an attended save of a path-changing edit while
playing (with and without a recording).

## Checks (Windows)

| Check | Result |
| --- | --- |
| `cargo test -p audiorouter-dsp --lib binaural` | 5 passed (+1 ignored timing) |
| Release timing, 10 s of 7.1 | 5.19 ms CPU per second of audio (0.52% of a core) |
| `cargo test -p audiorouter-windows-audio --lib binaural_converter` | 1 passed |
| Live loopback, Sonar Gaming (8 ch, mask 0x63F, 96 kHz) | opened; 0 packets while idle (loopback delivers nothing without playback) |
| Live loopback, CABLE-B Input (2 ch) while Siege played | 49 packets, 23 520 frames in 500 ms |
| `cargo test -p audiorouter-domain --lib spatial` | 1 passed |
| Control library | 222 passed, 8 ignored, 1 failed: `recorder_factory_creates_attaches_and_indexes_a_wav_before_arm` (deterministic, 6/6 runs; belongs to the uncommitted threaded-recorder work, which these changes do not touch; not re-run on a tree without them) |
| Transport library | 25 passed |
| UI `npm test` / typecheck | 45 files, 445 tests passed |
| `e2e/spatial-audio.pw.ts` | 3 passed; screenshots [dark](screenshots/2026-10-02-spatial-audio/spatial-audio-dark.png), [light](screenshots/2026-10-02-spatial-audio/spatial-audio-light.png), [high contrast](screenshots/2026-10-02-spatial-audio/spatial-audio-high-contrast.png) |

Review build: `target/reviews/spatial-audio-20261002/` (shell SHA-256
`7254794165dcf1b2720c9f48f2da310e213fcf548a5ea9b00b4701436454e711`, worker
`0d75f5f62406856c92a91eb9ae3873af6652b6f8dda6df87a7b43465e5bfde31`), built with
`AUDIOROUTER_BUILD_ID=review-spatial-audio-20261002`, `--release --features
custom-protocol --offline` (the shell lock gained the local `audiorouter-dsp`
path dependency). UI bundle 19:16, shell 19:18. Not launched: the user's
release-preview shell (PID 51128) is running.

Open: an end-to-end 7.1 signal through prepare/start (needs CABLE-B Input set
to 7.1 by the user) and the attended Siege listening test; fresh-install and
installer gates are not claimed.
