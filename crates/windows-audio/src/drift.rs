//! Clock-drift correction shared by the output pump and the input feeder.
//!
//! Every audio device runs on its own clock. Two devices at "48 kHz" differ
//! by tens to hundreds of parts per million, so a queue between them slowly
//! fills or drains. This module holds the two pieces that keep such a queue
//! at a chosen depth without an audible gap:
//!
//! * [`slip_frames`] plays one block one frame shorter or longer by cubic
//!   interpolation. A block starts and ends on a source frame, so blocks
//!   that are not slipped stay bit-exact and need no resampler state beyond
//!   the previous block's last frame.
//! * [`MeanDepth`] measures a queue's depth as a mean over time. Devices
//!   deliver and take audio a 10 ms period at a time, so a depth read at
//!   any one moment is a staircase: under drift it stays level for many
//!   seconds and then jumps a whole period. The time-weighted mean moves
//!   smoothly with the drift, which lets a correction start early.
//!
//! Nothing here allocates, waits or logs; all of it runs on the audio path.
//! See `docs/plans/active/audio-latency.md` for the measurements behind it.

/// Queue depths (frames) between which a queue is left alone, measured from
/// its target. Outside it the queue is slipped until it is back near the
/// target ([`SLIP_RELEASE_FRAMES`]). 5 ms: half a device period.
pub(crate) const SLIP_ENGAGE_FRAMES: i64 = 240;
/// A correction stops once the mean depth is within this of the target.
pub(crate) const SLIP_RELEASE_FRAMES: i64 = 48;

/// One device period at the internal rate (10 ms). A queue in front of a
/// device, or behind one, swings by this much within every period.
pub(crate) const DEVICE_PERIOD_FRAMES: u64 = 480;

/// One finished measurement window of a queue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DepthWindow {
    /// Time-weighted mean depth, in frames.
    pub(crate) mean: f64,
    /// Lowest depth observed, in frames.
    pub(crate) lowest: u64,
}

/// Time-weighted mean of a queue depth over fixed windows.
///
/// Each observation gives the clock and the depth found; [`Self::hold`]
/// then records the depth the caller left behind. Between two observations
/// the depth is taken as the average of what was left and what was found,
/// since a device period may have ended anywhere in between.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MeanDepth {
    window_100ns: u64,
    /// Clock and depth at the end of the last observation.
    held: Option<(u64, u64)>,
    elapsed_100ns: u64,
    /// Sum of depth multiplied by time (frames times 100 ns).
    area: f64,
    lowest: u64,
}

impl MeanDepth {
    pub(crate) fn new(window_100ns: u64) -> Self {
        Self {
            window_100ns: window_100ns.max(1),
            held: None,
            elapsed_100ns: 0,
            area: 0.0,
            lowest: u64::MAX,
        }
    }

    /// Record the depth found at `now_100ns`. Returns the window when this
    /// observation completes one.
    pub(crate) fn observe(&mut self, now_100ns: u64, depth: u64) -> Option<DepthWindow> {
        self.lowest = self.lowest.min(depth);
        if let Some((then, left)) = self.held {
            let elapsed = now_100ns.saturating_sub(then);
            self.area += (left as f64 + depth as f64) * 0.5 * elapsed as f64;
            self.elapsed_100ns = self.elapsed_100ns.saturating_add(elapsed);
        }
        self.held = Some((now_100ns, depth));
        if self.elapsed_100ns < self.window_100ns {
            return None;
        }
        let window = DepthWindow {
            mean: self.area / self.elapsed_100ns as f64,
            lowest: self.lowest,
        };
        self.elapsed_100ns = 0;
        self.area = 0.0;
        self.lowest = u64::MAX;
        Some(window)
    }

    /// Record the depth the caller left after acting on an observation.
    pub(crate) fn hold(&mut self, depth: u64) {
        if let Some((_, held)) = self.held.as_mut() {
            *held = depth;
        }
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::new(self.window_100ns);
    }
}

