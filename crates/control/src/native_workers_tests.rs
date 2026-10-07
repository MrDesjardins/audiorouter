//! Tests for `native_workers.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn native_worker_lifecycle_cannot_bypass_running_session_boundary() {
    let mut plane = ControlPlane::default();
    let mut owned = session();
    owned.id = EntityId::new("native-boundary");
    plane.insert_session(owned.clone()).unwrap();
    plane.session_start(&owned.id).unwrap();
    plane.native_endpoint_session = Some(owned.id.clone());

    assert!(matches!(
        plane.stop_native_endpoint_worker(),
        Err(ControlError::InvalidRequest(message))
            if message == "stop the session before stopping its native endpoint worker"
    ));
    assert!(matches!(
        plane.detach_native_endpoint_worker(),
        Err(ControlError::InvalidRequest(message))
            if message == "stop the session before detaching its native endpoint worker"
    ));
}

#[test]
fn native_graph_activation_rejects_missing_worker_before_graph_work() {
    let mut plane = ControlPlane::default();
    let mut owned = session();
    owned.id = EntityId::new("native-graph-without-worker");
    plane.create_session(owned.clone()).unwrap();
    let started = plane.session_start(&owned.id).unwrap();
    let generation = started["generation"].as_u64().unwrap();
    plane.native_endpoint_session = Some(owned.id.clone());

    let error = plane
        .activate_native_graph(&owned.id, generation, 48_000)
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message)
            if message == "native endpoint worker is not attached"
    ));
    assert!(plane.native_endpoint_taps.is_none());
}

