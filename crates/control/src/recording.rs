//! Recorder nodes: attaching recorder workers, one-click recording and recorder control.

use super::*;

/// Automatic file splitting for a one-click recording. WAV splits in place
/// without a gap; other formats roll over to a new file at the boundary.
#[derive(Clone, Debug)]
pub(crate) struct RecordingSplit {
    pub(crate) every_frames: u64,
    started_at: Option<u64>,
    in_place: bool,
}

/// One-click recording settings stored on a Recorder node.
pub(crate) struct RecorderNodeSettings {
    format: FileRecorderFormat,
    format_name: &'static str,
    auto_record: bool,
    split_minutes: u64,
    channels: u16,
}

pub(crate) fn recorder_node_settings(node: &audiorouter_domain::Node) -> RecorderNodeSettings {
    let (format, format_name) = match node
        .parameters
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("wavPcm24")
    {
        "wavPcm16" => (FileRecorderFormat::Wav(WavFormat::Pcm16), "wavPcm16"),
        "wavFloat32" => (FileRecorderFormat::Wav(WavFormat::Float32), "wavFloat32"),
        "flac16" => (
            FileRecorderFormat::Flac {
                bits_per_sample: 16,
            },
            "flac16",
        ),
        "flac24" => (
            FileRecorderFormat::Flac {
                bits_per_sample: 24,
            },
            "flac24",
        ),
        "mp3" => (FileRecorderFormat::Mp3, "mp3"),
        _ => (FileRecorderFormat::Wav(WavFormat::Pcm24), "wavPcm24"),
    };
    let channels = node
        .ports
        .iter()
        .find(|port| port.direction == PortDirection::Input)
        .map_or(2, |port| u16::from(port.channels).clamp(1, 2));
    RecorderNodeSettings {
        format,
        format_name,
        auto_record: node
            .parameters
            .get("autoRecord")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        split_minutes: node
            .parameters
            .get("splitMinutes")
            .and_then(Value::as_f64)
            .filter(|minutes| minutes.is_finite())
            .map_or(0, |minutes| minutes.clamp(0.0, 240.0) as u64),
        channels,
    }
}

pub(crate) fn recorder_state_name(state: audiorouter_recording::RecorderState) -> &'static str {
    match state {
        audiorouter_recording::RecorderState::Idle => "idle",
        audiorouter_recording::RecorderState::Armed => "armed",
        audiorouter_recording::RecorderState::Recording => "recording",
        audiorouter_recording::RecorderState::Paused => "paused",
        audiorouter_recording::RecorderState::Stopping => "stopping",
        audiorouter_recording::RecorderState::Completed => "completed",
        audiorouter_recording::RecorderState::Failed => "failed",
    }
}

pub(crate) fn recorder_is_active(state: RecorderState) -> bool {
    matches!(
        state,
        RecorderState::Armed
            | RecorderState::Recording
            | RecorderState::Paused
            | RecorderState::Stopping
    )
}

impl ControlPlane {
    /// Drain recorder queues only after the endpoint pump has returned to the
    /// control thread. The realtime tap remains a bounded enqueue-only path;
    /// file encoding and flushing never run in the audio callback.
    pub(crate) fn drain_attached_recorders(&mut self) -> Result<usize, ControlError> {
        let begin = std::time::Instant::now();
        let mut drained = 0usize;
        for worker in self.recorder_workers.values_mut() {
            let count = worker
                .drain_pending(MAX_RECORDER_PUMP_CHUNKS)
                .map_err(|error| {
                    ControlError::InvalidRequest(format!("recorder drain failed: {error}"))
                })?;
            drained = drained.saturating_add(count);
        }
        // A failed one-click recording must not stop the audio pump or the
        // other recorders: remember why, keep its written prefix for Stop.
        for (node_id, worker) in self.recorder_node_workers.iter_mut() {
            match worker.drain_pending(MAX_RECORDER_PUMP_CHUNKS) {
                Ok(count) => drained = drained.saturating_add(count),
                Err(error) => {
                    self.recorder_node_failures
                        .entry(node_id.clone())
                        .or_insert(error);
                }
            }
        }
        self.audio_service.recorder_drain_micros = self
            .audio_service
            .recorder_drain_micros
            .saturating_add(u64::try_from(begin.elapsed().as_micros()).unwrap_or(u64::MAX));
        Ok(drained)
    }

    /// Attach the encoder/queue owner for one session. Replacing an existing
    /// worker is rejected so a live destination cannot be orphaned silently.
    pub fn attach_recorder_worker(
        &mut self,
        session_id: EntityId,
        worker: Box<dyn RecorderWorker>,
    ) -> Result<(), ControlError> {
        if self.recorder_workers.contains_key(&session_id) {
            return Err(ControlError::InvalidRequest(
                "recorder worker is already attached".into(),
            ));
        }
        self.recorder_workers.insert(session_id, worker);
        Ok(())
    }

