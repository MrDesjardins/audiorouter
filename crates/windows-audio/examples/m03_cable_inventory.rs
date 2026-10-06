//! VM tool (WP-05/A3/A9 evidence): read-only inventory of the AudioRouter
//! cable endpoints. For every active endpoint whose name mentions
//! "AudioRouter" it reports the friendly name, the shared-mode mix format,
//! `IAudioClient3::GetSharedModeEnginePeriod`, and which of the 60 required
//! device formats (VCAB-11: 44.1/48/96 kHz x 1/2/4/6/8 channels x float32,
//! PCM16, PCM24-in-32, PCM32) Windows accepts in exclusive mode.
//!
//! It never starts a stream, changes a format or default, or opens a
//! microphone. Exit 0 when every expected endpoint is present, supports all
//! 60 formats and offers a minimum period <= --max-min-period (default 128
//! frames); exit 1 otherwise, with the reasons printed.
//!
//! Build: `cargo build --release -p audiorouter-windows-audio --example m03_cable_inventory`

const USAGE: &str =
    "usage: m03_cable_inventory [--cables 2] [--max-min-period 128] [--out PATH.json]";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Encoding {
    Float32,
    Pcm16,
    Pcm24In32,
    Pcm32,
}

impl Encoding {
    const ALL: [Encoding; 4] = [
        Encoding::Float32,
        Encoding::Pcm16,
        Encoding::Pcm24In32,
        Encoding::Pcm32,
    ];
    fn label(self) -> &'static str {
        match self {
            Encoding::Float32 => "float32",
            Encoding::Pcm16 => "pcm16",
            Encoding::Pcm24In32 => "pcm24in32",
            Encoding::Pcm32 => "pcm32",
        }
    }
    /// (container bits, valid bits, is float)
    fn bits(self) -> (u16, u16, bool) {
        match self {
            Encoding::Float32 => (32, 32, true),
            Encoding::Pcm16 => (16, 16, false),
            Encoding::Pcm24In32 => (32, 24, false),
            Encoding::Pcm32 => (32, 32, false),
        }
    }
}

const RATES: [u32; 3] = [44_100, 48_000, 96_000];
const CHANNELS: [u16; 5] = [1, 2, 4, 6, 8];

/// Standard KSAUDIO_SPEAKER_* masks for the advertised layouts.
fn channel_mask(channels: u16) -> u32 {
    match channels {
        1 => 0x4,   // MONO (front center)
        2 => 0x3,   // STEREO
        4 => 0x33,  // QUAD
        6 => 0x3F,  // 5.1
        8 => 0x63F, // 7.1 surround
        _ => 0,
    }
}

/// Bytes of a WAVEFORMATEXTENSIBLE (as used by the Windows audio APIs).
fn extensible_fields(
    rate: u32,
    channels: u16,
    encoding: Encoding,
) -> (u16, u32, u16, u16, u16, u32) {
    let (container, valid, _) = encoding.bits();
    let block_align = channels * (container / 8);
    (
        channels,
        rate,
        block_align,
        container,
        valid,
        rate * u32::from(block_align),
    )
}

fn expected_names(cables: u8) -> Vec<String> {
    (0..cables)
        .flat_map(|cable| {
            let letter = (b'A' + cable) as char;
            [
                format!("AudioRouter Cable {letter} Input"),
                format!("AudioRouter Cable {letter} Output"),
            ]
        })
        .collect()
}

/// Same rule as the helper and the VM runner: the exact name or
/// "<description> (<cable name>)".
fn name_matches(friendly: &str, expected: &str) -> bool {
    friendly == expected
        || friendly
            .strip_suffix(&format!(" ({expected})"))
            .is_some_and(|prefix| !prefix.is_empty() && !prefix.contains(['(', ')']))
        || friendly
            .strip_prefix(&format!("{expected} ("))
            .is_some_and(|rest| rest.ends_with(')') && !rest[..rest.len() - 1].contains(['(', ')']))
}

