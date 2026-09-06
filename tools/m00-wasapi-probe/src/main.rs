//! Read-only M00 WASAPI endpoint inventory.
//!
//! This probe intentionally does not capture audio, alter defaults, install drivers,
//! or write outside stdout. It may initialize a shared-mode client briefly to test
//! capability and buffer negotiation, then resets and releases it. Stream data and
//! process-loopback probes remain separate follow-up work.

use std::sync::{Arc, Condvar, Mutex};
use windows::core::Result;
use windows::core::{implement, Interface};
use windows::Win32::Media::Audio::{
    eAll, ActivateAudioInterfaceAsync, IActivateAudioInterfaceAsyncOperation,
    IActivateAudioInterfaceCompletionHandler, IAudioClient, IMMDeviceEnumerator,
    MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
    AUDCLNT_STREAMFLAGS_LOOPBACK, AUDCLNT_STREAMFLAGS_NOPERSIST, AUDIOCLIENT_ACTIVATION_PARAMS,
    AUDIOCLIENT_ACTIVATION_PARAMS_0, AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
    AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS, DEVICE_STATE_ACTIVE,
    PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE, VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
    WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::StructuredStorage::{
    PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemAlloc, CoTaskMemFree, CoUninitialize, BLOB,
    CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::System::Variant::VT_BLOB;

fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let result = enumerate();
        CoUninitialize();
        result
    }
}

#[implement(IActivateAudioInterfaceCompletionHandler)]
struct ProcessLoopbackHandler {
    completion: Arc<(Mutex<Option<i32>>, Condvar)>,
}

impl windows::Win32::Media::Audio::IActivateAudioInterfaceCompletionHandler_Impl
    for ProcessLoopbackHandler_Impl
{
    fn ActivateCompleted(
        &self,
        operation: windows::core::Ref<IActivateAudioInterfaceAsyncOperation>,
    ) -> windows::core::Result<()> {
        let mut activation_result = windows::core::HRESULT(0);
        let mut activated_interface = None;
        let final_hresult = unsafe {
            match operation.ok().and_then(|operation| {
                operation.GetActivateResult(&mut activation_result, &mut activated_interface)
            }) {
                Ok(()) => activation_result.0,
                Err(error) => error.code().0,
            }
        };
        let (lock, wake) = &*self.completion;
        *lock.lock().unwrap() = Some(final_hresult);
        wake.notify_one();
        Ok(())
    }
}

#[allow(dead_code)]
unsafe fn process_loopback_initialize() -> i32 {
    let process_id = GetCurrentProcessId();
    let process_params = AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
        TargetProcessId: process_id,
        ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
    };
    let activation_params = AUDIOCLIENT_ACTIVATION_PARAMS {
        ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
        Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
            ProcessLoopbackParams: process_params,
        },
    };
    let blob_data = CoTaskMemAlloc(std::mem::size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>());
    if blob_data.is_null() {
        return -4;
    }
    std::ptr::copy_nonoverlapping(
        std::ptr::addr_of!(activation_params) as *const u8,
        blob_data as *mut u8,
        std::mem::size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>(),
    );
    let property = PROPVARIANT {
        Anonymous: PROPVARIANT_0 {
            Anonymous: std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
                vt: VT_BLOB,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: PROPVARIANT_0_0_0 {
                    blob: BLOB {
                        cbSize: std::mem::size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32,
                        pBlobData: blob_data as *mut u8,
                    },
                },
            }),
        },
    };
    let completion = Arc::new((Mutex::new(None), Condvar::new()));
    let handler: IActivateAudioInterfaceCompletionHandler = ProcessLoopbackHandler {
        completion: Arc::clone(&completion),
    }
    .into();
    let operation = match ActivateAudioInterfaceAsync(
        VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
        &IAudioClient::IID,
        Some(std::ptr::addr_of!(property)),
        &handler,
    ) {
        Ok(operation) => operation,
        Err(error) => return error.code().0,
    };
    let (lock, wake) = &*completion;
    let mut result = lock.lock().unwrap();
    if result.is_none() {
        let (updated, timeout) = wake
            .wait_timeout(result, std::time::Duration::from_secs(5))
            .unwrap();
        result = updated;
        if timeout.timed_out() && result.is_none() {
            // The API owns the callback lifetime until it completes. Do not drop the
            // operation/handler on timeout while Windows may still call back.
            std::mem::forget(operation);
            std::mem::forget(handler);
            CoTaskMemFree(Some(blob_data));
            return -2;
        }
    }
    CoTaskMemFree(Some(blob_data));
    result.unwrap_or(-3)
}

