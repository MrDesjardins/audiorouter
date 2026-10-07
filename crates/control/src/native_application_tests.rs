//! Tests for `native_application.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn native_application_preparation_requires_device_administration_before_platform_access() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(91)),
            method: "nativeApplications.prepare".into(),
            params: Some(json!({ "processId": 1 })),
        },
        &ClientGrant::read_only(),
    );
    assert_eq!(response.error.unwrap().code, -32001);
    assert!(plane.endpoint_monitor.is_none());
    assert!(plane.native_endpoint_worker.is_none());
}

#[test]
fn native_application_worker_rejects_missing_graph_source_before_platform_access() {
    let mut plane = ControlPlane::default();
    let mut owned = session();
    owned.id = EntityId::new("application-worker-source-required");
    plane.create_session(owned).unwrap();
    let render = audiorouter_windows_audio::EndpointInfo {
        id: "render".into(),
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
    let error = plane
        .prepare_native_application_worker(
            EntityId::new("application-worker-source-required"),
            NativeApplicationWorkerConfig {
                process_id: 12_345,
                expected_executable: "probe.exe",
                expected_executable_path: None,
                expected_creation_time_100ns: 1,
                mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                render: &render,
                buffer_duration_100ns: 0,
                max_attempts: 1,
                retry_delay_ms: 0,
            },
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message)
            if message == "application worker requires a matching enabled applicationCapture node"
    ));
    assert!(plane.native_endpoint_worker.is_none());
    assert!(plane.endpoint_monitor.is_none());
}

