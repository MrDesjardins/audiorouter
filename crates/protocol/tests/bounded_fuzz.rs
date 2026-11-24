//! Q4.3 bounded fuzzing of the untrusted JSON-RPC frame decoder.
//!
//! Fixed seeds, fixed iteration counts, no I/O. Every case mutates a valid
//! frame (bit flips, truncation, length-prefix corruption, insertions) and
//! asserts: no panic, no allocation far larger than the input, and a decoded
//! message that satisfies the documented limits. `AUDIOROUTER_FUZZ_SCALE`
//! multiplies the iterations for a longer local run.
use audiorouter_protocol::{
    decode_frame, decode_rpc_frame, encode_frame, parse_rpc_message, FrameError, JsonRpcRequest,
    MessageError, RpcMessage, MAX_BATCH_REQUESTS, MAX_FRAME_BYTES, MAX_METHOD_NAME_BYTES,
    MAX_REQUEST_ID_BYTES,
};
use serde_json::{json, Value};

#[path = "../../../tests/support/fuzz.rs"]
mod fuzz;

#[global_allocator]
static ALLOCATOR: fuzz::TrackingAllocator = fuzz::TrackingAllocator;

const SEED: u64 = 0x5052_4F54_4F43_4F4C; // "PROTOCOL"
/// A decoder may build a JSON tree, but never an allocation that is large
/// compared with the bytes it was given.
const ALLOCATION_ALLOWANCE_BYTES: usize = 256 * 1024;
const ALLOCATION_PER_INPUT_BYTE: usize = 64;

fn frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = (payload.len() as u32).to_le_bytes().to_vec();
    frame.extend_from_slice(payload);
    frame
}

