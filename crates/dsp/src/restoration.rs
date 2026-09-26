//! Audio restoration processors: click repair.
//!
//! All state is allocated in the constructors; `process` performs no
//! allocation, locking, or I/O and is safe for the realtime callback.

/// Output delay of [`Declicker`], in samples. A click is repaired from clean
/// samples on both sides, so the output lags the input by this lookahead.
pub const DECLICK_LOOKAHEAD: usize = 64;
/// Longest run of samples repaired as one click.
pub const DECLICK_MAX_REGION: usize = 32;
const RING: usize = 128;
const MAX_REGIONS: usize = 8;

#[derive(Clone, Copy, Debug, Default)]
struct Region {
    start: u64,
    end: u64,
}

/// Mono click and crackle repair (Audio Hijack "Declick" equivalent).
///
/// The second difference `x[n] - 2x[n-1] + x[n-2]` is near zero for smooth
/// program material but large for an impulsive click. A sample is flagged
/// when that residual exceeds `k` times its running mean level, where the
/// threshold maps 0 % to `k = 3` (repairs more, risks false positives) and
/// 100 % to `k = 30` (repairs fewer). Flagged runs of at most
/// [`DECLICK_MAX_REGION`] samples are replaced by linear interpolation
/// between the clean samples on either side before they leave the
/// [`DECLICK_LOOKAHEAD`] delay line.
#[derive(Clone, Debug)]
pub struct Declicker {
    factor: f32,
    level: f32,
    decay: f32,
    warmup: u64,
    ring: [f32; RING],
    written: u64,
    previous: [f32; 2],
    regions: [Region; MAX_REGIONS],
    region_count: usize,
    repaired: u64,
}

impl Declicker {
    pub fn new(threshold_percent: f32, sample_rate: f32) -> Self {
        let threshold = if threshold_percent.is_finite() {
            threshold_percent.clamp(0.0, 100.0)
        } else {
            50.0
        };
        let sample_rate = if sample_rate.is_finite() && sample_rate > 0.0 {
            sample_rate
        } else {
            48_000.0
        };
        Self {
            factor: 3.0 + threshold / 100.0 * 27.0,
            level: 0.0,
            // About a 20 ms time constant for the residual level.
            decay: (-1.0 / (0.020 * sample_rate)).exp(),
            // Detect only after two level time constants of history.
            warmup: (0.040 * sample_rate) as u64,
            ring: [0.0; RING],
            written: 0,
            previous: [0.0; 2],
            regions: [Region::default(); MAX_REGIONS],
            region_count: 0,
            repaired: 0,
        }
    }

    /// Total samples replaced since construction or the last reset.
    pub fn repaired_samples(&self) -> u64 {
        self.repaired
    }

    pub fn reset(&mut self) {
        self.level = 0.0;
        self.ring = [0.0; RING];
        self.written = 0;
        self.previous = [0.0; 2];
        self.region_count = 0;
        self.repaired = 0;
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            let input = if sample.is_finite() { *sample } else { 0.0 };
            *sample = self.push(input);
        }
    }

    fn push(&mut self, input: f32) -> f32 {
        let index = self.written;
        self.ring[(index as usize) % RING] = input;
        let residual = input - 2.0 * self.previous[0] + self.previous[1];
        self.previous = [input, self.previous[0]];
        let magnitude = residual.abs();
        // Require some history before detecting, and a small absolute floor
        // so digital silence and tiny noise never register as clicks.
        let click = index >= self.warmup && magnitude > self.factor * self.level + 1.0e-4;
        // A click must not inflate the level it is measured against.
        let tracked = if click { self.level } else { magnitude };
        self.level = self.decay * self.level + (1.0 - self.decay) * tracked;
        if self.level < 1.0e-12 {
            self.level = 0.0;
        }
        if click {
            self.mark(index);
        }
        self.written += 1;
        self.repair_ready();
        if index < DECLICK_LOOKAHEAD as u64 {
            return 0.0;
        }
        self.ring[((index - DECLICK_LOOKAHEAD as u64) as usize) % RING]
    }

    fn mark(&mut self, index: u64) {
        // The second difference places a click's energy one sample earlier.
        let start = index.saturating_sub(1);
        let end = index + 1;
        if self.region_count > 0 {
            let last = &mut self.regions[self.region_count - 1];
            if start <= last.end + 1 {
                if end - last.start < DECLICK_MAX_REGION as u64 {
                    last.end = last.end.max(end);
                }
                return;
            }
        }
        if self.region_count < MAX_REGIONS {
            self.regions[self.region_count] = Region { start, end };
            self.region_count += 1;
        }
    }

    /// Repair every region whose trailing clean sample has arrived and whose
    /// leading clean sample is still in the ring and not yet output.
    fn repair_ready(&mut self) {
        let mut index = 0;
        while index < self.region_count {
            let region = self.regions[index];
            // Wait two samples past the region so a continuing click can
            // still extend it.
            if region.end + 2 >= self.written {
                index += 1;
                continue;
            }
            let oldest_unsent = self.written.saturating_sub(DECLICK_LOOKAHEAD as u64);
            if region.start >= 1 && region.start > oldest_unsent {
                let before = self.ring[((region.start - 1) as usize) % RING];
                let after = self.ring[((region.end + 1) as usize) % RING];
                let span = (region.end + 2 - region.start) as f32;
                for position in region.start..=region.end {
                    let t = (position + 1 - region.start) as f32 / span;
                    self.ring[(position as usize) % RING] = before + (after - before) * t;
                }
                self.repaired += region.end + 1 - region.start;
            }
            self.regions.copy_within(index + 1..self.region_count, index);
            self.region_count -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frames: usize, frequency: f32, amplitude: f32) -> Vec<f32> {
        (0..frames)
            .map(|frame| amplitude * (std::f32::consts::TAU * frequency * frame as f32 / 48_000.0).sin())
            .collect()
    }

    #[test]
    fn repairs_an_isolated_click_and_delays_by_the_lookahead() {
        let clean = sine(4_800, 440.0, 0.3);
        let mut clicked = clean.clone();
        clicked[2_000] += 0.8;
        clicked[2_001] -= 0.6;
        let mut declicker = Declicker::new(50.0, 48_000.0);
        let mut output = clicked.clone();
        for chunk in output.chunks_mut(128) {
            declicker.process(chunk);
        }
        assert!(declicker.repaired_samples() > 0);
        // Compare the delayed output with the clean input around the click.
        let worst = (1_990..2_010)
            .map(|frame| (output[frame + DECLICK_LOOKAHEAD] - clean[frame]).abs())
            .fold(0.0_f32, f32::max);
        assert!(worst < 0.05, "residual click error {worst}");
    }

    #[test]
    fn leaves_smooth_program_material_untouched() {
        let clean = sine(9_600, 1_000.0, 0.5);
        let mut declicker = Declicker::new(50.0, 48_000.0);
        let mut output = clean.clone();
        declicker.process(&mut output);
        assert_eq!(declicker.repaired_samples(), 0);
        for frame in 0..clean.len() - DECLICK_LOOKAHEAD {
            assert_eq!(output[frame + DECLICK_LOOKAHEAD], clean[frame]);
        }
    }

    #[test]
    fn stays_finite_and_silent_for_silence_and_non_finite_input() {
        let mut declicker = Declicker::new(0.0, 48_000.0);
        let mut samples = vec![0.0; 512];
        samples[100] = f32::NAN;
        samples[200] = f32::INFINITY;
        declicker.process(&mut samples);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        declicker.reset();
        assert_eq!(declicker.repaired_samples(), 0);
    }
}
