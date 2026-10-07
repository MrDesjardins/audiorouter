//! API discovery: output schemas for every method.

use super::*;

pub(crate) fn method_output_schema(name: &str) -> Value {
    match name {
        "system.quit" => json!({
            "type": "object",
            "properties": {
                "state": { "const": "stopped" },
                "sessions": { "type": "array", "maxItems": audiorouter_domain::MAX_ACTIVE_SESSIONS },
                "recorders": { "type": "array", "maxItems": MAX_ACTIVE_RECORDERS }
            },
            "required": ["state", "sessions", "recorders"],
            "additionalProperties": false
        }),
        "system.osTransition" => json!({
            "type": "object",
            "properties": {
                "transition": { "enum": ["lock", "signOut", "sleep", "resume"] },
                "action": { "enum": ["keepRunning", "stopAndRelease", "revalidateBeforeRestart", "remainStopped"] },
                "endpointInventory": { "enum": ["refreshed", "notStarted"] },
                "nativeSessionIds": { "type": "array", "maxItems": audiorouter_domain::MAX_ACTIVE_SESSIONS, "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } },
                "sessionIds": { "type": "array", "maxItems": audiorouter_domain::MAX_ACTIVE_SESSIONS, "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } }
            },
            "required": ["transition", "action", "endpointInventory", "nativeSessionIds", "sessionIds"],
            "additionalProperties": false
        }),
        "recorders.create" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "recorderId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "format": { "enum": ["wavPcm16", "wavPcm24", "wavFloat32", "flac16", "flac24", "mp3"] },
                "path": { "type": "string", "minLength": 1 },
                "state": { "const": "idle" },
                "armed": { "const": false }
            },
            "required": ["sessionId", "recorderId", "format", "path", "state", "armed"],
            "additionalProperties": false
        }),
        "recorders.list" => json!({
            "type": "array",
            "maxItems": MAX_ACTIVE_RECORDERS,
            "items": {
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                    "nodeId": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                    "state": { "enum": ["idle", "armed", "recording", "paused", "stopping", "completed", "failed"] },
                    "lastFrame": { "type": ["integer", "null"], "minimum": 0 }
                },
                "required": ["sessionId", "state", "lastFrame"],
                "additionalProperties": false
            }
        }),
        "devices.getAccess" | "devices.setAccess" => json!({
            "type": "object",
            "properties": { "allowed": { "type": "boolean" } },
            "required": ["allowed"],
            "additionalProperties": false
        }),
        "recordings.getRoot" => json!({
            "type": "object",
            "properties": {
                "root": { "type": ["string", "null"] },
                "suggestedRoot": { "type": ["string", "null"] }
            },
            "required": ["root", "suggestedRoot"],
            "additionalProperties": false
        }),
        "recordings.setRoot" => json!({
            "type": "object",
            "properties": {
                "root": { "type": "string" },
                "created": { "type": "boolean" }
            },
            "required": ["root", "created"],
            "additionalProperties": false
        }),
        "recorders.startRecording" | "recorders.stopRecording" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "enum": ["idle", "armed", "recording", "paused", "stopping", "completed", "failed"] },
                "format": { "enum": ["wavPcm16", "wavPcm24", "wavFloat32", "flac16", "flac24", "mp3"] },
                "path": { "type": ["string", "null"] },
                "splitMinutes": { "type": "integer", "minimum": 0, "maximum": 240 },
                "alreadyRecording": { "type": "boolean" },
                "reason": { "type": "string" },
                "paths": { "type": "array", "items": { "type": "string" } },
                "parts": { "type": "array", "maxItems": audiorouter_recording::MAX_CHECKPOINT_PARTS },
                "pauses": { "type": "array", "maxItems": audiorouter_recording::MAX_CHECKPOINT_PAUSES },
                "lastFrame": { "type": ["integer", "null"] }
            },
            "required": ["nodeId", "state"],
            "additionalProperties": false
        }),
        "recorders.arm" | "recorders.start" | "recorders.pause" | "recorders.resume"
        | "recorders.split" | "recorders.stop" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "enum": ["idle", "armed", "recording", "paused", "stopping", "completed", "failed"] },
                "parts": { "type": "array", "maxItems": audiorouter_recording::MAX_CHECKPOINT_PARTS },
                "pauses": { "type": "array", "maxItems": audiorouter_recording::MAX_CHECKPOINT_PAUSES },
                "lastFrame": { "type": ["integer", "null"] }
            },
            "required": ["sessionId", "state", "parts", "pauses", "lastFrame"],
            "additionalProperties": false
        }),
        "system.describe" => json!({
            "type": "object",
            "properties": {
                "protocolVersion": {
                    "type": "object",
                    "properties": {
                        "major": { "type": "integer", "minimum": 0 },
                        "minor": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["major", "minor"],
                    "additionalProperties": false
                },
                "schemaVersion": { "type": "integer", "minimum": 0 },
                "build": { "type": "string", "minLength": 1 },
                "methods": {
                    "type": "array",
                    "maxItems": API_METHODS.len(),
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "minLength": 1 },
                            "description": { "type": "string", "minLength": 1 },
                            "permission": { "type": "string", "minLength": 1 },
                            "sideEffect": { "type": "string", "minLength": 1 },
                            "inputSchema": { "type": "object" },
                            "outputSchema": { "type": "object" }
                        },
                        "required": ["name", "description", "permission", "sideEffect", "inputSchema", "outputSchema"],
                        "additionalProperties": false
                    }
                },
                "nodeTypes": { "type": "array", "maxItems": audiorouter_domain::node_registry().len(), "items": { "type": "object" } },
                "processors": { "type": "array", "maxItems": MAX_PROCESSOR_CATALOG_ITEMS, "items": processor_item_schema() },
                "presets": {
                    "type": "object",
                    "properties": {
                        "voiceChains": {
                            "type": "array",
                            "maxItems": audiorouter_dsp::VoiceChainPresetId::ALL.len(),
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "string", "minLength": 1 },
                                    "version": { "const": 1 },
                                    "name": { "type": "string", "minLength": 1 },
                                    "description": { "type": "string", "minLength": 1 }
                                },
                                "required": ["id", "version", "name", "description"],
                                "additionalProperties": false
                            }
                        },
                        "eq": {
                            "type": "array",
                            "maxItems": audiorouter_dsp::EqPresetId::ALL.len(),
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "string", "minLength": 1 },
                                    "version": { "const": 1 },
                                    "name": { "type": "string", "minLength": 1 },
                                    "description": { "type": "string", "minLength": 1 }
                                },
                                "required": ["id", "version", "name", "description"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["voiceChains", "eq"],
                    "additionalProperties": false
                },
                "limits": {
                    "type": "object",
                    "properties": {
                        "maxNodesPerSession": { "type": "integer", "minimum": 1 },
                        "maxEdgesPerSession": { "type": "integer", "minimum": 1 },
                        "maxNodesGlobal": { "type": "integer", "minimum": 1 },
                        "maxEdgesGlobal": { "type": "integer", "minimum": 1 },
                        "maxSessionsGlobal": { "type": "integer", "minimum": 1 },
                        "maxActiveSessions": { "type": "integer", "minimum": 1 },
                        "maxActiveRecorders": { "type": "integer", "minimum": 1 },
                        "maxClientEnrollments": { "type": "integer", "minimum": 1 },
                        "maxOperationJournalEntries": { "type": "integer", "minimum": 1 },
                        "maxVirtualBuses": { "type": "integer", "minimum": 1 },
                        "maxVirtualBusNameChars": { "type": "integer", "minimum": 1 },
                        "maxEntityIdBytes": { "type": "integer", "minimum": 1 },
                        "maxDisplayNameBytes": { "type": "integer", "minimum": 1 },
                        "maxPortNameBytes": { "type": "integer", "minimum": 1 },
                        "maxPortsPerNode": { "type": "integer", "minimum": 1 },
                        "maxChannelMatrixCoefficients": { "type": "integer", "minimum": 1 },
                        "maxControlValueDepth": { "type": "integer", "minimum": 1 },
                        "maxControlStringBytes": { "type": "integer", "minimum": 1 },
                        "maxControlValueCount": { "type": "integer", "minimum": 1 },
                        "maxMethodNameBytes": { "type": "integer", "minimum": 1 },
                        "maxRequestIdBytes": { "type": "integer", "minimum": 1 },
                        "maxRevisionCursorBytes": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["maxNodesPerSession", "maxEdgesPerSession", "maxNodesGlobal", "maxEdgesGlobal", "maxSessionsGlobal", "maxActiveSessions", "maxActiveRecorders", "maxClientEnrollments", "maxOperationJournalEntries", "maxVirtualBuses", "maxVirtualBusNameChars", "maxEntityIdBytes", "maxDisplayNameBytes", "maxPortNameBytes", "maxPortsPerNode", "maxChannelMatrixCoefficients", "maxControlValueDepth", "maxControlStringBytes", "maxControlValueCount", "maxMethodNameBytes", "maxRequestIdBytes", "maxRevisionCursorBytes"],
                    "additionalProperties": false
                },
                "events": {
                    "type": "object",
                    "properties": {
                        "stateCategories": { "type": "array", "maxItems": STATE_CATEGORIES.len(), "items": { "type": "string", "minLength": 1 } },
                        "meterReplay": { "const": false },
                        "retention": {
                            "type": "object",
                            "properties": {
                                "maxEvents": { "type": "integer", "minimum": 1 },
                                "maxAgeSeconds": { "type": "integer", "minimum": 1 }
                            },
                            "required": ["maxEvents", "maxAgeSeconds"],
                            "additionalProperties": false
                        }
                    },
                    "required": ["stateCategories", "meterReplay", "retention"],
                    "additionalProperties": false
                }
            },
            "required": ["protocolVersion", "schemaVersion", "build", "methods", "nodeTypes", "processors", "presets", "limits", "events"],
            "additionalProperties": false
        }),
        "system.handshake" => json!({
            "type": "object",
            "properties": {
                "compatible": { "const": true },
                "requested": {
                    "type": "object",
                    "properties": {
                        "major": { "type": "integer", "minimum": 0 },
                        "minor": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["major", "minor"],
                    "additionalProperties": false
                },
                "negotiated": {
                    "type": "object",
                    "properties": {
                        "major": { "const": 1 },
                        "minor": { "const": 0 }
                    },
                    "required": ["major", "minor"],
                    "additionalProperties": false
                },
                "schemaVersion": { "type": "integer", "minimum": 0 }
            },
            "required": ["compatible", "requested", "negotiated", "schemaVersion"],
            "additionalProperties": false
        }),
        "status.get" => status_output_schema(),
        "system.diagnostics" => diagnostics_output_schema(),
        "diagnostics.getVerbose" | "diagnostics.setVerbose" => json!({
            "type": "object",
            "properties": {
                "enabled": { "type": "boolean" },
                "expiresAtUnixMs": { "type": ["integer", "null"], "minimum": 0 },
                "remainingSeconds": { "type": "integer", "minimum": 0, "maximum": audiorouter_protocol::diagnostics::MAX_VERBOSE_DIAGNOSTICS_MS / 1000 },
                "maxSeconds": { "const": audiorouter_protocol::diagnostics::MAX_VERBOSE_DIAGNOSTICS_MS / 1000 }
            },
            "required": ["enabled", "expiresAtUnixMs", "remainingSeconds", "maxSeconds"],
            "additionalProperties": false
        }),
        "recovery.clearSafeMode" => json!({
            "type": "object",
            "properties": {
                "safeMode": { "const": false },
                "recentCrashes": { "type": "integer", "minimum": 0 },
                "persistence": { "enum": ["durable", "memory"] }
            },
            "required": ["safeMode", "recentCrashes", "persistence"],
            "additionalProperties": false
        }),
        "startup.get" => json!({
            "type": "object",
            "properties": {
                "enabled": { "type": "boolean" },
                "registration": { "const": "unavailable" },
                "reason": { "type": "string", "minLength": 1 }
            },
            "required": ["enabled", "registration", "reason"],
            "additionalProperties": false
        }),
        "startup.plan" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "enabled": { "type": "boolean" },
                "registration": { "const": "unavailable" },
                "reason": { "type": "string", "minLength": 1 },
                "requiredScopes": { "type": "array", "maxItems": MAX_PLAN_REQUIRED_SCOPES, "items": { "type": "string" } },
                "warnings": { "type": "array", "maxItems": MAX_PLAN_WARNINGS, "items": { "type": "string" } }
            },
            "required": ["planId", "enabled", "registration", "reason", "requiredScopes", "warnings"],
            "additionalProperties": false
        }),
        "startup.apply" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "unavailable" },
                "registration": { "const": "unavailable" },
                "reason": { "type": "string", "minLength": 1 }
            },
            "required": ["planId", "state", "registration", "reason"],
            "additionalProperties": false
        }),
        "sessions.list" => {
            let item = session_item_schema();
            json!({
                "type": "object",
                "properties": {
                    "items": { "type": "array", "maxItems": MAX_SESSION_LIST_ITEMS, "items": item },
                    "nextCursor": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
                },
                "required": ["items", "nextCursor"],
                "additionalProperties": false
            })
        }
        "sessions.active.get" => json!({
            "type": "object",
            "properties": { "sessionId": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } },
            "required": ["sessionId"],
            "additionalProperties": false
        }),
        "sessions.active.set" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } }),
            &["sessionId"],
        ),
        "graph.history" => {
            let item = session_item_schema();
            json!({
                "type": "object",
                "properties": {
                    "items": { "type": "array", "maxItems": MAX_GRAPH_HISTORY_ITEMS, "items": item },
                    "nextCursor": { "type": ["string", "null"], "maxLength": MAX_REVISION_CURSOR_BYTES }
                },
                "required": ["items", "nextCursor"],
                "additionalProperties": false
            })
        }
        "sessions.get" | "sessions.export" => session_item_schema(),
        "sessions.exportFile" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string" },
                "path": { "type": "string" },
                "revision": { "type": "integer", "minimum": 0 },
                "bytes": { "type": "integer", "minimum": 0 }
            },
            "required": ["sessionId", "path", "revision", "bytes"],
            "additionalProperties": false
        }),
        "sessions.importFile" => json!({
            "type": "object",
            "properties": {
                "session": session_item_schema(),
                "state": { "const": "stopped" },
                "renamed": { "type": "boolean" },
                "mediaRestored": { "type": "integer", "minimum": 0 },
                "pluginStatesRestored": { "type": "integer", "minimum": 0 },
                "missingAssets": { "type": "integer", "minimum": 0 }
            },
            "required": ["session", "state", "renamed", "mediaRestored", "pluginStatesRestored", "missingAssets"],
            "additionalProperties": false
        }),
        "sessions.importPlan" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "expiresInMs": { "type": "integer", "minimum": 1 },
                "session": session_item_schema()
            },
            "required": ["planId", "expiresInMs", "session"],
            "additionalProperties": false
        }),
        "sessions.importCommit" => json!({
            "type": "object",
            "properties": {
                "session": session_item_schema(),
                "state": { "const": "stopped" },
                "imported": { "const": true }
            },
            "required": ["session", "state", "imported"],
            "additionalProperties": false
        }),
        "sessions.create" | "sessions.duplicate" => json!({
            "type": "object",
            "properties": {
                "session": session_item_schema(),
                "state": { "const": "stopped" }
            },
            "required": ["session", "state"],
            "additionalProperties": false
        }),
        "sessions.delete" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "deleted": { "const": true }
            },
            "required": ["sessionId", "deleted"],
            "additionalProperties": false
        }),
        "session.start" | "sessions.start" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "running" },
                "generation": { "type": "integer", "minimum": 1 },
                "runtime": { "enum": ["fake", "native"] },
                "preview": { "type": "boolean" },
                "savedRevision": { "type": "integer", "minimum": 0 }
            },
            "required": ["sessionId", "state", "generation", "runtime"],
            "additionalProperties": false
        }),
        "session.stop" | "sessions.stop" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "stopped" },
                "runtime": { "enum": ["fake", "native"] },
                "recorders": {
                    "type": "array",
                    "maxItems": audiorouter_domain::MAX_ACTIVE_SESSIONS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                            "state": { "const": "completed" },
                            "fileFinalized": { "const": true },
                            "recoverable": { "const": false }
                        },
                        "required": ["sessionId", "state", "fileFinalized", "recoverable"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["sessionId", "state", "runtime", "recorders"],
            "additionalProperties": false
        }),
        "graph.undoPlan" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "baseRevision": { "type": "integer", "minimum": 0 },
                "expiresInMs": { "type": "integer", "minimum": 1 }
            },
            "required": ["planId", "baseRevision", "expiresInMs"],
            "additionalProperties": false
        }),
        "safety.setPrivacyMute" => json!({
            "type": "object",
            "properties": {
                "muted": { "type": "boolean" },
                "persistence": { "enum": ["durable", "memory"] },
                "audioEffect": { "type": "string", "minLength": 1 }
            },
            "required": ["muted", "persistence", "audioEffect"],
            "additionalProperties": false
        }),
        "events.subscribe" => json!({
            "type": "object",
            "properties": {
                "backendEpoch": { "type": "integer", "minimum": 0 },
                "events": {
                    "type": "array",
                    "maxItems": MAX_EVENT_SUBSCRIPTION_ITEMS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "sequence": { "type": "integer", "minimum": 1 },
                            "backendEpoch": { "type": "integer", "minimum": 0 },
                            "resourceRevision": { "type": "integer", "minimum": 0 },
                            "operationId": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_EVENT_OPERATION_ID_BYTES },
                            "category": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_EVENT_CATEGORY_BYTES },
                            "sessionId": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
                        },
                        "required": ["sequence", "backendEpoch", "resourceRevision", "operationId", "category", "sessionId"],
                        "additionalProperties": false
                    }
                },
                "nextSequence": { "type": "integer", "minimum": 0 },
                "resyncRequired": { "type": "boolean" },
                "reason": { "type": "string", "minLength": 1 },
                "snapshot": {
                    "type": "object",
                    "properties": {
                        "sessions": {
                            "type": "object",
                            "properties": {
                                "items": { "type": "array", "maxItems": MAX_EVENT_SUBSCRIPTION_ITEMS, "items": session_item_schema() },
                "nextCursor": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
                            },
                            "required": ["items", "nextCursor"],
                            "additionalProperties": false
                        }
                    },
                    "required": ["sessions"],
                    "additionalProperties": false
                }
            },
            "required": ["backendEpoch", "events", "nextSequence"],
            "additionalProperties": false
        }),
        "routes.inspect" => json!({
            "type": "object",
            "properties": {
                "destinationNode": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "reachable": { "type": "boolean" },
                "complete": { "type": "boolean" },
                "paths": {
                    "type": "array",
                    "maxItems": audiorouter_domain::MAX_ROUTE_PATHS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "nodes": { "type": "array", "maxItems": audiorouter_domain::MAX_NODES_PER_SESSION, "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } },
                            "edges": { "type": "array", "maxItems": audiorouter_domain::MAX_EDGES_PER_SESSION, "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } },
                            "channelMaps": { "type": "array", "maxItems": audiorouter_domain::MAX_EDGES_PER_SESSION, "items": { "type": "array", "maxItems": audiorouter_domain::MAX_CHANNEL_MATRIX_COEFFICIENTS, "items": { "type": "number", "minimum": -2, "maximum": 2 } } },
                            "latencySamples": { "type": "integer", "minimum": 0 }
                        },
                        "required": ["nodes", "edges", "channelMaps", "latencySamples"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["destinationNode", "reachable", "complete", "paths"],
            "additionalProperties": false
        }),
        "graph.plan" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "baseRevision": { "type": "integer", "minimum": 0 },
                "expiresInMs": { "type": "integer", "minimum": 1 },
                "diff": { "type": "array", "maxItems": MAX_GRAPH_DIFF_ITEMS },
                "affectedDestinations": {
                    "type": "array",
                    "maxItems": MAX_GRAPH_AFFECTED_DESTINATIONS,
                    "items": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": audiorouter_domain::MAX_DISPLAY_NAME_BYTES
                    }
                },
                "warnings": { "type": "array", "maxItems": MAX_PLAN_WARNINGS, "items": { "type": "string", "minLength": 1 } },
                "requiredScopes": { "type": "array", "maxItems": MAX_PLAN_REQUIRED_SCOPES, "items": { "type": "string", "minLength": 1 } }
            },
            "required": ["planId", "baseRevision", "expiresInMs", "diff", "affectedDestinations", "warnings", "requiredScopes"],
            "additionalProperties": false
        }),
        "graph.commit" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "revision": { "type": "integer", "minimum": 0 },
                "idempotentReplay": { "type": "boolean" },
                "activation": { "type": "object" }
            },
            "required": ["sessionId", "revision"],
            "additionalProperties": false
        }),
        "operations.get" => json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "operationId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                        "operation": { "type": "string", "minLength": 1 },
                        "status": { "const": "completed" },
                        "durable": { "type": "boolean" },
                        "revision": { "type": "integer", "minimum": 0 },
                        "createdAt": { "type": ["integer", "null"] },
                        "result": { "type": "object" }
                    },
                    "required": ["operationId", "operation", "status", "durable", "revision", "createdAt", "result"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "operationId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                        "status": { "const": "unknown" },
                        "durable": { "const": false }
                    },
                    "required": ["operationId", "status", "durable"],
                    "additionalProperties": false
                }
            ]
        }),
        "operations.cancel" => json!({
            "type": "object",
            "properties": {
                "operationId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                "status": { "const": "completed" },
                "cancelled": { "const": false },
                "reason": { "const": "alreadyCompleted" }
            },
            "required": ["operationId", "status", "cancelled", "reason"],
            "additionalProperties": false
        }),
        "apps.list" | "applications.list" => json!({
            "type": "array",
            "maxItems": audiorouter_windows_audio::MAX_APPLICATIONS,
            "items": {
                "type": "object",
                "properties": {
                    "processId": { "type": "integer", "minimum": 1 },
                    "executable": { "type": "string", "maxLength": 260 },
                    "executablePath": { "type": ["string", "null"], "maxLength": 32768 },
                    "creationTime100ns": { "type": ["string", "null"] },
                    "audioActivity": { "enum": ["active", "inactive", "none"] },
                    "captureCapability": { "enum": ["observed", "notObserved"] },
                    "audioSessionCount": { "type": "integer", "minimum": 0 },
                    "activeAudioSessionCount": { "type": "integer", "minimum": 0 },
                    "captureSessionCount": { "type": "integer", "minimum": 0 },
                    "renderSessionCount": { "type": "integer", "minimum": 0 },
                    "audioDisplayNames": {
                        "type": "array",
                        "maxItems": audiorouter_windows_audio::MAX_APPLICATION_AUDIO_DISPLAY_NAMES,
                        "items": {
                            "type": "string",
                            "maxLength": audiorouter_windows_audio::MAX_APPLICATION_AUDIO_DISPLAY_NAME_BYTES
                        }
                    }
                },
                "required": ["processId", "executable", "executablePath", "creationTime100ns", "audioActivity", "captureCapability", "audioSessionCount", "activeAudioSessionCount", "captureSessionCount", "renderSessionCount", "audioDisplayNames"],
                "additionalProperties": false
            }
        }),
        "devices.list" => {
            let item = device_item_schema();
            json!({
                "oneOf": [
                    { "type": "array", "maxItems": MAX_DEVICE_LIST_ITEMS, "items": item.clone() },
                    {
                        "type": "object",
                        "properties": {
                            "items": { "type": "array", "maxItems": MAX_DEVICE_LIST_ITEMS, "items": item },
                                "nextCursor": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
                        },
                        "required": ["items", "nextCursor"],
                        "additionalProperties": false
                    }
                ]
            })
        }
        "nativeEndpoints.prepare" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "configured-stopped" },
                "captureEndpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES },
                "renderEndpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES }
            },
            "required": ["sessionId", "state", "captureEndpointId", "renderEndpointId"],
            "additionalProperties": false
        }),
        "nativeOutputs.prepare" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "state": { "const": "configured-stopped" },
                "renderEndpointIds": { "type": "array", "minItems": 1, "maxItems": audiorouter_engine::MAX_AUDIO_TAPS, "items": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES } },
                "outputCount": { "type": "integer", "minimum": 1, "maximum": audiorouter_engine::MAX_AUDIO_TAPS }
            },
            "required": ["sessionId", "generation", "state", "renderEndpointIds", "outputCount"],
            "additionalProperties": false
        }),
        "nativeMultiInputs.prepare" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "state": { "const": "configured-stopped" },
                "sources": { "type": "array", "minItems": 2, "maxItems": audiorouter_engine::MAX_MIXER_INPUTS, "items": { "type": "object", "properties": { "kind": { "enum": ["physical", "application", "generated"] }, "endpointId": { "type": "string" }, "processId": { "type": "integer" }, "executable": { "type": "string" } }, "required": ["kind"], "additionalProperties": false } },
                "sourceNodeIds": { "type": "array", "minItems": 2, "maxItems": audiorouter_engine::MAX_MIXER_INPUTS, "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } },
                "branchNodeIds": { "type": "array", "minItems": 2, "maxItems": audiorouter_engine::MAX_AUDIO_TAPS, "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } }
            },
            "required": ["sessionId", "generation", "state", "sources", "sourceNodeIds", "branchNodeIds"],
            "additionalProperties": false
        }),
        "nativePaths.prepare" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "state": { "const": "configured-stopped" },
                "pathCount": { "type": "integer", "minimum": 1, "maximum": audiorouter_engine::MAX_AUDIO_TAPS },
                "sourceNodeIds": { "type": "array", "minItems": 1, "maxItems": audiorouter_engine::MAX_AUDIO_TAPS, "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } },
                "branchNodeIds": { "type": "array", "minItems": 1, "maxItems": audiorouter_engine::MAX_AUDIO_TAPS, "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } },
                "renderEndpointIds": { "type": "array", "maxItems": audiorouter_engine::MAX_AUDIO_TAPS, "items": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES } }
            },
            "required": ["sessionId", "generation", "state", "pathCount", "sourceNodeIds", "branchNodeIds", "renderEndpointIds"],
            "additionalProperties": false
        }),
        "nativeBridges.prepare" => json!({
            "type": "object",
            "properties": {
                "busId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "state": { "const": "configured-stopped" },
                "directions": { "const": ["renderSource", "captureSink"] }
            },
            "required": ["busId", "generation", "state", "directions"],
            "additionalProperties": false
        }),
        "nativeBridges.detach" => json!({
            "type": "object",
            "properties": {
                "busId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "detached" }
            },
            "required": ["busId", "state"],
            "additionalProperties": false
        }),
        "nativeBridges.heartbeat" => json!({
            "type": "object",
            "properties": {
                "state": { "const": "healthy" },
                "bindings": { "type": "integer", "minimum": 0 }
            },
            "required": ["state", "bindings"],
            "additionalProperties": false
        }),
        "nativeEndpoints.rebind" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "configured-stopped" },
                "captureEndpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES },
                "renderEndpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES }
            },
            "required": ["sessionId", "state", "captureEndpointId", "renderEndpointId"],
            "additionalProperties": false
        }),
        "nativeEndpoints.detach" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "detached" }
            },
            "required": ["sessionId", "state"],
            "additionalProperties": false
        }),
        "nativeDuplex.detach" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "detached" }
            },
            "required": ["sessionId", "state"],
            "additionalProperties": false
        }),
        "nativeApplications.prepare" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "configured-stopped" },
                "processId": { "type": "integer", "minimum": 1, "maximum": u32::MAX },
                "executable": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES },
                "executablePath": { "type": ["string", "null"], "maxLength": MAX_CONTROL_STRING_BYTES },
                "creationTime100ns": { "type": "string", "pattern": "^[0-9]+$", "maxLength": 20 },
                "mode": { "enum": ["include", "exclude"] },
                "renderEndpointId": { "type": "string", "minLength": 1, "maxLength": MAX_CONTROL_STRING_BYTES }
            },
            "required": ["sessionId", "state", "processId", "executable", "executablePath", "creationTime100ns", "mode", "renderEndpointId"],
            "additionalProperties": false
        }),
        "nativeEndpoints.pump" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "audioService": audio_service_output_schema(),
                "packets": { "type": "integer", "minimum": 0 },
                "capturedFrames": { "type": "integer", "minimum": 0 },
                "processedQuanta": { "type": "integer", "minimum": 0 },
                "renderedFrames": { "type": "integer", "minimum": 0 },
                "droppedRenderFrames": { "type": "integer", "minimum": 0 },
                "renderBackpressureEvents": { "type": "integer", "minimum": 0 },
                "recorderChunksDrained": { "type": "integer", "minimum": 0 }
            },
            "required": ["sessionId", "generation", "packets", "capturedFrames", "processedQuanta", "renderedFrames", "droppedRenderFrames", "renderBackpressureEvents", "recorderChunksDrained"],
            "additionalProperties": false
        }),
        "nativeDuplex.pump" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "audioService": audio_service_output_schema(),
                "input": {
                    "type": "object",
                    "properties": {
                        "packets": { "type": "integer", "minimum": 0 },
                        "capturedFrames": { "type": "integer", "minimum": 0 },
                        "processedQuanta": { "type": "integer", "minimum": 0 },
                        "renderedFrames": { "type": "integer", "minimum": 0 },
                        "droppedRenderFrames": { "type": "integer", "minimum": 0 },
                        "renderBackpressureEvents": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["packets", "capturedFrames", "processedQuanta", "renderedFrames", "droppedRenderFrames", "renderBackpressureEvents"],
                    "additionalProperties": false
                },
                "output": {
                    "type": "object",
                    "properties": {
                        "packets": { "type": "integer", "minimum": 0 },
                        "capturedFrames": { "type": "integer", "minimum": 0 },
                        "processedQuanta": { "type": "integer", "minimum": 0 },
                        "renderedFrames": { "type": "integer", "minimum": 0 },
                        "droppedRenderFrames": { "type": "integer", "minimum": 0 },
                        "renderBackpressureEvents": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["packets", "capturedFrames", "processedQuanta", "renderedFrames", "droppedRenderFrames", "renderBackpressureEvents"],
                    "additionalProperties": false
                }
            },
            "required": ["sessionId", "generation", "input", "output"],
            "additionalProperties": false
        }),
        "nativeRenderSources.pump" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "audioService": audio_service_output_schema(),
                "packets": { "type": "integer", "minimum": 0 },
                "processedQuanta": { "type": "integer", "minimum": 0 },
                "renderedFrames": { "type": "integer", "minimum": 0 },
                "droppedRenderFrames": { "type": "integer", "minimum": 0 }
            },
            "required": ["sessionId", "generation", "packets", "processedQuanta", "renderedFrames", "droppedRenderFrames"],
            "additionalProperties": false
        }),
        "nativeMultiInputs.pump" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "audioService": audio_service_output_schema(),
                "inputs": { "type": "integer", "minimum": 0 },
                "capturedFrames": { "type": "integer", "minimum": 0 },
                "submittedQuanta": { "type": "integer", "minimum": 0 },
                "outputCount": { "type": "integer", "minimum": 0 },
                "deliveredQuanta": { "type": "integer", "minimum": 0 },
                "renderedFrames": { "type": "integer", "minimum": 0 },
                "renderBackpressureEvents": { "type": "integer", "minimum": 0 },
                "outputUnderruns": { "type": "integer", "minimum": 0 },
                "recorderChunksDrained": { "type": "integer", "minimum": 0 }
            },
            "required": ["sessionId", "generation", "inputs", "capturedFrames", "submittedQuanta", "outputCount", "deliveredQuanta", "renderedFrames", "renderBackpressureEvents"],
            "additionalProperties": false
        }),
        "nativeMultiInputs.bindBranches" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "generation": { "type": "integer", "minimum": 1 },
                "branchNodeIds": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": audiorouter_engine::MAX_AUDIO_TAPS,
                    "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
                },
                "boundBranches": { "type": "integer", "minimum": 1 }
            },
            "required": ["sessionId", "generation", "branchNodeIds", "boundBranches"],
            "additionalProperties": false
        }),
        "plugins.saveState" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "stateId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "sizeBytes": { "type": "integer", "minimum": 1 }
            },
            "required": ["sessionId", "nodeId", "stateId", "sizeBytes"],
            "additionalProperties": false
        }),
        "plugins.openEditor" | "plugins.closeEditor" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "enum": ["open", "closed"] }
            },
            "required": ["sessionId", "nodeId", "state"],
            "additionalProperties": false
        }),
        "plugins.inventory" => json!({
            "type": "object",
            "properties": {
                "inventories": {
                    "type": "array",
                    "maxItems": MAX_PLUGIN_INVENTORY_ROOTS,
                    "items": method_output_schema("plugins.list")
                }
            },
            "required": ["inventories"],
            "additionalProperties": false
        }),
        "plugins.scan" | "plugins.list" | "plugins.retry" => json!({
            "type": "object",
            "properties": {
                "directory": { "type": "string", "minLength": 1 },
                "entries": {
                    "type": "array",
                    "maxItems": audiorouter_plugin_host::MAX_SCAN_CANDIDATES,
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "minLength": 1 },
                            "identity": {
                                "type": ["object", "null"],
                                "properties": {
                                    "path": { "type": "string", "minLength": 1 },
                                    "binaryPath": { "type": "string", "minLength": 1 },
                                    "format": { "enum": ["vst3", "vst2", "unknown"] },
                                    "architecture": { "enum": ["x64", "x86", "arm64", "unknown"] },
                                    "fileBytes": { "type": "integer", "minimum": 1, "maximum": audiorouter_plugin_host::MAX_PLUGIN_BYTES },
                                    "sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
                                    "vendor": { "type": ["string", "null"], "maxLength": 128 },
                                    "version": { "type": ["string", "null"], "maxLength": 128 },
                                    "classIds": { "type": "array", "maxItems": 256, "items": { "type": "string", "maxLength": 32 } },
                                    "compatibility": { "enum": ["supportedVst3X64", "supportedVst2X64Gated", "unsupportedFormat"] }
                                },
                                "required": ["path", "binaryPath", "format", "architecture", "fileBytes", "sha256", "vendor", "version", "classIds", "compatibility"],
                                "additionalProperties": false
                            },
                            "error": { "type": ["string", "null"] },
                            "errorCode": { "enum": ["outsideConfiguredRoot", "unsupportedExtension", "missing", "tooLarge", "notPe", "unsupportedArchitecture", "cancelled", "deadlineExceeded", "io", null] }
                        },
                        "required": ["path", "identity", "error", "errorCode"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["directory", "entries"],
            "additionalProperties": false
        }),
        "plugins.inspect" => json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "minLength": 1 },
                "identity": {
                    "type": ["object", "null"],
                    "properties": {
                        "path": { "type": "string", "minLength": 1 },
                        "binaryPath": { "type": "string", "minLength": 1 },
                        "format": { "enum": ["vst3", "vst2", "unknown"] },
                        "architecture": { "enum": ["x64", "x86", "arm64", "unknown"] },
                        "fileBytes": { "type": "integer", "minimum": 1, "maximum": audiorouter_plugin_host::MAX_PLUGIN_BYTES },
                        "sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
                        "vendor": { "type": ["string", "null"], "maxLength": 128 },
                        "version": { "type": ["string", "null"], "maxLength": 128 },
                        "classIds": { "type": "array", "maxItems": 256, "items": { "type": "string", "maxLength": 32 } },
                        "compatibility": { "enum": ["supportedVst3X64", "supportedVst2X64Gated", "unsupportedFormat"] }
                    },
                    "required": ["path", "binaryPath", "format", "architecture", "fileBytes", "sha256", "vendor", "version", "classIds", "compatibility"],
                    "additionalProperties": false
                },
                "error": { "type": ["string", "null"] },
                "errorCode": { "enum": ["outsideConfiguredRoot", "unsupportedExtension", "missing", "tooLarge", "notPe", "unsupportedArchitecture", "cancelled", "deadlineExceeded", "io", null] }
            },
            "required": ["path", "identity", "error", "errorCode"],
            "additionalProperties": false
        }),
        "virtualDevices.list" => json!({
            "oneOf": [
                {
                    "type": "array",
                    "maxItems": audiorouter_domain::MAX_VIRTUAL_BUSES,
                    "items": virtual_device_item_schema()
                },
                {
                    "type": "object",
                    "properties": {
                        "items": { "type": "array", "maxItems": MAX_VIRTUAL_DEVICE_LIST_ITEMS, "items": virtual_device_item_schema() },
                        "nextCursor": { "type": ["string", "null"] }
                    },
                    "required": ["items", "nextCursor"],
                    "additionalProperties": false
                }
            ]
        }),
        "virtualDevices.plan" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1 },
                "expiresInMs": { "type": "integer", "minimum": 1 },
                "operation": virtual_device_operation_schema(),
                "availability": {
                    "type": "object",
                    "properties": {
                        "status": { "const": "unavailable" },
                        "reason": { "type": "string", "minLength": 1 }
                    },
                    "required": ["status", "reason"],
                    "additionalProperties": false
                },
                "requiredScopes": { "type": "array", "maxItems": MAX_PLAN_REQUIRED_SCOPES, "items": { "type": "string" } },
                "warnings": { "type": "array", "maxItems": MAX_PLAN_WARNINGS, "items": { "type": "string" } }
            },
            "required": ["planId", "expiresInMs", "operation", "availability", "requiredScopes", "warnings"],
            "additionalProperties": false
        }),
        "virtualDevices.apply" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1 },
                "state": { "const": "applied" },
                "availability": { "type": "object" },
                "operation": virtual_device_operation_schema()
            },
            "required": ["planId", "state", "availability", "operation"],
            "additionalProperties": false
        }),
        "virtualDevices.provision" => json!({
            "type": "object",
            "properties": {
                "operationId": { "type": "string", "minLength": 1 },
                "state": { "const": "completed" },
                "busId": { "type": "string", "minLength": 1 },
                "driverInstanceId": { "type": "string", "minLength": 1 },
                "availability": { "type": "object", "properties": { "status": { "const": "unavailable" }, "reason": { "type": "string", "minLength": 1 } }, "required": ["status", "reason"], "additionalProperties": false }
            },
            "required": ["operationId", "state", "busId", "driverInstanceId", "availability"],
            "additionalProperties": false
        }),
        "virtualDevices.remove" => json!({
            "type": "object",
            "properties": {
                "operationId": { "type": "string", "minLength": 1 },
                "state": { "const": "completed" },
                "busId": { "type": "string", "minLength": 1 },
                "driverInstanceId": { "const": null },
                "availability": { "type": "object", "properties": { "status": { "const": "unavailable" }, "reason": { "type": "string", "minLength": 1 } }, "required": ["status", "reason"], "additionalProperties": false }
            },
            "required": ["operationId", "state", "busId", "driverInstanceId", "availability"],
            "additionalProperties": false
        }),
        "virtualRoutes.list" => json!({
            "type": "object",
            "properties": {
                "revision": { "type": "integer", "minimum": 0 },
                "routes": { "type": "array", "maxItems": audiorouter_domain::MAX_VIRTUAL_BUS_ROUTES, "items": virtual_bus_route_schema() }
            },
            "required": ["revision", "routes"],
            "additionalProperties": false
        }),
        "virtualRoutes.replace" => json!({
            "type": "object",
            "properties": {
                "state": { "type": "string", "const": "applied" },
                "revision": { "type": "integer", "minimum": 1 },
                "routes": { "type": "array", "maxItems": audiorouter_domain::MAX_VIRTUAL_BUS_ROUTES, "items": virtual_bus_route_schema() }
            },
            "required": ["state", "revision", "routes"],
            "additionalProperties": false
        }),
        "nodes.types" | "nodes.describe" => json!({
            "type": "array",
            "maxItems": audiorouter_domain::node_registry().len(),
            "items": node_type_item_schema()
        }),
        "presets.list" => json!({
            "type": "object",
            "properties": {
                "voiceChains": {
                    "type": "array",
                    "maxItems": audiorouter_dsp::VoiceChainPresetId::ALL.len(),
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "minLength": 1 },
                            "version": { "const": 1 },
                            "name": { "type": "string", "minLength": 1 },
                            "description": { "type": "string", "minLength": 1 }
                        },
                        "required": ["id", "version", "name", "description"],
                        "additionalProperties": false
                    }
                },
                "eq": {
                    "type": "array",
                    "maxItems": audiorouter_dsp::EqPresetId::ALL.len(),
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "minLength": 1 },
                            "version": { "const": 1 },
                            "name": { "type": "string", "minLength": 1 },
                            "description": { "type": "string", "minLength": 1 }
                        },
                        "required": ["id", "version", "name", "description"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["voiceChains", "eq"],
            "additionalProperties": false
        }),
        "processors.list" => json!({
            "type": "array",
            "maxItems": MAX_PROCESSOR_CATALOG_ITEMS,
            "items": processor_item_schema()
        }),
        "processors.response" => json!({
            "type": "object",
            "properties": {
                "frequenciesHz": { "type": "array", "minItems": 1, "maxItems": MAX_RESPONSE_FREQUENCIES, "items": { "type": "number" } },
                "magnitudeDb": { "type": "array", "minItems": 1, "maxItems": MAX_RESPONSE_FREQUENCIES, "items": { "type": "number" } }
            }, "required": ["frequenciesHz", "magnitudeDb"], "additionalProperties": false
        }),
        "clients.list" => json!({
            "type": "array",
            "maxItems": audiorouter_storage::MAX_CLIENT_ENROLLMENTS,
            "items": {
                "type": "object",
                "properties": {
                    "clientId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                    "role": { "enum": ["observer", "editor", "operator"] },
                    "revoked": { "type": "boolean" }
                },
                "required": ["clientId", "role", "revoked"],
                "additionalProperties": false
            }
        }),
        "clients.authorize" => json!({
            "type": "object",
            "properties": {
                "clientId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "role": { "enum": ["observer", "editor", "operator"] },
                "revoked": { "const": false }
            },
            "required": ["clientId", "role", "revoked"],
            "additionalProperties": false
        }),
        "clients.revoke" => json!({
            "type": "object",
            "properties": {
                "clientId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "revoked": { "const": true },
                "changed": { "type": "boolean" }
            },
            "required": ["clientId", "revoked", "changed"],
            "additionalProperties": false
        }),
        "recordings.list" => {
            let item = recording_item_schema();
            json!({
                "oneOf": [
                    { "type": "array", "maxItems": MAX_RECORDING_LIST_ITEMS, "items": item.clone() },
                    {
                        "type": "object",
                        "properties": {
                            "items": { "type": "array", "maxItems": MAX_RECORDING_LIST_ITEMS, "items": item },
                            "nextCursor": { "type": ["string", "null"] }
                        },
                        "required": ["items", "nextCursor"],
                        "additionalProperties": false
                    }
                ]
            })
        }
        "audioMedia.beginUpload" => json!({
            "type": "object", "properties": {
                "uploadId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "chunkBytes": { "const": AUDIO_UPLOAD_CHUNK_BYTES }
            }, "required": ["uploadId", "chunkBytes"], "additionalProperties": false
        }),
        "audioMedia.uploadChunk" => json!({
            "type": "object", "properties": {
                "receivedBytes": { "type": "integer", "minimum": 0, "maximum": audiorouter_storage::MAX_AUDIO_MEDIA_BYTES },
                "nextChunkIndex": { "type": "integer", "minimum": 0, "maximum": 1024 }
            }, "required": ["receivedBytes", "nextChunkIndex"], "additionalProperties": false
        }),
        "audioMedia.finishUpload" => json!({
            "type": "object", "properties": {
                "mediaId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "fileName": { "type": "string", "minLength": 1, "maxLength": 256 },
                "format": { "enum": ["wav", "mp3"] },
                "durationMs": { "type": "integer", "minimum": 1, "maximum": 120000 },
                "channels": { "type": "integer", "enum": [1, 2] },
                "sampleRateHz": { "type": "integer", "minimum": 8000, "maximum": 192000 }
            }, "required": ["mediaId", "fileName", "format", "durationMs", "channels", "sampleRateHz"], "additionalProperties": false
        }),
        "audioMedia.importTemporaryRecording" => json!({
            "type": "object", "properties": {
                "mediaId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "fileName": { "const": "Temporary voice take.wav" },
                "format": { "const": "wav" },
                "durationMs": { "type": "integer", "minimum": 1, "maximum": 120000 },
                "channels": { "type": "integer", "enum": [1, 2] },
                "sampleRateHz": { "type": "integer", "minimum": 8000, "maximum": 192000 },
                "expiresAt": { "type": "integer", "minimum": 0 },
                "sourceRemoved": { "const": true }
            }, "required": ["mediaId", "fileName", "format", "durationMs", "channels", "sampleRateHz", "expiresAt", "sourceRemoved"], "additionalProperties": false
        }),
        "audioMedia.delete" => json!({
            "type": "object", "properties": { "deleted": { "type": "boolean" } },
            "required": ["deleted"], "additionalProperties": false
        }),
        "timeShift.transport" => json!({
            "type": "object", "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "enum": ["live", "delayed", "paused"] },
                "delaySeconds": { "type": "number", "minimum": 0 },
                "bufferedSeconds": { "type": "number", "minimum": 0 },
                "capacitySeconds": { "type": "number", "minimum": 0 }
            }, "required": ["sessionId", "nodeId", "state", "delaySeconds", "bufferedSeconds", "capacitySeconds"], "additionalProperties": false
        }),
        "meters.reset" => {
            json!({"type":"object", "properties":{"sessionId":{"type":"string"},"nodeId":{"type":"string"},"reset":{"type":"boolean"}},"required":["sessionId","nodeId","reset"],"additionalProperties":false})
        }
        "audioSources.transport" => json!({
            "type": "object", "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "enum": ["playing", "paused", "stopped"] },
                "loop": { "type": "boolean" }
            }, "required": ["sessionId", "nodeId", "state", "loop"], "additionalProperties": false
        }),
        "recordings.get" => recording_item_schema(),
        "recordings.recovery" => json!({
            "oneOf": [{
                "type": "object",
                "properties": {
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "status": { "enum": ["missing", "available"] },
                "checkpoint": recorder_checkpoint_schema()
                },
                "required": ["recordingId", "status"],
                "additionalProperties": false
            }, {
                "type": "object",
                "properties": {
                    "items": {
                        "type": "array",
                        "maxItems": MAX_RECORDING_LIST_ITEMS,
                        "items": {
                            "type": "object",
                            "properties": {
                                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                                "status": { "enum": ["missing", "available", "invalid"] },
                                "checkpoint": recorder_checkpoint_schema()
                            },
                            "required": ["recordingId", "status"],
                            "additionalProperties": false
                        }
                    },
                    "nextCursor": { "type": ["string", "null"], "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES }
                },
                "required": ["items", "nextCursor"],
                "additionalProperties": false
            }]
        }),
        "recordings.preview" => json!({
            "type": "object",
            "properties": {
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "preview": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "status": { "const": "present" },
                                "format": { "const": "wav" },
                                "channels": { "type": "integer", "minimum": 1, "maximum": 2 },
                                "sampleRate": { "type": "integer", "minimum": 1 },
                                "frames": { "type": "integer", "minimum": 0 },
                                "dataBytes": { "type": "integer", "minimum": 0 },
                                "fileBytes": { "type": "integer", "minimum": 0 }
                            },
                            "required": ["status", "format", "channels", "sampleRate", "frames", "dataBytes", "fileBytes"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "status": { "const": "present" },
                                "format": { "const": "flac" },
                                "channels": { "type": "integer", "minimum": 1, "maximum": 2 },
                                "sampleRate": { "type": "integer", "minimum": 1 },
                                "bitsPerSample": { "type": "integer", "minimum": 1 },
                                "frames": { "type": "integer", "minimum": 0 },
                                "fileBytes": { "type": "integer", "minimum": 0 }
                            },
                            "required": ["status", "format", "channels", "sampleRate", "bitsPerSample", "frames", "fileBytes"],
                            "additionalProperties": false
                        },
                        { "type": "object", "properties": { "status": { "enum": ["missing", "invalid"] } }, "required": ["status"], "additionalProperties": false }
                    ]
                }
            },
            "required": ["recordingId", "preview"],
            "additionalProperties": false
        }),
        "recordings.setMetadata" => json!({
            "type": "object",
            "properties": {
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "updated": { "const": true }
            },
            "required": ["recordingId", "updated"],
            "additionalProperties": false
        }),
        "recordings.rename" => json!({
            "type": "object",
            "properties": {
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "renamed": { "const": true },
                "path": { "type": "string", "minLength": 1 },
                "fileAction": { "const": "renamed" }
            },
            "required": ["recordingId", "renamed", "path", "fileAction"],
            "additionalProperties": false
        }),
        "recordings.removeEntry" => json!({
            "type": "object",
            "properties": {
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "removed": { "const": true },
                "fileAction": { "const": "none" }
            },
            "required": ["recordingId", "removed", "fileAction"],
            "additionalProperties": false
        }),
        "recordings.reveal" => json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "revealed": { "const": true }
                    },
                    "required": ["recordingId", "path", "revealed"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "revealed": { "const": false },
                        "reason": { "const": "missing" }
                    },
                    "required": ["recordingId", "path", "revealed", "reason"],
                    "additionalProperties": false
                }
            ]
        }),
        "recordings.recycle" => json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "fileAction": { "const": "none" },
                        "reason": { "const": "missing" }
                    },
                    "required": ["recordingId", "path", "fileAction", "reason"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "fileAction": { "const": "recycle" },
                        "preview": { "const": true }
                    },
                    "required": ["recordingId", "path", "fileAction", "preview"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "fileAction": { "const": "recycled" },
                        "missing": { "const": true }
                    },
                    "required": ["recordingId", "path", "fileAction", "missing"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "fileAction": { "const": "none" },
                        "reason": { "const": "recycleUnavailable" }
                    },
                    "required": ["recordingId", "path", "fileAction", "reason"],
                    "additionalProperties": false
                }
            ]
        }),
        _ => json!({ "type": "object" }),
    }
}

