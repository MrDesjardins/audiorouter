//! Tests for `authorization.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn mutation_rate_limiter_enforces_burst_and_refill_rate() {
    let mut limiter = MutationRateLimiter::default();
    let start = Instant::now();
    for _ in 0..40 {
        assert!(limiter.allow_at("client", start).is_ok());
    }
    assert_eq!(limiter.allow_at("client", start), Err(50));
    assert!(limiter
        .allow_at("client", start + std::time::Duration::from_millis(50))
        .is_ok());
}

#[test]
fn mutation_rate_limiter_bounds_distinct_client_retention() {
    let mut limiter = MutationRateLimiter::default();
    let start = Instant::now();
    for index in 0..MAX_MUTATION_BUCKETS {
        assert!(limiter.allow_at(&format!("client-{index}"), start).is_ok());
    }
    assert_eq!(limiter.buckets.len(), MAX_MUTATION_BUCKETS);
    assert_eq!(limiter.allow_at("new-client", start), Err(1_000));

    let after_retention = start + MUTATION_BUCKET_RETENTION + Duration::from_millis(1);
    assert!(limiter.allow_at("new-client", after_retention).is_ok());
    assert!(limiter.buckets.len() <= MAX_MUTATION_BUCKETS);
}

#[test]
fn authenticated_dispatch_returns_rate_limit_metadata() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original).unwrap();
    let grant = ClientGrant::for_role(ClientRole::Operator);
    let request = || JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "session.start".into(),
        params: Some(json!({
            "sessionId": "session",
            "idempotencyKey": "rate-limit-start"
        })),
    };
    for _ in 0..40 {
        assert!(plane
            .dispatch_authorized_for_client(request(), "client", &grant)
            .result
            .is_some());
    }
    let response = plane.dispatch_authorized_for_client(request(), "client", &grant);
    assert_eq!(response.error.as_ref().unwrap().code, -32000);
    let data = response.error.as_ref().unwrap().data.as_ref().unwrap();
    assert_eq!(data["code"], "rateLimited");
    let retry_after_ms = data["retryAfterMs"].as_u64().unwrap();
    assert!((1..=50).contains(&retry_after_ms));
    assert_eq!(data["retryable"], true);
}

#[test]
fn dispatch_rejects_parameters_outside_discovered_schema() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "sessions.list".into(),
        params: Some(json!({ "unexpected": true })),
    });
    assert_eq!(response.error.unwrap().code, -32602);

    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(8)),
        method: "events.subscribe".into(),
        params: Some(json!({ "categories": ["not-a-real-event"] })),
    });
    assert_eq!(response.error.unwrap().code, -32602);

    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "devices.list".into(),
        params: Some(json!([])),
    });
    assert_eq!(response.error.unwrap().code, -32602);
}

#[test]
fn dispatch_rejects_overdeep_and_oversized_parameter_values() {
    let mut nested = json!("leaf");
    for _ in 0..=MAX_CONTROL_VALUE_DEPTH {
        nested = json!({ "nested": nested });
    }
    let mut plane = ControlPlane::default();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "sessions.list".into(),
        params: Some(nested),
    });
    assert_eq!(response.error.unwrap().code, -32602);

    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "sessions.list".into(),
        params: Some(json!({ "cursor": "x".repeat(MAX_CONTROL_STRING_BYTES + 1) })),
    });
    assert_eq!(response.error.unwrap().code, -32602);

    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "sessions.list".into(),
        params: Some(json!({ "values": vec![json!(null); MAX_CONTROL_VALUE_COUNT] })),
    });
    assert_eq!(response.error.unwrap().code, -32602);
}

#[test]
fn nullable_optional_parameters_are_treated_as_omitted() {
    let mut plane = ControlPlane::default();
    let mut source = session();
    source.id = EntityId::new("source");
    plane.insert_session(source).unwrap();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(7)),
        method: "sessions.duplicate".into(),
        params: Some(json!({
            "sourceSessionId": "source",
            "sessionId": "copy",
            "name": null,
            "idempotencyKey": "duplicate-null-name"
        })),
    });
    assert!(response.result.is_some());
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(8)),
        method: "sessions.list".into(),
        params: Some(json!({ "cursor": null })),
    });
    assert!(response.result.is_some());
}

