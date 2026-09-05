//! Portable AudioRouter domain contracts and graph validation.
//!
//! This crate deliberately contains no Windows, audio, IPC, filesystem, or
//! realtime code. It is the authority for the in-memory graph shape used by
//! later control-plane adapters.

use std::collections::{HashMap, HashSet};

pub const MAX_NODES_PER_SESSION: usize = 64;
pub const MAX_EDGES_PER_SESSION: usize = 128;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EntityId(String);

impl EntityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Port {
    pub name: String,
    pub direction: PortDirection,
    pub channels: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: EntityId,
    pub kind: NodeKind,
    pub name: String,
    pub enabled: bool,
    pub bypass: bool,
    pub ports: Vec<Port>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: EntityId,
    pub source_node: EntityId,
    pub source_port: String,
    pub destination_node: EntityId,
    pub destination_port: String,
    pub matrix: Vec<f32>,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeState {
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
    InvalidGraph(Vec<ValidationError>),
    RevisionConflict { expected: u64, actual: u64 },
    EmptyIdempotencyKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
}

/// In-memory revision store used to prove M01 transaction semantics before
/// SQLite and the named-pipe control process are introduced.
#[derive(Debug, Default)]
pub struct GraphStore {
    sessions: HashMap<EntityId, Session>,
    plans: HashMap<EntityId, GraphPlan>,
    committed_keys: HashMap<String, CommitResult>,
    next_plan: u64,
}

impl GraphStore {
    pub fn insert_session(&mut self, session: Session) -> Result<(), StoreError> {
        validate_session(&session).map_err(StoreError::InvalidGraph)?;
        self.sessions.insert(session.id.clone(), session);
        Ok(())
    }

    pub fn session(&self, id: &EntityId) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn plan_graph(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
        candidate: Session,
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
        if let Some(result) = self.committed_keys.get(idempotency_key) {
            let mut replay = result.clone();
            replay.idempotent_replay = true;
            return Ok(replay);
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
        self.sessions.insert(committed.id.clone(), committed);
        self.committed_keys
            .insert(idempotency_key.into(), result.clone());
        Ok(result)
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::Stopped
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
}
