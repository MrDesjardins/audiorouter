//! Sessions: create, duplicate, delete, list, start, stop, import and export.

use super::*;

pub(crate) fn session_runtime_label(native_attached: bool) -> &'static str {
    if native_attached {
        "native"
    } else {
        "fake"
    }
}

pub(crate) fn session_id_from_params(params: Option<Value>) -> Result<EntityId, ControlError> {
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

/// An absolute path to a `.audiorouter` session file.
pub(crate) fn session_file_path(
    params: Option<&Value>,
) -> Result<std::path::PathBuf, ControlError> {
    let path = params
        .and_then(|params| params.get("path"))
        .and_then(Value::as_str)
        .filter(|path| {
            !path.is_empty() && path.len() <= 1024 && !path.chars().any(char::is_control)
        })
        .map(std::path::PathBuf::from)
        .ok_or_else(|| ControlError::InvalidRequest("path is required".into()))?;
    let extension_ok = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("audiorouter"));
    if !path.is_absolute() || !extension_ok {
        return Err(ControlError::InvalidRequest(
            "path must be an absolute path to a .audiorouter file".into(),
        ));
    }
    Ok(path)
}

impl ControlPlane {
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
        if self.active_session_id.is_none() {
            self.active_session_id = Some(session.id.clone());
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
        let duplicate_id = duplicate.id.clone();
        let result = self.create_session(duplicate)?;
        // The copy keeps the plugins' latest settings. Shared captures are
        // never deleted while any node still restores them.
        if let Some(storage) = &self.storage {
            for (node_id, state_id) in storage
                .plugin_node_states(source_id.as_str())
                .map_err(storage_error)?
            {
                storage
                    .set_plugin_node_state(duplicate_id.as_str(), &node_id, &state_id)
                    .map_err(storage_error)?;
            }
        }
        Ok(result)
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
        if self
            .native_endpoint_worker_for_session(id)
            .is_some_and(audiorouter_windows_audio::NativeAudioWorker::is_running)
        {
            return Err(ControlError::InvalidRequest(
                "stop the native endpoint worker before deleting the session".into(),
            ));
        }
        #[cfg(windows)]
        if self.native_duplex_worker_session.as_ref() == Some(id)
            && self
                .native_duplex_worker
                .as_ref()
                .is_some_and(|worker| worker.is_running())
        {
            return Err(ControlError::InvalidRequest(
                "stop the native duplex worker before deleting the session".into(),
            ));
        }
        #[cfg(windows)]
        if self.native_render_source_worker_session.as_ref() == Some(id)
            && self
                .native_render_source_worker
                .as_ref()
                .is_some_and(|worker| worker.is_running())
        {
            return Err(ControlError::InvalidRequest(
                "stop the native render-source worker before deleting the session".into(),
            ));
        }
        #[cfg(windows)]
        if self.native_output_fanout_session.as_ref() == Some(id)
            && self
                .native_output_fanout
                .as_ref()
                .is_some_and(|fanout| fanout.is_running())
        {
            return Err(ControlError::InvalidRequest(
                "stop the native output fan-out before deleting the session".into(),
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
        self.deactivate_virtual_route_bridges(id);
        // A stopped worker is part of the deleted session's transient
        // ownership, not durable session state. Retain it through the
        // persistence operation so a failed delete leaves the owner intact;
        // only a successful store mutation may clear the binding and taps.
        if self.native_endpoint_session_is_attached(id) {
            if let Some(worker_slot) = self.native_endpoint_worker_slot_for_session_mut(id) {
                worker_slot.take();
            }
            if self.native_endpoint_session.as_ref() == Some(id) {
                self.native_endpoint_session = None;
                self.native_endpoint_taps = None;
            } else {
                self.native_endpoint_session_secondary = None;
                self.native_endpoint_taps_secondary = None;
            }
        }
        #[cfg(windows)]
        if self.native_duplex_worker_session.as_ref() == Some(id) {
            self.native_duplex_worker.take();
            self.native_duplex_worker_session = None;
            self.native_duplex_worker_generation = None;
        }
        #[cfg(windows)]
        if self.native_render_source_worker_session.as_ref() == Some(id) {
            self.native_render_source_worker.take();
            self.native_render_source_worker_session = None;
            self.native_render_source_worker_generation = None;
            self.native_render_source_taps = None;
        }
        #[cfg(windows)]
        if self.native_output_fanout_session.as_ref() == Some(id) {
            self.native_output_fanout.take();
            self.native_output_fanout_session = None;
            self.native_output_fanout_generation = None;
        }
        #[cfg(windows)]
        if self.native_multi_input_worker_session.as_ref() == Some(id) {
            self.native_multi_input_worker.take();
            self.native_multi_input_worker_session = None;
            self.native_multi_input_worker_generation = None;
            self.native_multi_input_applied_generation = None;
            #[cfg(windows)]
            self.multi_input_application_sources.clear();
        }
        self.runtimes.remove(id);
        for node in session
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Recorder)
        {
            self.recorder_node_workers.remove(&node.id);
            self.sync_recorder_inlet(&node.id);
        }
        self.events
            .append(session.revision, None, "session.deleted", Some(id.clone()));
        Ok(json!({ "sessionId": id, "deleted": true }))
    }

    pub fn get_session(&self, id: &EntityId) -> Result<&Session, ControlError> {
        self.store
            .session(id)
            .ok_or(ControlError::InvalidRequest("session not found".into()))
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
        if !(1..=MAX_SESSION_LIST_ITEMS).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        // The UI can hold a session in the live graph inventory while the
        // durable index is being repaired or has not yet been refreshed.
        // Merge both views so API inventory matches the sessions this backend
        // can actually open; live graph values win for duplicate IDs.
        let mut by_id = BTreeMap::<String, Session>::new();
        if let Some(storage) = &self.storage {
            for session in storage
                .list_sessions_after(cursor, limit)
                .map_err(storage_error)?
            {
                by_id.insert(session.id.as_str().to_owned(), session);
            }
        }
        for session in self.store.sessions_after(cursor, limit) {
            by_id.insert(session.id.as_str().to_owned(), session);
        }
        let sessions = by_id.into_values().take(limit).collect::<Vec<_>>();
        let next_cursor = (sessions.len() == limit)
            .then(|| {
                sessions
                    .last()
                    .map(|session| session.id.as_str().to_owned())
            })
            .flatten();
        Ok(json!({ "items": sessions, "nextCursor": next_cursor }))
    }

    pub fn session_start(&mut self, id: &EntityId) -> Result<Value, ControlError> {
        self.session_start_with_candidate(id, None)
    }

    pub fn session_preview_start(
        &mut self,
        id: &EntityId,
        candidate: &Session,
    ) -> Result<Value, ControlError> {
        self.session_start_with_candidate(id, Some(candidate))
    }

    pub(crate) fn session_start_with_candidate(
        &mut self,
        id: &EntityId,
        candidate: Option<&Session>,
    ) -> Result<Value, ControlError> {
        self.ensure_session_loaded(id)?;
        let saved_session = self.get_session(id)?.clone();
        let saved_revision = saved_session.revision;
        let session = if let Some(candidate) = candidate {
            if candidate.id != *id || candidate.revision != saved_session.revision {
                return Err(ControlError::InvalidRequest(
                    "preview candidate must target the current saved session revision".into(),
                ));
            }
            if !self.native_endpoint_session_is_attached(id)
                || self.native_multi_input_worker_session.as_ref() == Some(id)
            {
                return Err(ControlError::InvalidRequest(
                "temporary draft preview requires a prepared single-endpoint audio route. Multiple capture-source mixes currently require saving the Mixer route and preparing the exact sources and outputs in Devices".into(),
                ));
            }
            self.validate_plugin_placeholders(candidate)?;
            candidate.clone()
        } else {
            saved_session
        };
        if candidate.is_some()
            && self
                .runtimes
                .get(id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
        {
            return Err(ControlError::InvalidRequest(
                "stop the active session before previewing a changed route".into(),
            ));
        }
        let has_enabled_plugin = session
            .nodes
            .iter()
            .any(|node| node.enabled && node.kind == NodeKind::Plugin);
        let native_endpoint_attached = self.native_endpoint_session_is_attached(id);
        let mut native_graph_attached = native_endpoint_attached;
        let mut native_attached = native_endpoint_attached;
        #[cfg(windows)]
        {
            let native_multi_input_attached = self.native_multi_input_worker_session.as_ref()
                == Some(id)
                && self.native_multi_input_worker.is_some();
            let native_duplex_attached = self.native_duplex_worker_session.as_ref() == Some(id)
                && self.native_duplex_worker.is_some();
            // The multi-input worker binds its plugin stages while it is
            // prepared (`prepare_native_path_worker`), so it can run them.
            native_graph_attached =
                native_graph_attached || native_duplex_attached || native_multi_input_attached;
            native_graph_attached = native_graph_attached
                || (self.native_render_source_worker_session.as_ref() == Some(id)
                    && self.native_render_source_worker.is_some());
            native_attached = native_attached
                || native_multi_input_attached
                || native_duplex_attached
                || (self.native_render_source_worker_session.as_ref() == Some(id)
                    && self.native_render_source_worker.is_some());
        }
        if has_enabled_plugin && !native_graph_attached {
            return Err(ControlError::InvalidRequest(
                "Audio route is not prepared. Before session.start or sessions.start, call nativePaths.prepare with this sessionId (HTTP: POST /api/v1/nativePaths/prepare), then retry Start with a new idempotencyKey. Preparation uses the saved node devices and requires DeviceAdministration permission; desktop Play performs preparation first.".into(),
            ));
        }
        if let Some(runtime) = self.runtimes.get(id) {
            if runtime.state() == RuntimeState::Running {
                return Ok(
                    json!({ "sessionId": id, "state": "running", "generation": runtime.generation(), "runtime": session_runtime_label(native_attached) }),
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

        // A session with an explicitly attached endpoint pair must cross the
        // native boundary as part of the same start operation.  Previously we
        // left the native worker stopped and reported the fake runtime as
        // running, which made the control plane appear healthy while no
        // samples could reach the endpoint.  Prepare and publish the graph
        // before starting the worker; any failure rolls back the runtime and
        // selected bridge generation so the session cannot be half-started.
        #[cfg(windows)]
        if self.native_multi_input_worker_session.as_ref() == Some(id)
            && self.native_multi_input_worker.is_some()
        {
            let branch_node_ids = self
                .native_multi_input_worker
                .as_ref()
                .expect("multi-input worker is attached")
                .output_node_ids()
                .to_vec();
            let capture_bus_ids = session_virtual_capture_bus_ids(&session);
            if let Err(error) = self
                .prepare_virtual_route_bridges(id, generation, &capture_bus_ids)
                .and_then(|_| {
                    self.bind_native_multi_input_branches(id, generation, &branch_node_ids)
                        .map(|_| ())
                })
                .and_then(|_| self.start_native_multi_input_worker())
            {
                let _ = self
                    .native_multi_input_worker
                    .as_mut()
                    .expect("multi-input worker is attached")
                    .stop();
                if let Some(runtime) = self.runtimes.get_mut(id) {
                    runtime.stop();
                }
                self.deactivate_virtual_route_bridges(id);
                self.native_endpoint_taps = None;
                self.native_render_source_taps = None;
                return Err(error);
            }
        }

        if native_endpoint_attached {
            let sample_rate_hz = self
                .native_endpoint_worker_for_session(id)
                .expect("native_attached implies an endpoint worker")
                .bridge()
                .sample_rate_hz();
            let activated = if candidate.is_some() {
                self.activate_native_graph_candidate(id, generation, sample_rate_hz, Some(&session))
            } else {
                self.activate_native_graph(id, generation, sample_rate_hz)
            };
            if let Err(error) = activated {
                if let Some(runtime) = self.runtimes.get_mut(id) {
                    runtime.stop();
                }
                self.deactivate_virtual_route_bridges(id);
                if let Some(taps) = self.native_endpoint_taps_for_session_mut(id) {
                    *taps = None;
                }
                self.native_render_source_taps = None;
                return Err(error);
            }
            #[cfg(windows)]
            let application_waiting = self
                .application_capture_runtime
                .as_ref()
                .is_some_and(|binding| binding.session_id == *id)
                && self.maintain_application_capture(id, true)?;
            #[cfg(not(windows))]
            let application_waiting = false;
            if !application_waiting {
                let start_result = self
                    .native_endpoint_worker_for_session_mut(id)
                    .expect("native_attached implies an endpoint worker")
                    .start()
                    .map_err(audio_control_error);
                if let Err(error) = start_result {
                    if let Some(runtime) = self.runtimes.get_mut(id) {
                        runtime.stop();
                    }
                    self.deactivate_virtual_route_bridges(id);
                    if let Some(taps) = self.native_endpoint_taps_for_session_mut(id) {
                        *taps = None;
                    }
                    self.native_render_source_taps = None;
                    return Err(error);
                }
                #[cfg(windows)]
                if self
                    .application_capture_runtime
                    .as_ref()
                    .is_some_and(|binding| binding.session_id == *id)
                {
                    self.set_application_capture_state(
                        "connected",
                        "Connected to this application. Audio flow appears when the app produces sound.",
                    );
                }
            }
            #[cfg(windows)]
            if self.native_output_fanout_session.as_ref() == Some(id)
                && self.native_output_fanout.is_some()
            {
                if let Err(error) = self.start_native_output_fanout() {
                    let _ = self
                        .native_endpoint_worker_for_session_mut(id)
                        .expect("native_attached implies an endpoint worker")
                        .stop();
                    if let Some(runtime) = self.runtimes.get_mut(id) {
                        runtime.stop();
                    }
                    self.deactivate_virtual_route_bridges(id);
                    if let Some(taps) = self.native_endpoint_taps_for_session_mut(id) {
                        *taps = None;
                    }
                    return Err(error);
                }
            }
        }
        #[cfg(windows)]
        if !native_endpoint_attached
            && self.native_duplex_worker_session.as_ref() == Some(id)
            && self.native_duplex_worker.is_some()
        {
            if let Err(error) = self.start_native_duplex_worker() {
                if let Some(runtime) = self.runtimes.get_mut(id) {
                    runtime.stop();
                }
                self.deactivate_virtual_route_bridges(id);
                return Err(error);
            }
        }
        #[cfg(windows)]
        if self.native_render_source_worker_session.as_ref() == Some(id)
            && self.native_render_source_worker.is_some()
        {
            let sample_rate_hz = self
                .native_render_source_worker
                .as_ref()
                .expect("native render-source worker is attached")
                .bridge()
                .sample_rate_hz();
            if let Err(error) =
                self.activate_native_render_source_graph(id, generation, sample_rate_hz)
            {
                let _ = self
                    .native_duplex_worker
                    .as_mut()
                    .filter(|_| self.native_duplex_worker_session.as_ref() == Some(id))
                    .map(audiorouter_windows_audio::NativeBridgeDuplexWorker::stop);
                if let Some(runtime) = self.runtimes.get_mut(id) {
                    runtime.stop();
                }
                self.deactivate_virtual_route_bridges(id);
                self.native_endpoint_taps = None;
                return Err(error);
            }
            if let Err(error) = self.start_native_render_source_worker() {
                let _ = self
                    .native_duplex_worker
                    .as_mut()
                    .filter(|_| self.native_duplex_worker_session.as_ref() == Some(id))
                    .map(audiorouter_windows_audio::NativeBridgeDuplexWorker::stop);
                let _ = self
                    .native_endpoint_worker
                    .as_mut()
                    .filter(|_| self.native_endpoint_session.as_ref() == Some(id))
                    .map(audiorouter_windows_audio::NativeAudioWorker::stop);
                let _ = self
                    .native_output_fanout
                    .as_mut()
                    .filter(|_| self.native_output_fanout_session.as_ref() == Some(id))
                    .map(audiorouter_windows_audio::WasapiOutputFanout::stop);
                if let Some(runtime) = self.runtimes.get_mut(id) {
                    runtime.stop();
                }
                self.deactivate_virtual_route_bridges(id);
                self.native_endpoint_taps = None;
                return Err(error);
            }
        }
        self.events
            .append(session.revision, None, "runtime.started", Some(id.clone()));
        Ok(json!({
            "sessionId": id,
            "state": "running",
            "generation": generation,
            "runtime": session_runtime_label(native_attached),
            "preview": candidate.is_some(),
            "savedRevision": saved_revision
        }))
    }

    pub fn session_stop(&mut self, id: &EntityId) -> Result<Value, ControlError> {
        self.ensure_session_loaded(id)?;
        // Plugins lose their settings when their workers stop; keep them.
        self.capture_playing_plugin_states(id);
        let recorder_node_ids = self
            .get_session(id)?
            .nodes
            .iter()
            .filter(|node| {
                node.kind == NodeKind::Recorder && self.recorder_node_workers.contains_key(&node.id)
            })
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        for node_id in recorder_node_ids {
            self.stop_node_recording(&node_id)?;
        }
        let has_node_worker = self
            .get_session(id)?
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Recorder)
            .any(|node| self.recorder_node_workers.contains_key(&node.id));
        if has_node_worker {
            return Err(ControlError::InvalidRequest(
                "node-keyed recorder lifecycle must be finalized before stopping the session"
                    .into(),
            ));
        }
        let mut recorder_outcomes = Vec::new();
        let active_frame = self
            .recorders
            .get(id)
            .filter(|recorder| {
                matches!(
                    recorder.state(),
                    RecorderState::Recording | RecorderState::Paused | RecorderState::Stopping
                )
            })
            .map(|recorder| {
                let checkpoint = recorder.checkpoint();
                checkpoint
                    .stop_frame
                    .or(checkpoint.last_frame)
                    .ok_or_else(|| {
                        ControlError::InvalidRequest(
                            "active recorder has no committed frame boundary".into(),
                        )
                    })
            })
            .transpose()?;
        if let Some(frame) = active_frame {
            let mut recorder_candidate = self.recorders.get(id).cloned().ok_or_else(|| {
                ControlError::InvalidRequest("active recorder disappeared".into())
            })?;
            let boundary_result = if recorder_candidate.state() == RecorderState::Stopping {
                recorder_candidate.complete()
            } else {
                recorder_candidate.stop(frame)
            };
            boundary_result.map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "recorder boundary finalization failed: {error:?}"
                ))
            })?;
            let worker = self.recorder_workers.get_mut(id).ok_or_else(|| {
                ControlError::InvalidRequest(
                    "finalize the active recorder before stopping the session".into(),
                )
            })?;
            let outcome = worker.finalize(frame).map_err(|error| {
                ControlError::InvalidRequest(format!("recorder finalization failed: {error}"))
            })?;
            if outcome.state != "completed" || !outcome.file_finalized || outcome.recoverable {
                return Err(ControlError::InvalidRequest(
                    "recorder finalization did not produce a completed file".into(),
                ));
            }
            let finalized_recordings = worker.finalized_recordings();
            self.recorders.insert(id.clone(), recorder_candidate);
            let checkpoint = self
                .recorders
                .get(id)
                .expect("recorder candidate inserted above")
                .checkpoint();
            if let Some(storage) = &self.storage {
                storage
                    .save_recording_checkpoint(id.as_str(), &checkpoint)
                    .map_err(storage_error)?;
                for recording in &finalized_recordings {
                    storage.save_recording(recording).map_err(storage_error)?;
                }
            }
            self.events.append(
                self.get_session(id)?.revision,
                None,
                "recorder.changed",
                Some(id.clone()),
            );
            self.recorder_workers.remove(id);
            recorder_outcomes.push(json!({
                "sessionId": id,
                "state": "completed",
                "fileFinalized": true,
                "recoverable": false
            }));
        }

        // A session stop is also the native endpoint shutdown boundary. The
        // worker is deliberately stopped only after recorder finalization so
        // a failed recorder operation leaves the session and its endpoint
        // available for explicit recovery. Once this point is reached, stop
        // the exact bound worker before retiring the runtime generation; even
        // a worker stop error must not leave an audio client running against
        // a session that is reported stopped.
        let native_attached = self.native_endpoint_session_is_attached(id) || {
            #[cfg(windows)]
            {
                self.native_multi_input_worker_session.as_ref() == Some(id)
                    && self.native_multi_input_worker.is_some()
            }
            #[cfg(not(windows))]
            {
                false
            }
        };
        let native_stop_error = if self.native_endpoint_session_is_attached(id) {
            self.native_endpoint_worker_for_session_mut(id)
                .map(audiorouter_windows_audio::NativeAudioWorker::stop)
                .transpose()
                .err()
        } else {
            None
        };
        #[cfg(windows)]
        let native_output_stop_error = if self.native_output_fanout_session.as_ref() == Some(id) {
            self.native_output_fanout
                .as_mut()
                .map(audiorouter_windows_audio::WasapiOutputFanout::stop)
                .transpose()
                .err()
        } else {
            None
        };
        #[cfg(windows)]
        let native_duplex_stop_error = if self.native_duplex_worker_session.as_ref() == Some(id) {
            self.native_duplex_worker
                .as_mut()
                .map(audiorouter_windows_audio::NativeBridgeDuplexWorker::stop)
                .transpose()
                .err()
        } else {
            None
        };
        self.deactivate_virtual_route_bridges(id);
        let revision = self.get_session(id)?.revision;
        if let Some(runtime) = self.runtimes.get_mut(id) {
            runtime.stop();
        }
        #[cfg(windows)]
        if self
            .application_capture_runtime
            .as_ref()
            .is_some_and(|binding| &binding.session_id == id)
        {
            self.set_application_capture_state(
                "configured-stopped",
                "Prepared for this application. Start the route to capture audio.",
            );
        }
        #[cfg(windows)]
        {
            self.native_render_source_taps = None;
        }
        #[cfg(windows)]
        let native_render_source_stop_error =
            if self.native_render_source_worker_session.as_ref() == Some(id) {
                self.native_render_source_worker
                    .as_mut()
                    .map(audiorouter_windows_audio::NativeBridgeInputWorker::stop)
                    .transpose()
                    .err()
            } else {
                None
            };
        #[cfg(windows)]
        let native_multi_input_stop_error =
            if self.native_multi_input_worker_session.as_ref() == Some(id) {
                self.native_multi_input_worker
                    .as_mut()
                    .map(audiorouter_windows_audio::NativeMultiInputWorker::stop)
                    .transpose()
                    .err()
            } else {
                None
            };
        self.events
            .append(revision, None, "runtime.stopped", Some(id.clone()));
        if let Some(error) = native_stop_error {
            return Err(audio_control_error(error));
        }
        #[cfg(windows)]
        if let Some(error) = native_output_stop_error {
            return Err(ControlError::InvalidRequest(format!(
                "native output fan-out stop failed: {error:?}"
            )));
        }
        #[cfg(windows)]
        if let Some(error) = native_duplex_stop_error {
            return Err(ControlError::InvalidRequest(format!(
                "native duplex worker stop failed: {error:?}"
            )));
        }
        #[cfg(windows)]
        if let Some(error) = native_render_source_stop_error {
            return Err(ControlError::InvalidRequest(format!(
                "native render-source worker stop failed: {error:?}"
            )));
        }
        #[cfg(windows)]
        if let Some(error) = native_multi_input_stop_error {
            return Err(ControlError::InvalidRequest(format!(
                "native multi-input worker stop failed: {error:?}"
            )));
        }
        Ok(json!({
            "sessionId": id,
            "state": "stopped",
            "runtime": if native_attached { "native" } else { "fake" },
            "recorders": recorder_outcomes
        }))
    }

    pub(crate) fn ensure_session_loaded(&mut self, id: &EntityId) -> Result<(), ControlError> {
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

    pub(crate) fn dispatch_session_start(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and idempotencyKey are required".into())
        })?;
        let id = session_id_from_params(Some(params.clone()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let candidate = params
            .get("candidate")
            .cloned()
            .map(serde_json::from_value::<Session>)
            .transpose()
            .map_err(|error| {
                ControlError::InvalidRequest(format!("invalid preview candidate: {error}"))
            })?;
        let operation = (
            self.scoped_idempotency_key("sessions.start", idempotency_key),
            Self::request_hash(
                &json!({ "sessionId": id, "action": "start", "candidate": candidate }),
            ),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        let mut result = if let Some(candidate) = candidate.as_ref() {
            self.session_preview_start(&id, candidate)?
        } else {
            self.session_start(&id)?
        };
        if candidate.is_none() {
            self.start_auto_recordings(&id, &mut result)?;
        }
        self.journal_idempotent_result(&operation.0, "sessions.start", &operation.1, &result)?;
        Ok(result)
    }

    pub(crate) fn dispatch_session_get(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let id = session_id_from_params(params)?;
        self.ensure_session_loaded(&id)?;
        serde_json::to_value(self.get_session(&id)?)
            .map_err(|error| ControlError::Json(error.to_string()))
    }

    pub(crate) fn dispatch_session_export(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let id = session_id_from_params(params)?;
        self.ensure_session_loaded(&id)?;
        serde_json::to_value(self.get_session(&id)?)
            .map_err(|error| ControlError::Json(error.to_string()))
    }

    /// Write the saved session, with the imported audio and plugin states it
    /// references, to one `.audiorouter` file (never overwriting).
    pub(crate) fn dispatch_session_export_file(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let id = session_id_from_params(params.clone())?;
        let path = session_file_path(params.as_ref())?;
        self.ensure_session_loaded(&id)?;
        let revision = self.get_session(&id)?.revision;
        let replace = params
            .as_ref()
            .and_then(|params| params.get("replace"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let existing = std::fs::symlink_metadata(&path).ok();
        if let Some(metadata) = &existing {
            if !replace {
                return Err(ControlError::InvalidRequest(
                    "a file with that name already exists; choose another name".into(),
                ));
            }
            if !metadata.is_file() {
                return Err(ControlError::InvalidRequest(
                    "only an existing regular .audiorouter file can be replaced".into(),
                ));
            }
        }
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("session storage is unavailable".into()))?;
        // Database reads stay here; reading plugin state files, hashing and
        // writing the ZIP run while running routes keep being serviced.
        let prepared = storage.prepare_bundle_export(&id).map_err(storage_error)?;
        let replacing = existing.is_some();
        let bytes = self.while_servicing_audio(|| -> Result<u64, ControlError> {
            if replacing {
                // Write beside the file first so a failed export keeps the old one.
                let staged = path.with_extension(format!("audiorouter.{}.tmp", std::process::id()));
                let _ = std::fs::remove_file(&staged);
                prepared.write(&staged).map_err(storage_error)?;
                std::fs::rename(&staged, &path).map_err(|error| {
                    let _ = std::fs::remove_file(&staged);
                    ControlError::InvalidRequest(format!(
                        "unable to replace the session file: {error}"
                    ))
                })?;
            } else {
                prepared.write(&path).map_err(storage_error)?;
            }
            Ok(std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0))
        })?;
        Ok(
            json!({ "sessionId": id, "path": path.to_string_lossy(), "revision": revision, "bytes": bytes }),
        )
    }

    /// Read a `.audiorouter` file into a new stopped session. A session ID
    /// already used here gets a fresh ID and an "(imported)" name, so an
    /// import never replaces an existing session.
    pub(crate) fn dispatch_session_import_file(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let path = session_file_path(params.as_ref())?;
        if !path.is_file() {
            return Err(ControlError::InvalidRequest(
                "the session file does not exist".into(),
            ));
        }
        if self.storage.is_none() {
            return Err(ControlError::InvalidRequest(
                "session storage is unavailable".into(),
            ));
        }
        let staging = std::env::temp_dir().join("audiorouter-session-import");
        // Unpacking, hash checks and asset reads run while running routes
        // keep being serviced; the database step stays on this thread.
        let staged = self
            .while_servicing_audio(|| Storage::stage_session_bundle(&path, &staging))
            .map_err(storage_error)?;
        let (mut session, report) = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("session storage is unavailable".into()))?
            .restore_session_bundle(staged)
            .map_err(storage_error)?;
        let mut renamed = false;
        let taken = |plane: &mut Self, id: &EntityId| {
            let _ = plane.ensure_session_loaded(id);
            plane.store.session(id).is_some()
        };
        if taken(self, &session.id) {
            renamed = true;
            let base: String = session.id.as_str().chars().take(40).collect();
            let mut suffix = 1u32;
            let fresh = loop {
                let candidate = EntityId::new(format!("{base}-imported-{suffix}"));
                if !taken(self, &candidate) {
                    break candidate;
                }
                suffix += 1;
                if suffix > 999 {
                    return Err(ControlError::InvalidRequest(
                        "too many imported copies of this session".into(),
                    ));
                }
            };
            session.id = fresh;
            session.name = format!("{} (imported)", session.name);
        }
        session.revision = 0;
        let mut result = self.create_session(session)?;
        result["renamed"] = json!(renamed);
        result["mediaRestored"] = json!(report.media_restored);
        result["pluginStatesRestored"] = json!(report.plugin_states_restored);
        result["missingAssets"] = json!(report.missing_assets);
        Ok(result)
    }

    pub(crate) fn dispatch_session_import_plan(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("session is required".into()))?;
        let session: Session = serde_json::from_value(
            params
                .get("session")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("session is required".into()))?,
        )
        .map_err(|error| ControlError::InvalidRequest(error.to_string()))?;
        validate_session(&session).map_err(|errors| {
            ControlError::InvalidRequest(format!(
                "invalid session import: {}",
                format_validation_errors(&errors)
            ))
        })?;
        self.validate_plugin_placeholders(&session)?;
        if self.store.session(&session.id).is_some() {
            return Err(ControlError::InvalidRequest(
                "session already exists".into(),
            ));
        }
        let now = Instant::now();
        self.session_import_plans
            .retain(|_, (_, expires_at)| *expires_at > now);
        if self.session_import_plans.len() >= MAX_PENDING_PLAN_RECORDS {
            return Err(ControlError::InvalidRequest(
                "too many pending session import plans".into(),
            ));
        }
        let plan_id = allocate_counter_plan_id(
            "session-import",
            &mut self.next_session_import_plan,
            |id| self.session_import_plans.contains_key(id),
            "session import plan ID space is exhausted",
        )?;
        self.session_import_plans.insert(
            plan_id.clone(),
            (session.clone(), Instant::now() + VIRTUAL_DEVICE_PLAN_TTL),
        );
        Ok(json!({ "planId": plan_id, "expiresInMs": 300000, "session": session }))
    }

