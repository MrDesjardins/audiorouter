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
//!        [--tone HZ --render-endpoint ID --capture-endpoint ID]
//!
//! `--tone` plays a quiet sine (amplitude 0.05) into the render endpoint
//! (Cable A Input) for the whole run and records the capture endpoint
//! (Cable B Output) plus a loopback reference of the render endpoint. From
//! 2 s after start until 1 s before stop it counts every discontinuity in
//! each recording with the sine recurrence `x[n] = 2cos(w)x[n-1] - x[n-2]`
//! (the method of `crates/transport/tests/live_audio_continuity.rs`),
//! streaming, so a long run needs no recording buffer.
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
    let mut tone_hz = None;
    let mut render_endpoint = None;
    let mut capture_endpoint = None;
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
            "--tone" => {
                tone_hz = Some(
                    value()?
                        .parse::<f64>()
                        .ok()
                        .filter(|hz| (20.0..=4_000.0).contains(hz))
                        .ok_or("--tone must be 20..4000 Hz")?,
                )
            }
            "--render-endpoint" => render_endpoint = Some(value()?),
            "--capture-endpoint" => capture_endpoint = Some(value()?),
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
    #[cfg(windows)]
    let tone = match tone_hz {
        Some(hz) => Some(tone::Run::start(
            hz,
            render_endpoint.ok_or("--tone needs --render-endpoint")?,
            capture_endpoint.ok_or("--tone needs --capture-endpoint")?,
        )?),
        None => None,
    };
    #[cfg(not(windows))]
    let _ = (tone_hz, render_endpoint, capture_endpoint);
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
        // Analyse from 2 s after the route started until 1 s before it stops.
        #[cfg(windows)]
        if let Some(tone) = &tone {
            let elapsed = start.elapsed();
            tone.set_measuring(
                elapsed >= Duration::from_secs(2)
                    && elapsed + Duration::from_secs(1) < Duration::from_secs(seconds),
            );
        }
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
    #[cfg(windows)]
    if let Some(tone) = tone {
        let (result, reference) = tone.finish()?;
        println!("tone result (Cable B Output): {result}");
        println!("tone reference (Cable A Input loopback): {reference}");
    }
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

