---
format: 1920x1080
duration: 70s
message: "See your sound. Shape it. Send it anywhere."
arc: Promise → effortless setup → visual proof → creator workflow → control → invitation
audience: streamers, gamers, podcasters and creators
mode: collaborative
music: uplifting punchy electronic pop, bright bass, bouncy drums, energetic opening
---

# AudioRouter launch — real-screen review v4

## Changes from v1

User feedback: “It is ‘okay’ but I feel my video I did was better. You have the right content but ‘Drag. Connect. Create’ does not show that. It is static. It should have zoom, pan, etc and show the animated audio move around the nodes. The ‘Find your tone’ looks boring. Same with ‘Levels, under control’ it barely see any movement. It can be so much better. You are not representing well. The videos are very basic, you can make it more dynamic, more movement.” Follow-up: “You need camera movement when you move in the canvas. The property windows does not have scrollbar where you can pan quickly to region, show change of configuration, show with fake voice audio how it moves.”

The rejected version used static tool images, weak camera movement, and drawn
signal graphics. This pass uses two fresh AudioRouter-only Window Capture takes.
Every selected clip is genuine app footage; controls, presets, and live graphs
move inside the recording. GSAP animates only the framing and scene entrances.
There are no drawn-on EQ curves, signal traces, or fake app elements. Selected
shots come from a disposable copy with a built-in synthetic voice sample.
Network settings use a sanitized loopback example; no cross-computer transfer
is claimed. API status and the generic MCP setup panel appear in the 12-second
compatibility beat. MCP activity logs, tokens, private paths, and user audio are
excluded.

## Production state (2026-10-02)

The approved 70-second story and local English voiceover/music are built.
Fresh real app footage shows canvas motion, Graphic EQ and Bass & Treble
adjustment, Voice Compressor response to synthetic audio, Speech Denoise, Hum
Removal, Duck, ReaEQ, Network Send/Receive, Timing, API, and the generic MCP
setup panel. Shots contain actual app changes; no final MP4 has been rendered.
The HyperFrames background preview is available at
`http://localhost:3002/#project/audiorouter-launch`. The remaining gate is
reviewing full motion and audio before the final render.

## Direction

58 seconds of product storytelling including the close, 12 seconds of API/MCP.
The 54-second main demo moves from a visual promise through canvas setup,
focused inspectors, plugin and recording workflow, streaming setup, and timing.
Dark app footage dominates, surrounded by large aqua/violet headlines. Cut every
2–3 seconds inside longer sequences; give parameter gestures enough time to read.
Spring entrances and rhythmic handoffs frame real clips, never substitute for
them. Never show code, keys, private recordings or raw personal paths. Avoid
unmeasured processing-speed claims: demonstrate live controls and Timing instead.

## Frame 1 — Own your sound (0–4s)

- scene: A live Advanced EQ response with a clear opening promise.
- duration: 4s
- transition_in: cut
- status: animated
- src: index.html#s1
- type: hook
- persuasion: Outcome promise
- beat: excitement
- voiceover: "Your sound. Your setup. Your way."
- asset_candidates:

On screen: "YOUR SOUND. YOUR WAY." beside the real moving EQ response.
First motion in the first 0.2s.
Footage: real EQ point and curve. The first screenshot is framed on the Properties inspector.
Why: emotional promise before a tour; the product already looks alive.
Truth: real app only, no invented waveform or fake performance number.

## Frame 2 — Build it by sight (4–11s)

- scene: Drag a tool into a route, connect it, Arrange, open Properties.
- duration: 7s
- transition_in: cut
- status: animated
- src: index.html#s2
- type: demo
- persuasion: Friction reduction
- beat: ease
- voiceover: "Move around your mix. See every stage. Make it yours."
- asset_candidates:

On screen: "MOVE THROUGH. YOUR ROUTE." A real app recording pans across the
running canvas; the Timing pane remains visible beside it. No route or signal
animation is added in post.
Do not imply that an unbound physical input works without device selection.
Why: prove that a powerful chain is approachable.

## Frame 3 — Shape every detail (11–21s)

- scene: Advanced EQ, Graphic EQ, and Bass & Treble visual controls.
- duration: 10s
- transition_in: cut
- status: animated
- src: index.html#s3
- type: feature
- persuasion: Show-don't-tell proof
- beat: control
- voiceover: "Shape your tone. Choose a starting point. Make it yours."
- asset_candidates:

