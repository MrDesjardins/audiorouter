//! Q4.3 bounded fuzzing of the plugin host's byte parsers: VST2 `.fxp`/`.fxb`
//! presets (read from user-chosen files in the backend) and the worker message
//! frames (read from the isolated plugin worker process, which may be hostile).
//!
//! Fixed seeds and iteration counts; every case mutates a valid input and
//! asserts no panic, allocation proportional to the input rather than to any
//! declared size, and a result that satisfies the documented limits.
use audiorouter_plugin_host::vst2_preset::{
    parse, Vst2BankProgram, Vst2Preset, Vst2PresetBody, Vst2PresetError, MAX_VST2_PRESET_BYTES,
    MAX_VST2_PRESET_PARAMETERS, MAX_VST2_PRESET_PROGRAMS,
};
use audiorouter_plugin_host::{
    decode_worker_message, encode_worker_message, read_worker_message, read_worker_message_into,
    PluginStateAsset, WorkerFrame, WorkerMessage, WorkerMessageError, MAX_WORKER_MESSAGE_BYTES,
    MAX_WORKER_STATE_BYTES, WORKER_PROTOCOL_VERSION, WORKER_SCRATCH_RETAIN_BYTES,
};

#[path = "../../../tests/support/fuzz.rs"]
mod fuzz;

#[global_allocator]
static ALLOCATOR: fuzz::TrackingAllocator = fuzz::TrackingAllocator;

const SEED: u64 = 0x504C_5547_494E_4658; // "PLUGINFX"
const NAME_BYTES: usize = 28;
/// `Vec::with_capacity` of at most 4096 floats or 256 programs is the largest
/// allocation a tiny preset may cause; anything past this is proportional to a
/// declared count instead of to the file.
const PRESET_ALLOWANCE_BYTES: usize = 128 * 1024;

fn name_bytes(text: &str) -> [u8; NAME_BYTES] {
    let mut raw = [0_u8; NAME_BYTES];
    let length = text.len().min(NAME_BYTES - 1);
    raw[..length].copy_from_slice(&text.as_bytes()[..length]);
    raw
}

fn preset_file(kind: &[u8; 4], version: u32, fx_id: u32, body: &[u8]) -> Vec<u8> {
    let mut data = kind.to_vec();
    data.extend_from_slice(&version.to_be_bytes());
    data.extend_from_slice(&fx_id.to_be_bytes());
    data.extend_from_slice(&7_u32.to_be_bytes());
    data.extend_from_slice(body);
    let mut out = b"CcnK".to_vec();
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&data);
    out
}

fn parameter_program(name: &str, values: &[f32]) -> Vec<u8> {
    let mut body = (values.len() as u32).to_be_bytes().to_vec();
    body.extend_from_slice(&name_bytes(name));
    for value in values {
        body.extend_from_slice(&value.to_be_bytes());
    }
    body
}

fn chunk_program(name: &str, parameter_count: u32, chunk: &[u8]) -> Vec<u8> {
    let mut body = parameter_count.to_be_bytes().to_vec();
    body.extend_from_slice(&name_bytes(name));
    body.extend_from_slice(&(chunk.len() as u32).to_be_bytes());
    body.extend_from_slice(chunk);
    body
}

fn bank_header(count: u32, current: u32) -> Vec<u8> {
    let mut body = count.to_be_bytes().to_vec();
    body.extend_from_slice(&current.to_be_bytes());
    body.extend_from_slice(&[0_u8; 124]);
    body
}

const FX_ID: u32 = 0x1234_5678;

fn seed_presets() -> Vec<Vec<u8>> {
    let program = preset_file(
        b"FxCk",
        1,
        FX_ID,
        &parameter_program("Lead", &[0.0, 0.5, 1.0]),
    );
    let chunk = preset_file(
        b"FPCh",
        1,
        FX_ID,
        &chunk_program("Chunk", 3, b"plugin-owned"),
    );
    let mut parameter_bank = bank_header(2, 1);
    parameter_bank.extend_from_slice(&preset_file(
        b"FxCk",
        1,
        FX_ID,
        &parameter_program("One", &[0.25]),
    ));
    parameter_bank.extend_from_slice(&preset_file(
        b"FPCh",
        1,
        FX_ID,
        &chunk_program("Two", 2, b"xyz"),
    ));
    let bank = preset_file(b"FxBk", 2, FX_ID, &parameter_bank);
    let mut chunk_bank = bank_header(0, 0);
    chunk_bank.extend_from_slice(&5_u32.to_be_bytes());
    chunk_bank.extend_from_slice(b"bank!");
    let bank_chunk = preset_file(b"FBCh", 2, FX_ID, &chunk_bank);
    vec![program, chunk, bank, bank_chunk]
}

