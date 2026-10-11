//! Mapped audio workers. Control and disk work never run in these loops.
use super::{fill_tone_block, Options};
use audiorouter_windows_audio::{
    AudioServiceThreadCapabilities, AudioServiceThreadGuard, CableRouteProcessor,
    NativeBridgeRegion, NativeBridgeRegionError,
};
use crossbeam_queue::ArrayQueue;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, OnceLock,
};
use std::time::{Duration, Instant};

pub(super) struct Packet {
    pub samples: Vec<f64>,
    pub count: usize,
}

pub(super) struct PanicSignal<'a>(pub &'a Shared);

impl Drop for PanicSignal<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.failed.store(true, Ordering::Release);
        }
    }
}

pub(super) struct Shared {
    pub start: OnceLock<Instant>,
    pub retiring: AtomicBool,
    pub stop: AtomicBool,
    pub render_armed: AtomicBool,
    pub done: AtomicBool,
    pub capture_blocks: AtomicU64,
    pub render_blocks: AtomicU64,
    pub render_gaps: AtomicU64,
    pub capture_gap_us: AtomicU64,
    pub render_gap_us: AtomicU64,
    pub capture_interval_gap_us: AtomicU64,
    pub render_interval_gap_us: AtomicU64,
    pub disk_us: AtomicU64,
    pub disk_finish_us: AtomicU64,
    pub failed: AtomicBool,
    pub free: ArrayQueue<Packet>,
    pub recorded: ArrayQueue<Packet>,
    pub renderer_done: AtomicBool,
    // Diagnostic pass-through (`--passthrough`): render-source blocks are
    // republished into the capture sink, like an AudioRouter cable route.
    // Preallocated buffers; the audio workers never allocate.
    pub relay_free: ArrayQueue<Vec<f64>>,
    pub relay_ready: ArrayQueue<Vec<f64>>,
    pub relay_forwarded: AtomicU64,
    pub relay_silence: AtomicU64,
    pub relay_dropped: AtomicU64,
    // `--engine`: quanta the engine processed, and quanta it replaced with
    // silence (no output); published by the render worker.
    pub engine_processed: AtomicU64,
    pub engine_silent: AtomicU64,
}

/// Relay depth: enough to absorb one late capture poll, small enough to keep
/// the pass-through delay bounded (the oldest block is dropped beyond it).
pub(super) const RELAY_BLOCKS: usize = 3;

impl Shared {
    pub fn new(samples: usize, capacity: usize) -> Arc<Self> {
        let state = Arc::new(Self {
            start: OnceLock::new(),
            retiring: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            render_armed: AtomicBool::new(false),
            done: AtomicBool::new(false),
            capture_blocks: AtomicU64::new(1),
            render_blocks: AtomicU64::new(0),
            render_gaps: AtomicU64::new(0),
            capture_gap_us: AtomicU64::new(0),
            render_gap_us: AtomicU64::new(0),
            capture_interval_gap_us: AtomicU64::new(0),
            render_interval_gap_us: AtomicU64::new(0),
            disk_us: AtomicU64::new(0),
            disk_finish_us: AtomicU64::new(0),
            failed: AtomicBool::new(false),
            free: ArrayQueue::new(capacity),
            recorded: ArrayQueue::new(capacity),
            renderer_done: AtomicBool::new(false),
            relay_free: ArrayQueue::new(RELAY_BLOCKS),
            relay_ready: ArrayQueue::new(RELAY_BLOCKS),
            relay_forwarded: AtomicU64::new(0),
            relay_silence: AtomicU64::new(0),
            relay_dropped: AtomicU64::new(0),
            engine_processed: AtomicU64::new(0),
            engine_silent: AtomicU64::new(0),
        });
        for _ in 0..RELAY_BLOCKS {
            assert!(state.relay_free.push(vec![0.0; samples]).is_ok());
        }
        for _ in 0..capacity {
            assert!(state
                .free
                .push(Packet {
                    samples: vec![0.0; samples],
                    count: 0
                })
                .is_ok());
        }
        state
    }

    fn wait_start(&self) -> Option<Instant> {
        loop {
            if self.stop.load(Ordering::Acquire) {
                return None;
            }
            if let Some(start) = self.start.get() {
                return Some(*start);
            }
            std::thread::park_timeout(Duration::from_millis(1));
        }
    }
}

