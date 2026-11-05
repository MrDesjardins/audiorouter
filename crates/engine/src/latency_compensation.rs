use crate::{AudioBlock, MAX_CHANNELS, MAX_DELAY_FRAMES, MAX_EXTRA_COMPENSATION_MS};

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
        let (channels, capacity, delay) = (self.channels, self.capacity_frames, self.delay_frames);
        let frames = block.frames();
        let start = self.write_frame;
        // Channels are independent, so each planar channel runs through the
        // ring on its own; the ring keeps its interleaved layout.
        for (channel, samples) in block.samples.chunks_exact_mut(frames).enumerate() {
            let mut write = start;
            let mut read = (start + capacity - delay) % capacity;
            for sample in samples {
                let input = if sample.is_finite() { *sample } else { 0.0 };
                let delayed = self.buffer[read * channels + channel];
                self.buffer[write * channels + channel] = input;
                *sample = if delay == 0 { input } else { delayed };
                write = if write + 1 == capacity { 0 } else { write + 1 };
                read = if read + 1 == capacity { 0 } else { read + 1 };
            }
        }
        self.write_frame = (start + frames) % capacity;
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
