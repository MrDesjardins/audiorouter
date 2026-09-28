//! Opt-in private speech qualification; never embeds voice fixtures in source.
use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::DecodedAudio;
use audiorouter_engine::{
    AudioBlock, RuntimeGeneration, compile_session_at_sample_rate_with_plugins_and_audio,
};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};

fn session(kind: NodeKind, parameters: Value) -> Session {
    let node = |id: &str, kind, ports| Node {
        id: EntityId::new(id),
        kind,
        name: id.into(),
        type_version: 1,
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports,
    };
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 1,
    };
    let edge = |id: &str, from: &str, to: &str| Edge {
        id: EntityId::new(id),
        source_node: EntityId::new(from),
        source_port: "out".into(),
        destination_node: EntityId::new(to),
        destination_port: "in".into(),
        matrix: vec![1.0],
        enabled: true,
    };
    let mut session = Session {
        id: EntityId::new("private-voice-tools"),
        name: "Private voice qualification".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            node(
                "source",
                NodeKind::PhysicalInput,
                vec![port("out", PortDirection::Output)],
            ),
            node(
                "tool",
                kind,
                vec![
                    port("in", PortDirection::Input),
                    port("out", PortDirection::Output),
                ],
            ),
            node(
                "sink",
                NodeKind::PhysicalOutput,
                vec![port("in", PortDirection::Input)],
            ),
        ],
        edges: vec![edge("in", "source", "tool"), edge("out", "tool", "sink")],
    };
    session.nodes[1].parameters = serde_json::from_value(parameters).unwrap();
    session
}

fn run(
    kind: NodeKind,
    parameters: Value,
    input: &[f32],
    rate: u32,
    assets: &HashMap<String, Arc<DecodedAudio>>,
) -> Vec<f32> {
    let graph = compile_session_at_sample_rate_with_plugins_and_audio(
        &session(kind, parameters),
        RuntimeGeneration::new(1),
        rate,
        &Default::default(),
        assets,
    )
    .unwrap();
    let mut output = Vec::with_capacity(input.len());
    for chunk in input.chunks(128) {
        let mut block = AudioBlock::new(1, chunk.len()).unwrap();
        block.channel_mut(0).unwrap().copy_from_slice(chunk);
        graph.process(&mut block);
        output.extend_from_slice(block.channel(0).unwrap());
    }
    assert!(
        output.iter().all(|sample| sample.is_finite()),
        "{kind:?}: invalid samples"
    );
    output
}
fn rms(samples: &[f32]) -> f64 {
    (samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / samples.len() as f64)
        .sqrt()
}
fn db(ratio: f64) -> f64 {
    20.0 * ratio.max(1e-12).log10()
}
fn error(a: &[f32], b: &[f32], delay: usize) -> f64 {
    let count = a.len() - delay;
    (a[delay..]
        .iter()
        .zip(&b[..count])
        .map(|(a, b)| f64::from(a - b).powi(2))
        .sum::<f64>()
        / count as f64)
        .sqrt()
}
// Sparse windowed DFT band probes quantify tone independently of total level.
fn bands(samples: &[f32], rate: u32) -> [f64; 3] {
    let mut power = [0.0_f64; 3];
    for chunk in samples.chunks(2048).filter(|chunk| chunk.len() >= 1024) {
        for (band, frequencies) in [
            &[93.75, 140.625, 234.375, 328.125][..],
            &[703.125, 984.375, 1406.25][..],
            &[2531.25, 3281.25, 4218.75, 5625.0][..],
        ]
        .iter()
        .enumerate()
        {
            for frequency in *frequencies {
                let w = std::f64::consts::TAU * frequency / f64::from(rate);
                let (mut re, mut im) = (0.0, 0.0);
                for (n, sample) in chunk[..1024].iter().enumerate() {
                    let window = 0.5 - 0.5 * (std::f64::consts::TAU * n as f64 / 1023.0).cos();
                    re += f64::from(*sample) * window * (w * n as f64).cos();
                    im += f64::from(*sample) * window * (w * n as f64).sin();
                }
                power[band] += re * re + im * im;
            }
        }
    }
    power.map(|power| power.sqrt())
}
fn write_wav(path: &std::path::Path, samples: &[f32], rate: u32, scale: f32) {
    let size = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(size as usize + 44);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(size + 36).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&size.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(
            &(((*sample * scale).clamp(-0.99, 0.99) * 32767.0).round() as i16).to_le_bytes(),
        );
    }
    std::fs::write(path, bytes).unwrap();
}

