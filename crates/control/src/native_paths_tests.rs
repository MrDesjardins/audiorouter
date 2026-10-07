//! Tests for `native_paths.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn native_multi_inputs_prepare_param_allowlist_matches_the_real_request_shape() {
    let real_request = json!({
        "sessionId": "demo-session",
        "generation": 1,
        "sources": [
            { "kind": "physical", "endpointId": "capture-1" },
            {
                "kind": "application",
                "processId": 4242,
                "executable": "Zoom.exe",
                "executablePath": null,
                "creationTime100ns": "999999999",
                "mode": "include",
            },
        ],
    });
    assert!(validate_method_params("nativeMultiInputs.prepare", Some(&real_request)).is_ok());

    let stale_shape = json!({
        "sessionId": "demo-session",
        "generation": 1,
        "captureEndpointIds": ["capture-1", "capture-2"],
    });
    let error =
        validate_method_params("nativeMultiInputs.prepare", Some(&stale_shape)).unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message) if message.contains("unknown parameter")
    ));
}

#[test]
fn native_paths_prepare_takes_only_the_session_and_needs_device_administration() {
    let request = json!({ "sessionId": "patrick-main", "generation": 2 });
    assert!(validate_method_params("nativePaths.prepare", Some(&request)).is_ok());
    let with_sources = json!({ "sessionId": "patrick-main", "sources": [] });
    assert!(validate_method_params("nativePaths.prepare", Some(&with_sources)).is_err());
    let spec = API_METHODS
        .iter()
        .find(|spec| spec.name == "nativePaths.prepare")
        .expect("nativePaths.prepare is discoverable");
    assert_eq!(spec.permission, PermissionScope::DeviceAdministration);
    let response = ControlPlane::default().dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "nativePaths.prepare".into(),
            params: Some(request),
        },
        &ClientGrant::for_desktop_shell(),
    );
    assert_eq!(response.error.map(|error| error.code), Some(-32001));
}

#[cfg(windows)]
#[test]
fn native_paths_read_a_mono_endpoint_through_a_stereo_device_node() {
    let stereo = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let node = |id: &str, kind, ports: Vec<Port>| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports,
    };
    let session_id = EntityId::new("mono-mic");
    let mut plane = ControlPlane::default();
    plane
        .insert_session(Session {
            id: session_id.clone(),
            name: "mono mic".into(),
            schema_version: 1,
            revision: 0,
            nodes: vec![
                node(
                    "mic",
                    NodeKind::PhysicalInput,
                    vec![stereo("out", PortDirection::Output)],
                ),
                node(
                    "cable-a",
                    NodeKind::PhysicalOutput,
                    vec![stereo("in", PortDirection::Input)],
                ),
            ],
            edges: vec![Edge {
                id: EntityId::new("mic-cable-a"),
                source_node: EntityId::new("mic"),
                source_port: "out".into(),
                destination_node: EntityId::new("cable-a"),
                destination_port: "in".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            }],
        })
        .unwrap();
    plane.native_multi_input_mono_nodes = vec![EntityId::new("mic")];
    let adapted = plane.native_paths_session(&session_id).unwrap();
    assert_eq!(adapted.nodes[0].ports[0].channels, 1);
    assert_eq!(adapted.edges[0].matrix, vec![1.0, 1.0]);
    assert!(audiorouter_domain::validate_session(&adapted).is_ok());
    // The saved session keeps its stereo device node.
    assert_eq!(
        plane.get_session(&session_id).unwrap().nodes[0].ports[0].channels,
        2
    );
}

