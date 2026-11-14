//! Shell-owned diagnostics remain available even when the control pipe is down.
use serde::Serialize;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Starting,
    Running,
    Restarting,
    SafeMode,
    Stopped,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Reason {
    None,
    NewerSchema,
    CorruptDatabase,
    StorageUnavailable,
    EnrollmentRevoked,
    BackendFailure,
    RepeatedFailures,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendState {
    pub phase: Phase,
    pub reason: Reason,
    pub failure_count: usize,
    pub message: &'static str,
}

impl BackendState {
    pub fn new(phase: Phase, reason: Reason, failure_count: usize) -> Self {
        let message = match reason {
            Reason::NewerSchema => "Your saved data needs a newer AudioRouter. Update AudioRouter to open it.",
            Reason::CorruptDatabase => "AudioRouter could not read your saved data. Restore a validated backup to continue.",
            Reason::StorageUnavailable => "AudioRouter cannot access its saved data. Check available disk space and folder access, then restart.",
            Reason::EnrollmentRevoked => "This user's AudioRouter access was revoked. Restore access with an authorized local administrator before reconnecting.",
            Reason::RepeatedFailures => "AudioRouter entered safe mode after repeated failures. Routes remain stopped.",
            Reason::BackendFailure => "The audio service failed. AudioRouter is attempting a bounded restart.",
            Reason::None => match phase {
                Phase::Starting => "Starting the audio service…",
                Phase::Running => "The audio service is running.",
                Phase::Restarting => "Restarting the audio service…",
                Phase::SafeMode => "AudioRouter is in safe mode. Routes remain stopped.",
                Phase::Stopped => "The audio service is stopped.",
            },
        };
        Self {
            phase,
            reason,
            failure_count,
            message,
        }
    }

    pub fn storage_failure(error: &audiorouter_storage::StorageError) -> Self {
        use audiorouter_storage::StorageError;
        let reason = match error {
            StorageError::NewerSchema { .. } => Reason::NewerSchema,
            StorageError::CorruptDatabase(_) => Reason::CorruptDatabase,
            _ => Reason::StorageUnavailable,
        };
        Self::new(Phase::Stopped, reason, 0)
    }
}

static CURRENT: Mutex<Option<BackendState>> = Mutex::new(None);

pub fn get() -> BackendState {
    CURRENT
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
        .unwrap_or_else(|| BackendState::new(Phase::Starting, Reason::None, 0))
}

pub fn set(state: BackendState) {
    *CURRENT.lock().unwrap_or_else(|error| error.into_inner()) = Some(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_schema_reports_update_without_suggesting_corruption_restore() {
        let state =
            BackendState::storage_failure(&audiorouter_storage::StorageError::NewerSchema {
                found: 99,
                supported: 4,
            });
        assert_eq!(state.phase, Phase::Stopped);
        assert_eq!(state.reason, Reason::NewerSchema);
        assert!(state.message.contains("Update AudioRouter"));
        assert!(!state.message.contains("Restore"));
        let json = serde_json::to_value(state).unwrap();
        assert_eq!(json["reason"], "newerSchema");
    }

    #[test]
    fn storage_diagnostics_never_publish_private_error_text() {
        let state = BackendState::storage_failure(
            &audiorouter_storage::StorageError::CorruptDatabase("private path".into()),
        );
        let json = serde_json::to_string(&state).unwrap();
        assert!(!json.contains("private"));
        assert_eq!(state.reason, Reason::CorruptDatabase);
    }
}
