//! Portable control-plane façade for M01.
//!
//! Transport, authorization, and durable storage are deliberately separate
//! follow-up layers. This façade proves that all adapters can share one domain
//! authority and that unsupported audio capabilities are discoverable.

use audiorouter_domain::{
    inspect_routes, node_registry, ApiMethodSpec, EntityId, EventLog, EventReplayError,
    FakeRuntime, GraphStore, PermissionScope, RuntimeError, RuntimeState, Session, API_METHODS,
};
use audiorouter_protocol::{
    decode_rpc_frame, encode_frame, FrameError, JsonRpcRequest, JsonRpcResponse, RpcMessage,
};
use audiorouter_storage::{GraphPlanRecord, Storage, StorageError, GRAPH_PLAN_RETENTION_SECONDS};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

const MUTATION_RATE_PER_SECOND: f64 = 20.0;
const MUTATION_BURST: f64 = 40.0;
const MAX_MEMORY_OPERATION_OUTCOMES: usize = 100;
const APPLICATION_SNAPSHOT_TTL: std::time::Duration = std::time::Duration::from_millis(100);

fn unix_epoch_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[derive(Debug)]
struct MutationBucket {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Debug, Default)]
struct MutationRateLimiter {
    buckets: HashMap<String, MutationBucket>,
}

impl MutationRateLimiter {
    fn allow(&mut self, client_id: &str) -> Result<(), u64> {
        self.allow_at(client_id, Instant::now())
    }

    fn allow_at(&mut self, client_id: &str, now: Instant) -> Result<(), u64> {
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodDescription {
    pub name: &'static str,
    pub description: &'static str,
    pub permission: audiorouter_domain::PermissionScope,
    pub side_effect: audiorouter_domain::SideEffectClass,
    pub input_schema: Value,
    pub output_schema: Value,
}

impl From<ApiMethodSpec> for MethodDescription {
    fn from(spec: ApiMethodSpec) -> Self {
        Self {
            name: spec.name,
            description: method_description(spec.name),
            permission: spec.permission,
            side_effect: spec.side_effect,
            input_schema: method_input_schema(spec.name),
            output_schema: method_output_schema(spec.name),
        }
    }
}

fn method_description(name: &str) -> &'static str {
    match name {
        "system.describe" => "Describe protocol capabilities, methods, node types, and limits.",
        "system.handshake" => "Negotiate a compatible protocol version before requests.",
        "status.get" => "Return backend, runtime, and audio availability status.",
        "system.diagnostics" => "Return a redacted backend diagnostic snapshot.",
        "clients.list" => "List enrolled local client identities and roles.",
        "clients.authorize" => "Authorize a client with an explicit built-in role.",
        "clients.revoke" => "Revoke a client enrollment without deleting its audit record.",
        "operations.get" => "Read the durable outcome of an idempotent operation.",
        "operations.cancel" => "Cancel a pending operation when it has not completed.",
        "recordings.list" => "List persisted recording metadata without touching audio files.",
        "recordings.get" => {
            "Read one persisted recording metadata resource without touching its file."
        }
        "recordings.reveal" => "Reveal a recorded file in the operating system file browser.",
        "recordings.preview" => "Inspect recording file format metadata without decoding audio.",
        "recordings.setMetadata" => {
            "Update recording metadata without changing audio content or path."
        }
        "recordings.rename" => "Rename a recording within its approved directory.",
        "safety.setPrivacyMute" => "Latch or clear process-local privacy mute for capture paths.",
        "recordings.removeEntry" => "Remove a recording library entry without deleting its file.",
        "devices.list" => "List authoritative audio endpoint descriptors.",
        "apps.list" | "applications.list" => {
            "List discoverable application identities for binding."
        }
        "nodes.types" => "List supported node types and their availability.",
        "routes.inspect" => "Inspect upstream route provenance for a destination node.",
        "graph.history" => "List bounded committed graph revisions for a session.",
        "graph.undoPlan" => "Prepare an inverse graph plan from retained history.",
        "events.subscribe" => "Replay retained state events from an optional cursor.",
        "nodes.describe" => "Describe node types, availability, and realtime cost.",
        "sessions.get" => "Return one session resource by opaque identifier.",
        "sessions.list" => "List session resources with stable cursor pagination.",
        "sessions.create" => "Create a validated stopped session resource.",
        "sessions.duplicate" => "Clone a session into a new stopped resource.",
        "sessions.delete" => "Delete a stopped session resource and its history.",
        "graph.plan" => "Validate and preview a graph candidate without mutation.",
        "graph.commit" => "Commit an unexpired graph plan with idempotent mutation.",
        "session.start" | "sessions.start" => {
            "Start a session runtime through the available backend."
        }
        "session.stop" | "sessions.stop" => {
            "Stop a session runtime and publish its lifecycle result."
        }
        _ => "Invoke an AudioRouter control-plane method.",
    }
}

fn object_schema(properties: Value, required: &[&str]) -> Value {
    let required = required
        .iter()
        .map(|value| Value::String((*value).into()))
        .collect::<Vec<_>>();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn method_input_schema(name: &str) -> Value {
    match name {
        "system.handshake" => object_schema(
            json!({
                "protocolVersion": {
                    "type": "object",
                    "properties": {
                        "major": { "type": "integer", "minimum": 0 },
                        "minor": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["major", "minor"],
                    "additionalProperties": false
                }
            }),
            &["protocolVersion"],
        ),
        "clients.authorize" => object_schema(
            json!({
                "clientId": { "type": "string", "minLength": 1 },
                "role": { "enum": ["observer", "editor", "operator"] }
            }),
            &["clientId", "role"],
        ),
        "clients.revoke" => object_schema(
            json!({ "clientId": { "type": "string", "minLength": 1 } }),
            &["clientId"],
        ),
        "operations.get" => object_schema(
            json!({ "operationId": { "type": "string", "minLength": 1 } }),
            &["operationId"],
        ),
        "operations.cancel" => object_schema(
            json!({ "operationId": { "type": "string", "minLength": 1 } }),
            &["operationId"],
        ),
        "recordings.list" => object_schema(
            json!({ "sessionId": { "type": ["string", "null"], "minLength": 1 } }),
            &[],
        ),
        "recordings.get" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1 } }),
            &["recordingId"],
        ),
        "recordings.reveal" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1 } }),
            &["recordingId"],
        ),
        "recordings.preview" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1 } }),
            &["recordingId"],
        ),
        "recordings.setMetadata" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1 },
                "title": { "type": ["string", "null"], "maxLength": 256 },
                "artist": { "type": ["string", "null"], "maxLength": 256 },
                "comment": { "type": ["string", "null"], "maxLength": 256 }
            }),
            &["recordingId"],
        ),
        "recordings.rename" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1 },
                "newPath": { "type": "string", "minLength": 1 }
            }),
            &["recordingId", "newPath"],
        ),
        "safety.setPrivacyMute" => {
            object_schema(json!({ "muted": { "type": "boolean" } }), &["muted"])
        }
        "recordings.removeEntry" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1 } }),
            &["recordingId"],
        ),
        "sessions.get" | "sessions.delete" | "session.start" | "sessions.start"
        | "session.stop" | "sessions.stop" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1 } }),
            &["sessionId"],
        ),
        "sessions.list" => object_schema(
            json!({
                "cursor": { "type": ["string", "null"] },
                "limit": { "type": "integer", "minimum": 1, "maximum": 500 }
            }),
            &[],
        ),
        "sessions.create" => {
            object_schema(json!({ "session": { "type": "object" } }), &["session"])
        }
        "sessions.duplicate" => object_schema(
            json!({
                "sourceSessionId": { "type": "string", "minLength": 1 },
                "sessionId": { "type": "string", "minLength": 1 },
                "name": { "type": ["string", "null"] }
            }),
            &["sourceSessionId", "sessionId"],
        ),
        "routes.inspect" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1 },
                "destinationNode": { "type": "string", "minLength": 1 }
            }),
            &["sessionId", "destinationNode"],
        ),
        "graph.history" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1 },
                "cursor": { "type": ["string", "null"] },
                "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
            }),
            &["sessionId"],
        ),
        "graph.undoPlan" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1 },
                "baseRevision": { "type": "integer", "minimum": 0 }
            }),
            &["sessionId", "baseRevision"],
        ),
        "events.subscribe" => object_schema(
            json!({
                "afterSequence": { "type": "integer", "minimum": 0 },
                "backendEpoch": { "type": "integer", "minimum": 0 },
                "limit": { "type": "integer", "minimum": 1, "maximum": 500 },
                "sessionId": { "type": ["string", "null"] }
            }),
            &[],
        ),
        "graph.plan" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1 },
                "baseRevision": { "type": "integer", "minimum": 0 },
                "candidate": { "type": "object" }
            }),
            &["sessionId", "baseRevision", "candidate"],
        ),
        "graph.commit" => object_schema(
            json!({
                "planId": { "type": "string", "minLength": 1 },
                "baseRevision": { "type": "integer", "minimum": 0 },
                "idempotencyKey": { "type": "string", "minLength": 1 },
                "acknowledgments": {
                    "type": ["array", "null"],
                    "items": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "maxItems": 100
                }
            }),
            &["planId", "baseRevision", "idempotencyKey"],
        ),
        _ => object_schema(json!({}), &[]),
    }
}

