//! Graph planning, commits, history, route inspection and endpoint feedback checks.

use super::*;

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
        let mut pending = vec![source.id.clone()];
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
                if node.kind == NodeKind::PhysicalOutput && node.enabled && !node.bypass {
                    if let Some(render) = node.parameters.get("endpointId").and_then(Value::as_str)
                    {
                        if ((source.kind == NodeKind::EndpointLoopback
                            || node_spatial_headphones(source))
                            && render == capture)
                            || returns.iter().any(|(r, c)| r == render && c == capture)
                        {
                            return Err(ControlError::InvalidRequest(format!(
                                "Audio feedback loop: \"{}\" feeds \"{}\", whose output returns to that input. You may keep this selection, but playback is blocked until you choose a different output cable or remove the return connection. Use separate cables for source audio and the mixed/recording feed.", source.name, node.name
                            )));
                        }
                    }
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
    }
    Ok(())
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

    pub fn commit_graph(
        &mut self,
        plan_id: &EntityId,
        base_revision: u64,
        idempotency_key: &str,
    ) -> Result<Value, ControlError> {
        self.commit_graph_scoped(plan_id, base_revision, idempotency_key, idempotency_key)
    }

    pub(crate) fn commit_graph_scoped(
        &mut self,
        plan_id: &EntityId,
        base_revision: u64,
        idempotency_key: &str,
        display_operation_id: &str,
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
            storage
                .delete_graph_plan(plan_id.as_str())
                .map_err(storage_error)?;
        }
        // Committing runs on the audio service thread; let running audio
        // advance between the durable write and the graph rebuild so a save
        // while playing cannot starve the outputs (heard as a click).
        self.service_running_native_audio(std::time::Instant::now());
        if !result.idempotent_replay {
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
            let runtime = self.runtimes.get_mut(&result.session_id).unwrap();
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
            let flags_only = checkpoint
                .session(&result.session_id)
                .map(|previous| {
                    let mut normalized = session.clone();
                    normalized.revision = previous.revision;
                    for node in &mut normalized.nodes {
                        if let Some(old) = previous.nodes.iter().find(|old| old.id == node.id) {
                            node.enabled = old.enabled;
                            node.bypass = old.bypass;
                        }
                    }
                    serde_json::to_value(&normalized).ok() == serde_json::to_value(previous).ok()
                })
                .unwrap_or(false);
            self.service_running_native_audio(std::time::Instant::now());
            // Network sockets are reconfigured first; a failure there must
            // not be reported as applied.
            #[cfg(windows)]
            let network = match checkpoint.session(&result.session_id).cloned() {
                Some(previous) => self
                    .reconfigure_running_network_nodes(&result.session_id, &previous)
                    .map(|_| ()),
                None => Ok(()),
            };
            #[cfg(not(windows))]
            let network: Result<(), ControlError> = Ok(());
            let mut generation = generation;
            let native = match network.and_then(|()| {
                self.republish_running_native_graph(&result.session_id, generation, flags_only)
            }) {
                Ok(Some(adapter)) => json!({ "state": "applied", "adapter": adapter }),
                Ok(None) => Value::Null,
                Err(error) => {
                    let reason = control_error_message(&error);
                    match self.restart_or_keep_multi_input_route(&result.session_id, generation) {
                        Some(Ok(restarted)) => {
                            generation = restarted;
                            json!({ "state": "restarted", "adapter": "multi-input", "reason": reason })
                        }
                        Some(Err(restart)) => json!({
                            "state": "restartRequired",
                            "reason": format!("{reason}; automatic restart failed: {}", control_error_message(&restart)),
                        }),
                        None => json!({ "state": "restartRequired", "reason": reason }),
                    }
                }
            };
            let still_running = self
                .runtimes
                .get(&result.session_id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running);
            response["activation"] = json!({
                "state": if still_running { "running" } else { "stopped" },
                "generation": generation,
                "runtime": if native.is_null() { "fake" } else { "native" },
                "native": native,
            });
        } else {
            response["activation"] = json!({ "state": "pending", "runtime": "fake" });
        }
        self.remember_operation_outcome(idempotency_key, response.clone(), "graph.commit", None);
        Ok(response)
    }

    pub(crate) fn dispatch_plan(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
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
            "requiredScopes": ["graph.write"]
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
        let scoped_key = self.scoped_idempotency_key("graph.commit", key);
        self.commit_graph_scoped(&plan_id, base_revision, &scoped_key, key)
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
