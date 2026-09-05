//! Read-only M00 WASAPI endpoint inventory.
//!
//! This probe intentionally does not open streams, alter defaults, install drivers,
//! or write outside stdout. It establishes that the selected Windows Rust bindings
//! can enumerate endpoint identities and states. Stream/format/period and
//! process-loopback probes remain separate follow-up work.

use windows::core::Result;
use windows::Win32::Media::Audio::{eAll, DEVICE_STATE_ACTIVE, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
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
        println!("endpoint index={index} state=0x{:08x} id={}", state.0, id.to_string()?);
    }

    Ok(())
}
