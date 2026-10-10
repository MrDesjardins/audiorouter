use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use audiorouter_windows_audio::{
    enumerate_active_endpoint_display_info, enumerate_active_endpoints, AudioServiceThreadGuard,
    EndpointDirection, EndpointDisplayInfo, EndpointInfo, SharedCapture, SharedRender,
};
use serde_json::{json, Value};

mod signal;
pub use signal::analyze;

const RATE: usize = 48_000;
const STRIDE: usize = 8;
const MAX_SECONDS: usize = 100;

pub fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(
        serde_json::to_string_pretty(value)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(|e| e.to_string())
}

fn select_endpoint(
    displays: &[EndpointDisplayInfo],
    formats: &[EndpointInfo],
    name: &str,
    direction: EndpointDirection,
) -> Result<EndpointInfo, String> {
    let found: Vec<_> = displays
        .iter()
        .filter(|e| e.name == name && e.direction == direction)
        .collect();
    if found.len() != 1 {
        return Err(format!("expected one active {name}: found {}", found.len()));
    }
    let format = formats
        .iter()
        .find(|e| e.id == found[0].id && e.direction == direction)
        .ok_or_else(|| format!("mix format missing for {name}"))?;
    if !format.is_ieee_float32()
        || format.sample_rate_hz != RATE as u32
        || format.channels != 2
        || format.bytes_per_frame().map_err(|e| e.to_string())? != STRIDE
    {
        return Err(format!(
            "{name}: requires stereo IEEE float32 at 48000 Hz, observed {format:?}"
        ));
    }
    Ok(format.clone())
}

pub fn record(directory: &Path) -> Result<(), String> {
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || pump(worker_stop, ready_tx));
    // Control/file I/O stays off the service thread. Even if this controller
    // stalls, the service thread has its own 100-second monotonic deadline.
    let control: Result<(), String> = (|| {
        let metadata = ready_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|e| format!("source readiness: {e}"))?;
        write_json(&directory.join("ready.pending"), &metadata)?;
        std::fs::rename(
            directory.join("ready.pending"),
            directory.join("ready.json"),
        )
        .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(90);
        while !directory.join("stop").exists() && !worker.is_finished() && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(50));
        }
        if Instant::now() >= deadline {
            return Err("controller deadline reached".into());
        }
        Ok(())
    })();
    stop.store(true, Ordering::Release);
    let recording = worker
        .join()
        .map_err(|_| "audio worker panicked".to_string())??;
    signal::write_wav(&directory.join("cable-b-output.wav"), &recording.bytes)?;
    write_json(&directory.join("recording.json"), &recording.report)?;
    control?;
    if let Some(error) = recording.error {
        return Err(error);
    }
    Ok(())
}

struct Recording {
    bytes: Vec<u8>,
    report: Value,
    error: Option<String>,
}

