//! Portable control-plane façade for M01.
//!
//! Transport, authorization, and durable storage are deliberately separate
//! follow-up layers. This façade proves that all adapters can share one domain
//! authority and that unsupported audio capabilities are discoverable.

use audiorouter_domain::{
    node_registry, ApiMethodSpec, EntityId, FakeRuntime, GraphStore, RuntimeError, RuntimeState,
    Session, API_METHODS,
};
use audiorouter_protocol::{JsonRpcRequest, JsonRpcResponse, RpcMessage};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

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
        }
    }

    pub fn insert_session(&mut self, session: Session) -> Result<(), ControlError> {
        self.store.insert_session(session).map_err(Into::into)
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
        json!({ "protocolVersion": { "major": 1, "minor": 0 }, "schemaVersion": 1, "build": self.build, "methods": methods, "nodeTypes": nodes, "limits": { "maxNodesPerSession": audiorouter_domain::MAX_NODES_PER_SESSION, "maxEdgesPerSession": audiorouter_domain::MAX_EDGES_PER_SESSION } })
    }

    pub fn get_session(&self, id: &EntityId) -> Result<&Session, ControlError> {
        self.store
            .session(id)
            .ok_or(ControlError::InvalidRequest("session not found".into()))
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
        let result = self
            .store
            .commit_graph(plan_id, base_revision, idempotency_key)?;
        serde_json::to_value(result).map_err(|error| ControlError::Json(error.to_string()))
    }

    pub fn session_start(&mut self, id: &EntityId) -> Result<Value, ControlError> {
        let session = self.get_session(id)?.clone();
        let runtime = self.runtimes.entry(id.clone()).or_default();
        if runtime.state() == RuntimeState::Running {
            return Ok(
                json!({ "sessionId": id, "state": "running", "generation": runtime.generation(), "runtime": "fake" }),
            );
        }
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
        Ok(
            json!({ "sessionId": id, "state": "running", "generation": generation, "runtime": "fake" }),
        )
    }

    pub fn session_stop(&mut self, id: &EntityId) -> Result<Value, ControlError> {
        self.get_session(id)?;
        if let Some(runtime) = self.runtimes.get_mut(id) {
            runtime.stop();
        }
        Ok(json!({ "sessionId": id, "state": "stopped", "runtime": "fake" }))
    }

    pub fn dispatch(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();
        if request.validate().is_err() {
            return JsonRpcResponse::failure(id, -32600, "invalid request");
        }
        let mutating = matches!(
            request.method.as_str(),
            "graph.plan" | "graph.commit" | "session.start" | "session.stop"
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
                json!({ "build": self.build, "audio": "unavailable", "reason": "M02 Windows audio adapters not implemented" }),
            ),
            "devices.list" | "apps.list" => Ok(json!([])),
            "nodes.types" => Ok(self.describe()["nodeTypes"].clone()),
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

    pub fn dispatch_message(&mut self, message: RpcMessage) -> Vec<JsonRpcResponse> {
        match message {
            RpcMessage::Single(request) => vec![self.dispatch(request)],
            RpcMessage::Batch(requests) => requests
                .into_iter()
                .map(|request| self.dispatch(request))
                .collect(),
        }
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
        let plan_id = self.plan_graph(&session_id, base_revision, candidate)?;
        Ok(json!({ "planId": plan_id, "baseRevision": base_revision, "expiresInMs": 30000 }))
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

    fn dispatch_session_stop(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let id = session_id_from_params(params)?;
        self.session_stop(&id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_domain::{Edge, Node, NodeKind, Port, PortDirection};

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
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "graph.plan"));
        assert!(description["nodeTypes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["type"] == "physical-input@1"
                && node["availability"]["status"] == "unavailable"));
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
        let plan_id = plane.dispatch(plan_request).result.unwrap()["planId"].clone();
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
}
