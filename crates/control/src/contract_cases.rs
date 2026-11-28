//! Adapter contract cases (plan Q4.1), shared by every adapter's tests.
//!
//! The control crate's tests include this file directly, and the CLI/MCP and
//! shell HTTP tests include the very same source with `#[path]`, so all three
//! adapters are held to one table of cases generated from the published
//! method schemas (`system.describe`), never from a hand-kept method list.
//!
//! An adapter implements [`Adapter`]: one call in, one [`Outcome`] out. The
//! runners below assert the backend's documented error categories and that
//! an adapter reports exactly what `ControlPlane::dispatch` reports.
#![allow(dead_code)]

use audiorouter_protocol::JsonRpcResponse;
use serde_json::{json, Value};

/// One string byte over the backend's `MAX_CONTROL_STRING_BYTES` (4096). The
/// control crate's own test pins this to the real constant.
pub const OVERSIZE_STRING_BYTES: usize = 4097;
/// Deeper than the backend's `MAX_CONTROL_VALUE_DEPTH` (32) and shallower than
/// the JSON parser's own limit (128), so the backend rule decides.
pub const DEEP_NESTING: usize = 40;

/// What a caller saw for one request, whatever the adapter.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub ok: bool,
    /// JSON-RPC error code, 0 on success.
    pub rpc_code: i64,
    /// `error.data.code`, empty when absent.
    pub category: String,
    pub message: String,
    pub result: Value,
    /// The HTTP status, for the HTTP adapter only.
    pub status: Option<u16>,
    /// The adapter itself failed (not a backend answer).
    pub panicked: bool,
}

impl Outcome {
    pub fn from_response(response: &JsonRpcResponse) -> Self {
        match &response.error {
            None => Self {
                ok: true,
                rpc_code: 0,
                category: String::new(),
                message: String::new(),
                result: response.result.clone().unwrap_or(Value::Null),
                status: None,
                panicked: false,
            },
            Some(error) => Self {
                ok: false,
                rpc_code: i64::from(error.code),
                category: error
                    .data
                    .as_ref()
                    .and_then(|data| data["code"].as_str())
                    .unwrap_or_default()
                    .to_owned(),
                message: error.message.clone(),
                result: Value::Null,
                status: None,
                panicked: false,
            },
        }
    }

    /// An `{ code?, message, data? }` error object as MCP and HTTP carry it.
    pub fn from_error_value(error: &Value) -> Self {
        Self {
            ok: false,
            rpc_code: error["code"].as_i64().unwrap_or(0),
            category: error["data"]["code"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            message: error["message"].as_str().unwrap_or_default().to_owned(),
            result: Value::Null,
            status: None,
            panicked: false,
        }
    }

    pub fn panic() -> Self {
        Self {
            ok: false,
            rpc_code: i64::MIN,
            category: "PANIC".into(),
            message: "the adapter or backend panicked".into(),
            result: Value::Null,
            status: None,
            panicked: true,
        }
    }

    /// The category a script branches on: the backend code, or the JSON-RPC
    /// class when the backend gave none.
    pub fn label(&self) -> String {
        if self.ok {
            "ok".into()
        } else if self.category.is_empty() {
            format!("rpc{}", self.rpc_code)
        } else {
            format!("{}/{}", self.rpc_code, self.category)
        }
    }
}

/// One way to reach the backend: dispatch itself, the MCP `call_api` tool or
/// the HTTP API. Each adapter is bound to one grant when it is built.
pub trait Adapter {
    fn call(&mut self, method: &str, params: Option<Value>) -> Outcome;

    /// A category this adapter itself answers for `method` before the request
    /// can reach input validation (window-only methods are refused to every
    /// non-desktop grant). `None` when the backend decides.
    fn refuses_before_validation(&self, _method: &str) -> Option<&'static str> {
        None
    }

    /// The parameters as this adapter can express them (HTTP always sends a
    /// JSON object, so omitted parameters become `{}`).
    fn normalize_params(&self, params: Option<Value>) -> Option<Value> {
        params
    }

    /// Cases this adapter answers itself, with its own message, before the
    /// backend sees them: still checked for the category, not for the text.
    fn answers_itself(&self, _case_label: &str) -> bool {
        false
    }
}

/// An [`Adapter`] built from closures, for adapters whose backend lives in the
/// test (an in-memory `ControlPlane`) or behind a socket.
pub struct FnAdapter<'a> {
    pub call_fn: CallFn<'a>,
    pub refuse_fn: RefuseFn<'a>,
}

