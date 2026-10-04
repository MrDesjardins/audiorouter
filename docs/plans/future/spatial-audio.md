# Spatial audio

Status: first slice implemented 2026-10-02 as **CAP-14 — Surround to
headphones** ([spec](../../spec/05-windows-capture.md)); execution record in the
[active plan](../active/current.md) and
[evidence](../active/evidence/2026-10-02-spatial-audio.md). The user chose real
5.1/7.1 capture rendered with a measured HRTF (MIT KEMAR) over a stereo-only
virtual-speaker processor.

Still future (not authorized by the first slice):

- ~~Speaker-mode output (crosstalk cancellation)~~ — done 2026-10-03 as
  `spatialMode: "speakers"` (RACE, see [CAP-14](../../spec/05-windows-capture.md)).
  Still future: a separate speaker HRTF set and per-listener speaker angle.
- ~~Room/reverb control~~ — done 2026-10-03 as `spatialRoomPercent` (small-room
  feedback delay network; higher sounds further away). Still future:
  per-speaker distance, gain or angle editing and an "immersion" control. The
  SteelSeries Sonar controls describe product behavior, not a reproducible
  algorithm.
- Attended listening for both: speaker placement and room level have objective
  tests only (crosstalk separation in a simulated speaker-to-ear path, tail
  decay, stability).
- Head tracking, elevation/height channels, object audio (Windows Sonic /
  Spatial Sound APIs) and personalised HRTFs.
- ~~Capturing 44.1/96 kHz surround endpoints with sample-rate conversion~~ —
  done 2026-10-03: multi-path inputs and outputs at 8–192 kHz are resampled to
  the 48 kHz graph by the Windows audio engine ([CAP-14](../../spec/05-windows-capture.md)).
  Attended listening on a 96 kHz 7.1 device remains.
- A listening test that scores front/back and left/right localization and
  coloration across several listeners. The first slice has objective tests
  (ear energy per speaker, front/back difference) and one attended Siege check.
- Whether Outplayed or a Recorder should receive the spatialized mix or an
  unprocessed branch; today a surround input feeds every connected branch the
  same binaural stereo.

Official context: [SteelSeries Sonar settings](https://support.steelseries.com/hc/en-us/articles/22291026664717-Getting-to-know-your-sonar-settings)
describes virtual speaker proximity and immersion; its [GG 20 release notes](https://techblog.steelseries.com/2022/06/21/GG-notes-20.0.0.html)
state that headphone/speaker mode changes its HRTF. The
[KEMAR data](https://sound.media.mit.edu/resources/KEMAR.html) is free to use
with citation.
