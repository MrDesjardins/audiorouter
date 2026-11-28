# Multichannel output (A4 option a): audit and preparation

Date: 2026-11-27. Branch: `t3/multichannel-output-qualification`, stacked on
`feat/multichannel-output` (implementation `bb479548d`). Owning plan:
[owner decisions](../owner-decisions-2026-11-27.md). No driver, publication,
release or live-audio change was made in this task.

## Scope

Owner decision A4 (2026-11-27): a stereo route plays into the front left/right
of any multichannel output device, and the other channels stay silent, so
whatever virtual cable a PC has is usable. This record audits that
implementation, records the local checks, and prepares the two hardware gates.
It does not claim either hardware gate is passed.

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
  `prepare_native_output_fanout_with`; only the device frame width grows. The
  pump's frame counts, ring shape and drift controller are unchanged.
- `output_format_supported` accepts IEEE-float32 render devices with 2 to 32
  channels at 8-192 kHz; mono is still refused. The single application worker
  and process-loopback render binding stay stereo, as before.

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
  predicate and the widening rule. The single-endpoint worker row was left as
  stereo because that worker is unchanged.

No code defect was reproduced by the audit or the tests below. The plan's
original step 1 wording mentioned reading the channel mask; the shipped rule is
the equivalent first-two-channels rule described above, and the plan status was
corrected to match the code.

## Local checks (Windows 11 dev PC, Rust 1.96.0)

All were run on 2026-11-27 against the current branch; nothing below used audio
devices or the owner's data.

| Check | Command | Result |
| --- | --- | --- |
| Rust formatting | `cargo fmt --all -- --check` | clean |
| Rust formatting (shell) | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | clean |
| Clippy workspace | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| Clippy shell | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | clean (after `ui/dist` was built; the first attempt failed only because the embed path was absent) |
| Windows audio | `cargo test -p audiorouter-windows-audio --lib` | 169 passed, 4 ignored |
| Control | `cargo test -p audiorouter-control --lib` | 530 passed, 31 ignored |
| Engine, all targets | `cargo test -p audiorouter-engine` | 189 unit + every integration suite passed |
| Source layouts | inside the engine run: `source_combinations` | 6/6 passed |
| Tool layouts | inside the engine run: `tool_combinations` | 6/6 passed |
| Docs | `powershell -NoProfile -ExecutionPolicy Bypass -File .\\tests\\acceptance\\docs.ps1` | 331 files, 1952 links passed |

The widening unit tests cover 3, 4, 6, 8 and 16 device channels, equal-width
pass-through, partial/non-float/oversized refusals, and the float32 mix-format
predicate. The setup-check regression now expects a 7.1 output to play and a
mono or non-float output to be refused through the same predicate Play uses.

Desktop preparation: `powershell -NoProfile -ExecutionPolicy Bypass -File
tools/build-desktop.ps1` completed. Shell
`src-tauri/target/release/audiorouter-shell.exe` SHA-256
`2ADA29A93BAD12F493E45ADF130A48E44E0A1459F21037C95839169CCCFD13E0`,
built after `ui/dist`. Other workers and `audiorouter-cli.exe` were produced
with their own hashes by the same script. The continuity test binary was
compiled without running it:
`cargo test -p audiorouter-transport --test live_audio_continuity --release --locked --no-run`.

## Hardware gate 1: continuity through CABLE-B at 7.1

This is A4's required hardware proof and reuses the C1 harness. It needs the
owner's quiet window, and it must not overlap the live-reliability thread's
CABLE runs.

Preparation and prerequisites (to confirm in the window):

1. CABLE-B Input is configured for 7.1 at 48 kHz (the owner states it is). Its
   current ID is expected to be
   `{0.0.0.00000000}.{03498a2a-ef6a-4dd4-b9ba-72e22f567d7a}`; CABLE-B Output
   capture is expected to be
   `{0.0.1.00000000}.{ef18e30a-69c9-4299-bae2-1a2452fb8e4f}`. Read the active
   formats without changing Windows Sound settings; use the disposable profile
   or the M00 probe only inside the quiet window, because the probe starts each
   render stream briefly.
2. Close Discord, Zoom, games and AudioRouter; nobody may be listening to
   CABLE-A or CABLE-B. Confirm `Get-Process audiorouter-shell*` prints nothing.
3. Do not run this while C1 or any other cable run is active.

Agent command (the same route as C1, so CABLE-B Input is the route's render
endpoint and the stereo graph must be widened):

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
check. The unit tests already prove channels 2..N are exact silence. Pass: zero
routed glitches and no silent run on a clean reference.

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

- No continuity or built-app Play has run yet: the quiet/attended window is
  not available to this thread. Both gates remain open and A4 is not hardware
  complete until they pass.
- The live-reliability thread owns C1; this run must be scheduled separately
  (sequentially, never at the same time). The same endpoints are involved.
- Not re-run: full UI test suites (no UI code changed), the eight-hour soak
  (owner-deferred), and the release qualification gates.
