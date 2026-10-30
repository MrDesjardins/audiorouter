//! Soundboard source: up to nine slots of short, already decoded sounds that
//! play on demand (SND-01 to SND-20).
//!
//! Decoding, resampling and slot setup are control-plane work. Triggers reach
//! the audio callback through a bounded lock-free queue, and `process` only
//! reads immutable decoded samples, drains that queue and mixes into the
//! caller's block: no allocation, locking, filesystem access, logging or
//! waiting. If the voice state is busy (a telemetry reader), the block is
//! silent rather than blocked.

use crate::{AudioBlock, DecodedAudio, RealtimeDsp};
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::Arc;

pub const SOUNDBOARD_SLOT_COUNT: usize = 9;
/// Sounds that may overlap at once; the oldest voice is replaced beyond this.
const MAX_VOICES: usize = 16;
const COMMAND_CAPACITY: usize = 64;
/// Triggers of one slot closer than this are ignored (SND-20).
const MIN_TRIGGER_GAP_MS: u64 = 50;
/// A stopped or restarted sound fades out over this long, so it never clicks.
const STOP_FADE_MS: u64 = 5;
/// Output stays below this ceiling no matter how many sounds overlap.
const LIMITER_KNEE: f32 = 0.8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotMode {
    /// Plays once; a second trigger restarts it (SND-16 default).
    Once,
    Loop,
    /// The next trigger stops it.
    Toggle,
    /// Plays while the trigger is held; `release` stops it.
    Hold,
}

impl SlotMode {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "once" => Some(Self::Once),
            "loop" => Some(Self::Loop),
            "toggle" => Some(Self::Toggle),
            "hold" => Some(Self::Hold),
            _ => None,
        }
    }
}

/// One prepared slot. More than one sound makes it a random set (SND-17).
#[derive(Clone, Debug)]
pub struct SoundboardSlot {
    pub sounds: Vec<Arc<DecodedAudio>>,
    pub gain_db: f32,
    pub mode: SlotMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    /// The slot, and a mode that replaces the slot's own for this press.
    Trigger(u8, Option<SlotMode>),
    Release(u8),
    Stop(u8),
    StopAll,
}

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    slot: u8,
    sound: u8,
    looping: bool,
    /// Started in `hold` mode: a release stops it.
    hold: bool,
    cursor: usize,
    gain: f32,
    /// Frames of fade-out left, or 0 when the voice is not fading.
    fade_left: u32,
    /// Frames of fade-out at the start, for the ramp slope.
    fade_total: u32,
}

const IDLE_VOICE: Voice = Voice {
    active: false,
    slot: 0,
    sound: 0,
    looping: false,
    hold: false,
    cursor: 0,
    gain: 0.0,
    fade_left: 0,
    fade_total: 0,
};

struct SoundboardVoices {
    voices: [Voice; MAX_VOICES],
    /// Frames rendered so far; the clock for the trigger gap.
    clock: u64,
    last_trigger: [Option<u64>; SOUNDBOARD_SLOT_COUNT],
    last_sound: [Option<u8>; SOUNDBOARD_SLOT_COUNT],
    rng: u32,
}

/// Shared handle: control code triggers sounds and reads status; the audio
/// callback renders them.
pub struct SoundboardSource {
    slots: [Option<SoundboardSlot>; SOUNDBOARD_SLOT_COUNT],
    sample_rate_hz: u32,
    commands: ArrayQueue<Command>,
    playing: AtomicU16,
    /// Per slot, how far the furthest voice is through its sound (0 to 1000).
    progress: [AtomicU16; SOUNDBOARD_SLOT_COUNT],
    ignored: AtomicU64,
    dropped: AtomicU64,
    state: RealtimeDsp<SoundboardVoices>,
}

impl std::fmt::Debug for SoundboardSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SoundboardSource")
            .field("playing", &self.playing_mask())
            .finish_non_exhaustive()
    }
}

