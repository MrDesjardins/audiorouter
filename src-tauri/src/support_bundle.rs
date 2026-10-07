//! "Copy support bundle" (P2-3): one local ZIP with the app version, the
//! Windows version, the last lines of the JSONL logs and the window's client
//! diagnostics, saved in the logs folder. Nothing is uploaded.
//!
//! The logs are written without parameters, paths or audio. As a second line
//! of defence every line is passed through
//! [`audiorouter_protocol::diagnostics::redact_user_paths`] before it enters
//! the archive.

use audiorouter_protocol::diagnostics::{redact_user_paths, tail_lines};
use std::path::{Path, PathBuf};

/// Log files included, newest records last. Each is read only when it is a
/// regular file (no links) within the diagnostic size limit.
pub const BUNDLE_LOG_FILES: [&str; 3] = ["backend.jsonl", "shell.jsonl", "mcp-activity.jsonl"];
/// Lines kept from the end of each log.
pub const BUNDLE_LOG_LINES: usize = 2_000;
/// Bytes kept from the end of each log.
const BUNDLE_LOG_BYTES: usize = 2 * 1024 * 1024;
/// Largest log file read; larger files are skipped rather than loaded.
const MAX_LOG_FILE_BYTES: u64 = 8 * 1024 * 1024;
/// Client diagnostics rows and characters per row (the window keeps 80 × 320).
const MAX_CLIENT_ROWS: usize = 100;
const MAX_CLIENT_ROW_CHARS: usize = 400;

/// What goes into one bundle, gathered before anything is written.
#[derive(Debug, Default, PartialEq)]
pub struct SupportBundle {
    /// `(archive entry name, UTF-8 text)` in archive order.
    pub entries: Vec<(String, String)>,
}

/// Collect the bundle from `log_directory`. Missing or unreadable logs are
/// listed in the manifest as absent instead of failing the whole bundle.
pub fn collect(
    log_directory: &Path,
    client_diagnostics: &[String],
    os_version: Option<&str>,
    created_unix_ms: u64,
) -> SupportBundle {
    let mut entries = Vec::new();
    let mut files = Vec::new();
    for name in BUNDLE_LOG_FILES {
        let path = log_directory.join(name);
        let included = match read_bounded(&path) {
            Some(text) => {
                let tail = tail_lines(&text, BUNDLE_LOG_LINES, BUNDLE_LOG_BYTES);
                let redacted = tail
                    .lines()
                    .map(redact_user_paths)
                    .collect::<Vec<_>>()
                    .join("\n");
                let lines = tail.lines().count();
                entries.push((
                    format!("logs/{name}"),
                    if redacted.is_empty() {
                        redacted
                    } else {
                        redacted + "\n"
                    },
                ));
                serde_json::json!({ "name": name, "included": true, "lines": lines })
            }
            None => serde_json::json!({ "name": name, "included": false, "lines": 0 }),
        };
        files.push(included);
    }
    let client = client_diagnostics
        .iter()
        .take(MAX_CLIENT_ROWS)
        .map(|row| {
            let bounded = row
                .chars()
                .filter(|character| !character.is_control())
                .take(MAX_CLIENT_ROW_CHARS)
                .collect::<String>();
            redact_user_paths(&bounded)
        })
        .collect::<Vec<_>>();
    let client_rows = client.len();
    entries.push((
        "client-diagnostics.txt".into(),
        if client.is_empty() {
            String::new()
        } else {
            client.join("\n") + "\n"
        },
    ));
    let manifest = serde_json::json!({
        "app": "AudioRouter",
        "version": env!("CARGO_PKG_VERSION"),
        "buildId": option_env!("AUDIOROUTER_BUILD_ID").unwrap_or("development"),
        "os": {
            "family": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "version": os_version.map(redact_user_paths),
        },
        "createdUnixMs": created_unix_ms,
        "logs": files,
        "logLineLimit": BUNDLE_LOG_LINES,
        "clientDiagnosticRows": client_rows,
        "privacy": "Logs contain method names, outcomes, counts and request IDs only; path-like text is replaced with <path>.",
    });
    entries.insert(
        0,
        (
            "manifest.json".into(),
            serde_json::to_string_pretty(&manifest).unwrap_or_default() + "\n",
        ),
    );
    SupportBundle { entries }
}

fn read_bounded(path: &Path) -> Option<String> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_LOG_FILE_BYTES
    {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// File name of a bundle made at `created_unix_ms`.
pub fn bundle_file_name(created_unix_ms: u64) -> String {
    format!("audiorouter-support-{created_unix_ms}.zip")
}

/// Write `bundle` as a new ZIP in `directory`; never overwrites a file.
pub fn write_zip(
    directory: &Path,
    bundle: &SupportBundle,
    created_unix_ms: u64,
) -> Result<PathBuf, String> {
    use std::io::Write;
    std::fs::create_dir_all(directory)
        .map_err(|_| "Could not create the logs folder. Check access to your local app data.")?;
    let path = directory.join(bundle_file_name(created_unix_ms));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "Could not create the support bundle file. Try again in a moment.")?;
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, text) in &bundle.entries {
        archive
            .start_file(name.as_str(), options)
            .and_then(|()| archive.write_all(text.as_bytes()).map_err(Into::into))
            .map_err(|_| "Could not write the support bundle.")?;
    }
    archive
        .finish()
        .map_err(|_| "Could not finish the support bundle.")?;
    Ok(path)
}

