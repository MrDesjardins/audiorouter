//! Q4.1 contract tests for the dispatcher, the reference every adapter (MCP,
//! HTTP, CLI, the window bridge) must agree with. The cases are generated from
//! the published schemas, see `contract_cases.rs`.

use crate::contract_cases::*;
use crate::*;
use audiorouter_domain::PermissionScope;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// A grant holding every scope any method asks for, but never the desktop
/// window's device consent.
pub(crate) fn every_scope() -> ClientGrant {
    ClientGrant::with_scopes(API_METHODS.iter().map(|spec| spec.permission))
}

pub(crate) fn no_scopes() -> ClientGrant {
    ClientGrant::with_scopes([])
}

/// `ControlPlane::dispatch` as an adapter: trusted (`None`) or under a grant.
struct Dispatch {
    plane: ControlPlane,
    grant: Option<ClientGrant>,
    next_id: u64,
}

impl Dispatch {
    fn new(grant: Option<ClientGrant>) -> Self {
        let mut plane = ControlPlane::default();
        plane.insert_session(fixture_session("contract")).unwrap();
        Self {
            plane,
            grant,
            next_id: 0,
        }
    }
}

impl Adapter for Dispatch {
    fn call(&mut self, method: &str, params: Option<Value>) -> Outcome {
        self.next_id += 1;
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(self.next_id)),
            method: method.into(),
            params,
        };
        let plane = &mut self.plane;
        let grant = &self.grant;
        match catch_unwind(AssertUnwindSafe(|| match grant {
            Some(grant) => plane.dispatch_authorized(request, grant),
            None => plane.dispatch(request),
        })) {
            Ok(response) => Outcome::from_response(&response),
            Err(_) => Outcome::panic(),
        }
    }

    fn refuses_before_validation(&self, method: &str) -> Option<&'static str> {
        // Only the desktop window may change device consent.
        (self.grant.is_some() && method == "devices.setAccess").then_some("permissionDenied")
    }
}

#[test]
fn generated_cases_use_the_dispatcher_limits() {
    assert_eq!(OVERSIZE_STRING_BYTES, MAX_CONTROL_STRING_BYTES + 1);
    assert!(DEEP_NESTING > MAX_CONTROL_VALUE_DEPTH);
    // The JSON parser's own recursion limit must not decide first.
    assert!(DEEP_NESTING < 128);
}

#[test]
fn the_published_schemas_describe_every_api_method() {
    let methods = describe_methods(&mut Dispatch::new(None));
    assert_eq!(methods.len(), API_METHODS.len());
    for spec in API_METHODS {
        let method = methods
            .iter()
            .find(|method| method.name == spec.name)
            .unwrap_or_else(|| panic!("{} is not described", spec.name));
        assert_eq!(method.input_schema["type"], "object", "{}", spec.name);
        for name in method.required() {
            assert!(
                method.properties().contains_key(&name),
                "{}: required `{name}` is not a property",
                spec.name
            );
        }
    }
}