#[test]
fn native_application_worker_rejects_graph_identity_mismatch_before_platform_access() {
    let mut owned = session();
    owned.id = EntityId::new("application-worker-identity-required");
    owned.nodes[0] = Node {
        id: EntityId::new("application"),
        kind: NodeKind::ApplicationCapture,
        type_version: 1,
        name: "Application capture".into(),
        enabled: true,
        bypass: false,
        parameters: [
            ("executable".into(), json!("actual.exe")),
            ("processPolicy".into(), json!("selectedInstance")),
            ("processId".into(), json!(42)),
            ("creationTime100ns".into(), json!("100")),
        ]
        .into_iter()
        .collect(),
        ports: vec![Port {
            name: "main".into(),
            direction: PortDirection::Output,
            channels: 2,
        }],
    };
    owned.nodes[1].ports[0].channels = 2;
    owned.edges[0].source_node = EntityId::new("application");
    owned.edges[0].matrix = vec![1.0, 0.0, 0.0, 1.0];
    let mut plane = ControlPlane::default();
    plane.create_session(owned).unwrap();
    let render = audiorouter_windows_audio::EndpointInfo {
        id: "render".into(),
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
    let error = plane
        .prepare_native_application_worker(
            EntityId::new("application-worker-identity-required"),
            NativeApplicationWorkerConfig {
                process_id: 42,
                expected_executable: "ACTUAL.EXE",
                expected_executable_path: None,
                expected_creation_time_100ns: 101,
                mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                render: &render,
                buffer_duration_100ns: 0,
                max_attempts: 1,
                retry_delay_ms: 0,
            },
        )
        .unwrap_err();
    assert!(
        matches!(error, ControlError::InvalidRequest(message) if message.contains("matching enabled"))
    );
    assert!(plane.endpoint_monitor.is_none());
    assert!(plane.native_endpoint_worker.is_none());
}

#[cfg(windows)]
#[test]
#[ignore = "requires explicit application identity, render endpoint, and AUDIOROUTER_ALLOW_LIVE_AUDIO=1"]
fn guarded_live_native_application_worker_lifecycle_uses_one_control_plane() {
    if std::env::var("AUDIOROUTER_ALLOW_LIVE_AUDIO").as_deref() != Ok("1") {
        return;
    }
    let process_id = std::env::var("AUDIOROUTER_APPLICATION_PROCESS_ID")
        .expect("AUDIOROUTER_APPLICATION_PROCESS_ID is required")
        .parse::<u32>()
        .expect("application process ID must be numeric");
    let executable = std::env::var("AUDIOROUTER_APPLICATION_EXECUTABLE")
        .expect("AUDIOROUTER_APPLICATION_EXECUTABLE is required");
    let creation_time = std::env::var("AUDIOROUTER_APPLICATION_CREATION_TIME_100NS")
        .expect("AUDIOROUTER_APPLICATION_CREATION_TIME_100NS is required")
        .parse::<u64>()
        .expect("application creation time must be numeric");
    let mode = match std::env::var("AUDIOROUTER_APPLICATION_MODE")
        .unwrap_or_else(|_| "include".into())
        .as_str()
    {
        "include" => audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
        "exclude" => audiorouter_windows_audio::ProcessLoopbackMode::ExcludeTargetTree,
        value => panic!("application mode must be include or exclude, got {value}"),
    };
    let render_id = std::env::var("AUDIOROUTER_RENDER_ENDPOINT_ID")
        .expect("AUDIOROUTER_RENDER_ENDPOINT_ID is required");
    let render = audiorouter_windows_audio::enumerate_active_endpoints()
        .unwrap()
        .into_iter()
        .find(|endpoint| {
            endpoint.id == render_id
                && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render
        })
        .expect("configured render endpoint is not active");
    let mut owned = session();
    owned.id = EntityId::new("guarded-live-application");
    owned.nodes[0] = Node {
        id: EntityId::new("application"),
        kind: NodeKind::ApplicationCapture,
        type_version: 1,
        name: "Application capture".into(),
        enabled: true,
        bypass: false,
        parameters: [
            ("executable".into(), json!(executable)),
            ("processPolicy".into(), json!("selectedInstance")),
            ("processId".into(), json!(process_id)),
            ("creationTime100ns".into(), json!(creation_time.to_string())),
        ]
        .into_iter()
        .collect(),
        ports: vec![Port {
            name: "main".into(),
            direction: PortDirection::Output,
            channels: 2,
        }],
    };
    owned.nodes[1].ports[0].channels = 2;
    owned.edges[0].source_node = EntityId::new("application");
    owned.edges[0].matrix = vec![1.0, 0.0, 0.0, 1.0];
    let mut plane = ControlPlane::default();
    plane.create_session(owned).unwrap();
    plane
        .prepare_native_application_worker(
            EntityId::new("guarded-live-application"),
            NativeApplicationWorkerConfig {
                process_id,
                expected_executable: &executable,
                expected_executable_path: std::env::var("AUDIOROUTER_APPLICATION_PATH")
                    .ok()
                    .as_deref(),
                expected_creation_time_100ns: creation_time,
                mode,
                render: &render,
                buffer_duration_100ns: 0,
                max_attempts: 3,
                retry_delay_ms: 100,
            },
        )
        .unwrap();
    let started = plane
        .session_start(&EntityId::new("guarded-live-application"))
        .unwrap();
    let generation = started["generation"].as_u64().unwrap();
    let started_at = Instant::now();
    let mut packets = 0_u64;
    let mut processed_quanta = 0_u64;
    let mut rendered_frames = 0_u64;
    while started_at.elapsed() < Duration::from_millis(500) {
        let result = plane
            .pump_native_endpoint_worker_with_bound_taps(
                &EntityId::new("guarded-live-application"),
                generation,
                64,
            )
            .unwrap();
        packets = packets.saturating_add(result["packets"].as_u64().unwrap_or(0));
        processed_quanta =
            processed_quanta.saturating_add(result["processedQuanta"].as_u64().unwrap_or(0));
        rendered_frames =
            rendered_frames.saturating_add(result["renderedFrames"].as_u64().unwrap_or(0));
        std::thread::sleep(Duration::from_millis(1));
    }
    plane
        .session_stop(&EntityId::new("guarded-live-application"))
        .unwrap();
    let restarted = plane
        .session_start(&EntityId::new("guarded-live-application"))
        .unwrap();
    let restarted_generation = restarted["generation"].as_u64().unwrap();
    let restarted_at = Instant::now();
    while restarted_at.elapsed() < Duration::from_millis(500) {
        let result = plane
            .pump_native_endpoint_worker_with_bound_taps(
                &EntityId::new("guarded-live-application"),
                restarted_generation,
                64,
            )
            .unwrap();
        packets = packets.saturating_add(result["packets"].as_u64().unwrap_or(0));
        processed_quanta =
            processed_quanta.saturating_add(result["processedQuanta"].as_u64().unwrap_or(0));
        rendered_frames =
            rendered_frames.saturating_add(result["renderedFrames"].as_u64().unwrap_or(0));
        std::thread::sleep(Duration::from_millis(1));
    }
    plane
        .session_stop(&EntityId::new("guarded-live-application"))
        .unwrap();
    plane.detach_native_endpoint_worker().unwrap();
    assert!(packets > 0);
    assert!(processed_quanta > 0);
    assert!(rendered_frames > 0);
}