/// "Windows 11 Pro 24H2 (build 26100.4061)" from the registry, or `None`.
#[cfg(windows)]
pub fn os_version() -> Option<String> {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
    };
    const KEY: PCWSTR = w!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
    let text = |name: PCWSTR| -> Option<String> {
        let mut buffer = [0u16; 128];
        let mut bytes = (buffer.len() * 2) as u32;
        // SAFETY: `KEY` and `name` are static NUL-terminated strings; the
        // buffer and its byte length describe the same live stack array, and
        // RRF_RT_REG_SZ makes the API NUL-terminate within that length.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                KEY,
                name,
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut bytes),
            )
        };
        (status == ERROR_SUCCESS).then(|| {
            let words = &buffer[..(bytes as usize / 2).min(buffer.len())];
            String::from_utf16_lossy(words)
                .trim_end_matches('\0')
                .chars()
                .take(64)
                .collect()
        })
    };
    let mut ubr = 0u32;
    let mut ubr_bytes = std::mem::size_of::<u32>() as u32;
    // SAFETY: as above; the output is one live, correctly sized u32.
    let ubr_status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            KEY,
            w!("UBR"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut ubr as *mut u32).cast()),
            Some(&mut ubr_bytes),
        )
    };
    let product = text(w!("ProductName"))?;
    let display = text(w!("DisplayVersion"));
    let build = text(w!("CurrentBuildNumber"));
    let mut version = product;
    if let Some(display) = display {
        version.push(' ');
        version.push_str(&display);
    }
    if let Some(build) = build {
        version.push_str(&format!(" (build {build}"));
        if ubr_status == ERROR_SUCCESS {
            version.push_str(&format!(".{ubr}"));
        }
        version.push(')');
    }
    Some(version)
}

#[cfg(not(windows))]
pub fn os_version() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_directory(label: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "audiorouter-support-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn bundle_holds_manifest_log_tails_and_client_rows_without_paths() {
        let directory = temp_directory("collect");
        let backend = (0..2_500)
            .map(|index| format!("{{\"method\":\"graph.commit\",\"requestId\":\"R{index}\"}}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(directory.join("backend.jsonl"), backend).unwrap();
        std::fs::write(
            directory.join("shell.jsonl"),
            "{\"method\":\"plugins.scan\",\"leak\":\"C:\\\\Users\\\\Ana\\\\VST3\"}\n",
        )
        .unwrap();
        let bundle = collect(
            &directory,
            &[
                "20:14:03 RPC failed: sessions.play (permissionDenied) [req K7Q2M9XD]".into(),
                "20:14:05 saved to D:\\Music\\take.wav".into(),
            ],
            Some("Windows 11 Pro 24H2 (build 26100.4061)"),
            1_234,
        );
        let names = bundle
            .entries
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "manifest.json",
                "logs/backend.jsonl",
                "logs/shell.jsonl",
                "client-diagnostics.txt"
            ]
        );
        let text = |name: &str| {
            bundle
                .entries
                .iter()
                .find(|(entry, _)| entry == name)
                .unwrap()
                .1
                .clone()
        };
        let manifest: serde_json::Value = serde_json::from_str(&text("manifest.json")).unwrap();
        assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(
            manifest["os"]["version"],
            "Windows 11 Pro 24H2 (build 26100.4061)"
        );
        assert_eq!(manifest["createdUnixMs"], 1_234);
        assert_eq!(manifest["logs"][0]["lines"], BUNDLE_LOG_LINES);
        assert_eq!(manifest["logs"][2]["included"], false);
        assert_eq!(manifest["clientDiagnosticRows"], 2);
        let backend = text("logs/backend.jsonl");
        assert_eq!(backend.lines().count(), BUNDLE_LOG_LINES);
        assert!(backend.lines().last().unwrap().contains("R2499"));
        assert!(!backend.contains("\"R499\""));
        let shell = text("logs/shell.jsonl");
        assert!(!shell.contains("Ana"), "{shell}");
        assert!(serde_json::from_str::<serde_json::Value>(shell.trim()).is_ok());
        let client = text("client-diagnostics.txt");
        assert!(client.contains("[req K7Q2M9XD]"));
        assert!(!client.contains("Music"), "{client}");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn bundle_zip_is_written_once_and_lists_every_entry() {
        let directory = temp_directory("zip");
        std::fs::write(
            directory.join("mcp-activity.jsonl"),
            "{\"tool\":\"play\"}\n",
        )
        .unwrap();
        let bundle = collect(&directory, &[], None, 42);
        let path = write_zip(&directory, &bundle, 42).unwrap();
        assert_eq!(path.file_name().unwrap(), "audiorouter-support-42.zip");
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let mut names = (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().to_owned())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(
            names,
            [
                "client-diagnostics.txt",
                "logs/mcp-activity.jsonl",
                "manifest.json"
            ]
        );
        let mut activity = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("logs/mcp-activity.jsonl").unwrap(),
            &mut activity,
        )
        .unwrap();
        assert_eq!(activity, "{\"tool\":\"play\"}\n");
        // A second bundle in the same millisecond never overwrites the first.
        assert!(write_zip(&directory, &bundle, 42).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn linked_log_files_are_not_followed() {
        let directory = temp_directory("link");
        let outside = directory.join("outside.txt");
        std::fs::write(&outside, "private").unwrap();
        std::os::unix::fs::symlink(&outside, directory.join("backend.jsonl")).unwrap();
        let bundle = collect(&directory, &[], None, 1);
        assert!(bundle
            .entries
            .iter()
            .all(|(name, text)| name != "logs/backend.jsonl" && !text.contains("private")));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
