# Graphic EQ guide audio level — 2026-10-15

## Objective and evidence

Owner reported that Play Example on the Graphic EQ guide produced no audible
sound. In installed Edge, the generated Graphic EQ samples had nonzero input
and output RMS of 0.143, the AudioContext was `running`, and the selected
audition was output. The guide monitor gain was only 0.16, approximately
16 dB below the synthesized samples, which made the examples too quiet on the
owner's system.

## Change

Raise the shared synthetic-example monitor gain from 0.16 to 0.5 (3.125×) and
retain the downstream DynamicsCompressor as peak protection, including when
EQ bands boost the source. Update the signal-panel note and site README to
state the 50% monitor level. Playback remains user-initiated. Add an Edge
regression that checks the active monitor gain/state and specifically opens
Graphic EQ, verifies its rendered sample RMS and plays through the normal
transport.

## Verification

Windows host, Node 22.22.3, Microsoft Edge. No microphone, audio device,
hardware or network is used by browser automation.

- `node node_modules/@playwright/test/cli.js test --config=.guide-audio-review.config.ts e2e/site-guides.pw.ts --grep "audible fixtures"` from `ui/` — 1 passed, including the Mixer fixtures and Graphic EQ sample/output checks.
- `node node_modules/typescript/bin/tsc --noEmit` from `ui/` — passed.
- Focused ESLint and Prettier check for the modified Edge test and audio runtime — passed.
- `node site/check-guides.mjs` — passed: 35 guides, 101 property anchors, 1,011 local links/assets and UI/native parity.
- `git diff --check` — passed.

The regression observes a running AudioContext and the connected 0.5 monitor
gain; it cannot establish that a particular physical computer has its browser
or speakers unmuted. Reload the local guide with Ctrl+F5 so the browser fetches
the updated audio script.

## Rollback and handoff

Rollback the monitor-level line in `site/assets/guide-audio.js`, the signal note
in `site/assets/guide-playback.js`, the regression in `ui/e2e/site-guides.pw.ts`,
the site README description and this plan/index entry. No session, database,
desktop source or website publication changed. No desktop rebuild is needed
for the static page; website publication remains separate authorization.

Next action: owner reloads the Graphic EQ guide and tries Play Example again.
