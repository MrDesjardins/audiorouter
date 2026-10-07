//! API discovery: method descriptions and input schemas.

use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodDescription {
    pub name: &'static str,
    pub description: &'static str,
    pub permission: audiorouter_domain::PermissionScope,
    pub side_effect: audiorouter_domain::SideEffectClass,
    pub input_schema: Value,
    pub output_schema: Value,
}

impl From<ApiMethodSpec> for MethodDescription {
    fn from(spec: ApiMethodSpec) -> Self {
        Self {
            name: spec.name,
            description: method_description(spec.name),
            permission: spec.permission,
            side_effect: spec.side_effect,
            input_schema: method_input_schema(spec.name),
            output_schema: method_output_schema(spec.name),
        }
    }
}

pub(crate) fn method_description(name: &str) -> &'static str {
    match name {
        "system.describe" => "Describe protocol capabilities, methods, node types, and limits.",
        "system.handshake" => "Negotiate a compatible protocol version before requests.",
        "status.get" => "Return backend, runtime, and audio availability status.",
        "system.osTransition" => {
            "Apply a lock, sign-out, sleep, or resume lifecycle transition policy."
        }
        "system.diagnostics" => "Return a redacted backend diagnostic snapshot.",
        "diagnostics.getVerbose" => "Report whether opt-in verbose logging is on and how long it has left.",
        "diagnostics.setVerbose" => "Switch opt-in verbose logging on for one hour (routine reads and request durations, never parameters, paths or audio), or off.",
        "system.quit" => "Finalize active recorders and stop all running sessions before the owner exits.",
        "clients.list" => "List enrolled local client identities and roles.",
        "clients.authorize" => "Authorize a client with an explicit built-in role.",
        "clients.revoke" => "Revoke a client enrollment without deleting its audit record.",
        "operations.get" => "Read the durable outcome of an idempotent operation.",
        "operations.cancel" => "Cancel a pending operation when it has not completed.",
        "recordings.list" => "List persisted recording metadata without touching audio files.",
        "audioMedia.beginUpload" => "Begin a bounded WAV/MP3 upload that will be decoded off the audio callback and stored under an opaque backend media ID.",
        "audioMedia.uploadChunk" => "Append the next ordered bounded chunk of a selected WAV/MP3 file.",
        "audioMedia.finishUpload" => "Validate, decode, and persist a complete WAV/MP3 audio source.",
        "audioMedia.importTemporaryRecording" => "Convert a completed temporary-take WAV into expiring graph media, then remove its temporary recording file and library entry.",
        "audioMedia.delete" => "Delete imported audio media that is not referenced by any session graph.",
        "timeShift.transport" => "Pause, resume, jump back or forward 10 s, or return to live on one running Time Shift node; status reads its buffer.",
        "meters.reset" => "Reset held sample peaks and clipping statistics of one prepared Meter without changing audio or the saved graph.",
        "audioSources.transport" => "Play or stop one prepared Test Signal or audio-file source without stopping other graph routes; pause applies to audio files.",
        "recorders.list" => "List live in-memory recorder states and frame boundaries.",
                "recorders.create" => "Create and attach an unarmed file recorder under the approved root. Omitted dither defaults to TPDF for integer output and is disabled for Float32 and MP3.",
        "recorders.arm" => "Arm a session recorder without opening an audio device.",
        "recorders.start" => "Start a recorder at an explicit engine frame boundary.",
        "recorders.pause" => "Pause a recorder at an explicit engine frame boundary.",
        "recorders.resume" => "Resume a recorder at an explicit engine frame boundary.",
        "recorders.split" => "Split a recorder at an explicit engine frame boundary.",
        "recorders.startRecording" => "Start recording a Recorder node now with its own settings (format, automatic split). Works while the route plays; returns the new file path.",
        "recorders.stopRecording" => "Stop a Recorder node's recording and finalize its file. Does nothing when it is not recording.",
        "recorders.stop" => "Stop a recorder at an explicit engine frame boundary.",
        "devices.getAccess" => "Whether the user allowed the desktop app to open audio devices when Play is pressed.",
        "devices.setAccess" => "Allow or withdraw the desktop app's permission to open audio devices on Play. Desktop window only.",
        "recordings.getRoot" => "Read the approved recording folder (null until the user chooses one) and a suggested folder.",
        "recordings.setRoot" => "Approve a local folder for every recording; create it when create is true. Desktop app only.",
        "recordings.get" => {
            "Read one persisted recording metadata resource without touching its file."
        }
        "recordings.recovery" => {
            "Read a validated recorder recovery checkpoint without touching audio files."
        }
        "recordings.reveal" => "Reveal a recorded file in the operating system file browser.",
        "recordings.preview" => "Inspect recording file format metadata without decoding audio.",
        "recordings.setMetadata" => {
            "Update recording metadata without changing audio content or path."
        }
        "recordings.rename" => "Rename a recording within its approved directory.",
        "safety.setPrivacyMute" => "Latch or clear process-local privacy mute for capture paths.",
        "recovery.clearSafeMode" => {
            "Clear the latched crash-recovery safe mode after an operator confirms stability."
        }
        "startup.get" => "Report the desired sign-in startup policy and registration capability.",
        "startup.plan" => "Preview a sign-in startup policy change without applying OS registration.",
        "startup.apply" => "Apply a validated sign-in startup policy when native registration is available.",
        "recordings.removeEntry" => "Remove a recording library entry without deleting its file.",
        "recordings.recycle" => {
            "Move a recording to the operating system Recycle Bin after explicit confirmation."
        }
        "devices.list" => "List authoritative audio endpoint descriptors.",
        "nativeEndpoints.prepare" => {
            "Prepare exact capture/render clients without starting audio."
        }
        "nativeOutputs.prepare" => {
            "Prepare multiple exact physical render clients as independent bounded output branches without starting audio."
        }
        "nativeMultiInputs.prepare" => {
            "Prepare multiple exact physical and/or application-capture clients for a validated mixer/fan-out graph, including pre-bound plugin stages, without starting audio."
        }
        "nativePaths.prepare" => {
            "Prepare every independent path of a saved session from the exact devices and applications stored on its nodes, without starting audio."
        }
        "nativeBridges.prepare" => {
            "Prepare one exact project-driver render-source and capture-sink lease pair without starting audio."
        }
        "nativeBridges.detach" => {
            "Detach one stopped project-driver bridge lease pair for an exact virtual bus."
        }
        "nativeBridges.heartbeat" => {
            "Refresh all prepared project-driver bridge leases on the control thread."
        }
        "nativeEndpoints.rebind" => {
            "Rebind an attached stopped native worker to refreshed exact endpoints without starting audio."
        }
        "nativeEndpoints.detach" => {
            "Detach a stopped native endpoint worker so exact bindings can be replaced."
        }
        "nativeDuplex.detach" => {
            "Detach a stopped native duplex bridge so its exact driver binding can be replaced."
        }
        "nativeApplications.prepare" => {
            "Prepare a verified application process-loopback capture and exact render client without starting audio."
        }
        "nativeEndpoints.pump" => {
            "Drain a bounded amount of already-available native audio for a running session."
        }
        "nativeDuplex.pump" => {
            "Drain bounded already-available audio from both directions of a running duplex session."
        }
        "nativeRenderSources.pump" => {
            "Drain bounded already-available audio from a running virtual render-source worker."
        }
        "nativeMultiInputs.pump" => {
            "Drain bounded already-available audio from several capture workers into their prepared fan-out graph."
        }
        "nativeMultiInputs.bindBranches" => {
            "Bind ordered validated fan-out destination nodes to branch-local virtual, recording, and tool observers."
        }
        "plugins.scan" => "Inspect an explicitly selected plugin directory without loading plugin code.",
        "plugins.list" => "List the last bounded plugin scan inventory without scanning or loading plugin code.",
        "plugins.inventory" => "List every remembered plugin scan (one per scanned folder, persisted across restarts) without scanning or loading plugin code.",
        "plugins.retry" => "Explicitly refresh a plugin inventory after a prior scan failure or quarantine decision.",
        "plugins.inspect" => "Inspect one explicitly selected plugin binary without loading plugin code.",
        "plugins.saveState" => "Capture the opaque state of a plugin node in a playing route and store it; set the returned stateId on the node to restore it on the next start.",
        "plugins.openEditor" => "Open the native editor of a plugin node in a playing route inside a parent window owned by the calling desktop shell.",
        "plugins.closeEditor" => "Close a plugin node editor. The editor shows the instance that processes audio, so edits already apply live.",
        "plugins.parameters" => "Load one currently scanned plugin in its isolated worker and return bounded parameter descriptors.",
        "virtualDevices.list" => "List managed virtual bus desired state without activating endpoints.",
        "virtualDevices.plan" => "Validate a managed virtual bus lifecycle change without applying it.",
        "virtualDevices.apply" => "Apply a validated virtual bus lifecycle plan to desired state.",
        "virtualDevices.provision" => "Provision one explicitly selected managed virtual bus device.",
        "virtualDevices.remove" => "Remove one explicitly selected managed virtual bus device.",
        "virtualRoutes.list" => "List explicit cross-session virtual-bus routes without activating audio.",
        "virtualRoutes.replace" => "Atomically replace explicit cross-session routes using a revision and idempotency key.",
        "apps.list" | "applications.list" => {
            "List discoverable application identities and observed Windows audio-session activity for binding."
        }
        "nodes.types" => "List supported node types and their availability.",
        "routes.inspect" => "Inspect upstream route provenance for a destination node.",
        "graph.history" => "List bounded committed graph revisions for a session.",
        "graph.undoPlan" => "Prepare an inverse graph plan from retained history.",
        "events.subscribe" => "Replay retained state events from an optional cursor.",
        "nodes.describe" => "Describe node types, availability, and realtime cost.",
        "presets.list" => "List explainable built-in processing presets.",
        "processors.list" => "List built-in DSP processor metadata and availability.",
        "processors.response" => "Evaluate a bounded parametric-EQ magnitude response using the DSP coefficient path.",
        "sessions.get" => "Return one session resource by opaque identifier.",
        "sessions.export" => "Export one persisted canonical session document without changing state.",
        "sessions.exportFile" => "Write the saved session, with its imported audio and plugin states, to a new .audiorouter file.",
        "sessions.importFile" => "Import a .audiorouter session file as a new stopped session.",
        "sessions.importPlan" => "Validate a stopped session import without persisting it.",
        "sessions.importCommit" => "Commit a previously validated stopped session import.",
        "sessions.list" => "List session resources with stable cursor pagination.",
        "sessions.active.get" => "Return the currently selected editing session, without starting audio.",
        "sessions.active.set" => "Select an existing editing session and notify connected desktop clients; this does not start audio.",
        "sessions.create" => "Create a validated stopped session resource.",
        "sessions.duplicate" => "Clone a session into a new stopped resource.",
        "sessions.delete" => "Delete a stopped session resource and its history.",
        "graph.plan" => "Validate and preview a graph candidate without mutation.",
        "graph.commit" => "Commit an unexpired graph plan with idempotent mutation.",
        "session.start" | "sessions.start" => {
            "Start an already-prepared committed route. For a saved multi-path audio session, call nativePaths.prepare (HTTP POST /api/v1/nativePaths/prepare) with sessionId first; preparation requires deviceAdministration and does not start playback. Desktop Play performs preparation first. Alternatively temporarily preview a validated candidate graph without saving a new session revision; candidate preview requires both sessionControl and graphWrite grants plus a prepared single-endpoint native route."
        }
        "session.stop" | "sessions.stop" => {
            "Stop a session runtime and publish its lifecycle result."
        }
        "sessions.play" => "Play a saved session (default: the active session): prepares every device chosen on its nodes and starts, like the Play button. Needs the device administration grant.",
        "sessions.togglePlay" => "Play the session if it is stopped, stop it if it is playing (one StreamDeck key). Needs the device administration grant.",
        "sessions.summary" => "Describe a session in plain terms: nodes with readable settings, connections by name, playing and privacy-mute state.",
        "safety.togglePrivacyMute" => "Mute the microphone if it is live, unmute it if it is muted. Returns the new state.",
        "nodes.catalog" => "List the tools and devices that can be added by kind, with a plain description, inputs, outputs and settings.",
        "nodes.set" => "Change one node, found by ID or name: settings (parameters), enabled, bypass or name. Applied live and saved.",
        "nodes.toggle" => "Flip one node's enabled, bypass, an on/off setting or a two-choice setting (for example an Input Switch between A and B). Applied live and saved.",
        "nodes.add" => "Add a tool or device by kind (see nodes.catalog), optionally between two connected nodes or after a node. Saved.",
        "nodes.remove" => "Remove a node by ID or name; by default its upstream and downstream nodes are reconnected. Saved.",
        "connections.add" => "Connect two nodes by ID or name (first output to the first free input). Saved.",
        "connections.remove" => "Remove the connection between two nodes. Saved.",
        "meters.levels" => "Current level of each playing node (peak and RMS dBFS, clipping, and reduction for Gate, Compressor, Limiter and Duck).",
        _ => "Invoke an AudioRouter control-plane method.",
    }
}

