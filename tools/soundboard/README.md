# Soundboard sound library tools

Everything needed to rebuild or extend the built-in sound library
(`resources/sound-library/`). Rules and background: the
[sound library plan](../../docs/plans/future/soundboard-sound-library.md) and the
[sourcing record](../../docs/plans/active/evidence/2026-10-29-soundboard-sourcing.md).

## What is where

| Item | Location |
| --- | --- |
| Prepared library (FLAC, manifest, notices) | `resources/sound-library/` (in the repository) |
| Picks that produced it | `tools/soundboard/provenance/picks-1.txt`, `picks-2.txt` |
| Freesound license evidence (one record per sound page) | `tools/soundboard/provenance/freesound-licenses.json` |
| Raw downloaded sources (Kenney packs, Freesound files, generated originals, excluded files) | **Not in the repository.** Backed up on the owner's PC at `C:\audiorouter-assets\sound-sourcing` (also `target/sourcing`, which `cargo clean` deletes) |

The raw sources are CC0 and can be downloaded again from the URLs in
`library.json`, but the backup saves that work.

## Tools

- `generate-original.mjs <folder>`: makes the project's own synthesized sounds
  (deterministic, seeded).
- `check-freesound-license.mjs <folder> <out.json>`: reads each downloaded
  Freesound file's license from its sound page (one request every 1.2 s).
- `make-audition-page.mjs <folder> [page.html]`: a local page to listen to
  candidates and tick the ones to keep. Folders whose name starts with `_` are skipped.
- `prepare-library.mjs <sourcing> <picks[,picks]> <licenses[,licenses]> <out>`:
  converts picks to mono 44.1 kHz FLAC, trims silence, matches level, fades
  the end, writes `library.json` (with source, author, URL, license and SHA-256 per
  sound) and `NOTICES.md`. A picks line may end with `| excerpt=SECONDS`.

Needs Node 22 and `ffmpeg`/`ffprobe` on the PATH.

## Adding sounds later

1. Download CC0 sounds into `target/sourcing/<folder>` (Freesound: filter "Creative
   Commons 0", 5 seconds or less; Freesound pack downloads include a readme with
   each license).
2. `node tools/soundboard/check-freesound-license.mjs <staging> <out.json>` and keep
   only CC0 results. Add the output to `provenance/` and extend the licenses list.
3. Listen with `make-audition-page.mjs`, write a new `picks-N.txt`.
4. Run `prepare-library.mjs` with all picks files, review, copy the result into
   `resources/sound-library/`.

**Stable ids.** Sound ids (`lib:...`) are written into saved sessions and must never
change or be reused once released. `prepare-library.mjs` numbers titles in sorted
order within a group, so adding picks can renumber *new* ids only if they sort ahead
of existing ones; before a release, freeze the manifest and add new sounds under new
names. The CI check described in the plan compares against the last released manifest.

Titles are still generated from file names ("Fart 03"); better names can be set in
a later change before the first release, while ids are not yet frozen.