pub(crate) fn status_output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "build": { "type": "string" },
            "audio": { "enum": ["available", "unavailable"] },
            "deviceDiscovery": { "const": "available" },
            "reason": { "type": "string", "minLength": 1 },
            "storage": { "enum": ["memory", "sqlite"] },
            "sessionCount": { "type": "integer", "minimum": 0 },
            "activeSessionCount": { "type": "integer", "minimum": 0 },
            "activeSessionIds": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_ACTIVE_SESSIONS,
                "items": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES
                }
            },
            "privacyMute": {
                "type": "object",
                "properties": {
                    "muted": { "type": "boolean" },
                    "persistence": { "enum": ["durable", "memory"] },
                    "audioEffect": { "type": "string", "minLength": 1 }
                },
                "required": ["muted", "persistence", "audioEffect"],
                "additionalProperties": false
            },
            "recovery": {
                "type": "object",
                "properties": {
                    "safeMode": { "type": "boolean" },
                    "recentCrashes": { "type": "integer", "minimum": 0 },
                    "persistence": { "enum": ["durable", "memory"] }
                },
                "required": ["safeMode", "recentCrashes", "persistence"],
                "additionalProperties": false
            },
            "eventCursor": {
                "type": "object",
                "properties": {
                    "backendEpoch": { "type": "integer", "minimum": 0 },
                    "latestSequence": { "type": "integer", "minimum": 0 }
                },
                "required": ["backendEpoch", "latestSequence"],
                "additionalProperties": false
            }
        },
        "required": ["build", "audio", "deviceDiscovery", "reason", "storage", "sessionCount", "activeSessionCount", "activeSessionIds", "privacyMute", "recovery", "eventCursor"],
        "additionalProperties": false
    })
}

