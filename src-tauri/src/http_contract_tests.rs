//! Q4.1: the HTTP adapter held to the backend's contract.
//!
//! Every API method is called over a real socket through `POST /api/v1/...`,
//! with the cases generated from the published schemas (shared with the
//! control and MCP tests), and the answer must be the backend's own category
//! with the HTTP status that category maps to.

use super::contract_cases::*;
use super::*;
use audiorouter_control::{ClientGrant, ClientRole, ControlPlane};
use audiorouter_domain::API_METHODS;
use std::sync::mpsc::{sync_channel, SyncSender};

fn every_scope() -> ClientGrant {
    ClientGrant::with_scopes(API_METHODS.iter().map(|spec| spec.permission))
}

/// A backend on its own thread (it owns Windows COM objects there) reached
/// through the real HTTP listener, under `grant`. With `rotate`, requests use a
/// new client identity every 30 calls so a long sweep never meets the
/// backend's per-client mutation rate limit; retry scenarios keep one.
fn serve(grant: ClientGrant, rotate: bool) -> HttpApi {
    let (sender, receiver) = sync_channel::<(JsonRpcRequest, SyncSender<JsonRpcResponse>)>(8);
    std::thread::spawn(move || {
        let mut plane = ControlPlane::new("http-contract");
        plane.insert_session(fixture_session("contract")).unwrap();
        let mut count = 0u64;
        for (request, reply) in receiver {
            count += 1;
            let client = if rotate {
                format!("http-{}", count / 30)
            } else {
                "http".to_owned()
            };
            let _ = reply.send(plane.dispatch_authorized_for_client(request, &client, &grant));
        }
    });
    let forward: Arc<Forward> = Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
        let (reply, response) = sync_channel(1);
        sender
            .send((request.clone(), reply))
            .map_err(|_| "Backend closed".to_owned())?;
        response
            .recv_timeout(Duration::from_secs(30))
            .map_err(|_| "Backend timeout".to_owned())
    });
    HttpApi::start(0, forward).expect("HTTP API starts")
}

/// One HTTP exchange; the response status and its JSON body.
fn exchange(api: &HttpApi, path: &str, body: &str) -> (u16, Value) {
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        api.port,
        api.token,
        body.len()
    );
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, api.port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    let (headers, body) = text.split_once("\r\n\r\n").expect("an HTTP response");
    let status = headers.split(' ').nth(1).unwrap().parse().unwrap();
    (
        status,
        serde_json::from_str(body).unwrap_or_else(|_| panic!("a JSON body, got `{body}`")),
    )
}

/// `POST /api/v1/<method>` as an adapter. It records every status that does
/// not match the backend's category.
struct Http {
    api: HttpApi,
    status_mismatches: Vec<String>,
}

impl Http {
    fn new(grant: ClientGrant, rotate: bool) -> Self {
        Self {
            api: serve(grant, rotate),
            status_mismatches: Vec::new(),
        }
    }
}

impl Adapter for Http {
    fn call(&mut self, method: &str, params: Option<Value>) -> Outcome {
        let body = params.unwrap_or_else(|| json!({})).to_string();
        let (status, answer) = loop {
            let (status, answer) = exchange(&self.api, &operation_path(method), &body);
            // The adapter's own 20/s budget: wait for it to refill.
            if status == 429 {
                std::thread::sleep(Duration::from_millis(120));
                continue;
            }
            break (status, answer);
        };
        let mut outcome = if status == 200 {
            let mut outcome =
                Outcome::from_response(&JsonRpcResponse::success(None, answer.clone()));
            outcome.result = answer;
            outcome
        } else {
            let mut outcome = Outcome::from_error_value(&answer["error"]);
            // A refusal the adapter answered before the backend carries no
            // JSON-RPC code; a malformed request is -32602 there too.
            if answer["error"]["code"].is_null() && status == 400 {
                outcome.rpc_code = -32602;
            }
            outcome
        };
        outcome.status = Some(status);
        let expected = if outcome.ok {
            200
        } else if outcome.category.is_empty() {
            400
        } else {
            http_status_for_error_code(&outcome.category)
        };
        if status != expected {
            self.status_mismatches.push(format!(
                "{method}: HTTP {status} for {}, expected {expected}",
                outcome.label()
            ));
        }
        outcome
    }

    fn refuses_before_validation(&self, method: &str) -> Option<&'static str> {
        // Window-only methods are refused by the adapter, and device consent
        // is refused by the backend to any caller but the window.
        matches!(http_access(method), HttpAccess::DesktopOnly(_))
            .then_some("permissionDenied")
            .or_else(|| (method == "devices.setAccess").then_some("permissionDenied"))
    }

    fn normalize_params(&self, params: Option<Value>) -> Option<Value> {
        // The body is always a JSON object.
        Some(params.unwrap_or_else(|| json!({})))
    }

    fn answers_itself(&self, case_label: &str) -> bool {
        // A body that is not an object never reaches the backend.
        case_label == "params-not-an-object"
    }
}