    pub(crate) fn dispatch_session_import_commit(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("planId and idempotencyKey are required".into())
        })?;
        let plan_id = params
            .get("planId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?;
        let key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|key| !key.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let plan_entity_id = EntityId::new(plan_id);
        let scoped_key = self.scoped_idempotency_key("sessions.importCommit", key);
        let hash = Self::request_hash(&json!({ "planId": plan_id }));
        if let Some(result) = self.lookup_idempotent_result(&scoped_key, &hash)? {
            return Ok(result);
        }
        let Some((session, expires_at)) = self.session_import_plans.get(&plan_entity_id).cloned()
        else {
            return Err(ControlError::InvalidRequest("import plan not found".into()));
        };
        if expires_at <= Instant::now() {
            self.session_import_plans.remove(&plan_entity_id);
            return Err(ControlError::InvalidRequest("import plan expired".into()));
        }
        if self.store.session(&session.id).is_some() {
            return Err(ControlError::InvalidRequest(
                "session already exists".into(),
            ));
        }
        self.insert_session(session.clone())?;
        self.session_import_plans.remove(&plan_entity_id);
        let result = json!({ "session": session, "state": "stopped", "imported": true });
        self.journal_idempotent_result(&scoped_key, "sessions.importCommit", &hash, &result)?;
        Ok(result)
    }

    pub(crate) fn dispatch_session_create(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("session is required".into()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let session: Session = serde_json::from_value(
            params
                .get("session")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("session is required".into()))?,
        )
        .map_err(|error| ControlError::InvalidRequest(error.to_string()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "session": session });
                (
                    self.scoped_idempotency_key("sessions.create", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let result = self.create_session(session)?;
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "sessions.create", &hash, &result)?;
        }
        Ok(result)
    }

    pub(crate) fn dispatch_session_duplicate(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("duplicate parameters are required".into())
        })?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
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
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({
                    "sourceSessionId": source_id,
                    "sessionId": duplicate_id,
                    "name": name
                });
                (
                    self.scoped_idempotency_key("sessions.duplicate", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let result = self.duplicate_session(&source_id, duplicate_id, name)?;
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "sessions.duplicate", &hash, &result)?;
        }
        Ok(result)
    }

    pub(crate) fn dispatch_session_delete(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and idempotencyKey are required".into())
        })?;
        let id = session_id_from_params(Some(params.clone()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "sessionId": id });
                (
                    self.scoped_idempotency_key("sessions.delete", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let result = self.delete_session(&id)?;
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "sessions.delete", &hash, &result)?;
        }
        Ok(result)
    }

    pub(crate) fn dispatch_sessions_list(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
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

    pub(crate) fn dispatch_active_session_get(&self) -> Value {
        let session_id = self
            .active_session_id
            .as_ref()
            .filter(|id| self.store.session(id).is_some())
            .map(|id| id.as_str());
        json!({ "sessionId": session_id })
    }

    pub(crate) fn dispatch_active_session_set(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let session_id: EntityId =
            serde_json::from_value(params.get("sessionId").cloned().unwrap_or(Value::Null))
                .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let request = json!({ "sessionId": session_id });
        let key = self.scoped_idempotency_key("sessions.active.set", key);
        let hash = Self::request_hash(&request);
        if let Some(previous) = self.lookup_idempotent_result(&key, &hash)? {
            return Ok(previous);
        }
        let session = self
            .store
            .session(&session_id)
            .ok_or_else(|| ControlError::from(audiorouter_domain::StoreError::SessionNotFound))?;
        if self.active_session_id.as_ref() != Some(&session_id) {
            if let Some(storage) = self.storage.as_ref() {
                storage
                    .save_active_session_id(&session_id)
                    .map_err(storage_error)?;
            }
            self.active_session_id = Some(session_id.clone());
            self.events
                .append(session.revision, None, "session.selectionChanged", None);
        }
        let result = json!({ "sessionId": session_id.as_str() });
        self.journal_idempotent_result(&key, "sessions.active.set", &hash, &result)?;
        Ok(result)
    }

    pub(crate) fn dispatch_session_stop(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and idempotencyKey are required".into())
        })?;
        let id = session_id_from_params(Some(params.clone()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "sessionId": id, "action": "stop" });
                (
                    self.scoped_idempotency_key("sessions.stop", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let result = self.session_stop(&id)?;
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "sessions.stop", &hash, &result)?;
        }
        Ok(result)
    }
}