#[cfg(windows)]
fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut cables = 2_u8;
    let mut max_min_period = 128_u32;
    let mut out: Option<std::path::PathBuf> = None;
    // Developer self-check only: inspect other endpoints (e.g. "CABLE") to
    // prove the read-only COM path on a machine without the driver.
    let mut matcher: Option<String> = None;
    let mut iter = arguments.iter();
    while let Some(flag) = iter.next() {
        let value = iter.next().unwrap_or_else(|| {
            eprintln!("{USAGE}");
            std::process::exit(64)
        });
        match flag.as_str() {
            "--cables" => {
                cables = value
                    .parse()
                    .ok()
                    .filter(|n| (1..=8).contains(n))
                    .unwrap_or_else(|| exit_usage())
            }
            "--max-min-period" => max_min_period = value.parse().unwrap_or_else(|_| exit_usage()),
            "--out" => out = Some(value.into()),
            "--match" => matcher = Some(value.clone()),
            _ => exit_usage(),
        }
    }
    let (report, problems) = inventory(cables, max_min_period, matcher.as_deref());
    let text = serde_json::to_string_pretty(&report).unwrap();
    println!("{text}");
    if let Some(path) = out {
        if let Err(error) = std::fs::write(&path, &text) {
            eprintln!("cannot write {}: {error}", path.display());
            std::process::exit(1);
        }
    }
    if problems.is_empty() {
        eprintln!("PASS: {} AudioRouter endpoints, all 60 formats, minimum period <= {max_min_period} frames", cables * 2);
    } else {
        for problem in &problems {
            eprintln!("FAIL: {problem}");
        }
        std::process::exit(1);
    }
}

fn exit_usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(64)
}

#[cfg(windows)]
fn inventory(
    cables: u8,
    max_min_period: u32,
    matcher: Option<&str>,
) -> (serde_json::Value, Vec<String>) {
    use windows::Win32::Media::Audio::{
        IAudioClient3, IMMDeviceEnumerator, MMDeviceEnumerator, AUDCLNT_SHAREMODE_EXCLUSIVE,
        WAVEFORMATEX, WAVEFORMATEXTENSIBLE, WAVEFORMATEXTENSIBLE_0,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
        COINIT_MULTITHREADED,
    };
    const SUBTYPE_PCM: windows::core::GUID =
        windows::core::GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71);
    const SUBTYPE_FLOAT: windows::core::GUID =
        windows::core::GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);
    const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

    let mut problems = Vec::new();
    let all =
        audiorouter_windows_audio::enumerate_active_endpoint_display_info().unwrap_or_default();
    let needle = matcher.unwrap_or("AudioRouter");
    let ours: Vec<_> = all
        .into_iter()
        .filter(|info| info.name.contains(needle))
        .collect();
    let mut endpoints = Vec::new();
    for expected in if matcher.is_none() {
        expected_names(cables)
    } else {
        Vec::new()
    } {
        let found: Vec<_> = ours
            .iter()
            .filter(|info| name_matches(&info.name, &expected))
            .collect();
        if found.len() != 1 {
            problems.push(format!("{expected}: {} matching endpoints", found.len()));
        }
    }
    if matcher.is_none() && ours.len() != usize::from(cables) * 2 {
        problems.push(format!(
            "{} AudioRouter endpoints present, expected {}",
            ours.len(),
            usize::from(cables) * 2
        ));
    }
    // SAFETY: COM is initialized for this thread for the duration of the
    // loop and uninitialized afterwards; every format pointer returned by
    // GetMixFormat is freed with CoTaskMemFree; the WAVEFORMATEXTENSIBLE
    // passed to IsFormatSupported is a live local of the declared size.
    unsafe {
        if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
            problems.push("COM initialization failed".into());
            return (serde_json::json!({ "endpoints": [] }), problems);
        }
        let enumerator: Option<IMMDeviceEnumerator> =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok();
        for info in &ours {
            let id: Vec<u16> = info.id.encode_utf16().chain(std::iter::once(0)).collect();
            let client: Option<IAudioClient3> = enumerator
                .as_ref()
                .and_then(|e| e.GetDevice(windows::core::PCWSTR(id.as_ptr())).ok())
                .and_then(|device| device.Activate::<IAudioClient3>(CLSCTX_ALL, None).ok());
            let Some(client) = client else {
                problems.push(format!("{}: cannot activate IAudioClient3", info.name));
                continue;
            };
            let mut mix = serde_json::Value::Null;
            let mut periods = serde_json::Value::Null;
            if let Ok(format) = client.GetMixFormat() {
                // WAVEFORMATEX is packed: copy it out, never borrow its fields.
                let f: WAVEFORMATEX = std::ptr::read_unaligned(format);
                let (rate, channels, bits, tag) = (
                    f.nSamplesPerSec,
                    f.nChannels,
                    f.wBitsPerSample,
                    f.wFormatTag,
                );
                mix = serde_json::json!({ "rate": rate, "channels": channels, "bits": bits, "tag": tag });
                let (mut default, mut fundamental, mut min, mut max) = (0, 0, 0, 0);
                match client.GetSharedModeEnginePeriod(
                    format as *const WAVEFORMATEX,
                    &mut default,
                    &mut fundamental,
                    &mut min,
                    &mut max,
                ) {
                    Ok(()) => {
                        periods = serde_json::json!({ "default": default, "fundamental": fundamental, "min": min, "max": max });
                        // Compare at the 48 kHz reference used by MinPeriodFrames.
                        let min_at_48k = u64::from(min) * 48_000 / u64::from(rate.max(1));
                        if min_at_48k > u64::from(max_min_period) {
                            problems.push(format!("{}: minimum shared period {min} frames at {} Hz (> {max_min_period} at 48 kHz)", info.name, rate));
                        }
                    }
                    Err(error) => problems.push(format!(
                        "{}: GetSharedModeEnginePeriod failed: {}",
                        info.name,
                        error.message()
                    )),
                }
                CoTaskMemFree(Some(format as *const core::ffi::c_void));
            } else {
                problems.push(format!("{}: GetMixFormat failed", info.name));
            }
            let mut unsupported = Vec::new();
            let mut supported = 0;
            for rate in RATES {
                for channels in CHANNELS {
                    for encoding in Encoding::ALL {
                        let (channels, rate, block_align, container, valid, bytes_per_second) =
                            extensible_fields(rate, channels, encoding);
                        let format = WAVEFORMATEXTENSIBLE {
                            Format: WAVEFORMATEX {
                                wFormatTag: WAVE_FORMAT_EXTENSIBLE,
                                nChannels: channels,
                                nSamplesPerSec: rate,
                                nAvgBytesPerSec: bytes_per_second,
                                nBlockAlign: block_align,
                                wBitsPerSample: container,
                                cbSize: 22,
                            },
                            Samples: WAVEFORMATEXTENSIBLE_0 {
                                wValidBitsPerSample: valid,
                            },
                            dwChannelMask: channel_mask(channels),
                            SubFormat: if encoding.bits().2 {
                                SUBTYPE_FLOAT
                            } else {
                                SUBTYPE_PCM
                            },
                        };
                        let result = client.IsFormatSupported(
                            AUDCLNT_SHAREMODE_EXCLUSIVE,
                            &format.Format,
                            None,
                        );
                        if result.0 == 0 {
                            supported += 1;
                        } else {
                            unsupported.push(format!(
                                "{rate}Hz/{channels}ch/{} (0x{:08X})",
                                encoding.label(),
                                result.0 as u32
                            ));
                        }
                    }
                }
            }
            if supported != 60 {
                problems.push(format!("{}: {supported}/60 formats accepted", info.name));
            }
            endpoints.push(serde_json::json!({
                "name": info.name,
                "id": info.id,
                "direction": format!("{:?}", info.direction),
                "mixFormat": mix,
                "enginePeriodFrames": periods,
                "formatsSupported": supported,
                "formatsUnsupported": unsupported,
            }));
        }
        CoUninitialize();
    }
    (
        serde_json::json!({ "expectedCables": cables, "maxMinPeriodFrames": max_min_period, "endpoints": endpoints }),
        problems,
    )
}

