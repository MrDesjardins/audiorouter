//! Tests for `catalog.rs`.

use super::*;

#[test]
fn describe_exposes_versions_methods_limits_and_unavailable_nodes() {
    let plane = ControlPlane::new("test-build");
    let description = plane.describe();
    assert_eq!(description["build"], "test-build");
    assert_eq!(description["protocolVersion"]["major"], 1);
    let describe_method = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "system.describe")
        .unwrap();
    for field in describe_method["outputSchema"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
    {
        assert!(
            description.get(field).is_some(),
            "system.describe required field {field} missing from response"
        );
    }
    for field in describe_method["outputSchema"]["properties"]["limits"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
    {
        assert!(
            description["limits"].get(field).is_some(),
            "system.describe limits field {field} missing from response"
        );
    }
    for field in describe_method["outputSchema"]["properties"]["events"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
    {
        assert!(
            description["events"].get(field).is_some(),
            "system.describe events field {field} missing from response"
        );
    }
    assert_eq!(description["limits"]["maxNodesPerSession"], 64);
    assert_eq!(description["limits"]["maxNodesGlobal"], 128);
    assert_eq!(description["limits"]["maxEdgesGlobal"], 256);
    assert_eq!(
        description["limits"]["maxSessionsGlobal"],
        audiorouter_domain::MAX_SESSIONS_GLOBAL
    );
    assert_eq!(description["limits"]["maxActiveSessions"], 2);
    assert_eq!(
        description["limits"]["maxClientEnrollments"],
        audiorouter_storage::MAX_CLIENT_ENROLLMENTS
    );
    assert_eq!(
        description["limits"]["maxOperationJournalEntries"],
        audiorouter_storage::MAX_OPERATION_JOURNAL_ENTRIES
    );
    assert_eq!(description["limits"]["maxVirtualBuses"], 8);
    assert_eq!(
        description["limits"]["maxVirtualBusNameChars"],
        audiorouter_domain::MAX_VIRTUAL_BUS_NAME_CHARS
    );
    assert_eq!(
        description["limits"]["maxEntityIdBytes"],
        audiorouter_domain::MAX_ENTITY_ID_BYTES
    );
    assert_eq!(
        description["limits"]["maxDisplayNameBytes"],
        audiorouter_domain::MAX_DISPLAY_NAME_BYTES
    );
    assert_eq!(
        description["limits"]["maxPortNameBytes"],
        audiorouter_domain::MAX_PORT_NAME_BYTES
    );
    assert_eq!(
        description["limits"]["maxPortsPerNode"],
        audiorouter_domain::MAX_PORTS_PER_NODE
    );
    assert_eq!(
        description["limits"]["maxChannelMatrixCoefficients"],
        audiorouter_domain::MAX_CHANNEL_MATRIX_COEFFICIENTS
    );
    assert_eq!(
        description["limits"]["maxControlValueDepth"],
        MAX_CONTROL_VALUE_DEPTH
    );
    assert_eq!(
        description["limits"]["maxControlStringBytes"],
        MAX_CONTROL_STRING_BYTES
    );
    assert_eq!(
        description["limits"]["maxControlValueCount"],
        MAX_CONTROL_VALUE_COUNT
    );
    assert_eq!(
        description["limits"]["maxMethodNameBytes"],
        MAX_METHOD_NAME_BYTES
    );
    assert_eq!(
        description["limits"]["maxRequestIdBytes"],
        MAX_REQUEST_ID_BYTES
    );
    assert_eq!(
        description["limits"]["maxRevisionCursorBytes"],
        MAX_REVISION_CURSOR_BYTES
    );
    let create = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "sessions.create")
        .unwrap();
    let session_schema = &create["outputSchema"]["properties"]["session"];
    assert_eq!(session_schema["properties"]["name"]["maxLength"], 256);
    assert_eq!(session_schema["properties"]["nodes"]["maxItems"], 64);
    assert_eq!(session_schema["properties"]["edges"]["maxItems"], 128);
    let node_schema = &session_schema["properties"]["nodes"]["items"];
    assert_eq!(node_schema["properties"]["ports"]["maxItems"], 16);
    assert_eq!(
        node_schema["properties"]["parameters"]["maxProperties"],
        audiorouter_domain::MAX_PARAMETERS_PER_NODE
    );
    assert_eq!(
        node_schema["properties"]["parameters"]["propertyNames"]["maxLength"],
        128
    );
    assert_eq!(
        node_schema["properties"]["ports"]["items"]["properties"]["channels"]["maximum"],
        2
    );
    let edge_schema = &session_schema["properties"]["edges"]["items"];
    assert_eq!(edge_schema["properties"]["matrix"]["maxItems"], 4);
    assert_eq!(
        edge_schema["properties"]["matrix"]["items"]["minimum"],
        -2.0
    );
    assert_eq!(edge_schema["properties"]["matrix"]["items"]["maximum"], 2.0);
    let create_input = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "sessions.create")
        .unwrap();
    assert_eq!(
        create_input["inputSchema"]["properties"]["session"]["properties"]["nodes"]["maxItems"],
        64
    );
    let import_input = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "sessions.importPlan")
        .unwrap();
    assert_eq!(
        import_input["inputSchema"]["properties"]["session"]["properties"]["edges"]["maxItems"],
        128
    );
    let graph_plan_input = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "graph.plan")
        .unwrap();
    assert_eq!(
        graph_plan_input["inputSchema"]["properties"]["candidate"]["properties"]["nodes"]
            ["maxItems"],
        64
    );
    let graph_plan = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "graph.plan")
        .unwrap();
    assert_eq!(
        graph_plan["outputSchema"]["properties"]["diff"]["maxItems"],
        MAX_GRAPH_DIFF_ITEMS
    );
    assert_eq!(
        graph_plan["outputSchema"]["properties"]["affectedDestinations"]["maxItems"],
        MAX_GRAPH_AFFECTED_DESTINATIONS
    );
    assert_eq!(
        graph_plan["outputSchema"]["properties"]["affectedDestinations"]["items"]["maxLength"],
        audiorouter_domain::MAX_DISPLAY_NAME_BYTES
    );
    assert_eq!(
        graph_plan["outputSchema"]["properties"]["requiredScopes"]["maxItems"],
        MAX_PLAN_REQUIRED_SCOPES
    );
    assert_eq!(
        graph_plan["outputSchema"]["properties"]["warnings"]["maxItems"],
        MAX_PLAN_WARNINGS
    );
    assert_eq!(description["events"]["retention"]["maxEvents"], 10_000);
    assert_eq!(description["events"]["retention"]["maxAgeSeconds"], 900);
    assert_eq!(description["events"]["meterReplay"], false);
    let events_method = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "events.subscribe")
        .unwrap();
    assert_eq!(
        events_method["outputSchema"]["properties"]["events"]["maxItems"],
        MAX_EVENT_SUBSCRIPTION_ITEMS
    );
    assert_eq!(
        events_method["outputSchema"]["properties"]["events"]["items"]["properties"]["category"]
            ["maxLength"],
        audiorouter_domain::MAX_EVENT_CATEGORY_BYTES
    );
    assert_eq!(
        events_method["inputSchema"]["properties"]["limit"]["maximum"],
        MAX_EVENT_SUBSCRIPTION_ITEMS
    );
    let recovery = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "recordings.recovery")
        .unwrap();
    assert_eq!(
        recovery["outputSchema"]["oneOf"][0]["properties"]["checkpoint"]["properties"]["parts"]
            ["maxItems"],
        audiorouter_recording::MAX_CHECKPOINT_PARTS
    );
    assert_eq!(
        recovery["outputSchema"]["oneOf"][0]["properties"]["checkpoint"]["properties"]["pauses"]
            ["maxItems"],
        audiorouter_recording::MAX_CHECKPOINT_PAUSES
    );
    assert_eq!(
        recovery["outputSchema"]["oneOf"][0]["properties"]["checkpoint"]["properties"]
            ["stop_frame"]["type"],
        json!(["integer", "null"])
    );
    assert_eq!(
        recovery["outputSchema"]["oneOf"][1]["properties"]["items"]["items"]["properties"]
            ["checkpoint"]["properties"]["parts"]["maxItems"],
        audiorouter_recording::MAX_CHECKPOINT_PARTS
    );
    let recorder_transition = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "recorders.start")
        .unwrap();
    assert_eq!(
        recorder_transition["outputSchema"]["properties"]["parts"]["maxItems"],
        audiorouter_recording::MAX_CHECKPOINT_PARTS
    );
    assert_eq!(
        recorder_transition["outputSchema"]["properties"]["pauses"]["maxItems"],
        audiorouter_recording::MAX_CHECKPOINT_PAUSES
    );
    let virtual_devices = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "virtualDevices.list")
        .unwrap();
    assert_eq!(
        virtual_devices["outputSchema"]["oneOf"][1]["properties"]["items"]["maxItems"],
        MAX_VIRTUAL_DEVICE_LIST_ITEMS
    );
    assert_eq!(
        virtual_devices["inputSchema"]["properties"]["limit"]["maximum"],
        MAX_VIRTUAL_DEVICE_LIST_ITEMS
    );
    assert_eq!(
        virtual_devices["outputSchema"]["oneOf"][0]["items"]["properties"]["id"]["maxLength"],
        audiorouter_domain::MAX_ENTITY_ID_BYTES
    );
    let virtual_device_plan = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "virtualDevices.plan")
        .unwrap();
    assert_eq!(
        virtual_device_plan["inputSchema"]["properties"]["operation"]["properties"]["id"]
            ["maxLength"],
        audiorouter_domain::MAX_ENTITY_ID_BYTES
    );
    assert_eq!(
        virtual_device_plan["outputSchema"]["properties"]["operation"]["properties"]["id"]
            ["maxLength"],
        audiorouter_domain::MAX_ENTITY_ID_BYTES
    );
    let virtual_device_apply = description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "virtualDevices.apply")
        .unwrap();
    assert_eq!(
        virtual_device_apply["outputSchema"]["properties"]["operation"]["properties"]["name"]
            ["maxLength"],
        audiorouter_domain::MAX_VIRTUAL_BUS_NAME_CHARS
    );
    assert!(description["events"]["stateCategories"]
        .as_array()
        .unwrap()
        .iter()
        .any(|category| category == "graph.committed"));
    assert_eq!(
        description["events"]["stateCategories"]
            .as_array()
            .unwrap()
            .len(),
        STATE_CATEGORIES.len()
    );
    let describe_schema = method_output_schema("system.describe");
    assert_eq!(
        describe_schema["properties"]["events"]["properties"]["stateCategories"]["maxItems"],
        STATE_CATEGORIES.len()
    );
    let events_subscribe_schema = method_input_schema("events.subscribe");
    assert_eq!(
        events_subscribe_schema["properties"]["categories"]["items"]["enum"],
        json!(STATE_CATEGORIES)
    );
    assert!(description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|method| method["name"] == "graph.plan"));
    assert!(description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|method| method["name"] == "nodes.describe"));
    assert!(description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|method| method["name"] == "sessions.get"));
    assert!(description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|method| method["name"] == "applications.list"));
    assert!(description["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|method| method["name"] == "system.diagnostics"));
    assert!(description["nodeTypes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["type"] == "physical-input@1"
            && node["availability"]["status"] == "available"));
    assert!(description["nodeTypes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["type"] == "application-capture@1"
            && node["availability"]["status"] == "available"));
    assert!(description["nodeTypes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["type"] == "endpoint-loopback@1"
            && node["availability"]["status"] == "available"));
    assert!(description["nodeTypes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["type"] == "virtual-render-source@1"
            && node["availability"]["status"] == "unavailable"));
    assert_eq!(description["processors"].as_array().unwrap().len(), 18);
    assert_eq!(
        description["processors"][0]["availability"]["status"],
        "available"
    );
    assert_eq!(
        description["processors"][0]["parameters"][0]["type"],
        "number"
    );
    assert_eq!(
        description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "processors.list")
            .unwrap()["outputSchema"]["items"]["properties"]["availability"]["properties"]
            ["status"]["enum"][1],
        "unavailable"
    );
    let pitch = description["processors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|processor| processor["id"] == "pitch")
        .unwrap();
    assert_eq!(pitch["latencySamples"], 1024);
    assert_eq!(pitch["availability"]["status"], "available");
    let parametric_eq = description["processors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|processor| processor["id"] == "parametricEq")
        .unwrap();
    assert_eq!(parametric_eq["parameters"][0]["default"], 1000.0);
    assert_eq!(parametric_eq["parameters"][1]["default"], 1.0);
    let compressor = description["processors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|processor| processor["id"] == "compressor")
        .unwrap();
    assert_eq!(compressor["parameters"][4]["name"], "kneeDb");
    assert_eq!(compressor["parameters"][4]["default"], 6.0);
    let gate = description["processors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|processor| processor["id"] == "gate")
        .unwrap();
    assert_eq!(gate["parameters"][2]["name"], "hysteresisDb");
    assert_eq!(gate["parameters"][5]["name"], "holdMs");
    assert_eq!(gate["parameters"][5]["default"], 50.0);
    let gain = description["nodeTypes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["type"] == "gain@1")
        .unwrap();
    assert_eq!(gain["parameters"][0]["name"], "gainDb");
    assert_eq!(gain["parameters"][0]["minimum"], -60.0);
    assert_eq!(gain["parameters"][0]["maximum"], 24.0);
    assert_eq!(
        description["presets"]["voiceChains"][0]["id"],
        "voiceNeutral"
    );
    assert_eq!(description["presets"]["voiceChains"][0]["version"], 1);
    assert_eq!(
        description["presets"]["voiceChains"][1]["name"],
        "Voice gate and compression"
    );
    assert!(description["presets"]["voiceChains"][0]["description"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(description["presets"]["eq"].as_array().unwrap().len(), 3);
    assert_eq!(description["presets"]["eq"][1]["id"], "hum50Hz");
    assert_eq!(description["presets"]["eq"][1]["version"], 1);
    let presets = ControlPlane::default().dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "presets.list".into(),
        params: None,
    });
    let result = presets.result.unwrap();
    assert_eq!(result["voiceChains"].as_array().unwrap().len(), 2);
    assert_eq!(result["eq"].as_array().unwrap().len(), 3);
    assert!(result["voiceChains"]
        .as_array()
        .unwrap()
        .iter()
        .all(|preset| preset["version"] == 1));
    let processors = ControlPlane::default().dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "processors.list".into(),
        params: None,
    });
    let result = processors.result.unwrap();
    assert_eq!(result.as_array().unwrap().len(), 18);
    assert_eq!(result[0]["availability"]["status"], "available");
}

#[test]
fn describe_exposes_input_and_output_schemas_for_methods() {
    let document = ControlPlane::default().describe();
    let methods = document["methods"].as_array().unwrap().clone();
    let commit = methods
        .iter()
        .find(|method| method["name"] == "graph.commit")
        .unwrap();
    assert_eq!(
        commit["description"],
        "Commit an unexpired graph plan with idempotent mutation."
    );
    assert_eq!(
        commit["inputSchema"]["required"],
        json!(["planId", "baseRevision", "idempotencyKey"])
    );
    for (method_name, field_name) in [
        ("graph.commit", "planId"),
        ("virtualDevices.apply", "planId"),
        ("startup.apply", "planId"),
        ("sessions.importCommit", "planId"),
        ("sessions.get", "sessionId"),
        ("sessions.export", "sessionId"),
        ("sessions.duplicate", "sourceSessionId"),
        ("routes.inspect", "destinationNode"),
        ("graph.plan", "sessionId"),
        ("graph.undoPlan", "sessionId"),
    ] {
        let method = methods
            .iter()
            .find(|method| method["name"] == method_name)
            .unwrap();
        assert_eq!(
            method["inputSchema"]["properties"][field_name]["maxLength"],
            audiorouter_domain::MAX_ENTITY_ID_BYTES,
            "{method_name} input {field_name} bound"
        );
    }
    for method_name in ["operations.get", "operations.cancel"] {
        let method = methods
            .iter()
            .find(|method| method["name"] == method_name)
            .unwrap();
        assert_eq!(
            method["inputSchema"]["properties"]["operationId"]["maxLength"],
            audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES,
            "{method_name} input operationId bound"
        );
    }
    for method_name in [
        "startup.plan",
        "startup.apply",
        "sessions.importPlan",
        "graph.undoPlan",
        "graph.plan",
    ] {
        let method = methods
            .iter()
            .find(|method| method["name"] == method_name)
            .unwrap();
        assert_eq!(
            method["outputSchema"]["properties"]["planId"]["maxLength"],
            audiorouter_domain::MAX_ENTITY_ID_BYTES,
            "{method_name} output planId bound"
        );
    }
    let events = methods
        .iter()
        .find(|method| method["name"] == "events.subscribe")
        .unwrap();
    assert_eq!(
        events["outputSchema"]["properties"]["snapshot"]["properties"]["sessions"]["properties"]
            ["nextCursor"]["maxLength"],
        audiorouter_domain::MAX_ENTITY_ID_BYTES,
        "events snapshot session cursor bound"
    );
    assert_eq!(commit["outputSchema"]["type"], "object");
    let devices = methods
        .iter()
        .find(|method| method["name"] == "devices.list")
        .unwrap();
    assert_eq!(devices["inputSchema"]["additionalProperties"], false);
    assert_eq!(
        devices["inputSchema"]["properties"]["limit"]["maximum"],
        500
    );
    assert_eq!(
        devices["outputSchema"]["oneOf"][1]["properties"]["items"]["items"]["oneOf"][0]
            ["properties"]["direction"]["enum"],
        json!(["capture", "render"])
    );
    assert_eq!(
        devices["outputSchema"]["oneOf"][0]["maxItems"],
        MAX_DEVICE_LIST_ITEMS
    );
    let virtual_devices = methods
        .iter()
        .find(|method| method["name"] == "virtualDevices.list")
        .unwrap();
    assert_eq!(
        virtual_devices["outputSchema"]["oneOf"][0]["maxItems"],
        audiorouter_domain::MAX_VIRTUAL_BUSES
    );
    assert_eq!(
        devices["outputSchema"]["oneOf"][1]["properties"]["items"]["items"]["oneOf"][0]
            ["properties"]["format"]["required"],
        json!([
            "sampleRateHz",
            "channels",
            "bitsPerSample",
            "formatTag",
            "bytesPerFrame"
        ])
    );
    let node_types = methods
        .iter()
        .find(|method| method["name"] == "nodes.types")
        .unwrap();
    assert_eq!(
        node_types["outputSchema"]["items"]["properties"]["type"]["type"],
        "string"
    );
    assert_eq!(
        node_types["outputSchema"]["items"]["properties"]["parameters"]["items"]["required"],
        json!(["name", "type", "default"])
    );
    let clients = methods
        .iter()
        .find(|method| method["name"] == "clients.list")
        .unwrap();
    assert_eq!(
        clients["outputSchema"]["items"]["properties"]["role"]["enum"],
        json!(["observer", "editor", "operator"])
    );
    let status = methods
        .iter()
        .find(|method| method["name"] == "status.get")
        .unwrap();
    assert_eq!(
        status["outputSchema"]["properties"]["audio"]["enum"],
        json!(["available", "unavailable"])
    );
    assert_eq!(
        status["outputSchema"]["properties"]["activeSessionIds"]["maxItems"],
        audiorouter_domain::MAX_ACTIVE_SESSIONS
    );
    assert_eq!(
        status["outputSchema"]["properties"]["activeSessionIds"]["items"]["maxLength"],
        audiorouter_domain::MAX_ENTITY_ID_BYTES
    );
    assert_eq!(
        status["outputSchema"]["properties"]["eventCursor"]["required"],
        json!(["backendEpoch", "latestSequence"])
    );
    let diagnostics = methods
        .iter()
        .find(|method| method["name"] == "system.diagnostics")
        .unwrap();
    assert_eq!(
        diagnostics["outputSchema"]["properties"]["redacted"]["const"],
        true
    );
    assert_eq!(
        diagnostics["outputSchema"]["properties"]["eventLog"]["required"],
        json!(["latestSequence", "retained"])
    );
    let startup = methods
        .iter()
        .find(|method| method["name"] == "startup.get")
        .unwrap();
    assert_eq!(
        startup["outputSchema"]["properties"]["registration"]["const"],
        "unavailable"
    );
    let event_categories = document["events"]["stateCategories"].as_array().unwrap();
    for category in [
        "devices.bindingInvalidated",
        "virtualDevice.changed",
        "recording.metadataChanged",
        "recording.renamed",
        "recording.entryRemoved",
        "recording.recycled",
    ] {
        assert!(event_categories.iter().any(|value| value == category));
    }
    let recovery_clear = methods
        .iter()
        .find(|method| method["name"] == "recovery.clearSafeMode")
        .unwrap();
    assert_eq!(
        recovery_clear["outputSchema"]["properties"]["safeMode"]["const"],
        false
    );
    let sessions = methods
        .iter()
        .find(|method| method["name"] == "sessions.list")
        .unwrap();
    assert_eq!(
        sessions["outputSchema"]["properties"]["nextCursor"]["type"],
        json!(["string", "null"])
    );
    assert_eq!(
        sessions["outputSchema"]["properties"]["items"]["items"]["properties"]["revision"]
            ["minimum"],
        0
    );
    assert_eq!(
        sessions["outputSchema"]["properties"]["items"]["maxItems"],
        MAX_SESSION_LIST_ITEMS
    );
    let history = methods
        .iter()
        .find(|method| method["name"] == "graph.history")
        .unwrap();
    assert_eq!(
        history["outputSchema"]["properties"]["items"]["maxItems"],
        MAX_GRAPH_HISTORY_ITEMS
    );
    assert_eq!(
        history["inputSchema"]["properties"]["cursor"]["maxLength"],
        MAX_REVISION_CURSOR_BYTES
    );
    assert_eq!(
        history["outputSchema"]["properties"]["nextCursor"]["maxLength"],
        MAX_REVISION_CURSOR_BYTES
    );
    let session_get = methods
        .iter()
        .find(|method| method["name"] == "sessions.get")
        .unwrap();
    assert_eq!(session_get["outputSchema"]["type"], "object");
    assert_eq!(
        session_get["outputSchema"]["properties"]["nodes"]["type"],
        "array"
    );
    assert_eq!(
        session_get["outputSchema"]["properties"]["edges"]["items"]["properties"]["sourceNode"]
            ["type"],
        "string"
    );
    let routes = methods
        .iter()
        .find(|method| method["name"] == "routes.inspect")
        .unwrap();
    assert_eq!(
        routes["outputSchema"]["properties"]["paths"]["maxItems"],
        audiorouter_domain::MAX_ROUTE_PATHS
    );
    let handshake = methods
        .iter()
        .find(|method| method["name"] == "system.handshake")
        .unwrap();
    assert_eq!(
        handshake["outputSchema"]["properties"]["negotiated"]["properties"]["major"]["const"],
        1
    );
    let describe = methods
        .iter()
        .find(|method| method["name"] == "system.describe")
        .unwrap();
    assert_eq!(
        describe["outputSchema"]["properties"]["methods"]["type"],
        "array"
    );
    assert_eq!(
        describe["outputSchema"]["properties"]["methods"]["maxItems"],
        API_METHODS.len()
    );
    assert_eq!(
        describe["outputSchema"]["properties"]["nodeTypes"]["maxItems"],
        audiorouter_domain::node_registry().len()
    );
    assert_eq!(
        describe["outputSchema"]["properties"]["processors"]["maxItems"],
        MAX_PROCESSOR_CATALOG_ITEMS
    );
    assert_eq!(
        describe["outputSchema"]["properties"]["presets"]["properties"]["voiceChains"]["maxItems"],
        audiorouter_dsp::VoiceChainPresetId::ALL.len()
    );
    assert_eq!(
        describe["outputSchema"]["properties"]["presets"]["properties"]["eq"]["maxItems"],
        audiorouter_dsp::EqPresetId::ALL.len()
    );
    assert_eq!(
        describe["outputSchema"]["properties"]["events"]["properties"]["meterReplay"]["const"],
        false
    );
    let session_create = methods
        .iter()
        .find(|method| method["name"] == "sessions.create")
        .unwrap();
    assert_eq!(
        session_create["outputSchema"]["properties"]["session"]["properties"]["nodes"]["type"],
        "array"
    );
    let session_start = methods
        .iter()
        .find(|method| method["name"] == "session.start")
        .unwrap();
    assert_eq!(
        session_start["outputSchema"]["properties"]["generation"]["minimum"],
        1
    );
    let undo = methods
        .iter()
        .find(|method| method["name"] == "graph.undoPlan")
        .unwrap();
    assert_eq!(
        undo["outputSchema"]["required"],
        json!(["planId", "baseRevision", "expiresInMs"])
    );
    let privacy = methods
        .iter()
        .find(|method| method["name"] == "safety.setPrivacyMute")
        .unwrap();
    assert_eq!(
        privacy["outputSchema"]["properties"]["muted"]["type"],
        "boolean"
    );
    let authorize = methods
        .iter()
        .find(|method| method["name"] == "clients.authorize")
        .unwrap();
    assert_eq!(
        authorize["outputSchema"]["properties"]["revoked"]["const"],
        false
    );
    let metadata = methods
        .iter()
        .find(|method| method["name"] == "recordings.setMetadata")
        .unwrap();
    assert_eq!(
        metadata["outputSchema"]["required"],
        json!(["recordingId", "updated"])
    );
    let rename = methods
        .iter()
        .find(|method| method["name"] == "recordings.rename")
        .unwrap();
    assert_eq!(
        rename["outputSchema"]["properties"]["fileAction"]["const"],
        "renamed"
    );
    let reveal = methods
        .iter()
        .find(|method| method["name"] == "recordings.reveal")
        .unwrap();
    assert_eq!(reveal["outputSchema"]["oneOf"].as_array().unwrap().len(), 2);
    let preview = methods
        .iter()
        .find(|method| method["name"] == "recordings.preview")
        .unwrap();
    assert_eq!(
        preview["outputSchema"]["properties"]["preview"]["oneOf"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let recycle = methods
        .iter()
        .find(|method| method["name"] == "recordings.recycle")
        .unwrap();
    assert_eq!(
        recycle["outputSchema"]["oneOf"].as_array().unwrap().len(),
        4
    );
    let history = methods
        .iter()
        .find(|method| method["name"] == "graph.history")
        .unwrap();
    assert_eq!(
        history["outputSchema"]["properties"]["items"]["type"],
        "array"
    );
    let routes = methods
        .iter()
        .find(|method| method["name"] == "routes.inspect")
        .unwrap();
    assert_eq!(
        routes["outputSchema"]["properties"]["reachable"]["type"],
        "boolean"
    );
    assert_eq!(
        routes["outputSchema"]["properties"]["paths"]["items"]["required"],
        json!(["nodes", "edges", "channelMaps", "latencySamples"])
    );
    assert_eq!(
        routes["outputSchema"]["properties"]["complete"]["type"],
        "boolean"
    );
    let plugins = methods
        .iter()
        .find(|method| method["name"] == "plugins.scan")
        .unwrap();
    assert_eq!(
        plugins["outputSchema"]["properties"]["entries"]["maxItems"],
        audiorouter_plugin_host::MAX_SCAN_CANDIDATES
    );
    let node_types = methods
        .iter()
        .find(|method| method["name"] == "nodes.types")
        .unwrap();
    assert_eq!(
        node_types["outputSchema"]["maxItems"],
        audiorouter_domain::node_registry().len()
    );
    let processors = methods
        .iter()
        .find(|method| method["name"] == "processors.list")
        .unwrap();
    assert_eq!(
        processors["outputSchema"]["maxItems"],
        MAX_PROCESSOR_CATALOG_ITEMS
    );
    let presets = methods
        .iter()
        .find(|method| method["name"] == "presets.list")
        .unwrap();
    assert_eq!(
        presets["outputSchema"]["properties"]["voiceChains"]["maxItems"],
        audiorouter_dsp::VoiceChainPresetId::ALL.len()
    );
    assert_eq!(
        presets["outputSchema"]["properties"]["eq"]["maxItems"],
        audiorouter_dsp::EqPresetId::ALL.len()
    );
    assert_eq!(
        routes["outputSchema"]["properties"]["paths"]["items"]["properties"]["nodes"]["maxItems"],
        audiorouter_domain::MAX_NODES_PER_SESSION
    );
    assert_eq!(
        routes["outputSchema"]["properties"]["paths"]["items"]["properties"]["edges"]["maxItems"],
        audiorouter_domain::MAX_EDGES_PER_SESSION
    );
    assert_eq!(
        routes["outputSchema"]["properties"]["paths"]["items"]["properties"]["channelMaps"]
            ["items"]["maxItems"],
        audiorouter_domain::MAX_CHANNEL_MATRIX_COEFFICIENTS
    );
    let plan = methods
        .iter()
        .find(|method| method["name"] == "graph.plan")
        .unwrap();
    assert_eq!(
        plan["outputSchema"]["properties"]["expiresInMs"]["minimum"],
        1
    );
    assert_eq!(
        plan["outputSchema"]["properties"]["warnings"]["type"],
        "array"
    );
    let commit = methods
        .iter()
        .find(|method| method["name"] == "graph.commit")
        .unwrap();
    for method_name in [
        "sessions.delete",
        "session.start",
        "sessions.start",
        "session.stop",
        "sessions.stop",
        "graph.commit",
    ] {
        let method = methods
            .iter()
            .find(|method| method["name"] == method_name)
            .unwrap();
        assert_eq!(
            method["outputSchema"]["properties"]["sessionId"]["maxLength"],
            audiorouter_domain::MAX_ENTITY_ID_BYTES,
            "{method_name} output sessionId bound"
        );
    }
    assert_eq!(
        commit["outputSchema"]["properties"]["revision"]["minimum"],
        0
    );
    let operation = methods
        .iter()
        .find(|method| method["name"] == "operations.get")
        .unwrap();
    assert_eq!(
        operation["outputSchema"]["oneOf"][1]["properties"]["status"]["const"],
        "unknown"
    );
    let cancel = methods
        .iter()
        .find(|method| method["name"] == "operations.cancel")
        .unwrap();
    assert_eq!(
        cancel["outputSchema"]["properties"]["reason"]["const"],
        "alreadyCompleted"
    );
    let applications = methods
        .iter()
        .find(|method| method["name"] == "applications.list")
        .unwrap();
    assert_eq!(
        applications["outputSchema"]["items"]["properties"]["audioActivity"]["enum"][0],
        "active"
    );
    assert_eq!(
        applications["outputSchema"]["items"]["properties"]["captureSessionCount"]["type"],
        "integer"
    );
    assert_eq!(
        applications["outputSchema"]["items"]["properties"]["renderSessionCount"]["type"],
        "integer"
    );
    assert_eq!(
        applications["outputSchema"]["items"]["properties"]["executable"]["maxLength"],
        260
    );
    assert_eq!(
        applications["outputSchema"]["items"]["properties"]["executablePath"]["maxLength"],
        32_768
    );
    assert_eq!(
        applications["outputSchema"]["maxItems"],
        audiorouter_windows_audio::MAX_APPLICATIONS
    );
    assert_eq!(
        applications["outputSchema"]["items"]["properties"]["audioDisplayNames"]["maxItems"],
        audiorouter_windows_audio::MAX_APPLICATION_AUDIO_DISPLAY_NAMES
    );
    assert_eq!(
        applications["outputSchema"]["items"]["properties"]["audioDisplayNames"]["items"]
            ["maxLength"],
        audiorouter_windows_audio::MAX_APPLICATION_AUDIO_DISPLAY_NAME_BYTES
    );
    let recordings = methods
        .iter()
        .find(|method| method["name"] == "recordings.list")
        .unwrap();
    assert_eq!(
        recordings["outputSchema"]["oneOf"][1]["properties"]["items"]["items"]["properties"]
            ["format"]["enum"],
        json!(["wav", "flac", "mp3"])
    );
    assert_eq!(
        recordings["outputSchema"]["oneOf"][1]["properties"]["nextCursor"]["type"],
        json!(["string", "null"])
    );
    assert_eq!(
        recordings["outputSchema"]["oneOf"][1]["properties"]["items"]["maxItems"],
        MAX_RECORDING_LIST_ITEMS
    );
    assert_eq!(
        recordings["outputSchema"]["oneOf"][0]["maxItems"],
        MAX_RECORDING_LIST_ITEMS
    );
    let recording = methods
        .iter()
        .find(|method| method["name"] == "recordings.get")
        .unwrap();
    assert_eq!(
        recording["outputSchema"]["properties"]["sampleRate"]["enum"],
        json!([44100, 48000])
    );
    for method_name in [
        "recordings.get",
        "recordings.recovery",
        "recordings.preview",
        "recordings.setMetadata",
        "recordings.rename",
        "recordings.reveal",
        "recordings.recycle",
        "recordings.removeEntry",
    ] {
        let method = methods
            .iter()
            .find(|method| method["name"] == method_name)
            .unwrap();
        let schema = &method["outputSchema"];
        let schema = if method_name == "recordings.get"
            || method_name == "recordings.preview"
            || method_name == "recordings.setMetadata"
            || method_name == "recordings.rename"
            || method_name == "recordings.removeEntry"
        {
            schema
        } else {
            &schema["oneOf"][0]
        };
        let field = if method_name == "recordings.get" {
            "id"
        } else {
            "recordingId"
        };
        assert_eq!(
            schema["properties"][field]["maxLength"],
            audiorouter_storage::MAX_RECORDING_ID_BYTES,
            "{method_name} output identity bound"
        );
    }
}

#[test]
fn canonical_application_list_alias_uses_the_same_discovery_result() {
    let mut plane = ControlPlane::default();
    let legacy = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(5)),
        method: "apps.list".into(),
        params: None,
    });
    let canonical = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(6)),
        method: "applications.list".into(),
        params: None,
    });
    assert_eq!(legacy.result, canonical.result);
    let applications = legacy.result.unwrap();
    assert!(applications.as_array().unwrap().iter().all(|application| {
        application.get("processId").is_some()
            && application.get("executable").is_some()
            && application.get("executablePath").is_some()
            && application.get("audioActivity").is_some()
            && application.get("captureCapability").is_some()
            && application.get("audioSessionCount").is_some()
            && application.get("activeAudioSessionCount").is_some()
            && application.get("captureSessionCount").is_some()
            && application.get("renderSessionCount").is_some()
            && application.get("audioDisplayNames").is_some()
    }));
}

