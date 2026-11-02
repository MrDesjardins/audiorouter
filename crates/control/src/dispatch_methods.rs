//! Typed routing from discovered API names to backend handlers. Validation
//! and authorization stay in `ControlPlane::dispatch*`, before these calls.

use super::*;

type MethodHandler = fn(&mut ControlPlane, &str, Option<Value>) -> Result<Value, ControlError>;

struct MethodEntry {
    name: &'static str,
    handler: MethodHandler,
}

macro_rules! method_table {
    (
        params { $( $param_name:literal => $param_handler:ident ),* $(,)? }
        no_params { $( $empty_name:literal => $empty_handler:ident ),* $(,)? }
        family { $( $family_name:literal => $family_handler:ident ),* $(,)? }
        target { $( $target_name:literal => $target_handler:ident ),* $(,)? }
        custom { $( $custom_name:literal => $custom_handler:expr ),* $(,)? }
    ) => {
        const METHOD_HANDLERS: &[MethodEntry] = &[
            $(MethodEntry { name: $param_name, handler: |plane, _, params| plane.$param_handler(params) },)*
            $(MethodEntry { name: $empty_name, handler: |plane, _, _| plane.$empty_handler() },)*
            $(MethodEntry { name: $family_name, handler: ControlPlane::$family_handler },)*
            $(MethodEntry { name: $target_name, handler: |plane, _, params| {
                let params = plane.resolve_node_target(params)?;
                plane.$target_handler(params)
            } },)*
            $(MethodEntry { name: $custom_name, handler: $custom_handler },)*
        ];
    };
}

