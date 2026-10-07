//! The planar block kernels must match simple per-sample reference loops bit
//! for bit (code review P2-7). The references are the frame-major loops the
//! kernels replaced; the inputs include NaN, infinities, -0.0 and clipping.
use audiorouter_engine::{AudioBlock, BlockMeter, FixedDelay, GainRamp, StreamingResampler};

const FRAMES: usize = 128;

fn awkward(seed: u32, index: usize) -> f32 {
    let mut x = seed
        .wrapping_mul(2_654_435_761)
        .wrapping_add(index as u32 * 40_503);
    x ^= x >> 13;
    x = x.wrapping_mul(0x5bd1_e995);
    x ^= x >> 15;
    match x % 23 {
        0 => f32::NAN,
        1 => f32::INFINITY,
        2 => f32::NEG_INFINITY,
        3 => -0.0,
        4 => 0.0,
        5 => 1.5,
        6 => -1.25,
        _ => (x as f32 / u32::MAX as f32) * 2.2 - 1.1,
    }
}

fn block(channels: usize, frames: usize, seed: u32) -> AudioBlock {
    let mut block = AudioBlock::new(channels, frames).unwrap();
    for channel in 0..channels {
        for (frame, sample) in block.channel_mut(channel).unwrap().iter_mut().enumerate() {
            *sample = awkward(seed + channel as u32, frame);
        }
    }
    block
}

/// Exact sample bits, except that every NaN compares equal: Rust leaves the
/// sign and payload of a NaN produced by arithmetic unspecified (the
/// compiler may swap the operands of a commutative multiply), so NaN bits
/// were never deterministic. The graph sanitizes NaN to silence anyway.
fn bits(block: &AudioBlock) -> Vec<u32> {
    (0..block.channels())
        .flat_map(|channel| {
            block.channel(channel).unwrap().iter().map(|s| {
                if s.is_nan() {
                    f32::NAN.to_bits()
                } else {
                    s.to_bits()
                }
            })
        })
        .collect()
}

fn get(block: &AudioBlock, channel: usize, frame: usize) -> f32 {
    block.channel(channel).unwrap()[frame]
}

fn set(block: &mut AudioBlock, channel: usize, frame: usize, value: f32) {
    block.channel_mut(channel).unwrap()[frame] = value;
}

#[test]
fn interleave_conversions_match_reference() {
    for channels in 1..=2 {
        for frames in [1, 7, FRAMES] {
            let source = block(channels, frames, 11);
            let mut interleaved = vec![0.0_f32; channels * frames];
            source.copy_to_interleaved(&mut interleaved).unwrap();
            let mut pcm = vec![0_i16; channels * frames];
            source.copy_to_interleaved_pcm16(&mut pcm).unwrap();
            for frame in 0..frames {
                for channel in 0..channels {
                    let value = get(&source, channel, frame);
                    let index = frame * channels + channel;
                    assert_eq!(interleaved[index].to_bits(), value.to_bits());
                    let scaled = if value.is_finite() {
                        (value.clamp(-1.0, 1.0) * 32_768.0).round()
                    } else {
                        0.0
                    };
                    let expected =
                        (scaled as i32).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
                    assert_eq!(pcm[index], expected);
                }
            }
            let mut back = AudioBlock::new(channels, frames).unwrap();
            back.copy_from_interleaved(&interleaved).unwrap();
            assert_eq!(bits(&back), bits(&source));
            back.copy_from_interleaved_pcm16(&pcm).unwrap();
            for frame in 0..frames {
                for channel in 0..channels {
                    let expected = f32::from(pcm[frame * channels + channel]) / 32_768.0;
                    assert_eq!(get(&back, channel, frame).to_bits(), expected.to_bits());
                }
            }
        }
    }
}

