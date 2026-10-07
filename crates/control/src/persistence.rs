//! Storage glue: database access, the recording root and idempotent operation records.

use super::*;

pub(crate) fn unix_epoch_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub(crate) fn unix_epoch_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub(crate) fn allocate_timestamped_plan_id<F>(
    prefix: &str,
    timestamp_millis: u128,
    counter: &mut u64,
    is_occupied: F,
    exhausted_message: &'static str,
) -> Result<EntityId, ControlError>
where
    F: Fn(&EntityId) -> bool,
{
    loop {
        let current = *counter;
        let plan_id = EntityId::new(format!("{prefix}-{timestamp_millis}-{current}"));
        if !is_occupied(&plan_id) {
            *counter = current.saturating_add(1);
            return Ok(plan_id);
        }
        if current == u64::MAX {
            return Err(ControlError::InvalidRequest(exhausted_message.into()));
        }
        *counter += 1;
    }
}

pub(crate) fn allocate_counter_plan_id<F>(
    prefix: &str,
    counter: &mut u64,
    is_occupied: F,
    exhausted_message: &'static str,
) -> Result<EntityId, ControlError>
where
    F: Fn(&EntityId) -> bool,
{
    loop {
        let current = *counter;
        let plan_id = EntityId::new(format!("{prefix}-{current}"));
        if !is_occupied(&plan_id) {
            *counter = current.saturating_add(1);
            return Ok(plan_id);
        }
        if current == u64::MAX {
            return Err(ControlError::InvalidRequest(exhausted_message.into()));
        }
        *counter += 1;
    }
}

pub(crate) fn remaining_persisted_plan_duration(
    expires_at: i64,
    now: i64,
    maximum: Duration,
) -> Option<Duration> {
    expires_at
        .checked_sub(now)
        .filter(|remaining| *remaining > 0)
        .and_then(|remaining| u64::try_from(remaining).ok())
        .map(Duration::from_secs)
        .map(|remaining| remaining.min(maximum))
}

/// Shown when a recording starts before any folder was approved.
pub(crate) const RECORDING_ROOT_MISSING: &str = "No recording folder is set yet. Choose one under \"Recording folder\" in the Recorder's Properties (or in the Recording tab), then press Record again.";

/// A canonical Windows path without its `\\?\` prefix, as people write it.
pub(crate) fn display_path(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix("\\\\?\\").unwrap_or(&text).to_owned()
}

/// `Music\AudioRouter Recordings` in the user's profile, offered (never
/// applied) until the user approves a recording folder.
pub(crate) fn suggested_recording_root() -> Option<std::path::PathBuf> {
    let profile = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(
        std::path::PathBuf::from(profile)
            .join("Music")
            .join("AudioRouter Recordings"),
    )
}

pub(crate) fn storage_error(error: StorageError) -> ControlError {
    match error {
        StorageError::CorruptDatabase(_) => {
            ControlError::CorruptDatabase("database integrity check failed".into())
        }
        StorageError::IdempotencyConflict => ControlError::IdempotencyConflict,
        StorageError::InvalidSession(message)
        | StorageError::InvalidBundle(message)
        | StorageError::InvalidRecording(message)
        | StorageError::InvalidAudioMedia(message)
        | StorageError::InvalidPluginState(message)
        | StorageError::InvalidEnrollment(message)
        | StorageError::InvalidPlan(message)
        | StorageError::InvalidJournal(message)
        | StorageError::InvalidBackupPath(message) => ControlError::InvalidRequest(message),
        StorageError::JournalLimitReached => {
            ControlError::InvalidRequest("operation journal limit reached".into())
        }
        StorageError::DocumentTooLarge { maximum, .. } => ControlError::InvalidRequest(format!(
            "document exceeds the maximum permitted size of {maximum} bytes"
        )),
        StorageError::InvalidRecoveryTimestamp => {
            ControlError::InvalidRequest("invalid recovery timestamp".into())
        }
        StorageError::Io(_) => ControlError::Storage("storage I/O operation failed".into()),
        StorageError::Sql(_) => ControlError::Storage("database operation failed".into()),
        StorageError::Json(_) => ControlError::Storage("stored document is invalid".into()),
    }
}

