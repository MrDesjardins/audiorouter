//! Safety and recovery: crash recovery, OS transitions, privacy mute, startup and quit.

use super::*;

impl ControlPlane {
    pub(crate) fn recovery_status(&mut self) -> Result<(usize, bool), ControlError> {
        let now = unix_epoch_seconds() as u64;
        if let Some(storage) = &self.storage {
            let status = storage.recovery_status(now).map_err(storage_error)?;
            return Ok((status.recent_crashes, status.safe_mode));
        }
        let recent_crashes = self.recovery_tracker.crash_count(now);
        Ok((
            recent_crashes,
            self.recovery_tracker.mode() == RecoveryMode::SafeMode,
        ))
    }

    /// Record one backend/runtime crash and return the bounded recovery
    /// decision that a future process supervisor must apply. This boundary
    /// records policy state only: it never creates a process, starts a session,
    /// resumes a route, or opens an audio stream.
    pub fn record_runtime_crash(
        &mut self,
        timestamp_seconds: u64,
    ) -> Result<RecoveryDecision, ControlError> {
        let (mode, recent_crashes) = if let Some(storage) = &self.storage {
            storage
                .record_recovery_crash(timestamp_seconds)
                .map_err(storage_error)?;
            let status = storage
                .recovery_status(timestamp_seconds)
                .map_err(storage_error)?;
            (
                if status.safe_mode {
                    RecoveryMode::SafeMode
                } else {
                    RecoveryMode::RestoreEligible
                },
                status.recent_crashes,
            )
        } else {
            let mode = self.recovery_tracker.record_crash(timestamp_seconds);
            let recent_crashes = self.recovery_tracker.crash_count(timestamp_seconds);
            (mode, recent_crashes)
        };
        let mut session_ids = if mode == RecoveryMode::SafeMode || recent_crashes == 0 {
            Vec::new()
        } else {
            self.runtimes
                .iter()
                .filter(|(id, runtime)| {
                    runtime.state() == RuntimeState::Running
                        && !self.native_worker_attached_to(id)
                        && !self.recorders.get(*id).is_some_and(|recorder| {
                            matches!(
                                recorder.state(),
                                RecorderState::Armed
                                    | RecorderState::Recording
                                    | RecorderState::Paused
                                    | RecorderState::Stopping
                            )
                        })
                })
                .map(|(id, _)| id.clone())
                .collect()
        };
        session_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        Ok(RecoveryDecision { mode, session_ids })
    }