#[test]
fn processors_response_uses_bounded_shared_eq_coefficients() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "processors.response".into(),
        params: Some(json!({
            "sampleRateHz": 48_000.0,
            "bands": [{"enabled": true, "type": "peaking", "frequencyHz": 1000.0, "q": 1.0, "gainDb": 6.0}],
            "frequenciesHz": [100.0, 1000.0, 10_000.0]
        })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["frequenciesHz"], json!([100.0, 1000.0, 10000.0]));
    assert_eq!(result["magnitudeDb"].as_array().unwrap().len(), 3);
    assert!(result["magnitudeDb"][1].as_f64().unwrap() > 5.0);
    for kind in ["bandPass", "allPass"] {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(), id: Some(json!(5)), method: "processors.response".into(),
            params: Some(json!({ "sampleRateHz": 48000.0,
                "bands": [{ "enabled": true, "type": kind, "frequencyHz": 1000.0, "q": 1.0, "gainDb": 24.0 }],
                "frequenciesHz": [100.0, 1000.0, 10000.0] })),
        });
        let result = response.result.unwrap();
        assert!(result["magnitudeDb"][1].as_f64().unwrap().abs() < 0.02);
        if kind == "bandPass" {
            assert!(result["magnitudeDb"][0].as_f64().unwrap() < -10.0);
            assert!(result["magnitudeDb"][2].as_f64().unwrap() < -10.0);
        } else {
            assert!(result["magnitudeDb"]
                .as_array()
                .unwrap()
                .iter()
                .all(|value| value.as_f64().unwrap().abs() < 0.02));
        }
    }
    let flat_band = json!({"enabled": false, "type": "peaking", "frequencyHz": 1000.0, "q": 1.0, "gainDb": 0.0});
    let bounded_bands = vec![flat_band.clone(); MAX_RESPONSE_BANDS];
    let maximum = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "processors.response".into(),
        params: Some(
            json!({"sampleRateHz": 48_000.0, "bands": bounded_bands, "frequenciesHz": [1000.0]}),
        ),
    });
    assert!(maximum.error.is_none());
    let too_many = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "processors.response".into(),
        params: Some(json!({"sampleRateHz": 48_000.0, "bands": vec![flat_band; MAX_RESPONSE_BANDS + 1], "frequenciesHz": [1000.0]})),
    });
    assert!(too_many.error.is_some());
    let invalid = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "processors.response".into(),
        params: Some(json!({"sampleRateHz": 48_000.0, "bands": [], "frequenciesHz": []})),
    });
    assert!(invalid.error.is_some());
}
