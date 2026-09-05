//! Read-only M00 WASAPI endpoint inventory.
//!
//! This probe intentionally does not capture audio, alter defaults, install drivers,
//! or write outside stdout. It may initialize a shared-mode client briefly to test
//! capability and buffer negotiation, then resets and releases it. Stream data and
//! process-loopback probes remain separate follow-up work.

use windows::core::Result;
use windows::Win32::Media::Audio::{
    eAll, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
    AUDCLNT_STREAMFLAGS_NOPERSIST, DEVICE_STATE_ACTIVE, IAudioClient, IMMDeviceEnumerator,
    MMDeviceEnumerator, WAVEFORMATEX,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_MULTITHREADED,
};

fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let result = enumerate();
        CoUninitialize();
        result
    }
}

unsafe fn enumerate() -> Result<()> {
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let devices = enumerator.EnumAudioEndpoints(eAll, DEVICE_STATE_ACTIVE)?;
    let count = devices.GetCount()?;
    println!("active_endpoint_count={count}");

    for index in 0..count {
        let device = devices.Item(index)?;
        let id = device.GetId()?;
        let state = device.GetState()?;
        let id_string = id.to_string()?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        let mut default_period = 0i64;
        let mut minimum_period = 0i64;
        client.GetDevicePeriod(Some(&mut default_period), Some(&mut minimum_period))?;
        let format = client.GetMixFormat()?;
        let format_value = *format;
        let format_tag = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.wFormatTag));
        let channels = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.nChannels));
        let sample_rate =
            std::ptr::read_unaligned(std::ptr::addr_of!(format_value.nSamplesPerSec));
        let bits = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.wBitsPerSample));
        let support_44100_mono = format_support(&client, 44_100, 1).0;
        let support_44100_stereo = format_support(&client, 44_100, 2).0;
        let support_48000_mono = format_support(&client, 48_000, 1).0;
        let support_48000_stereo = format_support(&client, 48_000, 2).0;
        let stream_result = client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_NOPERSIST,
            0,
            0,
            format,
            None,
        );
        let (initialize_hresult, start_hresult, buffer_frames, stream_latency_100ns) = match stream_result {
            Ok(()) => {
                let buffer = client.GetBufferSize()?;
                let latency = client.GetStreamLatency()?;
                let start_hresult = if id_string.starts_with("{0.0.0.") {
                    let result = client.Start();
                    if result.is_ok() {
                        client.Stop()?;
                    }
                    result.map(|()| 0).unwrap_or_else(|error| error.code().0)
                } else {
                    0
                };
                client.Reset()?;
                (0, start_hresult, Some(buffer), Some(latency))
            }
            Err(error) => (error.code().0, -1, None, None),
        };
        CoTaskMemFree(Some(format.cast()));
        println!(
            "endpoint index={index} state=0x{:08x} id={} format_tag={} channels={} rate_hz={} bits={} default_period_100ns={} minimum_period_100ns={} is_supported_44100_mono=0x{support_44100_mono:08x} is_supported_44100_stereo=0x{support_44100_stereo:08x} is_supported_48000_mono=0x{support_48000_mono:08x} is_supported_48000_stereo=0x{support_48000_stereo:08x} initialize_hresult=0x{initialize_hresult:08x} start_hresult=0x{start_hresult:08x} buffer_frames={} stream_latency_100ns={}",
            state.0,
            id_string,
            format_tag,
            channels,
            sample_rate,
            bits,
            default_period,
            minimum_period,
            buffer_frames.map_or_else(|| "-".to_string(), |value| value.to_string()),
            stream_latency_100ns.map_or_else(|| "-".to_string(), |value| value.to_string()),
        );
    }

    Ok(())
}

unsafe fn format_support(client: &IAudioClient, sample_rate: u32, channels: u16) -> windows::core::HRESULT {
    let format = WAVEFORMATEX {
        wFormatTag: 3,
        nChannels: channels,
        nSamplesPerSec: sample_rate,
        nAvgBytesPerSec: sample_rate * channels as u32 * 4,
        nBlockAlign: channels * 4,
        wBitsPerSample: 32,
        cbSize: 0,
    };
    let mut closest: *mut WAVEFORMATEX = std::ptr::null_mut();
    let result = client.IsFormatSupported(
        AUDCLNT_SHAREMODE_SHARED,
        &format,
        Some(&mut closest),
    );
    if !closest.is_null() {
        CoTaskMemFree(Some(closest.cast()));
    }
    result
}