fn pump(stop: Arc<AtomicBool>, ready: mpsc::SyncSender<Value>) -> Result<Recording, String> {
    let (_guard, capabilities) = AudioServiceThreadGuard::enter();
    if !capabilities.com_multithreaded
        || !capabilities.mmcss_pro_audio
        || !capabilities.one_millisecond_timer
    {
        return Err(format!("required scheduling unavailable: {capabilities:?}"));
    }
    let displays = enumerate_active_endpoint_display_info().map_err(|e| e.to_string())?;
    let formats = enumerate_active_endpoints().map_err(|e| e.to_string())?;
    let source = select_endpoint(
        &displays,
        &formats,
        "AudioRouter Cable A Input (AudioRouter Virtual Cable)",
        EndpointDirection::Render,
    )?;
    let sink = select_endpoint(
        &displays,
        &formats,
        "AudioRouter Cable B Output (AudioRouter Virtual Cable)",
        EndpointDirection::Capture,
    )?;
    let mut capture = SharedCapture::open(&sink.id, 0).map_err(|e| e.to_string())?;
    let mut render = SharedRender::open(&source.id, 0).map_err(|e| e.to_string())?;
    let refreshed = enumerate_active_endpoints().map_err(|e| e.to_string())?;
    if !refreshed.contains(&source) || !refreshed.contains(&sink) {
        return Err("endpoint mix format changed while clients opened; no stream started".into());
    }
    let capacity = render.buffer_frames() as usize;
    if capacity == 0 || capacity > RATE {
        return Err("unexpected render buffer capacity".into());
    }
    // Allocate every storage area before Start. Packet metadata is bounded
    // even for 128-frame capture packets over the complete watchdog interval.
    let mut bytes = Vec::with_capacity(RATE * MAX_SECONDS * STRIDE);
    let mut packets = Vec::with_capacity(60_000);
    let mut packet = vec![0u8; RATE * STRIDE];
    let mut generated = vec![0u8; capacity * STRIDE];
    let mut submitted = 0u64;
    fill_source(&mut generated, submitted);
    submitted += u64::from(
        render
            .submit_bytes(&generated, STRIDE)
            .map_err(|e| e.to_string())?,
    );
    capture.start().map_err(|e| e.to_string())?;
    render.start().map_err(|e| e.to_string())?;
    ready
        .try_send(json!({"sourceId":source.id,"captureId":sink.id,"rate":RATE,
        "channels":2,"bits":32,"format":"IEEE_FLOAT","sourceFrequencies":[440,660],
        "captureFrequencies":[997,47],"renderBufferFrames":capacity}))
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut previous = start;
    let mut max_gap_us = 0u128;
    let mut error = None;
    let outcome = (|| {
        while !stop.load(Ordering::Acquire)
            && start.elapsed() < Duration::from_secs(MAX_SECONDS as u64)
        {
            let now = Instant::now();
            max_gap_us = max_gap_us.max(now.duration_since(previous).as_micros());
            previous = now;
            // Limit one pass so a large catch-up backlog cannot starve render.
            for _ in 0..16 {
                let Some((info, count)) = capture
                    .next_packet_into(&mut packet, STRIDE)
                    .map_err(|e| e.to_string())?
                else {
                    break;
                };
                if count != info.frames as usize * STRIDE
                    || bytes.len() + count > bytes.capacity()
                    || packets.len() == packets.capacity()
                {
                    return Err("bounded capture storage exhausted or packet shape changed".into());
                }
                packets.push((bytes.len() / STRIDE, info));
                bytes.extend_from_slice(&packet[..count]);
            }
            fill_source(&mut generated, submitted);
            submitted += u64::from(
                render
                    .submit_bytes(&generated, STRIDE)
                    .map_err(|e| e.to_string())?,
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        if !stop.load(Ordering::Acquire) {
            return Err("audio worker deadline reached".into());
        }
        Ok(())
    })();
    if let Err(e) = outcome {
        error = Some(e);
    }
    // Stop/reset both clients before any recording/report file is written.
    for result in [capture.stop(), render.stop()] {
        if let Err(e) = result {
            error.get_or_insert_with(|| e.to_string());
        }
    }
    let packet_report: Vec<_> = packets
        .iter()
        .map(|(offset, p)| {
            json!({"fileFrame":offset,"frames":p.frames,
        "flags":p.flags,"devicePosition":p.device_position,"qpc100ns":p.qpc_position})
        })
        .collect();
    Ok(Recording {
        bytes,
        report: json!({"elapsedSeconds":start.elapsed().as_secs_f64(),"sourceFrames":submitted,
        "maxPumpGapUs":max_gap_us,"packets":packet_report,"error":error}),
        error,
    })
}

fn fill_source(bytes: &mut [u8], first_frame: u64) {
    for (offset, frame) in bytes.chunks_exact_mut(STRIDE).enumerate() {
        for (channel, frequency) in [440.0, 660.0].into_iter().enumerate() {
            let phase = std::f64::consts::TAU * frequency * (first_frame + offset as u64) as f64
                / RATE as f64;
            frame[channel * 4..channel * 4 + 4]
                .copy_from_slice(&(0.25 * phase.sin() as f32).to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_submissions_preserve_source_phase() {
        let mut full = vec![0; 480 * STRIDE];
        fill_source(&mut full, 0);
        let mut remainder = vec![0; 360 * STRIDE];
        fill_source(&mut remainder, 120);
        assert_eq!(&full[120 * STRIDE..], remainder);
    }
    #[test]
    fn endpoint_mix_rate_and_sample_type_must_match() {
        let display = EndpointDisplayInfo {
            id: "id".into(),
            direction: EndpointDirection::Capture,
            name: "expected".into(),
            device_description: String::new(),
            driver_inf_section: String::new(),
        };
        let format = EndpointInfo {
            id: "id".into(),
            direction: EndpointDirection::Capture,
            default_period_100ns: 100000,
            minimum_period_100ns: 26667,
            sample_rate_hz: 48000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 3,
            subformat_guid: String::new(),
        };
        assert!(select_endpoint(
            std::slice::from_ref(&display),
            std::slice::from_ref(&format),
            "expected",
            EndpointDirection::Capture
        )
        .is_ok());
        for (rate, tag) in [(44100, 3), (48000, 1)] {
            let mut wrong = format.clone();
            wrong.sample_rate_hz = rate;
            wrong.format_tag = tag;
            assert!(select_endpoint(
                std::slice::from_ref(&display),
                &[wrong],
                "expected",
                EndpointDirection::Capture
            )
            .is_err());
        }
        assert!(
            select_endpoint(&[display], &[format], "expected", EndpointDirection::Render).is_err()
        );
    }
    #[test]
    fn missing_and_duplicate_names_never_fall_back() {
        assert!(select_endpoint(&[], &[], "expected", EndpointDirection::Capture).is_err());
        let display = EndpointDisplayInfo {
            id: "id".into(),
            direction: EndpointDirection::Capture,
            name: "expected".into(),
            device_description: String::new(),
            driver_inf_section: String::new(),
        };
        assert!(select_endpoint(
            &[display.clone(), display],
            &[],
            "expected",
            EndpointDirection::Capture
        )
        .is_err());
    }
}
