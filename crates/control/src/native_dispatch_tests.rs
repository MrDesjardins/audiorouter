//! Tests for `native_dispatch.rs`.

use super::*;

/// A fresh install: Play needs device administration, which the desktop
/// grant lacks until the user consents once in the app. Consent persists
/// across restarts, only the desktop window's grant may give it, and it
/// can be withdrawn. The release 0.0.1 shipped without this path.
#[test]
fn desktop_consent_lets_a_fresh_install_open_devices_and_persists() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-device-consent-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let shell = ClientGrant::for_desktop_shell();
    let cli = ClientGrant::for_role(ClientRole::Operator);
    let call = |plane: &mut ControlPlane, grant: &ClientGrant, method: &str, params: Value| {
        plane.dispatch_authorized_for_client(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(params),
            },
            "consent-test",
            grant,
        )
    };
    let denied = |response: &JsonRpcResponse| {
        response
            .error
            .as_ref()
            .is_some_and(|error| error.message.contains("permission denied"))
    };
    {
        let mut plane = ControlPlane::with_storage("consent-first", Storage::open(&path).unwrap());
        // Before consent: Play's device preparation is refused.
        assert_eq!(
            call(&mut plane, &shell, "devices.getAccess", json!({}))
                .result
                .unwrap()["allowed"],
            false
        );
        let prepare = call(
            &mut plane,
            &shell,
            "nativePaths.prepare",
            json!({ "sessionId": "missing" }),
        );
        assert!(denied(&prepare), "{prepare:?}");
        // A CLI/MCP operator cannot give consent, even for itself.
        let refused = call(
            &mut plane,
            &cli,
            "devices.setAccess",
            json!({ "allowed": true, "idempotencyKey": "cli" }),
        );
        assert!(denied(&refused), "{refused:?}");
        // The desktop window gives it.
        let allowed = call(
            &mut plane,
            &shell,
            "devices.setAccess",
            json!({ "allowed": true, "idempotencyKey": "allow" }),
        );
        assert_eq!(allowed.result.unwrap()["allowed"], true);
        let prepare = call(
            &mut plane,
            &shell,
            "nativePaths.prepare",
            json!({ "sessionId": "missing" }),
        );
        assert!(
            !denied(&prepare),
            "now authorized; fails only on the missing session: {prepare:?}"
        );
        // Consent never widens other grants.
        let cli_prepare = call(
            &mut plane,
            &cli,
            "nativePaths.prepare",
            json!({ "sessionId": "missing" }),
        );
        assert!(denied(&cli_prepare), "{cli_prepare:?}");
    }
    {
        // After a restart (and a fresh grant object) the consent holds.
        let mut plane = ControlPlane::with_storage("consent-second", Storage::open(&path).unwrap());
        assert_eq!(
            call(&mut plane, &shell, "devices.getAccess", json!({}))
                .result
                .unwrap()["allowed"],
            true
        );
        let prepare = call(
            &mut plane,
            &shell,
            "nativePaths.prepare",
            json!({ "sessionId": "missing" }),
        );
        assert!(!denied(&prepare), "{prepare:?}");
        // Withdrawing it refuses device preparation again.
        call(
            &mut plane,
            &shell,
            "devices.setAccess",
            json!({ "allowed": false, "idempotencyKey": "withdraw" }),
        );
        let prepare = call(
            &mut plane,
            &shell,
            "nativePaths.prepare",
            json!({ "sessionId": "missing" }),
        );
        assert!(denied(&prepare), "{prepare:?}");
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn recovery_endpoint_resnapshot_publishes_a_bounded_device_change_event() {
    let mut plane = ControlPlane::default();
    let endpoint = audiorouter_windows_audio::EndpointInfo {
        id: "recovery-endpoint".into(),
        direction: audiorouter_windows_audio::EndpointDirection::Render,
        default_period_100ns: 100_000,
        minimum_period_100ns: 30_000,
        sample_rate_hz: 48_000,
        channels: 2,
        bits_per_sample: 32,
        format_tag: 3,
        channel_mask: 3,
        subformat_guid: "00000003-0000-0010-8000-00aa00389b71".into(),
    };
    let before = plane.events.latest_sequence();

    plane.retain_endpoint_changes(&[audiorouter_windows_audio::EndpointChange::Added(endpoint)]);

    assert_eq!(plane.events.latest_sequence(), before + 1);
    assert_eq!(
        plane.events.since(before, 1).unwrap()[0].category,
        "devices.changed"
    );
}

#[test]
fn devices_list_rejects_invalid_paging_parameters_before_enumeration() {
    let mut plane = ControlPlane::default();
    for (id, params) in [
        (1, json!({ "limit": 0 })),
        (2, json!({ "limit": 501 })),
        (3, json!({ "cursor": 42 })),
    ] {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(id)),
            method: "devices.list".into(),
            params: Some(params),
        });
        assert_eq!(response.error.unwrap().code, -32602);
    }
}

