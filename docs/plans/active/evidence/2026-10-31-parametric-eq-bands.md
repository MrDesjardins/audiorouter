# Advanced EQ bands 2–16 — 2026-10-31

Requirements: UI-20 (guide recordings), UI-17 (no layout shift), DSP-03.
Branch `docs/guide-parametric-eq-bands`. Follows the
[all-tool recordings](2026-10-30-all-tool-videos.md) and the
[capture decision](2026-10-29-tool-guide-capture-decision.md).

## Decision: group bands 2–16 under one representative

The manifest listed 75 pending rows for bands 2–16 (five fields × 15 bands).
Recording each would add 75 near-identical videos. Before grouping, the bands
were compared in the source:

- **Schema.** `crates/control/src/catalog.rs` generates the same five
  parameters for every index 0–15: `Enabled` (default off), `Type` (default
  Peaking, the same eight filters), `FrequencyHz` (20–20,000, default 1,000),
  `Q` (0.1–20, default 1), `GainDb` (−24–24, default 0).
- **Validation.** `valid_parametric_band_parameter` in
  `crates/domain/src/lib.rs` applies one rule set to every index below 16.
- **Engine.** `crates/engine/src/lib.rs` builds each band from the same
  `band{index}…` keys with the same defaults.
- **Editor.** `ui/src/AdvancedEqEditor.tsx` has one set of controls (Select
  point, Filter, Frequency, Gain, Q) that edits whichever point is selected;
  there is no per-band control.

The only per-index difference is that band 1 (index 0) falls back to the old
unnumbered `frequencyHz`, `q` and `gainDb` values in sessions saved before the
bands existed. That applies to old saved sessions, not to anything a reader
does in the current editor, and it does not change what a new band does. So
bands 2–16 are documented as "the same as band 1", and the page says so.

## What was added

- **One new audible clip, "Other bands"**
  (`site/assets/tool-guides/parametricEq--other-bands.mp4`, 488,716 bytes,
  H.264 720×1280 + AAC, 8.0 s, plus its poster). It uses the real Properties
  panel against the fixture adapter and the production engine
  (`tool_guide_property_render`) on the pink-noise fixture. Band 1 stays at
  +9 dB at 1,000 Hz. At 3.2 s the recording scrolls to the graph, double-clicks
  it to add band 2, enters 5,000 Hz and −9 dB, reveals the Point 2 controls,
  then returns to the live readings at the top.
- **Audio matches the panel.** A Goertzel analysis of the rendered output
  against its input (left channel) gives 1,000 Hz +8.9 dB before and +8.5 dB
  after the change, and 5,000 Hz +0.4 dB before and −8.5 dB after. The
  capture fails unless the panel ends on Point 2, Peaking, 5000 Hz, −9 dB,
  Q 1 (the values the audio was rendered with), the double-click lands within
  5 % of the target frequency and 1 dB of the gain, and the panel settles
  within 0.4 s of the audible change (measured 0.098 s; event at 3.199 s).
- **Generic family mechanism** (reusable for Soundboard buttons 2–9):
  `tools/site/tool-guide-families.json` declares a family (member key pattern,
  representative index, shared clip); `tool-guide-families.mjs` holds the
  matching and the coverage rules; `render-tool-guide-families.mjs` renders the
  shared clip; `tool-guide-family-actions.mjs` holds the recorder steps per
  tool; `build-tool-guide-property-manifest.mjs` marks each member row with its
  `representativeKey`; `build-tool-guide-review.mjs` publishes the clip and
  lists `[shared clip, representative's clips]` on every member row.
- **Coverage check.** `check-tool-guide-property-coverage.mjs` now fails when
  a member row is `recorded` without the shared clip, without every clip of its
  representative, while its representative is not `recorded`, or when the
  shared clip's files are missing. Nine unit tests are in
  `tool-guide-families.test.mjs`. Hiding the new MP4 made the check report all
  75 rows.
- **Page text.** The Advanced EQ page's Settings note now says the recordings
  change band 1 field by field and that bands 2–16 work the same way; the clip
  description repeats it in plain words.
- **Firefox check now covers per-property clips.** It only probed each tool's
  `-overview` and `-demo` files; it now also plays every other clip in
  `recordings.json`, and requires sound from the audible ones.
