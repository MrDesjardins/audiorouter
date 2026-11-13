//! Task-shaped control methods for people and tools that are not audio
//! engineers: StreamDeck keys, scripts over the local HTTP API, and LLM
//! assistants over MCP. Each method maps to one intention ("play", "turn the
//! game down", "add a compressor after my mic") and goes through the same
//! validation, plan/commit, persistence and live activation as the UI.
//!
//! Common rules: `sessionId` is optional (default: the active session);
//! nodes are addressed by ID or by exact name (case-insensitive); a name that
//! matches several nodes is refused with the candidates; mutating calls need
//! an `idempotencyKey`.

use super::*;
use audiorouter_domain::{Edge, Node, Port};

/// Methods served by this module, with whether they mutate.
pub(crate) const SIMPLE_METHODS: &[(&str, bool)] = &[
    ("sessions.play", true),
    ("sessions.togglePlay", true),
    ("sessions.summary", false),
    ("safety.togglePrivacyMute", true),
    ("nodes.catalog", false),
    ("nodes.set", true),
    ("nodes.toggle", true),
    ("nodes.add", true),
    ("nodes.remove", true),
    ("connections.add", true),
    ("connections.remove", true),
    ("meters.levels", false),
];

/// Display name, ports and a one-line plain description of a node kind that
/// can be added by kind alone. Kinds that need an identity chosen in the UI
/// (plugins, application capture, endpoint loopback, virtual buses) are absent.
pub(crate) fn node_template(kind: NodeKind) -> Option<(&'static str, Vec<Port>, &'static str)> {
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let input = || port("in", PortDirection::Input);
    let output = || port("out", PortDirection::Output);
    let tool = |name, description| Some((name, vec![input(), output()], description));
    match kind {
        NodeKind::PhysicalInput => Some(("Input device", vec![output()], "A microphone or other input device. Choose the device with nodes.set parameters.endpointId (see devices.list).")),
        NodeKind::PhysicalOutput => Some(("Output device", vec![input()], "Speakers, headphones or a virtual cable such as CABLE Input. Choose the device with parameters.endpointId (see devices.list).")),
        NodeKind::TestSignal => Some(("Test Signal", vec![output()], "A test tone to check a route without a microphone.")),
        NodeKind::Soundboard => Some(("Soundboard", vec![output()], "Up to nine sounds you trigger on demand (from the canvas, the API or a Stream Deck). Choose sounds in the node's parameters (slot1MediaId ... slot9MediaId).")),
        NodeKind::AudioFile => Some(("Audio file", vec![output()], "Plays an uploaded WAV or MP3 file.")),
        NodeKind::NetworkSend => Some(("Network Send", vec![input()], "Streams audio to AudioRouter on another computer (set host and port).")),
        NodeKind::NetworkReceive => Some(("Network Receive", vec![output()], "Plays audio sent by Network Send on another computer (set sender and port).")),
        NodeKind::Mixer => tool("Mixer", "Combines several sources into one; each input has its own volume."),
        NodeKind::InputSwitch => Some(("Input Switch", vec![port("a", PortDirection::Input), port("b", PortDirection::Input), output()], "Passes either input A or input B, with a short crossfade. Toggle selected to switch.")),
        NodeKind::OutputSwitch => Some(("Output Switch", vec![input(), port("a", PortDirection::Output), port("b", PortDirection::Output)], "Sends one input to output A or B. Only the selected output receives audio. Toggle selected to switch.")),
        NodeKind::Duck => tool("Duck", "Turns this audio down while another node (keyNodeId, usually the microphone) is loud, e.g. game sound while you talk."),
        NodeKind::Volume => tool("Volume", "Sets a level in percent (100 = unchanged)."),
        NodeKind::Gain => tool("Gain", "Raises or lowers the level in dB (0 = unchanged)."),
        NodeKind::Mute => tool("Mute", "Silences the audio while muted is true."),
        NodeKind::Meter => tool("Meter", "Shows levels and clipping; audio passes unchanged."),
        NodeKind::Recorder => tool("Recorder", "Records what reaches it to a file (recorders.startRecording / stopRecording)."),
        NodeKind::ParametricEq => tool("Advanced EQ", "Shapes the tone with up to 16 EQ points."),
        NodeKind::GraphicEq => tool("Graphic EQ", "Ten-band equalizer from 31.5 Hz to 16 kHz."),
        NodeKind::BassTreble => tool("Bass & Treble", "Simple warmth (bass) and brightness (treble) controls."),
        NodeKind::Compressor => tool("Compressor", "Evens out loud and quiet speech; lower the threshold for more control."),
        NodeKind::Gate => tool("Gate", "Silences background noise between words; set the threshold between room noise and voice."),
        NodeKind::Limiter => tool("Limiter", "Stops peaks from going over the ceiling, preventing clipping."),
        NodeKind::Delay => tool("Sync", "Delays audio (0-1000 ms) to line it up with video."),
        NodeKind::Pitch => tool("Pitch shift", "Moves the voice up or down in semitones."),
        NodeKind::Dehum => tool("Dehum", "Removes 50/60 Hz electrical hum."),
        NodeKind::Declick => tool("Declick", "Repairs clicks and pops."),
        NodeKind::Denoise => tool("Denoise", "Learns a steady noise (fan, hiss) and removes it."),
        NodeKind::SpeechDenoise => tool("Speech Denoise", "Automatically reduces background noise around speech."),
        NodeKind::SpectralGate => tool("Spectral Gate", "Learns noise per frequency and blocks what stays below it."),
        NodeKind::FirFilter => tool("FIR Filter", "Applies a room or speaker impulse response."),
        NodeKind::TimeShift => tool("Time Shift", "A live-audio DVR: pause, jump back and return to live."),
        NodeKind::FunVoiceChanger => tool("Fun Voice Changer", "Playful voice presets: pitch correct, deep, robot, radio and more; tweak any of them."),
        NodeKind::Plugin
        | NodeKind::ApplicationCapture
        | NodeKind::EndpointLoopback
        | NodeKind::VirtualRenderSource
        | NodeKind::VirtualCaptureSink => None,
    }
}

