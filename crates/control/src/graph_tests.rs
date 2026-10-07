//! Tests for `graph.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn endpoint_feedback_names_the_return_and_accepts_a_separate_recording_cable() {
    let mut session = feedback_fixture();
    let returns = vec![("cable-b-input".into(), "cable-b-output".into())];
    for bypass in [false, true] {
        session.nodes[1].bypass = bypass;
        let error = reject_endpoint_feedback(&session, &returns).unwrap_err();
        let message = control_error_message(&error);
        assert!(
            message.contains("game")
                && message.contains("recording")
                && message.contains("different output cable")
        );
    }
    session.nodes[3]
        .parameters
        .insert("endpointId".into(), json!("cable-c-input"));
    reject_endpoint_feedback(&session, &returns).unwrap();
    session.nodes[3]
        .parameters
        .insert("endpointId".into(), json!("cable-b-input"));
    session.edges[1].enabled = false;
    reject_endpoint_feedback(&session, &returns).unwrap();
}

#[test]
fn surround_loopback_input_cannot_play_back_into_its_own_device() {
    let mut session = feedback_fixture();
    // A 7.1 playback device captured by loopback and rendered for headphones.
    session.nodes[0]
        .parameters
        .insert("endpointId".into(), json!("cable-b-input"));
    session.nodes[0]
        .parameters
        .insert("spatialMode".into(), json!("headphones"));
    assert!(reject_endpoint_feedback(&session, &[]).is_err());
    session.nodes[3]
        .parameters
        .insert("endpointId".into(), json!("headphones"));
    reject_endpoint_feedback(&session, &[]).unwrap();
    // An ordinary input naming the same ID is a recording device, not a loop.
    session.nodes[0]
        .parameters
        .insert("spatialMode".into(), json!("off"));
    session.nodes[3]
        .parameters
        .insert("endpointId".into(), json!("cable-b-input"));
    reject_endpoint_feedback(&session, &[]).unwrap();
}

#[test]
fn automatic_route_restart_requires_device_and_lifecycle_authority() {
    assert!(!caller_can_restart_devices(
        &ClientGrant::with_scopes([PermissionScope::GraphWrite, PermissionScope::SessionControl,]),
        true
    ));
    assert!(!caller_can_restart_devices(
        &ClientGrant::with_scopes([PermissionScope::DeviceAdministration,]),
        false
    ));
    assert!(caller_can_restart_devices(
        &ClientGrant::with_scopes([
            PermissionScope::DeviceAdministration,
            PermissionScope::SessionControl,
        ]),
        false
    ));
    let desktop = ClientGrant::for_desktop_shell();
    assert!(!caller_can_restart_devices(&desktop, false));
    assert!(caller_can_restart_devices(&desktop, true));
    let mut plane = ControlPlane {
        active_device_restart_allowed: Some(false),
        ..Default::default()
    };
    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "system.diagnostics".into(),
            params: None,
        },
        &ClientGrant::with_scopes([PermissionScope::Read]),
    );
    assert!(response.error.is_none(), "{:?}", response.error);
    assert_eq!(plane.active_device_restart_allowed, Some(false));
}

#[test]
fn feedback_selection_can_be_saved_only_after_review_but_not_prepared() {
    let mut candidate = feedback_fixture();
    candidate.nodes[0].kind = NodeKind::EndpointLoopback;
    candidate.nodes[0]
        .parameters
        .insert("endpointId".into(), json!("same-render-endpoint"));
    candidate.nodes[3]
        .parameters
        .insert("endpointId".into(), json!("same-render-endpoint"));
    let mut original = candidate.clone();
    original.edges[2].enabled = false;
    let mut plane = ControlPlane::new("feedback-review");
    plane.insert_session(original).unwrap();
    let request = |method: &str, params| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    };
    let result = plane
        .dispatch(request(
            "graph.plan",
            json!({"sessionId":"feedback","baseRevision":0,"candidate":candidate}),
        ))
        .result
        .unwrap();
    let warning = result["warnings"][0].as_str().unwrap();
    assert!(
        warning.contains("game")
            && warning.contains("recording")
            && warning.contains("playback is blocked")
    );
    let mut commit =
        json!({"planId":result["planId"],"baseRevision":0,"idempotencyKey":"review-feedback"});
    assert!(plane
        .dispatch(request("graph.commit", commit.clone()))
        .error
        .unwrap()
        .message
        .contains("acknowledge"));
    commit["acknowledgments"] = result["warnings"].clone();
    let saved = plane.dispatch(request("graph.commit", commit));
    assert!(saved.error.is_none(), "{saved:?}");
    assert_eq!(
        plane
            .get_session(&EntityId::new("feedback"))
            .unwrap()
            .revision,
        1
    );
    assert!(plane
        .validate_endpoint_feedback(plane.get_session(&EntityId::new("feedback")).unwrap())
        .is_err());
}

