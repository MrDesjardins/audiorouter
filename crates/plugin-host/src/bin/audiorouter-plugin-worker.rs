#[cfg(windows)]
use audiorouter_plugin_host::vst2::{Vst2EditorThread, Vst2Library};
#[cfg(feature = "test-fixtures")]
use audiorouter_plugin_host::ParameterDescriptor;
use audiorouter_plugin_host::{
    read_worker_message, worker_clock_tick, write_worker_message, PluginStateAsset,
    SharedAudioLayout, SharedAudioTransport, WorkerAudioBusLayout, WorkerFrameGuard, WorkerMessage,
    WorkerSession, DEFAULT_WORKER_SAMPLE_RATE_HZ, MAX_WORKER_SAMPLE_RATE_HZ,
    MIN_WORKER_SAMPLE_RATE_HZ, WORKER_PROTOCOL_VERSION,
};
#[cfg(feature = "test-fixtures")]
use std::io::Write;
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;
use std::process::ExitCode;
#[cfg(feature = "test-fixtures")]
use std::thread;
#[cfg(feature = "test-fixtures")]
use std::time::Duration;

type WorkerArguments = (
    String,
    u16,
    u32,
    Option<(PathBuf, PathBuf)>,
    Option<String>,
    Option<PathBuf>,
    Option<WorkerAudioBusLayout>,
);

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("plugin worker stopped: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let (
        plugin_sha256,
        channels,
        sample_rate_hz,
        shared_paths,
        _fixture_mode,
        plugin_path,
        multi_bus_layout,
    ) = parse_arguments()?;
    if let Some(layout) = multi_bus_layout {
        return run_multi_bus(plugin_sha256, layout);
    }
    #[cfg(windows)]
    let mut vst2_plugin = plugin_path
        .clone()
        .map(|path| Vst2Library::load(&path))
        .transpose()
        .map_err(|error| format!("VST2 load failed: {error:?}"))?;
    #[cfg(windows)]
    let vst2_editor = plugin_path
        .as_deref()
        .map(Vst2EditorThread::spawn)
        .transpose()
        .map_err(|error| format!("VST2 editor thread failed: {error:?}"))?;
    #[cfg(not(windows))]
    if plugin_path.is_some() {
        return Err("VST2 loading requires Windows".into());
    }
    let mut session = WorkerSession::new(&plugin_sha256, channels)
        .map_err(|error| format!("invalid worker configuration: {error:?}"))?;
    let mut shared = shared_paths
        .map(|(input_path, output_path)| {
            SharedAudioTransport::open(
                input_path,
                output_path,
                SharedAudioLayout::new(channels).map_err(|error| format!("{error:?}"))?,
            )
            .map_err(|error| format!("shared transport open failed: {error:?}"))
        })
        .transpose()?;
    let mut state: Option<PluginStateAsset> = None;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    write_worker_message(
        &mut writer,
        &WorkerMessage::Hello {
            protocol_version: WORKER_PROTOCOL_VERSION,
            plugin_sha256,
            channels,
        },
    )
    .map_err(|error| format!("hello write failed: {error:?}"))?;
    session
        .hello_sent()
        .map_err(|error| format!("hello state transition failed: {error:?}"))?;
    let ready = read_worker_message(&mut reader)
        .map_err(|error| format!("ready read failed: {error:?}"))?;
    if ready != WorkerMessage::Ready {
        return Err("worker requires Ready after Hello".into());
    }
    session
        .accept(&ready, 0)
        .map_err(|error| format!("ready rejected: {error:?}"))?;

    #[cfg(feature = "test-fixtures")]
    if _fixture_mode.as_deref() == Some("crash") {
        return Err("controlled fixture crash".into());
    }

    #[cfg(feature = "test-fixtures")]
    if _fixture_mode.as_deref() == Some("hang") {
        loop {
            thread::sleep(Duration::from_secs(60));
        }
    }

    loop {
        let message = read_worker_message(&mut reader)
            .map_err(|error| format!("message read failed: {error:?}"))?;
        if let Err(error) = session.accept(&message, worker_clock_tick()) {
            write_worker_message(
                &mut writer,
                &WorkerMessage::Failure {
                    code: format!("session:{error:?}"),
                },
            )
            .map_err(|write_error| format!("failure write failed: {write_error:?}"))?;
            return Err(format!("worker session rejected message: {error:?}"));
        }
        match message {
            WorkerMessage::ProcessShared {
                sequence,
                deadline_tick,
                channels: frame_channels,
                frames,
                parameters: _,
            } => {
                let transport = shared
                    .as_mut()
                    .ok_or_else(|| "shared transport was not configured".to_string())?;
                let frame = transport
                    .read_input()
                    .map_err(|error| format!("shared input read failed: {error:?}"))?;
                if frame.sequence != sequence
                    || frame.deadline_tick != deadline_tick
                    || frame.channels != frame_channels
                    || frame.frame_count() != frames as usize
                {
                    return Err("shared input metadata mismatch".into());
                }
                transport
                    .write_output(&frame)
                    .map_err(|error| format!("shared output write failed: {error:?}"))?;
                write_worker_message(
                    &mut writer,
                    &WorkerMessage::ProcessedShared {
                        sequence,
                        deadline_tick,
                        channels: frame_channels,
                        frames,
                    },
                )
                .map_err(|error| format!("processed shared write failed: {error:?}"))?;
            }
            WorkerMessage::Process {
                mut frame,
                parameters,
            } => {
                if frame.channels != channels {
                    write_worker_message(
                        &mut writer,
                        &WorkerMessage::Failure {
                            code: "channelMismatch".into(),
                        },
                    )
                    .map_err(|error| format!("failure write failed: {error:?}"))?;
                    return Err("process frame channel count does not match Hello".into());
                }
                #[cfg(windows)]
                if let Some(plugin) = vst2_plugin.as_mut() {
                    if let Err(error) =
                        process_vst2_frame(plugin, &mut frame, &parameters, sample_rate_hz)
                    {
                        let failure = format!("vst2Processing:{error}");
                        write_worker_message(
                            &mut writer,
                            &WorkerMessage::Failure { code: failure },
                        )
                        .map_err(|write_error| {
                            format!("VST2 failure write failed: {write_error:?}")
                        })?;
                        return Err(format!("VST2 processing failed: {error}"));
                    }
                }
                #[cfg(feature = "test-fixtures")]
                if _fixture_mode.as_deref() == Some("invalid-output") {
                    let payload = br#"{"Processed":{"frame":{"sequence":1,"deadline_tick":1,"channels":1,"samples":[null]}}}"#;
                    writer
                        .write_all(&(payload.len() as u32).to_le_bytes())
                        .and_then(|_| writer.write_all(payload))
                        .and_then(|_| writer.flush())
                        .map_err(|error| format!("invalid fixture write failed: {error}"))?;
                    return Err("controlled fixture invalid output".into());
                }
                write_worker_message(&mut writer, &WorkerMessage::Processed { frame })
                    .map_err(|error| format!("processed write failed: {error:?}"))?;
            }
            WorkerMessage::Latency(latency) => {
                #[cfg(windows)]
                let latency = if let Some(plugin) = vst2_plugin.as_ref() {
                    plugin
                        .latency(latency.sample_rate_hz)
                        .map_err(|error| format!("VST2 latency query failed: {error:?}"))?
                } else {
                    latency
                };
                #[cfg(feature = "test-fixtures")]
                let latency = if _fixture_mode.as_deref() == Some("latency") {
                    audiorouter_plugin_host::WorkerLatency::new(
                        latency.samples.saturating_add(64),
                        latency.sample_rate_hz,
                    )
                    .map_err(|error| format!("fixture latency invalid: {error:?}"))?
                } else {
                    latency
                };
                write_worker_message(&mut writer, &WorkerMessage::Latency(latency))
                    .map_err(|error| format!("latency write failed: {error:?}"))?;
            }
            WorkerMessage::DescribeParameters => {
                let fixture_descriptors = {
                    #[cfg(feature = "test-fixtures")]
                    {
                        if _fixture_mode.as_deref() == Some("descriptors") {
                            vec![
                                ParameterDescriptor::new(1, "Mix", 0.5, 0.0, 1.0).map_err(
                                    |error| format!("fixture descriptor invalid: {error:?}"),
                                )?,
                                ParameterDescriptor::new(2, "Output", 0.0, 0.0, 1.0).map_err(
                                    |error| format!("fixture descriptor invalid: {error:?}"),
                                )?,
                            ]
                        } else {
                            Vec::new()
                        }
                    }
                    #[cfg(not(feature = "test-fixtures"))]
                    {
                        Vec::new()
                    }
                };
                #[cfg(windows)]
                let descriptors = if let Some(plugin) = vst2_plugin.as_mut() {
                    plugin
                        .parameter_descriptors()
                        .map_err(|error| format!("VST2 parameter description failed: {error:?}"))?
                } else {
                    fixture_descriptors
                };
                #[cfg(not(windows))]
                let descriptors = fixture_descriptors;
                write_worker_message(&mut writer, &WorkerMessage::Parameters { descriptors })
                    .map_err(|error| format!("parameter description write failed: {error:?}"))?;
            }
            WorkerMessage::DescribeEditor => {
                #[cfg(windows)]
                let descriptor = if let Some(plugin) = vst2_plugin.as_mut() {
                    plugin
                        .editor_descriptor()
                        .map_err(|error| format!("VST2 editor description failed: {error:?}"))?
                } else {
                    audiorouter_plugin_host::EditorDescriptor::new(false, 0, 0)
                        .map_err(|error| format!("editor descriptor invalid: {error:?}"))?
                };
                #[cfg(not(windows))]
                let descriptor = audiorouter_plugin_host::EditorDescriptor::new(false, 0, 0)
                    .map_err(|error| format!("editor descriptor invalid: {error:?}"))?;
                write_worker_message(&mut writer, &WorkerMessage::Editor(descriptor))
                    .map_err(|error| format!("editor description write failed: {error:?}"))?;
            }
            WorkerMessage::EditorOpen {
                parent_window,
                parent_process_id,
                authorization_token,
            } => {
                #[cfg(windows)]
                let result = match vst2_editor.as_ref() {
                    Some(editor) => usize::try_from(parent_window)
                        .map_err(|_| "invalid parent window".to_string())
                        .and_then(|parent| {
                            editor.open(parent, parent_process_id, &authorization_token)
                        }),
                    None => Err("editorUnavailable".to_string()),
                };
                #[cfg(not(windows))]
                let result: Result<(), String> = Err("editorUnavailable".into());
                match result {
                    Ok(()) => write_worker_message(
                        &mut writer,
                        &WorkerMessage::EditorOpened { parent_window },
                    )
                    .map_err(|error| format!("editor open write failed: {error:?}"))?,
                    Err(error) => {
                        write_worker_message(&mut writer, &WorkerMessage::Failure { code: error })
                            .map_err(|write_error| {
                            format!("editor failure write failed: {write_error:?}")
                        })?
                    }
                }
            }
            WorkerMessage::EditorClose => {
                #[cfg(windows)]
                let result = vst2_editor.as_ref().map_or_else(
                    || Err("editorUnavailable".to_string()),
                    Vst2EditorThread::close,
                );
                #[cfg(not(windows))]
                let result: Result<(), String> = Err("editorUnavailable".into());
                match result {
                    Ok(()) => write_worker_message(&mut writer, &WorkerMessage::EditorClosed)
                        .map_err(|error| format!("editor close write failed: {error:?}"))?,
                    Err(error) => {
                        write_worker_message(&mut writer, &WorkerMessage::Failure { code: error })
                            .map_err(|write_error| {
                            format!("editor failure write failed: {write_error:?}")
                        })?
                    }
                }
            }
            WorkerMessage::StateRestore { asset } => {
                #[cfg(windows)]
                if let Some(plugin) = vst2_plugin.as_mut() {
                    if let Err(error) = plugin.restore_state(&asset.bytes) {
                        write_worker_message(
                            &mut writer,
                            &WorkerMessage::Failure {
                                code: format!("vst2StateRestore:{error:?}"),
                            },
                        )
                        .map_err(|write_error| {
                            format!("VST2 state failure write failed: {write_error:?}")
                        })?;
                        continue;
                    }
                }
                state = Some(asset.clone());
                write_worker_message(&mut writer, &WorkerMessage::State { asset })
                    .map_err(|error| format!("state restore write failed: {error:?}"))?;
            }
            WorkerMessage::StateSave => {
                #[cfg(windows)]
                let plugin_asset = if let Some(plugin) = vst2_plugin.as_mut() {
                    match plugin.save_state() {
                        Ok(bytes) => Some(
                            PluginStateAsset::new(1, bytes)
                                .map_err(|error| format!("VST2 state asset invalid: {error:?}"))?,
                        ),
                        Err(error) => {
                            write_worker_message(
                                &mut writer,
                                &WorkerMessage::Failure {
                                    code: format!("vst2StateSave:{error:?}"),
                                },
                            )
                            .map_err(|write_error| {
                                format!("VST2 state failure write failed: {write_error:?}")
                            })?;
                            continue;
                        }
                    }
                } else {
                    None
                };
                #[cfg(not(windows))]
                let plugin_asset = None;
                let Some(asset) = plugin_asset.or_else(|| state.clone()) else {
                    write_worker_message(
                        &mut writer,
                        &WorkerMessage::Failure {
                            code: "stateUnavailable".into(),
                        },
                    )
                    .map_err(|error| format!("state failure write failed: {error:?}"))?;
                    return Err("no opaque state has been restored".into());
                };
                write_worker_message(&mut writer, &WorkerMessage::State { asset })
                    .map_err(|error| format!("state save write failed: {error:?}"))?;
            }
            WorkerMessage::Shutdown => return Ok(()),
            _ => {
                write_worker_message(
                    &mut writer,
                    &WorkerMessage::Failure {
                        code: "unexpectedMessage".into(),
                    },
                )
                .map_err(|error| format!("failure write failed: {error:?}"))?;
                return Err("unexpected worker message".into());
            }
        }
    }
}