impl SoundboardSource {
    /// `slots[i]` is slot `i + 1`; a slot with no usable sound is `None`.
    /// Sounds must already be at `sample_rate_hz` (decoded for the graph).
    pub fn new(
        slots: [Option<SoundboardSlot>; SOUNDBOARD_SLOT_COUNT],
        sample_rate_hz: u32,
    ) -> Self {
        Self {
            slots,
            sample_rate_hz,
            commands: ArrayQueue::new(COMMAND_CAPACITY),
            playing: AtomicU16::new(0),
            progress: std::array::from_fn(|_| AtomicU16::new(0)),
            ignored: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            state: RealtimeDsp::new(SoundboardVoices {
                voices: [IDLE_VOICE; MAX_VOICES],
                clock: 0,
                last_trigger: [None; SOUNDBOARD_SLOT_COUNT],
                last_sound: [None; SOUNDBOARD_SLOT_COUNT],
                rng: 0x9E37_79B9,
            }),
        }
    }

    /// Whether slot `slot` (0-based) has a sound to play.
    pub fn slot_loaded(&self, slot: usize) -> bool {
        self.slots.get(slot).is_some_and(Option::is_some)
    }

    fn push(&self, command: Command) -> bool {
        if self.commands.push(command).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        true
    }

    /// Start slot `slot` (0-based). False when the slot is empty or the
    /// queue is full.
    pub fn trigger(&self, slot: usize) -> bool {
        self.trigger_with(slot, None)
    }

    /// Like `trigger`, with a mode that replaces the slot's own for this press
    /// (a Stream Deck key chooses how it plays).
    pub fn trigger_with(&self, slot: usize, mode: Option<SlotMode>) -> bool {
        slot < SOUNDBOARD_SLOT_COUNT
            && self.slot_loaded(slot)
            && self.push(Command::Trigger(slot as u8, mode))
    }

    /// The trigger was let go; stops a `hold` slot.
    pub fn release(&self, slot: usize) -> bool {
        slot < SOUNDBOARD_SLOT_COUNT && self.push(Command::Release(slot as u8))
    }

    pub fn stop(&self, slot: usize) -> bool {
        slot < SOUNDBOARD_SLOT_COUNT && self.push(Command::Stop(slot as u8))
    }

    pub fn stop_all(&self) -> bool {
        self.push(Command::StopAll)
    }

    /// Bit `i` is set while slot `i + 1` is playing.
    pub fn playing_mask(&self) -> u16 {
        self.playing.load(Ordering::Acquire)
    }

    /// 0 to 1000 through the furthest voice of the slot.
    pub fn progress_per_mille(&self, slot: usize) -> u16 {
        self.progress
            .get(slot)
            .map_or(0, |value| value.load(Ordering::Acquire))
    }

    /// Triggers ignored because they came inside the minimum gap.
    pub fn ignored_triggers(&self) -> u64 {
        self.ignored.load(Ordering::Relaxed)
    }