impl ControlPlane {
    pub fn try_with_storage(
        build: impl Into<String>,
        storage: Storage,
    ) -> Result<Self, audiorouter_storage::StorageError> {
        // Fail closed if the durable latch cannot be read: a persistence
        // failure must never silently unmute a capture path.
        let privacy_muted = storage.load_privacy_mute()?;
        let startup_enabled = storage.load_startup_enabled()?;
        let device_access_allowed = storage.load_device_access_allowed()?;
        let recording_policy = storage
            .load_recording_root()?
            .map(RecordingPathPolicy::new)
            .transpose()
            .map_err(|error| {
                audiorouter_storage::StorageError::InvalidRecording(format!(
                    "invalid recording root: {error:?}"
                ))
            })?;
        let virtual_buses = storage.load_virtual_buses()?;
        let (virtual_bus_routes, virtual_bus_route_revision) =
            storage.load_virtual_bus_route_state()?;
        let mut persisted_sessions = Vec::new();
        let mut session_cursor = None;
        loop {
            let page = storage.list_sessions_after(
                session_cursor.as_deref(),
                audiorouter_storage::MAX_SESSION_LIST_ITEMS,
            )?;
            if page.is_empty() {
                break;
            }
            let page_len = page.len();
            session_cursor = page.last().map(|session| session.id.as_str().to_owned());
            persisted_sessions.extend(page);
            if page_len < audiorouter_storage::MAX_SESSION_LIST_ITEMS {
                break;
            }
        }
        let mut store = GraphStore::default();
        for id in storage.list_graph_plan_ids()? {
            if let Some(counter) = id
                .strip_prefix("plan-")
                .and_then(|suffix| suffix.parse::<u64>().ok())
            {
                store.advance_plan_counter(counter);
            }
        }
        for session in persisted_sessions {
            let history = storage.load_history(&session.id, 100)?;
            if history.is_empty() {
                store.insert_session(session).map_err(|error| {
                    audiorouter_storage::StorageError::InvalidSession(format!("{error:?}"))
                })?;
            } else {
                store.restore_history(history).map_err(|error| {
                    audiorouter_storage::StorageError::InvalidSession(format!("{error:?}"))
                })?;
            }
        }
        let now = unix_epoch_seconds();
        let mut virtual_bus_plans = HashMap::new();
        for (id, operation, expires_at) in storage.load_virtual_device_plans()? {
            let Some(remaining) =
                remaining_persisted_plan_duration(expires_at, now, VIRTUAL_DEVICE_PLAN_TTL)
            else {
                continue;
            };
            let operation = virtual_bus_operation_from_value(&operation).map_err(|_| {
                audiorouter_storage::StorageError::InvalidPlan(
                    "invalid persisted virtual-device plan".into(),
                )
            })?;
            virtual_bus_plans.insert(
                id,
                VirtualBusPlan {
                    operation,
                    expires_at: Instant::now() + remaining,
                },
            );
        }
        let mut startup_plans = HashMap::new();
        for (id, enabled, expires_at) in storage.load_startup_plans()? {
            let Some(remaining) =
                remaining_persisted_plan_duration(expires_at, now, VIRTUAL_DEVICE_PLAN_TTL)
            else {
                continue;
            };
            startup_plans.insert(id, (enabled, Instant::now() + remaining));
        }
        // Claim a new epoch only after every persisted state surface has been
        // read and validated successfully; failed startup must not mutate the
        // durable database while reporting an initialization error.
        let backend_epoch = storage.claim_backend_epoch()?;
        // The last selected session survives a restart (tray Play and
        // autoplay at sign-in play it); otherwise the first session.
        let active_session_id = storage
            .load_active_session_id()?
            .filter(|id| store.session(id).is_some())
            .or_else(|| {
                store
                    .sessions_after(None, 1)
                    .first()
                    .map(|session| session.id.clone())
            });
        let mut plane = Self {
            store,
            verbose_diagnostics: Default::default(),
            active_session_id,
            build: build.into(),
            runtimes: HashMap::new(),
            recorders: HashMap::new(),
            recorder_workers: HashMap::new(),
            recorder_node_workers: HashMap::new(),
            recorder_inlets: Default::default(),
            recorder_node_states: HashMap::new(),
            recorder_node_sessions: HashMap::new(),
            recording_splits: HashMap::new(),
            recording_sequence: 0,
            recording_maintained_at: None,
            recorder_node_failures: HashMap::new(),
            recording_policy,
            storage: Some(storage),
            audio_upload: None,
            next_audio_upload: 1,
            next_audio_media: 1,
            audio_file_sources: HashMap::new(),
            test_signal_sources: HashMap::new(),
            enrollments: HashMap::new(),
            events: EventLog::new(backend_epoch),
            mutation_limiter: MutationRateLimiter::default(),
            operation_outcomes: HashMap::new(),
            operation_names: HashMap::new(),
            operation_order: VecDeque::new(),
            idempotency_hashes: HashMap::new(),
            application_snapshot: None,
            application_capture_runtime: None,
            plugin_inventories: HashMap::new(),
            plugin_bridges: Default::default(),
            plugin_inventory_order: VecDeque::new(),
            privacy_muted,
            startup_enabled,
            device_access_allowed,
            recovery_tracker: CrashRecoveryTracker::default(),
            os_suspended_sessions: Vec::new(),
            os_suspended_native_sessions: Vec::new(),
            virtual_buses,
            virtual_bus_routes,
            virtual_bus_route_revision,
            virtual_bridges: VirtualBusBridgeSet::new(
                8,
                2,
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
            )
            .expect("valid default virtual bridge collection"),
            virtual_bus_plans,
            next_virtual_bus_plan: 1,
            startup_plans,
            next_startup_plan: 1,
            session_import_plans: HashMap::new(),
            next_session_import_plan: 1,
            active_idempotency_scope: None,
            active_device_restart_allowed: None,
            endpoint_monitor: None,
            pending_endpoint_changes: Vec::new(),
            audio_service: AudioServiceStats::default(),
            native_endpoint_worker: None,
            native_endpoint_session: None,
            native_endpoint_worker_secondary: None,
            native_endpoint_session_secondary: None,
            #[cfg(windows)]
            native_multi_input_worker: None,
            #[cfg(windows)]
            native_multi_input_worker_session: None,
            #[cfg(windows)]
            native_multi_input_worker_generation: None,
            native_multi_input_applied_generation: None,
            #[cfg(windows)]
            multi_input_application_sources: Vec::new(),
            #[cfg(windows)]
            native_multi_input_mono_nodes: Vec::new(),
            network_log: network_log::Sampler::default(),
            siege_round_feed: Default::default(),
            native_endpoint_taps: None,
            native_endpoint_taps_secondary: None,
            native_endpoint_rejections: 0,
            #[cfg(windows)]
            native_output_fanout: None,
            #[cfg(windows)]
            native_output_fanout_session: None,
            #[cfg(windows)]
            native_output_fanout_generation: None,
            #[cfg(windows)]
            native_capture_sink_bindings: HashMap::new(),
            #[cfg(windows)]
            native_render_source_bindings: HashMap::new(),
            #[cfg(windows)]
            native_duplex_bindings: HashMap::new(),
            #[cfg(windows)]
            native_duplex_worker: None,
            #[cfg(windows)]
            native_duplex_worker_session: None,
            #[cfg(windows)]
            native_duplex_worker_generation: None,
            #[cfg(windows)]
            native_render_source_worker: None,
            #[cfg(windows)]
            native_render_source_worker_session: None,
            #[cfg(windows)]
            native_render_source_worker_generation: None,
            #[cfg(windows)]
            native_render_source_taps: None,
            #[cfg(windows)]
            managed_software_devices:
                audiorouter_windows_audio::ManagedSoftwareDeviceInventory::default(),
        };
        plane.restore_plugin_inventories();
        Ok(plane)
    }

