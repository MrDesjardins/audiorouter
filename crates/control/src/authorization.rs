//! Client grants, enrollment, request validation and mutation rate limits.

use super::*;

#[derive(Debug)]
pub(crate) struct MutationBucket {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Debug, Default)]
pub(crate) struct MutationRateLimiter {
    pub(crate) buckets: HashMap<String, MutationBucket>,
}

impl MutationRateLimiter {
    pub(crate) fn allow(&mut self, client_id: &str) -> Result<(), u64> {
        self.allow_at(client_id, Instant::now())
    }

    pub(crate) fn allow_at(&mut self, client_id: &str, now: Instant) -> Result<(), u64> {
        if !self.buckets.contains_key(client_id) && self.buckets.len() >= MAX_MUTATION_BUCKETS {
            self.buckets.retain(|_, bucket| {
                now.saturating_duration_since(bucket.last_refill) <= MUTATION_BUCKET_RETENTION
            });
            if self.buckets.len() >= MAX_MUTATION_BUCKETS {
                return Err(1_000);
            }
        }
        let bucket = self
            .buckets
            .entry(client_id.to_owned())
            .or_insert_with(|| MutationBucket {
                tokens: MUTATION_BURST,
                last_refill: now,
            });
        let elapsed = now
            .saturating_duration_since(bucket.last_refill)
            .as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * MUTATION_RATE_PER_SECOND).min(MUTATION_BURST);
        bucket.last_refill = now;
        if bucket.tokens < 1.0 {
            let retry_after_ms =
                (((1.0 - bucket.tokens) / MUTATION_RATE_PER_SECOND) * 1000.0).ceil() as u64;
            return Err(retry_after_ms.max(1));
        }
        bucket.tokens -= 1.0;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClientGrant {
    scopes: std::collections::HashSet<PermissionScope>,
    /// Only the local desktop shell: the user can consent once (in the app
    /// window) to let it open audio devices on Play, which adds device
    /// administration to this grant. CLI, MCP and remote grants never do.
    device_consent: bool,
}

pub(crate) fn caller_can_restart_devices(grant: &ClientGrant, consent: bool) -> bool {
    grant.allows(PermissionScope::SessionControl)
        && (grant.allows(PermissionScope::DeviceAdministration)
            || (grant.accepts_device_consent() && consent))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientRole {
    Observer,
    Editor,
    Operator,
}

impl ClientGrant {
    pub fn read_only() -> Self {
        Self::with_scopes([PermissionScope::Read])
    }

    pub fn with_scopes(scopes: impl IntoIterator<Item = PermissionScope>) -> Self {
        Self {
            scopes: scopes.into_iter().collect(),
            device_consent: false,
        }
    }

    /// Whether the user's in-app consent can add device administration.
    pub fn accepts_device_consent(&self) -> bool {
        self.device_consent
    }

    /// Map an enrolled role to the narrowest built-in grant for that role.
    /// Capture, recording, and device administration are never implied by these
    /// convenience roles and require a separately constructed explicit grant.
    pub fn for_role(role: ClientRole) -> Self {
        match role {
            ClientRole::Observer => Self::read_only(),
            ClientRole::Editor => {
                Self::with_scopes([PermissionScope::Read, PermissionScope::GraphWrite])
            }
            ClientRole::Operator => Self::with_scopes([
                PermissionScope::Read,
                PermissionScope::GraphWrite,
                PermissionScope::SessionControl,
            ]),
        }
    }

    /// Grant the desktop shell's explicitly local user surface the startup
    /// capability and permission to create explicitly requested recordings
    /// in approved roots. Record does not authorize opening capture devices;
    /// this is not an enrolled role and must not be used for remote, MCP, or
    /// CLI clients. PluginScan (explicit metadata scans of chosen folders;
    /// plugins only ever execute in isolated workers) was authorized for the
    /// local shell by the user on 2026-09-25.
    pub fn for_desktop_shell() -> Self {
        let mut grant = Self::with_scopes([
            PermissionScope::Read,
            PermissionScope::GraphWrite,
            PermissionScope::SessionControl,
            PermissionScope::Record,
            PermissionScope::StartupWrite,
            PermissionScope::PluginScan,
        ]);
        // Opening audio devices on Play needs the user's one-time consent
        // in the app (devices.setAccess), persisted in the database.
        grant.device_consent = true;
        grant
    }

    pub(crate) fn allows(&self, scope: PermissionScope) -> bool {
        self.scopes.contains(&scope)
    }
}

pub(crate) fn validate_method_params(
    method: &str,
    params: Option<&Value>,
) -> Result<(), ControlError> {
    let Some(params) = params else {
        return Ok(());
    };
    let mut value_count = 0;
    if method == "audioMedia.uploadChunk" {
        let mut bounded = params.clone();
        let encoded = bounded
            .get("dataBase64")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("dataBase64 is required".into()))?;
        if encoded.len() > AUDIO_UPLOAD_CHUNK_BYTES * 4 / 3 + 8 {
            return Err(ControlError::InvalidRequest(
                "audio upload chunk exceeds its size limit".into(),
            ));
        }
        if let Some(object) = bounded.as_object_mut() {
            object.insert("dataBase64".into(), Value::String(String::new()));
        }
        validate_control_value_budget(&bounded, 0, &mut value_count)?;
    } else {
        validate_control_value_budget(params, 0, &mut value_count)?;
    }
    let Some(object) = params.as_object() else {
        return Err(ControlError::InvalidRequest(
            "method params must be an object".into(),
        ));
    };
    let allowed: &[&str] = match method {
        "meters.reset" => &["sessionId", "nodeId"],
        "sessions.get" | "sessions.export" => &["sessionId"],
        "sessions.exportFile" => &["sessionId", "path", "replace"],
        "sessions.importFile" => &["path"],
        "sessions.importPlan" => &["session"],
        "sessions.importCommit" => &["planId", "idempotencyKey"],
        "sessions.delete" => &["sessionId", "idempotencyKey"],
        "sessions.active.get" => &[],
        "sessions.active.set" => &["sessionId", "idempotencyKey"],
        "session.start" | "sessions.start" => &["sessionId", "idempotencyKey", "candidate"],
        "session.stop" | "sessions.stop" => &["sessionId", "idempotencyKey"],
        "sessions.list" => &["cursor", "limit"],
        "sessions.create" => &["session", "idempotencyKey"],
        "sessions.duplicate" => &["sourceSessionId", "sessionId", "name", "idempotencyKey"],
        "routes.inspect" => &["sessionId", "destinationNode"],
        "graph.history" => &["sessionId", "cursor", "limit"],
        "graph.undoPlan" => &["sessionId", "baseRevision"],
        "events.subscribe" => &[
            "afterSequence",
            "backendEpoch",
            "categories",
            "limit",
            "sessionId",
        ],
        "graph.plan" => &["sessionId", "baseRevision", "candidate"],
        "graph.commit" => &[
            "planId",
            "baseRevision",
            "idempotencyKey",
            "acknowledgments",
        ],
        "system.handshake" => &["protocolVersion"],
        "system.quit" => &["idempotencyKey"],
        "system.osTransition" => &["transition", "idempotencyKey"],
        "clients.authorize" => &["clientId", "role", "idempotencyKey"],
        "clients.revoke" => &["clientId", "idempotencyKey"],
        "operations.get" => &["operationId"],
        "operations.cancel" => &["operationId", "idempotencyKey"],
        "recordings.list" => &["sessionId", "cursor", "limit"],
        "audioMedia.beginUpload" => &["fileName", "sizeBytes"],
        "audioMedia.uploadChunk" => &["uploadId", "chunkIndex", "dataBase64"],
        "audioMedia.finishUpload" => &["uploadId"],
        "audioMedia.importTemporaryRecording" => &["recordingId"],
        "audioMedia.delete" => &["mediaId"],
        "audioSources.transport" => &["sessionId", "nodeId", "action"],
        "timeShift.transport" => &["sessionId", "nodeId", "action"],
        "recorders.list" => &[],
        "recorders.create" => &[
            "sessionId",
            "nodeId",
            "recorderId",
            "format",
            "sequence",
            "channels",
            "sampleRate",
            "dither",
            "queueCapacity",
            "maximumChunksPerPass",
            "idempotencyKey",
        ],
        "recorders.arm" => &["sessionId", "nodeId", "idempotencyKey"],
        "recorders.start" | "recorders.pause" | "recorders.resume" | "recorders.split"
        | "recorders.stop" => &["sessionId", "nodeId", "frame", "idempotencyKey"],
        "recorders.startRecording" | "recorders.stopRecording" => {
            &["sessionId", "nodeId", "idempotencyKey"]
        }
        "devices.getAccess" => &[],
        "devices.setAccess" => &["allowed", "idempotencyKey"],
        "recordings.getRoot" => &[],
        "recordings.setRoot" => &["root", "create", "idempotencyKey"],
        "recordings.get" | "recordings.reveal" | "recordings.preview" => &["recordingId"],
        "recordings.recovery" => &["recordingId", "cursor", "limit"],
        "recordings.setMetadata" => &[
            "recordingId",
            "title",
            "artist",
            "comment",
            "idempotencyKey",
        ],
        "recordings.rename" => &["recordingId", "newPath", "idempotencyKey"],
        "safety.setPrivacyMute" => &["muted", "idempotencyKey"],
        "recovery.clearSafeMode" => &["idempotencyKey"],
        "recordings.removeEntry" => &["recordingId", "idempotencyKey"],
        "recordings.recycle" => &["recordingId", "confirm", "idempotencyKey"],
        "devices.list" => &["cursor", "limit"],
        "nativeEndpoints.prepare" | "nativeEndpoints.rebind" => {
            &["sessionId", "captureEndpointId", "renderEndpointId"]
        }
        "nativeOutputs.prepare" => &["sessionId", "generation", "renderEndpointIds"],
        "nativeMultiInputs.prepare" => &["sessionId", "generation", "sources"],
        "nativePaths.prepare" => &["sessionId", "generation"],
        "sessions.play" => &["sessionId", "idempotencyKey"],
        "sessions.togglePlay" => &["sessionId", "idempotencyKey"],
        "sessions.summary" => &["sessionId"],
        "meters.levels" => &["sessionId"],
        "nodes.catalog" => &[],
        "safety.togglePrivacyMute" => &["idempotencyKey"],
        "nodes.set" => &[
            "sessionId",
            "node",
            "parameters",
            "enabled",
            "bypass",
            "name",
            "idempotencyKey",
        ],
        "nodes.toggle" => &["sessionId", "node", "target", "idempotencyKey"],
        "nodes.add" => &[
            "sessionId",
            "kind",
            "name",
            "parameters",
            "between",
            "after",
            "idempotencyKey",
        ],
        "nodes.remove" => &["sessionId", "node", "bridge", "idempotencyKey"],
        "connections.add" => &["sessionId", "from", "to", "idempotencyKey"],
        "connections.remove" => &["sessionId", "from", "to", "idempotencyKey"],
        "nativeBridges.prepare" => &[
            "busId",
            "generation",
            "devicePath",
            "renderMappingPath",
            "captureMappingPath",
        ],
        "nativeBridges.detach" => &["busId"],
        "nativeBridges.heartbeat" => &[],
        "nativeEndpoints.detach" => &["sessionId"],
        "nativeDuplex.detach" => &["sessionId"],
        "nativeApplications.prepare" => &[
            "sessionId",
            "processId",
            "executable",
            "executablePath",
            "creationTime100ns",
            "mode",
            "renderEndpointId",
        ],
        "nativeEndpoints.pump" => &["sessionId", "generation", "maxPackets"],
        "nativeDuplex.pump" => &[
            "sessionId",
            "generation",
            "maxInputQuanta",
            "maxOutputPackets",
        ],
        "nativeRenderSources.pump" => &["sessionId", "generation", "maxQuanta"],
        "nativeMultiInputs.pump" => &["sessionId", "generation", "maxPackets"],
        "nativeMultiInputs.bindBranches" => &["sessionId", "generation", "branchNodeIds"],
        "plugins.scan" => &["directory"],
        "plugins.list" => &["directory"],
        "plugins.inventory" => &[],
        "plugins.retry" => &["directory", "idempotencyKey"],
        "plugins.inspect" => &["path"],
        "plugins.parameters" => &["path"],
        "plugins.saveState" | "plugins.closeEditor" => &["sessionId", "nodeId"],
        "plugins.openEditor" => &["sessionId", "nodeId", "parentWindow", "ownerProcessId"],
        "virtualDevices.list" => &["cursor", "limit"],
        "virtualDevices.plan" => &["operation"],
        "virtualDevices.apply" => &["planId", "idempotencyKey"],
        "virtualDevices.provision" => &["busId", "instanceId", "idempotencyKey"],
        "virtualDevices.remove" => &["busId", "idempotencyKey"],
        "virtualRoutes.list" => &[],
        "virtualRoutes.replace" => &["baseRevision", "routes", "idempotencyKey"],
        "startup.plan" => &["enabled"],
        "startup.apply" => &["planId", "idempotencyKey"],
        "diagnostics.getVerbose" => &[],
        "diagnostics.setVerbose" => &["enabled"],
        "system.describe" | "status.get" | "system.diagnostics" | "startup.get" | "apps.list"
        | "applications.list" | "nodes.types" | "nodes.describe" | "presets.list"
        | "processors.list" | "clients.list" => &[],
        "processors.response" => &["sampleRateHz", "bands", "frequenciesHz"],
        _ => return Ok(()),
    };
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(ControlError::InvalidRequest(format!(
            "unknown parameter: {field}"
        )));
    }
    Ok(())
}