unsafe fn enumerate() -> Result<()> {
    let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
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
        let sample_rate = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.nSamplesPerSec));
        let bits = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.wBitsPerSample));
        let block_align = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.nBlockAlign));
        let avg_bytes = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.nAvgBytesPerSec));
        let extra_size = std::ptr::read_unaligned(std::ptr::addr_of!(format_value.cbSize));
        let support_44100_mono = format_support(&client, 44_100, 1).0;
        let support_44100_stereo = format_support(&client, 44_100, 2).0;
        let support_48000_mono = format_support(&client, 48_000, 1).0;
        let support_48000_stereo = format_support(&client, 48_000, 2).0;
        CoTaskMemFree(Some(format.cast()));
        // Use a fresh client for Initialize. Capability queries are intentionally
        // isolated from stream lifecycle state on the client used for negotiation.
        let stream_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        let stream_format = stream_client.GetMixFormat()?;
        let stream_extensible = if id_string.starts_with("{0.0.1.") {
            Some(std::ptr::read_unaligned(
                stream_format.cast::<WAVEFORMATEXTENSIBLE>(),
            ))
        } else {
            None
        };
        let initialize_format = stream_extensible.as_ref().map_or(stream_format, |format| {
            std::ptr::addr_of!(format.Format) as *mut WAVEFORMATEX
        });
        let mut closest_capture_format = std::ptr::null_mut();
        let negotiated_format = if id_string.starts_with("{0.0.1.") {
            let requested = WAVEFORMATEX {
                wFormatTag: 3,
                nChannels: channels,
                nSamplesPerSec: sample_rate,
                nAvgBytesPerSec: sample_rate * channels as u32 * 4,
                nBlockAlign: channels * 4,
                wBitsPerSample: 32,
                cbSize: 0,
            };
            let support = stream_client.IsFormatSupported(
                AUDCLNT_SHAREMODE_SHARED,
                &requested,
                Some(&mut closest_capture_format),
            );
            if support.0 == 1 && !closest_capture_format.is_null() {
                closest_capture_format
            } else {
                initialize_format
            }
        } else {
            initialize_format
        };
        let stream_flags = if id_string.starts_with("{0.0.0.") {
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_NOPERSIST
        } else {
            0
        };
        let buffer_duration = if id_string.starts_with("{0.0.0.") {
            0
        } else {
            10_000_000
        };
        let stream_result = stream_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            stream_flags,
            buffer_duration,
            0,
            negotiated_format,
            None,
        );
        let (initialize_hresult, start_hresult, buffer_frames, stream_latency_100ns) =
            match stream_result {
                Ok(()) => {
                    let buffer = stream_client.GetBufferSize()?;
                    let latency = stream_client.GetStreamLatency()?;
                    let start_hresult = if id_string.starts_with("{0.0.0.") {
                        let result = stream_client.Start();
                        if result.is_ok() {
                            stream_client.Stop()?;
                        }
                        result.map(|()| 0).unwrap_or_else(|error| error.code().0)
                    } else {
                        0
                    };
                    stream_client.Reset()?;
                    (0, start_hresult, Some(buffer), Some(latency))
                }
                Err(error) => (error.code().0, -1, None, None),
            };
        let loopback_hresult = if id_string.starts_with("{0.0.0.") {
            let loopback_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let loopback_format = loopback_client.GetMixFormat()?;
            let result = loopback_client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK
                    | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                    | AUDCLNT_STREAMFLAGS_NOPERSIST,
                0,
                0,
                loopback_format,
                None,
            );
            let hresult = match result {
                Ok(()) => {
                    loopback_client.Reset()?;
                    0
                }
                Err(error) => error.code().0,
            };
            CoTaskMemFree(Some(loopback_format.cast()));
            hresult
        } else {
            -1
        };
        let capture_original_hresult = if id_string.starts_with("{0.0.1.") {
            capture_initialize_variant(&device, 0)
        } else {
            -1
        };
        let capture_extensible_hresult = if id_string.starts_with("{0.0.1.") {
            capture_initialize_variant(&device, 1)
        } else {
            -1
        };
        let capture_float_hresult = if id_string.starts_with("{0.0.1.") {
            capture_initialize_variant(&device, 2)
        } else {
            -1
        };
        CoTaskMemFree(Some(stream_format.cast()));
        if !closest_capture_format.is_null() {
            CoTaskMemFree(Some(closest_capture_format.cast()));
        }
        println!(
            "endpoint index={index} state=0x{:08x} id={} format_tag={} channels={} rate_hz={} bits={} block_align={} avg_bytes={} cb_size={} default_period_100ns={} minimum_period_100ns={} is_supported_44100_mono=0x{support_44100_mono:08x} is_supported_44100_stereo=0x{support_44100_stereo:08x} is_supported_48000_mono=0x{support_48000_mono:08x} is_supported_48000_stereo=0x{support_48000_stereo:08x} initialize_hresult=0x{initialize_hresult:08x} start_hresult=0x{start_hresult:08x} loopback_hresult=0x{loopback_hresult:08x} capture_original_hresult=0x{capture_original_hresult:08x} capture_extensible_hresult=0x{capture_extensible_hresult:08x} capture_float_hresult=0x{capture_float_hresult:08x} buffer_frames={} stream_latency_100ns={}",
            state.0,
            id_string,
            format_tag,
            channels,
            sample_rate,
            bits,
            block_align,
            avg_bytes,
            extra_size,
            default_period,
            minimum_period,
            buffer_frames.map_or_else(|| "-".to_string(), |value| value.to_string()),
            stream_latency_100ns.map_or_else(|| "-".to_string(), |value| value.to_string()),
        );
    }

    Ok(())
}

