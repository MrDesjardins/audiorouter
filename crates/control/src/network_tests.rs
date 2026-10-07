//! Tests for `network.rs`.

use super::*;
use crate::test_support::*;

#[cfg(windows)]
#[test]
fn network_log_keeps_only_the_windows_socket_error_code() {
    assert_eq!(os_error_code("network I/O failed: Only one usage of each socket address is normally permitted. (os error 10048)"), Some(10048));
    assert_eq!(os_error_code("no code here"), None);
    assert_eq!(os_error_code("(os error x)"), None);
}

/// Live check of a saved multi-path session on this machine's devices:
/// prepare, start (with plugins), pump, read signal timing, stop. Privacy
/// mute is set first so nothing is audible. Point
/// `AUDIOROUTER_LIVE_PATHS_DATABASE` at a COPY of a database holding the
/// session (`AUDIOROUTER_LIVE_PATHS_SESSION`, default
/// `patrick-main-session`) and `AUDIOROUTER_PLUGIN_WORKER_PATH` at a built
/// plugin worker.
/// The whole product path of the network tools, with no audio device:
/// Test Signal -> Network Send -> UDP (127.0.0.1) -> Network Receive ->
/// Recorder, prepared and started like Play, serviced by the backend
/// audio loop, reported in telemetry, and recorded as a continuous tone.
#[cfg(windows)]
#[test]
fn network_send_and_receive_carry_a_continuous_tone_through_the_backend() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-net-route-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let port_number = std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let node = |id: &str, kind, ports, parameters: Value| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(parameters).unwrap(),
        ports,
    };
    let edge = |from: &str, to: &str| audiorouter_domain::Edge {
        id: EntityId::new(format!("{from}-{to}")),
        source_node: EntityId::new(from),
        source_port: "out".into(),
        destination_node: EntityId::new(to),
        destination_port: "in".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    };
    let tone = node(
        "tone",
        NodeKind::TestSignal,
        vec![port("out", PortDirection::Output)],
        json!({ "frequencyHz": 997.0, "levelDb": -12.0, "durationMs": 600000.0 }),
    );
    let send = node(
        "net-send",
        NodeKind::NetworkSend,
        vec![port("in", PortDirection::Input)],
        json!({ "host": "127.0.0.1", "port": port_number }),
    );
    let receive = node(
        "net-receive",
        NodeKind::NetworkReceive,
        vec![port("out", PortDirection::Output)],
        json!({ "sender": "127.0.0.1", "port": port_number, "bufferMs": 40.0 }),
    );
    let rec = node(
        "rec",
        NodeKind::Recorder,
        vec![port("in", PortDirection::Input)],
        json!({ "format": "wavFloat32" }),
    );
    let session = |id: &str, nodes: Vec<Node>, edges| Session {
        id: EntityId::new(id),
        name: id.into(),
        schema_version: 1,
        revision: 0,
        nodes,
        edges,
    };
    // One session holding both ends, then two separate backends (two
    // computers: each its own worker, clock and timeline).
    for two_computers in [false, true] {
        let sessions = if two_computers {
            vec![
                session(
                    "sending-pc",
                    vec![tone.clone(), send.clone()],
                    vec![edge("tone", "net-send")],
                ),
                session(
                    "receiving-pc",
                    vec![receive.clone(), rec.clone()],
                    vec![edge("net-receive", "rec")],
                ),
            ]
        } else {
            vec![session(
                "network-route",
                vec![tone.clone(), send.clone(), receive.clone(), rec.clone()],
                vec![edge("tone", "net-send"), edge("net-receive", "rec")],
            )]
        };
        let mut planes = sessions
            .into_iter()
            .map(|session| {
                let id = session.id.clone();
                let mut plane = ControlPlane::default();
                plane.insert_session(session).unwrap();
                plane.configure_recording_root(&root).unwrap();
                (plane, id)
            })
            .collect::<Vec<_>>();
        let call = |plane: &mut ControlPlane, method: &str, params: Value| {
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(params),
            });
            assert!(response.error.is_none(), "{method}: {:?}", response.error);
            response.result.unwrap()
        };
        // Receiver first, as on two computers it is usually already playing.
        for (plane, id) in planes.iter_mut().rev() {
            call(plane, "nativePaths.prepare", json!({ "sessionId": id }));
            call(
                plane,
                "session.start",
                json!({ "sessionId": id, "idempotencyKey": "net-start" }),
            );
        }
        let service = |planes: &mut Vec<(ControlPlane, EntityId)>, seconds: f64| {
            let until = Instant::now() + Duration::from_secs_f64(seconds);
            while Instant::now() < until {
                for (plane, _) in planes.iter_mut() {
                    plane.service_running_native_audio(Instant::now());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        };
        // Play the tone (as the node's Play button does), let the jitter
        // buffer fill, then record three seconds.
        let sender = 0;
        let receiver = planes.len() - 1;
        let (plane, id) = &mut planes[sender];
        call(
            plane,
            "audioSources.transport",
            json!({ "sessionId": id, "nodeId": "tone", "action": "play" }),
        );
        service(&mut planes, 0.5);
        let (plane, id) = &mut planes[receiver];
        let started = call(
            plane,
            "recorders.startRecording",
            json!({ "sessionId": id, "nodeId": "rec", "idempotencyKey": "net-rec" }),
        );
        let path = std::path::PathBuf::from(started["path"].as_str().unwrap());
        service(&mut planes, 3.0);
        let telemetry = |plane: &mut ControlPlane, node_id: &str| {
            let diagnostics = call(plane, "system.diagnostics", json!({}));
            diagnostics["nodeTelemetry"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["nodeId"] == node_id)
                .map(|item| item["network"].clone())
                .unwrap_or(Value::Null)
        };
        let send = telemetry(&mut planes[sender].0, "net-send");
        let receive = telemetry(&mut planes[receiver].0, "net-receive");
        // The window names the addresses each computer must use.
        assert_eq!(receive["thisAddress"], "127.0.0.1", "{receive}");
        assert_eq!(send["localAddress"], "127.0.0.1", "{send}");
        assert!(send.get("lastErrorCode").is_none(), "{send}");
        // The network log summarizes both sides from the same counters.
        let summaries = planes
            .iter()
            .flat_map(|(plane, id)| {
                plane.write_network_summaries(
                    id,
                    &mut network_log::Sampler::default(),
                    Instant::now(),
                )
            })
            .collect::<Vec<_>>();
        let summary = |role: &str| {
            summaries
                .iter()
                .find(|record| record["role"] == role)
                .cloned()
                .unwrap_or(Value::Null)
        };
        let (send_summary, receive_summary) = (summary("send"), summary("receive"));
        assert!(
            send_summary["sentPackets"].as_u64().unwrap_or(0) > 100,
            "{send_summary}"
        );
        assert_eq!(send_summary["sendErrors"], 0, "{send_summary}");
        assert!(
            send_summary["localAddress"]
                .as_str()
                .is_some_and(|address| address.contains(':')),
            "{send_summary}"
        );
        assert!(
            receive_summary["receivedPackets"].as_u64().unwrap_or(0) > 100,
            "{receive_summary}"
        );
        assert_eq!(receive_summary["rejectedDatagrams"], 0, "{receive_summary}");
        assert!(
            receive_summary["hint"].is_null(),
            "a healthy stream has no hint: {receive_summary}"
        );
        let (plane, id) = &mut planes[receiver];
        let stopped = call(
            plane,
            "recorders.stopRecording",
            json!({ "sessionId": id, "nodeId": "rec", "idempotencyKey": "net-rec-stop" }),
        );
        for (plane, id) in planes.iter_mut() {
            call(
                plane,
                "session.stop",
                json!({ "sessionId": id, "idempotencyKey": "net-stop" }),
            );
        }
        let layout = if two_computers {
            "two backends"
        } else {
            "one session"
        };
        eprintln!("{layout}: send {send}\nreceive {receive}\nstopped {stopped}");
        assert_eq!(stopped["state"], "completed", "{layout}: {stopped}");
        assert!(
            send["sentPackets"].as_u64().unwrap_or(0) > 1_000,
            "{layout}: sender telemetry: {send}"
        );
        assert_eq!(send["droppedPackets"], 0, "{layout}: {send}");
        assert!(
            receive["receivedPackets"].as_u64().unwrap_or(0) > 1_000,
            "{layout}: receiver telemetry: {receive}"
        );
        assert_eq!(receive["rejectedDatagrams"], 0, "{layout}: {receive}");
        assert_eq!(receive["lostPackets"], 0, "{layout}: {receive}");
        // The recorded tone: ~3 s, continuous (no gap, no repeat, no step).
        assert_playable_wav(&path);
        let bytes = std::fs::read(&path).unwrap();
        let data = bytes
            .windows(4)
            .position(|window| window == b"data")
            .unwrap()
            + 8;
        let left = bytes[data..]
            .chunks_exact(8)
            .map(|frame| f32::from_le_bytes(frame[..4].try_into().unwrap()))
            .collect::<Vec<_>>();
        assert!(
            left.len() > 48_000 * 2,
            "{layout}: recorded {} frames",
            left.len()
        );
        let peak = left
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        assert!(
            (peak - 10_f32.powf(-12.0 / 20.0)).abs() < 0.02,
            "{layout}: level preserved over the network: {peak}"
        );
        let start = left
            .iter()
            .position(|sample| sample.abs() > peak / 2.0)
            .unwrap();
        let coefficient = (2.0 * (2.0 * std::f64::consts::PI * 997.0 / 48_000.0).cos()) as f32;
        let steps = left[start..]
            .windows(3)
            .filter(|window| {
                (window[2] - (coefficient * window[1] - window[0])).abs() > peak * 0.05
            })
            .count();
        assert_eq!(steps, 0, "{layout}: the received tone is continuous");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Two computers set up with mistakes, corrected while playing: the
/// receiver expects the wrong sending computer and the sender uses the
/// wrong port. Telemetry names the real sender, and fixing either
/// setting (as the UI's auto-save or a StreamDeck `nodes.set` does)
/// takes effect at once, without Stop/Play.
#[cfg(windows)]
#[test]
fn network_settings_corrected_while_playing_take_effect_without_restart() {
    let send_port = std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let wrong_port = std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let node = |id: &str, kind, ports, parameters: Value| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(parameters).unwrap(),
        ports,
    };
    let edge = |from: &str, to: &str| audiorouter_domain::Edge {
        id: EntityId::new(format!("{from}-{to}")),
        source_node: EntityId::new(from),
        source_port: "out".into(),
        destination_node: EntityId::new(to),
        destination_port: "in".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    };
    let session = |id: &str, nodes, edges| Session {
        id: EntityId::new(id),
        name: id.into(),
        schema_version: 1,
        revision: 0,
        nodes,
        edges,
    };
    let sending = session(
        "sending-pc",
        vec![
            node(
                "tone",
                NodeKind::TestSignal,
                vec![port("out", PortDirection::Output)],
                json!({ "frequencyHz": 997.0, "levelDb": -12.0, "durationMs": 600000.0 }),
            ),
            // Mistake 1: the receiving computer listens on another port.
            node(
                "net-send",
                NodeKind::NetworkSend,
                vec![port("in", PortDirection::Input)],
                json!({ "host": "127.0.0.1", "port": wrong_port }),
            ),
        ],
        vec![edge("tone", "net-send")],
    );
    let receiving = session(
        "receiving-pc",
        vec![
            // Mistake 2: the wrong sending computer's address.
            node(
                "net-receive",
                NodeKind::NetworkReceive,
                vec![port("out", PortDirection::Output)],
                json!({ "sender": "127.0.0.2", "port": send_port, "bufferMs": 40.0 }),
            ),
            node(
                "rec",
                NodeKind::Recorder,
                vec![port("in", PortDirection::Input)],
                json!({ "format": "wavFloat32" }),
            ),
        ],
        vec![edge("net-receive", "rec")],
    );
    let mut planes = [sending, receiving].map(|session| {
        let id = session.id.clone();
        let mut plane = ControlPlane::default();
        plane.insert_session(session).unwrap();
        (plane, id)
    });
    let call = |plane: &mut ControlPlane, method: &str, params: Value| {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        });
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
        response.result.unwrap()
    };
    for (plane, id) in planes.iter_mut() {
        call(plane, "nativePaths.prepare", json!({ "sessionId": id }));
        call(
            plane,
            "session.start",
            json!({ "sessionId": id, "idempotencyKey": "start" }),
        );
        call(
            plane,
            "sessions.active.set",
            json!({ "sessionId": id, "idempotencyKey": "active" }),
        );
    }
    let (plane, id) = &mut planes[0];
    call(
        plane,
        "audioSources.transport",
        json!({ "sessionId": id, "nodeId": "tone", "action": "play" }),
    );
    let service = |planes: &mut [(ControlPlane, EntityId); 2], seconds: f64| {
        let until = Instant::now() + Duration::from_secs_f64(seconds);
        while Instant::now() < until {
            for (plane, _) in planes.iter_mut() {
                plane.service_running_native_audio(Instant::now());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    };
    let receive_telemetry = |planes: &mut [(ControlPlane, EntityId); 2]| {
        let diagnostics = call(&mut planes[1].0, "system.diagnostics", json!({}));
        diagnostics["nodeTelemetry"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["nodeId"] == "net-receive")
            .unwrap()["network"]
            .clone()
    };
    service(&mut planes, 0.5);
    assert_eq!(
        receive_telemetry(&mut planes)["receivedPackets"],
        0,
        "nothing reaches the wrong port"
    );
    // Fix the sender's port while playing.
    let (plane, _) = &mut planes[0];
    let fixed = call(
        plane,
        "nodes.set",
        json!({ "node": "net-send", "parameters": { "port": send_port }, "idempotencyKey": "fix-port" }),
    );
    assert_ne!(
        fixed["activation"]["native"]["state"], "restartRequired",
        "{fixed}"
    );
    service(&mut planes, 0.5);
    let waiting = receive_telemetry(&mut planes);
    assert_eq!(
        waiting["receivedPackets"], 0,
        "still the wrong sender address: {waiting}"
    );
    assert!(
        waiting["rejectedDatagrams"].as_u64().unwrap_or(0) > 50,
        "{waiting}"
    );
    assert_eq!(
        waiting["rejectedFrom"], "127.0.0.1",
        "the real sender is named: {waiting}"
    );
    // Use the named address while playing (the UI's one-click fix).
    let (plane, _) = &mut planes[1];
    let fixed = call(
        plane,
        "nodes.set",
        json!({ "node": "net-receive", "parameters": { "sender": "127.0.0.1" }, "idempotencyKey": "fix-sender" }),
    );
    assert_ne!(
        fixed["activation"]["native"]["state"], "restartRequired",
        "{fixed}"
    );
    service(&mut planes, 1.0);
    let receiving = receive_telemetry(&mut planes);
    eprintln!("after both fixes: {receiving}");
    assert!(
        receiving["receivedPackets"].as_u64().unwrap_or(0) > 200,
        "audio flows after the live fix: {receiving}"
    );
    assert!(
        receiving.get("rejectedFrom").is_none(),
        "the old hint is cleared: {receiving}"
    );
    // A port change on the receiver opens a new socket, also live.
    let new_port = std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let (plane, _) = &mut planes[1];
    call(
        plane,
        "nodes.set",
        json!({ "node": "net-receive", "parameters": { "port": new_port }, "idempotencyKey": "move-port" }),
    );
    let (plane, _) = &mut planes[0];
    call(
        plane,
        "nodes.set",
        json!({ "node": "net-send", "parameters": { "port": new_port }, "idempotencyKey": "follow-port" }),
    );
    service(&mut planes, 1.0);
    let moved = receive_telemetry(&mut planes);
    assert!(
        moved["receivedPackets"].as_u64().unwrap_or(0) > 200,
        "audio follows the new port: {moved}"
    );
    // P2-5: pair the receiver while playing. The unpaired sender is now
    // refused (counted, never played) until it gets the same key.
    const PAIRING_KEY: &str = "K7QW2X9MPAIRSTUDIO4HJ8NV";
    let (plane, _) = &mut planes[1];
    call(
        plane,
        "nodes.set",
        json!({ "node": "net-receive", "parameters": { "pairingKey": PAIRING_KEY }, "idempotencyKey": "pair-receive" }),
    );
    service(&mut planes, 0.3);
    let before = receive_telemetry(&mut planes)["receivedPackets"]
        .as_u64()
        .unwrap_or(0);
    service(&mut planes, 0.5);
    let refused = receive_telemetry(&mut planes);
    assert_eq!(refused["paired"], true, "{refused}");
    assert_eq!(
        refused["receivedPackets"].as_u64().unwrap_or(0),
        before,
        "nothing unpaired is played: {refused}"
    );
    assert!(
        refused["authFailures"].as_u64().unwrap_or(0) > 100,
        "{refused}"
    );
    assert_eq!(refused["authProblem"], "senderNotPaired", "{refused}");
    let (plane, _) = &mut planes[0];
    call(
        plane,
        "nodes.set",
        json!({ "node": "net-send", "parameters": { "pairingKey": PAIRING_KEY }, "idempotencyKey": "pair-send" }),
    );
    service(&mut planes, 1.0);
    let paired = receive_telemetry(&mut planes);
    assert!(
        paired["receivedPackets"].as_u64().unwrap_or(0) > before + 200,
        "paired audio flows: {paired}"
    );
    assert_eq!(paired["replayedPackets"], 0, "{paired}");
    for (plane, _) in planes.iter_mut() {
        let diagnostics = call(plane, "system.diagnostics", json!({}));
        assert!(
            !diagnostics.to_string().contains(PAIRING_KEY),
            "diagnostics never carry the key"
        );
    }
    for (plane, id) in planes.iter_mut() {
        call(
            plane,
            "session.stop",
            json!({ "sessionId": id, "idempotencyKey": "stop" }),
        );
    }
}