pub(crate) fn validate_control_value_budget(
    value: &Value,
    depth: usize,
    value_count: &mut usize,
) -> Result<(), ControlError> {
    *value_count = value_count.checked_add(1).ok_or_else(|| {
        ControlError::InvalidRequest("method parameters exceed the value budget".into())
    })?;
    if *value_count > MAX_CONTROL_VALUE_COUNT {
        return Err(ControlError::InvalidRequest(
            "method parameters contain too many JSON values".into(),
        ));
    }
    if depth > MAX_CONTROL_VALUE_DEPTH {
        return Err(ControlError::InvalidRequest(
            "method parameters exceed the maximum JSON nesting depth".into(),
        ));
    }
    match value {
        Value::String(string) if string.len() > MAX_CONTROL_STRING_BYTES => {
            Err(ControlError::InvalidRequest(
                "method parameter string exceeds the maximum length".into(),
            ))
        }
        Value::Array(values) => values
            .iter()
            .try_for_each(|value| validate_control_value_budget(value, depth + 1, value_count)),
        Value::Object(values) => values.iter().try_for_each(|(key, value)| {
            if key.len() > MAX_CONTROL_STRING_BYTES {
                return Err(ControlError::InvalidRequest(
                    "method parameter key exceeds the maximum length".into(),
                ));
            }
            validate_control_value_budget(value, depth + 1, value_count)
        }),
        _ => Ok(()),
    }
}

