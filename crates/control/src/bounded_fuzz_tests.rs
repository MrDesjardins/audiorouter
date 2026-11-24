//! Q4.3 bounded fuzzing of the control plane's request-decoding limits and the
//! session-import plan, driven through `ControlPlane::dispatch` (the entry the
//! pipe, HTTP and MCP adapters all use) and through the upload base64 decoder.
//!
//! Fixed seeds and counts; `AUDIOROUTER_FUZZ_SCALE` lengthens a local run.
//! Only methods with no device, file or process side effects are dispatched:
//! the plan store is in memory and nothing here opens a pipe or an endpoint.
use super::*;
use crate::test_support::*;

#[path = "../../../tests/support/fuzz.rs"]
mod fuzz;

#[global_allocator]
static ALLOCATOR: fuzz::TrackingAllocator = fuzz::TrackingAllocator;

const SEED: u64 = 0x434F_4E54_524F_4C21; // "CONTROL!"

// ---------------------------------------------------------------------------
// Upload chunk base64
// ---------------------------------------------------------------------------

fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for group in bytes.chunks(3) {
        let n = (u32::from(group[0]) << 16)
            | (u32::from(*group.get(1).unwrap_or(&0)) << 8)
            | u32::from(*group.get(2).unwrap_or(&0));
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if group.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if group.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[test]
fn base64_chunks_round_trip_and_only_canonical_text_decodes() {
    let max_encoded = AUDIO_UPLOAD_CHUNK_BYTES * 4 / 3 + 8;
    // Every small length, then the chunk-size boundary.
    for length in (1..64).chain([AUDIO_UPLOAD_CHUNK_BYTES - 1, AUDIO_UPLOAD_CHUNK_BYTES]) {
        let bytes: Vec<u8> = (0..length).map(|index| (index * 7 + 3) as u8).collect();
        let text = encode_base64(&bytes);
        assert_eq!(
            decode_base64_chunk(&text).unwrap(),
            bytes,
            "length {length}"
        );
    }
    let mut accepted = 0_usize;
    fuzz::run("control/base64", SEED, 6_000, |_, rng| {
        let bytes = rng.blob(1, 300);
        let mut text = encode_base64(&bytes).into_bytes();
        for _ in 0..rng.below(4) {
            match rng.below(5) {
                0 if !text.is_empty() => {
                    let index = rng.below(text.len());
                    text[index] = *rng.pick(b"=+/-_ \n\0AZaz09\xFF");
                }
                1 if !text.is_empty() => {
                    text.truncate(rng.below(text.len()));
                }
                2 => {
                    let at = rng.below(text.len() + 1);
                    text.insert(at, *rng.pick(b"=A \n"));
                }
                3 if !text.is_empty() => {
                    let index = text.len() - 1;
                    text[index] = b'=';
                }
                _ => {
                    let extra = rng.below(8);
                    text.extend(std::iter::repeat_n(b'A', extra));
                }
            }
        }
        // The handler takes a `&str`; JSON strings are valid UTF-8.
        let text = String::from_utf8_lossy(&text).into_owned();
        let (result, largest) = fuzz::measure_allocation(|| decode_base64_chunk(&text));
        assert!(
            largest <= 2 * text.len() + 64,
            "{largest} for {}",
            text.len()
        );
        if let Ok(decoded) = result {
            accepted += 1;
            // Only canonical text decodes: re-encoding returns the input.
            assert_eq!(encode_base64(&decoded), text, "non-canonical text accepted");
            assert!(decoded.len() <= text.len() / 4 * 3);
        }
    });
    assert!(accepted > 500, "only {accepted} mutated chunks decoded");
    // Framing limits.
    for length in [
        0_usize,
        3,
        5,
        max_encoded - (max_encoded % 4),
        max_encoded + 4,
    ] {
        let text = "A".repeat(length);
        let result = decode_base64_chunk(&text);
        if length == 0 || length % 4 != 0 || length > max_encoded {
            assert!(result.is_err(), "{length}");
        }
    }
}

// ---------------------------------------------------------------------------
// Parameter budgets
// ---------------------------------------------------------------------------

fn random_tree(rng: &mut fuzz::Rng, depth: usize, budget: &mut usize) -> Value {
    *budget = budget.saturating_sub(1);
    let leaf = depth == 0 || *budget == 0 || rng.one_in(3);
    if leaf {
        return match rng.below(5) {
            0 => json!(null),
            1 => json!(rng.next_u32()),
            2 => json!(rng.one_in(2)),
            3 => json!("s".repeat(*rng.pick(&[
                0,
                1,
                100,
                MAX_CONTROL_STRING_BYTES,
                MAX_CONTROL_STRING_BYTES + 1
            ]))),
            _ => json!(1.5),
        };
    }
    let width = rng.range(0, 6);
    if rng.one_in(2) {
        Value::Array(
            (0..width)
                .map(|_| random_tree(rng, depth - 1, budget))
                .collect(),
        )
    } else {
        let mut object = serde_json::Map::new();
        for index in 0..width {
            let key = if rng.one_in(40) {
                "k".repeat(MAX_CONTROL_STRING_BYTES + 1)
            } else {
                format!("key{index}")
            };
            object.insert(key, random_tree(rng, depth - 1, budget));
        }
        Value::Object(object)
    }
}

/// An independent statement of the documented budget: at most
/// `MAX_CONTROL_VALUE_DEPTH` levels below the top, `MAX_CONTROL_VALUE_COUNT`
/// values and `MAX_CONTROL_STRING_BYTES` bytes per string or key.
fn within_budget(value: &Value, depth: usize, count: &mut usize) -> bool {
    *count += 1;
    if *count > MAX_CONTROL_VALUE_COUNT || depth > MAX_CONTROL_VALUE_DEPTH {
        return false;
    }
    match value {
        Value::String(text) => text.len() <= MAX_CONTROL_STRING_BYTES,
        Value::Array(items) => items
            .iter()
            .all(|item| within_budget(item, depth + 1, count)),
        Value::Object(items) => items.iter().all(|(key, item)| {
            key.len() <= MAX_CONTROL_STRING_BYTES && within_budget(item, depth + 1, count)
        }),
        _ => true,
    }
}

#[test]
fn parameter_budgets_accept_exactly_what_they_document() {
    let mut accepted = 0_usize;
    let mut refused = 0_usize;
    fuzz::run("control/param-budgets", SEED ^ 1, 3_000, |_, rng| {
        let mut budget = *rng.pick(&[20_usize, 400, 9_000]);
        let depth = rng.range(1, 40);
        let tree = random_tree(rng, depth, &mut budget);
        let params = match tree {
            Value::Object(_) => tree,
            other => json!({ "value": other }),
        };
        let expected = within_budget(&params, 0, &mut 0);
        // An unknown method skips the per-method allowlist, leaving the
        // budget as the only rule.
        let (result, largest) =
            fuzz::measure_allocation(|| validate_method_params("budget.probe", Some(&params)));
        assert!(largest <= 4096, "validation requested {largest} bytes");
        assert_eq!(
            result.is_ok(),
            expected,
            "budget disagreement for {}",
            serde_json::to_string(&params)
                .unwrap()
                .chars()
                .take(200)
                .collect::<String>()
        );
        if expected {
            accepted += 1;
        } else {
            refused += 1;
        }
    });
    assert!(accepted > 100, "only {accepted} trees were within budget");
    assert!(refused > 100, "only {refused} trees exceeded the budget");
}

/// The exact edges, one at a time.
#[test]
fn parameter_budget_edges_are_exact() {
    let nest = |levels: usize| {
        let mut value = json!("leaf");
        for _ in 0..levels {
            value = json!({ "n": value });
        }
        value
    };
    // `nest(n)` has n wrapping objects; the leaf sits at depth n.
    assert!(validate_method_params("budget.probe", Some(&nest(MAX_CONTROL_VALUE_DEPTH))).is_ok());
    assert!(
        validate_method_params("budget.probe", Some(&nest(MAX_CONTROL_VALUE_DEPTH + 1))).is_err()
    );
    // Values: the top object counts as one.
    let list = |count: usize| json!({ "a": vec![0; count] });
    assert!(
        validate_method_params("budget.probe", Some(&list(MAX_CONTROL_VALUE_COUNT - 2))).is_ok()
    );
    assert!(
        validate_method_params("budget.probe", Some(&list(MAX_CONTROL_VALUE_COUNT - 1))).is_err()
    );
    // Strings and keys.
    let text = |length: usize| json!({ "a": "x".repeat(length) });
    assert!(validate_method_params("budget.probe", Some(&text(MAX_CONTROL_STRING_BYTES))).is_ok());
    assert!(
        validate_method_params("budget.probe", Some(&text(MAX_CONTROL_STRING_BYTES + 1))).is_err()
    );
    let key = |length: usize| json!({ "k".repeat(length): 1 });
    assert!(validate_method_params("budget.probe", Some(&key(MAX_CONTROL_STRING_BYTES))).is_ok());
    assert!(
        validate_method_params("budget.probe", Some(&key(MAX_CONTROL_STRING_BYTES + 1))).is_err()
    );
    // Only objects are accepted, and the upload method bounds its payload by
    // the chunk limit instead of the string limit.
    for other in [json!([]), json!(1), json!("x"), json!(null), json!(true)] {
        assert!(
            validate_method_params("budget.probe", Some(&other)).is_err(),
            "{other}"
        );
    }
    let chunk = |length: usize| json!({ "dataBase64": "A".repeat(length) });
    let limit = AUDIO_UPLOAD_CHUNK_BYTES * 4 / 3 + 8;
    assert!(validate_method_params("audioMedia.uploadChunk", Some(&chunk(limit))).is_ok());
    assert!(validate_method_params("audioMedia.uploadChunk", Some(&chunk(limit + 1))).is_err());
    assert!(validate_method_params("audioMedia.uploadChunk", Some(&json!({}))).is_err());
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

fn mutate_value(rng: &mut fuzz::Rng, value: &mut Value) {
    let extremes = [
        json!(null),
        json!(-1),
        json!(u64::MAX),
        json!(1e308),
        json!(""),
        json!("x".repeat(5_000)),
        json!([]),
        json!({}),
        json!([1, 2, 3]),
        json!(true),
    ];
    // Walk to a random node and replace or remove it.
    let mut path: Vec<String> = Vec::new();
    let mut cursor = &*value;
    for _ in 0..rng.range(0, 4) {
        let next = match cursor {
            Value::Object(object) if !object.is_empty() => {
                let keys: Vec<&String> = object.keys().collect();
                let key = keys[rng.below(keys.len())].clone();
                path.push(key.clone());
                &object[&key]
            }
            Value::Array(items) if !items.is_empty() => {
                let index = rng.below(items.len());
                path.push(index.to_string());
                &items[index]
            }
            _ => break,
        };
        cursor = next;
    }
    let mut target = value;
    for step in &path {
        target = match target {
            Value::Object(object) => object.get_mut(step).unwrap(),
            Value::Array(items) => &mut items[step.parse::<usize>().unwrap()],
            _ => unreachable!(),
        };
    }
    match rng.below(4) {
        0 => *target = rng.pick(&extremes).clone(),
        1 => {
            if let Some(object) = target.as_object_mut() {
                let keys: Vec<String> = object.keys().cloned().collect();
                if let Some(key) = keys.get(rng.below(keys.len().max(1))) {
                    object.remove(key);
                }
            }
        }
        2 => {
            if let Some(items) = target.as_array_mut() {
                let copy = items.first().cloned().unwrap_or(json!(0));
                items.extend(std::iter::repeat_n(copy, rng.range(1, 40)));
            }
        }
        _ => {
            if let Some(object) = target.as_object_mut() {
                object.insert("injected".into(), rng.pick(&extremes).clone());
            }
        }
    }
}

fn request(id: u32, method: &str, params: Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(id)),
        method: method.into(),
        params: Some(params),
    }
}

