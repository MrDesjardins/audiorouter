//! WP-06 VM tool: exercise the AudioRouter cable bridge without the engine.
//!
//! Writes a 997 Hz tone (even channels) and a 47 Hz tone (odd channels) into
//! the capture-sink lease of one cable, so a recorder on that cable's
//! "Output" endpoint hears them, and records the render-source lease of
//! another cable (what apps play into its "Input" endpoint) to a WAV file.
//! It prints the driver's QUERY report and both leases' stream counters.
//!
//! Run it only in the test VM with the test-signed driver loaded. It never
//! opens a microphone; the WAV holds only audio that was played into the
//! render-source cable during the run.
//!
//! Build: `cargo build --release -p audiorouter-windows-audio --example m03_bridge_tone`
//! (output: `target\release\examples\m03_bridge_tone.exe`).

use std::io::{Seek, SeekFrom, Write};

const USAGE: &str = "usage: m03_bridge_tone [--seconds N] [--rate 44100|48000|96000] \
[--channels 1-8] [--frames N] [--capture-bus cable-b] [--render-bus cable-a] \
[--out PATH.wav] [--wav64] [--stall-ms N] [--device PATH]";

#[derive(Clone, Debug, PartialEq)]
struct Options {
    device: String,
    seconds: u32,
    rate: u32,
    channels: u16,
    frames: u16,
    capture_bus: String,
    render_bus: String,
    out: std::path::PathBuf,
    wav64: bool,
    stall_ms: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            device: r"\\.\AudioRouterVirtualBridge".to_owned(),
            seconds: 10,
            rate: 48_000,
            channels: 2,
            frames: 480,
            capture_bus: "cable-b".to_owned(),
            render_bus: "cable-a".to_owned(),
            out: std::path::PathBuf::from("render-source.wav"),
            wav64: false,
            stall_ms: 0,
        }
    }
}

fn parse_options(arguments: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut iter = arguments.iter();
    while let Some(flag) = iter.next() {
        let mut value = || {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--device" => options.device = value()?,
            "--seconds" => options.seconds = number(&value()?, 1, 3_600)?,
            "--rate" => {
                options.rate = number(&value()?, 1, 192_000)?;
                if ![44_100, 48_000, 96_000].contains(&options.rate) {
                    return Err("--rate must be 44100, 48000 or 96000".to_owned());
                }
            }
            "--channels" => options.channels = number(&value()?, 1, 8)? as u16,
            "--frames" => options.frames = number(&value()?, 16, 4_096)? as u16,
            "--capture-bus" => options.capture_bus = cable(&value()?)?,
            "--render-bus" => options.render_bus = cable(&value()?)?,
            "--out" => options.out = std::path::PathBuf::from(value()?),
            "--wav64" => options.wav64 = true,
            "--stall-ms" => options.stall_ms = number(&value()?, 0, 60_000)?,
            "--help" | "-h" => return Err(USAGE.to_owned()),
            other => return Err(format!("unknown option {other}\n{USAGE}")),
        }
    }
    if options.capture_bus == options.render_bus {
        // One cable can carry both directions, but separate cables are what
        // the crosstalk check needs; allow it only explicitly via distinct IDs.
        return Err("use different cables for --capture-bus and --render-bus".to_owned());
    }
    Ok(options)
}

fn number(text: &str, min: u32, max: u32) -> Result<u32, String> {
    let value: u32 = text.parse().map_err(|_| format!("not a number: {text}"))?;
    if !(min..=max).contains(&value) {
        return Err(format!("{value} is outside {min}..={max}"));
    }
    Ok(value)
}

fn cable(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    if bytes.len() == 7 && text.starts_with("cable-") && (b'a'..=b'h').contains(&bytes[6]) {
        Ok(text.to_owned())
    } else {
        Err(format!("cable IDs are cable-a … cable-h, not {text}"))
    }
}

/// Bounded control-thread snapshots. No output operation is possible during
/// collection: console and disk backpressure must not delay lease heartbeats
/// or the run deadline. Flush only after leases and audio workers stop.
struct ProgressLog {
    lines: Vec<String>,
    limit: usize,
}

impl ProgressLog {
    fn new(seconds: u32, stall_ms: u32) -> Self {
        let limit = (seconds.min(3_600) + stall_ms.min(60_000).div_ceil(1_000) + 1) as usize;
        Self {
            lines: Vec::with_capacity(limit),
            limit,
        }
    }

