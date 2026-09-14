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

    fn value_text(key: HKEY, name: PCWSTR) -> Result<Option<String>, String> {
        let mut value_type = REG_VALUE_TYPE(0);
        let mut byte_len = 0u32;
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
            let existing = value_text(raw, PCWSTR(name.as_ptr()))?;
            let executable_text = String::from_utf16(&executable[..executable.len() - 1])
                .map_err(|_| "startup executable path is not valid UTF-16".to_owned())?;
            if existing
                .as_deref()
                .is_some_and(|value| value != executable_text)
            {
                return Err("existing startup registration is owned by another command".into());
            }
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
            let Some(existing) = value_text(raw, PCWSTR(name.as_ptr()))? else {
                return Ok(());
            };
            let executable_text = String::from_utf16(&executable[..executable.len() - 1])
                .map_err(|_| "startup executable path is not valid UTF-16".to_owned())?;
            if existing != executable_text {
                return Err("startup registration is owned by another command".into());
            }
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
        Ok(value_text(raw, PCWSTR(name.as_ptr()))?.as_deref() == Some(executable_text.as_str()))
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
