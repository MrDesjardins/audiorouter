# Future plans and explicit v1 exclusions

The AudioRouter-owned kernel-driver, PortCls, production driver-signing, and
managed-driver qualification track in
[M03-driver-signing.md](M03-driver-signing.md) was reopened by the user on
2026-10-05 (DEC-18): AudioRouter will ship its own signed virtual cable.
Execution status is in the [active plan](../active/current.md); the
certificate purchase and first driver release need a separate user go-ahead.

The Windows app installer and manual GitHub draft-release track was promoted
to M08 by DEC-17. Its current status is in the
[active plan](../active/current.md) and
[distribution runbook](../../operations/distribution.md).

These ideas are recorded for later prioritization. They are not authorized implementation tasks and do not delay v1 unless the user explicitly changes scope. Core user requests—routing, built-in voice effects including pitch, VST3, recording, virtual devices, visual editing, and API/CLI/MCP parity—remain in the v1 milestones. The VST2 row is now promoted to an explicitly authorized, gated M06 extension by the 2026-09-08 decision recorded in the active plan and delivery register.

## Delivered features that were previously listed here

These are implemented; only the specific follow-ups in the table below remain
future work:

- The API request builder (HTTP-08) is implemented. The Stream Deck plugin is
  packaged with AudioRouter, and the user confirms the integration works well.
- The measured Siege Footstep EQ chain is applied to the user's session. Its
  plan records v2 listening feedback as the only reason to tune it further.
- Spatial audio already includes surround-to-headphones, speaker mode, room
  control, and graph-rate conversion for supported endpoint rates.
- FIR Filter already imports WAV/MP3 impulse responses and performs bounded,
  partitioned convolution (up to 2 seconds, with disclosed latency).

| Candidate | User value | Reconsider only when |
| --- | --- | --- |
| [Code review 2026-10-07](code-review-2026-10-07.md) | Prioritized (P0–P3) fixes for performance, reliability, tests, debuggability, UX and security across backend, API, MCP and UI | Each item is picked up individually with the user's go-ahead; start with P0 |
| [External app integrations](external-app-integrations.md) | Extensions to the delivered request builder: temporary overrides, atomic scenes/batches, integration-specific grants, and affected-output/preparation hints | Define temporary-state restoration and atomic batch behavior; approve any new grant scope |
| [Siege footstep EQ](siege-footstep-eq.md) | Optional tuning follow-up to the applied measured EQ/dynamics chain | User reports v2 listening results or requests an adjustment |
| [Hardware control panel](hardware-control-panel.md) | Raspberry Pi touch console with knobs and a shaped bezel to glance at and adjust any session while playing | Local-network listener (HTTP-09) done 2026-10-04; prototype, live-gesture method, pins and pairing each need the user's go-ahead |
| [Stream Deck extensions](stream-deck.md) | Optional dials, reduced-scope pairing/grants, faster change notifications, atomic scenes and profile polish; the core plugin is shipped and user-confirmed working well | Pick a specific extension; hardware-specific latency/CPU or usability claims need a corresponding device check |
| [Spatial audio extensions](spatial-audio.md) | Per-speaker angle/distance, head tracking, height/object audio and personalized HRTFs; complete attended listening for existing modes | Define reproducible algorithm, listening/measurement criteria and latency for each extension |
| VST2/ReaPlugs legacy hosting or migration | Reuse exact existing effects | The explicitly authorized gated M06 extension is already tracked in the active milestone; further promotion beyond the current x64 worker evidence still requires rights, editor compatibility, independent coverage, maintenance, and release qualification |
| Native ARM64 Windows | Support ARM laptops | Driver, plugin architecture, shell, and hardware test matrix funded; still Windows-only |
| 8–64 channel virtual devices | DAW/multitrack studio routing | Stereo workflows stable and receiving-app/channel/driver constraints tested |
| ASIO and exclusive-mode options | Lower latency on selected interfaces | Shared-mode targets met and ownership conflicts/driver licensing addressed |
| Multi-app exclusions from system capture | Exclude calls/notifications from desktop mix | A correct Windows implementation proven beyond one process-tree exclusion |
| Automatic “mute when capturing” | Avoid manual output rerouting | Capture remains audible while normal playback is suppressed through a supported/tested mechanism |
| ~~Basic noise reduction and impulse-response convolution~~ — Denoise, Speech Denoise, Dehum, Declick, Spectral Gate and FIR Filter are implemented | Cleaner sources and room/monitor treatment | Delivered; see DSP-12–17. Still future: model-based voice isolation after license, latency and offline-cost review |
| Multiband compressor and automatic gain control | More complex broadcast mixing and level consistency | Current Compressor is single-band; Duck, Gate and Limiter are separate tools. Define behavior, latency, loudness/pumping criteria and protected-voice failure handling |
| ~~Time-shift~~ — implemented as the Time Shift tool (pause, jump back or forward, return to live). Still future: saving an instant-replay clip | Review recent audio | Explicit privacy, bounded storage and file model for saved clips |
| Silence-driven recording/schedules | Unattended recording | Crash/file durability and consent/startup behavior proven |
| MP3/AAC/ALAC/AIFF and advanced metadata | Additional distribution/archive formats | Encoder maintenance/license/quality/format tests evaluated |
| Transcription and audio analysis | Search recordings or detect hum | Explicit audio grants, offline/cloud boundary, model costs/privacy defined |
| Broadcasting/RTMP/Icecast | Direct streams | Networking, credentials, encoding, reconnection, and destination authorization specified |
| Soundboard with hotkeys and fades (Input Switch and Audio File play/pause/stop exist; see also the [Stream Deck plan](stream-deck.md)) | Live show control | Mixing/shortcuts stable and additional UI remains understandable |
| Standalone browser/remote API | Control from other devices | Partly addressed 2026-10-04 by the opt-in local-network listener (HTTP-09, [hardware control panel](hardware-control-panel.md)). Pairing, per-device grants, TLS and anything beyond one private network still need authentication, transport and threat model approval |
| Reusable nested subgraph definitions | Share complex processing chains | Parameter scoping, cycle/version migration, and transparent route introspection designed |
| Scripting/event schedules | Advanced local automation | Capability-scoped execution and resource limits specified; no arbitrary privileged shell |
| Acoustic echo cancellation | Speaker-based conferencing | Establish reference-signal ownership, device-clock alignment, double-talk behavior, and quality testing |

## Promoting a future plan

Capture the user outcome, expected scope, affected current requirements, dependencies, risks, acceptance criteria, migration plan, and implementation sequence. Link an explicit scope decision. Add a new milestone or revise affected milestone contracts and traceability before implementation. Update the active plan; leave this entry with a link to its disposition so later agents understand why scope changed.

## Rejected scope for this project

Linux/macOS versions, copying competitor assets, DRM bypass, anti-cheat injection, mandatory cloud accounts, and a UI-only configuration engine conflict with the current product direction. Reconsideration requires a clear new user instruction, not an agent inference from a dependency's cross-platform support.
