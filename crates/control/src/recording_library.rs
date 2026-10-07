//! The recordings library: list, preview, metadata, rename, reveal, removal and recovery.

use super::*;

impl ControlPlane {
    pub(crate) fn dispatch_recordings_list(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let session_id = params.get("sessionId").and_then(Value::as_str);
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
        if !(1..=MAX_RECORDING_LIST_ITEMS as u64).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        let Some(storage) = &self.storage else {
            return Ok(if paged {
                json!({ "items": [], "nextCursor": null })
            } else {
                json!([])
            });
        };
        let (records, has_more) = storage
            .list_recordings_page(
                session_id,
                cursor,
                if paged {
                    limit as usize
                } else {
                    MAX_RECORDING_LIST_ITEMS
                },
            )
            .map_err(storage_error)?;
        if !paged && has_more {
            return Err(ControlError::InvalidRequest(
                "recordings.list requires cursor pagination when more than 500 records exist"
                    .into(),
            ));
        }
        let values = records
            .into_iter()
            .map(|record| {
                let node_id = storage
                    .load_recording_node_binding(record.id.as_str())
                    .map_err(storage_error)?;
                Ok(json!({
                    "id": record.id,
                    "sessionId": record.session_id,
                    "recorderId": record.recorder_id,
                    "nodeId": node_id,
                    "path": record.path,
                    "format": record.format,
                    "channels": record.channels,
                    "sampleRate": record.sample_rate,
                    "frames": record.frames,
                    "fileBytes": record.file_bytes,
                    "startTime": record.start_time,
                    "state": record.state,
                    "missing": record.missing,
                    "title": record.title,
                    "artist": record.artist,
                    "comment": record.comment,
                    "dither": record.dither,
                    "conversion": record.conversion
                }))
            })
            .collect::<Result<Vec<_>, ControlError>>()?;
        if paged {
            let next_cursor = has_more
                .then(|| values.last().and_then(|value| value["id"].as_str()))
                .flatten();
            Ok(json!({ "items": values, "nextCursor": next_cursor }))
        } else {
            Ok(json!(values))
        }
    }

    pub(crate) fn dispatch_recorders_list(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        if let Some(params) = params {
            if !params.is_object() || !params.as_object().is_some_and(|object| object.is_empty()) {
                return Err(ControlError::InvalidRequest(
                    "recorders.list does not accept parameters".into(),
                ));
            }
        }
        let mut recorders = self
            .recorders
            .iter()
            .map(|(session_id, recorder)| {
                let checkpoint = recorder.checkpoint();
                json!({
                    "sessionId": session_id,
                    "state": recorder_state_name(recorder.state()),
                    "lastFrame": checkpoint.last_frame
                })
            })
            .collect::<Vec<_>>();
        recorders.extend(
            self.recorder_node_states
                .iter()
                .filter_map(|(node_id, recorder)| {
                    let session_id = self.recorder_node_sessions.get(node_id)?;
                    let checkpoint = recorder.checkpoint();
                    // The newest audio the tap received: clients stop or
                    // split "now" at this frame without knowing the timeline.
                    let received = self
                        .recorder_node_workers
                        .get(node_id)
                        .and_then(|worker| worker.committed_end_frame());
                    let last_frame = match (checkpoint.last_frame, received) {
                        (Some(transition), Some(received)) => Some(transition.max(received)),
                        (transition, received) => transition.or(received),
                    };
                    Some(json!({
                        "sessionId": session_id,
                        "nodeId": node_id,
                        "state": recorder_state_name(recorder.state()),
                        "lastFrame": last_frame
                    }))
                }),
        );
        recorders.sort_by(|left, right| {
            left["sessionId"]
                .as_str()
                .cmp(&right["sessionId"].as_str())
                .then_with(|| left["nodeId"].as_str().cmp(&right["nodeId"].as_str()))
        });
        Ok(Value::Array(recorders))
    }

