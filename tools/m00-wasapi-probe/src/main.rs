//! M00 WASAPI endpoint inventory and opt-in adapter smoke probe.
//!
//! The default inventory path does not capture audio, alter defaults, install
//! drivers, or write outside stdout. The explicit `adapter-smoke` mode opens
//! bounded production Rust capture/render clients, reads caller-owned capture
//! data, submits zero-valued caller-owned render buffers, then stops and resets
//! both streams.

use audiorouter_engine::{RealtimeScheduler, RuntimeGeneration, RuntimeGraph};
use audiorouter_windows_audio::{
    enumerate_active_endpoints, AudioError, EndpointDirection, SharedCapture, SharedRender,
};
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
    if std::env::args().nth(1).as_deref() == Some("adapter-smoke") {
        let duration_ms = std::env::args()
            .nth(2)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(500);
        if let Err(error) = adapter_smoke(duration_ms) {
            eprintln!("adapter_smoke_error={error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let result = enumerate();
        CoUninitialize();
        result
    }
}

fn adapter_smoke(duration_ms: u64) -> std::result::Result<(), AudioError> {
    let endpoints = enumerate_active_endpoints()?;
    let capture_info = endpoints
        .iter()
        .find(|endpoint| endpoint.direction == EndpointDirection::Capture)
        .ok_or_else(|| {
            AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80070490u32 as i32),
                "no active capture endpoint",
            ))
        })?;
    let render_info = endpoints
        .iter()
        .find(|endpoint| endpoint.direction == EndpointDirection::Render)
        .ok_or_else(|| {
            AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80070490u32 as i32),
                "no active render endpoint",
            ))
        })?;
    let capture_bytes_per_frame = usize::from(capture_info.channels)
        .checked_mul(usize::from(capture_info.bits_per_sample / 8))
        .ok_or(AudioError::InvalidFrameSize)?;
    let render_bytes_per_frame = usize::from(render_info.channels)
        .checked_mul(usize::from(render_info.bits_per_sample / 8))
        .ok_or(AudioError::InvalidFrameSize)?;
    if capture_bytes_per_frame == 0 || render_bytes_per_frame == 0 {
        return Err(AudioError::InvalidFrameSize);
    }
    if capture_info.bits_per_sample != 32 || capture_info.channels > 2 {
        return Err(AudioError::InvalidFrameSize);
    }
    let mut capture = SharedCapture::open(&capture_info.id, 1_000_000)?;
    let mut render = SharedRender::open(&render_info.id, 1_000_000)?;
    let scheduler = RealtimeScheduler::new(8, usize::from(capture_info.channels), 128)
        .map_err(|_| AudioError::InvalidFrameSize)?;
    scheduler
        .processor()
        .publish(RuntimeGraph::prepare(RuntimeGeneration::new(1), vec![]));
    capture.start()?;
    render.start()?;
    let result = (|| {
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_millis(duration_ms))
            .unwrap_or_else(std::time::Instant::now);
        let mut capture_packets = 0u32;
        let mut capture_frames = 0u32;
        let mut capture_bytes = 0usize;
        let mut scheduler_frames = 0u32;
        let mut render_frames = 0u32;
        let mut destination = vec![0u8; 1_048_576];
        let render_source = vec![0u8; 1_048_576];
        let mut staging = vec![0u8; 128 * capture_bytes_per_frame];
        let mut pending_frames = 0usize;
        while std::time::Instant::now() < deadline {
            if capture.wait_for_data(10)? {
                while let Some((packet, bytes)) =
                    capture.next_packet_into(&mut destination, capture_bytes_per_frame)?
                {
                    capture_packets = capture_packets.saturating_add(1);
                    capture_frames = capture_frames.saturating_add(packet.frames);
                    capture_bytes = capture_bytes.saturating_add(bytes);
                    let mut packet_frame_offset = 0usize;
                    while packet_frame_offset < packet.frames as usize {
                        let frames_to_copy = (128 - pending_frames)
                            .min(packet.frames as usize - packet_frame_offset);
                        let source_start = packet_frame_offset * capture_bytes_per_frame;
                        let source_end = source_start + frames_to_copy * capture_bytes_per_frame;
                        let staging_start = pending_frames * capture_bytes_per_frame;
                        let staging_end = staging_start + frames_to_copy * capture_bytes_per_frame;
                        staging[staging_start..staging_end]
                            .copy_from_slice(&destination[source_start..source_end]);
                        pending_frames += frames_to_copy;
                        packet_frame_offset += frames_to_copy;
                        if pending_frames != 128 {
                            continue;
                        }
                        let mut block =
                            scheduler
                                .acquire_input()
                                .ok_or(AudioError::BufferTooSmall {
                                    required: 128,
                                    available: 0,
                                })?;
                        for frame in 0..128 {
                            for channel in 0..usize::from(capture_info.channels) {
                                let offset = frame * capture_bytes_per_frame + channel * 4;
                                let sample = f32::from_le_bytes(
                                    staging[offset..offset + 4]
                                        .try_into()
                                        .map_err(|_| AudioError::InvalidFrameSize)?,
                                );
                                block.channel_mut(channel).unwrap()[frame] = sample;
                            }
                        }
                        if let Err(block) = scheduler.submit_input(block) {
                            scheduler
                                .input()
                                .try_recycle(block)
                                .map_err(|_| AudioError::InvalidFrameSize)?;
                            continue;
                        }
                        scheduler
                            .process_once()
                            .map_err(|_| AudioError::InvalidFrameSize)?;
                        if let Some(output) = scheduler.receive_output() {
                            scheduler
                                .output()
                                .try_recycle(output)
                                .map_err(|_| AudioError::InvalidFrameSize)?;
                            scheduler_frames = scheduler_frames.saturating_add(128);
                        }
                        pending_frames = 0;
                    }
                }
            }
            render_frames = render_frames
                .saturating_add(render.submit_bytes(&render_source, render_bytes_per_frame)?);
        }
        if capture_packets == 0 || scheduler_frames == 0 || render_frames == 0 {
            return Err(AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80004005u32 as i32),
                "adapter smoke test received no bounded stream data",
            )));
        }
        println!(
            "adapter_smoke capture_endpoint={} render_endpoint={} capture_packets={} capture_frames={} capture_bytes={} scheduler_frames={} pending_frames={} render_frames={}",
            capture_info.id,
            render_info.id,
            capture_packets,
            capture_frames,
            capture_bytes,
            scheduler_frames,
            pending_frames,
            render_frames
        );
        Ok(())
    })();
    let capture_stop = capture.stop();
    let render_stop = render.stop();
    result.and(capture_stop).and(render_stop)
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
        let channel_mask = stream_extensible
            .as_ref()
            .map(|format| format.dwChannelMask)
            .unwrap_or(0);
        let subformat = stream_extensible
            .as_ref()
            .map(|format| {
                let guid = std::ptr::read_unaligned(std::ptr::addr_of!(format.SubFormat));
                format!("{guid:?}")
            })
            .unwrap_or_else(|| "-".into());
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
        // Use the engine-selected shared-mode period so this probe does not
        // impose an additional buffer-size choice. A zero duration is the
        // required value for shared event-driven streams and the minimum-
        // latency choice for other shared streams; this does not by itself
        // establish the cause of any E_INVALIDARG result.
        let buffer_duration = 0;
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
            "endpoint index={index} state=0x{:08x} id={} format_tag={} channels={} rate_hz={} bits={} block_align={} avg_bytes={} cb_size={} channel_mask=0x{channel_mask:08x} subformat={subformat} default_period_100ns={} minimum_period_100ns={} is_supported_44100_mono=0x{support_44100_mono:08x} is_supported_44100_stereo=0x{support_44100_stereo:08x} is_supported_48000_mono=0x{support_48000_mono:08x} is_supported_48000_stereo=0x{support_48000_stereo:08x} initialize_hresult=0x{initialize_hresult:08x} start_hresult=0x{start_hresult:08x} loopback_hresult=0x{loopback_hresult:08x} capture_original_hresult=0x{capture_original_hresult:08x} capture_extensible_hresult=0x{capture_extensible_hresult:08x} capture_float_hresult=0x{capture_float_hresult:08x} buffer_frames={} stream_latency_100ns={}",
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