#[cfg(windows)]
#[test]
#[ignore = "requires explicit live endpoint IDs and AUDIOROUTER_ALLOW_LIVE_AUDIO=1"]
fn guarded_live_native_endpoint_session_lifecycle_uses_one_control_plane() {
    if std::env::var("AUDIOROUTER_ALLOW_LIVE_AUDIO").as_deref() != Ok("1") {
        return;
    }
    let capture_id = std::env::var("AUDIOROUTER_CAPTURE_ENDPOINT_ID")
        .expect("AUDIOROUTER_CAPTURE_ENDPOINT_ID is required");
    let render_id = std::env::var("AUDIOROUTER_RENDER_ENDPOINT_ID")
        .expect("AUDIOROUTER_RENDER_ENDPOINT_ID is required");
    let fanout_endpoint_ids = std::env::var("AUDIOROUTER_OUTPUT_FANOUT_ENDPOINT_IDS")
        .unwrap_or_default()
        .split('|')
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let endpoints = audiorouter_windows_audio::enumerate_active_endpoints().unwrap();
    let capture = endpoints
        .iter()
        .find(|endpoint| {
            endpoint.id == capture_id
                && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Capture
        })
        .expect("configured capture endpoint is not an active exact match")
        .clone();
    let render = endpoints
        .iter()
        .find(|endpoint| {
            endpoint.id == render_id
                && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render
        })
        .expect("configured render endpoint is not an active exact match")
        .clone();
    let mut plane = ControlPlane::default();
    let mut owned = session();
    owned.id = EntityId::new("guarded-live-native");
    owned.nodes.insert(
        1,
        Node {
            id: EntityId::new("eq"),
            kind: NodeKind::ParametricEq,
            type_version: 1,
            name: "Live EQ".into(),
            enabled: true,
            bypass: false,
            parameters: [
                ("band0Enabled".into(), json!(true)),
                ("band0Type".into(), json!("peaking")),
                ("band0FrequencyHz".into(), json!(1000.0)),
                ("band0Q".into(), json!(1.0)),
                ("band0GainDb".into(), json!(-6.0)),
            ]
            .into_iter()
            .collect(),
            ports: vec![
                Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels: 1,
                },
                Port {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    channels: 1,
                },
            ],
        },
    );
    owned.nodes.insert(
        2,
        Node {
            id: EntityId::new("gate"),
            kind: NodeKind::Gate,
            type_version: 1,
            name: "Live Gate".into(),
            enabled: true,
            bypass: false,
            parameters: [
                ("thresholdDb".into(), json!(-45.0)),
                ("rangeDb".into(), json!(60.0)),
                ("hysteresisDb".into(), json!(3.0)),
                ("ratio".into(), json!(4.0)),
                ("attackMs".into(), json!(5.0)),
                ("holdMs".into(), json!(50.0)),
                ("releaseMs".into(), json!(150.0)),
            ]
            .into_iter()
            .collect(),
            ports: vec![
                Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels: 1,
                },
                Port {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    channels: 1,
                },
            ],
        },
    );
    owned.nodes.insert(
        3,
        Node {
            id: EntityId::new("compressor"),
            kind: NodeKind::Compressor,
            type_version: 1,
            name: "Live Compressor".into(),
            enabled: true,
            bypass: false,
            parameters: [
                ("thresholdDb".into(), json!(-18.0)),
                ("ratio".into(), json!(3.0)),
                ("attackMs".into(), json!(10.0)),
                ("releaseMs".into(), json!(150.0)),
                ("kneeDb".into(), json!(6.0)),
                ("makeupDb".into(), json!(0.0)),
            ]
            .into_iter()
            .collect(),
            ports: vec![
                Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels: 1,
                },
                Port {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    channels: 1,
                },
            ],
        },
    );
    owned.nodes.insert(
        4,
        Node {
            id: EntityId::new("pitch"),
            kind: NodeKind::Pitch,
            type_version: 1,
            name: "Live Pitch".into(),
            enabled: true,
            bypass: false,
            parameters: [
                ("semitones".into(), json!(2.0)),
                ("cents".into(), json!(0.0)),
            ]
            .into_iter()
            .collect(),
            ports: vec![
                Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels: 1,
                },
                Port {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    channels: 1,
                },
            ],
        },
    );
    owned.nodes.insert(
        5,
        Node {
            id: EntityId::new("limiter"),
            kind: NodeKind::Limiter,
            type_version: 1,
            name: "Live Limiter".into(),
            enabled: true,
            bypass: false,
            parameters: [
                ("ceilingDb".into(), json!(-1.0)),
                ("lookaheadMs".into(), json!(5.0)),
                ("releaseMs".into(), json!(100.0)),
            ]
            .into_iter()
            .collect(),
            ports: vec![
                Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels: 1,
                },
                Port {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    channels: 1,
                },
            ],
        },
    );
    owned.edges = vec![
        Edge {
            id: EntityId::new("edge-in-eq"),
            source_node: EntityId::new("in"),
            source_port: "main".into(),
            destination_node: EntityId::new("eq"),
            destination_port: "in".into(),
            matrix: vec![1.0],
            enabled: true,
        },
        Edge {
            id: EntityId::new("edge-eq-gate"),
            source_node: EntityId::new("eq"),
            source_port: "out".into(),
            destination_node: EntityId::new("gate"),
            destination_port: "in".into(),
            matrix: vec![1.0],
            enabled: true,
        },
        Edge {
            id: EntityId::new("edge-gate-compressor"),
            source_node: EntityId::new("gate"),
            source_port: "out".into(),
            destination_node: EntityId::new("compressor"),
            destination_port: "in".into(),
            matrix: vec![1.0],
            enabled: true,
        },
        Edge {
            id: EntityId::new("edge-compressor-pitch"),
            source_node: EntityId::new("compressor"),
            source_port: "out".into(),
            destination_node: EntityId::new("pitch"),
            destination_port: "in".into(),
            matrix: vec![1.0],
            enabled: true,
        },
        Edge {
            id: EntityId::new("edge-pitch-limiter"),
            source_node: EntityId::new("pitch"),
            source_port: "out".into(),
            destination_node: EntityId::new("limiter"),
            destination_port: "in".into(),
            matrix: vec![1.0],
            enabled: true,
        },
        Edge {
            id: EntityId::new("edge-limiter-out"),
            source_node: EntityId::new("limiter"),
            source_port: "out".into(),
            destination_node: EntityId::new("out"),
            destination_port: "main".into(),
            matrix: vec![1.0],
            enabled: true,
        },
    ];
    plane.insert_session(owned.clone()).unwrap();
    plane
        .prepare_native_endpoint_worker(owned.id.clone(), &capture, &render, 0, 3, 100)
        .unwrap();
    if !fanout_endpoint_ids.is_empty() {
        plane
            .prepare_native_output_fanout(owned.id.clone(), 1, &fanout_endpoint_ids)
            .unwrap();
    }
    let started = plane.session_start(&owned.id).unwrap();
    assert_eq!(started["runtime"], "native");
    assert_eq!(
        plane.native_endpoint_lifecycle_telemetry()["successfulStarts"],
        1
    );
    let generation = started["generation"].as_u64().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
    let mut captured_frames = 0_u64;
    let mut processed_quanta = 0_u64;
    let mut rendered_frames = 0_u64;
    let mut fanout_packets = 0_u64;
    let mut fanout_rendered_frames = 0_u64;
    while std::time::Instant::now() < deadline {
        let pump = plane
            .pump_native_endpoint_worker_with_bound_taps(&owned.id, generation, 64)
            .unwrap();
        captured_frames = captured_frames.saturating_add(pump["capturedFrames"].as_u64().unwrap());
        processed_quanta =
            processed_quanta.saturating_add(pump["processedQuanta"].as_u64().unwrap());
        rendered_frames = rendered_frames.saturating_add(pump["renderedFrames"].as_u64().unwrap());
        if let Some(fanout) = pump.get("outputFanout") {
            fanout_packets = fanout_packets.saturating_add(fanout["packets"].as_u64().unwrap());
            fanout_rendered_frames =
                fanout_rendered_frames.saturating_add(fanout["renderedFrames"].as_u64().unwrap());
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(captured_frames > 0, "native worker did not capture frames");
    assert!(
        processed_quanta > 0,
        "native worker did not process graph quanta"
    );
    assert!(rendered_frames > 0, "native worker did not render frames");
    if !fanout_endpoint_ids.is_empty() {
        assert!(fanout_packets > 0, "output fan-out did not drain packets");
        assert!(
            fanout_rendered_frames > 0,
            "output fan-out did not render frames"
        );
    }
    eprintln!(
        "guarded_native_lifecycle capture_frames={captured_frames} processed_quanta={processed_quanta} rendered_frames={rendered_frames} fanout_packets={fanout_packets} fanout_rendered_frames={fanout_rendered_frames}"
    );
    let privacy_enabled = plane
        .dispatch_privacy_mute(Some(json!({
            "muted": true,
            "idempotencyKey": "guarded-live-privacy-enable"
        })))
        .unwrap();
    assert_eq!(privacy_enabled["muted"], true);
    assert_eq!(
        plane.status_snapshot().unwrap()["privacyMute"]["muted"],
        true
    );
    let mut mute_dispatch_to_processed_block = Vec::with_capacity(8);
    for sample in 0..8 {
        let before = plane
            .native_scheduler_telemetry()
            .get("processedQuanta")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let started = std::time::Instant::now();
        let response = plane
            .dispatch_privacy_mute(Some(json!({
                "muted": true,
                "idempotencyKey": format!("guarded-live-privacy-sample-{sample}"),
            })))
            .unwrap();
        assert_eq!(response["muted"], true);
        let deadline = started + std::time::Duration::from_millis(100);
        let mut observed = false;
        while std::time::Instant::now() < deadline {
            let pump = plane
                .pump_native_endpoint_worker_with_bound_taps(&owned.id, generation, 64)
                .unwrap();
            if pump["processedQuanta"].as_u64().unwrap_or(0) > 0 {
                observed = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(observed, "privacy mute did not reach a processed block");
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        mute_dispatch_to_processed_block.push(elapsed_ms);
        let response = plane
            .dispatch_privacy_mute(Some(json!({
                "muted": false,
                "idempotencyKey": format!("guarded-live-privacy-clear-{sample}"),
            })))
            .unwrap();
        assert_eq!(response["muted"], false);
        assert!(
            plane
                .native_scheduler_telemetry()
                .get("processedQuanta")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= before
        );
    }
    mute_dispatch_to_processed_block.sort_by(f64::total_cmp);
    let p95_index = (mute_dispatch_to_processed_block.len() * 95).div_ceil(100) - 1;
    let p95_ms = mute_dispatch_to_processed_block[p95_index];
    assert!(
        p95_ms <= 100.0,
        "privacy mute dispatch-to-processed p95 exceeded 100 ms: {p95_ms:.3} ms"
    );
    eprintln!("guarded_native_privacy_mute_dispatch_to_processed_p95_ms={p95_ms:.3}");
    let privacy_disabled = plane
        .dispatch_privacy_mute(Some(json!({
            "muted": false,
            "idempotencyKey": "guarded-live-privacy-disable"
        })))
        .unwrap();
    assert_eq!(privacy_disabled["muted"], false);
    assert_eq!(
        plane.status_snapshot().unwrap()["privacyMute"]["muted"],
        false
    );
    let stopped = plane.session_stop(&owned.id).unwrap();
    assert_eq!(stopped["runtime"], "native");
    assert_eq!(
        plane.native_endpoint_lifecycle_telemetry()["successfulStops"],
        1
    );
    if !fanout_endpoint_ids.is_empty() {
        plane.detach_native_output_fanout().unwrap();
    }
    plane
        .rebind_native_endpoint_worker(&owned.id, &capture_id, &render_id, 0, 3, 100)
        .unwrap();
    assert_eq!(
        plane.native_endpoint_lifecycle_telemetry()["successfulStarts"],
        1
    );
    let rebound = plane.session_start(&owned.id).unwrap();
    assert_eq!(rebound["runtime"], "native");
    plane.session_stop(&owned.id).unwrap();
    assert_eq!(
        plane.native_endpoint_lifecycle_telemetry()["successfulStops"],
        2
    );
}

#[cfg(windows)]
#[test]
#[ignore = "requires explicit live endpoint IDs and AUDIOROUTER_ALLOW_LIVE_AUDIO=1"]
fn guarded_live_test_signal_reaches_destination_meter() {
    if std::env::var("AUDIOROUTER_ALLOW_LIVE_AUDIO").as_deref() != Ok("1") {
        return;
    }
    let capture_id = std::env::var("AUDIOROUTER_CAPTURE_ENDPOINT_ID")
        .expect("AUDIOROUTER_CAPTURE_ENDPOINT_ID is required");
    let render_id = std::env::var("AUDIOROUTER_RENDER_ENDPOINT_ID")
        .expect("AUDIOROUTER_RENDER_ENDPOINT_ID is required");
    let endpoints = audiorouter_windows_audio::enumerate_active_endpoints().unwrap();
    let capture = endpoints
        .iter()
        .find(|endpoint| {
            endpoint.id == capture_id
                && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Capture
        })
        .expect("configured capture endpoint is not an active exact match")
        .clone();
    let render = endpoints
        .iter()
        .find(|endpoint| {
            endpoint.id == render_id
                && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render
        })
        .expect("configured render endpoint is not an active exact match")
        .clone();

    let session_id = EntityId::new("guarded-live-test-signal");
    let mut owned = session();
    owned.id = session_id.clone();
    owned.nodes = vec![
        Node {
            id: EntityId::new("test-signal"),
            kind: NodeKind::TestSignal,
            type_version: 1,
            name: "Test Signal".into(),
            enabled: true,
            bypass: false,
            parameters: [
                ("frequencyHz".into(), json!(440.0)),
                ("levelDb".into(), json!(-18.0)),
                ("durationMs".into(), json!(600_000.0)),
            ]
            .into_iter()
            .collect(),
            ports: vec![Port {
                name: "main".into(),
                direction: PortDirection::Output,
                channels: 2,
            }],
        },
        Node {
            id: EntityId::new("out"),
            kind: NodeKind::PhysicalOutput,
            type_version: 1,
            name: "Destination".into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![Port {
                name: "main".into(),
                direction: PortDirection::Input,
                channels: 2,
            }],
        },
    ];
    owned.edges = vec![Edge {
        id: EntityId::new("edge-test-signal-output"),
        source_node: EntityId::new("test-signal"),
        source_port: "main".into(),
        destination_node: EntityId::new("out"),
        destination_port: "main".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    }];

    let mut plane = ControlPlane::default();
    let mut initial = session();
    initial.id = session_id.clone();
    plane.insert_session(initial).unwrap();
    let plan_id = plane.plan_graph(&session_id, 0, owned).unwrap();
    let committed = plane
        .commit_graph(&plan_id, 0, "guarded-test-signal")
        .unwrap();
    assert_eq!(committed["revision"], 1);
    plane
        .prepare_native_endpoint_worker(session_id.clone(), &capture, &render, 0, 3, 100)
        .unwrap();
    let started = plane.session_start(&session_id).unwrap();
    let generation = started["generation"].as_u64().unwrap();
    plane
        .dispatch_audio_source_transport(Some(json!({
            "sessionId": session_id,
            "nodeId": "test-signal",
            "action": "play"
        })))
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
    let mut processed_quanta = 0_u64;
    while std::time::Instant::now() < deadline {
        let pump = plane
            .pump_native_endpoint_worker_with_bound_taps(&session_id, generation, 64)
            .unwrap();
        processed_quanta =
            processed_quanta.saturating_add(pump["processedQuanta"].as_u64().unwrap_or(0));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        processed_quanta > 0,
        "Test Signal graph did not process quanta"
    );
    let telemetry = plane.native_node_telemetry();
    let destination = telemetry
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["nodeId"] == "out"))
        .expect("destination node telemetry is missing");
    assert!(
        destination["meter"]["peakDb"].as_f64().unwrap_or(-120.0) > -120.0,
        "destination meter remained at the finite silence floor: {destination}"
    );
    eprintln!(
        "guarded_test_signal_meter processed_quanta={} destination_peak_db={:.3}",
        processed_quanta,
        destination["meter"]["peakDb"].as_f64().unwrap_or(-120.0)
    );
    plane.session_stop(&session_id).unwrap();
    plane.detach_native_endpoint_worker().unwrap();
}

#[cfg(windows)]
#[test]
#[ignore = "live Windows audio devices and plugins"]
fn live_native_paths_start_pump_and_report_signal_timing() {
    let Some(database) = std::env::var_os("AUDIOROUTER_LIVE_PATHS_DATABASE") else {
        return;
    };
    let session_id = std::env::var("AUDIOROUTER_LIVE_PATHS_SESSION")
        .unwrap_or_else(|_| "patrick-main-session".into());
    let storage = audiorouter_storage::Storage::open(std::path::Path::new(&database)).unwrap();
    let mut plane = ControlPlane::with_storage("live-paths", storage);
    // Idempotency survives database copies and backend restart. Each
    // qualification is a new operation, rather than replaying a previous
    // test's successful start response on a currently stopped backend.
    let run_id = format!(
        "live-paths-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut call = |method: &str, params: Value| {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: (!params.is_null()).then_some(params),
        });
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
        response.result.unwrap()
    };
    call(
        "safety.setPrivacyMute",
        json!({ "muted": true, "idempotencyKey": format!("{run_id}-mute") }),
    );
    let prepared = call("nativePaths.prepare", json!({ "sessionId": session_id }));
    eprintln!("prepared: {prepared}");
    let started = call(
        "session.start",
        json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-start") }),
    );
    eprintln!("started: {started}");
    assert_eq!(started["runtime"], "native");
    let generation = started["generation"].as_u64().unwrap();
    let until = Instant::now() + Duration::from_secs(4);
    let mut delivered = 0_u64;
    while Instant::now() < until {
        let pump = call(
            "nativeMultiInputs.pump",
            json!({ "sessionId": session_id, "generation": generation, "maxPackets": 64 }),
        );
        delivered += pump["deliveredQuanta"].as_u64().unwrap_or(0);
        std::thread::sleep(Duration::from_millis(10));
    }
    let diagnostics = call("system.diagnostics", Value::Null);
    if let Some(path) = std::env::var_os("AUDIOROUTER_LIVE_DIAGNOSTICS_DUMP") {
        std::fs::write(path, serde_json::to_vec_pretty(&diagnostics).unwrap()).unwrap();
    }
    for item in diagnostics["nodeTelemetry"].as_array().unwrap() {
        eprintln!(
            "{} timing={} plugin={}",
            item["nodeId"], item["timing"], item["plugin"]
        );
        if let Some(levels) = item["spectrum"]["levelsDb"].as_array() {
            eprintln!(
                "{} spectrum (first 8 of {} bands, dB): {:?}",
                item["nodeId"],
                levels.len(),
                &levels[..8.min(levels.len())]
            );
        }
    }
    eprintln!("delivered branch blocks: {delivered}");
    call(
        "session.stop",
        json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-stop") }),
    );
    let timed = diagnostics["nodeTelemetry"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["timing"]["delayMs"].is_number())
        .count();
    assert!(
        timed > 0,
        "signal timing is reported while the session runs"
    );
    let failed_plugins = diagnostics["nodeTelemetry"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["plugin"].is_object() && item["plugin"]["state"] != "running")
        .map(|item| item["nodeId"].clone())
        .collect::<Vec<_>>();
    assert!(
        failed_plugins.is_empty(),
        "plugins failed while running: {failed_plugins:?}"
    );
    assert!(delivered > 0, "both paths must deliver audio blocks");
    if session_id == "patrick-main-session" {
        // The session's consecutive ReaPlugs share one worker.
        let session = EntityId::new(&session_id);
        let plugin_ids = plane
            .get_session(&session)
            .unwrap()
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Plugin && node.enabled)
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let members = plugin_ids
            .iter()
            .map(|id| plane.plugin_bridge(&session, id).unwrap())
            .collect::<Vec<_>>();
        assert!(
            members
                .iter()
                .all(|member| member.shares_worker_with(&members[0])),
            "Patrick's ReaPlugs must share one worker"
        );
        eprintln!(
            "verified shared plugin worker: {}",
            plugin_ids
                .iter()
                .map(EntityId::as_str)
                .collect::<Vec<_>>()
                .join(" -> ")
        );
    }
}

