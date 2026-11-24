//! Q4.3 bounded fuzzing of two engine boundaries that take untrusted input:
//!
//! * the audio-file decoder behind the media upload path (`decode_audio_bytes`,
//!   which hands attacker-controlled bytes to the third-party symphonia
//!   decoders), and
//! * the graph compiler (`compile_native_paths_with_plugins_and_audio`), which
//!   must return `Ok` or a structured `GraphCompileError` for any session a
//!   client can store, never panic, and never allocate from an unchecked size.
//!
//! Fixed seeds and counts; `AUDIOROUTER_FUZZ_SCALE` lengthens a local run.
//! Complements `source_combinations.rs` and `tool_combinations.rs`, which
//! cover valid layouts exhaustively; these cover malformed and extreme ones.
use std::collections::HashMap;
use std::sync::Arc;

use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{
    compile_native_paths_with_plugins_and_audio, decode_audio_bytes, AudioBlock,
    AudioFileDecodeError, DecodedAudio, GraphCompileError, RuntimeGeneration,
};
use serde_json::{json, Value};

#[path = "../../../tests/support/fuzz.rs"]
mod fuzz;

#[global_allocator]
static ALLOCATOR: fuzz::TrackingAllocator = fuzz::TrackingAllocator;

const SEED: u64 = 0x454E_4749_4E45_2121; // "ENGINE!!"
const GRAPH_RATE_HZ: u32 = 48_000;
/// The decoder's own output ceiling (64 MiB of samples) plus working copies.
const DECODE_ALLOCATION_CAP_BYTES: usize = 256 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Audio file decoding
// ---------------------------------------------------------------------------

/// A canonical little-endian WAV file.
fn wav(format_tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
    let block_align = channels.wrapping_mul(bits / 8);
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&format_tag.to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&rate.to_le_bytes());
    fmt.extend_from_slice(&rate.wrapping_mul(u32::from(block_align)).to_le_bytes());
    fmt.extend_from_slice(&block_align.to_le_bytes());
    fmt.extend_from_slice(&bits.to_le_bytes());
    let mut body = b"WAVE".to_vec();
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    body.extend_from_slice(&fmt);
    body.extend_from_slice(b"data");
    body.extend_from_slice(&(data.len() as u32).to_le_bytes());
    body.extend_from_slice(data);
    let mut file = b"RIFF".to_vec();
    file.extend_from_slice(&(body.len() as u32).to_le_bytes());
    file.extend_from_slice(&body);
    file
}

fn tone(frames: usize, channels: usize, encode: impl Fn(f32) -> Vec<u8>) -> Vec<u8> {
    let mut data = Vec::new();
    for frame in 0..frames {
        let value = (frame as f32 * 0.1).sin() * 0.5;
        for _ in 0..channels {
            data.extend_from_slice(&encode(value));
        }
    }
    data
}

fn seed_wavs() -> Vec<Vec<u8>> {
    let pcm16 = |value: f32| ((value * 32_767.0) as i16).to_le_bytes().to_vec();
    let float32 = |value: f32| value.to_le_bytes().to_vec();
    let pcm24 = |value: f32| ((value * 8_388_607.0) as i32).to_le_bytes()[..3].to_vec();
    let pcm8 = |value: f32| vec![(128.0 + value * 127.0) as u8];
    vec![
        wav(1, 1, 48_000, 16, &tone(240, 1, pcm16)),
        wav(1, 2, 44_100, 16, &tone(200, 2, pcm16)),
        wav(3, 2, 48_000, 32, &tone(160, 2, float32)),
        wav(1, 1, 22_050, 24, &tone(120, 1, pcm24)),
        wav(1, 1, 8_000, 8, &tone(80, 1, pcm8)),
    ]
}

/// A FLAC signature plus a STREAMINFO block and noise where frames belong: it
/// reaches the FLAC probe and metadata parser, then must fail cleanly.
fn seed_flac() -> Vec<u8> {
    let mut file = b"fLaC".to_vec();
    file.push(0x80); // last metadata block, type 0 (STREAMINFO)
    file.extend_from_slice(&[0, 0, 34]);
    let mut info = Vec::new();
    info.extend_from_slice(&4096_u16.to_be_bytes()); // min block size
    info.extend_from_slice(&4096_u16.to_be_bytes()); // max block size
    info.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // min/max frame size
                                                 // 20-bit rate (48000), 3-bit channels-1 (1), 5-bit bits-1 (15), 36-bit samples.
    let packed: u64 = (48_000_u64 << 44) | (1_u64 << 41) | (15_u64 << 36) | 1_000;
    info.extend_from_slice(&packed.to_be_bytes());
    info.extend_from_slice(&[0_u8; 16]); // MD5
    file.extend_from_slice(&info);
    file.extend_from_slice(&[0xFF, 0xF8, 0xC9, 0x18, 0x00, 0x00]);
    file.extend_from_slice(&[0x55; 64]);
    file
}

