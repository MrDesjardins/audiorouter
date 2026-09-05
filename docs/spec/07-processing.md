# 07 — Built-in processors and plugin hosting

Milestone ownership: M04 required built-in voice chain; M06 pitch and isolated VST3; M08 signal/compatibility evidence.

## Requirements

- **DSP-01 — Core controls.** Every processor shall expose typed parameters, units, defaults, valid ranges, versioned presets, reset, bypass, mute where appropriate, meters, latency, and failure behavior through the node registry/API. Built-ins are usable without external plugins. Displayed values must match effective backend values.
- **DSP-02 — EQ.** Supply a ten-band graphic EQ and an eight-band parametric EQ. Parametric bands support peaking, low/high shelf, high/low pass, and notch; each band has enable, frequency, Q, and applicable gain/slope. A visual response curve is computed from the same coefficient specification as DSP and does not become a separate source of truth.
- **DSP-03 — Dynamics.** Supply a compressor with threshold, ratio, attack, release, knee, makeup gain, and linked stereo detection; and a gate/downward expander with threshold, hysteresis, ratio/range, attack, hold, and release. A gate is amplitude-based, not a substitute for frequency filtering. Basic mode groups advanced controls while preserving direct API access.
- **DSP-04 — Limiting/gain.** Gain supports attenuation, boost, and click-free mute. Limiter supports output ceiling and disclosed lookahead/release. v1 guarantees a tested sample-peak ceiling; do not call it true-peak protection unless oversampling/inter-sample testing is implemented. Gain reduction and clipping have distinct meters.
- **DSP-05 — Delay.** Delay adds 0–1,000 ms with bounded preallocation and de-clicked parameter transitions. It supports manual audio/video alignment and is separate from automatic mixer path compensation. Do not promise perceptually seamless large delay changes; report a rebuilding/warming state if needed.
- **DSP-06 — Pitch.** M06 supplies a time-preserving pitch-shift node, -12 to +12 semitones plus -100 to +100 cents, with explicit algorithmic latency. A VST3-only workaround does not meet the built-in node requirement. Formant control is optional/future. At ±12 semitones, duration remains within 0.1% in a 60-second offline test; pitch error for steady tones is within 10 cents after warmup. Test speech for intelligibility and artifacts separately.
- **DSP-07 — Metering.** Expose per-channel sample peak, RMS, clipping count, gate state, and gain reduction where relevant. RMS window defaults to 300 ms; peak hold defaults to 1 second. Silence is represented by a documented floor or null dB value, never JSON `-Infinity`. Telemetry rate is bounded by [14](14-quality.md).
- **DSP-08 — Smooth and stable.** Parameter ramps avoid avoidable discontinuities. Filter coefficients remain stable throughout changes, including near Nyquist; clamp frequency eligibility to the graph sample rate via schema limits before acceptance. Handle denormals, NaN/Inf, silence, maximum supported gain, and overload without callback failure.
- **DSP-09 — Presets.** Include conservative `Voice neutral`, `Voice gate and compression`, `50 Hz hum notch`, and `60 Hz hum notch` presets with explainable parameters. They are starting points, not automatic calibration or guarantees. Importing a preset never enables microphone monitoring or recording automatically.
- **PLUG-01 — Format boundary.** Support installed x64 VST3 audio effects in M06. VST2, x86 bridging, Audio Units, instruments/MIDI, and arbitrary executable scripting are outside v1. Do not infer ReaPlugs compatibility from the product word “VST”; inspect actual formats and report unsupported plugins explicitly.
- **PLUG-02 — Discovery.** Scan configured local plugin locations only after an explicit scan request/initial user choice. Run scanning in a disposable process with a 10-second per-plugin deadline, cancellation, and crash quarantine. Identify plugins by class ID, vendor, version, architecture, and file fingerprint; paths alone are insufficient. Do not download plugins automatically.
- **PLUG-03 — Isolation.** Host each plugin instance in a separate worker initially; optimization to shared workers needs failure-containment evidence. Exchange fixed-size audio buffers with sequence numbers through bounded shared memory. The realtime engine never waits for the worker. Late/missing frames follow declared silence/dry policy and increment counters. Account for worker pipeline delay in path latency.
- **PLUG-04 — State and UI.** Persist opaque plugin state as versioned binary assets with size limits and hashes; expose automatable parameters through typed descriptors. Native plugin editors may open in worker-owned Windows windows. Opening/closing an editor must not restart processing or prevent CLI control. Generic parameter editing remains available without a native editor.
- **PLUG-05 — Failure.** Detect crash, hang, invalid samples, bus-layout changes, or unsupported latency changes. Quarantine a repeatedly failing plugin, show its identity, and require deliberate retry after three failures within ten minutes. Do not restart-loop forever or automatically send dry microphone audio to a protected sink.
- **PLUG-06 — Compatibility and licenses.** Pin the actual VST3 SDK license/version and retain required notices. Record a tested plugin list with exact binaries/versions and parameter/state/editor results. User-installed plugins are not redistributed by default. VST3 licensing does not confer VST2 rights; review any proposed legacy hosting separately.

