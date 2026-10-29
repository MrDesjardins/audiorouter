# Soundboard tool

Status: future plan, owner request 2026-10-29. Not authorized for implementation;
promote it by the steps at the end of [the future-plans index](README.md).

## Outcome

A new canvas tool, **Soundboard**, holds up to nine short sounds. Each sound has a
button on the tool itself. Pressing a button sends that sound out of the tool
into the route (for example into the virtual microphone, so people on a call hear
it). The same trigger is available from the local API, so a Stream Deck key can
fire one specific sound, and configuring that key must be easy.

Nine is a hard limit that keeps the tool a sensible height. A person who needs
more adds another Soundboard.

## Why this fits the product

- The Audio File tool already plays one imported WAV or MP3 into a session with
  play, pause and stop (`audioSources.transport`). A Soundboard is nine of those
  with a mixed output and instant, overlapping triggers; it is not a new kind of
  audio source.
- The Stream Deck plugin already drives the local HTTP API with a bearer token
  and already resolves a tool by id or name (`step`, `toggle`, `play` actions in
  `tools/streamdeck/src`). A soundboard action is the same pattern.
- The existing backlog row "Dedicated soundboard UI with hotkeys and fades"
  asked for a definition of mixing, shortcut conflicts and fade behavior. This
  plan answers those.

## Requirements to define (proposed IDs, not yet in the spec)

New IDs would go in `docs/spec/` (tool catalog, processing, interface and API
sections) when the plan is promoted. Existing requirements it touches: UI-17
(no layout shift), UI-20 (tool guide page for every addable tool), API/CLI/MCP
parity, the audio-callback rules in `AGENTS.md`, and the media upload limits.

| Proposed ID | Statement |
| --- | --- |
| SND-01 | The Soundboard has exactly nine slots. Each holds at most one imported audio file, or is empty. |
| SND-02 | A slot's button on the canvas and its Play button in Properties start that slot's sound from the beginning. |
| SND-03 | Pressing a slot that is already playing restarts it. A **Stop all** control silences the tool at once. |
| SND-04 | Slots can overlap. The tool's output is the mix of all playing slots, with headroom so overlap cannot clip hard. |
| SND-05 | Each slot has a caption. When the caption is empty, the file name is shown and used. |
| SND-06 | The user can add, replace and remove the file of a slot, and rename its caption, from Properties. |
| SND-07 | The backend owns the trigger: `soundboard.trigger` (slot) and `soundboard.stop` (one slot or all) in the versioned API, with CLI and MCP parity. |
| SND-08 | Triggering needs `SessionControl` permission, not graph-write permission, so a Stream Deck key cannot change the graph. |
| SND-09 | A trigger that cannot play (session not running, empty slot, tool bypassed) returns a clear error and is shown on the Stream Deck key. |

## Behavior decisions

1. **Playback style.** One-shot by default. A slot does not loop. Looping is out
   of scope unless the owner asks, to keep the nine buttons predictable.
2. **Retrigger.** Restart the sound. This is what people expect from a soundboard
   and avoids stacking the same clip into noise.
3. **Overlap.** Different slots may overlap. Mixing is summed with a fixed
   per-voice headroom, then a safety limiter on the tool output so nine loud
   clips cannot clip the route. Open question: fixed headroom only, or headroom
   plus limiter. Decide with a measurement on nine full-scale files.
4. **Output.** Stereo, one output port. Mono files are duplicated to both
   channels. Files are decoded and resampled to the session rate when the session
   is prepared, using the existing decoder in
   `crates/engine/src/audio_file_source.rs`.
5. **When it works.** Only while the session is running. On a stopped session the
   buttons are disabled with a short reason, and the API returns a stable error
   (SND-09). Triggering never starts a session by itself.
6. **Level.** No per-slot level in the first version. The tool sits in a graph,
   so a Gain or Volume after it covers loudness. Reconsider if testing shows
   mismatched file loudness is a real problem.
7. **Fades.** None in the first version. The old backlog row mentioned fades;
   a short fixed fade at the end of Stop all (to avoid a click) is enough, and
   it is part of making Stop all safe, not a user setting.

## Canvas card

- Show the slots as a **vertical list of buttons** on the tool card, one per row,
  caption as the label. A list fits captions better than a 3 by 3 grid, because
  a caption is the part people read.
