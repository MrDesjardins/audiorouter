//! Graph planning, commits, history, route inspection and endpoint feedback checks.

use super::*;
use crate::status::{elapsed_micros, LivePublicationTiming};

/// How a committed graph differs from the graph that was playing.
///
/// `FlagsOnly` changes enabled/bypass, `ParametersOnly` changes parameter
/// values without changing node identity, names, ports, edges or session
/// fields, and `Structural` changes anything else. The distinction decides
/// whether the live route can keep its prepared plugins and transports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiveEdit {
    FlagsOnly,
    ParametersOnly,
    Structural,
}

impl LiveEdit {
    /// Whether the prepared source transports (Test Signal, Audio File,
    /// Soundboard) can keep playing through this edit.
    pub(crate) fn retains_sources(self) -> bool {
        !matches!(self, LiveEdit::Structural)
    }
}

/// How source transports survive a rebuild.
#[derive(Clone, Copy)]
pub(crate) enum SourceRetention<'a> {
    /// Rebuild every source (a structural edit or a fresh start).
    None,
    /// Keep every existing transport (a flag-only republish with no saved
    /// session to compare, such as hold-to-compare).
    All,
    /// Keep only sources whose kind and settings are unchanged.
    Unchanged(&'a Session),
}

pub(crate) fn source_retention<'a>(
    live: LiveEdit,
    previous: Option<&'a Session>,
) -> SourceRetention<'a> {
    match (live.retains_sources(), previous) {
        (false, _) => SourceRetention::None,
        (true, Some(previous)) => SourceRetention::Unchanged(previous),
        (true, None) => SourceRetention::All,
    }
}

/// Whether a failed live publication of the saved change may stop and restart
/// the route. Only a layout the playing worker cannot absorb does; a failed
/// preparation must leave the old graph playing.
pub(crate) fn failed_activation_may_restart(error: &ControlError) -> bool {
    matches!(error, ControlError::RestartRequired(_))
}

/// Classify `current` against the previously committed `previous`.
///
/// The commit itself carries a revision, so revision equality is not part of
/// the comparison. A parameter change wins over a simultaneous flag change:
/// parameters may need to be pushed into a live plugin bridge.
pub(crate) fn classify_live_edit(previous: &Session, current: &Session) -> LiveEdit {
    if previous.id != current.id
        || previous.name != current.name
        || previous.edges != current.edges
        || previous.nodes.len() != current.nodes.len()
    {
        return LiveEdit::Structural;
    }
    let mut parameters_changed = false;
    for (before, after) in previous.nodes.iter().zip(&current.nodes) {
        if before.id != after.id
            || before.kind != after.kind
            || before.name != after.name
            || before.ports != after.ports
        {
            return LiveEdit::Structural;
        }
        if before.parameters != after.parameters {
            parameters_changed = true;
        }
    }
    if parameters_changed {
        LiveEdit::ParametersOnly
    } else {
        LiveEdit::FlagsOnly
    }
}

/// Whether a Physical Input renders its 5.1/7.1 capture to two ears
/// (`spatialMode` headphones or speakers).
pub(crate) fn node_spatial_headphones(node: &audiorouter_domain::Node) -> bool {
    node_spatial_options(node).is_some()
}

/// The surround rendering a Physical Input asks for, when it renders one:
/// headphones or speakers (crosstalk cancellation) and the room blend.
pub(crate) fn node_spatial_options(
    node: &audiorouter_domain::Node,
) -> Option<audiorouter_dsp::binaural::SpatialOptions> {
    use audiorouter_dsp::binaural::{SpatialOptions, SpatialOutput};
    if node.kind != NodeKind::PhysicalInput {
        return None;
    }
    let output = match node.parameters.get("spatialMode").and_then(Value::as_str) {
        Some("headphones") => SpatialOutput::Headphones,
        Some("speakers") => SpatialOutput::Speakers,
        _ => return None,
    };
    let room_percent = node
        .parameters
        .get("spatialRoomPercent")
        .and_then(Value::as_f64)
        .unwrap_or(0.0) as f32;
    Some(SpatialOptions {
        output,
        room_percent,
    })
}

