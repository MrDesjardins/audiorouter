//! Render a 5.1/7.1 speaker stream to two ears with measured head-related
//! impulse responses ("surround to headphones").
//!
//! Each speaker channel is placed at its standard horizontal position and
//! convolved with the KEMAR response pair for that direction. Positions are
//! static: there is no head tracking or room model. The LFE channel carries
//! no direction and is added to both ears at -6 dB.
//!
//! All storage is allocated in [`BinauralRenderer::new`]; rendering is
//! allocation-free and lock-free, so it may run on the audio path.

use crate::kemar_hrir::{KEMAR_AZIMUTHS_DEG, KEMAR_HRIR_48K, KEMAR_TAPS};

/// Sample rate of the embedded responses. Callers must render at this rate.
pub const BINAURAL_SAMPLE_RATE_HZ: u32 = 48_000;

/// Largest speaker stream accepted (7.1).
pub const MAX_BINAURAL_INPUT_CHANNELS: usize = 8;

const LFE_GAIN: f32 = 0.5;

// Windows `WAVEFORMATEXTENSIBLE.dwChannelMask` speaker bits.
const SPEAKER_FRONT_LEFT: u32 = 0x1;
const SPEAKER_FRONT_RIGHT: u32 = 0x2;
const SPEAKER_FRONT_CENTER: u32 = 0x4;
const SPEAKER_LOW_FREQUENCY: u32 = 0x8;
const SPEAKER_BACK_LEFT: u32 = 0x10;
const SPEAKER_BACK_RIGHT: u32 = 0x20;
const SPEAKER_SIDE_LEFT: u32 = 0x200;
const SPEAKER_SIDE_RIGHT: u32 = 0x400;