fn parse_checked(input: &[u8]) -> Result<Vst2Preset, Vst2PresetError> {
    let (result, largest) = fuzz::measure_allocation(|| parse(input));
    assert!(
        largest <= PRESET_ALLOWANCE_BYTES + 4 * input.len(),
        "parse requested {largest} bytes for {} input bytes: {}",
        input.len(),
        fuzz::hex(input)
    );
    result
}

/// What every accepted preset promises, whatever bytes it came from.
fn assert_preset_within_limits(preset: &Vst2Preset, input: &[u8]) {
    let name_ok = |name: &str| {
        assert!(name.len() < NAME_BYTES, "{}", fuzz::hex(input));
        assert!(!name.chars().any(char::is_control), "{}", fuzz::hex(input));
    };
    let parameters_ok = |parameters: &[f32]| {
        assert!(parameters.len() <= MAX_VST2_PRESET_PARAMETERS);
        assert!(
            parameters
                .iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value)),
            "{}",
            fuzz::hex(input)
        );
    };
    match &preset.body {
        Vst2PresetBody::ProgramParameters { name, parameters } => {
            name_ok(name);
            parameters_ok(parameters);
        }
        Vst2PresetBody::ProgramChunk {
            name,
            parameter_count,
            chunk,
        } => {
            name_ok(name);
            assert!(*parameter_count <= MAX_VST2_PRESET_PARAMETERS);
            assert!(!chunk.is_empty() && chunk.len() <= input.len());
        }
        Vst2PresetBody::Bank {
            current_program,
            programs,
        } => {
            assert!((1..=MAX_VST2_PRESET_PROGRAMS).contains(&programs.len()));
            assert!((*current_program as usize) < programs.len() || *current_program == 0);
            for program in programs {
                match program {
                    Vst2BankProgram::Parameters { name, parameters } => {
                        name_ok(name);
                        parameters_ok(parameters);
                    }
                    Vst2BankProgram::Chunk {
                        name,
                        parameter_count,
                        chunk,
                    } => {
                        name_ok(name);
                        assert!(*parameter_count <= MAX_VST2_PRESET_PARAMETERS);
                        assert!(!chunk.is_empty() && chunk.len() <= input.len());
                    }
                }
            }
        }
        Vst2PresetBody::BankChunk { chunk, .. } => {
            assert!(!chunk.is_empty() && chunk.len() <= input.len());
        }
    }
}

#[test]
fn seed_presets_parse_and_every_strict_prefix_or_extension_is_rejected() {
    for seed in seed_presets() {
        let preset = parse_checked(&seed).expect("the seed is a valid preset");
        assert_preset_within_limits(&preset, &seed);
        for length in 0..seed.len() {
            assert!(
                parse_checked(&seed[..length]).is_err(),
                "a {length}-byte prefix of a {}-byte preset parsed",
                seed.len()
            );
        }
        let mut longer = seed.clone();
        longer.push(0);
        assert!(parse_checked(&longer).is_err(), "trailing byte accepted");
    }
}

#[test]
fn mutated_presets_never_panic_and_stay_within_limits() {
    let seeds = seed_presets();
    let mut accepted = 0_usize;
    let mut rejected = 0_usize;
    fuzz::run("plugin-host/vst2-preset", SEED, 6_000, |_, rng| {
        let seed = rng.pick(&seeds).clone();
        let input = if rng.one_in(3) {
            fuzz::mutate(rng, &seed, 4 * 1024)
        } else {
            // Corrupt the body but keep the declared size honest, so the
            // parser reaches the count, size and name fields.
            let mut input = fuzz::mutate(rng, &seed[8..], 4 * 1024);
            let mut file = b"CcnK".to_vec();
            file.extend_from_slice(&(input.len() as u32).to_be_bytes());
            file.append(&mut input);
            file
        };
        match parse_checked(&input) {
            Ok(preset) => {
                accepted += 1;
                assert_preset_within_limits(&preset, &input);
            }
            Err(_) => rejected += 1,
        }
    });
    assert!(accepted > 50, "only {accepted} mutated presets parsed");
    assert!(rejected > 1_000, "only {rejected} mutated presets failed");
}

