# 01 — Product and release scope

Milestone ownership: M00 establishes feasibility; M01–M07 deliver behavior; M08 verifies the complete v1 release.

## Product intent

AudioRouter gives Windows users a visible, understandable path from a sound source through optional processing to one or more destinations. It combines routing and useful effects in one product. An external assistant can inspect and edit exactly the same configuration as the person using the canvas. An LLM is optional; audio operation must not depend on one.

The user's words “compensation tools with gates” are interpreted as dynamics processing: compressor, makeup gain, noise gate/expander, and peak limiting. “Compensation” also includes explicit path latency compensation. These are distinct controls. “Blocking frequencies” means high/low-pass or notch filtering. Exact presets should be tuned using the user's equipment during implementation, not guessed as universal microphone settings.

## Requirements

- **PROD-01 — Platform.** v1 shall support Windows 11 x64 only. Windows 10, Linux, macOS, Windows Server, and Windows on ARM are not supported targets. M00 shall pin tested Windows 11 builds; M08 shall test the then-supported release matrix. Shared packages may be portable internally without creating a cross-platform support obligation.
- **PROD-02 — Backend ownership.** Every supported audio/configuration action available in the UI shall be available through the public local API and CLI; MCP shall expose discovery and dispatch for the same actions subject to identical permissions. Presentation-only actions such as window placement need no audio API equivalent.
- **PROD-03 — Primary workflow.** The product shall replace the need to combine a virtual mixer and a separate plugin host for the reference microphone/Discord/headphones/game-recording setup. A complete v1 shall include first-class virtual endpoint provisioning; use of a separately installed cable is an interim prototype path only.
- **PROD-04 — Understandability.** Users shall see source, processing order, branching, active destinations, channel mapping, bypass, mute, error, and recording state without opening unrelated mixer applications. Templates and sensible defaults shall produce usable routes before advanced settings are needed.
- **PROD-05 — Background operation.** Closing the editor shall leave explicitly running sessions operating. Saved endpoints shall survive a reboot. Audio processing may resume after user sign-in when enabled; operation before sign-in is outside v1.
- **PROD-06 — Offline operation.** After installation, routing, effects, recording, editing, API discovery, and CLI/MCP operation shall function without an account, subscription, internet access, or cloud inference. An external LLM may have its own requirements, outside AudioRouter.
- **PROD-07 — Honest capabilities.** Unsupported capture modes, unavailable drivers/plugins, untested app compatibility, high device latency, and pending restart shall be visible and machine-readable. A capability cannot be advertised merely because a UI control exists.
- **PROD-08 — Scope and simplicity.** Deliver mono/stereo routing, up to eight virtual buses, and the limits in [14](14-quality.md) before multichannel studio features. Advanced controls stay progressive. Do not copy third-party product assets, names, file formats, or exact visual design.

## Release slices

| Slice | Delivered value | Not a completion claim |
| --- | --- | --- |
| M00 feasibility | Measured Windows capture/render and a credible signed-driver path | No product yet |
| M01–M02 foundation | Headless API, simulated graph, then physical audio and app capture | Virtual microphone workflow incomplete |
| M03–M04 functional alpha | Virtual buses, built-in voice chain, recording, CLI routing | UX, plugin compatibility, recovery unfinished |
| M05–M06 usable beta | Visual editor and isolated VST3/pitch support | Release hardening unfinished |
| M07–M08 v1 | MCP parity, recovery, signed installation, acceptance evidence | Future feature list is excluded |

## v1 feature boundary

Required: physical input/output selection; endpoint loopback; individual process-tree capture; explicit desktop sink routing; fan-out and explicit mixers; mono/stereo channel mapping; eight persistent virtual buses; enable/bypass/mute/remove; per-path gain; EQ including notch; compressor and gate; limiter; delay; pitch shift; VST3 x64 effects; WAV/FLAC recording; independent recorder branches; session/preset import/export; meters; API/CLI/MCP; tray/background operation; safe startup, recovery, and uninstall; keyboard-accessible visual editing.

Deferred: arbitrary multi-application system exclusions, automatic silencing of captured apps, 64-channel devices, ASIO/exclusive-mode tuning, VST2 compatibility, 32-bit plugin bridging, Audio Units, cloud accounts, broadcasting, transcription, time-shift, spectral restoration, convolution, complex recording schedules, scripting runtimes, MIDI instruments, and collaborative editing. See [future plans](../plans/future/README.md). Speech denoise and multiband compression are future; multiband EQ and ordinary compression are required now.

Built-in processing satisfies the user's EQ/dynamics needs even if their particular ReaPlugs build cannot be loaded. Exact compatibility with an existing VST2 plugin must not be implied by VST3 support. [Plugin requirements](07-processing.md) define the compatibility gate.

## Product success

At M08, at least four of five first-time Windows participants shall complete the primary template within ten minutes, including selecting Discord and recording inputs in those applications. Every participant shall correctly identify which sources Discord receives. The same setup shall be reproducible from CLI commands using discovered identifiers. Timing excludes download/install/reboot time, which is measured separately. A failed usability target requires a documented UX revision and retest, not relabeling participants as experts.
