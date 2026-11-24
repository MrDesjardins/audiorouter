//! Tests for `graph.rs`.

use super::*;
use crate::test_support::*;

#[cfg(windows)]
#[test]
#[ignore = "requires rebuilt VST2 worker and AUDIOROUTER_TEST_VST2_BUS_FIXTURE"]
fn discovered_sidechain_port_plans_commit_replay_and_undo_atomically() {
    use audiorouter_plugin_host::{inspect_binary, SupervisedWorkerProcess};
    let plugin =
        std::path::PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST2_BUS_FIXTURE").unwrap());
    let worker = std::path::PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST2_WORKER").unwrap());
    let root = plugin.parent().unwrap().to_path_buf();
    let identity = inspect_binary(&plugin, std::slice::from_ref(&root)).unwrap();
    let mut worker = SupervisedWorkerProcess::spawn_verified_with_sample_rate(
        worker,
        &identity,
        std::slice::from_ref(&root),
        2,
        48_000,
        Instant::now(),
    )
    .unwrap();
    let binding = worker
        .verify_sidechain_binding(
            std::slice::from_ref(&root),
            &identity.sha256,
            None,
            (2, 2),
            Instant::now(),
        )
        .unwrap();
    assert!(worker.shutdown().unwrap().success());
    let mut plane = ControlPlane::default();
    let scan = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "plugins.scan".into(),
        params: Some(json!({ "directory": root })),
    });
    assert!(scan.error.is_none(), "{:?}", scan.error);
    let mut original = session();
    original.nodes.truncate(1);
    original.edges.clear();
    let node = &mut original.nodes[0];
    node.kind = NodeKind::Plugin;
    node.parameters = serde_json::from_value(
        json!({ "path": plugin, "format": "vst2", "fingerprint": identity.sha256 }),
    )
    .unwrap();
    node.ports = vec![
        audiorouter_domain::Port {
            name: "in".into(),
            direction: PortDirection::Input,
            channels: 2,
        },
        audiorouter_domain::Port {
            name: "out".into(),
            direction: PortDirection::Output,
            channels: 2,
        },
    ];
    let node_id = node.id.clone();
    plane.insert_session(original.clone()).unwrap();
    let mut wrong_identity = original.clone();
    wrong_identity.id = EntityId::new("wrong-plugin-binding");
    wrong_identity.nodes[0]
        .parameters
        .insert("fingerprint".into(), json!("f".repeat(64)));
    plane.insert_session(wrong_identity.clone()).unwrap();
    assert!(plane
        .plan_bound_plugin_sidechain(&wrong_identity.id, 0, &node_id, &binding)
        .is_err());
    assert_eq!(
        plane.get_session(&wrong_identity.id).unwrap(),
        &wrong_identity
    );
    let mut wrong_width = original.clone();
    wrong_width.id = EntityId::new("wrong-plugin-width");
    for port in &mut wrong_width.nodes[0].ports {
        port.channels = 1;
    }
    plane.insert_session(wrong_width.clone()).unwrap();
    assert!(plane
        .plan_bound_plugin_sidechain(&wrong_width.id, 0, &node_id, &binding)
        .is_err());
    assert_eq!(plane.get_session(&wrong_width.id).unwrap(), &wrong_width);
    let plan_id = plane
        .plan_bound_plugin_sidechain(&original.id, 0, &node_id, &binding)
        .unwrap();
    assert_eq!(
        plane.get_session(&original.id).unwrap(),
        &original,
        "planning cannot mutate the session"
    );
    for expected_replay in [false, true] {
        let result = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(), id: Some(json!(2)), method: "graph.commit".into(),
            params: Some(json!({ "planId": plan_id, "baseRevision": 0, "idempotencyKey": "sidechain-binding" })),
        });
        assert!(result.error.is_none(), "{:?}", result.error);
        assert_eq!(result.result.unwrap()["idempotentReplay"], expected_replay);
    }
    let changed = plane.get_session(&original.id).unwrap().clone();
    assert_eq!(changed.revision, 1);
    assert_eq!(
        changed.nodes[0].plugin_sidechain_port().unwrap().channels,
        2
    );
    assert!(
        plane
            .plan_bound_plugin_sidechain(&original.id, 0, &node_id, &binding)
            .is_err(),
        "stale revision must refuse"
    );
    assert_eq!(plane.get_session(&original.id).unwrap(), &changed);
    let undo = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "graph.undoPlan".into(),
        params: Some(json!({ "sessionId": original.id, "baseRevision": 1 })),
    });
    assert!(undo.error.is_none(), "{:?}", undo.error);
    let undo_plan = undo.result.unwrap()["planId"].clone();
    let result = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "graph.commit".into(),
        params: Some(
            json!({ "planId": undo_plan, "baseRevision": 1, "idempotencyKey": "undo-sidechain" }),
        ),
    });
    assert!(result.error.is_none(), "{:?}", result.error);
    let restored = plane.get_session(&original.id).unwrap();
    assert_eq!(restored.revision, 2);
    assert_eq!(restored.nodes, original.nodes);
    assert_eq!(restored.edges, original.edges);
}

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
fn graph_commit_can_require_its_session_to_be_stopped_for_recovery_edits() {
    let original = feedback_fixture();
    let mut plane = ControlPlane::new("stopped-commit-guard");
    plane.insert_session(original.clone()).unwrap();
    let request = |method: &str, params| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    };
    let mut candidate = original.clone();
    candidate.nodes[0].name = "Renamed input".into();
    let plan = plane
        .dispatch(request(
            "graph.plan",
            json!({"sessionId":original.id,"baseRevision":0,"candidate":candidate}),
        ))
        .result
        .unwrap();
    // A value of another type, and an empty string, are refused by the dispatcher's
    // schema check; only `null` reaches the handler's own message.
    for (key, value, message) in [
        (
            "null-stopped-guard",
            Value::Null,
            "must be a non-empty session ID",
        ),
        (
            "false-stopped-guard",
            json!(false),
            "requireStoppedSessionId must be a string",
        ),
        (
            "number-stopped-guard",
            json!(17),
            "requireStoppedSessionId must be a string",
        ),
        (
            "empty-stopped-guard",
            json!(""),
            "must be at least 1 character",
        ),
    ] {
        let invalid = plane.dispatch(request(
            "graph.commit",
            json!({
                "planId":plan["planId"],
                "baseRevision":0,
                "idempotencyKey":key,
                "requireStoppedSessionId":value
            }),
        ));
        assert!(invalid.error.unwrap().message.contains(message), "{key}");
    }
    let mismatched = plane.dispatch(request(
        "graph.commit",
        json!({
            "planId":plan["planId"],
            "baseRevision":0,
            "idempotencyKey":"wrong-session-guard",
            "requireStoppedSessionId":"another-session"
        }),
    ));
    assert!(mismatched
        .error
        .unwrap()
        .message
        .contains("does not match the graph plan"));
    plane.session_start(&original.id).unwrap();
    let response = plane.dispatch(request(
        "graph.commit",
        json!({
            "planId":plan["planId"],
            "baseRevision":0,
            "idempotencyKey":"stopped-only-rebind",
            "requireStoppedSessionId":"feedback"
        }),
    ));
    assert!(response
        .error
        .unwrap()
        .message
        .contains("session to be stopped"));
    assert_eq!(plane.get_session(&original.id).unwrap().revision, 0);
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
    assert!(
        plan_result["timing"]["planMicros"].as_u64().is_some(),
        "graph.plan reports its duration: {plan_result}"
    );
    let plan_id = plan_result["planId"].clone();
    let commit_request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "graph.commit".into(),
        params: Some(json!({ "planId": plan_id, "baseRevision": 0, "idempotencyKey": "api-op" })),
    };
    let committed = plane.dispatch(commit_request.clone()).result.unwrap();
    assert_eq!(committed["revision"], 1);
    // A simulated commit publishes no audio, so it reports no per-request time.
    assert_eq!(
        committed["activation"],
        json!({ "state": "pending", "runtime": "fake" })
    );
    // An idempotent replay must return the same activation shape; a fresh
    // per-request number here would differ and break clients that compare it.
    let replayed = plane.dispatch(commit_request).result.unwrap();
    assert_eq!(replayed["activation"], committed["activation"]);
    assert_eq!(replayed["revision"], committed["revision"]);
}