    pub(crate) fn dispatch_recordings_get(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let recording_id = params
            .as_ref()
            .and_then(|params| {
                params
                    .get("recordingId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let Some(storage) = &self.storage else {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        };
        let record = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let node_id = storage
            .load_recording_node_binding(record.id.as_str())
            .map_err(storage_error)?;
        Ok(json!({
            "id": record.id,
            "sessionId": record.session_id,
            "recorderId": record.recorder_id,
            "nodeId": node_id,
            "path": record.path,
            "format": record.format,
            "channels": record.channels,
            "sampleRate": record.sample_rate,
            "frames": record.frames,
            "fileBytes": record.file_bytes,
            "startTime": record.start_time,
            "state": record.state,
            "missing": record.missing,
            "title": record.title,
            "artist": record.artist,
            "comment": record.comment
        }))
    }

    pub(crate) fn dispatch_recording_recovery(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let storage = self.storage.as_ref().ok_or_else(|| {
            ControlError::InvalidRequest("recording recovery is unavailable".into())
        })?;
        let params = params.unwrap_or_else(|| json!({}));
        let recording_id_supplied = params.get("recordingId").is_some();
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        if recording_id_supplied && recording_id.is_none() {
            return Err(ControlError::InvalidRequest(
                "recordingId must be a non-empty string".into(),
            ));
        }
        if recording_id.is_none() {
            let cursor = params
                .get("cursor")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty());
            let limit = params
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| usize::try_from(value).unwrap_or(usize::MAX))
                .unwrap_or(MAX_RECORDING_LIST_ITEMS);
            let (ids, has_more) = storage
                .list_recording_checkpoint_ids(cursor, limit)
                .map_err(storage_error)?;
            let mut items = Vec::with_capacity(ids.len());
            for id in &ids {
                let item = match storage.load_recording_checkpoint(id) {
                    Ok(Some(checkpoint)) => json!({
                        "recordingId": id,
                        "status": "available",
                        "checkpoint": checkpoint
                    }),
                    Ok(None) => json!({
                        "recordingId": id,
                        "status": "missing"
                    }),
                    Err(StorageError::InvalidRecording(_)) => json!({
                        "recordingId": id,
                        "status": "invalid"
                    }),
                    Err(error) => return Err(storage_error(error)),
                };
                items.push(item);
            }
            let next_cursor = has_more.then(|| ids.last().cloned()).flatten();
            return Ok(json!({ "items": items, "nextCursor": next_cursor }));
        }
        let recording_id = recording_id.expect("recording ID checked above");
        let Some(checkpoint) = storage
            .load_recording_checkpoint(&recording_id)
            .map_err(storage_error)?
        else {
            return Ok(json!({
                "recordingId": recording_id,
                "status": "missing"
            }));
        };
        Ok(json!({
            "recordingId": recording_id,
            "status": "available",
            "checkpoint": checkpoint
        }))
    }

    pub(crate) fn dispatch_recording_reveal(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let recording_id = params
            .and_then(|params| {
                params
                    .get("recordingId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let record = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let path = std::path::Path::new(&record.path);
        if !path.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "recording path must be absolute".into(),
            ));
        }
        if !path.is_file() {
            return Ok(
                json!({ "recordingId": recording_id, "path": record.path, "revealed": false, "reason": "missing" }),
            );
        }
        audiorouter_storage::validate_recording_file_path(path).map_err(storage_error)?;
        #[cfg(windows)]
        let revealed = std::process::Command::new("explorer.exe")
            .args(["/select,", &record.path])
            .spawn()
            .map(|_| true)
            .map_err(|error| {
                ControlError::InvalidRequest(format!("unable to reveal recording: {error}"))
            })?;
        #[cfg(not(windows))]
        let revealed = false;
        Ok(json!({ "recordingId": recording_id, "path": record.path, "revealed": revealed }))
    }

    pub(crate) fn dispatch_recordings_preview(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let recording_id = params
            .and_then(|params| {
                params
                    .get("recordingId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let record = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let path = std::path::Path::new(&record.path);
        if !path.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "recording path must be absolute".into(),
            ));
        }
        if path.is_file() {
            audiorouter_storage::validate_recording_file_path(path).map_err(storage_error)?;
        }
        let status = audiorouter_recording::inspect_recording(&record.path).map_err(|error| {
            ControlError::InvalidRequest(format!("recording preview failed: {error:?}"))
        })?;
        let result = match status {
            audiorouter_recording::RecordingFileStatus::Present(info) => json!({
                "status": "present",
                "format": "wav",
                "channels": info.channels,
                "sampleRate": info.sample_rate,
                "frames": info.frames,
                "dataBytes": info.data_bytes,
                "fileBytes": info.file_bytes
            }),
            audiorouter_recording::RecordingFileStatus::FlacPresent(info) => json!({
                "status": "present",
                "format": "flac",
                "channels": info.channels,
                "sampleRate": info.sample_rate,
                "bitsPerSample": info.bits_per_sample,
                "frames": info.frames,
                "fileBytes": info.file_bytes
            }),
            audiorouter_recording::RecordingFileStatus::Mp3Present(info) => json!({
                "status": "present",
                "format": "mp3",
                "fileBytes": info.file_bytes
            }),
            audiorouter_recording::RecordingFileStatus::Missing => json!({ "status": "missing" }),
            audiorouter_recording::RecordingFileStatus::Invalid => json!({ "status": "invalid" }),
        };
        Ok(json!({ "recordingId": recording_id, "preview": result }))
    }

    pub(crate) fn dispatch_recording_metadata(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({
                    "recordingId": recording_id,
                    "title": params.get("title").and_then(Value::as_str),
                    "artist": params.get("artist").and_then(Value::as_str),
                    "comment": params.get("comment").and_then(Value::as_str)
                });
                (
                    self.scoped_idempotency_key("recordings.setMetadata", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let session_id = storage
            .get_recording(recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?
            .session_id;
        let updated = storage
            .update_recording_metadata(
                recording_id,
                params.get("title").and_then(Value::as_str),
                params.get("artist").and_then(Value::as_str),
                params.get("comment").and_then(Value::as_str),
            )
            .map_err(storage_error)?;
        if !updated {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        }
        let result = json!({ "recordingId": recording_id, "updated": true });
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "recordings.setMetadata", &hash, &result)?;
        }
        self.events.append(
            0,
            None,
            "recording.metadataChanged",
            Some(EntityId::new(session_id)),
        );
        Ok(result)
    }

    pub(crate) fn dispatch_recording_rename(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("recordingId and newPath are required".into())
        })?;
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let new_path = params
            .get("newPath")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("newPath is required".into()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "recordingId": recording_id, "newPath": new_path });
                (
                    self.scoped_idempotency_key("recordings.rename", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let session_id = storage
            .get_recording(recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?
            .session_id;
        if !storage
            .rename_recording(recording_id, new_path)
            .map_err(storage_error)?
        {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        }
        let result = json!({
            "recordingId": recording_id,
            "renamed": true,
            "path": new_path,
            "fileAction": "renamed"
        });
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "recordings.rename", &hash, &result)?;
        }
        self.events.append(
            0,
            None,
            "recording.renamed",
            Some(EntityId::new(session_id)),
        );
        Ok(result)
    }