#[test]
fn every_method_refuses_invalid_input_with_a_structured_category() {
    let methods = describe_methods(&mut Dispatch::new(None));
    for (label, grant) in [
        ("trusted dispatch", None),
        ("full grant", Some(every_scope())),
    ] {
        let failures = invalid_input_failures(&mut Dispatch::new(grant), &methods);
        assert!(
            failures.is_empty(),
            "{label}: {} contract failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

/// Regression: an optional scalar of the wrong type was read as absent, so a
/// numeric `sessionId` made `sessions.summary` answer about the active session
/// and a text `limit` was ignored. `null` still means "not given".
#[test]
fn a_wrong_typed_optional_parameter_is_refused_not_ignored() {
    let mut adapter = Dispatch::new(None);
    let opened = adapter.call(
        "sessions.active.set",
        Some(json!({ "sessionId": "contract", "idempotencyKey": "open" })),
    );
    assert!(opened.ok, "{opened:?}");
    for (method, params, message) in [
        (
            "sessions.summary",
            json!({ "sessionId": 7 }),
            "sessionId must be a string",
        ),
        (
            "sessions.list",
            json!({ "limit": "ten" }),
            "limit must be an integer",
        ),
        (
            "recordings.list",
            json!({ "sessionId": 7 }),
            "sessionId must be a string",
        ),
        (
            "toolPresets.list",
            json!({ "kind": 7 }),
            "kind must be a string",
        ),
    ] {
        let outcome = adapter.call(method, Some(params.clone()));
        assert!(!outcome.ok, "{method} {params} was accepted");
        assert_eq!(outcome.rpc_code, -32602, "{method}: {outcome:?}");
        assert_eq!(outcome.message, message, "{method}");
    }
    for (method, params) in [
        ("sessions.summary", json!({ "sessionId": null })),
        ("sessions.list", json!({ "limit": null })),
        ("sessions.summary", json!({ "sessionId": "contract" })),
    ] {
        let outcome = adapter.call(method, Some(params.clone()));
        assert!(outcome.ok, "{method} {params} was refused: {outcome:?}");
    }
}

/// A caller is allowed exactly the methods its scopes cover, and a refusal is
/// the same structured error for every method, before any parameter is read.
#[test]
fn every_method_is_refused_to_a_role_without_its_permission() {
    let methods = describe_methods(&mut Dispatch::new(None));
    let roles: [(&str, ClientGrant); 4] = [
        ("none", no_scopes()),
        ("observer", ClientGrant::for_role(ClientRole::Observer)),
        ("editor", ClientGrant::for_role(ClientRole::Editor)),
        ("operator", ClientGrant::for_role(ClientRole::Operator)),
    ];
    let mut failures = Vec::new();
    for (role, grant) in roles {
        let mut adapter = Dispatch::new(Some(grant.clone()));
        for (name, outcome) in authorization_results(&mut adapter, &methods) {
            let spec = API_METHODS.iter().find(|spec| spec.name == name).unwrap();
            let allowed = grant.allows(spec.permission) && name != "devices.setAccess";
            if allowed {
                if outcome.category == "permissionDenied" || outcome.rpc_code == -32001 {
                    failures.push(format!("{role}: {name} was refused: {}", outcome.label()));
                }
            } else if outcome.rpc_code != -32001
                || outcome.category != "permissionDenied"
                || !outcome.message.starts_with("permission denied")
            {
                failures.push(format!(
                    "{role}: {name} ({:?}) answered {} `{}`",
                    spec.permission,
                    outcome.label(),
                    outcome.message
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every revision-guarded method answers a stale revision with the same
/// retryable `revisionConflict`, and the scenarios cover every such method the
/// schemas publish.
#[test]
fn a_stale_revision_is_a_revision_conflict_on_every_guarded_method() {
    let mut adapter = Dispatch::new(Some(every_scope()));
    let methods = describe_methods(&mut adapter);
    let outcomes = stale_revision_outcomes(&mut adapter);
    let mut guarded = revision_guarded(&methods);
    guarded.sort();
    let mut covered: Vec<String> = outcomes.iter().map(|(name, _)| name.clone()).collect();
    covered.sort();
    assert_eq!(
        guarded, covered,
        "a method took a baseRevision without a stale-revision scenario"
    );
    for (method, outcome) in outcomes {
        assert!(!outcome.ok, "{method} applied a stale revision");
        assert_eq!(
            outcome.category, "revisionConflict",
            "{method}: {outcome:?}"
        );
        assert_eq!(outcome.rpc_code, -32000, "{method}");
        assert_plain_message(&method, "stale revision", &outcome);
    }
}

/// A retry with the same operation key returns the first result and applies
/// nothing twice; the same key with other content is an idempotency conflict.
#[test]
fn a_retried_mutation_is_applied_once_and_a_reused_key_is_refused() {
    let runs = retry_runs(|| Box::new(Dispatch::new(Some(every_scope()))));
    let failures = retry_failures(&runs);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The retry scenarios name every method the schemas publish with an
/// `idempotencyKey`, or give the reason one is exercised elsewhere.
#[test]
fn every_idempotent_method_has_a_retry_scenario_or_a_recorded_reason() {
    let methods = describe_methods(&mut Dispatch::new(None));
    let exercised: Vec<&str> = retries().iter().map(|retry| retry.method).collect();
    let mut missing = Vec::new();
    for method in idempotent_methods(&methods) {
        if exercised.contains(&method.as_str()) {
            continue;
        }
        if !RETRY_ELSEWHERE.iter().any(|(name, _)| *name == method) {
            missing.push(method);
        }
    }
    assert!(
        missing.is_empty(),
        "add a retry scenario (contract_cases::retries) or a reason in RETRY_ELSEWHERE for {missing:?}"
    );
    for (name, reason) in RETRY_ELSEWHERE {
        assert!(
            API_METHODS.iter().any(|spec| spec.name == *name),
            "{name} is not an API method"
        );
        assert!(
            !exercised.contains(name),
            "{name} has a scenario; drop its exclusion"
        );
        assert!(reason.len() > 10, "{name} needs a reason");
    }
    let keyed = idempotent_methods(&methods);
    let unkeyed: Vec<&str> = RETRY_ELSEWHERE
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !keyed.iter().any(|method| method == name))
        .collect();
    assert!(
        unkeyed.is_empty(),
        "these publish no idempotencyKey; drop their exclusion: {unkeyed:?}"
    );
}

/// Idempotent methods whose retry needs state or hardware a bare in-memory
/// backend does not have, so the retry contract is not exercised on them here
/// (the scenarios above run the backend's shared idempotency store).
const RETRY_ELSEWHERE: &[(&str, &str)] = &[
    ("graph.commit", "needs a plan; retried in the stale-revision scenario and authorization_tests::authorized_idempotency_keys_are_scoped_to_client_and_method"),
    ("sessions.start", "needs prepared audio devices"),
    ("session.start", "alias of sessions.start"),
    ("sessions.stop", "needs a running session"),
    ("session.stop", "alias of sessions.stop"),
    ("sessions.play", "opens audio devices"),
    ("sessions.togglePlay", "opens audio devices"),
    ("sessions.importCommit", "needs an import plan made from a session file"),
    ("sessions.createFromGoal", "needs the goal catalog and audio devices"),
    ("nodes.compare", "a press-and-hold audio comparison on a running route"),
    ("toolPresets.delete", "needs the id of a saved preset"),
    ("recorders.create", "needs a Recorder node and the approved recording folder"),
    ("recorders.arm", "needs a prepared recorder"),
    ("recorders.start", "needs a prepared recorder"),
    ("recorders.pause", "needs a prepared recorder"),
    ("recorders.resume", "needs a prepared recorder"),
    ("recorders.split", "needs a prepared recorder"),
    ("recorders.stop", "needs a prepared recorder"),
    ("recorders.startRecording", "needs a running Recorder and the approved recording folder"),
    ("recorders.stopRecording", "needs a running Recorder"),
    ("recordings.setRoot", "window-only; creates a folder"),
    ("recordings.setMetadata", "needs a finished recording file"),
    ("recordings.rename", "needs a finished recording file"),
    ("recordings.removeEntry", "needs a recording entry"),
    ("recordings.recycle", "moves a recording file to the recycle bin"),
    ("devices.setAccess", "window-only consent"),
    ("safety.setPrivacyMute", "a latch with no saved state a caller can read back here"),
    ("safety.togglePrivacyMute", "a latch with no saved state a caller can read back here"),
    ("recovery.clearSafeMode", "needs a database that started in safe mode"),
    ("system.quit", "stops the backend"),
    ("system.osTransition", "lock, sleep and sign-out lifecycle transitions"),
    ("clients.authorize", "changes who may call the backend; needs a client enrollment store"),
    ("clients.revoke", "changes who may call the backend; needs a client enrollment store"),
    ("operations.cancel", "needs a pending operation"),
    ("plugins.retry", "scans plugin folders on disk"),
    ("virtualDevices.apply", "needs a virtual device plan and the Windows driver"),
    ("virtualDevices.provision", "needs the Windows driver"),
    ("virtualDevices.remove", "needs the Windows driver"),
    ("virtualRoutes.replace", "covered by the stale-revision scenario; applies bus routes"),
    ("startup.apply", "writes the sign-in startup entry"),
    ("obs.connect", "opens a network connection to OBS"),
    ("obs.disconnect", "needs an OBS connection"),
    ("obs.apply", "needs an OBS connection and preview"),
    ("obs.restore", "needs an OBS connection"),
];

#[test]
fn the_roles_cover_the_scopes_the_methods_need() {
    // The role grants are the fixtures the adapter tests rely on: an Observer
    // must be refused something an Editor may do, and an Editor something an
    // Operator may do, or the authorization table above proves nothing.
    let denied = |role: ClientRole| {
        API_METHODS
            .iter()
            .filter(|spec| !ClientGrant::for_role(role).allows(spec.permission))
            .count()
    };
    assert!(denied(ClientRole::Observer) > denied(ClientRole::Editor));
    assert!(denied(ClientRole::Editor) > denied(ClientRole::Operator));
    assert!(denied(ClientRole::Operator) > 0);
    assert!(API_METHODS
        .iter()
        .any(|spec| spec.permission == PermissionScope::Read));
}