#[cfg(windows)]
#[test]
fn native_multi_input_preparation_validates_mixed_physical_and_application_sources() {
    let session_id = EntityId::new("mixed-multi-input");
    let session = Session {
        id: session_id.clone(),
        name: "mixed multi-input".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            Node {
                id: EntityId::new("mic"),
                kind: NodeKind::PhysicalInput,
                type_version: 1,
                name: "Microphone".into(),
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
                id: EntityId::new("app"),
                kind: NodeKind::ApplicationCapture,
                type_version: 1,
                name: "Zoom capture".into(),
                enabled: true,
                bypass: false,
                parameters: [
                    ("executable".to_string(), json!("Zoom.exe")),
                    ("processPolicy".to_string(), json!("selectedInstance")),
                    ("processId".to_string(), json!(4242u64)),
                    ("creationTime100ns".to_string(), json!("999999999")),
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
                id: EntityId::new("mic-mixer"),
                source_node: EntityId::new("mic"),
                source_port: "main".into(),
                destination_node: EntityId::new("mixer"),
                destination_port: "in".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("app-mixer"),
                source_node: EntityId::new("app"),
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
    let mic_endpoint = audiorouter_windows_audio::EndpointInfo {
        id: "mic-endpoint".into(),
        direction: audiorouter_windows_audio::EndpointDirection::Capture,
        default_period_100ns: 100_000,
        minimum_period_100ns: 30_000,
        sample_rate_hz: audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
        channels: 2,
        bits_per_sample: 32,
        format_tag: 3,
        channel_mask: 3,
        subformat_guid: "00000003-0000-0010-8000-00aa00389b71".into(),
    };

    // A binding kind that does not match the node kind at that graph
    // position is rejected before any device or process is opened.
    let mut plane = ControlPlane::default();
    plane.insert_session(session.clone()).unwrap();
    let swapped_kind_error = plane
        .prepare_native_multi_input_worker(
            session_id.clone(),
            1,
            &[
                NativeMultiInputSourceBinding::Application {
                    process_id: 4242,
                    expected_executable: "Zoom.exe",
                    expected_executable_path: None,
                    expected_creation_time_100ns: 999_999_999,
                    mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                },
                NativeMultiInputSourceBinding::Physical(&mic_endpoint),
            ],
            0,
            1,
            0,
        )
        .unwrap_err();
    assert!(matches!(
        swapped_kind_error,
        ControlError::InvalidRequest(message) if message.contains("physical inputs or application captures")
    ));

    // An application binding with an identity that does not match the
    // enabled node's persisted identity is rejected the same way.
    let mut plane = ControlPlane::default();
    plane.insert_session(session.clone()).unwrap();
    let mismatched_identity_error = plane
        .prepare_native_multi_input_worker(
            session_id.clone(),
            1,
            &[
                NativeMultiInputSourceBinding::Physical(&mic_endpoint),
                NativeMultiInputSourceBinding::Application {
                    process_id: 9999,
                    expected_executable: "Zoom.exe",
                    expected_executable_path: None,
                    expected_creation_time_100ns: 999_999_999,
                    mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                },
            ],
            0,
            1,
            0,
        )
        .unwrap_err();
    assert!(matches!(
        mismatched_identity_error,
        ControlError::InvalidRequest(message) if message.contains("does not match the enabled node's identity")
    ));

    // A correctly matching application binding passes identity
    // validation and proceeds to open a real process-loopback capture,
    // which fails in this test environment for audio/process reasons
    // rather than an identity or kind mismatch.
    let mut plane = ControlPlane::default();
    plane.insert_session(session).unwrap();
    let progressed_error = plane
        .prepare_native_multi_input_worker(
            session_id,
            1,
            &[
                NativeMultiInputSourceBinding::Physical(&mic_endpoint),
                NativeMultiInputSourceBinding::Application {
                    process_id: 4242,
                    expected_executable: "Zoom.exe",
                    expected_executable_path: None,
                    expected_creation_time_100ns: 999_999_999,
                    mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                },
            ],
            0,
            1,
            0,
        )
        .unwrap_err();
    assert!(
        !matches!(progressed_error, ControlError::InvalidRequest(ref message) if message.contains("does not match")
            || message.contains("physical inputs or application captures")),
        "expected validation to pass and fail later while opening the real source, got {progressed_error:?}"
    );
}