#[test]
#[ignore = "private local voice sample; explicit opt-in required"]
fn voice_sample_configurations_have_their_declared_effects() {
    let input_path = std::env::var("AUDIOROUTER_VOICE_SAMPLE").expect("approved local sample path");
    let bytes = std::fs::read(input_path).unwrap();
    assert!(bytes.len() < 2_000_000);
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 1);
    let rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    assert_eq!(rate, 48_000);
    let voice = bytes[44..]
        .chunks_exact(2)
        .map(|sample| f32::from(i16::from_le_bytes(sample.try_into().unwrap())) / 32768.0)
        .collect::<Vec<_>>();
    let output_dir =
        std::env::temp_dir().join(format!("audiorouter-tool-voice-{}", std::process::id()));
    std::fs::create_dir(&output_dir).unwrap();
    let dry_bands = bands(&voice, rate);
    let dry_rms = rms(&voice);
    assert!(dry_rms > 1e-6);
    let mut results = Vec::new();
    let mut assets = HashMap::new();
    assets.insert(
        "voice-ir".into(),
        Arc::new(DecodedAudio {
            channels: 1,
            sample_rate_hz: rate,
            samples: Arc::from([0.5_f32, 0.5]),
        }),
    );
    let mut save = |name: &str, output: &[f32]| {
        let spectral = bands(output, rate);
        let gains = std::array::from_fn::<_, 3, _>(|i| db(spectral[i] / dry_bands[i]));
        let peak = output
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        let scale = (dry_rms / rms(output).max(1e-9)) as f32;
        write_wav(
            &output_dir.join(format!("{name}.wav")),
            output,
            rate,
            scale.min(0.8 / peak.max(1e-9)),
        );
        let result =
            json!({"case":name,"rmsChangeDb":db(rms(output)/dry_rms),"bandsDb":gains,"peak":peak});
        eprintln!("{result}");
        results.push(result);
        gains
    };
    save("flat", &voice);
    for filter in [
        "peaking",
        "lowShelf",
        "highShelf",
        "lowPass",
        "highPass",
        "bandPass",
        "allPass",
        "notch",
    ] {
        let output = run(
            NodeKind::ParametricEq,
            json!({"band0Enabled":true,"band0Type":filter,"band0FrequencyHz":1000,"band0GainDb":12,"band0Q":0.707}),
            &voice,
            rate,
            &assets,
        );
        save(&format!("advanced-eq-{filter}"), &output);
        assert!(
            error(&output, &voice, 0) > 1e-6,
            "{filter} modifies the signal (all pass changes phase)"
        );
    }
    for (name, bass, treble) in [
        ("bass-boost", 12.0, 0.0),
        ("bass-cut", -12.0, 0.0),
        ("treble-boost", 0.0, 12.0),
        ("treble-cut", 0.0, -12.0),
    ] {
        let output = run(
            NodeKind::BassTreble,
            json!({"bassDb":bass,"trebleDb":treble}),
            &voice,
            rate,
            &assets,
        );
        save(name, &output);
    }
    let hum = voice
        .iter()
        .enumerate()
        .map(|(i, s)| s + 0.05 * (std::f32::consts::TAU * 60.0 * i as f32 / rate as f32).sin())
        .collect::<Vec<_>>();
    let output = run(
        NodeKind::Dehum,
        json!({"frequencyHz":60,"harmonics":1,"amountPercent":100}),
        &hum,
        rate,
        &assets,
    );
    save("dehum", &output);
    assert!(
        error(&output[4800..], &voice[4800..], 0) < error(&hum[4800..], &voice[4800..], 0) * 0.5,
        "remove added mains hum"
    );
    let mut clicks = voice.clone();
    for i in (4800..clicks.len() - 100).step_by(12000) {
        clicks[i] += 0.8;
    }
    let output = run(
        NodeKind::Declick,
        json!({"thresholdPercent":50}),
        &clicks,
        rate,
        &assets,
    );
    save("declick", &output);
    assert!(
        error(&output, &voice, 64) < error(&clicks, &voice, 0) * 0.5,
        "repair injected impulses"
    );
    let output = run(
        NodeKind::FirFilter,
        json!({"mediaId":"voice-ir","wetPercent":100}),
        &voice,
        rate,
        &assets,
    );
    save("fir-configured", &output);
    assert!(
        error(&output, &voice, 128) > dry_rms * 0.1,
        "configured convolution differs from dry voice"
    );
    let mut seed = 7_u32;
    let noise = (0..voice.len())
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            ((seed >> 8) as f32 / 16777216.0 * 2.0 - 1.0) * 0.01
        })
        .collect::<Vec<_>>();
    let noisy = voice
        .iter()
        .zip(&noise)
        .map(|(v, n)| v + n)
        .collect::<Vec<_>>();
    for kind in [
        NodeKind::Denoise,
        NodeKind::SpectralGate,
        NodeKind::SpeechDenoise,
    ] {
        let parameters = if kind == NodeKind::SpeechDenoise {
            json!({"strengthPercent":100})
        } else {
            let graph = compile_session_at_sample_rate_with_plugins_and_audio(
                &session(kind, json!({"learning":true})),
                RuntimeGeneration::new(1),
                rate,
                &Default::default(),
                &assets,
            )
            .unwrap();
            for chunk in noise.chunks(128) {
                let mut block = AudioBlock::new(1, chunk.len()).unwrap();
                block.channel_mut(0).unwrap().copy_from_slice(chunk);
                graph.process(&mut block);
            }
            let profile = graph
                .noise_profile_for_node(&EntityId::new("tool"))
                .expect("learned profile");
            if kind == NodeKind::Denoise {
                json!({"noiseProfile":profile,"reductionPercent":100,"floorPercent":0})
            } else {
                json!({"noiseProfile":profile,"thresholdDb":3,"reductionDb":40})
            }
        };
        let output = run(kind, parameters.clone(), &noisy, rate, &assets);
        save(&format!("{kind:?}-noisy-voice"), &output);
        let suppressed = run(kind, parameters, &noise, rate, &assets);
        assert!(
            rms(&suppressed[rate as usize..]) < rms(&noise[rate as usize..]) * 0.6,
            "{kind:?}: suppress steady noise"
        );
        assert!(
            rms(&output) > dry_rms * 0.6,
            "{kind:?}: preserve voice level"
        );
    }
    for (name, bass, treble) in [
        ("wide-bass", 12.0, 0.0),
        ("wide-treble", 0.0, 12.0),
        ("warm-radio", 12.0, -12.0),
        ("thin-bright", -12.0, 12.0),
    ] {
        let output = run(
            NodeKind::BassTreble,
            json!({"bassDb":bass,"trebleDb":treble}),
            &voice,
            rate,
            &assets,
        );
        let spectral = save(name, &output);
        if name == "warm-radio" {
            assert!(
                spectral[0] - spectral[2] > 12.0,
                "expected strong warm/dark tilt"
            );
        }
        if name == "thin-bright" {
            assert!(
                spectral[2] - spectral[0] > 12.0,
                "expected strong thin/bright tilt"
            );
        }
    }
    for (name, kind, parameters, delay) in [
        ("gain-minus12", NodeKind::Gain, json!({"gainDb":-12}), 0),
        ("volume25", NodeKind::Volume, json!({"percent":25}), 0),
        ("mute", NodeKind::Mute, json!({"muted":true}), 0),
        ("meter", NodeKind::Meter, json!({}), 0),
        ("delay100", NodeKind::Delay, json!({"delayMs":100}), 4800),
        (
            "timeshift-live",
            NodeKind::TimeShift,
            json!({"bufferSeconds":10}),
            0,
        ),
        (
            "compressor",
            NodeKind::Compressor,
            json!({"thresholdDb":-45,"ratio":10,"attackMs":1,"releaseMs":30,"kneeDb":0}),
            0,
        ),
        (
            "gate-closed",
            NodeKind::Gate,
            json!({"thresholdDb":0,"rangeDb":60,"attackMs":1,"holdMs":0,"releaseMs":10}),
            0,
        ),
        (
            "limiter",
            NodeKind::Limiter,
            json!({"ceilingDb":-12,"lookaheadMs":5}),
            240,
        ),
        (
            "graphic-bright",
            NodeKind::GraphicEq,
            json!({"band2Db":-12,"band3Db":-12,"band4Db":-12,"band7Db":12,"band8Db":12}),
            0,
        ),
        ("pitch-up", NodeKind::Pitch, json!({"semitones":7}), 0),
        (
            "speech-denoise",
            NodeKind::SpeechDenoise,
            json!({"strengthPercent":100}),
            1024,
        ),
        ("fir-unconfigured", NodeKind::FirFilter, json!({}), 0),
    ] {
        let input = if kind == NodeKind::Limiter {
            voice.iter().map(|sample| sample * 8.0).collect::<Vec<_>>()
        } else {
            voice.clone()
        };
        let output = run(kind, parameters, &input, rate, &assets);
        save(name, &output);
        match kind {
            NodeKind::Gain | NodeKind::Volume => {
                assert!((db(rms(&output) / dry_rms) + 12.0).abs() < 0.2)
            }
            NodeKind::Mute => assert_eq!(rms(&output), 0.0),
            NodeKind::Meter | NodeKind::TimeShift | NodeKind::Delay | NodeKind::FirFilter => {
                assert!(
                    error(&output, &voice, delay) < 1e-6,
                    "{name} transparency/time alignment"
                )
            }
            NodeKind::Compressor => assert!(rms(&output) < dry_rms * 0.6),
            NodeKind::Gate => assert!(rms(&output) < dry_rms * 0.03),
            NodeKind::Limiter => assert!(output.iter().all(|sample| sample.abs() < 0.252)),
            NodeKind::SpeechDenoise => assert!(
                error(&output, &voice, delay) < dry_rms * 0.1,
                "clean speech should be preserved"
            ),
            _ => assert!(
                error(&output, &voice, delay) > dry_rms * 0.05,
                "{name} must have a measurable effect"
            ),
        }
    }
    // Source playback must reproduce the decoded voice, rather than only compile.
    assets.insert(
        "voice".into(),
        Arc::new(DecodedAudio {
            channels: 1,
            sample_rate_hz: rate,
            samples: voice.clone().into(),
        }),
    );
    let mut file_session = session(NodeKind::Meter, json!({}));
    file_session.nodes[0].kind = NodeKind::AudioFile;
    file_session.nodes[0].parameters = serde_json::from_value(json!({"mediaId":"voice"})).unwrap();
    let graph = compile_session_at_sample_rate_with_plugins_and_audio(
        &file_session,
        RuntimeGeneration::new(1),
        rate,
        &Default::default(),
        &assets,
    )
    .unwrap();
    graph
        .audio_file_source_for_node(&EntityId::new("source"))
        .unwrap()
        .play();
    let mut played = Vec::new();
    for chunk in voice.chunks(128) {
        let mut block = AudioBlock::new(1, chunk.len()).unwrap();
        graph.process(&mut block);
        played.extend_from_slice(block.channel(0).unwrap());
    }
    assert!(
        error(&played, &voice, 0) < 1e-6,
        "Audio file preserves the voice"
    );
    save("audio-file", &played);
    for (kind, selected, factor) in [
        (NodeKind::Mixer, "a", 1.25),
        (NodeKind::InputSwitch, "a", 1.0),
        (NodeKind::InputSwitch, "b", 0.25),
    ] {
        let mut route = session(kind, json!({}));
        let mut other = route.nodes[0].clone();
        other.id = EntityId::new("other");
        route.nodes.push(other);
        let mut edge = route.edges[0].clone();
        edge.id = EntityId::new("other-in");
        edge.source_node = EntityId::new("other");
        if kind == NodeKind::InputSwitch {
            route.nodes[1].ports[0].name = "a".into();
            let mut port = route.nodes[1].ports[0].clone();
            port.name = "b".into();
            route.nodes[1].ports.push(port);
            route.edges[0].destination_port = "a".into();
            edge.destination_port = "b".into();
            route.nodes[1]
                .parameters
                .insert("selected".into(), json!(selected));
        }
        route.edges.push(edge);
        let graph =
            audiorouter_engine::compile_mixer_fanout_session(&route, RuntimeGeneration::new(1))
                .unwrap();
        let mut result = Vec::new();
        for chunk in voice.chunks(128) {
            let mut a = AudioBlock::new(1, chunk.len()).unwrap();
            a.channel_mut(0).unwrap().copy_from_slice(chunk);
            let mut b = AudioBlock::new(1, chunk.len()).unwrap();
            for (s, v) in b.channel_mut(0).unwrap().iter_mut().zip(chunk) {
                *s = *v * 0.25;
            }
            let mut scratch = AudioBlock::new(1, chunk.len()).unwrap();
            let mut out = AudioBlock::new(1, chunk.len()).unwrap();
            graph
                .process(&[a, b], &mut scratch, &mut [&mut out])
                .unwrap();
            result.extend_from_slice(out.channel(0).unwrap());
        }
        let expected = voice.iter().map(|s| s * factor).collect::<Vec<_>>();
        assert!(
            error(&result, &expected, 0) < 1e-6,
            "{kind:?} selects/mixes voice correctly"
        );
        save(&format!("{kind:?}-{selected}"), &result);
    }
    let graph = compile_session_at_sample_rate_with_plugins_and_audio(
        &session(NodeKind::TimeShift, json!({"bufferSeconds":10})),
        RuntimeGeneration::new(1),
        rate,
        &Default::default(),
        &assets,
    )
    .unwrap();
    let state = audiorouter_engine::time_shift_state("private-voice-tools", "tool").unwrap();
    for chunk in voice.chunks(128) {
        let mut block = AudioBlock::new(1, chunk.len()).unwrap();
        block.channel_mut(0).unwrap().copy_from_slice(chunk);
        graph.process(&mut block);
    }
    state.post(audiorouter_dsp::timeshift::TimeShiftCommand::Pause);
    let mut block = AudioBlock::new(1, 128).unwrap();
    block.channel_mut(0).unwrap().copy_from_slice(&voice[..128]);
    graph.process(&mut block);
    assert_eq!(rms(block.channel(0).unwrap()), 0.0);
    assert!(state.status().paused);
    state.post(audiorouter_dsp::timeshift::TimeShiftCommand::Back);
    graph.process(&mut block);
    assert!(state.status().delay_seconds > 4.0);
    state.post(audiorouter_dsp::timeshift::TimeShiftCommand::Live);
    graph.process(&mut block);
    assert!(!state.status().paused);
    assert_eq!(state.status().delay_seconds, 0.0);
    std::fs::write(
        output_dir.join("metrics.json"),
        serde_json::to_vec_pretty(&results).unwrap(),
    )
    .unwrap();
    eprintln!(
        "Private, loudness-matched audition files: {}",
        output_dir.display()
    );
}