/// MPEG-1 Layer III frame headers followed by noise (128 kbit/s, 44.1 kHz).
fn seed_mp3() -> Vec<u8> {
    let mut file = b"ID3\x03\x00\x00\x00\x00\x00\x00".to_vec();
    for _ in 0..4 {
        file.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        file.extend_from_slice(&[0x33; 413]);
    }
    file
}

fn check_decode(bytes: &[u8], extension: &str) -> Result<DecodedAudio, AudioFileDecodeError> {
    let (result, largest) =
        fuzz::measure_allocation(|| decode_audio_bytes(bytes.to_vec(), extension, GRAPH_RATE_HZ));
    assert!(
        largest <= DECODE_ALLOCATION_CAP_BYTES,
        "decoder requested {largest} bytes for {} input bytes ({extension}): {}",
        bytes.len(),
        fuzz::hex(bytes)
    );
    if let Ok(audio) = &result {
        assert!((1..=2).contains(&audio.channels), "{}", fuzz::hex(bytes));
        assert_eq!(audio.sample_rate_hz, GRAPH_RATE_HZ);
        assert!(!audio.samples.is_empty() && audio.samples.len() % audio.channels == 0);
        assert!(audio.samples.len() * 4 <= 64 * 1024 * 1024);
        assert!(
            audio
                .samples
                .iter()
                .all(|sample| sample.is_finite() && (-1.0..=1.0).contains(sample)),
            "decoded a non-finite or out-of-range sample: {}",
            fuzz::hex(bytes)
        );
    }
    result
}

#[test]
fn seed_audio_decodes_and_the_extension_hint_cannot_break_it() {
    for seed in seed_wavs() {
        for extension in ["wav", "WAV", ".wav", "", "mp3", "flac", "ogg", "wav\0x"] {
            let audio = check_decode(&seed, extension).expect("a valid WAV decodes");
            assert!(audio.samples.len() >= 80);
        }
    }
    // The FLAC and MP3 seeds are malformed on purpose: an error, not a panic.
    for seed in [seed_flac(), seed_mp3()] {
        for extension in ["flac", "mp3", "wav", ""] {
            let _ = check_decode(&seed, extension);
        }
    }
}

#[test]
fn mutated_audio_files_never_panic_and_decode_within_bounds() {
    let mut seeds = seed_wavs();
    seeds.push(seed_flac());
    seeds.push(seed_mp3());
    let extensions = ["wav", "mp3", "flac", ""];
    let mut decoded = 0_usize;
    let mut refused = 0_usize;
    fuzz::run("engine/decode-audio", SEED, 3_000, |_, rng| {
        let seed = rng.pick(&seeds).clone();
        let input = fuzz::mutate(rng, &seed, 16 * 1024);
        let extension = *rng.pick(&extensions);
        match check_decode(&input, extension) {
            Ok(_) => decoded += 1,
            Err(_) => refused += 1,
        }
    });
    assert!(decoded > 100, "only {decoded} mutated files decoded");
    assert!(refused > 100, "only {refused} mutated files were refused");
}

/// Regression (found by the fuzz run above, 2026-11-24): symphonia's WAV
/// `fmt ` reader multiplies the channel count by the sample width in `u16`, so a
/// channel count of 32768 overflowed and panicked in debug builds. The file must
/// be refused as malformed, in every build profile.
#[test]
fn a_wav_whose_channel_count_overflows_the_block_align_is_refused_not_a_panic() {
    for channels in [0x8000_u16, 0xFFFF, 0x4001, 0x2AAB] {
        let file = wav(1, channels, 48_000, 16, &[0, 0, 0, 0]);
        assert!(
            matches!(
                check_decode(&file, "wav"),
                Err(AudioFileDecodeError::UnsupportedOrMalformed
                    | AudioFileDecodeError::UnsupportedChannels)
            ),
            "{channels:#x} channels"
        );
    }
}