/// The camelCase wire name of a node kind ("parametricEq").
fn kind_name(kind: NodeKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Default parameter values of a kind, from the node catalog. A parameter
/// family (`namePattern`, such as `slot<N>Mode`) is never a literal key.
pub(crate) fn default_parameters(kind: NodeKind) -> serde_json::Map<String, Value> {
    ControlPlane::node_parameter_schema(kind)
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|parameter| {
            let name = parameter["name"].as_str()?;
            if name.ends_with(':') || parameter.get("namePattern").is_some() {
                None
            } else {
                Some((name.to_owned(), parameter.get("default")?.clone()))
            }
        })
        .collect()
}

/// Default channel matrix between two channel counts (mirrors the UI).
fn channel_matrix(source: u8, destination: u8) -> Vec<f32> {
    match (source, destination) {
        (1, 2) => vec![1.0, 1.0],
        (2, 1) => vec![0.5, 0.5],
        (from, to) => (0..usize::from(to) * usize::from(from))
            .map(|index| {
                if index / usize::from(from) == index % usize::from(from) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect(),
    }
}

fn text_param<'v>(params: &'v Value, name: &str) -> Result<&'v str, ControlError> {
    params
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ControlError::InvalidRequest(format!("{name} is required")))
}

fn optional_text_param<'v>(params: &'v Value, name: &str) -> Result<Option<&'v str>, ControlError> {
    params
        .get(name)
        .map(|_| text_param(params, name))
        .transpose()
}

