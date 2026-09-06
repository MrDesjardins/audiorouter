use audiorouter_plugin_host::{
    read_worker_message, write_worker_message, WorkerMessage, WorkerSession,
    WORKER_PROTOCOL_VERSION,
};
use std::io::{self, BufReader, BufWriter};
use std::process::ExitCode;

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
    let (plugin_sha256, channels) = parse_arguments()?;
    let _session = WorkerSession::new(&plugin_sha256, channels)
        .map_err(|error| format!("invalid worker configuration: {error:?}"))?;
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
    if read_worker_message(&mut reader).map_err(|error| format!("ready read failed: {error:?}"))?
        != WorkerMessage::Ready
    {
        return Err("worker requires Ready after Hello".into());
    }

    loop {
        match read_worker_message(&mut reader)
            .map_err(|error| format!("message read failed: {error:?}"))?
        {
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

fn parse_arguments() -> Result<(String, u16), String> {
    let mut arguments = std::env::args().skip(1);
    let mut hash = None;
    let mut channels = None;
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
    Ok((hash, channels))
}
