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

**Update 2026-10-31:** property coverage is now 173 recorded, 25 pending and
53 exempted of 251 rows (the 26/48/177 split above predates the per-property
recordings). Advanced EQ is complete: bands 2–16 are covered by one "Other
bands" clip plus the band 1 field clips; see the
[Advanced EQ bands record](2026-10-31-parametric-eq-bands.md). The 25 pending
rows are Graphic EQ (9) and Soundboard buttons 2–9 (16).