- `build-tool-guide-review.mjs` takes `GUIDE_ENCODE_MISSING=1` to encode only
  clips that are not published yet, so adding a clip does not re-encode every
  other video.

- **Render report no longer loses rows.** `render-tool-guide-properties.mjs`
  started from an empty report (or, with `GUIDE_KIND`, kept only other tools'
  rows) and wrote only the properties it rendered, so any run would have
  removed the recorded band 1 rows and the family clip from
  `render-report.json`, and with them their videos from the next build. It now
  keeps every row it does not render again and replaces a row by id. Checked
  against a backup of the report: `GUIDE_KIND=parametricEq` (no pending rows)
  left the file byte-identical; `GUIDE_KIND=graphicEq` rendered one property,
  added its row, and kept all five Parametric EQ rows including
  `parametricEq--other-bands`. The report was restored afterwards.

## Renderer provenance

The clip's audio came from `tool_guide_property_render.exe` built in the
earlier tool-guide worktree on 2026-10-30 15:50 (PDT); it was copied, not
rebuilt, because no cargo command may run while the owner's app is live. Its
source file and `crates/engine` changed in later commits (voice-preset and
fixture additions, generator metering, connected Mixers). `crates/dsp` has no
change since the first recordings commit (`b768ec23`, Cargo.toml version only),
the Parametric EQ stage in `crates/engine/src/lib.rs` is unchanged, and the
`pink-noise` fixture is unchanged. The output was also measured directly (see
above), so the clip shows what the engine does, but it was not produced by an
exe built from this branch's source. Rebuild and re-render before a release
build if that distinction matters.

## Counts

Property rows: 251. Before: 98 recorded, 100 pending, 53 exempted. After:
**173 recorded, 25 pending, 53 exempted**. Parametric EQ is 80 of 80 recorded
(5 recorded field by field for band 1, 75 through the family clip). The 25
pending rows are Graphic EQ (9) and Soundboard buttons 2–9 (16). Videos in
`recordings.json`: 140 (48,820,420 bytes; each under 1 MB).

## Checks

Windows 11, Node 22, Edge for the capture and the layout review, installed
Firefox for playback.

| Check | Result |
| --- | --- |
| `node --test tools/site/tool-guide-guards.test.mjs tools/site/tool-guide-families.test.mjs` | Pass: 15 tests |
| `node tools/site/check-tool-guide-videos.mjs` | Pass: 38 tools, 140 videos |
| `node tools/site/check-tool-guide-catalog.mjs` | Pass |
| `node tools/site/check-tool-guide-property-coverage.mjs` | Fails only on the 25 pending Graphic EQ and Soundboard rows; no family issue |
| `node site/check-guides.mjs` | Pass: 38 pages, 144 property anchors, 1120 local links/assets |
| `node site/generate-social.mjs --check --check-changed` | Pass |
| `node tools/docs/validate.mjs` | Pass |
| `GUIDE_KIND=parametricEq node tools/site/check-tool-guide-video-firefox.mjs` | Pass: 7 of 7 EQ clips play after a real click; every audible clip, including Other bands (peak 0.058), produces sound ([results](2026-10-31-parametric-eq-bands-firefox.json), [media](2026-10-31-parametric-eq-bands-media.json)) |
| `node tools/site/review-tool-guide-layout.mjs` | Pass: 390 px Gate, Advanced EQ, Soundboard, Physical Input pages; shared playback |

CI's `policy`, `docs` and `site` checks cover changes under `site/**`,
`tools/site/**` and `docs/**` (`tools/quality/checks.mjs`). Run together with
`node tools/quality/verify.mjs --checks policy,docs,site`: all three pass.

