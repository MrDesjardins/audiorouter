//! Opt-in discovery of a VST2 shell DLL's sub-effects.
//!
//! The fixture is repository-owned and trusted; production discovery runs in
//! the isolated worker. Run with:
//!
//! ```text
//! AUDIOROUTER_TEST_VST2_SHELL_FIXTURE=<path to ar_test_instrument.dll>
//! cargo test -p audiorouter-plugin-host --test vst2_shell_discovery -- --ignored
//! ```
#![cfg(windows)]

use audiorouter_plugin_host::vst2::{Vst2EditorThread, Vst2Library, Vst2Role};

#[test]
#[ignore = "requires rebuilt worker and repository shell fixture"]
fn real_worker_keeps_exact_shell_sub_id_and_refuses_unknown_selection() {
    use audiorouter_plugin_host::{
        inspect_binary, worker_clock_tick, SupervisedWorkerProcess, WorkerFrame,
    };
    use std::{path::PathBuf, time::Instant};
    let path = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST2_SHELL_FIXTURE").unwrap());
    let worker = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST2_WORKER").unwrap());
    let roots = [path.parent().unwrap().to_path_buf()];
    let mut identity = inspect_binary(&path, &roots).unwrap();
    let frame = || {
        WorkerFrame::new(
            1,
            worker_clock_tick().saturating_add(10_000_000),
            2,
            vec![0.5; 512],
        )
        .unwrap()
    };
    let spawn = |identity: &audiorouter_plugin_host::PluginIdentity| {
        SupervisedWorkerProcess::spawn_verified_with_sample_rate(
            &worker,
            identity,
            &roots,
            2,
            48_000,
            Instant::now(),
        )
    };
    assert!(spawn(&identity)
        .unwrap()
        .process(frame(), vec![], Instant::now())
        .is_err());
    for (code, expected) in [(b"SHC1", 0.125), (b"SHD2", 0.25), (b"SHE3", 0.375)] {
        identity.metadata.class_ids = vec![format!("{:08X}", u32::from_be_bytes(*code))];
        let mut selected = spawn(&identity).unwrap();
        assert_eq!(
            selected
                .process(frame(), vec![], Instant::now())
                .unwrap()
                .samples,
            vec![expected; 512]
        );
    }
    identity.metadata.class_ids = vec!["4E4F4E45".into()];
    assert!(spawn(&identity).is_err());

    fn preset(kind: &[u8; 4], version: u32, body: &[u8]) -> Vec<u8> {
        let mut bytes = b"CcnK".to_vec();
        bytes.extend(((16 + body.len()) as u32).to_be_bytes());
        bytes.extend(kind);
        bytes.extend(version.to_be_bytes());
        bytes.extend(b"SHD2");
        bytes.extend(1_u32.to_be_bytes());
        bytes.extend(body);
        bytes
    }
    let program = |gain: f32| {
        let mut body = 1_u32.to_be_bytes().to_vec();
        let mut name = [0; 28];
        name[..4].copy_from_slice(b"Gain");
        body.extend(name);
        body.extend(gain.to_be_bytes());
        preset(b"FxCk", 1, &body)
    };
    let mut bank_body = 1_u32.to_be_bytes().to_vec();
    bank_body.extend([0; 128]);
    bank_body.extend(program(0.75));
    identity.metadata.class_ids = vec!["53484432".into()];
    let mut selected = spawn(&identity).unwrap();
    for (index, (bytes, expected)) in [
        (program(0.5), 0.125),
        (preset(b"FxBk", 2, &bank_body), 0.1875),
    ]
    .into_iter()
    .enumerate()
    {
        selected.apply_preset(&bytes, Instant::now()).unwrap();
        let mut input = frame();
        input.sequence = index as u64 + 1;
        assert_eq!(
            selected
                .process(input, vec![], Instant::now())
                .unwrap()
                .samples,
            vec![expected; 512]
        );
    }
}

