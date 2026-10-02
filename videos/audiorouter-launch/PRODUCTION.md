# Production evidence

2026-10-02: User approved the 70-second storyboard, 1920x1080 landscape,
English voiceover, and free local audio. The composition uses ten local Kokoro
`af_heart` voice clips and 70 seconds of locally synthesized instrumental
music. No paid media services are used.

## Fresh real-screen captures

Two OBS Window Capture takes were recorded from the user's actual open
AudioRouter window at 1920x1080/60fps. The captured source was AudioRouter only;
OBS audio sources were muted and the clips have no meaningful audio. The takes
are retained under ignored `capture/reshoot/`; no microphone, desktop audio,
private file path, API token, or machine-specific client identifier is included
in the selected footage. A disposable copied session was used for the fresh
property pass. Its unsaved draft changes were not saved. The temporary
localhost API server was stopped after recording.

Selected moving clips show the real canvas pan, Graphic EQ fader/preset changes,
Bass & Treble preset response, compressor controls and live response to the
built-in synthetic voice sample, Speech Denoise, Hum Removal, Duck, ReaEQ, and
Network Send/Receive UI. The MCP section is cropped from the genuine app
recording to show the generic setup panel only; its activity log is excluded.
A 12-second API/MCP section is retained. The streamer beat communicates a
Gaming PC to Streaming PC workflow; no second-computer network transfer was
performed during filming. Timing values are the readings visible in the app,
not a general performance benchmark.

The build script trims all footage from the recorded takes into the deliverable
clips and applies only camera moves around the real footage. It no longer pads
shots with frozen frames. Raw takes and frame sheets remain private under
`capture/` and are excluded from version control.

## Validation and handoff

HyperFrames CLI 0.8.107 `check` passed on 2026-10-02 with zero errors, zero
runtime or motion errors, and 25/25 contrast checks. Sixteen non-blocking lint
warnings remain (nested timeline containers and one dense track). Hardware GPU
capture used the local NVIDIA RTX 5080. Thirteen fresh scene snapshots were
inspected, including compressor response, API, and the cropped MCP panel.

The background HyperFrames preview is at
`http://localhost:3002/#project/audiorouter-launch`. Full-motion user review is
pending. No final MP4 has been rendered. After review approval, render the MP4,
then verify its exact duration and audio tracks.
