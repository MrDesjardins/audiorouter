# Active plan: Soundboard tool and sound library (2026-10-29)

Owner request, 2026-10-29: build a Soundboard tool (nine sounds, canvas buttons,
Stream Deck trigger through the API) with a library of about a hundred free short
sounds, a searchable picker, and the user's own sounds. "I want you to start."

- Specification and behavior: [Soundboard tool](../future/soundboard-tool.md)
  and [Soundboard sound library](../future/soundboard-sound-library.md). Those
  documents stay the detailed source; this plan tracks execution.
- Authorized: the tool, its API, UI, Stream Deck action, the bundled sound
  library and picker, and website guide pages for the tool.
- Not authorized: in-app global keyboard shortcuts (step 2 of the key-binding
  section), a ready-made Stream Deck profile, a downloadable sound pack, streaming
  of long files, attribution-licensed sounds. Each needs a separate decision.
- No pull request is opened without the owner's approval (AGENTS.md).

## Owner decisions (2026-10-29)

1. Built-in sounds are CC0, public domain or original sounds made for AudioRouter
   only. No attribution licenses in version one.
2. The library is bundled in the installer. No download and no network access.
3. User sounds: 120 seconds per slot and a total decoded budget for the board,
   shown at upload. No streaming of longer files in version one.

## Requirement IDs

SND-01 to SND-14 (proposed in the Soundboard tool plan; added to `docs/spec/`
in task 1). Touches UI-17, UI-20, UI-11, API/CLI/MCP parity and the audio
callback rules.

## Prerequisites and constraints

- Compile-heavy work (Rust builds, test suites, Playwright) waits while the
  owner's virtual cable VM session is running, because host load causes false
  audio-loss failures in the VM. Documentation, scripts and sourcing can proceed.
- A Freesound API key (free; see "Getting Freesound access" in the library plan) is needed only if Freesound is used for sourcing;
  Kenney, OpenGameArt, Wikimedia and generated sounds need nothing from the owner.
- Windows evidence is required for audio continuity, installer contents and the
  fresh-install run. Stream Deck hardware is required for the attended key test.

## Ordered tasks

1. Spec: add SND requirements, the tool catalog entry, API methods, the tool guide
   requirement and a decision record. Update the future plans to point here.
2. Sourcing and format spike (no Rust build): build the target list, collect and
   record candidates, write the generator for original sounds, compare WAV, MP3
   and FLAC for size and clean starts, choose normalization numbers by listening.
3. Domain: `NodeKind::Soundboard`, slot parameters, validation, `API_METHODS`.
4. Engine: prepared stage, voices, lock-free command queue, mixing and limiter;
   add to the tool-combination suite.
5. Control plane, API, CLI, MCP, HTTP; regenerate the website API catalog.
6. Library: manifest, checks in CI, installer resource folder, notices file,
   `soundLibrary.list` and preview read.
7. UI: card buttons, Properties editor, picker; three-theme screenshots and
   layout-shift sampling.
8. Stream Deck action and its tests.
9. Website: tool guide page and the Stream Deck page.
10. Qualification: audio continuity, fresh-install run, installer check, attended
    Stream Deck test, owner listening review.

## Validation matrix

See the validation tables in the two linked plans. Record each result here with
command, environment, result and evidence path as it is run.

## Risks

See the linked plans. Additional execution risks: a long implementation spanning
engine, UI, API and plugin must be kept in reviewable commits (one task each), and
the library licensing record must be complete before any sound is committed.

## Rollback

Each task is a separate commit on `feature/soundboard`. Reverting the branch
removes the tool, the API methods, the library and the pages. A saved session
containing a Soundboard needs the placeholder decision from the Soundboard plan
before release.

## Progress

- 2026-10-29: owner decisions recorded; plan promoted from `future/`. No code yet.
- 2026-10-29: library target changed to up to about 100 (about 80 is fine) with many
  variations of favorites (fart 10 to 12, claps, ta-da); Bodily humor category and
  content rules added; Freesound access steps written.