/// Preserve prepared transport identities during live flag changes. Silent
/// sources/sinks keep draining; matrices silence every outgoing branch.
pub(crate) fn normalize_live_path_flags(
    session: &mut Session,
    inputs: &[EntityId],
    outputs: &[EntityId],
) {
    for node in &mut session.nodes {
        if !matches!(
            node.kind,
            NodeKind::PhysicalInput
                | NodeKind::ApplicationCapture
                | NodeKind::PhysicalOutput
                | NodeKind::Mixer
                | NodeKind::InputSwitch
        ) {
            continue;
        }
        let unprepared = match node.kind {
            NodeKind::PhysicalInput | NodeKind::ApplicationCapture => !inputs.contains(&node.id),
            NodeKind::PhysicalOutput => !outputs.contains(&node.id),
            _ => false,
        };
        if unprepared && !node.enabled {
            continue;
        }
        if !node.enabled || node.bypass {
            for edge in &mut session.edges {
                if edge.source_node == node.id
                    || (node.kind == NodeKind::PhysicalOutput && edge.destination_node == node.id)
                {
                    edge.matrix.fill(0.0);
                }
            }
        }
        node.enabled = true;
        node.bypass = false;
    }
}

/// Reject a return path through an established endpoint transport. Exact IDs
/// establish the pairing; node names only explain the error.
pub(crate) fn reject_endpoint_feedback(
    session: &Session,
    returns: &[(String, String)],
) -> Result<(), ControlError> {
    for source in session.nodes.iter().filter(|node| {
        node.enabled
            && matches!(
                node.kind,
                NodeKind::PhysicalInput | NodeKind::EndpointLoopback
            )
    }) {
        let Some(capture) = source.parameters.get("endpointId").and_then(Value::as_str) else {
            continue;
        };
        for node in reachable_outputs(session, &source.id) {
            if let Some(render) = node.parameters.get("endpointId").and_then(Value::as_str) {
                if ((source.kind == NodeKind::EndpointLoopback || node_spatial_headphones(source))
                    && render == capture)
                    || returns.iter().any(|(r, c)| r == render && c == capture)
                {
                    return Err(ControlError::InvalidRequest(format!(
                        "Audio feedback loop: \"{}\" feeds \"{}\", whose output returns to that input. You may keep this selection, but playback is blocked until you choose a different output cable or remove the return connection. Use separate cables for source audio and the mixed/recording feed.", source.name, node.name
                    )));
                }
            }
        }
    }
    Ok(())
}

/// The enabled, unbypassed Physical Outputs that audio from `source` reaches
/// through enabled connections. Disabled pass-through tools keep their dry
/// path; a disabled Mixer or Input Switch stops it. Shared by the Play
/// feedback refusal and the setup check (UI-27).
pub(crate) fn reachable_outputs<'s>(
    session: &'s Session,
    source: &EntityId,
) -> Vec<&'s audiorouter_domain::Node> {
    let mut outputs: Vec<&audiorouter_domain::Node> = Vec::new();
    let mut pending = vec![source.clone()];
    let mut visited = std::collections::HashSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        for edge in session
            .edges
            .iter()
            .filter(|edge| edge.enabled && edge.source_node == id)
        {
            let Some(node) = session
                .nodes
                .iter()
                .find(|node| node.id == edge.destination_node)
            else {
                continue;
            };
            if node.kind == NodeKind::PhysicalOutput
                && node.enabled
                && !node.bypass
                && !outputs.iter().any(|output| output.id == node.id)
            {
                outputs.push(node);
            }
            if node.enabled
                || node
                    .ports
                    .iter()
                    .any(|port| port.direction == PortDirection::Input)
                    && node
                        .ports
                        .iter()
                        .any(|port| port.direction == PortDirection::Output)
                    && !matches!(node.kind, NodeKind::Mixer | NodeKind::InputSwitch)
            {
                pending.push(node.id.clone());
            }
        }
    }
    outputs
}