#[test]
fn matrices_match_reference() {
    let rows: &[&[f32]] = &[
        &[1.0, 0.0, 0.0, 1.0],
        &[0.5, -0.5, -0.0, 2.0],
        &[f32::NAN, 1.0, f32::INFINITY, 0.25],
        &[-1.0, 0.0, 0.0, -1.0],
    ];
    for &stereo_matrix in rows {
        for (destination_channels, source_channels) in [(1, 1), (1, 2), (2, 1), (2, 2)] {
            let matrix = &stereo_matrix[..destination_channels * source_channels];
            let source = block(source_channels, FRAMES, 3);
            let base = block(destination_channels, FRAMES, 7);
            let mut expected_map = base.clone_shape();
            let mut expected_mix = block(destination_channels, FRAMES, 7);
            for d in 0..destination_channels {
                for frame in 0..FRAMES {
                    let (mut map, mut mix) = (0.0_f32, 0.0_f32);
                    for s in 0..source_channels {
                        let coefficient = matrix[d * source_channels + s];
                        map += get(&source, s, frame) * coefficient;
                        let coefficient = if coefficient.is_finite() {
                            coefficient
                        } else {
                            0.0
                        };
                        mix += get(&source, s, frame) * coefficient;
                    }
                    set(&mut expected_map, d, frame, map);
                    let old = get(&expected_mix, d, frame);
                    set(&mut expected_mix, d, frame, old + mix);
                }
            }
            let mut mapped = AudioBlock::new(destination_channels, FRAMES).unwrap();
            mapped.map_from(&source, matrix).unwrap();
            assert_eq!(bits(&mapped), bits(&expected_map));
            let mut mixed = block(destination_channels, FRAMES, 7);
            mixed.mix_mapped_from(&source, matrix).unwrap();
            assert_eq!(bits(&mixed), bits(&expected_mix));

            if destination_channels == source_channels {
                let mut in_place = block(source_channels, FRAMES, 3);
                in_place.apply_channel_matrix(matrix).unwrap();
                let mut expected = AudioBlock::new(source_channels, FRAMES).unwrap();
                for frame in 0..FRAMES {
                    for d in 0..source_channels {
                        let mut value = 0.0_f32;
                        for s in 0..source_channels {
                            value += get(&source, s, frame) * matrix[d * source_channels + s];
                        }
                        set(&mut expected, d, frame, value);
                    }
                }
                assert_eq!(bits(&in_place), bits(&expected));
            }
        }
    }
}

trait CloneShape {
    fn clone_shape(&self) -> AudioBlock;
}

impl CloneShape for AudioBlock {
    fn clone_shape(&self) -> AudioBlock {
        AudioBlock::new(self.channels(), self.frames()).unwrap()
    }
}

#[test]
fn gain_ramp_matches_reference() {
    for channels in 1..=2 {
        for ramp in [0, 1, 50, 128, 300] {
            let mut ramp_under_test = GainRamp::new(0.8);
            ramp_under_test.set_target(-0.3, ramp);
            let (mut current, mut step, mut remaining, target) = (
                0.8_f32,
                (-0.3_f32 - 0.8) / ramp.max(1) as f32,
                ramp,
                -0.3_f32,
            );
            if ramp == 0 {
                current = target;
                step = 0.0;
            }
            for quantum in 0..4 {
                let mut actual = block(channels, FRAMES, 20 + quantum);
                let mut expected = block(channels, FRAMES, 20 + quantum);
                ramp_under_test.apply(&mut actual);
                for frame in 0..FRAMES {
                    if remaining > 0 {
                        current += step;
                        remaining -= 1;
                        if remaining == 0 {
                            current = target;
                            step = 0.0;
                        }
                    }
                    for channel in 0..channels {
                        let value = get(&expected, channel, frame) * current;
                        set(&mut expected, channel, frame, value);
                    }
                }
                assert_eq!(
                    bits(&actual),
                    bits(&expected),
                    "ramp {ramp} quantum {quantum}"
                );
            }
        }
    }
}

#[test]
fn fixed_delay_matches_reference() {
    for channels in 1..=2 {
        for delay in [0, 1, 5, 127, 128, 200] {
            let capacity = 201;
            let mut delay_line = FixedDelay::new(channels, capacity - 1).unwrap();
            delay_line.set_delay_frames(delay).unwrap();
            let mut ring = vec![0.0_f32; capacity * channels];
            let mut write = 0;
            for quantum in 0..5 {
                let mut actual = block(channels, FRAMES, 40 + quantum);
                let mut expected = block(channels, FRAMES, 40 + quantum);
                delay_line.process(&mut actual).unwrap();
                for frame in 0..FRAMES {
                    let read = (write + capacity - delay) % capacity;
                    for channel in 0..channels {
                        let input = get(&expected, channel, frame);
                        let input = if input.is_finite() { input } else { 0.0 };
                        let delayed = ring[read * channels + channel];
                        ring[write * channels + channel] = input;
                        set(
                            &mut expected,
                            channel,
                            frame,
                            if delay == 0 { input } else { delayed },
                        );
                    }
                    write = (write + 1) % capacity;
                }
                assert_eq!(bits(&actual), bits(&expected), "delay {delay} q {quantum}");
            }
        }
    }
}

