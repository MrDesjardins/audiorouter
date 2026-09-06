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
use audiorouter_storage::{Storage, StorageError};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

const MUTATION_RATE_PER_SECOND: f64 = 20.0;
const MUTATION_BURST: f64 = 40.0;

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
pub struct MethodDescription {
    pub name: &'static str,
    pub permission: audiorouter_domain::PermissionScope,
    pub side_effect: audiorouter_domain::SideEffectClass,
}

impl From<ApiMethodSpec> for MethodDescription {
    fn from(spec: ApiMethodSpec) -> Self {
        Self {
            name: spec.name,
            permission: spec.permission,
            side_effect: spec.side_effect,
        }
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
        }
    }

    pub fn with_storage(build: impl Into<String>, storage: Storage) -> Self {
        Self {
            store: GraphStore::default(),
            build: build.into(),
            runtimes: HashMap::new(),
            storage: Some(storage),
            enrollments: HashMap::new(),
            events: EventLog::new(1),
            mutation_limiter: MutationRateLimiter::default(),
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

    pub fn describe(&self) -> Value {
        let methods: Vec<MethodDescription> = API_METHODS.iter().copied().map(Into::into).collect();
        let nodes: Vec<Value> = node_registry().into_iter().map(|spec| {
            let availability = match spec.availability {
                audiorouter_domain::CapabilityAvailability::Available => json!({ "status": "available" }),
                audiorouter_domain::CapabilityAvailability::Unavailable(reason) => json!({ "status": "unavailable", "reason": reason }),
            };
            json!({ "type": format!("{}@{}", spec.kind.type_name(), spec.version), "availability": availability, "realtimeCostClass": spec.realtime_cost_class })
        }).collect();
        json!({ "protocolVersion": { "major": 1, "minor": 0 }, "schemaVersion": 1, "build": self.build, "methods": methods, "nodeTypes": nodes, "limits": { "maxNodesPerSession": audiorouter_domain::MAX_NODES_PER_SESSION, "maxEdgesPerSession": audiorouter_domain::MAX_EDGES_PER_SESSION, "maxNodesGlobal": audiorouter_domain::MAX_NODES_GLOBAL, "maxEdgesGlobal": audiorouter_domain::MAX_EDGES_GLOBAL, "maxActiveSessions": audiorouter_domain::MAX_ACTIVE_SESSIONS } })
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
        let limit = limit.clamp(1, 500);
        let history = if self.store.session(session_id).is_some() {
            self.store.history(session_id, limit)
        } else if let Some(storage) = &self.storage {
            storage
                .load_history(session_id, limit)
                .map_err(storage_error)?
        } else {
            Vec::new()
        };
        serde_json::to_value(history).map_err(|error| ControlError::Json(error.to_string()))
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
        if !(1..=500).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        let sessions = if let Some(storage) = &self.storage {
            storage.list_sessions(limit).map_err(storage_error)?
        } else {
            self.store.sessions(limit)
        };
        serde_json::to_value(sessions).map_err(|error| ControlError::Json(error.to_string()))
    }

    pub fn plan_graph(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
        candidate: Session,
    ) -> Result<EntityId, ControlError> {
        self.store
            .plan_graph(session_id, base_revision, candidate)
            .map_err(Into::into)
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
        let result = self
            .store
            .commit_graph(plan_id, base_revision, idempotency_key)?;
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
        let mutating = matches!(
            request.method.as_str(),
            "graph.plan" | "graph.undoPlan" | "graph.commit" | "session.start" | "session.stop"
        );
        if request.is_notification() && mutating {
            return JsonRpcResponse::failure(
                None,
                -32600,
                "mutating notifications are not supported",
            );
        }
        let result = match request.method.as_str() {
            "system.describe" => Ok(self.describe()),
            "status.get" => Ok(
                json!({ "build": self.build, "audio": "unavailable", "deviceDiscovery": "available", "reason": "M02 realtime graph engine and routing are not implemented" }),
            ),
            "devices.list" => self.dispatch_devices_list(),
            "apps.list" => self.dispatch_apps_list(),
            "nodes.types" => Ok(self.describe()["nodeTypes"].clone()),
            "nodes.describe" => Ok(self.describe()["nodeTypes"].clone()),
            "sessions.get" => self.dispatch_session_get(request.params),
            "sessions.list" => self.dispatch_sessions_list(request.params),
            "routes.inspect" => self.dispatch_routes_inspect(request.params),
            "graph.history" => self.dispatch_graph_history(request.params),
            "graph.undoPlan" => self.dispatch_graph_undo_plan(request.params),
            "events.subscribe" => self.dispatch_events_subscribe(request.params),
            "session.start" => self.dispatch_session_start(request.params),
            "session.stop" => self.dispatch_session_stop(request.params),
            "graph.plan" => self.dispatch_plan(request.params),
            "graph.commit" => self.dispatch_commit(request.params),
            _ => Err(ControlError::InvalidRequest("method not found".into())),
        };
        match result {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(ControlError::InvalidRequest(message)) if message == "method not found" => {
                JsonRpcResponse::failure(id, -32601, message)
            }
            Err(ControlError::InvalidRequest(message)) => {
                JsonRpcResponse::failure(id, -32602, message)
            }
            Err(error) => JsonRpcResponse::failure(id, -32000, format!("{error:?}")),
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
            return JsonRpcResponse::failure(
                id,
                -32001,
                format!("permission denied: {:?}", spec.permission),
            );
        }
        if is_mutating_method(&request.method) {
            if let Some(client_id) = client_id {
                if let Err(retry_after_ms) = self.mutation_limiter.allow(client_id) {
                    let mut response = JsonRpcResponse::failure(id, -32000, "rate limited");
                    if let Some(error) = response.error.as_mut() {
                        error.data = Some(json!({
                            "code": "rateLimited",
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
                let omit = request.is_notification()
                    && !matches!(
                        request.method.as_str(),
                        "graph.plan"
                            | "graph.undoPlan"
                            | "graph.commit"
                            | "session.start"
                            | "session.stop"
                    );
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
                    let omit = request.is_notification()
                        && !matches!(
                            request.method.as_str(),
                            "graph.plan"
                                | "graph.undoPlan"
                                | "graph.commit"
                                | "session.start"
                                | "session.stop"
                        );
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
                            | "session.stop"
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
                                | "session.stop"
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
            "expiresInMs": 30000,
            "diff": diff,
            "affectedDestinations": affected_destinations,
            "warnings": [],
            "requiredScopes": ["graph.write"]
        }))
    }

    fn dispatch_commit(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("graph.commit params are required".into())
        })?;
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

    fn dispatch_sessions_list(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let limit = params
            .as_ref()
            .and_then(|value| value.get("limit"))
            .and_then(Value::as_u64)
            .unwrap_or(100);
        self.sessions_list(limit as usize)
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
        if limit == 0 || limit > 500 {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        self.graph_history(&session_id, limit as usize)
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
        Ok(json!({ "planId": plan_id, "baseRevision": base_revision, "expiresInMs": 30000 }))
    }

    fn dispatch_events_subscribe(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
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

    fn dispatch_apps_list(&self) -> Result<Value, ControlError> {
        let applications = audiorouter_windows_audio::enumerate_applications()
            .map_err(|error| ControlError::InvalidRequest(error.to_string()))?;
        Ok(json!(applications
            .into_iter()
            .map(|application| json!({
                "processId": application.process_id,
                "executable": application.executable,
                "creationTime100ns": application.creation_time_100ns.map(|value| value.to_string()),
            }))
            .collect::<Vec<_>>()))
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
        "graph.plan" | "graph.undoPlan" | "graph.commit" | "session.start" | "session.stop"
    )
}

fn storage_error(error: StorageError) -> ControlError {
    ControlError::Storage(format!("{error:?}"))
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
        assert!(description["nodeTypes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["type"] == "physical-input@1"
                && node["availability"]["status"] == "unavailable"));
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
        assert_eq!(history.as_array().unwrap().len(), 1);
        assert_eq!(history[0]["revision"], 1);
        assert_eq!(history[0]["name"], "revision-one");
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
