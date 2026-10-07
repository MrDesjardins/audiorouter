//! Native output fan-out, endpoint rebinding, virtual bridges and capture/render/duplex bindings.

use super::*;

impl ControlPlane {
    #[cfg(windows)]
    /// Prepare several exact physical render clients as independent output
    /// branches of one already-prepared native endpoint session. Every
    /// endpoint is resolved from the current active inventory before opening;
    /// no default or substitute endpoint is selected.
    pub fn prepare_native_output_fanout(
        &mut self,
        session_id: EntityId,
        generation: u64,
        render_endpoint_ids: &[String],
    ) -> Result<Value, ControlError> {
        self.prepare_native_output_fanout_with(session_id, generation, render_endpoint_ids, false)
    }

    #[cfg(windows)]
    /// As [`Self::prepare_native_output_fanout`]. With `allow_shared_endpoints`,
    /// one render endpoint may serve several independent paths, each through
    /// its own shared-mode client (Windows mixes them).
    pub(crate) fn prepare_native_output_fanout_with(
        &mut self,
        session_id: EntityId,
        generation: u64,
        render_endpoint_ids: &[String],
        allow_shared_endpoints: bool,
    ) -> Result<Value, ControlError> {
        let multi_owner = self.native_multi_input_worker_session.as_ref() == Some(&session_id)
            && self.native_multi_input_worker.is_some();
        if (!multi_owner && render_endpoint_ids.is_empty())
            || render_endpoint_ids.len() > audiorouter_engine::MAX_AUDIO_TAPS
            || generation == 0
        {
            return Err(ControlError::InvalidRequest(
                "output fan-out requires bounded endpoints and a nonzero generation".into(),
            ));
        }
        let endpoint_owner = self.native_endpoint_session_is_attached(&session_id);
        if !endpoint_owner && !multi_owner {
            return Err(ControlError::InvalidRequest(
                "output fan-out requires an attached native endpoint or multi-input worker".into(),
            ));
        }
        if endpoint_owner && self.native_output_fanout.is_some() {
            return Err(ControlError::InvalidRequest(
                "native output fan-out is already attached".into(),
            ));
        }
        if multi_owner {
            let branch_node_ids = self
                .native_multi_input_worker
                .as_ref()
                .expect("multi-input owner is attached")
                .output_node_ids()
                .to_vec();
            let session = self.get_session(&session_id)?.clone();
            let mut seen_tap_branch = false;
            let physical_branch_count = branch_node_ids
                .iter()
                .filter(|node_id| {
                    session.nodes.iter().any(|node| {
                        node.id == **node_id
                            && node.kind == NodeKind::PhysicalOutput
                            && node.enabled
                            && !node.bypass
                    })
                })
                .count();
            let branches_are_supported = branch_node_ids.iter().all(|node_id| {
                let Some(node) = session.nodes.iter().find(|node| node.id == *node_id) else {
                    return false;
                };
                let is_physical = node.kind == NodeKind::PhysicalOutput;
                if !is_physical {
                    seen_tap_branch = true;
                }
                !seen_tap_branch || !is_physical
            });
            if physical_branch_count != render_endpoint_ids.len() || !branches_are_supported {
                return Err(ControlError::InvalidRequest(
                    "multi-input output preparation requires exact render IDs for an initial physical branch prefix; virtual/recording branches follow it".into(),
                ));
            }
        }
        if render_endpoint_ids
            .iter()
            .any(|id| id.is_empty() || id.len() > MAX_CONTROL_STRING_BYTES)
        {
            return Err(ControlError::InvalidRequest(
                "render endpoint IDs exceed the bounded control string limit".into(),
            ));
        }
        let endpoints =
            audiorouter_windows_audio::enumerate_active_endpoints().map_err(audio_control_error)?;
        let mut outputs = Vec::with_capacity(render_endpoint_ids.len());
        for (index, endpoint_id) in render_endpoint_ids.iter().enumerate() {
            if !allow_shared_endpoints
                && render_endpoint_ids[..index]
                    .iter()
                    .any(|existing| existing == endpoint_id)
            {
                return Err(ControlError::InvalidRequest(
                    "output fan-out endpoint IDs must be unique".into(),
                ));
            }
            let endpoint = endpoints
                .iter()
                .find(|endpoint| {
                    endpoint.id == *endpoint_id
                        && endpoint.direction
                            == audiorouter_windows_audio::EndpointDirection::Render
                })
                .ok_or_else(|| {
                    ControlError::InvalidRequest(
                        "output fan-out endpoint is not an active exact inventory match".into(),
                    )
                })?;
            if !endpoint.is_ieee_float32()
                || endpoint.channels != 2
                || !multi_path_rate_supported(endpoint.sample_rate_hz)
            {
                return Err(ControlError::InvalidRequest(
                    "output fan-out endpoints must be stereo IEEE float32 at 8–192 kHz".into(),
                ));
            }
            // 50 ms device headroom and an 8-quantum ring absorb capture
            // bursts and late plugin blocks. Neither adds delay: the pump
            // queues only processed audio plus its bounded jitter cushion.
            let render = audiorouter_windows_audio::SharedRender::open_with_headroom_at_rate(
                endpoint_id,
                MULTI_INPUT_RENDER_HEADROOM_100NS,
                audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
            )
            .map_err(|error| endpoint_audio_control_error(error, endpoint_id))?;
            let ring = Arc::new(
                audiorouter_engine::AudioBlockRing::new(
                    8,
                    2,
                    audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
                )
                .map_err(|_| {
                    ControlError::InvalidRequest("output fan-out ring shape is invalid".into())
                })?,
            );
            outputs.push((render, ring));
        }
        let mut fanout = if outputs.is_empty() {
            audiorouter_windows_audio::WasapiOutputFanout::new_tap_only(
                generation,
                2,
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
            )
        } else {
            audiorouter_windows_audio::WasapiOutputFanout::new(
                outputs,
                generation,
                2,
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
            )
        }
        .map_err(|error| {
            ControlError::InvalidRequest(format!("output fan-out preparation failed: {error:?}"))
        })?;
        if multi_owner {
            let branch_node_ids = self
                .native_multi_input_worker
                .as_ref()
                .expect("multi-input owner is attached")
                .output_node_ids()
                .to_vec();
            let session = self.get_session(&session_id)?.clone();
            for node_id in branch_node_ids.iter().skip(render_endpoint_ids.len()) {
                let channels = session
                    .nodes
                    .iter()
                    .find(|node| node.id == *node_id)
                    .and_then(|node| {
                        node.ports
                            .iter()
                            .find(|port| port.direction == PortDirection::Input)
                    })
                    .map(|port| usize::from(port.channels))
                    .ok_or_else(|| {
                        ControlError::InvalidRequest("output branch input port is missing".into())
                    })?;
                if channels != 2 {
                    return Err(ControlError::InvalidRequest(
                        "multi-input tap-only output branches must be stereo".into(),
                    ));
                }
                // A Network Send branch streams its quantum from the graph tap;
                // the sender (socket and I/O thread) lives with the fan-out.
                let mut branch_taps = AudioTapSet::new();
                if let Some(node) = session
                    .nodes
                    .iter()
                    .find(|node| node.id == *node_id && node.kind == NodeKind::NetworkSend)
                {
                    let sender = start_network_sender(node)?;
                    branch_taps.add(sender.tap()).map_err(|_| {
                        ControlError::InvalidRequest("network send tap capacity exceeded".into())
                    })?;
                    fanout.keep_network_sender(sender);
                }
                fanout
                    .append_tap_branch(
                        branch_taps,
                        channels,
                        audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
                    )
                    .map_err(|error| {
                        ControlError::InvalidRequest(format!(
                            "tap-only branch preparation failed: {error:?}"
                        ))
                    })?;
            }
        }
        let output_count = fanout.output_count();
        if multi_owner {
            self.native_multi_input_worker
                .as_mut()
                .expect("multi-input owner is attached")
                .attach_output_fanout(fanout)
                .map_err(|error| {
                    ControlError::InvalidRequest(format!(
                        "multi-input output fan-out preparation failed: {error:?}"
                    ))
                })?;
        } else {
            self.attach_native_output_fanout(session_id.clone(), generation, fanout)?;
        }
        Ok(json!({
            "sessionId": session_id,
            "generation": generation,
            "state": "configured-stopped",
            "renderEndpointIds": render_endpoint_ids,
            "outputCount": output_count,
        }))
    }