/// Optional continuity statistics of the backend audio service, attached to
/// native pump results (see `AudioServiceStats`).
pub(crate) fn audio_service_output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "active": { "type": "boolean" },
            "passes": { "type": "integer", "minimum": 0 },
            "lateGaps": { "type": "integer", "minimum": 0 },
            "maxGapMicros": { "type": "integer", "minimum": 0 }
        },
        "required": ["active", "passes", "lateGaps", "maxGapMicros"],
        "additionalProperties": false
    })
}

pub(crate) fn object_schema(properties: Value, required: &[&str]) -> Value {
    let required = required
        .iter()
        .map(|value| Value::String((*value).into()))
        .collect::<Vec<_>>();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

pub(crate) fn method_input_schema(name: &str) -> Value {
    match name {
        "diagnostics.setVerbose" => {
            object_schema(json!({ "enabled": { "type": "boolean" } }), &["enabled"])
        }
        "system.quit" => object_schema(
            json!({
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["idempotencyKey"],
        ),
        "system.handshake" => object_schema(
            json!({
                "protocolVersion": {
                    "type": "object",
                    "properties": {
                        "major": { "type": "integer", "minimum": 0 },
                        "minor": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["major", "minor"],
                    "additionalProperties": false
                }
            }),
            &["protocolVersion"],
        ),
        "system.osTransition" => object_schema(
            json!({
                "transition": { "enum": ["lock", "signOut", "sleep", "resume"] },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["transition", "idempotencyKey"],
        ),
        "clients.authorize" => object_schema(
            json!({
                "clientId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "role": { "enum": ["observer", "editor", "operator"] },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["clientId", "role", "idempotencyKey"],
        ),
        "clients.revoke" => object_schema(
            json!({
                "clientId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["clientId", "idempotencyKey"],
        ),
        "operations.get" => object_schema(
            json!({ "operationId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["operationId"],
        ),
        "operations.cancel" => object_schema(
            json!({
                "operationId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["operationId", "idempotencyKey"],
        ),
        "recordings.list" => object_schema(
            json!({
                "sessionId": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "cursor": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_RECORDING_LIST_ITEMS }
            }),
            &[],
        ),
        "audioMedia.beginUpload" => object_schema(
            json!({
                "fileName": { "type": "string", "minLength": 1, "maxLength": 256 },
                "sizeBytes": { "type": "integer", "minimum": 1, "maximum": audiorouter_storage::MAX_AUDIO_MEDIA_BYTES }
            }),
            &["fileName", "sizeBytes"],
        ),
        "audioMedia.uploadChunk" => object_schema(
            json!({
                "uploadId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "chunkIndex": { "type": "integer", "minimum": 0, "maximum": 1024 },
                "dataBase64": { "type": "string", "minLength": 4, "maxLength": AUDIO_UPLOAD_CHUNK_BYTES * 4 / 3 + 8 }
            }),
            &["uploadId", "chunkIndex", "dataBase64"],
        ),
        "audioMedia.finishUpload" => object_schema(
            json!({
                "uploadId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
            }),
            &["uploadId"],
        ),
        "audioMedia.importTemporaryRecording" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES }
            }),
            &["recordingId"],
        ),
        "audioMedia.delete" => object_schema(
            json!({
                "mediaId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
            }),
            &["mediaId"],
        ),
        "timeShift.transport" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "action": { "enum": ["pause", "resume", "back", "forward", "live", "status"] }
            }),
            &["sessionId", "nodeId", "action"],
        ),
        "audioSources.transport" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "action": { "enum": ["play", "pause", "stop", "status"] }
            }),
            &["sessionId", "nodeId", "action"],
        ),
        "meters.reset" => object_schema(
            json!({"sessionId": {"type":"string", "minLength":1}, "nodeId":{"type":"string", "minLength":1}}),
            &["sessionId", "nodeId"],
        ),
        "recorders.list" => object_schema(json!({}), &[]),
        "recorders.create" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "recorderId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "format": { "enum": ["wavPcm16", "wavPcm24", "wavFloat32", "flac16", "flac24", "mp3"] },
                "sequence": { "type": "integer", "minimum": 0 },
                "channels": { "type": "integer", "enum": [1, 2] },
                "sampleRate": { "type": "integer", "enum": [44100, 48000] },
                "dither": { "type": "boolean", "description": "Optional; defaults to TPDF for integer WAV/FLAC and false for WAV Float32 or MP3." },
                "queueCapacity": { "type": "integer", "minimum": 1, "maximum": audiorouter_recording::MAX_RECORDING_QUEUE_CHUNKS },
                "maximumChunksPerPass": { "type": "integer", "minimum": 1 },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &[
                "sessionId",
                "nodeId",
                "recorderId",
                "format",
                "sequence",
                "channels",
                "sampleRate",
                "queueCapacity",
                "maximumChunksPerPass",
                "idempotencyKey",
            ],
        ),
        "recorders.arm" => recorder_input_schema(false),
        "recorders.start" | "recorders.pause" | "recorders.resume" | "recorders.split"
        | "recorders.stop" => recorder_input_schema(true),
        "devices.getAccess" => object_schema(json!({}), &[]),
        "devices.setAccess" => object_schema(
            json!({
                "allowed": { "type": "boolean" },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["allowed", "idempotencyKey"],
        ),
        "recordings.getRoot" => object_schema(json!({}), &[]),
        "recordings.setRoot" => object_schema(
            json!({
                "root": { "type": "string", "minLength": 1, "maxLength": 1024 },
                "create": { "type": "boolean" },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["root", "idempotencyKey"],
        ),
        "recorders.startRecording" | "recorders.stopRecording" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["nodeId", "idempotencyKey"],
        ),
        "devices.list" => object_schema(
            json!({
                "cursor": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_DEVICE_LIST_ITEMS },
                "includeInactive": { "type": "boolean" }
            }),
            &[],
        ),
        "nativeEndpoints.prepare" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "captureEndpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES },
                "renderEndpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES }
            }),
            &["sessionId", "captureEndpointId", "renderEndpointId"],
        ),
        "nativeOutputs.prepare" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "renderEndpointIds": { "type": "array", "minItems": 1, "maxItems": audiorouter_engine::MAX_AUDIO_TAPS, "items": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES } }
            }),
            &["sessionId", "renderEndpointIds"],
        ),
        "nativeMultiInputs.prepare" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "sources": { "type": "array", "minItems": 2, "maxItems": audiorouter_engine::MAX_MIXER_INPUTS, "items": { "type": "object", "oneOf": [
                    { "properties": { "kind": { "const": "physical" }, "endpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES } }, "required": ["kind", "endpointId"], "additionalProperties": false },
                    { "properties": { "kind": { "const": "generated" } }, "required": ["kind"], "additionalProperties": false },
                    { "properties": { "kind": { "const": "application" }, "processId": { "type": "integer", "minimum": 1 }, "executable": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES }, "executablePath": { "type": ["string", "null"] }, "creationTime100ns": { "type": "string", "minLength": 1, "maxLength": 20 }, "mode": { "enum": ["include", "exclude"] } }, "required": ["kind", "processId", "executable", "creationTime100ns", "mode"], "additionalProperties": false }
                ] } }
            }),
            &["sessionId", "sources"],
        ),
        "nativePaths.prepare" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 }
            }),
            &["sessionId"],
        ),
        "nativeBridges.prepare" => object_schema(
            json!({
                "busId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "devicePath": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES },
                "renderMappingPath": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES },
                "captureMappingPath": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES },
                "leaseMs": { "type": "integer", "minimum": 1, "maximum": audiorouter_protocol::MAX_AUDIO_BRIDGE_LEASE_MS }
            }),
            &[
                "busId",
                "generation",
                "devicePath",
                "renderMappingPath",
                "captureMappingPath",
            ],
        ),
        "nativeBridges.detach" => object_schema(
            json!({
                "busId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
            }),
            &["busId"],
        ),
        "nativeBridges.heartbeat" => object_schema(json!({}), &[]),
        "nativeEndpoints.rebind" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "captureEndpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES },
                "renderEndpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES }
            }),
            &["sessionId", "captureEndpointId", "renderEndpointId"],
        ),
        "nativeEndpoints.detach" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
            }),
            &["sessionId"],
        ),
        "nativeDuplex.detach" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
            }),
            &["sessionId"],
        ),
        "nativeApplications.prepare" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "processId": { "type": "integer", "minimum": 1, "maximum": u32::MAX },
                "executable": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES },
                "executablePath": { "type": ["string", "null"], "maxLength": MAX_CONTROL_STRING_BYTES },
                "creationTime100ns": { "type": "string", "pattern": "^[0-9]+$", "maxLength": 20 },
                "mode": { "enum": ["include", "exclude"] },
                "renderEndpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES }
            }),
            &[
                "sessionId",
                "processId",
                "executable",
                "creationTime100ns",
                "mode",
                "renderEndpointId",
            ],
        ),
        "nativeEndpoints.pump" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "maxPackets": { "type": "integer", "minimum": 1, "maximum": audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE }
            }),
            &["sessionId", "generation"],
        ),
        "nativeDuplex.pump" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "maxInputQuanta": { "type": "integer", "minimum": 1, "maximum": audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE },
                "maxOutputPackets": { "type": "integer", "minimum": 1, "maximum": audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE }
            }),
            &["sessionId", "generation"],
        ),
        "nativeRenderSources.pump" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "maxQuanta": { "type": "integer", "minimum": 1, "maximum": audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE }
            }),
            &["sessionId", "generation"],
        ),
        "nativeMultiInputs.pump" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "maxPackets": { "type": "integer", "minimum": 1, "maximum": audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE }
            }),
            &["sessionId", "generation"],
        ),
        "nativeMultiInputs.bindBranches" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "branchNodeIds": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": audiorouter_engine::MAX_AUDIO_TAPS,
                    "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
                }
            }),
            &["sessionId", "generation", "branchNodeIds"],
        ),
        "plugins.scan" => object_schema(
            json!({
                "directory": { "type": "string", "minLength": 1 }
            }),
            &["directory"],
        ),
        "plugins.list" => object_schema(
            json!({
                "directory": { "type": "string", "minLength": 1 }
            }),
            &["directory"],
        ),
        "plugins.inventory" => object_schema(json!({}), &[]),
        "plugins.retry" => object_schema(
            json!({
                "directory": { "type": "string", "minLength": 1 },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["directory", "idempotencyKey"],
        ),
        "plugins.inspect" => object_schema(
            json!({
                "path": { "type": "string", "minLength": 1 }
            }),
            &["path"],
        ),
        "plugins.parameters" => object_schema(
            json!({
                "path": { "type": "string", "minLength": 1 }
            }),
            &["path"],
        ),
        "plugins.saveState" | "plugins.closeEditor" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
            }),
            &["sessionId", "nodeId"],
        ),
        "plugins.openEditor" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "parentWindow": { "type": "integer", "minimum": 1 },
                "ownerProcessId": { "type": "integer", "minimum": 1, "maximum": u32::MAX }
            }),
            &["sessionId", "nodeId", "parentWindow", "ownerProcessId"],
        ),
        "virtualDevices.list" => object_schema(
            json!({
                "cursor": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_VIRTUAL_DEVICE_LIST_ITEMS }
            }),
            &[],
        ),
        "virtualDevices.plan" => object_schema(
            json!({
                "operation": virtual_device_operation_schema()
            }),
            &["operation"],
        ),
        "virtualDevices.apply" => object_schema(
            json!({
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["planId", "idempotencyKey"],
        ),
        "virtualDevices.provision" => object_schema(
            json!({
                "busId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "instanceId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_VIRTUAL_BUS_DRIVER_INSTANCE_ID_CHARS },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["busId", "instanceId", "idempotencyKey"],
        ),
        "virtualDevices.remove" => object_schema(
            json!({
                "busId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["busId", "idempotencyKey"],
        ),
        "virtualRoutes.replace" => object_schema(
            json!({
                "baseRevision": { "type": "integer", "minimum": 0 },
                "routes": { "type": "array", "maxItems": audiorouter_domain::MAX_VIRTUAL_BUS_ROUTES, "items": virtual_bus_route_schema() },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["baseRevision", "routes", "idempotencyKey"],
        ),
        "recordings.get" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES } }),
            &["recordingId"],
        ),
        "recordings.recovery" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "cursor": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_RECORDING_LIST_ITEMS }
            }),
            &[],
        ),
        "recordings.reveal" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES } }),
            &["recordingId"],
        ),
        "recordings.preview" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES } }),
            &["recordingId"],
        ),
        "recordings.setMetadata" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "title": { "type": ["string", "null"], "maxLength": 256 },
                "artist": { "type": ["string", "null"], "maxLength": 256 },
                "comment": { "type": ["string", "null"], "maxLength": 256 },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["recordingId", "idempotencyKey"],
        ),
        "recordings.rename" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "newPath": { "type": "string", "minLength": 1 },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["recordingId", "newPath", "idempotencyKey"],
        ),
        "safety.setPrivacyMute" => object_schema(
            json!({
                "muted": { "type": "boolean" },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["muted", "idempotencyKey"],
        ),
        "recovery.clearSafeMode" => object_schema(
            json!({ "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["idempotencyKey"],
        ),
        "startup.get" => object_schema(json!({}), &[]),
        "startup.plan" => object_schema(json!({ "enabled": { "type": "boolean" } }), &["enabled"]),
        "startup.apply" => object_schema(
            json!({
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["planId", "idempotencyKey"],
        ),
        "recordings.removeEntry" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES }, "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["recordingId", "idempotencyKey"],
        ),
        "recordings.recycle" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "confirm": { "type": "boolean" },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["recordingId"],
        ),
        "sessions.get" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } }),
            &["sessionId"],
        ),
        "sessions.export" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } }),
            &["sessionId"],
        ),
        "sessions.exportFile" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "path": { "type": "string", "minLength": 1, "maxLength": 1024 },
                "replace": { "type": "boolean" }
            }),
            &["sessionId", "path"],
        ),
        "sessions.importFile" => object_schema(
            json!({ "path": { "type": "string", "minLength": 1, "maxLength": 1024 } }),
            &["path"],
        ),
        "sessions.importPlan" => {
            object_schema(json!({ "session": session_item_schema() }), &["session"])
        }
        "sessions.importCommit" => object_schema(
            json!({
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["planId", "idempotencyKey"],
        ),
        "sessions.delete" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["sessionId", "idempotencyKey"],
        ),
        "session.start" | "sessions.start" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                "candidate": session_item_schema()
            }),
            &["sessionId", "idempotencyKey"],
        ),
        "session.stop" | "sessions.stop" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["sessionId", "idempotencyKey"],
        ),
        "sessions.list" => object_schema(
            json!({
                "cursor": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_SESSION_LIST_ITEMS }
            }),
            &[],
        ),
        "sessions.active.get" => object_schema(json!({}), &[]),
        "sessions.active.set" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["sessionId", "idempotencyKey"],
        ),
        "sessions.create" => object_schema(
            json!({
                "session": session_item_schema(),
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["session", "idempotencyKey"],
        ),
        "sessions.duplicate" => object_schema(
            json!({
                "sourceSessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "name": { "type": ["string", "null"] },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["sourceSessionId", "sessionId", "idempotencyKey"],
        ),
        "routes.inspect" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "destinationNode": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
            }),
            &["sessionId", "destinationNode"],
        ),
        "graph.history" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "cursor": { "type": ["string", "null"], "maxLength": MAX_REVISION_CURSOR_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_GRAPH_HISTORY_ITEMS }
            }),
            &["sessionId"],
        ),
        "graph.undoPlan" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "baseRevision": { "type": "integer", "minimum": 0 }
            }),
            &["sessionId", "baseRevision"],
        ),
        "events.subscribe" => object_schema(
            json!({
                "afterSequence": { "type": "integer", "minimum": 0 },
                "backendEpoch": { "type": "integer", "minimum": 0 },
                "categories": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1, "maxLength": 128, "enum": STATE_CATEGORIES },
                    "maxItems": 32
                },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_EVENT_SUBSCRIPTION_ITEMS },
                "sessionId": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
            }),
            &[],
        ),
        "graph.plan" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "baseRevision": { "type": "integer", "minimum": 0 },
                "candidate": session_item_schema()
            }),
            &["sessionId", "baseRevision", "candidate"],
        ),
        "graph.commit" => object_schema(
            json!({
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "baseRevision": { "type": "integer", "minimum": 0 },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                "acknowledgments": {
                    "type": ["array", "null"],
                    "items": { "type": "string", "minLength": 1, "maxLength": 2048 },
                    "maxItems": 100
                }
            }),
            &["planId", "baseRevision", "idempotencyKey"],
        ),
        "processors.response" => object_schema(
            json!({
                "sampleRateHz": { "type": "number", "minimum": 8000, "maximum": 192000 },
                "bands": { "type": "array", "maxItems": MAX_RESPONSE_BANDS, "items": {
                    "type": "object", "properties": {
                        "enabled": { "type": "boolean" },
                        "type": { "enum": ["peaking", "lowShelf", "highShelf", "lowPass", "highPass", "bandPass", "allPass", "notch"] },
                        "frequencyHz": { "type": "number", "minimum": 20, "maximum": 20000 },
                        "q": { "type": "number", "minimum": 0.1, "maximum": 20 },
                        "gainDb": { "type": "number", "minimum": -24, "maximum": 24 }
                    }, "required": ["type", "frequencyHz", "q", "gainDb"], "additionalProperties": false
                }},
                "frequenciesHz": { "type": "array", "minItems": 1, "maxItems": MAX_RESPONSE_FREQUENCIES, "items": { "type": "number", "minimum": 1, "maximum": 96000 } }
            }),
            &["sampleRateHz", "bands", "frequenciesHz"],
        ),
        "sessions.play" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Defaults to the active session." }, "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["idempotencyKey"],
        ),
        "sessions.togglePlay" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Defaults to the active session." }, "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["idempotencyKey"],
        ),
        "sessions.summary" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Defaults to the active session." } }),
            &[],
        ),
        "meters.levels" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Defaults to the active session." } }),
            &[],
        ),
        "nodes.catalog" => object_schema(json!({}), &[]),
        "safety.togglePrivacyMute" => object_schema(
            json!({ "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["idempotencyKey"],
        ),
        "nodes.set" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Defaults to the active session." }, "node": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Node ID or exact name." }, "parameters": { "type": "object", "description": "Settings to change, by name (see nodes.catalog)." }, "enabled": { "type": "boolean" }, "bypass": { "type": "boolean" }, "name": { "type": "string", "minLength": 1, "maxLength": 120 }, "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["node", "idempotencyKey"],
        ),
        "nodes.toggle" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Defaults to the active session." }, "node": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Node ID or exact name." }, "target": { "type": "string", "minLength": 1, "maxLength": 128, "description": "enabled, bypass, or an on/off or two-choice setting name." }, "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["node", "target", "idempotencyKey"],
        ),
        "nodes.add" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Defaults to the active session." }, "kind": { "type": "string", "minLength": 1, "maxLength": 64, "description": "Node kind from nodes.catalog, for example compressor." }, "name": { "type": "string", "minLength": 1, "maxLength": 120 }, "parameters": { "type": "object" }, "between": { "type": "object", "properties": { "from": { "type": "string" }, "to": { "type": "string" } }, "required": ["from", "to"] }, "after": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Node to place the new one after." }, "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["kind", "idempotencyKey"],
        ),
        "nodes.remove" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Defaults to the active session." }, "node": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Node ID or exact name." }, "bridge": { "type": "boolean", "description": "Reconnect its neighbours (default true)." }, "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["node", "idempotencyKey"],
        ),
        "connections.add" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Defaults to the active session." }, "from": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Source node ID or name." }, "to": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Destination node ID or name." }, "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["from", "to", "idempotencyKey"],
        ),
        "connections.remove" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Defaults to the active session." }, "from": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Source node ID or name." }, "to": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "Destination node ID or name." }, "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["from", "to", "idempotencyKey"],
        ),
        _ => object_schema(json!({}), &[]),
    }
}

pub(crate) fn recorder_input_schema(frame_required: bool) -> Value {
    let mut properties = json!({
        "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
    });
    if frame_required {
        properties["frame"] = json!({ "type": "integer", "minimum": 0 });
    }
    if frame_required {
        object_schema(properties, &["sessionId", "frame", "idempotencyKey"])
    } else {
        object_schema(properties, &["sessionId", "idempotencyKey"])
    }
}

pub(crate) fn spectrum_telemetry_schema() -> Value {
    let bands =
        json!({ "type": "array", "items": { "type": "number" }, "minItems": 64, "maxItems": 64 });
    json!({
        "type": "object",
        "properties": { "levelsDb": bands.clone(), "bandFrequenciesHz": bands },
        "required": ["levelsDb", "bandFrequenciesHz"],
        "additionalProperties": false
    })
}