    pub(crate) fn dispatch_recording_remove(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("recordingId and idempotencyKey are required".into())
        })?;
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "recordingId": recording_id });
                (
                    self.scoped_idempotency_key("recordings.removeEntry", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let session_id = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?
            .session_id;
        if !storage
            .remove_recording_entry(&recording_id)
            .map_err(storage_error)?
        {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        }
        let result = json!({ "recordingId": recording_id, "removed": true, "fileAction": "none" });
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "recordings.removeEntry", &hash, &result)?;
        }
        self.events.append(
            0,
            None,
            "recording.entryRemoved",
            Some(EntityId::new(session_id)),
        );
        Ok(result)
    }

    pub(crate) fn dispatch_recording_recycle(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("recordingId and confirm are required".into())
        })?;
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let confirm = params
            .get("confirm")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if confirm {
            params
                .get("idempotencyKey")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    ControlError::InvalidRequest(
                        "idempotencyKey is required for confirmed recycle".into(),
                    )
                })?;
        }
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let record = storage
            .get_recording(recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let path = std::path::Path::new(&record.path);
        if !path.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "recording path must be absolute".into(),
            ));
        }
        if !path.is_file() {
            return Ok(
                json!({ "recordingId": recording_id, "path": record.path, "fileAction": "none", "reason": "missing" }),
            );
        }
        audiorouter_storage::validate_recording_file_path(path).map_err(storage_error)?;
        if !confirm {
            return Ok(
                json!({ "recordingId": recording_id, "path": record.path, "fileAction": "recycle", "preview": true }),
            );
        }
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "recordingId": recording_id, "confirm": true });
                (
                    self.scoped_idempotency_key("recordings.recycle", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        #[cfg(windows)]
        {
            trash::delete(path).map_err(|error| {
                ControlError::InvalidRequest(format!("recycleUnavailable: {error}"))
            })?;
            storage
                .set_recording_missing(recording_id, true)
                .map_err(storage_error)?;
            let result = json!({ "recordingId": recording_id, "path": record.path, "fileAction": "recycled", "missing": true });
            if let Some((key, hash)) = operation {
                self.journal_idempotent_result(&key, "recordings.recycle", &hash, &result)?;
            }
            self.events.append(
                0,
                None,
                "recording.recycled",
                Some(EntityId::new(record.session_id.clone())),
            );
            Ok(result)
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Ok(
                json!({ "recordingId": recording_id, "path": record.path, "fileAction": "none", "reason": "recycleUnavailable" }),
            )
        }
    }
}
