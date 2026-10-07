//! Reversible per-user sign-in registration for the desktop shell.
//!
//! This module deliberately owns only the HKCU Run value. It does not create
//! services, scheduled tasks, machine-wide entries, or audio configuration.

#[cfg(windows)]
mod windows_registry {
    use std::path::Path;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE,
        REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE,
    };

    const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "AudioRouter";
    const MAX_STARTUP_VALUE_BYTES: u32 = 32 * 1024;

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn validate_executable(path: &Path) -> Result<Vec<u16>, String> {
        if !path.is_absolute() {
            return Err("startup executable must be an absolute path".into());
        }
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|error| format!("startup executable inspection failed: {error}"))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("startup executable must be a regular non-link file".into());
        }
        let text = path
            .to_str()
            .ok_or_else(|| "startup executable path is not valid UTF-8".to_owned())?;
        Ok(wide(text))
    }

    /// Ours: the bare or quoted path (earlier versions), or the quoted path
    /// with `--tray` (current). Any other argument is someone else's value.
    fn registration_matches(value: &str, executable: &str) -> bool {
        let command = value
            .strip_suffix(super::TRAY_ARGUMENT)
            .and_then(|command| command.strip_suffix(' '))
            .filter(|command| command.starts_with('"'))
            .unwrap_or(value);
        let unquoted = command
            .strip_prefix('"')
            .and_then(|candidate| candidate.strip_suffix('"'))
            .unwrap_or(command);
        unquoted.eq_ignore_ascii_case(executable)
    }

    fn value_text(key: HKEY, name: PCWSTR) -> Result<Option<String>, String> {
        let mut value_type = REG_VALUE_TYPE(0);
        let mut byte_len = 0u32;
        // SAFETY: a size query: `key` is an open key owned by the caller, `name` is
        // a NUL-terminated wide string alive for the call, no data buffer is
        // passed, and the type and length out pointers refer to live locals.
        let status = unsafe {
            RegQueryValueExW(
                key,
                name,
                None,
                Some(&mut value_type),
                None,
                Some(&mut byte_len),
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if status != ERROR_SUCCESS {
            return Err(format!("startup registry value lookup failed: {status:?}"));
        }
        if value_type != REG_SZ {
            return Err("startup registry value is not REG_SZ".into());
        }
        if byte_len == 0 || byte_len % 2 != 0 {
            return Err("startup registry value is not valid UTF-16".into());
        }
        if byte_len > MAX_STARTUP_VALUE_BYTES {
            return Err("startup registry value exceeds the bounded size".into());
        }
        let mut bytes = vec![0u8; byte_len as usize];
        // SAFETY: `bytes` is a live buffer of exactly `byte_len` bytes, the length
        // passed in, so the call writes within it and reports the written length;
        // the other pointers are as in the size query.
        let status = unsafe {
            RegQueryValueExW(
                key,
                name,
                None,
                Some(&mut value_type),
                Some(bytes.as_mut_ptr()),
                Some(&mut byte_len),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(format!("startup registry value read failed: {status:?}"));
        }
        if byte_len > bytes.len() as u32 || byte_len % 2 != 0 {
            return Err("startup registry value returned an invalid length".into());
        }
        let words = bytes[..byte_len as usize]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|word| *word != 0)
            .collect::<Vec<_>>();
        String::from_utf16(&words)
            .map(Some)
            .map_err(|_| "startup registry value is not valid UTF-16".into())
    }

    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // SAFETY: `self.0` is the key opened or created by a successful
            // registry call and owned only by this wrapper; it is closed once.
            // The close result does not affect ownership, so it is ignored.
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }

    pub fn apply(enabled: bool, executable: &Path) -> Result<(), String> {
        let executable = validate_executable(executable)?;
        let subkey = wide(RUN_SUBKEY);
        let name = wide(VALUE_NAME);
        let mut raw = HKEY::default();
        let status = if enabled {
            // SAFETY: `subkey` is a NUL-terminated wide string alive for the call, no
            // class or security attributes are passed, and `raw` is a live local that
            // receives the key, owned by `Key` below on success.
            unsafe {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(subkey.as_ptr()),
                    Some(0),
                    None,
                    REG_OPTION_NON_VOLATILE,
                    KEY_QUERY_VALUE | KEY_SET_VALUE,
                    None,
                    &mut raw,
                    None,
                )
            }
        } else {
            // SAFETY: `subkey` is a NUL-terminated wide string alive for the call and
            // `raw` is a live local that receives the key, owned by `Key` on success.
            unsafe {
                RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(subkey.as_ptr()),
                    None,
                    KEY_QUERY_VALUE | KEY_SET_VALUE,
                    &mut raw,
                )
            }
        };
        if status == ERROR_FILE_NOT_FOUND && !enabled {
            return Ok(());
        }
        if status != ERROR_SUCCESS {
            return Err(format!(
                "startup registry key {} failed: {status:?}",
                if enabled { "creation" } else { "lookup" }
            ));
        }
        let _key = Key(raw);
        let status = if enabled {
            let existing = value_text(raw, PCWSTR(name.as_ptr()))?;
            let executable_text = String::from_utf16(&executable[..executable.len() - 1])
                .map_err(|_| "startup executable path is not valid UTF-16".to_owned())?;
            if existing
                .as_deref()
                .is_some_and(|value| !registration_matches(value, &executable_text))
            {
                return Err("existing startup registration is owned by another command".into());
            }
            // Start in the tray at sign-in: quoted path plus `--tray`.
            let value = wide(&super::run_value(&executable_text));
            // SAFETY: `value` is a live Vec<u16>; its nul terminator is excluded and
            // the byte view preserves the UTF-16LE representation expected by
            // REG_SZ. The view is consumed before the Vec can move.
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    value.as_ptr().cast::<u8>(),
                    (value.len() - 1) * std::mem::size_of::<u16>(),
                )
            };
            // SAFETY: `raw` is the open key owned by `_key`, `name` is NUL-terminated,
            // and `bytes` is a live view of the REG_SZ data built above.
            unsafe { RegSetValueExW(raw, PCWSTR(name.as_ptr()), Some(0), REG_SZ, Some(bytes)) }
        } else {
            let Some(existing) = value_text(raw, PCWSTR(name.as_ptr()))? else {
                return Ok(());
            };
            let executable_text = String::from_utf16(&executable[..executable.len() - 1])
                .map_err(|_| "startup executable path is not valid UTF-16".to_owned())?;
            if !registration_matches(&existing, &executable_text) {
                return Err("startup registration is owned by another command".into());
            }
            // SAFETY: `raw` is the open key owned by `_key` and `name` is a
            // NUL-terminated wide string alive for the call.
            unsafe { RegDeleteValueW(raw, PCWSTR(name.as_ptr())) }
        };
        if !enabled && status == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        if status != ERROR_SUCCESS {
            return Err(format!("startup registry update failed: {status:?}"));
        }
        Ok(())
    }

    pub fn is_registered(executable: &Path) -> Result<bool, String> {
        let executable = validate_executable(executable)?;
        let executable_text = String::from_utf16(&executable[..executable.len() - 1])
            .map_err(|_| "startup executable path is not valid UTF-16".to_owned())?;
        let subkey = wide(RUN_SUBKEY);
        let name = wide(VALUE_NAME);
        let mut raw = HKEY::default();
        // SAFETY: `subkey` is a NUL-terminated wide string alive for the call and
        // `raw` is a live local that receives the key, owned by `Key` on success.
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                None,
                KEY_QUERY_VALUE,
                &mut raw,
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        if status != ERROR_SUCCESS {
            return Err(format!("startup registry key lookup failed: {status:?}"));
        }
        let _key = Key(raw);
        Ok(value_text(raw, PCWSTR(name.as_ptr()))?
            .as_deref()
            .is_some_and(|value| registration_matches(value, &executable_text)))
    }

    #[cfg(test)]
    mod tests {
        use super::{apply, is_registered, registration_matches};

        #[test]
        fn ownership_matches_windows_path_casing() {
            assert!(registration_matches(
                r#"C:\PROGRAM FILES\AUDIOROUTER\SHELL.EXE"#,
                r#"C:\Program Files\AudioRouter\shell.exe"#
            ));
        }

        #[test]
        fn ownership_accepts_one_whole_value_quote_pair() {
            assert!(registration_matches(
                r#""C:\Program Files\AudioRouter\shell.exe""#,
                r#"C:\Program Files\AudioRouter\shell.exe"#
            ));
            assert!(!registration_matches(
                r#""C:\Program Files\AudioRouter\shell.exe" --unexpected"#,
                r#"C:\Program Files\AudioRouter\shell.exe"#
            ));
        }

        #[test]
        fn ownership_accepts_the_tray_start_value_and_older_ones() {
            let exe = r#"C:\Program Files\AudioRouter\shell.exe"#;
            assert!(registration_matches(&super::super::run_value(exe), exe));
            assert!(
                registration_matches(exe, exe),
                "0.0.9 and earlier wrote the bare path"
            );
            assert!(
                !registration_matches(&format!("{exe} --tray"), exe),
                "--tray needs the quoted path"
            );
            assert!(!registration_matches(r#""C:\Other\shell.exe" --tray"#, exe));
        }

        #[test]
        fn opt_in_registry_round_trip_restores_an_unregistered_value() {
            if std::env::var_os("AUDIOROUTER_ALLOW_STARTUP_REGISTRY_TEST").is_none() {
                return;
            }
            let executable = std::env::current_exe().expect("startup test executable");
            assert!(!is_registered(&executable).expect("inspect startup registration"));
            struct Cleanup<'a>(&'a std::path::Path);
            impl Drop for Cleanup<'_> {
                fn drop(&mut self) {
                    let _ = apply(false, self.0);
                }
            }
            apply(true, &executable).expect("enable startup registration");
            let _cleanup = Cleanup(&executable);
            assert!(is_registered(&executable).expect("verify startup registration"));
            apply(false, &executable).expect("disable startup registration");
            assert!(!is_registered(&executable).expect("verify startup cleanup"));
        }
    }
}

