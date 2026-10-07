//! Audio media: uploads, temporary recordings and audio source transport.

use super::*;

pub(crate) fn decode_base64_chunk(value: &str) -> Result<Vec<u8>, ControlError> {
    fn digit(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    if value.is_empty()
        || value.len() > AUDIO_UPLOAD_CHUNK_BYTES * 4 / 3 + 8
        || value.len() % 4 != 0
    {
        return Err(ControlError::InvalidRequest(
            "audio upload chunk size or base64 framing is invalid".into(),
        ));
    }
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len() / 4 * 3);
    for (index, group) in bytes.chunks_exact(4).enumerate() {
        let last = index + 1 == bytes.len() / 4;
        let a = digit(group[0]).ok_or_else(|| {
            ControlError::InvalidRequest("audio upload chunk is not valid base64".into())
        })?;
        let b = digit(group[1]).ok_or_else(|| {
            ControlError::InvalidRequest("audio upload chunk is not valid base64".into())
        })?;
        output.push((a << 2) | (b >> 4));
        match (group[2], group[3]) {
            (b'=', b'=') if last => {
                if b & 0x0f != 0 {
                    return Err(ControlError::InvalidRequest(
                        "audio upload chunk has invalid base64 padding".into(),
                    ));
                }
            }
            (c, b'=') if last => {
                let c = digit(c).ok_or_else(|| {
                    ControlError::InvalidRequest("audio upload chunk is not valid base64".into())
                })?;
                if c & 0x03 != 0 {
                    return Err(ControlError::InvalidRequest(
                        "audio upload chunk has invalid base64 padding".into(),
                    ));
                }
                output.push((b << 4) | (c >> 2));
            }
            (c, d) => {
                if c == b'=' || d == b'=' {
                    return Err(ControlError::InvalidRequest(
                        "audio upload chunk has invalid base64 padding".into(),
                    ));
                }
                let c = digit(c).ok_or_else(|| {
                    ControlError::InvalidRequest("audio upload chunk is not valid base64".into())
                })?;
                let d = digit(d).ok_or_else(|| {
                    ControlError::InvalidRequest("audio upload chunk is not valid base64".into())
                })?;
                output.push((b << 4) | (c >> 2));
                output.push((c << 6) | d);
            }
        }
    }
    if output.len() > AUDIO_UPLOAD_CHUNK_BYTES {
        return Err(ControlError::InvalidRequest(
            "audio upload chunk exceeds its size limit".into(),
        ));
    }
    Ok(output)
}

pub(crate) struct AudioMediaUpload {
    id: String,
    file_name: String,
    format: String,
    expected_bytes: usize,
    next_chunk: u32,
    bytes: Vec<u8>,
    started_at: Instant,
}

