//! Q4.1: the MCP adapter held to the backend's contract.
//!
//! `call_api` reaches every method the backend publishes, so the generated
//! cases of `contract_cases.rs` run through the real `mcp_tool_call` entry
//! point and must answer exactly what `ControlPlane::dispatch_authorized`
//! answers for the same request and grant. The default tools are then checked
//! the same way: each one's first backend call is authorized by the caller's
//! role, not by the adapter.

use super::*;
use crate::contract_cases::*;
use audiorouter_control::{ClientGrant, ClientRole, ControlPlane};
use audiorouter_domain::API_METHODS;

fn every_scope() -> ClientGrant {
    ClientGrant::with_scopes(API_METHODS.iter().map(|spec| spec.permission))
}

fn plane() -> ControlPlane {
    // Backed by a database, as the app is: storage validates the keys it
    // journals, which an in-memory plane never does.
    let mut plane = ControlPlane::with_storage(
        "contract",
        audiorouter_storage::Storage::open_memory().unwrap(),
    );
    plane.insert_session(fixture_session("contract")).unwrap();
    plane
}

fn refuses(method: &str) -> Option<&'static str> {
    // Only the desktop window may change device consent.
    (method == "devices.setAccess").then_some("permissionDenied")
}

/// `ControlPlane::dispatch_authorized`, the reference every adapter must match.
fn direct(grant: ClientGrant) -> FnAdapter<'static> {
    let mut plane = plane();
    let mut next = 0;
    FnAdapter {
        call_fn: Box::new(move |method, params| {
            next += 1;
            let request = audiorouter_protocol::JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(next)),
                method: method.into(),
                params,
            };
            Outcome::from_response(&plane.dispatch_authorized(request, &grant))
        }),
        refuse_fn: Box::new(refuses),
    }
}

/// The MCP `call_api` tool through `mcp_tool_call`. With `rotate`, a new client
/// identity is used every 30 calls so a long sweep never meets the per-client
/// mutation rate limit; scenarios that retry keep one identity.
fn mcp(grant: ClientGrant, rotate: bool) -> FnAdapter<'static> {
    let mut plane = plane();
    let mut next = 0u64;
    FnAdapter {
        call_fn: Box::new(move |method, params| {
            next += 1;
            let client = if rotate {
                format!("contract-{}", next / 30)
            } else {
                "contract".to_owned()
            };
            let mut arguments = json!({ "method": method });
            if let Some(params) = params {
                arguments["params"] = params;
            }
            let reply = mcp_tool_call(
                &mut plane,
                &client,
                &grant,
                None,
                &json!({ "id": next, "params": { "name": "call_api", "arguments": arguments } }),
            );
            let result = &reply["result"];
            if result["isError"] == true {
                // An adapter-level failure has no backend error behind it.
                let error = &result["structuredContent"]["error"];
                assert!(
                    error.is_object(),
                    "{method}: MCP reported a failure without the backend error: {reply}"
                );
                return Outcome::from_error_value(error);
            }
            let mut outcome =
                Outcome::from_response(&audiorouter_protocol::JsonRpcResponse::success(
                    None,
                    result["structuredContent"]["result"].clone(),
                ));
            outcome.status = None;
            outcome
        }),
        refuse_fn: Box::new(refuses),
    }
}

fn roles() -> [(&'static str, ClientGrant); 4] {
    [
        ("none", ClientGrant::with_scopes([])),
        ("observer", ClientGrant::for_role(ClientRole::Observer)),
        ("editor", ClientGrant::for_role(ClientRole::Editor)),
        ("operator", ClientGrant::for_role(ClientRole::Operator)),
    ]
}

#[test]
fn mcp_publishes_the_same_method_schemas_as_the_backend() {
    let backend = describe_methods(&mut direct(every_scope()));
    let through_mcp = describe_methods(&mut mcp(every_scope(), false));
    assert_eq!(backend.len(), API_METHODS.len());
    assert_eq!(backend.len(), through_mcp.len());
    for (a, b) in backend.iter().zip(&through_mcp) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.input_schema, b.input_schema, "{}", a.name);
    }
}