    /// Commands lost because the queue was full.
    pub fn dropped_commands(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Renders the next block. Silent when the voice state is busy.
    pub fn process(&self, block: &mut AudioBlock) {
        block.clear();
        let _ = self.state.try_with(|state| self.render(state, block));
    }

    fn render(&self, state: &mut SoundboardVoices, block: &mut AudioBlock) {
        let frames = block.frames();
        let gap_frames = u64::from(self.sample_rate_hz) * MIN_TRIGGER_GAP_MS / 1000;
        let fade_frames = (u64::from(self.sample_rate_hz) * STOP_FADE_MS / 1000).max(1) as u32;
        while let Some(command) = self.commands.pop() {
            match command {
                Command::Trigger(slot, mode) => {
                    self.start(state, slot, mode, gap_frames, fade_frames)
                }
                Command::Release(slot) => fade_held(state, slot, fade_frames),
                Command::Stop(slot) => fade_slot(state, slot, fade_frames),
                Command::StopAll => {
                    for slot in 0..SOUNDBOARD_SLOT_COUNT as u8 {
                        fade_slot(state, slot, fade_frames);
                    }
                }
            }
        }
        let channels = block.channels();
        for voice in state.voices.iter_mut().filter(|voice| voice.active) {
            let Some(config) = self.slots[usize::from(voice.slot)].as_ref() else {
                voice.active = false;
                continue;
            };
            let sound = &config.sounds[usize::from(voice.sound).min(config.sounds.len() - 1)];
            let length = sound.frames();
            let source_channels = sound.channels;
            let remaining = if voice.looping {
                frames
            } else {
                length.saturating_sub(voice.cursor).min(frames)
            };
            for channel in 0..channels {
                let Some(out) = block.channel_mut(channel) else {
                    continue;
                };
                let source_channel = channel.min(source_channels - 1);
                for (frame, sample) in out.iter_mut().take(remaining).enumerate() {
                    let fade = if voice.fade_total == 0 {
                        1.0
                    } else {
                        let left = i64::from(voice.fade_left) - frame as i64;
                        if left <= 0 {
                            0.0
                        } else {
                            left as f32 / voice.fade_total as f32
                        }
                    };
                    let position = (voice.cursor + frame) % length;
                    *sample += sound.samples[position * source_channels + source_channel]
                        * voice.gain
                        * fade;
                }
            }
            voice.cursor += remaining;
            if voice.looping {
                voice.cursor %= length;
            }
            if voice.fade_total != 0 {
                voice.fade_left = voice.fade_left.saturating_sub(frames as u32);
                if voice.fade_left == 0 {
                    voice.active = false;
                }
            }
            if !voice.looping && voice.cursor >= length {
                voice.active = false;
            }
        }
        for channel in 0..channels {
            if let Some(out) = block.channel_mut(channel) {
                for sample in out.iter_mut() {
                    *sample = soft_limit(*sample);
                }
            }
        }
        state.clock += frames as u64;
        self.publish_status(state);
    }

    fn start(
        &self,
        state: &mut SoundboardVoices,
        slot: u8,
        mode: Option<SlotMode>,
        gap_frames: u64,
        fade_frames: u32,
    ) {
        let index = usize::from(slot);
        let Some(config) = self.slots[index].as_ref() else {
            return;
        };
        if state.last_trigger[index]
            .is_some_and(|last| state.clock.saturating_sub(last) < gap_frames)
        {
            self.ignored.fetch_add(1, Ordering::Relaxed);
            return;
        }
        state.last_trigger[index] = Some(state.clock);
        let playing = state
            .voices
            .iter()
            .any(|voice| voice.active && voice.slot == slot && voice.fade_total == 0);
        let mode = mode.unwrap_or(config.mode);
        // Toggle and Loop both stop on the next press.
        if matches!(mode, SlotMode::Toggle | SlotMode::Loop) && playing {
            fade_slot(state, slot, fade_frames);
            return;
        }
        // A retrigger restarts the sound; the old one fades out under it.
        fade_slot(state, slot, fade_frames);
        let sound = if config.sounds.len() > 1 {
            // Never the same sound twice in a row (SND-17).
            state.rng ^= state.rng << 13;
            state.rng ^= state.rng >> 17;
            state.rng ^= state.rng << 5;
            let count = config.sounds.len() as u32;
            let mut pick = (state.rng % count) as u8;
            if state.last_sound[index] == Some(pick) {
                pick = (pick + 1) % count as u8;
            }
            state.last_sound[index] = Some(pick);
            pick
        } else {
            0
        };
        let target = state
            .voices
            .iter()
            .position(|voice| !voice.active)
            .unwrap_or_else(|| {
                // Replace the voice that has played the longest.
                state
                    .voices
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, voice)| voice.cursor)
                    .map_or(0, |(position, _)| position)
            });
        state.voices[target] = Voice {
            active: true,
            slot,
            sound,
            looping: mode == SlotMode::Loop,
            hold: mode == SlotMode::Hold,
            cursor: 0,
            gain: 10f32.powf(config.gain_db / 20.0),
            fade_left: 0,
            fade_total: 0,
        };
    }

    fn publish_status(&self, state: &SoundboardVoices) {
        let mut mask = 0u16;
        for slot in 0..SOUNDBOARD_SLOT_COUNT {
            let mut best = 0u16;
            let mut any = false;
            for voice in state
                .voices
                .iter()
                .filter(|voice| voice.active && usize::from(voice.slot) == slot)
            {
                any = true;
                if let Some(config) = self.slots[slot].as_ref() {
                    let length = config.sounds
                        [usize::from(voice.sound).min(config.sounds.len() - 1)]
                    .frames()
                    .max(1);
                    best = best.max((voice.cursor.min(length) * 1000 / length) as u16);
                }
            }
            if any {
                mask |= 1 << slot;
            }
            self.progress[slot].store(best, Ordering::Release);
        }
        self.playing.store(mask, Ordering::Release);
    }
}

