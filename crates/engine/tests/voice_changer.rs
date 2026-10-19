//! Fun Voice Changer (DSP-20) through the engine: fixed delay, state carried
//! across live replacement only, fresh state after bypass, preset levels and
//! telemetry. Synthetic audio only.
use audiorouter_domain::voice_changer::{voice_preset, voice_preset_parameters, VOICE_PRESETS};
use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_dsp::voice_changer::VOICE_CHANGER_LATENCY_SAMPLES;
use audiorouter_engine::{compile_session, AudioBlock, RuntimeGeneration, RuntimeProcessor};
use serde_json::{json, Value};

const FRAMES: usize = 128;

fn route(parameters: Value, bypass: bool) -> Session {
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let node = |id: &str, kind, ports| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports,
    };
    let edge = |id: &str, from: &str, to: &str| Edge {
        id: EntityId::new(id),
        source_node: EntityId::new(from),
        source_port: "out".into(),
        destination_node: EntityId::new(to),
        destination_port: "in".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    };
    let mut voice = node(
        "voice",
        NodeKind::FunVoiceChanger,
        vec![
            port("in", PortDirection::Input),
            port("out", PortDirection::Output),
        ],
    );
    voice.parameters = serde_json::from_value(parameters).unwrap();
    voice.bypass = bypass;
    Session {
        id: EntityId::new("voice-changer"),
        name: "Voice changer".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            node(
                "source",
                NodeKind::PhysicalInput,
                vec![port("out", PortDirection::Output)],
            ),
            voice,
            node(
                "sink",
                NodeKind::PhysicalOutput,
                vec![port("in", PortDirection::Input)],
            ),
        ],
        edges: vec![edge("in", "source", "voice"), edge("out", "voice", "sink")],
    }
}

fn preset(id: &str) -> Value {
    Value::Object(voice_preset_parameters(voice_preset(id).unwrap()))
}

/// A voice-like test signal: harmonics of a slowly gliding 140 Hz with a
/// vowel-shaped envelope and syllable-rate level changes, peak about 0.3.
fn voice(frames: usize) -> Vec<f32> {
    let mut phase = 0.0_f64;
    (0..frames)
        .map(|frame| {
            let t = frame as f64 / 48_000.0;
            let f0 = 140.0 + 15.0 * (std::f64::consts::TAU * 0.7 * t).sin();
            phase += f0 / 48_000.0;
            let mut value = 0.0;
            for harmonic in 1..=30 {
                let frequency = f0 * harmonic as f64;
                // Formant-like weights around 700, 1200 and 2600 Hz.
                let weight = [(700.0, 200.0), (1_200.0, 250.0), (2_600.0, 400.0)]
                    .iter()
                    .map(|(center, width)| (-((frequency - center) / width).powi(2)).exp())
                    .sum::<f64>()
                    + 0.3 / harmonic as f64;
                value += weight * (std::f64::consts::TAU * phase * harmonic as f64).sin();
            }
            let syllable = 0.65 + 0.35 * (std::f64::consts::TAU * 3.0 * t).sin();
            (0.08 * value * syllable) as f32
        })
        .collect()
}

fn process(processor: &RuntimeProcessor, input: &[f32]) -> Vec<f32> {
    let mut output = Vec::with_capacity(input.len());
    for chunk in input.chunks(FRAMES) {
        let mut block = AudioBlock::new(2, FRAMES).unwrap();
        for channel in 0..2 {
            block.channel_mut(channel).unwrap()[..chunk.len()].copy_from_slice(chunk);
        }
        processor.process(&mut block);
        output.extend_from_slice(&block.channel(0).unwrap()[..chunk.len()]);
    }
    output
}

fn publish(processor: &RuntimeProcessor, parameters: Value, bypass: bool) {
    processor
        .publish(compile_session(&route(parameters, bypass), RuntimeGeneration::new(1)).unwrap());
}

fn rms(samples: &[f32]) -> f64 {
    (samples
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        / samples.len().max(1) as f64)
        .sqrt()
}

