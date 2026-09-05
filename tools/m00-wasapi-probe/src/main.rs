//! Read-only M00 WASAPI endpoint inventory.
//!
//! This probe intentionally does not open streams, alter defaults, install drivers,
//! or write outside stdout. It establishes that the selected Windows Rust bindings
//! can enumerate endpoint identities and states. Stream/format/period and
//! process-loopback probes remain separate follow-up work.

use windows::core::Result;
use windows::Win32::Media::Audio::{
    eAll, DEVICE_STATE_ACTIVE, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
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
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        let mut default_period = 0i64;
        let mut minimum_period = 0i64;
        client.GetDevicePeriod(Some(&mut default_period), Some(&mut minimum_period))?;
        let format = client.GetMixFormat()?;
        let format_value = *format;
        CoTaskMemFree(Some(format.cast()));
        let format_tag = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.wFormatTag));
        let channels = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.nChannels));
        let sample_rate =
            std::ptr::read_unaligned(std::ptr::addr_of!(format_value.nSamplesPerSec));
        let bits = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.wBitsPerSample));
        println!(
            "endpoint index={index} state=0x{:08x} id={} format_tag={} channels={} rate_hz={} bits={} default_period_100ns={} minimum_period_100ns={}",
            state.0,
            id.to_string()?,
            format_tag,
            channels,
            sample_rate,
            bits,
            default_period,
            minimum_period,
        );
    }

    Ok(())
}