#[test]
#[ignore = "requires AUDIOROUTER_TEST_VST2_SHELL_FIXTURE (built with --features shell-fixture)"]
fn a_shell_fixture_lists_three_named_sub_plugins() {
    let path = std::env::var("AUDIOROUTER_TEST_VST2_SHELL_FIXTURE")
        .expect("set AUDIOROUTER_TEST_VST2_SHELL_FIXTURE");
    let mut library = Vst2Library::load_as(std::path::Path::new(&path), Vst2Role::Effect)
        .expect("load shell fixture");
    assert!(
        library.is_shell(),
        "the fixture must report kPlugCategShell"
    );
    let plugins = library.shell_sub_plugins().expect("enumerate sub-plugins");
    assert_eq!(
        plugins
            .iter()
            .map(|plugin| plugin.name.as_str())
            .collect::<Vec<_>>(),
        ["Shell Clean", "Shell Drive", "Shell Echo"]
    );
    assert_eq!(plugins[0].id, u32::from_be_bytes(*b"SHC1"));
    assert_eq!(plugins[1].id, u32::from_be_bytes(*b"SHD2"));
    assert_eq!(plugins[2].id, u32::from_be_bytes(*b"SHE3"));
    drop(library);
    for plugin in plugins {
        let (mut selected, _editor) = Vst2EditorThread::spawn_loading_selected(
            std::path::Path::new(&path),
            Vst2Role::Effect,
            Some(plugin.id),
        )
        .expect("load the exact fixture sub-plugin");
        assert_eq!(selected.effect_name(), plugin.name);
    }
    assert!(Vst2Library::load_shell_sub_plugin_as(
        std::path::Path::new(&path),
        Vst2Role::Effect,
        u32::from_be_bytes(*b"NONE"),
    )
    .is_err());
}

#[test]
#[ignore = "requires AUDIOROUTER_TEST_VST2_SHELL (a real shell DLL)"]
fn a_real_shell_lists_and_loads_the_selected_sub_plugin() {
    let path =
        std::env::var("AUDIOROUTER_TEST_VST2_SHELL").expect("set AUDIOROUTER_TEST_VST2_SHELL");
    let path = std::path::PathBuf::from(path);
    let mut shell = Vst2Library::load_as(&path, Vst2Role::Effect).expect("load shell");
    assert!(shell.is_shell());
    let plugins = shell.shell_sub_plugins().expect("enumerate sub-plugins");
    assert!(
        !plugins.is_empty(),
        "the shell must list at least one effect"
    );
    assert!(
        plugins.len() <= 64,
        "the enumeration must stay bounded, got {}",
        plugins.len()
    );
    let selected = plugins[0].clone();
    drop(shell);
    let mut loaded = Vst2Library::load_shell_sub_plugin_as(&path, Vst2Role::Effect, selected.id)
        .expect("load the selected sub-plugin");
    assert_eq!(
        loaded.effect_name(),
        selected.name,
        "audioMasterCurrentId must select the requested sub-plugin"
    );
}

