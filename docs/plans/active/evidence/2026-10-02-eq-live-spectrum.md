# Advanced EQ live spectrum

Date: 2026-10-02. Windows 11 x64, MSVC, Node 22, Edge Playwright harness.
User request: while audio plays, show in the Advanced EQ which frequencies rise
when walking or shooting.

## Implementation

- DSP: `spectral::SpectrumAnalyzer`, display-only: Hann-windowed 1 024-sample
  frames every 512 samples (≈94 frames/s at 48 kHz), 64 log-spaced bands
  (the Spectral Gate layout), fast rise / slow fall, published through the
  existing lock-free `SpectrumTap`. It reads samples only (no audio change,
  no latency); storage is allocated at construction.
- Engine: the Advanced EQ stage (not Bass & Treble/Dehum, which share the
  filter stage) analyses its incoming audio (mean of both channels) and reports
  it through the existing `spectrum_levels_for_node` → `nodeTelemetry[].spectrum`
  path. A busy analyzer skips a block of display, never audio.
- UI: shaded "Live sound in (before EQ)" area behind the response curve while
  playing. Its 60 dB scale tracks a ceiling that jumps to new peaks and falls
  1 dB per update (floor −40 dB), so shots and footsteps stand out. The EQ's
  dB labels apply to the curve, not the shaded area.
- Limit: ~47 Hz FFT bins make the display coarse below ~150 Hz.

## Checks (Windows)

| Check | Result |
| --- | --- |
| DSP analyzer tests (250 Hz/1 kHz/6 kHz tone placement, silence) | 2 passed; dsp library 61 passed |
| Engine `advanced_eq_reports_the_incoming_spectrum_without_changing_audio` | passed (flat EQ output equals 0 dB Gain; 1 kHz peak band; Bass & Treble has no spectrum) |
| Engine library / deterministic / combinations / routes | 157 / 33 / 3 / 8 passed |
| Control library | 227 passed, 9 ignored |
| UI `npm test` / typecheck | 46 files, 453 tests passed |
| `e2e/eq-spectrum.pw.ts` | 3 passed; [dark](screenshots/2026-10-02-eq-spectrum/eq-spectrum-dark.png), [light](screenshots/2026-10-02-eq-spectrum/eq-spectrum-light.png), [high contrast](screenshots/2026-10-02-eq-spectrum/eq-spectrum-high-contrast.png) |

Debugging note: the first DSP test used 100 Hz, which spreads into the lowest
band because of the 47 Hz bin width; the test now uses 250 Hz and the limit is
documented above.

Review build `target/reviews/eq-spectrum-20261002/` (shell SHA-256
`db700bf764341d9c7263808fe343b8b5cb84ab096e13043092442e0ece38098a`),
includes the earlier Duck, Quit and combination-suite changes. Not launched:
the user's review shell (PID 58648) is running. Attended check: walk/shoot in
Siege with the Advanced EQ selected while playing.