- Rows have a fixed height and the caption is truncated with an ellipsis; the
  full caption and file name appear in a tooltip and in Properties.
- Show only the assigned slots plus one "Add sound" row, so a board with three
  sounds is not nine rows tall. The card changes height only when the user adds
  or removes a sound, which is a deliberate action (UI-17). A playing slot shows
  its state in a fixed-width marker, never by resizing.
- A button works without selecting the tool first. It needs the React Flow
  `nodrag` treatment so pressing it does not start a drag.
- Buttons are disabled with a reason when the session is stopped or the tool is
  bypassed.
- Playing state comes from live telemetry through `liveTelemetry.tsx`, not from
  app-level state (validated lesson: 20 Hz data must not re-render the app).

## Properties window

For each assigned slot, one row, with the caption above a full-width field:

- **File:** the file name, duration and format. This is the identification the
  owner asked for. It is read-only; the file itself is changed with **Replace**.
- **Caption:** text field. Placeholder shows the file name; empty means use it.
- **Play** and **Replace** and **Remove** buttons.
- An **Add sound** control for the next free slot, using the existing upload flow
  (`audioMedia.beginUpload`, `uploadChunk`, `finishUpload`; UI in
  `ui/src/audioUpload.ts` and `AudioFileNodeEditor.tsx`).
- A **Stop all** button and a short note: "Plays only while the session is running".

All fields use the app-wide field style (no per-panel CSS).

## Data model and API

Parameters on the node, following the Audio File naming (`mediaId`, `fileName`):

`slot1MediaId` to `slot9MediaId`, `slot1FileName` to `slot9FileName`,
`slot1Caption` to `slot9Caption`.

Validation lives in `crates/domain/src/lib.rs` next to the Audio File rules
(bounded string for the id, non-empty control-free file name up to 255 bytes,
caption up to a small limit with control characters rejected).

New methods, registered in `API_METHODS` (the website API page and Swagger
regenerate from this list):

- `soundboard.trigger { sessionId, nodeId, slot (1-9), idempotencyKey }`
- `soundboard.stop { sessionId, nodeId, slot? }` (no slot stops all)
- `soundboard.status` or a field on the existing node telemetry, so a client can
  read which slots are playing and each slot's caption without parsing the graph.

Open question: a trigger is a rapid, repeatable action. Decide how idempotency
keys behave for it (each press is a new action, so the key should be per press,
and a retried network call with the same key must not play twice).

## Engine

- New `NodeKind::Soundboard` and a prepared stage that owns nine decoded buffers
  and a small fixed set of voices. Memory is allocated when the session is
  prepared, never in the audio callback.
- Triggers reach the audio thread through a lock-free command queue. The audio
  callback only reads the queue and advances voices; it allocates nothing, logs
  nothing and waits for nothing.
- Per-file and total limits: decide a maximum length per file and a total decoded
  size for nine files, with a clear message at upload. Open question: the current
  Audio File limits need to be read and reused if they suit short sounds.
- Retriggering the same slot restarts the voice from frame 0 with the short
  fixed fade described above, to avoid a click.
- Add Soundboard to `crates/engine/tests/tool_combinations.rs` `TOOLS`
  (validated lesson: every new tool goes through the multi-path compiler with
  every flag pattern, bypass and connected-Mixer layout).

## Stream Deck

New action **Soundboard**, in the same pattern as `step` and `play`:

- Settings: the tool (chosen from a list of Soundboard tools in the session) and
  the slot (chosen from a list that shows each slot's caption, for example
  "3 - Airhorn", not just a number).
- The key face shows the caption (fitted, as the existing faces do), and a
  playing state (state 0 and 1 so users can set their own images).
- Press calls `soundboard.trigger`. On error the key shows the alert and a short
  reason (session stopped, empty slot).
- Make configuration easy:
  - If the session has one Soundboard, preselect it.
  - Show slot captions live, so renaming a caption changes the choices and the
    key face with no extra step.
  - Provide a "next unassigned slot" default when the user drags several actions
    onto the deck, so nine keys do not need nine manual slot choices.
  - Document the setup on `site/stream-deck.html` with a short real screenshot.