pub(crate) fn diagnostics_output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "build": { "type": "string" },
            "backend": { "const": "control-plane" },
            "storage": { "enum": ["memory", "sqlite"] },
            "audio": {
                "type": "object",
                "properties": {
                    "state": { "enum": ["available", "unavailable"] },
                    "reason": { "type": "string", "minLength": 1 }
                },
                "required": ["state", "reason"],
                "additionalProperties": false
            },
            "nativeAdapter": { "enum": ["implemented-not-activated", "configured-stopped", "running"] },
            "nativeAdapterKind": { "enum": ["endpoint", "process-loopback", "duplex", "render-source", "multi-input", null] },
            "applicationCaptureStates": {
                "type": "array",
                "maxItems": audiorouter_engine::MAX_MIXER_INPUTS,
                "items": {
                    "type": "object",
                    "properties": {
                        "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "state": { "enum": ["configured-stopped", "connected", "app-closed", "reconnecting", "ambiguous", "output-unavailable", "unsupported", "failed"] },
                        "detail": { "type": "string", "maxLength": 256 }
                    },
                    "required": ["sessionId", "nodeId", "state", "detail"],
                    "additionalProperties": false
                }
            },
            "nativeSessionId": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
            "schedulerTelemetry": {
                "oneOf": [
                    { "const": null },
                    {
                        "type": "object",
                        "properties": {
                            "activeGeneration": { "type": ["integer", "null"], "minimum": 0 },
                            "activeSampleRateHz": { "type": ["integer", "null"], "minimum": 1 },
                            "inputOverruns": { "type": "integer", "minimum": 0 },
                            "inputUnderruns": { "type": "integer", "minimum": 0 },
                            "outputOverruns": { "type": "integer", "minimum": 0 },
                            "outputUnderruns": { "type": "integer", "minimum": 0 },
                            "processedQuanta": { "type": "integer", "minimum": 0 },
                            "repairedSamples": { "type": "integer", "minimum": 0 },
                            "xruns": { "type": "integer", "minimum": 0 },
                            "processingTimeNsTotal": { "type": "integer", "minimum": 0 },
                            "processingTimeNsMax": { "type": "integer", "minimum": 0 },
                            "deadlineMisses": { "type": "integer", "minimum": 0 },
                            "deadlineLatenessNsTotal": { "type": "integer", "minimum": 0 },
                            "deadlineLatenessNsMax": { "type": "integer", "minimum": 0 }
                        },
                        "required": ["activeGeneration", "activeSampleRateHz", "inputOverruns", "inputUnderruns", "outputOverruns", "outputUnderruns", "processedQuanta", "repairedSamples", "xruns", "processingTimeNsTotal", "processingTimeNsMax", "deadlineMisses", "deadlineLatenessNsTotal", "deadlineLatenessNsMax"],
                        "additionalProperties": false
                    }
                ]
            },
            "nodeTelemetry": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_NODES_PER_SESSION,
                "items": {
                    "type": "object",
                    "properties": {
                        "nodeId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "kind": { "type": "string", "minLength": 1 },
                        "meter": {
                            "type": ["object", "null"],
                            "properties": {
                                "peakDb": { "type": "number" },
                                "rmsDb": { "type": "number" },
                                "clippedSamples": { "type": "integer", "minimum": 0 },
                                "currentPeakDb": {"type":"number"},
                                "channelCurrentPeakDb": {"type":"array", "maxItems":2, "items":{"type":"number"}},
                                "observedFrames": {"type":"integer", "minimum":0},
                                "sampleRateHz": {"type":"integer", "minimum":1},
                                "channelPeakDb": { "type": "array", "maxItems": 2, "items": { "type": "number" } },
                                "channelRmsDb": { "type": "array", "maxItems": 2, "items": { "type": "number" } },
                                "channelClippedSamples": { "type": "array", "maxItems": 2, "items": { "type": "integer", "minimum": 0 } }
                            },
                            "required": ["peakDb", "rmsDb", "clippedSamples", "channelPeakDb", "channelRmsDb", "channelClippedSamples"],
                            "additionalProperties": false
                        },
                        "processor": {
                            "type": ["object", "null"],
                            "properties": {
                                "gainReductionDb": { "type": "array", "maxItems": 2, "items": { "type": "number", "minimum": 0 } },
                                "gateOpen": { "type": "array", "maxItems": 2, "items": { "type": "boolean" } },
                                "inputLevelDb": { "type": "array", "maxItems": 2, "items": { "type": "number" } },
                                "outputLevelDb": { "type": "array", "maxItems": 2, "items": { "type": "number" } }
                            },
                            "required": ["gainReductionDb", "gateOpen"],
                            "additionalProperties": false
                        },
                        "plugin": {
                            "type": ["object", "null"],
                            "properties": {
                                "state": { "enum": ["unknown", "stopped", "running", "failed", "quarantined"] },
                                "failureCount": { "type": "integer", "minimum": 0 },
                                "outputMisses": { "type": "integer", "minimum": 0 },
                                "inputDrops": { "type": "integer", "minimum": 0 }
                            },
                            "required": ["state", "failureCount"],
                            "additionalProperties": false
                        },
                        "noiseProfile": { "type": "string", "pattern": "^[0-9a-fA-F]{128}$" },
                        "spectrum": spectrum_telemetry_schema(),
                        "network": {
                            "type": "object",
                            "properties": {
                                "direction": { "enum": ["send", "receive"] },
                                "sentPackets": { "type": "integer", "minimum": 0 },
                                "droppedPackets": { "type": "integer", "minimum": 0 },
                                "sendErrors": { "type": "integer", "minimum": 0 },
                                "receivedPackets": { "type": "integer", "minimum": 0 },
                                "lostPackets": { "type": "integer", "minimum": 0 },
                                "latePackets": { "type": "integer", "minimum": 0 },
                                "rejectedDatagrams": { "type": "integer", "minimum": 0 },
                                "underruns": { "type": "integer", "minimum": 0 },
                                "overflowPackets": { "type": "integer", "minimum": 0 },
                                "bufferedMs": { "type": "number", "minimum": 0 },
                                "rejectedFrom": { "type": "string", "maxLength": 64 },
                                "thisAddress": { "type": "string", "maxLength": 64 },
                                "localAddress": { "type": "string", "maxLength": 64 },
                                "lastErrorCode": { "type": "integer" },
                                "paired": { "type": "boolean" },
                                "authFailures": { "type": "integer", "minimum": 0 },
                                "authProblem": { "enum": ["wrongKey", "senderNotPaired", "receiverNotPaired"] },
                                "replayedPackets": { "type": "integer", "minimum": 0 }
                            },
                            "required": ["direction"],
                            "additionalProperties": false
                        },
                        "timing": {
                            "type": "object",
                            "properties": {
                                "delayMs": { "type": "number", "minimum": 0 },
                                "processingUsAvg": { "type": "number", "minimum": 0 },
                                "processingUsMax": { "type": "number", "minimum": 0 }
                            },
                            "required": ["delayMs"],
                            "additionalProperties": false
                        }
                    },
                    "required": ["nodeId", "kind", "meter", "processor", "plugin"],
                    "additionalProperties": false
                }
            },
            "privacyMute": {
                "type": "object",
                "properties": {
                    "muted": { "type": "boolean" },
                    "persistence": { "enum": ["durable", "memory"] }
                },
                "required": ["muted", "persistence"],
                "additionalProperties": false
            },
            "recovery": {
                "type": "object",
                "properties": {
                    "safeMode": { "type": "boolean" },
                    "recentCrashes": { "type": "integer", "minimum": 0 },
                    "persistence": { "enum": ["durable", "memory"] }
                },
                "required": ["safeMode", "recentCrashes", "persistence"],
                "additionalProperties": false
            },
            "eventLog": {
                "type": "object",
                "properties": {
                    "latestSequence": { "type": "integer", "minimum": 0 },
                    "retained": { "type": "integer", "minimum": 0 }
                },
                "required": ["latestSequence", "retained"],
                "additionalProperties": false
            },
            "gameRound": {
                "type": "object",
                "description": "Stats.cc feed for Ducks that follow the Siege round.",
                "properties": {
                    "source": { "const": "statsCc" },
                    "state": { "enum": ["off", "connecting", "waitingForUpdate", "connected", "unavailable"] },
                    "phase": { "enum": ["unknown", "menu", "prep", "betweenRounds", "action"] },
                    "feedConfigured": { "type": ["boolean", "null"] }
                },
                "required": ["source", "state", "phase", "feedConfigured"],
                "additionalProperties": false
            },
            "redacted": { "const": true }
        },
        "required": ["build", "backend", "storage", "audio", "nativeAdapter", "nativeAdapterKind", "nativeSessionId", "schedulerTelemetry", "nodeTelemetry", "applicationCaptureStates", "privacyMute", "recovery", "eventLog", "redacted"],
        "additionalProperties": false
    })
}

