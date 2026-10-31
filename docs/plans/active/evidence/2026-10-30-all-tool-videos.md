# All-tool Properties recordings — 2026-10-30

## Delivered recording set

[Open the searchable 38-tool review gallery](tool-guide-preview/all-tools/index.html).
Every domain node kind plus visual Group has an eight-second portrait
Properties-only overview. The 27 processing/source tools also have an
eight-second synthetic-audio example: 65 MP4s in total. The generated 38 tool
pages include these recordings under **In the app**, beside the existing
conceptual material. Native controls, posters, `preload="none"`, reserved video
dimensions and manual playback preserve accessibility and avoid layout shifts.

Requirements: UI-20, UI-17 and DSP-03. These are browser recordings of the real
React Properties views against a fixture adapter, with production engine audio
and telemetry. They are not device, driver, remote peer or arbitrary plugin
qualification. Configuration overviews are silent. Output Switch monitors A;
its audio becomes silent when B is selected. Audio File's short example shows
the Loop control but ends before the source repeats.

## Repairs and evidence

- Fixed Soundboard inventory numbering from 0–8 to the domain's 1–9.
- Supplied Audio File/Soundboard synthetic media, a 32-tap moving-average FIR
  response, learned noise profiles, and the Duck key source. Test Signal now
  receives Play and a source duration covering the recording.
- Replayed production EQ spectrum data in the top response graph. Changed
  controls are revealed briefly; playback returns to the top visual.
- Switches render through `RealtimeMixerFanout::replace_paths`, preserving
  the production transition state. A focused raw-output check confirms both
  input tones during the crossfade and the short Output A fade before silence.
- Reduced the capture viewport to 480×854 for readable labels; deliverables
  are H.264 720×1280 with AAC for audible examples and fast-start metadata.
- A guide recording and the conceptual interactive preview pause one another.
- Guide regeneration also runs `site/generate-social.mjs`; review caught that
  omitting this existing pipeline step removed canonical/social tags. The
  final generated pages preserve them.
- CI documentation validation caught stale heading anchors from the earlier
  spike. Fixed the three active-plan links and the UI-20 requirements link;
  `node tools/quality/verify.mjs --checks docs` now passes. The check's other
  annotation is GitHub's hosted `ubuntu-latest` image-migration notice, not a
  source diagnostic; it stays as runner-environment information. CodeQL's
  neutral check has no annotations.
- Development capture setup emitted React Flow sizing/identity warnings while
  the canvas was moved offscreen. Earlier empty-session Group captures failed;
  the final Group capture uses a populated fixture and succeeds. These are
  capture-layout experiments, not changes to product canvas styling. Do not
  interpret the recording checks as a zero-console-warning app qualification.
- Review follow-up (2026-10-31): the capture scripts now stop with an error
  instead of recording an empty Properties panel or writing a `NaN`/missing
  example value. A tool kind missing from `tool-guide-catalog.json`, an
  unknown or multi-kind `GUIDE_KIND` for a configuration capture, a
  `GUIDE_PROPERTY` filter without a render, and an example property with no
  safe changed value all fail. The node type version comes from the catalog.
  The shared checks are in `tools/site/tool-guide-guards.mjs`, covered by
  `node --test tools/site/tool-guide-guards.test.mjs` (6 pass; not yet run by CI).

Windows results:

| Check | Result |
| --- | --- |
| `node tools/site/check-tool-guide-videos.mjs` | Pass: 38 tools, 65 videos, 18,297,550 MP4 bytes; each under 1 MB, eight seconds, required codecs/dimensions |
| `node tools/site/check-tool-guide-video-firefox.mjs` | Pass: 65/65 installed Firefox real-pointer Play checks; nonzero analyser output for all 27 audio examples |
| `node tools/site/check-tool-guide-switch-fades.mjs` | Pass: Input A/B coexist during transition, then B remains; Output A has a short fade then silence |
| `node tools/site/review-tool-guide-layout.mjs` | Pass: 390 px Gate/EQ/Soundboard/Input pages, no horizontal overflow, manual controls, playback coordination |
| `node site/check-guides.mjs` | Pass: 38 pages, 144 property anchors, 1045 local links/assets, catalog/allowlist checks |
| Engine example Clippy with `-D warnings`; both Rust formatting checks; `git diff --check` | Pass |

Detailed [media hashes/formats](2026-10-30-tool-guide-media.json) and
[Firefox measurements](2026-10-30-tool-guide-firefox.json) accompany this record.
Phone screenshots are in [the layout evidence directory](tool-guide-layout/).

## Remaining plan scope

The all-tool recording set is present. Exhaustive property coverage from the
original plan remains incomplete: 26 property rows are recorded, 48
configuration rows have explicit exemptions and overview references, and 177
rows still need demonstrations. Do not mark the full plan complete or claim
every setting is demonstrated. Repeated EQ bands and Soundboard slots account
for many rows; any grouping must be explicit and reviewed. The independent
property coverage checker continues to fail until those rows are resolved.