- Shipping a ready-made nine-key profile is the biggest ease-of-use win, but the
  backlog already lists profiles as separate work in
  [Stream Deck extensions](stream-deck-extensions.md). Treat it as a follow-up
  that the owner can choose to include.

## Key bindings (keyboard shortcuts)

Realistic, in two steps:

1. **First, API and CLI only.** An external tool (AutoHotkey, a macro keyboard,
   Elgato software) can call the local HTTP API or the CLI. Document one
   copy-paste example for each. This needs no new capability in the app.
2. **Then, optional in-app global shortcuts**, if the owner wants them. Windows
   `RegisterHotKey` can deliver a registered shortcut to the shell while other
   programs have focus. It registers only the combinations the user chose and
   does not read other keystrokes. Points to settle before building:
   - Bindings are per computer, so store them in shell settings, not in the saved
     session that can be shared or moved.
   - Detect and report a combination another program already owns.
   - Games: register nothing while the user chooses none, and test with the
     owner's game setup (Siege and its anti-cheat) before shipping, since the
     product rule is no anti-cheat injection and no hooks. `RegisterHotKey` is
     not a hook, but the claim needs a test, not an assumption.
   - Shortcuts only work while AudioRouter runs.

Recommendation: ship step 1 with the tool, decide step 2 after the owner has
used the Stream Deck path.

## Privacy and safety

- Triggering sends audio into the route that other people may hear. The buttons
  and API use the existing authorization; nothing new is exposed to the network.
- A **Stop all** control is always reachable, on the card and in Properties.
- The soundboard is not a microphone source. It does not touch the privacy-mute
  or failed-processor rules, but a muted or bypassed Soundboard must not leak
  sound; test it.
- Imported files stay local. No cloud, no telemetry.

## Order of work

1. Spec: add SND requirements, the tool catalog entry, API methods and the
   tool guide requirement; decision record in the active plan.
2. Domain: node kind, parameters, validation, API method specs.
3. Engine: prepared stage, voices, command queue, mixing and limiter, with unit
   tests and the tool-combination suite.
4. Control plane and API: `soundboard.trigger` and `soundboard.stop`, CLI, MCP,
   HTTP, generated website API catalog.
5. UI: card buttons, Properties editor, three-theme screenshots, layout-shift
   sampling, jsdom and Edge tests.
6. Stream Deck action, with unit tests and an attended test on the owner's deck.
7. Website: tool guide page and the Stream Deck page.
8. Optional: global shortcuts, then a ready-made profile.

## Validation matrix

| Check | Environment |
| --- | --- |
| Domain validation, API contract and generated catalogs | Portable |
| Voice mixing, retrigger, stop, headroom and limiter | Portable unit tests |
| Tool-combination suite with Soundboard | Windows CI |
| Audio continuity: `live_backend_service_keeps_a_routed_tone_continuous` with the new worker, plus a clip played through the route captured and compared with the source | Windows, VB-Cable or the virtual cable |
| Nine large files: memory and prepare time within the limits | Windows |
| Canvas and Properties in dark, light and high-contrast; no layout shift while playing | Edge/Playwright |
| Fresh-install run: the tool works with default grants and no developer variables | Windows, built exe |
| Stream Deck action: unit tests, then the real deck, including an error key | Attended, needs a Stream Deck |
| Firefox is not needed for the app; it is needed for any new website audio | Website |

## Risks

- A trigger arrives faster than the audio thread consumes the queue. Use a
  bounded queue, drop or coalesce the oldest repeat of the same slot, and count
  drops in telemetry.
- Nine files use more memory than expected. Bound it and say so at upload.
- A Stream Deck key shows a stale caption. Re-read captions on the plugin's
  existing change refresh.
- Global shortcuts conflict with other programs or games. Covered above.

## Rollback

The tool is a new node kind. Rolling back removes the kind from the library; a
saved session that contains it needs a migration decision (keep the node as an
unavailable placeholder, as other removed kinds are handled). Decide that
before release.

## Prerequisites and unknowns to settle first

- Read the Audio File upload limits and how media is stored, exported and
  deleted, including whether a copied or pasted node shares a `mediaId` (removing
  a slot must not delete a file another node still uses).
- Decide headroom versus limiter (measurement).
- Decide shortcut scope (step 2 above).
- Owner decision on whether a ready-made Stream Deck profile is in scope.
