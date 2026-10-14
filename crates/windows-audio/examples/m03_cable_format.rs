//! VM tool (8-channel spike, 2026-10-12): read or set the Windows device
//! format of AudioRouter cable endpoints. Windows disables speaker setup for
//! the line-level cable render endpoints and lists capture formats only up
//! to the current channel count (r29), so no Sound-settings path reaches
//! 7.1. This tool tests the other route: the undocumented `IPolicyConfig`
//! interface that the Sound control panel's "Default Format" list uses.
//!
//! `show` is read-only. `set --channels N [--rate R] [--endpoint NAME]...`
//! changes the shared-mode device format of the named endpoints (default:
//! Cable A Input and Cable B Output) to float32 at R Hz (default 48000) with
//! the standard mask for N channels, then re-reads `GetMixFormat`. Exit 0
//! only when every endpoint's mix format now reports N channels, R Hz and
//! that mask. Restore with `set --channels 2`. Never touches defaults,
//! streams or any non-AudioRouter endpoint.
//!
//! Build: `cargo build --release -p audiorouter-windows-audio --example m03_cable_format`

const USAGE: &str = "usage: m03_cable_format show\n       m03_cable_format set --channels 1|2|4|6|8 [--rate 44100|48000|96000] [--encoding float32|pcm16|pcm24|pcm32] [--endpoint \"AudioRouter Cable A Input\"]...";

/// Device sample formats VCAB-11 offers: IEEE float32, PCM16, PCM24 in a
/// 32-bit container, PCM32. Windows may still mix in float32 (shared mode).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Encoding {
    Float32,
    Pcm16,
    Pcm24,
    Pcm32,
}

impl Encoding {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "float32" => Some(Self::Float32),
            "pcm16" => Some(Self::Pcm16),
            "pcm24" => Some(Self::Pcm24),
            "pcm32" => Some(Self::Pcm32),
            _ => None,
        }
    }

    /// (container bits, valid bits, IEEE float)
    fn layout(self) -> (u16, u16, bool) {
        match self {
            Self::Float32 => (32, 32, true),
            Self::Pcm16 => (16, 16, false),
            Self::Pcm24 => (32, 24, false),
            Self::Pcm32 => (32, 32, false),
        }
    }
}

/// Standard KSAUDIO_SPEAKER_* masks (same table as `m03_cable_inventory`).
fn channel_mask(channels: u16) -> Option<u32> {
    match channels {
        1 => Some(0x4),   // MONO (front center)
        2 => Some(0x3),   // STEREO
        4 => Some(0x33),  // QUAD
        6 => Some(0x3F),  // 5.1
        8 => Some(0x63F), // 7.1 surround
        _ => None,
    }
}

/// Same rule as the helper and `m03_cable_inventory`: the exact name or
/// "<description> (<cable name>)" or "<cable name> (<description>)".
fn name_matches(friendly: &str, expected: &str) -> bool {
    friendly == expected
        || friendly
            .strip_suffix(&format!(" ({expected})"))
            .is_some_and(|prefix| !prefix.is_empty() && !prefix.contains(['(', ')']))
        || friendly
            .strip_prefix(&format!("{expected} ("))
            .is_some_and(|rest| rest.ends_with(')') && !rest[..rest.len() - 1].contains(['(', ')']))
}

#[derive(Debug, PartialEq)]
enum Command {
    Show,
    Set {
        channels: u16,
        rate: u32,
        encoding: Encoding,
        endpoints: Vec<String>,
    },
}

fn parse(arguments: &[String]) -> Option<Command> {
    let (first, rest) = arguments.split_first()?;
    match first.as_str() {
        "show" if rest.is_empty() => Some(Command::Show),
        "set" => {
            let (mut channels, mut rate, mut endpoints) = (None, 48_000_u32, Vec::new());
            let mut encoding = Encoding::Float32;
            let mut iter = rest.iter();
            while let Some(flag) = iter.next() {
                let value = iter.next()?;
                match flag.as_str() {
                    "--channels" => {
                        channels = Some(value.parse().ok().filter(|c| channel_mask(*c).is_some())?)
                    }
                    "--rate" => {
                        rate = value
                            .parse()
                            .ok()
                            .filter(|r| [44_100, 48_000, 96_000].contains(r))?
                    }
                    "--encoding" => encoding = Encoding::parse(value)?,
                    "--endpoint" if value.starts_with("AudioRouter Cable ") => {
                        endpoints.push(value.clone())
                    }
                    _ => return None,
                }
            }
            if endpoints.is_empty() {
                endpoints = vec![
                    "AudioRouter Cable A Input".to_string(),
                    "AudioRouter Cable B Output".to_string(),
                ];
            }
            Some(Command::Set {
                channels: channels?,
                rate,
                encoding,
                endpoints,
            })
        }
        _ => None,
    }
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = parse(&arguments) else {
        eprintln!("{USAGE}");
        std::process::exit(64)
    };
    std::process::exit(platform::run(command));
}

