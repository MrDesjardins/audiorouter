//! Native path workers: lookup, attachment and preparation of endpoint and multi-input workers.

use super::*;

/// One mixer source binding for `prepare_native_multi_input_worker`. Order
/// must match the compiler's retained source-node order (`input_node_ids`),
/// the same convention already used for the single-kind capture list. An
/// application source re-supplies its full observed identity explicitly,
/// matching `dispatch_native_applications_prepare`'s revalidation contract,
/// rather than silently trusting the node's persisted parameters.
#[derive(Clone, Copy)]
pub enum NativeMultiInputSourceBinding<'a> {
    Physical(&'a audiorouter_windows_audio::EndpointInfo),
    Application {
        process_id: u32,
        expected_executable: &'a str,
        expected_executable_path: Option<&'a str>,
        expected_creation_time_100ns: u64,
        mode: audiorouter_windows_audio::ProcessLoopbackMode,
    },
    /// A Test Signal or Audio File: the Mixer input chain generates the
    /// audio, and the native input only supplies silent pacing packets.
    Generated,
    /// A Network Receive node: its validated sender/port/buffer parameters.
    Network(&'a audiorouter_domain::Node),
}

/// How `prepare_native_path_worker` receives source bindings: in the
/// compiled input order, or keyed by source node.
#[cfg(windows)]
pub(crate) enum MultiInputBindings<'s, 'a> {
    Ordered(&'s [NativeMultiInputSourceBinding<'a>]),
    ByNode(&'s HashMap<EntityId, NativeMultiInputSourceBinding<'a>>),
}

impl ControlPlane {
    pub(crate) fn any_native_worker_attached(&self) -> bool {
        self.native_endpoint_worker.is_some()
            || self.native_endpoint_worker_secondary.is_some()
            || {
                #[cfg(windows)]
                {
                    self.native_duplex_worker.is_some() || self.native_multi_input_worker.is_some()
                }
                #[cfg(not(windows))]
                {
                    false
                }
            }
    }

    pub(crate) fn native_endpoint_worker_for_session(
        &self,
        session_id: &EntityId,
    ) -> Option<&audiorouter_windows_audio::NativeAudioWorker> {
        if self.native_endpoint_session.as_ref() == Some(session_id) {
            self.native_endpoint_worker.as_ref()
        } else if self.native_endpoint_session_secondary.as_ref() == Some(session_id) {
            self.native_endpoint_worker_secondary.as_ref()
        } else {
            None
        }
    }

    pub(crate) fn native_endpoint_worker_for_session_mut(
        &mut self,
        session_id: &EntityId,
    ) -> Option<&mut audiorouter_windows_audio::NativeAudioWorker> {
        if self.native_endpoint_session.as_ref() == Some(session_id) {
            self.native_endpoint_worker.as_mut()
        } else if self.native_endpoint_session_secondary.as_ref() == Some(session_id) {
            self.native_endpoint_worker_secondary.as_mut()
        } else {
            None
        }
    }

    pub(crate) fn native_endpoint_session_is_attached(&self, session_id: &EntityId) -> bool {
        self.native_endpoint_worker_for_session(session_id)
            .is_some()
    }

    pub(crate) fn native_endpoint_worker_slot_for_session_mut(
        &mut self,
        session_id: &EntityId,
    ) -> Option<&mut Option<audiorouter_windows_audio::NativeAudioWorker>> {
        if self.native_endpoint_session.as_ref() == Some(session_id) {
            Some(&mut self.native_endpoint_worker)
        } else if self.native_endpoint_session_secondary.as_ref() == Some(session_id) {
            Some(&mut self.native_endpoint_worker_secondary)
        } else {
            None
        }
    }

    pub(crate) fn native_endpoint_taps_for_session_mut(
        &mut self,
        session_id: &EntityId,
    ) -> Option<&mut Option<AudioTapSet>> {
        if self.native_endpoint_session.as_ref() == Some(session_id) {
            Some(&mut self.native_endpoint_taps)
        } else if self.native_endpoint_session_secondary.as_ref() == Some(session_id) {
            Some(&mut self.native_endpoint_taps_secondary)
        } else {
            None
        }
    }

