//! Q4.3 bounded fuzzing of the recording file parsers: the header and
//! metadata readers the library listing runs over every recording, and the
//! crash-recovery routines that repair a WAV or streaming FLAC cut short by a
//! crash or a full disk.
//!
//! Fixed seeds and counts. Seeds are real writer output; cases truncate them
//! at every kind of position, flip bits, and corrupt length fields. Asserts:
//! no panic, no allocation beyond the small bound the parsers promise, and
//! that anything a recovery routine reports as repaired really inspects as a
//! consistent file of the reported frame count. `AUDIOROUTER_FUZZ_SCALE`
//! lengthens a local run.
use std::io::Cursor;
use std::path::{Path, PathBuf};

use audiorouter_recording::{
    inspect_flac_file, inspect_mp3_file, inspect_recording, inspect_wav_file, read_flac_metadata,
    read_wav_metadata, recover_streaming_flac_file, recover_wav_file, StreamingFlacWriter,
    WavFormat, WavMetadata, WavWriter,
};

#[path = "../../../tests/support/fuzz.rs"]
mod fuzz;

#[global_allocator]
static ALLOCATOR: fuzz::TrackingAllocator = fuzz::TrackingAllocator;

const SEED: u64 = 0x5245_434F_5244_4552; // "RECORDER"
const RATE: u32 = 48_000;
/// Metadata readers may hold one 64 KiB tag block; nothing else scales with a
/// declared length.
const READER_ALLOCATION_CAP_BYTES: usize = 256 * 1024;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("audiorouter-fuzz-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tone(frames: usize, channels: usize) -> Vec<f32> {
    (0..frames * channels)
        .map(|index| ((index / channels) as f32 * 0.05).sin() * 0.4)
        .collect()
}

fn wav_file(format: WavFormat, channels: u16, frames: usize, metadata: &WavMetadata) -> Vec<u8> {
    let mut writer =
        WavWriter::new(Cursor::new(Vec::new()), format, channels, RATE, false).unwrap();
    writer
        .write_interleaved(&tone(frames, usize::from(channels)))
        .unwrap();
    writer.finish_with_metadata(metadata).unwrap().into_inner()
}

fn flac_file(channels: u16, bits: u8, frames: usize, metadata: &WavMetadata) -> Vec<u8> {
    let mut writer = StreamingFlacWriter::new_with_metadata(
        Cursor::new(Vec::new()),
        channels,
        RATE,
        bits,
        false,
        metadata,
    )
    .unwrap();
    writer
        .write_interleaved(&tone(frames, usize::from(channels)))
        .unwrap();
    writer.finish().unwrap().into_inner()
}

fn tagged() -> WavMetadata {
    WavMetadata {
        title: Some("Take 1".into()),
        artist: Some("Fuzz".into()),
        comment: Some("seed recording".into()),
    }
}

fn seed_wavs() -> Vec<Vec<u8>> {
    vec![
        wav_file(WavFormat::Pcm16, 2, 300, &tagged()),
        wav_file(WavFormat::Pcm24, 1, 200, &WavMetadata::default()),
        wav_file(WavFormat::Float32, 2, 150, &tagged()),
    ]
}

fn seed_flacs() -> Vec<Vec<u8>> {
    vec![
        flac_file(2, 16, 5_000, &tagged()),
        flac_file(1, 24, 4_500, &WavMetadata::default()),
    ]
}

/// Runs a reader under the allocation bound.
fn bounded<R>(what: &str, bytes: &[u8], work: impl FnOnce() -> R) -> R {
    let (result, largest) = fuzz::measure_allocation(work);
    assert!(
        largest <= READER_ALLOCATION_CAP_BYTES,
        "{what} requested {largest} bytes for {} input bytes: {}",
        bytes.len(),
        fuzz::hex(bytes)
    );
    result
}

#[test]
fn seed_recordings_inspect_and_read_their_tags() {
    let scratch = Scratch::new("recording-seeds");
    for (index, seed) in seed_wavs().iter().enumerate() {
        let path = scratch.file(&format!("seed{index}.wav"), seed);
        let info = inspect_wav_file(&path).unwrap();
        assert!(info.frames > 0 && info.file_bytes == seed.len() as u64);
        let metadata = read_wav_metadata(&path).unwrap();
        if index != 1 {
            assert_eq!(metadata.title.as_deref(), Some("Take 1"));
        }
    }
    for (index, seed) in seed_flacs().iter().enumerate() {
        let path = scratch.file(&format!("seed{index}.flac"), seed);
        let info = inspect_flac_file(&path).unwrap();
        assert!(info.frames > 0);
        assert!(read_flac_metadata(&path).is_ok());
    }
}

#[test]
fn mutated_wav_files_never_panic_in_any_reader() {
    let scratch = Scratch::new("recording-wav");
    let seeds = seed_wavs();
    let mut inspected = 0_usize;
    fuzz::run("recording/wav-readers", SEED, 700, |_, rng| {
        let seed = rng.pick(&seeds).clone();
        let bytes = fuzz::mutate(rng, &seed, 16 * 1024);
        let path = scratch.file("case.wav", &bytes);
        let info = bounded("inspect_wav_file", &bytes, || inspect_wav_file(&path));
        if let Ok(info) = info {
            inspected += 1;
            assert!(
                info.data_bytes <= info.file_bytes - 44,
                "{}",
                fuzz::hex(&bytes)
            );
            let bytes_per_sample = match info.format {
                WavFormat::Pcm16 => 2,
                WavFormat::Pcm24 => 3,
                WavFormat::Float32 => 4,
            };
            assert_eq!(
                info.frames * u64::from(info.channels) * bytes_per_sample,
                info.data_bytes
            );
        }
        let _ = bounded("read_wav_metadata", &bytes, || read_wav_metadata(&path));
        let _ = inspect_recording(&path);
    });
    assert!(inspected > 20, "only {inspected} mutated WAVs inspected");
}

