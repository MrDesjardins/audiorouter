# Duck amount slider and ducking duck on the canvas

Date: 2026-10-02. Windows 11 x64, Node 22, Edge Playwright harness.
User request: a slider instead of amount buttons; show a duck on the canvas
widget while the Duck is active.

## Changes

- Duck Properties: "Turn down by" slider, 0–40 dB in 1 dB steps, value shown
  above, scale labels below (replaces the −3/−6/−10/−15/−20 dB buttons).
- Canvas widget: while ducking, the status dot is replaced by a duck in the
  ducking colour that pops in and gently bobs (no animation with
  `prefers-reduced-motion`). `DuckGlyph` is now shared with the Tools list.
- Defect fixed: a Siege-round Duck's widget said "Choose a trigger in
  Properties"; it now says "Siege round · full volume" when released and
  "Ducking −x dB" while active.

## Checks (Windows)

| Check | Result |
| --- | --- |
| UI `npm test` / typecheck | 46 files, 453 tests passed (Duck test drives the slider) |
| `e2e/duck-widget.pw.ts` + `duck-icon` + `duck-siege-round` | 13 passed |
| Screenshots | widget [dark](screenshots/2026-10-02-duck-widget-slider/duck-widget-dark.png), [light](screenshots/2026-10-02-duck-widget-slider/duck-widget-light.png), [high contrast](screenshots/2026-10-02-duck-widget-slider/duck-widget-high-contrast.png); slider [dark](screenshots/2026-10-02-duck-widget-slider/duck-slider-dark.png), [light](screenshots/2026-10-02-duck-widget-slider/duck-slider-light.png), [high contrast](screenshots/2026-10-02-duck-widget-slider/duck-slider-high-contrast.png) |

Review build `target/reviews/duck-widget-20261002/` (shell SHA-256
`ab1482b253177e9331c069494d821caa38272e8f2d0a5f639e305ceb624d1325`), includes
every earlier review change in this plan. UI-only change; backend unchanged.
