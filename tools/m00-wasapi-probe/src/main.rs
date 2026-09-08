//! M00 WASAPI endpoint inventory and opt-in adapter smoke probe.
//!
//! The default inventory path does not capture audio, alter defaults, install
//! drivers, or write outside stdout. The explicit `adapter-smoke` mode opens
//! bounded production Rust capture/render clients, reads caller-owned capture
//! data, submits zero-valued caller-owned render buffers, then stops and resets
//! both streams. The separately named `adapter-route` mode is an opt-in,
//! endpoint-ID-selected digital route smoke for compatible 32-bit endpoints.

use audiorouter_engine::{
    AudioBlock, DriftController, Pcm16QuantumAdapter, ProcessingStage, RealtimeScheduler,
    RuntimeGeneration, RuntimeGraph, StreamingResampler,
};
use audiorouter_windows_audio::{
    enumerate_active_endpoints, AudioError, EndpointDirection, EndpointMonitor,
    ProcessLoopbackCapture, ProcessLoopbackMode, SharedCapture, SharedRender,
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
        if let Err(error) = adapter_smoke(duration_ms, None, None, false) {
            eprintln!("adapter_smoke_error={error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("adapter-route") {
        let duration_ms = std::env::args()
            .nth(2)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(500);
        let capture_id = std::env::args().nth(3);
        let render_id = std::env::args().nth(4);
        if let Err(error) = adapter_smoke(
            duration_ms,
            capture_id.as_deref(),
            render_id.as_deref(),
            true,
        ) {
            eprintln!("adapter_route_error={error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("process-loopback") {
        let duration_ms = std::env::args()
            .nth(2)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(500);
        let mode = if std::env::args().nth(3).as_deref() == Some("exclude") {
            ProcessLoopbackMode::ExcludeTargetTree
        } else {
            ProcessLoopbackMode::IncludeTargetTree
        };
        if let Err(error) = process_loopback_smoke(duration_ms, mode) {
            eprintln!("process_loopback_error={error}");
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

fn process_loopback_smoke(
    duration_ms: u64,
    mode: ProcessLoopbackMode,
) -> std::result::Result<u32, AudioError> {
    if !(100..=2_000).contains(&duration_ms) {
        return Err(AudioError::InvalidFrameSize);
    }
    let mut capture = ProcessLoopbackCapture::open(
        unsafe { windows::Win32::System::Threading::GetCurrentProcessId() },
        mode,
    )?;
    capture.start()?;
    let mut buffer = vec![0u8; 65_536];
    let mut pcm16 = vec![0i16; buffer.len() / 2];
    let mut quantum = Pcm16QuantumAdapter::new(2).map_err(|_| AudioError::InvalidFrameSize)?;
    let mut block = AudioBlock::new(2, 128).map_err(|_| AudioError::InvalidFrameSize)?;
    let scheduler = RealtimeScheduler::new(8, 2, 128)
        .map_err(|_| AudioError::InvalidFrameSize)?;
    let generation = RuntimeGeneration::new(1);
    let _ = scheduler.publish(RuntimeGraph::prepare(
        generation,
        vec![ProcessingStage::Gain { linear: 0.5 }],
    ));
    let started = std::time::Instant::now();
    let mut packets = 0u32;
    let mut frames = 0u32;
    let mut quantum_blocks = 0u32;
    while started.elapsed() < std::time::Duration::from_millis(duration_ms) {
        while let Some(packet) = capture.read_packet(&mut buffer)? {
            packets = packets.saturating_add(1);
            frames = frames.saturating_add(packet.frames);
            let sample_count = packet.frames as usize * capture.bytes_per_frame() / 2;
            if sample_count > pcm16.len() || capture.bytes_per_frame() != 4 {
                return Err(AudioError::InvalidFrameSize);
            }
            for (sample, bytes) in pcm16[..sample_count].iter_mut().zip(buffer[..sample_count * 2].chunks_exact(2)) {
                *sample = i16::from_ne_bytes([bytes[0], bytes[1]]);
            }
            let mut offset = 0usize;
            while offset < packet.frames as usize {
                let consumed = quantum
                    .push_interleaved(&pcm16[offset * 2..sample_count])
                    .map_err(|_| AudioError::InvalidFrameSize)?;
                offset += consumed;
                if consumed == 0 {
                    if !quantum.pop_into(&mut block).map_err(|_| AudioError::InvalidFrameSize)? {
                        return Err(AudioError::InvalidFrameSize);
                    }
                    quantum_blocks = quantum_blocks.saturating_add(1);
                    let mut input = scheduler
                        .acquire_input()
                        .ok_or(AudioError::BufferTooSmall {
                            required: 128,
                            available: 0,
                        })?;
                    input
                        .copy_from(&block)
                        .map_err(|_| AudioError::InvalidFrameSize)?;
                    scheduler
                        .submit_input(input)
                        .map_err(|_| AudioError::InvalidFrameSize)?;
                    let processed = scheduler
                        .process_once()
                        .map_err(|_| AudioError::InvalidFrameSize)?;
                    if processed != Some(generation) {
                        return Err(AudioError::InvalidFrameSize);
                    }
                    let output = scheduler
                        .receive_output_for_generation(generation)
                        .ok_or(AudioError::InvalidFrameSize)?;
                    scheduler
                        .output()
                        .try_recycle(output)
                        .map_err(|_| AudioError::InvalidFrameSize)?;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    capture.stop()?;
    println!(
        "process_loopback mode={} bytes_per_frame={} packets={} frames={} quantum_blocks={} scheduler_generation={}",
        match mode {
            ProcessLoopbackMode::IncludeTargetTree => "include",
            ProcessLoopbackMode::ExcludeTargetTree => "exclude",
        },
        capture.bytes_per_frame(),
        packets,
        frames,
        quantum_blocks,
        generation.value()
    );
    if frames == 0 {
        return Err(AudioError::InvalidFrameSize);
    }
    Ok(frames)
}

fn adapter_smoke(
    duration_ms: u64,
    capture_id: Option<&str>,
    render_id: Option<&str>,
    route: bool,
) -> std::result::Result<(), AudioError> {
    let endpoints = enumerate_active_endpoints()?;
    let capture_info = select_endpoint(&endpoints, EndpointDirection::Capture, capture_id)
        .ok_or_else(|| {
            AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80070490u32 as i32),
                "no active capture endpoint",
            ))
        })?;
    let render_info = select_endpoint(&endpoints, EndpointDirection::Render, render_id)
        .ok_or_else(|| {
            AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80070490u32 as i32),
                "no active render endpoint",
            ))
        })?;
    let capture_bytes_per_frame = capture_info.bytes_per_frame()?;
    let render_bytes_per_frame = render_info.bytes_per_frame()?;
    if capture_bytes_per_frame == 0 || render_bytes_per_frame == 0 {
        return Err(AudioError::InvalidFrameSize);
    }
    if !capture_info.is_ieee_float32() || capture_info.channels > 2 {
        return Err(AudioError::InvalidFrameSize);
    }
    if route
        && (!render_info.is_ieee_float32()
            || render_info.channels == 0
            || render_info.channels > 2)
    {
        return Err(AudioError::InvalidFrameSize);
    }
    let mut monitor = EndpointMonitor::start()?;
    let mut capture =
        SharedCapture::open_refreshed_bound(&mut monitor, capture_info, 1_000_000)?;
    let mut render = SharedRender::open_refreshed_bound(&mut monitor, render_info, 1_000_000)?;
    let scheduler = RealtimeScheduler::new(8, usize::from(capture_info.channels), 128)
        .map_err(|_| AudioError::InvalidFrameSize)?;
    let generation = RuntimeGeneration::new(1);
    let _ = scheduler.publish(RuntimeGraph::prepare(
        generation,
        vec![ProcessingStage::Gain { linear: 0.5 }],
    ));
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
        let mut graph_blocks = 0u32;
        let mut render_frames = 0u32;
        let mut destination = vec![0u8; 1_048_576];
        let render_source = vec![0u8; 1_048_576];
        // Keep a finite carry queue for a render period that is temporarily
        // smaller than a processed block. At 64 graph blocks this is bounded
        // to roughly 170 ms for stereo float32, and exhaustion fails closed.
        let render_pending_capacity = 64 * 128 * render_bytes_per_frame;
        let mut render_pending = vec![0u8; render_pending_capacity];
        let mut render_pending_bytes = 0usize;
        let mut staging = vec![0u8; 128 * capture_bytes_per_frame];
        let mut source_block = AudioBlock::new(usize::from(capture_info.channels), 128)
            .map_err(|_| AudioError::InvalidFrameSize)?;
        let mut pending_frames = 0usize;
        let mut routed_frames = 0u32;
        let mut drift = (capture_info.sample_rate_hz != render_info.sample_rate_hz)
            // Keep the controller target at the midpoint of the bounded
            // resampler FIFO so both clock directions have room to recover.
            .then(|| DriftController::new(capture_info.sample_rate_hz, render_info.sample_rate_hz, 512, 100.0))
            .transpose()
            .map_err(|_| AudioError::InvalidFrameSize)?;
        let mut streaming_resampler = (capture_info.sample_rate_hz != render_info.sample_rate_hz)
            .then(|| StreamingResampler::new(usize::from(capture_info.channels), 1024))
            .transpose()
            .map_err(|_| AudioError::InvalidFrameSize)?;
        while std::time::Instant::now() < deadline {
            let mut render_submitted = false;
            let mut render_submitted_frames = 0u32;
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
                                source_block.channel_mut(channel).unwrap()[frame] = sample;
                            }
                        }
                        if capture_info.sample_rate_hz == render_info.sample_rate_hz {
                            for channel in 0..usize::from(capture_info.channels) {
                                block.channel_mut(channel).unwrap().copy_from_slice(
                                    source_block
                                        .channel(channel)
                                        .ok_or(AudioError::InvalidFrameSize)?,
                                );
                            }
                        } else {
                            let resampler = streaming_resampler
                                .as_mut()
                                .ok_or(AudioError::InvalidFrameSize)?;
                            let accepted = resampler.push(&source_block).map_err(|_| {
                                AudioError::InvalidFrameSize
                            })?;
                            if accepted != source_block.frames() {
                                return Err(AudioError::BufferTooSmall {
                                    required: source_block.frames(),
                                    available: accepted,
                                });
                            }
                            if let Some(controller) = drift.as_mut() {
                                controller.observe_queue(resampler.queued_frames());
                            }
                            let produced = resampler
                                .process(
                                    &mut block,
                                    drift.as_ref().map_or_else(
                                        || capture_info.sample_rate_hz as f64 / render_info.sample_rate_hz as f64,
                                        DriftController::adjusted_ratio,
                                    ),
                                )
                                .map_err(|_| AudioError::InvalidFrameSize)?;
                            if produced != block.frames() {
                                scheduler
                                    .input()
                                    .try_recycle(block)
                                    .map_err(|_| AudioError::InvalidFrameSize)?;
                                pending_frames = 0;
                                continue;
                            }
                        }
                        if let Err(block) = scheduler.submit_input(block) {
                            scheduler
                                .input()
                                .try_recycle(block)
                                .map_err(|_| AudioError::InvalidFrameSize)?;
                            continue;
                        }
                        let processed_generation = scheduler
                            .process_once()
                            .map_err(|_| AudioError::InvalidFrameSize)?;
                        if let Some(output) = scheduler.receive_output() {
                            if processed_generation != Some(generation)
                                || (0..output.channels()).any(|channel| {
                                    output.channel(channel).is_none_or(|samples| {
                                        samples.iter().any(|sample| !sample.is_finite())
                                    })
                                })
                            {
                                scheduler
                                    .output()
                                    .try_recycle(output)
                                    .map_err(|_| AudioError::InvalidFrameSize)?;
                                return Err(AudioError::Windows(windows::core::Error::new(
                                    windows::core::HRESULT(0x80004005u32 as i32),
                                    "adapter smoke graph output was invalid",
                                )));
                            }
                            graph_blocks = graph_blocks.saturating_add(1);
                            scheduler_frames = scheduler_frames.saturating_add(128);
                            let route_result = if route {
                                (|| {
                                    let required = 128 * render_bytes_per_frame;
                                    if render_pending_bytes + required > render_pending.len() {
                                        return Err(AudioError::BufferTooSmall {
                                            required: render_pending_bytes + required,
                                            available: render_pending.len(),
                                        });
                                    }
                                    for frame in 0..128 {
                                        for channel in 0..usize::from(render_info.channels) {
                                            let first = output
                                                .channel(0)
                                                .ok_or(AudioError::InvalidFrameSize)?[frame];
                                            let second = if capture_info.channels > 1 {
                                                Some(
                                                    output
                                                        .channel(1)
                                                        .ok_or(AudioError::InvalidFrameSize)?
                                                        [frame],
                                                )
                                            } else {
                                                None
                                            };
                                            let sample = map_route_sample(
                                                first,
                                                second,
                                                capture_info.channels,
                                                render_info.channels,
                                                channel,
                                            );
                                            let offset =
                                                frame * render_bytes_per_frame + channel * 4;
                                            let pending_offset = render_pending_bytes + offset;
                                            render_pending[pending_offset..pending_offset + 4]
                                                .copy_from_slice(&sample.to_le_bytes());
                                        }
                                    }
                                    render_pending_bytes += required;
                                    Ok(())
                                })()
                            } else {
                                Ok(())
                            };
                            let recycle_result = scheduler.output().try_recycle(output);
                            route_result?;
                            recycle_result.map_err(|_| AudioError::InvalidFrameSize)?;
                        }
                        pending_frames = 0;
                    }
                }
            }
            if route {
                let submitted = if render.wait_for_data(10)? {
                    drain_render_pending(
                        &render,
                        &mut render_pending,
                        &mut render_pending_bytes,
                        render_bytes_per_frame,
                    )?
                } else {
                    0
                };
                routed_frames = routed_frames.saturating_add(submitted);
                render_submitted_frames = render_submitted_frames.saturating_add(submitted);
                render_submitted |= submitted > 0;
            }
            if !render_submitted && (!route || render_pending_bytes == 0) {
                render_frames = render_frames
                    .saturating_add(render.submit_bytes(&render_source, render_bytes_per_frame)?);
            } else {
                render_frames = render_frames.saturating_add(render_submitted_frames);
            }
        }
        if capture_packets == 0
            || scheduler_frames == 0
            || render_frames == 0
            || (route && routed_frames == 0)
        {
            return Err(AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80004005u32 as i32),
                "adapter smoke test received no bounded stream data",
            )));
        }
        let telemetry = scheduler.telemetry();
        let resampler_queued_frames = streaming_resampler
            .as_ref()
            .map_or(0, StreamingResampler::queued_frames);
        let drift_correction_ppm = drift
            .as_ref()
            .map_or(0.0, DriftController::correction_ppm);
        if telemetry.active_generation != Some(generation)
            || telemetry.processed_quanta != u64::from(graph_blocks)
            || telemetry.xruns != 0
            || telemetry.input_overruns != 0
            || telemetry.output_overruns != 0
            || resampler_queued_frames > 1024
            || drift_correction_ppm.abs() > 100.0
        {
            return Err(AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80004005u32 as i32),
                "adapter smoke scheduler telemetry was invalid",
            )));
        }
        println!(
            "adapter_smoke capture_endpoint={} render_endpoint={} capture_packets={} capture_frames={} capture_bytes={} graph_generation={} graph_blocks={} scheduler_frames={} pending_frames={} render_frames={} routed_frames={} route={} resampler_queued_frames={} drift_correction_ppm={:.3} scheduler_processed_quanta={} scheduler_xruns={} scheduler_input_overruns={} scheduler_output_overruns={}",
            capture_info.id,
            render_info.id,
            capture_packets,
            capture_frames,
            capture_bytes,
            generation.value(),
            graph_blocks,
            scheduler_frames,
            pending_frames,
            render_frames,
            routed_frames,
            route,
            resampler_queued_frames,
            drift_correction_ppm,
            telemetry.processed_quanta,
            telemetry.xruns,
            telemetry.input_overruns,
            telemetry.output_overruns
        );
        Ok(())
    })();
    let capture_stop = capture.stop();
    let render_stop = render.stop();
    result.and(capture_stop).and(render_stop)
}