#[test]
#[ignore = "requires AUDIOROUTER_TEST_VST2_EFFECT and AUDIOROUTER_TEST_VST2_WORKER"]
fn the_worker_protocol_lists_and_selects_real_programs() {
    use audiorouter_plugin_host::{inspect_binary, SupervisedWorkerProcess};
    let effect = std::path::PathBuf::from(
        std::env::var("AUDIOROUTER_TEST_VST2_EFFECT").expect("set AUDIOROUTER_TEST_VST2_EFFECT"),
    );
    let worker = std::path::PathBuf::from(
        std::env::var("AUDIOROUTER_TEST_VST2_WORKER").expect("set AUDIOROUTER_TEST_VST2_WORKER"),
    );
    let root = effect.parent().unwrap().to_path_buf();
    let identity = inspect_binary(&effect, std::slice::from_ref(&root)).unwrap();
    let mut worker = SupervisedWorkerProcess::spawn_verified_with_sample_rate(
        worker,
        &identity,
        std::slice::from_ref(&root),
        2,
        48_000,
        std::time::Instant::now(),
    )
    .unwrap();
    let programs = worker.programs(std::time::Instant::now()).unwrap();
    assert!(!programs.programs.is_empty());
    worker
        .select_program(programs.programs[0].id, std::time::Instant::now())
        .unwrap();
    assert!(worker
        .select_program(65_535, std::time::Instant::now())
        .is_err());
    // A bounded synthetic program preset must travel the worker protocol.
    let preset_for = |fx_id: u32| {
        let mut body = Vec::new();
        body.extend_from_slice(b"FxCk");
        body.extend_from_slice(&1_u32.to_be_bytes());
        body.extend_from_slice(&fx_id.to_be_bytes());
        body.extend_from_slice(&7_u32.to_be_bytes());
        body.extend_from_slice(&4_u32.to_be_bytes());
        body.extend_from_slice(&[0_u8; 28]);
        body.extend_from_slice(&0.5_f32.to_be_bytes());
        body.extend_from_slice(&0.25_f32.to_be_bytes());
        body.extend_from_slice(&0.75_f32.to_be_bytes());
        body.extend_from_slice(&0.5_f32.to_be_bytes());
        let mut preset = b"CcnK".to_vec();
        preset.extend_from_slice(&(body.len() as u32).to_be_bytes());
        preset.extend_from_slice(&body);
        preset
    };
    // Regression (2026-11-18): a refused preset stopped the worker, so a
    // wrong .fxp silenced the route. Another plugin's preset and a damaged
    // file are refused, and the same worker keeps answering and processing.
    let still_running = |worker: &mut SupervisedWorkerProcess, sequence: u64| {
        let frame = audiorouter_plugin_host::WorkerFrame::new(
            sequence,
            audiorouter_plugin_host::worker_clock_tick().saturating_add(10_000_000),
            2,
            vec![0.25; 256],
        )
        .unwrap();
        let processed = worker
            .process(frame, Vec::new(), std::time::Instant::now())
            .expect("the worker keeps processing after a refused preset");
        assert!(processed.samples.iter().all(|sample| sample.is_finite()));
        worker
            .select_program(programs.programs[0].id, std::time::Instant::now())
            .expect("the worker keeps answering after a refused preset");
    };
    let refused = worker
        .apply_preset(&preset_for(0x1234_5678), std::time::Instant::now())
        .expect_err("a preset for another plugin is refused");
    assert!(
        matches!(
            &refused,
            audiorouter_plugin_host::WorkerProcessError::UnsupportedFeature(code)
                if code.contains("another plugin")
        ),
        "{refused:?}"
    );
    still_running(&mut worker, 1);
    let mut damaged = preset_for(0);
    damaged[0] = b'X';
    let refused = worker
        .apply_preset(&damaged, std::time::Instant::now())
        .expect_err("a damaged preset is refused");
    assert!(
        matches!(
            refused,
            audiorouter_plugin_host::WorkerProcessError::UnsupportedFeature(_)
        ),
        "{refused:?}"
    );
    still_running(&mut worker, 2);
    // A preset that names no plugin (fx ID 0) is applied.
    worker
        .apply_preset(&preset_for(0), std::time::Instant::now())
        .unwrap();
    still_running(&mut worker, 3);
    worker.shutdown().unwrap();
}

#[test]
#[ignore = "requires AUDIOROUTER_TEST_VST2_PLUGIN (a real VST2 effect)"]
fn a_real_plugin_lists_and_selects_bounded_programs() {
    let path =
        std::env::var("AUDIOROUTER_TEST_VST2_PLUGIN").expect("set AUDIOROUTER_TEST_VST2_PLUGIN");
    let mut plugin = Vst2Library::load_as(std::path::Path::new(&path), Vst2Role::Effect)
        .expect("load VST2 effect");
    let programs = plugin.programs().expect("list programs");
    assert!(!programs.is_empty(), "a real effect should expose programs");
    assert!(programs.len() <= 256);
    for program in programs.iter().take(8) {
        // Some effects (TDR Nova) do not report the selected index back, so
        // the observable contract here is a bounded, accepted selection.
        plugin.set_program(program.id).expect("select program");
    }
    assert!(plugin.set_program(u32::MAX).is_err());
}
