//! Allocation-free audio-block primitives for the M02 realtime boundary.
//!
//! Construction and graph preparation happen off the callback thread. Once an
//! `AudioBlock` exists, the operations below reuse its storage and perform no
//! heap allocation, locking, I/O, or logging.

pub const INTERNAL_SAMPLE_RATE_HZ: u32 = 48_000;
pub const PROCESSING_QUANTUM_FRAMES: usize = 128;
pub const MAX_CHANNELS: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockError {
    InvalidChannels,
    InvalidFrameCount,
    ShapeMismatch,
}

/// A preallocated planar float32 block. Samples are stored channel-major:
/// `channel * frames + frame`.
pub struct AudioBlock {
    channels: usize,
    frames: usize,
    samples: Vec<f32>,
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
        let gain = gain.is_finite().then_some(gain).unwrap_or(0.0);
        for sample in &mut self.samples {
            *sample *= gain;
        }
    }

    /// Add a same-shaped source block into this block without allocating.
    pub fn mix_from(&mut self, source: &Self, gain: f32) -> Result<(), BlockError> {
        if self.channels != source.channels || self.frames != source.frames {
            return Err(BlockError::ShapeMismatch);
        }
        let gain = gain.is_finite().then_some(gain).unwrap_or(0.0);
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
    fn channel_map_rejects_wrong_matrix_shape() {
        let source = AudioBlock::new(2, 4).unwrap();
        let mut destination = AudioBlock::new(1, 4).unwrap();
        assert_eq!(
            destination.map_from(&source, &[1.0]),
            Err(BlockError::ShapeMismatch)
        );
    }
}
