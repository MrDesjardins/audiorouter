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

    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // Closing a successfully created/opened HKCU key is infallible
            // for ownership purposes; the operation result is already known.
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
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                Some(0),
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut raw,
                None,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(format!("startup registry key creation failed: {status:?}"));
        }
        let _key = Key(raw);
        let status = if enabled {
            // `executable` is a live Vec<u16>; its nul terminator is excluded
            // and the byte view preserves the UTF-16LE representation expected
            // by REG_SZ. The view is consumed before the Vec can move.
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    executable.as_ptr().cast::<u8>(),
                    (executable.len() - 1) * std::mem::size_of::<u16>(),
                )
            };
            unsafe { RegSetValueExW(raw, PCWSTR(name.as_ptr()), Some(0), REG_SZ, Some(bytes)) }
        } else {
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

    pub fn is_registered() -> Result<bool, String> {
        let subkey = wide(RUN_SUBKEY);
        let name = wide(VALUE_NAME);
        let mut raw = HKEY::default();
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
        let mut value_type = REG_VALUE_TYPE(0);
        let mut byte_len = 0u32;
        let status = unsafe {
            RegQueryValueExW(
                raw,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut value_type),
                None,
                Some(&mut byte_len),
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        if status != ERROR_SUCCESS {
            return Err(format!("startup registry value lookup failed: {status:?}"));
        }
        Ok(byte_len > 0)
    }
}

#[cfg(windows)]
pub fn apply(enabled: bool, executable: &std::path::Path) -> Result<(), String> {
    windows_registry::apply(enabled, executable)
}

#[cfg(windows)]
pub fn is_registered() -> Result<bool, String> {
    windows_registry::is_registered()
}

#[cfg(not(windows))]
pub fn apply(_enabled: bool, _executable: &std::path::Path) -> Result<(), String> {
    Err("sign-in startup registration is only available on Windows".into())
}

#[cfg(not(windows))]
pub fn is_registered() -> Result<bool, String> {
    Err("sign-in startup registration is only available on Windows".into())
}

pub fn command_line(executable: &std::path::Path) -> Result<String, String> {
    if !executable.is_absolute() {
        return Err("startup executable must be an absolute path".into());
    }
    let text = executable
        .to_str()
        .ok_or_else(|| "startup executable path is not valid UTF-8".to_owned())?;
    Ok(format!(r#""{}""#, text.replace('"', "\\\"")))
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
            format!(r#""{}""#, path)
        );
    }

    #[test]
    fn command_line_rejects_relative_paths() {
        assert!(command_line(Path::new("audiorouter-shell.exe")).is_err());
    }
}
