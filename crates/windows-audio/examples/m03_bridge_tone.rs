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
fn run(options: &Options) -> Result<(), (i32, String)> {
    use audiorouter_windows_audio::{
        classify_native_bridge_error, NativeBridgeControlClient, NativeBridgeController,
        NativeBridgeControllerError,
    };
    let explain = |error: NativeBridgeControllerError| -> (i32, String) {
        match error {
            NativeBridgeControllerError::Windows(windows) => {
                let kind = classify_native_bridge_error(&windows);
                (1, format!("{kind:?}: {} ({windows})", kind.user_message()))
            }
            other => (1, format!("{other:?}")),
        }
    };

    // QUERY first: refuse an incompatible driver before creating any lease.
    let client = NativeBridgeControlClient::open(&options.device).map_err(|error| {
        let kind = classify_native_bridge_error(&error);
        (1, format!("{kind:?}: {} ({error})", kind.user_message()))
    })?;
    let info = client
        .query()
        .map_err(|error| (1, format!("QUERY failed: {error}")))?;
    println!("driver: {info:?}");
    info.check_compatible()
        .map_err(|error| (2, format!("{error:?}: {}", error.user_message())))?;
    drop(client);

    // Generations must increase across runs while the driver stays loaded;
    // wall-clock milliseconds satisfy that without persisted state.
    let generation = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(1)
        .max(1);
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
    let temp = std::env::temp_dir();
    let capture_path = temp.join(format!(
        "audiorouter-bridge-tone-{}-capture.slot",
        std::process::id()
    ));
    let render_path = temp.join(format!(
        "audiorouter-bridge-tone-{}-render.slot",
        std::process::id()
    ));
    let mut render = NativeBridgeController::create_with_section(
        &options.device,
        &render_path,
        hello(
            &options.render_bus,
            audiorouter_protocol::AudioBridgeDirection::RenderSource,
        ),
    )
    .map_err(explain)?;
    let mut wav = WavWriter::create(&options.out, options.channels, options.rate, options.wav64)
        .map_err(|error| {
            (
                1,
                format!("cannot create {}: {error}", options.out.display()),
            )
        })?;

    let samples_per_block = usize::from(options.frames) * usize::from(options.channels);
    let mut tone = vec![0.0_f64; samples_per_block];
    let mut received = vec![0.0_f64; samples_per_block];
    let block_period =
        std::time::Duration::from_secs_f64(f64::from(options.frames) / f64::from(options.rate));
    let total_blocks =
        u64::from(options.seconds) * u64::from(options.rate) / u64::from(options.frames);
    // Complete file/buffer/render setup before exposing the capture lease.
    // Its first block is valid before OPEN; no startup counter is reset or
    // subtracted to compensate for an active empty mapping.
    fill_tone_block(&mut tone, options.channels, 0, options.rate);
    let mut capture = NativeBridgeController::create_with_section_primed(
        &options.device,
        &capture_path,
        hello(
            &options.capture_bus,
            audiorouter_protocol::AudioBridgeDirection::CaptureSink,
        ),
        &tone,
    )
    .map_err(explain)?;
    let start = std::time::Instant::now();
    let mut last_heartbeat = start;
    let mut last_render_sequence = 0_u64;
    let mut written_blocks = 1_u64;
    let mut read_blocks = 0_u64;
    let mut stall_offset = std::time::Duration::ZERO;
    println!(
        "writing {} Hz/{} Hz into {} capture sink, recording {} render source for {} s ({} ch, {} Hz, {}-frame blocks)",
        997, 47, options.capture_bus, options.render_bus, options.seconds,
        options.channels, options.rate, options.frames
    );
    // 1 ms scheduler resolution for this process only, so the 1 ms poll below
    // is not stretched to Windows' default 15.6 ms tick.
    // SAFETY: plain winmm call; matched by timeEndPeriod after the loop.
    unsafe {
        windows::Win32::Media::timeBeginPeriod(1);
    }
    let run_for = std::time::Duration::from_secs(u64::from(options.seconds));
    let stall_at = start + run_for / 2;
    let mut stalled = options.stall_ms == 0;
    let mut next_block = 1_u64;
    let mut last_written = 1_u64;
    let mut last_write_at = start;
    let mut last_ack = 0_u64;
    let mut last_ack_at = start;
    let mut last_progress = start;
    println!("lease-open capture={:?}", capture.counters());
    println!(
        "first-write (primed before OPEN) capture={:?}",
        capture.counters()
    );
    while start.elapsed() < run_for + stall_offset {
        let now = std::time::Instant::now();
        if !stalled && now >= stall_at {
            // Deliberate stall: neither produce nor consume, so the driver's
            // underrun/overrun counters must rise (WP-06 acceptance).
            println!("stalling for {} ms", options.stall_ms);
            let stall = std::time::Duration::from_millis(u64::from(options.stall_ms));
            std::thread::sleep(stall);
            stall_offset += stall;
            stalled = true;
        }
        // Flow control: while the driver is consuming (a recorder or
        // "Listen" is open on the capture endpoint) it acknowledges each block
        // it takes; publish the next one right after the acknowledgement, so
        // the tone runs on the driver's clock. With no consumer, pace by wall
        // clock (unread blocks are simply replaced and not counted).
        let ack = capture.consumer_sequence();
        if ack != last_ack {
            last_ack = ack;
            last_ack_at = now;
        }
        let consumer_active = last_ack != 0 && last_ack_at.elapsed() < block_period * 3;
        let due = if consumer_active {
            last_written == 0 || ack >= last_written
        } else {
            last_written == 0 || now.duration_since(last_write_at) >= block_period
        };
        if due && next_block < total_blocks {
            fill_tone_block(
                &mut tone,
                options.channels,
                next_block * u64::from(options.frames),
                options.rate,
            );
            last_written = capture.write_f64(&tone).map_err(explain)?;
            last_write_at = now;
            next_block += 1;
            written_blocks += 1;
        }
        // The render-source lease holds one block (the newest). Poll faster
        // than one period so every block is read once; one replaced before we
        // read it is counted by the driver as an overrun, not lost silently.
        if let Ok(header) = render.read_into_f64_after(last_render_sequence, &mut received) {
            let count = usize::from(header.frames) * usize::from(header.channels);
            wav.append(&received[..count])
                .map_err(|error| (1, format!("WAV write failed: {error}")))?;
            last_render_sequence = header.sequence;
            read_blocks += 1;
        }
        if last_heartbeat.elapsed() >= std::time::Duration::from_millis(250) {
            capture.heartbeat().map_err(explain)?;
            render.heartbeat().map_err(explain)?;
            last_heartbeat = std::time::Instant::now();
        }
        // Diagnostic work belongs to this user-mode tool, never the audio
        // callback. Retain raw cumulative counts; do not subtract startup
        // errors or hide them from the final zero-counter acceptance check.
        if last_progress.elapsed() >= std::time::Duration::from_secs(1) {
            println!(
                "progress {} ms: written={written_blocks} read={read_blocks} ack={} capture={:?} render={:?}",
                start.elapsed().as_millis(),
                capture.consumer_sequence(),
                capture.counters(),
                render.counters()
            );
            last_progress = std::time::Instant::now();
        }
        // Stop as soon as the driver has taken the final tone block: from
        // then on it would correctly count silence until the lease closes,
        // which is the end of the test, not a glitch.
        if next_block == total_blocks
            && consumer_active
            && capture.consumer_sequence() >= last_written
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let capture_counters = capture.counters();
    let render_counters = render.counters();
    // SAFETY: matches timeBeginPeriod(1) above.
    unsafe {
        windows::Win32::Media::timeEndPeriod(1);
    }
    let recorded_frames = wav
        .finish(options.rate)
        .map_err(|error| (1, format!("WAV finish failed: {error}")))?;
    println!("capture-sink blocks written: {written_blocks}");
    println!(
        "render-source blocks read: {read_blocks}; WAV frames: {recorded_frames} -> {}",
        options.out.display()
    );
    println!(
        "capture-sink counters ({}): {capture_counters:?}",
        options.capture_bus
    );
    println!(
        "render-source counters ({}): {render_counters:?}",
        options.render_bus
    );
    capture.close().map_err(explain)?;
    render.close().map_err(explain)?;
    Ok(())
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
