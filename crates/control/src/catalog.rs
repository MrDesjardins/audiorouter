//! Discovery: describe, node parameter schemas, the processor catalog and processor responses.

use super::*;

/// Live pitch reading of a playing Fun Voice Changer for its tuner strip
/// (DSP-20). `None` when the node has no running processor.
pub(crate) fn voice_changer_telemetry(
    state: Option<std::sync::Arc<audiorouter_engine::VoiceChangerState>>,
) -> Option<Value> {
    let reading = state?.telemetry()?;
    let round = |value: f32| (f64::from(value) * 10.0).round() / 10.0;
    Some(json!({
        "voiced": reading.voiced,
        "f0Hz": reading.voiced.then(|| round(reading.f0_hz)),
        "note": reading.voiced.then_some(reading.note),
        "cents": reading.voiced.then(|| round(reading.cents)),
    }))
}

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
            "presets": { "voiceChains": voice_chains, "eq": eq, "voiceChanger": Self::voice_changer_presets() },
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
            // Plugin settings are per plugin (`plugins.parameters`); this is
            // AudioRouter's own guard. Its unset value is in `description`, as
            // `nodes.add` saves every `default`.
            audiorouter_domain::NodeKind::Plugin => json!([
                { "name": "passThroughAllowed", "type": "boolean", "description": "Allow this plugin to pass a microphone through unchanged. When off, a plugin fed by a recording device that returns its audio untouched for about 2 seconds (as an unlicensed plugin does) is silenced until plugins.resume. Turn on only for a plugin meant to be neutral. false when unset." }
            ]),
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
                { "name": "harmonics", "type": "number", "step": 1.0, "minimum": 1.0, "maximum": 8.0, "default": 4 }
            ]),
            audiorouter_domain::NodeKind::Denoise => json!([
                { "name": "reductionPercent", "type": "number", "unit": "%", "minimum": 0.0, "maximum": 100.0, "default": 70.0 },
                { "name": "floorPercent", "type": "number", "unit": "%", "minimum": 0.0, "maximum": 100.0, "default": 10.0 },
                { "name": "learning", "type": "boolean", "default": false }
            ]),
            audiorouter_domain::NodeKind::TimeShift => json!([
                { "name": "bufferSeconds", "type": "number", "step": 1.0, "unit": "s", "minimum": 10.0, "maximum": 120.0, "default": 60 }
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
            audiorouter_domain::NodeKind::FunVoiceChanger => Self::voice_changer_parameters(),
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
                { "name": "duckBetweenRounds", "type": "boolean", "default": true },
                { "name": "duckAction", "type": "boolean", "default": false }
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
            audiorouter_domain::NodeKind::OutputSwitch => json!([
                { "name": "selected", "type": "string", "enum": ["a", "b"], "default": "a" }
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
            // Device and source kinds carry no `default`: `nodes.add` saves
            // every default, and these nodes are added with no parameters.
            // The backend's value for an unset setting is in `description`.
            audiorouter_domain::NodeKind::PhysicalInput => json!([
                Self::endpoint_id_parameter(),
                { "name": "channelMode", "type": "string", "enum": ["stereo", "mono", "left", "right"], "description": "How a stereo device is sent: both channels, both mixed to mono, or the left or right input only. stereo when unset." },
                { "name": "spatialMode", "type": "string", "enum": ["off", "headphones", "speakers"], "description": "Render a 5.1/7.1 capture device to two ears for headphones or two speakers. off when unset." },
                { "name": "spatialRoomPercent", "type": "number", "unit": "%", "minimum": 0.0, "maximum": 100.0, "description": "Room blend around the virtual speakers. 0 when unset." }
            ]),
            audiorouter_domain::NodeKind::PhysicalOutput => {
                json!([Self::endpoint_id_parameter()])
            }
            audiorouter_domain::NodeKind::ApplicationCapture => json!([
                { "name": "executable", "type": "string", "maxLength": audiorouter_domain::MAX_APPLICATION_EXECUTABLE_BYTES, "description": "Required. Executable name of the application, from apps.list (for example Discord.exe)." },
                { "name": "executablePath", "type": "string", "maxLength": audiorouter_domain::MAX_APPLICATION_PATH_BYTES, "description": "Full executable path, from apps.list." },
                { "name": "processPolicy", "type": "string", "enum": ["selectedInstance", "allVerifiedInstances"], "description": "Required. selectedInstance captures one process and then needs processId and creationTime100ns; allVerifiedInstances captures every verified instance and must not have them." },
                { "name": "processId", "type": "number", "step": 1.0, "minimum": 1.0, "maximum": f64::from(u32::MAX), "description": "Process ID from apps.list; only with processPolicy selectedInstance." },
                { "name": "creationTime100ns", "type": "string", "description": "Opaque process start time from apps.list (a decimal string); only with processPolicy selectedInstance." }
            ]),
            audiorouter_domain::NodeKind::EndpointLoopback => json!([
                { "name": "endpointId", "type": "string", "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Render device ID from devices.list whose output is captured. Give exactly one of endpointId and defaultRole." },
                { "name": "defaultRole", "type": "string", "enum": ["console", "multimedia", "communications"], "description": "Follow the Windows default output of this role instead of a fixed device. Give exactly one of endpointId and defaultRole." }
            ]),
            audiorouter_domain::NodeKind::VirtualRenderSource
            | audiorouter_domain::NodeKind::VirtualCaptureSink => json!([
                { "name": "busId", "type": "string", "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Required. ID of the virtual bus (see virtualDevices.list)." }
            ]),
            // A Meter only measures; the validator accepts no setting.
            audiorouter_domain::NodeKind::Meter => json!([]),
            audiorouter_domain::NodeKind::AudioFile => json!([
                { "name": "mediaId", "type": "string", "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Opaque media ID returned by audioMedia.finishUpload." },
                { "name": "fileName", "type": "string", "maxLength": 255, "description": "File name shown for the media." },
                { "name": "loop", "type": "boolean", "description": "Repeat the file. false when unset." }
            ]),
            audiorouter_domain::NodeKind::NetworkSend => json!([
                { "name": "host", "type": "string", "description": "IPv4 or IPv6 address of the receiving computer (names are not resolved)." },
                Self::network_port_parameter(),
                Self::network_pairing_key_parameter()
            ]),
            audiorouter_domain::NodeKind::NetworkReceive => json!([
                { "name": "sender", "type": "string", "description": "IPv4 or IPv6 address of the sending computer (names are not resolved)." },
                Self::network_port_parameter(),
                Self::network_pairing_key_parameter(),
                { "name": "bufferMs", "type": "number", "unit": "ms", "minimum": audiorouter_domain::MIN_NETWORK_BUFFER_MS, "maximum": audiorouter_domain::MAX_NETWORK_BUFFER_MS, "description": format!("Jitter buffer. {} ms when unset.", audiorouter_domain::DEFAULT_NETWORK_BUFFER_MS) }
            ]),
            audiorouter_domain::NodeKind::Soundboard => Self::soundboard_parameters(),
        }
    }

    /// The exact Windows endpoint a device node binds to (CAP-02).
    fn endpoint_id_parameter() -> Value {
        json!({ "name": "endpointId", "type": "string", "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Device ID from devices.list." })
    }

    fn network_port_parameter() -> Value {
        json!({ "name": "port", "type": "number", "step": 1.0, "minimum": 1.0, "maximum": f64::from(u16::MAX), "description": "UDP port; the same on both computers." })
    }

    fn network_pairing_key_parameter() -> Value {
        json!({
            "name": "pairingKey", "type": "string", "maxLength": audiorouter_domain::MAX_NETWORK_PAIRING_KEY_CHARS,
            "description": format!(
                "Shared key typed identically on both computers: blank (not paired) or {}-{} printable ASCII characters without leading or trailing spaces.",
                audiorouter_domain::MIN_NETWORK_PAIRING_KEY_CHARS,
                audiorouter_domain::MAX_NETWORK_PAIRING_KEY_CHARS
            )
        })
    }

    /// Soundboard slot settings (SND-01): one family per field, `slot<N>`
    /// with N from 1 to `SOUNDBOARD_SLOTS`. Families carry no `default` so
    /// `nodes.add` never saves a literal `slot<N>...` key.
    pub(crate) fn soundboard_parameters() -> Value {
        let slots = audiorouter_domain::SOUNDBOARD_SLOTS;
        let family = |field: &str, mut entry: Value| {
            entry["name"] = json!(format!("slot<N>{field}"));
            entry["namePattern"] = json!(format!("slot<N>{field}"));
            entry["slots"] = json!({ "minimum": 1, "maximum": slots });
            entry
        };
        json!([
            family(
                "MediaId",
                json!({ "type": "string", "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": format!("Sound of slot N (1 to {slots}): a sound from soundLibrary.list ({}<id>) or an uploaded media ID.", audiorouter_domain::SOUND_LIBRARY_PREFIX) })
            ),
            family(
                "FileName",
                json!({ "type": "string", "maxLength": 255, "description": "File name shown for the slot's sound." })
            ),
            family(
                "Caption",
                json!({ "type": "string", "maxLength": audiorouter_domain::MAX_SOUNDBOARD_CAPTION_BYTES, "description": "Button caption; empty uses the file name." })
            ),
            family(
                "GainDb",
                json!({ "type": "number", "unit": "dB", "minimum": -24.0, "maximum": 6.0, "description": "Level of the slot's sound. 0 dB when unset." })
            ),
            family(
                "Mode",
                json!({ "type": "string", "enum": ["once", "loop", "toggle", "hold"], "description": "once plays to the end, loop repeats, toggle starts and stops, hold plays while held. once when unset." })
            ),
            family(
                "Set",
                json!({ "type": "string", "maxItems": audiorouter_domain::MAX_SOUNDBOARD_SET_SOUNDS, "description": format!("Random pick on each press: a category ID from soundLibrary.list, or an array of up to {} sound references ({}<id> or media IDs).", audiorouter_domain::MAX_SOUNDBOARD_SET_SOUNDS, audiorouter_domain::SOUND_LIBRARY_PREFIX) })
            )
        ])
    }

    /// Fun Voice Changer schema from the domain table (DSP-20), so discovery,
    /// validation and the engine cannot disagree.
    pub(crate) fn voice_changer_parameters() -> Value {
        use audiorouter_domain::voice_changer::{
            VoiceParameterKind, VOICE_CHANGER_PARAMETERS, VOICE_PRESETS,
        };
        let mut parameters = vec![json!({
            "name": "preset",
            "type": "string",
            "enum": VOICE_PRESETS.iter().map(|preset| preset.id).collect::<Vec<_>>(),
            "default": "custom",
        })];
        for spec in &VOICE_CHANGER_PARAMETERS {
            let default = spec.default.to_json();
            parameters.push(match spec.kind {
                VoiceParameterKind::Number {
                    minimum,
                    maximum,
                    integer,
                    unit,
                } => {
                    let mut entry = json!({
                        "name": spec.name, "type": "number",
                        "minimum": minimum, "maximum": maximum, "default": default,
                    });
                    if !unit.is_empty() {
                        entry["unit"] = json!(unit);
                    }
                    if integer {
                        entry["step"] = json!(1.0);
                    }
                    entry
                }
                VoiceParameterKind::Boolean => {
                    json!({ "name": spec.name, "type": "boolean", "default": default })
                }
                VoiceParameterKind::Choice(choices) => json!({
                    "name": spec.name, "type": "string", "enum": choices, "default": default,
                }),
            });
        }
        Value::Array(parameters)
    }

    /// Fun Voice Changer presets with every value they write, so a client
    /// applies the backend's recipe instead of keeping its own copy.
    pub(crate) fn voice_changer_presets() -> Value {
        use audiorouter_domain::voice_changer::{voice_preset_parameters, VOICE_PRESETS};
        VOICE_PRESETS
            .iter()
            .map(|preset| {
                json!({
                    "id": preset.id,
                    "version": preset.version,
                    "name": preset.name,
                    "description": preset.description,
                    "basicParameters": preset.basic,
                    "parameters": voice_preset_parameters(preset),
                })
            })
            .collect()
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
            Self::catalog_entry("outputSwitch", audiorouter_domain::NodeKind::OutputSwitch, "routing"),
            Self::catalog_entry("duck", audiorouter_domain::NodeKind::Duck, "dynamics"),
            Self::catalog_entry("funVoiceChanger", audiorouter_domain::NodeKind::FunVoiceChanger, "effect")
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
        if let Some(inventory) = &self.application_inventory {
            return inventory.list();
        }
        if let Some((captured_at, snapshot)) = &self.application_snapshot {
            if captured_at.elapsed() < APPLICATION_SNAPSHOT_TTL {
                return Ok(snapshot.clone());
            }
        }
        let snapshot = Self::build_application_inventory()?;
        self.application_snapshot = Some((Instant::now(), snapshot.clone()));
        Ok(snapshot)
    }

    /// Enumerate processes and their audio sessions (30-100 ms on a real
    /// PC). Runs on the inventory thread once the backend service starts.
    pub(crate) fn build_application_inventory() -> Result<Value, ControlError> {
        let applications =
            audiorouter_windows_audio::enumerate_applications().map_err(audio_control_error)?;
        let audio = audiorouter_windows_audio::enumerate_application_audio()
            .map_err(audio_control_error)?;
        Ok(json!(applications
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
                    "audioEndpoints": session.map_or_else(Vec::new, |item| item.endpoints.iter().map(|endpoint| json!({
                        "deviceId": endpoint.endpoint_id,
                        "direction": match endpoint.direction {
                            audiorouter_windows_audio::EndpointDirection::Capture => "capture",
                            audiorouter_windows_audio::EndpointDirection::Render => "render",
                        },
                        "active": endpoint.active,
                    })).collect()),
                })
            })
            .collect::<Vec<_>>()))
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

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