pub(crate) fn recording_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
            "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
            "recorderId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
            "nodeId": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
            "path": { "type": "string", "minLength": 1 },
            "format": { "enum": ["wav", "flac", "mp3"] },
            "channels": { "enum": [1, 2] },
            "sampleRate": { "enum": [44100, 48000] },
            "frames": { "type": "integer", "minimum": 0 },
            "fileBytes": { "type": "integer", "minimum": 0 },
            "startTime": { "type": "string", "minLength": 1 },
            "state": { "enum": ["armed", "recording", "paused", "completed", "failed"] },
            "missing": { "type": "boolean" },
            "title": { "type": ["string", "null"], "maxLength": 256 },
            "artist": { "type": ["string", "null"], "maxLength": 256 },
            "comment": { "type": ["string", "null"], "maxLength": 256 },
            "dither": { "type": "boolean" },
            "conversion": { "type": "string", "minLength": 1, "maxLength": 256 }
        },
        "required": [
            "id", "sessionId", "recorderId", "path", "format", "channels",
            "sampleRate", "frames", "fileBytes", "startTime", "state",
            "missing", "title", "artist", "comment", "dither", "conversion"
        ],
        "additionalProperties": false
    })
}

pub(crate) fn recorder_checkpoint_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "version": { "const": 1 },
            "state": { "enum": ["Idle", "Armed", "Recording", "Paused", "Stopping", "Completed", "Failed"] },
            "parts": {
                "type": "array",
                "maxItems": audiorouter_recording::MAX_CHECKPOINT_PARTS,
                "items": {
                    "type": "object",
                    "properties": {
                        "index": { "type": "integer", "minimum": 0 },
                        "start_frame": { "type": "integer", "minimum": 0 },
                        "end_frame": { "type": ["integer", "null"], "minimum": 0 }
                    },
                    "required": ["index", "start_frame", "end_frame"],
                    "additionalProperties": false
                }
            },
            "pauses": {
                "type": "array",
                "maxItems": audiorouter_recording::MAX_CHECKPOINT_PAUSES,
                "items": {
                    "type": "object",
                    "properties": {
                        "start_frame": { "type": "integer", "minimum": 0 },
                        "end_frame": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["start_frame", "end_frame"],
                    "additionalProperties": false
                }
            },
            "pause_start": { "type": ["integer", "null"], "minimum": 0 },
            "last_frame": { "type": ["integer", "null"], "minimum": 0 },
            "stop_frame": { "type": ["integer", "null"], "minimum": 0 }
        },
        "required": ["version", "state", "parts", "pauses", "pause_start", "last_frame", "stop_frame"],
        "additionalProperties": false
    })
}

