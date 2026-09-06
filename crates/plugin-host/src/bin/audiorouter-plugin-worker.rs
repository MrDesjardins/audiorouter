use audiorouter_plugin_host::{
    read_worker_message, write_worker_message, SharedAudioLayout, SharedAudioTransport,
    WorkerMessage, WorkerSession, WORKER_PROTOCOL_VERSION,
};
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;
use std::process::ExitCode;

type WorkerArguments = (String, u16, Option<(PathBuf, PathBuf)>);

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
    let (plugin_sha256, channels, shared_paths) = parse_arguments()?;
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

    loop {
        let message = read_worker_message(&mut reader)
            .map_err(|error| format!("message read failed: {error:?}"))?;
        if let Err(error) = session.accept(&message, 0) {
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
            WorkerMessage::Process { frame, parameters } => {
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
                // This binary is a protocol fixture, not a plugin host yet.
                // Echoing validated samples makes the process boundary testable
                // without executing untrusted plugin code.
                let _ = parameters;
                write_worker_message(&mut writer, &WorkerMessage::Processed { frame })
                    .map_err(|error| format!("processed write failed: {error:?}"))?;
            }
            WorkerMessage::Latency(latency) => {
                write_worker_message(&mut writer, &WorkerMessage::Latency(latency))
                    .map_err(|error| format!("latency write failed: {error:?}"))?;
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

fn parse_arguments() -> Result<WorkerArguments, String> {
    let mut arguments = std::env::args().skip(1);
    let mut hash = None;
    let mut channels = None;
    let mut input_path = None;
    let mut output_path = None;
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
            "--input-path" => input_path = arguments.next().map(PathBuf::from),
            "--output-path" => output_path = arguments.next().map(PathBuf::from),
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
    match (input_path, output_path) {
        (Some(input), Some(output)) => Ok((hash, channels, Some((input, output)))),
        (None, None) => Ok((hash, channels, None)),
        _ => Err("--input-path and --output-path must be supplied together".into()),
    }
}