## Initial parameter contract

These ranges are product decisions, subject to signal tests; defaults are normative unless M04 records a justified revision.

| Node | Parameters and ranges | Default |
| --- | --- | --- |
| Gain | -60 to +24 dB; separate mute | 0 dB, unmuted |
| Graphic EQ | 31.5/63/125/250/500/1k/2k/4k/8k/16k Hz, ±18 dB | Flat |
| Parametric EQ | 20–20,000 Hz; Q 0.1–20; gain ±24 dB; pass slope 12/24 dB per octave | All eight bands disabled |
| Compressor | threshold -60–0 dBFS; ratio 1–20; attack 0.1–200 ms; release 10–2,000 ms; knee 0–24 dB; makeup 0–24 dB | -18 dBFS, 3:1, 10 ms, 150 ms, 6 dB knee, 0 dB makeup |
| Gate/expander | threshold -80–0 dBFS; hysteresis 0–12 dB; ratio 1–20; range 0–80 dB; attack 0.1–100 ms; hold 0–1,000 ms; release 10–2,000 ms | -45 dBFS, 3 dB, 4:1, 60 dB range, 5/50/150 ms |
| Limiter | ceiling -12–0 dBFS; lookahead 0–10 ms; release 10–1,000 ms | -1 dBFS, 5 ms, 100 ms |
| Delay | 0–1,000 ms | 0 ms |
| Pitch | semitone and cent ranges above | 0, 0 |

Changing the gate's `range` to 0 removes attenuation but retains state metering. Compressor ratio 1:1 is neutral except for makeup gain. Stereo detector linking avoids channel image movement. The precise transfer functions, detector RMS/peak choice, and knee/hysteresis equations must be documented with reference vectors in M04 before acceptance.

## Verification

Test impulse/frequency response, step response, DC/silence, clipping, automation, bypass latency, and long-run numerical stability. Flat EQ shall null against the input within the numeric tolerance in [14](14-quality.md). Verify notch attenuation at both hum presets and compressor/gate transfer curves across thresholds. Compare dry and processed branches to prove isolation. Exercise plugin scan crash, runtime crash, hang, mismatched channel layouts, editor resizing, state save/reload, and dynamic latency. A plugin that fails is an explicit unsupported fixture, not evidence that all VST3 effects work.

Sources: [Cockos ReaPlugs page](https://www.reaper.fm/reaplugs/) documents the separately distributed effects suite; [Steinberg licensing FAQ](https://steinbergmedia.github.io/vst3_dev_portal/pages/FAQ/Licensing.html) distinguishes VST3 and VST2 terms. These inform the compatibility boundary, not a legal determination about any particular third-party binary.