pub(crate) fn device_item_schema() -> Value {
    json!({
        "oneOf": [active_device_item_schema(), inactive_device_item_schema()]
    })
}

pub(crate) fn active_device_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1 },
            "name": { "type": "string", "minLength": 1, "maxLength": 512 },
            "direction": { "enum": ["capture", "render"] },
            "state": { "const": "active" },
            "defaultRoles": {
                "type": "array",
                "items": { "enum": ["console", "multimedia", "communications"] },
                "uniqueItems": true,
                "maxItems": 3
            },
            "format": {
                "type": "object",
                "properties": {
                    "sampleRateHz": { "type": "integer", "minimum": 1 },
                    "channels": { "type": "integer", "minimum": 1 },
                    "bitsPerSample": { "type": "integer", "minimum": 1 },
                    "formatTag": { "type": "integer", "minimum": 0 },
                    "bytesPerFrame": { "type": "integer", "minimum": 1 }
                },
                "required": ["sampleRateHz", "channels", "bitsPerSample", "formatTag", "bytesPerFrame"],
                "additionalProperties": false
            },
            "periods": {
                "type": "object",
                "properties": {
                    "default100ns": { "type": "integer", "minimum": 0 },
                    "minimum100ns": { "type": "integer", "minimum": 0 }
                },
                "required": ["default100ns", "minimum100ns"],
                "additionalProperties": false
            }
        },
        "required": ["id", "name", "direction", "state", "defaultRoles", "format", "periods"],
        "additionalProperties": false
    })
}