#[test]
fn mcp_refuses_invalid_input_exactly_as_the_backend_does() {
    let methods = describe_methods(&mut direct(every_scope()));
    let failures = invalid_input_failures(&mut mcp(every_scope(), true), &methods);
    assert!(
        failures.is_empty(),
        "{} contract failures through MCP:\n{}",
        failures.len(),
        failures.join("\n")
    );
    let failures = parity_failures(
        &mut mcp(every_scope(), true),
        &mut direct(every_scope()),
        &methods,
        "MCP",
    );
    assert!(
        failures.is_empty(),
        "{} differences between MCP and dispatch:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn mcp_refuses_every_method_a_role_lacks_the_permission_for() {
    let methods = describe_methods(&mut direct(every_scope()));
    let mut failures = Vec::new();
    for (role, grant) in roles() {
        let expected = authorization_results(&mut direct(grant.clone()), &methods);
        let actual = authorization_results(&mut mcp(grant, true), &methods);
        let denied = expected
            .iter()
            .filter(|(_, outcome)| outcome.category == "permissionDenied")
            .count();
        // Without both outcomes present the comparison would prove nothing.
        if role == "none" {
            assert_eq!(denied, methods.len(), "{role}: something was allowed");
        } else {
            assert!(denied < methods.len(), "{role}: nothing was allowed");
        }
        if role != "operator" {
            assert!(denied > 0, "{role}: nothing was refused");
        }
        for ((name, expected), (_, actual)) in expected.iter().zip(&actual) {
            if actual.label() != expected.label() || actual.message != expected.message {
                failures.push(format!(
                    "{role}: {name}: MCP answered {} `{}`, dispatch {} `{}`",
                    actual.label(),
                    actual.message,
                    expected.label(),
                    expected.message
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A default tool's first backend call is authorized by the caller's role: an
/// Observer is refused every tool whose first call needs more than Read, and
/// is never refused one that needs only Read. The tool-to-method mapping is
/// read from the calls the adapter really makes, so a new tool is covered.
#[test]
fn every_default_tool_follows_the_callers_role() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for (role, grant) in roles() {
        let mut plane = plane();
        for tool in mcp_listed_tools(false).as_array().unwrap() {
            let name = tool["name"].as_str().unwrap();
            tests::MCP_BACKEND_CALLS.with(|calls| calls.borrow_mut().clear());
            let reply = mcp_tool_call(
                &mut plane,
                "contract",
                &grant,
                None,
                &json!({ "id": 1, "params": { "name": name, "arguments": {} } }),
            );
            let calls = tests::MCP_BACKEND_CALLS.with(|calls| calls.borrow().clone());
            let Some(first) = calls.first() else {
                continue;
            };
            // The reference answer for a probe that names no real parameter is
            // either a permission refusal or an input refusal, never both.
            let mut reference = direct(grant.clone());
            let mut refused_for_role = |method: &String| -> bool {
                reference
                    .call(method, Some(json!({ "zzUnknown": 1 })))
                    .category
                    == "permissionDenied"
            };
            let first_refused = refused_for_role(first);
            let any_refused = calls.iter().any(&mut refused_for_role);
            let error = &reply["result"]["structuredContent"]["error"];
            let denied = error["data"]["code"] == "permissionDenied";
            // The tool stops at the first call the role may not make...
            if first_refused && !denied {
                failures.push(format!(
                    "{role}: tool {name} ({calls:?}) was not refused: {reply}"
                ));
            }
            // ... and a refusal always traces back to a call the role lacks.
            if denied && !any_refused {
                failures.push(format!(
                    "{role}: tool {name} ({calls:?}) was refused though the role may make every call: {reply}"
                ));
            }
            checked += 1;
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // Most default tools call the backend with no arguments, so the sweep must
    // really have reached it for each role.
    assert!(
        checked > 4 * 40,
        "only {checked} tool/role pairs reached the backend"
    );
}

/// Stale revisions are the same `revisionConflict` through MCP.
#[test]
fn mcp_reports_a_stale_revision_as_a_revision_conflict() {
    let mut adapter = mcp(every_scope(), false);
    let outcomes = stale_revision_outcomes(&mut adapter);
    assert!(!outcomes.is_empty());
    for (method, outcome) in outcomes {
        assert_eq!(
            outcome.category, "revisionConflict",
            "{method}: {outcome:?}"
        );
        assert_eq!(outcome.rpc_code, -32000, "{method}");
    }
}

/// A retried mutation is applied once through MCP, and the default tools take
/// a missing idempotency key from the adapter without failing.
#[test]
fn mcp_retries_with_the_same_operation_key_apply_once() {
    let runs = retry_runs(|| Box::new(mcp(every_scope(), false)), None);
    let failures = retry_failures(&runs);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // ... and with the longest key the schemas allow: the client identity the
    // backend scopes a key with must not push it over the storage limit.
    let runs = retry_runs(|| Box::new(mcp(every_scope(), false)), Some(LONGEST_KEY));
    let failures = retry_failures(&runs);
    assert!(
        failures.is_empty(),
        "with a {LONGEST_KEY}-byte key:\n{}",
        failures.join("\n")
    );

    // The same through a default tool: add_tool with an explicit key.
    let grant = ClientGrant::for_role(ClientRole::Editor);
    let mut plane = plane();
    let mut add = |kind: &str| {
        mcp_tool_call(
            &mut plane,
            "contract",
            &grant,
            None,
            &json!({ "id": 1, "params": { "name": "add_tool", "arguments": {
                "sessionId": "contract", "kind": kind, "idempotencyKey": "tool-retry"
            } } }),
        )
    };
    let first = add("gain");
    let again = add("gain");
    let changed = add("gate");
    assert_eq!(first["result"]["isError"], false, "{first}");
    assert_eq!(
        first["result"], again["result"],
        "retry answered differently"
    );
    assert_eq!(
        changed["result"]["structuredContent"]["error"]["data"]["code"], "idempotencyConflict",
        "{changed}"
    );
    let session = plane
        .dispatch(audiorouter_protocol::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "sessions.get".into(),
            params: Some(json!({ "sessionId": "contract" })),
        })
        .result
        .unwrap();
    let gains = session["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["kind"] == "gain")
        .count();
    assert_eq!(gains, 1, "the retried add_tool applied twice");
    assert_eq!(session["nodes"].as_array().unwrap().len(), 3);
}

/// The tools that create a session and then open it derive a second operation
/// key from the caller's. With the longest key the schemas allow, that derived
/// key passed the storage limit: the session was created, the open step was
/// refused, and the caller was told the whole call failed. A retry must replay.
#[test]
fn creating_a_session_with_the_longest_key_opens_it_and_retries_cleanly() {
    // Opening a session needs SessionControl, which only an Operator has.
    let grant = ClientGrant::for_role(ClientRole::Operator);
    let key = "k".repeat(LONGEST_KEY);
    for (tool, arguments) in [
        ("create_session", json!({ "name": "Long key" })),
        ("start_from_goal", json!({ "goal": "clean-mic" })),
    ] {
        let mut plane = plane();
        let mut arguments = arguments;
        arguments["idempotencyKey"] = json!(key);
        let call = |plane: &mut ControlPlane| {
            mcp_tool_call(
                plane,
                "contract",
                &grant,
                None,
                &json!({ "id": 1, "params": { "name": tool, "arguments": arguments } }),
            )
        };
        let first = call(&mut plane);
        assert_eq!(first["result"]["isError"], false, "{tool}: {first}");
        let again = call(&mut plane);
        assert_eq!(
            again["result"]["isError"], false,
            "{tool}: the retry failed: {again}"
        );
        assert_eq!(first["result"], again["result"], "{tool}: retry differs");
        let sessions = plane
            .dispatch(audiorouter_protocol::JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "sessions.list".into(),
                params: None,
            })
            .result
            .unwrap();
        // The fixture session plus exactly one new one, which is open.
        assert_eq!(
            sessions["items"].as_array().map(Vec::len),
            Some(2),
            "{tool}: {sessions}"
        );
    }
}

#[test]
fn the_open_step_key_stays_within_the_limit_and_is_stable() {
    for length in [1, 100, 123, 124, 128] {
        let key = "a".repeat(length);
        let derived = mcp_open_key(&key);
        assert!(derived.len() <= 128, "{length}: {} bytes", derived.len());
        assert_eq!(derived, mcp_open_key(&key), "{length}: not deterministic");
    }
    // Keys that fit keep the readable form earlier records were journaled with.
    assert_eq!(mcp_open_key("create-1"), "create-1-open");
    assert_ne!(
        mcp_open_key(&"a".repeat(128)),
        mcp_open_key(&"b".repeat(128))
    );
}