pub(crate) fn role_name(role: ClientRole) -> &'static str {
    match role {
        ClientRole::Observer => "observer",
        ClientRole::Editor => "editor",
        ClientRole::Operator => "operator",
    }
}

pub(crate) fn role_from_name(name: &str) -> Option<ClientRole> {
    match name {
        "observer" => Some(ClientRole::Observer),
        "editor" => Some(ClientRole::Editor),
        "operator" => Some(ClientRole::Operator),
        _ => None,
    }
}

pub(crate) fn is_mutating_method(method: &str) -> bool {
    API_METHODS
        .iter()
        .find(|spec| spec.name == method)
        .is_some_and(|spec| spec.side_effect != audiorouter_domain::SideEffectClass::ReadOnly)
}

pub(crate) fn rate_limit_method(method: &str) -> bool {
    is_mutating_method(method)
        && !matches!(
            method,
            "nativeEndpoints.pump"
                | "nativeDuplex.pump"
                | "nativeRenderSources.pump"
                | "nativeMultiInputs.pump"
        )
}

impl ControlPlane {
    pub fn enroll_client(
        &mut self,
        client_id: impl Into<String>,
        role: ClientRole,
    ) -> Result<(), ControlError> {
        let client_id = client_id.into();
        if client_id.is_empty() {
            return Err(ControlError::InvalidRequest("client_id is required".into()));
        }
        if client_id.len() > audiorouter_domain::MAX_ENTITY_ID_BYTES {
            return Err(ControlError::InvalidRequest("client_id is too long".into()));
        }
        if self.storage.is_none()
            && !self.enrollments.contains_key(&client_id)
            && self.enrollments.len() >= audiorouter_storage::MAX_CLIENT_ENROLLMENTS
        {
            return Err(ControlError::InvalidRequest(
                "client enrollment limit reached".into(),
            ));
        }
        if let Some(storage) = &self.storage {
            storage
                .save_client_enrollment(&client_id, role_name(role))
                .map_err(storage_error)?;
        }
        self.enrollments.insert(client_id, (role, false));
        Ok(())
    }