pub(crate) fn inactive_device_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1 },
            "name": { "type": "string", "minLength": 1, "maxLength": 512 },
            "direction": { "enum": ["capture", "render"] },
            "state": { "enum": ["disabled", "unplugged", "notPresent", "unknown"] },
            "defaultRoles": {
                "type": "array",
                "items": { "enum": ["console", "multimedia", "communications"] },
                "uniqueItems": true,
                "maxItems": 3
            }
        },
        "required": ["id", "name", "direction", "state", "defaultRoles"],
        "additionalProperties": false
    })
}

pub(crate) fn virtual_device_operation_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": { "enum": ["create", "rename", "setEnabled", "delete"] },
            "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
            "name": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_VIRTUAL_BUS_NAME_CHARS },
            "enabled": { "type": "boolean" }
        },
        "required": ["action", "id"],
        "additionalProperties": false
    })
}

pub(crate) fn virtual_device_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
            "name": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_VIRTUAL_BUS_NAME_CHARS },
            "direction": { "const": "bidirectional" },
            "channels": { "const": 2 },
            "enabled": { "type": "boolean" },
            "availability": {
                "type": "object",
                "properties": {
                    "status": { "const": "unavailable" },
                    "reason": { "type": "string", "minLength": 1 }
                },
                "required": ["status", "reason"],
                "additionalProperties": false
            },
            "endpointIds": {
                "type": "object",
                "properties": {
                    "render": { "type": ["string", "null"] },
                    "capture": { "type": ["string", "null"] }
                },
                "required": ["render", "capture"],
                "additionalProperties": false
            },
            "driverInstanceId": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_VIRTUAL_BUS_DRIVER_INSTANCE_ID_CHARS },
            "capabilities": {
                "type": "object",
                "properties": {
                    "render": { "const": false },
                    "capture": { "const": false },
                    "channels": { "const": 2 }
                },
                "required": ["render", "capture", "channels"],
                "additionalProperties": false
            },
            "privilege": { "const": "deviceAdministration" },
            "restartRequired": { "const": false },
            "clientImpacts": { "type": "array", "items": { "type": "string" } },
            "leaseOwner": { "type": ["string", "null"] }
        },
        "required": ["id", "name", "driverInstanceId", "direction", "channels", "enabled", "availability", "endpointIds", "capabilities", "privilege", "restartRequired", "clientImpacts", "leaseOwner"],
        "additionalProperties": false
    })
}