method_table! {
    params {
        "system.handshake" => dispatch_handshake,
        "system.osTransition" => dispatch_os_transition,
        "system.quit" => dispatch_system_quit,
        "clients.authorize" => dispatch_client_authorize,
        "clients.revoke" => dispatch_client_revoke,
        "operations.get" => dispatch_operation_get,
        "operations.cancel" => dispatch_operation_cancel,
        "recordings.list" => dispatch_recordings_list,
        "audioMedia.beginUpload" => dispatch_audio_media_begin_upload,
        "audioMedia.uploadChunk" => dispatch_audio_media_upload_chunk,
        "audioMedia.finishUpload" => dispatch_audio_media_finish_upload,
        "audioMedia.importTemporaryRecording" => dispatch_audio_media_import_temporary_recording,
        "audioMedia.delete" => dispatch_audio_media_delete,
        "soundLibrary.read" => dispatch_sound_library_read,
        "recorders.list" => dispatch_recorders_list,
        "recorders.create" => dispatch_recorder_create,
        "devices.setAccess" => dispatch_device_access_set,
        "recordings.setRoot" => dispatch_recording_root_set,
        "recordings.get" => dispatch_recordings_get,
        "recordings.recovery" => dispatch_recording_recovery,
        "recordings.reveal" => dispatch_recording_reveal,
        "recordings.preview" => dispatch_recordings_preview,
        "recordings.setMetadata" => dispatch_recording_metadata,
        "recordings.rename" => dispatch_recording_rename,
        "recordings.removeEntry" => dispatch_recording_remove,
        "recordings.recycle" => dispatch_recording_recycle,
        "safety.setPrivacyMute" => dispatch_privacy_mute,
        "recovery.clearSafeMode" => dispatch_recovery_clear,
        "startup.plan" => dispatch_startup_plan,
        "startup.apply" => dispatch_startup_apply,
        "devices.list" => dispatch_devices_list,
        "nativeEndpoints.prepare" => dispatch_native_endpoints_prepare,
        "nativeOutputs.prepare" => dispatch_native_outputs_prepare,
        "nativeMultiInputs.prepare" => dispatch_native_multi_inputs_prepare,
        "nativePaths.prepare" => dispatch_native_paths_prepare,
        "nativeBridges.prepare" => dispatch_native_bridges_prepare,
        "nativeBridges.detach" => dispatch_native_bridges_detach,
        "nativeBridges.heartbeat" => dispatch_native_bridges_heartbeat,
        "nativeEndpoints.rebind" => dispatch_native_endpoints_rebind,
        "nativeEndpoints.detach" => dispatch_native_endpoints_detach,
        "nativeApplications.prepare" => dispatch_native_applications_prepare,
        "nativeDuplex.detach" => dispatch_native_duplex_detach,
        "nativeEndpoints.pump" => dispatch_native_endpoints_pump,
        "nativeDuplex.pump" => dispatch_native_duplex_pump,
        "nativeRenderSources.pump" => dispatch_native_render_source_pump,
        "nativeMultiInputs.pump" => dispatch_native_multi_input_pump,
        "nativeMultiInputs.bindBranches" => dispatch_native_multi_input_bind_branches,
        "plugins.scan" => dispatch_plugins_scan,
        "plugins.list" => dispatch_plugins_list,
        "plugins.retry" => dispatch_plugins_retry,
        "plugins.inspect" => dispatch_plugins_inspect,
        "plugins.parameters" => dispatch_plugins_parameters,
        "plugins.saveState" => dispatch_plugins_save_state,
        "virtualDevices.list" => dispatch_virtual_devices_list,
        "virtualDevices.plan" => dispatch_virtual_devices_plan,
        "virtualDevices.apply" => dispatch_virtual_devices_apply,
        "virtualDevices.provision" => dispatch_virtual_devices_provision,
        "virtualDevices.remove" => dispatch_virtual_devices_remove,
        "virtualRoutes.replace" => dispatch_virtual_routes_replace,
        "processors.response" => dispatch_processors_response,
        "sessions.get" => dispatch_session_get,
        "sessions.export" => dispatch_session_export,
        "sessions.exportFile" => dispatch_session_export_file,
        "sessions.importFile" => dispatch_session_import_file,
        "sessions.importPlan" => dispatch_session_import_plan,
        "sessions.importCommit" => dispatch_session_import_commit,
        "sessions.list" => dispatch_sessions_list,
        "sessions.active.set" => dispatch_active_session_set,
        "sessions.create" => dispatch_session_create,
        "sessions.duplicate" => dispatch_session_duplicate,
        "sessions.delete" => dispatch_session_delete,
        "routes.inspect" => dispatch_routes_inspect,
        "graph.history" => dispatch_graph_history,
        "graph.undoPlan" => dispatch_graph_undo_plan,
        "events.subscribe" => dispatch_events_subscribe,
        "session.start" => dispatch_session_start,
        "sessions.start" => dispatch_session_start,
        "session.stop" => dispatch_session_stop,
        "sessions.stop" => dispatch_session_stop,
        "graph.plan" => dispatch_plan,
        "graph.commit" => dispatch_commit
    }
    no_params {
        "status.get" => status_snapshot,
        "system.diagnostics" => dispatch_diagnostics_snapshot,
        "clients.list" => dispatch_clients_list,
        "soundLibrary.list" => dispatch_sound_library_list,
        "recordings.getRoot" => dispatch_recording_root_get,
        "virtualCable.status" => dispatch_virtual_cable_status,
        "virtualRoutes.list" => dispatch_virtual_routes_list,
        "apps.list" => dispatch_apps_list,
        "applications.list" => dispatch_apps_list
    }
    family {
        "recorders.arm" => dispatch_recorder,
        "recorders.start" => dispatch_recorder,
        "recorders.pause" => dispatch_recorder,
        "recorders.resume" => dispatch_recorder,
        "recorders.split" => dispatch_recorder,
        "recorders.stop" => dispatch_recorder,
        "recorders.startRecording" => dispatch_one_click_recording,
        "recorders.stopRecording" => dispatch_one_click_recording,
        "sessions.play" => dispatch_simple,
        "sessions.togglePlay" => dispatch_simple,
        "sessions.summary" => dispatch_simple,
        "safety.togglePrivacyMute" => dispatch_simple,
        "nodes.catalog" => dispatch_simple,
        "nodes.set" => dispatch_simple,
        "nodes.toggle" => dispatch_simple,
        "nodes.add" => dispatch_simple,
        "nodes.remove" => dispatch_simple,
        "connections.add" => dispatch_simple,
        "connections.remove" => dispatch_simple,
        "meters.levels" => dispatch_simple,
        "obs.status" => dispatch_obs,
        "obs.refresh" => dispatch_obs,
        "obs.connect" => dispatch_obs,
        "obs.disconnect" => dispatch_obs,
        "obs.preview" => dispatch_obs,
        "obs.apply" => dispatch_obs,
        "obs.restore" => dispatch_obs
    }
    target {
        "audioSources.transport" => dispatch_audio_source_transport,
        "soundboard.trigger" => dispatch_soundboard_trigger,
        "soundboard.stop" => dispatch_soundboard_stop,
        "soundboard.status" => dispatch_soundboard_status,
        "timeShift.transport" => dispatch_time_shift_transport,
        "meters.reset" => dispatch_meter_reset,
        "plugins.resume" => dispatch_plugins_resume
    }
    custom {
        "system.describe" => |plane, _, _| Ok(plane.describe()),
        "diagnostics.getVerbose" => |plane, _, _| plane.dispatch_verbose_diagnostics(None),
        "diagnostics.setVerbose" => |plane, _, params| plane.dispatch_verbose_diagnostics(Some(params.unwrap_or(Value::Null))),
        "devices.getAccess" => |plane, _, _| Ok(json!({ "allowed": plane.device_access_allowed })),
        "startup.get" => |plane, _, _| Ok(json!({
            "enabled": plane.startup_enabled,
            "registration": "unavailable",
            "reason": "native sign-in registration is owned by the desktop shell"
        })),
        "plugins.inventory" => |plane, _, _| Ok(json!({ "inventories": plane.remembered_plugin_inventories() })),
        "plugins.openEditor" => |plane, _, params| plane.dispatch_plugins_editor(params, true),
        "plugins.closeEditor" => |plane, _, params| plane.dispatch_plugins_editor(params, false),
        "nodes.types" => |plane, _, _| Ok(plane.describe()["nodeTypes"].clone()),
        "nodes.describe" => |plane, _, _| Ok(plane.describe()["nodeTypes"].clone()),
        "presets.list" => |plane, _, _| Ok(plane.describe()["presets"].clone()),
        "processors.list" => |plane, _, _| Ok(plane.describe()["processors"].clone()),
        "sessions.active.get" => |plane, _, _| Ok(plane.dispatch_active_session_get())
    }
}

