//! VM-only automatic source/recorder; analyze mode opens no Windows stream.
//! Streaming is bounded to the requested 30- or 300-second diagnostic plus a
//! fixed margin, with preallocated sample/packet storage.
#[path = "m03_direct_audio/mod.rs"]
mod m03_direct_audio;

use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [mode] if mode == "startup-check" => {
            println!("direct audio helper startup OK; no audio endpoint opened");
            Ok(())
        }
        [mode, directory] if mode == "speaker-record" => record(PathBuf::from(directory), None),
        [mode, directory, seconds @ ..] if mode == "record" && seconds.len() <= 1 => {
            m03_direct_audio::parse_seconds(seconds.first().map(String::as_str))
                .and_then(|seconds| record(PathBuf::from(directory), Some(seconds)))
        }
        [mode, wav, kind, report, seconds @ ..] if mode == "analyze" && seconds.len() <= 1 => {
            m03_direct_audio::parse_seconds(seconds.first().map(String::as_str)).and_then(
                |seconds| m03_direct_audio::analyze(Path::new(wav), kind, Path::new(report), seconds),
            )
        }
        _ => Err(
            "usage: m03_direct_audio record NEW_DIRECTORY [30|300] | speaker-record NEW_DIRECTORY | analyze WAV a|b REPORT.json [30|300]"
                .into(),
        ),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

/// `seconds` is `None` for the speaker loopback recorder (fixed 30 seconds).
fn record(directory: PathBuf, seconds: Option<u64>) -> Result<(), String> {
    if !std::env::var("COMPUTERNAME")
        .unwrap_or_default()
        .eq_ignore_ascii_case("AR-DriverTest")
    {
        return Err("record is restricted to AR-DriverTest; no endpoint was opened".into());
    }
    let canonical = directory.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(r"\\?\C:\ar") || canonical == Path::new(r"\\?\C:\ar") {
        return Err("record output must be an existing empty directory under C:\\ar".into());
    }
    if std::fs::read_dir(&canonical)
        .map_err(|e| e.to_string())?
        .next()
        .is_some()
    {
        return Err("record output directory must be empty".into());
    }
    match seconds {
        None => m03_direct_audio::record_speaker(&canonical),
        Some(seconds) => m03_direct_audio::record(&canonical, seconds),
    }
}
