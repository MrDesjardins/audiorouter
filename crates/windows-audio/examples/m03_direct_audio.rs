//! VM-only automatic source/recorder; analyze mode opens no Windows stream.
//! Streaming is bounded to 100 seconds with preallocated sample/packet storage.
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
        [mode, directory] if mode == "record" || mode == "speaker-record" => {
            record(PathBuf::from(directory), mode == "speaker-record")
        }
        [mode, wav, kind, report] if mode == "analyze" => {
            m03_direct_audio::analyze(Path::new(wav), kind, Path::new(report))
        }
        _ => {
            Err("usage: m03_direct_audio record|speaker-record NEW_DIRECTORY | analyze WAV a|b REPORT.json".into())
        }
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn record(directory: PathBuf, speaker: bool) -> Result<(), String> {
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
    if speaker {
        m03_direct_audio::record_speaker(&canonical)
    } else {
        m03_direct_audio::record(&canonical)
    }
}