pub(super) fn dispatch_method(
    plane: &mut ControlPlane,
    method: &str,
    params: Option<Value>,
) -> Result<Value, ControlError> {
    let entry = METHOD_HANDLERS
        .iter()
        .find(|entry| entry.name == method)
        .ok_or_else(|| ControlError::InvalidRequest("method not found".into()))?;
    (entry.handler)(plane, method, params)
}

impl ControlPlane {
    fn dispatch_diagnostics_snapshot(&mut self) -> Result<Value, ControlError> {
        let (recent_recovery_crashes, recovery_safe_mode) = self.recovery_status()?;
        let (audio, audio_reason) = self.audio_status();
        let mut node_telemetry = self
            .native_node_telemetry()
            .as_array()
            .cloned()
            .unwrap_or_default();
        node_telemetry.extend(
            self.native_multi_input_node_telemetry()
                .as_array()
                .cloned()
                .unwrap_or_default(),
        );
        Ok(json!({
            "build": self.build,
            "backend": "control-plane",
            "storage": if self.storage.is_some() { "sqlite" } else { "memory" },
            "audio": { "state": audio, "reason": audio_reason },
            "nativeAdapter": self.native_adapter_state(),
            "cableRoute": self.cable_route_diagnostics(),
            "nativeAdapterKind": self.native_adapter_kind(),
            "nativeSessionId": self.native_session_id().map(EntityId::as_str),
            "schedulerTelemetry": self.native_scheduler_telemetry(),
            "nodeTelemetry": node_telemetry,
            "applicationCaptureStates": self.application_capture_states(),
            "gameRound": self.siege_round_feed.status_json(),
            "privacyMute": {
                "muted": self.privacy_muted,
                "persistence": if self.storage.is_some() { "durable" } else { "memory" }
            },
            "recovery": {
                "safeMode": recovery_safe_mode,
                "recentCrashes": recent_recovery_crashes,
                "persistence": if self.storage.is_some() { "durable" } else { "memory" }
            },
            "eventLog": { "latestSequence": self.events.latest_sequence(), "retained": self.events.len() },
            "redacted": true
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_discovered_method_has_exactly_one_typed_handler() {
        assert_eq!(METHOD_HANDLERS.len(), API_METHODS.len());
        for spec in API_METHODS {
            assert_eq!(
                METHOD_HANDLERS
                    .iter()
                    .filter(|entry| entry.name == spec.name)
                    .count(),
                1,
                "{}",
                spec.name
            );
        }
        for entry in METHOD_HANDLERS {
            assert!(
                API_METHODS.iter().any(|spec| spec.name == entry.name),
                "undiscovered handler {}",
                entry.name
            );
        }
    }
}