fn plan_through_dispatch(
    plane: &mut ControlPlane,
    session_id: &str,
    name: &str,
) -> Result<String, String> {
    let mut candidate = plane
        .get_session(&EntityId::new(session_id))
        .unwrap()
        .clone();
    candidate.name = name.into();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "graph.plan".into(),
        params: Some(json!({ "sessionId": session_id, "baseRevision": 0, "candidate": candidate })),
    });
    match response.error {
        Some(error) => Err(error.message),
        None => Ok(response.result.unwrap()["planId"]
            .as_str()
            .unwrap()
            .to_owned()),
    }
}

fn commit_through_dispatch(plane: &mut ControlPlane, plan_id: &str, key: &str) -> JsonRpcResponse {
    plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "graph.commit".into(),
        params: Some(json!({ "planId": plan_id, "baseRevision": 0, "idempotencyKey": key })),
    })
}

fn plan_exists(plane: &ControlPlane, plan_id: &str) -> (bool, bool) {
    (
        plane
            .store
            .plan_candidate(&EntityId::new(plan_id))
            .is_some(),
        plane
            .storage
            .as_ref()
            .is_some_and(|storage| storage.load_graph_plan(plan_id).unwrap().is_some()),
    )
}

/// PLAN-1: an assistant that previews with `graph.plan` and never commits
/// filled the global cap of 100 for five minutes, and every save then failed
/// with "Too many plans are waiting". A commit left the other plans of its
/// session behind although their base revision could no longer match. They
/// stay (a late commit must still hear `revisionConflict`, "plan again") but
/// are the first to go when the inventory is full.
#[test]
fn abandoned_plans_of_one_session_do_not_block_another_sessions_save() {
    let mut plane = ControlPlane::with_storage("plan-cap", Storage::open_memory().unwrap());
    let session_a = session();
    let mut session_b = session();
    session_b.id = EntityId::new("session-b");
    plane.insert_session(session_a).unwrap();
    plane.insert_session(session_b).unwrap();
    let mut plans = Vec::new();
    for index in 0..audiorouter_domain::MAX_PENDING_GRAPH_PLANS {
        plans.push(
            plan_through_dispatch(&mut plane, "session", &format!("preview {index}")).unwrap(),
        );
    }
    let committed = commit_through_dispatch(&mut plane, &plans[7], "commit-a");
    assert!(committed.error.is_none(), "{committed:?}");
    // The commit moved session A to revision 1, so its other plans are stale,
    // and a late commit of one keeps the documented "plan again" answer.
    let late = commit_through_dispatch(&mut plane, &plans[8], "commit-late");
    let error = late.error.expect("a stale plan cannot commit");
    assert_eq!(error.data.as_ref().unwrap()["code"], "revisionConflict");
    let saved = plan_through_dispatch(&mut plane, "session-b", "real save");
    assert!(saved.is_ok(), "{saved:?}");
    // With the inventory full of stale plans, saving never fails and the live
    // plans of session B are kept.
    let mut live = vec![saved.unwrap()];
    for index in 0..5 {
        live.push(
            plan_through_dispatch(&mut plane, "session-b", &format!("b {index}"))
                .unwrap_or_else(|message| panic!("{message}")),
        );
    }
    for plan in &live {
        assert_eq!(plan_exists(&plane, plan), (true, true));
    }
    let stale_left = plans
        .iter()
        .filter(|plan| plan_exists(&plane, plan) != (false, false))
        .count();
    assert!(
        stale_left <= audiorouter_domain::MAX_PENDING_GRAPH_PLANS - live.len(),
        "{stale_left} stale plans still hold the shared cap"
    );
}

