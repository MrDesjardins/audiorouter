//! Time Shift: a bounded "DVR" buffer that can pause, jump back, jump
//! forward, and return to live (Audio Hijack "Time Shift" equivalent).
//!
//! The ring is allocated once by [`TimeShift::new`]. Transport commands are
//! posted from the control thread through atomics and applied by the audio
//! thread at the next block boundary; `process` never allocates, locks, or
//! waits.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

/// Largest buffer in seconds.
pub const MAX_TIME_SHIFT_SECONDS: u32 = 120;
/// Jump size for the back/forward commands.
pub const TIME_SHIFT_JUMP_SECONDS: u32 = 10;
/// Fade-in after a jump or resume, to avoid a click.
const JUMP_FADE_FRAMES: usize = 256;

const COMMAND_NONE: u8 = 0;

/// Transport command posted by the control thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TimeShiftCommand {
    Pause = 1,
    Resume = 2,
    Back = 3,
    Forward = 4,
    Live = 5,
}

/// Snapshot of the transport for the control plane and UI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeShiftStatus {
    pub paused: bool,
    /// How far behind live the output is, in seconds.
    pub delay_seconds: f32,
    /// How much recorded audio is available to rewind into, in seconds.
    pub buffered_seconds: f32,
    pub capacity_seconds: f32,
}

/// Lock-free command mailbox and status shared with the control thread.
#[derive(Debug, Default)]
pub struct TimeShiftTransport {
    command: AtomicU8,
    paused: AtomicU8,
    delay_frames: AtomicU64,
    buffered_frames: AtomicU64,
}

impl TimeShiftTransport {
    /// Post a command; a newer command replaces one not yet applied.
    pub fn post(&self, command: TimeShiftCommand) {
        self.command.store(command as u8, Ordering::Release);
    }
}

/// Multi-channel time-shift buffer.
#[derive(Debug)]
pub struct TimeShift {
    channels: usize,
    capacity: usize,
    sample_rate: u32,
    ring: Vec<f32>,
    written: u64,
    delay: u64,
    paused: bool,
    fade_remaining: usize,
    block_frames: usize,
}

impl TimeShift {
    pub fn new(channels: usize, seconds: u32, sample_rate: u32) -> Self {
        let channels = channels.clamp(1, 2);
        let sample_rate = sample_rate.max(1);
        let capacity = seconds.clamp(1, MAX_TIME_SHIFT_SECONDS) as usize * sample_rate as usize;
        Self {
            channels,
            capacity,
            sample_rate,
            ring: vec![0.0; channels * capacity],
            written: 0,
            delay: 0,
            paused: false,
            fade_remaining: 0,
            block_frames: 0,
        }
    }

