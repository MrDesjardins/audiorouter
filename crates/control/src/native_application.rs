//! Application capture: application workers and their liveness maintenance.

use super::*;

/// Explicit inputs for preparing a verified process-loopback worker. The
/// caller supplies the observed process identity and exact render binding;
/// preparation never chooses a default endpoint.
pub struct NativeApplicationWorkerConfig<'a> {
    pub process_id: u32,
    pub expected_executable: &'a str,
    pub expected_executable_path: Option<&'a str>,
    pub expected_creation_time_100ns: u64,
    pub mode: audiorouter_windows_audio::ProcessLoopbackMode,
    pub render: &'a audiorouter_windows_audio::EndpointInfo,
    pub buffer_duration_100ns: i64,
    pub max_attempts: u32,
    pub retry_delay_ms: u64,
}

/// Restart tracking for one application source feeding the native
/// multi-input Mixer. While the application is closed its input carries
/// silence so the other sources keep playing.
#[cfg(windows)]
pub(crate) struct MultiInputApplicationSource {
    pub(crate) session_id: EntityId,
    pub(crate) node_id: EntityId,
    pub(crate) input_index: usize,
    pub(crate) executable: String,
    /// Persisted selector from the node; restart matching starts here.
    pub(crate) executable_path: Option<String>,
    pub(crate) current_executable_path: Option<String>,
    pub(crate) process_id: u32,
    pub(crate) creation_time_100ns: u64,
    pub(crate) mode: audiorouter_windows_audio::ProcessLoopbackMode,
    pub(crate) state: &'static str,
    pub(crate) detail: &'static str,
    pub(crate) next_probe_at: Instant,
    pub(crate) retry_delay: Duration,
}

#[derive(Clone)]
pub(crate) struct ApplicationCaptureRuntime {
    pub(crate) session_id: EntityId,
    node_id: EntityId,
    executable: String,
    /// Persisted selector from the graph node; restart matching starts here.
    executable_path: Option<String>,
    /// Observed path of the process currently bound, which can differ from
    /// the selector after a self-updating app moves to a new version folder.
    current_executable_path: Option<String>,
    selected_process_id: u32,
    selected_creation_time_100ns: u64,
    process_id: u32,
    creation_time_100ns: u64,
    mode: audiorouter_windows_audio::ProcessLoopbackMode,
    state: &'static str,
    detail: &'static str,
    next_probe_at: Instant,
    retry_delay: Duration,
}

impl ControlPlane {
    /// Attach an already-opened process-loopback worker to a session. The
    /// worker must have been created from a currently verified application
    /// identity and remains stopped until the session is started.
    pub fn attach_native_application_worker(
        &mut self,
        session_id: EntityId,
        worker: audiorouter_windows_audio::ProcessLoopbackWorker,
    ) -> Result<(), ControlError> {
        self.get_session(&session_id)?;
        if self.any_native_worker_attached() {
            return Err(ControlError::InvalidRequest(
                "native worker is already attached".into(),
            ));
        }
        if worker.is_running() {
            return Err(ControlError::InvalidRequest(
                "native application worker must be stopped before attachment".into(),
            ));
        }
        worker.set_privacy_muted(self.privacy_muted);
        self.native_endpoint_worker =
            Some(audiorouter_windows_audio::NativeAudioWorker::ProcessLoopback(worker));
        self.native_endpoint_session = Some(session_id);
        Ok(())
    }