pub(crate) fn virtual_bus_route_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "busId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
            "producerSessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
            "consumerSessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
        },
        "required": ["busId", "producerSessionId", "consumerSessionId"],
        "additionalProperties": false
    })
}

pub(crate) fn node_type_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "type": { "type": "string", "pattern": "^[a-z0-9-]+@[0-9]+$" },
            "availability": {
                "type": "object",
                "properties": {
                    "status": { "enum": ["available", "unavailable"] },
                    "reason": { "type": "string", "minLength": 1 }
                },
                "required": ["status"],
                "additionalProperties": false
            },
            "realtimeCostClass": { "type": "string", "minLength": 1 },
            "latencySamples": { "type": "integer", "minimum": 0 },
            "parameters": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_PARAMETERS_PER_NODE,
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "type": { "enum": ["boolean", "number"] },
                        "unit": { "type": "string", "minLength": 1 },
                        "minimum": { "type": "number" },
                        "maximum": { "type": "number" },
                        "default": {}
                    },
                    "required": ["name", "type", "default"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["type", "availability", "realtimeCostClass", "latencySamples", "parameters"],
        "additionalProperties": false
    })
}

pub(crate) fn processor_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1 },
            "version": { "type": "integer", "minimum": 1 },
            "category": { "type": "string", "minLength": 1 },
            "availability": {
                "type": "object",
                "properties": {
                    "status": { "enum": ["available", "unavailable"] },
                    "reason": { "type": "string", "minLength": 1 }
                },
                "required": ["status"],
                "additionalProperties": false
            },
            "latencySamples": { "type": "integer", "minimum": 0 },
            "parameters": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_PARAMETERS_PER_NODE,
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "type": { "enum": ["boolean", "number", "string"] },
                        "unit": { "type": "string", "minLength": 1 },
                        "minimum": { "type": "number" },
                        "maximum": { "type": "number" },
                        "default": {}
                    },
                    "required": ["name", "type"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["id", "version", "category", "availability", "latencySamples", "parameters"],
        "additionalProperties": false
    })
}