#[cfg(windows)]
mod tone {
    use audiorouter_windows_audio::{
        enumerate_active_endpoints, AudioError, AudioServiceThreadGuard, EndpointDirection,
        SharedCapture, SharedRender,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread::JoinHandle;

    const RATE: f64 = 48_000.0;
    const AMPLITUDE: f32 = 0.05;

    fn sample(hz: f64, index: u64) -> f32 {
        (f64::from(AMPLITUDE) * (2.0 * std::f64::consts::PI * hz * index as f64 / RATE).sin())
            as f32
    }

    fn audio(error: AudioError) -> String {
        format!("{error:?}")
    }

    /// Streaming discontinuity count of a recorded pure tone (left channel).
    #[derive(Default)]
    pub struct Analysis {
        coefficient: f32,
        // The two previous samples, oldest first; prediction needs both.
        history: (Option<f32>, Option<f32>),
        samples: u64,
        tone_samples: u64,
        peak: f32,
        glitches: u64,
        last_glitch: Option<u64>,
        first_glitches_s: Vec<f64>,
        max_residual: f32,
        zero_run: u64,
        silent_runs: u64,
        silent_samples: u64,
        flags: u64,
    }

    impl Analysis {
        fn new(hz: f64) -> Self {
            Self {
                coefficient: (2.0 * (2.0 * std::f64::consts::PI * hz / RATE).cos()) as f32,
                ..Self::default()
            }
        }

        /// Measurement starts or stops: the next sample has no predecessor.
        fn reset_history(&mut self) {
            self.history = (None, None);
            self.zero_run = 0;
        }

        fn push(&mut self, x: f32) {
            self.samples += 1;
            self.peak = self.peak.max(x.abs());
            if x.abs() > AMPLITUDE / 4.0 {
                self.tone_samples += 1;
            }
            if let (Some(older), Some(old)) = self.history {
                let residual = (x - (self.coefficient * old - older)).abs();
                if residual > AMPLITUDE * 0.05 {
                    self.max_residual = self.max_residual.max(residual / AMPLITUDE);
                    // Detections within 64 samples are one glitch.
                    if self
                        .last_glitch
                        .is_none_or(|last| self.samples - last >= 64)
                    {
                        self.glitches += 1;
                        if self.first_glitches_s.len() < 20 {
                            self.first_glitches_s.push(self.samples as f64 / RATE);
                        }
                    }
                    self.last_glitch = Some(self.samples);
                }
            }
            self.history = (self.history.1, Some(x));
            // Underruns render exact digital silence; a run of zeros longer
            // than two samples cannot occur inside the sine.
            if x == 0.0 {
                self.zero_run += 1;
            } else {
                if self.zero_run > 2 {
                    self.silent_runs += 1;
                    self.silent_samples += self.zero_run;
                }
                self.zero_run = 0;
            }
        }
    }

    impl std::fmt::Display for Analysis {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let first: Vec<f64> = self
                .first_glitches_s
                .iter()
                .map(|seconds| (seconds * 1000.0).round() / 1000.0)
                .collect();
            write!(
                f,
                "analysed_s={:.1} tone_fraction={:.3} peak={:.4} glitches={} max_residual={:.2} silent_runs={} silent_samples={} discontinuity_flags={} first_glitches_s={first:?}",
                self.samples as f64 / RATE,
                self.tone_samples as f64 / self.samples.max(1) as f64,
                self.peak,
                self.glitches,
                self.max_residual,
                self.silent_runs,
                self.silent_samples,
                self.flags,
            )
        }
    }

    pub struct Run {
        stop: Arc<AtomicBool>,
        measuring: Arc<AtomicBool>,
        generator: JoinHandle<Result<(), String>>,
        result: JoinHandle<Result<Analysis, String>>,
        reference: JoinHandle<Result<Analysis, String>>,
    }

    fn generate(
        hz: f64,
        render_id: &str,
        channels: usize,
        stop: &AtomicBool,
    ) -> Result<(), String> {
        let (_guard, _) = AudioServiceThreadGuard::enter();
        let mut render = SharedRender::open_with_headroom(render_id, 500_000).map_err(audio)?;
        let mut next = 0_u64;
        let mut chunk = Vec::with_capacity(4096 * channels * 4);
        let mut fill = |render: &SharedRender, next: &mut u64| -> Result<(), String> {
            chunk.clear();
            for index in *next..*next + 4096 {
                let bytes = sample(hz, index).to_le_bytes();
                for _ in 0..channels {
                    chunk.extend_from_slice(&bytes);
                }
            }
            *next += u64::from(render.submit_bytes(&chunk, channels * 4).map_err(audio)?);
            Ok(())
        };
        fill(&render, &mut next)?;
        render.start().map_err(audio)?;
        while !stop.load(Ordering::Acquire) {
            render.wait_for_data(20).map_err(audio)?;
            fill(&render, &mut next)?;
        }
        Ok(())
    }

