//! Test-only stdio adapter. No listener, native audio or user database access.
use audiorouter_control::ControlPlane;
use audiorouter_protocol::{JsonRpcRequest, MAX_FRAME_BYTES};
use audiorouter_storage::Storage;
use serde_json::json;
use std::io::{self, BufRead, Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args().nth(1).ok_or("test directory required")?;
    let directory = std::path::PathBuf::from(directory).canonicalize()?;
    // Every write belongs to an exclusively created disposable fixture.
    if !directory
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("audiorouter-e2e-"))
    {
        return Err("not an e2e fixture directory".into());
    }
    let storage =
        Storage::open(directory.join("fixture.sqlite")).map_err(|error| format!("{error:?}"))?;
    let mut plane = ControlPlane::try_with_storage("browser-e2e-real-backend", storage)
        .map_err(|error| format!("{error:?}"))?;
    plane
        .configure_recording_root(&directory)
        .map_err(|error| format!("{error:?}"))?;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    loop {
        let mut line = Vec::new();
        let length = (&mut input)
            .take(MAX_FRAME_BYTES as u64 + 1)
            .read_until(b'\n', &mut line)?;
        if length == 0 {
            break;
        }
        if length > MAX_FRAME_BYTES || line.last() != Some(&b'\n') {
            return Err("oversized/incomplete fixture request".into());
        }
        let request: JsonRpcRequest = serde_json::from_slice(&line)?;
        let method = request.method.as_str();
        if method == "fixture.recordSyntheticQuantum" {
            use audiorouter_engine::AudioTap;
            let params = request
                .params
                .as_ref()
                .ok_or("fixture parameters required")?;
            let id = audiorouter_domain::EntityId::new(
                params["sessionId"]
                    .as_str()
                    .ok_or("fixture session required")?,
            );
            let frame = params["frame"]
                .as_u64()
                .filter(|frame| *frame <= 1_000_000)
                .ok_or("bounded fixture frame required")?;
            let taps = plane
                .recorder_tap_set(&id)
                .map_err(|error| format!("{error:?}"))?;
            let mut block = audiorouter_engine::AudioBlock::new(2, 128)
                .map_err(|error| format!("{error:?}"))?;
            for channel in 0..2 {
                block
                    .channel_mut(channel)
                    .unwrap()
                    .fill(if channel == 0 { 0.1 } else { -0.1 });
            }
            taps.on_processed_block(frame, &block);
            serde_json::to_writer(
                &mut output,
                &json!({"jsonrpc":"2.0", "id": request.id, "result": {"frames":128,"synthetic":true}}),
            )?;
            output.write_all(b"\n")?;
            output.flush()?;
            continue;
        }
        if method == "fixture.recordNodeQuantum" {
            // Synthetic audio through a Recorder node's playback inlet, as the
            // native graph delivers it after Play.
            use audiorouter_engine::AudioTap;
            let params = request.params.as_ref().ok_or("fixture parameters required")?;
            let id = audiorouter_domain::EntityId::new(params["sessionId"].as_str().ok_or("fixture session required")?);
            let node = audiorouter_domain::EntityId::new(params["nodeId"].as_str().ok_or("fixture node required")?);
            let frame = params["frame"].as_u64().filter(|frame| *frame <= 10_000_000).ok_or("bounded fixture frame required")?;
            let taps = plane.recorder_tap_set_for_node(&id, &node).map_err(|error| format!("{error:?}"))?;
            let mut block = audiorouter_engine::AudioBlock::new(2, 128).map_err(|error| format!("{error:?}"))?;
            for channel in 0..2 {
                block.channel_mut(channel).unwrap().fill(if channel == 0 { 0.1 } else { -0.1 });
            }
            taps.on_processed_block(frame, &block);
            serde_json::to_writer(&mut output, &json!({"jsonrpc":"2.0", "id": request.id, "result": {"frames":128,"synthetic":true}}))?;
            output.write_all(b"\n")?;
            output.flush()?;
            continue;
        }
        let allowed = matches!(
            method,
            "system.describe"
                | "system.diagnostics"
                | "status.get"
                | "routes.inspect"
                | "events.subscribe"
                | "safety.setPrivacyMute"
                | "processors.list"
                | "processors.response"
                | "presets.list"
                | "presets.get"
                | "recorders.list"
                | "recordings.list"
                | "clients.list"
                | "startup.get"
        ) || method.starts_with("sessions.")
            || method.starts_with("graph.")
            || method.starts_with("audioMedia.")
            || matches!(method, "session.start" | "session.stop");
        let allowed = allowed
            || method.starts_with("recorders.")
            || matches!(
                method,
                "recordings.preview" | "recordings.inspectRecovery" | "recordings.setMetadata"
            );
        if allowed {
            serde_json::to_writer(&mut output, &plane.dispatch(request))?;
        } else {
            serde_json::to_writer(
                &mut output,
                &json!({"jsonrpc":"2.0", "id":request.id,
                "error":{"code":-32601,"message":"Method excluded by the offline E2E fixture"}}),
            )?;
        }
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}
