# OBS and Discord setup help

Status: proposed. Not authorized for implementation. Investigation on
2026-10-23 at the owner's request ("Investigate OBS. We can do exception for
few very popular applications like OBS, Discord"). This exception is recorded
as DEC-22 in the [delivery specification](../../spec/15-delivery.md). It
relaxes the AGENTS.md rule against application-specific hooks for OBS Studio
and Discord only. Any other application needs its own owner decision.

## Objective

Make the most common streamer setup work the first time: a processed
microphone in Discord and OBS, game and Discord audio in the headphones and
on stream, and the voice recorded only once. AudioRouter cannot set another
app's devices permanently. It can see what each app actually uses, explain
the exact setting to change and, for OBS only, offer to apply it after the
user confirms.

## What the investigation found (2026-10-23, owner's PC)

- **Windows audio sessions (no app cooperation).** A read-only
  `IAudioSessionManager2` walk over every active render *and* capture
  endpoint names the processes using each one. Scratch script, not in the
  repository: `audio-sessions.ps1`.
  - The capture side was the unproven part, and it works. It showed Discord
    and Siege recording from CABLE-A Output, Discord playing to CABLE Input,
    and Siege playing to CABLE-B Input. That matches the owner's documented
    topology.
  - This would have caught the 2026-10-03 failure, when Discord silently
    moved to "CABLE In 16ch" (see the active plan).
  - Limitation: a loopback recorder of a playback device shows up as a
    playback session. `ow-obs` (Outplayed's embedded OBS) appeared on
    CABLE Input, which is the system audio it records. So a playback session
    means "playing to or recording the output of", and the UI must say so.
  - AudioRouter already enumerates render sessions for Application Capture.
    The capture-endpoint walk is new.
- **OBS Studio 32.2.2 (installed, not running).**
  - Its configuration is plain files in `%APPDATA%\obs-studio`: profile
    `basic.ini` and scene collections `basic\scenes\*.json`.
  - Audio sources store the same Windows endpoint ID strings AudioRouter
    uses (`device_id`, for example `{0.0.1.00000000}.{ef18e30a-…}` =
    CABLE-A Output), or `default`. Source kinds: `wasapi_input_capture`,
    `wasapi_output_capture` and `wasapi_process_output_capture`.
  - On the owner's scene collection, Mic/Aux was CABLE-A Output (the
    processed voice), but muted. Desktop Audio followed the Windows default
    playback device, which is the headphone mix that also carries the
    microphone. The stream therefore got the voice only through the
    headphone mix. Unmuting Mic/Aux would have recorded it twice, the same
    failure as the Outplayed double voice.
  - The obs-websocket server ships with OBS (port 4455, password
    authentication) but is **off by default** (`server_enabled: false`).
  - Protocol v5 (rpcVersion 1) provides:
    - `GetSpecialInputs` (Desktop Audio and Mic/Aux names), `GetInputList`,
      `GetInputSettings`/`SetInputSettings` (`device_id`), `CreateInput`
      (with a scene and, in OBS 32, `canvasUuid`) and
      `GetInputPropertiesListPropertyItems` (OBS's own device list);
    - mute, volume, monitor type, sync offset and per-input audio tracks
      (`Get/SetInputAudioTracks`), plus `GetStreamStatus`/`GetRecordStatus`;
    - live levels through the high-volume `InputVolumeMeters` event (every
      50 ms, opt-in subscription).

    Source: [obs-websocket protocol](https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md).
- **Discord 1.0.9264 (running).**
  - There is no supported settings file.
  - Its local RPC (IPC pipe `\\?\pipe\discord-ipc-{n}`) has
    `GET_VOICE_SETTINGS`/`SET_VOICE_SETTINGS`: input and output
    `device_id`, noise suppression, automatic gain control, echo
    cancellation and input mode.
  - However, RPC needs Discord's approval of the application. Unapproved,
    only up to 50 invited testers can use it.
  - Settings changed through RPC reset when the controlling app
    disconnects, and only one app may change them at a time. RPC therefore
    cannot set Discord up permanently.

  Sources: [Discord RPC](https://docs.discord.com/developers/topics/rpc),
  [discord.js scopes](https://discord.js.org/docs/packages/discord.js/14.22.1/OAuth2Scopes:Enum).

## Proposed stages

Each stage is a separate PR. Each changes no existing route behavior and
can be stopped after any stage.

| Stage | What the user gets | Mechanism | Risk |
| --- | --- | --- | --- |
| D1 | Every input and output shows which apps use its device ("Discord ✓ is using CABLE-A Output as its microphone"). An output named in a template warns when the expected app is on another device, such as Discord on "CABLE In 16ch". | Windows sessions, read-only, refreshed at most every few seconds off the audio thread | Low; new read-only backend query, API, contracts, UI |
| D2 | Guided setup by goal (Discord, OBS, clip recorder, headphones) builds a full route from existing tools and ends with exact per-app instructions, including turning off Discord noise suppression and automatic gain | Templates plus D1 for confirmation | Low–medium; UI and templates, no engine change |
| O1 | "OBS check": reads OBS's audio sources and says what each one hears (voice, game, Discord), and warns about voice twice or Desktop Audio following the default device | Read the active scene collection file when OBS is closed; obs-websocket `Get*` when it runs and the user enabled it | Medium; file parsing must be bounded and read-only; never write OBS files |
| O2 | "Fix it in OBS": one confirmed action sets Mic/Aux to AudioRouter's voice cable, adds Discord or game sources, and optionally puts voice, game and Discord on separate recording tracks | obs-websocket `SetInputSettings`, `CreateInput`, `SetInputAudioTracks`, each after a preview and user confirmation | Medium–high; writes to another app; needs undo (record previous settings and offer Restore) |
| X1 | Discord settings read, such as warning that Krisp is on | Discord RPC `GET_VOICE_SETTINGS` | Blocked: needs Discord's application approval; owner decision whether to apply |

## Rules for these integrations

- **Local only, opt-in, visible.** No cloud service. OBS is connected only
  to `127.0.0.1` after the user turns it on in AudioRouter. The OBS
  password is entered by the user and stored with Windows DPAPI for the
  current user, never in sessions, logs or exports.
- **Read before write; preview every write.** Every change to OBS is shown
  first, with its previous value, applied only after confirmation, and can
  be restored. AudioRouter never edits OBS or Discord files, and never
  changes Windows default devices (existing rule).
- **The backend owns it.** OBS and session queries go through the versioned
  API, so the UI, CLI, HTTP and MCP see the same results. Writes need an
  explicit grant: a new scope, or an existing scope with a decision record.
- **Real-time and privacy rules are unchanged.** All of this runs off the
  audio thread. Nothing here reads audio from another app or changes the
  protected voice path. The levels from `InputVolumeMeters` are OBS's own
  numbers, used only for display.
- **Failure is quiet and specific.** If OBS or Discord is not running,
  websocket is disabled, the password is wrong or a version is unsupported,
  show one clear status line. Never block Play or routing.

## Requirements and specification changes (before implementation)

- **CAP:** endpoint session inspection (D1), with the loopback ambiguity
  stated.
- **UI:** per-device "used by" status (D1), the setup-by-goal flow (D2),
  and the OBS check and fix panels (O1/O2) with UI-17 reserved slots.
- **AUTO/HTTP:** API methods and grants for session inspection and OBS
  read/write.
- **SEC:** OBS password storage, localhost-only connection, write
  confirmation and restore.
- **DEC-22:** the scope recorded in the delivery specification.

## Validation plan

- **D1:** unit tests with a fake session source (no app, one app, several
  apps, exited process, loopback ambiguity, device removed). A guarded
  Windows test on the owner's PC, compared with the scratch script's
  output; record app versions. No hosted-runner claim.
- **O1:** fixtures of real OBS 32 scene collections (redacted) and
  malformed, oversized and older files. Websocket tests against a fake v5
  server: authentication, wrong password, refused connection, unsupported
  rpcVersion.
- **O2:** confirm, apply and restore against a fake server, then one
  attended check with the owner's OBS on a copied profile (OBS `--profile`
  / `--collection`, never the live scene collection) before any release.
- **D2:** Vitest for each goal combination's graph (voice once per
  recorder, no feedback, CAP-10 clean), Edge in three themes, and
  first-run checks.

## Prerequisites and open decisions

1. Owner chooses the stages to authorize. Recommended order: D1, then O1,
   then D2, then O2.
2. OBS write access (O2): confirm that writing to OBS after confirmation is
   acceptable, and which grant governs it.
3. Discord RPC (X1): whether to apply for Discord's application approval.
   Without it, Discord support stays on Windows sessions plus written
   instructions.
4. Outplayed and Medal: only their embedded `ow-obs` is visible through
   Windows sessions. Any deeper integration is outside DEC-22 and needs its
   own decision.

Rollback: each stage is additive and can be reverted alone. No session
format change is planned. OBS changes made by O2 are restorable from the
recorded previous values.