    pub fn with_storage(build: impl Into<String>, storage: Storage) -> Self {
        Self::try_with_storage(build, storage).unwrap_or_else(|error| {
            panic!("AudioRouter storage initialization failed closed: {error:?}")
        })
    }

    /// Configure the backend-owned recording root. The policy is validated
    /// before replacing the current in-memory policy and is persisted before
    /// the new policy becomes active.
    pub fn configure_recording_root(
        &mut self,
        root: impl AsRef<std::path::Path>,
    ) -> Result<(), ControlError> {
        let policy = RecordingPathPolicy::new(root.as_ref()).map_err(|error| {
            ControlError::InvalidRequest(format!("invalid recording root: {error:?}"))
        })?;
        if let Some(storage) = &self.storage {
            storage
                .save_recording_root(root.as_ref())
                .map_err(storage_error)?;
        }
        self.recording_policy = Some(policy);
        Ok(())
    }

    /// The approved recording folder and a suggestion for first use.
    pub(crate) fn dispatch_recording_root_get(&self) -> Result<Value, ControlError> {
        Ok(json!({
            "root": self.recording_policy.as_ref().map(|policy| display_path(policy.root())),
            "suggestedRoot": suggested_recording_root().map(|root| display_path(&root)),
        }))
    }

    /// Approve a local folder for recordings. With `create`, a missing
    /// folder is created first; the same path policy as every recording
    /// (absolute, local, no reparse point) is enforced before it is saved.
    pub(crate) fn dispatch_recording_root_set(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("root and idempotencyKey are required".into())
        })?;
        let root = params
            .get("root")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("root is required".into()))?
            .to_owned();
        let create = params
            .get("create")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("recordings.setRoot", idempotency_key),
            Self::request_hash(&json!({ "root": root, "create": create })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        let path = std::path::PathBuf::from(&root);
        if !path.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "Choose a full folder path, for example C:\\Users\\you\\Music\\AudioRouter Recordings.".into(),
            ));
        }
        if root.starts_with("\\\\") || root.starts_with("//") {
            return Err(ControlError::InvalidRequest(
                "Recordings must be saved on this computer, not on a network share.".into(),
            ));
        }
        let mut created = false;
        if create && !path.exists() {
            std::fs::create_dir_all(&path).map_err(|error| {
                ControlError::InvalidRequest(format!("Cannot create the folder: {error}"))
            })?;
            created = true;
        }
        let policy = RecordingPathPolicy::new(&path).map_err(|error| {
            ControlError::InvalidRequest(
                match error {
                    audiorouter_recording::PathPolicyError::RootUnavailable(_) => {
                        "The folder does not exist. Create it, or choose an existing folder."
                    }
                    audiorouter_recording::PathPolicyError::RootNotDirectory => {
                        "That path is a file, not a folder."
                    }
                    audiorouter_recording::PathPolicyError::RootReparsePoint => {
                        "That folder is a link to another location; choose the real folder."
                    }
                    audiorouter_recording::PathPolicyError::NetworkRoot => {
                        "Recordings must be saved on this computer, not on a network share."
                    }
                    _ => "That folder cannot be used for recordings.",
                }
                .into(),
            )
        })?;
        if let Some(storage) = &self.storage {
            storage.save_recording_root(&path).map_err(storage_error)?;
        }
        let result = json!({ "root": display_path(policy.root()), "created": created });
        self.recording_policy = Some(policy);
        self.journal_idempotent_result(&operation.0, "recordings.setRoot", &operation.1, &result)?;
        Ok(result)
    }

    pub(crate) fn scoped_idempotency_key(&self, method: &str, key: &str) -> String {
        self.active_idempotency_scope
            .as_ref()
            .map(|client| format!("{client}\0{method}\0{key}"))
            .unwrap_or_else(|| key.to_owned())
    }

    pub(crate) fn operation_lookup_keys(&self, operation_id: &str) -> Vec<String> {
        self.active_idempotency_scope
            .as_ref()
            .map(|client| {
                vec![
                    format!("{client}\0graph.commit\0{operation_id}"),
                    format!("{client}\0virtualDevices.apply\0{operation_id}"),
                    format!("{client}\0virtualDevices.provision\0{operation_id}"),
                    format!("{client}\0virtualDevices.remove\0{operation_id}"),
                    format!("{client}\0virtualRoutes.replace\0{operation_id}"),
                    format!("{client}\0recordings.setMetadata\0{operation_id}"),
                    format!("{client}\0recordings.rename\0{operation_id}"),
                    format!("{client}\0recordings.removeEntry\0{operation_id}"),
                    format!("{client}\0recordings.recycle\0{operation_id}"),
                    format!("{client}\0sessions.delete\0{operation_id}"),
                    format!("{client}\0sessions.create\0{operation_id}"),
                    format!("{client}\0sessions.duplicate\0{operation_id}"),
                    format!("{client}\0safety.setPrivacyMute\0{operation_id}"),
                    format!("{client}\0recovery.clearSafeMode\0{operation_id}"),
                    format!("{client}\0clients.authorize\0{operation_id}"),
                    format!("{client}\0clients.revoke\0{operation_id}"),
                    format!("{client}\0operations.cancel\0{operation_id}"),
                ]
            })
            .unwrap_or_else(|| vec![operation_id.to_owned()])
    }

    pub(crate) fn remember_operation_outcome(
        &mut self,
        idempotency_key: &str,
        result: Value,
        operation: &str,
        request_hash: Option<&str>,
    ) {
        if !self.operation_outcomes.contains_key(idempotency_key) {
            while self.operation_outcomes.len() >= MAX_MEMORY_OPERATION_OUTCOMES {
                let Some(oldest) = self.operation_order.pop_front() else {
                    break;
                };
                if self.operation_outcomes.remove(&oldest).is_some() {
                    self.operation_names.remove(&oldest);
                    self.idempotency_hashes.remove(&oldest);
                    break;
                }
            }
            self.operation_order.push_back(idempotency_key.to_owned());
        }
        self.operation_outcomes
            .insert(idempotency_key.to_owned(), result);
        self.operation_names
            .insert(idempotency_key.to_owned(), operation.to_owned());
        if let Some(hash) = request_hash {
            self.idempotency_hashes
                .insert(idempotency_key.to_owned(), hash.to_owned());
        } else {
            self.idempotency_hashes.remove(idempotency_key);
        }
    }

    pub(crate) fn request_hash(value: &Value) -> String {
        let mut digest = Sha256::new();
        digest.update(serde_json::to_vec(value).unwrap_or_default());
        format!("{:x}", digest.finalize())
    }

    pub(crate) fn lookup_idempotent_result(
        &self,
        key: &str,
        request_hash: &str,
    ) -> Result<Option<Value>, ControlError> {
        if let Some(result) = self.operation_outcomes.get(key) {
            if self.idempotency_hashes.get(key).map(String::as_str) != Some(request_hash) {
                return Err(ControlError::InvalidRequest(
                    "idempotency key is already used for a different request".into(),
                ));
            }
            return Ok(Some(result.clone()));
        }
        let Some(storage) = &self.storage else {
            return Ok(None);
        };
        storage
            .journal_result_checked(key, request_hash)
            .map_err(storage_error)?
            .map(|result| {
                serde_json::from_str(&result).map_err(|error| ControlError::Json(error.to_string()))
            })
            .transpose()
    }

    pub(crate) fn journal_idempotent_result(
        &mut self,
        key: &str,
        operation: &str,
        request_hash: &str,
        result: &Value,
    ) -> Result<(), ControlError> {
        if let Some(storage) = &self.storage {
            let encoded = serde_json::to_string(result)
                .map_err(|error| ControlError::Json(error.to_string()))?;
            storage
                .journal_commit_with_hash(key, operation, &encoded, 0, request_hash)
                .map_err(storage_error)?;
        }
        self.remember_operation_outcome(key, result.clone(), operation, Some(request_hash));
        Ok(())
    }

    pub(crate) fn dispatch_operation_get(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let operation_id = params
            .get("operationId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let lookup_keys = self.operation_lookup_keys(operation_id);
        if let Some(storage) = &self.storage {
            for lookup_key in &lookup_keys {
                if let Some((operation, result, revision, created_at)) = storage
                    .operation_status(lookup_key)
                    .map_err(storage_error)?
                {
                    let result: Value = serde_json::from_str(&result)
                        .map_err(|error| ControlError::Json(error.to_string()))?;
                    return Ok(json!({
                        "operationId": operation_id,
                        "operation": operation,
                        "status": "completed",
                        "durable": true,
                        "revision": revision,
                        "createdAt": created_at,
                        "result": result
                    }));
                }
            }
        }
        for lookup_key in &lookup_keys {
            if let Some(result) = self.operation_outcomes.get(lookup_key) {
                let operation = self
                    .operation_names
                    .get(lookup_key)
                    .map(String::as_str)
                    .unwrap_or("graph.commit");
                return Ok(json!({
                    "operationId": operation_id,
                    "operation": operation,
                    "status": "completed",
                    "durable": false,
                    "revision": result["revision"],
                    "createdAt": Value::Null,
                    "result": result
                }));
            }
        }
        if self.storage.is_some() {
            return Err(ControlError::InvalidRequest("operation not found".into()));
        }
        Ok(json!({
            "operationId": operation_id,
            "status": "unknown",
            "durable": false
        }))
    }

    pub(crate) fn dispatch_operation_cancel(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let operation_id = params
            .get("operationId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let request = json!({ "operationId": operation_id });
        let operation = (
            self.scoped_idempotency_key("operations.cancel", idempotency_key),
            Self::request_hash(&request),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        let lookup_keys = self.operation_lookup_keys(operation_id);
        let exists = if let Some(storage) = &self.storage {
            lookup_keys.iter().try_fold(false, |found, key| {
                Ok::<_, ControlError>(
                    found
                        || storage
                            .operation_status(key)
                            .map_err(storage_error)?
                            .is_some(),
                )
            })?
        } else {
            lookup_keys
                .iter()
                .any(|key| self.operation_outcomes.contains_key(key))
        };
        if !exists {
            return Err(ControlError::InvalidRequest("operation not found".into()));
        }
        let result = json!({
            "operationId": operation_id,
            "status": "completed",
            "cancelled": false,
            "reason": "alreadyCompleted"
        });
        self.journal_idempotent_result(&operation.0, "operations.cancel", &operation.1, &result)?;
        Ok(result)
    }
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
