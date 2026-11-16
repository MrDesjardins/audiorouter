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

use audiorouter_plugin_host::vst2::{Vst2Library, Vst2Role};

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
    let mut body = Vec::new();
    body.extend_from_slice(b"FxCk");
    body.extend_from_slice(&1_u32.to_be_bytes());
    body.extend_from_slice(&0x1234_5678_u32.to_be_bytes());
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
    worker
        .apply_preset(&preset, std::time::Instant::now())
        .unwrap();
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
