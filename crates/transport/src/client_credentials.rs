//! Same-user DPAPI credentials for enrolled pipe clients. These authenticate
//! the adapter's configured identity, never a name supplied in tool arguments.
//! Hostile code already running under this Windows user can recover them.

use crate::TransportError;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::PathBuf;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
};

fn failure() -> TransportError {
    TransportError::Protocol("cannot authenticate enrolled pipe client".into())
}

fn credential_path(client_id: &str) -> Result<PathBuf, TransportError> {
    if client_id.is_empty() || client_id.len() > audiorouter_domain::MAX_ENTITY_ID_BYTES {
        return Err(failure());
    }
    let root = std::env::var_os("LOCALAPPDATA").ok_or_else(failure)?;
    let root = PathBuf::from(root).join("AudioRouter").join("pipe-clients");
    if !root.is_absolute() {
        return Err(failure());
    }
    let digest = Sha256::digest(client_id.as_bytes());
    let name = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let path = root.join(format!("{name}.dpapi"));
    check_path(&path)?;
    Ok(path)
}

fn check_path(path: &std::path::Path) -> Result<(), TransportError> {
    use std::os::windows::fs::MetadataExt;
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_attributes() & 0x400 != 0 => return Err(failure()),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(failure()),
        }
    }
    Ok(())
}

fn crypt(input: &[u8], protect: bool) -> Result<Vec<u8>, TransportError> {
    if input.is_empty() || input.len() > 4096 {
        return Err(failure());
    }
    let blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: DPAPI reads the live immutable input synchronously. Output is
    // allocated by Windows, copied only within its validated length and freed
    // exactly once with LocalFree. UI and machine-wide protection are disabled.
    unsafe {
        let result = if protect {
            CryptProtectData(
                &blob,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &blob,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        result.map_err(|_| failure())?;
        let valid = !output.pbData.is_null() && output.cbData > 0 && output.cbData <= 4096;
        let bytes = valid
            .then(|| std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec());
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        bytes.ok_or_else(failure)
    }
}

fn load(path: &std::path::Path) -> Result<String, TransportError> {
    check_path(path)?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| failure())?
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| failure())?;
    let token = String::from_utf8(crypt(&bytes, false)?).map_err(|_| failure())?;
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(failure());
    }
    Ok(token)
}

pub(crate) fn load_or_create(client_id: &str) -> Result<String, TransportError> {
    let path = credential_path(client_id)?;
    if path.try_exists().map_err(|_| failure())? {
        return load(&path);
    }
    std::fs::create_dir_all(path.parent().ok_or_else(failure)?).map_err(|_| failure())?;
    check_path(&path)?;
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| failure())?;
    let token = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let encrypted = crypt(token.as_bytes(), true)?;
    // Publish a fully written credential atomically. A racing adapter must
    // never read a partial file, and an existing corrupt credential fails closed.
    getrandom::fill(&mut bytes).map_err(|_| failure())?;
    let temporary_id = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let temporary = path.with_extension(format!("{}.{}.tmp", std::process::id(), temporary_id));
    let outcome = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| failure())?;
        file.write_all(&encrypted)
            .and_then(|()| file.sync_all())
            .map_err(|_| failure())?;
        drop(file);
        // hard_link publishes without replacing a concurrently created file.
        match std::fs::hard_link(&temporary, &path) {
            Ok(()) => Ok(token),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => load(&path),
            Err(_) => Err(failure()),
        }
    })();
    let _ = std::fs::remove_file(temporary);
    outcome
}

pub(crate) fn verify(client_id: &str, token: &str) -> bool {
    if token.len() != 64 {
        return false;
    }
    let Ok(expected) = credential_path(client_id).and_then(|path| load(&path)) else {
        return false;
    };
    token
        .bytes()
        .zip(expected.bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a real OneDrive placeholder root; metadata-only qualification"]
    fn real_onedrive_root_is_accepted_by_credential_path_policy() {
        let root = std::path::PathBuf::from(
            std::env::var_os("OneDrive").expect("OneDrive must be configured"),
        );
        check_path(&root.join("audiorouter-policy-probe.dpapi")).unwrap();
        assert!(!root.join("audiorouter-policy-probe.dpapi").exists());
    }

    #[test]
    fn credentials_are_dpapi_protected_identity_bound_and_corruption_fails_closed() {
        let id = format!(
            "credential-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let token = load_or_create(&id).unwrap();
        assert!(load_or_create(&id).unwrap() == token);
        assert!(verify(&id, &token));
        assert!(!verify(&format!("other-{id}"), &token));
        let path = credential_path(&id).unwrap();
        let encrypted = std::fs::read(&path).unwrap();
        assert!(!encrypted
            .windows(token.len())
            .any(|window| window == token.as_bytes()));
        std::fs::write(&path, b"corrupt protected credential").unwrap();
        assert!(
            load_or_create(&id).is_err(),
            "never silently rotate corrupt credentials"
        );
        assert!(!verify(&id, &token));
        std::fs::remove_file(path).unwrap();
    }
}
