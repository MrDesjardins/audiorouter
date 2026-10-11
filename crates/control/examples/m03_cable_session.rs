//! VM tool: run one product cable-to-cable route (WP-09) through the real
//! control plane for a fixed time, the way the desktop backend runs it.
//!
//! Creates an in-memory session `Cable <render> → Cable <capture>`, calls
//! `nativePaths.prepare` and `session.start` through the request dispatcher,
//! then runs the backend's audio service pass every millisecond (lease
//! heartbeats, failure handling) and prints the `cableRoute` diagnostics
//! once a second. Requires the AudioRouter driver; never touches the host
//! database or any other device.
//!
//! Usage: m03_cable_session [--render-bus cable-a] [--capture-bus cable-b] [--seconds N] [--check]
//!
//! `--check` runs every request except `session.start` (prepare,
//! diagnostics, stop), so the request shapes are verified without a driver.
use audiorouter_control::ControlPlane;
use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_protocol::JsonRpcRequest;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

fn cable_bus(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    if bytes.len() == 7 && text.starts_with("cable-") && (b'a'..=b'h').contains(&bytes[6]) {
        Ok(text.to_owned())
    } else {
        Err(format!("cable IDs are cable-a … cable-h, not {text}"))
    }
}

fn cable_node(id: &str, kind: NodeKind, bus: &str, direction: PortDirection) -> Node {
    Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: json!({ "busId": bus })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        ports: vec![Port {
            name: if direction == PortDirection::Output {
                "out"
            } else {
                "in"
            }
            .into(),
            direction,
            channels: 2,
        }],
    }
}

fn call(plane: &mut ControlPlane, method: &str, params: Value) -> Result<Value, String> {
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    });
    match (response.result, response.error) {
        (Some(result), None) => Ok(result),
        (_, Some(error)) => Err(format!("{method}: {}", error.message)),
        _ => Err(format!("{method}: empty response")),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut render_bus = "cable-a".to_owned();
    let mut capture_bus = "cable-b".to_owned();
    let mut seconds = 20_u64;
    let mut check = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--render-bus" => render_bus = cable_bus(&value()?)?,
            "--capture-bus" => capture_bus = cable_bus(&value()?)?,
            "--seconds" => {
                seconds = value()?
                    .parse()
                    .ok()
                    .filter(|seconds| (1..=3_600).contains(seconds))
                    .ok_or("--seconds must be 1..3600")?
            }
            "--check" => check = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if render_bus == capture_bus {
        return Err("choose two different cables".into());
    }
    let mut plane = ControlPlane::new("m03-cable-session");
    let session = Session {
        id: EntityId::new("vm-cable-route"),
        name: "VM cable route".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            cable_node(
                "cable-in",
                NodeKind::VirtualRenderSource,
                &render_bus,
                PortDirection::Output,
            ),
            cable_node(
                "cable-out",
                NodeKind::VirtualCaptureSink,
                &capture_bus,
                PortDirection::Input,
            ),
        ],
        edges: vec![Edge {
            id: EntityId::new("cable-in-out"),
            source_node: EntityId::new("cable-in"),
            source_port: "out".into(),
            destination_node: EntityId::new("cable-out"),
            destination_port: "in".into(),
            matrix: vec![1.0, 0.0, 0.0, 1.0],
            enabled: true,
        }],
    };
    plane
        .create_session(session)
        .map_err(|error| format!("session: {error:?}"))?;
    let prepared = call(
        &mut plane,
        "nativePaths.prepare",
        json!({ "sessionId": "vm-cable-route" }),
    )?;
    println!("prepared: {prepared}");
    if check {
        let diagnostics = call(&mut plane, "system.diagnostics", json!({}))?;
        println!(
            "check diagnostics nativeAdapterKind={}",
            diagnostics["nativeAdapterKind"]
        );
        let stopped = call(
            &mut plane,
            "session.stop",
            json!({ "sessionId": "vm-cable-route", "idempotencyKey": "vm-cable-route-stop" }),
        )?;
        println!("check stopped: {stopped}");
        return Ok(());
    }
    let started = call(
        &mut plane,
        "session.start",
        json!({ "sessionId": "vm-cable-route", "idempotencyKey": "vm-cable-route-start" }),
    )?;
    println!("started: {started}");
    println!("cable route running: {render_bus} -> {capture_bus} for {seconds} s");
    let start = Instant::now();
    let mut progress = start;
    let mut max_pass_gap = Duration::ZERO;
    let mut last = start;
    let mut failed = None;
    while start.elapsed() < Duration::from_secs(seconds) {
        let now = Instant::now();
        max_pass_gap = max_pass_gap.max(now - last);
        last = now;
        plane.service_running_native_audio(now);
        if progress.elapsed() >= Duration::from_secs(1) {
            progress = Instant::now();
            let diagnostics = call(&mut plane, "system.diagnostics", json!({}))?;
            println!(
                "progress {} ms: cableRoute={} max_service_gap_us={}",
                start.elapsed().as_millis(),
                diagnostics["cableRoute"],
                max_pass_gap.as_micros()
            );
            max_pass_gap = Duration::ZERO;
            if diagnostics["cableRoute"]["state"] != "running" {
                failed = Some(diagnostics["cableRoute"].to_string());
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let diagnostics = call(&mut plane, "system.diagnostics", json!({}))?;
    println!("final cableRoute={}", diagnostics["cableRoute"]);
    let stopped = call(
        &mut plane,
        "session.stop",
        json!({ "sessionId": "vm-cable-route", "idempotencyKey": "vm-cable-route-stop" }),
    )?;
    println!("stopped: {stopped}");
    let diagnostics = call(&mut plane, "system.diagnostics", json!({}))?;
    println!("after stop cableRoute={}", diagnostics["cableRoute"]);
    match failed {
        Some(route) => Err(format!("cable route stopped early: {route}")),
        None => Ok(()),
    }
}