#[cfg(windows)]
// The IPolicyConfig declaration keeps its COM method names.
#[allow(non_snake_case)]
mod platform {
    use super::{channel_mask, name_matches, Command, Encoding};
    use windows::core::{interface, IUnknown, IUnknown_Vtbl, GUID, HRESULT, PCWSTR};
    use windows::Win32::Media::Audio::{
        IAudioClient, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
        PKEY_AudioEndpoint_PhysicalSpeakers, AUDCLNT_SHAREMODE_EXCLUSIVE, WAVEFORMATEX,
        WAVEFORMATEXTENSIBLE, WAVEFORMATEXTENSIBLE_0,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
        COINIT_MULTITHREADED, STGM_READ,
    };

    const SUBTYPE_FLOAT: GUID = GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);
    const SUBTYPE_PCM: GUID = GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71);
    const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
    const VT_UI4: u16 = 19;
    /// `CPolicyConfigClient`, the coclass behind the Sound control panel.
    const CLSID_POLICY_CONFIG_CLIENT: GUID =
        GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

    // Undocumented, stable since Windows 7 and used by the Sound control
    // panel and many open-source tools. Only the vtable prefix up to
    // SetDeviceFormat is declared; the order must match exactly.
    #[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
    unsafe trait IPolicyConfig: IUnknown {
        fn GetMixFormat(&self, device: PCWSTR, format: *mut *mut WAVEFORMATEX) -> HRESULT;
        fn GetDeviceFormat(
            &self,
            device: PCWSTR,
            default: i32,
            format: *mut *mut WAVEFORMATEX,
        ) -> HRESULT;
        fn ResetDeviceFormat(&self, device: PCWSTR) -> HRESULT;
        fn SetDeviceFormat(
            &self,
            device: PCWSTR,
            endpoint_format: *const WAVEFORMATEX,
            mix_format: *const WAVEFORMATEX,
        ) -> HRESULT;
    }

    struct Format {
        channels: u16,
        rate: u32,
        bits: u16,
        tag: u16,
        mask: Option<u32>,
        /// "float" or "pcm<valid bits>" for WAVEFORMATEXTENSIBLE, else "".
        encoding: String,
    }

    impl std::fmt::Display for Format {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{} Hz, {} ch, {} bit, tag 0x{:04X}",
                self.rate, self.channels, self.bits, self.tag
            )?;
            if let Some(mask) = self.mask {
                write!(f, ", mask 0x{mask:X}")?;
            }
            if !self.encoding.is_empty() {
                write!(f, ", {}", self.encoding)?;
            }
            Ok(())
        }
    }

    /// Copies a CoTaskMem format out and frees it.
    ///
    /// # Safety
    /// `pointer` is null or a CoTaskMem allocation holding a WAVEFORMATEX
    /// followed by `cbSize` bytes; ownership passes to this function.
    unsafe fn take_format(pointer: *mut WAVEFORMATEX) -> Option<Format> {
        if pointer.is_null() {
            return None;
        }
        // SAFETY: per the contract; WAVEFORMATEX is packed, so read unaligned.
        let header: WAVEFORMATEX = unsafe { std::ptr::read_unaligned(pointer) };
        let extensible =
            (header.wFormatTag == WAVE_FORMAT_EXTENSIBLE && header.cbSize >= 22).then(|| {
                // SAFETY: cbSize >= 22 means the allocation is a full
                // WAVEFORMATEXTENSIBLE.
                unsafe { std::ptr::read_unaligned(pointer.cast::<WAVEFORMATEXTENSIBLE>()) }
            });
        let mask = extensible.map(|format| format.dwChannelMask);
        let encoding = extensible
            .map(|format| {
                let sub_format = format.SubFormat;
                // SAFETY: a union of u16 views; every value is a valid u16.
                let valid = unsafe { format.Samples.wValidBitsPerSample };
                if sub_format == SUBTYPE_FLOAT {
                    "float".to_owned()
                } else if sub_format == SUBTYPE_PCM {
                    format!("pcm{valid}")
                } else {
                    "other".to_owned()
                }
            })
            .unwrap_or_default();
        // SAFETY: the allocation came from CoTaskMemAlloc and is not used again.
        unsafe { CoTaskMemFree(Some(pointer as *const core::ffi::c_void)) };
        Some(Format {
            channels: header.nChannels,
            rate: header.nSamplesPerSec,
            bits: header.wBitsPerSample,
            tag: header.wFormatTag,
            mask,
            encoding,
        })
    }

    fn device_format(
        channels: u16,
        rate: u32,
        mask: u32,
        encoding: Encoding,
    ) -> WAVEFORMATEXTENSIBLE {
        let (bits, valid, float) = encoding.layout();
        let block_align = channels * (bits / 8);
        WAVEFORMATEXTENSIBLE {
            Format: WAVEFORMATEX {
                wFormatTag: WAVE_FORMAT_EXTENSIBLE,
                nChannels: channels,
                nSamplesPerSec: rate,
                nAvgBytesPerSec: rate * u32::from(block_align),
                nBlockAlign: block_align,
                wBitsPerSample: bits,
                cbSize: 22,
            },
            Samples: WAVEFORMATEXTENSIBLE_0 {
                wValidBitsPerSample: valid,
            },
            dwChannelMask: mask,
            SubFormat: if float { SUBTYPE_FLOAT } else { SUBTYPE_PCM },
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Mix format through a fresh IAudioClient (what applications see).
    ///
    /// # Safety
    /// COM is initialized on this thread.
    unsafe fn mix_format(device: &IMMDevice) -> Option<Format> {
        // SAFETY: COM is initialized; GetMixFormat returns a CoTaskMem block
        // whose ownership `take_format` takes.
        unsafe {
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None).ok()?;
            take_format(client.GetMixFormat().ok()?)
        }
    }

    /// # Safety
    /// COM is initialized on this thread.
    unsafe fn physical_speakers(device: &IMMDevice) -> String {
        // SAFETY: COM is initialized; the key is a live constant; a VT_UI4
        // PROPVARIANT owns no allocation, other types are only reported.
        unsafe {
            let Ok(store) = device.OpenPropertyStore(STGM_READ) else {
                return "unreadable".into();
            };
            match store.GetValue(&PKEY_AudioEndpoint_PhysicalSpeakers) {
                Ok(value) if value.Anonymous.Anonymous.vt.0 == VT_UI4 => {
                    format!("0x{:X}", value.Anonymous.Anonymous.Anonymous.ulVal)
                }
                Ok(value) => format!("not set (vt {})", value.Anonymous.Anonymous.vt.0),
                Err(error) => format!("unreadable (0x{:08X})", error.code().0 as u32),
            }
        }
    }

    pub fn run(command: Command) -> i32 {
        let ours: Vec<_> = audiorouter_windows_audio::enumerate_active_endpoint_display_info()
            .unwrap_or_default()
            .into_iter()
            .filter(|info| info.name.contains("AudioRouter"))
            .collect();
        if ours.is_empty() {
            eprintln!("FAIL: no active AudioRouter endpoint");
            return 1;
        }
        // SAFETY: COM is initialized for this thread before any interface is
        // created, every interface is dropped before CoUninitialize (the
        // inner function returns first), and each format pointer is either
        // a live local of the declared size or owned by `take_format`.
        unsafe {
            if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
                eprintln!("FAIL: COM initialization failed");
                return 1;
            }
            let code = run_initialized(&command, &ours);
            CoUninitialize();
            code
        }
    }

    /// # Safety
    /// COM is initialized on this thread.
    unsafe fn run_initialized(
        command: &Command,
        ours: &[audiorouter_windows_audio::EndpointDisplayInfo],
    ) -> i32 {
        // SAFETY: see `run`.
        unsafe {
            let Ok(enumerator) =
                CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
            else {
                eprintln!("FAIL: cannot create the device enumerator");
                return 1;
            };
            let policy: Option<IPolicyConfig> =
                CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL).ok();
            if policy.is_none() {
                eprintln!("note: IPolicyConfig is not available on this Windows build");
            }
            let describe = |info: &audiorouter_windows_audio::EndpointDisplayInfo| {
                let id = wide(&info.id);
                let Ok(device) = enumerator.GetDevice(PCWSTR(id.as_ptr())) else {
                    println!("{}: cannot open device", info.name);
                    return;
                };
                let mix = mix_format(&device).map_or("unreadable".into(), |f| f.to_string());
                let stored = policy.as_ref().and_then(|p| {
                    let mut pointer = std::ptr::null_mut();
                    p.GetDeviceFormat(PCWSTR(id.as_ptr()), 0, &mut pointer)
                        .is_ok()
                        .then(|| take_format(pointer))
                        .flatten()
                });
                println!("{} [{:?}]", info.name, info.direction);
                println!("  id: {}", info.id);
                println!("  mix format: {mix}");
                println!(
                    "  device format: {}",
                    stored.map_or("unreadable".into(), |f| f.to_string())
                );
                println!("  physical speakers: {}", physical_speakers(&device));
            };
            match command {
                Command::Show => {
                    ours.iter().for_each(describe);
                    0
                }
                Command::Set {
                    channels,
                    rate,
                    encoding,
                    endpoints,
                } => {
                    let Some(policy) = policy.as_ref() else {
                        eprintln!("FAIL: IPolicyConfig unavailable; nothing changed");
                        return 1;
                    };
                    let mask = channel_mask(*channels).expect("validated by parse");
                    let format = device_format(*channels, *rate, mask, *encoding);
                    let mut failures = 0;
                    for expected in endpoints {
                        let found: Vec<_> = ours
                            .iter()
                            .filter(|info| name_matches(&info.name, expected))
                            .collect();
                        let [info] = found.as_slice() else {
                            eprintln!("FAIL: {expected}: {} matching endpoints", found.len());
                            failures += 1;
                            continue;
                        };
                        let id = wide(&info.id);
                        let Ok(device) = enumerator.GetDevice(PCWSTR(id.as_ptr())) else {
                            eprintln!("FAIL: {expected}: cannot open device");
                            failures += 1;
                            continue;
                        };
                        if let Ok(client) = device.Activate::<IAudioClient>(CLSCTX_ALL, None) {
                            let supported = client.IsFormatSupported(
                                AUDCLNT_SHAREMODE_EXCLUSIVE,
                                &format.Format,
                                None,
                            );
                            println!(
                                "{expected}: driver accepts {rate} Hz/{channels} ch {encoding:?} (exclusive check): 0x{:08X}",
                                supported.0 as u32
                            );
                        }
                        let result = policy.SetDeviceFormat(
                            PCWSTR(id.as_ptr()),
                            &format.Format,
                            &format.Format,
                        );
                        println!("{expected}: SetDeviceFormat 0x{:08X}", result.0 as u32);
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        match mix_format(&device) {
                            Some(mix)
                                if mix.channels == *channels
                                    && mix.rate == *rate
                                    && mix.mask == Some(mask) =>
                            {
                                println!("PASS: {expected}: mix format now {mix}");
                            }
                            Some(mix) => {
                                eprintln!("FAIL: {expected}: mix format is {mix}");
                                failures += 1;
                            }
                            None => {
                                eprintln!("FAIL: {expected}: mix format unreadable");
                                failures += 1;
                            }
                        }
                        describe(info);
                    }
                    i32::from(failures > 0)
                }
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn run(_command: super::Command) -> i32 {
        eprintln!("m03_cable_format needs Windows");
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn set_defaults_to_the_bit_exact_runner_endpoints_at_48k() {
        assert_eq!(
            parse(&args("set --channels 8")),
            Some(Command::Set {
                channels: 8,
                rate: 48_000,
                encoding: Encoding::Float32,
                endpoints: vec![
                    "AudioRouter Cable A Input".into(),
                    "AudioRouter Cable B Output".into()
                ],
            })
        );
        assert_eq!(parse(&args("show")), Some(Command::Show));
    }

    #[test]
    fn refuses_unknown_layouts_rates_and_foreign_endpoints() {
        for bad in [
            "set",
            "set --channels 3",
            "set --channels 8 --rate 22050",
            "set --channels 8 --endpoint Speakers",
            "set --channels 2 --encoding pcm8",
            "set --channels",
            "show extra",
            "remove",
        ] {
            assert_eq!(parse(&args(bad)), None, "{bad}");
        }
        let mut quoted = args("set --channels 2 --endpoint");
        quoted.push("AudioRouter Cable C Input".into());
        assert!(
            matches!(parse(&quoted), Some(Command::Set { endpoints, .. }) if endpoints == ["AudioRouter Cable C Input"])
        );
    }

    #[test]
    fn encodings_parse_and_lay_out_as_vcab_11_requires() {
        assert!(matches!(
            parse(&args("set --channels 2 --rate 96000 --encoding pcm24")),
            Some(Command::Set {
                encoding: Encoding::Pcm24,
                rate: 96_000,
                ..
            })
        ));
        assert_eq!(Encoding::Float32.layout(), (32, 32, true));
        assert_eq!(Encoding::Pcm16.layout(), (16, 16, false));
        assert_eq!(Encoding::Pcm24.layout(), (32, 24, false));
        assert_eq!(Encoding::Pcm32.layout(), (32, 32, false));
    }

    #[test]
    fn masks_have_one_bit_per_channel() {
        for channels in [1, 2, 4, 6, 8] {
            assert_eq!(
                channel_mask(channels).unwrap().count_ones(),
                u32::from(channels)
            );
        }
        assert!(name_matches(
            "AudioRouter Cable A Input (AudioRouter Virtual Cable)",
            "AudioRouter Cable A Input"
        ));
    }
}
