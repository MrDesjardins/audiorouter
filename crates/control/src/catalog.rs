//! Discovery: describe, node parameter schemas, the processor catalog and processor responses.

use super::*;

/// Live spectrum of a FIR Filter Hz node for its Properties graph: level per
/// band (power dB, one decimal) and each band's centre frequency.
pub(crate) fn spectrum_telemetry(levels: &audiorouter_engine::SpectrumLevels) -> Value {
    let round = |value: f32| (f64::from(value) * 10.0).round() / 10.0;
    json!({
        "levelsDb": levels.iter().map(|level| round(*level)).collect::<Vec<_>>(),
        "bandFrequenciesHz": audiorouter_engine::spectrum_band_frequencies_hz().iter().map(|hz| round(*hz)).collect::<Vec<_>>(),
    })
}

impl ControlPlane {
    pub fn describe(&self) -> Value {
        let methods: Vec<MethodDescription> = API_METHODS.iter().copied().map(Into::into).collect();
        let nodes: Vec<Value> = node_registry().into_iter().map(|spec| {
            let availability = match spec.availability {
                audiorouter_domain::CapabilityAvailability::Available => json!({ "status": "available" }),
                audiorouter_domain::CapabilityAvailability::Unavailable(reason) => json!({ "status": "unavailable", "reason": reason }),
            };
            json!({ "type": format!("{}@{}", spec.kind.type_name(), spec.version), "availability": availability, "realtimeCostClass": spec.realtime_cost_class, "latencySamples": spec.latency_samples, "parameters": Self::node_parameter_schema(spec.kind) })
        }).collect();
        let voice_chains = audiorouter_dsp::VoiceChainPresetId::ALL
            .into_iter()
            .map(|preset| {
                json!({
                    "id": preset.id(),
                    "version": preset.version(),
                    "name": preset.name(),
                    "description": preset.description()
                })
            })
            .collect::<Vec<_>>();
        let eq = audiorouter_dsp::EqPresetId::ALL
            .into_iter()
            .map(|preset| {
                json!({
                    "id": preset.id(),
                    "version": preset.version(),
                    "name": preset.name(),
                    "description": preset.description()
                })
            })
            .collect::<Vec<_>>();
        json!({
            "protocolVersion": { "major": 1, "minor": 0 },
            "schemaVersion": 1,
            "build": self.build,
            "methods": methods,
            "nodeTypes": nodes,
            "processors": Self::processor_catalog(),
            "presets": { "voiceChains": voice_chains, "eq": eq },
            "limits": {
                "maxNodesPerSession": audiorouter_domain::MAX_NODES_PER_SESSION,
                "maxEdgesPerSession": audiorouter_domain::MAX_EDGES_PER_SESSION,
                "maxNodesGlobal": audiorouter_domain::MAX_NODES_GLOBAL,
                "maxEdgesGlobal": audiorouter_domain::MAX_EDGES_GLOBAL,
                "maxSessionsGlobal": audiorouter_domain::MAX_SESSIONS_GLOBAL,
                "maxActiveSessions": audiorouter_domain::MAX_ACTIVE_SESSIONS,
                "maxActiveRecorders": MAX_ACTIVE_RECORDERS,
                "maxClientEnrollments": audiorouter_storage::MAX_CLIENT_ENROLLMENTS,
                "maxOperationJournalEntries": audiorouter_storage::MAX_OPERATION_JOURNAL_ENTRIES,
                "maxVirtualBuses": audiorouter_domain::MAX_VIRTUAL_BUSES,
                "maxVirtualBusNameChars": audiorouter_domain::MAX_VIRTUAL_BUS_NAME_CHARS,
                "maxEntityIdBytes": audiorouter_domain::MAX_ENTITY_ID_BYTES,
                "maxDisplayNameBytes": audiorouter_domain::MAX_DISPLAY_NAME_BYTES,
                "maxPortNameBytes": audiorouter_domain::MAX_PORT_NAME_BYTES,
                "maxPortsPerNode": audiorouter_domain::MAX_PORTS_PER_NODE,
                "maxChannelMatrixCoefficients": audiorouter_domain::MAX_CHANNEL_MATRIX_COEFFICIENTS,
                "maxControlValueDepth": MAX_CONTROL_VALUE_DEPTH,
                "maxControlStringBytes": MAX_CONTROL_STRING_BYTES,
                "maxControlValueCount": MAX_CONTROL_VALUE_COUNT,
                "maxMethodNameBytes": MAX_METHOD_NAME_BYTES,
                "maxRequestIdBytes": MAX_REQUEST_ID_BYTES,
                "maxRevisionCursorBytes": MAX_REVISION_CURSOR_BYTES
            },
            "events": {
                "stateCategories": STATE_CATEGORIES,
                "meterReplay": false,
                "retention": {
                    "maxEvents": audiorouter_domain::MAX_RETAINED_EVENTS,
                    "maxAgeSeconds": 900
                }
            }
        })
    }

