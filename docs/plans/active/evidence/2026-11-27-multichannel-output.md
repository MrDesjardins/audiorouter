# Multichannel output (A4 option a): audit and preparation

Date: 2026-11-27. Branch: `t3/multichannel-output-qualification`. It contains
`feat/multichannel-output` (including `205892499`, the single-endpoint and
application-worker widening, and `0145e83f2`, the live results) merged, and the
docs/evidence commits are stacked on that branch. Owning plan:
[owner decisions](../owner-decisions-2026-11-27.md). No driver, publication,
release or live-audio change was made in this task.

## Scope

Owner decision A4 (2026-11-27): a stereo route plays into the front left/right
of any multichannel output device, and the other channels stay silent, so
whatever virtual cable a PC has is usable. This record audits that
implementation, records the local checks, and records the live continuity
result plus the remaining attended gate. It does not claim the feature or a
release is qualified.

## What the implementation does

- `SharedRender::submit_bytes` accepts either the device's own stride (an exact
  copy) or, for an IEEE-float32 stream, a narrower float32 stride. A narrower
  source is widened frame by frame into the borrowed WASAPI buffer; the source
  fills the frame's first channels and every remaining channel is written as
  exact zero bytes. `widen_float32_frames` never allocates and is bounded
  against both slices.
- Windows orders a standard channel mask by bit, and front left/right are the
  first two bits in every 4-, 6- and 8-channel layout (quad `0x33`, 5.1
  `0x3F`/`0x60F`, 7.1 `0x63F`). The implementation therefore uses channels 0
  and 1, which the specification states as "front left/right in any standard
  layout"; it does not need to read the mask. A mask without front left/right
  is not a layout this feature claims.
- The multi-path output fan-out is fixed at 2 graph channels in
  `prepare_native_output_fanout_with`, and the single-endpoint bridge keeps a
  stereo graph and stereo resamplers whatever the render device's width
  (`205892499`). Only the device frame width grows. The pump's frame counts,
  ring shape and drift controller are unchanged.
- `output_format_supported` and `render_channels_supported` accept IEEE-float32
  render devices with 2 to 32 channels (at 8-192 kHz where the rate is checked);
  mono is still refused. The multi-path fan-out, the single-endpoint bridge and
  the application worker use the same `MAX_RENDER_CHANNELS` limit.

## Coverage audit and defects fixed

The implementation and its tests were present and consistent. The audit found
three stale user/specification statements that still said an output must be
stereo, so step 4 of the plan ("docs say what a multichannel output plays") was
not actually complete:

- [spec/09-interface.md](../../../spec/09-interface.md): the setup-check contract
  called a 7.1 output wrong and described the output finding as "not stereo
  float". Corrected to "IEEE float32 with 2 to 32 channels".
- [quickstart.md](../../../operations/quickstart.md): the setup-check section said
  a 7.1 cable is not an output. Corrected, with the front-left/right and silence
  behavior stated and the same-cable example narrowed to capture.
- [api-reference.md](../../../operations/api-reference.md): `nativeOutputs.prepare`
  still described one to eight stereo 48 kHz clients. Corrected to the new
  predicate and the widening rule, and the `nativeEndpoints.prepare` paragraph
  now states that its render side plays the stereo graph on the first two
  channels too.

The audit of the first implementation (`bb479548d`) found no code defect. The
later `205892499` fixed a gap found during integration: the single-endpoint and
application workers still refused a wide output even though the UI uses them
for a simple route. The plan's original step 1 wording mentioned reading the
channel mask; the shipped rule is the equivalent first-two-channels rule
described above, and the plan status was corrected to match the code. The local
checks below were re-run after merging that implementation.

## Local checks (Windows 11 dev PC, Rust 1.96.0)

All were run on 2026-11-27 against the current branch; nothing below used audio
devices or the owner's data.

| Check | Command | Result |
| --- | --- | --- |
| Rust formatting | `cargo fmt --all -- --check` | clean |
| Rust formatting (shell) | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | clean |
| Clippy workspace | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| Clippy shell | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | clean (after `ui/dist` was built; the first attempt failed only because the embed path was absent) |
| Windows audio | `cargo test -p audiorouter-windows-audio --lib` | 170 passed, 4 ignored |
| Control | `cargo test -p audiorouter-control --lib` | 530 passed, 31 ignored |
| Engine, all targets | `cargo test -p audiorouter-engine` | 189 unit + every integration suite passed |
| Source layouts | inside the engine run: `source_combinations` | 6/6 passed |
| Tool layouts | inside the engine run: `tool_combinations` | 6/6 passed |
| Docs | `powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\docs.ps1` | 331 files, 1952 links passed |

