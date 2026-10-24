//! The backend's audio service pass and the multi-input pumps.

use super::*;

/// Run `work` on a scoped helper thread and call `service` on this thread
/// every [`AUDIO_SERVICE_PASS_DURING_WORK`] until it finishes. A panic in
/// `work` resumes on this thread, so the caller's panic handling still applies.
pub(crate) fn run_while_servicing<T: Send>(
    work: impl FnOnce() -> T + Send,
    mut service: impl FnMut(),
) -> T {
    std::thread::scope(|scope| {
        let worker = scope.spawn(work);
        while !worker.is_finished() {
            service();
            std::thread::sleep(AUDIO_SERVICE_PASS_DURING_WORK);
        }
        match worker.join() {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}

/// A log-safe category for a failed service pass: an audio error's stable
/// code, or the error's variant names with every other character removed
/// (messages can carry device names), bounded in length.
pub(crate) fn audio_pump_error_category(error: &ControlError) -> String {
    let text = match error {
        ControlError::Audio { code, .. } => return (*code).to_owned(),
        ControlError::InvalidRequest(message) => message.as_str(),
        _ => return "other".to_owned(),
    };
    let detail = text.split_once(": ").map_or(text, |(_, detail)| detail);
    detail
        .chars()
        .take_while(|character| *character != '"' && *character != '{')
        .filter(|character| character.is_ascii_alphanumeric() || "()_ -".contains(*character))
        .take(120)
        .collect()
}

/// Continuity of the backend-owned native audio service while at least one
/// native worker is running. A gap is the time between consecutive service
/// passes; long gaps mean something (for example a slow control request
/// holding the backend) delayed audio pumping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioServiceStats {
    /// A backend service thread owns pumping (UI pumps are then optional).
    pub active: bool,
    pub passes: u64,
    pub late_gaps: u64,
    pub max_gap_micros: u64,
    /// Multi-input service passes that ended in an error (the pass then
    /// skipped output draining, which is heard as a gap).
    pub pump_errors: u64,
    /// Internal phase probe, excluded from the application diagnostics contract.
    pub recorder_drain_micros: u64,
    last_pass: Option<std::time::Instant>,
}

impl AudioServiceStats {
    pub(crate) fn record(&mut self, now: std::time::Instant, serviced: bool) {
        if !serviced {
            // Nothing is running: the next running pass starts a new series.
            self.last_pass = None;
            return;
        }
        if let Some(previous) = self.last_pass {
            let gap = now.saturating_duration_since(previous);
            let micros = u64::try_from(gap.as_micros()).unwrap_or(u64::MAX);
            self.max_gap_micros = self.max_gap_micros.max(micros);
            if gap > AUDIO_SERVICE_LATE_GAP {
                self.late_gaps = self.late_gaps.saturating_add(1);
            }
        }
        self.passes = self.passes.saturating_add(1);
        self.last_pass = Some(now);
    }

    pub(crate) fn to_json(self) -> Value {
        json!({
            "active": self.active,
            "passes": self.passes,
            "lateGaps": self.late_gaps,
            "maxGapMicros": self.max_gap_micros,
        })
    }
}

impl ControlPlane {
    /// Run self-contained work (it must not borrow the plane) on a helper
    /// thread while this thread keeps servicing running native audio every
    /// [`AUDIO_SERVICE_PASS_DURING_WORK`]. Use it for long, pure work inside a
    /// request handler (folder scans, plugin hashing, audio decoding): the
    /// control thread is also the audio service thread, so running such work
    /// inline would starve render buffers of every playing route. Call it only
    /// where the plane's state is consistent, before the handler mutates it.
    pub(crate) fn while_servicing_audio<T: Send>(&mut self, work: impl FnOnce() -> T + Send) -> T {
        run_while_servicing(work, || {
            self.service_running_native_audio(std::time::Instant::now());
        })
    }

    pub fn service_running_native_audio(&mut self, now: std::time::Instant) -> usize {
        self.audio_service.recorder_drain_micros = 0;
        let running = |plane: &Self, session: &Option<EntityId>| -> Option<(EntityId, u64)> {
            let session = session.as_ref()?;
            plane
                .runtimes
                .get(session)
                .filter(|runtime| runtime.state() == RuntimeState::Running)
                .map(|runtime| (session.clone(), runtime.generation()))
        };
        let mut serviced = 0_usize;
        #[cfg(windows)]
        {
            let budget = audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE;
            if let Some((session, generation)) =
                running(self, &self.native_multi_input_worker_session)
                    .filter(|(_, generation)| self.multi_input_worker_serves(*generation))
            {
                if let Err(error) =
                    self.pump_native_multi_input_worker(&session, generation, budget)
                {
                    self.audio_service.pump_errors =
                        self.audio_service.pump_errors.saturating_add(1);
                    self.last_audio_pump_error = Some(audio_pump_error_category(&error));
                }
                serviced += 1;
            } else if let Some(worker) = self.native_multi_input_worker.as_ref() {
                // Not running, or a stale generation: the audio thread must
                // not pump it either.
                worker.set_pumping(false);
            }
            let playing = running(self, &self.native_multi_input_worker_session)
                .filter(|_| self.native_multi_input_worker.is_some())
                .map(|(session, _)| session);
            self.sample_network_diagnostics(now, playing);
            for slot in [
                self.native_endpoint_session.clone(),
                self.native_endpoint_session_secondary.clone(),
            ] {
                if let Some((session, generation)) = running(self, &slot) {
                    let _ = self
                        .pump_native_endpoint_worker_with_bound_taps(&session, generation, budget);
                    serviced += 1;
                }
            }
            if let Some((session, generation)) = running(self, &self.native_duplex_worker_session)
                .filter(|(_, generation)| self.native_duplex_worker_generation == Some(*generation))
            {
                let _ = self.pump_native_duplex_worker(&session, generation, budget, budget);
                serviced += 1;
            }
            if let Some((session, generation)) =
                running(self, &self.native_render_source_worker_session).filter(
                    |(_, generation)| {
                        self.native_render_source_worker_generation == Some(*generation)
                    },
                )
            {
                let _ = self.pump_native_render_source_worker(&session, generation, budget);
                serviced += 1;
            }
            if self.service_cable_route(now) {
                serviced += 1;
            }
        }
        #[cfg(not(windows))]
        let _ = running;
        // Every worker kind feeds recorder taps, but only some pumps drain
        // them (duplex and render-source did not). Drain once per pass for
        // all of them so a queue never fills while audio plays.
        let _ = self.drain_attached_recorders();
        self.maintain_node_recordings(now);
        self.audio_service.record(now, serviced != 0);
        serviced
    }

    /// Whether opt-in verbose logging is on at `now_unix_ms`. The window
    /// expires by itself after at most one hour.
    pub fn verbose_diagnostics_active(&self, now_unix_ms: u64) -> bool {
        self.verbose_diagnostics.is_active(now_unix_ms)
    }

    pub(crate) fn dispatch_verbose_diagnostics(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let now = audiorouter_protocol::diagnostics::unix_time_ms();
        if let Some(params) = params {
            let enabled = params
                .get("enabled")
                .and_then(Value::as_bool)
                .ok_or_else(|| ControlError::InvalidRequest("enabled must be a boolean".into()))?;
            self.verbose_diagnostics.set(enabled, now);
        }
        Ok(self.verbose_diagnostics.status(now))
    }

    /// Continuity statistics of the backend audio service, for diagnostics.
    pub fn audio_service_stats(&self) -> AudioServiceStats {
        self.audio_service
    }

    pub(crate) fn with_audio_service_stats(&self, mut value: Value) -> Value {
        if let Some(object) = value.as_object_mut() {
            object.insert("audioService".into(), self.audio_service.to_json());
            #[cfg(windows)]
            if self.audio_thread.is_some() {
                let counters = self.audio_thread_shared.counters();
                object.insert(
                    "audioThread".into(),
                    json!({
                        "passes": counters.passes,
                        "lateGaps": counters.late_gaps,
                        "maxGapMicros": counters.max_gap_micros,
                        "pumpErrors": counters.pump_errors,
                        "skippedPasses": counters.skipped_passes,
                    }),
                );
            }
        }
        value
    }

    /// Periodic audio health for the backend log while a native route plays:
    /// service continuity, and per input/output the counters that show where
    /// audio was lost. Node IDs only; no names, paths or raw messages. `None`
    /// when no native route is running.
    pub fn audio_health(&self) -> Option<Value> {
        let mut health = serde_json::Map::new();
        #[cfg(windows)]
        {
            let playing = self
                .native_multi_input_worker_session
                .as_ref()
                .and_then(|session| self.runtimes.get(session))
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running);
            if let Some(worker) = self
                .native_multi_input_worker
                .as_ref()
                .filter(|worker| playing && worker.is_running())
            {
                let ms = |value: Option<f64>| value.map(|value| (value * 10.0).round() / 10.0);
                let waits = worker.input_wait_ms();
                let discontinuities = worker.input_discontinuities();
                let input_trims = worker.input_trimmed_packets();
                let inputs = worker
                    .input_node_ids()
                    .iter()
                    .enumerate()
                    .map(|(index, id)| {
                        json!({
                            "nodeId": id.as_str(),
                            "waitMs": ms(waits.get(index).copied().flatten()),
                            "discontinuities": discontinuities.get(index).copied().unwrap_or(0),
                            "trimmedPackets": input_trims.get(index).copied().unwrap_or(0),
                        })
                    })
                    .collect::<Vec<_>>();
                let queues = worker.output_queue_ms();
                let underruns = worker.output_underruns_per_output();
                let drops = worker.output_full_drops();
                // Physical outputs come first in output-node order.
                let outputs = worker
                    .output_node_ids()
                    .iter()
                    .take(underruns.len())
                    .enumerate()
                    .map(|(index, id)| {
                        json!({
                            "nodeId": id.as_str(),
                            "queueMs": ms(queues.get(index).copied().flatten()),
                            "underruns": underruns[index],
                            "fullDrops": drops.get(index).copied().unwrap_or(0),
                        })
                    })
                    .collect::<Vec<_>>();
                health.insert("inputs".into(), json!(inputs));
                health.insert("outputs".into(), json!(outputs));
            }
        }
        if health.is_empty() {
            return None;
        }
        health.insert("audioService".into(), self.audio_service.to_json());
        #[cfg(windows)]
        if self.audio_thread.is_some() {
            let counters = self.audio_thread_shared.counters();
            let interval = self.audio_thread_shared.take_interval();
            let ms = |micros: u64| (micros as f64 / 100.0).round() / 10.0;
            health.insert(
                "audioThread".into(),
                json!({
                    "started": self.audio_thread_shared.thread_started(),
                    "mmcssProAudio": interval.mmcss_pro_audio,
                    "passes": counters.passes,
                    "lateGaps": counters.late_gaps,
                    "maxGapMicros": counters.max_gap_micros,
                    "pumpErrors": counters.pump_errors,
                    "skippedPasses": counters.skipped_passes,
                    "intervalMaxGapMs": ms(interval.max_gap_micros),
                    "intervalMaxPassMs": ms(interval.max_pass_micros),
                    "intervalSkippedPasses": interval.skipped_passes,
                }),
            );
        }
        health.insert("pumpErrors".into(), json!(self.audio_service.pump_errors));
        health.insert(
            "lastPumpError".into(),
            json!(self.last_audio_pump_error.as_deref()),
        );
        Some(Value::Object(health))
    }

    /// Record that a backend audio service thread now owns pumping.
    pub fn mark_audio_service_started(&mut self) {
        self.audio_service.active = true;
        // The multi-input worker gets its own audio thread, so a slow request
        // on this thread no longer delays it. If the thread cannot start, the
        // service pass keeps pumping inline.
        #[cfg(windows)]
        if self.audio_thread.is_none() {
            self.audio_thread =
                audio_thread::AudioThread::spawn(self.audio_thread_shared.clone()).ok();
        }
        // `apps.list` enumerates for 30-100 ms; build it on its own thread so
        // no request waits behind it. If the thread cannot start, it stays
        // inline.
        #[cfg(windows)]
        if self.application_inventory.is_none() {
            self.application_inventory = app_inventory::AppInventory::spawn(std::sync::Arc::new(
                Self::build_application_inventory,
            ))
            .ok();
        }
    }

    /// Whether the attached multi-input worker plays `generation`: the one it
    /// was prepared for, or the runtime generation a live update was applied
    /// to while it kept playing.
    #[cfg(windows)]
    pub(crate) fn multi_input_worker_serves(&self, generation: u64) -> bool {
        self.native_multi_input_worker_generation == Some(generation)
            || self.native_multi_input_applied_generation == Some(generation)
    }

    #[cfg(windows)]
    pub fn pump_native_multi_input_worker(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        max_packets: u32,
    ) -> Result<Value, ControlError> {
        let checked = self.check_native_multi_input_worker(session_id, generation);
        if self.audio_thread.is_some() {
            // The audio thread pumps only while every check passes.
            if let Some(worker) = self.native_multi_input_worker.as_ref() {
                worker.set_pumping(checked.is_ok());
            }
            // Capture-sink leases are renewed only here: past half a lease
            // without these checks the audio thread pauses (fail closed).
            self.audio_thread_shared.set_pump_valid_for(
                self.native_capture_sink_bindings
                    .values()
                    .map(audiorouter_windows_audio::NativeBridgeCaptureSinkBinding::heartbeat_interval)
                    .min(),
            );
        }
        let generation = checked?;
        let pumped = if self.audio_thread.is_some() {
            self.collect_audio_thread_pump(session_id, generation)
        } else {
            self.pump_native_multi_input_worker_once(session_id, generation, max_packets)
        };
        // Recorder branches of a multi-path session queue audio from their
        // graph taps; write it out here as the endpoint pump does, or the
        // bounded queue fills within milliseconds and the rest is dropped.
        let recorder_chunks_drained = self.drain_attached_recorders()?;
        match pumped.map(|mut value| {
            value["recorderChunksDrained"] = json!(recorder_chunks_drained);
            value
        }) {
            Err(error)
                if self
                    .multi_input_application_sources
                    .iter()
                    .any(|source| &source.session_id == session_id) =>
            {
                // A closed application can fail its capture before the next
                // one-second probe. Check now: if an input was switched to
                // silence the other sources resume on the next pump.
                if self.maintain_multi_input_applications(session_id, true) {
                    Ok(json!({
                        "sessionId": session_id,
                        "generation": generation,
                        "inputs": 0,
                        "capturedFrames": 0,
                        "submittedQuanta": 0,
                        "outputCount": self.native_multi_input_worker.as_ref().map_or(0, |worker| worker.output_count()),
                        "deliveredQuanta": 0,
                        "renderedFrames": 0,
                        "renderBackpressureEvents": 0,
                    }))
                } else {
                    Err(error)
                }
            }
            result => result,
        }
    }

    /// The checks every multi-input pump makes first, returning the worker's
    /// prepared generation. With an audio thread, they decide whether it may
    /// pump.
    #[cfg(windows)]
    fn check_native_multi_input_worker(
        &mut self,
        session_id: &EntityId,
        generation: u64,
    ) -> Result<u64, ControlError> {
        // Keep capture-sink leases alive before borrowing the realtime worker.
        // A failed lease is removed and its virtual route is deactivated by
        // the heartbeat helper, so the subsequent tap path remains
        // fail-closed instead of writing through an expired owner.
        self.heartbeat_native_capture_sink_bindings()?;
        if self.native_multi_input_worker_session.as_ref() != Some(session_id)
            || !self.multi_input_worker_serves(generation)
        {
            return Err(ControlError::InvalidRequest(
                "native multi-input worker binding is stale for the session".into(),
            ));
        }
        let runtime_generation = self
            .runtimes
            .get(session_id)
            .filter(|runtime| runtime.state() == RuntimeState::Running)
            .map(FakeRuntime::generation)
            .ok_or_else(|| ControlError::InvalidRequest("session runtime is not running".into()))?;
        if !self.multi_input_worker_serves(runtime_generation) {
            return Err(ControlError::InvalidRequest(
                "native multi-input worker generation is stale".into(),
            ));
        }
        // Callers may know either the prepared or the live-applied
        // generation; the worker itself runs its prepared one.
        let generation = self
            .native_multi_input_worker_generation
            .unwrap_or(generation);
        if self.poll_native_endpoint_lifecycle()? {
            return Err(ControlError::InvalidRequest(
                "native multi-input worker binding was invalidated; rebind before pumping".into(),
            ));
        }
        self.maintain_multi_input_applications(session_id, false);
        Ok(generation)
    }

    /// With an audio thread pumping, report what it parked instead of
    /// pumping here: its first error since the last call (formatted as the
    /// inline pump formats it), or the totals it pumped since Play.
    /// A panic on the audio thread is re-raised here, so the backend
    /// supervisor restarts the backend as it did for an inline pump panic.
    #[cfg(windows)]
    fn collect_audio_thread_pump(
        &self,
        session_id: &EntityId,
        generation: u64,
    ) -> Result<Value, ControlError> {
        if self.audio_thread_shared.is_faulted() {
            panic!("the audio thread's multi-input pump panicked");
        }
        let worker = self.native_multi_input_worker.as_ref().ok_or_else(|| {
            ControlError::InvalidRequest("native multi-input worker is not attached".into())
        })?;
        if let Some(error) = self.audio_thread_shared.take_error() {
            let pump = if worker.has_output_owner() {
                "input/output"
            } else {
                "input"
            };
            return Err(ControlError::InvalidRequest(format!(
                "native {pump} pump failed: {error:?}"
            )));
        }
        // Totals since Play: the audio thread pumps between these reads, so
        // a per-read delta would almost always be zero.
        let pumped = self.audio_thread_shared.pumped();
        Ok(json!({
            "sessionId": session_id,
            "generation": generation,
            "inputs": pumped.packets,
            "capturedFrames": pumped.captured_frames,
            "submittedQuanta": pumped.processed_quanta,
            "outputCount": worker.output_count(),
            "deliveredQuanta": pumped.delivered_quanta,
            "renderedFrames": pumped.rendered_frames,
            "renderBackpressureEvents": pumped.render_backpressure_events,
            "outputUnderruns": worker.output_underruns(),
        }))
    }

    #[cfg(windows)]
    pub(crate) fn pump_native_multi_input_worker_once(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        max_packets: u32,
    ) -> Result<Value, ControlError> {
        let worker = self.native_multi_input_worker.as_mut().ok_or_else(|| {
            ControlError::InvalidRequest("native multi-input worker is not attached".into())
        })?;
        let output_count = worker.output_count();
        // Any output owner, not only physical outputs: a session whose only
        // outputs are Network Send, Recorder or virtual sinks (for example
        // a microphone streamed to another computer) must still process.
        let (pump, delivered_quanta, render_pump) = if worker.has_output_owner() {
            let (input, delivered, render) =
                worker
                    .pump_and_process_outputs(max_packets)
                    .map_err(|error| {
                        ControlError::InvalidRequest(format!(
                            "native input/output pump failed: {error:?}"
                        ))
                    })?;
            (input, delivered, render)
        } else {
            let input = worker.pump_available(max_packets).map_err(|error| {
                ControlError::InvalidRequest(format!("native input pump failed: {error:?}"))
            })?;
            (
                input,
                0,
                audiorouter_windows_audio::WasapiSchedulerPump::default(),
            )
        };
        Ok(json!({
            "sessionId": session_id,
            "generation": generation,
            "inputs": pump.packets,
            "capturedFrames": pump.captured_frames,
            "submittedQuanta": pump.processed_quanta,
            "outputCount": output_count,
            "deliveredQuanta": delivered_quanta,
            "renderedFrames": render_pump.rendered_frames,
            "renderBackpressureEvents": render_pump.render_backpressure_events,
            "outputUnderruns": worker.output_underruns(),
        }))
    }

    #[cfg(windows)]
    /// Bind branch-local virtual/recording/tool observers to the exact
    /// validated output-node order retained by the prepared worker graph.
    /// This is control-thread preparation and must complete while stopped.
    pub fn bind_native_multi_input_branches(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        branch_node_ids: &[EntityId],
    ) -> Result<Value, ControlError> {
        if self.native_multi_input_worker_session.as_ref() != Some(session_id)
            || self.native_multi_input_worker_generation != Some(generation)
        {
            return Err(ControlError::InvalidRequest(
                "native multi-input worker binding is stale for the session".into(),
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
                "native multi-input worker generation is stale".into(),
            ));
        }
        let expected = self
            .native_multi_input_worker
            .as_ref()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native multi-input worker is not attached".into())
            })?
            .output_node_ids()
            .to_vec();
        if expected != branch_node_ids {
            return Err(ControlError::InvalidRequest(
                "branch node IDs do not match the prepared graph output order".into(),
            ));
        }
        let session = self.get_session(session_id)?.clone();
        let has_output_owner = self
            .native_multi_input_worker
            .as_ref()
            .ok_or_else(|| {
                ControlError::InvalidRequest("native multi-input worker is not attached".into())
            })?
            .has_output_owner();
        if !has_output_owner
            && branch_node_ids.iter().any(|node_id| {
                session
                    .nodes
                    .iter()
                    .any(|node| node.id == *node_id && node.kind == NodeKind::PhysicalOutput)
            })
        {
            return Err(ControlError::InvalidRequest(
                "physical output branches require a prepared output owner".into(),
            ));
        }
        let mut tap_sets = Vec::with_capacity(branch_node_ids.len());
        for node_id in branch_node_ids {
            let node = session
                .nodes
                .iter()
                .find(|node| &node.id == node_id)
                .ok_or_else(|| {
                    ControlError::InvalidRequest("branch node is not in the session".into())
                })?;
            let taps = match node.kind {
                // A Network Send branch already carries its sender tap.
                NodeKind::PhysicalOutput | NodeKind::NetworkSend => AudioTapSet::new(),
                NodeKind::VirtualCaptureSink => {
                    let bus_id = node
                        .parameters
                        .get("busId")
                        .and_then(Value::as_str)
                        .filter(|value| !value.is_empty())
                        .map(EntityId::new)
                        .ok_or_else(|| {
                            ControlError::InvalidRequest(
                                "virtual capture branch is missing its bus ID".into(),
                            )
                        })?;
                    self.virtual_route_tap_set(
                        session_id,
                        std::slice::from_ref(&bus_id),
                        generation,
                    )?
                }
                NodeKind::Recorder => self.recorder_tap_set_for_node(session_id, node_id)?,
                _ => {
                    return Err(ControlError::InvalidRequest(
                        "multi-input output branches must be physical outputs, virtual capture sinks, recorders, or network sends"
                            .into(),
                    ));
                }
            };
            tap_sets.push(taps);
        }
        let worker = self.native_multi_input_worker.as_mut().ok_or_else(|| {
            ControlError::InvalidRequest("native multi-input worker is not attached".into())
        })?;
        let bind_result = if !worker.has_output_owner() {
            worker.attach_tap_only_branches(
                tap_sets,
                2,
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
            )
        } else {
            worker.attach_output_branch_tap_sets(tap_sets)
        };
        bind_result.map_err(|error| {
            ControlError::InvalidRequest(format!(
                "native multi-input branch binding failed: {error:?}"
            ))
        })?;
        Ok(json!({
            "sessionId": session_id,
            "generation": generation,
            "branchNodeIds": branch_node_ids,
            "boundBranches": branch_node_ids.len(),
        }))
    }
}

#[cfg(test)]
#[path = "audio_service_tests.rs"]
mod tests;
