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

| Candidate | User value | Reconsider only when |
| --- | --- | --- |
| [Code review 2026-10-07](code-review-2026-10-07.md) | Prioritized (P0–P3) fixes for performance, reliability, tests, debuggability, UX and security across backend, API, MCP and UI | Each item is picked up individually with the user's go-ahead; start with P0 |
| [External app integrations](external-app-integrations.md) | Generate REST requests by selecting a session, tool and property; game menu/match volume events | Existing API use cases reviewed; user approves request-builder scope, temporary-state and multi-tool policies |
| [Siege footstep EQ](siege-footstep-eq.md) | Hear steps and drones better from a measured EQ + dynamics chain | User reference recordings exist; user requested 2026-10-03 |
| [Hardware control panel](hardware-control-panel.md) | Raspberry Pi touch console with knobs and a shaped bezel to glance at and adjust any session while playing | Local-network listener (HTTP-09) done 2026-10-04; prototype, live-gesture method, pins and pairing each need the user's go-ahead |
| [Stream Deck control](stream-deck.md) | Mute, bypass, enable, any on/off setting, live meters and session switching from an Elgato Stream Deck while playing | Plan written 2026-10-04; local HTTP API (HTTP-08/09) in place; plugin build needs the user's go-ahead and a Stream Deck to test |
| [Spatial audio](spatial-audio.md) | Speaker mode, distance/room, head tracking and other rates beyond the implemented 5.1/7.1 surround-to-headphones input (CAP-14) | Method, listening/measurement criteria and latency are specified per extension |
| VST2/ReaPlugs legacy hosting or migration | Reuse exact existing effects | The explicitly authorized gated M06 extension is already tracked in the active milestone; further promotion beyond the current x64 worker evidence still requires rights, editor compatibility, independent coverage, maintenance, and release qualification |
| Native ARM64 Windows | Support ARM laptops | Driver, plugin architecture, shell, and hardware test matrix funded; still Windows-only |
| 8–64 channel virtual devices | DAW/multitrack studio routing | Stereo workflows stable and receiving-app/channel/driver constraints tested |
| ASIO and exclusive-mode options | Lower latency on selected interfaces | Shared-mode targets met and ownership conflicts/driver licensing addressed |
| Multi-app exclusions from system capture | Exclude calls/notifications from desktop mix | A correct Windows implementation proven beyond one process-tree exclusion |
| Automatic “mute when capturing” | Avoid manual output rerouting | Capture remains audible while normal playback is suppressed through a supported/tested mechanism |
| ~~Speech denoise/profile denoise/dehum/declick~~ — implemented as built-in tools (Denoise with a learned profile, Speech denoise, Dehum, Declick, Spectral gate) | Cleaner noisy sources | Still future: model-based voice isolation, after license, latency and offline-cost review |
| Multiband compressor/automatic gain (ducking done: Duck with side-chain and game-round trigger; Compressor, Gate and Limiter exist) | More complex broadcast mixing | Simple dynamics UX and sidechain/failure/latency semantics are stable |
| Convolution with imported impulse responses (FIR Filter tool exists) | Room/monitor treatment | IR import safety and FFT/latency budget established |
| ~~Time-shift~~ — implemented as the Time Shift tool (pause, jump back or forward, return to live). Still future: saving an instant-replay clip | Review recent audio | Explicit privacy, bounded storage and file model for saved clips |
| Silence-driven recording/schedules | Unattended recording | Crash/file durability and consent/startup behavior proven |
| MP3/AAC/ALAC/AIFF and advanced metadata | Additional distribution/archive formats | Encoder maintenance/license/quality/format tests evaluated |
| Transcription and audio analysis | Search recordings or detect hum | Explicit audio grants, offline/cloud boundary, model costs/privacy defined |
| Broadcasting/RTMP/Icecast | Direct streams | Networking, credentials, encoding, reconnection, and destination authorization specified |
| Soundboard with hotkeys and fades (Input Switch and Audio File play/pause/stop exist; see also the [Stream Deck plan](stream-deck.md)) | Live show control | Mixing/shortcuts stable and additional UI remains understandable |
| Standalone browser/remote API | Control from other devices | Partly addressed 2026-10-04 by the opt-in local-network listener (HTTP-09, [hardware control panel](hardware-control-panel.md)). Pairing, per-device grants, TLS and anything beyond one private network still need authentication, transport and threat model approval |
| Reusable nested subgraph definitions | Share complex processing chains | Parameter scoping, cycle/version migration, and transparent route introspection designed |
| Scripting/event schedules | Advanced local automation | Capability-scoped execution and resource limits specified; no arbitrary privileged shell |
| Acoustic echo cancellation | Speaker-based conferencing | Reference signal, device clocks, double-talk behavior, and quality testing established |

## Promoting a future plan

Capture the user outcome, expected scope, affected current requirements, dependencies, risks, acceptance criteria, migration plan, and implementation sequence. Link an explicit scope decision. Add a new milestone or revise affected milestone contracts and traceability before implementation. Update the active plan; leave this entry with a link to its disposition so later agents understand why scope changed.

## Rejected scope for this project

Linux/macOS versions, copying competitor assets, DRM bypass, anti-cheat injection, mandatory cloud accounts, and a UI-only configuration engine conflict with the current product direction. Reconsideration requires a clear new user instruction, not an agent inference from a dependency's cross-platform support.