pub type CallFn<'a> = Box<dyn FnMut(&str, Option<Value>) -> Outcome + 'a>;
pub type RefuseFn<'a> = Box<dyn Fn(&str) -> Option<&'static str> + 'a>;

impl Adapter for FnAdapter<'_> {
    fn call(&mut self, method: &str, params: Option<Value>) -> Outcome {
        (self.call_fn)(method, params)
    }

    fn refuses_before_validation(&self, method: &str) -> Option<&'static str> {
        (self.refuse_fn)(method)
    }
}

/// A method as published by `system.describe`.
#[derive(Clone, Debug)]
pub struct Method {
    pub name: String,
    pub permission: String,
    pub side_effect: String,
    pub input_schema: Value,
}

impl Method {
    pub fn properties(&self) -> serde_json::Map<String, Value> {
        self.input_schema["properties"]
            .as_object()
            .cloned()
            .unwrap_or_default()
    }

    pub fn required(&self) -> Vec<String> {
        self.input_schema["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|name| name.as_str().map(str::to_owned))
            .collect()
    }

    /// External operations may open devices, files or sockets; their cases
    /// never carry a plausible-looking base request.
    pub fn is_external(&self) -> bool {
        self.side_effect == "externalOperation"
    }
}

/// Every method an adapter publishes, read through the adapter itself.
pub fn describe_methods(adapter: &mut dyn Adapter) -> Vec<Method> {
    let outcome = adapter.call("system.describe", None);
    assert!(outcome.ok, "system.describe failed: {outcome:?}");
    outcome.result["methods"]
        .as_array()
        .expect("system.describe lists methods")
        .iter()
        .map(|method| Method {
            name: method["name"].as_str().unwrap().to_owned(),
            permission: method["permission"].as_str().unwrap().to_owned(),
            side_effect: method["sideEffect"].as_str().unwrap().to_owned(),
            input_schema: method["inputSchema"].clone(),
        })
        .collect()
}

fn type_names(property: &Value) -> Vec<&str> {
    match &property["type"] {
        Value::String(name) => vec![name.as_str()],
        Value::Array(names) => names.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

fn matches_type(name: &str, value: &Value) -> bool {
    match name {
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "null" => value.is_null(),
        _ => false,
    }
}

/// A value that satisfies a property's declared type (enum first).
pub fn placeholder(property: &Value) -> Value {
    if let Some(first) = property["enum"].get(0) {
        return first.clone();
    }
    match type_names(property).first().copied() {
        Some("integer" | "number") => property.get("minimum").cloned().unwrap_or(json!(1)),
        Some("boolean") => json!(false),
        Some("object") => json!({}),
        Some("array") => json!([]),
        Some("null") => Value::Null,
        _ => json!("q4-contract-probe"),
    }
}

/// A value of the wrong JSON type for a property, or `None` when the schema
/// declares neither a type nor an enum (nothing to violate).
pub fn wrong_type(property: &Value) -> Option<Value> {
    let types = type_names(property);
    let candidates = [json!(7), json!("text"), json!(true), json!([]), json!({})];
    if let Some(allowed) = property["enum"].as_array() {
        return candidates
            .into_iter()
            .find(|value| !allowed.contains(value));
    }
    if types.is_empty() {
        return None;
    }
    candidates
        .into_iter()
        .find(|value| !types.iter().any(|name| matches_type(name, value)))
}

/// How strictly an invalid request must be refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Expect {
    /// The request is malformed: `-32602`, no application category.
    InvalidParams,
    /// Refused with `-32602` or a documented application category (the base
    /// request names resources that do not exist, so "not found" is also a
    /// correct refusal of the same request).
    Refused,
}

#[derive(Clone, Debug)]
pub struct InvalidCase {
    pub label: String,
    pub params: Option<Value>,
    pub expect: Expect,
}

fn nested(depth: usize) -> Value {
    let mut value = json!("leaf");
    for _ in 0..depth {
        value = json!({ "nested": value });
    }
    value
}

/// Cases generated from one method's schema.
pub fn invalid_cases(method: &Method) -> Vec<InvalidCase> {
    let properties = method.properties();
    let required = method.required();
    let mut base = serde_json::Map::new();
    for name in &required {
        base.insert(name.clone(), placeholder(&properties[name]));
    }
    // A request that is otherwise plausible gives each mutation of one field
    // a chance to be (wrongly) accepted; an external operation gets the field
    // alone, so no case can reach a device, file or socket.
    let seed = |field: &str, value: Value| {
        let mut params = if method.is_external() {
            serde_json::Map::new()
        } else {
            base.clone()
        };
        params.insert(field.to_owned(), value);
        Some(Value::Object(params))
    };
    let mut cases = vec![
        InvalidCase {
            label: "params-not-an-object".into(),
            params: Some(json!(["not", "an", "object"])),
            expect: Expect::InvalidParams,
        },
        InvalidCase {
            label: "deep-nesting".into(),
            params: Some(json!({ "zzDeep": nested(DEEP_NESTING) })),
            expect: Expect::InvalidParams,
        },
        InvalidCase {
            label: "unknown-parameter".into(),
            params: Some(json!({ "zzUnknown": 1 })),
            expect: Expect::InvalidParams,
        },
    ];
    if !required.is_empty() {
        cases.push(InvalidCase {
            label: "params-omitted".into(),
            params: None,
            expect: Expect::Refused,
        });
    }
    for name in &required {
        let mut params = base.clone();
        params.remove(name);
        // Without the field there is nothing left to execute for a single
        // required field; keep the rest for the plausible-request check.
        if method.is_external() {
            params.clear();
        }
        cases.push(InvalidCase {
            label: format!("missing:{name}"),
            params: Some(Value::Object(params)),
            expect: Expect::Refused,
        });
    }
    for (name, property) in &properties {
        if let Some(value) = wrong_type(property) {
            cases.push(InvalidCase {
                label: format!("wrong-type:{name}"),
                params: seed(name, value),
                expect: Expect::Refused,
            });
        }
        if type_names(property).contains(&"string") {
            if property["minLength"].as_u64().is_some_and(|min| min >= 1) {
                cases.push(InvalidCase {
                    label: format!("empty-string:{name}"),
                    params: seed(name, json!("")),
                    expect: Expect::Refused,
                });
            }
            if let Some(max) = property["maxLength"]
                .as_u64()
                .filter(|max| (*max as usize) < OVERSIZE_STRING_BYTES - 1)
            {
                cases.push(InvalidCase {
                    label: format!("over-maxLength:{name}"),
                    params: seed(name, json!("x".repeat(max as usize + 1))),
                    expect: Expect::Refused,
                });
            }
            cases.push(InvalidCase {
                label: format!("oversize-string:{name}"),
                params: seed(name, json!("x".repeat(OVERSIZE_STRING_BYTES))),
                expect: Expect::InvalidParams,
            });
        }
    }
    cases
}

/// Rust Debug syntax, panic text and source paths a caller must never see.
pub const LEAKS: &[&str] = &[
    "InvalidRequest(",
    "Internal(",
    "Store(",
    "Storage(",
    "Json(",
    "Audio {",
    "ControlError",
    "StoreError",
    "Error {",
    "Some(",
    "None",
    "Err(",
    "Ok(",
    "panicked",
    "called `",
    "RUST_BACKTRACE",
    "src\\",
    "src/",
    ".rs:",
    "kind: ",
    "ParseIntError",
    "ParseFloatError",
    "Utf8Error",
    "unwrap",
];

pub fn assert_plain_message(method: &str, label: &str, outcome: &Outcome) {
    assert!(
        !outcome.message.trim().is_empty(),
        "{method} {label}: an error needs a message"
    );
    assert!(
        outcome.message.len() <= 2000,
        "{method} {label}: error message is {} bytes",
        outcome.message.len()
    );
    for leak in LEAKS {
        assert!(
            !outcome.message.contains(leak),
            "{method} {label}: error message leaks `{leak}`: {}",
            outcome.message
        );
    }
}

/// Backend categories that mean "the request was refused as given". Anything
/// else (internalError, storageFailure, corruptDatabase) means the backend
/// failed on input the caller could have been told about.
pub const REFUSAL_CATEGORIES: &[&str] = &[
    "invalidRequest",
    "notFound",
    "invalidGraph",
    "permissionDenied",
    "nodeNotReady",
    "idempotencyConflict",
    "revisionConflict",
    "sessionNotStopped",
];

/// Run every generated invalid case for every method against `adapter` and
/// return the failures (so one run reports them all).
pub fn invalid_input_failures(adapter: &mut dyn Adapter, methods: &[Method]) -> Vec<String> {
    let mut failures = Vec::new();
    for method in methods {
        let refusal = adapter.refuses_before_validation(&method.name);
        for case in invalid_cases(method) {
            let outcome = adapter.call(&method.name, adapter.normalize_params(case.params.clone()));
            let here = format!("{} [{}]", method.name, case.label);
            if outcome.panicked {
                failures.push(format!("{here}: panicked"));
                continue;
            }
            if let Some(category) = refusal {
                if outcome.category != category {
                    failures.push(format!(
                        "{here}: expected the adapter's own `{category}` refusal, got {}",
                        outcome.label()
                    ));
                }
                continue;
            }
            if outcome.ok {
                failures.push(format!(
                    "{here}: invalid input was accepted: {}",
                    outcome
                        .result
                        .to_string()
                        .chars()
                        .take(160)
                        .collect::<String>()
                ));
                continue;
            }
            let strict = case.expect == Expect::InvalidParams;
            let good = if strict {
                outcome.rpc_code == -32602
            } else {
                outcome.rpc_code == -32602
                    || (outcome.rpc_code == -32000
                        && REFUSAL_CATEGORIES.contains(&outcome.category.as_str()))
            };
            if !good {
                failures.push(format!(
                    "{here}: expected {} but got {} ({})",
                    if strict {
                        "-32602"
                    } else {
                        "-32602 or a refusal category"
                    },
                    outcome.label(),
                    outcome.message
                ));
                continue;
            }
            let leak = LEAKS.iter().find(|leak| outcome.message.contains(**leak));
            if outcome.message.trim().is_empty() || leak.is_some() {
                failures.push(format!(
                    "{here}: unclear error message `{}` (leaks {leak:?})",
                    outcome.message
                ));
            }
        }
    }
    failures
}

/// Compare an adapter with the reference (dispatch): the same request must
/// give the same category and message through both.
pub fn parity_failures(
    adapter: &mut dyn Adapter,
    reference: &mut dyn Adapter,
    methods: &[Method],
    name: &str,
) -> Vec<String> {
    let mut failures = Vec::new();
    for method in methods {
        for case in invalid_cases(method) {
            let params = adapter.normalize_params(case.params.clone());
            let expected = reference.call(&method.name, params.clone());
            let actual = adapter.call(&method.name, params);
            let here = format!("{} [{}]", method.name, case.label);
            if let Some(category) = adapter.refuses_before_validation(&method.name) {
                if actual.category != category {
                    failures.push(format!("{here}: {name} answered {}", actual.label()));
                }
                continue;
            }
            if adapter.answers_itself(&case.label) {
                // The adapter names its own category (`invalidRequest`), the
                // dispatcher gives a bare JSON-RPC -32602: both are "invalid
                // params", which is the shared contract.
                if actual.rpc_code != expected.rpc_code {
                    failures.push(format!(
                        "{here}: {name} answered {} but dispatch answered {}",
                        actual.label(),
                        expected.label()
                    ));
                }
                continue;
            }
            if actual.label() != expected.label() || actual.message != expected.message {
                failures.push(format!(
                    "{here}: {name} answered {} `{}` but dispatch answered {} `{}`",
                    actual.label(),
                    actual.message,
                    expected.label(),
                    expected.message
                ));
            }
        }
    }
    failures
}

/// The parameter every authorization probe sends: refused as unknown once the
/// caller is allowed, so no allowed probe can execute anything.
pub fn authorization_probe() -> Option<Value> {
    Some(json!({ "zzUnknown": 1 }))
}

/// For each method: was the probe refused for permission (`true`) or did it
/// get past authorization (`false`)?
pub fn authorization_results(
    adapter: &mut dyn Adapter,
    methods: &[Method],
) -> Vec<(String, Outcome)> {
    methods
        .iter()
        .map(|method| {
            (
                method.name.clone(),
                adapter.call(&method.name, authorization_probe()),
            )
        })
        .collect()
}

/// A saved route to run the revision and retry scenarios on: a stereo input
/// feeding a stereo output.
pub fn fixture_session(id: &str) -> audiorouter_domain::Session {
    serde_json::from_value(json!({
        "id": id,
        "name": "Contract fixture",
        "schemaVersion": 1,
        "revision": 0,
        "nodes": [
            { "id": "in", "kind": "physicalInput", "typeVersion": 1, "name": "Mic in",
              "enabled": true, "bypass": false,
              "ports": [{ "name": "out", "direction": "output", "channels": 2 }] },
            { "id": "out", "kind": "physicalOutput", "typeVersion": 1, "name": "Speakers out",
              "enabled": true, "bypass": false,
              "ports": [{ "name": "in", "direction": "input", "channels": 2 }] }
        ],
        "edges": [{
            "id": "in-out", "sourceNode": "in", "sourcePort": "out",
            "destinationNode": "out", "destinationPort": "in",
            "matrix": [1.0, 0.0, 0.0, 1.0], "enabled": true
        }]
    }))
    .expect("contract fixture session")
}

// ---------------------------------------------------------------------------
// Stale revisions
// ---------------------------------------------------------------------------

/// Methods whose published input carries a `baseRevision` (the revision the
/// caller's change was based on).
pub fn revision_guarded(methods: &[Method]) -> Vec<String> {
    methods
        .iter()
        .filter(|method| method.properties().contains_key("baseRevision"))
        .map(|method| method.name.clone())
        .collect()
}

/// Every revision-guarded method, called with a revision that is no longer
/// current. Returns what each answered, for the caller to check against the
/// documented `revisionConflict`. The adapter must hold every scope.
pub fn stale_revision_outcomes(adapter: &mut dyn Adapter) -> Vec<(String, Outcome)> {
    let mut outcomes = Vec::new();
    let session = adapter
        .call("sessions.get", Some(json!({ "sessionId": "contract" })))
        .result;
    let mut renamed = session.clone();
    renamed["name"] = json!("Renamed once");
    let plan = |adapter: &mut dyn Adapter, base: u64, candidate: &Value| {
        adapter.call(
            "graph.plan",
            Some(json!({ "sessionId": "contract", "baseRevision": base, "candidate": candidate })),
        )
    };
    // Two plans made at revision 0; the first commit wins, the second is stale.
    let first = plan(adapter, 0, &renamed);
    let mut renamed_again = session.clone();
    renamed_again["name"] = json!("Renamed twice");
    let second = plan(adapter, 0, &renamed_again);
    assert!(first.ok && second.ok, "{first:?} {second:?}");
    let commit = |adapter: &mut dyn Adapter, plan: &Outcome, base: u64, key: &str| {
        adapter.call(
            "graph.commit",
            Some(json!({ "planId": plan.result["planId"], "baseRevision": base, "idempotencyKey": key })),
        )
    };
    let committed = commit(adapter, &first, 0, "contract-first");
    assert!(committed.ok, "{committed:?}");
    outcomes.push((
        "graph.commit".into(),
        commit(adapter, &second, 0, "contract-second"),
    ));
    outcomes.push(("graph.plan".into(), plan(adapter, 0, &renamed_again)));
    outcomes.push((
        "graph.undoPlan".into(),
        adapter.call(
            "graph.undoPlan",
            Some(json!({ "sessionId": "contract", "baseRevision": 0 })),
        ),
    ));
    outcomes.push((
        "graph.restorePlan".into(),
        adapter.call(
            "graph.restorePlan",
            Some(json!({ "sessionId": "contract", "baseRevision": 0, "revision": 1 })),
        ),
    ));
    outcomes.push((
        "virtualRoutes.replace".into(),
        adapter.call(
            "virtualRoutes.replace",
            Some(json!({ "baseRevision": 99, "routes": [], "idempotencyKey": "contract-routes" })),
        ),
    ));
    outcomes
}

// ---------------------------------------------------------------------------
// Idempotent retries
// ---------------------------------------------------------------------------

/// One mutation to repeat with the same operation key.
pub struct Retry {
    pub method: &'static str,
    pub params: Value,
    /// The same key with different content, which must be refused.
    pub changed: Value,
}

/// Calls that put the fixture in the state a retry scenario needs.
pub fn retry_setup(method: &str) -> Vec<(&'static str, Value)> {
    match method {
        // The fixture is already the active session: open a copy instead.
        "sessions.active.set" => vec![(
            "sessions.duplicate",
            json!({ "sourceSessionId": "contract", "sessionId": "other-open", "idempotencyKey": "setup-copy" }),
        )],
        // Connect what the fixture connects, so the call has something to add.
        "connections.add" => vec![(
            "connections.remove",
            json!({ "sessionId": "contract", "from": "in", "to": "out", "idempotencyKey": "setup-disconnect" }),
        )],
        _ => Vec::new(),
    }
}

/// What a caller can read back: every saved session and the active one.
pub fn saved_state(adapter: &mut dyn Adapter) -> Value {
    json!({
        "sessions": adapter.call("sessions.list", Some(json!({}))).result,
        "active": adapter.call("sessions.active.get", None).result,
        "presets": adapter.call("toolPresets.list", Some(json!({}))).result,
        // The privacy latch is read back through the session summary.
        "summary": adapter
            .call("sessions.summary", Some(json!({ "sessionId": "contract" })))
            .result,
    })
}

/// The mutations the retry contract is exercised on, each with a visible
/// effect on `saved_state`.
pub fn retries() -> Vec<Retry> {
    let created = |id: &str| serde_json::to_value(fixture_session(id)).unwrap();
    vec![
        Retry {
            method: "nodes.add",
            params: json!({ "sessionId": "contract", "kind": "gain", "idempotencyKey": "retry-add" }),
            changed: json!({ "sessionId": "contract", "kind": "gate", "idempotencyKey": "retry-add" }),
        },
        Retry {
            method: "nodes.set",
            params: json!({ "sessionId": "contract", "node": "Mic in", "name": "Renamed mic", "idempotencyKey": "retry-set" }),
            changed: json!({ "sessionId": "contract", "node": "Mic in", "name": "Another name", "idempotencyKey": "retry-set" }),
        },
        Retry {
            method: "nodes.toggle",
            params: json!({ "sessionId": "contract", "node": "Speakers out", "target": "bypass", "idempotencyKey": "retry-toggle" }),
            changed: json!({ "sessionId": "contract", "node": "Speakers out", "target": "enabled", "idempotencyKey": "retry-toggle" }),
        },
        Retry {
            method: "connections.remove",
            params: json!({ "sessionId": "contract", "from": "in", "to": "out", "idempotencyKey": "retry-disconnect" }),
            changed: json!({ "sessionId": "contract", "from": "in", "to": "in", "idempotencyKey": "retry-disconnect" }),
        },
        Retry {
            method: "nodes.remove",
            params: json!({ "sessionId": "contract", "node": "Speakers out", "idempotencyKey": "retry-remove" }),
            changed: json!({ "sessionId": "contract", "node": "Mic in", "idempotencyKey": "retry-remove" }),
        },
        Retry {
            method: "sessions.rename",
            params: json!({ "sessionId": "contract", "name": "Renamed once", "idempotencyKey": "retry-rename" }),
            changed: json!({ "sessionId": "contract", "name": "Renamed other", "idempotencyKey": "retry-rename" }),
        },
        Retry {
            method: "sessions.create",
            params: json!({ "session": created("created-once"), "idempotencyKey": "retry-create" }),
            changed: json!({ "session": created("created-other"), "idempotencyKey": "retry-create" }),
        },
        Retry {
            method: "sessions.duplicate",
            params: json!({ "sourceSessionId": "contract", "sessionId": "copy-once", "idempotencyKey": "retry-duplicate" }),
            changed: json!({ "sourceSessionId": "contract", "sessionId": "copy-other", "idempotencyKey": "retry-duplicate" }),
        },
        Retry {
            method: "sessions.delete",
            params: json!({ "sessionId": "contract", "idempotencyKey": "retry-delete" }),
            changed: json!({ "sessionId": "contract-other", "idempotencyKey": "retry-delete" }),
        },
        Retry {
            method: "sessions.active.set",
            params: json!({ "sessionId": "other-open", "idempotencyKey": "retry-open" }),
            changed: json!({ "sessionId": "contract", "idempotencyKey": "retry-open" }),
        },
        Retry {
            method: "connections.add",
            params: json!({ "sessionId": "contract", "from": "in", "to": "out", "idempotencyKey": "retry-connect" }),
            changed: json!({ "sessionId": "contract", "from": "in", "to": "out", "fromOutput": "out", "idempotencyKey": "retry-connect" }),
        },
        // Keys the backend derives from the caller's key (`toggle:{key}`) must
        // fit the storage limit at the longest key too.
        Retry {
            method: "safety.togglePrivacyMute",
            params: json!({ "idempotencyKey": "retry-mute-toggle" }),
            changed: Value::Null,
        },
        Retry {
            method: "safety.setPrivacyMute",
            params: json!({ "muted": true, "idempotencyKey": "retry-mute" }),
            changed: json!({ "muted": false, "idempotencyKey": "retry-mute" }),
        },
        Retry {
            method: "toolPresets.save",
            params: json!({ "kind": "gain", "name": "Contract preset", "parameters": { "gainDb": 3 }, "idempotencyKey": "retry-preset" }),
            changed: json!({ "kind": "gain", "name": "Contract preset", "parameters": { "gainDb": 6 }, "idempotencyKey": "retry-preset" }),
        },
    ]
}

/// The first call, the retry and the changed-content retry of one mutation,
/// with the saved state after each.
pub struct RetryRun {
    pub method: &'static str,
    pub before: Value,
    pub first: Outcome,
    pub after_first: Value,
    pub retry: Outcome,
    pub after_retry: Value,
    /// The same key with other content; `None` when the method has none.
    pub conflict: Option<Outcome>,
    pub after_conflict: Value,
}

/// The longest operation key the schemas allow (`maxLength` of every
/// `idempotencyKey`), for scenarios that must work at the limit.
pub const LONGEST_KEY: usize = 128;

fn pad_key(params: &mut Value, length: usize) {
    if let Some(key) = params["idempotencyKey"].as_str() {
        let padded = format!("{key:x<length$}");
        params["idempotencyKey"] = json!(padded);
    }
}

/// Run each retry scenario on a fresh backend from `fresh`. With `key_length`,
/// every operation key is padded to that many bytes first.
pub fn retry_runs(
    mut fresh: impl FnMut() -> Box<dyn Adapter>,
    key_length: Option<usize>,
) -> Vec<RetryRun> {
    retries()
        .into_iter()
        .map(|mut retry| {
            if let Some(length) = key_length {
                pad_key(&mut retry.params, length);
                pad_key(&mut retry.changed, length);
            }
            let mut adapter = fresh();
            for (method, params) in retry_setup(retry.method) {
                let outcome = adapter.call(method, Some(params));
                assert!(outcome.ok, "{} setup {method}: {outcome:?}", retry.method);
            }
            let before = saved_state(adapter.as_mut());
            let first = adapter.call(retry.method, Some(retry.params.clone()));
            let after_first = saved_state(adapter.as_mut());
            let again = adapter.call(retry.method, Some(retry.params.clone()));
            let after_retry = saved_state(adapter.as_mut());
            // `null` for a request with nothing to change but its key.
            let conflict = (!retry.changed.is_null())
                .then(|| adapter.call(retry.method, Some(retry.changed.clone())));
            let after_conflict = saved_state(adapter.as_mut());
            RetryRun {
                method: retry.method,
                before,
                first,
                after_first,
                retry: again,
                after_retry,
                conflict,
                after_conflict,
            }
        })
        .collect()
}

/// What the retry contract requires, as failures: the retry returns the very
/// same result and changes nothing; the same key with other content is refused
/// as `idempotencyConflict` and changes nothing.
pub fn retry_failures(runs: &[RetryRun]) -> Vec<String> {
    let mut failures = Vec::new();
    for run in runs {
        let method = run.method;
        if !run.first.ok {
            failures.push(format!(
                "{method}: the first call failed: {}",
                run.first.label()
            ));
            continue;
        }
        if run.after_first == run.before {
            failures.push(format!("{method}: the first call changed nothing visible"));
        }
        if run.retry != run.first {
            failures.push(format!(
                "{method}: the retry answered {:?}, not the first result {:?}",
                run.retry, run.first
            ));
        }
        if run.after_retry != run.after_first {
            failures.push(format!("{method}: the retry applied the change again"));
        }
        if let Some(conflict) = &run.conflict {
            if conflict.ok || conflict.category != "idempotencyConflict" {
                failures.push(format!(
                    "{method}: the same key with other content answered {}",
                    conflict.label()
                ));
            }
        }
        if run.after_conflict != run.after_retry {
            failures.push(format!("{method}: the conflicting retry changed state"));
        }
    }
    failures
}

/// Methods whose published input carries an `idempotencyKey`.
pub fn idempotent_methods(methods: &[Method]) -> Vec<String> {
    methods
        .iter()
        .filter(|method| method.properties().contains_key("idempotencyKey"))
        .map(|method| method.name.clone())
        .collect()
}