    /// Prepare a process-loopback worker after revalidating the exact
    /// executable, path, PID, and creation-time identity. Only the supplied
    /// render endpoint is opened; no default endpoint is selected.
    pub fn prepare_native_application_worker(
        &mut self,
        session_id: EntityId,
        config: NativeApplicationWorkerConfig<'_>,
    ) -> Result<(), ControlError> {
        let session = self.get_session(&session_id)?.clone();
        let process_id_value = serde_json::Value::from(u64::from(config.process_id));
        let creation_time_value =
            serde_json::Value::from(config.expected_creation_time_100ns.to_string());
        let matching_capture_node = session.nodes.iter().find(|node| {
            node.enabled
                && node.kind == NodeKind::ApplicationCapture
                && node.parameters.get("processPolicy").and_then(Value::as_str)
                    == Some("selectedInstance")
                && node
                    .parameters
                    .get("processId")
                    .is_some_and(|value| value == &process_id_value)
                && node
                    .parameters
                    .get("executable")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case(config.expected_executable))
                && node
                    .parameters
                    .get("creationTime100ns")
                    .is_some_and(|value| value == &creation_time_value)
        });
        let Some(matching_capture_node) = matching_capture_node else {
            return Err(ControlError::InvalidRequest(
                "application worker requires a matching enabled applicationCapture node".into(),
            ));
        };
        if self.any_native_worker_attached() {
            return Err(ControlError::InvalidRequest(
                "native worker is already attached".into(),
            ));
        }
        if config.render.direction != audiorouter_windows_audio::EndpointDirection::Render
            || !config.render.is_ieee_float32()
            || config.render.channels != 2
        {
            return Err(ControlError::InvalidRequest(
                "process-loopback render binding must be stereo IEEE float32".into(),
            ));
        }
        let application = audiorouter_windows_audio::bind_application_or_restarted(
            config.process_id,
            config.expected_executable,
            config.expected_executable_path,
            config.expected_creation_time_100ns,
        )
        .map_err(audio_control_error)?;
        let application_creation_time_100ns = application
            .creation_time_100ns
            .unwrap_or(config.expected_creation_time_100ns);
        let capture = audiorouter_windows_audio::ProcessLoopbackCapture::open(
            application.process_id,
            config.mode,
        )
        .map_err(audio_control_error)?;
        let monitor = if let Some(monitor) = self.endpoint_monitor.as_mut() {
            monitor
        } else {
            self.endpoint_monitor = Some(
                audiorouter_windows_audio::EndpointMonitor::start().map_err(audio_control_error)?,
            );
            self.endpoint_monitor.as_mut().expect("monitor initialized")
        };
        let render_client =
            audiorouter_windows_audio::SharedRender::open_refreshed_bound_with_retry(
                monitor,
                config.render,
                config.buffer_duration_100ns,
                config.max_attempts,
                config.retry_delay_ms,
            )
            .map_err(audio_control_error)?;
        let bridge = audiorouter_windows_audio::WasapiSchedulerBridge::new(
            8,
            2,
            audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
            audiorouter_windows_audio::MAX_PROCESS_LOOPBACK_PACKET_FRAMES as usize,
        )
        .map_err(audio_control_error)?;
        self.attach_native_application_worker(
            session_id,
            audiorouter_windows_audio::ProcessLoopbackWorker::new(capture, render_client, bridge),
        )?;
        self.application_capture_runtime = Some(ApplicationCaptureRuntime {
            session_id: session.id.clone(),
            node_id: matching_capture_node.id.clone(),
            executable: config.expected_executable.to_owned(),
            executable_path: config.expected_executable_path.map(str::to_owned),
            current_executable_path: application.executable_path.clone(),
            selected_process_id: config.process_id,
            selected_creation_time_100ns: config.expected_creation_time_100ns,
            process_id: application.process_id,
            creation_time_100ns: application_creation_time_100ns,
            mode: config.mode,
            state: "configured-stopped",
            detail: "Prepared for this application. Start the route to capture audio.",
            next_probe_at: Instant::now() + APPLICATION_CAPTURE_LIVENESS_POLL,
            retry_delay: APPLICATION_CAPTURE_RETRY_MIN,
        });
        Ok(())
    }

    pub(crate) fn set_application_capture_state(
        &mut self,
        state: &'static str,
        detail: &'static str,
    ) {
        let Some(binding) = self.application_capture_runtime.as_mut() else {
            return;
        };
        if binding.state == state && binding.detail == detail {
            return;
        }
        binding.state = state;
        binding.detail = detail;
        let session_id = binding.session_id.clone();
        let revision = self
            .store
            .session(&session_id)
            .map_or(0, |session| session.revision);
        self.events.append(
            revision,
            None,
            "application.captureStateChanged",
            Some(session_id),
        );
    }

    pub(crate) fn application_capture_states(&self) -> Value {
        let running = |session_id: &EntityId| {
            self.runtimes
                .get(session_id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
        };
        let stopped = (
            "configured-stopped",
            "Prepared for this application. Start the route to capture audio.",
        );
        let mut states = Vec::new();
        if let Some(binding) = self.application_capture_runtime.as_ref() {
            let (state, detail) = if running(&binding.session_id) {
                (binding.state, binding.detail)
            } else {
                stopped
            };
            states.push(json!({
                "sessionId": binding.session_id,
                "nodeId": binding.node_id,
                "state": state,
                "detail": detail,
            }));
        }
        #[cfg(windows)]
        for source in &self.multi_input_application_sources {
            let (state, detail) = if running(&source.session_id) {
                (source.state, source.detail)
            } else {
                stopped
            };
            states.push(json!({
                "sessionId": source.session_id,
                "nodeId": source.node_id,
                "state": state,
                "detail": detail,
            }));
        }
        Value::Array(states)
    }

    #[cfg(windows)]
    pub(crate) fn application_capture_runtime_is_current(
        &self,
        binding: &ApplicationCaptureRuntime,
    ) -> bool {
        self.store
            .session(&binding.session_id)
            .is_some_and(|session| {
                session.nodes.iter().any(|node| {
                    node.id == binding.node_id
                        && node.kind == NodeKind::ApplicationCapture
                        && node.enabled
                        && node.parameters.get("processPolicy").and_then(Value::as_str)
                            == Some("selectedInstance")
                        && node
                            .parameters
                            .get("executable")
                            .and_then(Value::as_str)
                            .is_some_and(|value| value.eq_ignore_ascii_case(&binding.executable))
                        && node
                            .parameters
                            .get("executablePath")
                            .and_then(Value::as_str)
                            .is_some_and(|value| {
                                binding
                                    .executable_path
                                    .as_deref()
                                    .is_some_and(|expected| value.eq_ignore_ascii_case(expected))
                            })
                        && node.parameters.get("processId").and_then(Value::as_u64)
                            == Some(u64::from(binding.selected_process_id))
                        && node
                            .parameters
                            .get("creationTime100ns")
                            .and_then(Value::as_str)
                            .and_then(|value| value.parse::<u64>().ok())
                            == Some(binding.selected_creation_time_100ns)
                })
            })
    }

    /// Observe process exit and reconnect only to a unique full-path match.
    /// Called from the bounded control-plane audio pump, never from the audio
    /// callback. One process snapshot per second bounds discovery overhead.
    #[cfg(windows)]
    pub(crate) fn maintain_application_capture(
        &mut self,
        session_id: &EntityId,
        force_probe: bool,
    ) -> Result<bool, ControlError> {
        let now = Instant::now();
        let Some(mut binding) = self
            .application_capture_runtime
            .clone()
            .filter(|binding| &binding.session_id == session_id)
        else {
            return Ok(false);
        };
        if !self
            .runtimes
            .get(session_id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
        {
            self.set_application_capture_state(
                "configured-stopped",
                "Prepared for this application. Start the route to capture audio.",
            );
            return Ok(false);
        }
        if !force_probe && now < binding.next_probe_at {
            return Ok(binding.state != "connected");
        }
        binding.next_probe_at = now + APPLICATION_CAPTURE_LIVENESS_POLL;
        if !force_probe
            && matches!(binding.state, "connected" | "configured-stopped")
            && self.application_capture_runtime_is_current(&binding)
            && binding
                .current_executable_path
                .as_deref()
                .is_some_and(|path| {
                    audiorouter_windows_audio::application_identity_is_current(
                        binding.process_id,
                        binding.creation_time_100ns,
                        path,
                    )
                    .unwrap_or(false)
                })
        {
            self.set_application_capture_state(
                "connected",
                "Connected to this application. Audio flow appears when the app produces sound.",
            );
            if let Some(current) = self.application_capture_runtime.as_mut() {
                current.next_probe_at = binding.next_probe_at;
            }
            return Ok(false);
        }
        let applications = match audiorouter_windows_audio::enumerate_applications() {
            Ok(applications) => applications,
            Err(_) => {
                if let Some(worker) = self.native_endpoint_worker_for_session_mut(session_id) {
                    let _ = worker.stop();
                }
                self.set_application_capture_state(
                    "failed",
                    "Windows could not check this application. Audio is paused while AudioRouter retries.",
                );
                if let Some(current) = self.application_capture_runtime.as_mut() {
                    current.next_probe_at = now + binding.retry_delay;
                    current.retry_delay =
                        (binding.retry_delay * 2).min(APPLICATION_CAPTURE_RETRY_MAX);
                }
                return Ok(true);
            }
        };
        if !self.application_capture_runtime_is_current(&binding) {
            if let Some(worker) = self.native_endpoint_worker_for_session_mut(session_id) {
                let _ = worker.stop();
            }
            self.set_application_capture_state(
                "unsupported",
                "The application source changed while audio was running. Stop the route and prepare the updated source again.",
            );
            if let Some(current) = self.application_capture_runtime.as_mut() {
                current.next_probe_at = now + APPLICATION_CAPTURE_RETRY_MAX;
            }
            return Ok(true);
        }
        let current_is_alive = applications.iter().any(|application| {
            application.process_id == binding.process_id
                && application.creation_time_100ns == Some(binding.creation_time_100ns)
                && application
                    .executable
                    .eq_ignore_ascii_case(&binding.executable)
                && binding
                    .current_executable_path
                    .as_deref()
                    .is_some_and(|expected| {
                        application
                            .executable_path
                            .as_deref()
                            .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
                    })
        });
        if current_is_alive {
            self.set_application_capture_state(
                "connected",
                "Connected to this application. Audio flow appears when the app produces sound.",
            );
            if let Some(current) = self.application_capture_runtime.as_mut() {
                current.next_probe_at = now + APPLICATION_CAPTURE_LIVENESS_POLL;
                current.retry_delay = APPLICATION_CAPTURE_RETRY_MIN;
            }
            return Ok(false);
        }

        if self
            .native_endpoint_worker_for_session(session_id)
            .is_some_and(audiorouter_windows_audio::NativeAudioWorker::is_running)
        {
            if let Some(worker) = self.native_endpoint_worker_for_session_mut(session_id) {
                worker.stop().map_err(audio_control_error)?;
            }
        }
        if binding.executable_path.is_none() {
            self.set_application_capture_state(
                "unsupported",
                "This app cannot be safely matched after restart. Choose it again from the application picker.",
            );
            if let Some(current) = self.application_capture_runtime.as_mut() {
                current.next_probe_at = now + APPLICATION_CAPTURE_RETRY_MAX;
            }
            return Ok(true);
        }

        match audiorouter_windows_audio::resolve_application_restart_with_path(
            &applications,
            &binding.executable,
            binding.executable_path.as_deref(),
        ) {
            Err(audiorouter_windows_audio::AudioError::ApplicationRestartNotFound { .. }) => {
                self.set_application_capture_state(
                    "app-closed",
                    "Application is closed. Audio is silent; AudioRouter will reconnect when it starts.",
                );
                if let Some(current) = self.application_capture_runtime.as_mut() {
                    current.next_probe_at = now + APPLICATION_CAPTURE_RETRY_MIN;
                    current.retry_delay = APPLICATION_CAPTURE_RETRY_MIN;
                }
                Ok(true)
            }
            Err(audiorouter_windows_audio::AudioError::ApplicationRestartAmbiguous { .. }) => {
                self.set_application_capture_state(
                    "ambiguous",
                    "More than one matching app is running. Close extra instances or choose the intended one again.",
                );
                if let Some(current) = self.application_capture_runtime.as_mut() {
                    current.next_probe_at = now + APPLICATION_CAPTURE_LIVENESS_POLL;
                }
                Ok(true)
            }
            Err(audiorouter_windows_audio::AudioError::ApplicationRestartIdentityUnavailable {
                ..
            }) => {
                self.set_application_capture_state(
                    "unsupported",
                    "Windows cannot verify this app identity. Choose it again from the application picker.",
                );
                if let Some(current) = self.application_capture_runtime.as_mut() {
                    current.next_probe_at = now + APPLICATION_CAPTURE_RETRY_MAX;
                }
                Ok(true)
            }
            Err(_) => {
                self.set_application_capture_state(
                    "failed",
                    "Application capture could not be checked. Audio remains silent while AudioRouter retries.",
                );
                if let Some(current) = self.application_capture_runtime.as_mut() {
                    current.next_probe_at = now + binding.retry_delay;
                    current.retry_delay =
                        (binding.retry_delay * 2).min(APPLICATION_CAPTURE_RETRY_MAX);
                }
                Ok(true)
            }
            Ok(application) => {
                self.set_application_capture_state(
                    "reconnecting",
                    "Application started. Reconnecting its audio source.",
                );
                let Some(creation_time_100ns) = application.creation_time_100ns else {
                    self.set_application_capture_state(
                        "unsupported",
                        "Windows cannot verify this app identity. Choose it again from the application picker.",
                    );
                    if let Some(current) = self.application_capture_runtime.as_mut() {
                        current.next_probe_at = now + APPLICATION_CAPTURE_RETRY_MAX;
                    }
                    return Ok(true);
                };
                let capture = audiorouter_windows_audio::bind_application_with_path(
                    application.process_id,
                    &binding.executable,
                    application.executable_path.as_deref(),
                    Some(creation_time_100ns),
                )
                .and_then(|_| {
                    audiorouter_windows_audio::ProcessLoopbackCapture::open(
                        application.process_id,
                        binding.mode,
                    )
                });
                match capture {
                    Ok(capture) => {
                        let rebind_result = self
                            .native_endpoint_worker_for_session_mut(session_id)
                            .ok_or_else(|| "application worker is no longer attached".to_owned())
                            .and_then(|worker| {
                                worker.replace_process_capture(capture).map_err(|_| {
                                    "could not replace the process capture".to_owned()
                                })?;
                                if !worker.is_running() {
                                    worker.start().map_err(|_| {
                                        "could not restart the audio worker".to_owned()
                                    })?;
                                }
                                Ok(())
                            });
                        match rebind_result {
                            Ok(()) => {
                                if let Some(current) = self.application_capture_runtime.as_mut() {
                                    current.process_id = application.process_id;
                                    current.creation_time_100ns = creation_time_100ns;
                                    current.current_executable_path =
                                        application.executable_path.clone();
                                    current.next_probe_at = now + APPLICATION_CAPTURE_LIVENESS_POLL;
                                    current.retry_delay = APPLICATION_CAPTURE_RETRY_MIN;
                                }
                                self.set_application_capture_state(
                                    "connected",
                                    "Connected to the restarted application. Audio flow appears when it produces sound.",
                                );
                                Ok(false)
                            }
                            Err(_) => {
                                self.set_application_capture_state(
                                    "failed",
                                    "The app is running, but Windows could not restart its audio route. AudioRouter will retry.",
                                );
                                if let Some(current) = self.application_capture_runtime.as_mut() {
                                    current.next_probe_at = now + binding.retry_delay;
                                    current.retry_delay = (binding.retry_delay * 2)
                                        .min(APPLICATION_CAPTURE_RETRY_MAX);
                                }
                                Ok(true)
                            }
                        }
                    }
                    Err(_) => {
                        self.set_application_capture_state(
                            "failed",
                            "The app is running, but Windows could not reopen its audio source. AudioRouter will retry.",
                        );
                        if let Some(current) = self.application_capture_runtime.as_mut() {
                            current.next_probe_at = now + binding.retry_delay;
                            current.retry_delay =
                                (binding.retry_delay * 2).min(APPLICATION_CAPTURE_RETRY_MAX);
                        }
                        Ok(true)
                    }
                }
            }
        }
    }

    /// Keep each application source of a running multi-input Mixer bound to
    /// its application across restarts (CAP-06/CAP-11). Called from the
    /// control-plane pump, never the audio callback; connected inputs use
    /// direct identity checks, with full inventory reserved for recovery.
    /// An exited application's input is replaced by
    /// silence so the other sources keep playing, then reattached to the
    /// unique matching restarted instance. Returns whether any input changed.
    #[cfg(windows)]
    pub(crate) fn maintain_multi_input_applications(
        &mut self,
        session_id: &EntityId,
        force: bool,
    ) -> bool {
        let now = Instant::now();
        let due = |source: &MultiInputApplicationSource| {
            &source.session_id == session_id && (force || source.next_probe_at <= now)
        };
        if !self.multi_input_application_sources.iter().any(due) {
            return false;
        }
        // Routine liveness must not enumerate every process on the thread
        // that services WASAPI. Scan only when a bound source needs recovery.
        let all_current = !force
            && self
                .multi_input_application_sources
                .iter()
                .filter(|source| due(source))
                .all(|source| {
                    matches!(source.state, "connected" | "configured-stopped")
                        && self
                            .native_multi_input_worker
                            .as_ref()
                            .is_some_and(|worker| !worker.capture_is_silent(source.input_index))
                        && source
                            .current_executable_path
                            .as_deref()
                            .is_some_and(|path| {
                                audiorouter_windows_audio::application_identity_is_current(
                                    source.process_id,
                                    source.creation_time_100ns,
                                    path,
                                )
                                .unwrap_or(false)
                            })
                });
        if all_current {
            for source in self
                .multi_input_application_sources
                .iter_mut()
                .filter(|source| due(source))
            {
                source.state = "connected";
                source.detail = "Connected to this application. Audio flow appears when the app produces sound.";
                source.next_probe_at = now + APPLICATION_CAPTURE_LIVENESS_POLL;
                source.retry_delay = APPLICATION_CAPTURE_RETRY_MIN;
            }
            return false;
        }
        let applications = match audiorouter_windows_audio::enumerate_applications() {
            Ok(applications) => applications,
            Err(_) => {
                for source in self
                    .multi_input_application_sources
                    .iter_mut()
                    .filter(|source| due(source))
                {
                    source.state = "failed";
                    source.detail =
                        "Windows could not check this application. AudioRouter will retry.";
                    source.next_probe_at = now + source.retry_delay;
                    source.retry_delay =
                        (source.retry_delay * 2).min(APPLICATION_CAPTURE_RETRY_MAX);
                }
                return false;
            }
        };
        let mut changed = false;
        for index in 0..self.multi_input_application_sources.len() {
            if !due(&self.multi_input_application_sources[index]) {
                continue;
            }
            let (
                input_index,
                executable,
                executable_path,
                current_path,
                process_id,
                creation_time,
                mode,
            ) = {
                let source = &self.multi_input_application_sources[index];
                (
                    source.input_index,
                    source.executable.clone(),
                    source.executable_path.clone(),
                    source.current_executable_path.clone(),
                    source.process_id,
                    source.creation_time_100ns,
                    source.mode,
                )
            };
            let Some(worker) = self.native_multi_input_worker.as_mut() else {
                return changed;
            };
            let silent = worker.capture_is_silent(input_index);
            let alive = applications.iter().any(|application| {
                application.process_id == process_id
                    && application.creation_time_100ns == Some(creation_time)
                    && application.executable.eq_ignore_ascii_case(&executable)
                    && current_path.as_deref().is_some_and(|expected| {
                        application
                            .executable_path
                            .as_deref()
                            .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
                    })
            });
            let update =
                |sources: &mut Vec<MultiInputApplicationSource>, state, detail, delay: Duration| {
                    let source = &mut sources[index];
                    source.state = state;
                    source.detail = detail;
                    source.next_probe_at = now + delay;
                };
            if alive && !silent {
                update(
                    &mut self.multi_input_application_sources,
                    "connected",
                    "Connected to this application. Audio flow appears when the app produces sound.",
                    APPLICATION_CAPTURE_LIVENESS_POLL,
                );
                self.multi_input_application_sources[index].retry_delay =
                    APPLICATION_CAPTURE_RETRY_MIN;
                continue;
            }
            if !silent {
                let silence = audiorouter_windows_audio::MultiInputCaptureSource::Silence(
                    audiorouter_windows_audio::SilentCapture::new(
                        audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
                    ),
                );
                if worker.replace_capture(input_index, silence).is_ok() {
                    changed = true;
                }
            }
            if executable_path.is_none() {
                update(
                    &mut self.multi_input_application_sources,
                    "unsupported",
                    "This app cannot be safely matched after restart. Choose it again in the node properties.",
                    APPLICATION_CAPTURE_RETRY_MAX,
                );
                continue;
            }
            match audiorouter_windows_audio::resolve_application_restart_with_path(
                &applications,
                &executable,
                executable_path.as_deref(),
            ) {
                Err(audiorouter_windows_audio::AudioError::ApplicationRestartNotFound { .. }) => update(
                    &mut self.multi_input_application_sources,
                    "app-closed",
                    "Application is closed. Its input is silent; the other sources keep playing. AudioRouter reconnects when it starts.",
                    APPLICATION_CAPTURE_RETRY_MIN,
                ),
                Err(audiorouter_windows_audio::AudioError::ApplicationRestartAmbiguous { .. }) => update(
                    &mut self.multi_input_application_sources,
                    "ambiguous",
                    "More than one matching app is running. Close extra instances or choose the intended one again.",
                    APPLICATION_CAPTURE_LIVENESS_POLL,
                ),
                Err(audiorouter_windows_audio::AudioError::ApplicationRestartIdentityUnavailable { .. }) => update(
                    &mut self.multi_input_application_sources,
                    "unsupported",
                    "Windows cannot verify this app identity. Choose it again in the node properties.",
                    APPLICATION_CAPTURE_RETRY_MAX,
                ),
                Err(_) => {
                    let delay = self.multi_input_application_sources[index].retry_delay;
                    update(
                        &mut self.multi_input_application_sources,
                        "failed",
                        "Application capture could not be checked. Its input is silent while AudioRouter retries.",
                        delay,
                    );
                    self.multi_input_application_sources[index].retry_delay =
                        (delay * 2).min(APPLICATION_CAPTURE_RETRY_MAX);
                }
                Ok(application) => {
                    let reopened = audiorouter_windows_audio::bind_application_with_path(
                        application.process_id,
                        &executable,
                        application.executable_path.as_deref(),
                        application.creation_time_100ns,
                    )
                    .and_then(|_| audiorouter_windows_audio::ProcessLoopbackCapture::open(application.process_id, mode));
                    let replaced = match (reopened, self.native_multi_input_worker.as_mut()) {
                        (Ok(capture), Some(worker)) => worker
                            .replace_capture(
                                input_index,
                                audiorouter_windows_audio::MultiInputCaptureSource::ApplicationLoopback(capture),
                            )
                            .is_ok(),
                        _ => false,
                    };
                    if replaced {
                        changed = true;
                        let source = &mut self.multi_input_application_sources[index];
                        source.process_id = application.process_id;
                        source.creation_time_100ns = application.creation_time_100ns.unwrap_or(creation_time);
                        source.current_executable_path = application.executable_path.clone();
                        source.retry_delay = APPLICATION_CAPTURE_RETRY_MIN;
                        update(
                            &mut self.multi_input_application_sources,
                            "connected",
                            "Connected to the restarted application. Audio flow appears when it produces sound.",
                            APPLICATION_CAPTURE_LIVENESS_POLL,
                        );
                    } else {
                        let delay = self.multi_input_application_sources[index].retry_delay;
                        update(
                            &mut self.multi_input_application_sources,
                            "failed",
                            "The app is running, but Windows could not reopen its audio. AudioRouter will retry.",
                            delay,
                        );
                        self.multi_input_application_sources[index].retry_delay =
                            (delay * 2).min(APPLICATION_CAPTURE_RETRY_MAX);
                    }
                }
            }
        }
        changed
    }
}

#[cfg(test)]
#[path = "native_application_tests.rs"]
mod tests;
