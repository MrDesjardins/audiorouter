//! Realtime cable → engine → cable route (WP-09, spec 17 §7.3).
//!
//! [`CableRouteWorker`] moves audio from one cable's render-source bridge
//! (what applications play into "Cable A Input") through a compiled
//! [`CableRouteProcessor`] graph into another cable's capture-sink bridge
//! (what applications record from "Cable B Output"). No physical endpoint
//! paces it: both sides follow the driver's shared clock, so the route needs
//! neither drift correction nor resampling.
//!
//! Two realtime threads, the shape the VM-qualified diagnostic route
//! (`m03_bridge_tone --engine`) established:
//! - the render thread drains every published render-source block, runs the
//!   engine, and forwards each output block into a three-block relay;
//! - the capture thread publishes the next relay block only after the
//!   reader acknowledged the previous one, so an unread block is never
//!   overwritten. When nothing is ready it publishes silence (counted).
//!
//! The relay drops its oldest block when full and sheds a block left
//! standing after a silence insert ([`StandingDelayTrim`]), so the route
//! returns to its normal delay after a disturbance (latency recovery).
//! Both loops use preallocated buffers and lock-free queues: no allocation,
//! lock, log or wait other than a 1 ms sleep. Lease activation, heartbeats
//! and deactivation belong to the caller's control thread.

use crate::cable_route::{CableRouteProcessor, StandingDelayTrim, STANDING_DELAY_WINDOW_BLOCKS};
use crate::{AudioServiceThreadGuard, NativeBridgeRegion, NativeBridgeRegionError};
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Relay depth: absorbs one late capture tick while keeping the route's
/// delay bounded (the oldest block is dropped beyond it).
pub const CABLE_ROUTE_RELAY_BLOCKS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CableRouteWorkerError {
    /// Block size of zero samples.
    InvalidBlock,
    /// A worker thread could not be started.
    Spawn(String),
    /// A worker failed (mapping error or panic); the message is the first
    /// failure recorded.
    Worker(String),
}

/// Counter snapshot; every value only grows while the route runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CableRouteCounters {
    /// Render-source blocks read.
    pub render_blocks: u64,
    /// Render-source sequence numbers skipped between two reads.
    pub render_sequence_gaps: u64,
    /// Capture-sink blocks published after the primed first block.
    pub capture_blocks: u64,
    /// Relay blocks published to the capture sink.
    pub forwarded: u64,
    /// Silence blocks published because no relay block was ready.
    pub silence: u64,
    /// Relay blocks dropped because the relay was full.
    pub dropped: u64,
    /// Relay blocks dropped by latency recovery.
    pub trimmed: u64,
    pub engine_processed: u64,
    pub engine_silent: u64,
}

struct Shared {
    stop: AtomicBool,
    retiring: AtomicBool,
    armed: AtomicBool,
    failed: AtomicBool,
    failure: OnceLock<String>,
    relay_free: ArrayQueue<Vec<f64>>,
    relay_ready: ArrayQueue<Vec<f64>>,
    render_blocks: AtomicU64,
    render_sequence_gaps: AtomicU64,
    capture_blocks: AtomicU64,
    forwarded: AtomicU64,
    silence: AtomicU64,
    dropped: AtomicU64,
    trimmed: AtomicU64,
    engine_processed: AtomicU64,
    engine_silent: AtomicU64,
}

impl Shared {
    fn fail(&self, message: String) {
        let _ = self.failure.set(message);
        self.failed.store(true, Ordering::Release);
    }

