# Future plans and explicit v1 exclusions

This folder is for work that is not currently being implemented and is not
implicitly authorized by its presence here. Delivered scopes are in
[`../archived/`](../archived/README.md); active implementation and release
gates are in [`../active/current.md`](../active/current.md) and the linked
active plans.

The AudioRouter-owned virtual-cable track was reopened by the user on
2026-10-05 (DEC-18). Engineering and qualification status are in the
[active virtual-cable plan](../active/virtual-cable.md). The production
certificate purchase and first public driver release still need separate
user go-ahead; [M03-driver-signing.md](M03-driver-signing.md) preserves the
signing and release strategy.

The Windows app installer and manual GitHub draft-release track was promoted
to M08 by DEC-17. Its current status is in the
[active plan](../active/current.md) and
[distribution runbook](../../operations/distribution.md).

These ideas are recorded for later prioritization. They are not authorized implementation tasks and do not delay v1 unless the user explicitly changes scope. Core user requests—routing, built-in voice effects including pitch, VST3, recording, virtual devices, visual editing, and API/CLI/MCP parity—remain in the v1 milestones.

The user has explicitly requested that MCP remain vendor-neutral and
authorized evaluation of remote ChatGPT/Cowork support. The immediate UI
clarity correction is archived; an authenticated remote MCP endpoint and
secure relay/hosting remain planned work in
[Generic remote MCP](generic-remote-mcp.md), because the security and operating
model, service ownership and ongoing cost must be designed before public
reachability is enabled.

## Promoted quality work

[Application quality and regression prevention](../active/application-quality.md) records
the owner-requested plan (2026-10-21): consistent merge gates, state-transition
and fault tests, production/packaged UI verification, backend/audio regression
coverage, reliability trends and release/rollback qualification. Implementation was authorized on 2026-10-21 and is now active.

## Delivered features and their remaining scope

These are implemented; only the specific follow-ups in the table below remain
future work:

- The API request builder (HTTP-08) is implemented. Only the separate
  [integration extensions](external-app-integrations.md) remain proposed.
- The core Stream Deck plugin is packaged with AudioRouter and owner-confirmed
  working well. Its implementation plan is
  [archived](../archived/2026-10-15-stream-deck-core.md); only separately
  chosen [extensions](stream-deck-extensions.md) remain future work.
- The measured Siege EQ/dynamics chain is applied to the owner's session. Its
  completed record is [archived](../archived/2026-10-15-siege-footstep-eq.md);
  personal tuning is revisited only on request.
- Spatial audio already includes surround-to-headphones, speaker mode, room
  control, and graph-rate conversion for supported endpoint rates; remaining
  ideas are listed in [spatial audio extensions](spatial-audio.md).
- FIR Filter already imports WAV/MP3 impulse responses and performs bounded,
  partitioned convolution (up to 2 seconds, with disclosed latency).

The Windows installer/draft-release work is active under M08, not future.
VST2 hosting is an explicitly authorized but gated M06 extension. Consult the
[active plan](../active/current.md) for both.