#[test]
#[cfg(windows)]
#[ignore = "requires installed VB-Cable B; reads endpoint metadata only"]
fn installed_cable_feedback_is_rejected_before_opening_audio() {
    let endpoints = audiorouter_windows_audio::enumerate_active_endpoint_display_info().unwrap();
    let cable = |direction| {
        endpoints
            .iter()
            .find(|endpoint| {
                endpoint.direction == direction
                    && audiorouter_windows_audio::known_virtual_cable_key(
                        &endpoint.device_description,
                        &endpoint.driver_inf_section,
                    )
                    .as_deref()
                        == Some("b")
            })
            .expect("VB-Cable B endpoint")
    };
    let mut candidate = feedback_fixture();
    candidate.nodes[0].parameters.insert(
        "endpointId".into(),
        json!(cable(audiorouter_windows_audio::EndpointDirection::Capture).id),
    );
    candidate.nodes[3].parameters.insert(
        "endpointId".into(),
        json!(cable(audiorouter_windows_audio::EndpointDirection::Render).id),
    );
    let plane = ControlPlane::default();
    assert!(
        control_error_message(&plane.validate_endpoint_feedback(&candidate).unwrap_err())
            .contains("Audio feedback loop")
    );
}

#[test]
fn live_source_bypass_retains_shape_and_silences_every_output() {
    let mut session = feedback_fixture();
    session.nodes[3]
        .parameters
        .insert("endpointId".into(), json!("other-output"));
    let mut direct = session.nodes[3].clone();
    direct.id = EntityId::new("direct");
    session.nodes.push(direct);
    let mut edge = session.edges[1].clone();
    edge.id = EntityId::new("direct-branch");
    edge.destination_node = EntityId::new("direct");
    session.edges.push(edge);
    let inputs = vec![session.nodes[0].id.clone()];
    let outputs = vec![session.nodes[3].id.clone(), EntityId::new("direct")];
    for bypass in [true, false, true, false] {
        let mut candidate = session.clone();
        candidate.nodes[0].bypass = bypass;
        normalize_live_path_flags(&mut candidate, &inputs, &outputs);
        let set = audiorouter_engine::compile_native_paths_with_plugins_and_audio(
            &candidate,
            RuntimeGeneration::new(1),
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
        assert_eq!(set.input_node_ids(), inputs);
        assert_eq!(set.output_node_ids(), outputs);
        assert!(candidate.nodes[0].enabled && !candidate.nodes[0].bypass);
        assert_eq!(
            candidate.edges[0].matrix,
            if bypass {
                vec![0.0; 4]
            } else {
                vec![1.0, 0.0, 0.0, 1.0]
            }
        );
        let frames = audiorouter_engine::PROCESSING_QUANTUM_FRAMES;
        let mut runtime =
            audiorouter_engine::RealtimeMixerFanout::from_paths(set, 4, &[2], frames).unwrap();
        let mut block = audiorouter_engine::AudioBlock::new(2, frames).unwrap();
        for channel in 0..2 {
            block.channel_mut(channel).unwrap().fill(0.5);
        }
        let rings = (0..2)
            .map(|_| audiorouter_engine::AudioBlockRing::new(4, 2, frames).unwrap())
            .collect::<Vec<_>>();
        runtime
            .try_submit_input(0, RuntimeGeneration::new(1), &block)
            .unwrap();
        assert_eq!(
            runtime
                .process_once(&rings.iter().collect::<Vec<_>>())
                .unwrap(),
            2
        );
        for ring in &rings {
            let output = ring.try_receive().unwrap();
            assert!(output
                .channel(0)
                .unwrap()
                .iter()
                .all(|sample| *sample == if bypass { 0.0 } else { 0.5 }));
        }
    }
}

#[test]
fn routes_inspect_dispatch_returns_desired_provenance() {
    let mut plane = ControlPlane::default();
    let graph = session();
    plane.insert_session(graph).unwrap();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "routes.inspect".into(),
        params: Some(json!({
            "sessionId": "session",
            "destinationNode": "out"
        })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["reachable"], true);
    assert_eq!(result["paths"][0]["nodes"], json!(["in", "out"]));
    assert_eq!(result["paths"][0]["edges"], json!(["edge"]));
    assert_eq!(result["paths"][0]["channelMaps"], json!([[1.0]]));
    assert_eq!(result["paths"][0]["latencySamples"], 0);
}

#[test]
fn graph_history_dispatch_returns_newest_snapshot_first() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "revision-one".into();
    let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
    plane.commit_graph(&plan, 0, "history-api").unwrap();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "graph.history".into(),
        params: Some(json!({ "sessionId": "session", "limit": 1 })),
    });
    let history = response.result.unwrap();
    assert_eq!(history["items"].as_array().unwrap().len(), 1);
    assert_eq!(history["items"][0]["revision"], 1);
    assert_eq!(history["items"][0]["name"], "revision-one");
    assert_eq!(history["nextCursor"], "1");
    let oversized = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "graph.history".into(),
        params: Some(json!({
            "sessionId": "session",
            "cursor": "9".repeat(MAX_REVISION_CURSOR_BYTES + 1)
        })),
    });
    assert_eq!(oversized.error.unwrap().code, -32602);
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "graph.history".into(),
        params: Some(json!({ "sessionId": "session", "cursor": "1", "limit": 1 })),
    });
    let history = response.result.unwrap();
    assert_eq!(history["items"][0]["revision"], 0);
    assert!(history["nextCursor"].is_null());
}

