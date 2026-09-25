# Spatial audio exploration

Status: optional future processor; no implementation authorized by the EQ
screenshot alone. The user wants to consider spatial processing for Siege and
provided a SteelSeries GG image with headphone/speaker modes, a
performance-to-immersion control, and distance control.

The first product decision is the input/output channel contract: stereo game
audio cannot reliably reconstruct discrete rear/height objects that are not
present in the source. A multi-channel or object stream could carry more
directional information, but requires endpoint, game, graph, and recording
qualification. An initial experiment could compare unprocessed stereo,
Windows spatial output, and one explicit HRTF renderer using known directional
test material. Avoid stacking spatial processors in the listening path.

Before implementation, specify headphone and speaker behavior separately,
supported channel layouts, orientation/head-tracking policy, distance and
tuning semantics, latency/CPU bounds, bypass/failure behavior, preset storage,
and a listening test that checks front/back and left/right localization without
unacceptable coloration. Confirm whether Outplayed should receive spatialized
headphone audio or an unprocessed recording branch. The visual controls alone
do not define a reproducible DSP algorithm or equivalence to SteelSeries Sonar.

Official context: [SteelSeries Sonar settings](https://support.steelseries.com/hc/en-us/articles/22291026664717-Getting-to-know-your-sonar-settings)
describes virtual speaker proximity and immersion; its [GG 20 release notes](https://techblog.steelseries.com/2022/06/21/GG-notes-20.0.0.html)
state that headphone/speaker mode changes its HRTF. These are product behavior
descriptions, not a transferable implementation specification.
