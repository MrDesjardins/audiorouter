//! Bounded WAV/MP3 decoding and allocation-free graph source playback.
//!
//! File parsing and resampling are control-plane work. `AudioFileSource::process`
//! only reads immutable decoded samples and atomics; it performs no allocation,
//! locking, filesystem access, decoder calls, or logging in the audio callback.

use crate::{AudioBlock, MAX_GRAPH_SAMPLE_RATE_HZ, MIN_GRAPH_SAMPLE_RATE_HZ};
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const MAX_AUDIO_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_AUDIO_FILE_SECONDS: u64 = 120;
const MAX_DECODED_SAMPLE_BYTES: usize = 64 * 1024 * 1024;
const STATE_STOPPED: u8 = 0;
const STATE_PLAYING: u8 = 1;
const STATE_PAUSED: u8 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedAudio {
    pub channels: usize,
    pub sample_rate_hz: u32,
    /// Interleaved, finite f32 samples. Decoder output is resampled to the
    /// requested graph rate before this value is returned.
    pub samples: Arc<[f32]>,
}

impl DecodedAudio {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioFileDecodeError {
    Io,
    UnsupportedOrMalformed,
    UnsupportedChannels,
    UnsupportedSampleRate,
    Empty,
    TooLarge,
    TooLong,
    InvalidTargetRate,
}

impl std::fmt::Display for AudioFileDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Io => "audio file could not be read",
            Self::UnsupportedOrMalformed => "audio file is malformed or unsupported",
            Self::UnsupportedChannels => "audio file must contain mono or stereo audio",
            Self::UnsupportedSampleRate => "audio file sample rate is outside the supported range",
            Self::Empty => "audio file contains no audio samples",
            Self::TooLarge => "audio file exceeds the 64 MiB import limit",
            Self::TooLong => "audio file exceeds the 120 second duration limit",
            Self::InvalidTargetRate => "graph sample rate is outside the supported range",
        })
    }
}

impl std::error::Error for AudioFileDecodeError {}

/// Decode one WAV/MP3 file to bounded interleaved float samples at graph rate.
/// Call only on a control/worker thread. `Read + Seek` permits both imported
/// files and bounded upload buffers without exposing arbitrary callback I/O.
pub fn decode_audio<R: Read + Seek + MediaSource + 'static>(
    reader: R,
    extension: &str,
    target_sample_rate_hz: u32,
) -> Result<DecodedAudio, AudioFileDecodeError> {
    if !(MIN_GRAPH_SAMPLE_RATE_HZ..=MAX_GRAPH_SAMPLE_RATE_HZ).contains(&target_sample_rate_hz) {
        return Err(AudioFileDecodeError::InvalidTargetRate);
    }
    let mut hint = Hint::new();
    hint.with_extension(extension.trim_start_matches('.'));
    let source = MediaSourceStream::new(Box::new(reader), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|_| AudioFileDecodeError::UnsupportedOrMalformed)?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or(AudioFileDecodeError::UnsupportedOrMalformed)?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|_| AudioFileDecodeError::UnsupportedOrMalformed)?;
    let mut decoded = Vec::<f32>::new();
    let mut source_channels = None;
    let mut source_rate = None;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(SymphoniaError::ResetRequired) => {
                return Err(AudioFileDecodeError::UnsupportedOrMalformed)
            }
            Err(_) => return Err(AudioFileDecodeError::UnsupportedOrMalformed),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let audio = decoder
            .decode(&packet)
            .map_err(|_| AudioFileDecodeError::UnsupportedOrMalformed)?;
        let channels = audio.spec().channels.count();
        let rate = audio.spec().rate;
        if !(1..=2).contains(&channels) {
            return Err(AudioFileDecodeError::UnsupportedChannels);
        }
        if !(MIN_GRAPH_SAMPLE_RATE_HZ..=MAX_GRAPH_SAMPLE_RATE_HZ).contains(&rate) {
            return Err(AudioFileDecodeError::UnsupportedSampleRate);
        }
        if source_channels.is_some_and(|previous| previous != channels)
            || source_rate.is_some_and(|previous| previous != rate)
        {
            return Err(AudioFileDecodeError::UnsupportedOrMalformed);
        }
        source_channels = Some(channels);
        source_rate = Some(rate);
        let mut buffer = SampleBuffer::<f32>::new(audio.capacity() as u64, *audio.spec());
        buffer.copy_interleaved_ref(audio);
        let new_len = decoded
            .len()
            .checked_add(buffer.samples().len())
            .ok_or(AudioFileDecodeError::TooLarge)?;
        if new_len
            .checked_mul(std::mem::size_of::<f32>())
            .map_or(true, |bytes| bytes > MAX_DECODED_SAMPLE_BYTES)
        {
            return Err(AudioFileDecodeError::TooLong);
        }
        decoded.extend_from_slice(buffer.samples());
    }
    let channels = source_channels.ok_or(AudioFileDecodeError::Empty)?;
    let rate = source_rate.ok_or(AudioFileDecodeError::Empty)?;
    if decoded.is_empty() || decoded.len() % channels != 0 {
        return Err(AudioFileDecodeError::Empty);
    }
    let source_frames = decoded.len() / channels;
    if source_frames as u64 > u64::from(rate) * MAX_AUDIO_FILE_SECONDS {
        return Err(AudioFileDecodeError::TooLong);
    }
    let resampled = resample_linear(&decoded, channels, rate, target_sample_rate_hz)?;
    Ok(DecodedAudio {
        channels,
        sample_rate_hz: target_sample_rate_hz,
        samples: resampled.into(),
    })
}