    /// Apply one supervisor recovery decision to the portable runtime model.
    ///
    /// A crashed backend has no usable live runtime, so every currently
    /// running fake runtime is stopped before the policy result is applied.
    /// Only non-recording sessions returned by `record_runtime_crash` are
    /// restarted, and safe mode therefore leaves all sessions stopped. This
    /// method is deliberately limited to the fake runtime boundary: it does
    /// not create a process, open an audio stream, or claim native route
    /// recovery.
    pub fn recover_after_runtime_crash(
        &mut self,
        timestamp_seconds: u64,
    ) -> Result<RecoveryDecision, ControlError> {
        let decision = self.record_runtime_crash(timestamp_seconds)?;
        let crashed_session_ids = self
            .runtimes
            .iter()
            .filter(|(_, runtime)| runtime.state() == RuntimeState::Running)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for session_id in &crashed_session_ids {
            let revision = self.get_session(session_id)?.revision;
            self.events
                .append(revision, None, "runtime.crashed", Some(session_id.clone()));
        }
        for runtime in self.runtimes.values_mut() {
            if runtime.state() == RuntimeState::Running {
                runtime.stop();
            }
        }
        // Recovery is a fail-closed boundary for native ownership too. The
        // in-process supervisor path may retain worker objects while applying
        // the crash policy; stop and drop only workers owned by a crashed
        // session before any portable runtime is restarted. Dropping after a
        // stop attempt also releases the OS handles when a platform stop
        // reports an error, while the first error remains visible to the
        // supervisor.
        let mut native_recovery_error = None;
        for session_id in crashed_session_ids.clone() {
            if self.native_endpoint_session_is_attached(&session_id) {
                if let Some(worker) = self.native_endpoint_worker_for_session_mut(&session_id) {
                    if let Err(error) = worker.stop() {
                        native_recovery_error = Some(audio_control_error(error));
                    }
                }
                let _ = self.detach_native_endpoint_worker_for_session(&session_id);
            }
            if self.native_endpoint_session.as_ref() == Some(&session_id)
                && self.native_endpoint_worker.is_none()
            {
                self.native_endpoint_session = None;
                self.native_endpoint_taps = None;
            }
            if self.native_endpoint_session_secondary.as_ref() == Some(&session_id)
                && self.native_endpoint_worker_secondary.is_none()
            {
                self.native_endpoint_session_secondary = None;
                self.native_endpoint_taps_secondary = None;
            }
        }
        #[cfg(windows)]
        if self
            .native_output_fanout_session
            .as_ref()
            .is_some_and(|session_id| crashed_session_ids.contains(session_id))
        {
            if let Some(fanout) = self.native_output_fanout.as_mut() {
                if let Err(error) = fanout.stop() {
                    if native_recovery_error.is_none() {
                        native_recovery_error = Some(ControlError::InvalidRequest(format!(
                            "native output fan-out recovery stop failed: {error:?}"
                        )));
                    }
                }
            }
            self.native_output_fanout = None;
            self.native_output_fanout_session = None;
            self.native_output_fanout_generation = None;
        }
        #[cfg(windows)]
        if self
            .native_duplex_worker_session
            .as_ref()
            .is_some_and(|session_id| crashed_session_ids.contains(session_id))
        {
            if let Some(worker) = self.native_duplex_worker.as_mut() {
                if let Err(error) = worker.stop() {
                    if native_recovery_error.is_none() {
                        native_recovery_error = Some(ControlError::InvalidRequest(format!(
                            "native duplex recovery stop failed: {error:?}"
                        )));
                    }
                }
            }
            self.native_duplex_worker = None;
            self.native_duplex_worker_session = None;
            self.native_duplex_worker_generation = None;
        }
        #[cfg(windows)]
        if self
            .native_render_source_worker_session
            .as_ref()
            .is_some_and(|session_id| crashed_session_ids.contains(session_id))
        {
            if let Some(worker) = self.native_render_source_worker.as_mut() {
                if let Err(error) = worker.stop() {
                    if native_recovery_error.is_none() {
                        native_recovery_error = Some(ControlError::InvalidRequest(format!(
                            "native render-source recovery stop failed: {error:?}"
                        )));
                    }
                }
            }
            self.native_render_source_worker = None;
            self.native_render_source_worker_session = None;
            self.native_render_source_worker_generation = None;
            self.native_render_source_taps = None;
        }
        #[cfg(windows)]
        if self
            .native_multi_input_worker_session
            .as_ref()
            .is_some_and(|session_id| crashed_session_ids.contains(session_id))
        {
            if let Some(worker) = self.native_multi_input_worker.as_mut() {
                if let Err(error) = worker.stop() {
                    if native_recovery_error.is_none() {
                        native_recovery_error = Some(ControlError::InvalidRequest(format!(
                            "native multi-input recovery stop failed: {error:?}"
                        )));
                    }
                }
            }
            self.native_multi_input_worker = None;
            self.native_multi_input_worker_session = None;
            self.native_multi_input_worker_generation = None;
            self.native_multi_input_applied_generation = None;
            #[cfg(windows)]
            self.multi_input_application_sources.clear();
        }
        for session_id in &crashed_session_ids {
            self.deactivate_virtual_route_bridges(session_id);
        }
        if decision.mode == RecoveryMode::RestoreEligible {
            for session_id in &decision.session_ids {
                self.session_start(session_id)?;
            }
        }
        if let Some(error) = native_recovery_error {
            return Err(error);
        }
        Ok(decision)
    }