/// The fields of a WAV header that size buffers: channel count, rate, bit
/// depth and the data length, set to edge values one at a time.
#[test]
fn wav_header_fields_at_their_extremes_are_refused_or_bounded() {
    let base = seed_wavs().remove(1);
    // fmt chunk payload starts at 20; data length is the u32 at 40.
    let fields: [(&str, usize, usize); 6] = [
        ("format tag", 20, 2),
        ("channels", 22, 2),
        ("sample rate", 24, 4),
        ("byte rate", 28, 4),
        ("block align", 32, 2),
        ("bits per sample", 34, 2),
    ];
    for (_name, offset, width) in fields {
        for value in [
            0_u32,
            1,
            2,
            3,
            7,
            8,
            24,
            32,
            64,
            255,
            0xFFFF,
            0x7FFF_FFFF,
            u32::MAX,
        ] {
            let mut file = base.clone();
            file[offset..offset + width].copy_from_slice(&value.to_le_bytes()[..width]);
            let _ = check_decode(&file, "wav");
        }
    }
    for length in [0_u32, 1, 3, 4_096, 0x00FF_FFFF, 0x7FFF_FFFF, u32::MAX] {
        let mut file = base.clone();
        file[40..44].copy_from_slice(&length.to_le_bytes());
        let _ = check_decode(&file, "wav");
        // The RIFF size, too.
        let mut file = base.clone();
        file[4..8].copy_from_slice(&length.to_le_bytes());
        let _ = check_decode(&file, "wav");
    }
    // Every prefix of a valid file.
    for length in 0..base.len() {
        let _ = check_decode(&base[..length], "wav");
    }
}

// ---------------------------------------------------------------------------
// Graph compilation
// ---------------------------------------------------------------------------

fn port(name: &str, direction: PortDirection, channels: u8) -> Port {
    Port {
        name: name.into(),
        direction,
        channels,
    }
}

fn is_source(kind: NodeKind) -> bool {
    // Exhaustive on purpose (owner rule): a new NodeKind fails to compile
    // here until it is placed.
    match kind {
        NodeKind::PhysicalInput
        | NodeKind::ApplicationCapture
        | NodeKind::EndpointLoopback
        | NodeKind::VirtualRenderSource
        | NodeKind::TestSignal
        | NodeKind::AudioFile
        | NodeKind::NetworkReceive
        | NodeKind::Instrument
        | NodeKind::Soundboard => true,
        NodeKind::PhysicalOutput
        | NodeKind::VirtualCaptureSink
        | NodeKind::NetworkSend
        | NodeKind::Mixer
        | NodeKind::Gain
        | NodeKind::Mute
        | NodeKind::Meter
        | NodeKind::ParametricEq
        | NodeKind::Compressor
        | NodeKind::Gate
        | NodeKind::Limiter
        | NodeKind::Delay
        | NodeKind::GraphicEq
        | NodeKind::Pitch
        | NodeKind::Volume
        | NodeKind::BassTreble
        | NodeKind::Dehum
        | NodeKind::Declick
        | NodeKind::Denoise
        | NodeKind::SpeechDenoise
        | NodeKind::FirFilter
        | NodeKind::SpectralGate
        | NodeKind::TimeShift
        | NodeKind::Duck
        | NodeKind::FunVoiceChanger
        | NodeKind::Recorder
        | NodeKind::Plugin
        | NodeKind::InputSwitch
        | NodeKind::OutputSwitch => false,
    }
}

fn is_sink(kind: NodeKind) -> bool {
    match kind {
        NodeKind::PhysicalOutput | NodeKind::VirtualCaptureSink | NodeKind::NetworkSend => true,
        NodeKind::PhysicalInput
        | NodeKind::ApplicationCapture
        | NodeKind::EndpointLoopback
        | NodeKind::VirtualRenderSource
        | NodeKind::TestSignal
        | NodeKind::AudioFile
        | NodeKind::NetworkReceive
        | NodeKind::Instrument
        | NodeKind::Soundboard
        | NodeKind::Mixer
        | NodeKind::Gain
        | NodeKind::Mute
        | NodeKind::Meter
        | NodeKind::ParametricEq
        | NodeKind::Compressor
        | NodeKind::Gate
        | NodeKind::Limiter
        | NodeKind::Delay
        | NodeKind::GraphicEq
        | NodeKind::Pitch
        | NodeKind::Volume
        | NodeKind::BassTreble
        | NodeKind::Dehum
        | NodeKind::Declick
        | NodeKind::Denoise
        | NodeKind::SpeechDenoise
        | NodeKind::FirFilter
        | NodeKind::SpectralGate
        | NodeKind::TimeShift
        | NodeKind::Duck
        | NodeKind::FunVoiceChanger
        | NodeKind::Recorder
        | NodeKind::Plugin
        | NodeKind::InputSwitch
        | NodeKind::OutputSwitch => false,
    }
}