/// Qualify the exact saved Meter topology without writing the user DB or
/// recording audio. All test-owned output is privacy-muted. The caller
/// supplies an API session snapshot and must ensure endpoints are free.
#[cfg(windows)]
#[test]
#[ignore = "live exact Windows endpoints; privacy-muted isolated session"]
fn live_native_meter_saved_route_starts_pumps_and_resets() {
    let Some(path) = std::env::var_os("AUDIOROUTER_LIVE_METER_SESSION_JSON") else {
        return;
    };
    let text = std::fs::read_to_string(path).unwrap();
    let session: Session = serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap();
    assert!(
        session
            .nodes
            .iter()
            .all(|node| node.kind != NodeKind::Plugin),
        "this fixture is native-only"
    );
    let session_id = session.id.clone();
    let meter_id = session
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Meter)
        .expect("fixture contains a Meter")
        .id
        .clone();
    let mut plane = ControlPlane::default();
    plane.insert_session(session.clone()).unwrap();
    let key = format!("meter-{}", std::process::id());
    let request = |method: &str, params: Value| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    };
    assert!(plane
        .dispatch(request(
            "safety.setPrivacyMute",
            json!({"muted":true,"idempotencyKey":format!("{key}-mute")})
        ))
        .error
        .is_none());
    let prepared = plane.dispatch(request(
        "nativePaths.prepare",
        json!({"sessionId":session_id}),
    ));
    assert!(prepared.error.is_none(), "prepare: {:?}", prepared.error);
    let started = plane.dispatch(request(
        "session.start",
        json!({"sessionId":session_id,"idempotencyKey":format!("{key}-start")}),
    ));
    assert!(started.error.is_none(), "start: {:?}", started.error);
    let generation = started.result.unwrap()["generation"].as_u64().unwrap();
    let deadline = Instant::now() + Duration::from_secs(4);
    let mut delivered = 0;
    while Instant::now() < deadline {
        let pump = plane.dispatch(request(
            "nativeMultiInputs.pump",
            json!({"sessionId":session_id,"generation":generation,"maxPackets":64}),
        ));
        assert!(pump.error.is_none(), "pump: {:?}", pump.error);
        delivered += pump.result.unwrap()["deliveredQuanta"]
            .as_u64()
            .unwrap_or(0);
        std::thread::sleep(Duration::from_millis(2));
    }
    let diagnostics = plane
        .dispatch(request("system.diagnostics", json!({})))
        .result
        .unwrap();
    let meter = diagnostics["nodeTelemetry"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["nodeId"] == json!(meter_id))
        .expect("native Meter telemetry");
    assert!(meter["meter"]["observedFrames"].as_u64().unwrap() > 0);
    let reset = plane.dispatch(request(
        "meters.reset",
        json!({"sessionId":session_id,"nodeId":meter_id}),
    ));
    assert!(reset.error.is_none(), "reset: {:?}", reset.error);
    assert_eq!(reset.result.unwrap()["reset"], true);
    let after = plane
        .dispatch(request("system.diagnostics", json!({})))
        .result
        .unwrap();
    let meter = after["nodeTelemetry"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["nodeId"] == json!(meter_id))
        .unwrap();
    assert_eq!(meter["meter"]["observedFrames"], 0);
    assert_eq!(
        plane.get_session(&session_id).unwrap(),
        &session,
        "runtime reset does not save or alter graph"
    );
    assert!(plane
        .dispatch(request(
            "session.stop",
            json!({"sessionId":session_id,"idempotencyKey":format!("{key}-stop")})
        ))
        .error
        .is_none());
    assert!(delivered > 0);
    eprintln!("native Meter route prepared, started, delivered {delivered} branch blocks, exposed frames, reset and stopped; saved graph unchanged");
}