pub fn decode_audio_file(
    path: impl AsRef<Path>,
    target_sample_rate_hz: u32,
) -> Result<DecodedAudio, AudioFileDecodeError> {
    let path = path.as_ref();
    let metadata = std::fs::metadata(path).map_err(|_| AudioFileDecodeError::Io)?;
    if !metadata.is_file() {
        return Err(AudioFileDecodeError::Io);
    }
    if metadata.len() > MAX_AUDIO_FILE_BYTES {
        return Err(AudioFileDecodeError::TooLarge);
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    decode_audio(
        File::open(path).map_err(|_| AudioFileDecodeError::Io)?,
        extension,
        target_sample_rate_hz,
    )
}

/// Decode bounded in-memory upload bytes. Size is checked before the decoder
/// sees attacker-controlled data.
pub fn decode_audio_bytes(
    bytes: Vec<u8>,
    extension: &str,
    target_sample_rate_hz: u32,
) -> Result<DecodedAudio, AudioFileDecodeError> {
    if bytes.len() as u64 > MAX_AUDIO_FILE_BYTES {
        return Err(AudioFileDecodeError::TooLarge);
    }
    decode_audio(Cursor::new(bytes), extension, target_sample_rate_hz)
}

fn resample_linear(
    input: &[f32],
    channels: usize,
    source_rate: u32,
    target_rate: u32,
) -> Result<Vec<f32>, AudioFileDecodeError> {
    let source_frames = input.len() / channels;
    let target_frames_u64 = (source_frames as u64)
        .checked_mul(u64::from(target_rate))
        .ok_or(AudioFileDecodeError::TooLong)?
        / u64::from(source_rate);
    if target_frames_u64 > u64::from(target_rate) * MAX_AUDIO_FILE_SECONDS {
        return Err(AudioFileDecodeError::TooLong);
    }
    let target_frames =
        usize::try_from(target_frames_u64).map_err(|_| AudioFileDecodeError::TooLong)?;
    let target_samples = target_frames
        .checked_mul(channels)
        .filter(|samples| samples.saturating_mul(4) <= MAX_DECODED_SAMPLE_BYTES)
        .ok_or(AudioFileDecodeError::TooLong)?;
    let mut output = Vec::with_capacity(target_samples);
    for frame in 0..target_frames {
        let source_position = frame as f64 * f64::from(source_rate) / f64::from(target_rate);
        let first_frame = (source_position as usize).min(source_frames.saturating_sub(1));
        let second_frame = (first_frame + 1).min(source_frames.saturating_sub(1));
        let mix = (source_position - first_frame as f64) as f32;
        for channel in 0..channels {
            let a = input[first_frame * channels + channel];
            let b = input[second_frame * channels + channel];
            let sample = a + (b - a) * mix;
            output.push(if sample.is_finite() {
                sample.clamp(-1.0, 1.0)
            } else {
                0.0
            });
        }
    }
    Ok(output)
}

/// Shareable playback cursor. The graph callback owns the only sample reader;
/// the UI/API can change transport state with bounded atomics.
#[derive(Debug)]
pub struct AudioFileSource {
    audio: Arc<DecodedAudio>,
    cursor_frames: AtomicU64,
    state: AtomicU8,
    looping: AtomicBool,
}

impl AudioFileSource {
    pub fn new(audio: Arc<DecodedAudio>, looping: bool) -> Result<Self, AudioFileDecodeError> {
        if !matches!(audio.channels, 1 | 2)
            || !(MIN_GRAPH_SAMPLE_RATE_HZ..=MAX_GRAPH_SAMPLE_RATE_HZ)
                .contains(&audio.sample_rate_hz)
            || audio.samples.is_empty()
            || audio.samples.len() % audio.channels != 0
            || audio.samples.len().saturating_mul(4) > MAX_DECODED_SAMPLE_BYTES
        {
            return Err(AudioFileDecodeError::UnsupportedOrMalformed);
        }
        Ok(Self {
            audio,
            cursor_frames: AtomicU64::new(0),
            state: AtomicU8::new(STATE_STOPPED),
            looping: AtomicBool::new(looping),
        })
    }

    pub fn play(&self) {
        if self.cursor_frames.load(Ordering::Acquire) >= self.audio.frames() as u64 {
            self.cursor_frames.store(0, Ordering::Release);
        }
        self.state.store(STATE_PLAYING, Ordering::Release);
    }

    pub fn pause(&self) {
        self.state.store(STATE_PAUSED, Ordering::Release);
    }

    pub fn stop(&self) {
        self.state.store(STATE_STOPPED, Ordering::Release);
        self.cursor_frames.store(0, Ordering::Release);
    }

    pub fn set_looping(&self, looping: bool) {
        self.looping.store(looping, Ordering::Release);
    }

    pub fn is_looping(&self) -> bool {
        self.looping.load(Ordering::Acquire)
    }

    pub fn is_playing(&self) -> bool {
        self.state.load(Ordering::Acquire) == STATE_PLAYING
    }

    pub fn transport_state(&self) -> &'static str {
        match self.state.load(Ordering::Acquire) {
            STATE_PLAYING => "playing",
            STATE_PAUSED => "paused",
            _ => "stopped",
        }
    }

    pub fn process(&self, block: &mut AudioBlock) {
        block.clear();
        if self.state.load(Ordering::Acquire) != STATE_PLAYING {
            return;
        }
        let total_frames = self.audio.frames() as u64;
        let start = self.cursor_frames.load(Ordering::Relaxed);
        let looping = self.looping.load(Ordering::Acquire);
        let channels = block.channels();
        let source_channels = self.audio.channels;
        let data = &self.audio.samples;
        let mut consumed = 0u64;
        for frame in 0..block.frames() {
            let position = start.saturating_add(consumed);
            if position >= total_frames {
                if looping {
                    consumed = consumed.saturating_add(1);
                } else {
                    break;
                }
            } else {
                consumed = consumed.saturating_add(1);
            }
            let source_frame = if looping {
                position % total_frames
            } else {
                position
            } as usize;
            let left = data[source_frame * source_channels];
            let right = if source_channels == 2 {
                data[source_frame * 2 + 1]
            } else {
                left
            };
            if channels == 1 {
                if let Some(output) = block.channel_mut(0) {
                    output[frame] = (left + right) * 0.5;
                }
            } else {
                if let Some(output) = block.channel_mut(0) {
                    output[frame] = left;
                }
                if let Some(output) = block.channel_mut(1) {
                    output[frame] = right;
                }
            }
        }
        let next = start.saturating_add(consumed);
        if looping {
            self.cursor_frames
                .store(next % total_frames, Ordering::Release);
        } else if next >= total_frames {
            self.cursor_frames.store(total_frames, Ordering::Release);
            self.state.store(STATE_STOPPED, Ordering::Release);
        } else {
            self.cursor_frames.store(next, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn wav_mono_16(samples: &[i16], sample_rate: u32) -> Vec<u8> {
        let data_bytes = (samples.len() * 2) as u32;
        let mut wav = Vec::with_capacity(44 + data_bytes as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_bytes.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        wav
    }

    #[test]
    fn decodes_wav_to_target_rate_and_rejects_oversize() {
        let bytes = wav_mono_16(&[0, 16_384, -16_384, 0], 8_000);
        let decoded = decode_audio(Cursor::new(bytes), "wav", 48_000).unwrap();
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.sample_rate_hz, 48_000);
        assert_eq!(decoded.frames(), 24);
        assert!(decoded.samples.iter().all(|sample| sample.is_finite()));
        assert_eq!(
            decode_audio_bytes(vec![0; MAX_AUDIO_FILE_BYTES as usize + 1], "wav", 48_000),
            Err(AudioFileDecodeError::TooLarge)
        );
    }

    #[test]
    fn decodes_synthetic_mp3_fixture_to_finite_samples() {
        let bytes = include_bytes!("../tests/fixtures/synthetic-tone.mp3").to_vec();
        let decoded = decode_audio(Cursor::new(bytes), "mp3", 48_000).unwrap();
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.sample_rate_hz, 48_000);
        assert!(decoded.frames() > 0 && decoded.frames() < 48_000);
        assert!(decoded.samples.iter().all(|sample| sample.is_finite()));
        assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.001));
    }

    #[test]
    fn source_play_pause_stop_and_loop_run_without_allocating_in_process() {
        let audio = Arc::new(DecodedAudio {
            channels: 1,
            sample_rate_hz: 48_000,
            samples: vec![0.25, -0.25].into(),
        });
        let source = AudioFileSource::new(audio, false).unwrap();
        let mut block = AudioBlock::new(2, 2).unwrap();
        source.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.0, 0.0]);
        source.play();
        source.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.25, -0.25]);
        assert_eq!(block.channel(1).unwrap(), &[0.25, -0.25]);
        assert!(!source.is_playing());
        source.play();
        source.pause();
        source.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.0, 0.0]);
        source.set_looping(true);
        source.play();
        source.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.25, -0.25]);
        assert!(source.is_playing());
        source.stop();
        assert!(!source.is_playing());
    }
}
