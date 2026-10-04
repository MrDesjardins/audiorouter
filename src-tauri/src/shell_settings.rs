//! Desktop-shell preferences the shell needs before any window exists, kept
//! beside the database (`shell-settings.json`). Today: autoplay, which plays
//! the selected session when AudioRouter starts (for example at sign-in).
//! Unreadable or oversized content falls back to the defaults (autoplay off).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_SETTINGS_BYTES: u64 = 4 * 1024;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShellSettings {
    pub auto_play: bool,
}

pub fn path_beside(database_path: &Path) -> PathBuf {
    database_path.with_file_name("shell-settings.json")
}

pub fn load(path: &Path) -> ShellSettings {
    let Ok(metadata) = std::fs::metadata(path) else { return ShellSettings::default() };
    if !metadata.is_file() || metadata.len() > MAX_SETTINGS_BYTES {
        return ShellSettings::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Write through a temporary file so a crash never leaves half a file.
pub fn save(path: &Path, settings: &ShellSettings) -> Result<(), String> {
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, text).map_err(|error| format!("settings could not be written: {error}"))?;
    std::fs::rename(&temporary, path).map_err(|error| format!("settings could not be saved: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoplay_round_trips_and_bad_files_mean_off() {
        let folder = std::env::temp_dir().join(format!("audiorouter-shell-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let path = path_beside(&folder.join("state.sqlite"));
        assert_eq!(path.file_name().unwrap(), "shell-settings.json");
        assert_eq!(load(&path), ShellSettings::default(), "missing file: autoplay off");
        save(&path, &ShellSettings { auto_play: true }).unwrap();
        assert!(load(&path).auto_play);
        std::fs::write(&path, "{ not json").unwrap();
        assert!(!load(&path).auto_play, "corrupt file: off");
        std::fs::write(&path, format!("{{\"autoPlay\": true, \"pad\": \"{}\"}}", "x".repeat(5000))).unwrap();
        assert!(!load(&path).auto_play, "oversized file: off");
        std::fs::write(&path, "{\"autoPlay\": true, \"future\": 1}").unwrap();
        assert!(load(&path).auto_play, "unknown keys from a newer version are ignored");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