- 2026-10-29 sourcing, first pass (no key, host-light):
  - Wikimedia Commons API (CC0/public-domain, 6 s or less): mostly spoken-word
    recordings; usable hits are a few doorbells and two roosters. Secondary source.
  - OpenGameArt (CC0 sound-effect filter): almost nothing for fart, clap, applause
    or air horn; some fanfare and laugh packs to inspect. Secondary source.
  - kenney.nl returns 403 to scripted downloads; the packs need a manual download
    by the owner (not circumvented). Freesound needs the owner's API key.
  - `tools/soundboard/generate-original.mjs`: 33 original sounds (9 fart variations,
    3 ta-da, 3 air horns, rimshot, sad horn, buzzer, doorbell, coin, etc.), seeded and
    byte-identical across runs, all at most 3.3 s. Unreviewed by ear. Level check:
    peak is matched at -3 dBFS but average level varies from about -4 to -19 dB, so
    the library needs RMS or loudness matching as well as a peak ceiling.
- 2026-10-29 gap review added to the Soundboard plan: v1 additions per-slot level,
  play modes (loop, toggle, hold), random-variant slots, progress on the button,
  drag and drop, recently used, trigger cooldown (SND-15 to SND-20; owner to confirm).

- 2026-10-29 sourcing closed for now (owner: add more later). The library is
  committed: `resources/sound-library/` holds 105 sounds as mono 44.1 kHz FLAC with
  `library.json` (source, author, URL, license, SHA-256 each) and `NOTICES.md` (7.8 MB;
  hashes verified). Format spike result for 105 sounds: WAV 14.4 MB, FLAC 7.7 MB,
  MP3 2.7 MB with a 25 ms encoder start delay. Decision: FLAC, which needs the
  decoder's `flac` feature (to enable and verify in task 4). The picks, license
  evidence and tools are in `tools/soundboard/` (see its README); the raw sources are
  backed up at `C:\audiorouter-assets\sound-sourcing`.
- 2026-10-29 build start: Rust builds wait for the owner's VM session to finish
  (VM `AR-DriverTest` was still running); code is written first.

- 2026-10-28 code written, NOT yet compiled or tested (the owner's VM pool run was
  still in progress, so no cargo, tsc, vitest or Playwright ran on the host):
  - Domain: `NodeKind::Soundboard` (37 kinds), slot parameters `slotNMediaId`,
    `FileName`, `Caption`, `GainDb` (-24 to +6), `Mode` (once, loop, toggle, hold),
    `Set` (a category id or up to 8 sound refs), bounded validation and a unit test;
    API methods `soundboard.trigger`, `soundboard.stop`, `soundboard.status`,
    `soundLibrary.list`, `soundLibrary.read`.
  - Engine: `crates/engine/src/soundboard.rs` (16 voices, bounded command queue,
    5 ms stop fade, 50 ms trigger gap, random sets that never repeat, soft limiter) with
    unit tests; symphonia `flac` feature added (Cargo.lock will change on first build).
  - Control: `soundboard.rs` (API), `sound_library.rs` (manifest, SHA-256 check,
    base64 read, decoded-budget loading of `lib:` and user sounds).
  - UI: canvas card, Properties editor, picker, live status store, API client,
    Stream Deck Sound action and inspector, website guide, API catalog, Stream Deck page,
    spec UI-25.
  - Known gaps: a library category in `slotNSet` is accepted but not yet expanded (lists
    work); drag and drop (SND-19) is not built; the installer does not yet copy
    `resources/sound-library` next to the program; `tool_combinations.rs` and an
    engine route test for Soundboard are not written; no jsdom tests for the new UI.

## Next action

When the owner's VM pool run has finished (watch for `stage-a-pool-*.zip` in the
share): `cargo fmt`, `cargo check`/Clippy (workspace and `src-tauri`), engine and
control tests, then `npm` typecheck, lint, format and vitest for `ui/` and
`tools/streamdeck`. Fix what the first compile finds, then the known gaps above, then
the three-theme screenshots and the continuity, fresh-install and attended Stream Deck
qualification (task 10).