#[test]
fn a_full_plan_inventory_drops_the_oldest_plan_instead_of_refusing_the_newest() {
    for storage in [None, Some(Storage::open_memory().unwrap())] {
        let durable = storage.is_some();
        let mut plane = match storage {
            Some(storage) => ControlPlane::with_storage("plan-evict", storage),
            None => ControlPlane::default(),
        };
        plane.insert_session(session()).unwrap();
        let mut plans = Vec::new();
        for index in 0..audiorouter_domain::MAX_PENDING_GRAPH_PLANS {
            plans.push(
                plan_through_dispatch(&mut plane, "session", &format!("preview {index}")).unwrap(),
            );
        }
        let newest = plan_through_dispatch(&mut plane, "session", "one more")
            .unwrap_or_else(|message| panic!("durable={durable}: {message}"));
        assert_eq!(
            plan_exists(&plane, &plans[0]),
            (false, false),
            "durable={durable}: the oldest plan is dropped"
        );
        assert!(
            plan_exists(&plane, &plans[1]).0,
            "durable={durable}: the next oldest stays"
        );
        let committed = commit_through_dispatch(&mut plane, &newest, "commit-newest");
        assert!(
            committed.error.is_none(),
            "durable={durable}: {committed:?}"
        );
    }
}

#[test]
fn a_running_commit_reports_its_own_timing() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane.session_start(&original.id).unwrap();
    let mut candidate = original.clone();
    candidate.name = "timed-edit".into();
    let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
    let committed = plane.commit_graph(&plan, 0, "timed-edit-op").unwrap();
    assert_eq!(committed["activation"]["state"], "running");
    assert!(
        committed["activation"]["timing"]["commitMicros"]
            .as_u64()
            .is_some(),
        "a running graph.commit reports its duration: {committed}"
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

#[test]
fn live_edit_classification_distinguishes_parameters_flags_and_topology() {
    let saved = feedback_fixture();
    assert_eq!(classify_live_edit(&saved, &saved), LiveEdit::FlagsOnly);

    let mut flag = saved.clone();
    flag.nodes[1].bypass = true;
    assert_eq!(classify_live_edit(&saved, &flag), LiveEdit::FlagsOnly);

    let mut parameter = saved.clone();
    parameter.nodes[1]
        .parameters
        .insert("gainDb".into(), json!(-6.0));
    let live = classify_live_edit(&saved, &parameter);
    assert_eq!(live, LiveEdit::ParametersOnly);
    assert!(live.retains_sources());

    let mut renamed = saved.clone();
    renamed.nodes[1].name = "Renamed".into();
    assert_eq!(classify_live_edit(&saved, &renamed), LiveEdit::Structural);
    assert!(!LiveEdit::Structural.retains_sources());

    let mut rewired = saved.clone();
    rewired.edges.pop();
    assert_eq!(classify_live_edit(&saved, &rewired), LiveEdit::Structural);

    let mut both = saved.clone();
    both.nodes[1].bypass = true;
    both.nodes[1]
        .parameters
        .insert("gainDb".into(), json!(-6.0));
    assert_eq!(classify_live_edit(&saved, &both), LiveEdit::ParametersOnly);
}

#[test]
fn only_a_layout_change_may_restart_the_live_route() {
    // A changed path layout needs Stop/Play; the old graph is intact.
    assert!(failed_activation_may_restart(
        &ControlError::RestartRequired("stop and press Play".into())
    ));
    // A failed preparation must never stop working audio.
    assert!(!failed_activation_may_restart(
        &ControlError::NodeNotReady {
            node_id: "clear".into(),
            message: "plugin did not load".into(),
        }
    ));
    assert!(!failed_activation_may_restart(
        &ControlError::InvalidRequest("multi-input graph rejected".into())
    ));
}

#[test]
fn only_a_prepared_publication_clears_the_stale_graph_marker() {
    let mut plane = ControlPlane::default();
    let session = EntityId::new("stale-route");
    plane.live_graph_stale.insert(session.clone());

    // Hold-to-compare and a flag toggle reuse the playing bridges by node
    // ID, so a marker left by a failed save must survive them.
    plane.record_live_publication(&session, LiveEdit::FlagsOnly);
    assert!(plane.live_graph_stale.contains(&session));

    // A parameter or structural publication prepared from the saved session.
    plane.record_live_publication(&session, LiveEdit::ParametersOnly);
    assert!(!plane.live_graph_stale.contains(&session));
}
