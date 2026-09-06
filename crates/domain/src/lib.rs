//! Portable AudioRouter domain contracts and graph validation.
//!
//! This crate deliberately contains no Windows, audio, IPC, filesystem, or
//! realtime code. It is the authority for the in-memory graph shape used by
//! later control-plane adapters.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

pub const MAX_NODES_PER_SESSION: usize = 64;
pub const MAX_EDGES_PER_SESSION: usize = 128;
pub const MAX_NODES_GLOBAL: usize = 128;
pub const MAX_EDGES_GLOBAL: usize = 256;
pub const MAX_ACTIVE_SESSIONS: usize = 2;
pub const MAX_RETAINED_EVENTS: usize = 10_000;
const EVENT_RETENTION: Duration = Duration::from_secs(15 * 60);

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityId(String);

impl EntityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    PhysicalInput,
    ApplicationCapture,
    EndpointLoopback,
    PhysicalOutput,
    Mixer,
    Gain,
    Mute,
    Meter,
}

impl NodeKind {
    pub const ALL: [Self; 8] = [
        Self::PhysicalInput,
        Self::ApplicationCapture,
        Self::EndpointLoopback,
        Self::PhysicalOutput,
        Self::Mixer,
        Self::Gain,
        Self::Mute,
        Self::Meter,
    ];