/// How far a queue dips below its mean depth in every window.
///
/// A queue in front of a device swings as audio arrives in packets and
/// leaves in periods. To keep its low points at a chosen margin, its mean
/// must sit above that margin by the size of the dip. For 10 ms packets and
/// periods the dip is at most one period; a device or source that moves
/// audio in larger steps dips further. Taking the smallest dip of the last
/// windows ignores a single late packet, which is jitter, not swing.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct QueueSwing {
    dips: [f32; Self::WINDOWS],
    next: usize,
    filled: usize,
}

impl QueueSwing {
    const WINDOWS: usize = 8;
    /// Never plan for a dip beyond 40 ms.
    const LARGEST_FRAMES: f32 = 1_920.0;

    pub(crate) fn observe(&mut self, window: DepthWindow) {
        self.dips[self.next] = (window.mean - window.lowest as f64).max(0.0) as f32;
        self.next = (self.next + 1) % Self::WINDOWS;
        self.filled = (self.filled + 1).min(Self::WINDOWS);
    }

    /// The dip to allow for, in frames: at least one device period.
    pub(crate) fn frames(&self) -> u64 {
        if self.filled < Self::WINDOWS {
            return DEVICE_PERIOD_FRAMES;
        }
        let smallest = self.dips.iter().copied().fold(f32::INFINITY, f32::min);
        (smallest.min(Self::LARGEST_FRAMES).round() as u64).max(DEVICE_PERIOD_FRAMES)
    }
}

/// Which way a queue is being corrected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SlipDirection {
    #[default]
    None,
    /// The queue is too deep: play slightly fewer frames than arrive.
    Shorten,
    /// The queue is too shallow: play slightly more frames than arrive.
    Lengthen,
}

/// Decides, block by block, whether to slip, from the mean depth of each
/// window. The correction is proportional to the error with a time constant
/// of `time_constant_blocks` blocks, at most one frame per block, and has
/// hysteresis so a settled queue passes audio untouched.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SlipController {
    direction: SlipDirection,
    /// Fraction of a frame owed per block while correcting.
    rate: f64,
    credit: f64,
    time_constant_blocks: f64,
    allow_lengthen: bool,
}

impl SlipController {
    pub(crate) fn new(time_constant_blocks: f64, allow_lengthen: bool) -> Self {
        Self {
            direction: SlipDirection::None,
            rate: 0.0,
            credit: 0.0,
            time_constant_blocks: time_constant_blocks.max(1.0),
            allow_lengthen,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.direction = SlipDirection::None;
        self.rate = 0.0;
        self.credit = 0.0;
    }

    /// Update from one window's mean depth and the target depth (frames).
    pub(crate) fn update(&mut self, depth: u64, target: u64) {
        let error = i64::try_from(depth).unwrap_or(i64::MAX) - i64::try_from(target).unwrap_or(0);
        self.direction = match self.direction {
            SlipDirection::Shorten if error <= SLIP_RELEASE_FRAMES => SlipDirection::None,
            SlipDirection::Lengthen if error >= -SLIP_RELEASE_FRAMES => SlipDirection::None,
            SlipDirection::None if error > SLIP_ENGAGE_FRAMES => SlipDirection::Shorten,
            SlipDirection::None if error < -SLIP_ENGAGE_FRAMES && self.allow_lengthen => {
                SlipDirection::Lengthen
            }
            direction => direction,
        };
        if self.direction == SlipDirection::None {
            self.rate = 0.0;
            self.credit = 0.0;
        } else {
            // Never slower than one frame every 64 blocks, so a correction
            // near its end still finishes.
            self.rate =
                (error.unsigned_abs() as f64 / self.time_constant_blocks).clamp(1.0 / 64.0, 1.0);
        }
    }

    /// Whether the next block is slipped, and which way.
    pub(crate) fn next_block(&mut self) -> SlipDirection {
        if self.direction == SlipDirection::None {
            return SlipDirection::None;
        }
        self.credit += self.rate;
        if self.credit < 1.0 {
            return SlipDirection::None;
        }
        self.credit -= 1.0;
        self.direction
    }
}

/// 4-point, third-order (Catmull-Rom) interpolation between `p1` and `p2`.
/// It returns `p1` exactly at `t == 0`.
#[inline]
fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let a = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
    let b = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c = -0.5 * p0 + 0.5 * p2;
    ((a * t + b) * t + c) * t + p1
}