/// Counts and sizes at, just under and far over each limit, in every field
/// that drives an allocation or a loop. A hostile count must fail on the file
/// length, not by allocating or iterating that many times.
#[test]
fn declared_counts_and_sizes_are_checked_before_they_are_used() {
    let huge = [
        0_u32,
        1,
        MAX_VST2_PRESET_PARAMETERS as u32,
        MAX_VST2_PRESET_PARAMETERS as u32 + 1,
        MAX_VST2_PRESET_PROGRAMS as u32,
        MAX_VST2_PRESET_PROGRAMS as u32 + 1,
        0x00FF_FFFF,
        0x7FFF_FFFF,
        0x8000_0000,
        u32::MAX,
    ];
    for count in huge {
        // Parameter program with a declared parameter count but no floats.
        let mut body = count.to_be_bytes().to_vec();
        body.extend_from_slice(&name_bytes("x"));
        let result = parse_checked(&preset_file(b"FxCk", 1, FX_ID, &body));
        assert!(
            result.is_err() || count == 0,
            "{count:#x} parameters with no data parsed"
        );
        // Chunk program claiming `count` bytes of chunk.
        let mut body = 1_u32.to_be_bytes().to_vec();
        body.extend_from_slice(&name_bytes("x"));
        body.extend_from_slice(&count.to_be_bytes());
        body.extend_from_slice(b"tiny");
        let result = parse_checked(&preset_file(b"FPCh", 1, FX_ID, &body));
        assert!(
            result.is_err() || count == 4,
            "{count:#x}-byte chunk of 4 bytes parsed"
        );
        // Bank claiming `count` programs.
        let mut body = bank_header(count, 0);
        body.extend_from_slice(&preset_file(
            b"FxCk",
            1,
            FX_ID,
            &parameter_program("a", &[0.5]),
        ));
        let result = parse_checked(&preset_file(b"FxBk", 2, FX_ID, &body));
        assert!(
            result.is_err() || count == 1,
            "{count:#x} programs of one parsed"
        );
        // Bank chunk claiming `count` bytes.
        let mut body = bank_header(0, 0);
        body.extend_from_slice(&count.to_be_bytes());
        body.extend_from_slice(b"tiny");
        let result = parse_checked(&preset_file(b"FBCh", 2, FX_ID, &body));
        assert!(
            result.is_err() || count == 4,
            "{count:#x}-byte bank chunk of 4 bytes parsed"
        );
        // Outer body size: the declared size is the file's, so any other
        // value is a size mismatch or truncation.
        let mut file = seed_presets().remove(0);
        file[4..8].copy_from_slice(&count.to_be_bytes());
        assert!(
            parse_checked(&file).is_err(),
            "body size {count:#x} accepted"
        );
    }
    // Non-finite floats are rejected, out-of-range ones are clamped.
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let body = parameter_program("x", &[value]);
        assert!(parse_checked(&preset_file(b"FxCk", 1, FX_ID, &body)).is_err());
    }
    let body = parameter_program("x", &[-5.0, 7.0, f32::MAX, f32::MIN]);
    match parse_checked(&preset_file(b"FxCk", 1, FX_ID, &body))
        .unwrap()
        .body
    {
        Vst2PresetBody::ProgramParameters { parameters, .. } => {
            assert_eq!(parameters, vec![0.0, 1.0, 1.0, 0.0]);
        }
        other => panic!("{other:?}"),
    }
    // A file larger than the cap is refused up front.
    let oversized = vec![0_u8; MAX_VST2_PRESET_BYTES + 1];
    assert_eq!(parse(&oversized).unwrap_err(), Vst2PresetError::TooLarge);
}