/// Parameter names the compiler reads, plus a few unknown ones.
const PARAMETER_KEYS: [&str; 41] = [
    "attackMs",
    "bufferSeconds",
    "busId",
    "ceilingDb",
    "cents",
    "channelMode",
    "delayMs",
    "durationMs",
    "executable",
    "fade",
    "frequencyHz",
    "gainDb",
    "harmonics",
    "holdMs",
    "hysteresisDb",
    "keyNodeId",
    "kneeDb",
    "learning",
    "levelDb",
    "lookaheadMs",
    "loop",
    "makeupDb",
    "mediaId",
    "muted",
    "noiseProfile",
    "percent",
    "processPolicy",
    "rangeDb",
    "ratio",
    "releaseMs",
    "selected",
    "semitones",
    "spatialMode",
    "strengthPercent",
    "thresholdDb",
    "thresholdPercent",
    "trigger",
    "bands",
    "slot1MediaId",
    "unknown",
    "sender",
];

fn extreme_value(rng: &mut fuzz::Rng) -> Value {
    match rng.below(14) {
        0 => json!(0),
        1 => json!(-1),
        2 => json!(1e308),
        3 => json!(-1e308),
        4 => json!(u64::MAX),
        5 => json!(i64::MIN),
        6 => json!(0.000_000_1),
        7 => json!(f64::MAX),
        8 => json!("off"),
        9 => json!(""),
        10 => json!(null),
        11 => json!([1, 2, 3]),
        12 => json!({"nested": {"deeper": [0]}}),
        _ => json!(true),
    }
}

fn plausible_value(rng: &mut fuzz::Rng) -> Value {
    match rng.below(5) {
        0 => json!(rng.below(200) as f64 / 10.0),
        1 => json!(rng.below(2_000)),
        2 => json!(-(rng.below(60) as f64)),
        3 => json!(rng.below(2) == 0),
        _ => json!(*rng.pick(&["mono", "left", "right", "off", "media-1", "game.exe"])),
    }
}

/// The settings a node of this kind needs to pass validation at all (the same
/// ones `source_combinations.rs` uses), so a case reaches the compiler.
fn required_parameters(kind: NodeKind) -> serde_json::Map<String, Value> {
    let parameters = match kind {
        NodeKind::AudioFile => json!({ "mediaId": "media-1" }),
        NodeKind::Soundboard => json!({ "slot1MediaId": "media-1" }),
        NodeKind::EndpointLoopback => json!({ "defaultRole": "console" }),
        NodeKind::VirtualRenderSource | NodeKind::VirtualCaptureSink => {
            json!({ "busId": "bus-1" })
        }
        NodeKind::NetworkSend => json!({ "host": "192.168.1.30", "port": 48000 }),
        NodeKind::NetworkReceive => json!({ "sender": "192.168.1.20", "port": 48000 }),
        NodeKind::ApplicationCapture => {
            json!({ "executable": "game.exe", "processPolicy": "allVerifiedInstances" })
        }
        _ => json!({}),
    };
    serde_json::from_value(parameters).unwrap()
}

/// Adds one random setting with an arbitrary (often extreme) value, keeping the
/// change only if the domain validator still accepts the session, so what
/// reaches the compiler is exactly what a client could have stored. Returns
/// whether a setting was kept.
fn perturb_parameter(rng: &mut fuzz::Rng, session: &mut Session) -> bool {
    if session.nodes.is_empty() {
        return false;
    }
    for _ in 0..12 {
        let node = rng.below(session.nodes.len());
        let key = *rng.pick(&PARAMETER_KEYS);
        let value = if rng.one_in(2) {
            extreme_value(rng)
        } else {
            plausible_value(rng)
        };
        let mut candidate = session.clone();
        candidate.nodes[node]
            .parameters
            .insert(key.to_owned(), value);
        if audiorouter_domain::validate_session(&candidate).is_ok() {
            *session = candidate;
            return true;
        }
    }
    false
}