/// Resample one channel of `frames_in` source frames to `frames_out` frames
/// that cover the same time span, so the block is played slightly faster or
/// slower. `sample(frame)` reads a source frame; `previous` is the frame
/// before the block (the last frame of the previous block). Output frame 0
/// equals source frame 0, and the frame after the block is estimated by
/// extending the block's last step, which only the final output frame uses
/// with a weight below 1%.
///
/// `frames_in` must be at least 2 and `frames_out` at least 1.
#[inline]
pub(crate) fn slip_frames(
    frames_in: usize,
    frames_out: usize,
    previous: f32,
    sample: impl Fn(usize) -> f32,
    mut write: impl FnMut(usize, f32),
) {
    debug_assert!(frames_in >= 2 && frames_out >= 1);
    let last = sample(frames_in - 1);
    let last_step = last - sample(frames_in - 2);
    let at = |frame: isize| -> f32 {
        if frame < 0 {
            previous
        } else if (frame as usize) < frames_in {
            sample(frame as usize)
        } else {
            last + last_step * (frame as usize - (frames_in - 1)) as f32
        }
    };
    let step = frames_in as f64 / frames_out as f64;
    for frame in 0..frames_out {
        let position = frame as f64 * step;
        let lower = position.floor();
        let t = (position - lower) as f32;
        let lower = lower as isize;
        write(
            frame,
            catmull_rom(at(lower - 1), at(lower), at(lower + 1), at(lower + 2), t),
        );
    }
}