    pub(crate) fn node_parameter_schema(kind: audiorouter_domain::NodeKind) -> Value {
        match kind {
            audiorouter_domain::NodeKind::TestSignal => json!([
                { "name": "frequencyHz", "type": "number", "unit": "Hz", "minimum": 20.0, "maximum": 20000.0, "default": 440.0 },
                { "name": "levelDb", "type": "number", "unit": "dBFS", "minimum": -60.0, "maximum": 0.0, "default": -18.0 },
                { "name": "durationMs", "type": "number", "unit": "ms", "minimum": 1.0, "maximum": 600000.0, "default": 1000.0 }
            ]),
            audiorouter_domain::NodeKind::Gain => json!([{
                "name": "gainDb",
                "type": "number",
                "unit": "dB",
                "minimum": -60.0,
                "maximum": 24.0,
                "default": 0.0
            }]),
            audiorouter_domain::NodeKind::Mute => json!([{
                "name": "muted",
                "type": "boolean",
                "default": false
            }]),
            audiorouter_domain::NodeKind::ParametricEq => Self::parametric_eq_parameters(),
            audiorouter_domain::NodeKind::Compressor => json!([
                { "name": "thresholdDb", "type": "number", "unit": "dBFS", "minimum": -60.0, "maximum": 0.0, "default": -18.0 },
                { "name": "ratio", "type": "number", "minimum": 1.0, "maximum": 20.0, "default": 3.0 },
                { "name": "attackMs", "type": "number", "unit": "ms", "minimum": 0.1, "maximum": 200.0, "default": 10.0 },
                { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 2000.0, "default": 150.0 },
                { "name": "kneeDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 24.0, "default": 6.0 },
                { "name": "makeupDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 24.0, "default": 0.0 }
            ]),
            audiorouter_domain::NodeKind::Gate => json!([
                { "name": "thresholdDb", "type": "number", "unit": "dBFS", "minimum": -80.0, "maximum": 0.0, "default": -45.0 },
                { "name": "rangeDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 80.0, "default": 60.0 },
                { "name": "attackMs", "type": "number", "unit": "ms", "minimum": 0.1, "maximum": 100.0, "default": 5.0 },
                { "name": "hysteresisDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 12.0, "default": 3.0 },
                { "name": "ratio", "type": "number", "minimum": 1.0, "maximum": 20.0, "default": 4.0 },
                { "name": "holdMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 1000.0, "default": 50.0 },
                { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 2000.0, "default": 150.0 }
            ]),
            audiorouter_domain::NodeKind::Limiter => json!([
                { "name": "ceilingDb", "type": "number", "unit": "dBFS", "minimum": -12.0, "maximum": 0.0, "default": -1.0 },
                { "name": "lookaheadMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 10.0, "default": 5.0 },
                { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 1000.0, "default": 100.0 }
            ]),
            audiorouter_domain::NodeKind::Delay => json!([
                { "name": "delayMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 1000.0, "default": 0.0 }
            ]),
            audiorouter_domain::NodeKind::GraphicEq => json!((0..10)
                .map(|index| json!({
                    "name": format!("band{index}Db"), "type": "number", "unit": "dB",
                    "minimum": -18.0, "maximum": 18.0, "default": 0.0
                }))
                .collect::<Vec<_>>()),
            audiorouter_domain::NodeKind::Pitch => json!([
                { "name": "semitones", "type": "number", "unit": "semitones", "minimum": -12.0, "maximum": 12.0, "default": 0.0 },
                { "name": "cents", "type": "number", "unit": "cents", "minimum": -100.0, "maximum": 100.0, "default": 0.0 }
            ]),
            audiorouter_domain::NodeKind::Plugin => json!([]),
            audiorouter_domain::NodeKind::Volume => json!([{
                "name": "percent",
                "type": "number",
                "unit": "%",
                "minimum": 0.0,
                "maximum": 200.0,
                "default": 100.0
            }]),
            audiorouter_domain::NodeKind::BassTreble => json!([
                { "name": "bassDb", "type": "number", "unit": "dB", "minimum": -12.0, "maximum": 12.0, "default": 0.0 },
                { "name": "trebleDb", "type": "number", "unit": "dB", "minimum": -12.0, "maximum": 12.0, "default": 0.0 },
                { "name": "bassFrequencyHz", "type": "number", "unit": "Hz", "minimum": 80.0, "maximum": 1000.0, "default": 500.0 },
                { "name": "trebleFrequencyHz", "type": "number", "unit": "Hz", "minimum": 800.0, "maximum": 12000.0, "default": 1500.0 }
            ]),
            audiorouter_domain::NodeKind::Dehum => json!([
                { "name": "frequencyHz", "type": "number", "unit": "Hz", "minimum": 45.0, "maximum": 65.0, "default": 60.0 },
                { "name": "amountPercent", "type": "number", "unit": "%", "minimum": 0.0, "maximum": 100.0, "default": 50.0 },
                { "name": "harmonics", "type": "number", "step": 1.0, "minimum": 1.0, "maximum": 8.0, "default": 4.0 }
            ]),
            audiorouter_domain::NodeKind::Denoise => json!([
                { "name": "reductionPercent", "type": "number", "unit": "%", "minimum": 0.0, "maximum": 100.0, "default": 70.0 },
                { "name": "floorPercent", "type": "number", "unit": "%", "minimum": 0.0, "maximum": 100.0, "default": 10.0 },
                { "name": "learning", "type": "boolean", "default": false }
            ]),
            audiorouter_domain::NodeKind::TimeShift => json!([
                { "name": "bufferSeconds", "type": "number", "step": 1.0, "unit": "s", "minimum": 10.0, "maximum": 120.0, "default": 60.0 }
            ]),
            audiorouter_domain::NodeKind::FirFilter => json!([
                { "name": "wetPercent", "type": "number", "unit": "%", "minimum": 0.0, "maximum": 100.0, "default": 100.0 },
                { "name": "gainDb", "type": "number", "unit": "dB", "minimum": -24.0, "maximum": 12.0, "default": 0.0 }
            ]),
            audiorouter_domain::NodeKind::SpeechDenoise => json!([
                { "name": "strengthPercent", "type": "number", "unit": "%", "minimum": 0.0, "maximum": 100.0, "default": 70.0 }
            ]),
            audiorouter_domain::NodeKind::SpectralGate => json!([
                { "name": "thresholdDb", "type": "number", "unit": "dB", "minimum": -20.0, "maximum": 20.0, "default": 3.0 },
                { "name": "reductionDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 80.0, "default": 40.0 },
                { "name": "learning", "type": "boolean", "default": false }
            ]),
            // `keyNodeId` names another node of the same session; "reference":
            // "node" tells clients to offer node names and send the node ID.
            audiorouter_domain::NodeKind::Duck => json!([
                { "name": "thresholdDb", "type": "number", "unit": "dBFS", "minimum": -80.0, "maximum": 0.0, "default": -35.0 },
                { "name": "amountDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 40.0, "default": 6.0 },
                { "name": "attackMs", "type": "number", "unit": "ms", "minimum": 1.0, "maximum": 500.0, "default": 20.0 },
                { "name": "holdMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 2000.0, "default": 300.0 },
                { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 20.0, "maximum": 5000.0, "default": 500.0 },
                { "name": "trigger", "type": "string", "enum": ["level", "siegeRound"], "default": "level" },
                { "name": "keyNodeId", "type": "string", "reference": "node" },
                { "name": "duckMenu", "type": "boolean", "default": true },
                { "name": "duckPrep", "type": "boolean", "default": true },
                { "name": "duckBetweenRounds", "type": "boolean", "default": true }
            ]),
            audiorouter_domain::NodeKind::Recorder => json!([
                { "name": "format", "type": "string", "enum": ["wavPcm24", "wavPcm16", "wavFloat32", "flac24", "flac16", "mp3"], "default": "wavPcm24" },
                { "name": "autoRecord", "type": "boolean", "default": false },
                { "name": "splitMinutes", "type": "number", "step": 1.0, "unit": "min", "minimum": 0.0, "maximum": 240.0, "default": 0.0 }
            ]),
            audiorouter_domain::NodeKind::InputSwitch => json!([
                { "name": "selected", "type": "string", "enum": ["a", "b"], "default": "a" },
                { "name": "fade", "type": "string", "enum": ["normal", "slow"], "default": "normal" }
            ]),
            audiorouter_domain::NodeKind::Declick => json!([
                { "name": "thresholdPercent", "type": "number", "unit": "%", "minimum": 0.0, "maximum": 100.0, "default": 50.0 }
            ]),
            // Per-input volume is keyed by the upstream node id, so it is
            // described as a parameter family rather than a fixed name.
            audiorouter_domain::NodeKind::Mixer => json!([{
                "name": audiorouter_domain::MIXER_INPUT_VOLUME_PREFIX,
                "namePattern": "inputVolume:<upstreamNodeId>",
                "type": "number",
                "unit": "%",
                "minimum": 0.0,
                "maximum": 100.0,
                "default": 100.0
            }]),
            _ => json!([]),
        }
    }

    pub(crate) fn parametric_eq_parameters() -> Value {
        let mut parameters = vec![
            json!({ "name": "frequencyHz", "type": "number", "unit": "Hz", "minimum": 20.0, "maximum": 20000.0, "default": 1000.0 }),
            json!({ "name": "q", "type": "number", "minimum": 0.1, "maximum": 20.0, "default": 1.0 }),
            json!({ "name": "gainDb", "type": "number", "unit": "dB", "minimum": -24.0, "maximum": 24.0, "default": 0.0 }),
        ];
        for index in 0..audiorouter_dsp::PARAMETRIC_EQ_BANDS {
            parameters.extend([
                json!({ "name": format!("band{index}Enabled"), "type": "boolean", "default": false }),
                json!({ "name": format!("band{index}Type"), "type": "string", "enum": ["peaking", "lowShelf", "highShelf", "lowPass", "highPass", "bandPass", "allPass", "notch"], "default": "peaking" }),
                json!({ "name": format!("band{index}FrequencyHz"), "type": "number", "unit": "Hz", "minimum": 20.0, "maximum": 20000.0, "default": 1000.0 }),
                json!({ "name": format!("band{index}Q"), "type": "number", "minimum": 0.1, "maximum": 20.0, "default": 1.0 }),
                json!({ "name": format!("band{index}GainDb"), "type": "number", "unit": "dB", "minimum": -24.0, "maximum": 24.0, "default": 0.0 }),
            ]);
        }
        Value::Array(parameters)
    }

    pub(crate) fn processor_catalog() -> Value {
        let available = json!({ "status": "available" });
        json!([
            {
                "id": "graphicEq", "version": 1, "category": "equalizer",
                "availability": available, "latencySamples": 0,
                "parameters": (0..10).map(|index| json!({ "name": format!("band{index}Db"), "type": "number", "unit": "dB", "minimum": -18.0, "maximum": 18.0, "default": 0.0 })).collect::<Vec<_>>()
            },
            {
                "id": "parametricEq", "version": 1, "category": "equalizer",
                "availability": available, "latencySamples": 0,
                "parameters": Self::parametric_eq_parameters()
            },
            {
                "id": "gate", "version": 1, "category": "dynamics",
                "availability": available, "latencySamples": 0,
                "parameters": [
                    { "name": "thresholdDb", "type": "number", "unit": "dBFS", "minimum": -80.0, "maximum": 0.0, "default": -45.0 },
                    { "name": "rangeDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 80.0, "default": 60.0 },
                    { "name": "hysteresisDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 12.0, "default": 3.0 },
                    { "name": "ratio", "type": "number", "minimum": 1.0, "maximum": 20.0, "default": 4.0 },
                    { "name": "attackMs", "type": "number", "unit": "ms", "minimum": 0.1, "maximum": 100.0, "default": 5.0 },
                    { "name": "holdMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 1000.0, "default": 50.0 },
                    { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 2000.0, "default": 150.0 }
                ]
            },
            {
                "id": "compressor", "version": 1, "category": "dynamics",
                "availability": available, "latencySamples": 0,
                "parameters": [
                    { "name": "thresholdDb", "type": "number", "unit": "dBFS", "minimum": -60.0, "maximum": 0.0, "default": -18.0 },
                    { "name": "ratio", "type": "number", "minimum": 1.0, "maximum": 20.0, "default": 3.0 },
                    { "name": "attackMs", "type": "number", "unit": "ms", "minimum": 0.1, "maximum": 200.0, "default": 10.0 },
                    { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 2000.0, "default": 150.0 },
                    { "name": "kneeDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 24.0, "default": 6.0 },
                    { "name": "makeupDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 24.0, "default": 0.0 }
                ]
            },
            {
                "id": "limiter", "version": 1, "category": "dynamics",
                "availability": available, "latencySamples": 240,
                "parameters": [
                    { "name": "ceilingDb", "type": "number", "unit": "dBFS", "minimum": -12.0, "maximum": 0.0, "default": -1.0 },
                    { "name": "lookaheadMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 10.0, "default": 5.0 },
                    { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 1000.0, "default": 100.0 }
                ]
            },
            {
                "id": "delay", "version": 1, "category": "time",
                "availability": available, "latencySamples": 0,
                "parameters": [{ "name": "delayMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 1000.0, "default": 0.0 }]
            },
            {
                "id": "pitch", "version": 1, "category": "pitch",
                "availability": available, "latencySamples": 1024,
                "parameters": [
                    { "name": "semitones", "type": "number", "unit": "semitones", "minimum": -12.0, "maximum": 12.0, "default": 0.0 },
                    { "name": "cents", "type": "number", "unit": "cents", "minimum": -100.0, "maximum": 100.0, "default": 0.0 }
                ]
            },
            Self::catalog_entry("volume", audiorouter_domain::NodeKind::Volume, "level"),
            Self::catalog_entry("bassTreble", audiorouter_domain::NodeKind::BassTreble, "equalizer"),
            Self::catalog_entry("dehum", audiorouter_domain::NodeKind::Dehum, "restoration"),
            Self::catalog_entry("declick", audiorouter_domain::NodeKind::Declick, "restoration"),
            Self::catalog_entry("denoise", audiorouter_domain::NodeKind::Denoise, "restoration"),
            Self::catalog_entry("speechDenoise", audiorouter_domain::NodeKind::SpeechDenoise, "restoration"),
            Self::catalog_entry("spectralGate", audiorouter_domain::NodeKind::SpectralGate, "restoration"),
            Self::catalog_entry("firFilter", audiorouter_domain::NodeKind::FirFilter, "convolution"),
            Self::catalog_entry("timeShift", audiorouter_domain::NodeKind::TimeShift, "time"),
            Self::catalog_entry("inputSwitch", audiorouter_domain::NodeKind::InputSwitch, "routing"),
            Self::catalog_entry("duck", audiorouter_domain::NodeKind::Duck, "dynamics")
        ])
    }

    /// Catalog entry for a processor whose parameters and latency come from
    /// the node registry, so discovery and the catalog cannot disagree.
    pub(crate) fn catalog_entry(
        id: &str,
        kind: audiorouter_domain::NodeKind,
        category: &str,
    ) -> Value {
        let latency = audiorouter_domain::node_registry()
            .iter()
            .find(|spec| spec.kind == kind)
            .map_or(0, |spec| spec.latency_samples);
        json!({
            "id": id, "version": 1, "category": category,
            "availability": { "status": "available" }, "latencySamples": latency,
            "parameters": Self::node_parameter_schema(kind)
        })
    }

    pub(crate) fn dispatch_apps_list(&mut self) -> Result<Value, ControlError> {
        if let Some((captured_at, snapshot)) = &self.application_snapshot {
            if captured_at.elapsed() < APPLICATION_SNAPSHOT_TTL {
                return Ok(snapshot.clone());
            }
        }
        let applications =
            audiorouter_windows_audio::enumerate_applications().map_err(audio_control_error)?;
        let audio = audiorouter_windows_audio::enumerate_application_audio()
            .map_err(audio_control_error)?;
        let snapshot = json!(applications
            .into_iter()
            .map(|application| {
                let session = audio.iter().find(|item| item.process_id == application.process_id);
                json!({
                    "processId": application.process_id,
                    "executable": application.executable,
                    "executablePath": application.executable_path,
                    "creationTime100ns": application.creation_time_100ns.map(|value| value.to_string()),
                    "audioActivity": session.map_or("none", |item| if item.active_session_count > 0 { "active" } else { "inactive" }),
                    "captureCapability": session.map_or("notObserved", |item| if item.capture_session_count > 0 { "observed" } else { "notObserved" }),
                    "audioSessionCount": session.map_or(0, |item| item.total_session_count),
                    "activeAudioSessionCount": session.map_or(0, |item| item.active_session_count),
                    "captureSessionCount": session.map_or(0, |item| item.capture_session_count),
                    "renderSessionCount": session.map_or(0, |item| item.render_session_count),
                    "audioDisplayNames": session.map_or_else(Vec::new, |item| item.display_names.clone()),
                })
            })
            .collect::<Vec<_>>());
        self.application_snapshot = Some((Instant::now(), snapshot.clone()));
        Ok(snapshot)
    }

    pub(crate) fn dispatch_processors_response(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("response parameters are required".into())
        })?;
        let sample_rate = params["sampleRateHz"]
            .as_f64()
            .ok_or_else(|| ControlError::InvalidRequest("sampleRateHz is required".into()))?
            as f32;
        let frequencies = params["frequenciesHz"]
            .as_array()
            .ok_or_else(|| ControlError::InvalidRequest("frequenciesHz is required".into()))?;
        if frequencies.is_empty() || frequencies.len() > MAX_RESPONSE_FREQUENCIES {
            return Err(ControlError::InvalidRequest(
                "frequenciesHz count is outside the bounded response limit".into(),
            ));
        }
        let mut bands = [None; audiorouter_dsp::PARAMETRIC_EQ_BANDS];
        let band_values = params["bands"]
            .as_array()
            .ok_or_else(|| ControlError::InvalidRequest("bands is required".into()))?;
        if band_values.len() > MAX_RESPONSE_BANDS {
            return Err(ControlError::InvalidRequest(
                "too many response bands".into(),
            ));
        }
        for (index, band) in band_values.iter().enumerate() {
            if band.get("enabled").and_then(Value::as_bool) == Some(false) {
                continue;
            }
            let kind = match band["type"].as_str() {
                Some("peaking") => audiorouter_dsp::FilterKind::Peaking,
                Some("lowShelf") => audiorouter_dsp::FilterKind::LowShelf,
                Some("highShelf") => audiorouter_dsp::FilterKind::HighShelf,
                Some("lowPass") => audiorouter_dsp::FilterKind::LowPass,
                Some("highPass") => audiorouter_dsp::FilterKind::HighPass,
                Some("bandPass") => audiorouter_dsp::FilterKind::BandPass,
                Some("allPass") => audiorouter_dsp::FilterKind::AllPass,
                Some("notch") => audiorouter_dsp::FilterKind::Notch,
                _ => {
                    return Err(ControlError::InvalidRequest(format!(
                        "invalid response band {index} type"
                    )))
                }
            };
            let number = |name: &str| {
                band[name]
                    .as_f64()
                    .map(|value| value as f32)
                    .ok_or_else(|| {
                        ControlError::InvalidRequest(format!(
                            "response band {index} {name} is required"
                        ))
                    })
            };
            bands[index] = Some(audiorouter_dsp::BiquadParams {
                kind,
                frequency_hz: number("frequencyHz")?,
                q: number("q")?,
                gain_db: number("gainDb")?,
                sample_rate,
            });
        }
        let eq = audiorouter_dsp::ParametricEq::new(bands, 1).map_err(|error| {
            ControlError::InvalidRequest(format!("invalid response EQ: {error:?}"))
        })?;
        let frequencies = frequencies
            .iter()
            .map(|value| {
                value.as_f64().map(|value| value as f32).ok_or_else(|| {
                    ControlError::InvalidRequest("frequenciesHz must contain numbers".into())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let magnitude_db = frequencies
            .iter()
            .map(|frequency| {
                eq.magnitude_db_at(*frequency).map_err(|error| {
                    ControlError::InvalidRequest(format!("invalid response frequency: {error:?}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({ "frequenciesHz": frequencies, "magnitudeDb": magnitude_db }))
    }
}