pub(super) fn observe_max(value: &AtomicU64, elapsed: Duration) {
    value.fetch_max(
        elapsed.as_micros().min(u128::from(u64::MAX)) as u64,
        Ordering::Relaxed,
    );
}

/// Render side of the pass-through: copy one received block into the relay,
/// dropping the oldest queued block when all relay buffers are in use.
pub(super) fn relay_forward(state: &Shared, samples: &[f64]) {
    let mut buffer = match state.relay_free.pop() {
        Some(buffer) => buffer,
        None => match state.relay_ready.pop() {
            Some(oldest) => {
                state.relay_dropped.fetch_add(1, Ordering::Relaxed);
                oldest
            }
            // Both queues are momentarily empty only while the capture worker
            // holds a buffer; count the block as dropped rather than block.
            None => {
                state.relay_dropped.fetch_add(1, Ordering::Relaxed);
                return;
            }
        },
    };
    let count = samples.len().min(buffer.len());
    buffer[..count].copy_from_slice(&samples[..count]);
    buffer[count..].fill(0.0);
    if let Err(buffer) = state.relay_ready.push(buffer) {
        let _ = state.relay_free.push(buffer);
        state.relay_dropped.fetch_add(1, Ordering::Relaxed);
    }
}

/// Capture side of the pass-through: the next block to publish, or silence
/// (counted) when nothing has arrived from the render source yet.
pub(super) fn relay_take(state: &Shared, block: &mut [f64]) {
    match state.relay_ready.pop() {
        Some(buffer) => {
            let count = block.len().min(buffer.len());
            block[..count].copy_from_slice(&buffer[..count]);
            block[count..].fill(0.0);
            let _ = state.relay_free.push(buffer);
            state.relay_forwarded.fetch_add(1, Ordering::Relaxed);
        }
        None => {
            block.fill(0.0);
            state.relay_silence.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn is_expected_mapping_retirement(retiring: bool, error: &NativeBridgeRegionError) -> bool {
    retiring && *error == NativeBridgeRegionError::SampleSizeMismatch
}

/// Disk thread loop, shared by the production writer and stalled-I/O tests.
pub(super) fn record(
    state: &Shared,
    mut append: impl FnMut(&[f64]) -> Result<(), String>,
) -> Result<(), String> {
    let mut failure = None;
    loop {
        if let Some(packet) = state.recorded.pop() {
            let before = Instant::now();
            if failure.is_none() {
                if let Err(error) = append(&packet.samples[..packet.count]) {
                    failure = Some(error);
                    state.failed.store(true, Ordering::Release);
                }
            }
            observe_max(&state.disk_us, before.elapsed());
            if state.free.push(packet).is_err() {
                failure = Some("recording buffer ownership violated".to_owned());
                state.failed.store(true, Ordering::Release);
            }
        } else if state.stop.load(Ordering::Acquire) && state.renderer_done.load(Ordering::Acquire)
        {
            break;
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug)]
pub(super) struct WorkerScheduling {
    pub capabilities: AudioServiceThreadCapabilities,
    pub highest_priority_fallback: bool,
}

#[cfg(windows)]
pub(super) fn prepare_thread() -> Result<(AudioServiceThreadGuard, WorkerScheduling), String> {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST,
    };
    let (guard, capabilities) = AudioServiceThreadGuard::enter();
    let highest_priority_fallback = !capabilities.mmcss_pro_audio;
    if highest_priority_fallback {
        // SAFETY: the pseudo handle refers only to this thread; no ownership
        // is transferred. HIGHEST does not select the realtime process class.
        unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST) }
            .map_err(|error| format!("audio thread fallback priority failed: {error}"))?;
    }
    Ok((
        guard,
        WorkerScheduling {
            capabilities,
            highest_priority_fallback,
        },
    ))
}

pub(super) fn capture(
    region: &NativeBridgeRegion,
    state: &Shared,
    options: &Options,
    generation: u64,
    tone: &mut [f64],
) -> Result<(), String> {
    let Some(start) = state.wait_start() else {
        return Ok(());
    };
    let mut last = Instant::now();
    let mut sequence = 1;
    let mut stalled = options.stall_ms == 0;
    let run_for = Duration::from_secs(u64::from(options.seconds));
    loop {
        let stopping = state.stop.load(Ordering::Acquire);
        let now = Instant::now();
        observe_max(&state.capture_gap_us, now.duration_since(last));
        observe_max(&state.capture_interval_gap_us, now.duration_since(last));
        last = now;
        if !stalled && start.elapsed() >= run_for / 2 {
            std::thread::sleep(Duration::from_millis(u64::from(options.stall_ms)));
            stalled = true;
        }
        if start.elapsed() >= run_for + Duration::from_millis(u64::from(options.stall_ms)) {
            state.done.store(true, Ordering::Release);
        }
        // Keep supplying audio until the control thread closes the lease.
        // Never overwrite an unacknowledged block when no recorder is open.
        if stopping {
            break;
        }
        if region.consumer_sequence() >= sequence {
            if options.passthrough {
                relay_take(state, tone);
            } else {
                fill_tone_block(
                    tone,
                    options.channels,
                    sequence * u64::from(options.frames),
                    options.rate,
                );
            }
            sequence = sequence
                .checked_add(1)
                .ok_or("capture sequence exhausted")?;
            match region.write_f64(generation, sequence, tone) {
                Ok(()) => {}
                Err(error)
                    if is_expected_mapping_retirement(
                        state.retiring.load(Ordering::Acquire),
                        &error,
                    ) =>
                {
                    break;
                }
                Err(error) => return Err(format!("capture mapping: {error:?}")),
            }
            state.capture_blocks.store(sequence, Ordering::Release);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

pub(super) fn render(
    region: &NativeBridgeRegion,
    state: &Shared,
    options: &Options,
    generation: u64,
    mut engine: Option<&mut CableRouteProcessor>,
) -> Result<(), String> {
    let mut packet = state
        .free
        .pop()
        .ok_or("recording buffer unavailable at startup")?;
    let Some(start) = state.wait_start() else {
        return Ok(());
    };
    let mut last = Instant::now();
    let mut sequence = 0;
    let mut stalled = options.stall_ms == 0;
    loop {
        let stopping = state.stop.load(Ordering::Acquire);
        let now = Instant::now();
        let poll_gap = now.duration_since(last);
        observe_max(&state.render_gap_us, poll_gap);
        observe_max(&state.render_interval_gap_us, poll_gap);
        last = now;
        if !stalled && start.elapsed() >= Duration::from_secs(u64::from(options.seconds)) / 2 {
            std::thread::sleep(Duration::from_millis(u64::from(options.stall_ms)));
            stalled = true;
        }
        match region.read_into_f64_after(generation, sequence, &mut packet.samples) {
            Ok(header) => {
                let expected_sequence = sequence.saturating_add(1);
                let skipped_sequences = header.sequence.saturating_sub(expected_sequence);
                state
                    .render_gaps
                    .fetch_add(skipped_sequences, Ordering::Relaxed);
                if skipped_sequences > 0 && options.stall_ms == 0 {
                    return Err(format!(
                        "render sequence gap: previous={sequence}, expected={expected_sequence}, observed={}, skipped={skipped_sequences}, elapsed_ms={}, poll_gap_us={}, max_poll_gap_us={}",
                        header.sequence,
                        start.elapsed().as_millis(),
                        poll_gap.as_micros(),
                        state.render_gap_us.load(Ordering::Relaxed),
                    ));
                }
                sequence = header.sequence;
                packet.count = usize::from(header.frames) * usize::from(header.channels);
                match engine.as_deref_mut() {
                    Some(route) => {
                        route.push_block(&packet.samples[..packet.count], |block| {
                            relay_forward(state, block)
                        });
                        state
                            .engine_processed
                            .store(route.processed_quanta(), Ordering::Release);
                        state
                            .engine_silent
                            .store(route.silent_quanta(), Ordering::Release);
                    }
                    None if options.passthrough => {
                        relay_forward(state, &packet.samples[..packet.count]);
                    }
                    None => {}
                }
                state
                    .recorded
                    .push(packet)
                    .map_err(|_| "recording queue full".to_owned())?;
                state.render_blocks.fetch_add(1, Ordering::Release);
                packet = state
                    .free
                    .pop()
                    .ok_or("recording buffer pool exhausted; disk worker stalled")?;
                // Drain a just-published successor immediately. Sleeping
                // here adds an avoidable blind interval when Windows
                // publishes adjacent render quanta in a burst.
                continue;
            }
            Err(
                NativeBridgeRegionError::Empty
                | NativeBridgeRegionError::Busy
                | NativeBridgeRegionError::TornRead
                | NativeBridgeRegionError::SequenceRegression,
            ) => {
                // The control thread waits for this first empty poll before
                // activating the render lease. That way the worker is already
                // watching the slot when Windows starts publishing blocks.
                state.render_armed.store(true, Ordering::Release);
                if stopping {
                    break;
                }
            }
            // CLOSE retires the driver's view while this read-only mapping
            // remains alive long enough to drain a final published block.
            // A cleared format header is therefore a normal stop condition.
            Err(error)
                if is_expected_mapping_retirement(
                    state.retiring.load(Ordering::Acquire),
                    &error,
                ) =>
            {
                break;
            }
            Err(error) => return Err(format!("render mapping: {error:?}")),
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::WavWriter;
    use super::*;
    use std::path::PathBuf;

    struct Mapping {
        path: PathBuf,
        region: Option<NativeBridgeRegion>,
    }

    impl Mapping {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "tone-worker-{}-{}.slot",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let region = NativeBridgeRegion::create(&path, 2, 480).unwrap();
            Self {
                path,
                region: Some(region),
            }
        }

        fn view(&self) -> NativeBridgeRegion {
            NativeBridgeRegion::open(&self.path, 2, 480).unwrap()
        }
    }

    impl Drop for Mapping {
        fn drop(&mut self) {
            self.region.take();
            std::fs::remove_file(&self.path).unwrap();
        }
    }

    fn until(mut predicate: impl FnMut() -> bool) {
        let start = Instant::now();
        while !predicate() {
            assert!(start.elapsed() < Duration::from_secs(5), "worker timeout");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    struct Stop<'a>(&'a Shared);
    impl Drop for Stop<'_> {
        fn drop(&mut self) {
            self.0.stop.store(true, Ordering::Release);
        }
    }

    #[test]
    fn mapped_audio_keeps_serving_during_blocked_recording_and_control_and_wav_is_ordered() {
        let capture_map = Mapping::new();
        let render_map = Mapping::new();
        let capture_view = capture_map.view();
        let render_view = render_map.view();
        let mut tone = vec![0.0; 960];
        fill_tone_block(&mut tone, 2, 0, 48_000);
        capture_map
            .region
            .as_ref()
            .unwrap()
            .write_f64(1, 1, &tone)
            .unwrap();
        let state = Shared::new(960, 64);
        let options = Options::default();
        let wav_path = capture_map.path.with_extension("wav");
        let mut wav = WavWriter::create(&wav_path, 2, 48_000, true).unwrap();
        let (blocked_tx, blocked_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
        std::thread::scope(|scope| {
            let _stop = Stop(&state);
            let capture = scope.spawn(|| capture(&capture_view, &state, &options, 1, &mut tone));
            let render = scope.spawn(|| {
                let result = render(&render_view, &state, &options, 1, None);
                state.renderer_done.store(true, Ordering::Release);
                result
            });
            let disk_state = &state;
            let writer = scope.spawn(move || {
                let mut first = true;
                record(disk_state, |samples| {
                    if first {
                        first = false;
                        blocked_tx.send(()).unwrap();
                        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                    }
                    wav.append(samples).map_err(|error| error.to_string())
                })
                .unwrap();
                wav.finish(48_000).unwrap()
            });
            state.start.set(Instant::now()).unwrap();
            // This is a mapped-slot peer, not a loaded kernel driver.
            let peer = scope.spawn(|| {
                let mut got = vec![0.0; 960];
                let mut expected = vec![0.0; 960];
                for sequence in 1..=30 {
                    until(|| {
                        capture_map
                            .region
                            .as_ref()
                            .unwrap()
                            .read_into_f64_after(1, sequence - 1, &mut got)
                            .is_ok()
                    });
                    fill_tone_block(&mut expected, 2, (sequence - 1) * 480, 48_000);
                    assert_eq!(got, expected);
                    render_map
                        .region
                        .as_ref()
                        .unwrap()
                        .write_f64(1, sequence, &vec![sequence as f64; 960])
                        .unwrap();
                    until(|| render_map.region.as_ref().unwrap().consumer_sequence() == sequence);
                    std::thread::sleep(Duration::from_millis(10));
                }
            });
            blocked_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            // Main represents a blocked heartbeat/console operation. The disk
            // thread is held independently, while both audio workers advance.
            std::thread::sleep(Duration::from_millis(160));
            assert!(state.capture_blocks.load(Ordering::Acquire) >= 10);
            assert!(state.render_blocks.load(Ordering::Acquire) >= 10);
            assert!(!state.failed.load(Ordering::Acquire));
            release_tx.send(()).unwrap();
            peer.join().unwrap();
            state.stop.store(true, Ordering::Release);
            capture.join().unwrap().unwrap();
            render.join().unwrap().unwrap();
            assert_eq!(writer.join().unwrap(), 30 * 480);
        });
        assert!(state.disk_us.load(Ordering::Relaxed) >= 150_000);
        let bytes = std::fs::read(&wav_path).unwrap();
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize,
            30 * 960 * 8
        );
        for (index, sample) in bytes[44..].chunks_exact(8).enumerate() {
            assert_eq!(
                f64::from_le_bytes(sample.try_into().unwrap()),
                (index / 960 + 1) as f64
            );
        }
        std::fs::remove_file(wav_path).unwrap();
    }

    #[test]
    fn relay_preserves_order_counts_silence_and_bounds_its_depth() {
        let state = Shared::new(4, 2);
        let mut block = [9.0; 4];
        relay_take(&state, &mut block);
        assert_eq!(block, [0.0; 4], "empty relay publishes silence");
        assert_eq!(state.relay_silence.load(Ordering::Relaxed), 1);
        for value in 1..=2 {
            relay_forward(&state, &[f64::from(value); 4]);
        }
        for value in 1..=2 {
            relay_take(&state, &mut block);
            assert_eq!(block, [f64::from(value); 4], "FIFO order");
        }
        assert_eq!(state.relay_forwarded.load(Ordering::Relaxed), 2);
        // More blocks than relay buffers: the oldest are dropped, never the
        // newest, so the pass-through delay stays bounded.
        for value in 1..=5 {
            relay_forward(&state, &[f64::from(value); 4]);
        }
        assert_eq!(state.relay_dropped.load(Ordering::Relaxed), 2);
        for value in 3..=5 {
            relay_take(&state, &mut block);
            assert_eq!(block, [f64::from(value); 4]);
        }
        relay_take(&state, &mut block);
        assert_eq!(block, [0.0; 4]);
        assert_eq!(
            state.relay_free.len(),
            RELAY_BLOCKS,
            "every buffer returned"
        );
        // A shorter source block is zero-padded, never mixed with stale data.
        relay_forward(&state, &[7.0; 2]);
        relay_take(&state, &mut block);
        assert_eq!(block, [7.0, 7.0, 0.0, 0.0]);
    }

    #[test]
    fn passthrough_republishes_render_source_blocks_into_the_capture_sink_in_order() {
        let capture_map = Mapping::new();
        let render_map = Mapping::new();
        let capture_view = capture_map.view();
        let render_view = render_map.view();
        // The tool primes the capture sink with silence in pass-through mode.
        let mut block = vec![0.0; 960];
        capture_map
            .region
            .as_ref()
            .unwrap()
            .write_f64(1, 1, &block)
            .unwrap();
        let state = Shared::new(960, 64);
        let options = Options {
            passthrough: true,
            ..Options::default()
        };
        std::thread::scope(|scope| {
            let _stop = Stop(&state);
            let capture = scope.spawn(|| capture(&capture_view, &state, &options, 1, &mut block));
            let render = scope.spawn(|| {
                let result = render(&render_view, &state, &options, 1, None);
                state.renderer_done.store(true, Ordering::Release);
                result
            });
            let disk = scope.spawn(|| record(&state, |_| Ok(())));
            state.start.set(Instant::now()).unwrap();
            // Peer stands in for the driver: it publishes render-source block
            // n (value n) and then consumes capture-sink blocks, acknowledging
            // each one as the capture callback would.
            let mut received = Vec::new();
            let mut got = vec![0.0; 960];
            let mut capture_sequence = 0;
            for sequence in 1..=40u64 {
                render_map
                    .region
                    .as_ref()
                    .unwrap()
                    .write_f64(1, sequence, &vec![sequence as f64; 960])
                    .unwrap();
                until(|| render_map.region.as_ref().unwrap().consumer_sequence() == sequence);
                until(|| {
                    capture_map
                        .region
                        .as_ref()
                        .unwrap()
                        .read_into_f64_after(1, capture_sequence, &mut got)
                        .map(|header| capture_sequence = header.sequence)
                        .is_ok()
                });
                assert!(
                    got.iter().all(|sample| *sample == got[0]),
                    "whole block, no mixing"
                );
                received.push(got[0]);
            }
            state.stop.store(true, Ordering::Release);
            capture.join().unwrap().unwrap();
            render.join().unwrap().unwrap();
            disk.join().unwrap().unwrap();
            // After the primed/silent start, the capture sink carries exactly
            // the render-source blocks, in order, each once.
            let forwarded: Vec<f64> = received.into_iter().filter(|value| *value != 0.0).collect();
            assert!(forwarded.len() >= 35, "{forwarded:?}");
            for pair in forwarded.windows(2) {
                assert_eq!(pair[1], pair[0] + 1.0, "{forwarded:?}");
            }
        });
        assert_eq!(state.relay_dropped.load(Ordering::Relaxed), 0);
        assert!(state.relay_forwarded.load(Ordering::Relaxed) >= 35);
    }

    #[test]
    fn engine_route_republishes_a_continuous_unchanged_stream() {
        let capture_map = Mapping::new();
        let render_map = Mapping::new();
        let capture_view = capture_map.view();
        let render_view = render_map.view();
        let mut block = vec![0.0; 960];
        capture_map
            .region
            .as_ref()
            .unwrap()
            .write_f64(1, 1, &block)
            .unwrap();
        let state = Shared::new(960, 64);
        let options = Options {
            passthrough: true,
            engine: true,
            ..Options::default()
        };
        let graph = audiorouter_engine::compile_session_at_sample_rate(
            &audiorouter_windows_audio::cable_route_session("cable-a", "cable-b", 2),
            audiorouter_engine::RuntimeGeneration::new(1),
            48_000,
        )
        .unwrap();
        let mut route = CableRouteProcessor::new(2, 480, graph).unwrap();
        // Exact float32 ramp across block boundaries: re-blocking must keep
        // every sample, in order, unchanged.
        let step = 1.0 / f64::from(1u32 << 23);
        let mut received = Vec::new();
        std::thread::scope(|scope| {
            let _stop = Stop(&state);
            let capture = scope.spawn(|| capture(&capture_view, &state, &options, 1, &mut block));
            let render = scope.spawn(|| {
                let result = render(&render_view, &state, &options, 1, Some(&mut route));
                state.renderer_done.store(true, Ordering::Release);
                result
            });
            let disk = scope.spawn(|| record(&state, |_| Ok(())));
            state.start.set(Instant::now()).unwrap();
            let mut got = vec![0.0; 960];
            let mut capture_sequence = 0;
            for sequence in 1..=40u64 {
                let first = (sequence - 1) as f64 * 960.0;
                let samples: Vec<f64> = (0..960)
                    .map(|index| (first + index as f64 + 1.0) * step)
                    .collect();
                render_map
                    .region
                    .as_ref()
                    .unwrap()
                    .write_f64(1, sequence, &samples)
                    .unwrap();
                until(|| render_map.region.as_ref().unwrap().consumer_sequence() == sequence);
                until(|| {
                    capture_map
                        .region
                        .as_ref()
                        .unwrap()
                        .read_into_f64_after(1, capture_sequence, &mut got)
                        .map(|header| capture_sequence = header.sequence)
                        .is_ok()
                });
                // Whole silent blocks are relay gaps while the engine fills.
                if got.iter().any(|sample| *sample != 0.0) {
                    received.extend_from_slice(&got);
                }
            }
            state.stop.store(true, Ordering::Release);
            capture.join().unwrap().unwrap();
            render.join().unwrap().unwrap();
            disk.join().unwrap().unwrap();
        });
        // The route starts with its constant re-blocking delay of silence.
        let delay = audiorouter_windows_audio::reblock_delay_frames(480) * 2;
        assert!(received[..delay].iter().all(|sample| *sample == 0.0));
        let received = &received[delay..];
        assert!(received.len() >= 30 * 960, "{} samples", received.len());
        for (index, sample) in received.iter().enumerate() {
            assert_eq!(*sample, (index as f64 + 1.0) * step, "sample {index}");
        }
        assert_eq!(state.relay_dropped.load(Ordering::Relaxed), 0);
        assert_eq!(state.engine_silent.load(Ordering::Relaxed), 0);
        assert_eq!(
            state.engine_processed.load(Ordering::Relaxed),
            40 * 480 / 128
        );
    }

    #[test]
    fn recording_backpressure_is_an_explicit_worker_error() {
        let mapping = Mapping::new();
        let view = mapping.view();
        let state = Shared::new(960, 2);
        let options = Options::default();
        std::thread::scope(|scope| {
            let _stop = Stop(&state);
            let worker = scope.spawn(|| render(&view, &state, &options, 1, None));
            state.start.set(Instant::now()).unwrap();
            for sequence in 1..=2 {
                mapping
                    .region
                    .as_ref()
                    .unwrap()
                    .write_f64(1, sequence, &[0.5; 960])
                    .unwrap();
                until(|| mapping.region.as_ref().unwrap().consumer_sequence() == sequence);
            }
            assert!(worker
                .join()
                .unwrap()
                .unwrap_err()
                .contains("pool exhausted"));
            assert_eq!(state.recorded.len(), 2);
        });
    }

    #[test]
    fn worker_waiting_for_activation_can_be_cancelled_without_publishing() {
        let mapping = Mapping::new();
        let view = mapping.view();
        let state = Shared::new(960, 2);
        let options = Options::default();
        let mut tone = vec![0.0; 960];
        std::thread::scope(|scope| {
            let _stop = Stop(&state);
            let capture = scope.spawn(|| capture(&view, &state, &options, 1, &mut tone));
            state.stop.store(true, Ordering::Release);
            capture.join().unwrap().unwrap();
        });
        assert_eq!(state.capture_blocks.load(Ordering::Acquire), 1);
        assert!(matches!(
            view.read_into_f64_after(1, 0, &mut tone),
            Err(NativeBridgeRegionError::Empty)
        ));
    }

    #[test]
    fn true_producer_stall_remains_visible_and_does_not_replay_samples() {
        let mapping = Mapping::new();
        let view = mapping.view();
        let state = Shared::new(960, 2);
        let options = Options {
            seconds: 1,
            stall_ms: 120,
            ..Options::default()
        };
        let mut tone = vec![0.0; 960];
        fill_tone_block(&mut tone, 2, 0, 48_000);
        mapping
            .region
            .as_ref()
            .unwrap()
            .write_f64(1, 1, &tone)
            .unwrap();
        std::thread::scope(|scope| {
            let _stop = Stop(&state);
            let worker = scope.spawn(|| capture(&view, &state, &options, 1, &mut tone));
            state.start.set(Instant::now()).unwrap();
            let mut got = vec![0.0; 960];
            let mut sequence = 0;
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(800) {
                if let Ok(header) = mapping
                    .region
                    .as_ref()
                    .unwrap()
                    .read_into_f64_after(1, sequence, &mut got)
                {
                    assert_eq!(header.sequence, sequence + 1);
                    sequence = header.sequence;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            state.stop.store(true, Ordering::Release);
            worker.join().unwrap().unwrap();
            assert!(sequence > 20);
        });
        assert!(state.capture_gap_us.load(Ordering::Relaxed) >= 120_000);
    }

    #[test]
    fn interval_gap_report_resets_without_erasing_lifetime_peak() {
        let state = Shared::new(960, 2);
        observe_max(&state.capture_gap_us, Duration::from_millis(163));
        observe_max(&state.capture_interval_gap_us, Duration::from_millis(163));
        assert_eq!(
            state.capture_interval_gap_us.swap(0, Ordering::Relaxed),
            163_000
        );
        observe_max(&state.capture_gap_us, Duration::from_millis(2));
        observe_max(&state.capture_interval_gap_us, Duration::from_millis(2));
        assert_eq!(
            state.capture_interval_gap_us.swap(0, Ordering::Relaxed),
            2_000
        );
        assert_eq!(state.capture_gap_us.load(Ordering::Relaxed), 163_000);
    }

    #[test]
    fn disk_failure_is_reported_and_buffers_are_returned() {
        let state = Shared::new(960, 2);
        let mut packet = state.free.pop().unwrap();
        packet.count = 960;
        assert!(state.recorded.push(packet).is_ok());
        state.stop.store(true, Ordering::Release);
        state.renderer_done.store(true, Ordering::Release);
        assert_eq!(
            record(&state, |_| Err("disk full".to_owned())).unwrap_err(),
            "disk full"
        );
        assert!(state.failed.load(Ordering::Acquire));
        assert_eq!(state.free.len(), 2);
    }

    #[test]
    fn render_drains_the_final_published_block_after_lease_shutdown() {
        let mapping = Mapping::new();
        let view = mapping.view();
        let state = Shared::new(960, 4);
        let options = Options::default();
        std::thread::scope(|scope| {
            let _stop = Stop(&state);
            let worker = scope.spawn(|| render(&view, &state, &options, 1, None));
            state.start.set(Instant::now()).unwrap();
            mapping
                .region
                .as_ref()
                .unwrap()
                .write_f64(1, 1, &[0.25; 960])
                .unwrap();
            until(|| mapping.region.as_ref().unwrap().consumer_sequence() == 1);
            // A retired kernel mapping has a stable final publication.
            mapping
                .region
                .as_ref()
                .unwrap()
                .write_f64(1, 2, &[0.5; 960])
                .unwrap();
            state.stop.store(true, Ordering::Release);
            worker.join().unwrap().unwrap();
        });
        assert_eq!(state.recorded.len(), 2);
        assert_eq!(state.recorded.pop().unwrap().samples[0], 0.25);
        assert_eq!(state.recorded.pop().unwrap().samples[0], 0.5);
        assert_eq!(state.render_gaps.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn render_worker_polls_empty_slot_before_first_published_block() {
        let mapping = Mapping::new();
        let view = mapping.view();
        let state = Shared::new(960, 4);
        let options = Options::default();
        std::thread::scope(|scope| {
            let _stop = Stop(&state);
            let worker = scope.spawn(|| render(&view, &state, &options, 1, None));
            state.start.set(Instant::now()).unwrap();
            until(|| state.render_armed.load(Ordering::Acquire));
            assert_eq!(state.render_blocks.load(Ordering::Acquire), 0);

            mapping
                .region
                .as_ref()
                .unwrap()
                .write_f64(1, 1, &[0.25; 960])
                .unwrap();
            until(|| state.render_blocks.load(Ordering::Acquire) == 1);
            mapping
                .region
                .as_ref()
                .unwrap()
                .write_f64(1, 2, &[0.5; 960])
                .unwrap();
            until(|| state.render_blocks.load(Ordering::Acquire) == 2);
            state.stop.store(true, Ordering::Release);
            worker.join().unwrap().unwrap();
        });
        assert_eq!(state.render_blocks.load(Ordering::Acquire), 2);
        assert_eq!(state.render_gaps.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn malformed_generation_is_not_treated_as_an_empty_slot() {
        let mapping = Mapping::new();
        let view = mapping.view();
        let state = Shared::new(960, 2);
        mapping
            .region
            .as_ref()
            .unwrap()
            .write_f64(2, 1, &[0.25; 960])
            .unwrap();
        state.start.set(Instant::now()).unwrap();
        assert!(render(&view, &state, &Options::default(), 1, None)
            .unwrap_err()
            .contains("StaleGeneration"));
        assert_eq!(state.render_blocks.load(Ordering::Acquire), 0);
    }

    #[test]
    fn render_sequence_gap_reports_sequence_and_poll_timing() {
        let mapping = Mapping::new();
        let view = mapping.view();
        let state = Shared::new(960, 2);
        let options = Options::default();
        mapping
            .region
            .as_ref()
            .unwrap()
            .write_f64(1, 2, &[0.25; 960])
            .unwrap();
        state.start.set(Instant::now()).unwrap();

        let error = render(&view, &state, &options, 1, None).unwrap_err();

        assert!(error.contains("previous=0"));
        assert!(error.contains("expected=1"));
        assert!(error.contains("observed=2"));
        assert!(error.contains("skipped=1"));
        assert!(error.contains("elapsed_ms="));
        assert!(error.contains("poll_gap_us="));
        assert!(error.contains("max_poll_gap_us="));
        assert_eq!(state.render_gaps.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn cleared_format_header_is_accepted_only_after_retirement_begins() {
        let error = NativeBridgeRegionError::SampleSizeMismatch;
        assert!(is_expected_mapping_retirement(true, &error));
        assert!(!is_expected_mapping_retirement(false, &error));
        assert!(!is_expected_mapping_retirement(
            true,
            &NativeBridgeRegionError::StaleGeneration
        ));
        assert!(!is_expected_mapping_retirement(
            false,
            &NativeBridgeRegionError::StaleGeneration
        ));
    }
}