    pub fn revoke_client(&mut self, client_id: &str) -> Result<bool, ControlError> {
        if client_id.len() > audiorouter_domain::MAX_ENTITY_ID_BYTES {
            return Err(ControlError::InvalidRequest("client_id is too long".into()));
        }
        let changed = if let Some(storage) = &self.storage {
            storage
                .revoke_client_enrollment(client_id)
                .map_err(storage_error)?
        } else {
            self.enrollments
                .get_mut(client_id)
                .map(|entry| {
                    let changed = !entry.1;
                    entry.1 = true;
                    changed
                })
                .unwrap_or(false)
        };
        if let Some(entry) = self.enrollments.get_mut(client_id) {
            entry.1 = true;
        }
        Ok(changed)
    }

    pub fn grant_for_client(&self, client_id: &str) -> Result<Option<ClientGrant>, ControlError> {
        let enrollment = self.enrollments.get(client_id).copied();
        let enrollment = match (enrollment, &self.storage) {
            (Some(value), _) => Some(value),
            (None, Some(storage)) => storage
                .load_client_enrollment(client_id)
                .map_err(storage_error)?
                .and_then(|(role, revoked)| role_from_name(&role).map(|role| (role, revoked))),
            (None, None) => None,
        };
        Ok(enrollment
            .filter(|(_, revoked)| !revoked)
            .map(|(role, _)| ClientGrant::for_role(role)))
    }