#[test]
fn neutral_node_passes_both_channels_through_after_the_declared_delay() {
    let processor = RuntimeProcessor::default();
    publish(&processor, json!({}), false);
    let input = voice(24_000);
    let output = process(&processor, &input);
    for frame in VOICE_CHANGER_LATENCY_SAMPLES..input.len() {
        let expected = input[frame - VOICE_CHANGER_LATENCY_SAMPLES];
        assert!(
            (output[frame] - expected).abs() < 1.0e-6,
            "frame {frame}: {} vs {expected}",
            output[frame]
        );
    }
}

#[test]
fn a_live_preset_change_keeps_the_voice_playing() {
    let input = voice(96_000);
    // 20 ms windows: longer than a glottal period, so a window is quiet only
    // when the output really drops out.
    let windows = |samples: &[f32]| -> Vec<f64> { samples.chunks(960).map(rms).collect() };
    let processor = RuntimeProcessor::default();
    publish(&processor, preset("deep"), false);
    let before = process(&processor, &input[..48_000]);
    // A parameter-only edit while playing: compiled anew, then published.
    publish(&processor, preset("chipmunk"), false);
    let after = process(&processor, &input[48_000..]);
    let steady = rms(&before[24_000..]);
    for (index, level) in windows(&after[..9_600]).iter().enumerate() {
        assert!(
            *level > steady * 0.3,
            "window {index} after the change at {level:.4} (steady {steady:.4})"
        );
    }
    // The check is sensitive: a fresh processor at that moment starts with
    // about 21 ms of silence.
    let fresh = RuntimeProcessor::default();
    publish(&fresh, preset("chipmunk"), false);
    let restarted = process(&fresh, &input[48_000..]);
    assert!(windows(&restarted[..9_600])[0] < steady * 0.05);
}

#[test]
fn returning_from_bypass_starts_with_no_stale_audio() {
    let processor = RuntimeProcessor::default();
    publish(&processor, preset("cave"), false);
    process(&processor, &voice(48_000));
    publish(&processor, preset("cave"), true);
    process(&processor, &vec![0.0; 4_800]);
    publish(&processor, preset("cave"), false);
    // Silence in: a carried state would still ring with the cave's tail.
    let output = process(&processor, &vec![0.0; 9_600]);
    assert!(output.iter().all(|value| *value == 0.0));
}

#[test]
fn independently_compiled_graphs_never_share_a_processor() {
    let first = RuntimeProcessor::default();
    let second = RuntimeProcessor::default();
    publish(&first, preset("robot"), false);
    publish(&second, preset("robot"), false);
    let input = voice(24_000);
    let alone = process(&first, &input);
    // The second graph sees silence; its output must not depend on the
    // first graph's audio, and the first one's output is unaffected.
    let silent = process(&second, &vec![0.0; 24_000]);
    assert!(silent.iter().all(|value| *value == 0.0));
    let fresh = RuntimeProcessor::default();
    publish(&fresh, preset("robot"), false);
    assert_eq!(alone, process(&fresh, &input));
}

#[test]
fn every_preset_keeps_the_voice_level_within_three_db() {
    let input = voice(96_000);
    let reference = rms(&input[48_000..]);
    for preset_spec in &VOICE_PRESETS {
        let processor = RuntimeProcessor::default();
        publish(&processor, preset(preset_spec.id), false);
        let output = process(&processor, &input);
        let ratio_db = 20.0 * (rms(&output[48_000..]) / reference).log10();
        assert!(
            ratio_db.abs() <= 3.0,
            "{}: {ratio_db:+.1} dB",
            preset_spec.id
        );
        assert!(output.iter().all(|value| value.is_finite()));
    }
}

#[test]
fn telemetry_reports_the_detected_note_while_pitch_presets_run() {
    let processor = RuntimeProcessor::default();
    publish(&processor, preset("pitchCorrectHard"), false);
    let id = EntityId::new("voice");
    assert!(
        !processor
            .voice_changer_for_node(&id)
            .unwrap()
            .telemetry()
            .unwrap()
            .voiced
    );
    process(&processor, &voice(48_000));
    let reading = processor
        .voice_changer_for_node(&id)
        .unwrap()
        .telemetry()
        .unwrap();
    assert!(reading.voiced, "{reading:?}");
    assert!((120.0..170.0).contains(&reading.f0_hz), "{reading:?}");
}