Linear processors are prepared again at the parameter boundary; this can reset
processor history. The switch examples now preserve the production fade state,
but the broader continuous-automation qualification remains open. This PR stays
a draft. No site deployment, hardware qualification or release was performed.

Rollback: revert the site recording metadata/generated pages, video assets,
capture scripts and engine example changes. Existing interactive guides remain
the previous implementation. Next task: finish the outstanding property groups
and qualify continuous parameter transitions before completing the full plan.

**Update 2026-10-31:** with Advanced EQ (see the
[Advanced EQ bands record](2026-10-31-parametric-eq-bands.md)) and Graphic EQ (below)
merged, property coverage is 182 recorded, 16 pending and 53 exempted of 251
rows (the 26/48/177 split above predates the per-property recordings). Advanced
EQ bands 2–16 are covered by one "Other bands" clip plus the band 1 field clips.
The 16 pending rows are Soundboard buttons 2–9. The coverage figure inside the
Graphic EQ section below was measured on its own branch, before this merge.

## Graphic EQ bands — 2026-10-31

Requirements: UI-20, UI-17 and DSP-03. The nine Graphic EQ bands that had no
clip of their own now each have an eight-second portrait Properties recording:
31.5 Hz, 63 Hz, 125 Hz, 250 Hz, 500 Hz, 2 kHz, 4 kHz, 8 kHz and 16 kHz. The
1 kHz band keeps the existing `graphicEq-demo.mp4`. Each clip lifts one band
from 0 dB to +12 dB about three seconds in, with synthetic pink noise through
the production Graphic EQ. The noise has energy at every band, so no separate
fixture was needed. The clips show the real Properties panel only, briefly
reveal that band's fader, then return to the response graph and live readings.

Check on the rendered audio (Goertzel power around each band centre, output
against input, windowed 4–7 s): +10.6 to +11.5 dB at the changed band, 0.0 dB
before the change, and an unchanged 500 Hz comparison band for every band except
250 Hz (2.5 dB, one octave away, where the neighbouring band's skirt overlaps) and
500 Hz itself (so 500 Hz has no off-band comparison, and 250 Hz is the only
band whose neighbour was measured to move). Frame review of all nine clips at
the six-second mark shows the response graph peaking at the right frequency
each time. The Level reading in those single frames, against about −30.7 dB
before the change, has moved by under 1.5 dB for 31.5 Hz, 63 Hz and 2 kHz,
about 3–4 dB for 125–500 Hz, and 4–6 dB for 4–16 kHz. Those are snapshots of a
live meter, not measurements; the low-frequency lifts change the overall level
least.

Tooling changes:

- `render-tool-guide-properties.mjs` skips repeated bands only for Advanced EQ
  (`parametricEq`); Graphic EQ bands are renderable. Soundboard buttons 2–9 are
  still skipped. Each Graphic EQ band uses +12 dB.
- `build-tool-guide-review.mjs` accepts `GUIDE_KIND=<kind>[,<kind>]`: other tools
  keep their `recordings.json` entries and gallery cards without re-encoding or
  copying their media. `GUIDE_REUSE_ENCODED=1` keeps already-encoded files and
  encodes only new ones. The gallery `index.html` regenerated byte-identical, and
  `git status` showed no other tool's media changed.
- `check-tool-guide-video-firefox.mjs` now plays every clip in `recordings.json`
  for the selected tools, including the per-property clips, which it never played
  before. Audible clips must still produce nonzero analyser output. It no longer
  needs an earlier results file when `GUIDE_KIND` is set.

Windows results: Firefox real-click playback passes for all 11 Graphic EQ clips
([results](2026-10-31-graphic-eq-bands-firefox.json); the nine new and the demo
are audible, the overview is silent by design). `check-tool-guide-videos` (38
tools, 148 videos, each under 1 MB), `check-tool-guide-catalog`,
`site/check-guides` (1,128 local links/assets), `generate-social --check
--check-changed`, `tools/docs/validate.mjs` and `node --test
tools/site/tool-guide-guards.test.mjs` (6) pass. A phone-width (390 px) check of
the Graphic EQ page passes: no horizontal overflow, 11 manual-play clips in
frequency order, none autoplaying. `review-tool-guide-layout` passes; it does not
list the Graphic EQ page, so the same check was run for it separately.

Coverage now: 107 recorded, 53 exempted, 91 pending (251 rows). The pending rows
are Advanced EQ points 2–16 and Soundboard buttons 2–9;
`check-tool-guide-property-coverage` still fails on them. Not run: Edge visual
check of the page in light and high-contrast themes, a site deployment, and a
full (all-tool) run of the extended Firefox checker: it now plays every
per-property clip, and clips whose sound starts after the 1.5 s sample may fail
its nonzero-peak test until that is tuned. `node tools/quality/verify.mjs
--checks site,docs`, the entry point CI uses, passes. The audio comes from the
existing renderer executable built at `7a2b1f84`; no Rust code changed here, and
the engine changes on main since then (generator Mixer metering, source
combination tests) do not touch the Graphic EQ processor. Rollback: revert this commit and rerun the generators.