fn select_endpoint<'a>(
    endpoints: &'a [audiorouter_windows_audio::EndpointInfo],
    direction: EndpointDirection,
    endpoint_id: Option<&str>,
) -> Option<&'a audiorouter_windows_audio::EndpointInfo> {
    endpoints.iter().find(|endpoint| {
        endpoint.direction == direction && endpoint_id.is_none_or(|id| endpoint.id == id)
    })
}

fn map_route_sample(
    first: f32,
    second: Option<f32>,
    source_channels: u16,
    destination_channels: u16,
    destination_channel: usize,
) -> f32 {
    match (source_channels, destination_channels, destination_channel) {
        (2, 1, 0) => (first + second.unwrap_or(first)) * 0.5,
        (1, 2, _) => first,
        _ => {
            if destination_channel == 0 {
                first
            } else {
                second.unwrap_or(first)
            }
        }
    }
}

fn drain_render_pending(
    render: &SharedRender,
    pending: &mut [u8],
    pending_bytes: &mut usize,
    bytes_per_frame: usize,
) -> std::result::Result<u32, AudioError> {
    let mut submitted_total = 0u32;
    while *pending_bytes > 0 {
        let submitted = render.submit_bytes(&pending[..*pending_bytes], bytes_per_frame)?;
        if submitted == 0 {
            break;
        }
        let submitted_bytes = usize::try_from(submitted)
            .ok()
            .and_then(|frames| frames.checked_mul(bytes_per_frame))
            .ok_or(AudioError::InvalidFrameSize)?;
        if submitted_bytes > *pending_bytes {
            return Err(AudioError::InvalidFrameSize);
        }
        pending.copy_within(submitted_bytes..*pending_bytes, 0);
        *pending_bytes -= submitted_bytes;
        submitted_total = submitted_total.saturating_add(submitted);
    }
    Ok(submitted_total)
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
        _ => std::ptr::addr_of!(float_format) as *mut WAVEFORMATEX,
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

#[cfg(test)]
mod tests {
    use super::map_route_sample;
    use super::select_endpoint;
    use audiorouter_windows_audio::{EndpointDirection, EndpointInfo};

    fn endpoint(id: &str, direction: EndpointDirection) -> EndpointInfo {
        EndpointInfo {
            id: id.into(),
            direction,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 0,
            subformat_guid: String::new(),
        }
    }

    #[test]
    fn route_selection_matches_opaque_id_and_direction() {
        let endpoints = vec![
            endpoint("capture-a", EndpointDirection::Capture),
            endpoint("render-a", EndpointDirection::Render),
            endpoint("capture-b", EndpointDirection::Capture),
        ];
        assert_eq!(
            select_endpoint(&endpoints, EndpointDirection::Capture, Some("capture-b"))
                .map(|item| item.id.as_str()),
            Some("capture-b")
        );
        assert!(
            select_endpoint(&endpoints, EndpointDirection::Render, Some("capture-b")).is_none()
        );
        assert!(select_endpoint(&endpoints, EndpointDirection::Capture, Some("missing")).is_none());
    }

    #[test]
    fn route_selection_without_id_only_supports_explicit_direction() {
        let endpoints = vec![
            endpoint("render-a", EndpointDirection::Render),
            endpoint("capture-a", EndpointDirection::Capture),
        ];
        assert_eq!(
            select_endpoint(&endpoints, EndpointDirection::Capture, None)
                .map(|item| item.id.as_str()),
            Some("capture-a")
        );
    }

    #[test]
    fn route_channel_mapping_handles_mono_and_stereo() {
        assert_eq!(map_route_sample(0.25, Some(0.75), 2, 1, 0), 0.5);
        assert_eq!(map_route_sample(0.25, None, 1, 2, 0), 0.25);
        assert_eq!(map_route_sample(0.25, None, 1, 2, 1), 0.25);
        assert_eq!(map_route_sample(0.25, Some(0.75), 2, 2, 1), 0.75);
    }
}