pub(crate) fn session_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "UTF-8 byte limit is advertised in limits.maxEntityIdBytes when applicable." },
            "name": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_DISPLAY_NAME_BYTES, "description": "Maximum 256 UTF-8 bytes." },
            "schemaVersion": { "const": 1 },
            "revision": { "type": "integer", "minimum": 0 },
            "nodes": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_NODES_PER_SESSION,
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "kind": { "type": "string", "minLength": 1 },
                        "typeVersion": { "const": 1 },
                        "name": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_DISPLAY_NAME_BYTES, "description": "Maximum 256 UTF-8 bytes." },
                        "enabled": { "type": "boolean" },
                        "bypass": { "type": "boolean" },
                        "parameters": {
                            "type": "object",
                            "maxProperties": audiorouter_domain::MAX_PARAMETERS_PER_NODE,
                            "propertyNames": {
                                "maxLength": audiorouter_domain::MAX_PARAMETER_NAME_BYTES,
                                "description": "Maximum 128 UTF-8 bytes per parameter name."
                            }
                        },
                        "ports": {
                            "type": "array",
                                "maxItems": audiorouter_domain::MAX_PORTS_PER_NODE,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_PORT_NAME_BYTES, "description": "Maximum 128 UTF-8 bytes." },
                                    "direction": { "enum": ["input", "output"] },
                                    "channels": { "type": "integer", "minimum": 1, "maximum": 2 }
                                },
                                "required": ["name", "direction", "channels"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["id", "kind", "typeVersion", "name", "enabled", "bypass", "parameters", "ports"],
                    "additionalProperties": false
                }
            },
            "edges": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_EDGES_PER_SESSION,
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "sourceNode": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "sourcePort": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_PORT_NAME_BYTES },
                        "destinationNode": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "destinationPort": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_PORT_NAME_BYTES },
                        "matrix": {
                            "type": "array",
                            "maxItems": audiorouter_domain::MAX_CHANNEL_MATRIX_COEFFICIENTS,
                            "items": { "type": "number", "minimum": -2.0, "maximum": 2.0 }
                        },
                        "enabled": { "type": "boolean" }
                    },
                    "required": ["id", "sourceNode", "sourcePort", "destinationNode", "destinationPort", "matrix", "enabled"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["id", "name", "schemaVersion", "revision", "nodes", "edges"],
        "additionalProperties": false
    })
}