/// [`slip_frames`] for every channel of interleaved native-endian float32
/// frames. `source` holds `frames_in` frames and `destination` receives
/// `frames_out`; `previous` holds the frame before the block per channel.
/// Returns `false`, writing nothing, when a buffer is too small.
pub(crate) fn slip_interleaved(
    source: &[u8],
    destination: &mut [u8],
    channels: usize,
    frames_in: usize,
    frames_out: usize,
    previous: &[f32],
) -> bool {
    let stride = channels * std::mem::size_of::<f32>();
    if channels == 0
        || frames_in < 2
        || frames_out == 0
        || previous.len() < channels
        || source.len() < frames_in * stride
        || destination.len() < frames_out * stride
    {
        return false;
    }
    let read = |bytes: &[u8], frame: usize, channel: usize| {
        let start = frame * stride + channel * 4;
        f32::from_ne_bytes([
            bytes[start],
            bytes[start + 1],
            bytes[start + 2],
            bytes[start + 3],
        ])
    };
    for (channel, &previous) in previous.iter().enumerate().take(channels) {
        slip_frames(
            frames_in,
            frames_out,
            previous,
            |frame| read(source, frame, channel),
            |frame, value| {
                let start = frame * stride + channel * 4;
                destination[start..start + 4].copy_from_slice(&value.to_ne_bytes());
            },
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frequency_hz: f64, frame: f64) -> f32 {
        (std::f64::consts::TAU * frequency_hz * frame / 48_000.0).sin() as f32
    }

    /// Slip every block of a continuous sine and compare the result with the
    /// same sine sampled at the positions the slip claims to produce.
    fn worst_error(frequency_hz: f64, frames_out: usize) -> f32 {
        const FRAMES: usize = 128;
        let mut worst = 0.0_f32;
        for block in 0..64_usize {
            let base = (block * FRAMES) as f64;
            let mut output = vec![0.0_f32; frames_out];
            slip_frames(
                FRAMES,
                frames_out,
                sine(frequency_hz, base - 1.0),
                |frame| sine(frequency_hz, base + frame as f64),
                |frame, value| output[frame] = value,
            );
            for (frame, value) in output.iter().enumerate() {
                let position = base + frame as f64 * FRAMES as f64 / frames_out as f64;
                worst = worst.max((value - sine(frequency_hz, position)).abs());
            }
        }
        worst
    }

    #[test]
    fn a_slipped_block_follows_the_signal_closely() {
        for frames_out in [127, 129] {
            // Voice band: more than 60 dB below the signal.
            assert!(worst_error(1_000.0, frames_out) < 1e-3, "1 kHz");
            // 5 kHz: more than 34 dB below.
            assert!(worst_error(5_000.0, frames_out) < 2e-2, "5 kHz");
        }
    }

    #[test]
    fn a_slipped_block_starts_on_its_first_source_frame() {
        let source: Vec<f32> = (0..128).map(|frame| sine(700.0, frame as f64)).collect();
        for frames_out in [127, 129] {
            let mut first = f32::NAN;
            slip_frames(
                128,
                frames_out,
                -0.25,
                |frame| source[frame],
                |frame, value| {
                    if frame == 0 {
                        first = value;
                    }
                },
            );
            assert_eq!(first, source[0]);
        }
    }

    #[test]
    fn interleaved_slip_keeps_channels_apart_and_checks_sizes() {
        let frames = 64;
        let mut source = vec![0_u8; frames * 8];
        for frame in 0..frames {
            source[frame * 8..frame * 8 + 4].copy_from_slice(&0.5_f32.to_ne_bytes());
            source[frame * 8 + 4..frame * 8 + 8].copy_from_slice(&(-0.25_f32).to_ne_bytes());
        }
        let mut destination = vec![0_u8; (frames - 1) * 8];
        assert!(slip_interleaved(
            &source,
            &mut destination,
            2,
            frames,
            frames - 1,
            &[0.5, -0.25],
        ));
        for frame in destination.chunks_exact(8) {
            let left = f32::from_ne_bytes(frame[..4].try_into().unwrap());
            let right = f32::from_ne_bytes(frame[4..].try_into().unwrap());
            assert!((left - 0.5).abs() < 1e-6 && (right + 0.25).abs() < 1e-6);
        }
        // A destination one frame too small is refused, not overrun.
        assert!(!slip_interleaved(
            &source,
            &mut destination[..(frames - 2) * 8],
            2,
            frames,
            frames - 1,
            &[0.0, 0.0],
        ));
    }

    #[test]
    fn mean_depth_weights_each_depth_by_the_time_it_lasted() {
        let mut depth = MeanDepth::new(1_000);
        assert_eq!(depth.observe(0, 100), None);
        depth.hold(600);
        // 600 frames held for 900 units, found at 600.
        assert_eq!(depth.observe(900, 600), None);
        depth.hold(600);
        // A period ended somewhere in the last 100 units: 600 left, 120
        // found, so 360 on average.
        let window = depth.observe(1_000, 120).unwrap();
        assert!((window.mean - (600.0 * 900.0 + 360.0 * 100.0) / 1_000.0).abs() < 1e-9);
        assert_eq!(window.lowest, 100);
        // The next window starts from the depth left behind.
        depth.hold(500);
        assert_eq!(depth.observe(1_500, 500), None);
        let window = depth.observe(2_000, 500).unwrap();
        assert_eq!(window.mean, 500.0);
        assert_eq!(window.lowest, 500);
    }

    #[test]
    fn controller_engages_outside_the_band_and_releases_near_the_target() {
        let mut controller = SlipController::new(750.0, true);
        controller.update(480 + 200, 480);
        assert_eq!(controller.next_block(), SlipDirection::None);
        controller.update(480 + 900, 480);
        // 900 frames over a 750-block time constant: every block slips.
        assert_eq!(controller.next_block(), SlipDirection::Shorten);
        // Still correcting inside the engage band, until near the target.
        controller.update(480 + 100, 480);
        let slips = (0..64)
            .filter(|_| controller.next_block() == SlipDirection::Shorten)
            .count();
        assert!((1..=16).contains(&slips), "{slips}");
        controller.update(480 + 40, 480);
        assert_eq!(controller.next_block(), SlipDirection::None);

        controller.update(100, 480);
        let lengthened = (0..8)
            .filter(|_| controller.next_block() == SlipDirection::Lengthen)
            .count();
        assert!(lengthened >= 3, "{lengthened}");
        let mut one_sided = SlipController::new(750.0, false);
        one_sided.update(0, 480);
        assert_eq!(one_sided.next_block(), SlipDirection::None);
    }
}