    pub fn type_name(self) -> &'static str {
        match self {
            Self::PhysicalInput => "physical-input",
            Self::ApplicationCapture => "application-capture",
            Self::EndpointLoopback => "endpoint-loopback",
            Self::PhysicalOutput => "physical-output",
            Self::Mixer => "mixer",
            Self::Gain => "gain",
            Self::Mute => "mute",
            Self::Meter => "meter",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityAvailability {
    Available,
    Unavailable(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeTypeSpec {
    pub kind: NodeKind,
    pub version: u32,
    pub availability: CapabilityAvailability,
    pub realtime_cost_class: &'static str,
}

pub fn node_registry() -> [NodeTypeSpec; 8] {
    NodeKind::ALL.map(|kind| NodeTypeSpec {
        kind,
        version: 1,
        availability: match kind {
            NodeKind::Mixer | NodeKind::Gain | NodeKind::Mute | NodeKind::Meter => {
                CapabilityAvailability::Available
            }
            _ => CapabilityAvailability::Unavailable("requires M02 Windows audio adapters"),
        },
        realtime_cost_class: match kind {
            NodeKind::Mixer | NodeKind::Gain | NodeKind::Mute | NodeKind::Meter => "low",
            _ => "device-bound",
        },
    })
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionScope {
    Read,
    GraphWrite,
    SessionControl,
    Capture,
    Record,
    DeviceAdministration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SideEffectClass {
    ReadOnly,
    PlanOnly,
    Mutating,
    ExternalOperation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMethodSpec {
    pub name: &'static str,
    pub permission: PermissionScope,
    pub side_effect: SideEffectClass,
}

pub const API_METHODS: [ApiMethodSpec; 32] = [
    ApiMethodSpec {
        name: "system.describe",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "system.handshake",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "status.get",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "system.diagnostics",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "clients.list",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "clients.authorize",
        permission: PermissionScope::DeviceAdministration,
        side_effect: SideEffectClass::Mutating,
    },
    ApiMethodSpec {
        name: "clients.revoke",
        permission: PermissionScope::DeviceAdministration,
        side_effect: SideEffectClass::Mutating,
    },
    ApiMethodSpec {
        name: "operations.get",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "recordings.list",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "recordings.get",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "recordings.setMetadata",
        permission: PermissionScope::Record,
        side_effect: SideEffectClass::Mutating,
    },
    ApiMethodSpec {
        name: "recordings.removeEntry",
        permission: PermissionScope::Record,
        side_effect: SideEffectClass::Mutating,
    },
    ApiMethodSpec {
        name: "devices.list",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "apps.list",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "applications.list",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "nodes.types",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "routes.inspect",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "graph.history",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "graph.undoPlan",
        permission: PermissionScope::GraphWrite,
        side_effect: SideEffectClass::PlanOnly,
    },
    ApiMethodSpec {
        name: "events.subscribe",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "nodes.describe",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "sessions.get",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "sessions.list",
        permission: PermissionScope::Read,
        side_effect: SideEffectClass::ReadOnly,
    },
    ApiMethodSpec {
        name: "sessions.create",
        permission: PermissionScope::GraphWrite,
        side_effect: SideEffectClass::Mutating,
    },
    ApiMethodSpec {
        name: "sessions.duplicate",
        permission: PermissionScope::GraphWrite,
        side_effect: SideEffectClass::Mutating,
    },
    ApiMethodSpec {
        name: "sessions.delete",
        permission: PermissionScope::GraphWrite,
        side_effect: SideEffectClass::Mutating,
    },
    ApiMethodSpec {
        name: "graph.plan",
        permission: PermissionScope::GraphWrite,
        side_effect: SideEffectClass::PlanOnly,
    },
    ApiMethodSpec {
        name: "graph.commit",
        permission: PermissionScope::GraphWrite,
        side_effect: SideEffectClass::Mutating,
    },
    ApiMethodSpec {
        name: "session.start",
        permission: PermissionScope::SessionControl,
        side_effect: SideEffectClass::ExternalOperation,
    },
    ApiMethodSpec {
        name: "sessions.start",
        permission: PermissionScope::SessionControl,
        side_effect: SideEffectClass::ExternalOperation,
    },
    ApiMethodSpec {
        name: "session.stop",
        permission: PermissionScope::SessionControl,
        side_effect: SideEffectClass::ExternalOperation,
    },
    ApiMethodSpec {
        name: "sessions.stop",
        permission: PermissionScope::SessionControl,
        side_effect: SideEffectClass::ExternalOperation,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Port {
    pub name: String,
    pub direction: PortDirection,
    pub channels: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: EntityId,
    pub kind: NodeKind,
    pub name: String,
    pub enabled: bool,
    pub bypass: bool,
    pub ports: Vec<Port>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub id: EntityId,
    pub source_node: EntityId,
    pub source_port: String,
    pub destination_node: EntityId,
    pub destination_port: String,
    pub matrix: Vec<f32>,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: EntityId,
    pub name: String,
    pub schema_version: u32,
    pub revision: u64,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    EmptyId {
        path: String,
    },
    DuplicateId {
        path: String,
        id: String,
    },
    LimitExceeded {
        path: String,
        requested: usize,
        maximum: usize,
    },
    MissingNode {
        path: String,
        id: String,
    },
    MissingPort {
        path: String,
        node: String,
        port: String,
    },
    WrongDirection {
        path: String,
    },
    InvalidChannels {
        path: String,
        channels: u8,
    },
    InvalidMatrix {
        path: String,
    },
    DuplicateEdge {
        path: String,
    },
    MultipleInputEdges {
        path: String,
    },
    Cycle {
        path: String,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub fn validate_session(session: &Session) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    if session.nodes.len() > MAX_NODES_PER_SESSION {
        errors.push(ValidationError::LimitExceeded {
            path: "nodes".into(),
            requested: session.nodes.len(),
            maximum: MAX_NODES_PER_SESSION,
        });
    }
    if session.edges.len() > MAX_EDGES_PER_SESSION {
        errors.push(ValidationError::LimitExceeded {
            path: "edges".into(),
            requested: session.edges.len(),
            maximum: MAX_EDGES_PER_SESSION,
        });
    }
    let mut nodes = HashMap::new();
    for (index, node) in session.nodes.iter().enumerate() {
        let path = format!("nodes[{index}]");
        if node.id.as_str().is_empty() {
            errors.push(ValidationError::EmptyId {
                path: format!("{path}.id"),
            });
        }
        if nodes.insert(node.id.clone(), node).is_some() {
            errors.push(ValidationError::DuplicateId {
                path: format!("{path}.id"),
                id: node.id.as_str().into(),
            });
        }
        for port in &node.ports {
            if !(1..=2).contains(&port.channels) {
                errors.push(ValidationError::InvalidChannels {
                    path: format!("{path}.ports.{}", port.name),
                    channels: port.channels,
                });
            }
        }
    }
    let mut edge_keys = HashSet::new();
    let mut input_counts = HashMap::<(EntityId, String), usize>::new();
    let mut adjacency = HashMap::<EntityId, Vec<EntityId>>::new();
    for (index, edge) in session.edges.iter().enumerate() {
        let path = format!("edges[{index}]");
        if edge.id.as_str().is_empty() {
            errors.push(ValidationError::EmptyId {
                path: format!("{path}.id"),
            });
        }
        let source = nodes.get(&edge.source_node);
        let destination = nodes.get(&edge.destination_node);
        let (Some(source), Some(destination)) = (source, destination) else {
            if source.is_none() {
                errors.push(ValidationError::MissingNode {
                    path: format!("{path}.sourceNode"),
                    id: edge.source_node.as_str().into(),
                });
            }
            if destination.is_none() {
                errors.push(ValidationError::MissingNode {
                    path: format!("{path}.destinationNode"),
                    id: edge.destination_node.as_str().into(),
                });
            }
            continue;
        };
        let source_port = source.ports.iter().find(|p| p.name == edge.source_port);
        let destination_port = destination
            .ports
            .iter()
            .find(|p| p.name == edge.destination_port);
        if source_port.is_none() {
            errors.push(ValidationError::MissingPort {
                path: format!("{path}.sourcePort"),
                node: edge.source_node.as_str().into(),
                port: edge.source_port.clone(),
            });
        }
        if destination_port.is_none() {
            errors.push(ValidationError::MissingPort {
                path: format!("{path}.destinationPort"),
                node: edge.destination_node.as_str().into(),
                port: edge.destination_port.clone(),
            });
        }
        if let (Some(source_port), Some(destination_port)) = (source_port, destination_port) {
            if source_port.direction != PortDirection::Output
                || destination_port.direction != PortDirection::Input
            {
                errors.push(ValidationError::WrongDirection { path: path.clone() });
            }
            let expected = destination_port.channels as usize * source_port.channels as usize;
            if edge.matrix.len() != expected
                || edge
                    .matrix
                    .iter()
                    .any(|value| !value.is_finite() || !(-2.0..=2.0).contains(value))
            {
                errors.push(ValidationError::InvalidMatrix {
                    path: format!("{path}.matrix"),
                });
            }
        }
        let key = (
            edge.source_node.clone(),
            edge.source_port.clone(),
            edge.destination_node.clone(),
            edge.destination_port.clone(),
        );
        if !edge_keys.insert(key) {
            errors.push(ValidationError::DuplicateEdge { path: path.clone() });
        }
        let input_key = (edge.destination_node.clone(), edge.destination_port.clone());
        let count = input_counts.entry(input_key).or_default();
        *count += 1;
        if *count > 1 && destination.kind != NodeKind::Mixer {
            errors.push(ValidationError::MultipleInputEdges { path });
        }
        adjacency
            .entry(edge.source_node.clone())
            .or_default()
            .push(edge.destination_node.clone());
    }
    if has_cycle(&adjacency) {
        errors.push(ValidationError::Cycle {
            path: "edges".into(),
        });
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePath {
    pub nodes: Vec<EntityId>,
    pub edges: Vec<EntityId>,
    pub channel_maps: Vec<Vec<f32>>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteInspection {
    pub destination_node: EntityId,
    pub reachable: bool,
    pub paths: Vec<RoutePath>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateEvent {
    pub sequence: u64,
    pub backend_epoch: u64,
    pub resource_revision: u64,
    pub operation_id: Option<String>,
    pub category: String,
    pub session_id: Option<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventReplayError {
    InvalidLimit,
    ResyncRequired,
}

/// Bounded state-event history for reconnecting clients. Meter data is not
/// stored here; callers append only durable/control state transitions.
#[derive(Clone, Debug)]
pub struct EventLog {
    backend_epoch: u64,
    next_sequence: u64,
    events: VecDeque<RetainedEvent>,
}

#[derive(Clone, Debug)]
struct RetainedEvent {
    event: StateEvent,
    retained_at: Instant,
}

impl EventLog {
    pub fn new(backend_epoch: u64) -> Self {
        Self {
            backend_epoch,
            next_sequence: 1,
            events: VecDeque::new(),
        }
    }

    pub fn backend_epoch(&self) -> u64 {
        self.backend_epoch
    }

    pub fn latest_sequence(&self) -> u64 {
        self.next_sequence.saturating_sub(1)
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn append(
        &mut self,
        resource_revision: u64,
        operation_id: Option<String>,
        category: impl Into<String>,
        session_id: Option<EntityId>,
    ) -> u64 {
        self.append_at(
            resource_revision,
            operation_id,
            category,
            session_id,
            Instant::now(),
        )
    }

    fn append_at(
        &mut self,
        resource_revision: u64,
        operation_id: Option<String>,
        category: impl Into<String>,
        session_id: Option<EntityId>,
        retained_at: Instant,
    ) -> u64 {
        self.prune_expired(retained_at);
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.events.push_back(RetainedEvent {
            event: StateEvent {
                sequence,
                backend_epoch: self.backend_epoch,
                resource_revision,
                operation_id,
                category: category.into(),
                session_id,
            },
            retained_at,
        });
        while self.events.len() > MAX_RETAINED_EVENTS {
            self.events.pop_front();
        }
        sequence
    }

    fn prune_expired(&mut self, now: Instant) {
        while self
            .events
            .front()
            .map(|event| now.saturating_duration_since(event.retained_at) >= EVENT_RETENTION)
            .unwrap_or(false)
        {
            self.events.pop_front();
        }
    }

    pub fn since(
        &mut self,
        after_sequence: u64,
        limit: usize,
    ) -> Result<Vec<StateEvent>, EventReplayError> {
        if !(1..=500).contains(&limit) {
            return Err(EventReplayError::InvalidLimit);
        }
        self.prune_expired(Instant::now());
        if let Some(first) = self.events.front() {
            if after_sequence.saturating_add(1) < first.event.sequence {
                return Err(EventReplayError::ResyncRequired);
            }
        }
        Ok(self
            .events
            .iter()
            .filter(|event| event.event.sequence > after_sequence)
            .take(limit)
            .map(|event| event.event.clone())
            .collect())
    }
}

/// Inspect desired upstream provenance for one destination without opening or
/// activating any runtime resource. Disabled edges are excluded, while
/// disabled nodes remain visible in the returned desired path.
pub fn inspect_routes(
    session: &Session,
    destination_node: &EntityId,
) -> Result<RouteInspection, Vec<ValidationError>> {
    validate_session(session)?;
    if !session
        .nodes
        .iter()
        .any(|node| node.id == *destination_node)
    {
        return Err(vec![ValidationError::MissingNode {
            path: "destinationNode".into(),
            id: destination_node.as_str().into(),
        }]);
    }
    let mut incoming = HashMap::<EntityId, Vec<&Edge>>::new();
    for edge in session.edges.iter().filter(|edge| edge.enabled) {
        incoming
            .entry(edge.destination_node.clone())
            .or_default()
            .push(edge);
    }
    fn walk(
        node: &EntityId,
        incoming: &HashMap<EntityId, Vec<&Edge>>,
        nodes: &mut Vec<EntityId>,
        edges: &mut Vec<EntityId>,
        channel_maps: &mut Vec<Vec<f32>>,
        paths: &mut Vec<RoutePath>,
    ) {
        let Some(parents) = incoming.get(node) else {
            paths.push(RoutePath {
                nodes: nodes.iter().rev().cloned().collect(),
                edges: edges.iter().rev().cloned().collect(),
                channel_maps: channel_maps.iter().rev().cloned().collect(),
            });
            return;
        };
        for edge in parents {
            nodes.push(edge.source_node.clone());
            edges.push(edge.id.clone());
            channel_maps.push(edge.matrix.clone());
            walk(
                &edge.source_node,
                incoming,
                nodes,
                edges,
                channel_maps,
                paths,
            );
            nodes.pop();
            edges.pop();
            channel_maps.pop();
        }
    }
    let mut nodes = vec![destination_node.clone()];
    let mut edges = Vec::new();
    let mut channel_maps = Vec::new();
    let mut paths = Vec::new();
    walk(
        destination_node,
        &incoming,
        &mut nodes,
        &mut edges,
        &mut channel_maps,
        &mut paths,
    );
    Ok(RouteInspection {
        destination_node: destination_node.clone(),
        reachable: paths.iter().any(|path| path.nodes.len() > 1),
        paths,
    })
}

fn has_cycle(adjacency: &HashMap<EntityId, Vec<EntityId>>) -> bool {
    fn visit(
        node: &EntityId,
        graph: &HashMap<EntityId, Vec<EntityId>>,
        visiting: &mut HashSet<EntityId>,
        visited: &mut HashSet<EntityId>,
    ) -> bool {
        if visiting.contains(node) {
            return true;
        }
        if !visited.insert(node.clone()) {
            return false;
        }
        visiting.insert(node.clone());
        let cycle = graph
            .get(node)
            .into_iter()
            .flatten()
            .any(|child| visit(child, graph, visiting, visited));
        visiting.remove(node);
        cycle
    }
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    adjacency
        .keys()
        .any(|node| visit(node, adjacency, &mut visiting, &mut visited))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RuntimeState {
    #[default]
    Stopped,
    Preparing,
    Running,
    Failed,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RuntimeError {
    InvalidGraph(Vec<ValidationError>),
    NotPrepared,
}

/// Deterministic control-plane runtime used by M01 tests. It never opens an
/// endpoint and has no audio callback; M02 owns the real adapter.
#[derive(Debug, Default)]
pub struct FakeRuntime {
    state: RuntimeState,
    generation: u64,
    prepared_revision: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreError {
    SessionNotFound,
    PlanNotFound,
    PlanExpired,
    InvalidGraph(Vec<ValidationError>),
    RevisionConflict { expected: u64, actual: u64 },
    EmptyIdempotencyKey,
    NoUndoAvailable,
    IdempotencyConflict,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CommitResult {
    pub session_id: EntityId,
    pub revision: u64,
    pub idempotent_replay: bool,
}

#[derive(Clone, Debug)]
struct GraphPlan {
    session_id: EntityId,
    base_revision: u64,
    candidate: Session,
    expires_at: std::time::Instant,
}

/// In-memory revision store used to prove M01 transaction semantics before
/// SQLite and the named-pipe control process are introduced.
#[derive(Clone, Debug, Default)]
pub struct GraphStore {
    sessions: HashMap<EntityId, Session>,
    history: HashMap<EntityId, Vec<Session>>,
    plans: HashMap<EntityId, GraphPlan>,
    committed_keys: HashMap<String, (CommitResult, EntityId)>,
    next_plan: u64,
}

impl GraphStore {
    pub fn remove_session(&mut self, id: &EntityId) -> Result<Session, StoreError> {
        let session = self
            .sessions
            .remove(id)
            .ok_or(StoreError::SessionNotFound)?;
        self.history.remove(id);
        self.plans.retain(|_, plan| plan.session_id != *id);
        self.committed_keys
            .retain(|_, (result, _)| result.session_id != *id);
        Ok(session)
    }

    pub fn insert_session(&mut self, session: Session) -> Result<(), StoreError> {
        validate_session(&session).map_err(StoreError::InvalidGraph)?;
        let (nodes, edges) = self
            .sessions
            .values()
            .filter(|existing| existing.id != session.id)
            .fold((0usize, 0usize), |(nodes, edges), existing| {
                (nodes + existing.nodes.len(), edges + existing.edges.len())
            });
        if nodes + session.nodes.len() > MAX_NODES_GLOBAL {
            return Err(StoreError::InvalidGraph(vec![
                ValidationError::LimitExceeded {
                    path: "global.nodes".into(),
                    requested: nodes + session.nodes.len(),
                    maximum: MAX_NODES_GLOBAL,
                },
            ]));
        }
        if edges + session.edges.len() > MAX_EDGES_GLOBAL {
            return Err(StoreError::InvalidGraph(vec![
                ValidationError::LimitExceeded {
                    path: "global.edges".into(),
                    requested: edges + session.edges.len(),
                    maximum: MAX_EDGES_GLOBAL,
                },
            ]));
        }
        self.history
            .entry(session.id.clone())
            .or_default()
            .push(session.clone());
        if let Some(entries) = self.history.get_mut(&session.id) {
            if entries.len() > 100 {
                entries.remove(0);
            }
        }
        self.sessions.insert(session.id.clone(), session);
        Ok(())
    }

    pub fn session(&self, id: &EntityId) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn sessions(&self, limit: usize) -> Vec<Session> {
        self.sessions_after(None, limit)
    }

    pub fn sessions_after(&self, cursor: Option<&str>, limit: usize) -> Vec<Session> {
        let mut sessions = self.sessions.values().cloned().collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
        if let Some(cursor) = cursor {
            sessions.retain(|session| session.id.as_str() > cursor);
        }
        sessions.truncate(limit.min(500));
        sessions
    }

    pub fn history(&self, id: &EntityId, limit: usize) -> Vec<Session> {
        self.history_before(id, None, limit)
    }

    pub fn history_before(
        &self,
        id: &EntityId,
        before_revision: Option<u64>,
        limit: usize,
    ) -> Vec<Session> {
        self.history
            .get(id)
            .into_iter()
            .flatten()
            .rev()
            .filter(|session| {
                before_revision
                    .map(|revision| session.revision < revision)
                    .unwrap_or(true)
            })
            .take(limit.min(100))
            .cloned()
            .collect()
    }

    pub fn restore_history(&mut self, entries: Vec<Session>) -> Result<(), StoreError> {
        let checkpoint = self.clone();
        for session in entries.into_iter().rev() {
            validate_session(&session).map_err(StoreError::InvalidGraph)?;
            let history = self.history.entry(session.id.clone()).or_default();
            if history
                .iter()
                .all(|existing| existing.revision != session.revision)
            {
                history.push(session.clone());
                if history.len() > 100 {
                    history.remove(0);
                }
            }
            let replace = self
                .sessions
                .get(&session.id)
                .map(|current| current.revision <= session.revision)
                .unwrap_or(true);
            if replace {
                self.sessions.insert(session.id.clone(), session);
            }
        }
        let (nodes, edges) = self
            .sessions
            .values()
            .fold((0usize, 0usize), |(nodes, edges), session| {
                (nodes + session.nodes.len(), edges + session.edges.len())
            });
        if nodes > MAX_NODES_GLOBAL {
            *self = checkpoint;
            return Err(StoreError::InvalidGraph(vec![
                ValidationError::LimitExceeded {
                    path: "global.nodes".into(),
                    requested: nodes,
                    maximum: MAX_NODES_GLOBAL,
                },
            ]));
        }
        if edges > MAX_EDGES_GLOBAL {
            *self = checkpoint;
            return Err(StoreError::InvalidGraph(vec![
                ValidationError::LimitExceeded {
                    path: "global.edges".into(),
                    requested: edges,
                    maximum: MAX_EDGES_GLOBAL,
                },
            ]));
        }
        Ok(())
    }

    pub fn undo_plan(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
    ) -> Result<EntityId, StoreError> {
        let candidate = self
            .history
            .get(session_id)
            .and_then(|entries| entries.iter().rev().nth(1))
            .cloned()
            .ok_or(StoreError::NoUndoAvailable)?;
        self.plan_graph(session_id, base_revision, candidate)
    }

    pub fn plan_graph(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
        candidate: Session,
    ) -> Result<EntityId, StoreError> {
        self.plan_graph_with_ttl(
            session_id,
            base_revision,
            candidate,
            std::time::Duration::from_secs(30),
        )
    }

    pub fn plan_graph_with_ttl(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
        candidate: Session,
        ttl: std::time::Duration,
    ) -> Result<EntityId, StoreError> {
        let current = self
            .sessions
            .get(session_id)
            .ok_or(StoreError::SessionNotFound)?;
        if current.revision != base_revision {
            return Err(StoreError::RevisionConflict {
                expected: base_revision,
                actual: current.revision,
            });
        }
        if candidate.id != *session_id {
            return Err(StoreError::SessionNotFound);
        }
        validate_session(&candidate).map_err(StoreError::InvalidGraph)?;
        self.next_plan += 1;
        let plan_id = EntityId::new(format!("plan-{}", self.next_plan));
        self.plans.insert(
            plan_id.clone(),
            GraphPlan {
                session_id: session_id.clone(),
                base_revision,
                candidate,
                expires_at: std::time::Instant::now() + ttl,
            },
        );
        Ok(plan_id)
    }

    pub fn commit_graph(
        &mut self,
        plan_id: &EntityId,
        base_revision: u64,
        idempotency_key: &str,
    ) -> Result<CommitResult, StoreError> {
        if idempotency_key.is_empty() {
            return Err(StoreError::EmptyIdempotencyKey);
        }
        if let Some((result, committed_plan_id)) = self.committed_keys.get(idempotency_key) {
            if committed_plan_id.as_str() != plan_id.as_str() {
                return Err(StoreError::IdempotencyConflict);
            }
            let mut replay = result.clone();
            replay.idempotent_replay = true;
            return Ok(replay);
        }
        let expires_at = self
            .plans
            .get(plan_id)
            .ok_or(StoreError::PlanNotFound)?
            .expires_at;
        if expires_at <= std::time::Instant::now() {
            return Err(StoreError::PlanExpired);
        }
        let plan = self.plans.remove(plan_id).ok_or(StoreError::PlanNotFound)?;
        let current = self
            .sessions
            .get(&plan.session_id)
            .ok_or(StoreError::SessionNotFound)?;
        if plan.base_revision != base_revision || current.revision != base_revision {
            return Err(StoreError::RevisionConflict {
                expected: base_revision,
                actual: current.revision,
            });
        }
        let mut committed = plan.candidate;
        committed.revision = base_revision + 1;
        let result = CommitResult {
            session_id: plan.session_id,
            revision: committed.revision,
            idempotent_replay: false,
        };
        self.history
            .entry(committed.id.clone())
            .or_default()
            .push(committed.clone());
        if let Some(entries) = self.history.get_mut(&committed.id) {
            if entries.len() > 100 {
                entries.remove(0);
            }
        }
        self.sessions.insert(committed.id.clone(), committed);
        self.committed_keys
            .insert(idempotency_key.into(), (result.clone(), plan_id.clone()));
        Ok(result)
    }
}

impl FakeRuntime {
    pub fn state(&self) -> RuntimeState {
        self.state
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn prepare(&mut self, session: &Session) -> Result<u64, RuntimeError> {
        if let Err(errors) = validate_session(session) {
            self.state = RuntimeState::Failed;
            self.prepared_revision = None;
            return Err(RuntimeError::InvalidGraph(errors));
        }
        self.state = RuntimeState::Preparing;
        self.prepared_revision = Some(session.revision);
        Ok(session.revision)
    }

    pub fn start(&mut self) -> Result<u64, RuntimeError> {
        if self.state == RuntimeState::Running {
            return Ok(self.generation);
        }
        if self.prepared_revision.is_none() {
            return Err(RuntimeError::NotPrepared);
        }
        self.generation += 1;
        self.state = RuntimeState::Running;
        Ok(self.generation)
    }

    pub fn stop(&mut self) {
        self.state = RuntimeState::Stopped;
        self.prepared_revision = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: &str, kind: NodeKind, direction: PortDirection) -> Node {
        Node {
            id: EntityId::new(id),
            kind,
            name: id.into(),
            enabled: true,
            bypass: false,
            ports: vec![Port {
                name: "main".into(),
                direction,
                channels: 1,
            }],
        }
    }
    fn edge(id: &str, source: &str, destination: &str) -> Edge {
        Edge {
            id: EntityId::new(id),
            source_node: EntityId::new(source),
            source_port: "main".into(),
            destination_node: EntityId::new(destination),
            destination_port: "main".into(),
            matrix: vec![1.0],
            enabled: true,
        }
    }
    fn session(nodes: Vec<Node>, edges: Vec<Edge>) -> Session {
        Session {
            id: EntityId::new("session"),
            name: "test".into(),
            schema_version: 1,
            revision: 0,
            nodes,
            edges,
        }
    }

    #[test]
    fn accepts_valid_directed_graph() {
        assert!(validate_session(&session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input)
            ],
            vec![edge("e", "in", "out")]
        ))
        .is_ok());
    }

    #[test]
    fn removing_session_clears_current_history_and_plans() {
        let mut store = GraphStore::default();
        let graph = session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input),
            ],
            vec![edge("e", "in", "out")],
        );
        store.insert_session(graph.clone()).unwrap();
        assert_eq!(store.remove_session(&graph.id).unwrap(), graph);
        assert!(store.session(&EntityId::new("session")).is_none());
        assert_eq!(store.sessions(10), Vec::new());
        assert_eq!(store.history(&EntityId::new("session"), 10), Vec::new());
        assert_eq!(
            store.remove_session(&EntityId::new("session")),
            Err(StoreError::SessionNotFound)
        );
    }

    #[test]
    fn route_inspection_reports_enabled_upstream_provenance() {
        let graph = session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input),
            ],
            vec![edge("in-out", "in", "out")],
        );
        let inspection = inspect_routes(&graph, &EntityId::new("out")).unwrap();
        assert!(inspection.reachable);
        assert_eq!(inspection.paths.len(), 1);
        assert_eq!(
            inspection.paths[0].nodes,
            vec![EntityId::new("in"), EntityId::new("out")]
        );
        assert_eq!(inspection.paths[0].edges, vec![EntityId::new("in-out")]);
        assert_eq!(inspection.paths[0].channel_maps, vec![vec![1.0]]);
    }

    #[test]
    fn route_inspection_excludes_disabled_edges_and_rejects_unknown_destinations() {
        let mut disabled = edge("e", "in", "out");
        disabled.enabled = false;
        let graph = session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input),
            ],
            vec![disabled],
        );
        let inspection = inspect_routes(&graph, &EntityId::new("out")).unwrap();
        assert!(!inspection.reachable);
        assert_eq!(inspection.paths[0].nodes, vec![EntityId::new("out")]);
        assert!(matches!(
            inspect_routes(&graph, &EntityId::new("missing")),
            Err(errors) if matches!(errors.as_slice(), [ValidationError::MissingNode { .. }])
        ));
    }

    #[test]
    fn event_log_replays_ordered_state_and_reports_retention_loss() {
        let mut log = EventLog::new(42);
        assert_eq!(
            log.append(3, Some("op-1".into()), "graph.committed", None),
            1
        );
        assert_eq!(log.append(3, None, "runtime.activated", None), 2);
        let events = log.since(0, 10).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].backend_epoch, 42);
        assert_eq!(events[0].operation_id.as_deref(), Some("op-1"));
        assert_eq!(events[1].sequence, 2);

        for _ in 0..MAX_RETAINED_EVENTS {
            log.append(4, None, "state.changed", None);
        }
        assert_eq!(log.len(), MAX_RETAINED_EVENTS);
        assert!(matches!(
            log.since(1, 10),
            Err(EventReplayError::ResyncRequired)
        ));
        assert_eq!(log.since(log.latest_sequence(), 10).unwrap(), Vec::new());
    }

    #[test]
    fn event_log_expires_entries_after_fifteen_minutes() {
        let mut log = EventLog::new(1);
        let now = Instant::now();
        log.append_at(
            1,
            None,
            "old",
            None,
            now - EVENT_RETENTION - Duration::from_secs(1),
        );
        log.append_at(2, None, "current", None, now);
        assert_eq!(log.len(), 1);
        assert_eq!(log.since(0, 10), Err(EventReplayError::ResyncRequired));
        assert_eq!(log.since(1, 10).unwrap()[0].category, "current");
    }

    #[test]
    fn event_log_rejects_unbounded_replay_requests() {
        let mut log = EventLog::new(1);
        assert_eq!(log.since(0, 0), Err(EventReplayError::InvalidLimit));
        assert_eq!(log.since(0, 501), Err(EventReplayError::InvalidLimit));
    }

    #[test]
    fn rejects_dangling_edge_and_cycle() {
        let result = validate_session(&session(
            vec![
                node("a", NodeKind::Gain, PortDirection::Output),
                node("b", NodeKind::Gain, PortDirection::Input),
            ],
            vec![
                edge("e1", "a", "b"),
                edge("e2", "b", "a"),
                edge("e3", "missing", "b"),
            ],
        ));
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|error| matches!(error, ValidationError::MissingNode { .. })));
        assert!(errors
            .iter()
            .any(|error| matches!(error, ValidationError::Cycle { .. })));
    }
    #[test]
    fn rejects_multiple_edges_to_non_mixer() {
        assert!(validate_session(&session(
            vec![
                node("a", NodeKind::Gain, PortDirection::Output),
                node("b", NodeKind::Gain, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input)
            ],
            vec![edge("e1", "a", "out"), edge("e2", "b", "out")]
        ))
        .is_err());
    }
    #[test]
    fn rejects_bad_matrix() {
        let mut e = edge("e", "in", "out");
        e.matrix = vec![f32::NAN];
        assert!(validate_session(&session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input)
            ],
            vec![e]
        ))
        .is_err());
    }

    #[test]
    fn fake_runtime_is_idempotent_and_generation_bound() {
        let graph = session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input),
            ],
            vec![edge("e", "in", "out")],
        );
        let mut runtime = FakeRuntime::default();
        assert_eq!(runtime.start(), Err(RuntimeError::NotPrepared));
        assert_eq!(runtime.prepare(&graph), Ok(0));
        assert_eq!(runtime.start(), Ok(1));
        assert_eq!(runtime.start(), Ok(1));
        assert_eq!(runtime.state(), RuntimeState::Running);
        runtime.stop();
        assert_eq!(runtime.state(), RuntimeState::Stopped);
        assert_eq!(runtime.start(), Err(RuntimeError::NotPrepared));
    }

    #[test]
    fn failed_prepare_does_not_leave_a_running_generation() {
        let mut graph = session(
            vec![node("in", NodeKind::PhysicalInput, PortDirection::Output)],
            vec![],
        );
        let mut runtime = FakeRuntime::default();
        assert!(matches!(runtime.prepare(&graph), Ok(0)));
        assert_eq!(runtime.start(), Ok(1));
        graph.edges.push(edge("bad", "in", "missing"));
        assert!(matches!(
            runtime.prepare(&graph),
            Err(RuntimeError::InvalidGraph(_))
        ));
        assert_eq!(runtime.state(), RuntimeState::Failed);
        assert_eq!(runtime.generation(), 1);
    }

    #[test]
    fn registry_reports_unimplemented_audio_capabilities_explicitly() {
        let registry = node_registry();
        assert_eq!(registry.len(), 8);
        let physical = registry
            .iter()
            .find(|spec| spec.kind == NodeKind::PhysicalInput)
            .unwrap();
        assert_eq!(physical.kind.type_name(), "physical-input");
        assert_eq!(physical.version, 1);
        assert_eq!(
            physical.availability,
            CapabilityAvailability::Unavailable("requires M02 Windows audio adapters")
        );
        let gain = registry
            .iter()
            .find(|spec| spec.kind == NodeKind::Gain)
            .unwrap();
        assert_eq!(gain.availability, CapabilityAvailability::Available);
    }

    #[test]
    fn graph_store_rejects_stale_commit_and_replays_idempotently() {
        let original = session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input),
            ],
            vec![edge("e", "in", "out")],
        );
        let mut store = GraphStore::default();
        store.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "updated".into();
        let plan = store.plan_graph(&original.id, 0, candidate).unwrap();
        let committed = store.commit_graph(&plan, 0, "operation-1").unwrap();
        assert_eq!(committed.revision, 1);
        assert!(!committed.idempotent_replay);
        assert_eq!(
            store.commit_graph(&plan, 0, "operation-1"),
            Ok(CommitResult {
                session_id: original.id.clone(),
                revision: 1,
                idempotent_replay: true
            })
        );
        let mut second = store.session(&original.id).unwrap().clone();
        second.name = "second".into();
        let second_plan = store.plan_graph(&original.id, 1, second).unwrap();
        assert_eq!(
            store.commit_graph(&second_plan, 0, "operation-2"),
            Err(StoreError::RevisionConflict {
                expected: 0,
                actual: 1
            })
        );
    }

    #[test]
    fn graph_store_enforces_global_node_budget_with_replacement_accounting() {
        let make_session = |id: &str, count: usize| {
            let mut value = session(
                (0..count)
                    .map(|index| {
                        node(
                            &format!("{id}-{index}"),
                            NodeKind::Gain,
                            PortDirection::Output,
                        )
                    })
                    .collect(),
                vec![],
            );
            value.id = EntityId::new(id);
            value
        };
        let mut store = GraphStore::default();
        store.insert_session(make_session("one", 64)).unwrap();
        store.insert_session(make_session("two", 64)).unwrap();
        let error = store.insert_session(make_session("three", 1)).unwrap_err();
        assert!(matches!(
            error,
            StoreError::InvalidGraph(errors)
                if matches!(errors.as_slice(), [ValidationError::LimitExceeded { path, maximum: MAX_NODES_GLOBAL, .. }] if path == "global.nodes")
        ));
        store.insert_session(make_session("one", 1)).unwrap();
        assert_eq!(store.sessions(10).len(), 2);
    }

    #[test]
    fn restoring_history_applies_global_budget_transactionally() {
        let make_session = |id: &str, count: usize| {
            let mut value = session(
                (0..count)
                    .map(|index| {
                        node(
                            &format!("{id}-{index}"),
                            NodeKind::Gain,
                            PortDirection::Output,
                        )
                    })
                    .collect(),
                vec![],
            );
            value.id = EntityId::new(id);
            value
        };
        let mut store = GraphStore::default();
        store.insert_session(make_session("existing", 64)).unwrap();
        store.insert_session(make_session("full", 64)).unwrap();
        let error = store
            .restore_history(vec![make_session("restored", 1)])
            .unwrap_err();
        assert!(matches!(
            error,
            StoreError::InvalidGraph(errors)
                if matches!(errors.as_slice(), [ValidationError::LimitExceeded { path, maximum: MAX_NODES_GLOBAL, .. }] if path == "global.nodes")
        ));
        assert!(store.session(&EntityId::new("restored")).is_none());
        assert_eq!(
            store
                .session(&EntityId::new("existing"))
                .unwrap()
                .nodes
                .len(),
            64
        );
    }

    #[test]
    fn graph_store_rejects_idempotency_key_reuse_for_another_plan() {
        let original = session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input),
            ],
            vec![edge("e", "in", "out")],
        );
        let mut store = GraphStore::default();
        store.insert_session(original.clone()).unwrap();
        let mut first_candidate = original.clone();
        first_candidate.name = "first".into();
        let first_plan = store.plan_graph(&original.id, 0, first_candidate).unwrap();
        store.commit_graph(&first_plan, 0, "same-key").unwrap();
        let mut second_candidate = store.session(&original.id).unwrap().clone();
        second_candidate.name = "second".into();
        let second_plan = store.plan_graph(&original.id, 1, second_candidate).unwrap();
        assert_eq!(
            store.commit_graph(&second_plan, 1, "same-key"),
            Err(StoreError::IdempotencyConflict)
        );
    }

    #[test]
    fn graph_store_returns_newest_bounded_revision_history() {
        let original = session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input),
            ],
            vec![edge("e", "in", "out")],
        );
        let mut store = GraphStore::default();
        store.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "updated".into();
        let plan = store.plan_graph(&original.id, 0, candidate).unwrap();
        store.commit_graph(&plan, 0, "history-1").unwrap();
        let history = store.history(&original.id, 1);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].revision, 1);
        assert_eq!(history[0].name, "updated");
    }

    #[test]
    fn graph_store_drops_oldest_snapshots_past_retention_bound() {
        let mut store = GraphStore::default();
        for revision in 0..=100 {
            let mut snapshot = session(
                vec![
                    node("in", NodeKind::PhysicalInput, PortDirection::Output),
                    node("out", NodeKind::PhysicalOutput, PortDirection::Input),
                ],
                vec![edge("e", "in", "out")],
            );
            snapshot.revision = revision;
            snapshot.name = format!("revision-{revision}");
            store.insert_session(snapshot).unwrap();
        }
        let history = store.history(&EntityId::new("session"), 500);
        assert_eq!(history.len(), 100);
        assert_eq!(history.first().unwrap().revision, 100);
        assert_eq!(history.last().unwrap().revision, 1);
    }

    #[test]
    fn expired_plan_cannot_mutate_session() {
        let original = session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input),
            ],
            vec![edge("e", "in", "out")],
        );
        let mut store = GraphStore::default();
        store.insert_session(original.clone()).unwrap();
        let plan = store
            .plan_graph_with_ttl(&original.id, 0, original.clone(), std::time::Duration::ZERO)
            .unwrap();
        assert_eq!(
            store.commit_graph(&plan, 0, "expired"),
            Err(StoreError::PlanExpired)
        );
        assert_eq!(store.session(&original.id).unwrap().revision, 0);
    }

    #[test]
    fn session_json_round_trip_preserves_contract_shape() {
        let original = session(
            vec![
                node("in", NodeKind::PhysicalInput, PortDirection::Output),
                node("out", NodeKind::PhysicalOutput, PortDirection::Input),
            ],
            vec![edge("e", "in", "out")],
        );
        let encoded = serde_json::to_string_pretty(&original).unwrap();
        assert!(encoded.contains("physicalInput"));
        assert!(encoded.contains("sourceNode"));
        let decoded: Session = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn checked_fixture_deserializes_and_validates() {
        let fixture: Session =
            serde_json::from_str(include_str!("../../../tests/fixtures/valid-session.json"))
                .unwrap();
        assert_eq!(fixture.id.as_str(), "session-fixture");
        assert!(validate_session(&fixture).is_ok());
    }

    #[test]
    fn method_discovery_classifies_permissions_and_side_effects() {
        let plan = API_METHODS
            .iter()
            .find(|method| method.name == "graph.plan")
            .unwrap();
        assert_eq!(plan.permission, PermissionScope::GraphWrite);
        assert_eq!(plan.side_effect, SideEffectClass::PlanOnly);
        let describe = API_METHODS
            .iter()
            .find(|method| method.name == "system.describe")
            .unwrap();
        assert_eq!(describe.side_effect, SideEffectClass::ReadOnly);
        let start = API_METHODS
            .iter()
            .find(|method| method.name == "session.start")
            .unwrap();
        assert_eq!(start.side_effect, SideEffectClass::ExternalOperation);
    }
}