    pub fn capacity_seconds(&self) -> u32 {
        (self.capacity / self.sample_rate as usize) as u32
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Audio that can be rewound into: recorded frames minus one block of
    /// headroom so the read position never overtakes the write position.
    fn available(&self, frames: usize) -> u64 {
        self.written.min(self.capacity.saturating_sub(frames) as u64)
    }

    /// Apply any posted command, then record this block. Call once per block
    /// before [`Self::read_channel`] for each channel.
    pub fn begin_block(&mut self, transport: &TimeShiftTransport, inputs: [Option<&[f32]>; 2]) {
        let frames = inputs[0].map_or(0, <[f32]>::len);
        self.block_frames = frames;
        let jump = TIME_SHIFT_JUMP_SECONDS as u64 * self.sample_rate as u64;
        let before = (self.delay, self.paused);
        match transport.command.swap(COMMAND_NONE, Ordering::Acquire) {
            code if code == TimeShiftCommand::Pause as u8 => self.paused = true,
            code if code == TimeShiftCommand::Resume as u8 => self.paused = false,
            code if code == TimeShiftCommand::Back as u8 => {
                self.delay = (self.delay + jump).min(self.available(frames));
            }
            code if code == TimeShiftCommand::Forward as u8 => self.delay = self.delay.saturating_sub(jump),
            code if code == TimeShiftCommand::Live as u8 => {
                self.delay = 0;
                self.paused = false;
            }
            _ => {}
        }
        if (self.delay, self.paused) != before && !self.paused {
            self.fade_remaining = JUMP_FADE_FRAMES;
        }
        for (channel, input) in inputs.iter().enumerate().take(self.channels) {
            let Some(input) = input else { continue };
            for (offset, sample) in input.iter().enumerate() {
                let position = ((self.written + offset as u64) % self.capacity as u64) as usize;
                self.ring[channel * self.capacity + position] = if sample.is_finite() { *sample } else { 0.0 };
            }
        }
    }

    /// Fill `output` for one channel from the delayed position.
    pub fn read_channel(&self, channel: usize, output: &mut [f32]) {
        if self.paused || channel >= self.channels {
            output.fill(0.0);
            return;
        }
        for (offset, sample) in output.iter_mut().enumerate() {
            let position = ((self.written + offset as u64 - self.delay) % self.capacity as u64) as usize;
            let fade = if offset < self.fade_remaining {
                (JUMP_FADE_FRAMES - self.fade_remaining + offset) as f32 / JUMP_FADE_FRAMES as f32
            } else {
                1.0
            };
            *sample = self.ring[channel * self.capacity + position] * fade.min(1.0);
        }
    }

    /// Finish the block: advance time and publish status. While paused the
    /// output falls further behind live, up to the buffer capacity.
    pub fn end_block(&mut self, transport: &TimeShiftTransport) {
        let frames = self.block_frames as u64;
        self.written += frames;
        if self.paused {
            self.delay = (self.delay + frames).min(self.available(self.block_frames));
        }
        self.fade_remaining = self.fade_remaining.saturating_sub(self.block_frames);
        transport.paused.store(u8::from(self.paused), Ordering::Relaxed);
        transport.delay_frames.store(self.delay, Ordering::Relaxed);
        transport
            .buffered_frames
            .store(self.available(self.block_frames), Ordering::Relaxed);
    }

    pub fn reset(&mut self) {
        self.ring.fill(0.0);
        self.written = 0;
        self.delay = 0;
        self.paused = false;
        self.fade_remaining = 0;
    }

    pub fn status(&self, transport: &TimeShiftTransport) -> TimeShiftStatus {
        let rate = self.sample_rate as f32;
        TimeShiftStatus {
            paused: transport.paused.load(Ordering::Relaxed) != 0,
            delay_seconds: transport.delay_frames.load(Ordering::Relaxed) as f32 / rate,
            buffered_seconds: transport.buffered_frames.load(Ordering::Relaxed) as f32 / rate,
            capacity_seconds: self.capacity as f32 / rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `blocks` blocks of 100 frames where each sample's value is its
    /// absolute input frame index, returning the last output block.
    fn run(shift: &mut TimeShift, transport: &TimeShiftTransport, start: &mut u64, blocks: usize) -> Vec<f32> {
        let mut output = vec![0.0; 100];
        for _ in 0..blocks {
            let input: Vec<f32> = (0..100).map(|offset| (*start + offset) as f32).collect();
            shift.begin_block(transport, [Some(&input), None]);
            shift.read_channel(0, &mut output);
            shift.end_block(transport);
            *start += 100;
        }
        output
    }

    #[test]
    fn live_pause_back_forward_and_live() {
        // 1 frame per "second" keeps the arithmetic readable: jumps are 10 frames.
        let transport = TimeShiftTransport::default();
        let mut shift = TimeShift::new(1, 100, 100);
        let mut frame = 0;
        let live = run(&mut shift, &transport, &mut frame, 20);
        assert_eq!(live[99], 1_999.0, "live passes the current input");

        transport.post(TimeShiftCommand::Pause);
        let paused = run(&mut shift, &transport, &mut frame, 3);
        assert!(paused.iter().all(|sample| *sample == 0.0));
        let status = shift.status(&transport);
        assert!(status.paused && (status.delay_seconds - 3.0).abs() < 1e-6);

        // Each check runs three blocks so the 256-frame fade-in has finished.
        transport.post(TimeShiftCommand::Resume);
        let resumed = run(&mut shift, &transport, &mut frame, 3);
        assert_eq!(resumed[99], 2_599.0 - 300.0, "resumes where it paused");

        transport.post(TimeShiftCommand::Back);
        let back = run(&mut shift, &transport, &mut frame, 3);
        assert_eq!(back[99], 2_899.0 - 1_300.0);

        transport.post(TimeShiftCommand::Forward);
        let forward = run(&mut shift, &transport, &mut frame, 3);
        assert_eq!(forward[99], 3_199.0 - 300.0);

        transport.post(TimeShiftCommand::Live);
        let again = run(&mut shift, &transport, &mut frame, 3);
        assert_eq!(again[99], 3_499.0);
        assert_eq!(shift.status(&transport).delay_seconds, 0.0);
    }

    #[test]
    fn rewind_is_bounded_by_recorded_audio_and_capacity() {
        let transport = TimeShiftTransport::default();
        let mut shift = TimeShift::new(2, 3, 100);
        let mut frame = 0;
        run(&mut shift, &transport, &mut frame, 2);
        transport.post(TimeShiftCommand::Back);
        run(&mut shift, &transport, &mut frame, 1);
        assert!(shift.status(&transport).delay_seconds <= 2.0);
        transport.post(TimeShiftCommand::Pause);
        run(&mut shift, &transport, &mut frame, 10);
        assert!(shift.status(&transport).delay_seconds <= 3.0 - 1.0 + 1e-6);
        assert_eq!(shift.capacity_seconds(), 3);
    }
}