fn run_multi_bus(plugin_sha256: String, layout: WorkerAudioBusLayout) -> Result<(), String> {
    if layout.input_buses() != layout.output_buses() {
        return Err("multi-bus fixture requires symmetric input/output buses".into());
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    let mut frame_guard = WorkerFrameGuard::new();
    write_worker_message(
        &mut writer,
        &WorkerMessage::HelloBuses {
            protocol_version: WORKER_PROTOCOL_VERSION,
            plugin_sha256,
            layout: layout.clone(),
        },
    )
    .map_err(|error| format!("multi-bus hello write failed: {error:?}"))?;
    let ready = read_worker_message(&mut reader)
        .map_err(|error| format!("multi-bus ready read failed: {error:?}"))?;
    if ready != WorkerMessage::Ready {
        return Err("multi-bus worker requires Ready after HelloBuses".into());
    }
    loop {
        let message = read_worker_message(&mut reader)
            .map_err(|error| format!("multi-bus message read failed: {error:?}"))?;
        match message {
            WorkerMessage::ProcessBuses {
                layout: message_layout,
                frames,
                parameters,
            } if message_layout == layout => {
                if !parameters.is_empty() {
                    return Err("multi-bus echo fixture does not accept parameters".into());
                }
                let frames = layout
                    .input_frames(frames)
                    .map_err(|error| format!("multi-bus input rejected: {error:?}"))?;
                if let Err(error) = frame_guard.accept(
                    frames
                        .frames()
                        .first()
                        .ok_or_else(|| "multi-bus input has no main bus".to_string())?,
                    worker_clock_tick(),
                ) {
                    write_worker_message(
                        &mut writer,
                        &WorkerMessage::Failure {
                            code: format!("multiBusIdentity:{error:?}"),
                        },
                    )
                    .map_err(|write_error| {
                        format!("multi-bus identity failure write failed: {write_error:?}")
                    })?;
                    return Err(format!("multi-bus identity rejected: {error:?}"));
                }
                write_worker_message(
                    &mut writer,
                    &WorkerMessage::ProcessedBuses {
                        layout: layout.clone(),
                        frames: frames.frames().to_vec(),
                    },
                )
                .map_err(|error| format!("multi-bus response write failed: {error:?}"))?;
            }
            WorkerMessage::Shutdown => return Ok(()),
            _ => {
                write_worker_message(
                    &mut writer,
                    &WorkerMessage::Failure {
                        code: "multiBusUnexpectedMessage".into(),
                    },
                )
                .map_err(|error| format!("multi-bus failure write failed: {error:?}"))?;
                return Err("multi-bus worker rejected message".into());
            }
        }
    }
}

fn parse_arguments() -> Result<WorkerArguments, String> {
    let mut arguments = std::env::args().skip(1);
    let mut hash = None;
    let mut channels = None;
    let mut sample_rate_hz = None;
    let mut input_path = None;
    let mut output_path = None;
    let mut plugin_path = None;
    let mut input_buses = None;
    let mut output_buses = None;
    #[cfg(feature = "test-fixtures")]
    let mut fixture_mode = None;
    #[cfg(not(feature = "test-fixtures"))]
    let fixture_mode: Option<String> = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--plugin-sha256" => hash = arguments.next(),
            "--channels" => {
                channels = Some(
                    arguments
                        .next()
                        .ok_or_else(|| "--channels requires a value".to_string())?
                        .parse::<u16>()
                        .map_err(|_| "--channels must be 1 or 2".to_string())?,
                )
            }
            "--sample-rate" => {
                sample_rate_hz = Some(
                    arguments
                        .next()
                        .ok_or_else(|| "--sample-rate requires a value".to_string())?
                        .parse::<u32>()
                        .map_err(|_| "--sample-rate must be an integer".to_string())?,
                )
            }
            "--input-path" => input_path = arguments.next().map(PathBuf::from),
            "--output-path" => output_path = arguments.next().map(PathBuf::from),
            "--plugin-path" => {
                plugin_path =
                    Some(PathBuf::from(arguments.next().ok_or_else(|| {
                        "--plugin-path requires a value".to_string()
                    })?));
            }
            "--input-buses" => input_buses = arguments.next(),
            "--output-buses" => output_buses = arguments.next(),
            "--fixture-mode" => {
                #[cfg(feature = "test-fixtures")]
                {
                    let mode = arguments
                        .next()
                        .ok_or_else(|| "--fixture-mode requires a value".to_string())?;
                    if !matches!(
                        mode.as_str(),
                        "crash" | "hang" | "invalid-output" | "descriptors" | "latency"
                    ) {
                        return Err("unsupported --fixture-mode".into());
                    }
                    fixture_mode = Some(mode);
                }
                #[cfg(not(feature = "test-fixtures"))]
                return Err("--fixture-mode is unavailable in this build".into());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    let hash = hash.ok_or_else(|| "--plugin-sha256 is required".to_string())?;
    let channels = channels.ok_or_else(|| "--channels is required".to_string())?;
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("--plugin-sha256 must be 64 hexadecimal characters".into());
    }
    if !matches!(channels, 1 | 2) {
        return Err("--channels must be 1 or 2".into());
    }
    let sample_rate_hz = sample_rate_hz.unwrap_or(DEFAULT_WORKER_SAMPLE_RATE_HZ);
    if !(MIN_WORKER_SAMPLE_RATE_HZ..=MAX_WORKER_SAMPLE_RATE_HZ).contains(&sample_rate_hz) {
        return Err("--sample-rate must be between 8000 and 192000 Hz".into());
    }
    let multi_bus_layout = match (input_buses, output_buses) {
        (Some(input), Some(output)) => {
            if plugin_path.is_some() || input_path.is_some() || output_path.is_some() {
                return Err("multi-bus mode cannot use plugin or shared paths".into());
            }
            let parse_buses = |value: String| -> Result<Vec<u16>, String> {
                let buses = value
                    .split(',')
                    .map(|bus| {
                        bus.parse::<u16>()
                            .map_err(|_| "bus channels must be 1 or 2".to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if buses.is_empty() {
                    return Err("bus list must not be empty".into());
                }
                Ok(buses)
            };
            Some(
                WorkerAudioBusLayout::new(&parse_buses(input)?, &parse_buses(output)?)
                    .map_err(|error| format!("invalid multi-bus layout: {error:?}"))?,
            )
        }
        (None, None) => None,
        _ => return Err("--input-buses and --output-buses must be supplied together".into()),
    };
    match (input_path, output_path) {
        (Some(input), Some(output)) => Ok((
            hash,
            channels,
            sample_rate_hz,
            Some((input, output)),
            fixture_mode,
            plugin_path,
            multi_bus_layout,
        )),
        (None, None) => Ok((
            hash,
            channels,
            sample_rate_hz,
            None,
            fixture_mode,
            plugin_path,
            multi_bus_layout,
        )),
        _ => Err("--input-path and --output-path must be supplied together".into()),
    }
}

#[cfg(windows)]
fn process_vst2_frame(
    plugin: &mut Vst2Library,
    frame: &mut audiorouter_plugin_host::WorkerFrame,
    parameters: &[audiorouter_plugin_host::ParameterEvent],
    sample_rate_hz: u32,
) -> Result<(), String> {
    let channels = usize::from(frame.channels);
    let frames = frame.samples.len() / channels;
    if plugin.output_channels() != channels {
        return Err(format!(
            "VST2 output channels {} do not match graph channels {channels}",
            plugin.output_channels()
        ));
    }
    let mut input_channels = vec![vec![0.0_f32; frames]; plugin.input_channels()];
    let mut output_channels = vec![vec![0.0_f32; frames]; plugin.output_channels()];
    for (index, sample) in frame.samples.iter().copied().enumerate() {
        input_channels[index % channels][index / channels] = sample;
    }
    let inputs: Vec<&[f32]> = input_channels.iter().map(Vec::as_slice).collect();
    let mut outputs: Vec<&mut [f32]> = output_channels.iter_mut().map(Vec::as_mut_slice).collect();
    plugin
        .set_processing_format(
            sample_rate_hz as f32,
            i32::try_from(frames).map_err(|_| "frame count overflow")?,
        )
        .map_err(|error| format!("format setup failed: {error:?}"))?;
    for event in parameters {
        if event.sample_offset >= frames {
            return Err(format!(
                "parameter sample offset {} exceeds block length {frames}",
                event.sample_offset
            ));
        }
        plugin
            .set_parameter(event.parameter_id, event.normalized_value)
            .map_err(|error| format!("parameter update failed: {error:?}"))?;
    }
    plugin
        .process_replacing(&inputs, &mut outputs)
        .map_err(|error| format!("process callback failed: {error:?}"))?;
    if output_channels
        .iter()
        .flat_map(|channel| channel.iter())
        .any(|sample| !sample.is_finite())
    {
        return Err("plugin produced a non-finite sample".into());
    }
    for (index, destination) in frame.samples.iter_mut().enumerate() {
        *destination = output_channels[index % channels][index / channels];
    }
    Ok(())
}