    /// Build the bounded realtime observer set for one validated recorder
    /// node. A node-owned worker is preferred; the legacy session worker is
    /// accepted only when the session contains exactly one enabled recorder.
    /// The returned set is immutable by convention after construction.
    pub fn recorder_tap_set_for_node(
        &self,
        session_id: &EntityId,
        node_id: &EntityId,
    ) -> Result<AudioTapSet, ControlError> {
        let session = self.get_session(session_id)?;
        let recorder_nodes = session
            .nodes
            .iter()
            .filter(|node| node.enabled && node.kind == NodeKind::Recorder)
            .collect::<Vec<_>>();
        let node = recorder_nodes
            .iter()
            .find(|node| node.id == *node_id)
            .ok_or_else(|| {
                ControlError::InvalidRequest("enabled recorder node is not in the session".into())
            })?;
        let tap = self.recorder_inlet(session_id, &node.id, recorder_nodes.len() == 1);
        let mut set = AudioTapSet::new();
        set.add_shared(tap)
            .map_err(|_| ControlError::InvalidRequest("recorder tap capacity exceeded".into()))?;
        Ok(set)
    }

    /// The stable inlet of a Recorder node, pointed at its current worker (or
    /// at nothing, in which case audio is dropped until Record is pressed).
    pub(crate) fn recorder_inlet(
        &self,
        session_id: &EntityId,
        node_id: &EntityId,
        single_recorder: bool,
    ) -> Arc<dyn audiorouter_engine::AudioTap> {
        let inlet = self
            .recorder_inlets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(node_id.clone())
            .or_default()
            .clone();
        let target = self
            .recorder_node_workers
            .get(node_id)
            .and_then(|worker| worker.shared_audio_tap())
            .or_else(|| {
                single_recorder
                    .then(|| {
                        self.recorder_workers
                            .get(session_id)
                            .and_then(|worker| worker.shared_audio_tap())
                    })
                    .flatten()
            });
        inlet.set(target);
        inlet
    }

    /// Point a bound inlet at the node's current worker after attach/remove.
    pub(crate) fn sync_recorder_inlet(&self, node_id: &EntityId) {
        if let Some(inlet) = self
            .recorder_inlets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(node_id)
        {
            inlet.set(
                self.recorder_node_workers
                    .get(node_id)
                    .and_then(|worker| worker.shared_audio_tap()),
            );
        }
    }

    /// Build the bounded engine observer set for one attached recorder. The
    /// returned set is immutable by convention after construction and can be
    /// handed to the realtime scheduler's tap-set method.
    pub fn recorder_tap_set(&self, session_id: &EntityId) -> Result<AudioTapSet, ControlError> {
        let worker = self.recorder_workers.get(session_id).ok_or_else(|| {
            ControlError::InvalidRequest("recorder worker is not attached".into())
        })?;
        let tap = worker.shared_audio_tap().ok_or_else(|| {
            ControlError::InvalidRequest("recorder worker has no realtime tap".into())
        })?;
        let mut set = AudioTapSet::new();
        set.add_shared(tap)
            .map_err(|_| ControlError::InvalidRequest("recorder tap capacity exceeded".into()))?;
        Ok(set)
    }

    /// Bind control-owned recorder workers to the session's validated recorder
    /// nodes. The compatibility session worker is accepted for one node;
    /// multiple nodes require one independently attached node worker each.
    pub fn recorder_tap_bindings(
        &self,
        session_id: &EntityId,
        generation: RuntimeGeneration,
    ) -> Result<RecorderTapBindings, ControlError> {
        let session = self.get_session(session_id)?;
        let recorder_nodes: Vec<&EntityId> = session
            .nodes
            .iter()
            .filter(|node| node.enabled && node.kind == NodeKind::Recorder)
            .map(|node| &node.id)
            .collect();
        if recorder_nodes.is_empty() {
            return Err(ControlError::InvalidRequest(
                "session must contain an enabled recorder node".into(),
            ));
        }
        let recorder_node_count = recorder_nodes.len();
        let mut bindings = RecorderTapBindings::new();
        for node_id in recorder_nodes {
            let tap = self.recorder_inlet(session_id, node_id, recorder_node_count == 1);
            bindings
                .add_shared(node_id.as_str(), generation, tap)
                .map_err(|_| {
                    ControlError::InvalidRequest("recorder node binding is invalid".into())
                })?;
        }
        Ok(bindings)
    }

    /// Attach a worker and independent lifecycle state to one validated
    /// recorder node. The node worker is the independent-sink path; JSON-RPC
    /// lifecycle addressing remains session-scoped until recorder IDs are
    /// promoted into that API.
    pub fn attach_recorder_worker_to_node(
        &mut self,
        session_id: &EntityId,
        node_id: EntityId,
        worker: Box<dyn RecorderWorker>,
    ) -> Result<(), ControlError> {
        let session = self.get_session(session_id)?;
        let node = session
            .nodes
            .iter()
            .find(|node| node.id == node_id && node.kind == NodeKind::Recorder && node.enabled)
            .ok_or_else(|| {
                ControlError::InvalidRequest("enabled recorder node is not in the session".into())
            })?;
        let _ = node;
        if self.recorder_node_workers.contains_key(&node_id) {
            return Err(ControlError::InvalidRequest(
                "recorder node worker is already attached".into(),
            ));
        }
        if worker.shared_audio_tap().is_none() {
            return Err(ControlError::InvalidRequest(
                "recorder worker has no realtime tap".into(),
            ));
        }
        self.recorder_node_workers.insert(node_id.clone(), worker);
        self.sync_recorder_inlet(&node_id);
        self.recorder_node_states
            .insert(node_id.clone(), RecorderController::new());
        self.recorder_node_sessions
            .insert(node_id, session_id.clone());
        Ok(())
    }