    #[cfg(windows)]
    /// Rebind an attached stopped native endpoint worker to exact refreshed
    /// capture/render IDs. The worker is left stopped so the caller can
    /// inspect the result before deliberately restarting the session.
    pub fn rebind_native_endpoint_worker(
        &mut self,
        session_id: &EntityId,
        capture_endpoint_id: &str,
        render_endpoint_id: &str,
        buffer_duration_100ns: i64,
        max_attempts: u32,
        retry_delay_ms: u64,
    ) -> Result<(), ControlError> {
        self.get_session(session_id)?;
        if !self.native_endpoint_session_is_attached(session_id) {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker is not bound to the session".into(),
            ));
        }
        if self
            .runtimes
            .get(session_id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
        {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker must be rebound while the session is stopped".into(),
            ));
        }
        if self.endpoint_monitor.is_none() {
            self.endpoint_monitor = Some(
                audiorouter_windows_audio::EndpointMonitor::start().map_err(audio_control_error)?,
            );
        }
        let (capture, render) = {
            let monitor = self
                .endpoint_monitor
                .as_mut()
                .expect("endpoint monitor initialized above");
            monitor.refresh_changes().map_err(audio_control_error)?;
            let capture = monitor
                .snapshot()
                .iter()
                .find(|endpoint| {
                    endpoint.id == capture_endpoint_id
                        && endpoint.direction
                            == audiorouter_windows_audio::EndpointDirection::Capture
                })
                .cloned()
                .ok_or_else(|| {
                    ControlError::InvalidRequest("capture endpoint is not active".into())
                })?;
            let render = monitor
                .snapshot()
                .iter()
                .find(|endpoint| {
                    endpoint.id == render_endpoint_id
                        && endpoint.direction
                            == audiorouter_windows_audio::EndpointDirection::Render
                })
                .cloned()
                .ok_or_else(|| {
                    ControlError::InvalidRequest("render endpoint is not active".into())
                })?;
            (capture, render)
        };
        let mut monitor = self.endpoint_monitor.take().ok_or_else(|| {
            ControlError::InvalidRequest("endpoint monitor is not available".into())
        })?;
        let result = self
            .native_endpoint_worker_for_session_mut(session_id)
            .ok_or_else(|| {
                ControlError::InvalidRequest("native endpoint worker is not attached".into())
            })?
            .rebind_with_refreshed_bound_with_retry(
                &mut monitor,
                &capture,
                &render,
                buffer_duration_100ns,
                max_attempts,
                retry_delay_ms,
            )
            .map_err(audio_control_error);
        self.endpoint_monitor = Some(monitor);
        if result.is_ok() {
            self.pending_endpoint_changes.clear();
        }
        result
    }

    #[cfg(windows)]
    /// Prepare one explicit project-driver capture-sink binding. This opens
    /// only the supplied driver path and mapping, remains stopped, and is
    /// selected by graph activation only when the graph names the same bus.
    pub fn prepare_native_capture_sink_binding(
        &mut self,
        bus_id: EntityId,
        device_path: &str,
        mapping_path: impl AsRef<std::path::Path>,
        hello: audiorouter_protocol::AudioBridgeHello,
    ) -> Result<(), ControlError> {
        let known_enabled = self
            .virtual_buses
            .list()
            .iter()
            .any(|bus| bus.id() == &bus_id && bus.enabled());
        if !known_enabled {
            return Err(ControlError::InvalidRequest(
                "native capture sink requires a known enabled virtual bus".into(),
            ));
        }
        if hello.bus_id != bus_id.as_str()
            || hello.direction != audiorouter_protocol::AudioBridgeDirection::CaptureSink
        {
            return Err(ControlError::InvalidRequest(
                "native capture sink hello does not match the requested bus".into(),
            ));
        }
        if self.native_capture_sink_bindings.contains_key(&bus_id) {
            return Err(ControlError::InvalidRequest(
                "native capture sink binding is already prepared".into(),
            ));
        }
        let binding = audiorouter_windows_audio::NativeBridgeCaptureSinkBinding::create(
            device_path,
            mapping_path,
            hello,
        )
        .map_err(|error| {
            ControlError::InvalidRequest(format!(
                "native capture sink preparation failed: {error:?}"
            ))
        })?;
        self.native_capture_sink_bindings.insert(bus_id, binding);
        Ok(())
    }

    #[cfg(windows)]
    /// Prepare one explicit project-driver render-source binding. The
    /// endpoint worker consumes it later; this method only owns the
    /// negotiated lease and keeps it stopped-by-default.
    pub fn prepare_native_render_source_binding(
        &mut self,
        bus_id: EntityId,
        device_path: &str,
        mapping_path: impl AsRef<std::path::Path>,
        hello: audiorouter_protocol::AudioBridgeHello,
    ) -> Result<(), ControlError> {
        let known_enabled = self
            .virtual_buses
            .list()
            .iter()
            .any(|bus| bus.id() == &bus_id && bus.enabled());
        if !known_enabled {
            return Err(ControlError::InvalidRequest(
                "native render source requires a known enabled virtual bus".into(),
            ));
        }
        if hello.bus_id != bus_id.as_str()
            || hello.direction != audiorouter_protocol::AudioBridgeDirection::RenderSource
        {
            return Err(ControlError::InvalidRequest(
                "native render source hello does not match the requested bus".into(),
            ));
        }
        if self.native_render_source_bindings.contains_key(&bus_id) {
            return Err(ControlError::InvalidRequest(
                "native render source binding is already prepared".into(),
            ));
        }
        let binding = audiorouter_windows_audio::NativeBridgeRenderSourceBinding::create(
            device_path,
            mapping_path,
            hello,
        )
        .map_err(|error| {
            ControlError::InvalidRequest(format!(
                "native render source preparation failed: {error:?}"
            ))
        })?;
        self.native_render_source_bindings.insert(bus_id, binding);
        Ok(())
    }

    #[cfg(windows)]
    /// Prepare both directional project-driver leases for one enabled bus.
    /// The duplex controller rolls back the render lease if capture claiming
    /// fails, so callers never retain a half-prepared bus.
    pub fn prepare_native_duplex_binding(
        &mut self,
        bus_id: EntityId,
        device_path: &str,
        render_mapping_path: impl AsRef<std::path::Path>,
        capture_mapping_path: impl AsRef<std::path::Path>,
        render_hello: audiorouter_protocol::AudioBridgeHello,
        capture_hello: audiorouter_protocol::AudioBridgeHello,
    ) -> Result<(), ControlError> {
        let known_enabled = self
            .virtual_buses
            .list()
            .iter()
            .any(|bus| bus.id() == &bus_id && bus.enabled());
        if !known_enabled {
            return Err(ControlError::InvalidRequest(
                "native duplex requires a known enabled virtual bus".into(),
            ));
        }
        if render_hello.bus_id != bus_id.as_str()
            || capture_hello.bus_id != bus_id.as_str()
            || render_hello.direction != audiorouter_protocol::AudioBridgeDirection::RenderSource
            || capture_hello.direction != audiorouter_protocol::AudioBridgeDirection::CaptureSink
            || render_hello.generation != capture_hello.generation
        {
            return Err(ControlError::InvalidRequest(
                "native duplex hellos do not match the requested bus".into(),
            ));
        }
        if self.native_duplex_bindings.contains_key(&bus_id) {
            return Err(ControlError::InvalidRequest(
                "native duplex binding is already prepared".into(),
            ));
        }
        let binding = audiorouter_windows_audio::NativeBridgeDuplexBinding::create(
            device_path,
            render_mapping_path,
            capture_mapping_path,
            render_hello,
            capture_hello,
        )
        .map_err(|error| {
            ControlError::InvalidRequest(format!("native duplex preparation failed: {error:?}"))
        })?;
        self.native_duplex_bindings.insert(bus_id, binding);
        Ok(())
    }

    #[cfg(windows)]
    /// Prepare one paired project-driver bridge through the shared control
    /// boundary. The caller supplies only an exact broker device path and two
    /// new absolute mapping paths; hello identity is derived from the
    /// requested bus/generation so adapters cannot disagree on direction or
    /// protocol shape.
    pub fn prepare_native_bridge(
        &mut self,
        bus_id: EntityId,
        device_path: &str,
        render_mapping_path: &str,
        capture_mapping_path: &str,
        generation: u64,
        lease_ms: u32,
    ) -> Result<Value, ControlError> {
        if device_path != r"\\.\AudioRouterVirtualBridge" {
            return Err(ControlError::InvalidRequest(
                "native bridge device path is not the AudioRouter broker".into(),
            ));
        }
        if generation == 0
            || !(1..=audiorouter_protocol::MAX_AUDIO_BRIDGE_LEASE_MS).contains(&lease_ms)
            || render_mapping_path.is_empty()
            || capture_mapping_path.is_empty()
            || render_mapping_path.len() > MAX_CONTROL_STRING_BYTES
            || capture_mapping_path.len() > MAX_CONTROL_STRING_BYTES
            || render_mapping_path == capture_mapping_path
        {
            return Err(ControlError::InvalidRequest(
                "native bridge paths, lease, and generation are invalid".into(),
            ));
        }
        let render_path = std::path::Path::new(render_mapping_path);
        let capture_path = std::path::Path::new(capture_mapping_path);
        if !render_path.is_absolute() || !capture_path.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "native bridge mapping paths must be absolute".into(),
            ));
        }
        let render_hello = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: bus_id.as_str().to_owned(),
            direction: audiorouter_protocol::AudioBridgeDirection::RenderSource,
            generation,
            sample_rate_hz: audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
            channels: 2,
            frames_per_quantum: audiorouter_engine::PROCESSING_QUANTUM_FRAMES as u16,
            lease_ms,
        };
        let capture_hello = audiorouter_protocol::AudioBridgeHello {
            direction: audiorouter_protocol::AudioBridgeDirection::CaptureSink,
            ..render_hello.clone()
        };
        self.prepare_native_duplex_binding(
            bus_id.clone(),
            device_path,
            render_path,
            capture_path,
            render_hello,
            capture_hello,
        )?;
        Ok(json!({
            "busId": bus_id,
            "generation": generation,
            "state": "configured-stopped",
            "directions": ["renderSource", "captureSink"]
        }))
    }

    #[cfg(windows)]
    pub fn detach_native_bridge(&mut self, bus_id: &EntityId) -> Result<Value, ControlError> {
        if self
            .native_render_source_worker
            .as_ref()
            .is_some_and(|worker| worker.bus_id() == *bus_id)
        {
            self.detach_native_render_source_worker()?;
        } else if self.native_duplex_bindings.contains_key(bus_id) {
            self.detach_native_duplex_binding(bus_id)?;
        } else if self.native_render_source_bindings.contains_key(bus_id) {
            self.detach_native_render_source_binding(bus_id)?;
        } else {
            self.detach_native_capture_sink_binding(bus_id)?;
        }
        Ok(json!({ "busId": bus_id, "state": "detached" }))
    }

    #[cfg(windows)]
    pub fn heartbeat_native_bridges(&mut self) -> Result<Value, ControlError> {
        let staged_bindings = self.native_capture_sink_bindings.len()
            + self.native_render_source_bindings.len()
            + self.native_duplex_bindings.len();
        let mut first_error = self.heartbeat_native_capture_sink_bindings().err();
        if let Err(error) = self.heartbeat_native_render_source_bindings() {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
        if let Err(error) = self.heartbeat_native_duplex_bindings() {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
        let worker_bus_id = self
            .native_duplex_worker
            .as_ref()
            .map(audiorouter_windows_audio::NativeBridgeDuplexWorker::bus_id);
        let worker_present = worker_bus_id.is_some();
        if let Some(bus_id) = worker_bus_id.as_ref() {
            let heartbeat_result = self
                .native_duplex_worker
                .as_mut()
                .expect("native duplex worker remains present")
                .heartbeat();
            if let Err(error) = heartbeat_result {
                self.deactivate_virtual_bridge(bus_id);
                self.publish_virtual_bridge_failure(bus_id);
                if first_error.is_none() {
                    first_error = Some(ControlError::InvalidRequest(format!(
                        "native duplex heartbeat failed; bridge deactivated: {error:?}"
                    )));
                }
            }
        }
        let render_worker_bus_id = self
            .native_render_source_worker
            .as_ref()
            .map(audiorouter_windows_audio::NativeBridgeInputWorker::bus_id);
        let render_worker_present = render_worker_bus_id.is_some();
        if let Some(bus_id) = render_worker_bus_id.as_ref() {
            let heartbeat_result = self
                .native_render_source_worker
                .as_mut()
                .expect("native render-source worker remains present")
                .heartbeat();
            if let Err(error) = heartbeat_result {
                self.deactivate_virtual_bridge(bus_id);
                self.publish_virtual_bridge_failure(bus_id);
                if first_error.is_none() {
                    first_error = Some(ControlError::InvalidRequest(format!(
                        "native render-source worker heartbeat failed; bridge deactivated: {error:?}"
                    )));
                }
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        Ok(json!({
            "state": "healthy",
            "bindings": staged_bindings
                + usize::from(worker_present)
                + usize::from(render_worker_present)
        }))
    }

    #[cfg(windows)]
    /// Service prepared capture-sink leases at their negotiated cadence.
    /// Failure containment remains per binding so another route can continue.
    pub fn heartbeat_native_capture_sink_bindings(&mut self) -> Result<(), ControlError> {
        let bus_ids = self
            .native_capture_sink_bindings
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut first_error = None;
        for bus_id in bus_ids {
            let result = self
                .native_capture_sink_bindings
                .get_mut(&bus_id)
                .expect("captured native binding key must remain present")
                .heartbeat_if_due();
            if let Err(error) = result {
                if let Some(binding) = self.native_capture_sink_bindings.remove(&bus_id) {
                    let _ = binding.close();
                }
                if let Some(bridge) = self.virtual_bridges.get(&bus_id) {
                    bridge.deactivate();
                }
                self.publish_virtual_bridge_failure(&bus_id);
                if first_error.is_none() {
                    first_error = Some(format!(
                        "native capture sink heartbeat failed; binding detached: {error:?}"
                    ));
                }
            }
        }
        first_error.map_or(Ok(()), |message| Err(ControlError::InvalidRequest(message)))
    }

    #[cfg(windows)]
    pub fn heartbeat_native_render_source_bindings(&mut self) -> Result<(), ControlError> {
        let bus_ids = self
            .native_render_source_bindings
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut first_error = None;
        for bus_id in bus_ids {
            let result = self
                .native_render_source_bindings
                .get_mut(&bus_id)
                .expect("captured native binding key must remain present")
                .heartbeat_if_due();
            if let Err(error) = result {
                if let Some(binding) = self.native_render_source_bindings.remove(&bus_id) {
                    let _ = binding.close();
                }
                if let Some(bridge) = self.virtual_bridges.get(&bus_id) {
                    bridge.deactivate();
                }
                self.publish_virtual_bridge_failure(&bus_id);
                if first_error.is_none() {
                    first_error = Some(format!(
                        "native render source heartbeat failed; binding detached: {error:?}"
                    ));
                }
            }
        }
        first_error.map_or(Ok(()), |message| Err(ControlError::InvalidRequest(message)))
    }

    #[cfg(windows)]
    pub fn heartbeat_native_duplex_bindings(&mut self) -> Result<(), ControlError> {
        let bus_ids = self
            .native_duplex_bindings
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut first_error = None;
        for bus_id in bus_ids {
            let result = self
                .native_duplex_bindings
                .get_mut(&bus_id)
                .expect("captured native binding key must remain present")
                .heartbeat();
            if let Err(error) = result {
                if let Some(binding) = self.native_duplex_bindings.remove(&bus_id) {
                    let _ = binding.close();
                }
                if let Some(bridge) = self.virtual_bridges.get(&bus_id) {
                    bridge.deactivate();
                }
                self.publish_virtual_bridge_failure(&bus_id);
                if first_error.is_none() {
                    first_error = Some(format!(
                        "native duplex heartbeat failed; binding detached: {error:?}"
                    ));
                }
            }
        }
        first_error.map_or(Ok(()), |message| Err(ControlError::InvalidRequest(message)))
    }

    pub(crate) fn deactivate_virtual_bridge(&self, bus_id: &EntityId) {
        if let Some(bridge) = self.virtual_bridges.get(bus_id) {
            bridge.deactivate();
        }
    }

    #[cfg(windows)]
    pub fn detach_native_capture_sink_binding(
        &mut self,
        bus_id: &EntityId,
    ) -> Result<(), ControlError> {
        let Some(binding) = self.native_capture_sink_bindings.remove(bus_id) else {
            return Err(ControlError::InvalidRequest(
                "native capture sink binding is not prepared".into(),
            ));
        };
        let close_result = binding.close();
        self.deactivate_virtual_bridge(bus_id);
        close_result.map_err(|error| {
            ControlError::InvalidRequest(format!("native capture sink close failed: {error:?}"))
        })
    }

    #[cfg(windows)]
    pub fn detach_native_render_source_binding(
        &mut self,
        bus_id: &EntityId,
    ) -> Result<(), ControlError> {
        let Some(binding) = self.native_render_source_bindings.remove(bus_id) else {
            return Err(ControlError::InvalidRequest(
                "native render source binding is not prepared".into(),
            ));
        };
        let close_result = binding.close();
        self.deactivate_virtual_bridge(bus_id);
        close_result.map_err(|error| {
            ControlError::InvalidRequest(format!("native render source close failed: {error:?}"))
        })
    }

    #[cfg(windows)]
    pub fn detach_native_duplex_binding(&mut self, bus_id: &EntityId) -> Result<(), ControlError> {
        let Some(binding) = self.native_duplex_bindings.remove(bus_id) else {
            return Err(ControlError::InvalidRequest(
                "native duplex binding is not prepared".into(),
            ));
        };
        let close_result = binding.close();
        self.deactivate_virtual_bridge(bus_id);
        close_result.map_err(|error| {
            ControlError::InvalidRequest(format!("native duplex close failed: {error:?}"))
        })
    }

    #[cfg(windows)]
    /// Transfer a paired bus lease to the two stopped endpoint workers after
    /// checking the exact graph generation.
    pub fn take_native_duplex_worker_parts(
        &mut self,
        bus_id: &EntityId,
        generation: u64,
    ) -> Result<
        (
            audiorouter_windows_audio::NativeBridgeController,
            audiorouter_windows_audio::NativeBridgeController,
            std::sync::Arc<audiorouter_windows_audio::NativeBridgeRealtimeWriter>,
        ),
        ControlError,
    > {
        let binding = self.native_duplex_bindings.get(bus_id).ok_or_else(|| {
            ControlError::InvalidRequest("native duplex binding is not prepared".into())
        })?;
        if binding.generation() != generation {
            return Err(ControlError::InvalidRequest(
                "native duplex binding generation is stale".into(),
            ));
        }
        // Once ownership moves to the worker, the portable bridge must not
        // remain active as a fallback tap. Otherwise a route lookup after the
        // transfer can publish stale portable frames alongside the worker's
        // exact native lease.
        self.deactivate_virtual_bridge(bus_id);
        Ok(self
            .native_duplex_bindings
            .remove(bus_id)
            .expect("binding was checked above")
            .into_worker_parts())
    }

    #[cfg(windows)]
    /// Compose a prepared paired bridge binding with its already-opened,
    /// stopped endpoint workers and attach the resulting duplex worker to one
    /// session. Conflicting native workers are rejected before the lease is
    /// consumed; a failed attachment drops the transferred lease safely.
    pub fn attach_prepared_native_duplex_worker(
        &mut self,
        bus_id: &EntityId,
        session_id: EntityId,
        generation: u64,
        input_render: audiorouter_windows_audio::SharedRender,
        input_bridge: audiorouter_windows_audio::WasapiSchedulerBridge,
        output_endpoint: audiorouter_windows_audio::WasapiEndpointWorker,
    ) -> Result<(), ControlError> {
        self.get_session(&session_id)?;
        if generation == 0 {
            return Err(ControlError::InvalidRequest(
                "native duplex worker generation must be nonzero".into(),
            ));
        }
        if self.any_native_worker_attached() {
            return Err(ControlError::InvalidRequest(
                "native worker is already attached".into(),
            ));
        }
        let binding = self.native_duplex_bindings.get(bus_id).ok_or_else(|| {
            ControlError::InvalidRequest("native duplex binding is not prepared".into())
        })?;
        if binding.generation() != generation {
            return Err(ControlError::InvalidRequest(
                "native duplex binding generation is stale".into(),
            ));
        }
        let binding = self
            .native_duplex_bindings
            .remove(bus_id)
            .expect("binding was checked above");
        self.deactivate_virtual_bridge(bus_id);
        let worker = audiorouter_windows_audio::NativeBridgeDuplexWorker::from_binding(
            binding,
            input_render,
            input_bridge,
            output_endpoint,
        );
        self.attach_native_duplex_worker(session_id, generation, worker)
    }

    #[cfg(windows)]
    /// Transfer a validated render-source lease to an endpoint worker. The
    /// generation is checked before removal so a stale graph cannot consume
    /// a binding prepared for an earlier graph.
    pub fn take_native_render_source_controller(
        &mut self,
        bus_id: &EntityId,
        generation: u64,
    ) -> Result<audiorouter_windows_audio::NativeBridgeController, ControlError> {
        let binding = self
            .native_render_source_bindings
            .get(bus_id)
            .ok_or_else(|| {
                ControlError::InvalidRequest("native render source binding is not prepared".into())
            })?;
        if binding.generation() != generation {
            return Err(ControlError::InvalidRequest(
                "native render source binding generation is stale".into(),
            ));
        }
        // The worker now owns the native lease; prevent the portable bridge
        // from being selected as a stale fallback after the map entry moves.
        self.deactivate_virtual_bridge(bus_id);
        Ok(self
            .native_render_source_bindings
            .remove(bus_id)
            .expect("binding was checked above")
            .into_controller())
    }
}

#[cfg(test)]
#[path = "native_bindings_tests.rs"]
mod tests;
