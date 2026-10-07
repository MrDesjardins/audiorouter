//! RPC handlers for devices, native endpoints, outputs, paths, bridges and pumps.

use super::*;

impl ControlPlane {
    #[cfg(windows)]
    /// Pump bounded capture, graph, and prepared branch work for the exact
    /// running session generation. Physical output drains and branch-local
    /// virtual/recording/tool taps are owned by the attached worker; this
    /// control method only supplies the bounded scheduling budget.
    pub(crate) fn dispatch_native_multi_input_pump(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and generation are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let generation = params
            .get("generation")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| ControlError::InvalidRequest("generation is required".into()))?;
        let max_packets = params
            .get("maxPackets")
            .and_then(Value::as_u64)
            .map(|value| {
                u32::try_from(value)
                    .map_err(|_| ControlError::InvalidRequest("maxPackets is out of range".into()))
            })
            .transpose()?
            .unwrap_or(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE);
        self.pump_native_multi_input_worker(&session_id, generation, max_packets)
            .map(|value| self.with_audio_service_stats(value))
    }

    #[cfg(windows)]
    pub(crate) fn dispatch_native_multi_input_bind_branches(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest(
                "sessionId, generation, and branchNodeIds are required".into(),
            )
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let generation = params
            .get("generation")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| ControlError::InvalidRequest("generation is required".into()))?;
        let branch_node_ids = params
            .get("branchNodeIds")
            .and_then(Value::as_array)
            .ok_or_else(|| ControlError::InvalidRequest("branchNodeIds is required".into()))?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .map(EntityId::new)
                    .ok_or_else(|| {
                        ControlError::InvalidRequest(
                            "branchNodeIds must contain nonempty strings".into(),
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.bind_native_multi_input_branches(&session_id, generation, &branch_node_ids)
    }

    #[cfg(not(windows))]
    pub(crate) fn dispatch_native_multi_input_bind_branches(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        Err(ControlError::InvalidRequest(
            "native multi-input branch binding requires Windows".into(),
        ))
    }

    #[cfg(not(windows))]
    pub(crate) fn dispatch_native_multi_input_pump(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        Err(ControlError::InvalidRequest(
            "native multi-input pumping requires Windows".into(),
        ))
    }

    /// Persist the user's consent (or its withdrawal) for the desktop app to
    /// open audio devices on Play. The grant check in dispatch admits only
    /// the desktop shell's grant to this method.
    pub(crate) fn dispatch_device_access_set(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("allowed and idempotencyKey are required".into())
        })?;
        let allowed = params
            .get("allowed")
            .and_then(Value::as_bool)
            .ok_or_else(|| ControlError::InvalidRequest("allowed is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("devices.setAccess", idempotency_key),
            Self::request_hash(&json!({ "allowed": allowed })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        if let Some(storage) = &self.storage {
            storage
                .save_device_access_allowed(allowed)
                .map_err(storage_error)?;
        }
        self.device_access_allowed = allowed;
        let result = json!({ "allowed": allowed });
        self.journal_idempotent_result(&operation.0, "devices.setAccess", &operation.1, &result)?;
        Ok(result)
    }

    pub(crate) fn dispatch_devices_list(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let paged = params.get("cursor").is_some() || params.get("limit").is_some();
        let cursor = params
            .get("cursor")
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| ControlError::InvalidRequest("cursor must be a string".into()))
            })
            .transpose()?;
        let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(100);
        let include_inactive = params
            .get("includeInactive")
            .map(|value| {
                value.as_bool().ok_or_else(|| {
                    ControlError::InvalidRequest("includeInactive must be a boolean".into())
                })
            })
            .transpose()?
            .unwrap_or(false);
        if !(1..=MAX_DEVICE_LIST_ITEMS as u64).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        if self.endpoint_monitor.is_none() {
            self.endpoint_monitor = Some(
                audiorouter_windows_audio::EndpointMonitor::start().map_err(audio_control_error)?,
            );
        }
        let (endpoint_changes, endpoints) = {
            let monitor = self
                .endpoint_monitor
                .as_mut()
                .expect("endpoint monitor initialized above");
            let endpoint_changes = monitor.poll_changes().map_err(audio_control_error)?;
            let endpoints = monitor.snapshot().to_vec();
            (endpoint_changes, endpoints)
        };
        if !endpoint_changes.is_empty() {
            // Only retain exact-binding observations while an endpoint worker
            // exists. Unrelated endpoint churn must not accumulate in the
            // control plane or invalidate a worker later; a later prepare
            // operation resolves the current snapshot itself.
            let mut relevant_changes = [
                self.native_endpoint_worker.as_ref(),
                self.native_endpoint_worker_secondary.as_ref(),
            ]
            .into_iter()
            .flatten()
            .flat_map(|worker| {
                endpoint_changes
                    .iter()
                    .filter(|change| {
                        worker.endpoint_bindings_affected_by(std::slice::from_ref(*change))
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
            #[cfg(windows)]
            if let Some(worker) = self.native_multi_input_worker.as_ref() {
                relevant_changes.extend(
                    endpoint_changes
                        .iter()
                        .filter(|change| worker.bindings_affected_by(std::slice::from_ref(*change)))
                        .cloned(),
                );
            }
            if !relevant_changes.is_empty() {
                self.pending_endpoint_changes.extend(relevant_changes);
            }
            self.record_endpoint_changes(true);
        }
        let defaults = audiorouter_windows_audio::enumerate_default_endpoint_bindings()
            .map_err(audio_control_error)?;
        let display_info = audiorouter_windows_audio::enumerate_active_endpoint_display_info()
            .map_err(audio_control_error)?;
        let endpoint_states = include_inactive
            .then(audiorouter_windows_audio::enumerate_endpoint_states)
            .transpose()
            .map_err(audio_control_error)?
            .unwrap_or_default();
        let mut devices = endpoints
            .into_iter()
            .map(|endpoint| {
                let bytes_per_frame = endpoint.bytes_per_frame().map_err(audio_control_error)?;
                let default_roles = defaults
                    .iter()
                    .filter(|binding| {
                        binding.endpoint_id == endpoint.id
                            && binding.direction == endpoint.direction
                    })
                    .map(|binding| binding.role.as_str())
                    .collect::<Vec<_>>();
                let name = display_info
                    .iter()
                    .find(|info| info.id == endpoint.id && info.direction == endpoint.direction)
                    .map(|info| info.name.as_str())
                    .unwrap_or("Unknown audio endpoint");
                Ok(json!({
                    "id": endpoint.id,
                    "name": name,
                    "direction": match endpoint.direction {
                        audiorouter_windows_audio::EndpointDirection::Capture => "capture",
                        audiorouter_windows_audio::EndpointDirection::Render => "render",
                    },
                    "state": "active",
                    "defaultRoles": default_roles,
                    "format": {
                        "sampleRateHz": endpoint.sample_rate_hz,
                        "channels": endpoint.channels,
                        "bitsPerSample": endpoint.bits_per_sample,
                        "formatTag": endpoint.format_tag,
                        "bytesPerFrame": bytes_per_frame,
                    },
                    "periods": {
                        "default100ns": endpoint.default_period_100ns,
                        "minimum100ns": endpoint.minimum_period_100ns,
                    },
                }))
            })
            .collect::<Result<Vec<_>, ControlError>>()?;
        if include_inactive {
            for endpoint in endpoint_states {
                if matches!(
                    endpoint.state,
                    audiorouter_windows_audio::EndpointState::Active
                ) || devices.iter().any(|device| {
                    device["id"] == endpoint.id
                        && device["direction"]
                            == match endpoint.direction {
                                audiorouter_windows_audio::EndpointDirection::Capture => "capture",
                                audiorouter_windows_audio::EndpointDirection::Render => "render",
                            }
                }) {
                    continue;
                }
                let direction = match endpoint.direction {
                    audiorouter_windows_audio::EndpointDirection::Capture => "capture",
                    audiorouter_windows_audio::EndpointDirection::Render => "render",
                };
                let default_roles = defaults
                    .iter()
                    .filter(|binding| {
                        binding.endpoint_id == endpoint.id
                            && binding.direction == endpoint.direction
                    })
                    .map(|binding| binding.role.as_str())
                    .collect::<Vec<_>>();
                let name = display_info
                    .iter()
                    .find(|info| info.id == endpoint.id && info.direction == endpoint.direction)
                    .map(|info| info.name.as_str())
                    .unwrap_or("Unknown audio endpoint");
                let state = match endpoint.state {
                    audiorouter_windows_audio::EndpointState::Disabled => "disabled",
                    audiorouter_windows_audio::EndpointState::Unplugged => "unplugged",
                    audiorouter_windows_audio::EndpointState::NotPresent => "notPresent",
                    audiorouter_windows_audio::EndpointState::Unknown(_) => "unknown",
                    audiorouter_windows_audio::EndpointState::Active => unreachable!(),
                };
                devices.push(json!({
                    "id": endpoint.id,
                    "name": name,
                    "direction": direction,
                    "state": state,
                    "defaultRoles": default_roles,
                }));
            }
        }
        devices.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
        if !paged && devices.len() > MAX_DEVICE_LIST_ITEMS {
            return Err(ControlError::InvalidRequest(
                "devices.list requires cursor pagination when more than 500 endpoints exist".into(),
            ));
        }
        if let Some(cursor) = cursor {
            let Some(index) = devices.iter().position(|device| device["id"] == cursor) else {
                return Err(ControlError::InvalidRequest("invalid device cursor".into()));
            };
            devices.drain(..=index);
        }
        if !paged {
            return Ok(json!(devices));
        }
        let has_more = devices.len() > limit as usize;
        devices.truncate(limit as usize);
        let next_cursor = has_more
            .then(|| devices.last().and_then(|device| device["id"].as_str()))
            .flatten();
        Ok(json!({ "items": devices, "nextCursor": next_cursor }))
    }

    pub(crate) fn dispatch_native_endpoints_prepare(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and endpoint IDs are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let capture_id = params
            .get("captureEndpointId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("captureEndpointId is required".into()))?;
        let render_id = params
            .get("renderEndpointId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("renderEndpointId is required".into()))?;
        let endpoints =
            audiorouter_windows_audio::enumerate_active_endpoints().map_err(audio_control_error)?;
        let capture = endpoints
            .iter()
            .find(|endpoint| {
                endpoint.id == capture_id
                    && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Capture
            })
            .ok_or_else(|| {
                ControlError::InvalidRequest(
                    "capture endpoint is not an active exact inventory match".into(),
                )
            })?;
        let render = endpoints
            .iter()
            .find(|endpoint| {
                endpoint.id == render_id
                    && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render
            })
            .ok_or_else(|| {
                ControlError::InvalidRequest(
                    "render endpoint is not an active exact inventory match".into(),
                )
            })?;
        self.prepare_native_endpoint_worker(session_id.clone(), capture, render, 0, 3, 100)?;
        Ok(json!({
            "sessionId": session_id,
            "state": "configured-stopped",
            "captureEndpointId": capture_id,
            "renderEndpointId": render_id
        }))
    }

    #[cfg(windows)]
    pub(crate) fn dispatch_native_outputs_prepare(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest(
                "sessionId, generation, and renderEndpointIds are required".into(),
            )
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let generation = self.requested_or_next_generation(&params, &session_id)?;
        let endpoint_values = params
            .get("renderEndpointIds")
            .and_then(Value::as_array)
            .ok_or_else(|| ControlError::InvalidRequest("renderEndpointIds is required".into()))?;
        if endpoint_values.is_empty() || endpoint_values.len() > audiorouter_engine::MAX_AUDIO_TAPS
        {
            return Err(ControlError::InvalidRequest(
                "renderEndpointIds must contain 1..8 endpoints".into(),
            ));
        }
        let endpoint_ids = endpoint_values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .filter(|id| !id.is_empty() && id.len() <= MAX_CONTROL_STRING_BYTES)
                    .map(str::to_owned)
                    .ok_or_else(|| {
                        ControlError::InvalidRequest(
                            "renderEndpointIds must contain bounded nonempty strings".into(),
                        )
                    })
            })
            .collect::<Result<Vec<_>, ControlError>>()?;
        self.prepare_native_output_fanout(session_id, generation, &endpoint_ids)
    }

    #[cfg(windows)]
    pub(crate) fn dispatch_native_multi_inputs_prepare(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        enum DecodedMultiInputSource {
            Generated,
            Physical(String),
            Application {
                process_id: u32,
                executable: String,
                executable_path: Option<String>,
                creation_time_100ns: u64,
                mode: audiorouter_windows_audio::ProcessLoopbackMode,
            },
        }
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId, generation, and sources are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let generation = self.requested_or_next_generation(&params, &session_id)?;
        let source_values = params
            .get("sources")
            .and_then(Value::as_array)
            .ok_or_else(|| ControlError::InvalidRequest("sources is required".into()))?;
        if source_values.len() < 2 || source_values.len() > audiorouter_engine::MAX_MIXER_INPUTS {
            return Err(ControlError::InvalidRequest(
                "sources must contain 2..8 entries".into(),
            ));
        }
        let decoded = source_values
            .iter()
            .map(|value| match value.get("kind").and_then(Value::as_str) {
                Some("generated") => Ok(DecodedMultiInputSource::Generated),
                Some("physical") => value
                    .get("endpointId")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty() && id.len() <= MAX_CONTROL_STRING_BYTES)
                    .map(|id| DecodedMultiInputSource::Physical(id.to_owned()))
                    .ok_or_else(|| {
                        ControlError::InvalidRequest(
                            "a physical source requires a bounded nonempty endpointId".into(),
                        )
                    }),
                Some("application") => {
                    let process_id = value
                        .get("processId")
                        .and_then(Value::as_u64)
                        .and_then(|id| u32::try_from(id).ok())
                        .filter(|id| *id > 0)
                        .ok_or_else(|| {
                            ControlError::InvalidRequest(
                                "an application source requires processId".into(),
                            )
                        })?;
                    let executable = value
                        .get("executable")
                        .and_then(Value::as_str)
                        .filter(|value| !value.is_empty() && value.len() <= MAX_CONTROL_STRING_BYTES)
                        .map(str::to_owned)
                        .ok_or_else(|| {
                            ControlError::InvalidRequest(
                                "an application source requires executable".into(),
                            )
                        })?;
                    let executable_path = match value.get("executablePath") {
                        None | Some(Value::Null) => None,
                        Some(Value::String(path)) if !path.is_empty() => Some(path.clone()),
                        _ => {
                            return Err(ControlError::InvalidRequest(
                                "executablePath must be a nonempty string or null".into(),
                            ))
                        }
                    };
                    let creation_time_100ns = value
                        .get("creationTime100ns")
                        .and_then(Value::as_str)
                        .and_then(|value| value.parse::<u64>().ok())
                        .ok_or_else(|| {
                            ControlError::InvalidRequest(
                                "an application source requires creationTime100ns as a decimal string".into(),
                            )
                        })?;
                    let mode = match value.get("mode").and_then(Value::as_str) {
                        Some("include") => {
                            audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree
                        }
                        Some("exclude") => {
                            audiorouter_windows_audio::ProcessLoopbackMode::ExcludeTargetTree
                        }
                        _ => {
                            return Err(ControlError::InvalidRequest(
                                "an application source requires mode include or exclude".into(),
                            ))
                        }
                    };
                    Ok(DecodedMultiInputSource::Application {
                        process_id,
                        executable,
                        executable_path,
                        creation_time_100ns,
                        mode,
                    })
                }
                _ => Err(ControlError::InvalidRequest(
                    "each source requires kind physical, application, or generated".into(),
                )),
            })
            .collect::<Result<Vec<_>, ControlError>>()?;
        let endpoints =
            audiorouter_windows_audio::enumerate_active_endpoints().map_err(audio_control_error)?;
        let resolved_endpoints = decoded
            .iter()
            .map(|source| match source {
                DecodedMultiInputSource::Physical(endpoint_id) => endpoints
                    .iter()
                    .find(|endpoint| {
                        endpoint.id == *endpoint_id
                            && endpoint.direction
                                == audiorouter_windows_audio::EndpointDirection::Capture
                    })
                    .cloned()
                    .map(Some)
                    .ok_or_else(|| {
                        ControlError::InvalidRequest(
                            "capture endpoint is not an active exact inventory match".into(),
                        )
                    }),
                DecodedMultiInputSource::Application { .. }
                | DecodedMultiInputSource::Generated => Ok(None),
            })
            .collect::<Result<Vec<_>, ControlError>>()?;
        let bindings = decoded
            .iter()
            .zip(resolved_endpoints.iter())
            .map(|(source, endpoint)| match source {
                DecodedMultiInputSource::Generated => NativeMultiInputSourceBinding::Generated,
                DecodedMultiInputSource::Physical(_) => NativeMultiInputSourceBinding::Physical(
                    endpoint.as_ref().expect("physical source resolved above"),
                ),
                DecodedMultiInputSource::Application {
                    process_id,
                    executable,
                    executable_path,
                    creation_time_100ns,
                    mode,
                } => NativeMultiInputSourceBinding::Application {
                    process_id: *process_id,
                    expected_executable: executable,
                    expected_executable_path: executable_path.as_deref(),
                    expected_creation_time_100ns: *creation_time_100ns,
                    mode: *mode,
                },
            })
            .collect::<Vec<_>>();
        self.prepare_native_multi_input_worker(session_id, generation, &bindings, 0, 3, 100)
    }

    #[cfg(not(windows))]
    pub(crate) fn dispatch_native_multi_inputs_prepare(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        Err(ControlError::InvalidRequest(
            "native multi-input preparation requires Windows".into(),
        ))
    }

    #[cfg(windows)]
    /// Prepare every independent path of a saved session (GRAPH-15) from the
    /// exact devices and applications stored on its nodes (CAP-02). Nothing
    /// is chosen by name, default role or fallback: a node without a chosen,
    /// connected device stops preparation with its name. Outputs are opened
    /// after the sources; if they fail, the prepared worker is released.
    pub(crate) fn dispatch_native_paths_prepare(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let generation = self.requested_or_next_generation(&params, &session_id)?;
        let session = audiorouter_engine::prune_inactive_upstream(self.get_session(&session_id)?)
            .into_owned();
        let endpoints =
            audiorouter_windows_audio::enumerate_active_endpoints().map_err(audio_control_error)?;
        let endpoint_for = |node: &audiorouter_domain::Node, direction| {
            let endpoint_id = node
                .parameters
                .get("endpointId")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ControlError::InvalidRequest(format!(
                        "choose the device for {} in its Properties",
                        node.name
                    ))
                })?;
            endpoints
                .iter()
                .find(|endpoint| endpoint.id == endpoint_id && endpoint.direction == direction)
                .ok_or_else(|| {
                    ControlError::InvalidRequest(format!(
                        "the device chosen for {} is not connected; choose it again in its Properties",
                        node.name
                    ))
                })
        };
        let mut bindings = HashMap::new();
        let mut mono_nodes = Vec::new();
        for node in session.nodes.iter().filter(|node| {
            node.enabled
                && session
                    .edges
                    .iter()
                    .any(|edge| edge.enabled && edge.source_node == node.id)
        }) {
            let binding = match node.kind {
                NodeKind::PhysicalInput => {
                    // Surround to headphones may loopback-capture a 5.1/7.1
                    // playback device, such as a virtual cable set to 7.1.
                    let endpoint = match endpoint_for(
                        node,
                        audiorouter_windows_audio::EndpointDirection::Capture,
                    ) {
                        Err(_) if node_spatial_headphones(node) => endpoint_for(
                            node,
                            audiorouter_windows_audio::EndpointDirection::Render,
                        )?,
                        result => result?,
                    };
                    let node_channels = node
                        .ports
                        .iter()
                        .find(|port| port.direction == PortDirection::Output)
                        .map(|port| port.channels);
                    if endpoint.channels == 1
                        && node_channels == Some(2)
                        && !node_spatial_headphones(node)
                    {
                        mono_nodes.push(node.id.clone());
                    }
                    NativeMultiInputSourceBinding::Physical(endpoint)
                }
                NodeKind::TestSignal | NodeKind::AudioFile => {
                    NativeMultiInputSourceBinding::Generated
                }
                NodeKind::NetworkReceive => NativeMultiInputSourceBinding::Network(node),
                NodeKind::ApplicationCapture => {
                    let process_id = node
                        .parameters
                        .get("processId")
                        .and_then(Value::as_u64)
                        .and_then(|id| u32::try_from(id).ok());
                    let executable = node.parameters.get("executable").and_then(Value::as_str);
                    let creation_time = node
                        .parameters
                        .get("creationTime100ns")
                        .and_then(Value::as_str)
                        .and_then(|value| value.parse::<u64>().ok());
                    let (Some(process_id), Some(executable), Some(creation_time)) =
                        (process_id, executable, creation_time)
                    else {
                        return Err(ControlError::InvalidRequest(format!(
                            "select a running application for {} in its Properties",
                            node.name
                        )));
                    };
                    NativeMultiInputSourceBinding::Application {
                        process_id,
                        expected_executable: executable,
                        expected_executable_path: node
                            .parameters
                            .get("executablePath")
                            .and_then(Value::as_str),
                        expected_creation_time_100ns: creation_time,
                        mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                    }
                }
                NodeKind::EndpointLoopback | NodeKind::VirtualRenderSource => {
                    return Err(ControlError::InvalidRequest(format!(
                        "{} cannot feed a multi-path session yet",
                        node.name
                    )));
                }
                _ => continue,
            };
            bindings.insert(node.id.clone(), binding);
        }
        self.native_multi_input_mono_nodes = mono_nodes;
        let prepared = self.prepare_native_path_worker(
            session_id.clone(),
            generation,
            MultiInputBindings::ByNode(&bindings),
            0,
            3,
            100,
        )?;
        let (branch_node_ids, path_count) = {
            let worker = self
                .native_multi_input_worker
                .as_ref()
                .expect("worker attached above");
            (worker.output_node_ids().to_vec(), worker.path_count())
        };
        let render_endpoint_ids = branch_node_ids
            .iter()
            .filter_map(|node_id| {
                session
                    .nodes
                    .iter()
                    .find(|node| node.id == *node_id && node.kind == NodeKind::PhysicalOutput)
            })
            .map(|node| {
                endpoint_for(node, audiorouter_windows_audio::EndpointDirection::Render)
                    .map(|endpoint| endpoint.id.clone())
            })
            .collect::<Result<Vec<String>, ControlError>>();
        let outputs = render_endpoint_ids.and_then(|render_endpoint_ids| {
            self.prepare_native_output_fanout_with(
                session_id.clone(),
                generation,
                &render_endpoint_ids,
                true,
            )
            .map(|_| render_endpoint_ids)
        });
        let render_endpoint_ids = match outputs {
            Ok(ids) => ids,
            Err(error) => {
                let _ = self.detach_native_multi_input_worker();
                return Err(error);
            }
        };
        Ok(json!({
            "sessionId": session_id,
            "generation": generation,
            "state": "configured-stopped",
            "pathCount": path_count,
            "sourceNodeIds": prepared["sourceNodeIds"],
            "branchNodeIds": branch_node_ids,
            "renderEndpointIds": render_endpoint_ids,
        }))
    }

    #[cfg(not(windows))]
    pub(crate) fn dispatch_native_paths_prepare(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        Err(ControlError::InvalidRequest(
            "native path preparation requires Windows".into(),
        ))
    }

    #[cfg(not(windows))]
    pub(crate) fn dispatch_native_outputs_prepare(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        Err(ControlError::InvalidRequest(
            "native output fan-out preparation requires Windows".into(),
        ))
    }

    #[cfg(windows)]
    pub(crate) fn dispatch_native_bridges_prepare(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest(
                "busId, generation, devicePath, and mapping paths are required".into(),
            )
        })?;
        let text_param = |name: &str| {
            params
                .get(name)
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty() && value.len() <= MAX_CONTROL_STRING_BYTES)
                .ok_or_else(|| ControlError::InvalidRequest(format!("{name} is required")))
        };
        let bus_id = text_param("busId").map(EntityId::new)?;
        let generation = params
            .get("generation")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| ControlError::InvalidRequest("generation is required".into()))?;
        let device_path = text_param("devicePath")?;
        let render_mapping_path = text_param("renderMappingPath")?;
        let capture_mapping_path = text_param("captureMappingPath")?;
        let lease_ms = params
            .get("leaseMs")
            .and_then(Value::as_u64)
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    ControlError::InvalidRequest("leaseMs exceeds the bounded integer range".into())
                })
            })
            .transpose()?
            .unwrap_or(1_000);
        self.prepare_native_bridge(
            bus_id,
            device_path,
            render_mapping_path,
            capture_mapping_path,
            generation,
            lease_ms,
        )
    }

    #[cfg(not(windows))]
    pub(crate) fn dispatch_native_bridges_prepare(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        Err(ControlError::InvalidRequest(
            "native bridge preparation requires Windows".into(),
        ))
    }

    pub(crate) fn dispatch_native_bridges_detach(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("busId is required".into()))?;
        let bus_id = params
            .get("busId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("busId is required".into()))?;
        #[cfg(windows)]
        {
            self.detach_native_bridge(&bus_id)
        }
        #[cfg(not(windows))]
        {
            let _ = bus_id;
            Err(ControlError::InvalidRequest(
                "native bridge detachment requires Windows".into(),
            ))
        }
    }

    pub(crate) fn dispatch_native_bridges_heartbeat(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        #[cfg(windows)]
        {
            self.heartbeat_native_bridges()
        }
        #[cfg(not(windows))]
        {
            Err(ControlError::InvalidRequest(
                "native bridge heartbeat requires Windows".into(),
            ))
        }
    }

    #[cfg(windows)]
    pub(crate) fn dispatch_native_endpoints_rebind(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and endpoint IDs are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let capture_id = params
            .get("captureEndpointId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("captureEndpointId is required".into()))?;
        let render_id = params
            .get("renderEndpointId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("renderEndpointId is required".into()))?;
        self.rebind_native_endpoint_worker(&session_id, capture_id, render_id, 0, 3, 100)?;
        Ok(json!({
            "sessionId": session_id,
            "state": "configured-stopped",
            "captureEndpointId": capture_id,
            "renderEndpointId": render_id
        }))
    }

    #[cfg(not(windows))]
    pub(crate) fn dispatch_native_endpoints_rebind(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        Err(ControlError::InvalidRequest(
            "native endpoint rebinding requires Windows".into(),
        ))
    }

    pub(crate) fn dispatch_native_endpoints_detach(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        self.get_session(&session_id)?;
        if !self.native_endpoint_session_is_attached(&session_id)
            && self.native_multi_input_worker_session.as_ref() == Some(&session_id)
        {
            // The multi-input Mixer worker (and the outputs it owns) is
            // released the same way, so Play can switch a stopped session to
            // another adapter.
            if self
                .runtimes
                .get(&session_id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
            {
                return Err(ControlError::InvalidRequest(
                    "stop the session before detaching its native multi-input worker".into(),
                ));
            }
            self.detach_native_multi_input_worker()?;
            return Ok(json!({ "sessionId": session_id, "state": "detached" }));
        }
        if !self.native_endpoint_session_is_attached(&session_id) {
            return Err(ControlError::InvalidRequest(
                "native endpoint worker is not attached to this session".into(),
            ));
        }
        self.detach_native_endpoint_worker_for_session(&session_id)?;
        Ok(json!({ "sessionId": session_id, "state": "detached" }))
    }

    pub(crate) fn dispatch_native_duplex_detach(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        self.get_session(&session_id)?;
        #[cfg(windows)]
        {
            if self.native_duplex_worker_session.as_ref() != Some(&session_id) {
                return Err(ControlError::InvalidRequest(
                    "native duplex worker is not attached to this session".into(),
                ));
            }
            self.detach_native_duplex_worker()?;
            Ok(json!({ "sessionId": session_id, "state": "detached" }))
        }
        #[cfg(not(windows))]
        {
            let _ = session_id;
            Err(ControlError::InvalidRequest(
                "native duplex detachment requires Windows".into(),
            ))
        }
    }

    pub(crate) fn dispatch_native_applications_prepare(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest(
                "application identity and render endpoint are required".into(),
            )
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let process_id = params
            .get("processId")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
            .ok_or_else(|| ControlError::InvalidRequest("processId is required".into()))?;
        let executable = params
            .get("executable")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("executable is required".into()))?;
        let executable_path = params
            .get("executablePath")
            .and_then(|value| {
                if value.is_null() {
                    Some(None)
                } else {
                    value.as_str().map(Some)
                }
            })
            .flatten();
        let creation_time = params
            .get("creationTime100ns")
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| {
                ControlError::InvalidRequest("creationTime100ns must be a decimal string".into())
            })?;
        let mode = match params.get("mode").and_then(Value::as_str) {
            Some("include") => audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
            Some("exclude") => audiorouter_windows_audio::ProcessLoopbackMode::ExcludeTargetTree,
            _ => {
                return Err(ControlError::InvalidRequest(
                    "mode must be include or exclude".into(),
                ))
            }
        };
        let render_id = params
            .get("renderEndpointId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("renderEndpointId is required".into()))?;
        let endpoints =
            audiorouter_windows_audio::enumerate_active_endpoints().map_err(audio_control_error)?;
        let render = endpoints
            .iter()
            .find(|endpoint| {
                endpoint.id == render_id
                    && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render
            })
            .ok_or_else(|| {
                ControlError::InvalidRequest(
                    "render endpoint is not an active exact inventory match".into(),
                )
            })?;
        self.prepare_native_application_worker(
            session_id.clone(),
            NativeApplicationWorkerConfig {
                process_id,
                expected_executable: executable,
                expected_executable_path: executable_path,
                expected_creation_time_100ns: creation_time,
                mode,
                render,
                buffer_duration_100ns: 0,
                max_attempts: 3,
                retry_delay_ms: 100,
            },
        )?;
        Ok(
            json!({ "sessionId": session_id, "state": "configured-stopped", "processId": process_id,
            "executable": executable, "executablePath": executable_path, "creationTime100ns": creation_time.to_string(),
            "mode": if matches!(mode, audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree) { "include" } else { "exclude" },
            "renderEndpointId": render_id }),
        )
    }

    pub(crate) fn dispatch_native_endpoints_pump(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and generation are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let generation = params
            .get("generation")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| ControlError::InvalidRequest("generation is required".into()))?;
        let max_packets = params
            .get("maxPackets")
            .and_then(Value::as_u64)
            .map(|value| {
                u32::try_from(value)
                    .map_err(|_| ControlError::InvalidRequest("maxPackets is out of range".into()))
            })
            .transpose()?
            .unwrap_or(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE);
        self.pump_native_endpoint_worker_with_bound_taps(&session_id, generation, max_packets)
            .map(|value| self.with_audio_service_stats(value))
    }

    #[cfg(windows)]
    pub(crate) fn dispatch_native_duplex_pump(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and generation are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let generation = params
            .get("generation")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| ControlError::InvalidRequest("generation is required".into()))?;
        let max_input_quanta = params
            .get("maxInputQuanta")
            .and_then(Value::as_u64)
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    ControlError::InvalidRequest("maxInputQuanta is out of range".into())
                })
            })
            .transpose()?
            .unwrap_or(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE);
        let max_output_packets = params
            .get("maxOutputPackets")
            .and_then(Value::as_u64)
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    ControlError::InvalidRequest("maxOutputPackets is out of range".into())
                })
            })
            .transpose()?
            .unwrap_or(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE);
        self.pump_native_duplex_worker(
            &session_id,
            generation,
            max_input_quanta,
            max_output_packets,
        )
        .map(|value| self.with_audio_service_stats(value))
    }

    #[cfg(not(windows))]
    pub(crate) fn dispatch_native_duplex_pump(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        Err(ControlError::InvalidRequest(
            "native duplex pumping requires Windows".into(),
        ))
    }

    #[cfg(windows)]
    pub(crate) fn dispatch_native_render_source_pump(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and generation are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let generation = params
            .get("generation")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| ControlError::InvalidRequest("generation is required".into()))?;
        let max_quanta = params
            .get("maxQuanta")
            .and_then(Value::as_u64)
            .map(|value| {
                u32::try_from(value)
                    .map_err(|_| ControlError::InvalidRequest("maxQuanta is out of range".into()))
            })
            .transpose()?
            .unwrap_or(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE);
        self.pump_native_render_source_worker(&session_id, generation, max_quanta)
            .map(|value| self.with_audio_service_stats(value))
    }

    #[cfg(not(windows))]
    pub(crate) fn dispatch_native_render_source_pump(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        Err(ControlError::InvalidRequest(
            "native render-source pumping requires Windows".into(),
        ))
    }

    pub(crate) fn record_endpoint_changes(&mut self, changed: bool) {
        if changed {
            // EventLog is the bounded notification surface. Endpoint details
            // are intentionally refetched through devices.list so events do
            // not duplicate an unbounded or stale device payload.
            self.events.append(0, None, "devices.changed", None);
        }
    }

    /// Retain endpoint changes discovered by a recovery resnapshot or a
    /// read-only inventory call until a mutating native lifecycle boundary can
    /// fail closed or explicitly rebind. Resume must not consume the changes
    /// without publishing the same bounded event that `devices.list` emits.
    pub(crate) fn retain_endpoint_changes(
        &mut self,
        endpoint_changes: &[audiorouter_windows_audio::EndpointChange],
    ) {
        if endpoint_changes.is_empty() {
            return;
        }
        let mut relevant_changes = self
            .native_endpoint_worker
            .as_ref()
            .map(|worker| {
                endpoint_changes
                    .iter()
                    .filter(|change| {
                        worker.endpoint_bindings_affected_by(std::slice::from_ref(*change))
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        #[cfg(windows)]
        if let Some(worker) = self.native_multi_input_worker.as_ref() {
            relevant_changes.extend(
                endpoint_changes
                    .iter()
                    .filter(|change| worker.bindings_affected_by(std::slice::from_ref(*change)))
                    .cloned(),
            );
        }
        if !relevant_changes.is_empty() {
            self.pending_endpoint_changes.extend(relevant_changes);
        }
        self.record_endpoint_changes(true);
    }

    /// Fail closed when the read-only endpoint monitor observes a change to
    /// an endpoint owned by the native worker. The worker keeps its exact
    /// bindings and must be deliberately rebound against the refreshed
    /// snapshot; this method never chooses a replacement endpoint.
    pub(crate) fn invalidate_changed_native_endpoint_worker(
        &mut self,
        changes: &[audiorouter_windows_audio::EndpointChange],
    ) -> Result<(), ControlError> {
        let mut invalidated = false;
        for session_id in [
            self.native_endpoint_session.clone(),
            self.native_endpoint_session_secondary.clone(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(worker) = self.native_endpoint_worker_for_session_mut(&session_id) {
                if worker.endpoint_bindings_affected_by(changes) && worker.is_running() {
                    worker.stop().map_err(audio_control_error)?;
                    invalidated = true;
                }
            }
        }
        #[cfg(windows)]
        if let Some(worker) = self.native_multi_input_worker.as_mut() {
            if worker.bindings_affected_by(changes) && worker.is_running() {
                worker.stop().map_err(|error| {
                    ControlError::InvalidRequest(format!(
                        "native multi-input invalidation stop failed: {error:?}"
                    ))
                })?;
                invalidated = true;
            }
        }
        if !invalidated {
            return Ok(());
        }
        self.events
            .append(0, None, "devices.bindingInvalidated", None);
        Ok(())
    }

    /// Poll endpoint notifications from a mutating native lifecycle boundary.
    /// Read-only inventory must not stop an audio worker as a hidden side
    /// effect; the native pump is the point where an affected running worker
    /// is fail-closed and its staged audio is reset.
    pub(crate) fn poll_native_endpoint_lifecycle(&mut self) -> Result<bool, ControlError> {
        let Some(monitor) = self.endpoint_monitor.as_mut() else {
            return Ok(false);
        };
        let mut changes = std::mem::take(&mut self.pending_endpoint_changes);
        changes.extend(monitor.poll_changes().map_err(audio_control_error)?);
        if changes.is_empty() {
            return Ok(false);
        }
        let mut affected = [
            self.native_endpoint_worker.as_ref(),
            self.native_endpoint_worker_secondary.as_ref(),
        ]
        .into_iter()
        .flatten()
        .any(|worker| worker.endpoint_bindings_affected_by(&changes));
        #[cfg(windows)]
        {
            affected |= self
                .native_multi_input_worker
                .as_ref()
                .is_some_and(|worker| worker.bindings_affected_by(&changes));
        }
        self.invalidate_changed_native_endpoint_worker(&changes)?;
        self.record_endpoint_changes(true);
        Ok(affected)
    }
}

#[cfg(test)]
#[path = "native_dispatch_tests.rs"]
mod tests;
