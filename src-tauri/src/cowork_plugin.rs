//! Local Cowork adapter: configuration only, never bundled executables or secrets.
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

pub(crate) const CLIENT_ID: &str = "audiorouter-cowork-local";

pub(crate) fn archive(cli: &Path, database: &Path, pipe: &str) -> Result<Vec<u8>, String> {
    let manifest = serde_json::json!({
        "name": "audiorouter",
        "version": "1.0.0",
        "author": {"name": "Patrick Desjardins"},
        "license": "GPL-3.0-only",
        "description": "Inspect and manage AudioRouter through its enrolled local MCP client.",
        "mcpServers": {
            "audiorouter": {
                "command": cli,
                "args": ["mcp", "serve", "--client-id", CLIENT_ID,
                    "--database", database.to_string_lossy(), "--pipe", pipe]
            }
        }
    });
    let manifest = serde_json::to_vec_pretty(&manifest)
        .map_err(|_| "Could not prepare the Cowork plugin configuration.")?;
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, contents) in [
        (".claude-plugin/plugin.json", manifest.as_slice()),
        ("README.md", README.as_bytes()),
    ] {
        zip.start_file(name, options)
            .map_err(|_| "Could not create the Cowork plugin.")?;
        zip.write_all(contents)
            .map_err(|_| "Could not write the Cowork plugin.")?;
    }
    Ok(zip
        .finish()
        .map_err(|_| "Could not finish the Cowork plugin.")?
        .into_inner())
}

const README: &str = "# AudioRouter for local Cowork\n\nKeep AudioRouter running. In a supported local Cowork deployment, open Customize > Plugins and upload this ZIP. This plugin is configured for the PC that exported it; export again if AudioRouter moves. Cloud Cowork tasks cannot use this local plugin.\n\nThe client audiorouter-cowork-local starts with Observer (read-only) access. For session or graph edits, the owner must choose Editor in AudioRouter's MCP > Client permissions. Playback and other privileged operations may require additional scopes. Revoke the client there to remove access, and uninstall the plugin in Cowork. No audio, keys, executables or database contents are included. Local paths in the manifest are personal configuration; do not publish this ZIP.\n";

#[cfg(windows)]
pub(crate) fn export(
    cli: &Path,
    database: &Path,
    pipe: &str,
    folder: &Path,
) -> Result<PathBuf, String> {
    use std::os::windows::fs::MetadataExt;
    if !folder.is_absolute() || !folder.is_dir() {
        return Err("Your Downloads folder is unavailable.".into());
    }
    for ancestor in folder.ancestors() {
        let metadata = std::fs::symlink_metadata(ancestor)
            .map_err(|_| "Could not inspect the Downloads folder.")?;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(
                "Choose a Downloads folder without redirected paths before exporting.".into(),
            );
        }
    }
    crate::mcp_tunnel::verify_local_mcp_for_client(cli, database, pipe, CLIENT_ID)?;
    let bytes = archive(cli, database, pipe)?;
    let mut nonce = [0_u8; 8];
    getrandom::fill(&mut nonce).map_err(|_| "Could not generate a plugin filename.")?;
    let suffix = u64::from_le_bytes(nonce);
    let path = folder.join(format!("AudioRouter-Cowork-{suffix:016x}.zip"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "Could not save the Cowork plugin in Downloads.")?;
    if file
        .write_all(&bytes)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        drop(file);
        let _ = std::fs::remove_file(&path);
        return Err("Could not finish saving the Cowork plugin.".into());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_contains_only_configuration_with_exact_paths_and_identity() {
        let cli = Path::new(r"C:\Program Files\AudioRouter\audiorouter-cli.exe");
        let db = Path::new("C:\\Users\\Renée\\AudioRouter\\main.sqlite");
        let pipe = r"\\.\pipe\audiorouter-control";
        let bytes = archive(cli, db, pipe).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert_eq!(zip.len(), 2);
        assert_eq!(
            zip.by_index(0).unwrap().name().unwrap(),
            ".claude-plugin/plugin.json"
        );
        assert_eq!(zip.by_index(1).unwrap().name().unwrap(), "README.md");
        let manifest: serde_json::Value =
            serde_json::from_reader(zip.by_name(".claude-plugin/plugin.json").unwrap()).unwrap();
        let server = &manifest["mcpServers"]["audiorouter"];
        assert_eq!(server["command"], cli.to_string_lossy().as_ref());
        assert_eq!(
            server["args"],
            serde_json::json!([
                "mcp",
                "serve",
                "--client-id",
                CLIENT_ID,
                "--database",
                db.to_string_lossy(),
                "--pipe",
                pipe
            ])
        );
        assert!(server.get("env").is_none());
    }

    #[test]
    #[ignore = "On-request artifact for the installed Claude plugin validator"]
    fn write_plugin_validation_fixture() {
        let root = PathBuf::from(std::env::var("AUDIOROUTER_COWORK_FIXTURE").unwrap());
        assert!(root.is_absolute());
        std::fs::create_dir_all(&root).unwrap();
        let bytes = archive(
            Path::new(r"C:\code\audiorouter\src-tauri\target\release\audiorouter-cli.exe"),
            Path::new(r"C:\Users\miste\AppData\Local\AudioRouter\audiorouter.sqlite"),
            r"\\.\pipe\audiorouter-control",
        )
        .unwrap();
        std::fs::write(root.join("AudioRouter-Cowork.zip"), &bytes).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        zip.extract(root.join("plugin")).unwrap();
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "Isolated Windows pipe/export fixture; requires an explicit built CLI path"]
    fn export_preflights_real_cli_and_saves_exact_plugin() {
        let cli = PathBuf::from(std::env::var_os("AUDIOROUTER_COWORK_CLI").unwrap());
        let folder =
            std::env::temp_dir().join(format!("audiorouter-cowork-export-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let database = folder.join("fixture.sqlite");
        let storage = audiorouter_storage::Storage::open(&database).unwrap();
        let mut plane = audiorouter_control::ControlPlane::with_storage("cowork-export", storage);
        plane
            .enroll_client(CLIENT_ID, audiorouter_control::ClientRole::Observer)
            .unwrap();
        drop(plane);
        let pipe = format!(r"\\.\pipe\audiorouter-cowork-export-{}", std::process::id());
        let backend = std::thread::spawn({
            let pipe = pipe.clone();
            move || {
                audiorouter_transport::serve_control_connections_as_role(
                    &pipe,
                    1,
                    audiorouter_control::ControlPlane::new("cowork-export-backend"),
                    audiorouter_control::ClientRole::Observer,
                )
                .unwrap()
            }
        });
        let path = export(&cli, &database, &pipe, &folder).unwrap();
        backend.join().unwrap();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            archive(&cli, &database, &pipe).unwrap()
        );
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("AudioRouter-Cowork-"));
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(&database).unwrap();
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", database.display()));
        }
        std::fs::remove_dir(folder).unwrap();
    }
}