fn method_output_schema(name: &str) -> Value {
    match name {
        "devices.list" | "apps.list" | "applications.list" | "nodes.types" | "nodes.describe"
        | "clients.list" | "recordings.list" => {
            json!({ "type": "array" })
        }
        _ => json!({ "type": "object" }),
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ControlError {
    InvalidRequest(String),
    Store(audiorouter_domain::StoreError),
    Json(String),
    Storage(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClientGrant {
    scopes: std::collections::HashSet<PermissionScope>,
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
        }
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

    fn allows(&self, scope: PermissionScope) -> bool {
        self.scopes.contains(&scope)
    }
}

impl From<audiorouter_domain::StoreError> for ControlError {
    fn from(error: audiorouter_domain::StoreError) -> Self {
        Self::Store(error)
    }
}

pub struct ControlPlane {
    store: GraphStore,
    build: String,
    runtimes: HashMap<EntityId, FakeRuntime>,
    storage: Option<Storage>,
    enrollments: HashMap<String, (ClientRole, bool)>,
    events: EventLog,
    mutation_limiter: MutationRateLimiter,
    operation_outcomes: HashMap<String, Value>,
    application_snapshot: Option<(Instant, Value)>,
    privacy_muted: bool,
}

impl Default for ControlPlane {
    fn default() -> Self {
        Self::new("dev")
    }
}

impl ControlPlane {
    pub fn new(build: impl Into<String>) -> Self {
        Self {
            store: GraphStore::default(),
            build: build.into(),
            runtimes: HashMap::new(),
            storage: None,
            enrollments: HashMap::new(),
            events: EventLog::new(1),
            mutation_limiter: MutationRateLimiter::default(),
            operation_outcomes: HashMap::new(),
            application_snapshot: None,
            privacy_muted: false,
        }
    }

    pub fn with_storage(build: impl Into<String>, storage: Storage) -> Self {
        // Fail closed if the durable latch cannot be read: a persistence
        // failure must never silently unmute a capture path.
        let privacy_muted = storage.load_privacy_mute().unwrap_or(true);
        Self {
            store: GraphStore::default(),
            build: build.into(),
            runtimes: HashMap::new(),
            storage: Some(storage),
            enrollments: HashMap::new(),
            events: EventLog::new(1),
            mutation_limiter: MutationRateLimiter::default(),
            operation_outcomes: HashMap::new(),
            application_snapshot: None,
            privacy_muted,
        }
    }

    pub fn enroll_client(
        &mut self,
        client_id: impl Into<String>,
        role: ClientRole,
    ) -> Result<(), ControlError> {
        let client_id = client_id.into();
        if client_id.is_empty() {
            return Err(ControlError::InvalidRequest("client_id is required".into()));
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

    fn client_records(&self) -> Result<Vec<Value>, ControlError> {
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

    fn dispatch_clients_list(&self) -> Result<Value, ControlError> {
        Ok(Value::Array(self.client_records()?))
    }

    fn dispatch_client_authorize(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
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
        self.enroll_client(client_id, role)?;
        Ok(json!({ "clientId": client_id, "role": role_name_value, "revoked": false }))
    }

    fn dispatch_client_revoke(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("clientId is required".into()))?;
        let client_id = params
            .get("clientId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("clientId is required".into()))?;
        let changed = self.revoke_client(client_id)?;
        Ok(json!({ "clientId": client_id, "revoked": true, "changed": changed }))
    }

    fn dispatch_operation_get(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let operation_id = params
            .get("operationId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        if let Some(storage) = &self.storage {
            if let Some((operation, result, revision, created_at)) = storage
                .operation_status(operation_id)
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
        if let Some(result) = self.operation_outcomes.get(operation_id) {
            return Ok(json!({
                "operationId": operation_id,
                "operation": "graph.commit",
                "status": "completed",
                "durable": false,
                "revision": result["revision"],
                "createdAt": Value::Null,
                "result": result
            }));
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

    fn dispatch_operation_cancel(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let operation_id = params
            .get("operationId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let exists = if let Some(storage) = &self.storage {
            storage
                .operation_status(operation_id)
                .map_err(storage_error)?
                .is_some()
        } else {
            self.operation_outcomes.contains_key(operation_id)
        };
        if !exists {
            return Err(ControlError::InvalidRequest("operation not found".into()));
        }
        Ok(json!({
            "operationId": operation_id,
            "status": "completed",
            "cancelled": false,
            "reason": "alreadyCompleted"
        }))
    }

    pub fn insert_session(&mut self, session: Session) -> Result<(), ControlError> {
        let checkpoint = self.store.clone();
        self.store
            .insert_session(session.clone())
            .map_err(ControlError::from)?;
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_session(&session) {
                self.store = checkpoint;
                return Err(storage_error(error));
            }
        }
        self.events.append(
            session.revision,
            None,
            "session.created",
            Some(session.id.clone()),
        );
        Ok(())
    }

    pub fn create_session(&mut self, session: Session) -> Result<Value, ControlError> {
        if session.revision != 0 {
            return Err(ControlError::InvalidRequest(
                "new sessions must start at revision 0".into(),
            ));
        }
        self.insert_session(session.clone())?;
        Ok(json!({ "session": session, "state": "stopped" }))
    }

    pub fn duplicate_session(
        &mut self,
        source_id: &EntityId,
        duplicate_id: EntityId,
        name: Option<String>,
    ) -> Result<Value, ControlError> {
        self.ensure_session_loaded(source_id)?;
        let source = self.get_session(source_id)?.clone();
        if self.store.session(&duplicate_id).is_some() {
            return Err(ControlError::InvalidRequest(
                "duplicate session ID already exists".into(),
            ));
        }
        let duplicate = Session {
            id: duplicate_id,
            name: name.unwrap_or_else(|| format!("{} (copy)", source.name)),
            revision: 0,
            ..source
        };
        self.create_session(duplicate)
    }

    pub fn delete_session(&mut self, id: &EntityId) -> Result<Value, ControlError> {
        self.ensure_session_loaded(id)?;
        let session = self.get_session(id)?.clone();
        if self
            .runtimes
            .get(id)
            .map(|runtime| runtime.state() == RuntimeState::Running)
            .unwrap_or(false)
        {
            return Err(ControlError::InvalidRequest(
                "stop the session before deleting it".into(),
            ));
        }
        let checkpoint = self.store.clone();
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.delete_session(id) {
                self.store = checkpoint;
                return Err(storage_error(error));
            }
        }
        self.store.remove_session(id).map_err(ControlError::from)?;
        self.runtimes.remove(id);
        self.events
            .append(session.revision, None, "session.deleted", Some(id.clone()));
        Ok(json!({ "sessionId": id, "deleted": true }))
    }

    pub fn describe(&self) -> Value {
        let methods: Vec<MethodDescription> = API_METHODS.iter().copied().map(Into::into).collect();
        let nodes: Vec<Value> = node_registry().into_iter().map(|spec| {
            let availability = match spec.availability {
                audiorouter_domain::CapabilityAvailability::Available => json!({ "status": "available" }),
                audiorouter_domain::CapabilityAvailability::Unavailable(reason) => json!({ "status": "unavailable", "reason": reason }),
            };
            json!({ "type": format!("{}@{}", spec.kind.type_name(), spec.version), "availability": availability, "realtimeCostClass": spec.realtime_cost_class, "parameters": Self::node_parameter_schema(spec.kind) })
        }).collect();
        json!({
            "protocolVersion": { "major": 1, "minor": 0 },
            "schemaVersion": 1,
            "build": self.build,
            "methods": methods,
            "nodeTypes": nodes,
            "limits": {
                "maxNodesPerSession": audiorouter_domain::MAX_NODES_PER_SESSION,
                "maxEdgesPerSession": audiorouter_domain::MAX_EDGES_PER_SESSION,
                "maxNodesGlobal": audiorouter_domain::MAX_NODES_GLOBAL,
                "maxEdgesGlobal": audiorouter_domain::MAX_EDGES_GLOBAL,
                "maxActiveSessions": audiorouter_domain::MAX_ACTIVE_SESSIONS
            },
            "events": {
                "stateCategories": [
                    "session.created",
                    "session.deleted",
                    "graph.committed",
                    "runtime.started",
                    "runtime.activated",
                    "runtime.stopped",
                    "privacy.muteEnabled",
                    "privacy.muteDisabled"
                ],
                "meterReplay": false,
                "retention": {
                    "maxEvents": audiorouter_domain::MAX_RETAINED_EVENTS,
                    "maxAgeSeconds": 900
                }
            }
        })
    }

    fn node_parameter_schema(kind: audiorouter_domain::NodeKind) -> Value {
        match kind {
            audiorouter_domain::NodeKind::Gain => json!([{
                "name": "gainDb",
                "type": "number",
                "unit": "dB",
                "minimum": -60.0,
                "maximum": 12.0,
                "default": 0.0
            }]),
            audiorouter_domain::NodeKind::Mute => json!([{
                "name": "muted",
                "type": "boolean",
                "default": false
            }]),
            _ => json!([]),
        }
    }

    fn status_snapshot(&self) -> Result<Value, ControlError> {
        let mut active_session_ids = self
            .runtimes
            .iter()
            .filter(|(_, runtime)| runtime.state() == RuntimeState::Running)
            .map(|(id, _)| id.as_str().to_owned())
            .collect::<Vec<_>>();
        active_session_ids.sort();
        let session_count = if let Some(storage) = &self.storage {
            storage.count_sessions().map_err(storage_error)?
        } else {
            self.store.sessions(500).len()
        };
        Ok(json!({
            "build": self.build,
            "audio": "unavailable",
            "deviceDiscovery": "available",
            "reason": "M02 realtime graph engine and routing are not implemented",
            "storage": if self.storage.is_some() { "sqlite" } else { "memory" },
            "sessionCount": session_count,
            "activeSessionCount": active_session_ids.len(),
            "activeSessionIds": active_session_ids,
            "privacyMute": {
                "muted": self.privacy_muted,
                "persistence": if self.storage.is_some() { "durable" } else { "memory" },
                "audioEffect": "process-local-when-realtime-backend-is-available"
            },
            "eventCursor": {
                "backendEpoch": self.events.backend_epoch(),
                "latestSequence": self.events.latest_sequence()
            }
        }))
    }

    pub fn get_session(&self, id: &EntityId) -> Result<&Session, ControlError> {
        self.store
            .session(id)
            .ok_or(ControlError::InvalidRequest("session not found".into()))
    }

    pub fn inspect_routes(
        &self,
        session_id: &EntityId,
        destination_node: &EntityId,
    ) -> Result<Value, ControlError> {
        let session = self.get_session(session_id)?;
        serde_json::to_value(
            inspect_routes(session, destination_node).map_err(|errors| {
                ControlError::InvalidRequest(format!("invalid graph: {errors:?}"))
            })?,
        )
        .map_err(|error| ControlError::Json(error.to_string()))
    }

    pub fn graph_history(
        &self,
        session_id: &EntityId,
        limit: usize,
    ) -> Result<Value, ControlError> {
        self.graph_history_page(session_id, None, limit)
            .map(|page| page["items"].clone())
    }

    pub fn graph_history_page(
        &self,
        session_id: &EntityId,
        before_revision: Option<u64>,
        limit: usize,
    ) -> Result<Value, ControlError> {
        let limit = limit.clamp(1, 100);
        let history = if self.store.session(session_id).is_some() {
            self.store
                .history_before(session_id, before_revision, limit + 1)
        } else if let Some(storage) = &self.storage {
            storage
                .load_history_before(session_id, before_revision, limit + 1)
                .map_err(storage_error)?
        } else {
            Vec::new()
        };
        let has_more = history.len() > limit;
        let mut history = history;
        history.truncate(limit);
        let next_cursor = has_more
            .then(|| history.last().map(|session| session.revision))
            .flatten()
            .map(|revision| revision.to_string());
        Ok(json!({ "items": history, "nextCursor": next_cursor }))
    }

    pub fn graph_undo_plan(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
    ) -> Result<EntityId, ControlError> {
        self.ensure_session_loaded(session_id)?;
        if self.store.history(session_id, 2).len() < 2 {
            if let Some(storage) = &self.storage {
                let entries = storage
                    .load_history(session_id, 100)
                    .map_err(storage_error)?;
                self.store
                    .restore_history(entries)
                    .map_err(ControlError::from)?;
            }
        }
        self.store
            .undo_plan(session_id, base_revision)
            .map_err(Into::into)
    }

    pub fn sessions_list(&self, limit: usize) -> Result<Value, ControlError> {
        self.sessions_list_page(None, limit)
            .map(|page| page["items"].clone())
    }

    pub fn sessions_list_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, ControlError> {
        if !(1..=500).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        let sessions = if let Some(storage) = &self.storage {
            storage
                .list_sessions_after(cursor, limit)
                .map_err(storage_error)?
        } else {
            self.store.sessions_after(cursor, limit)
        };
        let next_cursor = (sessions.len() == limit)
            .then(|| {
                sessions
                    .last()
                    .map(|session| session.id.as_str().to_owned())
            })
            .flatten();
        Ok(json!({ "items": sessions, "nextCursor": next_cursor }))
    }

    pub fn plan_graph(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
        candidate: Session,
    ) -> Result<EntityId, ControlError> {
        let plan_id = self
            .store
            .plan_graph(session_id, base_revision, candidate.clone())
            .map_err(ControlError::from)?;
        if let Some(storage) = &self.storage {
            let expires_at = unix_epoch_seconds() + GRAPH_PLAN_RETENTION_SECONDS;
            storage
                .save_graph_plan(&GraphPlanRecord {
                    id: plan_id.as_str().to_owned(),
                    session_id: session_id.as_str().to_owned(),
                    base_revision,
                    candidate,
                    expires_at,
                })
                .map_err(storage_error)?;
        }
        Ok(plan_id)
    }

    pub fn commit_graph(
        &mut self,
        plan_id: &EntityId,
        base_revision: u64,
        idempotency_key: &str,
    ) -> Result<Value, ControlError> {
        let checkpoint = self.store.clone();
        let fingerprint = format!("graph.commit:{}:{}", plan_id.as_str(), base_revision);
        let request_hash = format!("{:x}", Sha256::digest(fingerprint.as_bytes()));
        if let Some(storage) = &self.storage {
            if let Some(result) = storage
                .journal_result_checked(idempotency_key, &request_hash)
                .map_err(storage_error)?
            {
                let mut response: Value = serde_json::from_str(&result)
                    .map_err(|error| ControlError::Json(error.to_string()))?;
                response["idempotentReplay"] = json!(true);
                response["activation"] = json!({ "state": "pending", "runtime": "fake" });
                return Ok(response);
            }
        }
        let result = match self
            .store
            .commit_graph(plan_id, base_revision, idempotency_key)
        {
            Ok(result) => result,
            Err(audiorouter_domain::StoreError::PlanNotFound) => {
                let storage = self.storage.as_ref().ok_or(ControlError::from(
                    audiorouter_domain::StoreError::PlanNotFound,
                ))?;
                let durable = storage
                    .load_graph_plan(plan_id.as_str())
                    .map_err(storage_error)?
                    .ok_or(ControlError::from(
                        audiorouter_domain::StoreError::PlanNotFound,
                    ))?;
                let remaining = durable.expires_at - unix_epoch_seconds();
                if remaining <= 0 {
                    storage
                        .delete_graph_plan(plan_id.as_str())
                        .map_err(storage_error)?;
                    return Err(ControlError::from(
                        audiorouter_domain::StoreError::PlanExpired,
                    ));
                }
                let durable_session_id = EntityId::new(durable.session_id.clone());
                self.ensure_session_loaded(&durable_session_id)?;
                self.store
                    .restore_plan_with_ttl(
                        EntityId::new(durable.id),
                        &durable_session_id,
                        durable.base_revision,
                        durable.candidate,
                        std::time::Duration::from_secs(remaining as u64),
                    )
                    .map_err(ControlError::from)?;
                self.store
                    .commit_graph(plan_id, base_revision, idempotency_key)
                    .map_err(ControlError::from)?
            }
            Err(error) => return Err(ControlError::from(error)),
        };
        if let Some(storage) = &self.storage {
            let session = self.store.session(&result.session_id).ok_or_else(|| {
                ControlError::InvalidRequest("committed session not found".into())
            })?;
            let result_document = serde_json::to_string(&result)
                .map_err(|error| ControlError::Json(error.to_string()))?;
            if let Err(error) = storage.save_session_with_journal_with_hash(
                session,
                idempotency_key,
                "graph.commit",
                &result_document,
                &request_hash,
                None,
            ) {
                self.store = checkpoint;
                return Err(storage_error(error));
            }
            storage
                .delete_graph_plan(plan_id.as_str())
                .map_err(storage_error)?;
        }
        if !result.idempotent_replay {
            self.events.append(
                result.revision,
                Some(idempotency_key.into()),
                "graph.committed",
                Some(result.session_id.clone()),
            );
        }
        let mut response =
            serde_json::to_value(&result).map_err(|error| ControlError::Json(error.to_string()))?;
        if !result.idempotent_replay
            && self
                .runtimes
                .get(&result.session_id)
                .map(|runtime| runtime.state() == RuntimeState::Running)
                .unwrap_or(false)
        {
            let session = self
                .store
                .session(&result.session_id)
                .cloned()
                .ok_or_else(|| {
                    ControlError::InvalidRequest("committed session not found".into())
                })?;
            let runtime = self.runtimes.get_mut(&result.session_id).unwrap();
            runtime.prepare(&session).map_err(|error| match error {
                RuntimeError::InvalidGraph(errors) => {
                    ControlError::InvalidRequest(format!("invalid graph: {errors:?}"))
                }
                RuntimeError::NotPrepared => {
                    ControlError::InvalidRequest("session was not prepared".into())
                }
            })?;
            let generation = runtime
                .start()
                .map_err(|_| ControlError::InvalidRequest("session was not prepared".into()))?;
            self.events.append(
                result.revision,
                Some(idempotency_key.into()),
                "runtime.activated",
                Some(result.session_id.clone()),
            );
            response["activation"] =
                json!({ "state": "running", "generation": generation, "runtime": "fake" });
        } else {
            response["activation"] = json!({ "state": "pending", "runtime": "fake" });
        }
        if self.operation_outcomes.len() >= MAX_MEMORY_OPERATION_OUTCOMES {
            if let Some(oldest) = self.operation_outcomes.keys().next().cloned() {
                self.operation_outcomes.remove(&oldest);
            }
        }
        self.operation_outcomes
            .insert(idempotency_key.to_owned(), response.clone());
        Ok(response)
    }

    pub fn session_start(&mut self, id: &EntityId) -> Result<Value, ControlError> {
        self.ensure_session_loaded(id)?;
        let session = self.get_session(id)?.clone();
        if let Some(runtime) = self.runtimes.get(id) {
            if runtime.state() == RuntimeState::Running {
                return Ok(
                    json!({ "sessionId": id, "state": "running", "generation": runtime.generation(), "runtime": "fake" }),
                );
            }
        }
        if self
            .runtimes
            .values()
            .filter(|runtime| runtime.state() == RuntimeState::Running)
            .count()
            >= audiorouter_domain::MAX_ACTIVE_SESSIONS
        {
            return Err(ControlError::InvalidRequest(
                "active session limit reached".into(),
            ));
        }
        let runtime = self.runtimes.entry(id.clone()).or_default();
        runtime.prepare(&session).map_err(|error| match error {
            RuntimeError::InvalidGraph(errors) => {
                ControlError::InvalidRequest(format!("invalid graph: {errors:?}"))
            }
            RuntimeError::NotPrepared => {
                ControlError::InvalidRequest("session was not prepared".into())
            }
        })?;
        let generation = runtime
            .start()
            .map_err(|_| ControlError::InvalidRequest("session was not prepared".into()))?;
        self.events
            .append(session.revision, None, "runtime.started", Some(id.clone()));
        Ok(
            json!({ "sessionId": id, "state": "running", "generation": generation, "runtime": "fake" }),
        )
    }

    pub fn session_stop(&mut self, id: &EntityId) -> Result<Value, ControlError> {
        self.ensure_session_loaded(id)?;
        let revision = self.get_session(id)?.revision;
        if let Some(runtime) = self.runtimes.get_mut(id) {
            runtime.stop();
        }
        self.events
            .append(revision, None, "runtime.stopped", Some(id.clone()));
        Ok(json!({ "sessionId": id, "state": "stopped", "runtime": "fake" }))
    }

    fn ensure_session_loaded(&mut self, id: &EntityId) -> Result<(), ControlError> {
        if self.store.session(id).is_some() {
            return Ok(());
        }
        let Some(storage) = &self.storage else {
            return Ok(());
        };
        if let Some(session) = storage.load_session(id).map_err(storage_error)? {
            self.store
                .insert_session(session)
                .map_err(ControlError::from)?;
        }
        Ok(())
    }

    pub fn dispatch(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();
        if request.validate().is_err() {
            return JsonRpcResponse::failure(id, -32600, "invalid request");
        }
        let mutating = is_mutating_method(&request.method);
        if request.is_notification() && mutating {
            return JsonRpcResponse::failure(
                None,
                -32600,
                "mutating notifications are not supported",
            );
        }
        let result =
            validate_method_params(&request.method, request.params.as_ref()).and_then(|_| {
                match request.method.as_str() {
                    "system.describe" => Ok(self.describe()),
                    "system.handshake" => self.dispatch_handshake(request.params),
                    "status.get" => self.status_snapshot(),
                    "system.diagnostics" => Ok(json!({
                        "build": self.build,
                        "backend": "control-plane",
                        "storage": if self.storage.is_some() { "sqlite" } else { "memory" },
                        "audio": {
                            "state": "unavailable",
                            "reason": "M02 realtime graph engine and routing are not implemented"
                        },
                        "nativeAdapter": "not activated",
                        "privacyMute": {
                            "muted": self.privacy_muted,
                            "persistence": if self.storage.is_some() { "durable" } else { "memory" }
                        },
                        "eventLog": {
                            "latestSequence": self.events.latest_sequence(),
                            "retained": self.events.len()
                        },
                        "redacted": true
                    })),
                    "clients.list" => self.dispatch_clients_list(),
                    "clients.authorize" => self.dispatch_client_authorize(request.params),
                    "clients.revoke" => self.dispatch_client_revoke(request.params),
                    "operations.get" => self.dispatch_operation_get(request.params),
                    "operations.cancel" => self.dispatch_operation_cancel(request.params),
                    "recordings.list" => self.dispatch_recordings_list(request.params),
                    "recordings.get" => self.dispatch_recordings_get(request.params),
                    "recordings.reveal" => self.dispatch_recording_reveal(request.params),
                    "recordings.preview" => self.dispatch_recordings_preview(request.params),
                    "recordings.setMetadata" => self.dispatch_recording_metadata(request.params),
                    "recordings.rename" => self.dispatch_recording_rename(request.params),
                    "recordings.removeEntry" => self.dispatch_recording_remove(request.params),
                    "safety.setPrivacyMute" => self.dispatch_privacy_mute(request.params),
                    "devices.list" => self.dispatch_devices_list(),
                    "apps.list" | "applications.list" => self.dispatch_apps_list(),
                    "nodes.types" => Ok(self.describe()["nodeTypes"].clone()),
                    "nodes.describe" => Ok(self.describe()["nodeTypes"].clone()),
                    "sessions.get" => self.dispatch_session_get(request.params),
                    "sessions.list" => self.dispatch_sessions_list(request.params),
                    "sessions.create" => self.dispatch_session_create(request.params),
                    "sessions.duplicate" => self.dispatch_session_duplicate(request.params),
                    "sessions.delete" => self.dispatch_session_delete(request.params),
                    "routes.inspect" => self.dispatch_routes_inspect(request.params),
                    "graph.history" => self.dispatch_graph_history(request.params),
                    "graph.undoPlan" => self.dispatch_graph_undo_plan(request.params),
                    "events.subscribe" => self.dispatch_events_subscribe(request.params),
                    "session.start" | "sessions.start" => {
                        self.dispatch_session_start(request.params)
                    }
                    "session.stop" | "sessions.stop" => self.dispatch_session_stop(request.params),
                    "graph.plan" => self.dispatch_plan(request.params),
                    "graph.commit" => self.dispatch_commit(request.params),
                    _ => Err(ControlError::InvalidRequest("method not found".into())),
                }
            });
        match result {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(ControlError::InvalidRequest(message)) if message == "method not found" => {
                JsonRpcResponse::failure(id, -32601, message)
            }
            Err(ControlError::InvalidRequest(message)) => {
                JsonRpcResponse::failure(id, -32602, message)
            }
            Err(error) => application_error_response(id, error),
        }
    }

    pub fn dispatch_authorized(
        &mut self,
        request: JsonRpcRequest,
        grant: &ClientGrant,
    ) -> JsonRpcResponse {
        self.dispatch_authorized_with_client(request, grant, None)
    }

    /// Dispatch an authenticated request with a stable client identity for
    /// mutation rate limiting. The identity is intentionally supplied by the
    /// transport after it has authenticated the caller.
    pub fn dispatch_authorized_for_client(
        &mut self,
        request: JsonRpcRequest,
        client_id: &str,
        grant: &ClientGrant,
    ) -> JsonRpcResponse {
        self.dispatch_authorized_with_client(request, grant, Some(client_id))
    }

    fn dispatch_authorized_with_client(
        &mut self,
        request: JsonRpcRequest,
        grant: &ClientGrant,
        client_id: Option<&str>,
    ) -> JsonRpcResponse {
        let id = request.id.clone();
        let Some(spec) = API_METHODS.iter().find(|spec| spec.name == request.method) else {
            return self.dispatch(request);
        };
        if !grant.allows(spec.permission) {
            let mut response = JsonRpcResponse::failure(
                id,
                -32001,
                format!("permission denied: {:?}", spec.permission),
            );
            if let Some(error) = response.error.as_mut() {
                error.data = Some(application_error_data("permissionDenied"));
            }
            return response;
        }
        if is_mutating_method(&request.method) {
            if let Some(client_id) = client_id {
                if let Err(retry_after_ms) = self.mutation_limiter.allow(client_id) {
                    let mut response = JsonRpcResponse::failure(id, -32000, "rate limited");
                    if let Some(error) = response.error.as_mut() {
                        error.data = Some(json!({
                            "code": "rateLimited",
                            "fieldPath": Value::Null,
                            "resourceIds": [],
                            "retryable": true,
                            "remediation": "wait for the retry hint before sending another mutation",
                            "retryAfterMs": retry_after_ms
                        }));
                    }
                    return response;
                }
            }
        }
        self.dispatch(request)
    }

    pub fn dispatch_message(&mut self, message: RpcMessage) -> Vec<JsonRpcResponse> {
        match message {
            RpcMessage::Single(request) => {
                let omit = request.is_notification() && !is_mutating_method(&request.method);
                let response = self.dispatch(request);
                if omit {
                    Vec::new()
                } else {
                    vec![response]
                }
            }
            RpcMessage::Batch(requests) => requests
                .into_iter()
                .filter_map(|request| {
                    let omit = request.is_notification() && !is_mutating_method(&request.method);
                    let response = self.dispatch(request);
                    if omit {
                        None
                    } else {
                        Some(response)
                    }
                })
                .collect(),
        }
    }

    /// Dispatch a parsed message through the caller's explicit permission grant.
    /// Authorization runs before method parameters are interpreted or state is
    /// mutated, including for batched messages and notifications.
    pub fn dispatch_message_authorized(
        &mut self,
        message: RpcMessage,
        grant: &ClientGrant,
    ) -> Vec<JsonRpcResponse> {
        self.dispatch_message_authorized_with_client(message, grant, None)
    }

    pub fn dispatch_message_authorized_for_client(
        &mut self,
        message: RpcMessage,
        client_id: &str,
        grant: &ClientGrant,
    ) -> Vec<JsonRpcResponse> {
        self.dispatch_message_authorized_with_client(message, grant, Some(client_id))
    }

    fn dispatch_message_authorized_with_client(
        &mut self,
        message: RpcMessage,
        grant: &ClientGrant,
        client_id: Option<&str>,
    ) -> Vec<JsonRpcResponse> {
        match message {
            RpcMessage::Single(request) => {
                let omit = request.is_notification()
                    && !matches!(
                        request.method.as_str(),
                        "graph.plan"
                            | "graph.undoPlan"
                            | "graph.commit"
                            | "session.start"
                            | "sessions.start"
                            | "session.stop"
                            | "sessions.stop"
                    );
                let response = self.dispatch_authorized_with_client(request, grant, client_id);
                if omit {
                    Vec::new()
                } else {
                    vec![response]
                }
            }
            RpcMessage::Batch(requests) => requests
                .into_iter()
                .filter_map(|request| {
                    let omit = request.is_notification()
                        && !matches!(
                            request.method.as_str(),
                            "graph.plan"
                                | "graph.undoPlan"
                                | "graph.commit"
                                | "session.start"
                                | "sessions.start"
                                | "session.stop"
                                | "sessions.stop"
                        );
                    let response = self.dispatch_authorized_with_client(request, grant, client_id);
                    if omit {
                        None
                    } else {
                        Some(response)
                    }
                })
                .collect(),
        }
    }

    pub fn dispatch_frame(&mut self, frame: &[u8]) -> Result<Vec<Vec<u8>>, FrameError> {
        let message = decode_rpc_frame(frame)?;
        self.dispatch_message(message)
            .into_iter()
            .map(|response| encode_frame(&response))
            .collect()
    }

    pub fn dispatch_frame_authorized(
        &mut self,
        frame: &[u8],
        grant: &ClientGrant,
    ) -> Result<Vec<Vec<u8>>, FrameError> {
        let message = decode_rpc_frame(frame)?;
        self.dispatch_message_authorized(message, grant)
            .into_iter()
            .map(|response| encode_frame(&response))
            .collect()
    }

    pub fn dispatch_frame_authorized_for_client(
        &mut self,
        frame: &[u8],
        client_id: &str,
        grant: &ClientGrant,
    ) -> Result<Vec<Vec<u8>>, FrameError> {
        let message = decode_rpc_frame(frame)?;
        self.dispatch_message_authorized_for_client(message, client_id, grant)
            .into_iter()
            .map(|response| encode_frame(&response))
            .collect()
    }

    fn dispatch_plan(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params
            .ok_or_else(|| ControlError::InvalidRequest("graph.plan params are required".into()))?;
        let session_id: EntityId = serde_json::from_value(
            params
                .get("sessionId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let base_revision = params
            .get("baseRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("baseRevision is required".into()))?;
        let candidate: Session = serde_json::from_value(
            params
                .get("candidate")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("candidate is required".into()))?,
        )
        .map_err(|error| ControlError::InvalidRequest(error.to_string()))?;
        self.ensure_session_loaded(&session_id)?;
        let existing = self.get_session(&session_id)?.clone();
        let diff = graph_diff(&existing, &candidate);
        let affected_destinations = candidate
            .nodes
            .iter()
            .filter(|node| node.kind == audiorouter_domain::NodeKind::PhysicalOutput)
            .map(|node| node.name.clone())
            .collect::<Vec<_>>();
        let plan_id = self.plan_graph(&session_id, base_revision, candidate)?;
        Ok(json!({
            "planId": plan_id,
            "baseRevision": base_revision,
            "expiresInMs": 300000,
            "diff": diff,
            "affectedDestinations": affected_destinations,
            "warnings": [],
            "requiredScopes": ["graph.write"]
        }))
    }

    fn dispatch_handshake(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params
            .ok_or_else(|| ControlError::InvalidRequest("protocolVersion is required".into()))?;
        let version = params
            .get("protocolVersion")
            .and_then(Value::as_object)
            .ok_or_else(|| ControlError::InvalidRequest("protocolVersion is required".into()))?;
        let major = version
            .get("major")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("protocol major is required".into()))?;
        let minor = version
            .get("minor")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("protocol minor is required".into()))?;
        if major != 1 {
            return Err(ControlError::InvalidRequest(format!(
                "unsupported protocol major: {major}"
            )));
        }
        Ok(json!({
            "compatible": true,
            "requested": { "major": major, "minor": minor },
            "negotiated": { "major": 1, "minor": 0 },
            "schemaVersion": 1
        }))
    }

    fn dispatch_commit(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("graph.commit params are required".into())
        })?;
        if let Some(acknowledgments) = params.get("acknowledgments") {
            let acknowledgments = acknowledgments.as_array().ok_or_else(|| {
                ControlError::InvalidRequest("acknowledgments must be an array or null".into())
            })?;
            if acknowledgments.iter().any(|value| match value.as_str() {
                Some(value) => value.is_empty() || value.len() > 128,
                None => true,
            }) {
                return Err(ControlError::InvalidRequest(
                    "acknowledgments must contain non-empty warning IDs".into(),
                ));
            }
            if !acknowledgments.is_empty() {
                return Err(ControlError::InvalidRequest(
                    "no warnings on this plan require acknowledgment".into(),
                ));
            }
        }
        let plan_id: EntityId = serde_json::from_value(
            params
                .get("planId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid planId".into()))?;
        let base_revision = params
            .get("baseRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("baseRevision is required".into()))?;
        let key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        self.commit_graph(&plan_id, base_revision, key)
    }

    fn dispatch_session_start(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let id = session_id_from_params(params)?;
        self.session_start(&id)
    }

    fn dispatch_session_get(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let id = session_id_from_params(params)?;
        self.ensure_session_loaded(&id)?;
        serde_json::to_value(self.get_session(&id)?)
            .map_err(|error| ControlError::Json(error.to_string()))
    }

    fn dispatch_session_create(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("session is required".into()))?;
        let session: Session = serde_json::from_value(
            params
                .get("session")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("session is required".into()))?,
        )
        .map_err(|error| ControlError::InvalidRequest(error.to_string()))?;
        self.create_session(session)
    }

    fn dispatch_session_duplicate(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("duplicate parameters are required".into())
        })?;
        let source_id: EntityId =
            serde_json::from_value(params.get("sourceSessionId").cloned().ok_or_else(|| {
                ControlError::InvalidRequest("sourceSessionId is required".into())
            })?)
            .map_err(|_| ControlError::InvalidRequest("invalid sourceSessionId".into()))?;
        let duplicate_id: EntityId = serde_json::from_value(
            params
                .get("sessionId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let name = params
            .get("name")
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| ControlError::InvalidRequest("name must be a string".into()))
            })
            .transpose()?;
        self.duplicate_session(&source_id, duplicate_id, name)
    }

    fn dispatch_session_delete(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let id = session_id_from_params(params)?;
        self.delete_session(&id)
    }

    fn dispatch_sessions_list(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let cursor = params
            .as_ref()
            .and_then(|value| value.get("cursor"))
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| ControlError::InvalidRequest("cursor must be a string".into()))
            })
            .transpose()?;
        let limit = params
            .as_ref()
            .and_then(|value| value.get("limit"))
            .and_then(Value::as_u64)
            .unwrap_or(100);
        self.sessions_list_page(cursor, limit as usize)
    }

    fn dispatch_session_stop(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let id = session_id_from_params(params)?;
        self.session_stop(&id)
    }

    fn dispatch_routes_inspect(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("routes.inspect params are required".into())
        })?;
        let session_id: EntityId = serde_json::from_value(
            params
                .get("sessionId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let destination_node: EntityId =
            serde_json::from_value(params.get("destinationNode").cloned().ok_or_else(|| {
                ControlError::InvalidRequest("destinationNode is required".into())
            })?)
            .map_err(|_| ControlError::InvalidRequest("invalid destinationNode".into()))?;
        self.inspect_routes(&session_id, &destination_node)
    }

    fn dispatch_graph_history(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("graph.history params are required".into())
        })?;
        let session_id: EntityId = serde_json::from_value(
            params
                .get("sessionId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(100);
        if limit == 0 || limit > 100 {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 100".into(),
            ));
        }
        let before_revision = params
            .get("cursor")
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| ControlError::InvalidRequest("cursor must be a string".into()))?
                    .parse::<u64>()
                    .map_err(|_| ControlError::InvalidRequest("invalid history cursor".into()))
            })
            .transpose()?;
        self.graph_history_page(&session_id, before_revision, limit as usize)
    }

    fn dispatch_graph_undo_plan(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("graph.undoPlan params are required".into())
        })?;
        let session_id: EntityId = serde_json::from_value(
            params
                .get("sessionId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let base_revision = params
            .get("baseRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("baseRevision is required".into()))?;
        let plan_id = self.graph_undo_plan(&session_id, base_revision)?;
        Ok(json!({ "planId": plan_id, "baseRevision": base_revision, "expiresInMs": 300000 }))
    }

    fn dispatch_recordings_list(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let session_id = params.get("sessionId").and_then(Value::as_str);
        let Some(storage) = &self.storage else {
            return Ok(json!([]));
        };
        let records = storage
            .list_recordings(session_id)
            .map_err(storage_error)?
            .into_iter()
            .map(|record| {
                json!({
                    "id": record.id,
                    "sessionId": record.session_id,
                    "recorderId": record.recorder_id,
                    "path": record.path,
                    "format": record.format,
                    "channels": record.channels,
                    "sampleRate": record.sample_rate,
                    "frames": record.frames,
                    "fileBytes": record.file_bytes,
                    "startTime": record.start_time,
                    "state": record.state,
                    "missing": record.missing,
                    "title": record.title,
                    "artist": record.artist,
                    "comment": record.comment
                })
            })
            .collect::<Vec<_>>();
        Ok(json!(records))
    }

    fn dispatch_recordings_get(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let recording_id = params
            .and_then(|params| {
                params
                    .get("recordingId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let Some(storage) = &self.storage else {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        };
        let record = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        Ok(json!({
            "id": record.id,
            "sessionId": record.session_id,
            "recorderId": record.recorder_id,
            "path": record.path,
            "format": record.format,
            "channels": record.channels,
            "sampleRate": record.sample_rate,
            "frames": record.frames,
            "fileBytes": record.file_bytes,
            "startTime": record.start_time,
            "state": record.state,
            "missing": record.missing,
            "title": record.title,
            "artist": record.artist,
            "comment": record.comment
        }))
    }

    fn dispatch_recording_reveal(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let recording_id = params
            .and_then(|params| {
                params
                    .get("recordingId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let record = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let path = std::path::Path::new(&record.path);
        if !path.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "recording path must be absolute".into(),
            ));
        }
        if !path.is_file() {
            return Ok(
                json!({ "recordingId": recording_id, "path": record.path, "revealed": false, "reason": "missing" }),
            );
        }
        #[cfg(windows)]
        let revealed = std::process::Command::new("explorer.exe")
            .args(["/select,", &record.path])
            .spawn()
            .map(|_| true)
            .map_err(|error| {
                ControlError::InvalidRequest(format!("unable to reveal recording: {error}"))
            })?;
        #[cfg(not(windows))]
        let revealed = false;
        Ok(json!({ "recordingId": recording_id, "path": record.path, "revealed": revealed }))
    }

    fn dispatch_recordings_preview(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let recording_id = params
            .and_then(|params| {
                params
                    .get("recordingId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let record = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let status = audiorouter_recording::inspect_recording(&record.path).map_err(|error| {
            ControlError::InvalidRequest(format!("recording preview failed: {error:?}"))
        })?;
        let result = match status {
            audiorouter_recording::RecordingFileStatus::Present(info) => json!({
                "status": "present",
                "format": "wav",
                "channels": info.channels,
                "sampleRate": info.sample_rate,
                "frames": info.frames,
                "dataBytes": info.data_bytes,
                "fileBytes": info.file_bytes
            }),
            audiorouter_recording::RecordingFileStatus::FlacPresent(info) => json!({
                "status": "present",
                "format": "flac",
                "channels": info.channels,
                "sampleRate": info.sample_rate,
                "bitsPerSample": info.bits_per_sample,
                "frames": info.frames,
                "fileBytes": info.file_bytes
            }),
            audiorouter_recording::RecordingFileStatus::Missing => json!({ "status": "missing" }),
            audiorouter_recording::RecordingFileStatus::Invalid => json!({ "status": "invalid" }),
        };
        Ok(json!({ "recordingId": recording_id, "preview": result }))
    }

    fn dispatch_recording_metadata(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let updated = storage
            .update_recording_metadata(
                recording_id,
                params.get("title").and_then(Value::as_str),
                params.get("artist").and_then(Value::as_str),
                params.get("comment").and_then(Value::as_str),
            )
            .map_err(storage_error)?;
        if !updated {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        }
        Ok(json!({ "recordingId": recording_id, "updated": true }))
    }

    fn dispatch_recording_rename(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("recordingId and newPath are required".into())
        })?;
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let new_path = params
            .get("newPath")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("newPath is required".into()))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        if !storage
            .rename_recording(recording_id, new_path)
            .map_err(storage_error)?
        {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        }
        Ok(json!({
            "recordingId": recording_id,
            "renamed": true,
            "path": new_path,
            "fileAction": "renamed"
        }))
    }

    fn dispatch_recording_remove(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let recording_id = params
            .and_then(|params| {
                params
                    .get("recordingId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        if !storage
            .remove_recording_entry(&recording_id)
            .map_err(storage_error)?
        {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        }
        Ok(json!({ "recordingId": recording_id, "removed": true, "fileAction": "none" }))
    }

    fn dispatch_privacy_mute(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let muted = params
            .and_then(|params| params.get("muted").and_then(Value::as_bool))
            .ok_or_else(|| ControlError::InvalidRequest("muted is required".into()))?;
        if let Some(storage) = &self.storage {
            storage.save_privacy_mute(muted).map_err(storage_error)?;
        }
        self.privacy_muted = muted;
        self.events.append(
            0,
            None,
            if muted {
                "privacy.muteEnabled"
            } else {
                "privacy.muteDisabled"
            },
            None,
        );
        Ok(json!({
            "muted": muted,
            "persistence": if self.storage.is_some() { "durable" } else { "memory" },
            "audioEffect": "process-local-when-realtime-backend-is-available"
        }))
    }

    fn dispatch_events_subscribe(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        if let Some(requested_epoch) = params.get("backendEpoch").and_then(Value::as_u64) {
            if requested_epoch != self.events.backend_epoch() {
                return Ok(json!({
                    "backendEpoch": self.events.backend_epoch(),
                    "resyncRequired": true,
                    "reason": "backendEpochChanged",
                    "snapshot": { "sessions": self.sessions_list(500)? },
                    "events": [],
                    "nextSequence": self.events.latest_sequence()
                }));
            }
        }
        let after_sequence = params
            .get("afterSequence")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(100);
        if !(1..=500).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        let session_filter = params
            .get("sessionId")
            .map(|value| {
                serde_json::from_value::<EntityId>(value.clone())
                    .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))
            })
            .transpose()?;
        let events = match self.events.since(after_sequence, limit as usize) {
            Ok(events) => events,
            Err(EventReplayError::InvalidLimit) => {
                return Err(ControlError::InvalidRequest(
                    "limit must be between 1 and 500".into(),
                ));
            }
            Err(EventReplayError::ResyncRequired) => {
                return Ok(json!({
                    "backendEpoch": self.events.backend_epoch(),
                    "resyncRequired": true,
                    "snapshot": { "sessions": self.sessions_list(500)? },
                    "events": [],
                    "nextSequence": self.events.latest_sequence()
                }));
            }
        }
        .into_iter()
        .filter(|event| {
            session_filter
                .as_ref()
                .map(|id| event.session_id.as_ref() == Some(id))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
        Ok(json!({
            "backendEpoch": self.events.backend_epoch(),
            "events": events,
            "nextSequence": self.events.latest_sequence(),
        }))
    }

    fn dispatch_devices_list(&self) -> Result<Value, ControlError> {
        let endpoints = audiorouter_windows_audio::enumerate_active_endpoints()
            .map_err(|error| ControlError::InvalidRequest(error.to_string()))?;
        Ok(json!(endpoints
            .into_iter()
            .map(|endpoint| json!({
                "id": endpoint.id,
                "direction": match endpoint.direction {
                    audiorouter_windows_audio::EndpointDirection::Capture => "capture",
                    audiorouter_windows_audio::EndpointDirection::Render => "render",
                },
                "state": "active",
                "format": {
                    "sampleRateHz": endpoint.sample_rate_hz,
                    "channels": endpoint.channels,
                    "bitsPerSample": endpoint.bits_per_sample,
                    "formatTag": endpoint.format_tag,
                },
                "periods": {
                    "default100ns": endpoint.default_period_100ns,
                    "minimum100ns": endpoint.minimum_period_100ns,
                },
            }))
            .collect::<Vec<_>>()))
    }

    fn dispatch_apps_list(&mut self) -> Result<Value, ControlError> {
        if let Some((captured_at, snapshot)) = &self.application_snapshot {
            if captured_at.elapsed() < APPLICATION_SNAPSHOT_TTL {
                return Ok(snapshot.clone());
            }
        }
        let applications = audiorouter_windows_audio::enumerate_applications()
            .map_err(|error| ControlError::InvalidRequest(error.to_string()))?;
        let snapshot = json!(applications
            .into_iter()
            .map(|application| json!({
                "processId": application.process_id,
                "executable": application.executable,
                "creationTime100ns": application.creation_time_100ns.map(|value| value.to_string()),
            }))
            .collect::<Vec<_>>());
        self.application_snapshot = Some((Instant::now(), snapshot.clone()));
        Ok(snapshot)
    }
}

fn session_id_from_params(params: Option<Value>) -> Result<EntityId, ControlError> {
    let params =
        params.ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
    serde_json::from_value(
        params
            .get("sessionId")
            .cloned()
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
    )
    .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))
}

fn graph_diff(before: &Session, after: &Session) -> Vec<Value> {
    let mut diff = Vec::new();
    if before.name != after.name {
        diff.push(json!({
            "path": "/name",
            "before": &before.name,
            "after": &after.name,
        }));
    }
    if before.nodes != after.nodes {
        diff.push(json!({
            "path": "/nodes",
            "before": &before.nodes,
            "after": &after.nodes,
        }));
    }
    if before.edges != after.edges {
        diff.push(json!({
            "path": "/edges",
            "before": &before.edges,
            "after": &after.edges,
        }));
    }
    diff
}

fn validate_method_params(method: &str, params: Option<&Value>) -> Result<(), ControlError> {
    let Some(params) = params else {
        return Ok(());
    };
    let Some(object) = params.as_object() else {
        return Err(ControlError::InvalidRequest(
            "method params must be an object".into(),
        ));
    };
    let allowed: &[&str] = match method {
        "sessions.get" | "sessions.delete" | "session.start" | "sessions.start"
        | "session.stop" | "sessions.stop" => &["sessionId"],
        "sessions.list" => &["cursor", "limit"],
        "sessions.create" => &["session"],
        "sessions.duplicate" => &["sourceSessionId", "sessionId", "name"],
        "routes.inspect" => &["sessionId", "destinationNode"],
        "graph.history" => &["sessionId", "cursor", "limit"],
        "graph.undoPlan" => &["sessionId", "baseRevision"],
        "events.subscribe" => &["afterSequence", "backendEpoch", "limit", "sessionId"],
        "graph.plan" => &["sessionId", "baseRevision", "candidate"],
        "graph.commit" => &[
            "planId",
            "baseRevision",
            "idempotencyKey",
            "acknowledgments",
        ],
        "system.handshake" => &["protocolVersion"],
        "clients.authorize" => &["clientId", "role"],
        "clients.revoke" => &["clientId"],
        "operations.get" | "operations.cancel" => &["operationId"],
        "recordings.list" => &["sessionId"],
        "recordings.get" | "recordings.reveal" | "recordings.preview" => &["recordingId"],
        "recordings.setMetadata" => &["recordingId", "title", "artist", "comment"],
        "recordings.rename" => &["recordingId", "newPath"],
        "safety.setPrivacyMute" => &["muted"],
        "recordings.removeEntry" => &["recordingId"],
        "system.describe" | "status.get" | "system.diagnostics" | "devices.list" | "apps.list"
        | "applications.list" | "nodes.types" | "nodes.describe" | "clients.list" => &[],
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

fn role_name(role: ClientRole) -> &'static str {
    match role {
        ClientRole::Observer => "observer",
        ClientRole::Editor => "editor",
        ClientRole::Operator => "operator",
    }
}

fn role_from_name(name: &str) -> Option<ClientRole> {
    match name {
        "observer" => Some(ClientRole::Observer),
        "editor" => Some(ClientRole::Editor),
        "operator" => Some(ClientRole::Operator),
        _ => None,
    }
}

fn is_mutating_method(method: &str) -> bool {
    matches!(
        method,
        "graph.plan"
            | "graph.undoPlan"
            | "graph.commit"
            | "session.start"
            | "sessions.start"
            | "session.stop"
            | "sessions.stop"
            | "sessions.create"
            | "sessions.duplicate"
            | "sessions.delete"
            | "clients.authorize"
            | "clients.revoke"
            | "recordings.setMetadata"
            | "recordings.reveal"
            | "recordings.removeEntry"
    )
}

fn storage_error(error: StorageError) -> ControlError {
    ControlError::Storage(format!("{error:?}"))
}

fn application_error_response(id: Option<Value>, error: ControlError) -> JsonRpcResponse {
    let code = match &error {
        ControlError::Store(error) => match error {
            audiorouter_domain::StoreError::SessionNotFound => "notFound",
            audiorouter_domain::StoreError::PlanNotFound => "notFound",
            audiorouter_domain::StoreError::PlanExpired => "planExpired",
            audiorouter_domain::StoreError::InvalidGraph(_) => "invalidGraph",
            audiorouter_domain::StoreError::RevisionConflict { .. } => "revisionConflict",
            audiorouter_domain::StoreError::EmptyIdempotencyKey => "invalidRequest",
            audiorouter_domain::StoreError::NoUndoAvailable => "noUndoAvailable",
            audiorouter_domain::StoreError::IdempotencyConflict => "idempotencyConflict",
        },
        ControlError::Storage(_) => "storageFailure",
        ControlError::Json(_) => "internalError",
        ControlError::InvalidRequest(_) => "invalidRequest",
    };
    let message = format!("{error:?}");
    let mut response = JsonRpcResponse::failure(id, -32000, message);
    if let Some(error) = response.error.as_mut() {
        error.data = Some(application_error_data(code));
    }
    response
}

fn application_error_data(code: &str) -> Value {
    let (retryable, remediation) = match code {
        "revisionConflict" => (
            true,
            "read the latest session revision and create a new plan",
        ),
        "planExpired" => (true, "create a new plan from the current session revision"),
        "storageFailure" => (
            true,
            "inspect backend health and retry after the failure is resolved",
        ),
        "deviceUnavailable" => (
            true,
            "inspect device availability and rebind the affected resource",
        ),
        "permissionDenied" => (
            false,
            "request the required permission scope for the target operation",
        ),
        "invalidRequest" | "invalidGraph" => {
            (false, "correct the request using the discovered schema")
        }
        _ => (
            false,
            "inspect the error code and correct or explicitly retry the operation",
        ),
    };
    json!({
        "code": code,
        "fieldPath": Value::Null,
        "resourceIds": [],
        "retryable": retryable,
        "remediation": remediation
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_domain::{Edge, Node, NodeKind, Port, PortDirection};

    #[test]
    fn mutation_rate_limiter_enforces_burst_and_refill_rate() {
        let mut limiter = MutationRateLimiter::default();
        let start = Instant::now();
        for _ in 0..40 {
            assert!(limiter.allow_at("client", start).is_ok());
        }
        assert_eq!(limiter.allow_at("client", start), Err(50));
        assert!(limiter
            .allow_at("client", start + std::time::Duration::from_millis(50))
            .is_ok());
    }

    #[test]
    fn authenticated_dispatch_returns_rate_limit_metadata() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original).unwrap();
        let grant = ClientGrant::for_role(ClientRole::Operator);
        let request = || JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "session.start".into(),
            params: Some(json!({ "sessionId": "session" })),
        };
        for _ in 0..40 {
            assert!(plane
                .dispatch_authorized_for_client(request(), "client", &grant)
                .result
                .is_some());
        }
        let response = plane.dispatch_authorized_for_client(request(), "client", &grant);
        assert_eq!(response.error.as_ref().unwrap().code, -32000);
        let data = response.error.as_ref().unwrap().data.as_ref().unwrap();
        assert_eq!(data["code"], "rateLimited");
        assert_eq!(data["retryAfterMs"], 50);
        assert_eq!(data["retryable"], true);
    }

    #[test]
    fn sessions_list_supports_stable_cursor_pages() {
        let mut plane = ControlPlane::default();
        for id in ["a", "b", "c"] {
            let mut value = session();
            value.id = EntityId::new(id);
            plane.insert_session(value).unwrap();
        }
        let first = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "sessions.list".into(),
                params: Some(json!({ "limit": 2 })),
            })
            .result
            .unwrap();
        assert_eq!(first["items"].as_array().unwrap().len(), 2);
        assert_eq!(first["items"][0]["id"], "a");
        assert_eq!(first["nextCursor"], "b");
        let second = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "sessions.list".into(),
                params: Some(json!({ "cursor": "b", "limit": 2 })),
            })
            .result
            .unwrap();
        assert_eq!(second["items"].as_array().unwrap().len(), 1);
        assert_eq!(second["items"][0]["id"], "c");
        assert!(second["nextCursor"].is_null());
    }

    #[test]
    fn session_create_and_delete_protect_running_resources() {
        let mut plane = ControlPlane::default();
        let mut created = session();
        created.id = EntityId::new("created");
        let result = plane.create_session(created.clone()).unwrap();
        assert_eq!(result["state"], "stopped");
        let duplicate = plane
            .duplicate_session(
                &created.id,
                EntityId::new("copy"),
                Some("Copied session".into()),
            )
            .unwrap();
        assert_eq!(duplicate["session"]["id"], "copy");
        assert_eq!(duplicate["session"]["name"], "Copied session");
        assert_eq!(duplicate["session"]["revision"], 0);
        assert!(matches!(
            plane.duplicate_session(&created.id, EntityId::new("copy"), None),
            Err(ControlError::InvalidRequest(message))
                if message == "duplicate session ID already exists"
        ));
        assert_eq!(plane.delete_session(&created.id).unwrap()["deleted"], true);
        assert_eq!(
            plane.delete_session(&EntityId::new("copy")).unwrap()["deleted"],
            true
        );
        assert!(matches!(
            plane.delete_session(&created.id),
            Err(ControlError::InvalidRequest(message)) if message == "session not found"
        ));

        let mut running = session();
        running.id = EntityId::new("running");
        plane.create_session(running.clone()).unwrap();
        plane.session_start(&running.id).unwrap();
        assert!(matches!(
            plane.delete_session(&running.id),
            Err(ControlError::InvalidRequest(message)) if message == "stop the session before deleting it"
        ));
        plane.session_stop(&running.id).unwrap();
        assert_eq!(plane.delete_session(&running.id).unwrap()["deleted"], true);
    }

    #[test]
    fn application_errors_include_stable_codes() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "changed".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "graph.commit".into(),
            params: Some(json!({
                "planId": plan,
                "baseRevision": 1,
                "idempotencyKey": "conflict"
            })),
        });
        assert_eq!(response.error.as_ref().unwrap().code, -32000);
        let data = response.error.as_ref().unwrap().data.as_ref().unwrap();
        assert_eq!(data["code"], "revisionConflict");
        assert_eq!(data["retryable"], true);
        assert!(data["remediation"].as_str().unwrap().contains("new plan"));
    }

    fn session() -> Session {
        Session {
            id: EntityId::new("session"),
            name: "test".into(),
            schema_version: 1,
            revision: 0,
            nodes: vec![
                Node {
                    id: EntityId::new("in"),
                    kind: NodeKind::PhysicalInput,
                    name: "Input".into(),
                    enabled: true,
                    bypass: false,
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    }],
                },
                Node {
                    id: EntityId::new("out"),
                    kind: NodeKind::PhysicalOutput,
                    name: "Output".into(),
                    enabled: true,
                    bypass: false,
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Input,
                        channels: 1,
                    }],
                },
            ],
            edges: vec![Edge {
                id: EntityId::new("edge"),
                source_node: EntityId::new("in"),
                source_port: "main".into(),
                destination_node: EntityId::new("out"),
                destination_port: "main".into(),
                matrix: vec![1.0],
                enabled: true,
            }],
        }
    }

    #[test]
    fn describe_exposes_versions_methods_limits_and_unavailable_nodes() {
        let plane = ControlPlane::new("test-build");
        let description = plane.describe();
        assert_eq!(description["build"], "test-build");
        assert_eq!(description["protocolVersion"]["major"], 1);
        assert_eq!(description["limits"]["maxNodesPerSession"], 64);
        assert_eq!(description["limits"]["maxNodesGlobal"], 128);
        assert_eq!(description["limits"]["maxEdgesGlobal"], 256);
        assert_eq!(description["limits"]["maxActiveSessions"], 2);
        assert_eq!(description["events"]["retention"]["maxEvents"], 10_000);
        assert_eq!(description["events"]["retention"]["maxAgeSeconds"], 900);
        assert_eq!(description["events"]["meterReplay"], false);
        assert!(description["events"]["stateCategories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|category| category == "graph.committed"));
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "graph.plan"));
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "nodes.describe"));
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "sessions.get"));
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "applications.list"));
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "system.diagnostics"));
        assert!(description["nodeTypes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["type"] == "physical-input@1"
                && node["availability"]["status"] == "unavailable"));
        let gain = description["nodeTypes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["type"] == "gain@1")
            .unwrap();
        assert_eq!(gain["parameters"][0]["name"], "gainDb");
        assert_eq!(gain["parameters"][0]["minimum"], -60.0);
        assert_eq!(gain["parameters"][0]["maximum"], 12.0);
    }

    #[test]
    fn describe_exposes_input_and_output_schemas_for_methods() {
        let methods = ControlPlane::default().describe()["methods"]
            .as_array()
            .unwrap()
            .clone();
        let commit = methods
            .iter()
            .find(|method| method["name"] == "graph.commit")
            .unwrap();
        assert_eq!(
            commit["description"],
            "Commit an unexpired graph plan with idempotent mutation."
        );
        assert_eq!(
            commit["inputSchema"]["required"],
            json!(["planId", "baseRevision", "idempotencyKey"])
        );
        assert_eq!(commit["outputSchema"]["type"], "object");
        let devices = methods
            .iter()
            .find(|method| method["name"] == "devices.list")
            .unwrap();
        assert_eq!(devices["inputSchema"]["additionalProperties"], false);
        assert_eq!(devices["outputSchema"]["type"], "array");
    }

    #[test]
    fn dispatch_rejects_parameters_outside_discovered_schema() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "sessions.list".into(),
            params: Some(json!({ "unexpected": true })),
        });
        assert_eq!(response.error.unwrap().code, -32602);

        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "devices.list".into(),
            params: Some(json!([])),
        });
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[test]
    fn canonical_application_list_alias_uses_the_same_discovery_result() {
        let mut plane = ControlPlane::default();
        let legacy = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "apps.list".into(),
            params: None,
        });
        let canonical = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(6)),
            method: "applications.list".into(),
            params: None,
        });
        assert_eq!(legacy.result, canonical.result);
    }

    #[test]
    fn nullable_optional_parameters_are_treated_as_omitted() {
        let mut plane = ControlPlane::default();
        let mut source = session();
        source.id = EntityId::new("source");
        plane.insert_session(source).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(7)),
            method: "sessions.duplicate".into(),
            params: Some(json!({
                "sourceSessionId": "source",
                "sessionId": "copy",
                "name": null
            })),
        });
        assert!(response.result.is_some());
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(8)),
            method: "sessions.list".into(),
            params: Some(json!({ "cursor": null })),
        });
        assert!(response.result.is_some());
    }

    #[test]
    fn diagnostics_are_redacted_and_report_backend_state() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(9)),
            method: "system.diagnostics".into(),
            params: None,
        });
        let result = response.result.unwrap();
        assert_eq!(result["backend"], "control-plane");
        assert_eq!(result["storage"], "memory");
        assert_eq!(result["redacted"], true);
        assert!(result.get("path").is_none());
    }

    #[test]
    fn status_snapshot_tracks_sessions_and_event_cursor() {
        let mut plane = ControlPlane::default();
        let initial = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(15)),
            method: "status.get".into(),
            params: None,
        });
        assert_eq!(initial.result.as_ref().unwrap()["sessionCount"], 0);
        let value = session();
        plane.insert_session(value.clone()).unwrap();
        let status = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(16)),
            method: "status.get".into(),
            params: None,
        });
        let result = status.result.unwrap();
        assert_eq!(result["sessionCount"], 1);
        assert_eq!(result["activeSessionCount"], 0);
        assert_eq!(result["eventCursor"]["latestSequence"], 1);
    }

    #[test]
    fn privacy_mute_is_authorized_and_durable_across_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-privacy-mute-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
        let enabled = first.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(19)),
            method: "safety.setPrivacyMute".into(),
            params: Some(json!({ "muted": true })),
        });
        assert_eq!(enabled.result.unwrap()["muted"], true);
        let denied = first.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(20)),
                method: "safety.setPrivacyMute".into(),
                params: Some(json!({ "muted": false })),
            },
            &ClientGrant::read_only(),
        );
        assert_eq!(denied.error.unwrap().code, -32001);
        drop(first);
        let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
        let status = second
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(21)),
                method: "status.get".into(),
                params: None,
            })
            .result
            .unwrap();
        assert_eq!(status["privacyMute"]["muted"], true);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn client_enrollment_api_lists_authorizes_and_revokes() {
        let mut plane = ControlPlane::default();
        let grant = ClientGrant::with_scopes([PermissionScope::DeviceAdministration]);
        let authorize = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(10)),
                method: "clients.authorize".into(),
                params: Some(json!({ "clientId": "desktop", "role": "editor" })),
            },
            &grant,
        );
        assert_eq!(authorize.result.unwrap()["revoked"], false);
        let listed = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(11)),
            method: "clients.list".into(),
            params: None,
        });
        assert_eq!(listed.result.unwrap()[0]["clientId"], "desktop");
        let revoked = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(12)),
                method: "clients.revoke".into(),
                params: Some(json!({ "clientId": "desktop" })),
            },
            &grant,
        );
        assert_eq!(revoked.result.unwrap()["changed"], true);
    }

    #[test]
    fn handshake_negotiates_minor_versions_and_rejects_unknown_major() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "system.handshake".into(),
            params: Some(json!({ "protocolVersion": { "major": 1, "minor": 99 } })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["compatible"], true);
        assert_eq!(result["negotiated"], json!({ "major": 1, "minor": 0 }));

        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "system.handshake".into(),
            params: Some(json!({ "protocolVersion": { "major": 2, "minor": 0 } })),
        });
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[test]
    fn handshake_golden_fixture_matches_dispatch_response() {
        let request: JsonRpcRequest = serde_json::from_str(include_str!(
            "../../../tests/fixtures/system-handshake-request.json"
        ))
        .unwrap();
        let expected: JsonRpcResponse = serde_json::from_str(include_str!(
            "../../../tests/fixtures/system-handshake-response.json"
        ))
        .unwrap();
        assert_eq!(ControlPlane::default().dispatch(request), expected);
    }

    #[test]
    fn routes_inspect_dispatch_returns_desired_provenance() {
        let mut plane = ControlPlane::default();
        let graph = session();
        plane.insert_session(graph).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "routes.inspect".into(),
            params: Some(json!({
                "sessionId": "session",
                "destinationNode": "out"
            })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["reachable"], true);
        assert_eq!(result["paths"][0]["nodes"], json!(["in", "out"]));
        assert_eq!(result["paths"][0]["edges"], json!(["edge"]));
        assert_eq!(result["paths"][0]["channelMaps"], json!([[1.0]]));
    }

    #[test]
    fn graph_history_dispatch_returns_newest_snapshot_first() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "revision-one".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "history-api").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "graph.history".into(),
            params: Some(json!({ "sessionId": "session", "limit": 1 })),
        });
        let history = response.result.unwrap();
        assert_eq!(history["items"].as_array().unwrap().len(), 1);
        assert_eq!(history["items"][0]["revision"], 1);
        assert_eq!(history["items"][0]["name"], "revision-one");
        assert_eq!(history["nextCursor"], "1");
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "graph.history".into(),
            params: Some(json!({ "sessionId": "session", "cursor": "1", "limit": 1 })),
        });
        let history = response.result.unwrap();
        assert_eq!(history["items"][0]["revision"], 0);
        assert!(history["nextCursor"].is_null());
    }

    #[test]
    fn graph_undo_plan_dispatches_through_revision_checked_planning() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "revision-one".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "undo-api").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "graph.undoPlan".into(),
            params: Some(json!({ "sessionId": "session", "baseRevision": 1 })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["baseRevision"], 1);
        assert!(result["planId"].as_str().unwrap().starts_with("plan-"));
    }

    #[test]
    fn graph_undo_plan_hydrates_prior_history_after_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-undo-restart-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let mut plane = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
            let original = session();
            plane.insert_session(original.clone()).unwrap();
            let mut candidate = original.clone();
            candidate.name = "persisted-edit".into();
            let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
            plane.commit_graph(&plan, 0, "restart-undo").unwrap();
        }
        let mut restarted = ControlPlane::with_storage("restarted", Storage::open(&path).unwrap());
        let response = restarted.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "graph.undoPlan".into(),
            params: Some(json!({ "sessionId": "session", "baseRevision": 1 })),
        });
        assert!(response.result.unwrap()["planId"].as_str().is_some());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn events_subscribe_replays_filtered_control_state() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "evented".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "event-commit").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "events.subscribe".into(),
            params: Some(json!({ "sessionId": "session", "afterSequence": 0 })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["backendEpoch"], 1);
        assert_eq!(result["events"].as_array().unwrap().len(), 2);
        assert_eq!(result["events"][1]["operationId"], "event-commit");
    }

    #[test]
    fn events_subscribe_returns_snapshot_when_cursor_expired() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        for _ in 0..=audiorouter_domain::MAX_RETAINED_EVENTS {
            plane.events.append(0, None, "state.test", None);
        }
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "events.subscribe".into(),
            params: Some(json!({ "afterSequence": 1 })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["resyncRequired"], true);
        assert_eq!(result["snapshot"]["sessions"].as_array().unwrap().len(), 1);
        assert!(result["events"].as_array().unwrap().is_empty());
    }

    #[test]
    fn events_subscribe_requires_resync_when_backend_epoch_changes() {
        let mut plane = ControlPlane::default();
        plane.insert_session(session()).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(6)),
            method: "events.subscribe".into(),
            params: Some(json!({ "backendEpoch": 999, "afterSequence": 0 })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["resyncRequired"], true);
        assert_eq!(result["reason"], "backendEpochChanged");
        assert_eq!(result["backendEpoch"], 1);
        assert_eq!(result["snapshot"]["sessions"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn recordings_list_dispatch_exposes_storage_metadata_without_file_actions() {
        let storage = Storage::open_memory().unwrap();
        storage
            .save_recording(&audiorouter_storage::RecordingRecord {
                id: "recording-1".into(),
                session_id: "session".into(),
                recorder_id: "recorder".into(),
                path: "C:\\recordings\\one.wav".into(),
                format: "wav".into(),
                channels: 2,
                sample_rate: 48_000,
                frames: 96_000,
                file_bytes: 384_000,
                start_time: "2026-09-06T00:00:00Z".into(),
                state: "completed".into(),
                missing: false,
                title: Some("Test".into()),
                artist: None,
                comment: None,
            })
            .unwrap();
        let mut plane = ControlPlane::with_storage("recordings", storage);
        let denied = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(7)),
                method: "recordings.list".into(),
                params: Some(json!({ "sessionId": "session" })),
            },
            &ClientGrant::read_only(),
        );
        assert_eq!(
            denied.error.unwrap().data.unwrap()["code"],
            "permissionDenied"
        );
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(8)),
            method: "recordings.list".into(),
            params: Some(json!({ "sessionId": "session" })),
        });
        let result = response.result.unwrap();
        assert_eq!(result.as_array().unwrap().len(), 1);
        assert_eq!(result[0]["id"], "recording-1");
        assert_eq!(result[0]["missing"], false);
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(9)),
            method: "recordings.get".into(),
            params: Some(json!({ "recordingId": "recording-1" })),
        });
        assert_eq!(response.result.unwrap()["title"], "Test");
    }

    #[test]
    fn recording_metadata_mutation_requires_record_scope_and_preserves_file_path() {
        let storage = Storage::open_memory().unwrap();
        storage
            .save_recording(&audiorouter_storage::RecordingRecord {
                id: "recording-edit".into(),
                session_id: "session".into(),
                recorder_id: "recorder".into(),
                path: "C:\\recordings\\keep.wav".into(),
                format: "wav".into(),
                channels: 1,
                sample_rate: 44_100,
                frames: 10,
                file_bytes: 44,
                start_time: "2026-09-06T00:00:00Z".into(),
                state: "completed".into(),
                missing: false,
                title: None,
                artist: None,
                comment: None,
            })
            .unwrap();
        let mut plane = ControlPlane::with_storage("recording-edit", storage);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(10)),
            method: "recordings.setMetadata".into(),
            params: Some(json!({ "recordingId": "recording-edit", "title": "Edited" })),
        };
        assert!(plane
            .dispatch_authorized(request.clone(), &ClientGrant::read_only())
            .error
            .is_some());
        let response = plane.dispatch_authorized(
            request,
            &ClientGrant::with_scopes([PermissionScope::Record]),
        );
        assert_eq!(response.result.unwrap()["updated"], true);
        let record = plane
            .storage
            .as_ref()
            .unwrap()
            .get_recording("recording-edit")
            .unwrap()
            .unwrap();
        assert_eq!(record.path, "C:\\recordings\\keep.wav");
        assert_eq!(record.title.as_deref(), Some("Edited"));
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(11)),
                method: "recordings.removeEntry".into(),
                params: Some(json!({ "recordingId": "recording-edit" })),
            },
            &ClientGrant::with_scopes([PermissionScope::Record]),
        );
        assert_eq!(response.result.unwrap()["fileAction"], "none");
        assert!(plane
            .storage
            .as_ref()
            .unwrap()
            .get_recording("recording-edit")
            .unwrap()
            .is_none());
    }

    #[test]
    fn control_plane_uses_shared_plan_commit_authority() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "changed".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        let result = plane.commit_graph(&plan, 0, "op-1").unwrap();
        assert_eq!(result["revision"], 1);
        assert_eq!(plane.get_session(&original.id).unwrap().name, "changed");
    }

    #[test]
    fn dispatch_rejects_mutating_notifications_and_unknown_methods() {
        let mut plane = ControlPlane::default();
        let notification = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "graph.commit".into(),
            params: None,
        };
        assert_eq!(plane.dispatch(notification).error.unwrap().code, -32600);
        let unknown = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "no.such.method".into(),
            params: None,
        };
        assert_eq!(plane.dispatch(unknown).error.unwrap().code, -32601);
    }

    #[test]
    fn dispatch_plan_and_commit_use_json_contracts() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "via-api".into();
        let plan_request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "graph.plan".into(),
            params: Some(
                json!({ "sessionId": "session", "baseRevision": 0, "candidate": candidate }),
            ),
        };
        let plan_result = plane.dispatch(plan_request).result.unwrap();
        assert_eq!(plan_result["diff"][0]["path"], "/name");
        assert_eq!(plan_result["requiredScopes"], json!(["graph.write"]));
        let plan_id = plan_result["planId"].clone();
        let commit_request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "graph.commit".into(),
            params: Some(
                json!({ "planId": plan_id, "baseRevision": 0, "idempotencyKey": "api-op" }),
            ),
        };
        assert_eq!(
            plane.dispatch(commit_request).result.unwrap()["revision"],
            1
        );
    }

    #[test]
    fn fake_session_lifecycle_is_idempotent_and_stoppable() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let first = plane.session_start(&original.id).unwrap();
        assert_eq!(first["state"], "running");
        assert_eq!(first["generation"], 1);
        assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 1);
        assert_eq!(
            plane.session_stop(&original.id).unwrap()["state"],
            "stopped"
        );
        let events = plane.events.since(0, 10).unwrap();
        assert_eq!(events.last().unwrap().resource_revision, original.revision);
        assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 2);
    }

    #[test]
    fn commit_reactivates_a_running_fake_session_as_one_generation() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 1);
        let mut candidate = original.clone();
        candidate.name = "live-edit".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        let result = plane.commit_graph(&plan, 0, "live-edit-op").unwrap();
        assert_eq!(result["activation"]["state"], "running");
        assert_eq!(result["activation"]["generation"], 2);
        assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 2);
    }

    #[test]
    fn session_lifecycle_requires_an_existing_session() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "session.start".into(),
            params: Some(json!({ "sessionId": "missing" })),
        };
        assert_eq!(plane.dispatch(request).error.unwrap().code, -32602);
    }

    #[test]
    fn session_start_enforces_the_two_active_session_limit() {
        let mut plane = ControlPlane::default();
        for id in ["one", "two", "three"] {
            let mut graph = session();
            graph.id = EntityId::new(id);
            plane.insert_session(graph).unwrap();
        }
        plane.session_start(&EntityId::new("one")).unwrap();
        plane.session_start(&EntityId::new("two")).unwrap();
        assert!(matches!(
            plane.session_start(&EntityId::new("three")),
            Err(ControlError::InvalidRequest(message)) if message == "active session limit reached"
        ));
    }

    #[test]
    fn storage_backed_control_persists_session_and_commit() {
        let storage = Storage::open_memory().unwrap();
        let mut plane = ControlPlane::with_storage("persistent-test", storage);
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "persisted-change".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        let result = plane.commit_graph(&plan, 0, "persist-op").unwrap();
        assert_eq!(plane.get_session(&original.id).unwrap().revision, 1);
        assert!(result["revision"] == 1);
    }

    #[test]
    fn operations_get_returns_durable_commit_outcome() {
        let storage = Storage::open_memory().unwrap();
        let mut plane = ControlPlane::with_storage("operation-test", storage);
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "operation-change".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "operation-id").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(13)),
            method: "operations.get".into(),
            params: Some(json!({ "operationId": "operation-id" })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["status"], "completed");
        assert_eq!(result["durable"], true);
        assert_eq!(result["revision"], 1);
        assert_eq!(result["result"]["revision"], 1);
    }

    #[test]
    fn operations_get_returns_live_memory_outcome_without_claiming_durability() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "memory-operation".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "memory-operation-id").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(14)),
            method: "operations.get".into(),
            params: Some(json!({ "operationId": "memory-operation-id" })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["status"], "completed");
        assert_eq!(result["durable"], false);
        assert_eq!(result["result"]["revision"], 1);
    }

    #[test]
    fn operations_cancel_reports_completed_operations_without_undoing_them() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "cancel-check".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "cancel-check-key").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(16)),
            method: "operations.cancel".into(),
            params: Some(json!({ "operationId": "cancel-check-key" })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["status"], "completed");
        assert_eq!(result["cancelled"], false);
        assert_eq!(result["reason"], "alreadyCompleted");
        assert_eq!(plane.get_session(&original.id).unwrap().revision, 1);
    }

    #[test]
    fn graph_commit_acknowledgments_are_validated_and_cannot_grant_scope() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(17)),
            method: "graph.commit".into(),
            params: Some(json!({
                "planId": "plan-1",
                "baseRevision": 0,
                "idempotencyKey": "ack-test",
                "acknowledgments": ["unverified-feedback"]
            })),
        });
        assert!(response.error.unwrap().message.contains("no warnings"));

        let invalid = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(18)),
            method: "graph.commit".into(),
            params: Some(json!({
                "planId": "plan-1",
                "baseRevision": 0,
                "idempotencyKey": "ack-test",
                "acknowledgments": [12]
            })),
        });
        assert!(invalid.error.unwrap().message.contains("warning IDs"));
    }

    #[test]
    fn durable_commit_replays_before_plan_lookup_after_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-idempotency-restart-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let original = session();
        let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
        first.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "durable-change".into();
        let plan = first.plan_graph(&original.id, 0, candidate).unwrap();
        first.commit_graph(&plan, 0, "restart-key").unwrap();
        drop(first);

        let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
        let replay = second.commit_graph(&plan, 0, "restart-key").unwrap();
        assert_eq!(replay["idempotentReplay"], true);
        assert_eq!(replay["revision"], 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn durable_uncommitted_plan_survives_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-plan-restart-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let original = session();
        let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
        first.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "survives-restart".into();
        let plan = first.plan_graph(&original.id, 0, candidate).unwrap();
        drop(first);

        let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
        let committed = second.commit_graph(&plan, 0, "restart-plan-key").unwrap();
        assert_eq!(committed["revision"], 1);
        assert_eq!(committed["idempotentReplay"], false);
        assert_eq!(
            second
                .dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(15)),
                    method: "sessions.get".into(),
                    params: Some(json!({ "sessionId": "session" })),
                })
                .result
                .unwrap()["name"],
            "survives-restart"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn framed_request_round_trips_through_dispatch() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(9)),
            method: "system.describe".into(),
            params: None,
        };
        let frame = audiorouter_protocol::encode_frame(&request).unwrap();
        let responses = plane.dispatch_frame(&frame).unwrap();
        let response: JsonRpcResponse = audiorouter_protocol::decode_frame(&responses[0]).unwrap();
        assert_eq!(response.id, Some(json!(9)));
        assert!(response.result.unwrap()["methods"].is_array());
    }

    #[test]
    fn scoped_authorization_denies_mutation_before_dispatch() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "graph.commit".into(),
            params: None,
        };
        let response = plane.dispatch_authorized(request, &ClientGrant::read_only());
        assert_eq!(response.error.unwrap().code, -32001);
    }

    #[test]
    fn scoped_authorization_allows_discovery_read() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "system.describe".into(),
            params: None,
        };
        let response = plane.dispatch_authorized(request, &ClientGrant::read_only());
        assert!(response.result.unwrap()["methods"].is_array());
    }

    #[test]
    fn authorized_framed_dispatch_denies_mutation_before_parameter_parsing() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(6)),
            method: "graph.commit".into(),
            params: None,
        };
        let frame = audiorouter_protocol::encode_frame(&request).unwrap();
        let responses = plane
            .dispatch_frame_authorized(&frame, &ClientGrant::read_only())
            .unwrap();
        let response: JsonRpcResponse = audiorouter_protocol::decode_frame(&responses[0]).unwrap();
        assert_eq!(response.error.unwrap().code, -32001);
    }

    #[test]
    fn authorized_batch_preserves_allowed_and_denied_responses_in_order() {
        let mut plane = ControlPlane::default();
        let message = RpcMessage::Batch(vec![
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "system.describe".into(),
                params: None,
            },
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "graph.commit".into(),
                params: None,
            },
        ]);
        let responses = plane.dispatch_message_authorized(message, &ClientGrant::read_only());
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].id, Some(json!(1)));
        assert!(responses[0].result.is_some());
        assert_eq!(responses[1].id, Some(json!(2)));
        assert_eq!(responses[1].error.as_ref().unwrap().code, -32001);
    }

    #[test]
    fn built_in_roles_are_deny_by_default_for_sensitive_scopes() {
        assert!(ClientGrant::for_role(ClientRole::Observer).allows(PermissionScope::Read));
        assert!(!ClientGrant::for_role(ClientRole::Observer).allows(PermissionScope::GraphWrite));
        assert!(ClientGrant::for_role(ClientRole::Editor).allows(PermissionScope::GraphWrite));
        assert!(!ClientGrant::for_role(ClientRole::Editor).allows(PermissionScope::SessionControl));
        assert!(ClientGrant::for_role(ClientRole::Operator).allows(PermissionScope::SessionControl));
        assert!(!ClientGrant::for_role(ClientRole::Operator).allows(PermissionScope::Capture));
        assert!(!ClientGrant::for_role(ClientRole::Operator)
            .allows(PermissionScope::DeviceAdministration));
    }

    #[test]
    fn enrollment_lookup_denies_unknown_and_revoked_clients() {
        let mut plane = ControlPlane::new("enrollment-test");
        assert!(plane.grant_for_client("unknown").unwrap().is_none());
        plane.enroll_client("client", ClientRole::Editor).unwrap();
        let grant = plane.grant_for_client("client").unwrap().unwrap();
        assert!(grant.allows(PermissionScope::GraphWrite));
        assert!(!grant.allows(PermissionScope::SessionControl));
        assert!(plane.revoke_client("client").unwrap());
        assert!(plane.grant_for_client("client").unwrap().is_none());
        assert!(!plane.revoke_client("client").unwrap());
    }

    #[test]
    fn storage_backed_enrollment_persists_and_revokes() {
        let storage = Storage::open_memory().unwrap();
        let mut first = ControlPlane::with_storage("enrollment-persist", storage);
        first
            .enroll_client("operator", ClientRole::Operator)
            .unwrap();
        assert!(first.grant_for_client("operator").unwrap().is_some());
        assert!(first.revoke_client("operator").unwrap());
        assert!(first.grant_for_client("operator").unwrap().is_none());
    }

    #[test]
    fn read_only_notifications_produce_no_response() {
        let mut plane = ControlPlane::default();
        let notification = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "system.describe".into(),
            params: None,
        };
        assert!(plane
            .dispatch_message(RpcMessage::Single(notification))
            .is_empty());
    }
}