    /// Render side: copy one engine output block into the relay, dropping the
    /// oldest queued block when every relay buffer is in use.
    fn relay_forward(&self, samples: &[f64]) {
        let mut buffer = match self.relay_free.pop() {
            Some(buffer) => buffer,
            None => match self.relay_ready.pop() {
                Some(oldest) => {
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                    oldest
                }
                // Both queues are momentarily empty only while the capture
                // thread holds a buffer; count the block rather than wait.
                None => {
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            },
        };
        let count = samples.len().min(buffer.len());
        buffer[..count].copy_from_slice(&samples[..count]);
        buffer[count..].fill(0.0);
        if let Err(buffer) = self.relay_ready.push(buffer) {
            let _ = self.relay_free.push(buffer);
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Capture side: the next block to publish, or counted silence, then
    /// latency recovery on what is left queued.
    fn relay_take(&self, trim: &mut StandingDelayTrim, block: &mut [f64]) {
        match self.relay_ready.pop() {
            Some(buffer) => {
                let count = block.len().min(buffer.len());
                block[..count].copy_from_slice(&buffer[..count]);
                block[count..].fill(0.0);
                let _ = self.relay_free.push(buffer);
                self.forwarded.fetch_add(1, Ordering::Relaxed);
            }
            None => {
                block.fill(0.0);
                self.silence.fetch_add(1, Ordering::Relaxed);
            }
        }
        if trim.after_take(self.relay_ready.len()) {
            if let Some(oldest) = self.relay_ready.pop() {
                let _ = self.relay_free.push(oldest);
                self.trimmed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

/// Generations and block shape of one route. The capture sink must already
/// hold its primed first block (sequence 1), as
/// `NativeBridgeSession::create_primed` writes it.
#[derive(Debug, Clone, Copy)]
pub struct CableRouteWorkerConfig {
    pub render_generation: u64,
    pub capture_generation: u64,
    /// Interleaved samples per bridge block (frames × channels).
    pub block_samples: usize,
}

/// Running cable route. Dropping it stops and joins both threads.
pub struct CableRouteWorker {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
}

impl CableRouteWorker {
    /// Start both threads over already opened mapped views. They poll before
    /// any lease is active; activate the render-source lease after
    /// [`Self::wait_armed`] so the first published block is seen.
    pub fn start(
        render_source: NativeBridgeRegion,
        capture_sink: NativeBridgeRegion,
        processor: CableRouteProcessor,
        config: CableRouteWorkerConfig,
    ) -> Result<Self, CableRouteWorkerError> {
        if config.block_samples == 0 {
            return Err(CableRouteWorkerError::InvalidBlock);
        }
        let relay_free = ArrayQueue::new(CABLE_ROUTE_RELAY_BLOCKS);
        for _ in 0..CABLE_ROUTE_RELAY_BLOCKS {
            let _ = relay_free.push(vec![0.0; config.block_samples]);
        }
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            retiring: AtomicBool::new(false),
            armed: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            failure: OnceLock::new(),
            relay_free,
            relay_ready: ArrayQueue::new(CABLE_ROUTE_RELAY_BLOCKS),
            render_blocks: AtomicU64::new(0),
            render_sequence_gaps: AtomicU64::new(0),
            capture_blocks: AtomicU64::new(0),
            forwarded: AtomicU64::new(0),
            silence: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            trimmed: AtomicU64::new(0),
            engine_processed: AtomicU64::new(0),
            engine_silent: AtomicU64::new(0),
        });
        let mut worker = Self {
            shared: Arc::clone(&shared),
            threads: Vec::with_capacity(2),
        };
        let capture_shared = Arc::clone(&shared);
        let capture_buffer = vec![0.0; config.block_samples];
        worker.threads.push(
            std::thread::Builder::new()
                .name("cable-route-capture".to_owned())
                .spawn(move || {
                    let _guard = AudioServiceThreadGuard::enter();
                    guarded(&capture_shared, || {
                        capture_loop(
                            &capture_sink,
                            &capture_shared,
                            config.capture_generation,
                            capture_buffer,
                        )
                    });
                })
                .map_err(|error| CableRouteWorkerError::Spawn(error.to_string()))?,
        );
        let render_shared = Arc::clone(&shared);
        let render_buffer = vec![0.0; config.block_samples];
        worker.threads.push(
            std::thread::Builder::new()
                .name("cable-route-render".to_owned())
                .spawn(move || {
                    let _guard = AudioServiceThreadGuard::enter();
                    guarded(&render_shared, || {
                        render_loop(
                            &render_source,
                            &render_shared,
                            config.render_generation,
                            processor,
                            render_buffer,
                        )
                    });
                })
                .map_err(|error| CableRouteWorkerError::Spawn(error.to_string()))?,
        );
        Ok(worker)
    }

    /// Wait until the render thread has polled the empty render-source slot
    /// once. Returns false on timeout or worker failure.
    pub fn wait_armed(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while !self.shared.armed.load(Ordering::Acquire) {
            if self.shared.failed.load(Ordering::Acquire) || Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        true
    }

    /// Call before deactivating either lease: deactivation clears the
    /// driver's mapping header, which the threads then treat as a normal
    /// stop instead of a failure.
    pub fn begin_retire(&self) {
        self.shared.retiring.store(true, Ordering::Release);
    }

    /// The first worker failure, if any. A failed route has stopped
    /// publishing; the caller tears it down (protected paths stay silent).
    pub fn failure(&self) -> Option<&str> {
        self.shared
            .failed
            .load(Ordering::Acquire)
            .then(|| self.shared.failure.get().map(String::as_str))
            .flatten()
    }

    pub fn counters(&self) -> CableRouteCounters {
        let s = &self.shared;
        CableRouteCounters {
            render_blocks: s.render_blocks.load(Ordering::Acquire),
            render_sequence_gaps: s.render_sequence_gaps.load(Ordering::Acquire),
            capture_blocks: s.capture_blocks.load(Ordering::Acquire),
            forwarded: s.forwarded.load(Ordering::Acquire),
            silence: s.silence.load(Ordering::Acquire),
            dropped: s.dropped.load(Ordering::Acquire),
            trimmed: s.trimmed.load(Ordering::Acquire),
            engine_processed: s.engine_processed.load(Ordering::Acquire),
            engine_silent: s.engine_silent.load(Ordering::Acquire),
        }
    }

    /// Stop and join both threads; returns the final counters, or the first
    /// worker failure.
    pub fn stop(mut self) -> Result<CableRouteCounters, CableRouteWorkerError> {
        self.join();
        let counters = self.counters();
        match self.failure() {
            Some(message) => Err(CableRouteWorkerError::Worker(message.to_owned())),
            None => Ok(counters),
        }
    }

    fn join(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        for thread in self.threads.drain(..) {
            if thread.join().is_err() {
                self.shared.fail("cable route thread panicked".to_owned());
            }
        }
    }
}

impl Drop for CableRouteWorker {
    fn drop(&mut self) {
        self.join();
    }
}

/// Run one loop and record its error; a panic is caught by `join`.
fn guarded(shared: &Shared, body: impl FnOnce() -> Result<(), String>) {
    if let Err(error) = body() {
        shared.fail(error);
    }
}

fn is_retirement(shared: &Shared, error: &NativeBridgeRegionError) -> bool {
    shared.retiring.load(Ordering::Acquire) && *error == NativeBridgeRegionError::SampleSizeMismatch
}

fn render_loop(
    region: &NativeBridgeRegion,
    shared: &Shared,
    generation: u64,
    mut processor: CableRouteProcessor,
    mut block: Vec<f64>,
) -> Result<(), String> {
    let mut sequence = 0_u64;
    loop {
        if shared.stop.load(Ordering::Acquire) || shared.failed.load(Ordering::Acquire) {
            return Ok(());
        }
        match region.read_into_f64_after(generation, sequence, &mut block) {
            Ok(header) => {
                let gap = header.sequence.saturating_sub(sequence.saturating_add(1));
                shared
                    .render_sequence_gaps
                    .fetch_add(gap, Ordering::Relaxed);
                sequence = header.sequence;
                let count =
                    (usize::from(header.frames) * usize::from(header.channels)).min(block.len());
                processor.push_block(&block[..count], |output| shared.relay_forward(output));
                shared
                    .engine_processed
                    .store(processor.processed_quanta(), Ordering::Release);
                shared
                    .engine_silent
                    .store(processor.silent_quanta(), Ordering::Release);
                shared.render_blocks.fetch_add(1, Ordering::Release);
                // Drain a just-published successor at once: Windows can
                // publish adjacent quanta in a burst.
                continue;
            }
            Err(
                NativeBridgeRegionError::Empty
                | NativeBridgeRegionError::Busy
                | NativeBridgeRegionError::TornRead
                | NativeBridgeRegionError::SequenceRegression
                | NativeBridgeRegionError::StaleGeneration,
            ) => shared.armed.store(true, Ordering::Release),
            Err(error) if is_retirement(shared, &error) => return Ok(()),
            Err(error) => return Err(format!("cable route render source: {error:?}")),
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn capture_loop(
    region: &NativeBridgeRegion,
    shared: &Shared,
    generation: u64,
    mut block: Vec<f64>,
) -> Result<(), String> {
    // Sequence 1 is the primed block written before the lease opened.
    let mut sequence = 1_u64;
    let mut trim = StandingDelayTrim::new(STANDING_DELAY_WINDOW_BLOCKS);
    loop {
        if shared.stop.load(Ordering::Acquire) || shared.failed.load(Ordering::Acquire) {
            return Ok(());
        }
        // Never overwrite a block the reader has not acknowledged.
        if region.consumer_sequence() >= sequence {
            shared.relay_take(&mut trim, &mut block);
            sequence = sequence
                .checked_add(1)
                .ok_or_else(|| "cable route capture sequence exhausted".to_owned())?;
            match region.write_f64(generation, sequence, &block) {
                Ok(()) => {
                    shared.capture_blocks.fetch_add(1, Ordering::Release);
                }
                Err(error) if is_retirement(shared, &error) => return Ok(()),
                Err(error) => return Err(format!("cable route capture sink: {error:?}")),
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cable_route::{cable_route_session, reblock_delay_frames};
    use audiorouter_engine::{compile_session_at_sample_rate, RuntimeGeneration};
    use std::path::PathBuf;

    const FRAMES: u16 = 480;
    const CHANNELS: u16 = 2;
    const BLOCK: usize = FRAMES as usize * CHANNELS as usize;

    /// A temporary bridge mapping; the test side stands in for the driver.
    struct Mapping {
        path: PathBuf,
        region: Option<NativeBridgeRegion>,
    }

    impl Mapping {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "cable-route-worker-{}-{}.slot",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let region = NativeBridgeRegion::create(&path, CHANNELS, FRAMES).unwrap();
            Self {
                path,
                region: Some(region),
            }
        }

        fn peer(&self) -> &NativeBridgeRegion {
            self.region.as_ref().unwrap()
        }

        fn view(&self) -> NativeBridgeRegion {
            NativeBridgeRegion::open(&self.path, CHANNELS, FRAMES).unwrap()
        }
    }

    impl Drop for Mapping {
        fn drop(&mut self) {
            self.region.take();
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn processor() -> CableRouteProcessor {
        let graph = compile_session_at_sample_rate(
            &cable_route_session("cable-a", "cable-b", CHANNELS as u8),
            RuntimeGeneration::new(1),
            48_000,
        )
        .unwrap();
        CableRouteProcessor::new(usize::from(CHANNELS), usize::from(FRAMES), graph).unwrap()
    }

    fn until(mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !condition() {
            assert!(Instant::now() < deadline, "timed out");
            std::thread::sleep(Duration::from_micros(200));
        }
    }

    fn start(render: &Mapping, capture: &Mapping) -> CableRouteWorker {
        // The capture sink is primed with silence before its lease opens.
        capture.peer().write_f64(2, 1, &[0.0; BLOCK]).unwrap();
        let worker = CableRouteWorker::start(
            render.view(),
            capture.view(),
            processor(),
            CableRouteWorkerConfig {
                render_generation: 1,
                capture_generation: 2,
                block_samples: BLOCK,
            },
        )
        .unwrap();
        assert!(worker.wait_armed(Duration::from_secs(5)));
        worker
    }

    /// Distinct float32-grid noise (exactly representable; 2^-24 steps).
    fn noise(block: u64) -> Vec<f64> {
        (0..BLOCK as u64)
            .map(|i| {
                let x = (block * BLOCK as u64 + i).wrapping_mul(2_654_435_761) % (1 << 24);
                (x as f64 - (1 << 23) as f64) / (1 << 24) as f64
            })
            .collect()
    }

    #[test]
    fn cable_route_worker_carries_audio_bit_exact_in_order() {
        let (render, capture) = (Mapping::new(), Mapping::new());
        let worker = start(&render, &capture);
        let mut received = Vec::new();
        let mut got = vec![0.0; BLOCK];
        let mut capture_sequence = 0;
        let blocks = 60_u64;
        for block in 1..=blocks {
            render.peer().write_f64(1, block, &noise(block)).unwrap();
            until(|| render.peer().consumer_sequence() == block);
            until(|| {
                capture
                    .peer()
                    .read_into_f64_after(2, capture_sequence, &mut got)
                    .map(|header| capture_sequence = header.sequence)
                    .is_ok()
            });
            received.push(got.clone());
        }
        // Drain what the route still holds.
        for _ in 0..4 {
            until(|| {
                capture
                    .peer()
                    .read_into_f64_after(2, capture_sequence, &mut got)
                    .map(|header| capture_sequence = header.sequence)
                    .is_ok()
            });
            received.push(got.clone());
        }
        let counters = worker.stop().unwrap();
        // Silence blocks (primed, or nothing ready yet) carry no audio; the
        // rest is the input after the constant re-blocking delay, exactly.
        let stream: Vec<f64> = received
            .into_iter()
            .filter(|block| block.iter().any(|sample| *sample != 0.0))
            .flatten()
            .collect();
        let delay = reblock_delay_frames(usize::from(FRAMES)) * usize::from(CHANNELS);
        assert!(stream[..delay].iter().all(|sample| *sample == 0.0));
        let expected: Vec<f64> = (1..=blocks).flat_map(noise).collect();
        let carried = (stream.len() - delay).min(expected.len());
        assert!(carried >= expected.len() - BLOCK, "{carried}");
        assert_eq!(&stream[delay..delay + carried], &expected[..carried]);
        assert_eq!(counters.render_blocks, blocks);
        assert_eq!(counters.render_sequence_gaps, 0);
        assert_eq!((counters.dropped, counters.trimmed), (0, 0));
        assert_eq!(counters.engine_silent, 0);
    }

    #[test]
    fn cable_route_worker_never_overwrites_an_unread_capture_block() {
        let (render, capture) = (Mapping::new(), Mapping::new());
        let worker = start(&render, &capture);
        // The reader never acknowledges the primed block.
        for block in 1..=20_u64 {
            render.peer().write_f64(1, block, &noise(block)).unwrap();
            until(|| render.peer().consumer_sequence() == block);
        }
        std::thread::sleep(Duration::from_millis(20));
        let counters = worker.stop().unwrap();
        assert_eq!(
            counters.capture_blocks, 0,
            "nothing published past the unread block"
        );
        let mut got = vec![1.0; BLOCK];
        let header = capture.peer().read_into_f64_after(2, 0, &mut got).unwrap();
        assert_eq!(header.sequence, 1);
        assert!(
            got.iter().all(|sample| *sample == 0.0),
            "primed block intact"
        );
        // The relay stays bounded: it holds its depth and drops the rest.
        assert!(counters.dropped >= 15, "{counters:?}");
        assert_eq!(counters.forwarded + counters.silence, 0);
    }

    #[test]
    fn cable_route_worker_publishes_silence_when_the_source_is_idle() {
        let (render, capture) = (Mapping::new(), Mapping::new());
        let worker = start(&render, &capture);
        let mut got = vec![1.0; BLOCK];
        let mut capture_sequence = 0;
        for _ in 0..5 {
            until(|| {
                capture
                    .peer()
                    .read_into_f64_after(2, capture_sequence, &mut got)
                    .map(|header| capture_sequence = header.sequence)
                    .is_ok()
            });
            assert!(got.iter().all(|sample| *sample == 0.0));
        }
        let counters = worker.stop().unwrap();
        assert!(counters.silence >= 4, "{counters:?}");
        assert_eq!(counters.forwarded, 0);
        assert!(CableRouteWorker::start(
            render.view(),
            capture.view(),
            processor(),
            CableRouteWorkerConfig {
                render_generation: 1,
                capture_generation: 2,
                block_samples: 0,
            },
        )
        .is_err());
    }
}