fn assert_well_formed_response(response: &JsonRpcResponse, id: u32, method: &str, params: &Value) {
    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, Some(json!(id)));
    assert!(
        response.result.is_some() != response.error.is_some(),
        "{method}: exactly one of result and error"
    );
    if let Some(error) = &response.error {
        assert!(
            (-32_099..=-32_000).contains(&error.code) || (-32_700..=-32_600).contains(&error.code),
            "{method}: code {}",
            error.code
        );
        // A client's malformed input is never an internal error or a caught
        // panic, which would hide a defect behind a generic failure.
        let internal = error
            .data
            .as_ref()
            .and_then(|data| data.get("code"))
            .and_then(Value::as_str)
            == Some("internalError");
        assert!(
            !internal && !error.message.contains("panicked"),
            "{method}: {} for {}",
            error.message,
            serde_json::to_string(params)
                .unwrap()
                .chars()
                .take(400)
                .collect::<String>()
        );
    }
}

#[test]
fn mutated_import_plans_and_read_requests_stay_structured_and_commit_nothing() {
    let mut plane = ControlPlane::default();
    plane.insert_session(session()).unwrap();
    let listed = |plane: &mut ControlPlane| {
        plane
            .dispatch(request(0, "sessions.list", json!({})))
            .result
            .expect("sessions.list")
    };
    let before = listed(&mut plane);
    let seed_session = serde_json::to_value(session()).unwrap();
    let mut planned = 0_usize;
    let mut refused = 0_usize;
    fuzz::run("control/dispatch", SEED ^ 2, 1_200, |index, rng| {
        let id = index as u32 + 1;
        let (method, params) = match rng.below(4) {
            0 | 1 => {
                let mut candidate = seed_session.clone();
                candidate["id"] = json!(format!("candidate-{index}"));
                for _ in 0..rng.range(1, 3) {
                    mutate_value(rng, &mut candidate);
                }
                ("sessions.importPlan", json!({ "session": candidate }))
            }
            2 => {
                let mut params = json!({
                    "sessionId": *rng.pick(&["session", "ghost", "", "../etc"]),
                    "cursor": "c",
                    "limit": 5,
                });
                mutate_value(rng, &mut params);
                (*rng.pick(&["sessions.get", "sessions.list"]), params)
            }
            _ => {
                let mut params = json!({ "kind": "gain", "sessionId": "session" });
                mutate_value(rng, &mut params);
                (*rng.pick(&["nodes.describe", "nodes.types"]), params)
            }
        };
        let (response, largest) =
            fuzz::measure_allocation(|| plane.dispatch(request(id, method, params.clone())));
        assert!(
            largest <= 32 * 1024 * 1024,
            "{method} requested {largest} bytes"
        );
        assert_well_formed_response(&response, id, method, &params);
        if method == "sessions.importPlan" {
            if response.error.is_none() {
                planned += 1;
            } else {
                refused += 1;
            }
        }
    });
    // Planning an import commits nothing: the stored sessions are unchanged.
    assert_eq!(listed(&mut plane), before);
    assert!(planned > 20, "only {planned} mutated plans were accepted");
    assert!(refused > 100, "only {refused} mutated plans were refused");
}

/// The notification and batch edges of a request, through the same entry.
#[test]
fn malformed_requests_get_a_structured_refusal_not_a_panic() {
    let mut plane = ControlPlane::default();
    let cases = [
        JsonRpcRequest {
            jsonrpc: "1.0".into(),
            id: Some(json!(1)),
            method: "sessions.list".into(),
            params: None,
        },
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: String::new(),
            params: None,
        },
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!([1])),
            method: "sessions.list".into(),
            params: None,
        },
        // A mutating notification must not run.
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "sessions.importPlan".into(),
            params: Some(json!({ "session": serde_json::to_value(session()).unwrap() })),
        },
        request(9, "no.such.method", json!({})),
        request(10, "sessions.list", json!([1, 2])),
        request(11, "sessions.importPlan", json!({})),
        request(
            12,
            "sessions.importPlan",
            json!({ "session": "not a session" }),
        ),
        request(13, "sessions.importPlan", json!({ "session": null })),
    ];
    for case in cases {
        let response = plane.dispatch(case);
        assert!(response.error.is_some(), "{response:?}");
    }
}