#[test]
fn client_enrollment_api_lists_authorizes_and_revokes() {
    let mut plane = ControlPlane::default();
    let grant = ClientGrant::with_scopes([PermissionScope::DeviceAdministration]);
    let authorize = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(10)),
            method: "clients.authorize".into(),
            params: Some(json!({
                "clientId": "desktop",
                "role": "editor",
                "idempotencyKey": "authorize-desktop-1"
            })),
        },
        &grant,
    );
    assert_eq!(authorize.result.unwrap()["revoked"], false);
    let listed = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(11)),
        method: "clients.list".into(),
        params: None,
    });
    assert_eq!(listed.result.unwrap()[0]["clientId"], "desktop");
    let revoked = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(12)),
            method: "clients.revoke".into(),
            params: Some(json!({
                "clientId": "desktop",
                "idempotencyKey": "revoke-desktop-1"
            })),
        },
        &grant,
    );
    assert_eq!(revoked.result.unwrap()["changed"], true);
    let oversized = "x".repeat(audiorouter_domain::MAX_ENTITY_ID_BYTES + 1);
    assert!(plane
        .enroll_client(oversized.clone(), ClientRole::Observer)
        .is_err());
    assert!(plane.revoke_client(&oversized).is_err());
}

#[test]
fn authorized_idempotency_keys_are_scoped_to_client_and_method() {
    let mut plane =
        ControlPlane::with_storage("scoped-idempotency", Storage::open_memory().unwrap());
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let grant = ClientGrant::for_role(ClientRole::Editor);
    let same_key = "shared-client-key";

    let mut first_candidate = original.clone();
    first_candidate.name = "first-client-change".into();
    let first_plan = plane.plan_graph(&original.id, 0, first_candidate).unwrap();
    let first = plane.dispatch_authorized_for_client(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "graph.commit".into(),
            params: Some(json!({
                "planId": first_plan,
                "baseRevision": 0,
                "idempotencyKey": same_key
            })),
        },
        "client-a",
        &grant,
    );
    assert_eq!(first.result.unwrap()["revision"], 1);

    let committed = plane.get_session(&original.id).unwrap().clone();
    let mut second_candidate = committed.clone();
    second_candidate.name = "second-client-change".into();
    let second_plan = plane.plan_graph(&original.id, 1, second_candidate).unwrap();
    let second = plane.dispatch_authorized_for_client(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "graph.commit".into(),
            params: Some(json!({
                "planId": second_plan,
                "baseRevision": 1,
                "idempotencyKey": same_key
            })),
        },
        "client-b",
        &grant,
    );
    assert_eq!(second.result.unwrap()["revision"], 2);
    assert_eq!(
        plane.get_session(&original.id).unwrap().name,
        "second-client-change"
    );

    let mut operation_lookup = |id: i32, client_id: &str| {
        plane
            .dispatch_authorized_for_client(
                JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(id)),
                    method: "operations.get".into(),
                    params: Some(json!({ "operationId": same_key })),
                },
                client_id,
                &grant,
            )
            .result
            .unwrap()
    };
    assert_eq!(operation_lookup(3, "client-a")["revision"], 1);
    assert_eq!(operation_lookup(4, "client-b")["revision"], 2);
}

#[test]
fn dispatch_rejects_mutating_notifications_and_unknown_methods() {
    let mut plane = ControlPlane::default();
    let notification = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: None,
        method: "graph.commit".into(),
        params: None,
    };
    assert_eq!(plane.dispatch(notification).error.unwrap().code, -32600);
    let privacy_notification = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: None,
        method: "safety.setPrivacyMute".into(),
        params: Some(json!({ "muted": true })),
    };
    assert_eq!(
        plane.dispatch(privacy_notification).error.unwrap().code,
        -32600
    );
    for method in ["operations.cancel", "recordings.rename"] {
        let notification = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: method.into(),
            params: None,
        };
        assert_eq!(plane.dispatch(notification).error.unwrap().code, -32600);
    }
    let unknown = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "no.such.method".into(),
        params: None,
    };
    assert_eq!(plane.dispatch(unknown).error.unwrap().code, -32601);
}