/// Find a node by ID, then by exact name ignoring case.
pub(crate) fn resolve_node<'s>(
    session: &'s Session,
    reference: &str,
) -> Result<&'s Node, ControlError> {
    if let Some(node) = session
        .nodes
        .iter()
        .find(|node| node.id.as_str() == reference)
    {
        return Ok(node);
    }
    let matches = session
        .nodes
        .iter()
        .filter(|node| node.name.eq_ignore_ascii_case(reference))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [node] => Ok(node),
        [] => Err(ControlError::InvalidRequest(format!(
            "no node named \"{reference}\" in this session; nodes: {}",
            session
                .nodes
                .iter()
                .map(|node| format!("\"{}\"", node.name))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
        many => Err(ControlError::InvalidRequest(format!(
            "several nodes are named \"{reference}\"; use one of these IDs: {}",
            many.iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Connect `from` to the first free input of `to`, refusing an occupied
/// single input (a Mixer input accepts several).
pub(crate) fn connect(
    session: &mut Session,
    from: &EntityId,
    to: &EntityId,
) -> Result<EntityId, ControlError> {
    connect_ports(session, from, to, None, None)
}

fn named_port<'a>(
    node: &'a Node,
    direction: PortDirection,
    name: &str,
) -> Result<&'a Port, ControlError> {
    node.ports
        .iter()
        .find(|port| port.direction == direction && port.name == name)
        .ok_or_else(|| {
            ControlError::InvalidRequest(format!(
                "\"{}\" has no {:?} port \"{name}\"; ports: {}",
                node.name,
                direction,
                node.ports
                    .iter()
                    .filter(|port| port.direction == direction)
                    .map(|port| port.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
}

fn connect_ports(
    session: &mut Session,
    from: &EntityId,
    to: &EntityId,
    from_output: Option<&str>,
    to_input: Option<&str>,
) -> Result<EntityId, ControlError> {
    let source = session
        .nodes
        .iter()
        .find(|node| node.id == *from)
        .cloned()
        .ok_or_else(|| ControlError::InvalidRequest("source node is missing".into()))?;
    let target = session
        .nodes
        .iter()
        .find(|node| node.id == *to)
        .cloned()
        .ok_or_else(|| ControlError::InvalidRequest("target node is missing".into()))?;
    let output = if let Some(name) = from_output {
        named_port(&source, PortDirection::Output, name)?
    } else {
        source
            .ports
            .iter()
            .find(|port| port.direction == PortDirection::Output)
            .ok_or_else(|| {
                ControlError::InvalidRequest(format!(
                    "\"{}\" has no output to connect from",
                    source.name
                ))
            })?
    };
    if let Some(name) = to_input {
        named_port(&target, PortDirection::Input, name)?;
    }
    if session.edges.iter().any(|edge| {
        edge.source_node == *from
            && edge.source_port == output.name
            && edge.destination_node == *to
            && to_input.is_none_or(|name| edge.destination_port == name)
    }) {
        return Err(ControlError::InvalidRequest(format!(
            "\"{}\" is already connected to \"{}\"",
            source.name, target.name
        )));
    }
    let input = target
        .ports
        .iter()
        .filter(|port| port.direction == PortDirection::Input)
        .filter(|port| to_input.is_none_or(|name| port.name == name))
        .find(|port| target.kind == NodeKind::Mixer || !session.edges.iter().any(|edge| edge.destination_node == *to && edge.destination_port == port.name))
        .ok_or_else(|| {
            ControlError::InvalidRequest(format!(
                "\"{}\" already has an input connected; remove that connection first or add a Mixer",
                target.name
            ))
        })?;
    let mut id = format!("{}-{}", from.as_str(), to.as_str());
    let mut suffix = 2;
    while session.edges.iter().any(|edge| edge.id.as_str() == id) {
        id = format!("{}-{}-{suffix}", from.as_str(), to.as_str());
        suffix += 1;
    }
    let id = EntityId::new(id);
    session.edges.push(Edge {
        id: id.clone(),
        source_node: from.clone(),
        source_port: output.name.clone(),
        destination_node: to.clone(),
        destination_port: input.name.clone(),
        matrix: channel_matrix(output.channels, input.channels),
        enabled: true,
    });
    Ok(id)
}

fn move_mixer_volume(session: &mut Session, target: &EntityId, old: &EntityId, new: &EntityId) {
    if let Some(mixer) = session
        .nodes
        .iter_mut()
        .find(|node| node.id == *target && node.kind == NodeKind::Mixer)
    {
        let prefix = audiorouter_domain::MIXER_INPUT_VOLUME_PREFIX;
        let volume = mixer
            .parameters
            .remove(&format!("{prefix}{}", old.as_str()));
        // An absent old key means 100%, not a stale value under the new ID.
        mixer
            .parameters
            .remove(&format!("{prefix}{}", new.as_str()));
        if let Some(volume) = volume {
            mixer
                .parameters
                .insert(format!("{prefix}{}", new.as_str()), volume);
        }
    }
}

fn insert_on_edge(
    session: &mut Session,
    original: &Edge,
    inserted: &EntityId,
) -> Result<(), ControlError> {
    session.edges.retain(|edge| edge.id != original.id);
    let before = connect_ports(
        session,
        &original.source_node,
        inserted,
        Some(&original.source_port),
        None,
    )?;
    let after = connect_ports(
        session,
        inserted,
        &original.destination_node,
        None,
        Some(&original.destination_port),
    )?;
    let downstream = session
        .edges
        .iter_mut()
        .find(|edge| edge.id == after)
        .unwrap();
    if downstream.matrix.len() == original.matrix.len() {
        downstream.matrix.clone_from(&original.matrix);
    } else {
        // If channel conversion moved to the downstream edge, the old matrix
        // still fits the upstream edge (for example stereo -> mono tool).
        let upstream = session
            .edges
            .iter_mut()
            .find(|edge| edge.id == before)
            .unwrap();
        if upstream.matrix.len() == original.matrix.len() {
            upstream.matrix.clone_from(&original.matrix);
        } else {
            // Neither side has the old shape (mono -> stereo tool -> mono).
            // Apply the original mapping after the tool's dry conversion back
            // to the source shape, rather than discarding its coefficients.
            let source = resolve_node(session, original.source_node.as_str())?;
            let source_channels =
                named_port(source, PortDirection::Output, &original.source_port)?.channels;
            let tool = resolve_node(session, inserted.as_str())?;
            let tool_channels = tool
                .ports
                .iter()
                .find(|port| port.direction == PortDirection::Output)
                .unwrap()
                .channels;
            let conversion = channel_matrix(tool_channels, source_channels);
            let rows = original.matrix.len() / usize::from(source_channels);
            let columns = usize::from(tool_channels);
            let middle = usize::from(source_channels);
            let mut matrix = vec![0.0; rows * columns];
            for row in 0..rows {
                for column in 0..columns {
                    for inner in 0..middle {
                        matrix[row * columns + column] += original.matrix[row * middle + inner]
                            * conversion[inner * columns + column];
                    }
                }
            }
            session
                .edges
                .iter_mut()
                .find(|edge| edge.id == after)
                .unwrap()
                .matrix = matrix;
        }
    }
    for edge in session
        .edges
        .iter_mut()
        .filter(|edge| edge.id == before || edge.id == after)
    {
        edge.enabled = original.enabled;
    }
    move_mixer_volume(
        session,
        &original.destination_node,
        &original.source_node,
        inserted,
    );
    Ok(())
}

fn bridge_edges(
    session: &mut Session,
    incoming: &Edge,
    outgoing: &Edge,
    removed: &Node,
) -> Result<(), ControlError> {
    let input_channels = usize::from(
        named_port(removed, PortDirection::Input, &incoming.destination_port)?.channels,
    );
    let output_channels =
        usize::from(named_port(removed, PortDirection::Output, &outgoing.source_port)?.channels);
    let source_channels = incoming.matrix.len() / input_channels;
    let destination_channels = outgoing.matrix.len() / output_channels;
    let dry = channel_matrix(input_channels as u8, output_channels as u8);
    let mut matrix = vec![0.0; source_channels * destination_channels];
    for destination in 0..destination_channels {
        for source in 0..source_channels {
            for output in 0..output_channels {
                for input in 0..input_channels {
                    matrix[destination * source_channels + source] += outgoing.matrix
                        [destination * output_channels + output]
                        * dry[output * input_channels + input]
                        * incoming.matrix[input * source_channels + source];
                }
            }
        }
    }
    let id = connect_ports(
        session,
        &incoming.source_node,
        &outgoing.destination_node,
        Some(&incoming.source_port),
        Some(&outgoing.destination_port),
    )?;
    let bridge = session.edges.iter_mut().find(|edge| edge.id == id).unwrap();
    bridge.matrix = matrix;
    bridge.enabled = incoming.enabled && outgoing.enabled;
    move_mixer_volume(
        session,
        &outgoing.destination_node,
        &removed.id,
        &incoming.source_node,
    );
    Ok(())
}

fn display_value(value: &Value, unit: Option<&str>) -> String {
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Bool(flag) => {
            if *flag {
                "on".into()
            } else {
                "off".into()
            }
        }
        Value::Number(number) => number.as_f64().map_or_else(
            || number.to_string(),
            |number| {
                if number.fract() == 0.0 {
                    format!("{number:.0}")
                } else {
                    format!("{number:.2}").trim_end_matches('0').to_owned()
                }
            },
        ),
        other => other.to_string(),
    };
    match unit {
        Some(unit) if !unit.is_empty() && !matches!(value, Value::String(_) | Value::Bool(_)) => {
            format!("{text} {unit}")
        }
        _ => text,
    }
}

impl ControlPlane {
    pub(crate) fn dispatch_simple(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let mutating = SIMPLE_METHODS
            .iter()
            .find(|(name, _)| *name == method)
            .is_some_and(|(_, mutating)| *mutating);
        let operation = if mutating {
            let key = text_param(&params, "idempotencyKey")?.to_owned();
            let scoped = self.scoped_idempotency_key(method, &key);
            let hash = Self::request_hash(&json!({ "method": method, "params": params }));
            if let Some(previous) = self.lookup_idempotent_result(&scoped, &hash)? {
                return Ok(previous);
            }
            Some((scoped, hash, key))
        } else {
            None
        };
        let result = match method {
            "nodes.catalog" => Ok(self.simple_catalog()),
            "safety.togglePrivacyMute" => {
                let key = &operation.as_ref().expect("mutating").2;
                self.dispatch_privacy_mute(Some(json!({ "muted": !self.privacy_muted, "idempotencyKey": format!("toggle:{key}") })))
            }
            _ => {
                let session_id = self.simple_session_id(&params)?;
                match method {
                    "sessions.play" => self.simple_play(&session_id),
                    "sessions.togglePlay" => {
                        if self.simple_running(&session_id) {
                            self.session_stop(&session_id)
                        } else {
                            self.simple_play(&session_id)
                        }
                    }
                    "sessions.summary" => self.simple_summary(&session_id),
                    "meters.levels" => self.simple_levels(&session_id),
                    _ => self.simple_edit(
                        method,
                        &session_id,
                        &params,
                        &operation.as_ref().expect("mutating").2,
                    ),
                }
            }
        }?;
        if let Some((scoped, hash, _)) = operation {
            self.journal_idempotent_result(&scoped, method, &hash, &result)?;
        }
        Ok(result)
    }

    pub(crate) fn simple_session_id(&self, params: &Value) -> Result<EntityId, ControlError> {
        if let Some(id) = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        {
            let id = EntityId::new(id);
            self.get_session(&id)?;
            return Ok(id);
        }
        self.active_session_id.clone().ok_or_else(|| {
            ControlError::InvalidRequest("sessionId is required: no session is active".into())
        })
    }

    /// Apply the common rules to a runtime call aimed at one node
    /// (`soundboard.*`, `audioSources.transport`, `timeShift.transport`,
    /// `meters.reset`): a missing `sessionId` means the active session, and
    /// `nodeId` may be the node's exact name. IDs are matched first, so a
    /// caller that already sends IDs (Stream Deck, the UI) is unaffected.
    pub(crate) fn resolve_node_target(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, ControlError> {
        let Some(mut params) = params else {
            return Ok(None);
        };
        if !params.is_object() {
            return Ok(Some(params));
        }
        let session_id = self.simple_session_id(&params)?;
        if let Some(reference) = params
            .get("nodeId")
            .and_then(Value::as_str)
            .filter(|reference| !reference.is_empty())
        {
            let session = self.get_session(&session_id)?;
            let node_id = resolve_node(session, reference)?.id.clone();
            params["nodeId"] = json!(node_id);
        }
        params["sessionId"] = json!(session_id);
        Ok(Some(params))
    }

    fn simple_running(&self, session_id: &EntityId) -> bool {
        self.runtimes
            .get(session_id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
    }

    /// Prepare the saved route's devices (every path) and start, as Play does.
    fn simple_play(&mut self, session_id: &EntityId) -> Result<Value, ControlError> {
        if self.simple_running(session_id) {
            return Ok(
                json!({ "sessionId": session_id, "state": "running", "alreadyPlaying": true }),
            );
        }
        self.dispatch_native_paths_prepare(Some(json!({ "sessionId": session_id })))?;
        let mut result = self.session_start(session_id)?;
        self.start_auto_recordings(session_id, &mut result)?;
        Ok(result)
    }

    fn simple_catalog(&self) -> Value {
        json!(NodeKind::ALL
            .iter()
            .filter_map(|kind| {
                let (name, ports, description) = node_template(*kind)?;
                Some(json!({
                    "kind": kind_name(*kind),
                    "name": name,
                    "description": description,
                    "inputs": ports.iter().filter(|port| port.direction == PortDirection::Input).map(|port| port.name.clone()).collect::<Vec<_>>(),
                    "outputs": ports.iter().filter(|port| port.direction == PortDirection::Output).map(|port| port.name.clone()).collect::<Vec<_>>(),
                    "parameters": Self::node_parameter_schema(*kind),
                }))
            })
            .collect::<Vec<_>>())
    }

    fn simple_summary(&self, session_id: &EntityId) -> Result<Value, ControlError> {
        let session = self.get_session(session_id)?;
        let name_of = |id: &EntityId| {
            session
                .nodes
                .iter()
                .find(|node| node.id == *id)
                .map_or_else(|| id.as_str().to_owned(), |node| node.name.clone())
        };
        let nodes = session
            .nodes
            .iter()
            .map(|node| {
                let schema = Self::node_parameter_schema(node.kind);
                let unit = |name: &str| schema.as_array().and_then(|items| items.iter().find(|item| item["name"] == name)).and_then(|item| item["unit"].as_str().map(str::to_owned));
                let mut settings = default_parameters(node.kind);
                for (name, value) in &node.parameters {
                    settings.insert(name.clone(), value.clone());
                }
                let readable = settings
                    .iter()
                    .filter(|(name, _)| !matches!(name.as_str(), "noiseProfile" | "mediaId" | "creationTime100ns"))
                    .map(|(name, value)| {
                        let shown = if name == "keyNodeId" { value.as_str().filter(|id| !id.is_empty()).map_or_else(|| "none".to_owned(), |id| name_of(&EntityId::new(id))) } else { display_value(value, unit(name).as_deref()) };
                        (name.clone(), json!(shown))
                    })
                    .collect::<serde_json::Map<_, _>>();
                json!({ "id": node.id, "name": node.name, "kind": kind_name(node.kind), "enabled": node.enabled, "bypass": node.bypass, "settings": readable })
            })
            .collect::<Vec<_>>();
        let connections = session
            .edges
            .iter()
            .map(|edge| json!({ "id": edge.id, "from": name_of(&edge.source_node), "to": name_of(&edge.destination_node), "fromOutput": edge.source_port, "toInput": edge.destination_port, "enabled": edge.enabled }))
            .collect::<Vec<_>>();
        Ok(json!({
            "sessionId": session.id,
            "name": session.name,
            "revision": session.revision,
            "playing": self.simple_running(session_id),
            "privacyMuted": self.privacy_muted,
            "nodes": nodes,
            "connections": connections,
        }))
    }

    fn simple_levels(&self, session_id: &EntityId) -> Result<Value, ControlError> {
        let session = self.get_session(session_id)?;
        let mut telemetry = self
            .native_node_telemetry()
            .as_array()
            .cloned()
            .unwrap_or_default();
        telemetry.extend(
            self.native_multi_input_node_telemetry()
                .as_array()
                .cloned()
                .unwrap_or_default(),
        );
        let levels = session
            .nodes
            .iter()
            .filter_map(|node| {
                let item = telemetry.iter().find(|item| item["nodeId"] == json!(node.id))?;
                let meter = &item["meter"];
                let processor = &item["processor"];
                Some(json!({
                    "nodeId": node.id,
                    "name": node.name,
                    "peakDb": meter.get("currentPeakDb").or_else(|| meter.get("peakDb")).cloned().unwrap_or(Value::Null),
                    "rmsDb": meter.get("rmsDb").cloned().unwrap_or(Value::Null),
                    "clipped": meter.get("clippedSamples").and_then(Value::as_u64).is_some_and(|count| count > 0),
                    "reductionDb": processor.get("gainReductionDb").and_then(Value::as_array).and_then(|values| values.iter().filter_map(Value::as_f64).reduce(f64::max)).map_or(Value::Null, |value| json!(value)),
                    "active": processor.get("gateOpen").and_then(Value::as_array).map_or(Value::Null, |values| json!(values.iter().any(|open| open.as_bool() == Some(true)))),
                }))
            })
            .collect::<Vec<_>>();
        Ok(
            json!({ "sessionId": session_id, "playing": self.simple_running(session_id), "levels": levels }),
        )
    }

    /// Apply one small edit through plan/commit, as the UI's autosave does.
    fn simple_edit(
        &mut self,
        method: &str,
        session_id: &EntityId,
        params: &Value,
        key: &str,
    ) -> Result<Value, ControlError> {
        let current = self.get_session(session_id)?.clone();
        let mut candidate = current.clone();
        let mut detail = json!({});
        match method {
            "nodes.set" => {
                let node_id = resolve_node(&current, text_param(params, "node")?)?
                    .id
                    .clone();
                let node = candidate
                    .nodes
                    .iter_mut()
                    .find(|node| node.id == node_id)
                    .expect("resolved");
                let mut changed = false;
                if let Some(parameters) = params.get("parameters") {
                    let parameters = parameters.as_object().ok_or_else(|| {
                        ControlError::InvalidRequest("parameters must be an object".into())
                    })?;
                    for (name, value) in parameters {
                        node.parameters.insert(name.clone(), value.clone());
                        changed = true;
                    }
                }
                for flag in ["enabled", "bypass"] {
                    if let Some(value) = params.get(flag) {
                        let value = value.as_bool().ok_or_else(|| {
                            ControlError::InvalidRequest(format!("{flag} must be true or false"))
                        })?;
                        if flag == "enabled" {
                            node.enabled = value
                        } else {
                            node.bypass = value
                        }
                        changed = true;
                    }
                }
                if let Some(name) = params.get("name") {
                    node.name = name
                        .as_str()
                        .filter(|name| !name.trim().is_empty())
                        .ok_or_else(|| {
                            ControlError::InvalidRequest("name must be non-empty text".into())
                        })?
                        .trim()
                        .to_owned();
                    changed = true;
                }
                if !changed {
                    return Err(ControlError::InvalidRequest(
                        "nothing to change: give parameters, enabled, bypass or name".into(),
                    ));
                }
                detail = json!({ "nodeId": node_id, "name": node.name, "enabled": node.enabled, "bypass": node.bypass, "parameters": node.parameters });
            }
            "nodes.toggle" => {
                let node_id = resolve_node(&current, text_param(params, "node")?)?
                    .id
                    .clone();
                let target = text_param(params, "target")?;
                let node = candidate
                    .nodes
                    .iter_mut()
                    .find(|node| node.id == node_id)
                    .expect("resolved");
                let value = match target {
                    "enabled" => {
                        node.enabled = !node.enabled;
                        json!(node.enabled)
                    }
                    "bypass" => {
                        node.bypass = !node.bypass;
                        json!(node.bypass)
                    }
                    name => {
                        let schema = Self::node_parameter_schema(node.kind);
                        let spec = schema
                            .as_array()
                            .and_then(|items| items.iter().find(|item| item["name"] == name))
                            .cloned()
                            .unwrap_or(Value::Null);
                        let current_value = node
                            .parameters
                            .get(name)
                            .cloned()
                            .or_else(|| spec.get("default").cloned())
                            // An unset on/off setting with no default (Audio
                            // file `loop`) is off.
                            .or_else(|| (spec["type"] == "boolean").then_some(json!(false)))
                            .unwrap_or(Value::Null);
                        let next = match (&current_value, spec["enum"].as_array()) {
                            (Value::Bool(flag), _) => json!(!flag),
                            (_, Some(options)) if options.len() == 2 => if options[0] == current_value { options[1].clone() } else { options[0].clone() },
                            _ => return Err(ControlError::InvalidRequest(format!(
                                "\"{name}\" cannot be toggled; toggle enabled, bypass, an on/off setting or a two-choice setting, or use nodes.set"
                            ))),
                        };
                        node.parameters.insert(name.to_owned(), next.clone());
                        next
                    }
                };
                detail = json!({ "nodeId": node_id, "target": target, "value": value });
            }
            "nodes.add" => {
                let kind_text = text_param(params, "kind")?;
                let kind: NodeKind = serde_json::from_value(json!(kind_text)).map_err(|_| {
                    ControlError::InvalidRequest(format!(
                        "unknown node kind \"{kind_text}\"; see nodes.catalog"
                    ))
                })?;
                let (display, ports, _) = node_template(kind).ok_or_else(|| ControlError::InvalidRequest(format!(
                    "\"{kind_text}\" needs an identity chosen in the AudioRouter window (plugin file, application or device bus)"
                )))?;
                let mut suffix = 1;
                while candidate
                    .nodes
                    .iter()
                    .any(|node| node.id.as_str() == format!("{kind_text}-{suffix}"))
                {
                    suffix += 1;
                }
                let id = EntityId::new(format!("{kind_text}-{suffix}"));
                let mut parameters = default_parameters(kind);
                if let Some(extra) = params.get("parameters") {
                    for (name, value) in extra.as_object().ok_or_else(|| {
                        ControlError::InvalidRequest("parameters must be an object".into())
                    })? {
                        parameters.insert(name.clone(), value.clone());
                    }
                }
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map_or_else(|| format!("{display} {suffix}"), str::to_owned);
                candidate.nodes.push(Node {
                    id: id.clone(),
                    kind,
                    type_version: 1,
                    name: name.clone(),
                    enabled: true,
                    bypass: false,
                    parameters,
                    ports,
                });
                if let Some(between) = params.get("between") {
                    let from = resolve_node(&current, text_param(between, "from")?)?
                        .id
                        .clone();
                    let to = resolve_node(&current, text_param(between, "to")?)?
                        .id
                        .clone();
                    let replaced = candidate
                        .edges
                        .iter()
                        .filter(|edge| edge.source_node == from && edge.destination_node == to)
                        .cloned()
                        .collect::<Vec<_>>();
                    if replaced.is_empty() {
                        return Err(ControlError::InvalidRequest(
                            "those two nodes are not connected; use after, or connect them first"
                                .into(),
                        ));
                    }
                    let [original] = replaced.as_slice() else {
                        return Err(ControlError::InvalidRequest("these nodes have several connections; choose and reconnect their ports explicitly".into()));
                    };
                    insert_on_edge(&mut candidate, original, &id)?;
                } else if let Some(after) = params.get("after").and_then(Value::as_str) {
                    let from = resolve_node(&current, after)?.id.clone();
                    let outgoing = candidate
                        .edges
                        .iter()
                        .filter(|edge| edge.source_node == from)
                        .cloned()
                        .collect::<Vec<_>>();
                    if let [original] = outgoing.as_slice() {
                        insert_on_edge(&mut candidate, original, &id)?;
                    } else {
                        connect(&mut candidate, &from, &id)?;
                    }
                }
                detail = json!({ "nodeId": id, "name": name, "kind": kind_text });
            }
            "nodes.remove" => {
                let node = resolve_node(&current, text_param(params, "node")?)?.clone();
                let incoming = current
                    .edges
                    .iter()
                    .filter(|edge| edge.destination_node == node.id)
                    .cloned()
                    .collect::<Vec<_>>();
                let outgoing = current
                    .edges
                    .iter()
                    .filter(|edge| edge.source_node == node.id)
                    .cloned()
                    .collect::<Vec<_>>();
                candidate
                    .nodes
                    .retain(|candidate_node| candidate_node.id != node.id);
                candidate
                    .edges
                    .retain(|edge| edge.source_node != node.id && edge.destination_node != node.id);
                let bridge = params
                    .get("bridge")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let mut bridged = false;
                if let ([from], [to], true) = (incoming.as_slice(), outgoing.as_slice(), bridge) {
                    bridge_edges(&mut candidate, from, to, &node)?;
                    bridged = true;
                }
                detail = json!({ "nodeId": node.id, "name": node.name, "bridged": bridged });
            }
            "connections.add" => {
                let from = resolve_node(&current, text_param(params, "from")?)?
                    .id
                    .clone();
                let to = resolve_node(&current, text_param(params, "to")?)?
                    .id
                    .clone();
                let edge = connect_ports(
                    &mut candidate,
                    &from,
                    &to,
                    optional_text_param(params, "fromOutput")?,
                    optional_text_param(params, "toInput")?,
                )?;
                detail = json!({ "connectionId": edge });
            }
            "connections.remove" => {
                let from = resolve_node(&current, text_param(params, "from")?)?
                    .id
                    .clone();
                let to = resolve_node(&current, text_param(params, "to")?)?
                    .id
                    .clone();
                let before = candidate.edges.len();
                let from_output = optional_text_param(params, "fromOutput")?;
                let to_input = optional_text_param(params, "toInput")?;
                if let Some(name) = from_output {
                    named_port(
                        resolve_node(&current, from.as_str())?,
                        PortDirection::Output,
                        name,
                    )?;
                }
                if let Some(name) = to_input {
                    named_port(
                        resolve_node(&current, to.as_str())?,
                        PortDirection::Input,
                        name,
                    )?;
                }
                candidate.edges.retain(|edge| {
                    !(edge.source_node == from
                        && edge.destination_node == to
                        && from_output.is_none_or(|name| edge.source_port == name)
                        && to_input.is_none_or(|name| edge.destination_port == name))
                });
                if candidate.edges.len() == before {
                    return Err(ControlError::InvalidRequest(
                        "those two nodes are not connected".into(),
                    ));
                }
                detail = json!({ "removed": before - candidate.edges.len() });
            }
            _ => return Err(ControlError::InvalidRequest("method not found".into())),
        }
        let plan = self.plan_graph(session_id, current.revision, candidate)?;
        let commit =
            self.commit_graph_scoped(&plan, current.revision, &format!("{method}:{key}"), key)?;
        let mut result = detail;
        if let Some(object) = result.as_object_mut() {
            object.insert("sessionId".into(), json!(session_id));
            object.insert(
                "revision".into(),
                commit.get("revision").cloned().unwrap_or(Value::Null),
            );
            object.insert(
                "activation".into(),
                commit.get("activation").cloned().unwrap_or(Value::Null),
            );
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "simple_tests.rs"]
mod tests;
