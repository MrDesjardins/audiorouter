//! Current-user DPAPI token storage. No plaintext credential is written to disk.
use std::path::{Path, PathBuf};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
};
use windows::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

pub fn valid(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn generate() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "Cannot securely generate API token")?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn default_path() -> Result<PathBuf, String> {
    let root = std::env::var_os("LOCALAPPDATA").ok_or("Cannot locate local API token storage")?;
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return Err("API token storage must be absolute".into());
    }
    Ok(root.join("AudioRouter").join("api-token.dpapi"))
}
fn check_path(path: &Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    if !path.is_absolute() {
        return Err("API token storage must be absolute".into());
    }
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_attributes() & 0x400 != 0 => {
                return Err("API token storage cannot use redirected paths".into())
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Cannot inspect API token storage".into()),
        }
    }
    Ok(())
}
fn crypt(bytes: &[u8], protect: bool) -> Result<Vec<u8>, String> {
    if bytes.is_empty() || bytes.len() > 4096 {
        return Err("Invalid encrypted API token".into());
    }
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: input is an immutable slice kept alive through the synchronous
    // DPAPI call, which reads but does not modify it. Output is an initialized
    // ABI blob owned by DPAPI, copied before LocalFree and freed exactly once.
    // No prompts, entropy, machine-wide key, or borrowed description are used.
    unsafe {
        let result = if protect {
            CryptProtectData(
                &input,
                w!("AudioRouter local API"),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        result.map_err(|_| "Cannot decrypt/save API token for this Windows user. Generate a new token in API settings.".to_owned())?;
        if output.cbData == 0 || output.cbData > 8192 || output.pbData.is_null() {
            let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
            return Err("Invalid encrypted API token".into());
        }
        let data = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        Ok(data)
    }
}
pub fn load_or_create(path: &Path) -> Result<String, String> {
    check_path(path)?;
    match std::fs::metadata(path) {
        Ok(meta) => {
            if !meta.is_file() || meta.len() > 4096 {
                return Err(
                    "Invalid API token storage; generate a new token in API settings".into(),
                );
            }
            let bytes = std::fs::read(path).map_err(|_| "Cannot read API token storage")?;
            let token =
                String::from_utf8(crypt(&bytes, false)?).map_err(|_| "Invalid saved API token")?;
            if !valid(&token) {
                return Err("Invalid saved API token; generate a new token in API settings".into());
            }
            Ok(token)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let token = generate()?;
            save(path, &token)?;
            Ok(token)
        }
        Err(_) => Err("Cannot read API token storage".into()),
    }
}
pub fn save(path: &Path, token: &str) -> Result<(), String> {
    use std::io::Write;
    if !valid(token) {
        return Err("Invalid API token".into());
    }
    check_path(path)?;
    let parent = path.parent().ok_or("Invalid API token path")?;
    std::fs::create_dir_all(parent).map_err(|_| "Cannot create API token storage")?;
    check_path(path)?;
    let encrypted = crypt(token.as_bytes(), true)?;
    let temporary = parent.join(format!("api-token-{}.tmp", generate()?));
    let outcome = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "Cannot write API token storage")?;
        file.write_all(&encrypted)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Cannot write API token storage")?;
        drop(file);
        let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: both absolute NUL-terminated paths live through this call.
        // The source is our uniquely created encrypted file, paths were checked
        // for reparses; replacement changes only this user's fixed token file.
        unsafe {
            MoveFileExW(
                PCWSTR(from.as_ptr()),
                PCWSTR(to.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(|_| "Cannot replace API token storage")?;
        Ok(())
    })();
    if outcome.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    outcome
}
use std::os::windows::ffi::OsStrExt;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persisted_token_survives_restart_and_explicit_rotation() {
        let dir =
            std::env::temp_dir().join(format!("audiorouter-api-token-{}", generate().unwrap()));
        let path = dir.join("api-token.dpapi");
        let first = load_or_create(&path).unwrap();
        assert_eq!(load_or_create(&path).unwrap(), first);
        assert!(!std::fs::read(&path)
            .unwrap()
            .windows(first.len())
            .any(|b| b == first.as_bytes()));
        let next = generate().unwrap();
        save(&path, &next).unwrap();
        assert_ne!(first, load_or_create(&path).unwrap());
        assert_eq!(load_or_create(&path).unwrap(), next);
        std::fs::write(&path, b"corrupt").unwrap();
        assert!(load_or_create(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"corrupt");
        save(&path, &first).unwrap();
        assert_eq!(load_or_create(&path).unwrap(), first);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