#[test]
fn mutation_classifier_matches_authoritative_method_metadata() {
    for method in API_METHODS {
        assert_eq!(
            is_mutating_method(method.name),
            method.side_effect != audiorouter_domain::SideEffectClass::ReadOnly,
            "mutation classification drifted for {}",
            method.name
        );
    }
}

#[test]
fn native_pump_is_not_counted_as_a_user_mutation() {
    assert!(!rate_limit_method("nativeEndpoints.pump"));
    assert!(!rate_limit_method("nativeDuplex.pump"));
    assert!(rate_limit_method("graph.commit"));
    assert!(is_mutating_method("nativeEndpoints.pump"));
    assert!(is_mutating_method("nativeDuplex.pump"));
}

#[test]
fn scoped_authorization_denies_mutation_before_dispatch() {
    let mut plane = ControlPlane::default();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "graph.commit".into(),
        params: None,
    };
    let response = plane.dispatch_authorized(request, &ClientGrant::read_only());
    assert_eq!(response.error.unwrap().code, -32001);
}

#[test]
fn authorized_dispatch_validates_known_methods_before_authorization() {
    let mut plane = ControlPlane::default();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!({ "nested": "x".repeat(MAX_REQUEST_ID_BYTES) })),
        method: "graph.commit".into(),
        params: None,
    };
    let response = plane.dispatch_authorized(request, &ClientGrant::read_only());
    assert_eq!(response.error.unwrap().code, -32600);
}

#[test]
fn scoped_authorization_allows_discovery_read() {
    let mut plane = ControlPlane::default();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(5)),
        method: "system.describe".into(),
        params: None,
    };
    let response = plane.dispatch_authorized(request, &ClientGrant::read_only());
    assert!(response.result.unwrap()["methods"].is_array());
}

#[test]
fn authorized_framed_dispatch_denies_mutation_before_parameter_parsing() {
    let mut plane = ControlPlane::default();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(6)),
        method: "graph.commit".into(),
        params: None,
    };
    let frame = audiorouter_protocol::encode_frame(&request).unwrap();
    let responses = plane
        .dispatch_frame_authorized(&frame, &ClientGrant::read_only())
        .unwrap();
    let response: JsonRpcResponse = audiorouter_protocol::decode_frame(&responses[0]).unwrap();
    assert_eq!(response.error.unwrap().code, -32001);
}

#[test]
fn authorized_batch_preserves_allowed_and_denied_responses_in_order() {
    let mut plane = ControlPlane::default();
    let message = RpcMessage::Batch(vec![
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "system.describe".into(),
            params: None,
        },
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "graph.commit".into(),
            params: None,
        },
    ]);
    let responses = plane.dispatch_message_authorized(message, &ClientGrant::read_only());
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0].id, Some(json!(1)));
    assert!(responses[0].result.is_some());
    assert_eq!(responses[1].id, Some(json!(2)));
    assert_eq!(responses[1].error.as_ref().unwrap().code, -32001);
}

