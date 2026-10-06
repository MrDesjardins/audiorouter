//! `audiorouter-driver-helper.exe` — elevated, short-lived driver tool
//! (spec 17 §6). Prints one JSON result and also writes it to `--result`,
//! because a non-elevated parent cannot read the stdout of a process it
//! started with `runas`.

use audiorouter_driver_helper::{parse_arguments, ExitKind, Outcome};

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let invocation = match parse_arguments(&arguments) {
        Ok(invocation) => invocation,
        Err(failure) => {
            let json = serde_json::json!({ "ok": false, "exitCode": failure.kind.code(), "error": failure.message });
            println!("{json}");
            std::process::exit(failure.kind.code());
        }
    };
    let outcome = execute(&invocation.request);
    let text = serde_json::to_string_pretty(&outcome.json).unwrap_or_else(|_| "{}".into());
    println!("{text}");
    if let Some(path) = &invocation.result_path {
        if let Err(message) = write_result(path, &text) {
            eprintln!("cannot write --result: {message}");
            if outcome.kind == ExitKind::Success {
                std::process::exit(ExitKind::InvalidRequest.code());
            }
        }
    }
    std::process::exit(outcome.kind.code());
}

#[cfg(windows)]
fn execute(request: &audiorouter_driver_helper::Request) -> Outcome {
    use audiorouter_driver_helper::{run, Context, StateStore};
    let program_data = std::env::var_os("ProgramData")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\ProgramData"));
    let state_dir = program_data.join("AudioRouter").join("driver");
    let helper_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_default();
    // 17 §9.1: test-signed packages only in a debug build with the explicit
    // opt-in variable; release builds never accept them.
    let allow_test_driver = cfg!(debug_assertions)
        && std::env::var("AUDIOROUTER_ALLOW_TEST_DRIVER").as_deref() == Ok("1");
    let context = Context {
        state: StateStore::new(&state_dir),
        log_dir: state_dir,
        helper_dir,
        allow_test_driver,
    };
    let mut platform = audiorouter_driver_helper::windows_platform::WindowsPlatform::new();
    run(request, &mut platform, &context)
}

#[cfg(not(windows))]
fn execute(_request: &audiorouter_driver_helper::Request) -> Outcome {
    Outcome {
        kind: ExitKind::InvalidRequest,
        json: serde_json::json!({ "ok": false, "exitCode": 1, "error": "the driver helper runs only on Windows" }),
    }
}

/// The result file is for the non-elevated parent: an absolute `.json` path
/// in an existing folder, never a reparse point, replaced atomically.
fn write_result(path: &std::path::Path, text: &str) -> Result<(), String> {
    if !path.is_absolute() || path.extension().and_then(|e| e.to_str()) != Some("json") {
        return Err("--result must be an absolute .json path".into());
    }
    let parent = path.parent().ok_or("--result has no folder")?;
    let metadata = std::fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("--result folder must be a plain existing directory".into());
    }
    if let Ok(existing) = std::fs::symlink_metadata(path) {
        if !existing.is_file() || existing.file_type().is_symlink() {
            return Err("--result must not be a link or directory".into());
        }
    }
    let temporary = parent.join(format!(".helper-result.{}.tmp", std::process::id()));
    std::fs::write(&temporary, text)
        .and_then(|_| std::fs::rename(&temporary, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&temporary);
            e.to_string()
        })
}