    pub(crate) fn native_endpoint_has_capacity(&self) -> bool {
        (self.native_endpoint_worker.is_none() || self.native_endpoint_worker_secondary.is_none())
            && {
                #[cfg(windows)]
                {
                    self.native_multi_input_worker.is_none() && self.native_duplex_worker.is_none()
                }
                #[cfg(not(windows))]
                {
                    true
                }
            }
    }

    pub(crate) fn native_worker_attached_to(&self, session_id: &EntityId) -> bool {
        self.native_endpoint_session.as_ref() == Some(session_id)
            || self.native_endpoint_session_secondary.as_ref() == Some(session_id)
            || {
                #[cfg(windows)]
                {
                    self.native_duplex_worker_session.as_ref() == Some(session_id)
                        || self.native_multi_input_worker_session.as_ref() == Some(session_id)
                        || self.native_render_source_worker_session.as_ref() == Some(session_id)
                }
                #[cfg(not(windows))]
                {
                    false
                }
            }
    }

    /// Attach an already-opened, exact-binding endpoint worker to one known
    /// session. Opening endpoints is outside this method and the worker stays
    /// stopped until an explicit start call is made.
    pub fn attach_native_endpoint_worker(
        &mut self,
        session_id: EntityId,
        worker: audiorouter_windows_audio::WasapiEndpointWorker,
    ) -> Result<(), ControlError> {
        self.get_session(&session_id)?;
        if self.native_endpoint_session_is_attached(&session_id)
            || !self.native_endpoint_has_capacity()
        {
            return Err(ControlError::InvalidRequest(
                "native endpoint workers are at capacity or the session is already attached".into(),
            ));
        }
        if worker.is_running() {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker must be stopped before attachment".into(),
            ));
        }
        worker.set_privacy_muted(self.privacy_muted);
        let worker = audiorouter_windows_audio::NativeAudioWorker::Endpoint(worker);
        if self.native_endpoint_worker.is_none() {
            self.native_endpoint_worker = Some(worker);
            self.native_endpoint_session = Some(session_id);
        } else {
            self.native_endpoint_worker_secondary = Some(worker);
            self.native_endpoint_session_secondary = Some(session_id);
        }
        Ok(())
    }

    #[cfg(windows)]
    /// Attach a stopped multi-capture worker to one known session. Its
    /// prepared generation must match the session generation; start and pump
    /// remain explicit control operations.
    pub fn attach_native_multi_input_worker(
        &mut self,
        session_id: EntityId,
        generation: u64,
        worker: audiorouter_windows_audio::NativeMultiInputWorker,
    ) -> Result<(), ControlError> {
        self.get_session(&session_id)?;
        if self.any_native_worker_attached() {
            return Err(ControlError::InvalidRequest(
                "native worker is already attached".into(),
            ));
        }
        if generation == 0 || worker.generation() != generation || worker.is_running() {
            return Err(ControlError::InvalidRequest(
                "native multi-input worker must be stopped and match a nonzero generation".into(),
            ));
        }
        worker.set_privacy_muted(self.privacy_muted);
        self.native_multi_input_worker = Some(worker);
        self.native_multi_input_worker_session = Some(session_id);
        self.native_multi_input_worker_generation = Some(generation);
        self.native_multi_input_applied_generation = None;
        Ok(())
    }

    #[cfg(windows)]
    /// Prepare a stopped multi-capture worker from exact source bindings and
    /// the session's validated mixer/fan-out topology. Binding order must
    /// match the compiler's retained source-node order; no endpoint or
    /// process is selected by label, default role, or fallback. A physical
    /// binding must match an enabled physical-input node; an application
    /// binding must match an enabled application-capture node's currently
    /// persisted identity, re-verified the same way a single-source
    /// application worker is.
    pub fn prepare_native_multi_input_worker(
        &mut self,
        session_id: EntityId,
        generation: u64,
        bindings: &[NativeMultiInputSourceBinding<'_>],
        buffer_duration_100ns: i64,
        max_attempts: u32,
        retry_delay_ms: u64,
    ) -> Result<Value, ControlError> {
        if generation == 0
            || bindings.len() < 2
            || bindings.len() > audiorouter_engine::MAX_MIXER_INPUTS
        {
            return Err(ControlError::InvalidRequest(
                "multi-input preparation requires 2..8 source bindings and a nonzero generation"
                    .into(),
            ));
        }
        self.native_multi_input_mono_nodes.clear();
        self.prepare_native_path_worker(
            session_id,
            generation,
            MultiInputBindings::Ordered(bindings),
            buffer_duration_100ns,
            max_attempts,
            retry_delay_ms,
        )
    }

    /// The session copy the multi-input worker compiles: disabled sources
    /// that feed nothing are dropped (only bound sources are opened), and a
    /// stereo device node bound to a mono endpoint reads that endpoint
    /// through `[1,1]` duplication folded into its outgoing matrices. The
    /// saved session is never changed.
    #[cfg(windows)]
    pub(crate) fn native_paths_session(
        &self,
        session_id: &EntityId,
    ) -> Result<Session, ControlError> {
        self.validate_endpoint_feedback(self.get_session(session_id)?)?;
        self.adapt_native_paths_session(
            audiorouter_engine::prune_inactive_upstream(self.get_session(session_id)?).into_owned(),
        )
    }

    #[cfg(windows)]
    pub(crate) fn adapt_native_paths_session(
        &self,
        mut session: Session,
    ) -> Result<Session, ControlError> {
        for node_id in &self.native_multi_input_mono_nodes {
            let Some(node) = session.nodes.iter_mut().find(|node| node.id == *node_id) else {
                continue;
            };
            let Some(port) = node
                .ports
                .iter_mut()
                .find(|port| port.direction == PortDirection::Output && port.channels == 2)
            else {
                continue;
            };
            port.channels = 1;
            let port_name = port.name.clone();
            for edge in session
                .edges
                .iter_mut()
                .filter(|edge| edge.source_node == *node_id && edge.source_port == port_name)
            {
                edge.matrix = edge.matrix.chunks(2).map(|row| row.iter().sum()).collect();
            }
        }
        Ok(session)
    }

    #[cfg(windows)]
    /// Prepare the stopped multi-input worker for every independent path of
    /// the session (GRAPH-15): one Mixer path through `nativeMultiInputs`, or
    /// any number of paths through `nativePaths`. All paths share one
    /// generation and lifecycle.
    pub(crate) fn prepare_native_path_worker(
        &mut self,
        session_id: EntityId,
        generation: u64,
        bindings: MultiInputBindings<'_, '_>,
        buffer_duration_100ns: i64,
        max_attempts: u32,
        retry_delay_ms: u64,
    ) -> Result<Value, ControlError> {
        if generation == 0 {
            return Err(ControlError::InvalidRequest(
                "multi-input preparation requires a nonzero generation".into(),
            ));
        }
        self.get_session(&session_id)?;
        if self.any_native_worker_attached() {
            return Err(ControlError::InvalidRequest(
                "native worker is already attached".into(),
            ));
        }
        let mut session = self.native_paths_session(&session_id)?;
        let plugin_stages =
            self.prepare_plugin_stages(&session, audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ)?;
        // A bypassed plugin still runs in the graph, with its bridge set to
        // pass audio dry. Leaving it out made a later live un-bypass insert a
        // stage whose worker pipeline was empty: a quantum of silence and a
        // latency step. Live flag changes then only flip the bridge flag.
        for node in session
            .nodes
            .iter_mut()
            .filter(|node| node.kind == NodeKind::Plugin && node.enabled && node.bypass)
        {
            if let Ok(bridge) = self.plugin_bridge(&session_id, &node.id) {
                bridge.set_processing_active(false);
                node.bypass = false;
            }
        }
        let media =
            self.session_audio_media(&session, audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ)?;
        let compiled = audiorouter_engine::compile_native_paths_with_plugins_and_audio(
            &session,
            RuntimeGeneration::new(generation),
            &plugin_stages,
            &media,
        )
        .map_err(|error| match error {
            audiorouter_engine::GraphCompileError::UnsupportedPath(path) => ControlError::InvalidRequest(format!(
                "{path} is not supported: a path needs one source or one Mixer, then a single chain, then its outputs"
            )),
            error => ControlError::InvalidRequest(format!("multi-input graph rejected: {error:?}")),
        })?;
        let source_node_ids = compiled.input_node_ids().to_vec();
        let bindings = match bindings {
            MultiInputBindings::Ordered(list) => {
                if source_node_ids.len() != list.len() {
                    return Err(ControlError::InvalidRequest(
                        "source binding count does not match the prepared graph source count"
                            .into(),
                    ));
                }
                list.iter().collect::<Vec<_>>()
            }
            MultiInputBindings::ByNode(map) => source_node_ids
                .iter()
                .map(|node_id| {
                    map.get(node_id).ok_or_else(|| {
                        let name = session
                            .nodes
                            .iter()
                            .find(|node| node.id == *node_id)
                            .map_or(node_id.as_str(), |node| node.name.as_str());
                        ControlError::InvalidRequest(format!(
                            "no device or application is chosen for {name}"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        };
        let mut source_channels = Vec::with_capacity(source_node_ids.len());
        // Per source: render a 5.1/7.1 capture to two ears (headphones or speakers).
        let mut binaural_sources = Vec::with_capacity(source_node_ids.len());
        for (node_id, binding) in source_node_ids.iter().zip(bindings.iter().copied()) {
            let node = session
                .nodes
                .iter()
                .find(|node| node.id == *node_id)
                .ok_or_else(|| {
                    ControlError::InvalidRequest("prepared source node is missing".into())
                })?;
            if !node.enabled || node.bypass {
                return Err(ControlError::InvalidRequest(
                    "multi-input capture sources must be enabled and not bypassed".into(),
                ));
            }
            let channels = node
                .ports
                .iter()
                .find(|port| port.direction == PortDirection::Output)
                .map(|port| usize::from(port.channels))
                .ok_or_else(|| {
                    ControlError::InvalidRequest("source node output port is missing".into())
                })?;
            match (node.kind, binding) {
                (
                    NodeKind::TestSignal | NodeKind::AudioFile,
                    NativeMultiInputSourceBinding::Generated,
                ) => {}
                (NodeKind::NetworkReceive, NativeMultiInputSourceBinding::Network(_)) => {
                    if !(1..=audiorouter_windows_audio::MAX_NETWORK_CHANNELS).contains(&channels) {
                        return Err(ControlError::InvalidRequest(
                            "a Network Receive node plays mono or stereo audio".into(),
                        ));
                    }
                }
                (NodeKind::PhysicalInput, NativeMultiInputSourceBinding::Physical(endpoint))
                    if node_spatial_headphones(node) =>
                {
                    if !endpoint.is_ieee_float32()
                        || !multi_path_rate_supported(endpoint.sample_rate_hz)
                        || channels != 2
                    {
                        return Err(ControlError::InvalidRequest(format!(
                            "{}: surround to headphones needs a float capture device at 8–192 kHz",
                            node.name
                        )));
                    }
                    if !matches!(endpoint.channels, 6 | 8)
                        || audiorouter_dsp::binaural::BinauralRenderer::new(
                            usize::from(endpoint.channels),
                            endpoint.channel_mask,
                        )
                        .is_err()
                    {
                        return Err(ControlError::InvalidRequest(format!(
                            "{}: surround to headphones needs a 5.1 or 7.1 device, but the selected device has {} channel(s). Set the game's playback device to 7.1 in Windows Sound settings (Configure speakers), or turn surround off for this input",
                            node.name, endpoint.channels
                        )));
                    }
                    binaural_sources.push(node_spatial_options(node));
                    source_channels.push(channels);
                    continue;
                }
                (NodeKind::PhysicalInput, NativeMultiInputSourceBinding::Physical(endpoint)) => {
                    if endpoint.direction != audiorouter_windows_audio::EndpointDirection::Capture
                        || !endpoint.is_ieee_float32()
                        || !multi_path_rate_supported(endpoint.sample_rate_hz)
                        || usize::from(endpoint.channels) != channels
                    {
                        return Err(ControlError::InvalidRequest(
                            "capture endpoints must exactly match the physical-input channel/rate/float32 contract".into(),
                        ));
                    }
                }
                (
                    NodeKind::ApplicationCapture,
                    NativeMultiInputSourceBinding::Application {
                        process_id,
                        expected_executable,
                        expected_creation_time_100ns,
                        ..
                    },
                ) => {
                    let process_id_value = Value::from(u64::from(*process_id));
                    let creation_time_value = Value::from(expected_creation_time_100ns.to_string());
                    let matches_identity = node
                        .parameters
                        .get("processId")
                        .is_some_and(|value| value == &process_id_value)
                        && node
                            .parameters
                            .get("executable")
                            .and_then(Value::as_str)
                            .is_some_and(|value| value.eq_ignore_ascii_case(expected_executable))
                        && node
                            .parameters
                            .get("creationTime100ns")
                            .is_some_and(|value| value == &creation_time_value);
                    if !matches_identity || channels != 2 {
                        return Err(ControlError::InvalidRequest(
                            "application capture binding does not match the enabled node's identity".into(),
                        ));
                    }
                }
                _ => {
                    return Err(ControlError::InvalidRequest(
                        "multi-input capture sources must be enabled physical inputs or application captures matching their binding kind".into(),
                    ));
                }
            }
            binaural_sources.push(None);
            source_channels.push(channels);
        }
        let mixer = audiorouter_engine::RealtimeMixerFanout::from_paths(
            compiled,
            4,
            &source_channels,
            audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
        )
        .map_err(|error| {
            ControlError::InvalidRequest(format!(
                "multi-input feeder preparation failed: {error:?}"
            ))
        })?;
        if self.endpoint_monitor.is_none() {
            self.endpoint_monitor = Some(
                audiorouter_windows_audio::EndpointMonitor::start().map_err(audio_control_error)?,
            );
        }
        let mut capture_clients = Vec::with_capacity(bindings.len());
        let mut application_sources = Vec::new();
        for (binding, binaural) in bindings
            .iter()
            .copied()
            .zip(binaural_sources.iter().copied())
        {
            match binding {
                NativeMultiInputSourceBinding::Network(node) => capture_clients.push(
                    audiorouter_windows_audio::MultiInputCaptureSource::Network(
                        start_network_receiver(node)?,
                    ),
                ),
                NativeMultiInputSourceBinding::Generated => capture_clients.push(
                    audiorouter_windows_audio::MultiInputCaptureSource::Silence(
                        audiorouter_windows_audio::SilentCapture::new(
                            audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
                        ),
                    ),
                ),
                NativeMultiInputSourceBinding::Physical(endpoint) => {
                    let monitor = self
                        .endpoint_monitor
                        .as_mut()
                        .expect("endpoint monitor initialized above");
                    // The multi-path graph runs at 48 kHz; a 44.1/96 kHz device
                    // is resampled by the Windows audio engine as it opens.
                    let _ = buffer_duration_100ns;
                    let opened = if endpoint.direction
                        == audiorouter_windows_audio::EndpointDirection::Render
                    {
                        audiorouter_windows_audio::SharedCapture::open_loopback_at_rate(
                            &endpoint.id,
                            audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
                        )
                    } else {
                        audiorouter_windows_audio::SharedCapture::open_bound_at_rate_with_retry(
                            monitor,
                            endpoint,
                            audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
                            max_attempts,
                            retry_delay_ms,
                        )
                    };
                    match opened {
                        Ok(client) if binaural.is_some() => capture_clients.push(
                            audiorouter_windows_audio::MultiInputCaptureSource::Binaural(
                                audiorouter_windows_audio::BinauralCapture::new(
                                    client,
                                    usize::from(endpoint.channels),
                                    endpoint.channel_mask,
                                    binaural.unwrap_or_default(),
                                )
                                .map_err(|error| {
                                    ControlError::InvalidRequest(format!(
                                        "surround to headphones cannot use this device layout: {error:?}"
                                    ))
                                })?,
                            ),
                        ),
                        Ok(client) => capture_clients.push(
                            audiorouter_windows_audio::MultiInputCaptureSource::Physical(client),
                        ),
                        Err(error) => return Err(endpoint_audio_control_error(error, &endpoint.id)),
                    }
                }
                NativeMultiInputSourceBinding::Application {
                    process_id,
                    expected_executable,
                    expected_executable_path,
                    expected_creation_time_100ns,
                    mode,
                } => {
                    // A closed application does not block the Mixer route: its
                    // input starts silent and reconnects when the app starts.
                    let bound = match audiorouter_windows_audio::bind_application_or_restarted(
                        *process_id,
                        expected_executable,
                        *expected_executable_path,
                        *expected_creation_time_100ns,
                    ) {
                        Ok(application) => Some(application),
                        Err(
                            audiorouter_windows_audio::AudioError::ApplicationNotFound { .. }
                            | audiorouter_windows_audio::AudioError::ApplicationIdentityChanged {
                                ..
                            }
                            | audiorouter_windows_audio::AudioError::ApplicationRestartNotFound {
                                ..
                            },
                        ) if expected_executable_path.is_some() => None,
                        Err(error) => return Err(audio_control_error(error)),
                    };
                    let (capture, application) = match bound {
                        Some(application) => (
                            audiorouter_windows_audio::MultiInputCaptureSource::ApplicationLoopback(
                                audiorouter_windows_audio::ProcessLoopbackCapture::open(
                                    application.process_id,
                                    *mode,
                                )
                                .map_err(audio_control_error)?,
                            ),
                            Some(application),
                        ),
                        None => (
                            audiorouter_windows_audio::MultiInputCaptureSource::Silence(
                                audiorouter_windows_audio::SilentCapture::new(
                                    audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
                                ),
                            ),
                            None,
                        ),
                    };
                    application_sources.push(MultiInputApplicationSource {
                        session_id: session_id.clone(),
                        node_id: source_node_ids[capture_clients.len()].clone(),
                        input_index: capture_clients.len(),
                        executable: (*expected_executable).to_owned(),
                        executable_path: expected_executable_path.map(str::to_owned),
                        current_executable_path: application
                            .as_ref()
                            .and_then(|application| application.executable_path.clone()),
                        process_id: application
                            .as_ref()
                            .map_or(0, |application| application.process_id),
                        creation_time_100ns: application
                            .as_ref()
                            .and_then(|application| application.creation_time_100ns)
                            .unwrap_or(*expected_creation_time_100ns),
                        mode: *mode,
                        state: "configured-stopped",
                        detail: "Prepared for this application. Start the route to capture audio.",
                        // A closed application is looked for right away.
                        next_probe_at: if application.is_some() {
                            Instant::now() + APPLICATION_CAPTURE_LIVENESS_POLL
                        } else {
                            Instant::now()
                        },
                        retry_delay: APPLICATION_CAPTURE_RETRY_MIN,
                    });
                    capture_clients.push(capture);
                }
            }
        }
        let feeder = audiorouter_windows_audio::WasapiMultiInputFanout::new(
            mixer,
            &source_channels,
            audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
            audiorouter_windows_audio::MAX_FLOAT32_ACCUMULATOR_FRAMES,
        )
        .map_err(|error| {
            ControlError::InvalidRequest(format!(
                "multi-input feeder preparation failed: {error:?}"
            ))
        })?;
        let worker =
            audiorouter_windows_audio::NativeMultiInputWorker::new(capture_clients, feeder)
                .map_err(|error| {
                    ControlError::InvalidRequest(format!(
                        "multi-input worker preparation failed: {error:?}"
                    ))
                })?;
        self.attach_native_multi_input_worker(session_id.clone(), generation, worker)?;
        self.multi_input_application_sources = application_sources;
        self.register_multi_input_generators(&session_id);
        self.pending_endpoint_changes.clear();
        Ok(json!({
            "sessionId": session_id,
            "generation": generation,
            "state": "configured-stopped",
            "sources": bindings.iter().map(|binding| match binding {
                NativeMultiInputSourceBinding::Physical(endpoint) => json!({ "kind": "physical", "endpointId": endpoint.id }),
                NativeMultiInputSourceBinding::Generated => json!({ "kind": "generated" }),
                NativeMultiInputSourceBinding::Network(node) => json!({ "kind": "network", "nodeId": node.id }),
                NativeMultiInputSourceBinding::Application { process_id, expected_executable, .. } => json!({ "kind": "application", "processId": process_id, "executable": expected_executable }),
            }).collect::<Vec<_>>(),
            "sourceNodeIds": source_node_ids,
            "branchNodeIds": self
                .native_multi_input_worker
                .as_ref()
                .expect("worker attached above")
                .output_node_ids(),
        }))
    }

    #[cfg(windows)]
    pub fn start_native_multi_input_worker(&mut self) -> Result<(), ControlError> {
        let session_id = self
            .native_multi_input_worker_session
            .clone()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native multi-input worker is not attached".into())
            })?;
        let generation = self.native_multi_input_worker_generation.ok_or_else(|| {
            ControlError::InvalidRequest("native multi-input worker generation is missing".into())
        })?;
        let runtime = self.runtimes.get(&session_id).ok_or_else(|| {
            ControlError::InvalidRequest("native multi-input worker session is unavailable".into())
        })?;
        if runtime.state() != RuntimeState::Running || runtime.generation() != generation {
            return Err(ControlError::InvalidRequest(
                "native multi-input worker requires the matching running session generation".into(),
            ));
        }
        self.native_multi_input_worker
            .as_mut()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native multi-input worker is not attached".into())
            })?
            .start()
            .map_err(|error| {
                ControlError::InvalidRequest(format!("native input start failed: {error:?}"))
            })
    }

    #[cfg(windows)]
    pub fn stop_native_multi_input_worker(&mut self) -> Result<(), ControlError> {
        if let Some(session_id) = self.native_multi_input_worker_session.as_ref() {
            if self
                .runtimes
                .get(session_id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
            {
                return Err(ControlError::InvalidRequest(
                    "stop the session before stopping its native multi-input worker".into(),
                ));
            }
        }
        self.native_multi_input_worker
            .as_mut()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native multi-input worker is not attached".into())
            })?
            .stop()
            .map_err(|error| {
                ControlError::InvalidRequest(format!("native input stop failed: {error:?}"))
            })
    }

    #[cfg(windows)]
    /// Attach several stopped exact physical render workers to the one
    /// control-owned capture/graph worker. The fan-out taps are merged into
    /// the next matching graph generation; no output stream is started by
    /// attachment.
    pub fn attach_native_output_fanout(
        &mut self,
        session_id: EntityId,
        generation: u64,
        fanout: audiorouter_windows_audio::WasapiOutputFanout,
    ) -> Result<(), ControlError> {
        self.get_session(&session_id)?;
        if !self.native_endpoint_session_is_attached(&session_id) {
            return Err(ControlError::InvalidRequest(
                "native output fan-out requires an attached endpoint worker".into(),
            ));
        }
        if self.native_output_fanout.is_some() {
            return Err(ControlError::InvalidRequest(
                "native output fan-out is already attached".into(),
            ));
        }
        if fanout.is_running() || fanout.generation() != generation || generation == 0 {
            return Err(ControlError::InvalidRequest(
                "native output fan-out must be stopped and match a nonzero generation".into(),
            ));
        }
        self.native_output_fanout = Some(fanout);
        self.native_output_fanout_session = Some(session_id);
        self.native_output_fanout_generation = Some(generation);
        Ok(())
    }

    #[cfg(windows)]
    /// Attach a stopped, already-negotiated duplex bridge worker to one
    /// session. Endpoint opening and bridge negotiation remain outside this
    /// control operation; the session generation is checked again at start.
    pub fn attach_native_duplex_worker(
        &mut self,
        session_id: EntityId,
        generation: u64,
        worker: audiorouter_windows_audio::NativeBridgeDuplexWorker,
    ) -> Result<(), ControlError> {
        self.get_session(&session_id)?;
        if self.any_native_worker_attached() {
            return Err(ControlError::InvalidRequest(
                "native worker is already attached".into(),
            ));
        }
        if self.native_render_source_worker.is_some() {
            return Err(ControlError::InvalidRequest(
                "native render-source worker conflicts with a duplex worker".into(),
            ));
        }
        if worker.is_running() {
            return Err(ControlError::InvalidRequest(
                "native duplex worker must be stopped before attachment".into(),
            ));
        }
        worker.set_privacy_muted(self.privacy_muted);
        self.native_duplex_worker = Some(worker);
        self.native_duplex_worker_session = Some(session_id);
        self.native_duplex_worker_generation = Some(generation);
        Ok(())
    }

    #[cfg(windows)]
    /// Transfer a prepared render-source lease into a stopped worker that
    /// renders the virtual bus to one exact physical output. This worker may
    /// coexist with the session's primary capture/graph worker, but its bus
    /// and generation remain independently bound and its endpoint stays
    /// stopped until session start.
    pub fn attach_prepared_native_render_source_worker(
        &mut self,
        bus_id: &EntityId,
        session_id: EntityId,
        generation: u64,
        render: audiorouter_windows_audio::SharedRender,
        bridge: audiorouter_windows_audio::WasapiSchedulerBridge,
    ) -> Result<(), ControlError> {
        self.get_session(&session_id)?;
        if generation == 0 {
            return Err(ControlError::InvalidRequest(
                "native render-source worker generation must be nonzero".into(),
            ));
        }
        if self.native_render_source_worker.is_some() {
            return Err(ControlError::InvalidRequest(
                "native render-source worker is already attached".into(),
            ));
        }
        if self.native_duplex_worker.is_some() {
            return Err(ControlError::InvalidRequest(
                "native render-source worker conflicts with a duplex worker".into(),
            ));
        }
        let binding = self
            .native_render_source_bindings
            .get(bus_id)
            .ok_or_else(|| {
                ControlError::InvalidRequest("native render-source binding is not prepared".into())
            })?;
        if binding.generation() != generation {
            return Err(ControlError::InvalidRequest(
                "native render-source binding generation is stale".into(),
            ));
        }
        let binding = self
            .native_render_source_bindings
            .remove(bus_id)
            .expect("binding was checked above");
        self.deactivate_virtual_bridge(bus_id);
        let worker = audiorouter_windows_audio::NativeBridgeInputWorker::from_render_source_binding(
            binding, render, bridge,
        );
        worker.set_privacy_muted(self.privacy_muted);
        self.native_render_source_worker = Some(worker);
        self.native_render_source_worker_session = Some(session_id);
        self.native_render_source_worker_generation = Some(generation);
        Ok(())
    }

    /// Prepare a stopped native worker from two endpoint descriptors returned
    /// by the authoritative device inventory. This is an explicit control
    /// operation: it refreshes and revalidates the exact persisted identities,
    /// opens clients in stopped state, and never selects a replacement. The
    /// caller must still invoke `start_native_endpoint_worker` (normally via
    /// `session_start`) to begin moving audio.
    pub fn prepare_native_endpoint_worker(
        &mut self,
        session_id: EntityId,
        capture: &audiorouter_windows_audio::EndpointInfo,
        render: &audiorouter_windows_audio::EndpointInfo,
        buffer_duration_100ns: i64,
        max_attempts: u32,
        retry_delay_ms: u64,
    ) -> Result<(), ControlError> {
        self.get_session(&session_id)?;
        if self.native_endpoint_session_is_attached(&session_id)
            || !self.native_endpoint_has_capacity()
        {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker limit reached or session is already attached".into(),
            ));
        }
        let bridge =
            audiorouter_windows_audio::WasapiSchedulerBridge::new_for_endpoints_at_graph_rate(
                2,
                capture,
                render,
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
                audiorouter_windows_audio::MAX_FLOAT32_ACCUMULATOR_FRAMES,
                audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
            )
            .map_err(audio_control_error)?;
        if self.endpoint_monitor.is_none() {
            self.endpoint_monitor = Some(
                audiorouter_windows_audio::EndpointMonitor::start().map_err(audio_control_error)?,
            );
        }
        let monitor = self
            .endpoint_monitor
            .as_mut()
            .expect("endpoint monitor initialized above");
        let capture_client =
            audiorouter_windows_audio::SharedCapture::open_refreshed_bound_with_retry(
                monitor,
                capture,
                buffer_duration_100ns,
                max_attempts,
                retry_delay_ms,
            )
            .map_err(audio_control_error)?;
        let render_client =
            match audiorouter_windows_audio::SharedRender::open_refreshed_bound_with_retry(
                monitor,
                render,
                buffer_duration_100ns,
                max_attempts,
                retry_delay_ms,
            ) {
                Ok(render_client) => render_client,
                Err(error) => {
                    drop(capture_client);
                    return Err(audio_control_error(error));
                }
            };
        let worker = audiorouter_windows_audio::WasapiEndpointWorker::new(
            capture_client,
            render_client,
            bridge,
        );
        let result = self.attach_native_endpoint_worker(session_id, worker);
        if result.is_ok() {
            // Preparation resolved the latest monitor snapshot explicitly;
            // older inventory observations must not invalidate this fresh
            // stopped binding when it is later started.
            self.pending_endpoint_changes.clear();
        }
        result
    }
}