fn request(id: Value, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

fn seed_frames() -> Vec<Vec<u8>> {
    let single = request(json!(7), "sessions.list", json!({"limit": 20}));
    let notification = json!({"jsonrpc": "2.0", "method": "events.subscribe"});
    let batch = json!([
        request(json!("a"), "sessions.get", json!({"sessionId": "s1"})),
        request(json!(2), "nodes.types", json!({})),
        notification.clone(),
    ]);
    let nested = request(
        json!(1),
        "sessions.importPlan",
        json!({"session": {"nodes": [{"parameters": {"a": [[[{"b": [1, 2, 3]}]]]}}]}}),
    );
    [single, notification, batch, nested]
        .iter()
        .map(|value| encode_frame(value).unwrap())
        .collect()
}

/// What a successful decode promises, whatever the input was.
fn assert_message_within_limits(message: &RpcMessage, input: &[u8]) {
    let check = |request: &JsonRpcRequest| {
        assert_eq!(request.jsonrpc, "2.0", "{}", fuzz::hex(input));
        assert!(
            (1..=MAX_METHOD_NAME_BYTES).contains(&request.method.len()),
            "{}",
            fuzz::hex(input)
        );
        if let Some(id) = &request.id {
            assert!(
                matches!(id, Value::String(_) | Value::Number(_) | Value::Null),
                "{}",
                fuzz::hex(input)
            );
            assert!(
                serde_json::to_vec(id).unwrap().len() <= MAX_REQUEST_ID_BYTES,
                "{}",
                fuzz::hex(input)
            );
        }
    };
    match message {
        RpcMessage::Single(request) => check(request),
        RpcMessage::Batch(requests) => {
            assert!(
                (1..=MAX_BATCH_REQUESTS).contains(&requests.len()),
                "{}",
                fuzz::hex(input)
            );
            requests.iter().for_each(check);
        }
    }
}

fn decode_checked(input: &[u8]) -> Result<RpcMessage, FrameError> {
    let (result, largest) = fuzz::measure_allocation(|| decode_rpc_frame(input));
    assert!(
        largest <= ALLOCATION_ALLOWANCE_BYTES + ALLOCATION_PER_INPUT_BYTE * input.len(),
        "decode_rpc_frame requested {largest} bytes for {} input bytes: {}",
        input.len(),
        fuzz::hex(input)
    );
    if let Ok(message) = &result {
        assert_message_within_limits(message, input);
    }
    result
}

#[test]
fn mutated_rpc_frames_never_panic_and_stay_within_limits() {
    let seeds = seed_frames();
    let mut accepted = 0_usize;
    let mut rejected = 0_usize;
    fuzz::run("protocol/mutated-frames", SEED, 4_000, |_, rng| {
        let seed = rng.pick(&seeds).clone();
        let input = if rng.one_in(2) {
            // Corrupt the whole frame, length prefix included.
            fuzz::mutate(rng, &seed, 8 * 1024)
        } else {
            // Corrupt only the JSON and keep the prefix honest, so the
            // mutation reaches the JSON and request validation.
            let payload = fuzz::mutate(rng, &seed[4..], 8 * 1024);
            frame(&payload)
        };
        match decode_checked(&input) {
            Ok(_) => accepted += 1,
            Err(_) => rejected += 1,
        }
        // The generic decoder shares the length checks.
        let generic: Result<Value, FrameError> = decode_frame(&input);
        if let Ok(value) = generic {
            assert!(value.is_array() || value.is_object() || !input.is_empty());
        }
    });
    // A harness that rejects everything would prove nothing.
    assert!(accepted > 50, "only {accepted} mutated frames decoded");
    assert!(
        rejected > 100,
        "only {rejected} mutated frames were rejected"
    );
}

#[test]
fn length_prefix_corruption_is_rejected_without_reading_past_the_frame() {
    let payload = serde_json::to_vec(&request(json!(1), "nodes.types", json!({}))).unwrap();
    let valid = frame(&payload);
    assert!(decode_checked(&valid).is_ok());
    for declared in [
        0_u32,
        1,
        payload.len() as u32 - 1,
        payload.len() as u32 + 1,
        MAX_FRAME_BYTES as u32,
        MAX_FRAME_BYTES as u32 + 1,
        0x7FFF_FFFF,
        0x8000_0000,
        u32::MAX,
    ] {
        let mut corrupt = valid.clone();
        corrupt[..4].copy_from_slice(&declared.to_le_bytes());
        let result = decode_checked(&corrupt);
        match declared as usize {
            length if length > MAX_FRAME_BYTES => {
                assert!(
                    matches!(result, Err(FrameError::TooLarge { .. })),
                    "{declared:#x}: {result:?}"
                );
            }
            length if length != payload.len() => {
                assert!(
                    matches!(result, Err(FrameError::LengthMismatch { .. })),
                    "{declared:#x}: {result:?}"
                );
            }
            _ => assert!(result.is_ok()),
        }
    }
    // Frames shorter than the prefix itself.
    for length in 0..4 {
        assert_eq!(
            decode_checked(&valid[..length]).unwrap_err(),
            FrameError::TooShort
        );
    }
}

/// Deep nesting must end in a structured JSON error, not a stack overflow.
/// serde_json's recursion limit is 128; check both sides of it, for arrays,
/// objects, and a mix, at a few depths up to 200 000 levels.
#[test]
fn deeply_nested_json_is_a_structured_error_not_a_stack_overflow() {
    for depth in [1_usize, 64, 126, 127, 128, 129, 130, 1_000, 200_000] {
        let array = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let object = format!("{}1{}", "{\"a\":".repeat(depth), "}".repeat(depth));
        let mixed = (0..depth)
            .map(|level| if level % 2 == 0 { "[" } else { "{\"k\":" })
            .collect::<String>();
        for text in [array, object, mixed] {
            let input = frame(text.as_bytes());
            if input.len() > MAX_FRAME_BYTES {
                continue;
            }
            // Never panics; deeper than serde_json's limit is a JSON error.
            let result = decode_checked(&input);
            if depth > 130 {
                assert!(
                    matches!(result, Err(FrameError::Json(_))),
                    "depth {depth}: {result:?}"
                );
            }
            // Unbalanced or unwrapped values are never a request.
            assert!(result.is_err(), "depth {depth} decoded as a request");
        }
    }
    // The same nesting inside `params` of an otherwise valid request.
    for depth in [8_usize, 100, 120, 126, 127, 128, 500] {
        let nested = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let text = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"sessions.list\",\"params\":{{\"x\":{nested}}}}}"
        );
        let result = decode_checked(&frame(text.as_bytes()));
        if depth > 125 {
            assert!(
                matches!(result, Err(FrameError::Json(_))),
                "depth {depth}: {result:?}"
            );
        } else {
            assert!(result.is_ok(), "depth {depth}: {result:?}");
        }
    }
}

