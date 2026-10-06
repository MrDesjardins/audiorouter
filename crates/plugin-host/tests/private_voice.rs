//! Local approved speech only; vendor code remains in isolated workers.
#[cfg(windows)]
#[test]
#[ignore = "installed approved plugins and private voice sample; explicit opt-in"]
fn installed_plugins_process_private_voice_and_parameter_edits() {
    use audiorouter_plugin_host::{
        inspect_binary, worker_clock_tick, ParameterEvent, SupervisedWorkerProcess, WorkerFrame,
    };
    use std::time::Instant;
    let voice_path = std::env::var("AUDIOROUTER_VOICE_SAMPLE").expect("approved sample");
    let root = std::path::PathBuf::from(
        std::env::var("AUDIOROUTER_VOICE_PLUGIN_ROOT").expect("approved installed plugin root"),
    );
    let worker_path =
        std::env::var("AUDIOROUTER_VOICE_PLUGIN_WORKER").expect("isolated worker exe");
    let bytes = std::fs::read(voice_path).unwrap();
    assert_eq!(&bytes[36..40], b"data");
    let voice = bytes[44..]
        .chunks_exact(2)
        .map(|s| f32::from(i16::from_le_bytes(s.try_into().unwrap())) / 32768.0)
        .collect::<Vec<_>>();
    let mut paths = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "dll"))
        .collect::<Vec<_>>();
    paths.sort();
    assert!(!paths.is_empty());
    for path in paths {
        let identity = inspect_binary(&path, std::slice::from_ref(&root)).unwrap();
        let process = |edits: bool| {
            let mut worker = SupervisedWorkerProcess::spawn_verified_with_sample_rate(
                &worker_path,
                &identity,
                std::slice::from_ref(&root),
                2,
                48000,
                Instant::now(),
            )
            .unwrap();
            let descriptors = worker.describe_parameters(Instant::now()).unwrap();
            let parameter = descriptors
                .iter()
                .find(|p| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .contains("reagate")
                        && p.title.to_lowercase().contains("thresh")
                })
                .or_else(|| {
                    descriptors
                        .iter()
                        .find(|p| p.title.to_lowercase().contains("wet"))
                })
                .or_else(|| {
                    descriptors
                        .iter()
                        .find(|p| p.title.to_lowercase().contains("gain"))
                })
                .or_else(|| {
                    descriptors
                        .iter()
                        .find(|p| p.title.to_lowercase().contains("threshold"))
                });
            eprintln!(
                "{}: edit {:?}",
                path.file_name().unwrap().to_string_lossy(),
                parameter.map(|p| (&p.title, p.default_value))
            );
            let mut output = Vec::new();
            for (index, chunk) in voice.chunks(128).enumerate() {
                let stereo = chunk.iter().flat_map(|s| [*s, *s]).collect::<Vec<_>>();
                let events = if edits && index == 0 {
                    parameter
                        .map(|p| {
                            vec![ParameterEvent::new(
                                p.parameter_id,
                                if p.default_value < 0.5 { 0.9 } else { 0.1 },
                                0,
                            )
                            .unwrap()]
                        })
                        .unwrap_or_default()
                } else {
                    vec![]
                };
                let frame = WorkerFrame::new(
                    index as u64 + 1,
                    worker_clock_tick().saturating_add(1_000_000),
                    2,
                    stereo,
                )
                .unwrap();
                let result = worker.process(frame, events, Instant::now()).unwrap();
                assert!(result.samples.iter().all(|s| s.is_finite()));
                output.extend(result.samples.chunks_exact(2).map(|s| s[0]));
            }
            worker.shutdown().unwrap();
            (output, parameter.is_some())
        };
        let (flat, _) = process(false);
        let (changed, has_parameter) = process(true);
        assert_eq!(flat.len(), voice.len());
        assert_eq!(changed.len(), voice.len());
        let difference = flat
            .iter()
            .zip(&changed)
            .map(|(a, b)| f64::from(a - b).powi(2))
            .sum::<f64>();
        assert!(
            !has_parameter || difference > 1e-8,
            "{} parameter must change voice",
            path.display()
        );
        eprintln!(
            "{}: finite voice processing and parameter response passed",
            path.file_name().unwrap().to_string_lossy()
        );
    }
}