fn random_matrix(rng: &mut fuzz::Rng, channels: usize) -> Vec<f32> {
    let length = match rng.below(8) {
        0 => rng.below(7),
        1 => channels * channels + 1,
        _ => channels * channels,
    };
    (0..length)
        .map(|index| match rng.below(12) {
            0 => f32::NAN,
            1 => f32::INFINITY,
            2 => f32::NEG_INFINITY,
            3 => 1e30,
            4 => -4.0,
            5 => 0.0,
            _ => {
                if index % (channels + 1) == 0 {
                    1.0
                } else {
                    0.0
                }
            }
        })
        .collect()
}

/// A graph whose shape is mostly valid: a source feeding a chain of random
/// tools into a sink, with random flags, parameters and edge matrices, plus
/// occasional extra nodes, edges to nowhere, cycles and wrong port names.
fn random_session(rng: &mut fuzz::Rng, label: &str) -> Session {
    let kinds = NodeKind::ALL;
    let sources: Vec<NodeKind> = kinds.iter().copied().filter(|k| is_source(*k)).collect();
    let sinks: Vec<NodeKind> = kinds.iter().copied().filter(|k| is_sink(*k)).collect();
    let others: Vec<NodeKind> = kinds
        .iter()
        .copied()
        .filter(|k| !is_source(*k) && !is_sink(*k))
        .collect();
    let processors: Vec<NodeKind> = others
        .iter()
        .copied()
        .filter(|kind| {
            !matches!(
                kind,
                NodeKind::Recorder
                    | NodeKind::Plugin
                    | NodeKind::InputSwitch
                    | NodeKind::OutputSwitch
            )
        })
        .collect();
    // Most cases are well-formed (valid shape, then extreme settings); the
    // rest are structurally hostile (bad widths, ghosts, duplicates, empties).
    let hostile = rng.one_in(3);
    let chain_width: u8 = if rng.one_in(4) { 1 } else { 2 };
    let channels = |rng: &mut fuzz::Rng| -> u8 {
        if !hostile {
            return chain_width;
        }
        match rng.below(20) {
            0 => 0,
            1 => 3,
            2 => 9,
            3..=5 => 1,
            _ => 2,
        }
    };
    let mut nodes = Vec::new();
    let mut chain = Vec::new();
    let make = |rng: &mut fuzz::Rng, kind: NodeKind, id: String, nodes: &mut Vec<Node>| {
        let width = channels(rng);
        let mut ports = Vec::new();
        if !is_source(kind) {
            ports.push(port("in", PortDirection::Input, width));
        }
        if !is_sink(kind) {
            ports.push(port("out", PortDirection::Output, width));
        }
        if hostile && rng.one_in(30) {
            ports.clear();
        }
        let parameters = required_parameters(kind);
        nodes.push(Node {
            id: EntityId::new(id),
            kind,
            type_version: if hostile && rng.one_in(40) { 0 } else { 1 },
            name: format!("node {}", nodes.len()),
            enabled: !rng.one_in(8),
            bypass: rng.one_in(6),
            parameters,
            ports,
        });
    };
    let source = *rng.pick(&sources);
    make(rng, source, "n0".into(), &mut nodes);
    chain.push("n0".to_owned());
    for index in 0..rng.range(0, 5) {
        // Workers and routing switches need devices the harness lacks; most
        // cases use the tools that compile on their own.
        let kind = if rng.one_in(10) {
            *rng.pick(&others)
        } else {
            *rng.pick(&processors)
        };
        let id = format!("t{index}");
        make(rng, kind, id.clone(), &mut nodes);
        chain.push(id);
    }
    let sink = *rng.pick(&sinks);
    make(rng, sink, "out".into(), &mut nodes);
    chain.push("out".to_owned());
    let mut edges: Vec<Edge> = chain
        .windows(2)
        .map(|pair| {
            let width = nodes
                .iter()
                .find(|node| node.id.as_str() == pair[0])
                .and_then(|node| node.ports.first())
                .map_or(2, |port| usize::from(port.channels).max(1));
            Edge {
                id: EntityId::new(format!("{}->{}", pair[0], pair[1])),
                source_node: EntityId::new(pair[0].clone()),
                source_port: "out".into(),
                destination_node: EntityId::new(pair[1].clone()),
                destination_port: "in".into(),
                matrix: if hostile || rng.one_in(25) {
                    random_matrix(rng, width)
                } else {
                    (0..width * width)
                        .map(|cell| if cell % (width + 1) == 0 { 1.0 } else { 0.0 })
                        .collect()
                },
                enabled: !rng.one_in(10),
            }
        })
        .collect();
    // Hostile extras.
    for extra in 0..if hostile { rng.below(4) } else { 0 } {
        let pick = |rng: &mut fuzz::Rng, nodes: &[Node]| -> String {
            if rng.one_in(6) {
                "ghost".to_owned()
            } else {
                nodes[rng.below(nodes.len())].id.as_str().to_owned()
            }
        };
        let source_node = pick(rng, &nodes);
        let destination_node = pick(rng, &nodes);
        edges.push(Edge {
            id: EntityId::new(if rng.one_in(8) {
                "dup".to_owned()
            } else {
                format!("extra{extra}")
            }),
            source_node: EntityId::new(source_node),
            source_port: if rng.one_in(5) {
                "x".into()
            } else {
                "out".into()
            },
            destination_node: EntityId::new(destination_node),
            destination_port: if rng.one_in(5) {
                "x".into()
            } else {
                "in".into()
            },
            matrix: random_matrix(rng, 2),
            enabled: true,
        });
    }
    if hostile && rng.one_in(12) {
        let kind = *rng.pick(&kinds);
        make(rng, kind, "t0".into(), &mut nodes); // duplicate node id
    }
    if hostile && rng.one_in(12) {
        nodes.clear();
    }
    let mut session = Session {
        id: EntityId::new(label),
        name: label.into(),
        schema_version: 1,
        revision: 1,
        nodes,
        edges,
    };
    if !hostile {
        for _ in 0..rng.below(4) {
            perturb_parameter(rng, &mut session);
        }
    }
    session
}

