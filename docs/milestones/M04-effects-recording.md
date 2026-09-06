# M04 — Built-in effects and independent recording

Status: portable DSP/recording foundation implemented; M03-dependent live acceptance remains open. Prerequisite: M03. Outcome: a complete headless voice-processing and game-recording alpha using built-in effects.

## Read first

[Workflows](../spec/02-workflows.md), [graph](../spec/04-graph.md), [processing](../spec/07-processing.md), [recording](../spec/08-recording.md), [persistence](../spec/12-persistence.md), and [quality](../spec/14-quality.md).

## Ordered implementation

1. Specify DSP transfer functions, detector modes, parameter units/ranges, ramps, bypass/failure behavior, and latency in registry schemas and reference vectors.
2. Implement graphic/parametric EQ, gate/expander, compressor/makeup gain, sample-peak limiter, delay, and useful meters/presets. Keep pitch and VST hosting for M06.
3. Implement mixer path compensation and protected voice-sink failure propagation. Prove branch-specific processing does not alter sibling routes.
4. Implement bounded recorder queues/workers, WAV/FLAC formats, dither/conversion, frame clocks, arming, independent control, and gap/split metadata.
5. Implement safe naming/roots, partial-file checkpoints/recovery, file library API, metadata, preview, remove-entry, and separately authorized recycle. No audio files are uploaded or committed as incidental test artifacts.
6. Add typed CLI commands and method fixtures for every effect/recorder/library operation. Extend export/import with versioned node presets and recording metadata references without bundling recordings.
7. Build UC-02/08 fixtures and rerun the W1 reference route with real effects and recording.

## Acceptance gate

DSP-01–05/07–09; GRAPH-10/14 built-in behavior; REC-01–12; NFR-01–06 applicable W1 budgets; QUAL-01–06 applicable signal/file evidence. UC-01 now includes EQ/gate/compression/limiter, independent monitor gain, and optional file recording. UC-08 disk/encoder errors do not interrupt live voice.

Built-ins replace the need for an external host for the required neutral voice chain. An advertised notch measurably attenuates its target; compressor/gate transfer curves and limiter ceiling meet targets. Recorder pause/split/stop leave other routes continuous. A missing/faulted effect silences protected contributions; deliberate bypass is visible and behaves as specified.

## Verification

Run deterministic signal vectors and decoded-file assertions, synchronized impulse tests, slow/full disk simulation, process-kill recovery, Unicode/path traversal/collision cases, and Windows W1 soak. Keep raw timing/signal metrics and format decoder results. Listening tests supplement measurable checks and record device/voice conditions.

## Boundaries and rollback

No MP3/AAC, speech denoise, silence editing, transcription, cloud storage, or scripting runtime. Avoid introducing a new DSP framework/license without a dependency decision. Recorder failure preserves completed files; cleanup/recycling targets only explicitly identified test files.

## Handoff

Provide machine-readable node schemas, transfer-function docs, presets, recording state/format semantics, CLI recipes, and test artifacts so M05 can build controls without recreating backend logic.

Suggested request: “Implement M04's built-in EQ/dynamics/delay and WAV/FLAC recorder branches, verify signal/file behavior and live-audio isolation, and document all API/CLI controls.”
