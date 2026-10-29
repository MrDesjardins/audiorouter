# Soundboard sound sourcing, first pass (2026-10-29)

Plan: [active Soundboard plan](../soundboard.md), library rules in
[the sound library plan](../../future/soundboard-sound-library.md). Audio files are
kept outside the repository (`target/sourcing`, git-ignored) until the owner has
chosen what joins the library.

## What was collected

| Source | Files | License evidence | Result |
| --- | --- | --- | --- |
| Kenney: Interface Sounds, Impact Sounds, Digital Audio, Sci-Fi Sounds, UI Audio | 416 (OGG) | Each pack's `License.txt` states Creative Commons Zero (CC0); downloaded by the owner from kenney.nl (scripted download is blocked by the site with a 403 and was not worked around) | All CC0. Mostly very short interface clicks and impacts (average 0.3 to 0.8 s); sci-fi has two files over 5 s |
| Freesound, owner-downloaded | 47 | `tools/soundboard/check-freesound-license.mjs` read the license from each sound's own page (one request every 1.2 s) | 42 CC0, 5 CC-BY 4.0 |
| Original (generated) | 33 | Made by `tools/soundboard/generate-original.mjs` | Project-owned; unreviewed by ear |
| Wikimedia Commons, OpenGameArt | not downloaded | Searched through their APIs / pages | Little suited to a soundboard (Commons: spoken-word recordings, a few doorbells and two roosters; OpenGameArt: almost no fart, clap, applause or air horn) |

## Freesound results

- **Fart folder (17):** 16 are 5 seconds or less. One is longer. One file in the
  folder is a burp.
- **Burp (20):** all CC0, all 5 seconds or less. They need the Bodily humor category
  name to be clear and a "mild" content check by listening.
- **Ta-da (4):** CC0, 5 seconds or less.
- **Applause (5, moved to `_excluded`):** license is CC-BY 4.0 (not allowed in
  version one by the owner's decision) and each is about 17 seconds long.
- **"Oh yeah" (1, moved to `_excluded`):** CC0, but it is a spoken phrase, which the
  content rules exclude.

## Gaps still to fill

Clapping, applause, laughter, rimshot recordings (a synthesized one exists), gasp,
groan, air horn recordings, animals (dog, cat), explosion, crowd cheer. Options:
the owner downloads more CC0 sounds from Freesound (filter "Creative Commons 0",
duration 5 seconds or less), or the owner provides an API key so the agent can
search directly. Generated sounds already cover air horn, rimshot, sad horn, boing,
slide whistle, record scratch, crickets, buzzer, doorbell, coin, power-up,
game over, laser and whoosh.

## Preparation findings

- The 33 generated sounds are matched at -3 dBFS peak but their average level
  ranges from about -4 to -19 dB. The library process needs loudness (RMS)
  matching in addition to a peak ceiling.
- One downloaded file is M4A (`787755__sadiquecat__short-fart.m4a`) and one AIFF;
  the engine decodes only WAV and MP3 today, so every shipped file is converted.
  ffmpeg is available on the owner's PC (WinGet) for this.

## Next steps

1. Owner listens with `target/sourcing/audition.html` and ticks the sounds to keep.
2. Fill gaps (above).
3. Convert, trim, level-match, name and write the manifest with source, author,
   license and SHA-256 for every kept file; commit the files and notices.
