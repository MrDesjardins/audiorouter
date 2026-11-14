//! A durable marker detects process termination that bypasses Rust panic hooks.
//! The desktop instance lock must be held before calling `begin`.
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub struct ProcessRun {
    path: PathBuf,
    pub interrupted: bool,
}

impl ProcessRun {
    pub fn begin(database: &Path) -> std::io::Result<Self> {
        let path = database.with_extension("running");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let interrupted = path.try_exists()?;
        // Contents contain no identifiers, paths, or audio. Sync before
        // starting the backend, so a native fault leaves durable evidence.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        file.write_all(b"AudioRouter running\n")?;
        file.sync_all()?;
        Ok(Self { path, interrupted })
    }

    // Deliberately no Drop cleanup: unwinding or a startup failure must leave
    // evidence. Only the shell's completed exit event calls this method.
    pub fn finish(&self) -> std::io::Result<()> {
        fs::remove_file(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_run_is_detected_until_explicit_clean_exit() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-process-run-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let database = root.join("state.sqlite");
        let first = ProcessRun::begin(&database).unwrap();
        assert!(!first.interrupted);
        drop(first); // Models termination without the completed exit event.
        let second = ProcessRun::begin(&database).unwrap();
        assert!(second.interrupted);
        second.finish().unwrap();
        let third = ProcessRun::begin(&database).unwrap();
        assert!(!third.interrupted);
        third.finish().unwrap();
        fs::remove_dir(root).unwrap();
    }
}