#[test]
fn batch_id_and_method_limits_hold_at_and_beyond_each_boundary() {
    let one = request(json!(1), "nodes.types", json!({}));
    for count in [
        0_usize,
        1,
        MAX_BATCH_REQUESTS - 1,
        MAX_BATCH_REQUESTS,
        33,
        300,
    ] {
        let batch = Value::Array(vec![one.clone(); count]);
        let result = parse_rpc_message(&serde_json::to_vec(&batch).unwrap());
        match count {
            0 => assert_eq!(result.unwrap_err(), MessageError::EmptyBatch),
            1..=MAX_BATCH_REQUESTS => match result.unwrap() {
                RpcMessage::Batch(requests) => assert_eq!(requests.len(), count),
                other => panic!("{other:?}"),
            },
            _ => assert!(
                matches!(result, Err(MessageError::BatchTooLarge { length, .. }) if length == count),
                "{count}: {result:?}"
            ),
        }
    }
    for length in [
        0_usize,
        1,
        MAX_METHOD_NAME_BYTES - 1,
        MAX_METHOD_NAME_BYTES,
        MAX_METHOD_NAME_BYTES + 1,
        100_000,
    ] {
        let value = request(json!(1), &"m".repeat(length), json!({}));
        let result = parse_rpc_message(&serde_json::to_vec(&value).unwrap());
        assert_eq!(
            result.is_ok(),
            (1..=MAX_METHOD_NAME_BYTES).contains(&length),
            "method length {length}"
        );
    }
    for length in [
        0_usize,
        1,
        MAX_REQUEST_ID_BYTES - 2,
        MAX_REQUEST_ID_BYTES - 1,
        129,
        100_000,
    ] {
        // The id is a JSON string, so the quotes count towards the limit.
        let value = request(Value::String("i".repeat(length)), "nodes.types", json!({}));
        let result = parse_rpc_message(&serde_json::to_vec(&value).unwrap());
        assert_eq!(
            result.is_ok(),
            length + 2 <= MAX_REQUEST_ID_BYTES,
            "id length {length}"
        );
    }
    // Structured ids are never accepted.
    for id in [json!([1]), json!({"a": 1}), json!(true)] {
        let value = request(id.clone(), "nodes.types", json!({}));
        assert!(
            parse_rpc_message(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{id}"
        );
    }
}

#[test]
fn raw_random_bytes_are_never_a_request() {
    fuzz::run("protocol/random-bytes", SEED ^ 1, 2_000, |_, rng| {
        let length = rng.range(0, 512);
        let payload = rng.bytes(length);
        assert!(decode_checked(&frame(&payload)).is_err());
        assert!(decode_checked(&payload).is_err() || payload.len() < 4);
        let _ = parse_rpc_message(&payload);
    });
}

#[test]
fn encoded_requests_round_trip_for_generated_values() {
    fuzz::run("protocol/round-trip", SEED ^ 2, 1_000, |_, rng| {
        let id = match rng.below(3) {
            0 => json!(rng.next_u64() as i64),
            1 => json!(format!("id-{}", rng.next_u32())),
            _ => Value::Null,
        };
        let method = format!("area{}.method{}", rng.below(9), rng.below(9));
        let params = if rng.one_in(2) {
            json!({"limit": rng.below(500), "cursor": format!("c{}", rng.next_u32())})
        } else {
            json!({"nested": {"list": [rng.below(10), "x", null, {"k": rng.next_u32()}]}})
        };
        let original = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            // `"id": null` and an absent id both decode as `None`.
            id: if matches!(id, Value::Null) {
                None
            } else {
                Some(id)
            },
            method,
            params: Some(params),
        };
        let encoded = encode_frame(&original).unwrap();
        match decode_checked(&encoded).unwrap() {
            RpcMessage::Single(decoded) => assert_eq!(decoded, original),
            other => panic!("{other:?}"),
        }
    });
}
