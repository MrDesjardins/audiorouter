//! Elevated, shell-only entry points for AudioRouter cable management.
//!
//! The control backend, MCP and HTTP adapters never call these functions.

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const HELPER_NAME: &str = "audiorouter-driver-helper.exe";
const RESULT_WAIT_MS: u32 = 10 * 60 * 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CableAction {
    Install,
    Update,
    Repair,
    Remove,
    SetCount(u8),
    SetLowLatency(bool),
    Configure(Vec<(u8, String)>),
}

impl CableAction {
    fn arguments(&self, package: &Path, result: &Path) -> Result<Vec<String>, String> {
        let mut args = match self {
            Self::Install | Self::Update | Self::Repair => {
                let operation = match self {
                    Self::Install => "install",
                    Self::Update => "update",
                    Self::Repair => "repair",
                    _ => unreachable!(),
                };
                vec![
                    operation.to_owned(),
                    "--package".into(),
                    package.to_string_lossy().into_owned(),
                ]
            }
            Self::Remove => vec!["remove".into()],
            Self::SetCount(count) if (1..=8).contains(count) => {
                vec!["set-cables".into(), "--count".into(), count.to_string()]
            }
            Self::SetCount(_) => return Err("Cable count must be between 1 and 8.".into()),
            Self::SetLowLatency(enabled) => vec![
                "configure".into(),
                "--param".into(),
                format!("DefaultPeriodFrames={}", if *enabled { 128 } else { 480 }),
            ],
            Self::Configure(names) => {
                if names.is_empty() {
                    return Err("Choose at least one cable name to save.".into());
                }
                let mut args = vec!["configure".into()];
                let mut seen = std::collections::BTreeSet::new();
                for (index, name) in names {
                    if !(1..=8).contains(index) || !seen.insert(*index) {
                        return Err("Cable name selection is invalid.".into());
                    }
                    let length = name.chars().count();
                    if length == 0
                        || length > 48
                        || name
                            .chars()
                            .any(|c| c.is_control() || matches!(c, '(' | ')'))
                    {
                        return Err("Cable names must be 1–48 printable characters and cannot contain parentheses.".into());
                    }
                    args.extend(["--name".into(), format!("{index}={name}")]);
                }
                args
            }
        };
        args.extend(["--result".into(), result.to_string_lossy().into_owned()]);
        Ok(args)
    }
}

pub fn resolve_package_paths(resource_dir: &Path, executable_dir: &Path) -> (PathBuf, PathBuf) {
    let helper = resource_dir.join(HELPER_NAME);
    let package = resource_dir.join("driver");
    if helper.is_file() && package.is_dir() {
        (helper, package)
    } else {
        (
            executable_dir.join(HELPER_NAME),
            executable_dir.join("driver"),
        )
    }
}

#[cfg(windows)]
pub fn run_elevated(
    app: &tauri::AppHandle,
    action: CableAction,
) -> Result<serde_json::Value, String> {
    use tauri::Manager;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe = std::env::current_exe().map_err(|e| format!("Cannot locate AudioRouter: {e}"))?;
    let exe_dir = exe
        .parent()
        .ok_or("AudioRouter has no installation folder")?;
    let resources = app
        .path()
        .resource_dir()
        .unwrap_or_else(|_| exe_dir.to_path_buf());
    let (helper, package) = resolve_package_paths(&resources, exe_dir);
    if !helper.is_file() {
        return Err(
            "Cable management is unavailable because the AudioRouter driver helper is missing."
                .into(),
        );
    }
    if matches!(
        &action,
        CableAction::Install | CableAction::Update | CableAction::Repair
    ) && !package.is_dir()
    {
        return Err("The AudioRouter driver package is missing from this installation.".into());
    }

    let mut random = [0u8; 16];
    getrandom::fill(&mut random)
        .map_err(|e| format!("Cannot create a private helper result name: {e}"))?;
    let result = std::env::temp_dir().join(format!(
        "audiorouter-cable-{}-{random:02x?}.json",
        std::process::id()
    ));
    let args = action.arguments(&package, &result)?;
    let parameters = args
        .iter()
        .map(|arg| quote_windows_argument(arg))
        .collect::<Vec<_>>()
        .join(" ");
    let helper_wide = helper
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let directory_wide = helper
        .parent()
        .unwrap_or(exe_dir)
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let parameters_wide = parameters.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(helper_wide.as_ptr()),
        lpParameters: PCWSTR(parameters_wide.as_ptr()),
        lpDirectory: PCWSTR(directory_wide.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: all UTF-16 arguments are NUL terminated and remain alive through
    // ShellExecuteExW. The helper path is resolved only beside the app/resources;
    // ShellExecuteExW receives no PATH lookup and only this fixed helper is elevated.
    unsafe { ShellExecuteExW(&mut info) }
        .map_err(|e| format!("Windows could not start the cable helper: {e}"))?;
    if info.hProcess.is_invalid() {
        return Err("Windows did not return the cable helper process handle.".into());
    }
    // SAFETY: hProcess is the process handle returned by ShellExecuteExW and is
    // closed exactly once below, including the timeout and error paths.
    let wait = unsafe { WaitForSingleObject(info.hProcess, RESULT_WAIT_MS) };
    if wait != WAIT_OBJECT_0 {
        // SAFETY: this is the process handle returned above and is not used again.
        let _ = unsafe { CloseHandle(info.hProcess) };
        return Err("The cable operation did not finish within 10 minutes.".into());
    }
    let mut exit_code = 0u32;
    // SAFETY: hProcess is signaled and valid; exit_code is a writable u32.
    let exit_result = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    // SAFETY: this is the process handle returned above and is not used again.
    let _ = unsafe { CloseHandle(info.hProcess) };
    exit_result.map_err(|e| format!("Cannot read the cable helper result: {e}"))?;
    let text = std::fs::read_to_string(&result)
        .map_err(|e| format!("The cable helper did not return its result: {e}"));
    let _ = std::fs::remove_file(&result);
    let value: serde_json::Value = serde_json::from_str(&text?)
        .map_err(|e| format!("The cable helper returned invalid JSON: {e}"))?;
    if exit_code != 0 || value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(value
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Cable operation failed.")
            .to_owned());
    }
    Ok(value)
}