fn fade_slot(state: &mut SoundboardVoices, slot: u8, fade_frames: u32) {
    for voice in state
        .voices
        .iter_mut()
        .filter(|voice| voice.active && voice.slot == slot && voice.fade_total == 0)
    {
        voice.fade_total = fade_frames;
        voice.fade_left = fade_frames;
    }
}

/// A release stops only the voices that were started in `hold` mode.
fn fade_held(state: &mut SoundboardVoices, slot: u8, fade_frames: u32) {
    for voice in state
        .voices
        .iter_mut()
        .filter(|voice| voice.active && voice.slot == slot && voice.hold && voice.fade_total == 0)
    {
        voice.fade_total = fade_frames;
        voice.fade_left = fade_frames;
    }
}

/// Transparent below the knee, then a smooth curve that stays below 1.0.
fn soft_limit(sample: f32) -> f32 {
    let magnitude = sample.abs();
    if magnitude <= LIMITER_KNEE {
        return sample;
    }
    let room = 1.0 - LIMITER_KNEE;
    let limited = LIMITER_KNEE + room * ((magnitude - LIMITER_KNEE) / room).tanh();
    limited.copysign(sample)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn sound(frames: usize, value: f32) -> Arc<DecodedAudio> {
        Arc::new(DecodedAudio {
            channels: 1,
            sample_rate_hz: RATE,
            samples: vec![value; frames].into(),
        })
    }

    fn board(slots: Vec<(Vec<Arc<DecodedAudio>>, SlotMode)>) -> SoundboardSource {
        let mut prepared: [Option<SoundboardSlot>; SOUNDBOARD_SLOT_COUNT] =
            std::array::from_fn(|_| None);
        for (index, (sounds, mode)) in slots.into_iter().enumerate() {
            prepared[index] = Some(SoundboardSlot {
                sounds,
                gain_db: 0.0,
                mode,
            });
        }
        SoundboardSource::new(prepared, RATE)
    }

    fn block() -> AudioBlock {
        AudioBlock::new(2, crate::PROCESSING_QUANTUM_FRAMES).unwrap()
    }

    fn energy(block: &AudioBlock) -> f32 {
        block
            .channel(0)
            .unwrap()
            .iter()
            .map(|sample| sample.abs())
            .sum()
    }

    #[test]
    fn an_untriggered_board_is_silent() {
        let board = board(vec![(vec![sound(4_800, 0.5)], SlotMode::Once)]);
        let mut out = block();
        board.process(&mut out);
        assert_eq!(energy(&out), 0.0);
        assert_eq!(board.playing_mask(), 0);
    }

    #[test]
    fn a_trigger_plays_the_sound_once_then_stops() {
        let board = board(vec![(vec![sound(600, 0.5)], SlotMode::Once)]);
        assert!(board.trigger(0));
        let mut out = block();
        board.process(&mut out);
        assert!(energy(&out) > 0.0);
        assert_eq!(board.playing_mask(), 1);
        // 600 frames end within five 128-frame blocks.
        for _ in 0..5 {
            board.process(&mut out);
        }
        assert_eq!(board.playing_mask(), 0);
        board.process(&mut out);
        assert_eq!(energy(&out), 0.0);
    }

    #[test]
    fn an_empty_slot_cannot_be_triggered() {
        let board = board(vec![]);
        assert!(!board.trigger(0));
        assert!(!board.trigger(SOUNDBOARD_SLOT_COUNT));
    }

    #[test]
    fn triggers_inside_the_gap_are_ignored_and_counted() {
        let board = board(vec![(vec![sound(48_000, 0.1)], SlotMode::Once)]);
        let mut out = block();
        board.trigger(0);
        board.trigger(0);
        board.process(&mut out);
        assert_eq!(board.ignored_triggers(), 1);
    }

    #[test]
    fn toggle_stops_on_the_second_trigger() {
        let board = board(vec![(vec![sound(480_000, 0.2)], SlotMode::Toggle)]);
        let mut out = block();
        board.trigger(0);
        board.process(&mut out);
        assert_eq!(board.playing_mask(), 1);
        // Clear the gap, then toggle off; the fade ends within a few blocks.
        for _ in 0..20 {
            board.process(&mut out);
        }
        board.trigger(0);
        for _ in 0..4 {
            board.process(&mut out);
        }
        assert_eq!(board.playing_mask(), 0);
    }

    #[test]
    fn hold_stops_on_release_and_loop_keeps_going() {
        let held = board(vec![(vec![sound(480_000, 0.2)], SlotMode::Hold)]);
        let mut out = block();
        held.trigger(0);
        held.process(&mut out);
        assert_eq!(held.playing_mask(), 1);
        held.release(0);
        for _ in 0..4 {
            held.process(&mut out);
        }
        assert_eq!(held.playing_mask(), 0);

        let looped = board(vec![(vec![sound(100, 0.2)], SlotMode::Loop)]);
        looped.trigger(0);
        for _ in 0..20 {
            looped.process(&mut out);
        }
        assert_eq!(looped.playing_mask(), 1);
        looped.stop_all();
        for _ in 0..4 {
            looped.process(&mut out);
        }
        assert_eq!(looped.playing_mask(), 0);
    }

    #[test]
    fn a_press_can_choose_how_the_slot_plays() {
        let mut out = block();
        // A "play once" slot played as hold stops on release.
        let held = board(vec![(vec![sound(480_000, 0.2)], SlotMode::Once)]);
        held.trigger_with(0, Some(SlotMode::Hold));
        held.process(&mut out);
        assert_eq!(held.playing_mask(), 1);
        held.release(0);
        for _ in 0..4 {
            held.process(&mut out);
        }
        assert_eq!(held.playing_mask(), 0);
        // Without the override, the same slot ignores a release.
        let plain = board(vec![(vec![sound(480_000, 0.2)], SlotMode::Once)]);
        plain.trigger(0);
        plain.process(&mut out);
        plain.release(0);
        for _ in 0..4 {
            plain.process(&mut out);
        }
        assert_eq!(plain.playing_mask(), 1);
        // Loop plays until the same key is pressed again.
        let looped = board(vec![(vec![sound(100, 0.2)], SlotMode::Once)]);
        looped.trigger_with(0, Some(SlotMode::Loop));
        for _ in 0..30 {
            looped.process(&mut out);
        }
        assert_eq!(looped.playing_mask(), 1);
        looped.trigger_with(0, Some(SlotMode::Loop));
        for _ in 0..30 {
            looped.process(&mut out);
        }
        assert_eq!(looped.playing_mask(), 0);
    }

    #[test]
    fn a_stop_fades_instead_of_cutting() {
        let board = board(vec![(vec![sound(480_000, 0.5)], SlotMode::Once)]);
        let mut out = block();
        board.trigger(0);
        board.process(&mut out);
        board.stop(0);
        board.process(&mut out);
        let samples = out.channel(0).unwrap();
        let step = samples
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(step < 0.01, "largest step {step}");
        assert!(samples[0] > samples[samples.len() - 1]);
    }

    #[test]
    fn a_random_set_never_repeats_a_sound_twice_in_a_row() {
        let set = vec![sound(48, 0.1), sound(48, 0.2), sound(48, 0.3)];
        let board = board(vec![(set, SlotMode::Once)]);
        let mut out = block();
        let mut previous = None;
        for _ in 0..40 {
            board.trigger(0);
            board.process(&mut out);
            let level = out.channel(0).unwrap()[0];
            assert!(previous != Some(level.to_bits()));
            previous = Some(level.to_bits());
            // Let the trigger gap pass.
            for _ in 0..20 {
                board.process(&mut out);
            }
        }
    }

    #[test]
    fn overlapping_sounds_stay_below_full_scale() {
        let loud = sound(4_800, 0.9);
        let board = board(
            (0..SOUNDBOARD_SLOT_COUNT)
                .map(|_| (vec![Arc::clone(&loud)], SlotMode::Once))
                .collect(),
        );
        for slot in 0..SOUNDBOARD_SLOT_COUNT {
            board.trigger(slot);
        }
        let mut out = block();
        board.process(&mut out);
        let peak = out
            .channel(0)
            .unwrap()
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(peak <= 1.0 && peak > 0.9, "peak {peak}");
    }

    #[test]
    fn progress_rises_while_playing() {
        let board = board(vec![(vec![sound(4_800, 0.1)], SlotMode::Once)]);
        let mut out = block();
        board.trigger(0);
        board.process(&mut out);
        let first = board.progress_per_mille(0);
        board.process(&mut out);
        assert!(board.progress_per_mille(0) > first);
    }
}