On screen: "SHAPE THE SOUND." Quick Advanced EQ, Graphic EQ, and Bass & Treble
captures use restrained camera moves across the real property panes. No
highlight is drawn over the EQ response.
Why: Properties visuals, rather than a list of features, prove the promise.

## Frame 4 - Voice up, peaks down (21-30s)

- scene: Voice Compressor response to the built-in synthetic voice sample.
- duration: 9s
- status: animated
- voiceover: "Set your voice level in a tap. See the compressor follow a real voice sample as it plays."

Three cuts stay on the real compressor inspector while its controls and live
response are visible. No processing-speed number or zero-latency claim.

## Frame 5 - Make room for your voice (30-38s)

- scene: Speech Denoise, Hum Removal, and Duck configuration.
- duration: 8s
- status: animated
- voiceover: "Tame the room hum. Shape denoising. Let your game step back when you speak."

The real inspectors show the one-click denoise setting, 50Hz hum curve, and
Duck controls. Keep the configuration readable; no fabricated spectrogram or
signal overlay.

## Frame 6 - Plugins and a visual route (38-44s)

- scene: Installed ReaEQ controls and an actual canvas pan.
- duration: 6s
- status: animated
- voiceover: "Bring your favorite plugin into the route, then move through a canvas built around your sound."

## Frame 7 — Two PCs, one stream (44–49s)

- scene: Real Network Send and Network Receive settings, using a sanitized
  loopback example address.
- duration: 5s
- transition_in: cut
- status: animated
- src: index.html#s7
- type: feature
- persuasion: Creator outcome
- beat: possibility
- voiceover: "Gaming PC to streaming PC. Send your mix across your home network."
- asset_candidates:

On screen: "TWO PCS. ONE STREAM." Sender and receiver controls appear as
readable real clips with source/destination labels. Use sanitized showcase
addresses. No internet, encrypted streaming or remote-control claim.
Why: direct payoff for streamers and the user's proven two-PC use.

## Frame 8 — See the whole journey (49–54s)

- scene: Real Timing view for the showcase chain.
- duration: 5s
- transition_in: cut
- status: animated
- src: index.html#s8
- type: proof
- persuasion: Transparency
- beat: clarity
- voiceover: "Tune it live. See the timing of every tool, and your whole audio journey."
- asset_candidates:

On screen: "TUNE LIVE. SEE THE TIMING." Show actual Timing readings. Never
invent latency values or claim universally glitch-free/zero-latency audio.
Why: makes responsiveness and timing confidence visible in user language.

## Frame 9 — Control it your way (54–66s)

- scene: API compatibility view and a genuine successful MCP settings call.
- duration: 12s
- transition_in: cut
- status: animated
- src: index.html#s9
- type: compatibility
- persuasion: Extensibility without complexity
- beat: empowerment
- voiceover: "Make it part of your workflow. Connect your controls through the API. Or let an AI assistant help configure your sound with MCP."
- asset_candidates:

On screen: "API + MCP" then "YOUR CONTROLS. YOUR ASSISTANT." Six seconds
each. Show the generic MCP setup panel in the app. Keep the captured activity log,
tokens and machine-specific values out of frame.
Use benefit copy, not protocol, schema, installation commands or implementation.
Capture genuine successful tool calls; do not fabricate an assistant transcript.
Why: reserves the requested 10–15s for automation while visuals stay dominant.

## Frame 10 — Make your sound yours (66–70s)

- scene: Return to the opening route, then AudioRouter name and download CTA.
- duration: 4s
- transition_in: cut
- status: animated
- src: index.html#s10
- type: cta
- persuasion: Clear invitation
- beat: motivation
- voiceover: "See your sound. Shape it. Send it anywhere. AudioRouter."
- asset_candidates:

On screen: "AudioRouter" / "See your sound. Shape it. Send it anywhere."
/ "Get the Windows preview on GitHub" / "github.com/MrDesjardins/audiorouter".
Small readable "Windows 11 · Unsigned preview" footer. Route is the callback
to frame 1; music resolves with a clean, upbeat finish.

## Arithmetic and remaining gates

4 + 7 + 10 + 9 + 8 + 6 + 5 + 5 + 12 + 4 = 70 seconds.
Narration, captures, design, frame build, checks, and snapshots are complete.
Pending: review this v3 real-screen pass, then obtain final render approval.
Verify MP4 duration and audio after that approval. No MP4 yet.