/// `ControlPlane::dispatch_authorized`, the reference for parity.
fn direct(grant: ClientGrant) -> FnAdapter<'static> {
    let mut plane = ControlPlane::new("http-reference");
    plane.insert_session(fixture_session("contract")).unwrap();
    let mut next = 0;
    FnAdapter {
        call_fn: Box::new(move |method, params| {
            next += 1;
            let request = JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(next)),
                method: method.into(),
                params,
            };
            Outcome::from_response(&plane.dispatch_authorized(request, &grant))
        }),
        refuse_fn: Box::new(|method| (method == "devices.setAccess").then_some("permissionDenied")),
    }
}

#[test]
fn http_publishes_the_same_method_schemas_as_the_backend() {
    let backend = describe_methods(&mut direct(every_scope()));
    let mut http = Http::new(every_scope(), false);
    let published = describe_methods(&mut http);
    assert_eq!(published.len(), API_METHODS.len());
    for (a, b) in backend.iter().zip(&published) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.input_schema, b.input_schema, "{}", a.name);
    }
    // Every method has an HTTP operation in the OpenAPI document.
    let paths = http_openapi_paths(&http.api);
    for spec in API_METHODS {
        assert!(
            paths.contains(&operation_path(spec.name)),
            "{} has no OpenAPI operation",
            spec.name
        );
    }
}

fn http_openapi_paths(api: &HttpApi) -> Vec<String> {
    let request = format!(
        "GET /openapi.json HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
        api.port
    );
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, api.port)).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    let body = text.split_once("\r\n\r\n").unwrap().1;
    let document: Value = serde_json::from_str(body).unwrap();
    document["paths"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

#[test]
fn http_refuses_invalid_input_exactly_as_the_backend_does() {
    let methods = describe_methods(&mut direct(every_scope()));
    let mut http = Http::new(every_scope(), true);
    let failures = invalid_input_failures(&mut http, &methods);
    assert!(
        failures.is_empty(),
        "{} contract failures over HTTP:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        http.status_mismatches.is_empty(),
        "{}",
        http.status_mismatches.join("\n")
    );
    let mut http = Http::new(every_scope(), true);
    let failures = parity_failures(&mut http, &mut direct(every_scope()), &methods, "HTTP");
    assert!(
        failures.is_empty(),
        "{} differences between HTTP and dispatch:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        http.status_mismatches.is_empty(),
        "{}",
        http.status_mismatches.join("\n")
    );
}

#[test]
fn http_refuses_every_method_a_role_lacks_the_permission_for() {
    let methods = describe_methods(&mut direct(every_scope()));
    let roles = [
        ("observer", ClientRole::Observer),
        ("editor", ClientRole::Editor),
        ("operator", ClientRole::Operator),
    ];
    // One listener per role, each with its own request budget, run together.
    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = roles
            .iter()
            .map(|(role, client_role)| {
                let methods = &methods;
                scope.spawn(move || {
                    let grant = ClientGrant::for_role(*client_role);
                    let expected = authorization_results(&mut direct(grant.clone()), methods);
                    let mut http = Http::new(grant, true);
                    let actual = authorization_results(&mut http, methods);
                    let mut failures = http.status_mismatches.clone();
                    let denied = expected
                        .iter()
                        .filter(|(_, outcome)| outcome.category == "permissionDenied")
                        .count();
                    if *role != "operator" {
                        assert!(denied > 0, "{role}: nothing was refused");
                    }
                    assert!(denied < methods.len(), "{role}: nothing was allowed");
                    for ((name, expected), (_, actual)) in expected.iter().zip(&actual) {
                        if http.refuses_before_validation(name).is_some() {
                            if actual.category != "permissionDenied" || actual.status != Some(403) {
                                failures.push(format!(
                                    "{role}: {name} (window only) answered {} over HTTP",
                                    actual.label()
                                ));
                            }
                        } else if actual.label() != expected.label()
                            || actual.message != expected.message
                        {
                            failures.push(format!(
                                "{role}: {name}: HTTP answered {} `{}`, dispatch {} `{}`",
                                actual.label(),
                                actual.message,
                                expected.label(),
                                expected.message
                            ));
                        } else if expected.category == "permissionDenied"
                            && actual.status != Some(403)
                        {
                            failures.push(format!(
                                "{role}: {name}: permission refusal was HTTP {:?}",
                                actual.status
                            ));
                        }
                    }
                    failures
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect()
    });
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn http_reports_a_stale_revision_as_409() {
    let mut http = Http::new(every_scope(), false);
    let outcomes = stale_revision_outcomes(&mut http);
    assert!(!outcomes.is_empty());
    for (method, outcome) in outcomes {
        assert_eq!(
            outcome.category, "revisionConflict",
            "{method}: {outcome:?}"
        );
        assert_eq!(outcome.status, Some(409), "{method}");
    }
    assert!(
        http.status_mismatches.is_empty(),
        "{}",
        http.status_mismatches.join("\n")
    );
}

#[test]
fn http_retries_with_the_same_operation_key_apply_once() {
    // Window-only methods are not retried here; every scenario method is open.
    for key_length in [None, Some(LONGEST_KEY)] {
        let runs = retry_runs(|| Box::new(Http::new(every_scope(), false)), key_length);
        let failures = retry_failures(&runs);
        assert!(
            failures.is_empty(),
            "key length {key_length:?}:\n{}",
            failures.join("\n")
        );
        for run in &runs {
            assert_eq!(
                run.conflict.status,
                Some(409),
                "{}: a reused key must be HTTP 409",
                run.method
            );
        }
    }
}