unsafe fn capture_initialize_variant(
    device: &windows::Win32::Media::Audio::IMMDevice,
    mode: u8,
) -> i32 {
    let client: IAudioClient = match device.Activate(CLSCTX_ALL, None) {
        Ok(client) => client,
        Err(error) => return error.code().0,
    };
    let format = match client.GetMixFormat() {
        Ok(format) => format,
        Err(error) => return error.code().0,
    };
    let copied_extensible = std::ptr::read_unaligned(format.cast::<WAVEFORMATEXTENSIBLE>());
    let float_format = WAVEFORMATEX {
        wFormatTag: 3,
        nChannels: copied_extensible.Format.nChannels,
        nSamplesPerSec: copied_extensible.Format.nSamplesPerSec,
        nAvgBytesPerSec: copied_extensible.Format.nSamplesPerSec
            * copied_extensible.Format.nChannels as u32
            * 4,
        nBlockAlign: copied_extensible.Format.nChannels * 4,
        wBitsPerSample: 32,
        cbSize: 0,
    };
    let initialize_format = match mode {
        0 => format,
        1 => std::ptr::addr_of!(copied_extensible.Format) as *mut WAVEFORMATEX,
        _ => std::ptr::addr_of!(float_format) as *const WAVEFORMATEX as *mut WAVEFORMATEX,
    };
    let result = client.Initialize(AUDCLNT_SHAREMODE_SHARED, 0, 0, 0, initialize_format, None);
    let hresult = match result {
        Ok(()) => {
            let _ = client.Reset();
            0
        }
        Err(error) => error.code().0,
    };
    CoTaskMemFree(Some(format.cast()));
    hresult
}

unsafe fn format_support(
    client: &IAudioClient,
    sample_rate: u32,
    channels: u16,
) -> windows::core::HRESULT {
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
    let result = client.IsFormatSupported(AUDCLNT_SHAREMODE_SHARED, &format, Some(&mut closest));
    if !closest.is_null() {
        CoTaskMemFree(Some(closest.cast()));
    }
    result
}