    pub(crate) fn client_records(&self) -> Result<Vec<Value>, ControlError> {
        let records = if let Some(storage) = &self.storage {
            storage.list_client_enrollments().map_err(storage_error)?
        } else {
            let mut records = self
                .enrollments
                .iter()
                .map(|(client_id, (role, revoked))| {
                    (client_id.clone(), role_name(*role).to_owned(), *revoked)
                })
                .collect::<Vec<_>>();
            records.sort_by(|left, right| left.0.cmp(&right.0));
            records
        };
        Ok(records
            .into_iter()
            .map(|(client_id, role, revoked)| {
                json!({
                    "clientId": client_id,
                    "role": role,
                    "revoked": revoked
                })
            })
            .collect())
    }

    pub(crate) fn dispatch_clients_list(&self) -> Result<Value, ControlError> {
        Ok(Value::Array(self.client_records()?))
    }

    pub(crate) fn dispatch_client_authorize(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params
            .ok_or_else(|| ControlError::InvalidRequest("clientId and role are required".into()))?;
        let client_id = params
            .get("clientId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("clientId is required".into()))?;
        let role_name_value = params
            .get("role")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("role is required".into()))?;
        let role = role_from_name(role_name_value)
            .ok_or_else(|| ControlError::InvalidRequest("unknown client role".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("clients.authorize", idempotency_key),
            Self::request_hash(&json!({ "clientId": client_id, "role": role_name_value })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        self.enroll_client(client_id, role)?;
        let result = json!({ "clientId": client_id, "role": role_name_value, "revoked": false });
        self.journal_idempotent_result(&operation.0, "clients.authorize", &operation.1, &result)?;
        Ok(result)
    }

    pub(crate) fn dispatch_client_revoke(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("clientId is required".into()))?;
        let client_id = params
            .get("clientId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("clientId is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("clients.revoke", idempotency_key),
            Self::request_hash(&json!({ "clientId": client_id })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        let changed = self.revoke_client(client_id)?;
        let result = json!({ "clientId": client_id, "revoked": true, "changed": changed });
        self.journal_idempotent_result(&operation.0, "clients.revoke", &operation.1, &result)?;
        Ok(result)
    }
}
