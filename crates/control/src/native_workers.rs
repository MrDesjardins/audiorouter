//! Starting, stopping, pumping and telemetry of native workers, and native graph activation.

use super::*;

impl ControlPlane {
    /// Start the explicitly attached endpoint pair on the control thread.
    pub fn start_native_endpoint_worker(&mut self) -> Result<(), ControlError> {
        let session_id = self.native_endpoint_session.clone().ok_or_else(|| {
            ControlError::InvalidRequest("native endpoint worker is not attached".into())
        })?;
        if !self
            .runtimes
            .get(&session_id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
        {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker requires a running session".into(),
            ));
        }
        self.native_endpoint_worker_for_session_mut(&session_id)
            .ok_or_else(|| {
                ControlError::InvalidRequest("native endpoint worker is not attached".into())
            })?
            .start()
            .map_err(audio_control_error)
    }

    #[cfg(windows)]
    /// Start the attached physical output fan-out after its graph generation
    /// has been published. A failed fan-out start is rolled back by stopping
    /// any workers that were started by the fan-out itself.
    pub fn start_native_output_fanout(&mut self) -> Result<(), ControlError> {
        let session_id = self.native_output_fanout_session.clone().ok_or_else(|| {
            ControlError::InvalidRequest("native output fan-out is not attached".into())
        })?;
        let generation = self.native_output_fanout_generation.ok_or_else(|| {
            ControlError::InvalidRequest("native output fan-out generation is missing".into())
        })?;
        let runtime = self.runtimes.get(&session_id).ok_or_else(|| {
            ControlError::InvalidRequest("native output fan-out session is not running".into())
        })?;
        if runtime.state() != RuntimeState::Running || runtime.generation() != generation {
            return Err(ControlError::InvalidRequest(
                "native output fan-out requires the matching running session generation".into(),
            ));
        }
        self.native_output_fanout
            .as_mut()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native output fan-out is not attached".into())
            })?
            .start()
            .map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "native output fan-out start failed: {error:?}"
                ))
            })
    }

    #[cfg(windows)]
    /// Stop the output fan-out only after its owning session has stopped.
    pub fn stop_native_output_fanout(&mut self) -> Result<(), ControlError> {
        if let Some(session_id) = self.native_output_fanout_session.as_ref() {
            if self
                .runtimes
                .get(session_id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
            {
                return Err(ControlError::InvalidRequest(
                    "stop the session before stopping its native output fan-out".into(),
                ));
            }
        }
        self.native_output_fanout
            .as_mut()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native output fan-out is not attached".into())
            })?
            .stop()
            .map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "native output fan-out stop failed: {error:?}"
                ))
            })
    }

    #[cfg(windows)]
    /// Detach only a stopped output fan-out and clear its generation binding.
    pub fn detach_native_output_fanout(&mut self) -> Result<(), ControlError> {
        if self
            .native_output_fanout
            .as_ref()
            .is_some_and(|fanout| fanout.is_running())
        {
            return Err(ControlError::InvalidRequest(
                "native output fan-out must be stopped before detachment".into(),
            ));
        }
        if self.native_output_fanout.take().is_none() {
            return Err(ControlError::InvalidRequest(
                "native output fan-out is not attached".into(),
            ));
        }
        self.native_output_fanout_session = None;
        self.native_output_fanout_generation = None;
        self.native_endpoint_taps = None;
        Ok(())
    }

    #[cfg(windows)]
    /// Start an attached duplex bridge only for its exact running session
    /// generation. This does not discover endpoints or change defaults.
    pub fn start_native_duplex_worker(&mut self) -> Result<(), ControlError> {
        let session_id = self.native_duplex_worker_session.clone().ok_or_else(|| {
            ControlError::InvalidRequest("native duplex worker is not attached".into())
        })?;
        let expected_generation = self.native_duplex_worker_generation.ok_or_else(|| {
            ControlError::InvalidRequest("native duplex worker generation is missing".into())
        })?;
        let runtime = self.runtimes.get(&session_id).ok_or_else(|| {
            ControlError::InvalidRequest("native duplex worker session is not running".into())
        })?;
        if runtime.state() != RuntimeState::Running || runtime.generation() != expected_generation {
            return Err(ControlError::InvalidRequest(
                "native duplex worker requires the matching running session generation".into(),
            ));
        }
        self.native_duplex_worker
            .as_mut()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native duplex worker is not attached".into())
            })?
            .start()
            .map_err(|error| {
                ControlError::InvalidRequest(format!("native duplex start failed: {error:?}"))
            })
    }

    #[cfg(windows)]
    /// Start the attached render-source worker only for its exact running
    /// session generation. The worker consumes virtual bridge blocks and
    /// submits them to its already-opened physical render endpoint.
    pub fn start_native_render_source_worker(&mut self) -> Result<(), ControlError> {
        let session_id = self
            .native_render_source_worker_session
            .clone()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native render-source worker is not attached".into())
            })?;
        let expected_generation = self.native_render_source_worker_generation.ok_or_else(|| {
            ControlError::InvalidRequest("native render-source worker generation is missing".into())
        })?;
        let runtime = self.runtimes.get(&session_id).ok_or_else(|| {
            ControlError::InvalidRequest(
                "native render-source worker session is not running".into(),
            )
        })?;
        if runtime.state() != RuntimeState::Running || runtime.generation() != expected_generation {
            return Err(ControlError::InvalidRequest(
                "native render-source worker requires the matching running session generation"
                    .into(),
            ));
        }
        self.native_render_source_worker
            .as_mut()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native render-source worker is not attached".into())
            })?
            .start()
            .map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "native render-source worker start failed: {error:?}"
                ))
            })
    }

    #[cfg(windows)]
    /// Stop the render-source worker before its session is reported stopped.
    pub fn stop_native_render_source_worker(&mut self) -> Result<(), ControlError> {
        if let Some(session_id) = self.native_render_source_worker_session.as_ref() {
            if self
                .runtimes
                .get(session_id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
            {
                return Err(ControlError::InvalidRequest(
                    "stop the session before stopping its native render-source worker".into(),
                ));
            }
        }
        self.native_render_source_worker
            .as_mut()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native render-source worker is not attached".into())
            })?
            .stop()
            .map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "native render-source worker stop failed: {error:?}"
                ))
            })
    }

    #[cfg(windows)]
    /// Detach only a stopped render-source worker and clear its session and
    /// generation binding. Dropping the worker closes the transferred lease.
    pub fn detach_native_render_source_worker(&mut self) -> Result<(), ControlError> {
        if self
            .native_render_source_worker
            .as_ref()
            .is_some_and(|worker| worker.is_running())
        {
            return Err(ControlError::InvalidRequest(
                "native render-source worker must be stopped before detachment".into(),
            ));
        }
        if self.native_render_source_worker.take().is_none() {
            return Err(ControlError::InvalidRequest(
                "native render-source worker is not attached".into(),
            ));
        }
        self.native_render_source_worker_session = None;
        self.native_render_source_worker_generation = None;
        self.native_render_source_taps = None;
        Ok(())
    }

    #[cfg(windows)]
    /// Pump bounded, nonblocking work from the render-source bridge.
    pub fn pump_native_render_source_worker(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        max_quanta: u32,
    ) -> Result<Value, ControlError> {
        if self.native_render_source_worker_session.as_ref() != Some(session_id)
            || self.native_render_source_worker_generation != Some(generation)
        {
            return Err(ControlError::InvalidRequest(
                "native render-source worker is not bound to the requested session generation"
                    .into(),
            ));
        }
        let runtime = self.runtimes.get(session_id).ok_or_else(|| {
            ControlError::InvalidRequest(
                "native render-source worker session is unavailable".into(),
            )
        })?;
        if runtime.state() != RuntimeState::Running || runtime.generation() != generation {
            return Err(ControlError::InvalidRequest(
                "native render-source worker session generation is not running".into(),
            ));
        }
        let taps = self.native_render_source_taps.take().ok_or_else(|| {
            ControlError::InvalidRequest("native render-source graph taps are not prepared".into())
        })?;
        let worker = self.native_render_source_worker.as_mut().ok_or_else(|| {
            ControlError::InvalidRequest("native render-source worker is not attached".into())
        })?;
        let result = worker
            .pump_available_with_taps(max_quanta, &taps)
            .map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "native render-source worker pump failed: {error:?}"
                ))
            });
        self.native_render_source_taps = Some(taps);
        let pump = result?;
        Ok(json!({
            "sessionId": session_id,
            "generation": generation,
            "packets": pump.packets,
            "processedQuanta": pump.processed_quanta,
            "renderedFrames": pump.rendered_frames,
            "droppedRenderFrames": pump.dropped_render_frames,
        }))
    }

    #[cfg(windows)]
    /// Detach only a stopped multi-input worker. Its feeder generation is
    /// intentionally not rebound or silently advanced by detachment.
    pub fn detach_native_multi_input_worker(&mut self) -> Result<(), ControlError> {
        if self
            .native_multi_input_worker
            .as_ref()
            .is_some_and(|worker| worker.is_running())
        {
            return Err(ControlError::InvalidRequest(
                "native multi-input worker must be stopped before detachment".into(),
            ));
        }
        if self.native_multi_input_worker.take().is_none() {
            return Err(ControlError::InvalidRequest(
                "native multi-input worker is not attached".into(),
            ));
        }
        self.native_multi_input_worker_session = None;
        self.native_multi_input_worker_generation = None;
        self.native_multi_input_applied_generation = None;
        #[cfg(windows)]
        self.multi_input_application_sources.clear();
        Ok(())
    }

    #[cfg(windows)]
    /// Stop the duplex bridge before its session is reported stopped.
    pub fn stop_native_duplex_worker(&mut self) -> Result<(), ControlError> {
        if let Some(session_id) = self.native_duplex_worker_session.as_ref() {
            if self
                .runtimes
                .get(session_id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
            {
                return Err(ControlError::InvalidRequest(
                    "stop the session before stopping its native duplex worker".into(),
                ));
            }
        }
        self.native_duplex_worker
            .as_mut()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native duplex worker is not attached".into())
            })?
            .stop()
            .map_err(|error| {
                ControlError::InvalidRequest(format!("native duplex stop failed: {error:?}"))
            })
    }

    #[cfg(windows)]
    /// Detach only a stopped duplex worker from its session.
    pub fn detach_native_duplex_worker(&mut self) -> Result<(), ControlError> {
        if self
            .native_duplex_worker
            .as_ref()
            .is_some_and(|worker| worker.is_running())
        {
            return Err(ControlError::InvalidRequest(
                "native duplex worker must be stopped before detachment".into(),
            ));
        }
        if self.native_duplex_worker.take().is_none() {
            return Err(ControlError::InvalidRequest(
                "native duplex worker is not attached".into(),
            ));
        }
        self.native_duplex_worker_session = None;
        self.native_duplex_worker_generation = None;
        Ok(())
    }

    #[cfg(windows)]
    /// Pump bounded work from both directions without waiting or implicit
    /// recovery. The caller owns event wakeups and chooses both budgets.
    pub fn pump_native_duplex_worker(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        max_input_quanta: u32,
        max_output_packets: u32,
    ) -> Result<Value, ControlError> {
        if self.native_duplex_worker_session.as_ref() != Some(session_id)
            || self.native_duplex_worker_generation != Some(generation)
        {
            return Err(ControlError::InvalidRequest(
                "native duplex worker is not bound to the requested session generation".into(),
            ));
        }
        let runtime = self.runtimes.get(session_id).ok_or_else(|| {
            ControlError::InvalidRequest("native duplex worker session is unavailable".into())
        })?;
        if runtime.state() != RuntimeState::Running || runtime.generation() != generation {
            return Err(ControlError::InvalidRequest(
                "native duplex worker session generation is not running".into(),
            ));
        }
        let heartbeat = {
            let worker = self.native_duplex_worker.as_mut().ok_or_else(|| {
                ControlError::InvalidRequest("native duplex worker is not attached".into())
            })?;
            let bus_id = worker.bus_id();
            worker.heartbeat_if_due().map_err(|error| (bus_id, error))
        };
        if let Err((bus_id, error)) = heartbeat {
            if let Some(bridge) = self.virtual_bridges.get(&bus_id) {
                bridge.deactivate();
            }
            self.publish_virtual_bridge_failure(&bus_id);
            return Err(ControlError::InvalidRequest(format!(
                "native duplex heartbeat failed; bridge deactivated: {error:?}"
            )));
        }
        let worker = self.native_duplex_worker.as_mut().ok_or_else(|| {
            ControlError::InvalidRequest("native duplex worker is not attached".into())
        })?;
        let (input, output) = worker
            .pump_available(max_input_quanta, max_output_packets)
            .map_err(|error| {
                ControlError::InvalidRequest(format!("native duplex pump failed: {error:?}"))
            })?;
        Ok(json!({
            "sessionId": session_id,
            "generation": generation,
            "input": {
                "packets": input.packets,
                "capturedFrames": input.captured_frames,
                "processedQuanta": input.processed_quanta,
                "renderedFrames": input.rendered_frames,
                "droppedRenderFrames": input.dropped_render_frames,
                "renderBackpressureEvents": input.render_backpressure_events,
            },
            "output": {
                "packets": output.packets,
                "capturedFrames": output.captured_frames,
                "processedQuanta": output.processed_quanta,
                "renderedFrames": output.rendered_frames,
                "droppedRenderFrames": output.dropped_render_frames,
                "renderBackpressureEvents": output.render_backpressure_events,
            },
        }))
    }

    /// Stop the explicitly attached endpoint pair and clear staged bridge
    /// audio. The worker remains attached and may be deliberately restarted.
    pub fn stop_native_endpoint_worker(&mut self) -> Result<(), ControlError> {
        #[cfg(windows)]
        if self
            .native_output_fanout
            .as_ref()
            .is_some_and(|fanout| fanout.is_running())
        {
            return Err(ControlError::InvalidRequest(
                "stop the native output fan-out before stopping the endpoint worker".into(),
            ));
        }
        if let Some(session_id) = self.native_endpoint_session.as_ref() {
            if self
                .runtimes
                .get(session_id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
            {
                return Err(ControlError::InvalidRequest(
                    "stop the session before stopping its native endpoint worker".into(),
                ));
            }
        }
        self.native_endpoint_worker
            .as_mut()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native endpoint worker is not attached".into())
            })?
            .stop()
            .map_err(audio_control_error)
    }

    /// Pump already-available endpoint packets through a caller-owned graph
    /// tap for the exact running session generation. The bounded Windows
    /// worker performs no wait or implicit recovery here; endpoint wakeup and
    /// graph publication remain owned by the native scheduler.
    pub fn pump_native_endpoint_worker(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        max_packets: u32,
        tap: &dyn AudioTap,
    ) -> Result<Value, ControlError> {
        if !self.native_endpoint_session_is_attached(session_id) {
            self.native_endpoint_rejections = self.native_endpoint_rejections.saturating_add(1);
            return Err(ControlError::InvalidRequest(
                "native endpoint worker is not bound to the session".into(),
            ));
        }
        let runtime_generation = self
            .runtimes
            .get(session_id)
            .filter(|runtime| runtime.state() == RuntimeState::Running)
            .map(FakeRuntime::generation)
            .ok_or_else(|| ControlError::InvalidRequest("session runtime is not running".into()))?;
        if runtime_generation != generation {
            self.native_endpoint_rejections = self.native_endpoint_rejections.saturating_add(1);
            return Err(ControlError::InvalidRequest(
                "native endpoint worker generation is stale".into(),
            ));
        }
        if self.poll_native_endpoint_lifecycle()? {
            self.native_endpoint_rejections = self.native_endpoint_rejections.saturating_add(1);
            return Err(ControlError::InvalidRequest(
                "native endpoint worker binding was invalidated; rebind before pumping".into(),
            ));
        }
        #[cfg(windows)]
        if self.maintain_application_capture(session_id, false)? {
            let recorder_chunks_drained = self.drain_attached_recorders()?;
            return Ok(json!({
                "sessionId": session_id,
                "generation": generation,
                "packets": 0,
                "capturedFrames": 0,
                "processedQuanta": 0,
                "renderedFrames": 0,
                "droppedRenderFrames": 0,
                "renderBackpressureEvents": 0,
                "recorderChunksDrained": recorder_chunks_drained,
            }));
        }
        let pump = {
            let worker = self
                .native_endpoint_worker_for_session_mut(session_id)
                .ok_or_else(|| {
                    ControlError::InvalidRequest("native endpoint worker is not attached".into())
                })?;
            if worker.bridge().scheduler().telemetry().active_generation
                != Some(RuntimeGeneration::new(generation))
            {
                self.native_endpoint_rejections = self.native_endpoint_rejections.saturating_add(1);
                return Err(ControlError::InvalidRequest(
                    "native endpoint worker has no matching prepared graph".into(),
                ));
            }
            worker
                .pump_available_with_tap(max_packets, tap)
                .map_err(audio_control_error)?
        };
        let recorder_chunks_drained = self.drain_attached_recorders()?;
        Ok(json!({
            "sessionId": session_id,
            "generation": generation,
            "packets": pump.packets,
            "capturedFrames": pump.captured_frames,
            "processedQuanta": pump.processed_quanta,
            "renderedFrames": pump.rendered_frames,
            "droppedRenderFrames": pump.dropped_render_frames,
            "renderBackpressureEvents": pump.render_backpressure_events,
            "recorderChunksDrained": recorder_chunks_drained,
        }))
    }

    /// Compile and publish the validated session graph into the attached
    /// native scheduler. Graph preparation happens on the control thread and
    /// replaces the scheduler generation atomically; endpoint start remains a
    /// separate explicit operation.
    pub fn activate_native_graph(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        sample_rate_hz: u32,
    ) -> Result<(), ControlError> {
        self.activate_native_graph_candidate(session_id, generation, sample_rate_hz, None)
    }

    pub(crate) fn activate_native_graph_candidate(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        sample_rate_hz: u32,
        candidate: Option<&Session>,
    ) -> Result<(), ControlError> {
        self.activate_native_graph_candidate_with_flags(
            session_id,
            generation,
            sample_rate_hz,
            candidate,
            false,
        )
    }

    pub(crate) fn activate_native_graph_candidate_with_flags(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        sample_rate_hz: u32,
        candidate: Option<&Session>,
        flags_only: bool,
    ) -> Result<(), ControlError> {
        if !self.native_endpoint_session_is_attached(session_id) {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker is not attached".into(),
            ));
        }
        let runtime_generation = self
            .runtimes
            .get(session_id)
            .filter(|runtime| runtime.state() == RuntimeState::Running)
            .map(FakeRuntime::generation)
            .ok_or_else(|| ControlError::InvalidRequest("session runtime is not running".into()))?;
        if runtime_generation != generation {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker generation is stale".into(),
            ));
        }
        let mut session = candidate
            .cloned()
            .unwrap_or_else(|| self.get_session(session_id).expect("loaded above").clone());
        session = audiorouter_engine::prune_unfed_upstream(&session).into_owned();
        let recorder_node_ids = session
            .nodes
            .iter()
            .filter(|node| node.enabled && node.kind == NodeKind::Recorder)
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        let mut recorder_taps = if recorder_node_ids.is_empty() {
            AudioTapSet::new()
        } else {
            let bindings =
                self.recorder_tap_bindings(session_id, RuntimeGeneration::new(generation))?;
            bindings
                .tap_set_for_generation(RuntimeGeneration::new(generation), &recorder_node_ids)
                .map_err(|_| {
                    ControlError::InvalidRequest("recorder graph tap binding is invalid".into())
                })?
        };
        let mut bridge_flags = Vec::new();
        let plugin_stages = if flags_only {
            let mut stages: HashMap<EntityId, Arc<dyn RealtimePluginProcessor>> = HashMap::new();
            for node in session
                .nodes
                .iter_mut()
                .filter(|node| node.kind == NodeKind::Plugin)
            {
                let bridge = self.plugin_bridge(session_id, &node.id)?;
                bridge_flags.push((Arc::clone(&bridge), node.enabled && !node.bypass));
                stages.insert(node.id.clone(), bridge);
                node.enabled = true;
                node.bypass = false;
            }
            stages
        } else {
            self.prepare_plugin_stages(&session, sample_rate_hz)?
        };
        let graph = self.compile_session_graph_with_audio(
            &session,
            RuntimeGeneration::new(generation),
            sample_rate_hz,
            &plugin_stages,
        )?;

        let capture_bus_ids = session_virtual_capture_bus_ids(&session);
        self.prepare_virtual_route_bridges(session_id, generation, &capture_bus_ids)?;
        let virtual_taps = self.virtual_route_tap_set(session_id, &capture_bus_ids, generation)?;
        recorder_taps.append(&virtual_taps).map_err(|_| {
            ControlError::InvalidRequest("native graph tap capacity exceeded".into())
        })?;

        #[cfg(windows)]
        if let Some(fanout) = self.native_output_fanout.as_ref() {
            if self.native_output_fanout_session.as_ref() != Some(session_id)
                || self.native_output_fanout_generation != Some(generation)
            {
                return Err(ControlError::InvalidRequest(
                    "native output fan-out binding is stale for the graph generation".into(),
                ));
            }
            recorder_taps.append(fanout.tap_set()).map_err(|_| {
                ControlError::InvalidRequest(
                    "native output fan-out exceeds graph tap capacity".into(),
                )
            })?;
        }

        self.native_endpoint_worker_for_session_mut(session_id)
            .ok_or_else(|| {
                ControlError::InvalidRequest("native endpoint worker is not attached".into())
            })?
            .bridge_mut()
            .scheduler_mut()
            .publish(graph);
        let taps = self
            .native_endpoint_taps_for_session_mut(session_id)
            .ok_or_else(|| {
                ControlError::InvalidRequest("native endpoint worker is not attached".into())
            })?;
        *taps = Some(recorder_taps);
        for (bridge, active) in bridge_flags {
            bridge.set_processing_active(active);
        }
        Ok(())
    }

    #[cfg(windows)]
    /// Compile and publish the same authoritative graph for a standalone
    /// virtual render-source worker. This worker has no physical capture
    /// side, so it must receive its graph explicitly instead of appearing
    /// healthy with an unprepared scheduler.
    pub fn activate_native_render_source_graph(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        sample_rate_hz: u32,
    ) -> Result<(), ControlError> {
        if self.native_render_source_worker_session.as_ref() != Some(session_id)
            || self.native_render_source_worker.is_none()
        {
            return Err(ControlError::InvalidRequest(
                "native render-source worker is not bound to the session".into(),
            ));
        }
        let runtime_generation = self
            .runtimes
            .get(session_id)
            .filter(|runtime| runtime.state() == RuntimeState::Running)
            .map(FakeRuntime::generation)
            .ok_or_else(|| ControlError::InvalidRequest("session runtime is not running".into()))?;
        if runtime_generation != generation {
            return Err(ControlError::InvalidRequest(
                "native render-source worker generation is stale".into(),
            ));
        }
        let session = self.get_session(session_id)?.clone();
        let plugin_stages = self.prepare_plugin_stages(&session, sample_rate_hz)?;
        let graph = self.compile_session_graph_with_audio(
            &session,
            RuntimeGeneration::new(generation),
            sample_rate_hz,
            &plugin_stages,
        )?;
        let recorder_node_ids = session
            .nodes
            .iter()
            .filter(|node| node.enabled && node.kind == NodeKind::Recorder)
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        let mut taps = if recorder_node_ids.is_empty() {
            AudioTapSet::new()
        } else {
            let bindings =
                self.recorder_tap_bindings(session_id, RuntimeGeneration::new(generation))?;
            bindings
                .tap_set_for_generation(RuntimeGeneration::new(generation), &recorder_node_ids)
                .map_err(|_| {
                    ControlError::InvalidRequest("recorder graph tap binding is invalid".into())
                })?
        };
        let capture_bus_ids = session_virtual_capture_bus_ids(&session);
        self.prepare_virtual_route_bridges(session_id, generation, &capture_bus_ids)?;
        let virtual_taps = self.virtual_route_tap_set(session_id, &capture_bus_ids, generation)?;
        taps.append(&virtual_taps).map_err(|_| {
            ControlError::InvalidRequest("native render-source tap capacity exceeded".into())
        })?;
        self.native_render_source_worker
            .as_mut()
            .expect("worker presence checked above")
            .publish_graph(graph);
        self.native_render_source_taps = Some(taps);
        Ok(())
    }

    /// Make bridge ownership follow the exact native graph generation. This
    /// is control-plane work performed before publication; realtime taps only
    /// observe the already-selected, already-activated bridge objects.
    pub(crate) fn prepare_virtual_route_bridges(
        &mut self,
        producer_session_id: &EntityId,
        generation: u64,
        capture_bus_ids: &[EntityId],
    ) -> Result<(), ControlError> {
        let route_bus_ids = self
            .virtual_bus_routes
            .list()
            .iter()
            .filter(|route| route.producer_session_id == *producer_session_id)
            .map(|route| route.bus_id.clone())
            .collect::<Vec<_>>();
        let mut selected = Vec::new();
        for bus_id in &route_bus_ids {
            let enabled = self
                .virtual_buses
                .list()
                .iter()
                .any(|bus| bus.id() == bus_id && bus.enabled());
            if capture_bus_ids
                .iter()
                .any(|selected_bus_id| selected_bus_id == bus_id)
                && enabled
            {
                let bridge = self.virtual_bridges.get(bus_id).ok_or_else(|| {
                    ControlError::InvalidRequest("route bridge is not prepared".into())
                })?;
                if bridge.is_active() && bridge.generation() == generation {
                    selected.push(bus_id.clone());
                } else {
                    if generation <= bridge.generation() {
                        return Err(ControlError::InvalidRequest(
                            "native graph generation is stale for virtual route".into(),
                        ));
                    }
                    selected.push(bus_id.clone());
                }
            }
        }

        for bus_id in route_bus_ids {
            let Some(bridge) = self.virtual_bridges.get(&bus_id) else {
                continue;
            };
            if selected.iter().any(|selected_id| selected_id == &bus_id) {
                if !bridge.is_active() || bridge.generation() != generation {
                    bridge.activate(generation).map_err(|_| {
                        ControlError::InvalidRequest(
                            "virtual route bridge generation activation failed".into(),
                        )
                    })?;
                }
            } else if bridge.is_active() {
                bridge.deactivate();
            }
        }
        Ok(())
    }

    pub(crate) fn deactivate_virtual_route_bridges(&self, producer_session_id: &EntityId) {
        for route in self
            .virtual_bus_routes
            .list()
            .iter()
            .filter(|route| route.producer_session_id == *producer_session_id)
        {
            if let Some(bridge) = self.virtual_bridges.get(&route.bus_id) {
                bridge.deactivate();
            }
        }
    }

    pub(crate) fn virtual_route_tap_set(
        &self,
        producer_session_id: &EntityId,
        capture_bus_ids: &[EntityId],
        generation: u64,
    ) -> Result<AudioTapSet, ControlError> {
        let mut taps = AudioTapSet::new();
        if capture_bus_ids.is_empty() {
            return Ok(taps);
        }
        for route in self
            .virtual_bus_routes
            .list()
            .iter()
            .filter(|route| route.producer_session_id == *producer_session_id)
        {
            if !capture_bus_ids.iter().any(|bus_id| bus_id == &route.bus_id) {
                continue;
            }
            let enabled = self
                .virtual_buses
                .list()
                .iter()
                .any(|bus| bus.id() == &route.bus_id && bus.enabled());
            if enabled {
                #[cfg(windows)]
                if let Some(binding) = self.native_duplex_bindings.get(&route.bus_id) {
                    if binding.generation() != generation {
                        return Err(ControlError::InvalidRequest(
                            "native duplex binding generation is stale".into(),
                        ));
                    }
                    taps.add_shared(binding.capture_writer()).map_err(|_| {
                        ControlError::InvalidRequest(
                            "native duplex capture tap capacity exceeded".into(),
                        )
                    })?;
                    continue;
                }
                #[cfg(windows)]
                if let Some(binding) = self.native_capture_sink_bindings.get(&route.bus_id) {
                    if binding.generation() != generation {
                        return Err(ControlError::InvalidRequest(
                            "native capture sink binding generation is stale".into(),
                        ));
                    }
                    taps.add_shared(binding.writer()).map_err(|_| {
                        ControlError::InvalidRequest(
                            "native capture sink tap capacity exceeded".into(),
                        )
                    })?;
                    continue;
                }
                let bridge = self.virtual_bridges.get(&route.bus_id).ok_or_else(|| {
                    ControlError::InvalidRequest("route bridge is not prepared".into())
                })?;
                taps.add_shared(bridge).map_err(|_| {
                    ControlError::InvalidRequest("native graph tap capacity exceeded".into())
                })?;
            }
        }
        Ok(taps)
    }

    /// Pump using the recorder taps prepared by the last native graph
    /// activation. Taking the set for the duration of the call keeps the
    /// callback borrow explicit while avoiding construction or allocation in
    /// the packet path.
    pub fn pump_native_endpoint_worker_with_bound_taps(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        max_packets: u32,
    ) -> Result<Value, ControlError> {
        #[cfg(windows)]
        self.heartbeat_native_capture_sink_bindings()?;
        let Some(taps_slot) = self.native_endpoint_taps_for_session_mut(session_id) else {
            self.native_endpoint_rejections = self.native_endpoint_rejections.saturating_add(1);
            return Err(ControlError::InvalidRequest(
                "native graph recorder taps are not prepared".into(),
            ));
        };
        let taps = taps_slot.take().ok_or_else(|| {
            self.native_endpoint_rejections = self.native_endpoint_rejections.saturating_add(1);
            ControlError::InvalidRequest("native graph recorder taps are not prepared".into())
        })?;
        let result = self.pump_native_endpoint_worker(session_id, generation, max_packets, &taps);
        if let Some(taps_slot) = self.native_endpoint_taps_for_session_mut(session_id) {
            *taps_slot = Some(taps);
        }
        let primary = result?;
        #[cfg(windows)]
        let output_fanout = if let Some(fanout) = self.native_output_fanout.as_mut() {
            if self.native_output_fanout_session.as_ref() != Some(session_id)
                || self.native_output_fanout_generation != Some(generation)
            {
                return Err(ControlError::InvalidRequest(
                    "native output fan-out binding is stale for the requested generation".into(),
                ));
            }
            Some(fanout.pump_available(max_packets).map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "native output fan-out pump failed: {error:?}"
                ))
            })?)
        } else {
            None
        };
        let mut response = json!({
            "sessionId": session_id,
            "generation": generation,
            "packets": primary.get("packets").cloned().unwrap_or(Value::Null),
            "capturedFrames": primary.get("capturedFrames").cloned().unwrap_or(Value::Null),
            "processedQuanta": primary.get("processedQuanta").cloned().unwrap_or(Value::Null),
            "renderedFrames": primary.get("renderedFrames").cloned().unwrap_or(Value::Null),
            "droppedRenderFrames": primary.get("droppedRenderFrames").cloned().unwrap_or(Value::Null),
            "renderBackpressureEvents": primary.get("renderBackpressureEvents").cloned().unwrap_or(Value::Null),
            "recorderChunksDrained": primary.get("recorderChunksDrained").cloned().unwrap_or(Value::Null),
        });
        #[cfg(windows)]
        if let Some(output) = output_fanout {
            response["outputFanout"] = json!({
                "packets": output.packets,
                "renderedFrames": output.rendered_frames,
                "renderBackpressureEvents": output.render_backpressure_events,
            });
        }
        Ok(response)
    }

    /// Return bounded native endpoint lifecycle and control rejection counts
    /// without touching endpoint state.
    pub fn native_endpoint_lifecycle_telemetry(&self) -> Value {
        let worker = self
            .native_endpoint_worker
            .as_ref()
            .map(audiorouter_windows_audio::NativeAudioWorker::telemetry)
            .unwrap_or_default();
        json!({
            "startAttempts": worker.start_attempts,
            "successfulStarts": worker.successful_starts,
            "stopAttempts": worker.stop_attempts,
            "successfulStops": worker.successful_stops,
            "resetSuccesses": worker.reset_successes,
            "rejectedPumps": self.native_endpoint_rejections,
        })
    }

    /// Return the lock-free native scheduler counters for diagnostics. This
    /// is a control-thread read; the realtime path only updates atomics.
    /// `null` is deliberate when no worker is attached, so an unavailable
    /// adapter is never represented as a healthy zeroed scheduler.
    pub(crate) fn native_scheduler_telemetry(&self) -> Value {
        let Some(worker) = self.native_endpoint_worker.as_ref() else {
            return Value::Null;
        };
        let scheduler = worker.bridge().scheduler();
        let telemetry = scheduler.telemetry();
        json!({
            "activeGeneration": telemetry.active_generation.map(|generation| generation.value()),
            "activeSampleRateHz": scheduler.active_sample_rate_hz(),
            "inputOverruns": telemetry.input_overruns,
            "inputUnderruns": telemetry.input_underruns,
            "outputOverruns": telemetry.output_overruns,
            "outputUnderruns": telemetry.output_underruns,
            "processedQuanta": telemetry.processed_quanta,
            "repairedSamples": telemetry.repaired_samples,
            "xruns": telemetry.xruns,
            "processingTimeNsTotal": telemetry.processing_time_ns_total,
            "processingTimeNsMax": telemetry.processing_time_ns_max,
            "deadlineMisses": telemetry.deadline_misses,
            "deadlineLatenessNsTotal": telemetry.deadline_lateness_ns_total,
            "deadlineLatenessNsMax": telemetry.deadline_lateness_ns_max,
        })
    }

    /// Return bounded node-keyed observations from the attached prepared
    /// graph. This walks the immutable committed session only on the
    /// diagnostics thread; processor reads remain best-effort and lock-free.
    pub(crate) fn native_node_telemetry(&self) -> Value {
        let session_id = self
            .application_capture_runtime
            .as_ref()
            .map(|binding| &binding.session_id)
            .or(self.native_endpoint_session.as_ref())
            .or(self.native_endpoint_session_secondary.as_ref());
        let Some(session_id) = session_id else {
            return json!([]);
        };
        let Some(worker) = self.native_endpoint_worker_for_session(session_id) else {
            return json!([]);
        };
        let Some(session) = self.store.session(session_id) else {
            return json!([]);
        };
        let processor = worker.bridge().scheduler().processor();
        session
            .nodes
            .iter()
            .filter_map(|node| {
                let meter = processor.meter_snapshot_for_node(&node.id).map(|snapshot| {
                    json!({
                        "peakDb": snapshot.peak_db,
                        "currentPeakDb": snapshot.current_peak_db,
                        "channelCurrentPeakDb": snapshot.channel_current_peak_db,
                        "observedFrames": snapshot.observed_frames,
                        "sampleRateHz": audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
                        "rmsDb": snapshot.rms_db,
                        "clippedSamples": snapshot.clipped_samples,
                        "channelPeakDb": snapshot.channel_peak_db,
                        "channelRmsDb": snapshot.channel_rms_db,
                        "channelClippedSamples": snapshot.channel_clipped_samples,
                    })
                });
                let processor_telemetry =
                    processor
                        .processor_telemetry_for_node(&node.id)
                        .map(|telemetry| {
                            json!({
                                "gainReductionDb": telemetry.gain_reduction_db,
                                "inputLevelDb": telemetry.input_level_db,
                                "outputLevelDb": telemetry.output_level_db,
                                "gateOpen": telemetry.gate_open,
                            })
                        });
                let plugin_health = processor.plugin_health_for_node(&node.id).map(|health| {
                    json!({
                        "state": match health.state {
                            audiorouter_engine::PluginWorkerState::Unknown => "unknown",
                            audiorouter_engine::PluginWorkerState::Stopped => "stopped",
                            audiorouter_engine::PluginWorkerState::Running => "running",
                            audiorouter_engine::PluginWorkerState::Failed => "failed",
                            audiorouter_engine::PluginWorkerState::Quarantined => "quarantined",
                        },
                        "failureCount": health.failure_count,
                        "outputMisses": self.plugin_bridge(session_id, &node.id).ok().map(|bridge| bridge.continuity_counts().0).unwrap_or(0),
                        "inputDrops": self.plugin_bridge(session_id, &node.id).ok().map(|bridge| bridge.continuity_counts().1).unwrap_or(0),
                    })
                });
                let noise_profile = processor.noise_profile_for_node(&node.id);
                let spectrum = processor.spectrum_levels_for_node(&node.id);
                if meter.is_none() && processor_telemetry.is_none() && plugin_health.is_none() && noise_profile.is_none() && spectrum.is_none() {
                    return None;
                }
                let mut item = json!({
                    "nodeId": node.id,
                    "kind": node.kind.type_name(),
                    "meter": meter,
                    "processor": processor_telemetry,
                    "plugin": plugin_health,
                });
                if let Some(profile) = noise_profile {
                    item["noiseProfile"] = json!(profile);
                }
                if let Some(levels) = spectrum {
                    item["spectrum"] = spectrum_telemetry(&levels);
                }
                Some(item)
            })
            .collect::<Vec<_>>()
            .into()
    }

    /// Same best-effort per-node telemetry as `native_node_telemetry`, but
    /// for the separate multi-input mixer/fan-out worker, which only has
    /// processor/plugin telemetry and actual prepared input/tool/output levels.
    pub(crate) fn native_multi_input_node_telemetry(&self) -> Value {
        let (Some(worker), Some(session_id)) = (
            self.native_multi_input_worker.as_ref(),
            self.native_multi_input_worker_session.as_ref(),
        ) else {
            return json!([]);
        };
        let Some(session) = self.store.session(session_id) else {
            return json!([]);
        };
        let input_waits = worker.input_wait_ms();
        let output_queues = worker.output_queue_ms();
        let registry = audiorouter_domain::node_registry();
        let rate_ms = f64::from(audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ) / 1_000.0;
        // Where the signal spends time (UI signal timing): a source's audio
        // waits before pickup, a tool adds its own delay (fixed, a Delay's
        // setting, or a plugin's worker pipeline) and processing time, and an
        // output queues audio ahead of the device.
        let timing_for = |node: &audiorouter_domain::Node| -> Option<Value> {
            if let Some(index) = worker.input_node_ids().iter().position(|id| *id == node.id) {
                return input_waits
                    .get(index)
                    .copied()
                    .flatten()
                    .map(|wait| json!({ "delayMs": wait }));
            }
            if let Some(index) = worker
                .output_node_ids()
                .iter()
                .position(|id| *id == node.id)
            {
                return output_queues
                    .get(index)
                    .copied()
                    .flatten()
                    .map(|queue| json!({ "delayMs": queue }));
            }
            let timing = worker.stage_timing_for_node(&node.id)?;
            let fixed = registry
                .iter()
                .find(|spec| spec.kind == node.kind)
                .map_or(0.0, |spec| f64::from(spec.latency_samples) / rate_ms);
            let setting = if node.kind == NodeKind::Delay {
                node.parameters
                    .get("delayMs")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            let plugin = f64::from(timing.plugin_latency_samples.unwrap_or(0)) / rate_ms;
            let quanta = timing.processed_quanta.max(1) as f64;
            Some(json!({
                "delayMs": fixed + setting + plugin,
                "processingUsAvg": timing.processing_ns_total as f64 / quanta / 1_000.0,
                "processingUsMax": timing.processing_ns_max as f64 / 1_000.0,
            }))
        };
        session
            .nodes
            .iter()
            .filter_map(|node| {
                let timing = timing_for(node);
                let meter = worker.meter_snapshot_for_node(&node.id).map(|snapshot| {
                    json!({
                        "peakDb": snapshot.peak_db,
                        "currentPeakDb": snapshot.current_peak_db,
                        "channelCurrentPeakDb": snapshot.channel_current_peak_db,
                        "observedFrames": snapshot.observed_frames,
                        "sampleRateHz": audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
                        "rmsDb": snapshot.rms_db,
                        "clippedSamples": snapshot.clipped_samples,
                        "channelPeakDb": snapshot.channel_peak_db,
                        "channelRmsDb": snapshot.channel_rms_db,
                        "channelClippedSamples": snapshot.channel_clipped_samples,
                    })
                });
                let processor_telemetry =
                    worker
                        .processor_telemetry_for_node(&node.id)
                        .map(|telemetry| {
                            json!({
                                "gainReductionDb": telemetry.gain_reduction_db,
                                "inputLevelDb": telemetry.input_level_db,
                                "outputLevelDb": telemetry.output_level_db,
                                "gateOpen": telemetry.gate_open,
                            })
                        });
                let plugin_health = worker.plugin_health_for_node(&node.id).map(|health| {
                    json!({
                        "state": match health.state {
                            audiorouter_engine::PluginWorkerState::Unknown => "unknown",
                            audiorouter_engine::PluginWorkerState::Stopped => "stopped",
                            audiorouter_engine::PluginWorkerState::Running => "running",
                            audiorouter_engine::PluginWorkerState::Failed => "failed",
                            audiorouter_engine::PluginWorkerState::Quarantined => "quarantined",
                        },
                        "failureCount": health.failure_count,
                        "outputMisses": self.plugin_bridge(session_id, &node.id).ok().map(|bridge| bridge.continuity_counts().0).unwrap_or(0),
                        "inputDrops": self.plugin_bridge(session_id, &node.id).ok().map(|bridge| bridge.continuity_counts().1).unwrap_or(0),
                    })
                });
                let network = worker
                    .input_node_ids()
                    .iter()
                    .position(|id| *id == node.id)
                    .and_then(|index| worker.network_receive_stats(index))
                    .map(|stats| {
                        let mut telemetry = json!({
                            "direction": "receive",
                            "receivedPackets": stats.received_packets,
                            "lostPackets": stats.lost_packets,
                            "latePackets": stats.late_packets,
                            "rejectedDatagrams": stats.rejected_datagrams,
                            "underruns": stats.underruns,
                            "overflowPackets": stats.overflow_packets,
                            "bufferedMs": stats.buffered_frames as f64 * 1_000.0
                                / f64::from(audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ),
                            "paired": stats.paired,
                            "authFailures": stats.auth_failures,
                            "replayedPackets": stats.replayed_packets,
                        });
                        if let Some(failure) = stats.last_auth_failure {
                            telemetry["authProblem"] = json!(failure.as_str());
                        }
                        if let Some(address) = stats.last_rejected_sender {
                            telemetry["rejectedFrom"] = json!(address.to_string());
                        }
                        // The address the sending computer must target.
                        if let Some(address) = stats.local_address_toward_sender {
                            telemetry["thisAddress"] = json!(address.to_string());
                        }
                        telemetry
                    })
                    .or_else(|| {
                        let index = worker.output_node_ids().iter().position(|id| *id == node.id)?;
                        worker.network_send_stats(index).map(|stats| {
                            let mut telemetry = json!({
                                "direction": "send",
                                "sentPackets": stats.sent_packets,
                                "droppedPackets": stats.dropped_packets,
                                "sendErrors": stats.send_errors,
                                "paired": stats.paired,
                            });
                            // The address the receiver must accept.
                            if let Some(address) = stats.local_address {
                                telemetry["localAddress"] = json!(address.ip().to_string());
                            }
                            if let Some(code) = stats.last_error_code {
                                telemetry["lastErrorCode"] = json!(code);
                            }
                            telemetry
                        })
                    });
                let noise_profile = worker.noise_profile_for_node(&node.id);
                let spectrum = worker.spectrum_levels_for_node(&node.id);
                if processor_telemetry.is_none()
                    && plugin_health.is_none()
                    && noise_profile.is_none()
                    && spectrum.is_none()
                    && timing.is_none()
                    && meter.is_none()
                    && network.is_none()
                {
                    return None;
                }
                let mut item = json!({
                    "nodeId": node.id,
                    "kind": node.kind.type_name(),
                    "meter": meter,
                    "processor": processor_telemetry,
                    "plugin": plugin_health,
                });
                if let Some(network) = network {
                    item["network"] = network;
                }
                if let Some(profile) = noise_profile {
                    item["noiseProfile"] = json!(profile);
                }
                if let Some(levels) = spectrum {
                    item["spectrum"] = spectrum_telemetry(&levels);
                }
                if let Some(timing) = timing {
                    item["timing"] = timing;
                }
                Some(item)
            })
            .collect::<Vec<_>>()
            .into()
    }

    /// Detach only a stopped native worker; this never affects unrelated
    /// endpoints or machine audio configuration.
    pub fn detach_native_endpoint_worker(&mut self) -> Result<(), ControlError> {
        let session_id = self
            .native_endpoint_session
            .clone()
            .or_else(|| self.native_endpoint_session_secondary.clone())
            .ok_or_else(|| {
                ControlError::InvalidRequest("native endpoint worker is not attached".into())
            })?;
        self.detach_native_endpoint_worker_for_session(&session_id)
    }

    pub(crate) fn detach_native_endpoint_worker_for_session(
        &mut self,
        session_id: &EntityId,
    ) -> Result<(), ControlError> {
        #[cfg(windows)]
        if self.native_output_fanout_session.as_ref() == Some(session_id)
            && self.native_output_fanout.is_some()
        {
            return Err(ControlError::InvalidRequest(
                "detach the native output fan-out before detaching the endpoint worker".into(),
            ));
        }
        if self
            .runtimes
            .get(session_id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
        {
            return Err(ControlError::InvalidRequest(
                "stop the session before detaching its native endpoint worker".into(),
            ));
        }
        if !self.native_endpoint_session_is_attached(session_id) {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker is not attached to the session".into(),
            ));
        }
        if self
            .native_endpoint_worker_for_session(session_id)
            .is_some_and(audiorouter_windows_audio::NativeAudioWorker::is_running)
        {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker must be stopped before detachment".into(),
            ));
        }
        let Some(worker_slot) = self.native_endpoint_worker_slot_for_session_mut(session_id) else {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker is not attached".into(),
            ));
        };
        worker_slot.take();
        if self.native_endpoint_session.as_ref() == Some(session_id) {
            self.native_endpoint_session = None;
            self.native_endpoint_taps = None;
        } else {
            self.native_endpoint_session_secondary = None;
            self.native_endpoint_taps_secondary = None;
        }
        if self
            .application_capture_runtime
            .as_ref()
            .is_some_and(|binding| &binding.session_id == session_id)
        {
            self.application_capture_runtime = None;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "native_workers_tests.rs"]
mod tests;