    fn record(&mut self, line: String) -> Result<(), &'static str> {
        if self.lines.len() == self.limit {
            return Err("progress snapshot capacity exhausted");
        }
        self.lines.push(line);
        Ok(())
    }

    fn write_to(&self, output: &mut impl Write) -> std::io::Result<()> {
        for line in &self.lines {
            writeln!(output, "{line}")?;
        }
        Ok(())
    }
}

/// Fill one interleaved block: 997 Hz on even channels, 47 Hz on odd ones,
/// at -12 dBFS. Phase is derived from the absolute frame index so blocks
/// join without discontinuities.
fn fill_tone_block(block: &mut [f64], channels: u16, first_frame: u64, rate: u32) {
    let amplitude = 0.25_f64;
    let channels = usize::from(channels);
    for (frame_offset, frame) in block.chunks_exact_mut(channels).enumerate() {
        let t = (first_frame + frame_offset as u64) as f64 / f64::from(rate);
        for (channel, sample) in frame.iter_mut().enumerate() {
            let hz = if channel % 2 == 0 { 997.0 } else { 47.0 };
            *sample = amplitude * (2.0 * std::f64::consts::PI * hz * t).sin();
        }
    }
}

/// IEEE-float WAV with sizes patched on `finish`.
struct WavWriter {
    file: std::fs::File,
    channels: u16,
    bytes_per_sample: u16,
    data_bytes: u64,
}

impl WavWriter {
    fn create(
        path: &std::path::Path,
        channels: u16,
        rate: u32,
        wav64: bool,
    ) -> std::io::Result<Self> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        let bytes_per_sample: u16 = if wav64 { 8 } else { 4 };
        file.write_all(&wav_header(channels, rate, bytes_per_sample, 0))?;
        Ok(Self {
            file,
            channels,
            bytes_per_sample,
            data_bytes: 0,
        })
    }

    fn append(&mut self, samples: &[f64]) -> std::io::Result<()> {
        let mut bytes = Vec::with_capacity(samples.len() * usize::from(self.bytes_per_sample));
        for &sample in samples {
            if self.bytes_per_sample == 8 {
                bytes.extend_from_slice(&sample.to_le_bytes());
            } else {
                bytes.extend_from_slice(&(sample as f32).to_le_bytes());
            }
        }
        self.file.write_all(&bytes)?;
        self.data_bytes += bytes.len() as u64;
        Ok(())
    }

    fn finish(mut self, rate: u32) -> std::io::Result<u64> {
        let data = u32::try_from(self.data_bytes).unwrap_or(u32::MAX);
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&wav_header(
            self.channels,
            rate,
            self.bytes_per_sample,
            data,
        ))?;
        self.file.flush()?;
        Ok(self.data_bytes / u64::from(self.bytes_per_sample) / u64::from(self.channels))
    }
}

fn wav_header(channels: u16, rate: u32, bytes_per_sample: u16, data_bytes: u32) -> [u8; 44] {
    let block_align = channels * bytes_per_sample;
    let mut header = [0_u8; 44];
    header[0..4].copy_from_slice(b"RIFF");
    header[4..8].copy_from_slice(&(36_u32.saturating_add(data_bytes)).to_le_bytes());
    header[8..12].copy_from_slice(b"WAVE");
    header[12..16].copy_from_slice(b"fmt ");
    header[16..20].copy_from_slice(&16_u32.to_le_bytes());
    header[20..22].copy_from_slice(&3_u16.to_le_bytes()); // WAVE_FORMAT_IEEE_FLOAT
    header[22..24].copy_from_slice(&channels.to_le_bytes());
    header[24..28].copy_from_slice(&rate.to_le_bytes());
    header[28..32].copy_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
    header[32..34].copy_from_slice(&block_align.to_le_bytes());
    header[34..36].copy_from_slice(&(bytes_per_sample * 8).to_le_bytes());
    header[36..40].copy_from_slice(b"data");
    header[40..44].copy_from_slice(&data_bytes.to_le_bytes());
    header
}

#[cfg(windows)]
fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let options = match parse_options(&arguments) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(64);
        }
    };
    std::process::exit(match run(&options) {
        Ok(()) => 0,
        Err((code, message)) => {
            eprintln!("error: {message}");
            code
        }
    });
}