    fn record(
        hz: f64,
        id: &str,
        loopback: bool,
        stop: &AtomicBool,
        measuring: &AtomicBool,
    ) -> Result<Analysis, String> {
        let (_guard, _) = AudioServiceThreadGuard::enter();
        let endpoint = enumerate_active_endpoints()
            .map_err(audio)?
            .into_iter()
            .find(|endpoint| endpoint.id == id)
            .ok_or("recorded endpoint disappeared")?;
        let frame_bytes = usize::from(endpoint.channels) * 4;
        let mut capture = if loopback {
            SharedCapture::open_loopback(id)
        } else {
            SharedCapture::open(id, 0)
        }
        .map_err(audio)?;
        let mut buffer = vec![0_u8; 48_000 * frame_bytes];
        let mut analysis = Analysis::new(hz);
        let mut was_measuring = false;
        capture.start().map_err(audio)?;
        while !stop.load(Ordering::Acquire) {
            capture.wait_for_data(20).map_err(audio)?;
            while let Some((packet, bytes)) = capture
                .next_packet_into(&mut buffer, frame_bytes)
                .map_err(audio)?
            {
                let measure = measuring.load(Ordering::Acquire);
                if measure != was_measuring {
                    analysis.reset_history();
                    was_measuring = measure;
                }
                if !measure {
                    continue;
                }
                // AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY
                if packet.flags & 0x1 != 0 {
                    analysis.flags += 1;
                }
                for frame in buffer[..bytes].chunks_exact(frame_bytes) {
                    analysis.push(f32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]));
                }
            }
        }
        Ok(analysis)
    }

    impl Run {
        pub fn start(hz: f64, render_id: String, capture_id: String) -> Result<Self, String> {
            let endpoints = enumerate_active_endpoints().map_err(audio)?;
            let find = |id: &str, direction| {
                endpoints
                    .iter()
                    .find(|endpoint| endpoint.id == id && endpoint.direction == direction)
                    .cloned()
                    .ok_or_else(|| format!("endpoint {id} ({direction:?}) is not active"))
            };
            let render = find(&render_id, EndpointDirection::Render)?;
            find(&capture_id, EndpointDirection::Capture)?;
            if !render.is_ieee_float32() || render.sample_rate_hz != 48_000 {
                return Err("the render endpoint must be 48 kHz float32".into());
            }
            let stop = Arc::new(AtomicBool::new(false));
            let measuring = Arc::new(AtomicBool::new(false));
            let channels = usize::from(render.channels);
            let generator = {
                let stop = Arc::clone(&stop);
                let render_id = render_id.clone();
                std::thread::spawn(move || generate(hz, &render_id, channels, &stop))
            };
            let recorder = |id: String, loopback: bool| {
                let stop = Arc::clone(&stop);
                let measuring = Arc::clone(&measuring);
                std::thread::spawn(move || record(hz, &id, loopback, &stop, &measuring))
            };
            let result = recorder(capture_id, false);
            let reference = recorder(render_id, true);
            Ok(Self {
                stop,
                measuring,
                generator,
                result,
                reference,
            })
        }

        pub fn set_measuring(&self, measuring: bool) {
            self.measuring.store(measuring, Ordering::Release);
        }

        pub fn finish(self) -> Result<(Analysis, Analysis), String> {
            self.measuring.store(false, Ordering::Release);
            self.stop.store(true, Ordering::Release);
            let join = |handle: JoinHandle<Result<Analysis, String>>| {
                handle
                    .join()
                    .map_err(|_| "tone recorder panicked".to_owned())?
            };
            let result = join(self.result)?;
            let reference = join(self.reference)?;
            self.generator
                .join()
                .map_err(|_| "tone generator panicked".to_owned())??;
            Ok((result, reference))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn streaming_analysis_matches_the_live_harness_cases() {
            let mut clean = Analysis::new(997.0);
            (0..48_000).for_each(|index| clean.push(sample(997.0, index)));
            assert_eq!((clean.glitches, clean.silent_runs), (0, 0));
            // Drop one 480-frame block and zero another (live harness case).
            let mut damaged = Analysis::new(997.0);
            (0..10_000)
                .chain(10_480..30_000)
                .enumerate()
                .for_each(|(position, index)| {
                    damaged.push(if (20_000..20_480).contains(&position) {
                        0.0
                    } else {
                        sample(997.0, index)
                    })
                });
            assert_eq!(damaged.glitches, 3, "drop, zero start and zero end");
            assert_eq!((damaged.silent_runs, damaged.silent_samples), (1, 480));
            let mut low = Analysis::new(47.0);
            (0..96_000).for_each(|index| low.push(sample(47.0, index)));
            assert_eq!(low.glitches, 0);
        }
    }
}
