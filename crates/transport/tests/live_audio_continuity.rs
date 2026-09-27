//! Audio continuity qualification of the production backend loop.
//!
//! The live test routes a quiet sine through a real AudioRouter route and
//! records it again, then finds every sample-level discontinuity (the
//! audible "crackle" of a dropped or repeated block). It is ignored by
//! default and additionally requires `AUDIOROUTER_LIVE_CONTINUITY=1`:
//!
//!   harness render  -> CABLE Input          (VB-Audio Virtual Cable)
//!   AudioRouter     :  CABLE Output -> processors -> CABLE-B Input
//!   harness capture <- CABLE-B Output        (result)
//!   harness capture <- CABLE Output          (reference, isolates harness faults)
//!
//! Only virtual endpoints are used, so nothing is audible unless Windows
//! "Listen to this device" is enabled on either cable output. The backend is
//! served over a private named pipe exactly as the desktop shell serves it;
//! the client never sends pump requests at audio rate, it only generates a
//! UI-like load of diagnostics requests.

use serde_json::{json, Value};

const SAMPLE_RATE: f64 = 48_000.0;
const TONE_HZ: f64 = 997.0;

/// Live tone frequency; `AUDIOROUTER_CONTINUITY_TONE_HZ` selects a low tone
/// (for example 47 Hz) whose long period makes block-sized jumps unambiguous.
fn live_tone_hz() -> f64 {
    std::env::var("AUDIOROUTER_CONTINUITY_TONE_HZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(TONE_HZ)
}
const AMPLITUDE: f32 = 0.05;

/// One detected discontinuity: sample index and relative residual.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Glitch {
    index: usize,
    residual: f32,
}

/// Find discontinuities in a recorded pure tone. A sampled sine satisfies
/// `x[n] = 2cos(w)x[n-1] - x[n-2]` independent of phase and amplitude, so
/// any residual larger than a small fraction of the amplitude is a dropped,
/// repeated or zeroed block. Detections within 64 samples merge into one.
fn find_glitches(samples: &[f32], tone_hz: f64, sample_rate: f64, amplitude: f32) -> Vec<Glitch> {
    let coefficient = (2.0 * (2.0 * std::f64::consts::PI * tone_hz / sample_rate).cos()) as f32;
    let threshold = amplitude * 0.05;
    let mut glitches: Vec<Glitch> = Vec::new();
    for index in 2..samples.len() {
        let predicted = coefficient * samples[index - 1] - samples[index - 2];
        let residual = (samples[index] - predicted).abs();
        if residual > threshold {
            match glitches.last_mut() {
                Some(last) if index - last.index < 64 => {
                    last.residual = last.residual.max(residual / amplitude);
                }
                _ => glitches.push(Glitch {
                    index,
                    residual: residual / amplitude,
                }),
            }
        }
    }
    glitches
}

/// The analysed recording, from where the tone is present, minus a few
/// samples at each edge.
fn steady_region(samples: &[f32], amplitude: f32) -> Option<&[f32]> {
    let first = samples.iter().position(|sample| sample.abs() > amplitude / 4.0)?;
    let start = first + 64;
    let end = samples.len().checked_sub(64)?;
    (start + (SAMPLE_RATE as usize) < end).then(|| &samples[start..end])
}

fn tone(index: u64) -> f32 {
    (AMPLITUDE as f64 * (2.0 * std::f64::consts::PI * live_tone_hz() * index as f64 / SAMPLE_RATE).sin())
        as f32
}

#[test]
fn glitch_detector_is_silent_on_a_clean_tone_and_finds_dropped_blocks() {
    let clean: Vec<f32> = (0..48_000).map(tone).collect();
    assert!(find_glitches(&clean, TONE_HZ, SAMPLE_RATE, AMPLITUDE).is_empty());

    // Drop one 480-frame block (a missed 10 ms quantum) and zero another.
    let mut damaged: Vec<f32> = (0..10_000).map(tone).chain((10_480..30_000).map(tone)).collect();
    for sample in &mut damaged[20_000..20_480] {
        *sample = 0.0;
    }
    let glitches = find_glitches(&damaged, TONE_HZ, SAMPLE_RATE, AMPLITUDE);
    let positions: Vec<usize> = glitches.iter().map(|glitch| glitch.index).collect();
    assert_eq!(positions.len(), 3, "drop, zero start and zero end: {glitches:?}");
    assert!(positions[0].abs_diff(10_000) < 4);
    assert!(positions[1].abs_diff(20_000) < 4);
    assert!(positions[2].abs_diff(20_480) < 4);
}