Not run: Rust checks (no Rust changed; the renderer example was reused as
built and no cargo command ran, because the owner's app was running), the full
UI test suite, a re-recording of the existing band 1 clips, and a Firefox pass
over the other 37 tools' clips. The extended Firefox check now probes every
published clip, about 130 more than before; it plays through the default
output, so run it for those tools when the owner's app is not live, and expect
it to be the first run of the check on those files. Neither
`tool-guide-families.test.mjs` nor `tool-guide-guards.test.mjs` runs in CI.
Not covered: a recording of every band's own fields (by design), live devices,
hardware.

## Notes for the next run

- Do not run `render-tool-guide-properties.mjs` with `GUIDE_KIND=parametricEq`
  to add or refresh the family clip: it keeps only other tools' rows and
  renders only pending properties, so the recorded band 1 rows would drop out
  of `render-report.json` and their clips from the next build. Use
  `GUIDE_KIND=parametricEq node tools/site/render-tool-guide-families.mjs`.
- Record it with `GUIDE_PROPERTIES=1 GUIDE_KIND=parametricEq
  GUIDE_PROPERTY=otherBands node tools/site/record-tool-guide-overviews.mjs`,
  then run the manifest builder and
  `GUIDE_ENCODE_MISSING=1 node tools/site/build-tool-guide-review.mjs`.
- The generator copies `target/` overviews into `site/assets`. A `target/`
  older than the committed Physical Input and Application Capture overviews
  rewrites four files of each; restore them with `git checkout`.
- Soundboard buttons 2–9 can reuse the mechanism: add a `soundboard` entry to
  `tool-guide-families.json` (`memberPattern` for `slot<n>…`,
  `representativeIndex` 1, a clip) and a `soundboard` entry to
  `tool-guide-family-actions.mjs`. This branch leaves the existing
  `slot<n>Mode` exemptions alone, but the review build's family pass sets the
  status of every member row of a family (`recorded` when the representative
  and the shared clip exist, otherwise `pending`). Adding a Soundboard family
  therefore replaces those exemptions; decide that on purpose.

Rollback: revert the commit and rerun the manifest builder, the review build
(`GUIDE_METADATA_ONLY=1`) and `site/generate-guides.mjs` +
`site/generate-social.mjs`. Next task: Graphic EQ bands and Soundboard
buttons 2–9 (parallel threads), then qualify continuous parameter
transitions.

## Merge with the Graphic EQ branch

`origin/docs/guide-graphic-eq-bands` (`b973a77c`, `c3433677`: nine Graphic EQ
band clips) is merged into this branch. Conflicts were in three scripts and the
2026-10-30 record, and were resolved by keeping both behaviours:

- `check-tool-guide-video-firefox.mjs`: took the Graphic EQ version, which
  plays every `recordings.json` clip and no longer needs an earlier results
  file. It replaces the narrower change made here; same result (audible clips
  must produce sound).
- `build-tool-guide-review.mjs`: kept `GUIDE_KIND=<kind>[,…]` and the poster
  helper from Graphic EQ, and the family clip handling from here. One option
  selects "encode only new clips": `GUIDE_REUSE_ENCODED=1`, with
  `GUIDE_ENCODE_MISSING=1` kept as an alias, so callers and docs using either
  name work.
- `render-tool-guide-properties.mjs`: the Graphic EQ filter (skip repeated
  bands only for Advanced EQ) and the per-band +12 dB values merged without
  conflict with this branch's keep-every-row change.
- The generated manifest, `recordings.json`, `graphic-eq.html` and the review
  gallery were not hand-merged: this branch's version was taken, the nine
  Graphic EQ render and recording artifacts and report rows were copied into
  `target/`, and the manifest builder, the review build in reuse mode and
  `site/generate-guides.mjs` and `generate-social.mjs` were rerun. The result
  matches the Graphic EQ branch exactly for Graphic EQ (identical
  `graphic-eq.html` and recordings), and differs from it only in the 75
  Advanced EQ rows. The four Physical Input and Application Capture files the
  generator rewrote from the older `target/` were restored.

Merged counts: 251 rows, **182 recorded, 16 pending (Soundboard buttons 2–9),
53 exempted**; 38 tools, 149 videos (52,300,737 bytes). Passing after the
merge: guards and families tests (15), `check-tool-guide-videos`,
`check-tool-guide-catalog`, `site/check-guides` (1,129 local links/assets),
`generate-social --check --check-changed`, and Firefox real-click playback of
all 18 Graphic EQ and Advanced EQ clips (every audible clip produces sound;
[Advanced EQ results](2026-10-31-parametric-eq-bands-firefox.json), re-run with
the merged checker). `check-tool-guide-property-coverage` fails only on the 16
Soundboard rows. Not rerun after the merge: the 390 px layout review and the
Edge themes; no cargo command ran.