#[cfg(windows)]
pub fn apply(enabled: bool, executable: &std::path::Path) -> Result<(), String> {
    windows_registry::apply(enabled, executable)
}

#[cfg(windows)]
pub fn is_registered(executable: &std::path::Path) -> Result<bool, String> {
    windows_registry::is_registered(executable)
}

#[cfg(not(windows))]
pub fn apply(_enabled: bool, _executable: &std::path::Path) -> Result<(), String> {
    Err("sign-in startup registration is only available on Windows".into())
}

#[cfg(not(windows))]
pub fn is_registered(_executable: &std::path::Path) -> Result<bool, String> {
    Err("sign-in startup registration is only available on Windows".into())
}

/// Started at sign-in: stay in the tray and build no window (no WebView).
pub const TRAY_ARGUMENT: &str = "--tray";

/// The Run value: the quoted executable (Windows paths cannot contain `"`)
/// and the tray argument.
pub fn run_value(executable: &str) -> String {
    format!(r#""{executable}" {TRAY_ARGUMENT}"#)
}

pub fn command_line(executable: &std::path::Path) -> Result<String, String> {
    if !executable.is_absolute() {
        return Err("startup executable must be an absolute path".into());
    }
    let text = executable
        .to_str()
        .ok_or_else(|| "startup executable path is not valid UTF-8".to_owned())?;
    if text.contains('"') {
        return Err("startup executable path must not contain quotes".into());
    }
    Ok(run_value(text))
}

#[cfg(test)]
mod tests {
    use super::command_line;
    use std::path::Path;

    #[test]
    fn command_line_quotes_absolute_paths_without_shell_expansion() {
        let path = if cfg!(windows) {
            r#"C:\Program Files\AudioRouter\audiorouter-shell.exe"#
        } else {
            "/opt/Audio Router/audiorouter-shell"
        };
        assert_eq!(
            command_line(Path::new(path)).unwrap(),
            format!(r#""{}" --tray"#, path)
        );
    }

    #[test]
    fn command_line_rejects_relative_paths() {
        assert!(command_line(Path::new("audiorouter-shell.exe")).is_err());
    }
}