    /// Apply one lifecycle boundary to one node-owned recorder. All file and
    /// checkpoint work stays on this control/lifecycle path, never the audio
    /// callback. The returned shape mirrors the session recorder response but
    /// identifies the independent node.
    /// Start recording a Recorder node now, using its own settings (format,
    /// split). Plays and records are independent: the node's inlet is already
    /// bound, so this works while the route plays. Returns the new file path.
    pub fn start_node_recording(
        &mut self,
        session_id: &EntityId,
        node_id: &EntityId,
    ) -> Result<Value, ControlError> {
        let node = self
            .get_session(session_id)?
            .nodes
            .iter()
            .find(|node| node.id == *node_id && node.kind == NodeKind::Recorder && node.enabled)
            .cloned()
            .ok_or_else(|| {
                ControlError::InvalidRequest("enabled recorder node is not in the session".into())
            })?;
        let settings = recorder_node_settings(&node);
        let state = self
            .recorder_node_states
            .get(node_id)
            .map(RecorderController::state);
        if matches!(
            state,
            Some(RecorderState::Recording | RecorderState::Paused)
        ) {
            return Ok(
                json!({ "sessionId": session_id, "nodeId": node_id, "state": "recording", "path": Value::Null, "alreadyRecording": true }),
            );
        }
        let mut path = Value::Null;
        if !matches!(state, Some(RecorderState::Idle | RecorderState::Armed)) {
            // Completed/Failed leftovers are replaced by a fresh file.
            self.recorder_node_failures.remove(node_id);
            self.recorder_node_workers.remove(node_id);
            self.recorder_node_states.remove(node_id);
            self.recorder_node_sessions.remove(node_id);
            self.recording_sequence += 1;
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_millis() as u64);
            let mut recorder_id = format!("{}-{stamp}", node_id.as_str());
            recorder_id.truncate(audiorouter_domain::MAX_ENTITY_ID_BYTES);
            let config = FileRecorderConfig {
                version: FILE_RECORDER_CONFIG_VERSION,
                session_id: session_id.as_str(),
                recorder_id: &recorder_id,
                sequence: self.recording_sequence,
                format: settings.format,
                channels: settings.channels,
                sample_rate: 48_000,
                dither: default_dither_for_format(settings.format),
                // REC-08: two seconds of audio, so a slow service pass
                // (a busy request, a disk stall) never drops audio.
                queue_capacity: ONE_CLICK_RECORDER_QUEUE_CHUNKS,
                maximum_chunks_per_pass: 8,
            };
            path = json!(self.create_and_attach_configured_file_recorder_to_node(
                session_id.clone(),
                node_id.clone(),
                &config
            )?);
        }
        if self
            .recorder_node_states
            .get(node_id)
            .map(RecorderController::state)
            == Some(RecorderState::Idle)
        {
            self.control_recorder_node(node_id, "recorders.arm", None)?;
        }
        // Only blocks that arrive after attaching reach the file, so frame 0
        // means "from now".
        self.control_recorder_node(node_id, "recorders.start", Some(0))?;
        if settings.split_minutes > 0 {
            self.recording_splits.insert(
                node_id.clone(),
                RecordingSplit {
                    every_frames: settings.split_minutes * 60 * 48_000,
                    started_at: None,
                    in_place: matches!(settings.format, FileRecorderFormat::Wav(_)),
                },
            );
        } else {
            self.recording_splits.remove(node_id);
        }
        Ok(
            json!({ "sessionId": session_id, "nodeId": node_id, "state": "recording", "format": settings.format_name, "path": path, "splitMinutes": settings.split_minutes }),
        )
    }

    /// Recorders set to record automatically start with playback. A failure
    /// (no recording folder, limit reached) never blocks Play; it is reported
    /// in the start result's `autoRecording`.
    pub(crate) fn start_auto_recordings(
        &mut self,
        session_id: &EntityId,
        result: &mut Value,
    ) -> Result<(), ControlError> {
        let automatic = self
            .get_session(session_id)?
            .nodes
            .iter()
            .filter(|node| {
                node.kind == NodeKind::Recorder
                    && node.enabled
                    && recorder_node_settings(node).auto_record
            })
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if automatic.is_empty() {
            return Ok(());
        }
        let outcomes = automatic
            .iter()
            .map(|node_id| match self.start_node_recording(session_id, node_id) {
                Ok(started) => json!({ "nodeId": node_id, "state": "recording", "path": started["path"] }),
                Err(error) => json!({ "nodeId": node_id, "state": "failed", "error": format!("{error:?}") }),
            })
            .collect::<Vec<_>>();
        if let Some(object) = result.as_object_mut() {
            object.insert("autoRecording".into(), json!(outcomes));
        }
        Ok(())
    }

    /// Stop a Recorder node's recording and finalize its file at the last
    /// audio it received. Without an active recording this is a no-op.
    pub fn stop_node_recording(&mut self, node_id: &EntityId) -> Result<Value, ControlError> {
        self.recording_splits.remove(node_id);
        let session_id = self.recorder_node_sessions.get(node_id).cloned();
        let state = self
            .recorder_node_states
            .get(node_id)
            .map(RecorderController::state);
        match state {
            Some(RecorderState::Recording | RecorderState::Paused) => {
                let frame = self
                    .recorder_node_workers
                    .get(node_id)
                    .and_then(|worker| worker.committed_end_frame())
                    .unwrap_or(0);
                self.control_recorder_node(node_id, "recorders.stop", Some(frame))
            }
            Some(RecorderState::Stopping | RecorderState::Failed) => {
                Err(ControlError::InvalidRequest(
                    "recorder needs recovery in the Recording tab before it can stop".into(),
                ))
            }
            _ => {
                self.recorder_node_workers.remove(node_id);
                self.sync_recorder_inlet(node_id);
                self.recorder_node_states.remove(node_id);
                self.recorder_node_sessions.remove(node_id);
                Ok(json!({ "sessionId": session_id, "nodeId": node_id, "state": "idle" }))
            }
        }
    }

    /// Automatic file splitting for one-click recordings (control thread,
    /// throttled; called from the backend audio service loop).
    pub(crate) fn maintain_node_recordings(&mut self, now: std::time::Instant) {
        if self.recording_splits.is_empty()
            || self.recording_maintained_at.is_some_and(|at| {
                now.saturating_duration_since(at) < std::time::Duration::from_millis(250)
            })
        {
            return;
        }
        self.recording_maintained_at = Some(now);
        let workers = &self.recorder_node_workers;
        let due = self
            .recording_splits
            .iter_mut()
            .filter_map(|(node_id, split)| {
                let end = workers.get(node_id)?.committed_end_frame()?;
                let started = *split.started_at.get_or_insert(end);
                (end.saturating_sub(started) >= split.every_frames)
                    .then(|| (node_id.clone(), end, split.in_place))
            })
            .collect::<Vec<_>>();
        for (node_id, frame, in_place) in due {
            let split = if in_place {
                self.control_recorder_node(&node_id, "recorders.split", Some(frame))
                    .map(|_| ())
            } else {
                match self.recorder_node_sessions.get(&node_id).cloned() {
                    Some(session_id) => self
                        .stop_node_recording(&node_id)
                        .and_then(|_| self.start_node_recording(&session_id, &node_id))
                        .map(|_| ()),
                    None => Ok(()),
                }
            };
            match (split, self.recording_splits.get_mut(&node_id)) {
                (Ok(()), Some(state)) => state.started_at = Some(frame),
                (Err(_), _) => {
                    // Keep recording into the current file rather than lose audio.
                    self.recording_splits.remove(&node_id);
                }
                _ => {}
            }
        }
    }

    pub fn control_recorder_node(
        &mut self,
        node_id: &EntityId,
        method: &str,
        frame: Option<u64>,
    ) -> Result<Value, ControlError> {
        let session_id = self
            .recorder_node_sessions
            .get(node_id)
            .cloned()
            .ok_or_else(|| ControlError::InvalidRequest("recorder node is not attached".into()))?;
        let session_revision = self.get_session(&session_id)?.revision;
        if method == "recorders.arm"
            && !self
                .recorder_node_states
                .get(node_id)
                .is_some_and(|recorder| recorder_is_active(recorder.state()))
            && self
                .recorders
                .values()
                .chain(self.recorder_node_states.values())
                .filter(|recorder| recorder_is_active(recorder.state()))
                .count()
                >= MAX_ACTIVE_RECORDERS
        {
            return Err(ControlError::InvalidRequest(
                "active recorder limit reached".into(),
            ));
        }
        let frame = |required: bool| {
            if required {
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))
            } else {
                Ok(frame.unwrap_or_default())
            }
        };
        let mut recorder_candidate =
            self.recorder_node_states
                .get(node_id)
                .cloned()
                .ok_or_else(|| {
                    ControlError::InvalidRequest("recorder node state is not attached".into())
                })?;
        match method {
            "recorders.arm" => recorder_candidate.arm(),
            "recorders.start" => recorder_candidate.start(frame(true)?),
            "recorders.pause" => recorder_candidate.pause(frame(true)?),
            "recorders.resume" => recorder_candidate.resume(frame(true)?),
            "recorders.split" => recorder_candidate.split(frame(true)?),
            "recorders.stop" => recorder_candidate.stop(frame(true)?),
            _ => Err(audiorouter_recording::RecorderError::InvalidTransition {
                state: recorder_candidate.state(),
                action: "unknown",
            }),
        }
        .map_err(|error| {
            ControlError::InvalidRequest(format!("recorder transition failed: {error:?}"))
        })?;
        let mut finalized_recordings = Vec::new();
        let mut stopped_after_failure = false;
        {
            let worker = self.recorder_node_workers.get_mut(node_id).ok_or_else(|| {
                ControlError::InvalidRequest("recorder node worker is not attached".into())
            })?;
            let result = match method {
                "recorders.arm" => worker.arm(),
                "recorders.start" => worker.start(frame(true)?),
                "recorders.pause" => worker.pause(frame(true)?),
                "recorders.resume" => worker.resume(frame(true)?),
                "recorders.split" => worker.split(frame(true)?),
                "recorders.stop" => match worker.finalize(frame(true)?) {
                    Ok(outcome)
                        if outcome.state == "completed"
                            && outcome.file_finalized
                            && !outcome.recoverable =>
                    {
                        finalized_recordings = worker.finalized_recordings();
                        Ok(())
                    }
                    // Failed mid-take: the written prefix is finalized and
                    // kept, and the take is reported as failed (REC-08).
                    Ok(outcome) if outcome.state == "failed" && outcome.file_finalized => {
                        finalized_recordings = worker.finalized_recordings();
                        stopped_after_failure = true;
                        Ok(())
                    }
                    Ok(_) => Err("recorder finalization did not produce a completed file".into()),
                    Err(error) => Err(format!("recorder finalization failed: {error}")),
                },
                _ => return Err(ControlError::InvalidRequest("method not found".into())),
            };
            if let Err(error) = result {
                if let Some(state) = self.recorder_node_states.get_mut(node_id) {
                    state.fail();
                }
                return Err(ControlError::InvalidRequest(format!(
                    "recorder worker transition failed: {error}"
                )));
            }
        }
        let (checkpoint, state, parts, pauses) = {
            self.recorder_node_states
                .insert(node_id.clone(), recorder_candidate);
            let recorder = self
                .recorder_node_states
                .get(node_id)
                .expect("recorder node state inserted above");
            let checkpoint = recorder.checkpoint();
            let state = recorder_state_name(recorder.state());
            let parts = checkpoint
                .parts
                .iter()
                .map(|part| {
                    json!({
                        "index": part.index,
                        "startFrame": part.start_frame,
                        "endFrame": part.end_frame,
                    })
                })
                .collect::<Vec<_>>();
            let pauses = checkpoint
                .pauses
                .iter()
                .map(|pause| {
                    json!({
                        "startFrame": pause.start_frame,
                        "endFrame": pause.end_frame,
                    })
                })
                .collect::<Vec<_>>();
            (checkpoint, state, parts, pauses)
        };
        if let Some(storage) = &self.storage {
            storage
                .save_recording_checkpoint(node_id.as_str(), &checkpoint)
                .map_err(storage_error)?;
            for recording in &finalized_recordings {
                storage.save_recording(recording).map_err(storage_error)?;
                storage
                    .save_recording_node_binding(recording.id.as_str(), node_id.as_str())
                    .map_err(storage_error)?;
            }
        }
        let last_frame = checkpoint.last_frame;
        let mut result = json!({
            "nodeId": node_id.as_str(),
            "sessionId": session_id,
            "state": if stopped_after_failure { "failed" } else { state },
            "parts": parts,
            "pauses": pauses,
            "lastFrame": last_frame,
        });
        if stopped_after_failure {
            let seconds = finalized_recordings
                .iter()
                .map(|recording| recording.frames / u64::from(recording.sample_rate.max(1)))
                .sum::<u64>();
            let cause = self
                .recorder_node_failures
                .remove(node_id)
                .unwrap_or_default();
            // Without an I/O error the only way to fail mid-take is a gap in
            // the audio (found while playing or in Stop's final drain).
            let reason = if cause.contains("Io(") {
                "the file could not be written"
            } else {
                "audio was lost (the recorder fell behind or received a block twice)"
            };
            result["reason"] = json!(format!(
                "Recording stopped saving after {}:{:02} because {reason}. The file keeps everything up to that point.",
                seconds / 60,
                seconds % 60
            ));
            result["paths"] = json!(finalized_recordings
                .iter()
                .map(|recording| recording.path.clone())
                .collect::<Vec<_>>());
        }
        self.events
            .append(session_revision, None, "recorder.changed", Some(session_id));
        if method == "recorders.stop" {
            self.recorder_node_failures.remove(node_id);
            self.recorder_node_workers.remove(node_id);
            self.sync_recorder_inlet(node_id);
            self.recorder_node_states.remove(node_id);
            self.recorder_node_sessions.remove(node_id);
        }
        Ok(result)
    }

    /// Attach a single-file worker and configure its durable library identity
    /// before it can be started. This is the factory/session boundary for
    /// callers that create WAV or FLAC workers outside the control plane.
    pub fn attach_recorder_worker_with_identity(
        &mut self,
        session_id: EntityId,
        identity: FileRecordingIdentity,
        mut worker: Box<dyn RecorderWorker>,
    ) -> Result<(), ControlError> {
        worker
            .set_library_identity(identity)
            .map_err(ControlError::InvalidRequest)?;
        self.attach_recorder_worker(session_id, worker)
    }

    /// Create and attach a path-owned WAV/FLAC recorder before any lifecycle
    /// command can arm it. File allocation and worker construction happen on
    /// this control/lifecycle boundary, never in the audio callback.
    #[allow(clippy::too_many_arguments)]
    pub fn create_and_attach_file_recorder(
        &mut self,
        policy: &RecordingPathPolicy,
        session_id: EntityId,
        recorder_id: &str,
        sequence: u64,
        format: FileRecorderFormat,
        channels: u16,
        sample_rate: u32,
        dither: bool,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
    ) -> Result<std::path::PathBuf, ControlError> {
        let (path, worker) = create_file_recorder(
            policy,
            session_id.as_str(),
            recorder_id,
            sequence,
            format,
            channels,
            sample_rate,
            dither,
            queue_capacity,
            maximum_chunks_per_pass,
        )
        .map_err(ControlError::InvalidRequest)?;
        if let Err(error) = self.attach_recorder_worker(session_id, worker) {
            // File creation is intentionally before worker construction, but
            // attachment is the ownership handoff. If that handoff fails,
            // remove the exclusively-created empty file so a rejected
            // recorder cannot leave an orphaned recording artifact behind.
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        Ok(path)
    }

    pub fn create_and_attach_file_recorder_with_config(
        &mut self,
        policy: &RecordingPathPolicy,
        session_id: EntityId,
        config: &FileRecorderConfig<'_>,
    ) -> Result<std::path::PathBuf, ControlError> {
        if config.session_id != session_id.as_str() {
            return Err(ControlError::InvalidRequest(
                "file recorder session identity does not match attachment".into(),
            ));
        }
        let (path, worker) = create_file_recorder_with_config(policy, config)
            .map_err(ControlError::InvalidRequest)?;
        if let Err(error) = self.attach_recorder_worker(session_id, worker) {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        Ok(path)
    }

    /// Create and attach using the backend-owned, persisted recording root.
    /// No caller-supplied destination policy is accepted at this boundary.
    pub fn create_and_attach_configured_file_recorder(
        &mut self,
        session_id: EntityId,
        config: &FileRecorderConfig<'_>,
    ) -> Result<std::path::PathBuf, ControlError> {
        if config.session_id != session_id.as_str() {
            return Err(ControlError::InvalidRequest(
                "file recorder session identity does not match attachment".into(),
            ));
        }
        let (path, worker) = {
            let policy = self
                .recording_policy
                .as_ref()
                .ok_or_else(|| ControlError::InvalidRequest(RECORDING_ROOT_MISSING.into()))?;
            create_file_recorder_with_config(policy, config)
                .map_err(ControlError::InvalidRequest)?
        };
        if let Err(error) = self.attach_recorder_worker(session_id, worker) {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        Ok(path)
    }

    /// Create and attach a file recorder to one validated recorder node.
    /// Exclusive file creation is rolled back if node attachment fails.
    pub fn create_and_attach_configured_file_recorder_to_node(
        &mut self,
        session_id: EntityId,
        node_id: EntityId,
        config: &FileRecorderConfig<'_>,
    ) -> Result<std::path::PathBuf, ControlError> {
        if config.session_id != session_id.as_str() {
            return Err(ControlError::InvalidRequest(
                "file recorder session identity does not match attachment".into(),
            ));
        }
        let node_is_valid = self
            .get_session(&session_id)?
            .nodes
            .iter()
            .any(|node| node.id == node_id && node.kind == NodeKind::Recorder && node.enabled);
        if !node_is_valid {
            return Err(ControlError::InvalidRequest(
                "enabled recorder node is not in the session".into(),
            ));
        }
        let (path, worker) = {
            let policy = self
                .recording_policy
                .as_ref()
                .ok_or_else(|| ControlError::InvalidRequest(RECORDING_ROOT_MISSING.into()))?;
            create_file_recorder_with_config(policy, config)
                .map_err(ControlError::InvalidRequest)?
        };
        if let Err(error) = self.attach_recorder_worker_to_node(&session_id, node_id, worker) {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        Ok(path)
    }

    /// Whether any recorder of the session is taking a recording.
    pub(crate) fn session_is_recording(&self, session_id: &EntityId) -> bool {
        let node_recording = self.get_session(session_id).is_ok_and(|session| {
            session.nodes.iter().any(|node| {
                node.kind == NodeKind::Recorder && self.recorder_node_workers.contains_key(&node.id)
            })
        });
        node_recording
            || self.recorders.get(session_id).is_some_and(|recorder| {
                matches!(
                    recorder.state(),
                    RecorderState::Recording | RecorderState::Paused | RecorderState::Stopping
                )
            })
    }

    pub(crate) fn dispatch_recorder_create(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("recorder creation parameters are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let node_id = params
            .get("nodeId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new);
        let recorder_id = params
            .get("recorderId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("recorderId is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let request_hash = Self::request_hash(&params);
        let scoped_key = self.scoped_idempotency_key("recorders.create", idempotency_key);
        if let Some(result) = self.lookup_idempotent_result(&scoped_key, &request_hash)? {
            return Ok(result);
        }
        let sequence = params
            .get("sequence")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("sequence is required".into()))?;
        let channels = params
            .get("channels")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| ControlError::InvalidRequest("channels is required".into()))?;
        let sample_rate = params
            .get("sampleRate")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| ControlError::InvalidRequest("sampleRate is required".into()))?;
        let queue_capacity = params
            .get("queueCapacity")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| ControlError::InvalidRequest("queueCapacity is required".into()))?;
        let maximum_chunks_per_pass = params
            .get("maximumChunksPerPass")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| {
                ControlError::InvalidRequest("maximumChunksPerPass is required".into())
            })?;
        let format_name = params
            .get("format")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("format is required".into()))?;
        let format = match format_name {
            "wavPcm16" => FileRecorderFormat::Wav(WavFormat::Pcm16),
            "wavPcm24" => FileRecorderFormat::Wav(WavFormat::Pcm24),
            "wavFloat32" => FileRecorderFormat::Wav(WavFormat::Float32),
            "flac16" => FileRecorderFormat::Flac {
                bits_per_sample: 16,
            },
            "flac24" => FileRecorderFormat::Flac {
                bits_per_sample: 24,
            },
            "mp3" => FileRecorderFormat::Mp3,
            _ => {
                return Err(ControlError::InvalidRequest(
                    "unsupported recorder format".into(),
                ))
            }
        };
        // Integer formats default to TPDF dithering when the optional API
        // field is omitted. Float32 never applies dither; the finalization
        // metadata also records that effective policy.
        let dither = params
            .get("dither")
            .and_then(Value::as_bool)
            .unwrap_or(default_dither_for_format(format));
        let session = EntityId::new(session_id);
        if self.store.session(&session).is_none() {
            return Err(ControlError::InvalidRequest("session not found".into()));
        }
        let config = FileRecorderConfig {
            version: FILE_RECORDER_CONFIG_VERSION,
            session_id,
            recorder_id,
            sequence,
            format,
            channels,
            sample_rate,
            dither,
            queue_capacity,
            maximum_chunks_per_pass,
        };
        let path = if let Some(node_id) = node_id.as_ref() {
            self.create_and_attach_configured_file_recorder_to_node(
                session.clone(),
                node_id.clone(),
                &config,
            )?
        } else {
            self.create_and_attach_configured_file_recorder(session, &config)?
        };
        let result = json!({
            "sessionId": session_id,
            "nodeId": node_id,
            "recorderId": recorder_id,
            "format": format_name,
            "path": path,
            "state": "idle",
            "armed": false,
        });
        self.journal_idempotent_result(&scoped_key, "recorders.create", &request_hash, &result)?;
        Ok(result)
    }

    /// One-click Record / Stop for a Recorder node (UI button, StreamDeck,
    /// MCP). Idempotent per key; Stop without a recording is a no-op.
    pub(crate) fn dispatch_one_click_recording(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId, nodeId and idempotencyKey are required".into())
        })?;
        let field = |name: &str| {
            params
                .get(name)
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| ControlError::InvalidRequest(format!("{name} is required")))
        };
        // sessionId defaults to the active session; nodeId may be a node name.
        let session_id = self.simple_session_id(&params)?;
        let node_id = simple::resolve_node(self.get_session(&session_id)?, &field("nodeId")?)?
            .id
            .clone();
        let idempotency = field("idempotencyKey")?;
        let request_hash = Self::request_hash(
            &json!({ "method": method, "sessionId": session_id, "nodeId": node_id }),
        );
        let scoped_key = self.scoped_idempotency_key(method, &idempotency);
        if let Some(result) = self.lookup_idempotent_result(&scoped_key, &request_hash)? {
            return Ok(result);
        }
        self.get_session(&session_id)?;
        let result = if method == "recorders.startRecording" {
            self.start_node_recording(&session_id, &node_id)?
        } else {
            if self
                .recorder_node_sessions
                .get(&node_id)
                .is_some_and(|owner| *owner != session_id)
            {
                return Err(ControlError::InvalidRequest(
                    "recorder node belongs to another session".into(),
                ));
            }
            self.stop_node_recording(&node_id)?
        };
        self.journal_idempotent_result(&scoped_key, method, &request_hash, &result)?;
        Ok(result)
    }

    pub(crate) fn dispatch_recorder(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and idempotencyKey are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let session_id = EntityId::new(session_id);
        if self.store.session(&session_id).is_none() {
            return Err(ControlError::InvalidRequest("session not found".into()));
        }
        let node_id = params
            .get("nodeId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .map(EntityId::new);
        let frame = params.get("frame").and_then(Value::as_u64);
        let idempotency = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|key| !key.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let request_hash = Self::request_hash(&json!({
            "method": method,
            "sessionId": session_id,
            "nodeId": node_id,
            "frame": frame,
        }));
        let scoped_key = self.scoped_idempotency_key(method, &idempotency);
        if let Some(result) = self.lookup_idempotent_result(&scoped_key, &request_hash)? {
            return Ok(result);
        }
        if let Some(node_id) = node_id {
            if self.recorder_node_sessions.get(&node_id) != Some(&session_id) {
                return Err(ControlError::InvalidRequest(
                    "recorder node is not attached to the session".into(),
                ));
            }
            let result = self.control_recorder_node(&node_id, method, frame)?;
            self.journal_idempotent_result(&scoped_key, method, &request_hash, &result)?;
            return Ok(result);
        }
        let restored = if self.recorders.contains_key(&session_id) {
            None
        } else {
            self.storage
                .as_ref()
                .and_then(|storage| {
                    storage
                        .load_recording_checkpoint(session_id.as_str())
                        .transpose()
                })
                .transpose()
                .map_err(storage_error)?
        };
        if let Some(checkpoint) = restored {
            let recorder = RecorderController::restore(checkpoint).map_err(|error| {
                ControlError::InvalidRequest(format!("recorder checkpoint is invalid: {error:?}"))
            })?;
            self.recorders.insert(session_id.clone(), recorder);
        }
        if method == "recorders.arm"
            && !self
                .recorders
                .get(&session_id)
                .is_some_and(|recorder| recorder_is_active(recorder.state()))
            && self
                .recorders
                .values()
                .chain(self.recorder_node_states.values())
                .filter(|recorder| recorder_is_active(recorder.state()))
                .count()
                >= MAX_ACTIVE_RECORDERS
        {
            return Err(ControlError::InvalidRequest(
                "active recorder limit reached".into(),
            ));
        }
        let mut recorder_candidate = self.recorders.get(&session_id).cloned().unwrap_or_default();
        match method {
            "recorders.arm" => recorder_candidate.arm(),
            "recorders.start" => recorder_candidate.start(
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
            ),
            "recorders.pause" => recorder_candidate.pause(
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
            ),
            "recorders.resume" => recorder_candidate.resume(
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
            ),
            "recorders.split" => recorder_candidate.split(
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
            ),
            "recorders.stop" => recorder_candidate.stop(
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
            ),
            _ => Err(audiorouter_recording::RecorderError::InvalidTransition {
                state: recorder_candidate.state(),
                action: "unknown",
            }),
        }
        .map_err(|error| {
            ControlError::InvalidRequest(format!("recorder transition failed: {error:?}"))
        })?;
        let mut worker_finalized = false;
        let mut finalized_recordings = Vec::new();
        if let Some(worker) = self.recorder_workers.get_mut(&session_id) {
            let worker_result = match method {
                "recorders.arm" => worker.arm(),
                "recorders.start" => worker.start(
                    frame
                        .ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
                ),
                "recorders.pause" => worker.pause(
                    frame
                        .ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
                ),
                "recorders.resume" => worker.resume(
                    frame
                        .ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
                ),
                "recorders.split" => worker.split(
                    frame
                        .ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
                ),
                "recorders.stop" => {
                    let frame = frame
                        .ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?;
                    let outcome = worker.finalize(frame).map_err(|error| {
                        self.recorders.entry(session_id.clone()).or_default().fail();
                        ControlError::InvalidRequest(format!(
                            "recorder finalization failed: {error}"
                        ))
                    })?;
                    if outcome.state != "completed"
                        || !outcome.file_finalized
                        || outcome.recoverable
                    {
                        self.recorders.entry(session_id.clone()).or_default().fail();
                        return Err(ControlError::InvalidRequest(
                            "recorder finalization did not produce a completed file".into(),
                        ));
                    }
                    worker_finalized = true;
                    finalized_recordings = worker.finalized_recordings();
                    Ok(())
                }
                _ => Err("method not found".into()),
            };
            worker_result.map_err(|error| {
                self.recorders.entry(session_id.clone()).or_default().fail();
                ControlError::InvalidRequest(format!("recorder worker transition failed: {error}"))
            })?;
        }
        self.recorders
            .insert(session_id.clone(), recorder_candidate);
        let recorder = self
            .recorders
            .get(&session_id)
            .expect("recorder candidate inserted above");
        let checkpoint = recorder.checkpoint();
        let result = json!({
            "sessionId": session_id,
            "state": recorder_state_name(recorder.state()),
            "parts": checkpoint.parts.iter().map(|part| json!({
                "index": part.index,
                "startFrame": part.start_frame,
                "endFrame": part.end_frame,
            })).collect::<Vec<_>>(),
            "pauses": checkpoint.pauses.iter().map(|pause| json!({
                "startFrame": pause.start_frame,
                "endFrame": pause.end_frame,
            })).collect::<Vec<_>>(),
            "lastFrame": checkpoint.last_frame,
        });
        if let Some(storage) = &self.storage {
            storage
                .save_recording_checkpoint(session_id.as_str(), &checkpoint)
                .map_err(storage_error)?;
            for recording in &finalized_recordings {
                storage.save_recording(recording).map_err(storage_error)?;
            }
        }
        self.journal_idempotent_result(&scoped_key, method, &request_hash, &result)?;
        let revision = self
            .store
            .session(&session_id)
            .map(|session| session.revision)
            .unwrap_or_default();
        self.events
            .append(revision, None, "recorder.changed", Some(session_id.clone()));
        if worker_finalized {
            self.recorder_workers.remove(&session_id);
        }
        Ok(result)
    }

    /// Drive the shared Time Shift buffer of a running node. Commands are
    /// queued lock-free and applied by the audio thread at the next block;
    /// the returned status therefore reflects the previous block.
    pub(crate) fn dispatch_time_shift_transport(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId, nodeId, and action are required".into())
        })?;
        let text = |name: &str| {
            params
                .get(name)
                .and_then(Value::as_str)
                .filter(|value| {
                    !value.is_empty() && value.len() <= audiorouter_domain::MAX_ENTITY_ID_BYTES
                })
                .ok_or_else(|| ControlError::InvalidRequest(format!("{name} is required")))
        };
        let session_id = text("sessionId")?;
        let node_id = text("nodeId")?;
        let command = match text("action")? {
            "pause" => Some(audiorouter_dsp::timeshift::TimeShiftCommand::Pause),
            "resume" => Some(audiorouter_dsp::timeshift::TimeShiftCommand::Resume),
            "back" => Some(audiorouter_dsp::timeshift::TimeShiftCommand::Back),
            "forward" => Some(audiorouter_dsp::timeshift::TimeShiftCommand::Forward),
            "live" => Some(audiorouter_dsp::timeshift::TimeShiftCommand::Live),
            "status" => None,
            _ => {
                return Err(ControlError::InvalidRequest(
                    "action must be pause, resume, back, forward, live, or status".into(),
                ))
            }
        };
        let state = audiorouter_engine::time_shift_state(session_id, node_id).ok_or_else(|| {
            ControlError::InvalidRequest(
                "Time Shift is available while its route is playing".into(),
            )
        })?;
        if let Some(command) = command {
            state.post(command);
        }
        let status = state.status();
        Ok(json!({
            "sessionId": session_id,
            "nodeId": node_id,
            "state": if status.paused { "paused" } else if status.delay_seconds > 0.0 { "delayed" } else { "live" },
            "delaySeconds": status.delay_seconds,
            "bufferedSeconds": status.buffered_seconds,
            "capacitySeconds": status.capacity_seconds,
        }))
    }
}
