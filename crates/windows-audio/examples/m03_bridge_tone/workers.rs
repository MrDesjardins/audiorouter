//! Mapped audio workers. Control and disk work never run in these loops.
use super::{fill_tone_block, Options};
use audiorouter_windows_audio::{NativeBridgeRegion, NativeBridgeRegionError};
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
    pub stop: AtomicBool,
    pub done: AtomicBool,
    pub capture_blocks: AtomicU64,
    pub render_blocks: AtomicU64,
    pub render_gaps: AtomicU64,
    pub capture_gap_us: AtomicU64,
    pub render_gap_us: AtomicU64,
    pub disk_us: AtomicU64,
    pub disk_finish_us: AtomicU64,
    pub failed: AtomicBool,
    pub free: ArrayQueue<Packet>,
    pub recorded: ArrayQueue<Packet>,
    pub renderer_done: AtomicBool,
}

impl Shared {
    pub fn new(samples: usize, capacity: usize) -> Arc<Self> {
        let state = Arc::new(Self {
            start: OnceLock::new(),
            stop: AtomicBool::new(false),
            done: AtomicBool::new(false),
            capture_blocks: AtomicU64::new(1),
            render_blocks: AtomicU64::new(0),
            render_gaps: AtomicU64::new(0),
            capture_gap_us: AtomicU64::new(0),
            render_gap_us: AtomicU64::new(0),
            disk_us: AtomicU64::new(0),
            disk_finish_us: AtomicU64::new(0),
            failed: AtomicBool::new(false),
            free: ArrayQueue::new(capacity),
            recorded: ArrayQueue::new(capacity),
            renderer_done: AtomicBool::new(false),
        });
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
pub(super) fn prepare_thread() -> Result<(), String> {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST,
    };
    // SAFETY: the pseudo handle refers only to this thread; no ownership is
    // transferred. HIGHEST does not select the realtime process class.
    unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST) }
        .map_err(|error| format!("audio thread priority failed: {error}"))
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
            fill_tone_block(
                tone,
                options.channels,
                sequence * u64::from(options.frames),
                options.rate,
            );
            sequence = sequence
                .checked_add(1)
                .ok_or("capture sequence exhausted")?;
            region
                .write_f64(generation, sequence, tone)
                .map_err(|error| format!("capture mapping: {error:?}"))?;
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
        observe_max(&state.render_gap_us, now.duration_since(last));
        last = now;
        if !stalled && start.elapsed() >= Duration::from_secs(u64::from(options.seconds)) / 2 {
            std::thread::sleep(Duration::from_millis(u64::from(options.stall_ms)));
            stalled = true;
        }
        match region.read_into_f64_after(generation, sequence, &mut packet.samples) {
            Ok(header) => {
                state.render_gaps.fetch_add(
                    header.sequence.saturating_sub(sequence + 1),
                    Ordering::Relaxed,
                );
                if header.sequence != sequence + 1 && options.stall_ms == 0 {
                    return Err("render sequence gap in harness".to_owned());
                }
                sequence = header.sequence;
                packet.count = usize::from(header.frames) * usize::from(header.channels);
                state
                    .recorded
                    .push(packet)
                    .map_err(|_| "recording queue full".to_owned())?;
                state.render_blocks.fetch_add(1, Ordering::Release);
                packet = state
                    .free
                    .pop()
                    .ok_or("recording buffer pool exhausted; disk worker stalled")?;
            }
            Err(
                NativeBridgeRegionError::Empty
                | NativeBridgeRegionError::Busy
                | NativeBridgeRegionError::TornRead
                | NativeBridgeRegionError::SequenceRegression,
            ) => {
                if stopping {
                    break;
                }
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
                let result = render(&render_view, &state, &options, 1);
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
    fn recording_backpressure_is_an_explicit_worker_error() {
        let mapping = Mapping::new();
        let view = mapping.view();
        let state = Shared::new(960, 2);
        let options = Options::default();
        std::thread::scope(|scope| {
            let _stop = Stop(&state);
            let worker = scope.spawn(|| render(&view, &state, &options, 1));
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
            let worker = scope.spawn(|| render(&view, &state, &options, 1));
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
        assert!(render(&view, &state, &Options::default(), 1)
            .unwrap_err()
            .contains("StaleGeneration"));
        assert_eq!(state.render_blocks.load(Ordering::Acquire), 0);
    }
}