    /// Apply the side-effecting portion of an OS transition on the control
    /// thread. Sign-out and sleep stop sessions and release their ownership;
    /// resume only reports the previously running portable sessions that a
    /// platform adapter must re-enumerate and revalidate before calling
    /// `session_start`. Native and recording sessions are never resumed here.
    pub fn handle_os_transition(
        &mut self,
        transition: OsTransition,
    ) -> Result<Value, ControlError> {
        let mut running_sessions = self
            .runtimes
            .iter()
            .filter(|(_, runtime)| runtime.state() == RuntimeState::Running)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        running_sessions.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        let native_sessions = running_sessions
            .iter()
            .filter(|id| self.native_worker_attached_to(id))
            .cloned()
            .collect::<Vec<_>>();
        let recording_sessions = running_sessions
            .iter()
            .filter(|id| {
                self.recorders.get(*id).is_some_and(|recorder| {
                    matches!(
                        recorder.state(),
                        RecorderState::Armed
                            | RecorderState::Recording
                            | RecorderState::Paused
                            | RecorderState::Stopping
                    )
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        let decision = plan_os_transition(
            transition,
            &running_sessions,
            &native_sessions,
            &recording_sessions,
        );
        match transition {
            OsTransition::Lock => Ok(json!({
                "transition": "lock",
                "action": "keepRunning",
                "endpointInventory": "notStarted",
                "nativeSessionIds": Vec::<EntityId>::new(),
                "sessionIds": decision.session_ids,
            })),
            OsTransition::SignOut | OsTransition::Sleep => {
                if !recording_sessions.is_empty() {
                    return Err(ControlError::InvalidRequest(
                        "OS transition requires explicit recorder finalization".into(),
                    ));
                }
                let resumable = decision
                    .session_ids
                    .iter()
                    .filter(|id| !native_sessions.contains(id))
                    .cloned()
                    .collect::<Vec<_>>();
                self.os_suspended_native_sessions = if transition == OsTransition::Sleep {
                    native_sessions.clone()
                } else {
                    Vec::new()
                };
                for session_id in &decision.session_ids {
                    self.session_stop(session_id)?;
                }
                self.os_suspended_sessions = resumable;
                Ok(json!({
                    "transition": if transition == OsTransition::Sleep { "sleep" } else { "signOut" },
                    "action": "stopAndRelease",
                    "endpointInventory": "notStarted",
                    "nativeSessionIds": Vec::<EntityId>::new(),
                    "sessionIds": decision.session_ids,
                }))
            }
            OsTransition::Resume => {
                let endpoint_inventory = if let Some(monitor) = self.endpoint_monitor.as_mut() {
                    let endpoint_changes =
                        monitor.refresh_changes().map_err(audio_control_error)?;
                    self.retain_endpoint_changes(&endpoint_changes);
                    "refreshed"
                } else {
                    "notStarted"
                };
                let session_ids = std::mem::take(&mut self.os_suspended_sessions);
                let native_session_ids = std::mem::take(&mut self.os_suspended_native_sessions);
                let decision = plan_os_transition(transition, &session_ids, &[], &[]);
                Ok(json!({
                    "transition": "resume",
                    "action": if decision.session_ids.is_empty() && native_session_ids.is_empty() { "remainStopped" } else { "revalidateBeforeRestart" },
                    "endpointInventory": endpoint_inventory,
                    "nativeSessionIds": native_session_ids,
                    "sessionIds": decision.session_ids,
                }))
            }
        }
    }

    pub(crate) fn dispatch_system_quit(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|key| !key.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let request_hash = Self::request_hash(&json!({"action": "quit"}));
        let scoped_key = self.scoped_idempotency_key("system.quit", idempotency_key);
        if let Some(previous) = self.lookup_idempotent_result(&scoped_key, &request_hash)? {
            return Ok(previous);
        }

        let node_ids = self
            .recorder_node_sessions
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut finalized_recorders = Vec::new();
        for node_id in node_ids {
            let state = self
                .recorder_node_states
                .get(&node_id)
                .map(|recorder| recorder.state())
                .ok_or_else(|| {
                    ControlError::InvalidRequest("recorder node state is missing".into())
                })?;
            match state {
                RecorderState::Recording | RecorderState::Paused => {
                    let frame = self
                        .recorder_node_states
                        .get(&node_id)
                        .and_then(|recorder| {
                            let checkpoint = recorder.checkpoint();
                            checkpoint.stop_frame.or(checkpoint.last_frame)
                        })
                        .ok_or_else(|| {
                            ControlError::InvalidRequest(
                                "active recorder has no committed frame boundary".into(),
                            )
                        })?;
                    finalized_recorders.push(self.control_recorder_node(
                        &node_id,
                        "recorders.stop",
                        Some(frame),
                    )?);
                }
                RecorderState::Idle | RecorderState::Armed | RecorderState::Completed => {
                    self.recorder_node_workers.remove(&node_id);
                    self.sync_recorder_inlet(&node_id);
                    self.recorder_node_states.remove(&node_id);
                    self.recorder_node_sessions.remove(&node_id);
                }
                RecorderState::Stopping | RecorderState::Failed => {
                    return Err(ControlError::InvalidRequest(
                        "recorder node requires explicit recovery before quit".into(),
                    ));
                }
            }
        }

        let running_sessions = self
            .runtimes
            .iter()
            .filter(|(_, runtime)| runtime.state() == RuntimeState::Running)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let mut stopped_sessions = Vec::with_capacity(running_sessions.len());
        for session_id in running_sessions {
            stopped_sessions.push(self.session_stop(&session_id)?);
        }
        let result = json!({
            "state": "stopped",
            "sessions": stopped_sessions,
            "recorders": finalized_recorders,
        });
        self.journal_idempotent_result(&scoped_key, "system.quit", &request_hash, &result)?;
        Ok(result)
    }

    pub(crate) fn dispatch_os_transition(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("transition and idempotencyKey are required".into())
        })?;
        let transition_name = params
            .get("transition")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("transition is required".into()))?;
        let transition = match transition_name {
            "lock" => OsTransition::Lock,
            "signOut" => OsTransition::SignOut,
            "sleep" => OsTransition::Sleep,
            "resume" => OsTransition::Resume,
            _ => {
                return Err(ControlError::InvalidRequest(
                    "transition must be lock, signOut, sleep, or resume".into(),
                ))
            }
        };
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("system.osTransition", idempotency_key),
            Self::request_hash(&json!({ "transition": transition_name })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        let result = self.handle_os_transition(transition)?;
        self.journal_idempotent_result(&operation.0, "system.osTransition", &operation.1, &result)?;
        Ok(result)
    }

    pub(crate) fn dispatch_privacy_mute(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("muted and idempotencyKey are required".into())
        })?;
        let muted = params
            .get("muted")
            .and_then(Value::as_bool)
            .ok_or_else(|| ControlError::InvalidRequest("muted is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("safety.setPrivacyMute", idempotency_key),
            Self::request_hash(&json!({ "muted": muted })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        if let Some(storage) = &self.storage {
            storage.save_privacy_mute(muted).map_err(storage_error)?;
        }
        self.privacy_muted = muted;
        if let Some(worker) = self.native_endpoint_worker.as_ref() {
            worker.set_privacy_muted(muted);
        }
        if let Some(worker) = self.native_endpoint_worker_secondary.as_ref() {
            worker.set_privacy_muted(muted);
        }
        #[cfg(windows)]
        if let Some(worker) = self.native_multi_input_worker.as_ref() {
            worker.set_privacy_muted(muted);
        }
        #[cfg(windows)]
        if let Some(worker) = self.native_render_source_worker.as_ref() {
            worker.set_privacy_muted(muted);
        }
        #[cfg(windows)]
        if let Some(worker) = self.native_duplex_worker.as_ref() {
            worker.set_privacy_muted(muted);
        }
        self.events.append(
            0,
            None,
            if muted {
                "privacy.muteEnabled"
            } else {
                "privacy.muteDisabled"
            },
            None,
        );
        let result = json!({
            "muted": muted,
            "persistence": if self.storage.is_some() { "durable" } else { "memory" },
            "audioEffect": "process-local-when-realtime-backend-is-available"
        });
        self.journal_idempotent_result(
            &operation.0,
            "safety.setPrivacyMute",
            &operation.1,
            &result,
        )?;
        Ok(result)
    }

    pub(crate) fn dispatch_recovery_clear(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("recovery.clearSafeMode", idempotency_key),
            Self::request_hash(&json!({ "action": "clear" })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        let result = json!({
            "safeMode": false,
            "recentCrashes": 0,
            "persistence": if self.storage.is_some() { "durable" } else { "memory" }
        });
        if let Some(storage) = &self.storage {
            let encoded = serde_json::to_string(&result)
                .map_err(|error| ControlError::Json(error.to_string()))?;
            storage
                .clear_recovery_crashes_and_journal(
                    &operation.0,
                    "recovery.clearSafeMode",
                    &encoded,
                    &operation.1,
                )
                .map_err(storage_error)?;
            self.remember_operation_outcome(
                &operation.0,
                result.clone(),
                "recovery.clearSafeMode",
                Some(&operation.1),
            );
        } else {
            self.recovery_tracker.clear_after_stable_run();
            self.remember_operation_outcome(
                &operation.0,
                result.clone(),
                "recovery.clearSafeMode",
                Some(&operation.1),
            );
        }
        self.events
            .append(0, None, "recovery.safeModeCleared", None);
        Ok(result)
    }

    pub(crate) fn dispatch_startup_plan(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let enabled = params
            .as_ref()
            .and_then(|value| value.get("enabled"))
            .and_then(Value::as_bool)
            .ok_or_else(|| ControlError::InvalidRequest("enabled is required".into()))?;
        let now = Instant::now();
        self.startup_plans
            .retain(|_, (_, expires_at)| *expires_at > now);
        if self.startup_plans.len() >= MAX_PENDING_PLAN_RECORDS {
            return Err(ControlError::InvalidRequest(
                "too many pending startup plans".into(),
            ));
        }
        let timestamp_millis = unix_epoch_millis();
        let plan_id = allocate_timestamped_plan_id(
            "startup-plan",
            timestamp_millis,
            &mut self.next_startup_plan,
            |id| self.startup_plans.contains_key(id),
            "startup plan ID space is exhausted",
        )?;
        self.startup_plans.insert(
            plan_id.clone(),
            (enabled, Instant::now() + VIRTUAL_DEVICE_PLAN_TTL),
        );
        if let Some(storage) = &self.storage {
            let expires_at = unix_epoch_seconds() + VIRTUAL_DEVICE_PLAN_TTL.as_secs() as i64;
            if let Err(error) = storage.save_startup_plan(&plan_id, enabled, expires_at) {
                self.startup_plans.remove(&plan_id);
                return Err(storage_error(error));
            }
        }
        Ok(json!({
            "planId": plan_id,
            "enabled": enabled,
            "registration": "unavailable",
            "reason": "native sign-in registration is owned by the desktop shell",
            "requiredScopes": ["startupWrite"],
            "warnings": ["planning does not change Windows startup registration"]
        }))
    }

    pub(crate) fn dispatch_startup_apply(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?;
        let plan_id = params
            .get("planId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let plan_id_value = EntityId::new(plan_id);
        let scoped_key = self.scoped_idempotency_key("startup.apply", idempotency_key);
        // The plan is consumed after a journaled attempt, so check the
        // durable idempotency record before looking up the plan. This keeps a
        // retry safe across backend restart without allowing a new key to
        // replay the consumed authorization preview.
        let request_hash = Self::request_hash(&json!({ "planId": plan_id }));
        if let Some(previous) = self.lookup_idempotent_result(&scoped_key, &request_hash)? {
            self.remember_operation_outcome(
                &scoped_key,
                previous.clone(),
                "startup.apply",
                Some(&request_hash),
            );
            return Ok(previous);
        }
        let Some((enabled, expires_at)) = self.startup_plans.get(&plan_id_value).copied() else {
            return Err(ControlError::InvalidRequest(
                "startup plan was not found".into(),
            ));
        };
        if Instant::now() >= expires_at {
            self.startup_plans.remove(&plan_id_value);
            return Err(ControlError::InvalidRequest(
                "startup plan has expired".into(),
            ));
        }
        let result = json!({
            "planId": plan_id,
            "state": "unavailable",
            "registration": "unavailable",
            "reason": "native sign-in registration is owned by the desktop shell"
        });
        if let Some(storage) = self.storage.as_mut() {
            // Keep desired state, idempotency, and one-shot plan consumption
            // in one durable transaction. The in-memory fields are updated
            // only after the durable commit succeeds.
            storage
                .commit_startup_apply(
                    &plan_id_value,
                    enabled,
                    &scoped_key,
                    "startup.apply",
                    &result.to_string(),
                    &request_hash,
                )
                .map_err(storage_error)?;
        }
        self.startup_enabled = enabled;
        self.journal_idempotent_result(&scoped_key, "startup.apply", &request_hash, &result)?;
        // A plan is a short-lived authorization preview, not a durable
        // desired-state record. Consume it after the apply attempt has been
        // journaled, including the fail-closed unavailable result. This keeps
        // a stale plan from being replayed after a backend restart while the
        // idempotency journal still provides the documented retry result.
        self.startup_plans.remove(&plan_id_value);
        Ok(result)
    }
}