pub(crate) fn graph_diff(before: &Session, after: &Session) -> Vec<Value> {
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

impl ControlPlane {
    pub(crate) fn endpoint_feedback_warnings(
        &self,
        session: &Session,
    ) -> Result<Vec<String>, ControlError> {
        match self.validate_endpoint_feedback(session) {
            Ok(()) => Ok(Vec::new()),
            Err(ControlError::InvalidRequest(message))
                if message.starts_with("Audio feedback loop:") =>
            {
                Ok(vec![message])
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn validate_endpoint_feedback(&self, session: &Session) -> Result<(), ControlError> {
        validate_session(session)
            .map_err(|errors| ControlError::InvalidRequest(format_validation_errors(&errors)))?;
        #[cfg(windows)]
        {
            let endpoints = audiorouter_windows_audio::enumerate_active_endpoint_display_info()
                .map_err(|error| {
                    ControlError::InvalidRequest(format!(
                        "Cannot check output feedback: {error:?}. Refresh devices and retry."
                    ))
                })?;
            let mut returns = Vec::new();
            for render in endpoints.iter().filter(|endpoint| {
                endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render
            }) {
                let Some(key) = audiorouter_windows_audio::known_virtual_cable_key(
                    &render.device_description,
                    &render.driver_inf_section,
                ) else {
                    continue;
                };
                for capture in endpoints.iter().filter(|endpoint| {
                    endpoint.direction == audiorouter_windows_audio::EndpointDirection::Capture
                }) {
                    if audiorouter_windows_audio::known_virtual_cable_key(
                        &capture.device_description,
                        &capture.driver_inf_section,
                    )
                    .as_ref()
                        == Some(&key)
                    {
                        returns.push((render.id.clone(), capture.id.clone()));
                    }
                }
            }
            reject_endpoint_feedback(session, &returns)
        }
        #[cfg(not(windows))]
        reject_endpoint_feedback(session, &[])
    }

    pub fn inspect_routes(
        &self,
        session_id: &EntityId,
        destination_node: &EntityId,
    ) -> Result<Value, ControlError> {
        let session = self.get_session(session_id)?;
        serde_json::to_value(inspect_routes(session, destination_node).map_err(|errors| {
            ControlError::InvalidRequest(format!(
                "invalid graph: {}",
                format_validation_errors(&errors)
            ))
        })?)
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
        let limit = limit.clamp(1, MAX_GRAPH_HISTORY_ITEMS);
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

    pub fn plan_graph(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
        candidate: Session,
    ) -> Result<EntityId, ControlError> {
        self.validate_plugin_placeholders(&candidate)?;
        let checkpoint = self.store.clone();
        let plan_id = self
            .store
            .plan_graph(session_id, base_revision, candidate.clone())
            .map_err(ControlError::from)?;
        if let Some(storage) = &self.storage {
            let expires_at = unix_epoch_seconds() + GRAPH_PLAN_RETENTION_SECONDS;
            if let Err(error) = storage.save_graph_plan(&GraphPlanRecord {
                id: plan_id.as_str().to_owned(),
                session_id: session_id.as_str().to_owned(),
                base_revision,
                candidate,
                expires_at,
            }) {
                self.store = checkpoint;
                return Err(storage_error(error));
            }
        }
        Ok(plan_id)
    }

    /// Prepare a reversible port edit from live worker discovery. This is an
    /// internal backend building block; adapters must not synthesize a binding
    /// from request JSON. The existing plan/commit boundary owns persistence,
    /// revision conflicts and undo. Preparing the plan never publishes audio.
    pub fn plan_bound_plugin_sidechain(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
        node_id: &EntityId,
        binding: &audiorouter_plugin_host::bus_discovery::VerifiedSidechainBinding,
    ) -> Result<EntityId, ControlError> {
        self.ensure_session_loaded(session_id)?;
        let mut candidate = self.get_session(session_id)?.clone();
        let node = candidate
            .nodes
            .iter_mut()
            .find(|node| &node.id == node_id)
            .ok_or_else(|| {
                ControlError::InvalidRequest("sidechain plugin node was not found".into())
            })?;
        let format = node.parameters.get("format").and_then(Value::as_str);
        let class_matches = match format {
            Some("vst3") => {
                node.parameters.get("classId").and_then(Value::as_str) == binding.class_id()
            }
            Some("vst2") => binding.class_id().is_none(),
            _ => false,
        };
        if node.kind != NodeKind::Plugin
            || node.parameters.get("fingerprint").and_then(Value::as_str)
                != Some(binding.fingerprint())
            || !class_matches
        {
            return Err(ControlError::InvalidRequest(
                "sidechain binding does not match this plugin's fingerprint and class".into(),
            ));
        }
        let layout = binding.layout();
        let main = node
            .ports
            .iter()
            .find(|port| port.direction == PortDirection::Input && port.name != "sidechain");
        let output = node
            .ports
            .iter()
            .find(|port| port.direction == PortDirection::Output);
        if layout.input_buses().len() != 2
            || main.is_none_or(|port| u16::from(port.channels) != layout.input_buses()[0])
            || output.is_none_or(|port| u16::from(port.channels) != layout.output_buses()[0])
        {
            return Err(ControlError::InvalidRequest(
                "discovered plugin buses do not match the graph's main ports".into(),
            ));
        }
        let channels = layout.input_buses()[1] as u8;
        if let Some(port) = node.plugin_sidechain_port() {
            if port.channels != channels {
                return Err(ControlError::InvalidRequest("existing sidechain port width differs from discovery; remove its connections before changing the port".into()));
            }
        } else {
            node.ports.push(audiorouter_domain::Port {
                name: "sidechain".into(),
                direction: PortDirection::Input,
                channels,
            });
        }
        // Validation includes the entire candidate and every incident edge;
        // no existing channel map is rewritten to accommodate the new port.
        self.plan_graph(session_id, base_revision, candidate)
    }

    pub fn commit_graph(
        &mut self,
        plan_id: &EntityId,
        base_revision: u64,
        idempotency_key: &str,
    ) -> Result<Value, ControlError> {
        self.commit_graph_scoped(
            plan_id,
            base_revision,
            idempotency_key,
            idempotency_key,
            None,
        )
    }

    pub(crate) fn commit_graph_scoped(
        &mut self,
        plan_id: &EntityId,
        base_revision: u64,
        idempotency_key: &str,
        display_operation_id: &str,
        require_stopped_session: Option<EntityId>,
    ) -> Result<Value, ControlError> {
        let commit_started = std::time::Instant::now();
        // A plan can wait between preparation and commit. Re-check that every
        // plugin placeholder still has a current explicit scan result, so a
        // queued port or graph edit cannot bypass capability verification.
        if let Some(candidate) = self.store.plan_candidate(plan_id) {
            self.validate_plugin_placeholders(candidate)?;
        }
        let checkpoint = self.store.clone();
        let fingerprint = match &require_stopped_session {
            Some(session_id) => format!(
                "graph.commit:{}:{}:requireStoppedSessionId:{}",
                plan_id.as_str(),
                base_revision,
                session_id.as_str()
            ),
            None => format!("graph.commit:{}:{}", plan_id.as_str(), base_revision),
        };
        let request_hash = audiorouter_storage::lower_hex(&Sha256::digest(fingerprint.as_bytes()));
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
        if let Some(session_id) = &require_stopped_session {
            if self
                .runtimes
                .get(session_id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
                || self.native_worker_attached_to(session_id)
            {
                return Err(ControlError::InvalidRequest(
                    "this graph commit requires its session to be stopped".into(),
                ));
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
                let Some(remaining) = remaining_persisted_plan_duration(
                    durable.expires_at,
                    unix_epoch_seconds(),
                    Duration::from_secs(GRAPH_PLAN_RETENTION_SECONDS as u64),
                ) else {
                    storage
                        .delete_graph_plan(plan_id.as_str())
                        .map_err(storage_error)?;
                    return Err(ControlError::from(
                        audiorouter_domain::StoreError::PlanExpired,
                    ));
                };
                let durable_session_id = EntityId::new(durable.session_id.clone());
                self.ensure_session_loaded(&durable_session_id)?;
                self.store
                    .restore_plan_with_ttl(
                        EntityId::new(durable.id),
                        &durable_session_id,
                        durable.base_revision,
                        durable.candidate,
                        remaining,
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
            // The session and journal are already durable. Cleanup cannot
            // turn that success into a failure or skip live activation. A
            // leftover plan expires normally and its old base revision
            // prevents another commit; the same key replays the journal.
            let _ = storage.delete_graph_plan(plan_id.as_str());
        }
        // Committing runs on the audio service thread; let running audio
        // advance between the durable write and the graph rebuild so a save
        // while playing cannot starve the outputs (heard as a click).
        self.service_running_native_audio(std::time::Instant::now());
        if !result.idempotent_replay {
            // A save replaces what is playing, so any held comparison ends here.
            self.clear_comparison(&result.session_id);
            self.note_saved_revision(&result.session_id, result.revision);
            self.events.append(
                result.revision,
                Some(display_operation_id.into()),
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
            let runtime = self
                .runtimes
                .get_mut(&result.session_id)
                .ok_or(ControlError::Internal("committed session runtime missing"))?;
            runtime.prepare(&session).map_err(|error| match error {
                RuntimeError::InvalidGraph(errors) => ControlError::InvalidRequest(format!(
                    "invalid graph: {}",
                    format_validation_errors(&errors)
                )),
                RuntimeError::NotPrepared => {
                    ControlError::InvalidRequest("session was not prepared".into())
                }
            })?;
            let generation = runtime
                .start()
                .map_err(|_| ControlError::InvalidRequest("session was not prepared".into()))?;
            // Keep the prepared graph pumping while the new one is compiled
            // and its path set is built. The worker swap below is atomic; a
            // service pass in the middle of preparation must keep serving the
            // old graph instead of pausing the route.
            #[cfg(windows)]
            if self.native_multi_input_worker_session.as_ref() == Some(&result.session_id)
                && self.native_multi_input_worker.is_some()
            {
                self.native_multi_input_applied_generation = Some(generation);
            }
            self.events.append(
                result.revision,
                Some(display_operation_id.into()),
                "runtime.activated",
                Some(result.session_id.clone()),
            );
            // Apply the saved change to the audio already playing (GRAPH-08):
            // a parameter edit such as a Volume or Mixer input slider takes
            // effect without Stop/Play. A topology change that the running
            // adapter cannot absorb reports `restartRequired` instead.
            let previous_session = checkpoint.session(&result.session_id).cloned();
            // After a failed live application the playing graph is older than
            // the saved session. Never reuse that graph's plugins from it: try
            // a full preparation of the saved graph instead.
            let stale = self.live_graph_stale.contains(&result.session_id);
            let live = if stale {
                LiveEdit::Structural
            } else {
                previous_session
                    .as_ref()
                    .map_or(LiveEdit::Structural, |previous| {
                        classify_live_edit(previous, &session)
                    })
            };
            // Network sockets are independent of the plugin graph and are
            // always reconfigured from the previously saved session.
            #[cfg(windows)]
            let network = match previous_session.clone() {
                Some(previous) => self
                    .reconfigure_running_network_nodes(&result.session_id, &previous)
                    .map(|_| ()),
                None => Ok(()),
            };
            #[cfg(not(windows))]
            let network: Result<(), ControlError> = Ok(());
            let reuse_base = if stale {
                None
            } else {
                previous_session.as_ref()
            };
            let mut generation = generation;
            let _ = self.live_publication_warnings.remove(&result.session_id);
            let mut publication_timing: Option<LivePublicationTiming> = None;
            let native = match network.and_then(|()| {
                self.republish_running_native_graph(
                    &result.session_id,
                    generation,
                    live,
                    reuse_base,
                )
            }) {
                Ok(Some((adapter, timing))) => {
                    publication_timing = Some(timing);
                    json!({ "state": "applied", "adapter": adapter })
                }
                Ok(None) => {
                    self.record_live_publication(&result.session_id, live);
                    Value::Null
                }
                Err(error) => {
                    let reason = control_error_message(&error);
                    // Only a change the playing layout cannot absorb may stop
                    // and restart the route. A failed preparation (a plugin
                    // that will not load, a missing file, a compile error)
                    // must leave the old graph playing: restarting would stop
                    // working audio and then fail the same way.
                    if !failed_activation_may_restart(&error) {
                        self.live_graph_stale.insert(result.session_id.clone());
                        json!({ "state": "restartRequired", "reason": reason })
                    } else {
                        match self.restart_or_keep_multi_input_route(&result.session_id, generation)
                        {
                            Some(Ok(restarted)) => {
                                generation = restarted;
                                self.live_graph_stale.remove(&result.session_id);
                                json!({ "state": "restarted", "adapter": "multi-input", "reason": reason })
                            }
                            Some(Err(restart)) => {
                                self.live_graph_stale.insert(result.session_id.clone());
                                json!({
                                    "state": "restartRequired",
                                    "reason": format!("{reason}; automatic restart failed: {}", control_error_message(&restart)),
                                })
                            }
                            None => {
                                self.live_graph_stale.insert(result.session_id.clone());
                                json!({ "state": "restartRequired", "reason": reason })
                            }
                        }
                    }
                }
            };
            let warning = self.live_publication_warnings.remove(&result.session_id);
            let still_running = self
                .runtimes
                .get(&result.session_id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running);
            let mut activation = json!({
                "state": if still_running { "running" } else { "stopped" },
                "generation": generation,
                "runtime": if native.is_null() { "fake" } else { "native" },
                "native": native,
                "timing": {
                    "commitMicros": elapsed_micros(commit_started),
                    "prepareMicros": publication_timing.map(|timing| timing.prepare_micros),
                    "swapMicros": publication_timing.map(|timing| timing.swap_micros),
                    "pluginStarts": publication_timing.map(|timing| timing.plugin_starts),
                },
            });
            if let Some(warning) = warning {
                activation["native"]["warning"] = json!(warning);
            }
            response["activation"] = activation;
        } else {
            // A simulated commit publishes nothing, so there is no preparation
            // to time. Keeping the shape free of per-request numbers also keeps
            // an idempotent replay equal to the response it replays.
            response["activation"] = json!({ "state": "pending", "runtime": "fake" });
        }
        self.remember_operation_outcome(idempotency_key, response.clone(), "graph.commit", None);
        Ok(response)
    }

    pub(crate) fn dispatch_plan(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let plan_started = Instant::now();
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
        let warnings = self.endpoint_feedback_warnings(&candidate)?;
        let plan_id = self.plan_graph(&session_id, base_revision, candidate)?;
        Ok(json!({
            "planId": plan_id,
            "baseRevision": base_revision,
            "expiresInMs": 300000,
            "diff": diff,
            "affectedDestinations": affected_destinations,
            "warnings": warnings,
            "requiredScopes": ["graph.write"],
            "timing": { "planMicros": elapsed_micros(plan_started) }
        }))
    }

    pub(crate) fn dispatch_commit(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("graph.commit params are required".into())
        })?;
        let warning_plan_id: EntityId =
            serde_json::from_value(params.get("planId").cloned().unwrap_or(Value::Null))
                .map_err(|_| ControlError::InvalidRequest("invalid planId".into()))?;
        let candidate = match self.store.plan_candidate(&warning_plan_id) {
            Some(candidate) => Some(candidate.clone()),
            None => match self.storage.as_ref() {
                Some(storage) => storage
                    .load_graph_plan(warning_plan_id.as_str())
                    .map_err(storage_error)?
                    .map(|plan| plan.candidate),
                None => None,
            },
        };
        let require_stopped_session = params
            .get("requireStoppedSessionId")
            .map(|value| {
                value
                    .as_str()
                    .filter(|id| {
                        !id.is_empty() && id.len() <= audiorouter_domain::MAX_ENTITY_ID_BYTES
                    })
                    .map(EntityId::new)
                    .ok_or_else(|| {
                        ControlError::InvalidRequest(
                            "requireStoppedSessionId must be a non-empty session ID".into(),
                        )
                    })
            })
            .transpose()?;
        if candidate.as_ref().is_some_and(|candidate| {
            require_stopped_session
                .as_ref()
                .is_some_and(|session_id| candidate.id != *session_id)
        }) {
            return Err(ControlError::InvalidRequest(
                "requireStoppedSessionId does not match the graph plan".into(),
            ));
        }
        let expected_warnings = candidate
            .as_ref()
            .map(|candidate| self.endpoint_feedback_warnings(candidate))
            .transpose()?
            .unwrap_or_default();
        let supplied = params.get("acknowledgments").and_then(Value::as_array);
        if expected_warnings.iter().any(|warning| {
            !supplied.is_some_and(|items| items.iter().any(|item| item.as_str() == Some(warning)))
        }) {
            return Err(ControlError::InvalidRequest("Review and acknowledge this plan's audio feedback warning before saving. Playback will remain blocked.".into()));
        }
        if let Some(acknowledgments) = params
            .get("acknowledgments")
            .filter(|value| !value.is_null())
        {
            let acknowledgments = acknowledgments.as_array().ok_or_else(|| {
                ControlError::InvalidRequest("acknowledgments must be an array or null".into())
            })?;
            if acknowledgments.len() > 100
                || acknowledgments.iter().any(|value| match value.as_str() {
                    Some(value) => value.is_empty() || value.len() > 2048,
                    None => true,
                })
            {
                return Err(ControlError::InvalidRequest(
                    "acknowledgments must contain non-empty warning IDs".into(),
                ));
            }
            if acknowledgments.iter().any(|value| {
                !expected_warnings
                    .iter()
                    .any(|warning| value.as_str() == Some(warning))
                    && (candidate.is_some()
                        || !value
                            .as_str()
                            .is_some_and(|warning| warning.starts_with("Audio feedback loop:")))
            }) {
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
        // Check the key the caller sent, not the scoped one: with a client
        // identity in front of it an empty key looked valid and was answered
        // as an unknown plan.
        if key.is_empty() {
            return Err(audiorouter_domain::StoreError::EmptyIdempotencyKey.into());
        }
        if key.len() > audiorouter_domain::MAX_ENTITY_ID_BYTES {
            return Err(audiorouter_domain::StoreError::IdempotencyKeyTooLong.into());
        }
        let scoped_key = self.scoped_idempotency_key("graph.commit", key);
        self.commit_graph_scoped(
            &plan_id,
            base_revision,
            &scoped_key,
            key,
            require_stopped_session,
        )
    }

    pub(crate) fn dispatch_meter_reset(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let id = session_id_from_params(params.clone())?;
        let node_id = params
            .as_ref()
            .and_then(|p| p["nodeId"].as_str())
            .filter(|s| !s.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("nodeId is required".into()))?;
        self.ensure_session_loaded(&id)?;
        if !self
            .get_session(&id)?
            .nodes
            .iter()
            .any(|n| n.id == node_id && n.kind == NodeKind::Meter)
        {
            return Err(ControlError::InvalidRequest(
                "Choose an existing Meter node in this session".into(),
            ));
        }
        let mut reset = self
            .native_endpoint_worker_for_session(&id)
            .is_some_and(|w| {
                w.bridge()
                    .scheduler()
                    .processor()
                    .reset_meter_for_node(&node_id)
            });
        #[cfg(windows)]
        if self.native_multi_input_worker_session.as_ref() == Some(&id) {
            if let Some(worker) = self.native_multi_input_worker.as_ref() {
                reset |= worker.reset_meter_for_node(&node_id);
            }
        }
        if !reset {
            return Err(ControlError::InvalidRequest(
                "Meter is not prepared; press Play before resetting readings".into(),
            ));
        }
        Ok(json!({"sessionId":id,"nodeId":node_id,"reset":true}))
    }

    pub(crate) fn dispatch_routes_inspect(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        // Task-method rules: the open session by default, and the node by
        // its ID or exact name.
        let params = self.with_open_session(params)?;
        let session_id: EntityId = serde_json::from_value(params["sessionId"].clone())
            .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let reference = params
            .get("destinationNode")
            .and_then(Value::as_str)
            .filter(|reference| !reference.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("destinationNode is required".into()))?;
        let session = self.get_session(&session_id)?;
        let destination_node = crate::simple::resolve_node(session, reference)?.id.clone();
        self.inspect_routes(&session_id, &destination_node)
    }

    pub(crate) fn dispatch_graph_history(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
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
                let cursor = value.as_str().ok_or_else(|| {
                    ControlError::InvalidRequest("cursor must be a string".into())
                })?;
                if cursor.len() > MAX_REVISION_CURSOR_BYTES {
                    return Err(ControlError::InvalidRequest(
                        "history cursor exceeds the maximum length".into(),
                    ));
                }
                cursor
                    .parse::<u64>()
                    .map_err(|_| ControlError::InvalidRequest("invalid history cursor".into()))
            })
            .transpose()?;
        self.graph_history_page(&session_id, before_revision, limit as usize)
    }

    pub(crate) fn dispatch_graph_undo_plan(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
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
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