#[cfg(windows)]
mod live {
    use super::*;
    use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
    use audiorouter_windows_audio::{
        enumerate_active_endpoints, AudioServiceThreadGuard, EndpointDirection, SharedCapture,
        SharedRender,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// Exact endpoint ID from `id_env`, defaulting to this development
    /// machine's cable; it must be active in the expected direction.
    fn endpoint(id_env: &str, default: &str, direction: EndpointDirection) -> String {
        let id = std::env::var(id_env).unwrap_or_else(|_| default.to_owned());
        enumerate_active_endpoints()
            .unwrap()
            .into_iter()
            .find(|endpoint| endpoint.direction == direction && endpoint.id == id)
            .unwrap_or_else(|| panic!("endpoint {id:?} ({direction:?}) is not active"))
            .id
    }

    fn processor(kind: &str) -> NodeKind {
        serde_json::from_value(json!(kind)).unwrap_or_else(|_| panic!("unknown node kind {kind}"))
    }

    fn port_named(node: &Node, direction: PortDirection) -> String {
        node.ports
            .iter()
            .find(|port| port.direction == direction)
            .map(|port| port.name.clone())
            .expect("chain node has the needed port")
    }

    /// Source -> built-in tools -> `extra` nodes (for example saved plugin
    /// nodes) -> destination, all stereo, linked by identity edges.
    fn route(source: &str, destination: &str, chain: &[NodeKind], extra: Vec<Node>) -> Session {
        let port = |name: &str, direction| Port {
            name: name.into(),
            direction,
            channels: 2,
        };
        let node = |id: String, kind, ports, parameters: serde_json::Map<String, Value>| Node {
            id: EntityId::new(id.clone()),
            kind,
            type_version: 1,
            name: id,
            enabled: true,
            bypass: false,
            parameters,
            ports,
        };
        let endpoint_parameters = |id: &str| {
            let mut parameters = serde_json::Map::new();
            parameters.insert("endpointId".into(), json!(id));
            parameters
        };
        let mut nodes = vec![node(
            "source".into(),
            NodeKind::PhysicalInput,
            vec![port("out", PortDirection::Output)],
            endpoint_parameters(source),
        )];
        for (index, kind) in chain.iter().enumerate() {
            nodes.push(node(
                format!("tool-{index}"),
                *kind,
                vec![port("in", PortDirection::Input), port("out", PortDirection::Output)],
                serde_json::Map::new(),
            ));
        }
        for mut node in extra {
            for port in &mut node.ports {
                port.channels = 2;
            }
            nodes.push(node);
        }
        nodes.push(node(
            "destination".into(),
            NodeKind::PhysicalOutput,
            vec![port("in", PortDirection::Input)],
            endpoint_parameters(destination),
        ));
        let edges = nodes
            .windows(2)
            .map(|pair| Edge {
                id: EntityId::new(format!("{}-{}", pair[0].id.as_str(), pair[1].id.as_str())),
                source_node: pair[0].id.clone(),
                source_port: port_named(&pair[0], PortDirection::Output),
                destination_node: pair[1].id.clone(),
                destination_port: port_named(&pair[1], PortDirection::Input),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            })
            .collect();
        Session {
            id: EntityId::new("continuity"),
            name: "Continuity qualification".into(),
            schema_version: 1,
            revision: 0,
            nodes,
            edges,
        }
    }

    fn rpc(pipe: &str, method: &str, params: Value) -> Value {
        let request = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let frame = audiorouter_protocol::encode_frame(&request).unwrap();
        let response = audiorouter_transport::round_trip(pipe, &frame)
            .unwrap_or_else(|error| panic!("{method}: {error:?}"));
        let response: Value = audiorouter_protocol::decode_frame(&response).unwrap();
        assert!(response["error"].is_null(), "{method}: {}", response["error"]);
        response["result"].clone()
    }

    /// Render the reference tone continuously into `endpoint_id`.
    fn generator(endpoint_id: String, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let (_scheduling, _) = AudioServiceThreadGuard::enter();
            let mut render = SharedRender::open_with_headroom(&endpoint_id, 500_000).unwrap();
            eprintln!("tone render buffer: {} frames", render.buffer_frames());
            let mut next = 0_u64;
            let mut chunk = Vec::with_capacity(4096 * 8);
            let mut fill = |render: &SharedRender, next: &mut u64| {
                chunk.clear();
                for index in *next..*next + 4096 {
                    let bytes = tone(index).to_le_bytes();
                    chunk.extend_from_slice(&bytes);
                    chunk.extend_from_slice(&bytes);
                }
                *next += u64::from(render.submit_bytes(&chunk, 8).unwrap());
            };
            fill(&render, &mut next);
            render.start().unwrap();
            while !stop.load(Ordering::Acquire) {
                render.wait_for_data(20).unwrap();
                fill(&render, &mut next);
            }
        })
    }

    struct Recording {
        left: Vec<f32>,
        discontinuity_flags: usize,
    }

    /// Record the left channel of `endpoint_id` until stopped.
    /// Samples are kept only while `measuring` is set, so route startup and
    /// teardown stay outside the analysed window.
    fn recorder(
        endpoint_id: String,
        stop: Arc<AtomicBool>,
        measuring: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<Recording> {
        std::thread::spawn(move || {
            let (_scheduling, _) = AudioServiceThreadGuard::enter();
            let mut capture = SharedCapture::open(&endpoint_id, 0).unwrap();
            let mut buffer = vec![0_u8; 48_000 * 8];
            let mut recording = Recording {
                left: Vec::with_capacity(48_000 * 120),
                discontinuity_flags: 0,
            };
            capture.start().unwrap();
            while !stop.load(Ordering::Acquire) {
                capture.wait_for_data(20).unwrap();
                while let Some((packet, bytes)) = capture.next_packet_into(&mut buffer, 8).unwrap() {
                    if !measuring.load(Ordering::Acquire) {
                        continue;
                    }
                    // AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY
                    if packet.flags & 0x1 != 0 {
                        recording.discontinuity_flags += 1;
                    }
                    for frame in buffer[..bytes].chunks_exact(8) {
                        recording
                            .left
                            .push(f32::from_le_bytes(frame[..4].try_into().unwrap()));
                    }
                }
            }
            recording
        })
    }

    fn report(label: &str, recording: &Recording) -> usize {
        let Some(steady) = steady_region(&recording.left, AMPLITUDE / 4.0) else {
            panic!("{label}: no steady tone recorded ({} samples)", recording.left.len());
        };
        let peak = steady.iter().fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        let glitches = find_glitches(steady, live_tone_hz(), SAMPLE_RATE, peak);
        // Underruns render exact digital silence; a run of zeros longer than
        // two samples cannot occur inside the sine.
        let mut zero_runs = 0;
        let mut zero_samples = 0;
        let mut run = 0;
        for sample in steady {
            if *sample == 0.0 {
                run += 1;
            } else {
                if run > 2 {
                    zero_runs += 1;
                    zero_samples += run;
                }
                run = 0;
            }
        }
        eprintln!(
            "{label}: {:.1} s analysed, peak {:.4}, {} glitches, {} silent runs ({} samples), {} WASAPI discontinuity flags",
            steady.len() as f64 / SAMPLE_RATE,
            peak,
            glitches.len(),
            zero_runs,
            zero_samples,
            recording.discontinuity_flags
        );
        if let Ok(path) = std::env::var("AUDIOROUTER_CONTINUITY_DUMP_DIR") {
            let file = std::path::Path::new(&path).join(format!(
                "{}.f32",
                label.split_whitespace().next().unwrap_or("recording")
            ));
            let bytes: Vec<u8> = steady.iter().flat_map(|sample| sample.to_le_bytes()).collect();
            std::fs::write(file, bytes).unwrap();
        }
        for glitch in glitches.iter().take(20) {
            eprintln!(
                "  at {:8.3} s residual {:.2}x amplitude",
                glitch.index as f64 / SAMPLE_RATE,
                glitch.residual
            );
        }
        glitches.len()
    }

    #[test]
    #[ignore = "live Windows audio: renders a quiet tone into VB-Cable and CABLE-B"]
    fn live_backend_service_keeps_a_routed_tone_continuous() {
        if std::env::var("AUDIOROUTER_LIVE_CONTINUITY").as_deref() != Ok("1") {
            return;
        }
        let seconds: u64 = std::env::var("AUDIOROUTER_CONTINUITY_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(20);
        let chain: Vec<NodeKind> = std::env::var("AUDIOROUTER_CONTINUITY_CHAIN")
            .unwrap_or_else(|_| "gain".into())
            .split(',')
            .filter(|kind| !kind.is_empty())
            .map(processor)
            .collect();
        let tone_render = endpoint(
            "AUDIOROUTER_CONTINUITY_TONE_RENDER_ID",
            "{0.0.0.00000000}.{71f96f14-94c1-4189-b9b9-df8c960db8f2}",
            EndpointDirection::Render,
        );
        let route_capture = endpoint(
            "AUDIOROUTER_CONTINUITY_ROUTE_CAPTURE_ID",
            "{0.0.1.00000000}.{06268191-5f8c-42ed-827e-d3c7a19637ed}",
            EndpointDirection::Capture,
        );
        let route_render = endpoint(
            "AUDIOROUTER_CONTINUITY_ROUTE_RENDER_ID",
            "{0.0.0.00000000}.{7ebf5c2d-9a8e-4056-8dc9-277cb59ec3e8}",
            EndpointDirection::Render,
        );
        let result_capture = endpoint(
            "AUDIOROUTER_CONTINUITY_RESULT_CAPTURE_ID",
            "{0.0.1.00000000}.{65195e1b-3ee4-4e81-b1f6-a95d243fa1ce}",
            EndpointDirection::Capture,
        );

        let pipe = format!(r"\\.\pipe\audiorouter-continuity-{}", std::process::id());
        // Optional real plugins: nodes (with saved state) copied from a
        // session in a COPY of a user database. Needs a built worker in
        // AUDIOROUTER_PLUGIN_WORKER_PATH.
        let plugin_database = std::env::var_os("AUDIOROUTER_CONTINUITY_PLUGIN_DB");
        let plugin_ids: Vec<String> = std::env::var("AUDIOROUTER_CONTINUITY_PLUGINS")
            .unwrap_or_else(|_| "reafir,reaeq,reacomp,reagate".into())
            .split(',')
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .collect();
        let session_id = "continuity".to_owned();
        let server_pipe = pipe.clone();
        let (route_capture_id, route_render_id) = (route_capture.clone(), route_render.clone());
        let server_chain = chain.clone();
        std::thread::spawn(move || {
            let mut plane = match plugin_database {
                Some(path) => audiorouter_control::ControlPlane::with_storage(
                    "continuity",
                    audiorouter_storage::Storage::open(std::path::Path::new(&path)).unwrap(),
                ),
                None => audiorouter_control::ControlPlane::default(),
            };
            let extra = if plane.get_session(&EntityId::new("patrick-main-session")).is_ok()
                && std::env::var_os("AUDIOROUTER_CONTINUITY_PLUGIN_DB").is_some()
            {
                let saved = plane
                    .get_session(&EntityId::new(
                        std::env::var("AUDIOROUTER_CONTINUITY_PLUGIN_SESSION")
                            .unwrap_or_else(|_| "patrick-main-session".into()),
                    ))
                    .unwrap();
                plugin_ids
                    .iter()
                    .map(|id| {
                        saved
                            .nodes
                            .iter()
                            .find(|node| node.id.as_str() == id)
                            .unwrap_or_else(|| panic!("saved session has no node {id}"))
                            .clone()
                    })
                    .map(|mut node: Node| {
                        if std::env::var_os("AUDIOROUTER_CONTINUITY_CLEAR_BYPASS").is_some() {
                            node.bypass = false;
                        }
                        node
                    })
                    .collect()
            } else {
                Vec::new()
            };
            for node in &extra {
                eprintln!("plugin node: {}", serde_json::to_string(node).unwrap());
            }
            plane
                .insert_session(route(&route_capture_id, &route_render_id, &server_chain, extra))
                .unwrap();
            let grant = audiorouter_control::ClientGrant::with_scopes([
                audiorouter_domain::PermissionScope::Read,
                audiorouter_domain::PermissionScope::GraphWrite,
                audiorouter_domain::PermissionScope::SessionControl,
                audiorouter_domain::PermissionScope::DeviceAdministration,
            ]);
            let _ = audiorouter_transport::serve_control_connections_forever_with_grant(
                &server_pipe,
                plane,
                grant,
            );
        });
        std::thread::sleep(Duration::from_millis(300));

        let stop = Arc::new(AtomicBool::new(false));
        let tone = generator(tone_render, Arc::clone(&stop));
        let measuring = Arc::new(AtomicBool::new(false));
        let reference = recorder(route_capture.clone(), Arc::clone(&stop), Arc::clone(&measuring));
        let result = recorder(result_capture, Arc::clone(&stop), Arc::clone(&measuring));
        std::thread::sleep(Duration::from_millis(300));

        // `paths` (default) is the multi-input worker used by multi-device
        // sessions; `endpoint` is the single capture/render worker the UI
        // uses for a simple one-input, one-output route.
        let endpoint_mode = std::env::var("AUDIOROUTER_CONTINUITY_MODE").as_deref() == Ok("endpoint");
        let prepared = if endpoint_mode {
            rpc(
                &pipe,
                "nativeEndpoints.prepare",
                json!({ "sessionId": session_id, "captureEndpointId": route_capture, "renderEndpointId": route_render }),
            )
        } else {
            rpc(&pipe, "nativePaths.prepare", json!({ "sessionId": session_id }))
        };
        eprintln!("prepared: {prepared}");
        let started = rpc(
            &pipe,
            "session.start",
            json!({ "sessionId": session_id, "idempotencyKey": format!("continuity-start-{}", std::process::id()) }),
        );
        eprintln!("started: {started}");
        let generation = started["generation"].as_u64().unwrap();
        std::thread::sleep(Duration::from_secs(1));
        measuring.store(true, Ordering::Release);
        // UI-like control load: 20 Hz diagnostics and a 10 Hz counter pump.
        let until = Instant::now() + Duration::from_secs(seconds);
        let mut last_service = Value::Null;
        let mut last_underruns = Value::Null;
        let mut tick = 0_u64;
        while Instant::now() < until {
            rpc(&pipe, "system.diagnostics", Value::Null);
            if tick % 2 == 0 {
                let pump = rpc(
                    &pipe,
                    if endpoint_mode { "nativeEndpoints.pump" } else { "nativeMultiInputs.pump" },
                    json!({ "sessionId": session_id, "generation": generation, "maxPackets": 64 }),
                );
                last_service = pump["audioService"].clone();
                last_underruns = pump["outputUnderruns"].clone();
            }
            tick += 1;
            std::thread::sleep(Duration::from_millis(50));
        }
        measuring.store(false, Ordering::Release);
        let diagnostics = rpc(&pipe, "system.diagnostics", Value::Null);
        rpc(
            &pipe,
            "session.stop",
            json!({ "sessionId": session_id, "idempotencyKey": format!("continuity-stop-{}", std::process::id()) }),
        );
        std::thread::sleep(Duration::from_millis(300));
        stop.store(true, Ordering::Release);
        tone.join().unwrap();
        let reference = reference.join().unwrap();
        let result = result.join().unwrap();

        eprintln!("chain: {chain:?}; audio service: {last_service}; output underruns: {last_underruns}");
        for item in diagnostics["nodeTelemetry"].as_array().into_iter().flatten() {
            eprintln!("  {} timing={} plugin={}", item["nodeId"], item["timing"], item["plugin"]);
        }
        let reference_glitches = report("reference (CABLE Output)", &reference);
        let result_glitches = report("routed result (CABLE-B Output)", &result);
        assert_eq!(last_service["active"], true, "backend must own audio service");
        assert_eq!(
            reference_glitches, 0,
            "the harness tone itself was discontinuous; the result is inconclusive"
        );
        assert_eq!(result_glitches, 0, "the routed tone has audible discontinuities");
    }
}
