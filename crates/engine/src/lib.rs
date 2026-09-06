//! Allocation-free audio-block primitives for the M02 realtime boundary.
//!
//! Construction and graph preparation happen off the callback thread. Once an
//! `AudioBlock` exists, the operations below reuse its storage and perform no
//! heap allocation, locking, I/O, or logging.

use std::sync::atomic::{AtomicU64, Ordering};

pub const INTERNAL_SAMPLE_RATE_HZ: u32 = 48_000;
pub const PROCESSING_QUANTUM_FRAMES: usize = 128;
pub const MAX_CHANNELS: usize = 2;
pub const MAX_MIXER_INPUTS: usize = 8;
pub const MAX_FANOUT_BRANCHES: usize = 8;

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

    pub fn try_recycle(&self, block: AudioBlock) -> Result<(), AudioBlock> {
        self.free.try_release(block)
    }

    pub fn overruns(&self) -> u64 {
        self.ready.overruns()
    }

    pub fn underruns(&self) -> u64 {
        self.ready.underruns()
    }
}

impl AudioBlockPool {
    pub fn new(capacity: usize, channels: usize, frames: usize) -> Result<Self, QueueError> {
        if capacity == 0 {
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
        self.blocks.push(block)
    }
}

impl AudioBlockQueue {
    pub fn new(capacity: usize) -> Result<Self, QueueError> {
        if capacity == 0 {
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
        if capacity == 0 {
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
}

/// A preallocated planar float32 block. Samples are stored channel-major:
/// `channel * frames + frame`.
#[derive(Debug)]
pub struct AudioBlock {
    channels: usize,
    frames: usize,
    samples: Vec<f32>,
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
        })
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
        if self.channels != source.channels {
            return Err(BlockError::ShapeMismatch);
        }
        if input_rate_hz == 0 || output_rate_hz == 0 {
            return Err(BlockError::InvalidSampleRate);
        }
        let ratio = input_rate_hz as f64 / output_rate_hz as f64;
        for destination_channel in 0..self.channels {
            let destination = self.channel_mut(destination_channel).unwrap();
            let input = source.channel(destination_channel).unwrap();
            for (frame, sample) in destination.iter_mut().enumerate() {
                let position = frame as f64 * ratio;
                let lower = position.floor() as usize;
                let lower = lower.min(source.frames - 1);
                let upper = (lower + 1).min(source.frames - 1);
                let fraction = (position - lower as f64) as f32;
                *sample = input[lower] + (input[upper] - input[lower]) * fraction;
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
            || max_correction_ppm < 0.0
        {
            return Err(BlockError::InvalidSampleRate);
        }
        Ok(Self {
            nominal_ratio: input_rate_hz as f64 / output_rate_hz as f64,
            correction_ppm: 0.0,
            target_frames: target_frames as f64,
            max_correction_ppm,
        })
    }

    /// Update correction from bounded FIFO occupancy. The proportional gain
    /// is deliberately conservative; callers still need xrun/discontinuity
    /// policy around the stream scheduler.
    pub fn observe_queue(&mut self, queue_frames: usize) {
        let error = (queue_frames as f64 - self.target_frames) / self.target_frames;
        let requested = error * self.max_correction_ppm;
        self.correction_ppm = requested.clamp(-self.max_correction_ppm, self.max_correction_ppm);
    }

    pub fn correction_ppm(&self) -> f64 {
        self.correction_ppm
    }

    pub fn adjusted_ratio(&self) -> f64 {
        self.nominal_ratio * (1.0 + self.correction_ppm / 1_000_000.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessingStage {
    Gain { linear: f32 },
    Mute { muted: bool },
    ChannelMatrix { coefficients: Vec<f32> },
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
                || matrix.iter().any(|coefficient| !coefficient.is_finite())
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
/// activation, and disabled-node routing remain owned by the Windows
/// scheduler milestone. Gain has no scalar field in the v1 domain contract
/// yet, so a non-bypassed gain is unity.
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
            if !source.enabled || source.bypass || !destination.enabled || destination.bypass {
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
        if !node.enabled || node.bypass {
            continue;
        }
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
        match node.kind {
            NodeKind::Gain => stages.push(ProcessingStage::Gain { linear: 1.0 }),
            NodeKind::Mute => stages.push(ProcessingStage::Mute { muted: true }),
            NodeKind::PhysicalInput
            | NodeKind::ApplicationCapture
            | NodeKind::EndpointLoopback
            | NodeKind::PhysicalOutput
            | NodeKind::Mixer
            | NodeKind::Meter => {}
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
        self.publication.publish(graph);
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

impl RuntimeGraph {
    pub fn prepare(generation: RuntimeGeneration, stages: Vec<ProcessingStage>) -> Self {
        Self { stages, generation }
    }

    pub fn generation(&self) -> RuntimeGeneration {
        self.generation
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
            name: id.into(),
            enabled: true,
            bypass: false,
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
        assert!(matches!(
            output.resample_linear_from(&source, 0, 48_000),
            Err(BlockError::InvalidSampleRate)
        ));
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
                name: "Mute".into(),
                enabled: true,
                bypass: false,
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
                    name: "Source".into(),
                    enabled: true,
                    bypass: false,
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    }],
                },
                Node {
                    id: EntityId::new("sink"),
                    kind: NodeKind::PhysicalOutput,
                    name: "Sink".into(),
                    enabled: true,
                    bypass: false,
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
    fn compiler_rejects_fan_out_until_branch_buffers_exist() {
        use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
        let output = |id: &str| Node {
            id: EntityId::new(id),
            kind: NodeKind::PhysicalOutput,
            name: id.into(),
            enabled: true,
            bypass: false,
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
                    name: "Source".into(),
                    enabled: true,
                    bypass: false,
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
            vec![ProcessingStage::Gain { linear: 2.0 }],
        ));
        block.channel_mut(0).unwrap().fill(1.0);
        assert_eq!(
            processor.process(&mut block).map(RuntimeGeneration::value),
            Some(9)
        );
        assert_eq!(block.channel(0).unwrap(), &[2.0; 2]);
        assert_eq!(processor.meter().peak_abs(), 2.0);
        assert_eq!(processor.meter().clipped_samples(), 2);
        processor.set_privacy_muted(true);
        processor.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);
        assert_eq!(processor.metrics().processed_quanta(), 2);
        processor.deactivate();
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
}
