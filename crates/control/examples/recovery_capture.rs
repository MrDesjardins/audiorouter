//! Attended, bounded exact-endpoint capture recovery qualification. No files
//! containing audio, playback endpoints, saved profile or driver operations.
use audiorouter_control::ControlPlane;
use audiorouter_protocol::JsonRpcRequest;
use serde_json::{json, Value};
use std::io::{self, BufRead};
use std::time::{Duration, Instant};

fn call(plane: &mut ControlPlane, method: &str, params: Value) -> Result<Value, String> {
    let reply = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(method)),
        method: method.into(),
        params: Some(params),
    });
    if let Some(error) = reply.error {
        return Err(format!("{method}: {error:?}"));
    }
    reply
        .result
        .ok_or_else(|| format!("{method}: missing result"))
}

fn main() -> Result<(), String> {
    let endpoint = std::env::args()
        .nth(1)
        .ok_or("exact PD200X microphone endpoint ID required")?;
    let seconds = std::env::args()
        .nth(2)
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or(300);
    if !(1..=300).contains(&seconds) {
        return Err("duration must be 1..300 seconds".into());
    }
    let mut plane = ControlPlane::default();
    let inventory = call(&mut plane, "devices.list", json!({}))?;
    let device = inventory
        .as_array()
        .ok_or("endpoint array required")?
        .iter()
        .find(|device| {
            device["id"] == endpoint
                && device["direction"] == "capture"
                && device["name"] == "Microphone (PD200X Podcast Microphone)"
        })
        .ok_or("the selected exact PD200X capture device is not connected")?;
    println!("DEVICE {}", device["name"]);
    let original = json!({"id":"attended-recovery", "name":"Disposable USB recovery",
        "schemaVersion":1, "revision":0,
        "nodes":[
            {"id":"input", "name":"PD200X microphone", "kind":"physicalInput", "typeVersion":1,
             "enabled":true,"bypass":false,"parameters":{"endpointId":endpoint},
             "ports":[{"name":"out","direction":"output","channels":2}]},
            {"id":"meter", "name":"Meter", "kind":"meter", "typeVersion":1,
             "enabled":true,"bypass":false,"parameters":{},
             "ports":[{"name":"in","direction":"input","channels":2},
                      {"name":"out","direction":"output","channels":2}]},
            {"id":"sink", "name":"Unarmed sink", "kind":"recorder", "typeVersion":1,
             "enabled":true,"bypass":false,"parameters":{},
             "ports":[{"name":"in","direction":"input","channels":2}]}],
        "edges":[{"id":"input-meter","sourceNode":"input","sourcePort":"out",
                  "destinationNode":"meter","destinationPort":"in","matrix":[1,0,0,1],"enabled":true},
                 {"id":"meter-sink","sourceNode":"meter","sourcePort":"out",
                  "destinationNode":"sink","destinationPort":"in","matrix":[1,0,0,1],"enabled":true}]});
    call(
        &mut plane,
        "sessions.create",
        json!({"session":original,"idempotencyKey":"create"}),
    )?;
    let canonical = call(
        &mut plane,
        "sessions.get",
        json!({"sessionId":"attended-recovery"}),
    )?;
    println!("CANONICAL {canonical}");
    call(
        &mut plane,
        "devices.setAccess",
        json!({"allowed":true,"idempotencyKey":"consent"}),
    )?;
    let (commands, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            if let Ok(line) = line {
                if commands.send(line).is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    });
    let outcome: Result<(), String> = (|| {
        let started = call(
            &mut plane,
            "sessions.play",
            json!({"sessionId":"attended-recovery","idempotencyKey":"play"}),
        )?;
        let prepared = call(
            &mut plane,
            "sessions.get",
            json!({"sessionId":"attended-recovery"}),
        )?;
        if prepared != canonical {
            return Err(format!("preparation changed saved graph: {prepared}"));
        }
        println!("READY {started}");
        let deadline = Instant::now() + Duration::from_secs(seconds);
        let mut cursor = 0;
        let mut lost = false;
        let mut recovered = false;
        while Instant::now() < deadline {
            if receiver.try_recv().is_ok_and(|line| line.trim() == "quit") {
                break;
            }
            plane.service_running_native_audio(Instant::now());
            let events = call(
                &mut plane,
                "events.subscribe",
                json!({"afterSequence":cursor,"limit":100}),
            )?;
            if let Some(events) = events["events"].as_array() {
                for event in events {
                    cursor = event["sequence"].as_u64().ok_or("event sequence missing")?;
                    lost |= event["category"] == "devices.bindingInvalidated";
                    recovered |= event["category"] == "runtime.recovered";
                    println!("EVENT {event}");
                }
            }
            if recovered {
                let status = call(&mut plane, "status.get", json!({}))?;
                if status["activeSessionCount"] != 1 {
                    return Err("recovered event did not leave the route running".into());
                }
                println!("RECOVERED_STATUS {status}");
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let saved = call(
            &mut plane,
            "sessions.get",
            json!({"sessionId":"attended-recovery"}),
        )?;
        if saved != canonical {
            return Err(format!(
                "saved graph changed during device recovery: {saved}"
            ));
        }
        println!("GRAPH_UNCHANGED");
        println!(
            "RESULT {}",
            json!({"deviceLossObserved":lost,"recoveryObserved":recovered,"qualified":lost && recovered})
        );
        Ok(())
    })();
    let stopped = call(
        &mut plane,
        "sessions.stop",
        json!({"sessionId":"attended-recovery","idempotencyKey":"stop"}),
    );
    println!("STOP {stopped:?}");
    outcome?;
    stopped?;
    Ok(())
}