/// `KSAUDIO_SPEAKER_5POINT1` (back speakers), the Windows default for six channels.
pub const CHANNEL_MASK_5POINT1: u32 = 0x3f;
/// `KSAUDIO_SPEAKER_5POINT1_SURROUND` (side speakers).
pub const CHANNEL_MASK_5POINT1_SURROUND: u32 = 0x60f;
/// `KSAUDIO_SPEAKER_7POINT1_SURROUND`, the Windows default for eight channels.
pub const CHANNEL_MASK_7POINT1_SURROUND: u32 = 0x63f;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinauralError {
    /// Only 6 (5.1) and 8 (7.1) channel streams are rendered.
    UnsupportedChannelCount(usize),
    /// The channel mask names a speaker without a supported position, or its
    /// speaker count differs from the stream's channel count.
    UnsupportedChannelMask(u32),
    /// Input and output slices do not describe the same whole frame count.
    FrameMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Placement {
    /// Index into the response table and whether the source is on the left
    /// (mirrored: the ears swap).
    Directional { azimuth_index: usize, left_side: bool },
    LowFrequency,
}

/// Static binaural renderer for one interleaved speaker stream.
pub struct BinauralRenderer {
    channels: usize,
    placements: [Placement; MAX_BINAURAL_INPUT_CHANNELS],
    /// Per input channel: the left and right ear responses, time-reversed so
    /// a contiguous history window can be multiplied element-wise.
    reversed: Vec<[[f32; KEMAR_TAPS]; 2]>,
    /// Per input channel: a doubled history ring of `2 * KEMAR_TAPS` samples.
    /// Each sample is written twice so the newest `KEMAR_TAPS` samples are
    /// always one contiguous slice.
    history: Vec<f32>,
    position: usize,
}

impl BinauralRenderer {
    /// Prepare a renderer for `channels` interleaved speaker channels laid out
    /// by the Windows `channel_mask`. A zero mask selects the Windows default
    /// layout for the channel count.
    pub fn new(channels: usize, channel_mask: u32) -> Result<Self, BinauralError> {
        let mask = match (channels, channel_mask) {
            (6, 0) => CHANNEL_MASK_5POINT1,
            (8, 0) => CHANNEL_MASK_7POINT1_SURROUND,
            (6 | 8, mask) => mask,
            (other, _) => return Err(BinauralError::UnsupportedChannelCount(other)),
        };
        if mask.count_ones() as usize != channels {
            return Err(BinauralError::UnsupportedChannelMask(channel_mask));
        }
        let mut placements = [Placement::LowFrequency; MAX_BINAURAL_INPUT_CHANNELS];
        let mut reversed = Vec::with_capacity(channels);
        let mut channel = 0;
        // Interleaved channels appear in ascending speaker-bit order.
        for bit in (0..32).map(|shift| 1u32 << shift).filter(|bit| mask & bit != 0) {
            let placement = match bit {
                SPEAKER_FRONT_LEFT => directional(30, true),
                SPEAKER_FRONT_RIGHT => directional(30, false),
                SPEAKER_FRONT_CENTER => directional(0, false),
                SPEAKER_LOW_FREQUENCY => Placement::LowFrequency,
                SPEAKER_BACK_LEFT => directional(150, true),
                SPEAKER_BACK_RIGHT => directional(150, false),
                SPEAKER_SIDE_LEFT => directional(90, true),
                SPEAKER_SIDE_RIGHT => directional(90, false),
                _ => return Err(BinauralError::UnsupportedChannelMask(channel_mask)),
            };
            placements[channel] = placement;
            let mut pair = [[0.0; KEMAR_TAPS]; 2];
            if let Placement::Directional { azimuth_index, left_side } = placement {
                let measured = &KEMAR_HRIR_48K[azimuth_index];
                for (ear, taps) in pair.iter_mut().enumerate() {
                    // The table holds a source on the right; a left source
                    // uses the mirrored (swapped) ears.
                    let source = &measured[if left_side { 1 - ear } else { ear }];
                    for (index, tap) in taps.iter_mut().enumerate() {
                        *tap = source[KEMAR_TAPS - 1 - index];
                    }
                }
            }
            reversed.push(pair);
            channel += 1;
        }
        Ok(Self {
            channels,
            placements,
            reversed,
            history: vec![0.0; channels * 2 * KEMAR_TAPS],
            position: 0,
        })
    }

    pub fn input_channels(&self) -> usize {
        self.channels
    }

    /// Render interleaved speaker frames into interleaved stereo frames.
    /// Non-finite input samples are treated as silence.
    pub fn render_interleaved(
        &mut self,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<(), BinauralError> {
        if input.len() % self.channels != 0
            || output.len() % 2 != 0
            || input.len() / self.channels != output.len() / 2
        {
            return Err(BinauralError::FrameMismatch);
        }
        for (frame, out) in input.chunks_exact(self.channels).zip(output.chunks_exact_mut(2)) {
            let write = self.position;
            let mut left = 0.0f32;
            let mut right = 0.0f32;
            for (channel, &sample) in frame.iter().enumerate() {
                let sample = if sample.is_finite() { sample } else { 0.0 };
                let ring = &mut self.history[channel * 2 * KEMAR_TAPS..(channel + 1) * 2 * KEMAR_TAPS];
                ring[write] = sample;
                ring[write + KEMAR_TAPS] = sample;
                match self.placements[channel] {
                    Placement::LowFrequency => {
                        left += LFE_GAIN * sample;
                        right += LFE_GAIN * sample;
                    }
                    Placement::Directional { .. } => {
                        // Oldest-to-newest window ending at the sample just written.
                        let window = &ring[write + 1..write + 1 + KEMAR_TAPS];
                        let [left_taps, right_taps] = &self.reversed[channel];
                        left += dot(window, left_taps);
                        right += dot(window, right_taps);
                    }
                }
            }
            out[0] = left;
            out[1] = right;
            self.position = if write + 1 == KEMAR_TAPS { 0 } else { write + 1 };
        }
        Ok(())
    }

    /// Clear the convolution history, for example after a capture discontinuity.
    pub fn reset(&mut self) {
        self.history.fill(0.0);
        self.position = 0;
    }
}

fn directional(azimuth: u16, left_side: bool) -> Placement {
    let azimuth_index = KEMAR_AZIMUTHS_DEG
        .iter()
        .position(|candidate| *candidate == azimuth)
        .expect("speaker azimuth is in the embedded table");
    Placement::Directional { azimuth_index, left_side }
}

fn dot(window: &[f32], taps: &[f32; KEMAR_TAPS]) -> f32 {
    // Four partial sums let the optimizer vectorize the loop.
    let mut sums = [0.0f32; 4];
    for (samples, taps) in window.chunks_exact(4).zip(taps.chunks_exact(4)) {
        for lane in 0..4 {
            sums[lane] += samples[lane] * taps[lane];
        }
    }
    let tail = KEMAR_TAPS - KEMAR_TAPS % 4;
    let mut sum = sums[0] + sums[1] + sums[2] + sums[3];
    for index in tail..KEMAR_TAPS {
        sum += window[index] * taps[index];
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAMES: usize = 4_800;

    /// Deterministic broadband noise.
    fn noise(frames: usize) -> Vec<f32> {
        let mut state = 0x1234_5678u32;
        (0..frames)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                (state as f32 / u32::MAX as f32) * 2.0 - 1.0
            })
            .collect()
    }

    /// Render noise on one channel only and return (left, right) energy.
    fn render_one(mask: u32, channels: usize, active: usize) -> (f64, f64, Vec<f32>) {
        let mut renderer = BinauralRenderer::new(channels, mask).unwrap();
        let source = noise(FRAMES);
        let mut input = vec![0.0; FRAMES * channels];
        for (frame, value) in source.iter().enumerate() {
            input[frame * channels + active] = *value;
        }
        let mut output = vec![0.0; FRAMES * 2];
        renderer.render_interleaved(&input, &mut output).unwrap();
        let energy = |ear: usize| output.iter().skip(ear).step_by(2).map(|v| f64::from(*v).powi(2)).sum::<f64>();
        (energy(0), energy(1), output)
    }

    fn db(ratio: f64) -> f64 {
        10.0 * ratio.log10()
    }

    #[test]
    fn seven_one_channels_reach_the_expected_ear() {
        let mask = CHANNEL_MASK_7POINT1_SURROUND;
        // FL FR FC LFE BL BR SL SR
        let (l, r, _) = render_one(mask, 8, 0);
        assert!(db(l / r) > 3.0, "front left louder in left ear: {}", db(l / r));
        let (l, r, _) = render_one(mask, 8, 1);
        assert!(db(r / l) > 3.0, "front right louder in right ear: {}", db(r / l));
        let (l, r, _) = render_one(mask, 8, 6);
        assert!(db(l / r) > 8.0, "side left strongly in left ear: {}", db(l / r));
        let (l, r, _) = render_one(mask, 8, 7);
        assert!(db(r / l) > 8.0, "side right strongly in right ear: {}", db(r / l));
        let (l, r, _) = render_one(mask, 8, 4);
        assert!(db(l / r) > 3.0, "back left louder in left ear: {}", db(l / r));
        let (l, r, _) = render_one(mask, 8, 2);
        assert!(db(l / r).abs() < 0.5, "center balanced: {}", db(l / r));
    }

    #[test]
    fn front_and_back_speakers_on_one_side_sound_different() {
        let (_, _, front) = render_one(CHANNEL_MASK_7POINT1_SURROUND, 8, 0);
        let (_, _, back) = render_one(CHANNEL_MASK_7POINT1_SURROUND, 8, 4);
        let difference: f64 = front.iter().zip(&back).map(|(a, b)| f64::from(a - b).powi(2)).sum();
        let reference: f64 = front.iter().map(|v| f64::from(*v).powi(2)).sum();
        assert!(db(difference / reference) > -6.0, "front/back responses must differ");
    }

    #[test]
    fn frontal_center_keeps_unit_level_and_lfe_is_centered() {
        let (l, r, _) = render_one(CHANNEL_MASK_7POINT1_SURROUND, 8, 2);
        let source: f64 = noise(FRAMES).iter().map(|v| f64::from(*v).powi(2)).sum();
        assert!(db(l / source).abs() < 1.0, "center level {}", db(l / source));
        assert!(db(r / source).abs() < 1.0);
        let (l, r, output) = render_one(CHANNEL_MASK_7POINT1_SURROUND, 8, 3);
        assert_eq!(l, r);
        assert!((db(l / source) + 6.02).abs() < 0.1);
        assert!(output.chunks_exact(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn chunked_rendering_matches_one_pass_and_silence_stays_silent() {
        let channels = 6;
        let source = noise(FRAMES * channels);
        let mut whole = BinauralRenderer::new(channels, 0).unwrap();
        let mut expected = vec![0.0; FRAMES * 2];
        whole.render_interleaved(&source, &mut expected).unwrap();
        let mut chunked = BinauralRenderer::new(channels, 0).unwrap();
        let mut actual = vec![0.0; FRAMES * 2];
        for (input, output) in source.chunks(channels * 97).zip(actual.chunks_mut(2 * 97)) {
            chunked.render_interleaved(input, output).unwrap();
        }
        assert_eq!(expected, actual);
        chunked.reset();
        let mut silent = vec![1.0; 64 * 2];
        chunked.render_interleaved(&vec![0.0; 64 * channels], &mut silent).unwrap();
        assert!(silent.iter().all(|v| *v == 0.0));
    }

    #[test]
    #[ignore = "timing measurement; run with --release"]
    fn seven_one_rendering_cost_per_second_of_audio() {
        let mut renderer = BinauralRenderer::new(8, 0).unwrap();
        let input = noise(480 * 8);
        let mut output = vec![0.0; 480 * 2];
        let started = std::time::Instant::now();
        for _ in 0..1_000 {
            // 1 000 quanta of 10 ms = 10 s of 7.1 audio.
            renderer.render_interleaved(&input, &mut output).unwrap();
        }
        let per_second = started.elapsed().as_secs_f64() / 10.0;
        eprintln!("7.1 binaural: {:.2} ms of CPU per second of audio ({:.2}% of one core)", per_second * 1e3, per_second * 100.0);
        assert!(output.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn rejects_unsupported_layouts_and_mismatched_frames() {
        assert_eq!(BinauralRenderer::new(2, 0).err(), Some(BinauralError::UnsupportedChannelCount(2)));
        // 7.1 wide uses front-of-center speakers without a measured position.
        assert_eq!(BinauralRenderer::new(8, 0xff).err(), Some(BinauralError::UnsupportedChannelMask(0xff)));
        assert_eq!(BinauralRenderer::new(8, CHANNEL_MASK_5POINT1).err(), Some(BinauralError::UnsupportedChannelMask(CHANNEL_MASK_5POINT1)));
        assert!(BinauralRenderer::new(6, CHANNEL_MASK_5POINT1_SURROUND).is_ok());
        let mut renderer = BinauralRenderer::new(8, 0).unwrap();
        assert_eq!(renderer.render_interleaved(&[0.0; 16], &mut [0.0; 2]), Err(BinauralError::FrameMismatch));
        let mut output = [0.0; 2];
        renderer.render_interleaved(&[f32::NAN; 8], &mut output).unwrap();
        assert_eq!(output, [0.0, 0.0]);
    }
}