/// Banks must not recurse: a bank inside a bank, nested deeply, is refused
/// rather than parsed to a stack depth chosen by the file.
#[test]
fn nested_banks_are_refused_without_recursion() {
    let mut inner = preset_file(b"FxCk", 1, FX_ID, &parameter_program("a", &[0.5]));
    for _ in 0..2_000 {
        let mut body = bank_header(1, 0);
        body.extend_from_slice(&inner);
        inner = preset_file(b"FxBk", 2, FX_ID, &body);
        if inner.len() > MAX_VST2_PRESET_BYTES {
            break;
        }
    }
    assert!(parse_checked(&inner).is_err());
}

#[test]
fn generated_valid_presets_round_trip() {
    fuzz::run("plugin-host/vst2-roundtrip", SEED ^ 1, 600, |_, rng| {
        let fx_id = rng.next_u32();
        let bank = rng.one_in(2);
        let program_count = if bank { rng.range(1, 5) } else { 1 };
        let mut expected = Vec::new();
        let mut programs = Vec::new();
        for index in 0..program_count {
            let name = format!("Program {index}");
            if rng.one_in(2) {
                let values: Vec<f32> = (0..rng.range(0, 40))
                    .map(|_| rng.below(1001) as f32 / 1000.0)
                    .collect();
                programs.push(preset_file(
                    b"FxCk",
                    1,
                    fx_id,
                    &parameter_program(&name, &values),
                ));
                expected.push(Vst2BankProgram::Parameters {
                    name,
                    parameters: values,
                });
            } else {
                let chunk_length = rng.range(1, 64);
                let chunk = rng.bytes(chunk_length);
                let parameter_count = rng.below(50);
                programs.push(preset_file(
                    b"FPCh",
                    1,
                    fx_id,
                    &chunk_program(&name, parameter_count as u32, &chunk),
                ));
                expected.push(Vst2BankProgram::Chunk {
                    name,
                    parameter_count,
                    chunk,
                });
            }
        }
        if !bank {
            let file = programs.remove(0);
            let preset = parse_checked(&file).unwrap();
            let body = match expected.remove(0) {
                Vst2BankProgram::Parameters { name, parameters } => {
                    Vst2PresetBody::ProgramParameters { name, parameters }
                }
                Vst2BankProgram::Chunk {
                    name,
                    parameter_count,
                    chunk,
                } => Vst2PresetBody::ProgramChunk {
                    name,
                    parameter_count,
                    chunk,
                },
            };
            assert_eq!(preset.body, body);
            assert_eq!(preset.fx_id, fx_id);
        } else {
            let current = rng.below(program_count) as u32;
            let mut body = bank_header(program_count as u32, current);
            for program in &programs {
                body.extend_from_slice(program);
            }
            let preset = parse_checked(&preset_file(b"FxBk", 2, fx_id, &body)).unwrap();
            assert_eq!(
                preset.body,
                Vst2PresetBody::Bank {
                    current_program: current,
                    programs: expected
                }
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Worker message frames
// ---------------------------------------------------------------------------

fn hex64(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn seed_messages() -> Vec<Vec<u8>> {
    let messages = [
        WorkerMessage::Ready,
        WorkerMessage::Shutdown,
        WorkerMessage::Hello {
            protocol_version: WORKER_PROTOCOL_VERSION,
            plugin_sha256: hex64('a'),
            channels: 2,
        },
        WorkerMessage::SelectProgram { index: 3 },
        WorkerMessage::Failure {
            code: "load-failed".into(),
        },
        WorkerMessage::ApplyPreset {
            bytes: seed_presets().remove(0),
        },
        WorkerMessage::Process {
            frame: WorkerFrame::new(1, 100, 2, vec![0.25; 64]).unwrap(),
            parameters: Vec::new(),
        },
        WorkerMessage::ProcessShared {
            sequence: 4,
            deadline_tick: 99,
            channels: 2,
            frames: 128,
            parameters: Vec::new(),
        },
        WorkerMessage::EditorOpen {
            parent_window: 0x1234,
            parent_process_id: 42,
            authorization_token: "token".into(),
        },
    ];
    messages
        .iter()
        .map(|message| encode_worker_message(message).unwrap())
        .collect()
}

fn decode_checked(input: &[u8]) -> Result<WorkerMessage, WorkerMessageError> {
    let (result, largest) = fuzz::measure_allocation(|| decode_worker_message(input));
    assert!(
        largest <= 256 * 1024 + 128 * input.len(),
        "decode_worker_message requested {largest} bytes for {} input bytes: {}",
        input.len(),
        fuzz::hex(input)
    );
    // Whatever decodes must re-encode: validation is symmetric.
    if let Ok(message) = &result {
        assert!(
            encode_worker_message(message).is_ok(),
            "a decoded message failed encoding: {}",
            fuzz::hex(input)
        );
    }
    result
}

#[test]
fn mutated_worker_frames_never_panic_and_a_decoded_frame_re_encodes() {
    let seeds = seed_messages();
    for seed in &seeds {
        assert!(decode_checked(seed).is_ok());
    }
    let mut accepted = 0_usize;
    fuzz::run("plugin-host/worker-frames", SEED ^ 2, 4_000, |_, rng| {
        let seed = rng.pick(&seeds).clone();
        let input = if rng.one_in(2) {
            fuzz::mutate(rng, &seed, 8 * 1024)
        } else {
            let payload = fuzz::mutate(rng, &seed[4..], 8 * 1024);
            let mut frame = (payload.len() as u32).to_le_bytes().to_vec();
            frame.extend_from_slice(&payload);
            frame
        };
        if decode_checked(&input).is_ok() {
            accepted += 1;
        }
    });
    assert!(accepted > 10, "only {accepted} mutated frames decoded");
}

#[test]
fn worker_frame_length_prefix_is_checked_before_the_payload_is_read() {
    let valid = encode_worker_message(&WorkerMessage::Ready).unwrap();
    for declared in [
        0_u32,
        valid.len() as u32 - 5,
        valid.len() as u32 - 3,
        MAX_WORKER_MESSAGE_BYTES as u32,
        MAX_WORKER_MESSAGE_BYTES as u32 + 1,
        u32::MAX,
    ] {
        let mut frame = valid.clone();
        frame[..4].copy_from_slice(&declared.to_le_bytes());
        let result = decode_checked(&frame);
        if declared as usize > MAX_WORKER_MESSAGE_BYTES {
            assert!(matches!(result, Err(WorkerMessageError::TooLarge { .. })));
        } else {
            assert!(result.is_err(), "{declared:#x}");
        }
        // The streaming reader refuses an oversized declaration before it
        // reserves the payload, and never reserves more than the cap.
        let mut cursor = std::io::Cursor::new(frame.clone());
        let (streamed, largest) = fuzz::measure_allocation(|| read_worker_message(&mut cursor));
        assert!(streamed.is_err() || declared as usize == valid.len() - 4);
        assert!(
            largest <= MAX_WORKER_MESSAGE_BYTES + 4 + 4096,
            "read_worker_message reserved {largest} bytes for declared {declared:#x}"
        );
    }
}

/// Largest single allocation a hostile header may cause. The reader grows its
/// buffer with the bytes that arrive, so this is far below the 65 MiB cap.
const HOSTILE_HEADER_ALLOCATION_LIMIT: usize = 256 * 1024;

#[test]
fn a_hostile_length_header_with_little_data_does_not_reserve_the_frame() {
    for declared in [
        1_u32 << 20,
        16 << 20,
        MAX_WORKER_MESSAGE_BYTES as u32 / 2,
        MAX_WORKER_MESSAGE_BYTES as u32,
    ] {
        for sent in [0_usize, 1, 100, 4_096] {
            let mut frame = declared.to_le_bytes().to_vec();
            frame.extend(std::iter::repeat_n(b'{', sent));
            let mut cursor = std::io::Cursor::new(frame);
            let (result, largest) = fuzz::measure_allocation(|| read_worker_message(&mut cursor));
            assert!(
                matches!(result, Err(WorkerMessageError::Io(_))),
                "declared {declared:#x}, sent {sent}: {result:?}"
            );
            assert!(
                largest <= HOSTILE_HEADER_ALLOCATION_LIMIT,
                "declared {declared:#x} with {sent} bytes reserved {largest} bytes"
            );
        }
    }
}

#[test]
fn a_trickled_oversized_frame_allocates_only_what_has_arrived() {
    // The peer keeps declaring a 60 MiB frame but delivers 3 MiB of it. The
    // buffer may follow the data (with doubling slack), never the header.
    let declared = 60_u32 << 20;
    let mut frame = declared.to_le_bytes().to_vec();
    frame.resize(4 + (3 << 20), b' ');
    let mut cursor = std::io::Cursor::new(frame);
    let (result, largest) = fuzz::measure_allocation(|| read_worker_message(&mut cursor));
    assert!(matches!(result, Err(WorkerMessageError::Io(_))));
    assert!(
        largest <= 8 << 20,
        "3 MiB of data reserved {largest} bytes for a 60 MiB header"
    );
}

#[test]
fn a_worst_case_state_frame_still_decodes_through_the_stream_reader() {
    // 0xFF bytes encode to four JSON characters each, so this frame is within
    // a few KiB of the protocol maximum.
    let asset = PluginStateAsset::new(1, vec![0xFF; MAX_WORKER_STATE_BYTES]).unwrap();
    let message = WorkerMessage::StateRestore { asset };
    let frame = encode_worker_message(&message).unwrap();
    assert!(
        frame.len() > MAX_WORKER_MESSAGE_BYTES * 9 / 10 - 4,
        "frame of {} bytes is not near the {MAX_WORKER_MESSAGE_BYTES}-byte maximum",
        frame.len()
    );
    let mut cursor = std::io::Cursor::new(frame);
    let (result, largest) = fuzz::measure_allocation(|| read_worker_message(&mut cursor));
    assert_eq!(result.unwrap(), message);
    // Payload buffer (at most doubling slack) plus the decoded state; the
    // check is that no single request exceeds twice the declared frame.
    assert!(
        largest <= 2 * MAX_WORKER_MESSAGE_BYTES,
        "reserved {largest} bytes"
    );
}

#[test]
fn the_scratch_buffer_is_reused_across_normal_frames_and_trimmed_after_a_large_one() {
    let first = WorkerMessage::Ready;
    let mut stream = encode_worker_message(&first).unwrap();
    stream.extend(encode_worker_message(&first).unwrap());
    let mut cursor = std::io::Cursor::new(stream);
    let mut scratch = Vec::new();
    assert_eq!(
        read_worker_message_into(&mut cursor, &mut scratch).unwrap(),
        first
    );
    let pointer = scratch.as_ptr();
    let capacity = scratch.capacity();
    assert!(capacity > 0, "the scratch buffer should stay allocated");
    let (second, largest) =
        fuzz::measure_allocation(|| read_worker_message_into(&mut cursor, &mut scratch));
    assert_eq!(second.unwrap(), first);
    assert_eq!(scratch.as_ptr(), pointer, "a normal frame reallocated");
    assert_eq!(scratch.capacity(), capacity);
    // The decoded message may allocate small strings, never a payload buffer.
    assert!(
        largest < capacity.max(256),
        "second read allocated {largest}"
    );

    let asset = PluginStateAsset::new(1, vec![0xFF; 1 << 20]).unwrap();
    let big = WorkerMessage::StateRestore { asset };
    let mut cursor = std::io::Cursor::new(encode_worker_message(&big).unwrap());
    assert_eq!(
        read_worker_message_into(&mut cursor, &mut scratch).unwrap(),
        big
    );
    assert!(
        scratch.capacity() <= WORKER_SCRATCH_RETAIN_BYTES,
        "a {} byte scratch buffer stayed allocated after a large frame",
        scratch.capacity()
    );
}

#[test]
fn deeply_nested_worker_json_is_a_structured_error() {
    for depth in [10_usize, 127, 129, 5_000] {
        let nested = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let payload = format!("{{\"type\":\"Failure\",\"payload\":{{\"code\":{nested}}}}}");
        let mut frame = (payload.len() as u32).to_le_bytes().to_vec();
        frame.extend_from_slice(payload.as_bytes());
        assert!(decode_checked(&frame).is_err(), "depth {depth}");
    }
}
