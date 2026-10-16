# Synchronized tool guide playback — 2026-10-15

## Objective and scope

Unify synthetic audio and guide animation on one timeline; tailor Recorder,
Mixer and Meter examples to AudioRouter; explain the purpose and practical
adjustment of all ten Graphic EQ bands.

Requirements: UI-20, UI-17, DSP-01/02 and REC-01/02/07. This is a static-site
documentation task. It does not change the desktop app, backend, recording
engine, live audio routes or published website.

## Implementation

- Replaced separate animation and audio cards with one Play/Pause, Replay and
  seek transport for audio-relevant tools. AudioContext startup completes
  before the animation starts; seeking, pausing, replaying and completion
  share the visual playhead. Transport labels and status reserve stable widths.
- Added an explicit audio-relevance allowlist. Device and application capture,
  physical output, arbitrary VST and Visual Group pages remain animation-only.
  Audio File uses a generated example; no guide reads a user's file or route.
- Built Recorder's distinct Record → Stop → Play recording flow. A generated
  synthetic sample passes through the Recorder view during the take; playback
  moves the playhead to Audio File. The take is in page memory only. It does not
  arm MediaRecorder, request microphone permission or save a file.
- Built Mixer with two distinct generated tracks, independent 0–100% controls
  and an animated sum. Mixer values change their own Web Audio gains.
- Replaced Meter's generic waveform with stereo RMS bars, current peak and
  peak-hold markers, dBFS scale, over-full-scale region and clipping label,
  following the app's Meter inspector. The page identifies readings as
  illustrative rather than live measurements.
- Rewrote all ten Graphic EQ explanations with listening purpose, a practical
  example, lower/higher guidance and cautions.

## Decisions and limitations

Examples use deterministic, locally generated Web Audio at conservative gain.
They never capture microphone/application audio, load a remote sample, or
connect to another AudioRouter. Cleanup filters, FIR, network delay and other
complex processing are intentionally simplified browser illustrations; they
do not qualify DSP quality or measured latency. The plugin guide omits audio
because a generic preview would misrepresent arbitrary installed plugins.

## Verification evidence

- `node site/generate-guides.mjs` — generated 35 guides with 101 configuration
  entries.
- `node site/check-guides.mjs` — passed: 35 pages, 101 property anchors, 941
  local links/assets, all 35 catalog guide links, and UI/native kind parity.
- From `ui/`, `node node_modules/typescript/bin/tsc --noEmit` — passed.
- From `ui/`, `node node_modules/eslint/bin/eslint.js e2e/site-guides.pw.ts` —
  passed.
- Prettier check for guide runtime, generator, guide content and the changed
  Edge test — passed. `git diff --check` — passed.
- Microsoft Edge / Playwright: `node node_modules/@playwright/test/cli.js test
  --config=.guide-sync-check.config.ts e2e/site-guides.pw.ts` — 6 passed in
  1.6 minutes. The temporary config selected installed Edge, one worker and
  wrote results outside the repository. Evidence and screenshots:
  `C:/Users/miste/AppData/Local/Temp/audiorouter-guide-sync-final3/`.
  Coverage included every page, synchronized start/seek/pause, reduced motion,
  mobile width, Mixer balance, Meter rendering, Recorder capture/playback and
  replay, Graphic EQ explanations, and Network Send/Receive.

Visual review: FIR and Meter screenshots confirm the moving cursor tracks the
seek position; Mixer shows two separate paths and controls; Recorder shows the
take at Recorder while recording and at Audio File during playback. Desktop
hardware audio, driver paths, true recording, exact DSP response, and release
packaging were not tested because this task only changes static guide examples.

## Rollback and handoff

Revert this follow-up's edits in `site/assets/guide-animation.js`,
`site/assets/guide-audio.js`, `site/assets/guides.js`, `site/assets/guides.css`,
`site/generate-guides.mjs`, `site/guide-content/processing.json`, generated
`site/tools/*.html` and `site/assets/guide-manifest.json`,
`ui/e2e/site-guides.pw.ts`, and this plan/index entry. Do not revert unrelated
desktop UI changes in the shared working tree. No database or file migration
is needed.

No commit, desktop rebuild, publication or upload was performed. Next step:
owner reviews the local static guide pages; publication remains separately
authorized.