#[cfg(not(windows))]
pub fn run_elevated(_: &tauri::AppHandle, _: CableAction) -> Result<serde_json::Value, String> {
    Err("AudioRouter cable management is available only on Windows.".into())
}

/// Whether this installation ships the driver helper and package. App-only
/// releases do not, so the UI keeps cable management disabled and explains why.
#[tauri::command]
pub fn virtual_cable_management_available(app: tauri::AppHandle) -> bool {
    use tauri::Manager;
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(exe_dir) = exe.parent() else {
        return false;
    };
    let resources = app
        .path()
        .resource_dir()
        .unwrap_or_else(|_| exe_dir.to_path_buf());
    let (helper, package) = resolve_package_paths(&resources, exe_dir);
    helper.is_file() && package.is_dir()
}

#[tauri::command]
pub fn manage_virtual_cable(
    app: tauri::AppHandle,
    operation: String,
    count: Option<u8>,
    names: Option<Vec<(u8, String)>>,
) -> Result<serde_json::Value, String> {
    let action = action_from_request(&operation, count, names)?;
    run_elevated(&app, action)
}

fn action_from_request(
    operation: &str,
    count: Option<u8>,
    names: Option<Vec<(u8, String)>>,
) -> Result<CableAction, String> {
    match (operation, count, names) {
        ("install", None, None) => Ok(CableAction::Install),
        ("update", None, None) => Ok(CableAction::Update),
        ("repair", None, None) => Ok(CableAction::Repair),
        ("remove", None, None) => Ok(CableAction::Remove),
        ("setCount", Some(count), None) => Ok(CableAction::SetCount(count)),
        ("setLowLatencyOn", None, None) => Ok(CableAction::SetLowLatency(true)),
        ("setLowLatencyOff", None, None) => Ok(CableAction::SetLowLatency(false)),
        ("configure", None, Some(names)) => Ok(CableAction::Configure(names)),
        ("setCount", None, _) => Err("Cable count is required.".into()),
        ("configure", _, None) => Err("Cable names are required.".into()),
        (
            "install" | "update" | "repair" | "remove" | "setCount" | "configure"
            | "setLowLatencyOn" | "setLowLatencyOff",
            _,
            _,
        ) => Err("The cable operation received unexpected arguments.".into()),
        _ => Err("Unsupported cable operation.".into()),
    }
}

fn quote_windows_argument(value: &str) -> String {
    if !value.is_empty() && !value.chars().any(|c| c.is_whitespace() || c == '"') {
        return value.to_owned();
    }
    let mut quoted = String::from("\"");
    let mut slashes = 0usize;
    for ch in value.chars() {
        match ch {
            '\\' => slashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(slashes * 2 + 1));
                quoted.push('"');
                slashes = 0;
            }
            _ => {
                quoted.push_str(&"\\".repeat(slashes));
                slashes = 0;
                quoted.push(ch);
            }
        }
    }
    quoted.push_str(&"\\".repeat(slashes * 2));
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn management_arguments_are_fixed_and_bounded() {
        let package = Path::new("C:\\Program Files\\AudioRouter\\driver");
        let result = Path::new("C:\\Users\\user\\AppData\\Local\\Temp\\result.json");
        assert_eq!(
            CableAction::Install.arguments(package, result).unwrap()[0],
            "install"
        );
        assert_eq!(
            CableAction::SetCount(8).arguments(package, result).unwrap()[2],
            "8"
        );
        assert!(CableAction::SetLowLatency(true)
            .arguments(package, result)
            .unwrap()
            .windows(2)
            .any(|pair| pair[0] == "--param" && pair[1] == "DefaultPeriodFrames=128"));
        assert!(CableAction::SetLowLatency(false)
            .arguments(package, result)
            .unwrap()
            .windows(2)
            .any(|pair| pair[0] == "--param" && pair[1] == "DefaultPeriodFrames=480"));
        assert!(CableAction::SetCount(0).arguments(package, result).is_err());
        assert!(
            CableAction::Configure(vec![(1, "Game".into()), (1, "Other".into())])
                .arguments(package, result)
                .is_err()
        );
        assert!(CableAction::Configure(vec![(1, "Bad (name)".into())])
            .arguments(package, result)
            .is_err());
    }

    #[test]
    fn shell_operations_reject_unexpected_arguments() {
        assert_eq!(
            action_from_request("install", None, None),
            Ok(CableAction::Install)
        );
        assert!(action_from_request("install", Some(3), None).is_err());
        assert!(action_from_request("remove", None, Some(Vec::new())).is_err());
        assert!(action_from_request("setCount", None, None).is_err());
        assert!(action_from_request("unknown", None, None).is_err());
    }

    #[test]
    fn windows_arguments_escape_quotes_and_trailing_slashes() {
        assert_eq!(quote_windows_argument("plain"), "plain");
        assert_eq!(
            quote_windows_argument("C:\\Program Files\\AudioRouter\\"),
            "\"C:\\Program Files\\AudioRouter\\\\\""
        );
        assert_eq!(quote_windows_argument("a\"b"), "\"a\\\"b\"");
    }
}