#[test]
fn endpoint_change_signal_replays_through_event_cursor() {
    let mut plane = ControlPlane::default();
    plane.record_endpoint_changes(false);
    plane.record_endpoint_changes(true);
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(44)),
        method: "events.subscribe".into(),
        params: Some(json!({
            "afterSequence": 0,
            "categories": ["devices.changed"]
        })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["events"].as_array().unwrap().len(), 1);
    assert_eq!(result["events"][0]["category"], "devices.changed");
    assert_eq!(result["events"][0]["resourceRevision"], 0);
}

#[test]
fn native_endpoint_preparation_requires_device_administration_before_parameters() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(91)),
            method: "nativeEndpoints.prepare".into(),
            params: None,
        },
        &ClientGrant::for_role(ClientRole::Operator),
    );
    assert_eq!(response.error.unwrap().code, -32001);
    assert!(plane.native_endpoint_worker.is_none());
    assert!(plane.endpoint_monitor.is_none());
}

#[test]
fn native_bridge_preparation_requires_device_administration_before_parameters() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(96)),
            method: "nativeBridges.prepare".into(),
            params: Some(json!({
                "busId": "bus-guard",
                "generation": 1,
                "devicePath": "\\\\.\\AudioRouterVirtualBridge",
                "renderMappingPath": "C:\\render.slot",
                "captureMappingPath": "C:\\capture.slot",
            })),
        },
        &ClientGrant::for_role(ClientRole::Operator),
    );
    assert_eq!(response.error.unwrap().code, -32001);
    assert!(plane.native_duplex_bindings.is_empty());
    assert!(plane.native_duplex_worker.is_none());
}

#[test]
fn native_endpoint_detachment_requires_device_administration_and_exact_binding() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(92)),
            method: "nativeEndpoints.detach".into(),
            params: Some(json!({ "sessionId": "missing" })),
        },
        &ClientGrant::for_role(ClientRole::Operator),
    );
    assert_eq!(response.error.unwrap().code, -32001);

    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(93)),
            method: "nativeEndpoints.detach".into(),
            params: Some(json!({ "sessionId": "missing" })),
        },
        &ClientGrant::with_scopes([PermissionScope::DeviceAdministration]),
    );
    assert_eq!(response.error.unwrap().code, -32602);
    assert!(plane.native_endpoint_worker.is_none());
}

#[test]
fn native_duplex_detachment_requires_device_administration_and_windows() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(94)),
            method: "nativeDuplex.detach".into(),
            params: Some(json!({ "sessionId": "missing" })),
        },
        &ClientGrant::for_role(ClientRole::Operator),
    );
    assert_eq!(response.error.unwrap().code, -32001);

    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(95)),
            method: "nativeDuplex.detach".into(),
            params: Some(json!({ "sessionId": "missing" })),
        },
        &ClientGrant::with_scopes([PermissionScope::DeviceAdministration]),
    );
    assert_eq!(response.error.unwrap().code, -32602);
}

#[test]
fn native_endpoint_pump_requires_session_control_before_parameters() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(92)),
            method: "nativeEndpoints.pump".into(),
            params: None,
        },
        &ClientGrant::read_only(),
    );
    assert_eq!(response.error.unwrap().code, -32001);
    assert_eq!(plane.native_endpoint_rejections, 0);
}

#[test]
fn native_endpoint_pump_input_schema_bounds_packet_budget() {
    let description = ControlPlane::default().describe();
    let method = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "nativeEndpoints.pump")
        .unwrap();
    assert_eq!(
        method["inputSchema"]["properties"]["maxPackets"]["maximum"],
        json!(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE)
    );
}

#[test]
fn native_duplex_pump_input_schema_bounds_both_budgets() {
    let description = ControlPlane::default().describe();
    let method = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "nativeDuplex.pump")
        .unwrap();
    for property in ["maxInputQuanta", "maxOutputPackets"] {
        assert_eq!(
            method["inputSchema"]["properties"][property]["maximum"],
            json!(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE)
        );
    }
}

#[test]
fn native_render_source_pump_requires_session_control_before_parameters() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(93)),
            method: "nativeRenderSources.pump".into(),
            params: None,
        },
        &ClientGrant::read_only(),
    );
    assert_eq!(response.error.unwrap().code, -32001);
    assert_eq!(plane.native_endpoint_rejections, 0);
}

#[test]
fn native_render_source_pump_input_schema_bounds_quanta_budget() {
    let description = ControlPlane::default().describe();
    let method = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "nativeRenderSources.pump")
        .unwrap();
    assert_eq!(
        method["inputSchema"]["properties"]["maxQuanta"]["maximum"],
        json!(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE)
    );
    assert_eq!(
        method["inputSchema"]["required"],
        json!(["sessionId", "generation"])
    );
}