fn media() -> HashMap<String, Arc<DecodedAudio>> {
    HashMap::from([(
        "media-1".to_owned(),
        Arc::new(DecodedAudio {
            channels: 2,
            sample_rate_hz: GRAPH_RATE_HZ,
            samples: vec![0.1; 2 * 4_800].into(),
        }),
    )])
}

#[test]
fn random_sessions_compile_to_ok_or_a_structured_error() {
    let media = media();
    let mut compiled = 0_usize;
    let mut refused = 0_usize;
    let mut largest_request = 0_usize;
    fuzz::run("engine/compile", SEED ^ 1, 1_500, |index, rng| {
        let session = random_session(rng, &format!("fuzz-{index}"));
        let (result, largest) = fuzz::measure_allocation(|| {
            compile_native_paths_with_plugins_and_audio(
                &session,
                RuntimeGeneration::new(1),
                &HashMap::new(),
                &media,
            )
        });
        largest_request = largest_request.max(largest);
        assert!(
            largest <= 128 * 1024 * 1024,
            "compile requested {largest} bytes: {}",
            serde_json::to_string(&session).unwrap()
        );
        match result {
            Ok(set) => {
                compiled += 1;
                // A compiled path must also run a quantum without panicking
                // and keep its outputs finite.
                for path in set.paths() {
                    let inputs = path
                        .input_node_ids()
                        .iter()
                        .map(|_| AudioBlock::new(2, 128).unwrap())
                        .collect::<Vec<_>>();
                    let mut scratch = AudioBlock::new(path.mixer_channels(), 128).unwrap();
                    let mut outputs = path
                        .output_node_ids()
                        .iter()
                        .map(|_| AudioBlock::new(2, 128).unwrap())
                        .collect::<Vec<_>>();
                    let _ = path.process(
                        &inputs,
                        &mut scratch,
                        &mut outputs.iter_mut().collect::<Vec<_>>(),
                    );
                    for block in &outputs {
                        for channel in 0..2 {
                            assert!(
                                block
                                    .channel(channel)
                                    .unwrap()
                                    .iter()
                                    .all(|sample| sample.is_finite()),
                                "non-finite output: {}",
                                serde_json::to_string(&session).unwrap()
                            );
                        }
                    }
                }
            }
            Err(error) => {
                refused += 1;
                // Structured: one of the documented variants, with a message.
                match error {
                    GraphCompileError::InvalidGraph(errors) => assert!(!errors.is_empty()),
                    GraphCompileError::UnsupportedPath(message)
                    | GraphCompileError::MissingAudioMedia(message) => {
                        assert!(!message.is_empty())
                    }
                    GraphCompileError::UnsupportedTopology
                    | GraphCompileError::InvalidSampleRate => {}
                }
            }
        }
    });
    eprintln!(
        "engine/compile: {compiled} compiled, {refused} refused, largest request {largest_request} bytes"
    );
    assert!(compiled > 100, "only {compiled} random sessions compiled");
    assert!(refused > 100, "only {refused} random sessions were refused");
}
