# All-tool guide visuals and synthetic audio

Date: 2026-10-15 (owner request)

## Objective and requirements

Review every static guide against the actual AudioRouter tool controls and
behavior. Correct mismatched diagrams, especially FIR, Graphic EQ, Advanced
EQ, and Network Send/Receive. Provide an optional synthetic audio preview on
all 35 tool pages so visitors can hear simple effects while adjusting the
guide controls.

Requirements: UI-20 (individual visual tool guides), UI-11 (accessible
controls), UI-12 (renderer boundary), UI-17 (stable layout). Read
`docs/README.md`, the active plan, `docs/spec/09-interface.md`,
`docs/spec/07-processing.md`, the guide property data, and the actual UI node
editors. Existing application/session changes were out of scope and preserved.

## Review and implementation

- Audited all 35 guide kinds across processing and routing. Kept the accurate
  channel, file, buffer, route, group and recording concepts; separated
  previously generic clean-up diagrams for hum harmonics, short impulse repair,
  learned noise, speech-focused reduction and per-frequency spectral gating.
- FIR now illustrates a synthetic impulse response, convolution tail, equally
  delayed dry path, wet blend and output gain. The real tool still uses its
  selected WAV/MP3 file, up to two seconds, normalized to unit energy. The
  guide never loads an uploaded file.
- Graphic EQ shows all ten fixed centers and faders (31.5 Hz through 16 kHz)
  with one combined response. Advanced EQ shows four example bands in a
  combined response, selection of all eight supported shapes, and per-band
  frequency, gain and Q. All pass shows flat magnitude and a separate
  schematic phase curve. Four bands are a compact guide example; the product
  supports up to sixteen.
- Network Send and Receive show both PCs, each AudioRouter session, pairing
  wait, UDP packet movement, and the receive buffer. Receive adds its configured
  buffer time in the diagram. The views do not imply a relay, encrypted audio,
  or real delivery.
- Each guide now has Play/Stop synthetic audio. Native Web Audio creates a
  short tone phrase only after a click, shapes it with a simplified local
  effect graph, applies a conservative master level and output compressor, and
  stops after eight seconds, on Stop, or on page exit. Playback never reads a
  microphone, file, or AudioRouter session and never records. Unsupported or
  non-effect tools get a synthetic pass-through. Network Send/Receive use an
  illustrative 60/40 ms delay. Plugin audio is explicitly generic and does not
  load an installed plugin.
- Chose native Web Audio over Tone.js to keep the static offline guides
  dependency-free; Tone.js also requires its audio start to follow a user
  gesture ([official documentation](https://tonejs.github.io/)).
- The EQ selectors and sliders use the site's themed field style. Motion
  remains seekable and reduced-motion compatible; audio playback is separate
  from animation state.

Changed areas: `site/generate-guides.mjs`, `site/assets/guide-animation.js`,
`site/assets/guide-audio.js`, `site/assets/guides.js`, `site/assets/guides.css`,
`site/guide-content/processing.json`, `site/guide-content/routing.json`,
generated `site/tools/*.html`, guide manifest/catalog, and
`ui/e2e/site-guides.pw.ts`.

## Validation evidence

- Windows host, Node 22 and installed Microsoft Edge. `node
  site/generate-guides.mjs`: generated 35 guides with 101 configuration
  entries. `node site/check-guides.mjs`: PASS; 35 tool pages, 101 property
  anchors, 941 local links/assets, 35 catalog links, and parity with UI/native
  destination allowlists.
- `ui/e2e/site-guides.pw.ts` under Edge with the temporary explicit
  `executablePath` config and one worker: 5/5 passed (2026-10-15). Tests open
  every page offline, confirm no autoplay, check FIR response controls and
  Play/Stop, ten Graphic EQ bands, Advanced EQ shapes/all-pass, Network Receive
  pairing/buffer delay, Network Send pairing/packet motion, keyboard and
  reduced motion, mobile overflow, and guide search. Representative FIR,
  Graphic EQ, Advanced EQ and Network Receive screenshots were reviewed from
  the test output directory under `%TEMP%` (not committed).
- UI Prettier format check: PASS. UI TypeScript `tsc --noEmit`: PASS. Full UI
  ESLint: zero errors, 25 existing exhaustive-deps warnings. Earlier browser
  attempts exposed two stale expected labels and a stale selector; those tests
  were updated to the real default ratio and current Advanced EQ selectors,
  then the final run passed.
- Guide previews are explanatory browser audio only. No app DSP, Windows audio
  endpoints, Network sockets, plugin, driver, latency or sound-quality gate was
  exercised. No site publication or app rebuild was performed.

## Risks and rollback

Synthetic filters approximate selected product behavior and cannot reproduce
arbitrary impulse files, the production denoisers, time stretch, plugin code or
network transport. Labels identify these limits; no session data is changed.
Rollback by reverting the guide source/data, generated pages, and focused
browser test listed above. There is no backend or persistence migration.

## Handoff

The updated local site can be previewed from the repository's `site/` directory.
Owner review is next; publishing the site remains a separate action.
