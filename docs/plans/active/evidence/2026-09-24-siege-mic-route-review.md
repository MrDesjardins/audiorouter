# Siege and microphone route review — 2026-09-24

## Environment and scope

- Windows 11 desktop; AudioRouter was launched with `cargo tauri dev` from
  `src-tauri`, using the user's local AudioRouter database and explicit
  `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` opt-in.
- The shell embedded its own persistent control backend. No separate
  `audiorouter-cli backend serve` process was used.
- User clarified during setup that game and mic routing must be in one session
  for Siege, Discord, and Outplayed to run together.

## Live observations

- The first attempted configuration created `siege-game-eq` (Cable B capture
  -> Advanced EQ -> Focusrite render) and `voice-to-cable-a` (PD200X capture
  -> Advanced EQ -> Cable A render) as two sessions. Both exact endpoint pairs
  prepared successfully and reported `configured-stopped`.
- No route was started. After the user clarified the one-session requirement,
  both exact endpoint workers detached successfully. After the user explicitly
  authorized it on 2026-09-24, both assistant-created saved sessions and their
  histories (`siege-game-eq`, `voice-to-cable-a`) were deleted through the
  running backend.
- The saved EQs are flat with all bands disabled. No VST was selected because
  the user has not identified a plugin or settings curve.
- Siege and Outplayed were not confirmed running, so their application-level
  device selections were not changed or verified.

## Findings

- The current native multi-input compiler accepts direct sources into one
  mixer, then a shared processing chain and a common output fan-out. That would
  mix game audio into the voice-mic feed and is not acceptable for the requested
  setup.
- The standard realtime scheduler currently accepts one input block and
  produces one output block; the Windows endpoint adapter binds one capture and
  one render endpoint. Independent per-source chains and destinations are not
  currently qualified within one session.
- Do not present the two stopped sessions or a shared mixer as the final
  configuration. Implement per-source scheduling and exact node endpoint
  binding before activating a one-session route.

## Checks

- `cargo test -p audiorouter-windows-audio --locked`: 90 passed (after mono
  capture duplication changes; Windows unit tests, not live route evidence).
- `cargo test -p audiorouter-control --locked`: 185 passed, 4 ignored, 0 failed.
- `npm.cmd test` in `ui`: 314 passed across 23 files.
- `npm.cmd run build` in `ui`: TypeScript check passed; Vite output failed with
  `EPERM` while unlinking the existing `ui/dist/assets/index-D5vW3VSB.js`.
- `cargo tauri dev` compiled and launched the shell successfully. Its live UI
  event log is `%LOCALAPPDATA%\AudioRouter\logs\shell.jsonl`; the log records
  UI subscriptions but not the direct endpoint-prepare RPCs.
- `cargo fmt --check -p audiorouter-control -p audiorouter-windows-audio`
  reported formatting differences in the already-dirty control source.
- `git diff --check`: passed with no whitespace errors.

## Next acceptance

Build and save one session with Cable B game input -> independent game EQ ->
Scarlett output and PD200X mic -> independent voice EQ/VST -> Cable A output.
Then configure Siege output and Siege/Discord microphone selection, configure
Outplayed to capture the processed game and microphone paths, and verify both
paths at once with per-path meters and no game-to-voice-mic crossfeed. Record
Windows device, process, and live signal evidence before claiming success.