#[cfg(windows)]
#[test]
#[ignore = "requires explicit multi-capture/render endpoint IDs and AUDIOROUTER_ALLOW_LIVE_AUDIO=1"]
fn guarded_live_native_multi_input_many_output_lifecycle() {
    if std::env::var("AUDIOROUTER_ALLOW_LIVE_AUDIO").as_deref() != Ok("1") {
        return;
    }
    let capture_ids = std::env::var("AUDIOROUTER_MULTI_CAPTURE_ENDPOINT_IDS")
        .expect("AUDIOROUTER_MULTI_CAPTURE_ENDPOINT_IDS is required")
        .split('|')
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let render_ids = std::env::var("AUDIOROUTER_MULTI_RENDER_ENDPOINT_IDS")
        .expect("AUDIOROUTER_MULTI_RENDER_ENDPOINT_IDS is required")
        .split('|')
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert!(capture_ids.len() >= 2 && capture_ids.len() <= 8);
    assert!(render_ids.len() >= 2 && render_ids.len() <= 8);
    let endpoints = audiorouter_windows_audio::enumerate_active_endpoints().unwrap();
    let captures = capture_ids
        .iter()
        .map(|id| {
            endpoints
                .iter()
                .find(|endpoint| {
                    endpoint.id == *id
                        && endpoint.direction
                            == audiorouter_windows_audio::EndpointDirection::Capture
                })
                .expect("configured capture endpoint is not an active exact match")
                .clone()
        })
        .collect::<Vec<_>>();
    let _renders = render_ids
        .iter()
        .map(|id| {
            endpoints
                .iter()
                .find(|endpoint| {
                    endpoint.id == *id
                        && endpoint.direction
                            == audiorouter_windows_audio::EndpointDirection::Render
                })
                .expect("configured render endpoint is not an active exact match")
                .clone()
        })
        .collect::<Vec<_>>();
    let mut plane = ControlPlane::default();
    let session_id = EntityId::new("guarded-live-native-multi");
    let owned = Session {
        id: session_id.clone(),
        name: "guarded multi-input many-output".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            Node {
                id: EntityId::new("input-a"),
                kind: NodeKind::PhysicalInput,
                type_version: 1,
                name: "Input A".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![Port {
                    name: "main".into(),
                    direction: PortDirection::Output,
                    channels: 2,
                }],
            },
            Node {
                id: EntityId::new("input-b"),
                kind: NodeKind::PhysicalInput,
                type_version: 1,
                name: "Input B".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![Port {
                    name: "main".into(),
                    direction: PortDirection::Output,
                    channels: 2,
                }],
            },
            Node {
                id: EntityId::new("mixer"),
                kind: NodeKind::Mixer,
                type_version: 1,
                name: "Mixer".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![
                    Port {
                        name: "in".into(),
                        direction: PortDirection::Input,
                        channels: 2,
                    },
                    Port {
                        name: "out".into(),
                        direction: PortDirection::Output,
                        channels: 2,
                    },
                ],
            },
            Node {
                id: EntityId::new("output-a"),
                kind: NodeKind::PhysicalOutput,
                type_version: 1,
                name: "Output A".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![Port {
                    name: "main".into(),
                    direction: PortDirection::Input,
                    channels: 2,
                }],
            },
            Node {
                id: EntityId::new("output-b"),
                kind: NodeKind::PhysicalOutput,
                type_version: 1,
                name: "Output B".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![Port {
                    name: "main".into(),
                    direction: PortDirection::Input,
                    channels: 2,
                }],
            },
        ],
        edges: vec![
            Edge {
                id: EntityId::new("input-a-mixer"),
                source_node: EntityId::new("input-a"),
                source_port: "main".into(),
                destination_node: EntityId::new("mixer"),
                destination_port: "in".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("input-b-mixer"),
                source_node: EntityId::new("input-b"),
                source_port: "main".into(),
                destination_node: EntityId::new("mixer"),
                destination_port: "in".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("mixer-output-a"),
                source_node: EntityId::new("mixer"),
                source_port: "out".into(),
                destination_node: EntityId::new("output-a"),
                destination_port: "main".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("mixer-output-b"),
                source_node: EntityId::new("mixer"),
                source_port: "out".into(),
                destination_node: EntityId::new("output-b"),
                destination_port: "main".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            },
        ],
    };
    plane.insert_session(owned.clone()).unwrap();
    let bindings = captures
        .iter()
        .map(NativeMultiInputSourceBinding::Physical)
        .collect::<Vec<_>>();
    plane
        .prepare_native_multi_input_worker(session_id.clone(), 1, &bindings, 0, 3, 100)
        .unwrap();
    plane
        .prepare_native_output_fanout(session_id.clone(), 1, &render_ids)
        .unwrap();
    let started = plane.session_start(&session_id).unwrap();
    let generation = started["generation"].as_u64().unwrap();
    assert_eq!(generation, 1);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
    let mut captured_frames = 0_u64;
    let mut delivered_quanta = 0_u64;
    let mut rendered_frames = 0_u64;
    while std::time::Instant::now() < deadline {
        let pump = plane
            .pump_native_multi_input_worker(&session_id, generation, 64)
            .unwrap();
        captured_frames = captured_frames.saturating_add(pump["capturedFrames"].as_u64().unwrap());
        delivered_quanta =
            delivered_quanta.saturating_add(pump["deliveredQuanta"].as_u64().unwrap());
        rendered_frames = rendered_frames.saturating_add(pump["renderedFrames"].as_u64().unwrap());
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        captured_frames > 0,
        "multi-input worker did not capture frames"
    );
    assert!(
        delivered_quanta > 0,
        "multi-input worker delivered no quanta"
    );
    assert!(rendered_frames > 0, "multi-input worker rendered no frames");
    eprintln!(
        "guarded_native_multi capture_frames={captured_frames} delivered_quanta={delivered_quanta} rendered_frames={rendered_frames}"
    );
    let stopped = plane.session_stop(&session_id).unwrap();
    assert_eq!(stopped["state"], "stopped");
}
