//! Tests for `native_bindings.rs`.

use super::*;
use crate::test_support::*;

#[cfg(windows)]
#[test]
fn native_capture_sink_binding_rejects_mismatched_hello_before_driver_open() {
    let mut plane = ControlPlane::default();
    let error = plane
        .prepare_native_bridge(
            EntityId::new("bus-guard"),
            r"\\.\NotAudioRouter",
            r"C:\Temp\render.slot",
            r"C:\Temp\capture.slot",
            1,
            1_000,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message)
            if message.contains("native bridge device path is not the AudioRouter broker")
    ));
    let mut plane = ControlPlane::default();
    let bus_id = EntityId::new("capture-bus");
    plane.create_virtual_bus(bus_id.clone(), "Capture").unwrap();
    let hello = audiorouter_protocol::AudioBridgeHello {
        protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
        protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
        bus_id: "different-bus".into(),
        direction: audiorouter_protocol::AudioBridgeDirection::CaptureSink,
        generation: 1,
        sample_rate_hz: 48_000,
        channels: 2,
        frames_per_quantum: 128,
        lease_ms: 1_000,
    };
    let error = plane
        .prepare_native_capture_sink_binding(
            bus_id,
            "\\\\.\\AudioRouterVirtualBridge",
            "C:\\missing-capture.slot",
            hello,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message)
            if message == "native capture sink hello does not match the requested bus"
    ));
}

#[cfg(windows)]
#[test]
fn native_bridge_preparation_rejects_unbounded_or_relative_inputs_before_driver_open() {
    let mut plane = ControlPlane::default();
    let error = plane
        .prepare_native_bridge(
            EntityId::new("bounds-bus"),
            r"\\.\AudioRouterVirtualBridge",
            r"relative-render.slot",
            r"C:\capture.slot",
            1,
            0,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message)
            if message == "native bridge paths, lease, and generation are invalid"
    ));

    let mut plane = ControlPlane::default();
    let error = plane
        .prepare_native_bridge(
            EntityId::new("bounds-bus"),
            r"\\.\AudioRouterVirtualBridge",
            r"C:\render.slot",
            r"C:\capture.slot",
            0,
            1_000,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message)
            if message == "native bridge paths, lease, and generation are invalid"
    ));
}

#[cfg(windows)]
#[test]
fn native_render_source_binding_rejects_mismatched_hello_before_driver_open() {
    let mut plane = ControlPlane::default();
    let bus_id = EntityId::new("render-bus");
    plane.create_virtual_bus(bus_id.clone(), "Render").unwrap();
    let hello = audiorouter_protocol::AudioBridgeHello {
        protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
        protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
        bus_id: "different-bus".into(),
        direction: audiorouter_protocol::AudioBridgeDirection::RenderSource,
        generation: 1,
        sample_rate_hz: 48_000,
        channels: 2,
        frames_per_quantum: 128,
        lease_ms: 1_000,
    };
    let error = plane
        .prepare_native_render_source_binding(
            bus_id,
            "\\\\.\\AudioRouterVirtualBridge",
            "C:\\missing-render.slot",
            hello,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message)
            if message == "native render source hello does not match the requested bus"
    ));
}

#[cfg(windows)]
#[test]
fn native_duplex_binding_rejects_mismatched_hellos_before_driver_open() {
    let mut plane = ControlPlane::default();
    let bus_id = EntityId::new("duplex-bus");
    plane.create_virtual_bus(bus_id.clone(), "Duplex").unwrap();
    let render_hello = audiorouter_protocol::AudioBridgeHello {
        protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
        protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
        bus_id: bus_id.as_str().into(),
        direction: audiorouter_protocol::AudioBridgeDirection::RenderSource,
        generation: 2,
        sample_rate_hz: 48_000,
        channels: 2,
        frames_per_quantum: 128,
        lease_ms: 1_000,
    };
    let mut capture_hello = render_hello.clone();
    capture_hello.direction = audiorouter_protocol::AudioBridgeDirection::CaptureSink;
    capture_hello.generation = 3;
    let error = plane
        .prepare_native_duplex_binding(
            bus_id,
            "\\\\.\\AudioRouterVirtualBridge",
            "C:\\missing-render.slot",
            "C:\\missing-capture.slot",
            render_hello,
            capture_hello,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message)
            if message == "native duplex hellos do not match the requested bus"
    ));
}

#[test]
fn native_worker_preparation_rejects_mismatched_endpoint_shapes_before_opening() {
    let mut plane = ControlPlane::default();
    let mut owned = session();
    owned.id = EntityId::new("native-worker-shape-mismatch");
    plane.create_session(owned).unwrap();
    let endpoint = |direction, channels| audiorouter_windows_audio::EndpointInfo {
        id: format!("{direction:?}-{channels}"),
        direction,
        default_period_100ns: 100_000,
        minimum_period_100ns: 30_000,
        sample_rate_hz: 48_000,
        channels,
        bits_per_sample: 32,
        format_tag: 3,
        channel_mask: 3,
        subformat_guid: "00000003-0000-0010-8000-00aa00389b71".into(),
    };
    // Mono capture is intentionally supported for microphone routes and
    // duplicated into the stereo graph. Three channels remain invalid.
    let capture = endpoint(audiorouter_windows_audio::EndpointDirection::Capture, 3);
    let render = endpoint(audiorouter_windows_audio::EndpointDirection::Render, 2);

    let error = plane
        .prepare_native_endpoint_worker(
            EntityId::new("native-worker-shape-mismatch"),
            &capture,
            &render,
            0,
            1,
            0,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::Audio {
            code: "invalidArgument",
            ..
        }
    ));
    assert!(plane.native_endpoint_worker.is_none());
    assert!(plane.endpoint_monitor.is_none());
}