#[test]
fn mutated_flac_files_never_panic_in_any_reader() {
    let scratch = Scratch::new("recording-flac");
    let seeds = seed_flacs();
    let mut inspected = 0_usize;
    fuzz::run("recording/flac-readers", SEED ^ 1, 450, |_, rng| {
        let seed = rng.pick(&seeds).clone();
        let bytes = fuzz::mutate(rng, &seed, 16 * 1024);
        let path = scratch.file("case.flac", &bytes);
        let info = bounded("inspect_flac_file", &bytes, || inspect_flac_file(&path));
        if let Ok(info) = info {
            inspected += 1;
            assert!(info.frames > 0 && matches!(info.channels, 1 | 2));
        }
        let _ = bounded("read_flac_metadata", &bytes, || read_flac_metadata(&path));
        let _ = inspect_recording(&path);
    });
    assert!(inspected > 20, "only {inspected} mutated FLACs inspected");
}

/// Cut a recording at every kind of position, as a crash does, and require
/// the repair to either refuse or produce a file that inspects as consistent
/// with exactly the frame count it reported.
#[test]
fn a_truncated_wav_is_repaired_to_a_consistent_file_or_refused() {
    let scratch = Scratch::new("recording-wav-recover");
    let mut repaired = 0_usize;
    for (kind, format, channels) in [
        (0_u8, WavFormat::Pcm16, 2_u16),
        (1, WavFormat::Pcm24, 1),
        (2, WavFormat::Float32, 2),
    ] {
        let seed = wav_file(format, channels, 400, &WavMetadata::default());
        let lengths = (0..seed.len())
            .step_by(23)
            .chain([43, 44, 45, 46, 47, seed.len() - 1]);
        for length in lengths {
            let path = scratch.file("cut.wav", &seed[..length]);
            let mut file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();
            match recover_wav_file(&mut file, format, channels, RATE) {
                Ok(frames) => {
                    drop(file);
                    repaired += 1;
                    let info = inspect_wav_file(&path).unwrap_or_else(|error| {
                        panic!(
                            "kind {kind}, cut at {length}: repaired file does not inspect: {error:?}"
                        )
                    });
                    assert_eq!(info.frames, frames, "kind {kind}, cut at {length}");
                    assert_eq!(info.file_bytes, 44 + info.data_bytes);
                }
                Err(_) => assert!(length < 44, "kind {kind}: a {length}-byte file was refused"),
            }
        }
    }
    assert!(repaired > 100);
}

#[test]
fn a_truncated_or_corrupted_streaming_flac_is_repaired_to_a_consistent_file_or_refused() {
    let scratch = Scratch::new("recording-flac-recover");
    let seeds = [
        (flac_file(2, 16, 9_000, &tagged()), 2_u16, 16_u8),
        (flac_file(1, 24, 9_000, &WavMetadata::default()), 1, 24),
    ];
    let mut repaired = 0_usize;
    let mut refused = 0_usize;
    fuzz::run("recording/flac-recover", SEED ^ 2, 250, |_, rng| {
        let (seed, channels, bits) = rng.pick(&seeds).clone();
        let bytes = if rng.one_in(2) {
            // A crash: the tail is simply missing.
            seed[..rng.below(seed.len())].to_vec()
        } else {
            fuzz::mutate(rng, &seed, 64 * 1024)
        };
        let path = scratch.file("cut.flac", &bytes);
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let (result, largest) = fuzz::measure_allocation(|| {
            recover_streaming_flac_file(&mut file, channels, RATE, bits)
        });
        // One frame buffer (a few KiB) at most.
        assert!(
            largest <= READER_ALLOCATION_CAP_BYTES,
            "recovery requested {largest} bytes: {}",
            fuzz::hex(&bytes)
        );
        drop(file);
        match result {
            Ok(frames) => {
                repaired += 1;
                let info = inspect_flac_file(&path).unwrap_or_else(|error| {
                    panic!(
                        "repaired FLAC does not inspect ({error:?}): {}",
                        fuzz::hex(&bytes)
                    )
                });
                assert_eq!(info.frames, frames, "{}", fuzz::hex(&bytes));
                assert!(std::fs::metadata(&path).unwrap().len() <= bytes.len() as u64);
            }
            Err(_) => refused += 1,
        }
    });
    assert!(repaired > 30, "only {repaired} files were repaired");
    assert!(refused > 30, "only {refused} files were refused");
}

#[test]
fn mp3_and_unknown_extensions_are_classified_without_reading_past_the_header() {
    let scratch = Scratch::new("recording-mp3");
    fuzz::run("recording/mp3-inspect", SEED ^ 3, 300, |_, rng| {
        let mut bytes = rng.blob(0, 400);
        if rng.one_in(2) && bytes.len() >= 2 {
            bytes[0] = 0xFF;
            bytes[1] |= 0xE0;
        }
        for name in ["case.mp3", "case.MP3", "case.wav", "case.flac", "case"] {
            let path = scratch.file(name, &bytes);
            let _ = inspect_recording(&path);
        }
        let path = scratch.file("case.mp3", &bytes);
        let _ = inspect_mp3_file(&path);
    });
    // A missing file is data, not an error that stops a listing.
    assert!(inspect_recording(scratch.path().join("absent.wav")).is_ok());
}