#[test]
fn graph_undo_plan_dispatches_through_revision_checked_planning() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "revision-one".into();
    let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
    plane.commit_graph(&plan, 0, "undo-api").unwrap();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "graph.undoPlan".into(),
        params: Some(json!({ "sessionId": "session", "baseRevision": 1 })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["baseRevision"], 1);
    assert!(result["planId"].as_str().unwrap().starts_with("plan-"));
}

#[test]
fn graph_undo_plan_hydrates_prior_history_after_control_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-undo-restart-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    {
        let mut plane = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "persisted-edit".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "restart-undo").unwrap();
    }
    let mut restarted = ControlPlane::with_storage("restarted", Storage::open(&path).unwrap());
    let response = restarted.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(5)),
        method: "graph.undoPlan".into(),
        params: Some(json!({ "sessionId": "session", "baseRevision": 1 })),
    });
    assert!(response.result.unwrap()["planId"].as_str().is_some());
    let _ = std::fs::remove_file(path);
}

#[test]
fn dispatch_plan_and_commit_use_json_contracts() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "via-api".into();
    let plan_request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "graph.plan".into(),
        params: Some(json!({ "sessionId": "session", "baseRevision": 0, "candidate": candidate })),
    };
    let plan_result = plane.dispatch(plan_request).result.unwrap();
    assert_eq!(plan_result["diff"][0]["path"], "/name");
    assert_eq!(plan_result["requiredScopes"], json!(["graph.write"]));
    let plan_id = plan_result["planId"].clone();
    let commit_request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "graph.commit".into(),
        params: Some(json!({ "planId": plan_id, "baseRevision": 0, "idempotencyKey": "api-op" })),
    };
    assert_eq!(
        plane.dispatch(commit_request).result.unwrap()["revision"],
        1
    );
}

#[test]
fn graph_commit_acknowledgments_are_validated_and_cannot_grant_scope() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(17)),
        method: "graph.commit".into(),
        params: Some(json!({
            "planId": "plan-1",
            "baseRevision": 0,
            "idempotencyKey": "ack-test",
            "acknowledgments": ["unverified-feedback"]
        })),
    });
    assert!(response.error.unwrap().message.contains("no warnings"));

    let invalid = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(18)),
        method: "graph.commit".into(),
        params: Some(json!({
            "planId": "plan-1",
            "baseRevision": 0,
            "idempotencyKey": "ack-test",
            "acknowledgments": [12]
        })),
    });
    assert!(invalid.error.unwrap().message.contains("warning IDs"));
}