#[test]
fn resamplers_match_reference() {
    for channels in 1..=2 {
        for ratio in [0.5, 0.9997, 1.0, 1.0003, 1.5] {
            let source = block(channels, FRAMES, 60);
            let mut actual = AudioBlock::new(channels, FRAMES).unwrap();
            actual.resample_linear_with_ratio(&source, ratio).unwrap();
            let mut expected = AudioBlock::new(channels, FRAMES).unwrap();
            for channel in 0..channels {
                let input = source.channel(channel).unwrap();
                for frame in 0..FRAMES {
                    let position = frame as f64 * ratio;
                    let lower = (position.floor() as usize).min(FRAMES - 1);
                    let upper = (lower + 1).min(FRAMES - 1);
                    let fraction = (position - lower as f64) as f32;
                    let a = if input[lower].is_finite() {
                        input[lower]
                    } else {
                        0.0
                    };
                    let b = if input[upper].is_finite() {
                        input[upper]
                    } else {
                        0.0
                    };
                    set(&mut expected, channel, frame, a + (b - a) * fraction);
                }
            }
            assert_eq!(bits(&actual), bits(&expected));

            // Streaming: compare against a plain queue model, including the
            // partial underflow block.
            let mut streaming = StreamingResampler::new(channels, 1_024).unwrap();
            let mut queue: Vec<Vec<f32>> = vec![Vec::new(); channels];
            let mut phase = 0.0_f64;
            for quantum in 0..6 {
                if quantum % 3 != 2 {
                    let input = block(channels, FRAMES, 70 + quantum);
                    assert_eq!(streaming.push(&input).unwrap(), FRAMES);
                    for (channel, queued) in queue.iter_mut().enumerate() {
                        queued.extend_from_slice(input.channel(channel).unwrap());
                    }
                }
                let mut actual = AudioBlock::new(channels, FRAMES).unwrap();
                let produced = streaming.process(&mut actual, ratio).unwrap();
                let mut expected = AudioBlock::new(channels, FRAMES).unwrap();
                let mut count = 0;
                for frame in 0..FRAMES {
                    let position = phase + frame as f64 * ratio;
                    let lower = position.floor() as usize;
                    if lower + 1 >= queue[0].len() {
                        break;
                    }
                    let fraction = (position - lower as f64) as f32;
                    for (channel, queued) in queue.iter().enumerate() {
                        let a = queued[lower];
                        let b = queued[lower + 1];
                        let a = if a.is_finite() { a } else { 0.0 };
                        let b = if b.is_finite() { b } else { 0.0 };
                        set(&mut expected, channel, frame, a + (b - a) * fraction);
                    }
                    count += 1;
                }
                assert_eq!(bits(&actual), bits(&expected), "ratio {ratio} q {quantum}");
                if count == FRAMES {
                    assert_eq!(produced, FRAMES);
                    phase += FRAMES as f64 * ratio;
                    let consumed = (phase.floor() as usize).min(queue[0].len() - 1);
                    for queued in &mut queue {
                        queued.drain(..consumed);
                    }
                    phase -= consumed as f64;
                } else {
                    assert_eq!(produced, 0);
                }
            }
        }
    }
}

#[test]
fn block_meter_matches_reference() {
    for channels in 1..=2 {
        for frames in [1, 9, FRAMES] {
            let meter = BlockMeter::default();
            let mut clipped = [0_u64; 2];
            let mut peak_hold = [0.0_f32; 2];
            for quantum in 0..3 {
                let input = block(channels, frames, 90 + quantum);
                meter.observe(&input);
                let snapshot = meter.snapshot();
                let mut expected_current = [0.0_f32; 2];
                for channel in 0..channels {
                    let samples = input.channel(channel).unwrap();
                    expected_current[channel] = samples
                        .iter()
                        .filter(|s| s.is_finite())
                        .map(|s| s.abs())
                        .fold(0.0, f32::max);
                    peak_hold[channel] = peak_hold[channel].max(expected_current[channel]);
                    clipped[channel] += samples
                        .iter()
                        .filter(|s| s.is_finite() && s.abs() > 1.0)
                        .count() as u64;
                    assert_eq!(
                        snapshot.channel_rms[channel].to_bits(),
                        input.channel_rms(channel).unwrap().to_bits()
                    );
                    assert_eq!(snapshot.channel_peak_abs[channel], peak_hold[channel]);
                    assert_eq!(snapshot.channel_clipped_samples[channel], clipped[channel]);
                }
                assert_eq!(snapshot.peak_abs, peak_hold[0].max(peak_hold[1]));
                assert_eq!(snapshot.clipped_samples, clipped[0] + clipped[1]);
                // The whole-block RMS must equal `AudioBlock::rms`, which the
                // meter used to call.
                assert_eq!(
                    snapshot.rms_db.to_bits(),
                    rms_db_of(input.rms()).to_bits(),
                    "rms channels {channels} frames {frames}"
                );
                for (actual, expected) in snapshot
                    .channel_current_peak_db
                    .iter()
                    .zip(expected_current)
                {
                    assert_eq!(actual.to_bits(), rms_db_of(expected).to_bits());
                }
            }
        }
    }
}

/// The meter's dB projection, observed through a meter fed a constant block.
fn rms_db_of(value: f32) -> f32 {
    let meter = BlockMeter::default();
    let mut constant = AudioBlock::new(1, 1).unwrap();
    constant.channel_mut(0).unwrap()[0] = value;
    meter.observe(&constant);
    meter.snapshot().current_peak_db
}
