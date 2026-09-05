# Future plans and explicit v1 exclusions

These ideas are recorded for later prioritization. They are not authorized implementation tasks and do not delay v1 unless the user explicitly changes scope. Core user requests—routing, built-in voice effects including pitch, VST3, recording, virtual devices, visual editing, and API/CLI/MCP parity—remain in the v1 milestones.

| Candidate | User value | Reconsider only when |
| --- | --- | --- |
| VST2/ReaPlugs legacy hosting or migration | Reuse exact existing effects | Actual formats, redistribution/hosting rights, maintenance, and safe bridge feasibility established |
| Native ARM64 Windows | Support ARM laptops | Driver, plugin architecture, shell, and hardware test matrix funded; still Windows-only |
| 8–64 channel virtual devices | DAW/multitrack studio routing | Stereo workflows stable and receiving-app/channel/driver constraints tested |
| ASIO and exclusive-mode options | Lower latency on selected interfaces | Shared-mode targets met and ownership conflicts/driver licensing addressed |
| Multi-app exclusions from system capture | Exclude calls/notifications from desktop mix | A correct Windows implementation proven beyond one process-tree exclusion |
| Automatic “mute when capturing” | Avoid manual output rerouting | Capture remains audible while normal playback is suppressed through a supported/tested mechanism |
| Speech denoise/profile denoise/dehum/declick | Cleaner noisy sources | Quality, model/library licenses, offline cost, and latency evaluated |
| Multiband compressor/ducking/automatic gain | More complex broadcast mixing | Simple dynamics UX and sidechain/failure/latency semantics are stable |
| Convolution/FIR speaker correction | Room/monitor treatment | IR import safety and FFT/latency budget established |
| Time-shift/instant replay | Review recent audio | Explicit privacy, bounded storage, latency, and recording model designed |
| Silence-driven recording/schedules | Unattended recording | Crash/file durability and consent/startup behavior proven |
| MP3/AAC/ALAC/AIFF and advanced metadata | Additional distribution/archive formats | Encoder maintenance/license/quality/format tests evaluated |
| Transcription and audio analysis | Search recordings or detect hum | Explicit audio grants, offline/cloud boundary, model costs/privacy defined |
| Broadcasting/RTMP/Icecast | Direct streams | Networking, credentials, encoding, reconnection, and destination authorization specified |
| Full soundboard/input switching/fades | Live show control | Mixing/shortcuts stable and additional UI remains understandable |
| Standalone browser/remote API | Control from other devices | Authentication, origin/CSRF protections, transport, deployment, and threat model separately approved |
| Reusable nested subgraph definitions | Share complex processing chains | Parameter scoping, cycle/version migration, and transparent route introspection designed |
| Scripting/event schedules | Advanced local automation | Capability-scoped execution and resource limits specified; no arbitrary privileged shell |
| Acoustic echo cancellation | Speaker-based conferencing | Reference signal, device clocks, double-talk behavior, and quality testing established |

## Promoting a future plan

Capture the user outcome, expected scope, affected current requirements, dependencies, risks, acceptance criteria, migration plan, and implementation sequence. Link an explicit scope decision. Add a new milestone or revise affected milestone contracts and traceability before implementation. Update the active plan; leave this entry with a link to its disposition so later agents understand why scope changed.

## Rejected scope for this project

Linux/macOS versions, copying competitor assets, DRM bypass, anti-cheat injection, mandatory cloud accounts, and a UI-only configuration engine conflict with the current product direction. Reconsideration requires a clear new user instruction, not an agent inference from a dependency's cross-platform support.