impl ControlPlane {
    /// Decode, at the graph rate, the stored media referenced by enabled
    /// Audio File sources and FIR Filter impulse responses. Runs on the
    /// control thread before compilation, never in the callback.
    pub(crate) fn session_audio_media(
        &mut self,
        session: &Session,
        sample_rate_hz: u32,
    ) -> Result<HashMap<String, Arc<DecodedAudio>>, ControlError> {
        let mut media = HashMap::<String, Arc<DecodedAudio>>::new();
        for node in session.nodes.iter().filter(|node| {
            node.enabled
                && (node.kind == NodeKind::AudioFile
                    || (node.kind == NodeKind::FirFilter
                        && node.parameters.contains_key("mediaId")))
        }) {
            let media_id = node
                .parameters
                .get("mediaId")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    ControlError::InvalidRequest(
                        "select a WAV or MP3 file for every Audio File source".into(),
                    )
                })?;
            if media.contains_key(media_id) {
                continue;
            }
            let (_, format, bytes) = self
                .storage
                .as_ref()
                .ok_or_else(|| {
                    ControlError::InvalidRequest("audio source storage is unavailable".into())
                })?
                .load_audio_media(media_id)
                .map_err(storage_error)?
                .ok_or_else(|| {
                    ControlError::InvalidRequest(
                        "audio source media is missing; select the file again".into(),
                    )
                })?;
            let decoded = self
                .while_servicing_audio(|| {
                    audiorouter_engine::decode_audio_bytes(bytes, &format, sample_rate_hz)
                })
                .map_err(|error| {
                    ControlError::InvalidRequest(format!(
                        "audio source could not be decoded: {error}"
                    ))
                })?;
            media.insert(media_id.to_owned(), Arc::new(decoded));
        }
        Ok(media)
    }

    pub(crate) fn compile_session_graph_with_audio(
        &mut self,
        session: &Session,
        generation: RuntimeGeneration,
        sample_rate_hz: u32,
        plugins: &HashMap<EntityId, Arc<dyn RealtimePluginProcessor>>,
    ) -> Result<audiorouter_engine::RuntimeGraph, ControlError> {
        let media = self.session_audio_media(session, sample_rate_hz)?;
        // The application (process-loopback) worker opens no physical capture,
        // so a turned-off microphone or Test Signal left wired into a Mixer
        // only adds silence and is dropped before compiling. Endpoint routes
        // keep those nodes: a disabled physical input's mute stage is what
        // silences the microphone that worker actually captures.
        let application_worker = self
            .application_capture_runtime
            .as_ref()
            .is_some_and(|binding| binding.session_id == session.id)
            && self.native_adapter_kind() == Some("process-loopback");
        let pruned = if application_worker {
            audiorouter_engine::prune_inactive_upstream(session)
        } else {
            std::borrow::Cow::Borrowed(session)
        };
        let graph = audiorouter_engine::compile_session_at_sample_rate_with_plugins_and_audio(
            pruned.as_ref(),
            generation,
            sample_rate_hz,
            plugins,
            &media,
        )
        .map_err(|error| {
            ControlError::InvalidRequest(format!("native graph rejected: {error:?}"))
        })?;
        self.audio_file_sources
            .retain(|(owner_session, _), _| owner_session != &session.id);
        self.test_signal_sources
            .retain(|(owner_session, _), _| owner_session != &session.id);
        for node in session
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::AudioFile)
        {
            if let Some(source) = graph.audio_file_source_for_node(&node.id) {
                self.audio_file_sources
                    .insert((session.id.clone(), node.id.clone()), source);
            }
        }
        for node in session
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::TestSignal)
        {
            if let Some(source) = graph.test_signal_source_for_node(&node.id) {
                self.test_signal_sources
                    .insert((session.id.clone(), node.id.clone()), source);
            }
        }
        Ok(graph)
    }

    pub(crate) fn dispatch_audio_media_begin_upload(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("fileName and sizeBytes are required".into())
        })?;
        let file_name = params
            .get("fileName")
            .and_then(Value::as_str)
            .filter(|name| {
                !name.is_empty()
                    && name.len() <= 256
                    && !name.chars().any(|ch| matches!(ch, '/' | '\\' | ':'))
            })
            .ok_or_else(|| {
                ControlError::InvalidRequest("fileName must be a simple WAV or MP3 filename".into())
            })?;
        let format = file_name
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .filter(|extension| matches!(extension.as_str(), "wav" | "mp3"))
            .ok_or_else(|| {
                ControlError::InvalidRequest("only WAV and MP3 files are supported".into())
            })?;
        let size_bytes = params
            .get("sizeBytes")
            .and_then(Value::as_u64)
            .filter(|size| (1..=audiorouter_storage::MAX_AUDIO_MEDIA_BYTES as u64).contains(size))
            .ok_or_else(|| {
                ControlError::InvalidRequest("audio file size is empty or exceeds 64 MiB".into())
            })? as usize;
        if self
            .audio_upload
            .as_ref()
            .is_some_and(|upload| upload.started_at.elapsed() <= AUDIO_UPLOAD_TTL)
        {
            return Err(ControlError::InvalidRequest(
                "an audio upload is already in progress".into(),
            ));
        }
        self.audio_upload = None;
        let id = format!("audio-upload-{}", self.next_audio_upload);
        self.next_audio_upload = self.next_audio_upload.checked_add(1).ok_or_else(|| {
            ControlError::InvalidRequest("audio upload ID space exhausted".into())
        })?;
        self.audio_upload = Some(AudioMediaUpload {
            id: id.clone(),
            file_name: file_name.to_owned(),
            format,
            expected_bytes: size_bytes,
            next_chunk: 0,
            bytes: Vec::with_capacity(size_bytes),
            started_at: Instant::now(),
        });
        Ok(json!({ "uploadId": id, "chunkBytes": AUDIO_UPLOAD_CHUNK_BYTES }))
    }

    pub(crate) fn dispatch_audio_media_upload_chunk(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("upload chunk parameters are required".into())
        })?;
        let id = params.get("uploadId").and_then(Value::as_str).unwrap_or("");
        let index = params
            .get("chunkIndex")
            .and_then(Value::as_u64)
            .and_then(|index| u32::try_from(index).ok())
            .ok_or_else(|| {
                ControlError::InvalidRequest("chunkIndex must be a bounded integer".into())
            })?;
        let encoded = params
            .get("dataBase64")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("dataBase64 is required".into()))?;
        let chunk = decode_base64_chunk(encoded)?;
        let upload = self
            .audio_upload
            .as_mut()
            .filter(|upload| upload.id == id && upload.started_at.elapsed() <= AUDIO_UPLOAD_TTL)
            .ok_or_else(|| {
                ControlError::InvalidRequest("audio upload is missing or expired".into())
            })?;
        if index != upload.next_chunk
            || chunk.is_empty()
            || upload
                .bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|length| length > upload.expected_bytes)
        {
            return Err(ControlError::InvalidRequest(
                "audio upload chunks must be nonempty, ordered, and within the declared file size"
                    .into(),
            ));
        }
        upload.bytes.extend_from_slice(&chunk);
        upload.next_chunk = upload.next_chunk.checked_add(1).ok_or_else(|| {
            ControlError::InvalidRequest("audio upload chunk index exhausted".into())
        })?;
        Ok(json!({ "receivedBytes": upload.bytes.len(), "nextChunkIndex": upload.next_chunk }))
    }

    pub(crate) fn dispatch_audio_media_finish_upload(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let id = params
            .as_ref()
            .and_then(|value| value.get("uploadId"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let upload = self
            .audio_upload
            .take()
            .filter(|upload| upload.id == id && upload.started_at.elapsed() <= AUDIO_UPLOAD_TTL)
            .ok_or_else(|| {
                ControlError::InvalidRequest("audio upload is missing or expired".into())
            })?;
        if upload.bytes.len() != upload.expected_bytes {
            self.audio_upload = Some(upload);
            return Err(ControlError::InvalidRequest(
                "audio upload is incomplete".into(),
            ));
        }
        let decoded = self
            .while_servicing_audio(|| {
                audiorouter_engine::decode_audio_bytes(
                    upload.bytes.clone(),
                    &upload.format,
                    audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
                )
            })
            .map_err(|error| {
                ControlError::InvalidRequest(format!("audio file could not be imported: {error}"))
            })?;
        let media_id = loop {
            let candidate = format!("audio-media-{}", self.next_audio_media);
            self.next_audio_media = self.next_audio_media.checked_add(1).ok_or_else(|| {
                ControlError::InvalidRequest("audio media ID space exhausted".into())
            })?;
            if self
                .storage
                .as_ref()
                .ok_or_else(|| {
                    ControlError::InvalidRequest("persistent backend storage is unavailable".into())
                })?
                .load_audio_media(&candidate)
                .map_err(storage_error)?
                .is_none()
            {
                break candidate;
            }
        };
        let storage = self.storage.as_ref().ok_or_else(|| {
            ControlError::InvalidRequest("audio import requires persistent backend storage".into())
        })?;
        storage
            .store_audio_media(
                &media_id,
                &upload.file_name,
                &upload.format,
                &upload.bytes,
                None,
            )
            .map_err(storage_error)?;
        let duration_ms =
            (decoded.frames() as u64 * 1000 / u64::from(decoded.sample_rate_hz)).max(1);
        Ok(
            json!({ "mediaId": media_id, "fileName": upload.file_name, "format": upload.format,
            "durationMs": duration_ms, "channels": decoded.channels, "sampleRateHz": decoded.sample_rate_hz }),
        )
    }

    pub(crate) fn dispatch_audio_media_import_temporary_recording(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let recording_id = params
            .as_ref()
            .and_then(|value| value.get("recordingId"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let storage = self.storage.as_ref().ok_or_else(|| {
            ControlError::InvalidRequest(
                "temporary audio import requires persistent backend storage".into(),
            )
        })?;
        let record = storage
            .get_recording(recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| {
                ControlError::InvalidRequest("temporary WAV recording was not found".into())
            })?;
        if !record.recorder_id.starts_with("audio-file-take-")
            || record.format != "wav"
            || record.state != "completed"
            || record.missing
        {
            return Err(ControlError::InvalidRequest(
                "only completed temporary WAV takes can be imported".into(),
            ));
        }
        let duration_ms = record.frames.saturating_mul(1000) / u64::from(record.sample_rate.max(1));
        if duration_ms == 0
            || duration_ms > 120_000
            || record.file_bytes == 0
            || record.file_bytes > audiorouter_storage::MAX_AUDIO_MEDIA_BYTES as u64
        {
            return Err(ControlError::InvalidRequest(
                "temporary WAV take exceeds the 120-second or 64 MiB limit".into(),
            ));
        }
        let path = std::path::Path::new(&record.path);
        audiorouter_storage::validate_recording_file_path(path).map_err(storage_error)?;
        let metadata =
            std::fs::metadata(path).map_err(|error| storage_error(StorageError::Io(error)))?;
        if metadata.len() != record.file_bytes
            || metadata.len() > audiorouter_storage::MAX_AUDIO_MEDIA_BYTES as u64
        {
            return Err(ControlError::InvalidRequest(
                "temporary WAV file size changed or exceeds its bound".into(),
            ));
        }
        let wav_info = audiorouter_recording::inspect_wav_file(path).map_err(|error| {
            ControlError::InvalidRequest(format!("temporary WAV header is invalid: {error:?}"))
        })?;
        if u64::from(wav_info.channels) != u64::from(record.channels)
            || wav_info.sample_rate != record.sample_rate
            || wav_info.frames != record.frames
            || wav_info.file_bytes != record.file_bytes
        {
            return Err(ControlError::InvalidRequest(
                "temporary WAV metadata does not match its completed recording".into(),
            ));
        }
        let bytes = std::fs::read(path).map_err(|error| storage_error(StorageError::Io(error)))?;
        let decoded = self
            .while_servicing_audio(|| {
                audiorouter_engine::decode_audio_bytes(
                    bytes.clone(),
                    "wav",
                    audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
                )
            })
            .map_err(|error| {
                ControlError::InvalidRequest(format!("temporary WAV could not be decoded: {error}"))
            })?;
        let storage = self.storage.as_ref().ok_or_else(|| {
            ControlError::InvalidRequest(
                "temporary audio import requires persistent backend storage".into(),
            )
        })?;
        let media_id = loop {
            let candidate = format!("audio-media-{}", self.next_audio_media);
            self.next_audio_media = self.next_audio_media.checked_add(1).ok_or_else(|| {
                ControlError::InvalidRequest("audio media ID space exhausted".into())
            })?;
            if storage
                .load_audio_media(&candidate)
                .map_err(storage_error)?
                .is_none()
            {
                break candidate;
            }
        };
        let expires_at = unix_epoch_seconds().saturating_add(24 * 60 * 60);
        storage
            .store_audio_media(
                &media_id,
                "Temporary voice take.wav",
                "wav",
                &bytes,
                Some(expires_at),
            )
            .map_err(storage_error)?;
        if let Err(error) = std::fs::remove_file(path) {
            let _ = storage.delete_audio_media(&media_id);
            return Err(storage_error(StorageError::Io(error)));
        }
        if let Err(error) = storage.remove_recording_entry(recording_id) {
            let _ = storage.save_recording(&record);
            let _ = storage.delete_audio_media(&media_id);
            return Err(storage_error(error));
        }
        Ok(
            json!({ "mediaId": media_id, "fileName": "Temporary voice take.wav", "format": "wav",
            "durationMs": (decoded.frames() as u64 * 1000 / u64::from(decoded.sample_rate_hz)).max(1),
            "channels": decoded.channels, "sampleRateHz": decoded.sample_rate_hz,
            "expiresAt": expires_at, "sourceRemoved": true }),
        )
    }

    pub(crate) fn dispatch_audio_media_delete(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let media_id = params
            .as_ref()
            .and_then(|value| value.get("mediaId"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let referenced = self
            .store
            .sessions(audiorouter_domain::MAX_SESSIONS_GLOBAL)
            .iter()
            .any(|session| {
                session.nodes.iter().any(|node| {
                    node.kind == NodeKind::AudioFile
                        && node.parameters.get("mediaId").and_then(Value::as_str) == Some(media_id)
                })
            });
        if referenced {
            return Err(ControlError::InvalidRequest(
                "audio media is still referenced by a session".into(),
            ));
        }
        let deleted = self
            .storage
            .as_ref()
            .ok_or_else(|| {
                ControlError::InvalidRequest("persistent backend storage is unavailable".into())
            })?
            .delete_audio_media(media_id)
            .map_err(storage_error)?;
        Ok(json!({ "deleted": deleted }))
    }

    pub(crate) fn dispatch_audio_source_transport(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId, nodeId, and action are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let node_id = params
            .get("nodeId")
            .and_then(Value::as_str)
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("nodeId is required".into()))?;
        let action = params.get("action").and_then(Value::as_str).unwrap_or("");
        if !matches!(action, "play" | "pause" | "stop" | "status") {
            return Err(ControlError::InvalidRequest(
                "action must be play, pause, stop, or status".into(),
            ));
        }
        if !self
            .runtimes
            .get(&session_id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
        {
            return Err(ControlError::InvalidRequest(
                "start the prepared session before controlling its audio source".into(),
            ));
        }
        let key = (session_id.clone(), node_id.clone());
        if let Some(source) = self.test_signal_sources.get(&key) {
            match action {
                "play" => source.play(),
                "stop" => source.stop(),
                "pause" => {
                    return Err(ControlError::InvalidRequest(
                        "Test Signal supports play and stop, not pause".into(),
                    ));
                }
                _ => {}
            }
            return Ok(json!({ "sessionId": session_id, "nodeId": node_id,
                "state": if source.is_playing() { "playing" } else { "stopped" },
                "loop": false }));
        }
        let source = self.audio_file_sources.get(&key).ok_or_else(|| {
            ControlError::InvalidRequest(
                "audio source is not prepared for this running session".into(),
            )
        })?;
        match action {
            "play" => source.play(),
            "pause" => source.pause(),
            "stop" => source.stop(),
            _ => {}
        }
        Ok(json!({ "sessionId": session_id, "nodeId": node_id,
            "state": source.transport_state(),
            "loop": source.is_looping() }))
    }
}
