use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("cable_quality_error={error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().ok_or("analyzer mode is required")?;
    let first = PathBuf::from(args.next().ok_or("input WAV is required")?);
    let (report, metrics_path) = match mode.as_str() {
        "silence" => {
            let output = PathBuf::from(args.next().ok_or("metrics output path is required")?);
            let threshold = args.next().map(|value| value.parse::<f64>()
                .map_err(|_| "peak threshold must be numeric".to_string()))
                .transpose()?.unwrap_or(-140.0);
            (m03_cable_quality::analyze_silence_wav(&first, threshold)?, output)
        }
        "tone" => {
            let capture = PathBuf::from(args.next().ok_or("capture WAV is required")?);
            let output = PathBuf::from(args.next().ok_or("metrics output path is required")?);
            let frequency = args.next().ok_or("tone frequency is required")?.parse::<f64>()
                .map_err(|_| "tone frequency must be numeric".to_string())?;
            let threshold = args.next().ok_or("THD+N threshold is required")?.parse::<f64>()
                .map_err(|_| "THD+N threshold must be numeric".to_string())?;
            (m03_cable_quality::analyze_tone_pair_wav(&first, &capture, frequency, threshold)?, output)
        }
        "pcm" => {
            let capture = PathBuf::from(args.next().ok_or("capture WAV is required")?);
            let output = PathBuf::from(args.next().ok_or("metrics output path is required")?);
            let valid_bits = args.next().ok_or("PCM valid bit count is required")?.parse::<u16>()
                .map_err(|_| "PCM valid bit count must be an integer".to_string())?;
            (m03_cable_quality::analyze_pcm_pair_wav(&first, &capture, 96_000, valid_bits)?, output)
        }
        "sweep" => {
            let capture = PathBuf::from(args.next().ok_or("capture WAV is required")?);
            let output = PathBuf::from(args.next().ok_or("metrics output path is required")?);
            let start_hz = args.next().ok_or("sweep start frequency is required")?.parse::<f64>()
                .map_err(|_| "sweep start frequency must be numeric".to_string())?;
            let end_hz = args.next().ok_or("sweep end frequency is required")?.parse::<f64>()
                .map_err(|_| "sweep end frequency must be numeric".to_string())?;
            let tolerance = args.next().ok_or("sweep response tolerance is required")?.parse::<f64>()
                .map_err(|_| "sweep response tolerance must be numeric".to_string())?;
            (m03_cable_quality::analyze_sweep_pair_wav(&first, &capture, start_hz, end_hz, tolerance)?, output)
        }
        "analyze" => {
            let capture = PathBuf::from(args.next().ok_or("capture WAV is required")?);
            let output = PathBuf::from(args.next().ok_or("metrics output path is required")?);
            let max_offset = args.next().map(|value| value.parse::<usize>()
                .map_err(|_| "max offset must be a nonnegative integer".to_string()))
                .transpose()?.unwrap_or(96_000);
            let frequency = args.next().map(|value| value.parse::<f64>()
                .map_err(|_| "frequency must be numeric".to_string())).transpose()?;
            (m03_cable_quality::analyze_wav_pair(&first, &capture, max_offset, frequency)?, output)
        }
        _ => return Err("usage: m03-cable-quality analyze <reference.wav> <capture.wav> <metrics.json> [max-offset-frames] [frequency-hz] | silence <capture.wav> <metrics.json> [max-peak-dbfs] | tone <reference.wav> <capture.wav> <metrics.json> <frequency-hz> <max-thdn-db> | pcm <reference.wav> <capture.wav> <metrics.json> <valid-bits:16|24|32> | sweep <reference.wav> <capture.wav> <metrics.json> <start-hz> <end-hz> <tolerance-db>".into()),
    };
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
    let parent = metrics_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::write(
        &metrics_path,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("{}: {error}", metrics_path.display()))?;
    println!(
        "{}",
        serde_json::to_string(&report).map_err(|error| error.to_string())?
    );
    Ok(())
}