The widening unit tests cover 3, 4, 6, 8 and 16 device channels, equal-width
pass-through, partial/non-float/oversized refusals, and the float32 mix-format
predicate. The setup-check regression now expects a 7.1 output to play and a
mono or non-float output to be refused through the same predicate Play uses.
The endpoint-bridge regression keeps a stereo graph and resamplers for a
multichannel render device (`205892499`).

Desktop preparation: a fresh release build was made from the merged head for
the attended gate. Shell
`src-tauri/target/release/audiorouter-shell.exe` SHA-256
`5D06A08C7DA19EB9461BA4ECC812A964D17E8DAAB11DEE4136B09ED6719F89BA`,
built after `ui/dist`; the other workers and `audiorouter-cli.exe` were
produced by the same script. The live continuity run used the release
continuity binary on the feature branch.

## Hardware gate 1: continuity through CABLE-B at 7.1 (passed)

Passed on 2026-11-27 in the owner's quiet window and recorded with the full
table in the [owning plan](../owner-decisions-2026-11-27.md#a4-option-a-implementation-plan-branch-featmultichannel-output);
this record does not re-run or re-claim it. CABLE-B Input was 8 channels,
float32, 48 kHz. The harness routed CABLE Output -> Gain -> CABLE-B Input and
read CABLE-B Output. The multi-path runs had zero routed glitches with a clean
reference; the single-endpoint worker passed 3 of 3 clean runs after one
unattributed failure that kept no reference figures. The routed tone reached
full level, so the stereo route lands on the 7.1 cable's front left/right.
Channels 3-8 were not read on hardware (CABLE-B Output is stereo; the unit tests
cover the silence); the application worker was not run live.

Reproduction command (the same route as C1, so CABLE-B Input is the route's
render endpoint and the stereo graph must be widened):

```powershell
$env:CARGO_BUILD_JOBS=4
cargo test -p audiorouter-transport --test live_audio_continuity --release --locked --no-run
$env:AUDIOROUTER_LIVE_CONTINUITY='1'; $env:AUDIOROUTER_CONTINUITY_SECONDS='20'
$env:AUDIOROUTER_CONTINUITY_TONE_RENDER_ID='{0.0.0.00000000}.{71f96f14-94c1-4189-b9b9-df8c960db8f2}'
$env:AUDIOROUTER_CONTINUITY_ROUTE_CAPTURE_ID='{0.0.1.00000000}.{06268191-5f8c-42ed-827e-d3c7a19637ed}'
$env:AUDIOROUTER_CONTINUITY_ROUTE_RENDER_ID='{0.0.0.00000000}.{03498a2a-ef6a-4dd4-b9ba-72e22f567d7a}'
$env:AUDIOROUTER_CONTINUITY_RESULT_CAPTURE_ID='{0.0.1.00000000}.{ef18e30a-69c9-4299-bae2-1a2452fb8e4f}'
$env:AUDIOROUTER_CONTINUITY_CHAIN='gain'
cargo test -p audiorouter-transport --test live_audio_continuity --release --locked -- live::live_backend_service_keeps_a_routed_tone_continuous --ignored --exact --nocapture
```

The recorder analyses the result-capture left channel, which is front left
after widening; a wrongly routed or silent channel 0 fails the steady-tone
check. The unit tests already prove channels 2..N are exact silence.

## Hardware gate 2: attended Play in the built app

Uses the shell above, on a disposable profile, with the owner present. Do not
set `AUDIOROUTER_ALLOW_DEVICE_ADMIN`; answer the in-window device question.

1. Close every AudioRouter, then launch the built shell with a disposable
   `LOCALAPPDATA`, `AUDIOROUTER_DATABASE` and `WEBVIEW2_USER_DATA_FOLDER` as in
   [human-gated tasks](../human-gated-tasks.md#common-preparation).
2. Build an unsaved route: Test Signal -> Output device -> CABLE-B Input
   (shown as 8 channels / 7.1 in the device picker). Leave Windows Sound
   settings and every live route untouched.
3. Press Play; require no error, the output meter moving, and the route's
   device labelled with its channel count. The tone is the Test Signal.
4. Search that profile's `logs\backend.jsonl` for `"outcome":"error"` (expect
   none) and Stop.
5. This is attended UI/hardware evidence only if the owner saw Play and the
   meter; a CDP or unit check does not satisfy it.

## Not run / open

- Attended Play in the built app remains open; it needs the owner present. The
  live continuity gate passed (above), and the application worker was not run
  live (its code path is covered by the unit and compile checks).
- Hardware did not verify channels 3-8 exact silence; CABLE-B Output is read as
  stereo, and the unit tests cover the silence.
- Not re-run: full UI test suites (no UI code changed), the eight-hour soak
  (owner-deferred), and the release qualification gates. No feature or release
  is qualified.
