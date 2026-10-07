//! Runtime status, audio status, generations, event subscriptions and verbose diagnostics.

use super::*;

impl ControlPlane {
    pub(crate) fn status_snapshot(&mut self) -> Result<Value, ControlError> {
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
        let (recent_recovery_crashes, recovery_safe_mode) = self.recovery_status()?;
        let (audio, reason) = self.audio_status();
        Ok(json!({
            "build": self.build,
            "audio": audio,
            "deviceDiscovery": "available",
            "reason": reason,
            "storage": if self.storage.is_some() { "sqlite" } else { "memory" },
            "sessionCount": session_count,
            "activeSessionCount": active_session_ids.len(),
            "activeSessionIds": active_session_ids,
            "privacyMute": {
                "muted": self.privacy_muted,
                "persistence": if self.storage.is_some() { "durable" } else { "memory" },
                "audioEffect": "process-local-when-realtime-backend-is-available"
            },
            "recovery": {
                "safeMode": recovery_safe_mode,
                "recentCrashes": recent_recovery_crashes,
                "persistence": if self.storage.is_some() { "durable" } else { "memory" }
            },
            "eventCursor": {
                "backendEpoch": self.events.backend_epoch(),
                "latestSequence": self.events.latest_sequence()
            }
        }))
    }

    pub(crate) fn native_adapter_state(&self) -> &'static str {
        if self
            .native_endpoint_worker
            .as_ref()
            .is_some_and(audiorouter_windows_audio::NativeAudioWorker::is_running)
            || self
                .native_endpoint_worker_secondary
                .as_ref()
                .is_some_and(audiorouter_windows_audio::NativeAudioWorker::is_running)
        {
            return "running";
        }
        #[cfg(windows)]
        if self
            .native_duplex_worker
            .as_ref()
            .is_some_and(|worker| worker.is_running())
        {
            return "running";
        }
        #[cfg(windows)]
        if self
            .native_render_source_worker
            .as_ref()
            .is_some_and(|worker| worker.is_running())
        {
            return "running";
        }
        #[cfg(windows)]
        if self
            .native_multi_input_worker
            .as_ref()
            .is_some_and(|worker| worker.is_running())
        {
            return "running";
        }
        if self.native_endpoint_worker.is_some() || self.native_endpoint_worker_secondary.is_some()
        {
            return "configured-stopped";
        }
        #[cfg(windows)]
        if self.native_duplex_worker.is_some() {
            return "configured-stopped";
        }
        #[cfg(windows)]
        if self.native_render_source_worker.is_some() {
            return "configured-stopped";
        }
        #[cfg(windows)]
        if self.native_multi_input_worker.is_some() {
            return "configured-stopped";
        }
        "implemented-not-activated"
    }

    pub(crate) fn native_session_id(&self) -> Option<&EntityId> {
        if self.native_endpoint_worker.is_some() || self.native_endpoint_worker_secondary.is_some()
        {
            return self
                .native_endpoint_session
                .as_ref()
                .or(self.native_endpoint_session_secondary.as_ref());
        }
        #[cfg(windows)]
        if self.native_duplex_worker.is_some() {
            return self.native_duplex_worker_session.as_ref();
        }
        #[cfg(windows)]
        if self.native_render_source_worker.is_some() {
            return self.native_render_source_worker_session.as_ref();
        }
        #[cfg(windows)]
        if self.native_multi_input_worker.is_some() {
            return self.native_multi_input_worker_session.as_ref();
        }
        None
    }

    pub(crate) fn republish_running_native_graph(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        flags_only: bool,
    ) -> Result<Option<&'static str>, ControlError> {
        if self.native_endpoint_session_is_attached(session_id) {
            let sample_rate_hz = self
                .native_endpoint_worker_for_session(session_id)
                .expect("attached above")
                .bridge()
                .sample_rate_hz();
            self.activate_native_graph_candidate_with_flags(
                session_id,
                generation,
                sample_rate_hz,
                None,
                flags_only,
            )?;
            return Ok(Some(self.native_adapter_kind().unwrap_or("endpoint")));
        }
        #[cfg(windows)]
        if self.native_multi_input_worker_session.as_ref() == Some(session_id)
            && self.native_multi_input_worker.is_some()
        {
            // The surround renderer is chosen when the capture opens.
            let worker = self
                .native_multi_input_worker
                .as_ref()
                .expect("attached above");
            let current = self.get_session(session_id)?;
            for (index, node_id) in worker.input_node_ids().iter().enumerate() {
                if current.nodes.iter().any(|node| {
                    node.id == *node_id
                        && node.kind == NodeKind::PhysicalInput
                        && node_spatial_options(node) != worker.capture_spatial_options(index)
                }) {
                    return Err(ControlError::InvalidRequest(
                        "surround settings changed; stop and press Play to apply".into(),
                    ));
                }
            }
            let mut session = if flags_only {
                self.adapt_native_paths_session(self.get_session(session_id)?.clone())?
            } else {
                self.native_paths_session(session_id)?
            };
            let mut bridge_flags = Vec::new();
            let plugin_stages = if flags_only {
                let mut stages: HashMap<EntityId, Arc<dyn RealtimePluginProcessor>> =
                    HashMap::new();
                let worker = self
                    .native_multi_input_worker
                    .as_ref()
                    .expect("attached above");
                normalize_live_path_flags(
                    &mut session,
                    worker.input_node_ids(),
                    worker.output_node_ids(),
                );
                session = audiorouter_engine::prune_inactive_upstream(&session).into_owned();
                for node in session
                    .nodes
                    .iter_mut()
                    .filter(|node| node.kind == NodeKind::Plugin)
                {
                    let bridge = match self.plugin_bridge(session_id, &node.id) {
                        Ok(bridge) => bridge,
                        Err(_) if !node.enabled => continue,
                        Err(error) => return Err(error),
                    };
                    bridge_flags.push((Arc::clone(&bridge), node.enabled && !node.bypass));
                    stages.insert(node.id.clone(), bridge);
                    node.enabled = true;
                    node.bypass = false;
                }
                stages
            } else {
                self.prepare_plugin_stages(&session, audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ)?
            };
            let worker_generation = self
                .native_multi_input_worker_generation
                .unwrap_or(generation);
            let media =
                self.session_audio_media(&session, audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ)?;
            let compiled = audiorouter_engine::compile_native_paths_with_plugins_and_audio(
                &session,
                RuntimeGeneration::new(worker_generation),
                &plugin_stages,
                &media,
            )
            .map_err(|error| {
                ControlError::InvalidRequest(format!("multi-input graph rejected: {error:?}"))
            })?;
            self.native_multi_input_worker
                .as_mut()
                .expect("attached above")
                .replace_path_set(compiled)
                .map_err(|_| {
                    ControlError::InvalidRequest(
                        "the route's sources or outputs changed; stop and press Play to apply"
                            .into(),
                    )
                })?;
            for (bridge, active) in bridge_flags {
                bridge.set_processing_active(active);
            }
            // The commit started a new runtime generation; the worker keeps
            // playing its prepared one and now serves both.
            self.native_multi_input_applied_generation = Some(generation);
            self.register_multi_input_generators(session_id);
            return Ok(Some("multi-input"));
        }
        Ok(None)
    }

    /// A saved change the playing multi-path route could not absorb (new
    /// sources, outputs or path layout). The commit already started a new
    /// runtime generation, which the old worker does not serve: left alone,
    /// the service stops pumping it and the route falls silent. Restart it
    /// with the saved graph (Stop, detach, prepare, Play), as the desktop
    /// Play does. While a recording runs, a restart would split the take, so
    /// the old route keeps playing instead and Stop/Play stays the user's
    /// choice. `None` when no multi-path worker plays this session.
    #[cfg(windows)]
    pub(crate) fn restart_or_keep_multi_input_route(
        &mut self,
        session_id: &EntityId,
        generation: u64,
    ) -> Option<Result<u64, ControlError>> {
        if self.native_multi_input_worker_session.as_ref() != Some(session_id)
            || self.native_multi_input_worker.is_none()
        {
            return None;
        }
        if self.session_is_recording(session_id)
            || self.active_device_restart_allowed == Some(false)
        {
            self.native_multi_input_applied_generation = Some(generation);
            return None;
        }
        Some((|| {
            self.session_stop(session_id)?;
            self.detach_native_multi_input_worker()?;
            self.dispatch_native_paths_prepare(Some(json!({ "sessionId": session_id.as_str() })))?;
            let started = self.session_start(session_id)?;
            Ok(started
                .get("generation")
                .and_then(Value::as_u64)
                .unwrap_or(generation))
        })())
    }

    #[cfg(not(windows))]
    pub(crate) fn restart_or_keep_multi_input_route(
        &mut self,
        _session_id: &EntityId,
        _generation: u64,
    ) -> Option<Result<u64, ControlError>> {
        None
    }

    /// Connect the Stats.cc feed only while a playing session has a Duck
    /// following the Siege round.
    pub(crate) fn refresh_siege_round_feed(&mut self) {
        let wanted = self.runtimes.iter().any(|(session_id, runtime)| {
            runtime.state() == RuntimeState::Running
                && self
                    .store
                    .session(session_id)
                    .is_some_and(audiorouter_engine::session_follows_game_round)
        });
        self.siege_round_feed.set_wanted(wanted);
    }

    /// Expose Play/Stop for Test Signal and Audio File nodes that feed the
    /// multi-input Mixer, exactly as the single-route compiler does.
    #[cfg(windows)]
    pub(crate) fn register_multi_input_generators(&mut self, session_id: &EntityId) {
        let Some(session) = self.store.session(session_id).cloned() else {
            return;
        };
        let Some(worker) = self.native_multi_input_worker.as_ref() else {
            return;
        };
        let mut test_signals = Vec::new();
        let mut audio_files = Vec::new();
        for node in &session.nodes {
            if node.kind == NodeKind::TestSignal {
                if let Some(source) = worker.test_signal_source_for_node(&node.id) {
                    test_signals.push((node.id.clone(), source));
                }
            } else if node.kind == NodeKind::AudioFile {
                if let Some(source) = worker.audio_file_source_for_node(&node.id) {
                    audio_files.push((node.id.clone(), source));
                }
            }
        }
        self.test_signal_sources
            .retain(|(owner, _), _| owner != session_id);
        self.audio_file_sources
            .retain(|(owner, _), _| owner != session_id);
        for (node_id, source) in test_signals {
            self.test_signal_sources
                .insert((session_id.clone(), node_id), source);
        }
        for (node_id, source) in audio_files {
            self.audio_file_sources
                .insert((session_id.clone(), node_id), source);
        }
    }

    /// The generation a native preparation targets: the explicit
    /// `generation` parameter when present, otherwise the generation the
    /// session's next `session.start` will assign. A running session has no
    /// "next" generation for a new worker and must be stopped first.
    pub(crate) fn requested_or_next_generation(
        &self,
        params: &Value,
        session_id: &EntityId,
    ) -> Result<u64, ControlError> {
        match params.get("generation") {
            Some(value) => value.as_u64().filter(|value| *value > 0).ok_or_else(|| {
                ControlError::InvalidRequest("generation must be a positive integer".into())
            }),
            None => match self.runtimes.get(session_id) {
                Some(runtime) if runtime.state() == RuntimeState::Running => {
                    Err(ControlError::InvalidRequest(
                        "stop the session before preparing native audio for its next start".into(),
                    ))
                }
                Some(runtime) => Ok(runtime.generation().saturating_add(1)),
                None => Ok(1),
            },
        }
    }

    pub(crate) fn native_adapter_kind(&self) -> Option<&'static str> {
        if self
            .application_capture_runtime
            .as_ref()
            .is_some_and(|binding| self.native_endpoint_session_is_attached(&binding.session_id))
        {
            return Some("process-loopback");
        }
        if self.native_endpoint_worker.is_some() || self.native_endpoint_worker_secondary.is_some()
        {
            return Some("endpoint");
        }
        #[cfg(windows)]
        if self.native_duplex_worker.is_some() {
            return Some("duplex");
        }
        #[cfg(windows)]
        if self.native_render_source_worker.is_some() {
            return Some("render-source");
        }
        #[cfg(windows)]
        if self.native_multi_input_worker.is_some() {
            return Some("multi-input");
        }
        None
    }

    pub(crate) fn audio_status(&self) -> (&'static str, &'static str) {
        Self::audio_status_for(self.native_adapter_state(), self.native_adapter_kind())
    }

    pub(crate) fn audio_status_for(
        state: &'static str,
        kind: Option<&'static str>,
    ) -> (&'static str, &'static str) {
        match state {
            "running" => (
                "available",
                match kind {
                    Some("process-loopback") => {
                        "verified application audio is connected to the selected render endpoint"
                    }
                    Some("duplex") => {
                        "native duplex audio is running; managed driver qualification remains open"
                    }
                    Some("render-source") => {
                        "native render-source audio is running; managed driver qualification remains open"
                    }
                    Some("multi-input") => {
                        "native multi-input audio is running"
                    }
                    _ => "native endpoint audio is running",
                },
            ),
            "configured-stopped" => (
                "unavailable",
                match kind {
                    Some("process-loopback") => {
                        "application capture is prepared but stopped; the canvas shows application exit and reconnection status"
                    }
                    Some("duplex") => {
                        "native duplex worker is prepared but stopped; start a session explicitly"
                    }
                    Some("render-source") => {
                        "native render-source worker is prepared but stopped; start a session explicitly"
                    }
                    Some("multi-input") => {
                        "native multi-input worker is prepared but stopped; start a session explicitly"
                    }
                    _ => "native endpoint worker is prepared but stopped; start a session explicitly",
                },
            ),
            _ => (
                "unavailable",
                match kind {
                    Some("duplex") => {
                        "native duplex audio is not prepared; configure exact endpoints and the managed virtual bridge before starting"
                    }
                    Some("render-source") => {
                        "native render-source audio is not prepared; configure exact endpoints and the managed virtual bridge before starting"
                    }
                    Some("multi-input") => {
                        "multi-input audio is not prepared; select exact sources and outputs in Devices, prepare them, then Start session"
                    }
                    _ => "audio is not prepared; in Devices, select exact capture and render endpoints, then Prepare native endpoints and Start session",
                },
            ),
        }
    }

    pub(crate) fn dispatch_events_subscribe(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        if let Some(requested_epoch) = params.get("backendEpoch").and_then(Value::as_u64) {
            if requested_epoch != self.events.backend_epoch() {
                return Ok(json!({
                    "backendEpoch": self.events.backend_epoch(),
                    "resyncRequired": true,
                    "reason": "backendEpochChanged",
                    "snapshot": { "sessions": self.sessions_list_page(None, 500)? },
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
        if !(1..=MAX_EVENT_SUBSCRIPTION_ITEMS as u64).contains(&limit) {
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
        let category_filter = params
            .get("categories")
            .map(|value| {
                let categories = value.as_array().ok_or_else(|| {
                    ControlError::InvalidRequest("categories must be an array".into())
                })?;
                if categories.is_empty() || categories.len() > 32 {
                    return Err(ControlError::InvalidRequest(
                        "categories must contain between 1 and 32 items".into(),
                    ));
                }
                categories
                    .iter()
                    .map(|category| {
                        let category = category.as_str().ok_or_else(|| {
                            ControlError::InvalidRequest("event categories must be strings".into())
                        })?;
                        if category.is_empty() || category.chars().count() > 128 {
                            return Err(ControlError::InvalidRequest(
                                "event category must contain 1 to 128 characters".into(),
                            ));
                        }
                        if !STATE_CATEGORIES.contains(&category) {
                            return Err(ControlError::InvalidRequest(
                                "event category is not discoverable".into(),
                            ));
                        }
                        Ok(category.to_owned())
                    })
                    .collect::<Result<Vec<_>, ControlError>>()
            })
            .transpose()?;
        let (events, next_sequence) = match self.events.since_page(after_sequence, limit as usize) {
            Ok(page) => page,
            Err(EventReplayError::InvalidLimit) => {
                return Err(ControlError::InvalidRequest(
                    "limit must be between 1 and 500".into(),
                ));
            }
            Err(EventReplayError::ResyncRequired) => {
                return Ok(json!({
                    "backendEpoch": self.events.backend_epoch(),
                    "resyncRequired": true,
                    "snapshot": { "sessions": self.sessions_list_page(None, 500)? },
                    "events": [],
                    "nextSequence": self.events.latest_sequence()
                }));
            }
        };
        let events = events
            .into_iter()
            .filter(|event| {
                let session_matches = session_filter
                    .as_ref()
                    .map(|id| event.session_id.is_none() || event.session_id.as_ref() == Some(id))
                    .unwrap_or(true);
                let category_matches = category_filter
                    .as_ref()
                    .map(|categories| {
                        categories
                            .iter()
                            .any(|category| category == &event.category)
                    })
                    .unwrap_or(true);
                session_matches && category_matches
            })
            .collect::<Vec<_>>();
        Ok(json!({
            "backendEpoch": self.events.backend_epoch(),
            "events": events,
            "nextSequence": next_sequence,
        }))
    }
}

#[cfg(test)]
mod next_generation_tests {
    use super::*;

    fn session() -> Session {
        Session {
            id: EntityId::new("next-generation"),
            name: "next generation".into(),
            schema_version: 1,
            revision: 0,
            nodes: vec![],
            edges: vec![],
        }
    }

    #[test]
    fn native_preparation_defaults_to_the_next_start_generation() {
        let mut plane = ControlPlane::new("next-generation");
        let id = session().id;
        plane.insert_session(session()).unwrap();
        assert_eq!(
            plane.requested_or_next_generation(&json!({}), &id).unwrap(),
            1
        );
        assert_eq!(
            plane
                .requested_or_next_generation(&json!({ "generation": 5 }), &id)
                .unwrap(),
            5
        );
        assert!(plane
            .requested_or_next_generation(&json!({ "generation": 0 }), &id)
            .is_err());
        plane.session_start(&id).unwrap();
        // A running session has no next generation for a new worker.
        assert!(plane.requested_or_next_generation(&json!({}), &id).is_err());
        plane.session_stop(&id).unwrap();
        assert_eq!(
            plane.requested_or_next_generation(&json!({}), &id).unwrap(),
            2
        );
    }
}