#[cfg(not(windows))]
fn main() {
    eprintln!("m03_cable_inventory needs Windows");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_matrix_has_sixty_entries_with_valid_layouts() {
        let mut count = 0;
        for rate in RATES {
            for channels in CHANNELS {
                assert_eq!(channel_mask(channels).count_ones(), u32::from(channels));
                for encoding in Encoding::ALL {
                    let (_, _, block_align, container, valid, bytes) =
                        extensible_fields(rate, channels, encoding);
                    assert_eq!(block_align, channels * container / 8);
                    assert!(valid <= container);
                    assert_eq!(bytes, rate * u32::from(block_align));
                    count += 1;
                }
            }
        }
        assert_eq!(count, 60);
    }

    #[test]
    fn names_match_exact_and_composed_forms_only() {
        assert_eq!(expected_names(2).len(), 4);
        assert_eq!(expected_names(8)[15], "AudioRouter Cable H Output");
        for good in [
            "AudioRouter Cable A Input",
            "Speakers (AudioRouter Cable A Input)",
            "AudioRouter Cable A Input (AudioRouter Virtual Cable)",
            "AudioRouter Discord Input (AudioRouter Cable A Input)",
        ] {
            assert!(name_matches(good, "AudioRouter Cable A Input"), "{good}");
        }
        for bad in [
            "AudioRouter Cable A Output",
            "Speakers (AudioRouter Cable A Output)",
            "X (Y) (AudioRouter Cable A Input)",
            " (AudioRouter Cable A Input)",
            "AudioRouter Cable A Input 2",
        ] {
            assert!(!name_matches(bad, "AudioRouter Cable A Input"), "{bad}");
        }
    }
}