#[cfg(windows)]
#[path = "m03_bridge_tone/workers.rs"]
mod workers;

#[cfg(windows)]
struct TimerResolution;

#[cfg(windows)]
impl TimerResolution {
    fn acquire() -> Result<Self, (i32, String)> {
        // SAFETY: process timer request, paired with timeEndPeriod in Drop.
        let result = unsafe { windows::Win32::Media::timeBeginPeriod(1) };
        if result != 0 {
            return Err((1, format!("1 ms timer resolution failed: {result}")));
        }
        Ok(Self)
    }
}

#[cfg(windows)]
impl Drop for TimerResolution {
    fn drop(&mut self) {
        // SAFETY: matches this guard's successful timeBeginPeriod request.
        unsafe {
            windows::Win32::Media::timeEndPeriod(1);
        }
    }
}

#[cfg(windows)]
fn run(options: &Options) -> Result<(), (i32, String)> {
    use audiorouter_windows_audio::{
        NativeBridgeControlClient, NativeBridgeController, NativeBridgeRegion, NativeBridgeSession,
    };
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    fn fail(error: impl std::fmt::Debug) -> (i32, String) {
        (1, format!("{error:?}"))
    }
    let client = NativeBridgeControlClient::open(&options.device).map_err(fail)?;
    let info = client.query().map_err(fail)?;
    println!("driver: {info:?}");
    info.check_compatible()
        .map_err(|error| (2, format!("{error:?}")))?;
    let packet_counters = info.has(audiorouter_windows_audio::NATIVE_DRIVER_CAP_PACKET_COUNTERS);
    drop(client);
    let generation = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| (1, error.to_string()))?
        .as_millis() as u64;
    let hello = |bus: &str, direction| audiorouter_protocol::AudioBridgeHello {
        protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
        protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
        bus_id: bus.to_owned(),
        direction,
        generation,
        sample_rate_hz: options.rate,
        channels: options.channels,
        frames_per_quantum: options.frames,
        lease_ms: 2_000,
    };
    // Prepared sessions own the files even if a later preparation fails.
    struct Prepared(Option<NativeBridgeSession>);
    impl Drop for Prepared {
        fn drop(&mut self) {
            if let Some(session) = self.0.take() {
                let _ = session.remove_owned_mapping();
            }
        }
    }
    let temp = std::env::temp_dir();
    let capture_path = temp.join(format!(
        "audiorouter-bridge-tone-{}-capture.slot",
        std::process::id()
    ));
    let render_path = temp.join(format!(
        "audiorouter-bridge-tone-{}-render.slot",
        std::process::id()
    ));
    let samples = usize::from(options.frames) * usize::from(options.channels);
    let mut tone = vec![0.0; samples];
    fill_tone_block(&mut tone, options.channels, 0, options.rate);
    let mut prepared_capture = Prepared(Some(
        NativeBridgeSession::create_primed(
            &capture_path,
            hello(
                &options.capture_bus,
                audiorouter_protocol::AudioBridgeDirection::CaptureSink,
            ),
            &tone,
        )
        .map_err(fail)?,
    ));
    let mut prepared_render = Prepared(Some(
        NativeBridgeSession::create(
            &render_path,
            hello(
                &options.render_bus,
                audiorouter_protocol::AudioBridgeDirection::RenderSource,
            ),
        )
        .map_err(fail)?,
    ));
    let capture_view =
        NativeBridgeRegion::open(&capture_path, options.channels, options.frames).map_err(fail)?;
    let render_view =
        NativeBridgeRegion::open(&render_path, options.channels, options.frames).map_err(fail)?;
    let mut wav = WavWriter::create(&options.out, options.channels, options.rate, options.wav64)
        .map_err(|error| {
            (
                1,
                format!("cannot create {}: {error}", options.out.display()),
            )
        })?;
    let state = workers::Shared::new(samples, 64);
    let _timer = TimerResolution::acquire()?;
    println!("isolated capture/render workers; 64 preallocated recording blocks; {} seconds; intentional stall {} ms", options.seconds, options.stall_ms);

    std::thread::scope(|scope| -> Result<(), (i32, String)> {
        // Declared before spawning: every early return wakes waiting workers.
        struct Stop<'a>(&'a workers::Shared);
        impl Drop for Stop<'_> {
            fn drop(&mut self) {
                self.0.stop.store(true, Ordering::Release);
            }
        }
        let _stop = Stop(&state);
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(2);
        let capture_tx = ready_tx.clone();
        let capture_state = &state;
        let capture_region = &capture_view;
        let capture_worker = std::thread::Builder::new()
            .name("tone-capture".to_owned())
            .spawn_scoped(scope, move || {
                let _panic = workers::PanicSignal(capture_state);
                let setup = workers::prepare_thread();
                let readiness = match &setup {
                    Ok((_, scheduling)) => Ok(*scheduling),
                    Err(error) => Err(error.clone()),
                };
                let _ = capture_tx.send(("capture", readiness));
                let result = match setup {
                    Ok((_scheduling_guard, _scheduling)) => workers::capture(
                        capture_region,
                        capture_state,
                        options,
                        generation,
                        &mut tone,
                    ),
                    Err(error) => Err(error),
                };
                if result.is_err() {
                    capture_state.failed.store(true, Ordering::Release);
                }
                result
            })
            .map_err(|error| (1, format!("capture worker startup: {error}")))?;
        let render_state = &state;
        let render_region = &render_view;
        let render_worker = std::thread::Builder::new()
            .name("tone-render".to_owned())
            .spawn_scoped(scope, move || {
                let _panic = workers::PanicSignal(render_state);
                struct Finished<'a>(&'a workers::Shared);
                impl Drop for Finished<'_> {
                    fn drop(&mut self) {
                        self.0.renderer_done.store(true, Ordering::Release);
                    }
                }
                let _finished = Finished(render_state);
                let setup = workers::prepare_thread();
                let readiness = match &setup {
                    Ok((_, scheduling)) => Ok(*scheduling),
                    Err(error) => Err(error.clone()),
                };
                let _ = ready_tx.send(("render", readiness));
                let result = match setup {
                    Ok((_scheduling_guard, _scheduling)) => {
                        workers::render(render_region, render_state, options, generation)
                    }
                    Err(error) => Err(error),
                };
                if result.is_err() {
                    render_state.failed.store(true, Ordering::Release);
                }
                result
            })
            .map_err(|error| (1, format!("render worker startup: {error}")))?;
        for _ in 0..2 {
            let (worker_name, readiness) = ready_rx
                .recv_timeout(Duration::from_secs(10))
                .map_err(|error| (1, format!("worker readiness: {error}")))?;
            let scheduling = readiness.map_err(|error| (1, error))?;
            println!(
                "{worker_name} worker scheduling: COM_MTA={} MMCSS_Pro_Audio={} timer_1ms={} highest_priority_fallback={}",
                scheduling.capabilities.com_multithreaded,
                scheduling.capabilities.mmcss_pro_audio,
                scheduling.capabilities.one_millisecond_timer,
                scheduling.highest_priority_fallback,
            );
        }
        let disk_state = &state;
        let disk_worker = std::thread::Builder::new()
            .name("tone-recording".to_owned())
            .spawn_scoped(scope, move || {
                let _panic = workers::PanicSignal(disk_state);
                let recorded = workers::record(disk_state, |samples| {
                    wav.append(samples)
                        .map_err(|error| format!("WAV write: {error}"))
                });
                let before = Instant::now();
                let frames = wav
                    .finish(options.rate)
                    .map_err(|error| format!("WAV finish: {error}"))?;
                workers::observe_max(&disk_state.disk_finish_us, before.elapsed());
                recorded.map(|()| frames)
            })
            .map_err(|error| (1, format!("recording worker startup: {error}")))?;

        // Activation errors still stop/join audio and finalize the WAV.
        let mut render_controller: Option<NativeBridgeController> = None;
        let mut capture_controller: Option<NativeBridgeController> = None;
        let mut progress_log = ProgressLog::new(options.seconds, options.stall_ms);
        let mut final_report = None;
        let operation = (|| -> Result<(), (i32, String)> {
            let start = Instant::now();
            state
                .start
                .set(start)
                .map_err(|_| (1, "worker start already set".to_owned()))?;
            capture_worker.thread().unpark();
            render_worker.thread().unpark();

            let arm_deadline = Instant::now() + Duration::from_secs(10);
            while !state.render_armed.load(Ordering::Acquire) {
                if state.failed.load(Ordering::Acquire) {
                    return Err((1, "render worker failed before polling".to_owned()));
                }
                if Instant::now() >= arm_deadline {
                    return Err((1, "render worker did not begin polling".to_owned()));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            println!("render worker polling before lease activation");
            render_controller = Some(
                NativeBridgeController::activate_prepared_section(
                    &options.device,
                    prepared_render.0.take().unwrap(),
                )
                .map_err(fail)?,
            );
            capture_controller = Some(
                NativeBridgeController::activate_prepared_section(
                    &options.device,
                    prepared_capture.0.take().unwrap(),
                )
                .map_err(fail)?,
            );
            let mut heartbeat = start;
            let mut progress = start;
            let mut control_us = 0;
            let mut report_us = 0;
            let mut last_control = Instant::now();
            let mut control_gap_us = 0;
            let mut interval_control_gap_us = 0;
            let run_for = Duration::from_secs(u64::from(options.seconds))
                + Duration::from_millis(u64::from(options.stall_ms));
            let service = (|| -> Result<(), (i32, String)> {
                while start.elapsed() < run_for
                    && !state.done.load(Ordering::Acquire)
                    && !state.failed.load(Ordering::Acquire)
                {
                    let now = Instant::now();
                    let gap = now.duration_since(last_control).as_micros();
                    last_control = now;
                    control_gap_us = control_gap_us.max(gap);
                    interval_control_gap_us = interval_control_gap_us.max(gap);
                    if heartbeat.elapsed() >= Duration::from_millis(250) {
                        let before = Instant::now();
                        capture_controller
                            .as_mut()
                            .unwrap()
                            .heartbeat()
                            .map_err(fail)?;
                        render_controller
                            .as_mut()
                            .unwrap()
                            .heartbeat()
                            .map_err(fail)?;
                        control_us = control_us.max(before.elapsed().as_micros());
                        heartbeat = Instant::now();
                    }
                    if progress.elapsed() >= Duration::from_secs(1) {
                        let before = Instant::now();
                        progress_log.record(format!(
                            "progress {} ms: written={} read={} ack={} interval_capture_gap_us={} interval_render_gap_us={} interval_control_gap_us={} capture={:?} render={:?} render_packets={:?}",
                            start.elapsed().as_millis(),
                            state.capture_blocks.load(Ordering::Acquire),
                            state.render_blocks.load(Ordering::Acquire),
                            capture_controller.as_ref().unwrap().consumer_sequence(),
                            state.capture_interval_gap_us.swap(0, Ordering::Relaxed),
                            state.render_interval_gap_us.swap(0, Ordering::Relaxed),
                            interval_control_gap_us,
                            capture_controller.as_ref().unwrap().counters(),
                            render_controller.as_ref().unwrap().counters(),
                            packet_counters.then(|| render_controller.as_ref().unwrap().packet_counters())
                        )).map_err(|error| (1, error.to_owned()))?;
                        report_us = report_us.max(before.elapsed().as_micros());
                        interval_control_gap_us = 0;
                        progress = Instant::now();
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(())
            })();
            // Deactivate both native leases before disk flush, output or joins.
            // CLOSE retires/reset the driver's mapping header, so preserve its
            // final raw counters immediately before deactivation.
            let capture_counters = capture_view.counters();
            let render_counters = render_view.counters();
            let render_packets = packet_counters.then(|| render_view.packet_counters());
            let close_start = Instant::now();
            // Deactivation clears the driver's mapping header. Tell both
            // workers that a SampleSizeMismatch from this point is the
            // expected retirement signal, avoiding a close/stop race.
            state.retiring.store(true, Ordering::Release);
            let capture_close = capture_controller
                .as_mut()
                .map_or(Ok(()), |controller| controller.deactivate().map_err(fail));
            let render_close = render_controller
                .as_mut()
                .map_or(Ok(()), |controller| controller.deactivate().map_err(fail));
            let close_us = close_start.elapsed().as_micros();
            state.stop.store(true, Ordering::Release);
            final_report = Some((
                capture_counters,
                render_counters,
                render_packets,
                control_us,
                report_us,
                close_us,
                control_gap_us,
                start.elapsed().as_millis(),
            ));
            service.and(capture_close).and(render_close)
        })();
        state.stop.store(true, Ordering::Release);
        capture_worker.thread().unpark();
        render_worker.thread().unpark();
        let capture_result = capture_worker
            .join()
            .map_err(|_| (1, "capture worker panicked".to_owned()))
            .and_then(|result| result.map_err(|error| (1, error)));
        let render_result = render_worker
            .join()
            .map_err(|_| (1, "render worker panicked".to_owned()))
            .and_then(|result| result.map_err(|error| (1, error)));
        let disk_result = disk_worker
            .join()
            .map_err(|_| (1, "recording worker panicked".to_owned()))
            .and_then(|result| result.map_err(|error| (1, error)));
        // Keep the owner sessions and backing files alive until all workers
        // have stopped using their independent mapped views.
        let capture_cleanup = capture_controller
            .take()
            .map_or(Ok(()), |controller| controller.close().map_err(fail));
        let render_cleanup = render_controller
            .take()
            .map_or(Ok(()), |controller| controller.close().map_err(fail));
        // Every lease is deactivated and every audio/disk worker is joined.
        // Preserve snapshots on disk before any potentially blocked stdout.
        let progress_path = options.out.with_extension("progress.txt");
        let progress_result = std::fs::File::create(&progress_path)
            .and_then(|mut file| progress_log.write_to(&mut file))
            .map_err(|error| (1, format!("progress report: {error}")));
        if let Some((
            capture_counters,
            render_counters,
            render_packets,
            control_us,
            report_us,
            close_us,
            control_gap_us,
            elapsed_ms,
        )) = final_report
        {
            println!(
                "capture-sink counters ({}): {capture_counters:?}",
                options.capture_bus
            );
            println!(
                "render-source counters ({}): {render_counters:?}",
                options.render_bus
            );
            // SetWritePacket outcomes: Windows' packet submissions on time,
            // late (already transferring) or too far ahead (17 §5.2).
            match render_packets {
                Some(packets) => println!(
                    "render-source packet writes ({}): {packets:?}",
                    options.render_bus
                ),
                None => println!(
                    "render-source packet writes ({}): not reported by this driver",
                    options.render_bus
                ),
            }
            println!("maximum control heartbeat: {control_us} us; progress snapshot: {report_us} us; lease close: {close_us} us");
            println!("leases deactivated at {elapsed_ms} ms; maximum control loop gap: {control_gap_us} us");
        }
        println!("interval progress snapshots: {}", progress_path.display());
        println!(
            "capture-sink blocks written: {}",
            state.capture_blocks.load(Ordering::Acquire)
        );
        println!(
            "render-source blocks read: {}; WAV frames: {:?} -> {}",
            state.render_blocks.load(Ordering::Acquire),
            disk_result.as_ref().map_or(0, |frames| *frames),
            options.out.display()
        );
        println!(
            "maximum pump gaps: capture={} us render={} us; WAV append={} us finish={} us",
            state.capture_gap_us.load(Ordering::Relaxed),
            state.render_gap_us.load(Ordering::Relaxed),
            state.disk_us.load(Ordering::Relaxed),
            state.disk_finish_us.load(Ordering::Relaxed)
        );
        println!(
            "harness render sequence gaps: {}",
            state.render_gaps.load(Ordering::Relaxed)
        );
        operation
            .and(capture_cleanup)
            .and(render_cleanup)
            .and(capture_result)
            .and(render_result)
            .and(progress_result)
            .and(disk_result.map(|_| ()))?;
        if state.capture_blocks.load(Ordering::Acquire) == 1 {
            return Err((
                1,
                "no capture consumer acknowledged the primed block; enable Listen before running"
                    .to_owned(),
            ));
        }
        if state.render_blocks.load(Ordering::Acquire) == 0 {
            return Err((
                1,
                "no render audio recorded; play the Cable A Input test during the run".to_owned(),
            ));
        }
        Ok(())
    })
}

#[cfg(not(windows))]
fn main() {
    eprintln!("m03_bridge_tone needs Windows and the AudioRouter cable driver");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn progress_collection_is_bounded_and_does_not_write() {
        let mut log = ProgressLog::new(1, 0);
        let capacity = log.lines.capacity();
        assert_eq!(log.limit, 2);
        log.record("first".to_owned()).unwrap();
        log.record("last".to_owned()).unwrap();
        assert!(log.record("overflow".to_owned()).is_err());
        assert_eq!(log.lines.capacity(), capacity);
        let mut output = Vec::new();
        // The output sink is introduced only after collection has finished.
        log.write_to(&mut output).unwrap();
        assert_eq!(output, b"first\nlast\n");
        assert_eq!(ProgressLog::new(u32::MAX, u32::MAX).limit, 3_661);
    }

    #[test]
    fn active_lease_service_has_deadline_and_no_output_calls() {
        // Guard the production wiring, not just the collector: active lease
        // service must have its own duration check and no console/file I/O.
        let source = include_str!("m03_bridge_tone.rs");
        let service = source.split("let service = (||").nth(1).unwrap();
        let live = service
            .split("// Deactivate both native leases")
            .next()
            .unwrap();
        assert!(live.contains("start.elapsed() < run_for"));
        assert!(live.contains("progress_log.record(format!("));
        for io in ["println!", "write_to", "File::create"] {
            assert!(!live.contains(io), "live lease service calls {io}");
        }
        let cleanup = service.split("let render_cleanup =").nth(1).unwrap();
        assert!(cleanup.find("progress_log.write_to").unwrap() < cleanup.find("println!").unwrap());
        assert!(
            service.find("disk_worker\n            .join()").unwrap()
                < service.find("progress_log.write_to").unwrap()
        );
    }

    #[test]
    fn options_default_and_bounds() {
        assert_eq!(parse_options(&[]).unwrap(), Options::default());
        let parsed = parse_options(&args("--channels 8 --rate 96000 --frames 128 --capture-bus cable-h --render-bus cable-c --stall-ms 500 --wav64")).unwrap();
        assert_eq!(
            (parsed.channels, parsed.rate, parsed.frames),
            (8, 96_000, 128)
        );
        assert_eq!(
            (parsed.capture_bus.as_str(), parsed.render_bus.as_str()),
            ("cable-h", "cable-c")
        );
        assert!(parsed.wav64 && parsed.stall_ms == 500);
        for bad in [
            "--channels 9",
            "--rate 22050",
            "--frames 4097",
            "--capture-bus cable-i",
            "--render-bus Cable-A",
            "--capture-bus cable-a",
            "--bogus",
            "--seconds",
        ] {
            assert!(parse_options(&args(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn tone_blocks_join_without_discontinuity_and_put_47_hz_on_odd_channels() {
        let mut first = vec![0.0; 2 * 480];
        let mut second = vec![0.0; 2 * 480];
        fill_tone_block(&mut first, 2, 0, 48_000);
        fill_tone_block(&mut second, 2, 480, 48_000);
        let mut whole = vec![0.0; 2 * 960];
        fill_tone_block(&mut whole, 2, 0, 48_000);
        assert_eq!([first, second].concat(), whole);
        // 47 Hz moves far less per sample than 997 Hz.
        let step = |channel: usize| (whole[2 + channel] - whole[channel]).abs();
        assert!(step(1) < step(0));
        assert!(whole.iter().all(|sample| sample.abs() <= 0.25));
    }

    #[test]
    fn wav_header_describes_ieee_float_data() {
        let header = wav_header(8, 96_000, 8, 1_000);
        assert_eq!(&header[0..4], b"RIFF");
        assert_eq!(u16::from_le_bytes([header[20], header[21]]), 3);
        assert_eq!(u16::from_le_bytes([header[22], header[23]]), 8);
        assert_eq!(
            u32::from_le_bytes(header[24..28].try_into().unwrap()),
            96_000
        );
        assert_eq!(u16::from_le_bytes([header[32], header[33]]), 64);
        assert_eq!(u16::from_le_bytes([header[34], header[35]]), 64);
        assert_eq!(
            u32::from_le_bytes(header[40..44].try_into().unwrap()),
            1_000
        );
    }

    #[test]
    fn wav_writer_patches_sizes_and_keeps_float32_samples_exact() {
        let path = std::env::temp_dir().join(format!("m03-bridge-tone-{}.wav", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut wav = WavWriter::create(&path, 2, 48_000, false).unwrap();
        let samples = [0.5_f64, -0.25, f64::from(0.1_f32), 0.0];
        wav.append(&samples).unwrap();
        assert_eq!(wav.finish(48_000).unwrap(), 2);
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 44 + 16);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 16);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 36 + 16);
        assert_eq!(
            f32::from_le_bytes(bytes[52..56].try_into().unwrap()),
            0.1_f32
        );
        std::fs::remove_file(&path).unwrap();
    }
}