| Candidate | User value | Reconsider only when |
| --- | --- | --- |
| [Generic remote MCP](generic-remote-mcp.md) | Vendor-neutral remote MCP for ChatGPT, Cowork and other cloud clients; recommended AudioRouter-managed outbound relay architecture | Decide service ownership, hosting budget, account identity, privacy/retention and operational support; then schedule OAuth, relay and Streamable HTTP implementation |
| [Marketing plan](marketing.md) | Get audiorouter.org in front of streamers, Discord users and gamers on a near-zero budget: listings, community posts, weekly content, a gated Show HN/Product Hunt launch | Owner picks the Phase 0 tasks to start; outside posts are made by the owner from their own accounts |
| [Code review 2026-10-07](code-review-2026-10-07.md) | P0/P1 are resolved; several P2/P3 items are implemented, measured, or still await Windows evidence | Review the per-item status before selecting a remaining item; the old P0-first ordering is superseded |
| [External app integration extensions](external-app-integrations.md) | Temporary overrides, atomic scenes/batches, integration-specific grants, and affected-output/preparation hints | Define temporary-state restoration and atomic batch behavior; approve any new grant scope |
| [Android phone audio companion](android-phone-audio.md) | Send a phone microphone into an AudioRouter Network Receive node over the trusted local network | Approve an Android app milestone; confirm supported Android versions, connection UX, and distribution channel before implementation |
| [Hardware control panel](hardware-control-panel.md) | Raspberry Pi touch console with knobs and a shaped bezel to glance at and adjust any session while playing | Local-network listener (HTTP-09) done 2026-10-04; prototype, live-gesture method, pins and pairing each need the user's go-ahead |
| [Stream Deck extensions](stream-deck-extensions.md) | Optional dials, reduced-scope pairing/grants, lighter/faster updates, atomic scenes and profile polish; core plugin is complete | Pick a specific extension; hardware-specific latency/CPU or usability claims need a corresponding device check |
| [Code review follow-ups after 0.0.21](review-fixes-0.0.21.md) | All nine 0.0.21 review findings and the Edge flakes, with how each was resolved (2026-10-23) | Resolved; kept as the record until the next release ships it |
| [UAD plugin support](uad-plugin-support.md) | Use every UAD family a customer owns where technically possible (UADx/Spark, UAD-2 DSP, Apollo Console routing), with general host fixes: class choice, mono/stereo negotiation, latency reporting, VST3 editors, licence diagnostics | Owner chooses the UAD-2 hardware route and authorizes U1 (inventory, no code); PLUG-01/PLUG-05 changes need decision records |
| [Wave Link parity candidates](wave-link-parity.md) | Wave Link 3.x features compared with AudioRouter (listen to any mix, mixer grid, app groups, Output Switch, Sound Check, hidden devices, level target zone…), with effort and risk | App picker/groups (#4), level target zone (#11) and hidden devices (#14) merged; Sound Check (#10) merged in PR #60; Output Switch (#17) implemented and pending owner testing ([active plan](../active/output-switch.md)) |
| [Spatial audio extensions](spatial-audio.md) | Per-speaker angle/distance, head tracking, height/object audio and personalized HRTFs; complete attended listening for existing modes | Define reproducible algorithm, listening/measurement criteria and latency for each extension |
| Native ARM64 Windows | Support ARM laptops | Driver, plugin architecture, shell, and hardware test matrix funded; still Windows-only |
| 8–64 channel virtual devices | DAW/multitrack studio routing | Stereo workflows stable and receiving-app/channel/driver constraints tested |
| ASIO and exclusive-mode options | Lower latency on selected interfaces | Shared-mode targets met and ownership conflicts/driver licensing addressed |
| Multi-app exclusions from system capture | Exclude calls/notifications from desktop mix | A correct Windows implementation proven beyond one process-tree exclusion |
| Automatic “mute when capturing” | Avoid manual output rerouting | Capture remains audible while normal playback is suppressed through a supported/tested mechanism |
| [Fun Voice Changer](voice-effects.md) | One new tool with tweakable presets: Pitch Correct ("auto-tune" style), deep, chipmunk, robot, monster, radio, whisper… on the live microphone; existing tools unchanged (owner decision 2026-10-18) | Promoted 2026-10-18 and implemented as DSP-20; see the [active plan](../active/current.md) for status |
| Model-based voice isolation | Remove difficult background noise from speech | License, offline operation, latency and failure behavior are reviewed |
| Multiband compressor and automatic gain control | More complex broadcast mixing and level consistency | Current Compressor is single-band; Duck, Gate and Limiter are separate tools. Define behavior, latency, loudness/pumping criteria and protected-voice failure handling |
| ~~Time-shift~~ — implemented as the Time Shift tool (pause, jump back or forward, return to live). Still future: saving an instant-replay clip | Review recent audio | Explicit privacy, bounded storage and file model for saved clips |
| Silence-driven recording/schedules | Unattended recording | Crash/file durability and consent/startup behavior proven |
| AAC/ALAC/AIFF and advanced metadata | Additional distribution/archive formats (recording already supports WAV, FLAC and MP3) | Encoder maintenance/license/quality/format tests evaluated |
| Transcription and audio analysis | Search recordings or detect hum | Explicit audio grants, offline/cloud boundary, model costs/privacy defined |
| Broadcasting/RTMP/Icecast | Direct streams | Networking, credentials, encoding, reconnection, and destination authorization specified |
| Dedicated soundboard UI with hotkeys and fades (Audio File play/pause/stop already exist; see [Stream Deck extensions](stream-deck-extensions.md)) | Live show control | Define mixing, shortcut conflicts and safe fade behavior |
| Standalone browser/remote API | Control from other devices | Partly addressed 2026-10-04 by the opt-in local-network listener (HTTP-09, [hardware control panel](hardware-control-panel.md)). Pairing, per-device grants, TLS and anything beyond one private network still need authentication, transport and threat model approval |
| Reusable nested subgraph definitions | Share complex processing chains | Parameter scoping, cycle/version migration, and transparent route introspection designed |
| Scripting/event schedules | Advanced local automation | Capability-scoped execution and resource limits specified; no arbitrary privileged shell |
| Acoustic echo cancellation | Speaker-based conferencing | Establish reference-signal ownership, device-clock alignment, double-talk behavior, and quality testing |

## Promoting a future plan

Capture the user outcome, expected scope, affected current requirements, dependencies, risks, acceptance criteria, migration plan, and implementation sequence. Link an explicit scope decision. Add a new milestone or revise affected milestone contracts and traceability before implementation. Update the active plan; leave this entry with a link to its disposition so later agents understand why scope changed.

## Rejected scope for this project

Linux/macOS versions, copying competitor assets, DRM bypass, anti-cheat injection, mandatory cloud accounts, and a UI-only configuration engine conflict with the current product direction. Reconsideration requires a clear new user instruction, not an agent inference from a dependency's cross-platform support.