#[test]
fn built_in_roles_are_deny_by_default_for_sensitive_scopes() {
    assert!(ClientGrant::for_role(ClientRole::Observer).allows(PermissionScope::Read));
    assert!(!ClientGrant::for_role(ClientRole::Observer).allows(PermissionScope::GraphWrite));
    assert!(ClientGrant::for_role(ClientRole::Editor).allows(PermissionScope::GraphWrite));
    assert!(!ClientGrant::for_role(ClientRole::Editor).allows(PermissionScope::SessionControl));
    assert!(ClientGrant::for_role(ClientRole::Operator).allows(PermissionScope::SessionControl));
    assert!(!ClientGrant::for_role(ClientRole::Operator).allows(PermissionScope::Capture));
    assert!(!ClientGrant::for_role(ClientRole::Operator).allows(PermissionScope::StartupWrite));
    assert!(ClientGrant::for_desktop_shell().allows(PermissionScope::StartupWrite));
    assert!(ClientGrant::for_desktop_shell().allows(PermissionScope::Record));
    assert!(ClientGrant::for_desktop_shell().allows(PermissionScope::PluginScan));
    assert!(!ClientGrant::for_desktop_shell().allows(PermissionScope::Capture));
    assert!(!ClientGrant::for_desktop_shell().allows(PermissionScope::DeviceAdministration));
    assert!(
        !ClientGrant::for_role(ClientRole::Operator).allows(PermissionScope::DeviceAdministration)
    );
    assert!(!ClientGrant::read_only().allows(PermissionScope::PluginScan));
    assert!(
        ClientGrant::with_scopes([PermissionScope::PluginScan]).allows(PermissionScope::PluginScan)
    );
}

#[test]
fn enrollment_lookup_denies_unknown_and_revoked_clients() {
    let mut plane = ControlPlane::new("enrollment-test");
    assert!(plane.grant_for_client("unknown").unwrap().is_none());
    plane.enroll_client("client", ClientRole::Editor).unwrap();
    let grant = plane.grant_for_client("client").unwrap().unwrap();
    assert!(grant.allows(PermissionScope::GraphWrite));
    assert!(!grant.allows(PermissionScope::SessionControl));
    assert!(plane.revoke_client("client").unwrap());
    assert!(plane.grant_for_client("client").unwrap().is_none());
    assert!(!plane.revoke_client("client").unwrap());
}

#[test]
fn in_memory_client_enrollments_are_bounded() {
    let mut plane = ControlPlane::new("enrollment-limit");
    for index in 0..audiorouter_storage::MAX_CLIENT_ENROLLMENTS {
        plane
            .enroll_client(format!("client-{index}"), ClientRole::Observer)
            .unwrap();
    }
    assert!(matches!(
        plane.enroll_client("client-overflow", ClientRole::Observer),
        Err(ControlError::InvalidRequest(message))
            if message == "client enrollment limit reached"
    ));
    assert_eq!(
        plane.client_records().unwrap().len(),
        audiorouter_storage::MAX_CLIENT_ENROLLMENTS
    );
}

#[test]
fn storage_backed_enrollment_persists_and_revokes() {
    let storage = Storage::open_memory().unwrap();
    let mut first = ControlPlane::with_storage("enrollment-persist", storage);
    first
        .enroll_client("operator", ClientRole::Operator)
        .unwrap();
    assert!(first.grant_for_client("operator").unwrap().is_some());
    assert!(first.revoke_client("operator").unwrap());
    assert!(first.grant_for_client("operator").unwrap().is_none());
}

#[test]
fn file_backed_enrollment_authorizes_after_control_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-enrollment-restart-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    {
        let mut first =
            ControlPlane::with_storage("enrollment-first", Storage::open(&path).unwrap());
        first
            .enroll_client("operator", ClientRole::Operator)
            .unwrap();
    }
    let mut second = ControlPlane::with_storage("enrollment-second", Storage::open(&path).unwrap());
    let grant = second.grant_for_client("operator").unwrap().unwrap();
    assert!(grant.allows(PermissionScope::SessionControl));
    let response = second.dispatch_authorized_for_client(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(22)),
            method: "recovery.clearSafeMode".into(),
            params: Some(json!({ "idempotencyKey": "recovery-clear-3" })),
        },
        "operator",
        &grant,
    );
    assert_eq!(response.result.unwrap()["safeMode"], false);
    let _ = std::fs::remove_file(path);
}

#[test]
fn read_only_notifications_produce_no_response() {
    let mut plane = ControlPlane::default();
    let notification = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: None,
        method: "system.describe".into(),
        params: None,
    };
    assert!(plane
        .dispatch_message(RpcMessage::Single(notification))
        .is_empty());
}
