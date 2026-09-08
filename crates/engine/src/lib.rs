//! Allocation-free audio-block primitives for the M02 realtime boundary.
//!
//! Construction and graph preparation happen off the callback thread. Once an
//! `AudioBlock` exists, the operations below reuse its storage and perform no
//! heap allocation, locking, I/O, or logging.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub const INTERNAL_SAMPLE_RATE_HZ: u32 = 48_000;
pub const PROCESSING_QUANTUM_FRAMES: usize = 128;
pub const MAX_CHANNELS: usize = 2;
/// Maximum number of preallocated audio blocks owned by one queue or pool.
/// This bounds construction-time memory even when capacity originates at an
/// external control boundary.
pub const MAX_AUDIO_QUEUE_BLOCKS: usize = 2048;
pub const MAX_MIXER_INPUTS: usize = 8;
pub const MAX_FANOUT_BRANCHES: usize = 8;
pub const MAX_EXTRA_COMPENSATION_MS: u32 = 250;
pub const MAX_DELAY_FRAMES: usize = 48_000;
pub const MAX_DRIFT_CORRECTION_PPM: f64 = 999_999.0;

const PCM16_QUANTUM_SAMPLES: usize = MAX_CHANNELS * PROCESSING_QUANTUM_FRAMES;
pub const MAX_PCM16_PACKET_FRAMES: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatencyCompensationError {
    Empty,
    InvalidSampleRate,
    OverBudget {
        required_samples: u64,
        maximum_samples: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedDelayError {
    InvalidChannels,
    InvalidCapacity,
    InvalidDelay,
    ShapeMismatch,
}

/// Prepared sample delay for one compensated branch. Processing reuses the
/// ring and `AudioBlock` storage, so it performs no heap allocation.
pub struct FixedDelay {
    channels: usize,
    capacity_frames: usize,
    delay_frames: usize,
    buffer: Vec<f32>,
    write_frame: usize,
}

impl FixedDelay {
    pub fn new(channels: usize, maximum_delay_frames: usize) -> Result<Self, FixedDelayError> {
        if !(1..=MAX_CHANNELS).contains(&channels) {
            return Err(FixedDelayError::InvalidChannels);
        }
        if maximum_delay_frames == 0 || maximum_delay_frames > MAX_DELAY_FRAMES {
            return Err(FixedDelayError::InvalidCapacity);
        }
        let capacity_frames = maximum_delay_frames + 1;
        Ok(Self {
            channels,
            capacity_frames,
            delay_frames: 0,
            buffer: vec![0.0; capacity_frames * channels],
            write_frame: 0,
        })
    }

    pub fn delay_frames(&self) -> usize {
        self.delay_frames
    }

    pub fn set_delay_frames(&mut self, delay_frames: usize) -> Result<(), FixedDelayError> {
        if delay_frames >= self.capacity_frames {
            return Err(FixedDelayError::InvalidDelay);
        }
        if self.delay_frames != delay_frames {
            // A changed delay index must not reinterpret samples written for
            // the previous schedule as valid audio in the new generation.
            self.reset();
        }
        self.delay_frames = delay_frames;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_frame = 0;
    }

    pub fn process(&mut self, block: &mut AudioBlock) -> Result<(), FixedDelayError> {
        if block.channels() != self.channels {
            return Err(FixedDelayError::ShapeMismatch);
        }
        for frame in 0..block.frames() {
            let read_frame = (self.write_frame + self.capacity_frames - self.delay_frames)
                % self.capacity_frames;
            for channel in 0..self.channels {
                let input = block.channel(channel).unwrap()[frame];
                let input = if input.is_finite() { input } else { 0.0 };
                let index = self.write_frame * self.channels + channel;
                let delayed = self.buffer[read_frame * self.channels + channel];
                self.buffer[index] = input;
                block.channel_mut(channel).unwrap()[frame] = if self.delay_frames == 0 {
                    input
                } else {
                    delayed
                };
            }
            self.write_frame = (self.write_frame + 1) % self.capacity_frames;
        }
        Ok(())
    }
}

/// Returns per-branch delay samples needed to align paths at their mixer.
/// This is a preparation-time calculation; the realtime callback receives the
/// resulting fixed delays from a separately prepared graph.
pub fn calculate_latency_compensation(
    path_latencies_samples: &[u64],
    sample_rate_hz: u32,
) -> Result<Vec<u64>, LatencyCompensationError> {
    if path_latencies_samples.is_empty() {
        return Err(LatencyCompensationError::Empty);
    }
    if !(8_000..=192_000).contains(&sample_rate_hz) {
        return Err(LatencyCompensationError::InvalidSampleRate);
    }
    let maximum = *path_latencies_samples.iter().max().unwrap();
    let minimum = *path_latencies_samples.iter().min().unwrap();
    let required = maximum - minimum;
    let maximum_allowed =
        (u64::from(sample_rate_hz) * u64::from(MAX_EXTRA_COMPENSATION_MS)).div_ceil(1_000);
    if required > maximum_allowed {
        return Err(LatencyCompensationError::OverBudget {
            required_samples: required,
            maximum_samples: maximum_allowed,
        });
    }
    Ok(path_latencies_samples
        .iter()
        .map(|latency| maximum - latency)
        .collect())
}

#[derive(Debug, Eq, PartialEq)]
pub enum MeterError {
    InvalidCapacity,
}

/// Preallocated rolling RMS window. The window stores finite sample energy and
/// treats non-finite input as silence; pushing samples never allocates.
pub struct RmsWindow {
    samples: Vec<f32>,
    next: usize,
    len: usize,
    sum_squares: f64,
}

impl RmsWindow {
    pub fn new(capacity_samples: usize) -> Result<Self, MeterError> {
        if capacity_samples == 0 {
            return Err(MeterError::InvalidCapacity);
        }
        Ok(Self {
            samples: vec![0.0; capacity_samples],
            next: 0,
            len: 0,
            sum_squares: 0.0,
        })
    }

    pub fn capacity(&self) -> usize {
        self.samples.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push_block(&mut self, block: &AudioBlock) {
        for sample in &block.samples {
            let sample = if sample.is_finite() { *sample } else { 0.0 };
            if self.len == self.samples.len() {
                let old = self.samples[self.next];
                self.sum_squares -= f64::from(old) * f64::from(old);
            } else {
                self.len += 1;
            }
            self.samples[self.next] = sample;
            self.sum_squares += f64::from(sample) * f64::from(sample);
            self.next = (self.next + 1) % self.samples.len();
        }
    }

    pub fn rms(&self) -> f32 {
        if self.len == 0 {
            0.0
        } else {
            (self.sum_squares / self.len as f64).sqrt() as f32
        }
    }

    pub fn reset(&mut self) {
        self.samples.fill(0.0);
        self.next = 0;
        self.len = 0;
        self.sum_squares = 0.0;
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum QueueError {
    InvalidCapacity,
    InvalidShape,
}

/// Fixed-capacity nonblocking queue for preallocated audio blocks. The queue
/// allocates its slots during construction; push/pop never wait or allocate.
pub struct AudioBlockQueue {
    blocks: crossbeam_queue::ArrayQueue<AudioBlock>,
    shape: Option<(usize, usize)>,
    overruns: AtomicU64,
    underruns: AtomicU64,
    invalid_blocks: AtomicU64,
}

/// Reusable pool of fixed-shape blocks. All backing allocations happen during
/// construction; a well-formed acquire/release cycle performs no allocation or
/// deallocation and is suitable for a future callback-owned buffer ring.
pub struct AudioBlockPool {
    blocks: crossbeam_queue::ArrayQueue<AudioBlock>,
    shape: (usize, usize),
}

/// A bounded producer/consumer ring whose blocks are recycled from a fixed
/// pool. The normal acquire-submit-receive-recycle cycle does not allocate or
/// deallocate; callers retain ownership when a boundary is full or empty.
pub struct AudioBlockRing {
    free: AudioBlockPool,
    ready: AudioBlockQueue,
}

impl AudioBlockRing {
    pub fn new(capacity: usize, channels: usize, frames: usize) -> Result<Self, QueueError> {
        Ok(Self {
            free: AudioBlockPool::new(capacity, channels, frames)?,
            ready: AudioBlockQueue::new_for_shape(capacity, channels, frames)?,
        })
    }

    pub fn capacity(&self) -> usize {
        self.free.capacity()
    }

    pub fn available(&self) -> usize {
        self.free.available()
    }

    pub fn ready(&self) -> usize {
        self.ready.len()
    }

    pub fn try_acquire(&self) -> Option<AudioBlock> {
        self.free.try_acquire()
    }

    pub fn try_submit(&self, block: AudioBlock) -> Result<(), AudioBlock> {
        self.ready.try_push(block)
    }

    pub fn try_receive(&self) -> Option<AudioBlock> {
        self.ready.try_pop()
    }

    /// Receive only a block owned by the requested runtime generation.
    /// Stale blocks are recycled and never exposed to the caller.
    pub fn try_receive_generation(&self, generation: u64) -> Option<AudioBlock> {
        while let Some(block) = self.ready.try_pop() {
            if block.generation() == generation {
                return Some(block);
            }
            let _ = self.try_recycle(block);
        }
        None
    }

    pub fn try_recycle(&self, block: AudioBlock) -> Result<(), AudioBlock> {
        self.free.try_release(block)
    }

    /// Recycle every queued block, used by the control boundary when a new
    /// runtime generation is published. This never waits and restores the
    /// blocks to this ring's bounded pool instead of dropping their storage.
    pub fn recycle_all(&self) -> usize {
        let mut recycled = 0;
        while let Some(block) = self.ready.try_pop() {
            if self.try_recycle(block).is_ok() {
                recycled += 1;
            }
        }
        recycled
    }

    pub fn overruns(&self) -> u64 {
        self.ready.overruns()
    }

    pub fn underruns(&self) -> u64 {
        self.ready.underruns()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum VirtualBusBridgeError {
    InvalidGeneration,
}

/// Portable bridge boundary for a managed virtual bus. Render input is
/// bounded and is copied into a separate bounded capture ring. The bridge is
/// silent while inactive, rejects stale generations, and drops rather than
/// buffering without bound when the capture side has no free block.
pub struct VirtualBusBridge {
    render: AudioBlockRing,
    capture: AudioBlockRing,
    active: AtomicBool,
    generation: AtomicU64,
    activation_lock: AtomicBool,
    dropped: AtomicU64,
}

impl VirtualBusBridge {
    pub fn new(capacity: usize, channels: usize, frames: usize) -> Result<Self, QueueError> {
        Ok(Self {
            render: AudioBlockRing::new(capacity, channels, frames)?,
            capture: AudioBlockRing::new(capacity, channels, frames)?,
            active: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            activation_lock: AtomicBool::new(false),
            dropped: AtomicU64::new(0),
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Start a new ownership generation. Changing generations first discards
    /// all queued data so a replacement owner cannot receive stale frames.
    /// Activation is a control-plane operation; its narrow guard prevents two
    /// replacements from interleaving their drain/reactivate sequence. The
    /// realtime bridge methods never acquire this guard.
    pub fn activate(&self, generation: u64) -> Result<(), VirtualBusBridgeError> {
        if generation == 0 {
            return Err(VirtualBusBridgeError::InvalidGeneration);
        }
        self.acquire_activation_lock();
        let previous = self.generation.load(Ordering::Acquire);
        if generation <= previous {
            self.release_activation_lock();
            return Err(VirtualBusBridgeError::InvalidGeneration);
        }
        if self
            .generation
            .compare_exchange(previous, generation, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            self.release_activation_lock();
            return Err(VirtualBusBridgeError::InvalidGeneration);
        }
        self.active.store(false, Ordering::Release);
        self.drain();
        self.active.store(true, Ordering::Release);
        self.release_activation_lock();
        Ok(())
    }

    /// Stop processing and clear both bounded rings. Clearing is the safe
    /// silence/recovery behavior for backend or owner loss.
    pub fn deactivate(&self) {
        self.acquire_activation_lock();
        self.active.store(false, Ordering::Release);
        self.drain();
        self.release_activation_lock();
    }

    pub fn submit_render(&self, generation: u64, mut block: AudioBlock) -> Result<(), AudioBlock> {
        if !self.is_active() || self.generation() != generation {
            return Err(block);
        }
        block.generation = generation;
        self.render.try_submit(block)
    }

    /// Move as many render blocks as possible into the capture ring. The
    /// returned count is the number of captured blocks, not the number
    /// received from the producer; overflow is counted and recycled.
    pub fn process_once(&self) -> usize {
        if !self.is_active() {
            self.drain();
            return 0;
        }
        let mut processed = 0;
        while let Some(input) = self.render.try_receive() {
            let generation = self.generation();
            let Some(mut output) = self.capture.try_acquire() else {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                let _ = self.render.try_recycle(input);
                continue;
            };
            if output.copy_from(&input).is_err() {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                let _ = self.capture.try_recycle(output);
                let _ = self.render.try_recycle(input);
                continue;
            }
            output.sanitize_non_finite();
            if !self.owns_generation(generation) {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                let _ = self.capture.try_recycle(output);
                let _ = self.render.try_recycle(input);
                continue;
            }
            let _ = self.render.try_recycle(input);
            match self.capture.try_submit(output) {
                Ok(()) if self.owns_generation(generation) => processed += 1,
                Ok(()) => {
                    // Activation may have drained just before the submit. A
                    // post-submit check closes that window and removes the
                    // raced stale block before the new owner can consume it.
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                    self.drain_capture();
                }
                Err(output) => {
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                    let _ = self.capture.try_recycle(output);
                }
            }
        }
        processed
    }

    /// Fan out render blocks into caller-owned bounded destination rings.
    /// Each destination receives an independent copy when it has capacity;
    /// one slow destination cannot block the others or grow memory.
    pub fn fanout_once(&self, destinations: &[&AudioBlockRing]) -> usize {
        if !self.is_active() {
            self.drain();
            return 0;
        }
        let mut deliveries = 0;
        while let Some(input) = self.render.try_receive() {
            let generation = self.generation();
            let mut delivered = 0;
            for destination in destinations {
                if !self.owns_generation(generation) {
                    break;
                }
                let Some(mut output) = destination.try_acquire() else {
                    continue;
                };
                if output.copy_from(&input).is_err() {
                    let _ = destination.try_recycle(output);
                    continue;
                }
                output.sanitize_non_finite();
                match destination.try_submit(output) {
                    Ok(()) => {
                        delivered += 1;
                        deliveries += 1;
                    }
                    Err(output) => {
                        let _ = destination.try_recycle(output);
                    }
                }
            }
            if delivered == 0 {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
            let _ = self.render.try_recycle(input);
        }
        deliveries
    }

    fn owns_generation(&self, generation: u64) -> bool {
        self.active.load(Ordering::Acquire) && self.generation.load(Ordering::Acquire) == generation
    }

    fn acquire_activation_lock(&self) {
        while self
            .activation_lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
    }

    fn release_activation_lock(&self) {
        self.activation_lock.store(false, Ordering::Release);
    }

    pub fn try_receive_capture(&self) -> Option<AudioBlock> {
        if !self.is_active() {
            self.drain_capture();
            return None;
        }
        self.capture.try_receive_generation(self.generation())
    }

    /// Fill a caller-owned output block from the capture side. On underrun or
    /// inactive state the output is explicitly cleared to silence without an
    /// allocation; the boolean reports whether a queued block was delivered.
    pub fn receive_capture_into(&self, output: &mut AudioBlock) -> Result<bool, BlockError> {
        let Some(input) = self.try_receive_capture() else {
            output.clear();
            return Ok(false);
        };
        let result = output.copy_from(&input);
        let _ = self.capture.try_recycle(input);
        if result.is_err() {
            output.clear();
            return result.map(|()| true);
        }
        Ok(true)
    }

    pub fn try_recycle_capture(&self, block: AudioBlock) -> Result<(), AudioBlock> {
        self.capture.try_recycle(block)
    }

    fn drain(&self) {
        while let Some(block) = self.render.try_receive() {
            let _ = self.render.try_recycle(block);
        }
        self.drain_capture();
    }

    fn drain_capture(&self) {
        while let Some(block) = self.capture.try_receive() {
            let _ = self.capture.try_recycle(block);
        }
    }
}

impl AudioBlockPool {
    pub fn new(capacity: usize, channels: usize, frames: usize) -> Result<Self, QueueError> {
        if !(1..=MAX_AUDIO_QUEUE_BLOCKS).contains(&capacity) {
            return Err(QueueError::InvalidCapacity);
        }
        if !(1..=MAX_CHANNELS).contains(&channels)
            || !(1..=PROCESSING_QUANTUM_FRAMES).contains(&frames)
        {
            return Err(QueueError::InvalidShape);
        }
        let blocks = crossbeam_queue::ArrayQueue::new(capacity);
        for _ in 0..capacity {
            blocks
                .push(AudioBlock::new(channels, frames).unwrap())
                .expect("new pool has capacity for every block");
        }
        Ok(Self {
            blocks,
            shape: (channels, frames),
        })
    }

    pub fn capacity(&self) -> usize {
        self.blocks.capacity()
    }

    pub fn available(&self) -> usize {
        self.blocks.len()
    }

    pub fn try_acquire(&self) -> Option<AudioBlock> {
        self.blocks.pop()
    }

    pub fn try_release(&self, block: AudioBlock) -> Result<(), AudioBlock> {
        if (block.channels(), block.frames()) != self.shape {
            return Err(block);
        }
        let mut block = block;
        block.clear();
        block.generation = 0;
        self.blocks.push(block)
    }
}

impl AudioBlockQueue {
    pub fn new(capacity: usize) -> Result<Self, QueueError> {
        if !(1..=MAX_AUDIO_QUEUE_BLOCKS).contains(&capacity) {
            return Err(QueueError::InvalidCapacity);
        }
        Ok(Self {
            blocks: crossbeam_queue::ArrayQueue::new(capacity),
            shape: None,
            overruns: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
            invalid_blocks: AtomicU64::new(0),
        })
    }

    pub fn new_for_shape(
        capacity: usize,
        channels: usize,
        frames: usize,
    ) -> Result<Self, QueueError> {
        if !(1..=MAX_AUDIO_QUEUE_BLOCKS).contains(&capacity) {
            return Err(QueueError::InvalidCapacity);
        }
        if !(1..=MAX_CHANNELS).contains(&channels)
            || !(1..=PROCESSING_QUANTUM_FRAMES).contains(&frames)
        {
            return Err(QueueError::InvalidShape);
        }
        Ok(Self {
            blocks: crossbeam_queue::ArrayQueue::new(capacity),
            shape: Some((channels, frames)),
            overruns: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
            invalid_blocks: AtomicU64::new(0),
        })
    }

    pub fn capacity(&self) -> usize {
        self.blocks.capacity()
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn overruns(&self) -> u64 {
        self.overruns.load(Ordering::Relaxed)
    }

    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    pub fn invalid_blocks(&self) -> u64 {
        self.invalid_blocks.load(Ordering::Relaxed)
    }

    pub fn try_push(&self, block: AudioBlock) -> Result<(), AudioBlock> {
        if let Some((channels, frames)) = self.shape {
            if block.channels() != channels || block.frames() != frames {
                self.invalid_blocks.fetch_add(1, Ordering::Relaxed);
                return Err(block);
            }
        }
        match self.blocks.push(block) {
            Ok(()) => Ok(()),
            Err(block) => {
                self.overruns.fetch_add(1, Ordering::Relaxed);
                Err(block)
            }
        }
    }

    pub fn try_pop(&self) -> Option<AudioBlock> {
        match self.blocks.pop() {
            Some(block) => Some(block),
            None => {
                self.underruns.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Discard all currently queued blocks, used during stop/reconnect so old
    /// audio is never replayed into a new runtime generation. This operation
    /// is nonblocking and does not count the intentional discard as an xrun.
    pub fn drain(&self) -> usize {
        let mut discarded = 0;
        while self.blocks.pop().is_some() {
            discarded += 1;
        }
        discarded
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockError {
    InvalidChannels,
    InvalidFrameCount,
    ShapeMismatch,
    InvalidSampleRate,
    InvalidDriftCorrection,
}

/// Fixed-capacity interleaved PCM16 staging for the engine quantum. The
/// adapter accepts partial packets and stops accepting input when one complete
/// block is ready; callers must drain that block before pushing more samples.
/// This makes packet accumulation bounded and allocation-free after creation.
#[derive(Debug)]
pub struct Pcm16QuantumAdapter {
    channels: usize,
    pending_frames: usize,
    pending: [i16; PCM16_QUANTUM_SAMPLES],
}

impl Pcm16QuantumAdapter {
    /// Create a staging adapter for one mono or stereo 128-frame quantum.
    pub fn new(channels: usize) -> Result<Self, BlockError> {
        if !(1..=MAX_CHANNELS).contains(&channels) {
            return Err(BlockError::InvalidChannels);
        }
        Ok(Self {
            channels,
            pending_frames: 0,
            pending: [0; PCM16_QUANTUM_SAMPLES],
        })
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn pending_frames(&self) -> usize {
        self.pending_frames
    }

    /// Copy as many complete interleaved frames as fit. The returned count is
    /// the number of source frames consumed; a zero return means the caller
    /// must first call `pop_into`.
    pub fn push_interleaved(&mut self, source: &[i16]) -> Result<usize, BlockError> {
        if source.len() % self.channels != 0 {
            return Err(BlockError::ShapeMismatch);
        }
        let available = PROCESSING_QUANTUM_FRAMES - self.pending_frames;
        let frames = (source.len() / self.channels).min(available);
        let sample_count = frames * self.channels;
        self.pending[self.pending_frames * self.channels..][..sample_count]
            .copy_from_slice(&source[..sample_count]);
        self.pending_frames += frames;
        Ok(frames)
    }

    /// Admit one complete bounded device packet. Packets larger than the
    /// supported period ceiling are rejected before any staging copy; callers
    /// may use `push_interleaved` for already validated chunks.
    pub fn push_packet(&mut self, source: &[i16]) -> Result<usize, BlockError> {
        if source.len() % self.channels != 0 {
            return Err(BlockError::ShapeMismatch);
        }
        let frames = source.len() / self.channels;
        if frames == 0 || frames > MAX_PCM16_PACKET_FRAMES {
            return Err(BlockError::InvalidFrameCount);
        }
        self.push_interleaved(source)
    }

    /// Decode one complete quantum into a preallocated planar engine block.
    /// A block is left queued until this succeeds, so a shape error cannot
    /// silently discard captured samples.
    pub fn pop_into(&mut self, destination: &mut AudioBlock) -> Result<bool, BlockError> {
        if destination.channels() != self.channels
            || destination.frames() != PROCESSING_QUANTUM_FRAMES
        {
            return Err(BlockError::ShapeMismatch);
        }
        if self.pending_frames < PROCESSING_QUANTUM_FRAMES {
            return Ok(false);
        }
        destination.copy_from_interleaved_pcm16(
            &self.pending[..self.channels * PROCESSING_QUANTUM_FRAMES],
        )?;
        self.pending_frames = 0;
        Ok(true)
    }
}

/// A preallocated planar float32 block. Samples are stored channel-major:
/// `channel * frames + frame`.
#[derive(Debug)]
pub struct AudioBlock {
    channels: usize,
    frames: usize,
    samples: Vec<f32>,
    generation: u64,
}

/// Bounded per-frame gain transition for de-clicked parameter changes.
/// Construction and target changes occur off the callback thread; applying a
/// ramp only updates existing block samples and this small state object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GainRamp {
    current: f32,
    target: f32,
    step: f32,
    remaining_frames: usize,
}

/// Process-local privacy gate. It silences blocks at a boundary and does not
/// alter Windows privacy permissions or other applications' microphone use.
#[derive(Debug, Default)]
pub struct PrivacyMute {
    muted: std::sync::atomic::AtomicBool,
}

impl PrivacyMute {
    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Release);
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Acquire)
    }

    pub fn apply(&self, block: &mut AudioBlock) {
        if self.is_muted() {
            block.clear();
        }
    }
}

impl GainRamp {
    pub fn new(initial: f32) -> Self {
        let initial = if initial.is_finite() { initial } else { 0.0 };
        Self {
            current: initial,
            target: initial,
            step: 0.0,
            remaining_frames: 0,
        }
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    /// Set a finite target and transition over at most `ramp_frames` frames.
    /// A zero-length ramp changes the gain immediately.
    pub fn set_target(&mut self, target: f32, ramp_frames: usize) {
        let target = if target.is_finite() { target } else { 0.0 };
        self.target = target;
        if ramp_frames == 0 {
            self.current = target;
            self.step = 0.0;
            self.remaining_frames = 0;
        } else {
            self.step = (target - self.current) / ramp_frames as f32;
            self.remaining_frames = ramp_frames;
        }
    }

    /// Apply the current ramp to every channel of a block without allocating.
    pub fn apply(&mut self, block: &mut AudioBlock) {
        for frame in 0..block.frames {
            if self.remaining_frames > 0 {
                self.current += self.step;
                self.remaining_frames -= 1;
                if self.remaining_frames == 0 {
                    self.current = self.target;
                    self.step = 0.0;
                }
            }
            for channel in 0..block.channels {
                block.channel_mut(channel).unwrap()[frame] *= self.current;
            }
        }
    }
}

impl AudioBlock {
    /// Allocate a block during preparation, before entering the realtime path.
    pub fn new(channels: usize, frames: usize) -> Result<Self, BlockError> {
        if !(1..=MAX_CHANNELS).contains(&channels) {
            return Err(BlockError::InvalidChannels);
        }
        if !(1..=PROCESSING_QUANTUM_FRAMES).contains(&frames) {
            return Err(BlockError::InvalidFrameCount);
        }
        Ok(Self {
            channels,
            frames,
            samples: vec![0.0; channels * frames],
            generation: 0,
        })
    }

    /// Runtime ownership tag copied with the block through prepared rings.
    /// Zero means that no scheduler generation has claimed the block.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn channel(&self, channel: usize) -> Option<&[f32]> {
        (channel < self.channels).then(|| {
            let start = channel * self.frames;
            &self.samples[start..start + self.frames]
        })
    }

    pub fn channel_mut(&mut self, channel: usize) -> Option<&mut [f32]> {
        (channel < self.channels).then(|| {
            let start = channel * self.frames;
            &mut self.samples[start..start + self.frames]
        })
    }

    /// Clear the existing storage without allocating.
    pub fn clear(&mut self) {
        self.samples.fill(0.0);
    }

    /// Copy a same-shaped block without allocating.
    pub fn copy_from(&mut self, source: &Self) -> Result<(), BlockError> {
        if self.channels != source.channels || self.frames != source.frames {
            return Err(BlockError::ShapeMismatch);
        }
        self.samples.copy_from_slice(&source.samples);
        self.generation = source.generation;
        Ok(())
    }

    /// Copy interleaved `f32` samples into this planar block without
    /// allocating. The source must contain exactly one complete block.
    pub fn copy_from_interleaved(&mut self, source: &[f32]) -> Result<(), BlockError> {
        if source.len() != self.channels * self.frames {
            return Err(BlockError::ShapeMismatch);
        }
        for (frame, samples) in source.chunks_exact(self.channels).enumerate() {
            for (channel, sample) in samples.iter().enumerate() {
                self.channel_mut(channel).unwrap()[frame] = *sample;
            }
        }
        Ok(())
    }

    /// Decode one complete interleaved signed-16-bit PCM block into planar
    /// float32 samples without allocating. PCM16 uses the conventional
    /// symmetric engine scale: -32768 maps to -1.0 and 32767 maps just below
    /// 1.0. The source must contain exactly one complete block.
    pub fn copy_from_interleaved_pcm16(&mut self, source: &[i16]) -> Result<(), BlockError> {
        if source.len() != self.channels * self.frames {
            return Err(BlockError::ShapeMismatch);
        }
        for (frame, samples) in source.chunks_exact(self.channels).enumerate() {
            for (channel, sample) in samples.iter().enumerate() {
                self.channel_mut(channel).unwrap()[frame] = f32::from(*sample) / 32_768.0;
            }
        }
        Ok(())
    }

    /// Copy this planar block into an interleaved `f32` destination without
    /// allocating. The destination must contain exactly one complete block.
    pub fn copy_to_interleaved(&self, destination: &mut [f32]) -> Result<(), BlockError> {
        if destination.len() != self.channels * self.frames {
            return Err(BlockError::ShapeMismatch);
        }
        for (frame, samples) in destination.chunks_exact_mut(self.channels).enumerate() {
            for (channel, sample) in samples.iter_mut().enumerate() {
                *sample = self.channel(channel).unwrap()[frame];
            }
        }
        Ok(())
    }

    /// Encode one complete planar float32 block as interleaved signed-16-bit
    /// PCM without allocating. Non-finite values become silence and finite
    /// values are clamped to the PCM16 range before rounding.
    pub fn copy_to_interleaved_pcm16(&self, destination: &mut [i16]) -> Result<(), BlockError> {
        if destination.len() != self.channels * self.frames {
            return Err(BlockError::ShapeMismatch);
        }
        for (frame, samples) in destination.chunks_exact_mut(self.channels).enumerate() {
            for (channel, sample) in samples.iter_mut().enumerate() {
                let value = self.channel(channel).unwrap()[frame];
                let scaled = if value.is_finite() {
                    (value.clamp(-1.0, 1.0) * 32_768.0).round()
                } else {
                    0.0
                };
                *sample = (scaled as i32).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            }
        }
        Ok(())
    }

    /// Apply a constant gain without allocating. Non-finite gain is treated as
    /// zero so invalid control input cannot inject NaN/Inf into the graph.
    pub fn apply_gain(&mut self, gain: f32) {
        let gain = if gain.is_finite() { gain } else { 0.0 };
        for sample in &mut self.samples {
            *sample *= gain;
        }
    }

    /// Apply a same-channel destination-major matrix in place. The fixed
    /// two-channel scratch array keeps this operation allocation-free.
    pub fn apply_channel_matrix(&mut self, matrix: &[f32]) -> Result<(), BlockError> {
        if matrix.len() != self.channels * self.channels {
            return Err(BlockError::ShapeMismatch);
        }
        for frame in 0..self.frames {
            let mut input = [0.0; MAX_CHANNELS];
            for (channel, sample) in input.iter_mut().enumerate().take(self.channels) {
                *sample = self.channel(channel).unwrap()[frame];
            }
            for destination_channel in 0..self.channels {
                let mut value = 0.0;
                for source_channel in 0..self.channels {
                    let coefficient = matrix[destination_channel * self.channels + source_channel];
                    value += input[source_channel] * coefficient;
                }
                self.channel_mut(destination_channel).unwrap()[frame] = value;
            }
        }
        Ok(())
    }

    /// Add a same-shaped source block into this block without allocating.
    pub fn mix_from(&mut self, source: &Self, gain: f32) -> Result<(), BlockError> {
        if self.channels != source.channels || self.frames != source.frames {
            return Err(BlockError::ShapeMismatch);
        }
        let gain = if gain.is_finite() { gain } else { 0.0 };
        for (destination, source) in self.samples.iter_mut().zip(&source.samples) {
            *destination += *source * gain;
        }
        Ok(())
    }

    /// Apply an explicit source-channel-to-destination-channel matrix without
    /// allocating. Matrix order is destination-major: `dst * source_channels
    /// + src`. This keeps mono/stereo conversion visible in the compiled graph.
    pub fn map_from(&mut self, source: &Self, matrix: &[f32]) -> Result<(), BlockError> {
        if self.frames != source.frames
            || matrix.len() != self.channels.saturating_mul(source.channels)
        {
            return Err(BlockError::ShapeMismatch);
        }
        for destination_channel in 0..self.channels {
            let destination = self.channel_mut(destination_channel).unwrap();
            for (frame, sample) in destination.iter_mut().enumerate() {
                let mut value = 0.0;
                for source_channel in 0..source.channels {
                    value += source.channel(source_channel).unwrap()[frame]
                        * matrix[destination_channel * source.channels + source_channel];
                }
                *sample = value;
            }
        }
        Ok(())
    }

    /// Accumulate a source block through an explicit destination-major channel
    /// matrix. This is the primitive used by fan-out and explicit mixer inputs;
    /// it preserves the destination's existing samples and allocates nothing.
    pub fn mix_mapped_from(&mut self, source: &Self, matrix: &[f32]) -> Result<(), BlockError> {
        if self.frames != source.frames
            || matrix.len() != self.channels.saturating_mul(source.channels)
        {
            return Err(BlockError::ShapeMismatch);
        }
        for destination_channel in 0..self.channels {
            let destination = self.channel_mut(destination_channel).unwrap();
            for (frame, sample) in destination.iter_mut().enumerate() {
                let mut value = 0.0;
                for source_channel in 0..source.channels {
                    let coefficient =
                        matrix[destination_channel * source.channels + source_channel];
                    let coefficient = if coefficient.is_finite() {
                        coefficient
                    } else {
                        0.0
                    };
                    value += source.channel(source_channel).unwrap()[frame] * coefficient;
                }
                *sample += value;
            }
        }
        Ok(())
    }

    /// Linearly resample a same-channel source into this preallocated block.
    /// This is a bounded format-conversion primitive; clock-drift correction
    /// and cross-block phase management belong to the later stream scheduler.
    pub fn resample_linear_from(
        &mut self,
        source: &Self,
        input_rate_hz: u32,
        output_rate_hz: u32,
    ) -> Result<(), BlockError> {
        if input_rate_hz == 0 || output_rate_hz == 0 {
            return Err(BlockError::InvalidSampleRate);
        }
        self.resample_linear_with_ratio(source, input_rate_hz as f64 / output_rate_hz as f64)
    }

    /// Resample with a caller-supplied bounded ratio adjustment. The ratio is
    /// intended for a prepared drift controller; callers retain ownership of
    /// cross-block buffering and must provide a finite positive value.
    pub fn resample_linear_with_ratio(
        &mut self,
        source: &Self,
        ratio: f64,
    ) -> Result<(), BlockError> {
        if self.channels != source.channels {
            return Err(BlockError::ShapeMismatch);
        }
        if !ratio.is_finite() || ratio <= 0.0 {
            return Err(BlockError::InvalidSampleRate);
        }
        if source.frames == 0 {
            return Err(BlockError::InvalidFrameCount);
        }
        for destination_channel in 0..self.channels {
            let destination = self.channel_mut(destination_channel).unwrap();
            let input = source.channel(destination_channel).unwrap();
            for (frame, sample) in destination.iter_mut().enumerate() {
                let position = frame as f64 * ratio;
                let lower = position.floor() as usize;
                let lower = lower.min(source.frames - 1);
                let upper = (lower + 1).min(source.frames - 1);
                let fraction = (position - lower as f64) as f32;
                let lower_sample = if input[lower].is_finite() {
                    input[lower]
                } else {
                    0.0
                };
                let upper_sample = if input[upper].is_finite() {
                    input[upper]
                } else {
                    0.0
                };
                *sample = lower_sample + (upper_sample - lower_sample) * fraction;
            }
        }
        Ok(())
    }

    /// Replace non-finite samples with silence and return the number repaired.
    pub fn sanitize_non_finite(&mut self) -> usize {
        let mut repaired = 0;
        for sample in &mut self.samples {
            if !sample.is_finite() {
                *sample = 0.0;
                repaired += 1;
            }
        }
        repaired
    }

    pub fn all_finite(&self) -> bool {
        self.samples.iter().all(|sample| sample.is_finite())
    }

    /// Clamp finite samples to the interleaved output boundary [-1, 1] and
    /// return the number clipped. Non-finite values are first converted to
    /// silence and are not counted as over-range clipping.
    pub fn clamp_unit(&mut self) -> usize {
        let mut clipped = 0;
        for sample in &mut self.samples {
            if !sample.is_finite() {
                *sample = 0.0;
            } else if *sample > 1.0 {
                *sample = 1.0;
                clipped += 1;
            } else if *sample < -1.0 {
                *sample = -1.0;
                clipped += 1;
            }
        }
        clipped
    }

    /// Return the largest absolute finite sample, or zero for an empty
    /// conceptual block. This is a bounded meter primitive with no allocation.
    pub fn peak_abs(&self) -> f32 {
        self.samples
            .iter()
            .filter(|sample| sample.is_finite())
            .map(|sample| sample.abs())
            .fold(0.0, f32::max)
    }

    pub fn channel_peak_abs(&self, channel: usize) -> Option<f32> {
        self.channel(channel).map(|samples| {
            samples
                .iter()
                .filter(|sample| sample.is_finite())
                .map(|sample| sample.abs())
                .fold(0.0, f32::max)
        })
    }

    /// Return RMS over finite samples. Invalid samples are excluded so a bad
    /// value cannot poison the meter; sanitization remains a separate policy.
    pub fn channel_rms(&self, channel: usize) -> Option<f32> {
        self.channel(channel).map(|samples| {
            let (sum, count) = samples
                .iter()
                .filter(|sample| sample.is_finite())
                .fold((0.0_f64, 0usize), |(sum, count), sample| {
                    (sum + f64::from(*sample) * f64::from(*sample), count + 1)
                });
            if count == 0 {
                0.0
            } else {
                (sum / count as f64).sqrt() as f32
            }
        })
    }

    pub fn rms(&self) -> f32 {
        let (sum, count) = self
            .samples
            .iter()
            .filter(|sample| sample.is_finite())
            .fold((0.0_f64, 0usize), |(sum, count), sample| {
                (sum + f64::from(*sample) * f64::from(*sample), count + 1)
            });
        if count == 0 {
            0.0
        } else {
            (sum / count as f64).sqrt() as f32
        }
    }
}

/// Preallocated streaming linear resampler for mismatched device clocks.
/// Samples are retained across calls so packet/quantum boundaries do not
/// reset interpolation phase. The caller owns the xrun policy when the FIFO
/// cannot accept a complete source block or produce a complete destination
/// block.
pub struct StreamingResampler {
    channels: usize,
    capacity_frames: usize,
    fifo: Vec<f32>,
    read_frames: usize,
    queued_frames: usize,
    phase: f64,
}

impl StreamingResampler {
    pub fn new(channels: usize, capacity_frames: usize) -> Result<Self, BlockError> {
        if !(1..=MAX_CHANNELS).contains(&channels) {
            return Err(BlockError::InvalidChannels);
        }
        if !(1..=MAX_DELAY_FRAMES).contains(&capacity_frames) {
            return Err(BlockError::InvalidFrameCount);
        }
        Ok(Self {
            channels,
            capacity_frames,
            fifo: vec![0.0; channels * capacity_frames],
            read_frames: 0,
            queued_frames: 0,
            phase: 0.0,
        })
    }

    pub fn queued_frames(&self) -> usize {
        self.queued_frames
    }

    pub fn reset(&mut self) {
        self.read_frames = 0;
        self.queued_frames = 0;
        self.phase = 0.0;
    }

    /// Append a source block without allocating. Returns the number of frames
    /// accepted; zero is an explicit bounded-overflow signal and leaves the
    /// FIFO unchanged rather than partially admitting the source block.
    pub fn push(&mut self, source: &AudioBlock) -> Result<usize, BlockError> {
        if source.channels != self.channels {
            return Err(BlockError::ShapeMismatch);
        }
        let available = self.capacity_frames - self.queued_frames;
        if source.frames > available {
            return Ok(0);
        }
        let accepted = source.frames;
        for channel in 0..self.channels {
            let source_channel = source.channel(channel).unwrap();
            let start = channel * self.capacity_frames + self.read_frames + self.queued_frames;
            self.fifo[start..start + accepted].copy_from_slice(&source_channel[..accepted]);
        }
        self.queued_frames += accepted;
        Ok(accepted)
    }

    /// Produce one destination block when enough source samples are queued.
    /// The destination is cleared when a complete block is unavailable, and
    /// the returned zero makes the bounded underflow explicit. An incomplete
    /// block never advances FIFO ownership or interpolation phase.
    pub fn process(
        &mut self,
        destination: &mut AudioBlock,
        ratio: f64,
    ) -> Result<usize, BlockError> {
        if destination.channels != self.channels {
            return Err(BlockError::ShapeMismatch);
        }
        if !ratio.is_finite() || ratio <= 0.0 {
            return Err(BlockError::InvalidSampleRate);
        }
        destination.clear();
        let mut produced = 0;
        for frame in 0..destination.frames {
            let position = self.phase + frame as f64 * ratio;
            let lower = position.floor() as usize;
            if lower + 1 >= self.queued_frames {
                break;
            }
            let fraction = (position - lower as f64) as f32;
            for channel in 0..self.channels {
                let base = channel * self.capacity_frames + self.read_frames;
                let first = self.fifo[base + lower];
                let second = self.fifo[base + lower + 1];
                let first = if first.is_finite() { first } else { 0.0 };
                let second = if second.is_finite() { second } else { 0.0 };
                destination.channel_mut(channel).unwrap()[frame] =
                    first + (second - first) * fraction;
            }
            produced += 1;
        }
        if produced != destination.frames {
            return Ok(0);
        }
        if produced != 0 {
            self.phase += produced as f64 * ratio;
            let consumed = (self.phase.floor() as usize).min(self.queued_frames.saturating_sub(1));
            if consumed != 0 {
                for channel in 0..self.channels {
                    let base = channel * self.capacity_frames;
                    self.fifo.copy_within(
                        base + self.read_frames + consumed
                            ..base + self.read_frames + self.queued_frames,
                        base + self.read_frames,
                    );
                }
                self.read_frames = 0;
                self.queued_frames -= consumed;
                self.phase -= consumed as f64;
            }
        }
        Ok(produced)
    }
}

/// Fixed-quantum worker adapter for the prepared built-in DSP chain. Planar
/// engine blocks are copied through construction-time interleaved scratch;
/// `process` performs no allocation and rejects shape changes explicitly.
pub struct VoiceChainBlockProcessor {
    chain: audiorouter_dsp::VoiceChain,
    scratch: Vec<f32>,
    channels: usize,
    frames: usize,
}

impl VoiceChainBlockProcessor {
    pub fn new(
        config: audiorouter_dsp::VoiceChainConfig,
        channels: usize,
        frames: usize,
    ) -> Result<Self, audiorouter_dsp::VoiceChainError> {
        let chain = audiorouter_dsp::VoiceChain::new(config, channels)?;
        Ok(Self {
            chain,
            scratch: vec![0.0; channels.saturating_mul(frames)],
            channels,
            frames,
        })
    }

    pub fn process(&mut self, block: &mut AudioBlock) -> Result<(), BlockError> {
        if block.channels() != self.channels || block.frames() != self.frames {
            return Err(BlockError::ShapeMismatch);
        }
        for frame in 0..self.frames {
            for channel in 0..self.channels {
                self.scratch[frame * self.channels + channel] =
                    block.channel(channel).unwrap()[frame];
            }
        }
        self.chain.process_interleaved(&mut self.scratch);
        for frame in 0..self.frames {
            for channel in 0..self.channels {
                block.channel_mut(channel).unwrap()[frame] =
                    self.scratch[frame * self.channels + channel];
            }
        }
        Ok(())
    }

    pub fn meter(&self) -> audiorouter_dsp::MeterSnapshot {
        self.chain.meter()
    }

    pub fn reset(&mut self) {
        self.chain.reset();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeGeneration(u64);

impl RuntimeGeneration {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DriftController {
    nominal_ratio: f64,
    correction_ppm: f64,
    integral_ppm: f64,
    target_frames: f64,
    max_correction_ppm: f64,
}

impl DriftController {
    pub fn new(
        input_rate_hz: u32,
        output_rate_hz: u32,
        target_frames: usize,
        max_correction_ppm: f64,
    ) -> Result<Self, BlockError> {
        if input_rate_hz == 0
            || output_rate_hz == 0
            || target_frames == 0
            || !max_correction_ppm.is_finite()
            || !(0.0..=MAX_DRIFT_CORRECTION_PPM).contains(&max_correction_ppm)
        {
            return Err(
                if input_rate_hz == 0 || output_rate_hz == 0 || target_frames == 0 {
                    BlockError::InvalidSampleRate
                } else {
                    BlockError::InvalidDriftCorrection
                },
            );
        }
        Ok(Self {
            nominal_ratio: input_rate_hz as f64 / output_rate_hz as f64,
            correction_ppm: 0.0,
            integral_ppm: 0.0,
            target_frames: target_frames as f64,
            max_correction_ppm,
        })
    }

    /// Update correction from bounded FIFO occupancy. The integral term holds
    /// a steady clock offset at the target instead of requiring a permanent
    /// FIFO error; both the integral state and resulting correction are
    /// bounded. Callers still need xrun/discontinuity policy around the
    /// stream scheduler.
    pub fn observe_queue(&mut self, queue_frames: usize) {
        let error = (queue_frames as f64 - self.target_frames) / self.target_frames;
        self.integral_ppm = (self.integral_ppm + error * self.max_correction_ppm * 0.1)
            .clamp(-self.max_correction_ppm, self.max_correction_ppm);
        let requested = error * self.max_correction_ppm + self.integral_ppm;
        self.correction_ppm = requested.clamp(-self.max_correction_ppm, self.max_correction_ppm);
    }

    pub fn correction_ppm(&self) -> f64 {
        self.correction_ppm
    }

    /// Clear learned clock correction at a stream/reconnect boundary while
    /// retaining the prepared nominal ratio and configured bounds.
    pub fn reset(&mut self) {
        self.correction_ppm = 0.0;
        self.integral_ppm = 0.0;
    }

    pub fn adjusted_ratio(&self) -> f64 {
        self.nominal_ratio * (1.0 + self.correction_ppm / 1_000_000.0)
    }
}

#[derive(Debug)]
pub enum ProcessingStage {
    Gain {
        linear: f32,
    },
    Mute {
        muted: bool,
    },
    ChannelMatrix {
        coefficients: Vec<f32>,
    },
    Meter {
        index: usize,
    },
    ParametricEq {
        left: Box<std::sync::Mutex<audiorouter_dsp::ParametricEq>>,
        right: Option<Box<std::sync::Mutex<audiorouter_dsp::ParametricEq>>>,
    },
    Compressor {
        left: Box<std::sync::Mutex<audiorouter_dsp::Compressor>>,
        right: Option<Box<std::sync::Mutex<audiorouter_dsp::Compressor>>>,
    },
    Gate {
        left: Box<std::sync::Mutex<audiorouter_dsp::Gate>>,
        right: Option<Box<std::sync::Mutex<audiorouter_dsp::Gate>>>,
    },
    Limiter {
        limiter: audiorouter_dsp::PeakLimiter,
    },
    Delay {
        left: Box<std::sync::Mutex<audiorouter_dsp::DelayLine>>,
        right: Option<Box<std::sync::Mutex<audiorouter_dsp::DelayLine>>>,
    },
    GraphicEq {
        left: Box<std::sync::Mutex<audiorouter_dsp::GraphicEq>>,
        right: Option<Box<std::sync::Mutex<audiorouter_dsp::GraphicEq>>>,
    },
    Pitch {
        left: Box<std::sync::Mutex<audiorouter_dsp::StreamingPitchShifter>>,
        right: Option<Box<std::sync::Mutex<audiorouter_dsp::StreamingPitchShifter>>>,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum MixerError {
    InvalidChannels,
    InvalidMatrix,
    InputLimit,
    InputCount,
    Block(BlockError),
}

#[derive(Debug, Eq, PartialEq)]
pub enum FanoutError {
    BranchCount,
    Block(BlockError),
}

/// Prepared bounded mixer convergence. Matrices are owned and validated during
/// construction; `process` only clears and accumulates caller-owned blocks.
/// It performs no allocation, locking, I/O, or device access.
pub struct MixerStage {
    output_channels: usize,
    matrices: Vec<Vec<f32>>,
}

impl MixerStage {
    pub fn new(output_channels: usize, matrices: Vec<Vec<f32>>) -> Result<Self, MixerError> {
        if !(1..=MAX_CHANNELS).contains(&output_channels) || matrices.is_empty() {
            return Err(MixerError::InvalidChannels);
        }
        if matrices.len() > MAX_MIXER_INPUTS {
            return Err(MixerError::InputLimit);
        }
        if matrices.iter().any(|matrix| {
            matrix.is_empty()
                || matrix.len() % output_channels != 0
                || matrix.iter().any(|coefficient| {
                    !coefficient.is_finite() || !(-2.0..=2.0).contains(coefficient)
                })
        }) {
            return Err(MixerError::InvalidMatrix);
        }
        Ok(Self {
            output_channels,
            matrices,
        })
    }

    pub fn input_count(&self) -> usize {
        self.matrices.len()
    }

    pub fn process(
        &self,
        destination: &mut AudioBlock,
        sources: &[AudioBlock],
    ) -> Result<(), MixerError> {
        if destination.channels() != self.output_channels || sources.len() != self.matrices.len() {
            return Err(MixerError::InputCount);
        }
        if sources
            .iter()
            .zip(&self.matrices)
            .any(|(source, matrix)| matrix.len() != destination.channels() * source.channels())
        {
            return Err(MixerError::Block(BlockError::ShapeMismatch));
        }
        destination.clear();
        for (source, matrix) in sources.iter().zip(&self.matrices) {
            destination
                .mix_mapped_from(source, matrix)
                .map_err(MixerError::Block)?;
        }
        destination.sanitize_non_finite();
        Ok(())
    }
}

/// A prepared narrow mixer graph: two or more enabled source nodes converge
/// into one mixer node and then feed one destination. The caller supplies the
/// source blocks, mixer scratch block, and destination block at execution time.
pub struct CompiledMixerGraph {
    generation: RuntimeGeneration,
    mixer: MixerStage,
    output_matrix: Vec<f32>,
}

/// A prepared bounded fan-out graph. One enabled source feeds up to eight
/// physical outputs; each branch has its own channel matrix and destination
/// block supplied by the caller.
pub struct CompiledFanoutGraph {
    generation: RuntimeGeneration,
    matrices: Vec<Vec<f32>>,
}

impl CompiledFanoutGraph {
    pub fn generation(&self) -> RuntimeGeneration {
        self.generation
    }

    pub fn branch_count(&self) -> usize {
        self.matrices.len()
    }

    pub fn process(
        &self,
        source: &AudioBlock,
        destinations: &mut [&mut AudioBlock],
    ) -> Result<(), FanoutError> {
        if destinations.len() != self.matrices.len() {
            return Err(FanoutError::BranchCount);
        }
        if destinations
            .iter()
            .zip(&self.matrices)
            .any(|(destination, matrix)| {
                destination.frames() != source.frames()
                    || matrix.len() != destination.channels() * source.channels()
            })
        {
            return Err(FanoutError::Block(BlockError::ShapeMismatch));
        }
        for (destination, matrix) in destinations.iter_mut().zip(&self.matrices) {
            destination
                .map_from(source, matrix)
                .map_err(FanoutError::Block)?;
            destination.sanitize_non_finite();
        }
        Ok(())
    }
}

impl CompiledMixerGraph {
    pub fn generation(&self) -> RuntimeGeneration {
        self.generation
    }

    pub fn input_count(&self) -> usize {
        self.mixer.input_count()
    }

    pub fn process(
        &self,
        sources: &[AudioBlock],
        mixer_scratch: &mut AudioBlock,
        destination: &mut AudioBlock,
    ) -> Result<(), MixerError> {
        self.mixer.process(mixer_scratch, sources)?;
        mixer_scratch.sanitize_non_finite();
        destination
            .map_from(mixer_scratch, &self.output_matrix)
            .map_err(MixerError::Block)?;
        destination.sanitize_non_finite();
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct CallbackMetrics {
    processed_quanta: AtomicU64,
    repaired_samples: AtomicU64,
    clipped_samples: AtomicU64,
    xruns: AtomicU64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockMeterSnapshot {
    pub peak_abs: f32,
    pub clipped_samples: u64,
}

/// Lock-free peak/clipping meter for a prepared node boundary. The maximum
/// uses the monotonic positive-f32 bit representation, so observation never
/// takes a mutex or allocates.
#[derive(Debug, Default)]
pub struct BlockMeter {
    peak_bits: std::sync::atomic::AtomicU32,
    clipped_samples: AtomicU64,
}

impl BlockMeter {
    pub fn observe(&self, block: &AudioBlock) {
        let peak = block.peak_abs();
        let mut current = self.peak_bits.load(Ordering::Relaxed);
        while peak.to_bits() > current {
            match self.peak_bits.compare_exchange_weak(
                current,
                peak.to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
        self.clipped_samples.fetch_add(
            block
                .samples
                .iter()
                .filter(|sample| sample.abs() > 1.0)
                .count() as u64,
            Ordering::Relaxed,
        );
    }

    pub fn peak_abs(&self) -> f32 {
        f32::from_bits(self.peak_bits.load(Ordering::Relaxed))
    }

    pub fn clipped_samples(&self) -> u64 {
        self.clipped_samples.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> BlockMeterSnapshot {
        BlockMeterSnapshot {
            peak_abs: self.peak_abs(),
            clipped_samples: self.clipped_samples(),
        }
    }

    pub fn reset(&self) {
        self.peak_bits.store(0, Ordering::Relaxed);
        self.clipped_samples.store(0, Ordering::Relaxed);
    }
}

impl CallbackMetrics {
    pub fn processed_quanta(&self) -> u64 {
        self.processed_quanta.load(Ordering::Relaxed)
    }

    pub fn repaired_samples(&self) -> u64 {
        self.repaired_samples.load(Ordering::Relaxed)
    }

    pub fn clipped_samples(&self) -> u64 {
        self.clipped_samples.load(Ordering::Relaxed)
    }

    pub fn xruns(&self) -> u64 {
        self.xruns.load(Ordering::Relaxed)
    }

    pub fn record_clipping(&self, samples: usize) {
        self.clipped_samples
            .fetch_add(samples as u64, Ordering::Relaxed);
    }

    pub fn record_xrun(&self) {
        self.xruns.fetch_add(1, Ordering::Relaxed);
    }

    fn record(&self, repaired: usize) {
        self.processed_quanta.fetch_add(1, Ordering::Relaxed);
        self.repaired_samples
            .fetch_add(repaired as u64, Ordering::Relaxed);
    }
}

#[derive(Debug, PartialEq)]
pub enum GraphCompileError {
    InvalidGraph(Vec<audiorouter_domain::ValidationError>),
    UnsupportedTopology,
}

/// Prepare the currently supported processing subset of a validated domain
/// graph. The currently supported edge form is one same-channel linear path;
/// it uses in-place channel matrices. Fan-out, mixer convergence, device
/// activation remain owned by the Windows scheduler milestone. Compatible
/// processing nodes may be bypassed and preserve the dry path; disabled
/// device-bound nodes contribute silence in this supported linear subset.
pub fn compile_session(
    session: &audiorouter_domain::Session,
    generation: RuntimeGeneration,
) -> Result<RuntimeGraph, GraphCompileError> {
    use audiorouter_domain::{validate_session, NodeKind};
    use std::collections::{HashMap, VecDeque};

    validate_session(session).map_err(GraphCompileError::InvalidGraph)?;
    let enabled_edges = session
        .edges
        .iter()
        .filter(|edge| edge.enabled)
        .collect::<Vec<_>>();
    if enabled_edges.is_empty() && session.nodes.len() > 1 {
        return Err(GraphCompileError::UnsupportedTopology);
    }
    if !enabled_edges.is_empty() {
        let mut incoming =
            HashMap::<audiorouter_domain::EntityId, &audiorouter_domain::Edge>::new();
        let mut outgoing =
            HashMap::<audiorouter_domain::EntityId, &audiorouter_domain::Edge>::new();
        for edge in &enabled_edges {
            if incoming
                .insert(edge.destination_node.clone(), edge)
                .is_some()
                || outgoing.insert(edge.source_node.clone(), edge).is_some()
            {
                return Err(GraphCompileError::UnsupportedTopology);
            }
            let source = session
                .nodes
                .iter()
                .find(|node| node.id == edge.source_node)
                .unwrap();
            let destination = session
                .nodes
                .iter()
                .find(|node| node.id == edge.destination_node)
                .unwrap();
            if (source.bypass
                && !matches!(
                    source.kind,
                    NodeKind::Gain
                        | NodeKind::Mute
                        | NodeKind::Meter
                        | NodeKind::ParametricEq
                        | NodeKind::Compressor
                        | NodeKind::Gate
                        | NodeKind::Limiter
                        | NodeKind::Delay
                        | NodeKind::GraphicEq
                        | NodeKind::Pitch
                ))
                || (destination.bypass
                    && !matches!(
                        destination.kind,
                        NodeKind::Gain
                            | NodeKind::Mute
                            | NodeKind::Meter
                            | NodeKind::ParametricEq
                            | NodeKind::Compressor
                            | NodeKind::Gate
                            | NodeKind::Limiter
                            | NodeKind::Delay
                            | NodeKind::GraphicEq
                            | NodeKind::Pitch
                    ))
            {
                return Err(GraphCompileError::UnsupportedTopology);
            }
            let source_port = source
                .ports
                .iter()
                .find(|port| port.name == edge.source_port);
            let destination_port = destination
                .ports
                .iter()
                .find(|port| port.name == edge.destination_port);
            let (Some(source_port), Some(destination_port)) = (source_port, destination_port)
            else {
                return Err(GraphCompileError::UnsupportedTopology);
            };
            if source_port.channels != destination_port.channels
                || edge.matrix.len() != usize::from(source_port.channels).pow(2)
            {
                return Err(GraphCompileError::UnsupportedTopology);
            }
        }
        if enabled_edges.len() + 1
            != enabled_edges
                .iter()
                .flat_map(|edge| [edge.source_node.clone(), edge.destination_node.clone()])
                .collect::<std::collections::HashSet<_>>()
                .len()
        {
            return Err(GraphCompileError::UnsupportedTopology);
        }
    }
    let mut indegree = session
        .nodes
        .iter()
        .map(|node| (node.id.clone(), 0usize))
        .collect::<HashMap<_, _>>();
    let mut outgoing = HashMap::<audiorouter_domain::EntityId, Vec<_>>::new();
    for edge in session.edges.iter().filter(|edge| edge.enabled) {
        *indegree.get_mut(&edge.destination_node).unwrap() += 1;
        outgoing
            .entry(edge.source_node.clone())
            .or_default()
            .push(edge.destination_node.clone());
    }
    let mut ready = session
        .nodes
        .iter()
        .filter(|node| indegree[&node.id] == 0)
        .map(|node| node.id.clone())
        .collect::<VecDeque<_>>();
    let mut order = Vec::with_capacity(session.nodes.len());
    while let Some(node_id) = ready.pop_front() {
        order.push(node_id.clone());
        if let Some(children) = outgoing.get(&node_id) {
            for child in children {
                let count = indegree.get_mut(child).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.push_back(child.clone());
                }
            }
        }
    }

    let mut stages = Vec::new();
    let mut previous_node = None;
    for node_id in order {
        let node = session
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .unwrap();
        if let Some(edge) = enabled_edges
            .iter()
            .find(|edge| edge.destination_node == node_id)
        {
            if previous_node.as_ref() != Some(&edge.source_node) {
                return Err(GraphCompileError::UnsupportedTopology);
            }
            stages.push(ProcessingStage::ChannelMatrix {
                coefficients: edge.matrix.clone(),
            });
        }
        if !node.enabled {
            if matches!(
                node.kind,
                NodeKind::PhysicalInput
                    | NodeKind::ApplicationCapture
                    | NodeKind::EndpointLoopback
                    | NodeKind::PhysicalOutput
                    | NodeKind::VirtualRenderSource
                    | NodeKind::VirtualCaptureSink
                    | NodeKind::Mixer
            ) {
                stages.push(ProcessingStage::Mute { muted: true });
            }
            // A disabled compatible processor uses its defined dry bypass.
            previous_node = Some(node_id);
            continue;
        }
        if node.bypass {
            // Surrounding edge matrix stages preserve the dry signal.
            previous_node = Some(node_id);
            continue;
        }
        match node.kind {
            NodeKind::Gain => {
                let gain_db = node
                    .parameters
                    .get("gainDb")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0) as f32;
                stages.push(ProcessingStage::Gain {
                    linear: 10.0_f32.powf(gain_db / 20.0),
                });
            }
            NodeKind::Mute => {
                let muted = node
                    .parameters
                    .get("muted")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true);
                stages.push(ProcessingStage::Mute { muted });
            }
            NodeKind::ParametricEq => {
                let frequency_hz = node
                    .parameters
                    .get("frequencyHz")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(1_000.0) as f32;
                let q = node
                    .parameters
                    .get("q")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(1.0) as f32;
                let gain_db = node
                    .parameters
                    .get("gainDb")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0) as f32;
                let input_channels = node
                    .ports
                    .iter()
                    .find(|port| port.direction == audiorouter_domain::PortDirection::Input)
                    .map(|port| usize::from(port.channels))
                    .unwrap_or(1);
                let params = audiorouter_dsp::BiquadParams {
                    kind: audiorouter_dsp::FilterKind::Peaking,
                    frequency_hz,
                    q,
                    gain_db,
                    sample_rate: 48_000.0,
                };
                let left = audiorouter_dsp::ParametricEq::new(
                    [Some(params), None, None, None, None, None, None, None],
                    1,
                )
                .map_err(|_| GraphCompileError::UnsupportedTopology)?;
                let right = if input_channels == 2 {
                    Some(
                        audiorouter_dsp::ParametricEq::new(
                            [Some(params), None, None, None, None, None, None, None],
                            1,
                        )
                        .map_err(|_| GraphCompileError::UnsupportedTopology)?,
                    )
                } else {
                    None
                };
                stages.push(ProcessingStage::ParametricEq {
                    left: Box::new(std::sync::Mutex::new(left)),
                    right: right.map(|filter| Box::new(std::sync::Mutex::new(filter))),
                });
            }
            NodeKind::Compressor => {
                let threshold_db = node
                    .parameters
                    .get("thresholdDb")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(-18.0) as f32;
                let ratio = node
                    .parameters
                    .get("ratio")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(3.0) as f32;
                let attack_ms = node
                    .parameters
                    .get("attackMs")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(10.0) as f32;
                let release_ms = node
                    .parameters
                    .get("releaseMs")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(150.0) as f32;
                let makeup_db = node
                    .parameters
                    .get("makeupDb")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0) as f32;
                let input_channels = node
                    .ports
                    .iter()
                    .find(|port| port.direction == audiorouter_domain::PortDirection::Input)
                    .map(|port| usize::from(port.channels))
                    .unwrap_or(1);
                let params = audiorouter_dsp::CompressorParams {
                    threshold_db,
                    ratio,
                    attack_ms,
                    release_ms,
                    knee_db: 0.0,
                    makeup_db,
                    sample_rate: 48_000.0,
                };
                let left = audiorouter_dsp::Compressor::new(params, 1)
                    .map_err(|_| GraphCompileError::UnsupportedTopology)?;
                let right = if input_channels == 2 {
                    Some(
                        audiorouter_dsp::Compressor::new(params, 1)
                            .map_err(|_| GraphCompileError::UnsupportedTopology)?,
                    )
                } else {
                    None
                };
                stages.push(ProcessingStage::Compressor {
                    left: Box::new(std::sync::Mutex::new(left)),
                    right: right.map(|processor| Box::new(std::sync::Mutex::new(processor))),
                });
            }
            NodeKind::Gate => {
                let threshold_db = node
                    .parameters
                    .get("thresholdDb")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(-45.0) as f32;
                let range_db = node
                    .parameters
                    .get("rangeDb")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(60.0) as f32;
                let attack_ms = node
                    .parameters
                    .get("attackMs")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(5.0) as f32;
                let release_ms = node
                    .parameters
                    .get("releaseMs")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(150.0) as f32;
                let input_channels = node
                    .ports
                    .iter()
                    .find(|port| port.direction == audiorouter_domain::PortDirection::Input)
                    .map(|port| usize::from(port.channels))
                    .unwrap_or(1);
                let params = audiorouter_dsp::GateParams {
                    threshold_db,
                    hysteresis_db: 3.0,
                    ratio: 2.0,
                    range_db,
                    attack_ms,
                    hold_ms: 150.0,
                    release_ms,
                    sample_rate: 48_000.0,
                };
                let left = audiorouter_dsp::Gate::new(params, 1)
                    .map_err(|_| GraphCompileError::UnsupportedTopology)?;
                let right = if input_channels == 2 {
                    Some(
                        audiorouter_dsp::Gate::new(params, 1)
                            .map_err(|_| GraphCompileError::UnsupportedTopology)?,
                    )
                } else {
                    None
                };
                stages.push(ProcessingStage::Gate {
                    left: Box::new(std::sync::Mutex::new(left)),
                    right: right.map(|processor| Box::new(std::sync::Mutex::new(processor))),
                });
            }
            NodeKind::Limiter => {
                let ceiling_db = node
                    .parameters
                    .get("ceilingDb")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(-1.0) as f32;
                let limiter = audiorouter_dsp::PeakLimiter::new(audiorouter_dsp::LimiterParams {
                    ceiling_db,
                })
                .map_err(|_| GraphCompileError::UnsupportedTopology)?;
                stages.push(ProcessingStage::Limiter { limiter });
            }
            NodeKind::Delay => {
                let delay_ms = node
                    .parameters
                    .get("delayMs")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0) as f32;
                let input_channels = node
                    .ports
                    .iter()
                    .find(|port| port.direction == audiorouter_domain::PortDirection::Input)
                    .map(|port| usize::from(port.channels))
                    .unwrap_or(1);
                let left = audiorouter_dsp::DelayLine::new(1_000.0, 48_000.0, 1)
                    .and_then(|mut delay| {
                        delay.set_delay_ms(delay_ms)?;
                        Ok(delay)
                    })
                    .map_err(|_| GraphCompileError::UnsupportedTopology)?;
                let right = if input_channels == 2 {
                    Some(
                        audiorouter_dsp::DelayLine::new(1_000.0, 48_000.0, 1)
                            .and_then(|mut delay| {
                                delay.set_delay_ms(delay_ms)?;
                                Ok(delay)
                            })
                            .map_err(|_| GraphCompileError::UnsupportedTopology)?,
                    )
                } else {
                    None
                };
                stages.push(ProcessingStage::Delay {
                    left: Box::new(std::sync::Mutex::new(left)),
                    right: right.map(|delay| Box::new(std::sync::Mutex::new(delay))),
                });
            }
            NodeKind::GraphicEq => {
                let gains = std::array::from_fn(|index| {
                    node.parameters
                        .get(&format!("band{index}Db"))
                        .and_then(|value| value.as_f64())
                        .unwrap_or(0.0) as f32
                });
                let input_channels = node
                    .ports
                    .iter()
                    .find(|port| port.direction == audiorouter_domain::PortDirection::Input)
                    .map(|port| usize::from(port.channels))
                    .unwrap_or(1);
                let left = audiorouter_dsp::GraphicEq::new(gains, 48_000.0, 1)
                    .map_err(|_| GraphCompileError::UnsupportedTopology)?;
                let right = if input_channels == 2 {
                    Some(
                        audiorouter_dsp::GraphicEq::new(gains, 48_000.0, 1)
                            .map_err(|_| GraphCompileError::UnsupportedTopology)?,
                    )
                } else {
                    None
                };
                stages.push(ProcessingStage::GraphicEq {
                    left: Box::new(std::sync::Mutex::new(left)),
                    right: right.map(|processor| Box::new(std::sync::Mutex::new(processor))),
                });
            }
            NodeKind::Pitch => {
                let semitones = node
                    .parameters
                    .get("semitones")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0) as f32;
                let cents = node
                    .parameters
                    .get("cents")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0) as f32;
                let input_channels = node
                    .ports
                    .iter()
                    .find(|port| port.direction == audiorouter_domain::PortDirection::Input)
                    .map(|port| usize::from(port.channels))
                    .unwrap_or(1);
                let params = audiorouter_dsp::PitchShiftParams {
                    semitones,
                    cents,
                    sample_rate: 48_000.0,
                    channels: 1,
                    bypass: false,
                };
                let left = audiorouter_dsp::StreamingPitchShifter::new(params)
                    .map_err(|_| GraphCompileError::UnsupportedTopology)?;
                let right = if input_channels == 2 {
                    Some(
                        audiorouter_dsp::StreamingPitchShifter::new(params)
                            .map_err(|_| GraphCompileError::UnsupportedTopology)?,
                    )
                } else {
                    None
                };
                stages.push(ProcessingStage::Pitch {
                    left: Box::new(std::sync::Mutex::new(left)),
                    right: right.map(|processor| Box::new(std::sync::Mutex::new(processor))),
                });
            }
            NodeKind::PhysicalInput
            | NodeKind::ApplicationCapture
            | NodeKind::EndpointLoopback
            | NodeKind::PhysicalOutput
            | NodeKind::VirtualRenderSource
            | NodeKind::VirtualCaptureSink
            | NodeKind::Mixer => {}
            NodeKind::Meter => {
                let index = stages
                    .iter()
                    .filter_map(|stage| match stage {
                        ProcessingStage::Meter { index } => Some(*index),
                        _ => None,
                    })
                    .max()
                    .map_or(0, |index| index + 1);
                stages.push(ProcessingStage::Meter { index });
            }
        }
        previous_node = Some(node_id);
    }
    Ok(RuntimeGraph::prepare(generation, stages))
}

/// Compile the supported mixer-convergence topology. This intentionally has a
/// separate return type because the ordinary single-block `RuntimeGraph`
/// cannot represent multiple live upstream buffers without silently dropping
/// a branch.
pub fn compile_mixer_session(
    session: &audiorouter_domain::Session,
    generation: RuntimeGeneration,
) -> Result<CompiledMixerGraph, GraphCompileError> {
    use audiorouter_domain::{validate_session, NodeKind};

    validate_session(session).map_err(GraphCompileError::InvalidGraph)?;
    let mixers = session
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Mixer && node.enabled && !node.bypass)
        .collect::<Vec<_>>();
    if mixers.len() != 1 {
        return Err(GraphCompileError::UnsupportedTopology);
    }
    let mixer = mixers[0];
    let incoming = session
        .edges
        .iter()
        .filter(|edge| edge.enabled && edge.destination_node == mixer.id)
        .collect::<Vec<_>>();
    let outgoing = session
        .edges
        .iter()
        .filter(|edge| edge.enabled && edge.source_node == mixer.id)
        .collect::<Vec<_>>();
    if incoming.len() < 2
        || outgoing.len() != 1
        || session.edges.iter().filter(|edge| edge.enabled).count() != incoming.len() + 1
    {
        return Err(GraphCompileError::UnsupportedTopology);
    }
    let mixer_input = mixer
        .ports
        .iter()
        .find(|port| port.name == incoming[0].destination_port)
        .filter(|port| port.direction == audiorouter_domain::PortDirection::Input)
        .ok_or(GraphCompileError::UnsupportedTopology)?;
    let mut source_ids = std::collections::HashSet::with_capacity(incoming.len());
    let mut matrices = Vec::with_capacity(incoming.len());
    for edge in incoming {
        let source = session
            .nodes
            .iter()
            .find(|node| node.id == edge.source_node)
            .ok_or(GraphCompileError::UnsupportedTopology)?;
        if !source.enabled
            || source.bypass
            || edge.destination_port != mixer_input.name
            || !source_ids.insert(source.id.clone())
        {
            return Err(GraphCompileError::UnsupportedTopology);
        }
        let source_port = source
            .ports
            .iter()
            .find(|port| port.name == edge.source_port)
            .filter(|port| port.direction == audiorouter_domain::PortDirection::Output)
            .ok_or(GraphCompileError::UnsupportedTopology)?;
        if edge.matrix.len()
            != usize::from(mixer_input.channels) * usize::from(source_port.channels)
        {
            return Err(GraphCompileError::UnsupportedTopology);
        }
        matrices.push(edge.matrix.clone());
    }
    let output_edge = outgoing[0];
    let destination = session
        .nodes
        .iter()
        .find(|node| node.id == output_edge.destination_node)
        .ok_or(GraphCompileError::UnsupportedTopology)?;
    if destination.kind != NodeKind::PhysicalOutput || !destination.enabled || destination.bypass {
        return Err(GraphCompileError::UnsupportedTopology);
    }
    let mixer_output = mixer
        .ports
        .iter()
        .find(|port| port.name == output_edge.source_port)
        .filter(|port| port.direction == audiorouter_domain::PortDirection::Output)
        .ok_or(GraphCompileError::UnsupportedTopology)?;
    let destination_port = destination
        .ports
        .iter()
        .find(|port| port.name == output_edge.destination_port)
        .filter(|port| port.direction == audiorouter_domain::PortDirection::Input)
        .ok_or(GraphCompileError::UnsupportedTopology)?;
    if output_edge.matrix.len()
        != usize::from(destination_port.channels) * usize::from(mixer_output.channels)
    {
        return Err(GraphCompileError::UnsupportedTopology);
    }
    let mixer = MixerStage::new(usize::from(mixer_input.channels), matrices)
        .map_err(|_| GraphCompileError::UnsupportedTopology)?;
    Ok(CompiledMixerGraph {
        generation,
        mixer,
        output_matrix: output_edge.matrix.clone(),
    })
}

/// Compile the supported fan-out topology: one enabled source and multiple
/// physical-output destinations, with no unrelated enabled edges.
pub fn compile_fanout_session(
    session: &audiorouter_domain::Session,
    generation: RuntimeGeneration,
) -> Result<CompiledFanoutGraph, GraphCompileError> {
    use audiorouter_domain::{validate_session, NodeKind, PortDirection};

    validate_session(session).map_err(GraphCompileError::InvalidGraph)?;
    let enabled_edges = session
        .edges
        .iter()
        .filter(|edge| edge.enabled)
        .collect::<Vec<_>>();
    if !(2..=MAX_FANOUT_BRANCHES).contains(&enabled_edges.len()) {
        return Err(GraphCompileError::UnsupportedTopology);
    }
    let source_id = enabled_edges[0].source_node.clone();
    if enabled_edges
        .iter()
        .any(|edge| edge.source_node != source_id)
    {
        return Err(GraphCompileError::UnsupportedTopology);
    }
    let source = session
        .nodes
        .iter()
        .find(|node| node.id == source_id)
        .filter(|node| node.enabled && !node.bypass)
        .ok_or(GraphCompileError::UnsupportedTopology)?;
    let mut matrices = Vec::with_capacity(enabled_edges.len());
    let mut destinations = std::collections::HashSet::with_capacity(enabled_edges.len());
    for edge in enabled_edges {
        let source_port = source
            .ports
            .iter()
            .find(|port| port.name == edge.source_port)
            .filter(|port| port.direction == PortDirection::Output)
            .ok_or(GraphCompileError::UnsupportedTopology)?;
        let destination = session
            .nodes
            .iter()
            .find(|node| node.id == edge.destination_node)
            .filter(|node| node.kind == NodeKind::PhysicalOutput && node.enabled && !node.bypass)
            .ok_or(GraphCompileError::UnsupportedTopology)?;
        if !destinations.insert(destination.id.clone()) {
            return Err(GraphCompileError::UnsupportedTopology);
        }
        let destination_port = destination
            .ports
            .iter()
            .find(|port| port.name == edge.destination_port)
            .filter(|port| port.direction == PortDirection::Input)
            .ok_or(GraphCompileError::UnsupportedTopology)?;
        if edge.matrix.len()
            != usize::from(destination_port.channels) * usize::from(source_port.channels)
        {
            return Err(GraphCompileError::UnsupportedTopology);
        }
        matrices.push(edge.matrix.clone());
    }
    Ok(CompiledFanoutGraph {
        generation,
        matrices,
    })
}

/// An immutable, prepared processing schedule. The stage vector is created
/// before realtime execution; `process` only mutates the caller's block.
pub struct RuntimeGraph {
    stages: Vec<ProcessingStage>,
    generation: RuntimeGeneration,
    meters: Vec<BlockMeter>,
}

/// Publication point for prepared immutable graphs. Preparation and stores
/// happen on the control thread; readers obtain an owned immutable snapshot,
/// and the previous graph is reclaimed only after its last reader releases it.
pub struct RuntimePublication {
    current: arc_swap::ArcSwapOption<RuntimeGraph>,
}

impl Default for RuntimePublication {
    fn default() -> Self {
        Self {
            current: arc_swap::ArcSwapOption::empty(),
        }
    }
}

impl RuntimePublication {
    pub fn new(initial: Option<RuntimeGraph>) -> Self {
        Self {
            current: arc_swap::ArcSwapOption::from(initial.map(std::sync::Arc::new)),
        }
    }

    /// Publish a fully prepared graph. Existing readers continue using their
    /// old generation while new readers observe the replacement.
    pub fn publish(&self, graph: RuntimeGraph) {
        self.current.store(Some(std::sync::Arc::new(graph)));
    }

    /// Remove the active graph. Existing snapshots remain valid until their
    /// last reader releases them; future readers observe no active runtime.
    pub fn clear(&self) {
        self.current.store(None);
    }

    /// Load the current graph without taking a mutex. `None` means the runtime
    /// has not been activated yet.
    pub fn load(&self) -> Option<std::sync::Arc<RuntimeGraph>> {
        self.current.load_full()
    }
}

/// Integrated block-processing boundary used by a future Windows scheduler.
/// It provides safe silence before activation, publishes only prepared graphs,
/// applies the process-local privacy gate, and exposes callback counters.
#[derive(Default)]
pub struct RuntimeProcessor {
    publication: RuntimePublication,
    privacy_mute: PrivacyMute,
    metrics: CallbackMetrics,
    meter: BlockMeter,
}

impl RuntimeProcessor {
    pub fn publish(&self, graph: RuntimeGraph) {
        graph.reset_meters();
        self.publication.publish(graph);
    }

    /// Compile and atomically activate a supported session candidate.
    ///
    /// Preparation happens before publication, so an invalid or unsupported
    /// candidate cannot replace the currently active generation. The caller
    /// owns the generation identity; native device activation remains outside
    /// this portable boundary.
    pub fn activate_session(
        &self,
        session: &audiorouter_domain::Session,
        generation: RuntimeGeneration,
    ) -> Result<(), GraphCompileError> {
        let graph = compile_session(session, generation)?;
        self.publish(graph);
        Ok(())
    }

    pub fn deactivate(&self) {
        self.publication.clear();
    }

    pub fn set_privacy_muted(&self, muted: bool) {
        self.privacy_mute.set_muted(muted);
    }

    pub fn metrics(&self) -> &CallbackMetrics {
        &self.metrics
    }

    pub fn meter(&self) -> &BlockMeter {
        &self.meter
    }

    /// Read a prepared graph meter from the currently published snapshot.
    /// `None` means there is no active graph or the requested meter index was
    /// not prepared in that graph.
    pub fn meter_snapshot(&self, index: usize) -> Option<BlockMeterSnapshot> {
        self.publication
            .load()
            .and_then(|graph| graph.meter_snapshot(index))
    }

    /// Process one block and return the active generation. Before a graph is
    /// published, the block is cleared and `None` is returned.
    pub fn process(&self, block: &mut AudioBlock) -> Option<RuntimeGeneration> {
        let Some(graph) = self.publication.load() else {
            block.clear();
            return None;
        };
        self.privacy_mute.apply(block);
        graph.process_instrumented(block, &self.metrics);
        self.meter.observe(block);
        Some(graph.generation())
    }

    /// Consume one queued block into caller-owned output storage. This helper
    /// is for the control/worker path: dropping the popped block may reclaim
    /// its backing allocation, so a realtime callback must use a reusable
    /// block pool/ring instead. An empty queue produces safe silence and leaves
    /// the queue's underrun counter as the authoritative indication of missing
    /// input.
    pub fn process_queued(
        &self,
        queue: &AudioBlockQueue,
        output: &mut AudioBlock,
    ) -> Result<Option<RuntimeGeneration>, BlockError> {
        let Some(input) = queue.try_pop() else {
            output.clear();
            return Ok(None);
        };
        if let Err(error) = output.copy_from(&input) {
            output.clear();
            return Err(error);
        }
        Ok(self.process(output))
    }

    /// Process one block from an input ring into a destination-owned block and
    /// submit it to an output ring. This is the reusable-buffer worker path:
    /// the input is recycled and the output is acquired from its own pool, so
    /// rings never transfer allocation ownership between one another. If the
    /// output pool is empty, the input is recycled and an xrun is recorded.
    pub fn process_ring_once(
        &self,
        input: &AudioBlockRing,
        output: &AudioBlockRing,
    ) -> Result<Option<RuntimeGeneration>, BlockError> {
        let Some(block) = input.try_receive() else {
            return Ok(None);
        };
        let Some(mut destination) = output.try_acquire() else {
            input
                .try_recycle(block)
                .map_err(|_| BlockError::ShapeMismatch)?;
            self.metrics.record_xrun();
            return Ok(None);
        };
        if destination.copy_from(&block).is_err() {
            input
                .try_recycle(block)
                .map_err(|_| BlockError::ShapeMismatch)?;
            output
                .try_recycle(destination)
                .map_err(|_| BlockError::ShapeMismatch)?;
            return Err(BlockError::ShapeMismatch);
        }
        input
            .try_recycle(block)
            .map_err(|_| BlockError::ShapeMismatch)?;
        let generation = self.process(&mut destination);
        destination.generation = generation.map_or(0, RuntimeGeneration::value);
        if let Err(destination) = output.try_submit(destination) {
            output
                .try_recycle(destination)
                .map_err(|_| BlockError::ShapeMismatch)?;
            self.metrics.record_xrun();
            return Ok(None);
        }
        Ok(generation)
    }
}

/// Portable ownership boundary for a future native audio callback.
///
/// The scheduler owns fixed-shape input/output rings and performs one bounded
/// nonblocking processing step at a time. Native capture/render adapters can
/// bridge their endpoint buffers to these rings later; this type deliberately
/// does not open devices, wait on events, or change endpoint configuration.
pub struct RealtimeScheduler {
    processor: RuntimeProcessor,
    input: AudioBlockRing,
    output: AudioBlockRing,
}

/// A point-in-time, allocation-free scheduler health snapshot. The counters
/// are monotonic for the scheduler lifetime; callers may diff snapshots when
/// publishing diagnostics outside the realtime boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedulerTelemetry {
    pub input_overruns: u64,
    pub input_underruns: u64,
    pub output_overruns: u64,
    pub output_underruns: u64,
    pub processed_quanta: u64,
    pub repaired_samples: u64,
    pub xruns: u64,
    pub active_generation: Option<RuntimeGeneration>,
}

impl RealtimeScheduler {
    pub fn new(capacity: usize, channels: usize, frames: usize) -> Result<Self, QueueError> {
        Ok(Self {
            processor: RuntimeProcessor::default(),
            input: AudioBlockRing::new(capacity, channels, frames)?,
            output: AudioBlockRing::new(capacity, channels, frames)?,
        })
    }

    pub fn processor(&self) -> &RuntimeProcessor {
        &self.processor
    }

    /// Publish a prepared graph from the control boundary and recycle any
    /// already queued output from the prior generation. A callback that races
    /// this operation can still submit an old block; generation-filtered
    /// receive remains the final protection for that in-flight case.
    pub fn publish(&self, graph: RuntimeGraph) -> usize {
        self.processor.publish(graph);
        self.output.recycle_all()
    }

    /// Prepare and publish a graph through the scheduler's own lifecycle
    /// boundary. Invalid candidates are rejected before replacing the active
    /// generation, preserving the previous graph.
    pub fn activate_session(
        &self,
        session: &audiorouter_domain::Session,
        generation: RuntimeGeneration,
    ) -> Result<(), GraphCompileError> {
        self.processor.activate_session(session, generation)
    }

    /// Remove the active graph and leave subsequent scheduler steps silent.
    pub fn deactivate(&self) {
        self.processor.deactivate();
    }

    pub fn input(&self) -> &AudioBlockRing {
        &self.input
    }

    pub fn output(&self) -> &AudioBlockRing {
        &self.output
    }

    /// Read scheduler and processor health without taking a lock or touching
    /// an endpoint. This method is intended for a control/diagnostics thread;
    /// realtime processing only updates the underlying atomics.
    pub fn telemetry(&self) -> SchedulerTelemetry {
        SchedulerTelemetry {
            input_overruns: self.input.overruns(),
            input_underruns: self.input.underruns(),
            output_overruns: self.output.overruns(),
            output_underruns: self.output.underruns(),
            processed_quanta: self.processor.metrics().processed_quanta(),
            repaired_samples: self.processor.metrics().repaired_samples(),
            xruns: self.processor.metrics().xruns(),
            active_generation: self
                .processor
                .publication
                .load()
                .map(|graph| graph.generation()),
        }
    }

    /// Acquire an input block from the scheduler's bounded pool.
    pub fn acquire_input(&self) -> Option<AudioBlock> {
        self.input.try_acquire()
    }

    /// Submit a block previously acquired with [`Self::acquire_input`].
    pub fn submit_input(&self, block: AudioBlock) -> Result<(), AudioBlock> {
        self.input.try_submit(block)
    }

    pub fn receive_output(&self) -> Option<AudioBlock> {
        self.output.try_receive()
    }

    /// Receive only output produced by the requested prepared graph
    /// generation. Older output is recycled at the scheduler boundary.
    pub fn receive_output_for_generation(
        &self,
        generation: RuntimeGeneration,
    ) -> Option<AudioBlock> {
        self.output.try_receive_generation(generation.value())
    }

    /// Execute one nonblocking ownership-preserving scheduler step.
    pub fn process_once(&self) -> Result<Option<RuntimeGeneration>, BlockError> {
        self.processor.process_ring_once(&self.input, &self.output)
    }
}

impl RuntimeGraph {
    pub fn prepare(generation: RuntimeGeneration, stages: Vec<ProcessingStage>) -> Self {
        let meter_count = stages
            .iter()
            .filter_map(|stage| match stage {
                ProcessingStage::Meter { index } => Some(*index),
                _ => None,
            })
            .max()
            .map_or(0, |index| index + 1);
        Self {
            stages,
            generation,
            meters: (0..meter_count).map(|_| BlockMeter::default()).collect(),
        }
    }

    pub fn generation(&self) -> RuntimeGeneration {
        self.generation
    }

    /// Return the lock-free meter for a prepared Meter node. Meter storage is
    /// allocated during graph preparation and remains valid while the graph
    /// snapshot is retained by its reader.
    pub fn meter(&self, index: usize) -> Option<&BlockMeter> {
        self.meters.get(index)
    }

    pub fn meter_snapshot(&self, index: usize) -> Option<BlockMeterSnapshot> {
        self.meter(index).map(BlockMeter::snapshot)
    }

    /// Clear all prepared node meters at an activation boundary. This is
    /// lock-free and does not change the immutable processing schedule.
    pub fn reset_meters(&self) {
        for meter in &self.meters {
            meter.reset();
        }
    }

    pub fn process(&self, block: &mut AudioBlock) -> usize {
        self.process_inner(block, None)
    }

    /// Process one quantum and update optional atomic callback counters. The
    /// counters never allocate, lock, log, or perform I/O.
    pub fn process_instrumented(&self, block: &mut AudioBlock, metrics: &CallbackMetrics) -> usize {
        self.process_inner(block, Some(metrics))
    }

    fn process_inner(&self, block: &mut AudioBlock, metrics: Option<&CallbackMetrics>) -> usize {
        for stage in &self.stages {
            match stage {
                ProcessingStage::Gain { linear } => block.apply_gain(*linear),
                ProcessingStage::Mute { muted: true } => block.clear(),
                ProcessingStage::Mute { muted: false } => {}
                ProcessingStage::ChannelMatrix { coefficients } => {
                    if block.apply_channel_matrix(coefficients).is_err() {
                        block.clear();
                    }
                }
                ProcessingStage::Meter { index } => {
                    if let Some(meter) = self.meters.get(*index) {
                        meter.observe(block);
                    }
                }
                ProcessingStage::ParametricEq { left, right } => {
                    let Ok(mut left) = left.try_lock() else {
                        block.clear();
                        continue;
                    };
                    if block.channels() == 1 {
                        if let Some(samples) = block.channel_mut(0) {
                            left.process_interleaved(samples);
                        }
                    } else if block.channels() == 2 {
                        if let Some(samples) = block.channel_mut(0) {
                            left.process_interleaved(samples);
                        }
                        let Some(filter) = right.as_ref() else {
                            block.clear();
                            continue;
                        };
                        if let Some(samples) = block.channel_mut(1) {
                            if let Ok(mut filter) = filter.try_lock() {
                                filter.process_interleaved(samples);
                            } else {
                                block.clear();
                            }
                        }
                    }
                }
                ProcessingStage::Compressor { left, right } => {
                    let Ok(mut left) = left.try_lock() else {
                        block.clear();
                        continue;
                    };
                    if let Some(samples) = block.channel_mut(0) {
                        left.process_interleaved(samples);
                    }
                    if block.channels() == 2 {
                        let Some(processor) = right.as_ref() else {
                            block.clear();
                            continue;
                        };
                        if let Some(samples) = block.channel_mut(1) {
                            if let Ok(mut processor) = processor.try_lock() {
                                processor.process_interleaved(samples);
                            } else {
                                block.clear();
                            }
                        }
                    }
                }
                ProcessingStage::Gate { left, right } => {
                    let Ok(mut left) = left.try_lock() else {
                        block.clear();
                        continue;
                    };
                    if let Some(samples) = block.channel_mut(0) {
                        left.process_interleaved(samples);
                    }
                    if block.channels() == 2 {
                        let Some(processor) = right.as_ref() else {
                            block.clear();
                            continue;
                        };
                        if let Some(samples) = block.channel_mut(1) {
                            if let Ok(mut processor) = processor.try_lock() {
                                processor.process_interleaved(samples);
                            } else {
                                block.clear();
                            }
                        }
                    }
                }
                ProcessingStage::Limiter { limiter } => {
                    for channel in 0..block.channels() {
                        if let Some(samples) = block.channel_mut(channel) {
                            limiter.process_interleaved(samples);
                        }
                    }
                }
                ProcessingStage::Delay { left, right } => {
                    let Ok(mut left) = left.try_lock() else {
                        block.clear();
                        continue;
                    };
                    if let Some(samples) = block.channel_mut(0) {
                        left.process_interleaved(samples);
                    }
                    if block.channels() == 2 {
                        let Some(delay) = right.as_ref() else {
                            block.clear();
                            continue;
                        };
                        if let Some(samples) = block.channel_mut(1) {
                            if let Ok(mut delay) = delay.try_lock() {
                                delay.process_interleaved(samples);
                            } else {
                                block.clear();
                            }
                        }
                    }
                }
                ProcessingStage::GraphicEq { left, right } => {
                    let Ok(mut left) = left.try_lock() else {
                        block.clear();
                        continue;
                    };
                    if let Some(samples) = block.channel_mut(0) {
                        left.process_interleaved(samples);
                    }
                    if block.channels() == 2 {
                        let Some(processor) = right.as_ref() else {
                            block.clear();
                            continue;
                        };
                        if let Some(samples) = block.channel_mut(1) {
                            if let Ok(mut processor) = processor.try_lock() {
                                processor.process_interleaved(samples);
                            } else {
                                block.clear();
                            }
                        }
                    }
                }
                ProcessingStage::Pitch { left, right } => {
                    if block.frames() != audiorouter_dsp::StreamingPitchShifter::BLOCK_FRAMES {
                        block.clear();
                        continue;
                    }
                    let process_channel =
                        |processor: &std::sync::Mutex<audiorouter_dsp::StreamingPitchShifter>,
                         samples: &mut [f32]| {
                            let mut output =
                                [0.0_f32; audiorouter_dsp::StreamingPitchShifter::BLOCK_FRAMES];
                            let Ok(mut processor) = processor.try_lock() else {
                                return false;
                            };
                            if processor.process_block(samples, &mut output).is_err() {
                                return false;
                            }
                            samples.copy_from_slice(&output);
                            true
                        };
                    let left_ok = block
                        .channel_mut(0)
                        .is_some_and(|samples| process_channel(left, samples));
                    if !left_ok {
                        block.clear();
                        continue;
                    }
                    if block.channels() == 2 {
                        let right_ok = right.as_ref().is_some_and(|processor| {
                            block
                                .channel_mut(1)
                                .is_some_and(|samples| process_channel(processor, samples))
                        });
                        if !right_ok {
                            block.clear();
                        }
                    }
                }
            }
        }
        let repaired = block.sanitize_non_finite();
        if let Some(metrics) = metrics {
            metrics.record(repaired);
        }
        repaired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_reuses_planar_storage_for_gain_and_mix() {
        let mut destination = AudioBlock::new(2, 4).unwrap();
        let mut source = AudioBlock::new(2, 4).unwrap();
        source.channel_mut(0).unwrap().fill(1.0);
        source.channel_mut(1).unwrap().fill(-0.5);
        destination.mix_from(&source, 2.0).unwrap();
        destination.apply_gain(0.5);
        assert_eq!(destination.channel(0).unwrap(), &[1.0; 4]);
        assert_eq!(destination.channel(1).unwrap(), &[-0.5; 4]);
    }

    #[test]
    fn block_bridges_interleaved_f32_without_shape_guessing() {
        let mut block = AudioBlock::new(2, 3).unwrap();
        block
            .copy_from_interleaved(&[1.0, 10.0, 2.0, 20.0, 3.0, 30.0])
            .unwrap();
        assert_eq!(block.channel(0).unwrap(), &[1.0, 2.0, 3.0]);
        assert_eq!(block.channel(1).unwrap(), &[10.0, 20.0, 30.0]);

        let mut interleaved = [0.0; 6];
        block.copy_to_interleaved(&mut interleaved).unwrap();
        assert_eq!(interleaved, [1.0, 10.0, 2.0, 20.0, 3.0, 30.0]);
        assert_eq!(
            block.copy_from_interleaved(&[1.0, 2.0]),
            Err(BlockError::ShapeMismatch)
        );
        assert_eq!(
            block.copy_to_interleaved(&mut [0.0; 2]),
            Err(BlockError::ShapeMismatch)
        );
    }

    #[test]
    fn block_bridges_pcm16_without_allocation_or_shape_guessing() {
        let mut block = AudioBlock::new(2, 2).unwrap();
        block
            .copy_from_interleaved_pcm16(&[i16::MIN, 0, i16::MAX, 16_384])
            .unwrap();
        assert_eq!(block.channel(0).unwrap(), &[-1.0, 32_767.0 / 32_768.0]);
        assert_eq!(block.channel(1).unwrap(), &[0.0, 0.5]);

        block.channel_mut(0).unwrap()[0] = f32::NAN;
        block.channel_mut(0).unwrap()[1] = 2.0;
        block.channel_mut(1).unwrap()[0] = -2.0;
        block.channel_mut(1).unwrap()[1] = 0.5;
        let mut encoded = [0_i16; 4];
        block.copy_to_interleaved_pcm16(&mut encoded).unwrap();
        assert_eq!(encoded, [0, i16::MIN, i16::MAX, 16_384]);

        assert_eq!(
            block.copy_from_interleaved_pcm16(&[0_i16; 2]),
            Err(BlockError::ShapeMismatch)
        );
        assert_eq!(
            block.copy_to_interleaved_pcm16(&mut [0_i16; 2]),
            Err(BlockError::ShapeMismatch)
        );
    }

    #[test]
    fn pcm16_quantum_adapter_accumulates_split_packets_with_backpressure() {
        let mut adapter = Pcm16QuantumAdapter::new(2).unwrap();
        let mut first = vec![0_i16; 100 * 2];
        for (index, sample) in first.iter_mut().enumerate() {
            *sample = index as i16;
        }
        assert_eq!(adapter.push_interleaved(&first).unwrap(), 100);
        assert_eq!(adapter.pending_frames(), 100);

        let second = vec![1_i16; 40 * 2];
        assert_eq!(adapter.push_interleaved(&second).unwrap(), 28);
        assert_eq!(adapter.pending_frames(), PROCESSING_QUANTUM_FRAMES);
        assert_eq!(adapter.push_interleaved(&second).unwrap(), 0);

        let mut block = AudioBlock::new(2, PROCESSING_QUANTUM_FRAMES).unwrap();
        assert!(adapter.pop_into(&mut block).unwrap());
        assert_eq!(adapter.pending_frames(), 0);
        assert_eq!(block.channel(0).unwrap()[0], 0.0);
        assert_eq!(block.channel(1).unwrap()[0], 1.0 / 32_768.0);
        assert_eq!(block.channel(0).unwrap()[127], 1.0 / 32_768.0);
        assert!(!adapter.pop_into(&mut block).unwrap());

        assert_eq!(
            adapter.push_interleaved(&[0_i16]),
            Err(BlockError::ShapeMismatch)
        );
        assert_eq!(
            adapter.push_packet(&vec![0_i16; (MAX_PCM16_PACKET_FRAMES + 1) * 2]),
            Err(BlockError::InvalidFrameCount)
        );
        assert_eq!(adapter.push_packet(&[]), Err(BlockError::InvalidFrameCount));
        let mut wrong = AudioBlock::new(1, PROCESSING_QUANTUM_FRAMES).unwrap();
        assert_eq!(adapter.pop_into(&mut wrong), Err(BlockError::ShapeMismatch));
    }

    #[test]
    fn pcm16_quantum_adapter_handles_period_boundaries_without_growth() {
        let mut adapter = Pcm16QuantumAdapter::new(1).unwrap();
        let mut block = AudioBlock::new(1, PROCESSING_QUANTUM_FRAMES).unwrap();
        assert_eq!(adapter.push_packet(&vec![1_i16; 127]).unwrap(), 127);
        assert_eq!(adapter.push_packet(&[2_i16]).unwrap(), 1);
        assert!(adapter.pop_into(&mut block).unwrap());
        assert_eq!(block.channel(0).unwrap()[126], 1.0 / 32_768.0);
        assert_eq!(block.channel(0).unwrap()[127], 2.0 / 32_768.0);

        assert_eq!(adapter.push_packet(&vec![3_i16; 128]).unwrap(), 128);
        assert!(adapter.pop_into(&mut block).unwrap());

        let maximum_packet = vec![4_i16; MAX_PCM16_PACKET_FRAMES];
        let mut offset = 0;
        let mut blocks = 0;
        while offset < maximum_packet.len() {
            let consumed = adapter.push_packet(&maximum_packet[offset..]).unwrap();
            assert!(consumed > 0);
            offset += consumed;
            if adapter.pending_frames() == PROCESSING_QUANTUM_FRAMES {
                assert!(adapter.pop_into(&mut block).unwrap());
                blocks += 1;
            }
        }
        assert_eq!(blocks, MAX_PCM16_PACKET_FRAMES / PROCESSING_QUANTUM_FRAMES);
        assert_eq!(adapter.pending_frames(), 0);
    }

    #[test]
    fn scheduler_pressure_is_bounded_nonblocking_and_fail_closed() {
        let scheduler = RealtimeScheduler::new(1, 2, PROCESSING_QUANTUM_FRAMES).unwrap();
        let generation = RuntimeGeneration::new(7);
        scheduler
            .processor()
            .publish(RuntimeGraph::prepare(generation, Vec::new()));

        let first = scheduler.acquire_input().unwrap();
        assert!(scheduler.submit_input(first).is_ok());
        let second = AudioBlock::new(2, PROCESSING_QUANTUM_FRAMES).unwrap();
        assert!(scheduler.submit_input(second).is_err());
        assert_eq!(scheduler.telemetry().input_overruns, 1);

        assert_eq!(scheduler.process_once().unwrap(), Some(generation));
        let held_output = scheduler.receive_output_for_generation(generation).unwrap();

        let third = scheduler.acquire_input().unwrap();
        assert!(scheduler.submit_input(third).is_ok());
        assert_eq!(scheduler.process_once().unwrap(), None);
        assert_eq!(scheduler.telemetry().xruns, 1);
        scheduler.output().try_recycle(held_output).unwrap();
        assert!(scheduler
            .receive_output_for_generation(generation)
            .is_none());
    }

    #[test]
    fn prepared_mixer_converges_multiple_inputs_without_allocating_at_process_boundary() {
        let mixer = MixerStage::new(2, vec![vec![1.0, 0.0, 0.0, 1.0]; 2]).unwrap();
        let mut first = AudioBlock::new(2, 4).unwrap();
        let mut second = AudioBlock::new(2, 4).unwrap();
        first.channel_mut(0).unwrap().fill(0.25);
        first.channel_mut(1).unwrap().fill(-0.5);
        second.channel_mut(0).unwrap().fill(0.75);
        second.channel_mut(1).unwrap().fill(0.5);
        let mut destination = AudioBlock::new(2, 4).unwrap();
        mixer.process(&mut destination, &[first, second]).unwrap();
        assert_eq!(destination.channel(0).unwrap(), &[1.0; 4]);
        assert_eq!(destination.channel(1).unwrap(), &[0.0; 4]);
    }

    #[test]
    fn prepared_mixer_rejects_wrong_input_shapes_before_mutating_output() {
        let mixer = MixerStage::new(1, vec![vec![1.0], vec![1.0]]).unwrap();
        let mut destination = AudioBlock::new(1, 2).unwrap();
        destination.channel_mut(0).unwrap().fill(7.0);
        let source = AudioBlock::new(2, 2).unwrap();
        assert_eq!(
            mixer.process(&mut destination, &[source, AudioBlock::new(1, 2).unwrap()]),
            Err(MixerError::Block(BlockError::ShapeMismatch))
        );
        assert_eq!(destination.channel(0).unwrap(), &[7.0; 2]);
    }

    #[test]
    fn prepared_mixer_enforces_the_eight_input_bound() {
        let matrices = (0..=MAX_MIXER_INPUTS)
            .map(|_| vec![1.0])
            .collect::<Vec<_>>();
        assert!(matches!(
            MixerStage::new(1, matrices),
            Err(MixerError::InputLimit)
        ));
    }

    #[test]
    fn prepared_mixer_rejects_out_of_range_coefficients() {
        assert!(matches!(
            MixerStage::new(1, vec![vec![2.1]]),
            Err(MixerError::InvalidMatrix)
        ));
        assert!(matches!(
            MixerStage::new(1, vec![vec![f32::NAN]]),
            Err(MixerError::InvalidMatrix)
        ));
    }

    #[test]
    fn compiled_branch_paths_sanitize_non_finite_sink_samples() {
        let mixer = MixerStage::new(1, vec![vec![1.0]]).unwrap();
        let mut source = AudioBlock::new(1, 2).unwrap();
        source
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[f32::NAN, f32::INFINITY]);
        let mut destination = AudioBlock::new(1, 2).unwrap();
        mixer
            .process(&mut destination, std::slice::from_ref(&source))
            .unwrap();
        assert_eq!(destination.channel(0).unwrap(), &[0.0, 0.0]);

        let mut fanout_destination = AudioBlock::new(1, 2).unwrap();
        let fanout = CompiledFanoutGraph {
            generation: RuntimeGeneration::new(1),
            matrices: vec![vec![1.0]],
        };
        let mut destinations = [&mut fanout_destination];
        fanout
            .process(&source, &mut destinations)
            .expect("valid fan-out shape");
        assert_eq!(fanout_destination.channel(0).unwrap(), &[0.0, 0.0]);

        let fanout = CompiledFanoutGraph {
            generation: RuntimeGeneration::new(2),
            matrices: vec![vec![1.0], vec![1.0]],
        };
        let mut first = AudioBlock::new(1, 2).unwrap();
        first.channel_mut(0).unwrap().fill(9.0);
        let mut wrong = AudioBlock::new(2, 2).unwrap();
        let mut destinations = [&mut first, &mut wrong];
        assert_eq!(
            fanout.process(&source, &mut destinations),
            Err(FanoutError::Block(BlockError::ShapeMismatch))
        );
        assert_eq!(first.channel(0).unwrap(), &[9.0, 9.0]);
    }

    #[test]
    fn compiler_and_runtime_execute_a_two_source_mixer_graph() {
        use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
        let port = |name: &str, direction| Port {
            name: name.into(),
            direction,
            channels: 1,
        };
        let node = |id: &str, kind, ports| Node {
            id: EntityId::new(id),
            kind,
            type_version: 1,
            name: id.into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports,
        };
        let session = Session {
            id: EntityId::new("mixer-session"),
            name: "mixer".into(),
            schema_version: 1,
            revision: 0,
            nodes: vec![
                node(
                    "left",
                    NodeKind::Gain,
                    vec![port("out", PortDirection::Output)],
                ),
                node(
                    "right",
                    NodeKind::Gain,
                    vec![port("out", PortDirection::Output)],
                ),
                node(
                    "mixer",
                    NodeKind::Mixer,
                    vec![
                        port("in", PortDirection::Input),
                        port("out", PortDirection::Output),
                    ],
                ),
                node(
                    "output",
                    NodeKind::PhysicalOutput,
                    vec![port("in", PortDirection::Input)],
                ),
            ],
            edges: vec![
                Edge {
                    id: EntityId::new("left-edge"),
                    source_node: EntityId::new("left"),
                    source_port: "out".into(),
                    destination_node: EntityId::new("mixer"),
                    destination_port: "in".into(),
                    matrix: vec![1.0],
                    enabled: true,
                },
                Edge {
                    id: EntityId::new("right-edge"),
                    source_node: EntityId::new("right"),
                    source_port: "out".into(),
                    destination_node: EntityId::new("mixer"),
                    destination_port: "in".into(),
                    matrix: vec![1.0],
                    enabled: true,
                },
                Edge {
                    id: EntityId::new("output-edge"),
                    source_node: EntityId::new("mixer"),
                    source_port: "out".into(),
                    destination_node: EntityId::new("output"),
                    destination_port: "in".into(),
                    matrix: vec![1.0],
                    enabled: true,
                },
            ],
        };
        let graph = compile_mixer_session(&session, RuntimeGeneration::new(12)).unwrap();
        assert_eq!(graph.generation(), RuntimeGeneration::new(12));
        let mut left = AudioBlock::new(1, 4).unwrap();
        let mut right = AudioBlock::new(1, 4).unwrap();
        left.channel_mut(0).unwrap().fill(0.25);
        right.channel_mut(0).unwrap().fill(0.5);
        let mut scratch = AudioBlock::new(1, 4).unwrap();
        let mut output = AudioBlock::new(1, 4).unwrap();
        graph
            .process(&[left, right], &mut scratch, &mut output)
            .unwrap();
        assert_eq!(output.channel(0).unwrap(), &[0.75; 4]);

        let mut unrelated = session.clone();
        unrelated.nodes.push(node(
            "unrelated-source",
            NodeKind::Gain,
            vec![port("out", PortDirection::Output)],
        ));
        unrelated.nodes.push(node(
            "unrelated-output",
            NodeKind::PhysicalOutput,
            vec![port("in", PortDirection::Input)],
        ));
        unrelated.edges.push(Edge {
            id: EntityId::new("unrelated-edge"),
            source_node: EntityId::new("unrelated-source"),
            source_port: "out".into(),
            destination_node: EntityId::new("unrelated-output"),
            destination_port: "in".into(),
            matrix: vec![1.0],
            enabled: true,
        });
        assert!(matches!(
            compile_mixer_session(&unrelated, RuntimeGeneration::new(13)),
            Err(GraphCompileError::UnsupportedTopology)
        ));
    }

    #[test]
    fn invalid_shapes_and_bounds_are_rejected_before_allocation() {
        assert!(matches!(
            AudioBlock::new(0, 128),
            Err(BlockError::InvalidChannels)
        ));
        assert!(matches!(
            AudioBlock::new(3, 128),
            Err(BlockError::InvalidChannels)
        ));
        assert!(matches!(
            AudioBlock::new(2, 0),
            Err(BlockError::InvalidFrameCount)
        ));
        assert!(matches!(
            AudioBlock::new(2, 129),
            Err(BlockError::InvalidFrameCount)
        ));
        let mut block = AudioBlock::new(1, 4).unwrap();
        let other = AudioBlock::new(2, 4).unwrap();
        assert_eq!(block.copy_from(&other), Err(BlockError::ShapeMismatch));
        assert_eq!(block.mix_from(&other, 1.0), Err(BlockError::ShapeMismatch));
    }

    #[test]
    fn bounded_audio_queue_is_nonblocking_and_explicit_when_full() {
        assert!(matches!(
            AudioBlockQueue::new(0),
            Err(QueueError::InvalidCapacity)
        ));
        let queue = AudioBlockQueue::new(1).unwrap();
        let first = AudioBlock::new(1, 2).unwrap();
        let second = AudioBlock::new(1, 2).unwrap();
        assert!(queue.is_empty());
        queue.try_push(first).unwrap();
        assert_eq!(queue.len(), 1);
        let returned = queue.try_push(second).unwrap_err();
        assert_eq!(returned.frames(), 2);
        assert_eq!(queue.overruns(), 1);
        assert!(queue.try_pop().is_some());
        assert!(queue.try_pop().is_none());
        assert_eq!(queue.underruns(), 1);
        queue.try_push(AudioBlock::new(1, 2).unwrap()).unwrap();
        assert_eq!(queue.drain(), 1);
        assert!(queue.is_empty());
        assert_eq!(queue.underruns(), 1);

        assert!(matches!(
            AudioBlockQueue::new_for_shape(1, 0, 128),
            Err(QueueError::InvalidShape)
        ));
        let shaped = AudioBlockQueue::new_for_shape(1, 1, 2).unwrap();
        assert!(shaped.try_push(AudioBlock::new(2, 2).unwrap()).is_err());
        assert_eq!(shaped.invalid_blocks(), 1);
    }

    #[test]
    fn audio_queue_constructors_reject_unbounded_capacity_before_allocation() {
        assert!(matches!(
            AudioBlockQueue::new(MAX_AUDIO_QUEUE_BLOCKS + 1),
            Err(QueueError::InvalidCapacity)
        ));
        assert!(matches!(
            AudioBlockQueue::new(usize::MAX),
            Err(QueueError::InvalidCapacity)
        ));
        assert!(matches!(
            AudioBlockQueue::new_for_shape(usize::MAX, 1, 128),
            Err(QueueError::InvalidCapacity)
        ));
        assert!(matches!(
            AudioBlockPool::new(usize::MAX, 1, 128),
            Err(QueueError::InvalidCapacity)
        ));
    }

    #[test]
    fn block_pool_preallocates_and_recycles_only_its_shape() {
        let pool = AudioBlockPool::new(2, 1, 2).unwrap();
        assert_eq!(pool.capacity(), 2);
        assert_eq!(pool.available(), 2);
        let block = pool.try_acquire().unwrap();
        let mut block = block;
        block.channel_mut(0).unwrap().fill(1.0);
        pool.try_release(block).unwrap();
        let block = pool.try_acquire().unwrap();
        assert_eq!(block.channel(0).unwrap(), &[0.0, 0.0]);
        pool.try_release(block).unwrap();
        assert_eq!(pool.available(), 2);
        assert!(pool.try_release(AudioBlock::new(2, 2).unwrap()).is_err());
        assert!(pool.try_acquire().is_some());
        assert!(pool.try_acquire().is_some());
        assert!(pool.try_acquire().is_none());
    }

    #[test]
    fn block_ring_transfers_and_recycles_without_losing_ownership() {
        let ring = AudioBlockRing::new(1, 1, 2).unwrap();
        let mut block = ring.try_acquire().unwrap();
        assert_eq!(ring.available(), 0);
        block.channel_mut(0).unwrap()[0] = 0.5;
        ring.try_submit(block).unwrap();
        assert_eq!(ring.ready(), 1);
        assert!(ring.try_acquire().is_none());

        let block = ring.try_receive().unwrap();
        assert_eq!(block.channel(0).unwrap()[0], 0.5);
        ring.try_recycle(block).unwrap();
        assert_eq!(ring.available(), 1);
        assert_eq!(ring.ready(), 0);
        assert!(ring.try_receive().is_none());
        assert_eq!(ring.underruns(), 1);
    }

    #[test]
    fn virtual_bus_bridge_is_silent_bounded_and_generation_safe() {
        let bridge = VirtualBusBridge::new(1, 1, 2).unwrap();
        let mut inactive = AudioBlock::new(1, 2).unwrap();
        inactive.channel_mut(0).unwrap().fill(1.0);
        assert!(bridge.submit_render(1, inactive).is_err());
        assert_eq!(bridge.process_once(), 0);
        let mut silent = AudioBlock::new(1, 2).unwrap();
        silent.channel_mut(0).unwrap().fill(1.0);
        assert!(!bridge.receive_capture_into(&mut silent).unwrap());
        assert_eq!(silent.channel(0).unwrap(), &[0.0, 0.0]);

        bridge.activate(1).unwrap();
        let mut input = AudioBlock::new(1, 2).unwrap();
        input.channel_mut(0).unwrap().copy_from_slice(&[0.25, 0.5]);
        bridge.submit_render(1, input).unwrap();
        assert_eq!(bridge.process_once(), 1);
        let mut output = AudioBlock::new(1, 2).unwrap();
        assert!(bridge.receive_capture_into(&mut output).unwrap());
        assert_eq!(output.generation(), 1);
        assert_eq!(output.channel(0).unwrap(), &[0.25, 0.5]);

        bridge
            .submit_render(1, AudioBlock::new(1, 2).unwrap())
            .unwrap();
        assert_eq!(bridge.process_once(), 1);
        let mut wrong_shape = AudioBlock::new(2, 2).unwrap();
        assert_eq!(
            bridge.receive_capture_into(&mut wrong_shape),
            Err(BlockError::ShapeMismatch)
        );
        assert!(bridge.try_receive_capture().is_none());
        assert_eq!(
            bridge.activate(1),
            Err(VirtualBusBridgeError::InvalidGeneration)
        );
        assert!(bridge.is_active());

        let stale = AudioBlock::new(1, 2).unwrap();
        assert!(bridge.submit_render(0, stale).is_err());
        bridge.deactivate();
        assert!(!bridge.is_active());
        assert!(!bridge.receive_capture_into(&mut output).unwrap());
        assert_eq!(output.channel(0).unwrap(), &[0.0, 0.0]);
        assert_eq!(bridge.generation(), 1);
        assert_eq!(bridge.dropped(), 0);
        assert_eq!(
            bridge.activate(1),
            Err(VirtualBusBridgeError::InvalidGeneration)
        );
    }

    #[test]
    fn virtual_bus_bridge_drops_when_capture_has_no_free_block() {
        let bridge = VirtualBusBridge::new(1, 1, 2).unwrap();
        bridge.activate(1).unwrap();
        bridge
            .submit_render(1, AudioBlock::new(1, 2).unwrap())
            .unwrap();
        assert_eq!(bridge.process_once(), 1);
        bridge
            .submit_render(1, AudioBlock::new(1, 2).unwrap())
            .unwrap();
        assert_eq!(bridge.process_once(), 0);
        assert_eq!(bridge.dropped(), 1);
    }

    #[test]
    fn virtual_bus_bridge_fans_out_independent_copies() {
        let bridge = VirtualBusBridge::new(1, 1, 2).unwrap();
        let first = AudioBlockRing::new(1, 1, 2).unwrap();
        let second = AudioBlockRing::new(1, 1, 2).unwrap();
        bridge.activate(1).unwrap();
        let mut input = AudioBlock::new(1, 2).unwrap();
        input
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[f32::NAN, f32::INFINITY]);
        bridge.submit_render(1, input).unwrap();
        assert_eq!(bridge.fanout_once(&[&first, &second]), 2);

        let first_block = first.try_receive().unwrap();
        let second_block = second.try_receive().unwrap();
        assert_eq!(first_block.channel(0).unwrap(), &[0.0, 0.0]);
        assert_eq!(second_block.channel(0).unwrap(), &[0.0, 0.0]);
        first.try_recycle(first_block).unwrap();
        second.try_recycle(second_block).unwrap();
    }

    #[test]
    fn virtual_bus_bridge_allows_only_one_concurrent_activation_generation() {
        use std::sync::{Arc, Barrier};
        let bridge = Arc::new(VirtualBusBridge::new(1, 1, 2).unwrap());
        let barrier = Arc::new(Barrier::new(4));
        let handles = (0..4)
            .map(|_| {
                let bridge = Arc::clone(&bridge);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    bridge.activate(1).is_ok()
                })
            })
            .collect::<Vec<_>>();
        let successes = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .filter(|success| *success)
            .count();
        assert_eq!(successes, 1);
        assert_eq!(bridge.generation(), 1);
        assert!(bridge.is_active());
    }

    #[test]
    fn virtual_bus_bridge_serializes_concurrent_replacement_drains() {
        use std::sync::{Arc, Barrier};
        let bridge = Arc::new(VirtualBusBridge::new(1, 1, 2).unwrap());
        bridge.activate(1).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let handles = [2_u64, 3]
            .into_iter()
            .map(|generation| {
                let bridge = Arc::clone(&bridge);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    (generation, bridge.activate(generation).is_ok())
                })
            })
            .collect::<Vec<_>>();
        let results = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        let successful_generation = results
            .iter()
            .filter_map(|(generation, success)| success.then_some(*generation))
            .max()
            .unwrap();
        assert_eq!(bridge.generation(), successful_generation);
        assert!(bridge.is_active());
    }

    #[test]
    fn block_ring_generation_filter_recycles_stale_blocks() {
        let ring = AudioBlockRing::new(1, 1, 2).unwrap();
        ring.try_submit(AudioBlock::new(1, 2).unwrap()).unwrap();
        assert!(ring.try_receive_generation(7).is_none());
        assert_eq!(ring.available(), 1);
        assert_eq!(ring.ready(), 0);
    }

    #[test]
    fn block_ring_returns_full_submission_to_caller() {
        let ring = AudioBlockRing::new(1, 1, 2).unwrap();
        let first = ring.try_acquire().unwrap();
        ring.try_submit(first).unwrap();
        let second = AudioBlock::new(1, 2).unwrap();
        assert!(ring.try_submit(second).is_err());
        assert_eq!(ring.overruns(), 1);
    }

    #[test]
    fn processor_moves_recycled_block_between_rings() {
        let input = AudioBlockRing::new(1, 1, 2).unwrap();
        let output = AudioBlockRing::new(1, 1, 2).unwrap();
        let mut block = input.try_acquire().unwrap();
        block.channel_mut(0).unwrap().fill(0.25);
        input.try_submit(block).unwrap();

        let processor = RuntimeProcessor::default();
        processor.publish(RuntimeGraph::prepare(
            RuntimeGeneration::new(9),
            vec![ProcessingStage::Gain { linear: 2.0 }],
        ));
        assert_eq!(
            processor.process_ring_once(&input, &output).unwrap(),
            Some(RuntimeGeneration::new(9))
        );
        let block = output.try_receive().unwrap();
        assert_eq!(block.channel(0).unwrap(), &[0.5, 0.5]);
        output.try_recycle(block).unwrap();
        assert_eq!(processor.metrics().xruns(), 0);
    }

    #[test]
    fn realtime_scheduler_owns_bounded_rings_and_processes_without_activation() {
        let scheduler = RealtimeScheduler::new(1, 1, 2).unwrap();
        scheduler.processor().publish(RuntimeGraph::prepare(
            RuntimeGeneration::new(21),
            vec![ProcessingStage::Gain { linear: 2.0 }],
        ));
        let mut input = scheduler.acquire_input().unwrap();
        input.channel_mut(0).unwrap().copy_from_slice(&[0.25, -0.5]);
        scheduler.submit_input(input).unwrap();
        assert_eq!(
            scheduler.process_once().unwrap(),
            Some(RuntimeGeneration::new(21))
        );
        let output = scheduler.receive_output().unwrap();
        assert_eq!(output.channel(0).unwrap(), &[0.5, -1.0]);
        assert_eq!(output.generation(), 21);
        scheduler.output().try_recycle(output).unwrap();
        assert_eq!(
            scheduler.telemetry(),
            SchedulerTelemetry {
                input_overruns: 0,
                input_underruns: 0,
                output_overruns: 0,
                output_underruns: 0,
                processed_quanta: 1,
                repaired_samples: 0,
                xruns: 0,
                active_generation: Some(RuntimeGeneration::new(21)),
            }
        );
        assert_eq!(scheduler.input().ready(), 0);
        assert_eq!(scheduler.output().ready(), 0);
        scheduler.deactivate();
        let mut silent = scheduler.acquire_input().unwrap();
        silent.channel_mut(0).unwrap().copy_from_slice(&[1.0, 1.0]);
        scheduler.submit_input(silent).unwrap();
        assert_eq!(scheduler.process_once().unwrap(), None);
        let muted = scheduler.receive_output().unwrap();
        assert_eq!(muted.channel(0).unwrap(), &[0.0, 0.0]);
        scheduler.output().try_recycle(muted).unwrap();
        assert_eq!(scheduler.telemetry().active_generation, None);
    }

    #[test]
    fn realtime_scheduler_filters_outputs_from_replaced_generations() {
        let scheduler = RealtimeScheduler::new(1, 1, 2).unwrap();
        scheduler.processor().publish(RuntimeGraph::prepare(
            RuntimeGeneration::new(30),
            vec![ProcessingStage::Gain { linear: 1.0 }],
        ));
        scheduler
            .submit_input(scheduler.acquire_input().unwrap())
            .unwrap();
        scheduler.process_once().unwrap();
        assert!(scheduler
            .receive_output_for_generation(RuntimeGeneration::new(31))
            .is_none());
        assert_eq!(scheduler.output().available(), 1);
    }

    #[test]
    fn scheduler_publish_recycles_queued_generation_before_replacement() {
        let scheduler = RealtimeScheduler::new(2, 1, 2).unwrap();
        let first_generation = RuntimeGeneration::new(40);
        assert_eq!(
            scheduler.publish(RuntimeGraph::prepare(first_generation, Vec::new())),
            0
        );
        let first = scheduler.acquire_input().unwrap();
        scheduler.submit_input(first).unwrap();
        assert_eq!(scheduler.process_once().unwrap(), Some(first_generation));
        assert_eq!(scheduler.output().ready(), 1);
        assert_eq!(scheduler.output().available(), 1);

        let second_generation = RuntimeGeneration::new(41);
        assert_eq!(
            scheduler.publish(RuntimeGraph::prepare(second_generation, Vec::new())),
            1
        );
        assert_eq!(scheduler.output().ready(), 0);
        assert_eq!(scheduler.output().available(), 2);

        let second = scheduler.acquire_input().unwrap();
        scheduler.submit_input(second).unwrap();
        assert_eq!(scheduler.process_once().unwrap(), Some(second_generation));
        let output = scheduler
            .receive_output_for_generation(second_generation)
            .unwrap();
        assert_eq!(output.generation(), second_generation.value());
        scheduler.output().try_recycle(output).unwrap();
    }

    #[test]
    fn processor_recycles_input_when_output_pool_is_empty() {
        let input = AudioBlockRing::new(1, 1, 2).unwrap();
        let output = AudioBlockRing::new(1, 1, 2).unwrap();
        let held_output = output.try_acquire().unwrap();
        let block = input.try_acquire().unwrap();
        input.try_submit(block).unwrap();

        let processor = RuntimeProcessor::default();
        assert_eq!(processor.process_ring_once(&input, &output).unwrap(), None);
        assert_eq!(input.available(), 1);
        assert_eq!(output.ready(), 0);
        assert_eq!(processor.metrics().xruns(), 1);
        output.try_recycle(held_output).unwrap();
    }

    #[test]
    fn non_finite_samples_are_silenced_and_counted() {
        let mut block = AudioBlock::new(1, 4).unwrap();
        block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[1.0, f32::NAN, f32::INFINITY, -1.0]);
        assert!(!block.all_finite());
        assert_eq!(block.sanitize_non_finite(), 2);
        assert!(block.all_finite());
        assert_eq!(block.channel(0).unwrap(), &[1.0, 0.0, 0.0, -1.0]);
    }

    #[test]
    fn output_clamp_counts_overrange_samples_and_measures_peak() {
        let mut block = AudioBlock::new(1, 4).unwrap();
        block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[-2.0, 0.5, 1.5, f32::NAN]);
        assert_eq!(block.peak_abs(), 2.0);
        assert_eq!(block.clamp_unit(), 2);
        assert_eq!(block.channel(0).unwrap(), &[-1.0, 0.5, 1.0, 0.0]);
        assert_eq!(block.peak_abs(), 1.0);
    }

    #[test]
    fn meter_primitives_expose_channel_peak_and_rms() {
        let mut block = AudioBlock::new(2, 4).unwrap();
        block.channel_mut(0).unwrap().fill(0.5);
        block
            .channel_mut(1)
            .unwrap()
            .copy_from_slice(&[-1.0, 1.0, 0.0, 0.0]);
        assert_eq!(block.channel_peak_abs(0), Some(0.5));
        assert_eq!(block.channel_rms(0), Some(0.5));
        assert!((block.channel_rms(1).unwrap() - 0.70710677).abs() < 0.000001);
        assert!(block.rms() > 0.0);
        assert_eq!(block.channel_peak_abs(2), None);
    }

    #[test]
    fn rolling_rms_window_is_bounded_and_resettable() {
        assert!(matches!(
            RmsWindow::new(0),
            Err(MeterError::InvalidCapacity)
        ));
        let mut window = RmsWindow::new(2).unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().copy_from_slice(&[1.0, 0.0]);
        window.push_block(&block);
        assert_eq!(window.len(), 2);
        assert!((window.rms() - 0.70710677).abs() < 0.000001);
        block.channel_mut(0).unwrap().fill(-1.0);
        window.push_block(&block);
        assert_eq!(window.len(), 2);
        assert_eq!(window.rms(), 1.0);
        window.reset();
        assert_eq!(window.len(), 0);
        assert_eq!(window.rms(), 0.0);
    }

    #[test]
    fn block_meter_tracks_peak_and_clipping_until_reset() {
        let meter = BlockMeter::default();
        let mut block = AudioBlock::new(1, 3).unwrap();
        block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[0.5, -1.5, 0.25]);
        meter.observe(&block);
        assert_eq!(meter.peak_abs(), 1.5);
        assert_eq!(meter.clipped_samples(), 1);
        meter.reset();
        assert_eq!(meter.peak_abs(), 0.0);
        assert_eq!(meter.clipped_samples(), 0);
    }

    #[test]
    fn non_finite_gain_is_safe_silence() {
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        block.apply_gain(f32::NAN);
        assert_eq!(block.channel(0).unwrap(), &[0.0, 0.0]);
    }

    #[test]
    fn explicit_channel_maps_cover_mono_stereo_conversion() {
        let mut mono = AudioBlock::new(1, 2).unwrap();
        mono.channel_mut(0).unwrap().copy_from_slice(&[0.25, -0.5]);
        let mut stereo = AudioBlock::new(2, 2).unwrap();
        stereo.map_from(&mono, &[1.0, 1.0]).unwrap();
        assert_eq!(stereo.channel(0).unwrap(), &[0.25, -0.5]);
        assert_eq!(stereo.channel(1).unwrap(), &[0.25, -0.5]);

        let mut downmix = AudioBlock::new(1, 2).unwrap();
        downmix.map_from(&stereo, &[0.5, 0.5]).unwrap();
        assert_eq!(downmix.channel(0).unwrap(), &[0.25, -0.5]);
    }

    #[test]
    fn mapped_mix_accumulates_without_overwriting_existing_audio() {
        let mut source = AudioBlock::new(1, 2).unwrap();
        source.channel_mut(0).unwrap().fill(0.5);
        let mut destination = AudioBlock::new(2, 2).unwrap();
        destination.channel_mut(0).unwrap().fill(0.25);
        destination.channel_mut(1).unwrap().fill(-0.25);
        destination.mix_mapped_from(&source, &[1.0, 1.0]).unwrap();
        assert_eq!(destination.channel(0).unwrap(), &[0.75; 2]);
        assert_eq!(destination.channel(1).unwrap(), &[0.25; 2]);
    }

    #[test]
    fn channel_map_rejects_wrong_matrix_shape() {
        let source = AudioBlock::new(2, 4).unwrap();
        let mut destination = AudioBlock::new(1, 4).unwrap();
        assert_eq!(
            destination.map_from(&source, &[1.0]),
            Err(BlockError::ShapeMismatch)
        );
    }

    #[test]
    fn linear_resampler_converts_rates_into_preallocated_output() {
        let mut source = AudioBlock::new(1, 4).unwrap();
        source
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[0.0, 1.0, 2.0, 3.0]);
        let mut output = AudioBlock::new(1, 2).unwrap();
        output
            .resample_linear_from(&source, 48_000, 24_000)
            .unwrap();
        assert_eq!(output.channel(0).unwrap(), &[0.0, 2.0]);
        output.resample_linear_with_ratio(&source, 1.5).unwrap();
        assert_eq!(output.channel(0).unwrap(), &[0.0, 1.5]);
        assert!(matches!(
            output.resample_linear_from(&source, 0, 48_000),
            Err(BlockError::InvalidSampleRate)
        ));
        assert!(matches!(
            output.resample_linear_with_ratio(&source, 0.0),
            Err(BlockError::InvalidSampleRate)
        ));
    }

    #[test]
    fn linear_resampler_repairs_non_finite_source_samples() {
        let mut source = AudioBlock::new(1, 2).unwrap();
        source
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[f32::NAN, f32::INFINITY]);
        let mut output = AudioBlock::new(1, 2).unwrap();

        output
            .resample_linear_from(&source, 48_000, 48_000)
            .unwrap();

        assert_eq!(output.channel(0).unwrap(), &[0.0, 0.0]);
        assert!(output.all_finite());
    }

    #[test]
    fn streaming_resampler_preserves_phase_across_source_blocks() {
        let mut resampler = StreamingResampler::new(1, 16).unwrap();
        let mut first = AudioBlock::new(1, 4).unwrap();
        first
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[0.0, 1.0, 2.0, 3.0]);
        let mut second = AudioBlock::new(1, 4).unwrap();
        second
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[4.0, 5.0, 6.0, 7.0]);
        let mut output = AudioBlock::new(1, 2).unwrap();
        assert_eq!(resampler.push(&first).unwrap(), 4);
        assert_eq!(resampler.process(&mut output, 0.5).unwrap(), 2);
        assert_eq!(output.channel(0).unwrap(), &[0.0, 0.5]);
        assert_eq!(resampler.push(&second).unwrap(), 4);
        assert_eq!(resampler.process(&mut output, 0.5).unwrap(), 2);
        assert_eq!(output.channel(0).unwrap(), &[1.0, 1.5]);
        assert_eq!(resampler.queued_frames(), 6);
    }

    #[test]
    fn streaming_resampler_reports_bounded_underflow_and_shape_errors() {
        let mut resampler = StreamingResampler::new(1, 4).unwrap();
        let source = AudioBlock::new(1, 2).unwrap();
        let mut output = AudioBlock::new(1, 2).unwrap();
        output.channel_mut(0).unwrap().fill(1.0);
        assert_eq!(resampler.push(&source).unwrap(), 2);
        assert_eq!(resampler.process(&mut output, 1.0).unwrap(), 0);
        assert_eq!(output.channel(0).unwrap(), &[0.0, 0.0]);
        assert_eq!(resampler.queued_frames(), 2);
        let oversized = AudioBlock::new(1, 3).unwrap();
        assert_eq!(resampler.push(&oversized).unwrap(), 0);
        assert_eq!(resampler.queued_frames(), 2);
        assert!(matches!(
            resampler.process(&mut output, f64::NAN),
            Err(BlockError::InvalidSampleRate)
        ));
        let stereo = AudioBlock::new(2, 2).unwrap();
        assert_eq!(resampler.push(&stereo), Err(BlockError::ShapeMismatch));
    }

    #[test]
    fn drift_controller_clamps_fifo_correction() {
        let mut controller = DriftController::new(48_000, 48_000, 128, 100.0).unwrap();
        controller.observe_queue(256);
        assert_eq!(controller.correction_ppm(), 100.0);
        assert!(controller.adjusted_ratio() > 1.0);
        controller.observe_queue(0);
        assert_eq!(controller.correction_ppm(), -100.0);
        assert!(matches!(
            DriftController::new(0, 48_000, 128, 100.0),
            Err(BlockError::InvalidSampleRate)
        ));
        assert!(matches!(
            DriftController::new(48_000, 48_000, 128, 1_000_000.0),
            Err(BlockError::InvalidDriftCorrection)
        ));
        assert!(matches!(
            DriftController::new(48_000, 48_000, 128, f64::NAN),
            Err(BlockError::InvalidDriftCorrection)
        ));
    }

    #[test]
    fn drift_controller_keeps_both_clock_mismatches_bounded_for_eight_hours() {
        const BLOCKS_IN_EIGHT_HOURS: usize = 4_050_000;
        for clock_error_ppm in [-100.0_f64, 100.0] {
            let mut controller = DriftController::new(48_000, 48_000, 128, 100.0).unwrap();
            let mut queue_frames = 128.0;
            let mut minimum = queue_frames;
            let mut maximum = queue_frames;
            for _ in 0..BLOCKS_IN_EIGHT_HOURS {
                let input_frames = 128.0 * (1.0 + clock_error_ppm / 1_000_000.0);
                let output_frames = 128.0 * controller.adjusted_ratio();
                queue_frames = (queue_frames + input_frames - output_frames).clamp(0.0, 512.0);
                controller.observe_queue(queue_frames.round() as usize);
                minimum = minimum.min(queue_frames);
                maximum = maximum.max(queue_frames);
                assert!(controller.correction_ppm().abs() <= 100.0);
            }
            assert!(
                minimum > 100.0,
                "FIFO underflowed for {clock_error_ppm} ppm"
            );
            assert!(maximum < 160.0, "FIFO drifted for {clock_error_ppm} ppm");
            assert!((controller.correction_ppm() - clock_error_ppm).abs() < 5.0);
        }
    }

    #[test]
    fn drift_controller_reset_discards_previous_stream_correction() {
        let mut controller = DriftController::new(48_000, 44_100, 128, 100.0).unwrap();
        controller.observe_queue(256);
        assert_ne!(controller.correction_ppm(), 0.0);
        controller.reset();
        assert_eq!(controller.correction_ppm(), 0.0);
        assert!((controller.adjusted_ratio() - (48_000.0 / 44_100.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn drift_controller_target_can_be_centered_in_a_bounded_fifo() {
        let mut controller = DriftController::new(48_000, 44_100, 512, 100.0).unwrap();
        controller.observe_queue(512);
        assert_eq!(controller.correction_ppm(), 0.0);
        controller.observe_queue(1024);
        assert!(controller.correction_ppm() > 0.0);
        controller.observe_queue(0);
        assert!(controller.correction_ppm() < 0.0);
    }

    #[test]
    fn prepared_runtime_graph_processes_stages_in_order() {
        let graph = RuntimeGraph::prepare(
            RuntimeGeneration::new(7),
            vec![
                ProcessingStage::Gain { linear: 2.0 },
                ProcessingStage::Mute { muted: false },
                ProcessingStage::Gain { linear: 0.5 },
            ],
        );
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(0.75);
        assert_eq!(graph.process(&mut block), 0);
        assert_eq!(graph.generation().value(), 7);
        assert_eq!(block.channel(0).unwrap(), &[0.75, 0.75]);

        let mute = RuntimeGraph::prepare(
            RuntimeGeneration::new(8),
            vec![ProcessingStage::Mute { muted: true }],
        );
        assert_eq!(mute.process(&mut block), 0);
        assert_eq!(block.channel(0).unwrap(), &[0.0, 0.0]);
    }

    #[test]
    fn prepared_parametric_eq_processes_planar_channels_and_preserves_state() {
        let params = audiorouter_dsp::BiquadParams {
            kind: audiorouter_dsp::FilterKind::Peaking,
            frequency_hz: 1_000.0,
            q: 1.0,
            gain_db: 6.0,
            sample_rate: 48_000.0,
        };
        let stage = ProcessingStage::ParametricEq {
            left: Box::new(std::sync::Mutex::new(
                audiorouter_dsp::ParametricEq::new(
                    [Some(params), None, None, None, None, None, None, None],
                    1,
                )
                .unwrap(),
            )),
            right: None,
        };
        let graph = RuntimeGraph::prepare(RuntimeGeneration::new(10), vec![stage]);
        let mut block = AudioBlock::new(1, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.25);
        let before = block.channel(0).unwrap().to_vec();
        graph.process(&mut block);
        assert!(block.all_finite());
        assert_ne!(block.channel(0).unwrap(), before.as_slice());
    }

    #[test]
    fn prepared_compressor_stage_reduces_sustained_level_and_stays_finite() {
        let processor = audiorouter_dsp::Compressor::new(
            audiorouter_dsp::CompressorParams {
                threshold_db: -18.0,
                ratio: 3.0,
                attack_ms: 10.0,
                release_ms: 150.0,
                knee_db: 0.0,
                makeup_db: 0.0,
                sample_rate: 48_000.0,
            },
            1,
        )
        .unwrap();
        let graph = RuntimeGraph::prepare(
            RuntimeGeneration::new(11),
            vec![ProcessingStage::Compressor {
                left: Box::new(std::sync::Mutex::new(processor)),
                right: None,
            }],
        );
        let mut block = AudioBlock::new(1, 128).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        for _ in 0..64 {
            graph.process(&mut block);
        }
        assert!(block.all_finite());
        assert!(block
            .channel(0)
            .unwrap()
            .iter()
            .all(|sample| sample.abs() < 1.0));
    }

    #[test]
    fn stereo_stateful_stage_silences_when_right_state_is_missing() {
        let processor = audiorouter_dsp::Compressor::new(
            audiorouter_dsp::CompressorParams {
                threshold_db: -18.0,
                ratio: 3.0,
                attack_ms: 10.0,
                release_ms: 150.0,
                knee_db: 0.0,
                makeup_db: 0.0,
                sample_rate: 48_000.0,
            },
            1,
        )
        .unwrap();
        let graph = RuntimeGraph::prepare(
            RuntimeGeneration::new(14),
            vec![ProcessingStage::Compressor {
                left: Box::new(std::sync::Mutex::new(processor)),
                right: None,
            }],
        );
        let mut block = AudioBlock::new(2, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.5);
        block.channel_mut(1).unwrap().fill(-0.5);
        graph.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 128]);
        assert_eq!(block.channel(1).unwrap(), &[0.0; 128]);
    }

    #[test]
    fn prepared_graphic_eq_processes_planar_audio_and_stays_finite() {
        let stage = ProcessingStage::GraphicEq {
            left: Box::new(std::sync::Mutex::new(
                audiorouter_dsp::GraphicEq::new(
                    [6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                    48_000.0,
                    1,
                )
                .unwrap(),
            )),
            right: None,
        };
        let graph = RuntimeGraph::prepare(RuntimeGeneration::new(12), vec![stage]);
        let mut block = AudioBlock::new(1, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.25);
        let before = block.channel(0).unwrap().to_vec();
        graph.process(&mut block);
        assert!(block.all_finite());
        assert_ne!(block.channel(0).unwrap(), before.as_slice());
    }

    #[test]
    fn voice_chain_block_processor_bridges_planar_and_interleaved_storage() {
        let config = audiorouter_dsp::VoiceChainConfig {
            sample_rate: 48_000.0,
            eq: None,
            gate: None,
            compressor: None,
            delay_max_ms: None,
            delay_ms: 0.0,
            limiter: audiorouter_dsp::LimiterParams { ceiling_db: -1.0 },
        };
        let mut processor = VoiceChainBlockProcessor::new(config, 2, 4).unwrap();
        let mut block = AudioBlock::new(2, 4).unwrap();
        block.channel_mut(0).unwrap().fill(2.0);
        block.channel_mut(1).unwrap().fill(-2.0);
        processor.process(&mut block).unwrap();
        let ceiling = 10.0_f32.powf(-1.0 / 20.0);
        assert_eq!(block.channel(0).unwrap(), &[ceiling; 4]);
        assert_eq!(block.channel(1).unwrap(), &[-ceiling; 4]);
        assert!(processor
            .process(&mut AudioBlock::new(1, 4).unwrap())
            .is_err());
        processor.reset();
    }

    #[test]
    fn compiler_prepares_supported_processing_nodes() {
        use audiorouter_domain::{EntityId, Node, NodeKind, Session};

        let session = Session {
            id: EntityId::new("session"),
            name: "processing-only".into(),
            schema_version: 1,
            revision: 1,
            nodes: vec![Node {
                id: EntityId::new("mute"),
                kind: NodeKind::Mute,
                type_version: 1,
                name: "Mute".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![],
            }],
            edges: vec![],
        };
        let graph = compile_session(&session, RuntimeGeneration::new(3)).unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        graph.process(&mut block);
        assert_eq!(graph.generation().value(), 3);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);
    }

    #[test]
    fn compiler_prepares_graphic_eq_node_from_declared_band_parameters() {
        use audiorouter_domain::{EntityId, Node, NodeKind, Session};
        let mut parameters = serde_json::Map::new();
        parameters.insert("band0Db".into(), serde_json::json!(6.0));
        let session = Session {
            id: EntityId::new("graphic-eq-session"),
            name: "graphic-eq".into(),
            schema_version: 1,
            revision: 1,
            nodes: vec![Node {
                id: EntityId::new("graphic-eq"),
                kind: NodeKind::GraphicEq,
                type_version: 1,
                name: "Graphic EQ".into(),
                enabled: true,
                bypass: false,
                parameters,
                ports: vec![],
            }],
            edges: vec![],
        };
        let graph = compile_session(&session, RuntimeGeneration::new(4)).unwrap();
        let mut block = AudioBlock::new(1, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.25);
        let before = block.channel(0).unwrap().to_vec();
        graph.process(&mut block);
        assert!(block.all_finite());
        assert_ne!(block.channel(0).unwrap(), before.as_slice());
    }

    #[test]
    fn prepared_pitch_stage_requires_the_declared_graph_quantum() {
        let stage = ProcessingStage::Pitch {
            left: Box::new(std::sync::Mutex::new(
                audiorouter_dsp::StreamingPitchShifter::new(audiorouter_dsp::PitchShiftParams {
                    semitones: 7.0,
                    cents: 0.0,
                    sample_rate: 48_000.0,
                    channels: 1,
                    bypass: false,
                })
                .unwrap(),
            )),
            right: None,
        };
        let graph = RuntimeGraph::prepare(RuntimeGeneration::new(13), vec![stage]);
        let mut block = AudioBlock::new(1, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.25);
        graph.process(&mut block);
        assert!(block.all_finite());

        let mut wrong_shape = AudioBlock::new(1, 64).unwrap();
        wrong_shape.channel_mut(0).unwrap().fill(0.25);
        graph.process(&mut wrong_shape);
        assert_eq!(wrong_shape.channel(0).unwrap(), &[0.0; 64]);
    }

    #[test]
    fn compiler_rejects_multiple_disconnected_nodes() {
        use audiorouter_domain::{EntityId, Node, NodeKind, Session};

        let session = Session {
            id: EntityId::new("session"),
            name: "disconnected".into(),
            schema_version: 1,
            revision: 1,
            nodes: vec![
                Node {
                    id: EntityId::new("first"),
                    kind: NodeKind::Gain,
                    type_version: 1,
                    name: "First".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![],
                },
                Node {
                    id: EntityId::new("second"),
                    kind: NodeKind::Mute,
                    type_version: 1,
                    name: "Second".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![],
                },
            ],
            edges: vec![],
        };

        assert!(matches!(
            compile_session(&session, RuntimeGeneration::new(3)),
            Err(GraphCompileError::UnsupportedTopology)
        ));
    }

    #[test]
    fn compiler_prepares_a_valid_linear_edge_and_channel_matrix() {
        use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};

        let session = Session {
            id: EntityId::new("session"),
            name: "linear-route".into(),
            schema_version: 1,
            revision: 1,
            nodes: vec![
                Node {
                    id: EntityId::new("source"),
                    kind: NodeKind::PhysicalInput,
                    type_version: 1,
                    name: "Source".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    }],
                },
                Node {
                    id: EntityId::new("sink"),
                    kind: NodeKind::PhysicalOutput,
                    type_version: 1,
                    name: "Sink".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Input,
                        channels: 1,
                    }],
                },
            ],
            edges: vec![Edge {
                id: EntityId::new("route"),
                source_node: EntityId::new("source"),
                source_port: "main".into(),
                destination_node: EntityId::new("sink"),
                destination_port: "main".into(),
                matrix: vec![0.5],
                enabled: true,
            }],
        };
        let graph = compile_session(&session, RuntimeGeneration::new(4)).unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        graph.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.5, 0.5]);
    }

    #[test]
    fn compiler_preserves_dry_path_through_bypassed_processing_node() {
        use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
        let source = Node {
            id: EntityId::new("source"),
            kind: NodeKind::PhysicalInput,
            type_version: 1,
            name: "source".into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![Port {
                name: "main".into(),
                direction: PortDirection::Output,
                channels: 1,
            }],
        };
        let gain = Node {
            id: EntityId::new("gain"),
            kind: NodeKind::Gain,
            type_version: 1,
            name: "gain".into(),
            enabled: true,
            bypass: true,
            parameters: Default::default(),
            ports: vec![
                Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels: 1,
                },
                Port {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    channels: 1,
                },
            ],
        };
        let sink = Node {
            id: EntityId::new("sink"),
            kind: NodeKind::PhysicalOutput,
            type_version: 1,
            name: "sink".into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![Port {
                name: "main".into(),
                direction: PortDirection::Input,
                channels: 1,
            }],
        };
        let session = Session {
            id: EntityId::new("session"),
            name: "bypass-route".into(),
            schema_version: 1,
            revision: 1,
            nodes: vec![source, gain, sink],
            edges: vec![
                Edge {
                    id: EntityId::new("source-gain"),
                    source_node: EntityId::new("source"),
                    source_port: "main".into(),
                    destination_node: EntityId::new("gain"),
                    destination_port: "in".into(),
                    matrix: vec![1.0],
                    enabled: true,
                },
                Edge {
                    id: EntityId::new("gain-sink"),
                    source_node: EntityId::new("gain"),
                    source_port: "out".into(),
                    destination_node: EntityId::new("sink"),
                    destination_port: "main".into(),
                    matrix: vec![1.0],
                    enabled: true,
                },
            ],
        };
        let graph = compile_session(&session, RuntimeGeneration::new(9)).unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().copy_from_slice(&[0.25, -0.5]);
        graph.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.25, -0.5]);

        let mut configured = session.clone();
        configured.nodes[1].bypass = false;
        configured.nodes[1]
            .parameters
            .insert("gainDb".into(), serde_json::json!(-6.0));
        let graph = compile_session(&configured, RuntimeGeneration::new(12)).unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        graph.process(&mut block);
        assert!((block.channel(0).unwrap()[0] - 0.5011872).abs() < 0.00001);

        let mut disabled_source = session.clone();
        disabled_source.nodes[0].enabled = false;
        let graph = compile_session(&disabled_source, RuntimeGeneration::new(10)).unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        graph.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.0, 0.0]);

        let mut disabled_sink = session;
        disabled_sink.nodes[2].enabled = false;
        let graph = compile_session(&disabled_sink, RuntimeGeneration::new(11)).unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        graph.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.0, 0.0]);
    }

    #[test]
    fn compiler_rejects_fan_out_until_branch_buffers_exist() {
        use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
        let output = |id: &str| Node {
            id: EntityId::new(id),
            kind: NodeKind::PhysicalOutput,
            type_version: 1,
            name: id.into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![Port {
                name: "main".into(),
                direction: PortDirection::Input,
                channels: 1,
            }],
        };
        let session = Session {
            id: EntityId::new("session"),
            name: "fan-out".into(),
            schema_version: 1,
            revision: 1,
            nodes: vec![
                Node {
                    id: EntityId::new("source"),
                    kind: NodeKind::PhysicalInput,
                    type_version: 1,
                    name: "Source".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    }],
                },
                output("left"),
                output("right"),
            ],
            edges: vec![
                Edge {
                    id: EntityId::new("source-left"),
                    source_node: EntityId::new("source"),
                    source_port: "main".into(),
                    destination_node: EntityId::new("left"),
                    destination_port: "main".into(),
                    matrix: vec![1.0],
                    enabled: true,
                },
                Edge {
                    id: EntityId::new("source-right"),
                    source_node: EntityId::new("source"),
                    source_port: "main".into(),
                    destination_node: EntityId::new("right"),
                    destination_port: "main".into(),
                    matrix: vec![1.0],
                    enabled: true,
                },
            ],
        };
        assert!(matches!(
            compile_session(&session, RuntimeGeneration::new(5)),
            Err(GraphCompileError::UnsupportedTopology)
        ));
        let graph = compile_fanout_session(&session, RuntimeGeneration::new(6)).unwrap();
        assert_eq!(graph.branch_count(), 2);
        let mut source = AudioBlock::new(1, 2).unwrap();
        source.channel_mut(0).unwrap().fill(0.75);
        let mut left = AudioBlock::new(1, 2).unwrap();
        let mut right = AudioBlock::new(1, 2).unwrap();
        let mut destinations: [&mut AudioBlock; 2] = [&mut left, &mut right];
        graph.process(&source, &mut destinations).unwrap();
        assert_eq!(left.channel(0).unwrap(), &[0.75, 0.75]);
        assert_eq!(right.channel(0).unwrap(), &[0.75, 0.75]);
    }

    #[test]
    fn publication_replaces_generation_without_invalidating_old_reader() {
        let first = RuntimeGraph::prepare(RuntimeGeneration::new(1), vec![]);
        let second = RuntimeGraph::prepare(RuntimeGeneration::new(2), vec![]);
        let publication = RuntimePublication::new(Some(first));
        let old_reader = publication.load().unwrap();
        publication.publish(second);
        assert_eq!(old_reader.generation().value(), 1);
        assert_eq!(publication.load().unwrap().generation().value(), 2);
    }

    #[test]
    fn concurrent_graph_publication_never_processes_a_torn_generation() {
        use std::sync::{Arc, Barrier};
        let processor = Arc::new(RuntimeProcessor::default());
        processor.publish(RuntimeGraph::prepare(
            RuntimeGeneration::new(1),
            vec![ProcessingStage::Gain { linear: 1.0 }],
        ));
        let barrier = Arc::new(Barrier::new(2));
        let reader_processor = Arc::clone(&processor);
        let reader_barrier = Arc::clone(&barrier);
        let reader = std::thread::spawn(move || {
            reader_barrier.wait();
            for _ in 0..10_000 {
                let mut block = AudioBlock::new(1, 1).unwrap();
                block.channel_mut(0).unwrap()[0] = 1.0;
                let generation = reader_processor.process(&mut block).unwrap();
                let sample = block.channel(0).unwrap()[0];
                assert!(
                    (generation.value() == 1 && sample == 1.0)
                        || (generation.value() == 2 && sample == 2.0)
                );
            }
        });
        let writer_processor = Arc::clone(&processor);
        let writer = std::thread::spawn(move || {
            barrier.wait();
            for _ in 0..100 {
                writer_processor.publish(RuntimeGraph::prepare(
                    RuntimeGeneration::new(2),
                    vec![ProcessingStage::Gain { linear: 2.0 }],
                ));
                writer_processor.publish(RuntimeGraph::prepare(
                    RuntimeGeneration::new(1),
                    vec![ProcessingStage::Gain { linear: 1.0 }],
                ));
            }
        });
        reader.join().unwrap();
        writer.join().unwrap();
    }

    #[test]
    fn prepared_meter_stages_publish_each_boundary_without_allocating() {
        let graph = RuntimeGraph::prepare(
            RuntimeGeneration::new(3),
            vec![
                ProcessingStage::Meter { index: 0 },
                ProcessingStage::Gain { linear: 2.0 },
                ProcessingStage::Meter { index: 1 },
            ],
        );
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().copy_from_slice(&[0.5, -1.5]);
        graph.process(&mut block);

        assert_eq!(graph.meter(0).unwrap().peak_abs(), 1.5);
        assert_eq!(graph.meter(0).unwrap().clipped_samples(), 1);
        assert_eq!(graph.meter(1).unwrap().peak_abs(), 3.0);
        assert_eq!(graph.meter(1).unwrap().clipped_samples(), 1);
        assert_eq!(
            graph.meter_snapshot(1),
            Some(BlockMeterSnapshot {
                peak_abs: 3.0,
                clipped_samples: 1,
            })
        );
        graph.reset_meters();
        assert_eq!(graph.meter_snapshot(0).unwrap().peak_abs, 0.0);
        assert_eq!(graph.meter_snapshot(1).unwrap().clipped_samples, 0);
        assert!(graph.meter(2).is_none());
    }

    #[test]
    fn instrumented_processing_records_only_atomic_counters() {
        let graph = RuntimeGraph::prepare(RuntimeGeneration::new(1), vec![]);
        let metrics = CallbackMetrics::default();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[f32::NAN, 1.0]);
        assert_eq!(graph.process_instrumented(&mut block, &metrics), 1);
        assert_eq!(metrics.processed_quanta(), 1);
        assert_eq!(metrics.repaired_samples(), 1);
        metrics.record_clipping(2);
        metrics.record_xrun();
        assert_eq!(metrics.clipped_samples(), 2);
        assert_eq!(metrics.xruns(), 1);
    }

    #[test]
    fn gain_ramp_reaches_target_without_a_block_discontinuity() {
        let mut ramp = GainRamp::new(0.0);
        ramp.set_target(1.0, 4);
        let mut block = AudioBlock::new(1, 4).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        ramp.apply(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.25, 0.5, 0.75, 1.0]);
        assert_eq!(ramp.current(), 1.0);

        ramp.set_target(0.0, 0);
        assert_eq!(ramp.current(), 0.0);
    }

    #[test]
    fn privacy_mute_silences_only_the_process_local_block() {
        let mute = PrivacyMute::default();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        mute.set_muted(true);
        mute.apply(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);
        assert!(mute.is_muted());
        mute.set_muted(false);
        block.channel_mut(0).unwrap().fill(1.0);
        mute.apply(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[1.0; 2]);
    }

    #[test]
    fn processor_silences_before_activation_and_applies_published_generation() {
        let processor = RuntimeProcessor::default();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        assert_eq!(processor.process(&mut block), None);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);

        processor.publish(RuntimeGraph::prepare(
            RuntimeGeneration::new(9),
            vec![
                ProcessingStage::Meter { index: 0 },
                ProcessingStage::Gain { linear: 2.0 },
            ],
        ));
        block.channel_mut(0).unwrap().fill(1.0);
        assert_eq!(
            processor.process(&mut block).map(RuntimeGeneration::value),
            Some(9)
        );
        assert_eq!(block.channel(0).unwrap(), &[2.0; 2]);
        assert_eq!(processor.meter().peak_abs(), 2.0);
        assert_eq!(processor.meter().clipped_samples(), 2);
        assert_eq!(
            processor.meter_snapshot(0),
            Some(BlockMeterSnapshot {
                peak_abs: 1.0,
                clipped_samples: 0,
            })
        );
        assert_eq!(processor.meter_snapshot(1), None);
        processor.set_privacy_muted(true);
        processor.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);
        assert_eq!(processor.metrics().processed_quanta(), 2);
        processor.deactivate();
        assert_eq!(processor.meter_snapshot(0), None);
        block.channel_mut(0).unwrap().fill(1.0);
        assert_eq!(processor.process(&mut block), None);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);

        let queue = AudioBlockQueue::new_for_shape(1, 1, 2).unwrap();
        let mut queued = AudioBlock::new(1, 2).unwrap();
        queued.channel_mut(0).unwrap().fill(0.5);
        processor.set_privacy_muted(false);
        queue.try_push(queued).unwrap();
        processor.publish(RuntimeGraph::prepare(RuntimeGeneration::new(10), vec![]));
        assert_eq!(
            processor
                .process_queued(&queue, &mut block)
                .unwrap()
                .map(RuntimeGeneration::value),
            Some(10)
        );
        assert_eq!(block.channel(0).unwrap(), &[0.5; 2]);
        assert_eq!(processor.process_queued(&queue, &mut block).unwrap(), None);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);
    }

    #[test]
    fn session_activation_keeps_previous_generation_when_preparation_fails() {
        let session: audiorouter_domain::Session =
            serde_json::from_str(include_str!("../../../tests/fixtures/valid-session.json"))
                .unwrap();
        let processor = RuntimeProcessor::default();
        processor
            .activate_session(&session, RuntimeGeneration::new(20))
            .unwrap();

        let mut invalid = session.clone();
        invalid.edges[0].matrix.clear();
        assert!(matches!(
            processor.activate_session(&invalid, RuntimeGeneration::new(21)),
            Err(GraphCompileError::InvalidGraph(_))
        ));

        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(0.25);
        assert_eq!(
            processor.process(&mut block).map(RuntimeGeneration::value),
            Some(20)
        );
        assert_eq!(block.channel(0).unwrap(), &[0.25; 2]);
    }

    #[test]
    fn latency_compensation_aligns_branches_without_exceeding_budget() {
        assert_eq!(
            calculate_latency_compensation(&[240, 480, 4_800], 48_000).unwrap(),
            vec![4_560, 4_320, 0]
        );
        assert_eq!(
            calculate_latency_compensation(&[0, 12_001], 48_000),
            Err(LatencyCompensationError::OverBudget {
                required_samples: 12_001,
                maximum_samples: 12_000,
            })
        );
        assert_eq!(
            calculate_latency_compensation(&[], 48_000),
            Err(LatencyCompensationError::Empty)
        );
        assert_eq!(
            calculate_latency_compensation(&[1, 2], 7_999),
            Err(LatencyCompensationError::InvalidSampleRate)
        );
    }

    #[test]
    fn fixed_delay_applies_compensation_without_allocating_and_resets() {
        let mut delay = FixedDelay::new(1, 2).unwrap();
        delay.set_delay_frames(2).unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().copy_from_slice(&[1.0, 2.0]);
        delay.process(&mut block).unwrap();
        assert_eq!(block.channel(0).unwrap(), &[0.0, 0.0]);
        let mut next = AudioBlock::new(1, 2).unwrap();
        next.channel_mut(0).unwrap().copy_from_slice(&[3.0, 4.0]);
        delay.process(&mut next).unwrap();
        assert_eq!(next.channel(0).unwrap(), &[1.0, 2.0]);
        delay.reset();
        next.channel_mut(0).unwrap().copy_from_slice(&[5.0, 6.0]);
        delay.process(&mut next).unwrap();
        assert_eq!(next.channel(0).unwrap(), &[0.0, 0.0]);
        assert_eq!(
            delay.set_delay_frames(3),
            Err(FixedDelayError::InvalidDelay)
        );
        assert!(matches!(
            FixedDelay::new(1, MAX_DELAY_FRAMES + 1),
            Err(FixedDelayError::InvalidCapacity)
        ));
    }

    #[test]
    fn changing_fixed_delay_discards_old_ring_history() {
        let mut delay = FixedDelay::new(1, 4).unwrap();
        delay.set_delay_frames(2).unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        delay.process(&mut block).unwrap();
        delay.set_delay_frames(1).unwrap();
        block.channel_mut(0).unwrap().fill(2.0);
        delay.process(&mut block).unwrap();
        assert_eq!(block.channel(0).unwrap(), &[0.0, 2.0]);
    }
}
