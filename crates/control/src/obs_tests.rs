//! Tests for `obs.rs`: the protocol helpers, the OBS file parsers, change
//! planning, and the client against a fake obs-websocket v5 server.

use super::*;
use std::collections::BTreeMap;
use std::net::TcpListener;
use std::sync::atomic::{AtomicU64, Ordering};
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;

const PASSWORD: &str = "test-only-password";
const MIC: &str = "{0.0.1.00000000}.{11111111-0000-0000-0000-000000000001}";
const CABLE_A_OUTPUT: &str = "{0.0.1.00000000}.{11111111-0000-0000-0000-000000000002}";
const CABLE_INPUT: &str = "{0.0.0.00000000}.{11111111-0000-0000-0000-000000000003}";

static NEXT_FOLDER: AtomicU64 = AtomicU64::new(0);

/// A fresh OBS settings folder, with obs-websocket's config when a password
/// is given.
fn settings_folder(password: Option<&str>) -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "audiorouter-obs-test-{}-{}",
        std::process::id(),
        NEXT_FOLDER.fetch_add(1, Ordering::Relaxed)
    ));
    let config = folder.join("plugin_config").join("obs-websocket");
    std::fs::create_dir_all(&config).unwrap();
    if let Some(password) = password {
        let settings = json!({
            "server_enabled": true,
            "server_port": 4455,
            "auth_required": true,
            "server_password": password,
        });
        std::fs::write(config.join("config.json"), settings.to_string()).unwrap();
    }
    folder
}

fn endpoint(id: &str, direction: EndpointDirection) -> EndpointInfo {
    EndpointInfo {
        id: id.into(),
        direction,
        default_period_100ns: 100_000,
        minimum_period_100ns: 30_000,
        sample_rate_hz: 48_000,
        channels: 2,
        bits_per_sample: 32,
        format_tag: 3,
        channel_mask: 3,
        subformat_guid: String::new(),
    }
}

fn input(name: &str, kind: AudioKind, device: Option<&str>, muted: Option<bool>) -> ObsAudioInput {
    ObsAudioInput {
        name: name.into(),
        uuid: None,
        kind,
        special: None,
        device_id: device.map(str::to_owned),
        application: None,
        muted,
        active: None,
        audio_tracks: None,
        monitor_only: None,
    }
}

#[test]
fn base64_matches_rfc_4648_vectors() {
    for (plain, encoded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(base64(plain.as_bytes()), encoded, "{plain:?}");
    }
}

#[test]
fn authentication_matches_the_obs_websocket_worked_example() {
    // The example in obs-websocket's protocol documentation.
    assert_eq!(
        authentication(
            "supersecretpassword",
            "lM1GncleQOaCu9lT1yeUZhFYnqhsLLP1G5lAGo3ixaI=",
            "+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeYcTNS3PDaeY="
        ),
        "1Ct943GAT+6YQUUX47Ia/ncufilbe6+oD6lY+5kaCu4="
    );
}

#[test]
fn websocket_settings_keep_only_bounded_values() {
    let settings = parse_websocket_settings(
        &json!({
            "alerts_enabled": false,
            "auth_required": true,
            "first_load": false,
            "server_enabled": true,
            "server_password": "abc123",
            "server_port": 4455
        })
        .to_string(),
    );
    assert_eq!(settings.port, Some(4455));
    assert_eq!(settings.auth_required, Some(true));
    assert_eq!(settings.password.as_deref(), Some("abc123"));

    let odd = parse_websocket_settings(
        &json!({
            "server_port": 80,
            "server_password": "x".repeat(MAX_OBS_PASSWORD_BYTES + 1)
        })
        .to_string(),
    );
    assert_eq!(odd.port, None, "a privileged or invalid port is ignored");
    assert!(odd.password.is_none(), "an oversized password is ignored");
    assert!(parse_websocket_settings("not json").password.is_none());
    assert!(parse_websocket_settings("[]").port.is_none());
}

#[test]
fn scene_collection_file_is_a_bare_name_from_the_basic_section() {
    let ini = "[General]\nSceneCollectionFile=wrong\n\n[Basic]\nProfile=Untitled\nSceneCollection=Streaming\nSceneCollectionFile=Streaming\n";
    assert_eq!(scene_collection_file(ini).as_deref(), Some("Streaming"));
    for unsafe_name in ["..\\..\\secret", "a/b", "C:x", "..", ""] {
        let ini = format!("[Basic]\nSceneCollectionFile={unsafe_name}\n");
        assert_eq!(scene_collection_file(&ini), None, "{unsafe_name:?}");
    }
    assert_eq!(scene_collection_file("[Basic]\nProfile=Untitled\n"), None);
}

#[test]
fn application_capture_window_names_its_executable() {
    assert_eq!(
        window_executable("Discord:Chrome_WidgetWin_1:Discord.exe").as_deref(),
        Some("Discord.exe")
    );
    assert_eq!(
        window_executable("Title#3A with colon:Class:game.exe").as_deref(),
        Some("game.exe")
    );
    assert_eq!(window_executable(""), None);
}

#[test]
fn a_saved_scene_collection_lists_its_audio_sources() {
    let document = json!({
        "name": "Streaming",
        "DesktopAudioDevice1": {
            "id": "wasapi_output_capture", "versioned_id": "wasapi_output_capture",
            "name": "Desktop Audio", "uuid": "u-desktop", "settings": {}, "muted": false
        },
        "AuxAudioDevice1": {
            "id": "wasapi_input_capture", "name": "Mic/Aux", "uuid": "u-mic",
            "settings": { "device_id": CABLE_A_OUTPUT }, "muted": true
        },
        "sources": [
            { "id": "wasapi_input_capture", "name": "Raw mic", "settings": { "device_id": MIC } },
            { "id": "wasapi_process_output_capture", "name": "Discord",
              "settings": { "window": "Discord:Chrome_WidgetWin_1:Discord.exe" } },
            { "id": "browser_source", "name": "Alerts", "settings": { "url": "https://example.invalid/?token=secret" } },
            { "id": "wasapi_input_capture", "name": "Mic/Aux", "settings": {} },
            { "id": "wasapi_input_capture", "settings": {} },
            "not an object"
        ]
    });
    let (name, inputs) = parse_scene_collection(&document);
    assert_eq!(name.as_deref(), Some("Streaming"));
    let summary: Vec<_> = inputs
        .iter()
        .map(|input| {
            (
                input.name.as_str(),
                input.kind,
                input.special,
                input.device_id.as_deref(),
                input.application.as_deref(),
                input.muted,
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            (
                "Desktop Audio",
                AudioKind::Output,
                Some("desktop1"),
                Some("default"),
                None,
                Some(false)
            ),
            (
                "Mic/Aux",
                AudioKind::Input,
                Some("mic1"),
                Some(CABLE_A_OUTPUT),
                None,
                Some(true)
            ),
            ("Raw mic", AudioKind::Input, None, Some(MIC), None, None),
            (
                "Discord",
                AudioKind::Application,
                None,
                None,
                Some("Discord.exe"),
                None
            ),
        ]
    );
    // Only audio facts are kept: nothing from other sources' settings.
    assert!(!format!("{inputs:?}").contains("token"));
}

#[test]
fn planned_changes_name_the_value_they_replace() {
    let inputs = [
        input("Mic/Aux", AudioKind::Input, Some(MIC), Some(false)),
        input(
            "Desktop Audio",
            AudioKind::Output,
            Some("default"),
            Some(false),
        ),
        input("Discord", AudioKind::Application, None, Some(false)),
    ];
    let endpoints = [
        endpoint(MIC, EndpointDirection::Capture),
        endpoint(CABLE_A_OUTPUT, EndpointDirection::Capture),
        endpoint(CABLE_INPUT, EndpointDirection::Render),
    ];
    let changes = plan_changes(
        &inputs,
        &[
            json!({ "inputName": "Mic/Aux", "deviceId": CABLE_A_OUTPUT }),
            json!({ "inputName": "Desktop Audio", "deviceId": CABLE_INPUT }),
            json!({ "inputName": "Discord", "muted": true }),
        ],
        Some(&endpoints),
    )
    .unwrap();
    assert_eq!(
        changes,
        [
            Change::Device {
                input: "Mic/Aux".into(),
                before: MIC.into(),
                after: CABLE_A_OUTPUT.into()
            },
            Change::Device {
                input: "Desktop Audio".into(),
                before: "default".into(),
                after: CABLE_INPUT.into()
            },
            Change::Muted {
                input: "Discord".into(),
                before: false,
                after: true
            },
        ]
    );
    assert_eq!(changes[0].inverse().to_json()["after"], MIC);

    let refused = |request: Value| plan_changes(&inputs, &[request], Some(&endpoints)).unwrap_err();
    assert!(refused(json!({ "inputName": "Nope", "muted": true })).contains("no audio source"));
    assert!(refused(json!({ "inputName": "Discord", "deviceId": MIC })).contains("application"));
    // A recording device for an output capture, and an unknown device.
    assert!(refused(json!({ "inputName": "Desktop Audio", "deviceId": MIC })).contains("playback"));
    assert!(refused(json!({ "inputName": "Mic/Aux", "deviceId": "{gone}" })).contains("recording"));
    assert!(refused(json!({ "inputName": "Mic/Aux", "deviceId": MIC })).contains("already"));
    assert!(refused(json!({ "inputName": "Mic/Aux", "muted": false })).contains("already"));
    assert!(refused(json!({ "inputName": "Mic/Aux" })).contains("exactly one"));
    assert!(
        refused(json!({ "inputName": "Mic/Aux", "deviceId": "default", "muted": true }))
            .contains("exactly one")
    );
    assert!(refused(json!({ "inputName": "Mic/Aux", "url": "x" })).contains("unknown"));
    assert!(plan_changes(
        &inputs,
        &[
            json!({ "inputName": "Mic/Aux", "muted": true }),
            json!({ "inputName": "Mic/Aux", "muted": true })
        ],
        None
    )
    .unwrap_err()
    .contains("twice"));
    assert!(plan_changes(&inputs, &[], None).is_err());
    let too_many = vec![json!({ "inputName": "Mic/Aux", "muted": true }); MAX_OBS_CHANGES + 1];
    assert!(plan_changes(&inputs, &too_many, None).is_err());
    // "default" is always a valid device.
    assert!(plan_changes(
        &inputs,
        &[json!({ "inputName": "Mic/Aux", "deviceId": "default" })],
        Some(&endpoints)
    )
    .is_ok());
}

type FakeInputs = Arc<Mutex<BTreeMap<String, (String, Value, bool)>>>;

struct FakeContext {
    collection: String,
    uuids: BTreeMap<String, String>,
    active: Option<bool>,
    tracks: Value,
    monitor_type: Option<&'static str>,
    refuse_write: Option<String>,
    drop_write_uuid: Option<String>,
}

/// A fake obs-websocket v5 server on 127.0.0.1, serving connections one
/// after another. Removing a UUID also removes that source from the public
/// input list, modelling private global inputs on older OBS servers.
struct FakeObs {
    port: u16,
    inputs: FakeInputs,
    writes: Arc<Mutex<Vec<Value>>>,
    context: Arc<Mutex<FakeContext>>,
}

fn fake_obs(password: &'static str) -> FakeObs {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let inputs: FakeInputs = Arc::new(Mutex::new(BTreeMap::from([
        (
            "Desktop Audio".to_owned(),
            ("wasapi_output_capture".to_owned(), json!({}), false),
        ),
        (
            "Mic/Aux".to_owned(),
            (
                "wasapi_input_capture".to_owned(),
                json!({ "device_id": MIC }),
                false,
            ),
        ),
        (
            "Discord".to_owned(),
            (
                "wasapi_process_output_capture".to_owned(),
                json!({ "window": "Discord:Chrome_WidgetWin_1:Discord.exe" }),
                false,
            ),
        ),
        (
            "Alerts".to_owned(),
            (
                "browser_source".to_owned(),
                json!({ "url": "https://example.invalid" }),
                false,
            ),
        ),
    ])));
    let writes = Arc::new(Mutex::new(Vec::new()));
    let context = Arc::new(Mutex::new(FakeContext {
        collection: "Streaming".into(),
        uuids: inputs
            .lock()
            .unwrap()
            .keys()
            .map(|name| (name.clone(), format!("uuid-{name}")))
            .collect(),
        active: Some(true),
        tracks: json!({ "1": true, "2": false, "3": false, "4": false, "5": false, "6": false }),
        monitor_type: Some("OBS_MONITORING_TYPE_NONE"),
        refuse_write: None,
        drop_write_uuid: None,
    }));
    let served_context = Arc::clone(&context);
    let (served_inputs, served_writes) = (Arc::clone(&inputs), Arc::clone(&writes));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { break };
            serve(
                stream,
                password,
                &served_inputs,
                &served_writes,
                &served_context,
            );
        }
    });
    FakeObs {
        port,
        inputs,
        writes,
        context,
    }
}

fn text(value: Value) -> Message {
    Message::Text(value.to_string().into())
}

fn serve(
    stream: TcpStream,
    password: &str,
    inputs: &FakeInputs,
    writes: &Mutex<Vec<Value>>,
    context: &Mutex<FakeContext>,
) {
    let Ok(mut socket) = tungstenite::accept(stream) else {
        return;
    };
    let (salt, challenge) = ("c2FsdC1mb3ItdGVzdHM=", "Y2hhbGxlbmdlLWZvci10ZXN0cw==");
    let mut hello = json!({ "op": 0, "d": {
        "obsWebSocketVersion": "5.6.3",
        "rpcVersion": 1,
        "authentication": { "challenge": challenge, "salt": salt }
    }});
    if password.is_empty() {
        hello["d"].as_object_mut().unwrap().remove("authentication");
    }
    if socket.send(text(hello)).is_err() {
        return;
    }
    loop {
        let message = match socket.read() {
            Ok(Message::Text(message)) => message,
            Ok(_) => continue,
            Err(_) => return,
        };
        let message: Value = serde_json::from_str(message.as_str()).unwrap();
        match message["op"].as_u64() {
            Some(1) => {
                assert_eq!(message["d"]["rpcVersion"], 1);
                assert_eq!(message["d"]["eventSubscriptions"], 0);
                if !password.is_empty()
                    && message["d"]["authentication"]
                        != authentication(password, salt, challenge).as_str()
                {
                    let _ = socket.close(Some(CloseFrame {
                        code: CloseCode::from(AUTHENTICATION_FAILED),
                        reason: "Authentication failed.".into(),
                    }));
                    while socket.read().is_ok() {}
                    return;
                }
                let identified = json!({ "op": 2, "d": { "negotiatedRpcVersion": 1 } });
                if socket.send(text(identified)).is_err() {
                    return;
                }
            }
            Some(6) => {
                let request = &message["d"];
                let should_drop = {
                    let context = context.lock().unwrap();
                    matches!(
                        request["requestType"].as_str(),
                        Some("SetInputSettings" | "SetInputMute")
                    ) && context.drop_write_uuid.as_deref().is_some_and(|uuid| {
                        request["requestData"]["inputUuid"].as_str() == Some(uuid)
                    })
                };
                if should_drop {
                    let _ = socket.close(None);
                    return;
                }
                let (ok, data) = respond(request, inputs, writes, context);
                let response = json!({ "op": 7, "d": {
                    "requestType": request["requestType"],
                    "requestId": request["requestId"],
                    "requestStatus": { "result": ok, "code": if ok { 100 } else { 600 } },
                    "responseData": data
                }});
                if socket.send(text(response)).is_err() {
                    return;
                }
            }
            _ => {}
        }
    }
}

fn respond(
    request: &Value,
    inputs: &FakeInputs,
    writes: &Mutex<Vec<Value>>,
    context: &Mutex<FakeContext>,
) -> (bool, Value) {
    let data = &request["requestData"];
    let context = context.lock().unwrap();
    let name = data["inputName"]
        .as_str()
        .or_else(|| data["sourceName"].as_str())
        .or_else(|| {
            let uuid = data["inputUuid"].as_str()?;
            context
                .uuids
                .iter()
                .find(|(_, value)| value.as_str() == uuid)
                .map(|(name, _)| name.as_str())
        })
        .unwrap_or_default();
    let mut inputs = inputs.lock().unwrap();
    match request["requestType"].as_str().unwrap_or_default() {
        "GetSceneCollectionList" => (
            true,
            json!({ "currentSceneCollectionName": context.collection, "sceneCollections": [context.collection] }),
        ),
        "GetSpecialInputs" => (
            true,
            json!({ "desktop1": "Desktop Audio", "desktop2": null, "mic1": "Mic/Aux",
                    "mic2": null, "mic3": null, "mic4": null }),
        ),
        "GetInputList" => {
            let listed: Vec<Value> = inputs
                .iter()
                .filter(|(name, _)| context.uuids.contains_key(name.as_str()))
                .map(|(name, (kind, _, _))| {
                    json!({ "inputName": name, "inputUuid": context.uuids[name],
                            "inputKind": kind, "unversionedInputKind": kind })
                })
                .collect();
            (true, json!({ "inputs": listed }))
        }
        "GetInputSettings" => match inputs.get(name) {
            Some((kind, settings, _)) => (
                true,
                json!({ "inputSettings": settings, "inputKind": kind }),
            ),
            None => (false, Value::Null),
        },
        "GetInputMute" => match inputs.get(name) {
            Some((_, _, muted)) => (true, json!({ "inputMuted": muted })),
            None => (false, Value::Null),
        },
        "GetSourceActive" => (
            true,
            json!({ "videoActive": context.active, "videoShowing": true }),
        ),
        "GetInputAudioTracks" => (true, json!({ "inputAudioTracks": context.tracks })),
        "GetInputAudioMonitorType" => (true, json!({ "monitorType": context.monitor_type })),
        "SetInputSettings" => {
            if context.refuse_write.as_deref() == Some(name) {
                return (false, Value::Null);
            }
            writes.lock().unwrap().push(request.clone());
            assert_eq!(data["overlay"], true, "only the named setting changes");
            match inputs.get_mut(name) {
                Some((_, settings, _)) => {
                    for (key, value) in data["inputSettings"].as_object().unwrap() {
                        settings[key.as_str()] = value.clone();
                    }
                    (true, Value::Null)
                }
                None => (false, Value::Null),
            }
        }
        "SetInputMute" => {
            if context.refuse_write.as_deref() == Some(name) {
                return (false, Value::Null);
            }
            writes.lock().unwrap().push(request.clone());
            match inputs.get_mut(name) {
                Some((_, _, muted)) => {
                    *muted = data["inputMuted"].as_bool().unwrap();
                    (true, Value::Null)
                }
                None => (false, Value::Null),
            }
        }
        _ => (false, Value::Null),
    }
}

fn wait_for(integration: &mut ObsIntegration, done: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let status = integration.status();
        if done(&status) || Instant::now() > deadline {
            return status;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn input_named<'a>(status: &'a Value, name: &str) -> &'a Value {
    status["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|input| input["name"] == name)
        .unwrap_or_else(|| panic!("no input {name:?} in {status}"))
}

fn connected(obs: &FakeObs, settings_password: Option<&str>) -> ObsIntegration {
    let mut integration = ObsIntegration::new(ObsPaths {
        settings: Some(settings_folder(settings_password)),
        port: Some(obs.port),
    });
    integration.connect(None);
    let status = wait_for(&mut integration, |status| {
        status["connection"] == "connected"
    });
    assert_eq!(status["connection"], "connected", "{status}");
    integration
}

#[test]
fn connects_with_the_password_from_obs_settings_and_reads_audio_sources() {
    let obs = fake_obs(PASSWORD);
    let mut integration = connected(&obs, Some(PASSWORD));
    let status = integration.status();
    assert_eq!(status["source"], "live");
    assert_eq!(status["sceneCollection"], "Streaming");
    assert_eq!(status["passwordSource"], "obsSettings");
    assert_eq!(status["installed"], true);
    let names: Vec<_> = status["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|input| input["name"].as_str().unwrap())
        .collect();
    // The browser source is not audio.
    assert_eq!(names, ["Desktop Audio", "Discord", "Mic/Aux"]);
    let desktop = input_named(&status, "Desktop Audio");
    assert_eq!(desktop["special"], "desktop1");
    assert_eq!(desktop["kind"], "outputCapture");
    assert_eq!(desktop["deviceId"], "default");
    let mic = input_named(&status, "Mic/Aux");
    assert_eq!(mic["special"], "mic1");
    assert_eq!(mic["deviceId"], MIC);
    assert_eq!(mic["muted"], false);
    assert_eq!(mic["uuid"], "uuid-Mic/Aux");
    assert_eq!(mic["active"], true);
    assert_eq!(mic["audioTracks"], json!([1]));
    assert_eq!(mic["monitorOnly"], false);
    let discord = input_named(&status, "Discord");
    assert_eq!(discord["kind"], "applicationCapture");
    assert_eq!(discord["application"], "Discord.exe");
    assert!(discord["deviceId"].is_null());
    // The password never leaves the OBS thread.
    assert!(!status.to_string().contains(PASSWORD));
    // A preview sends nothing to OBS.
    integration
        .preview(&[json!({ "inputName": "Mic/Aux", "muted": true })], None)
        .unwrap();
    assert!(obs.writes.lock().unwrap().is_empty());
}

#[test]
fn apply_changes_only_what_was_previewed_and_restore_puts_it_back() {
    let obs = fake_obs(PASSWORD);
    let mut integration = connected(&obs, Some(PASSWORD));
    let preview = integration
        .preview(
            &[
                json!({ "inputName": "Mic/Aux", "deviceId": CABLE_A_OUTPUT }),
                json!({ "inputName": "Desktop Audio", "muted": true }),
            ],
            None,
        )
        .unwrap();
    assert_eq!(preview["changes"][0]["before"], MIC);
    assert_eq!(preview["changes"][0]["after"], CABLE_A_OUTPUT);
    assert_eq!(preview["changes"][1]["field"], "muted");
    let preview_id = preview["previewId"].as_str().unwrap().to_owned();
    assert!(integration.apply("obs-preview-unknown").is_err());
    assert_eq!(integration.apply(&preview_id).unwrap()["state"], "applying");
    // A preview is used once.
    assert!(integration.apply(&preview_id).is_err());
    let status = wait_for(&mut integration, |status| {
        status["lastChange"]["state"] == "applied"
    });
    assert_eq!(status["lastChange"]["state"], "applied", "{status}");
    assert_eq!(status["lastChange"]["changed"], 2);
    assert_eq!(status["lastChange"]["restorable"], true);
    // The status already shows OBS's new values.
    assert_eq!(input_named(&status, "Mic/Aux")["deviceId"], CABLE_A_OUTPUT);
    assert_eq!(input_named(&status, "Desktop Audio")["muted"], true);
    assert_eq!(obs.writes.lock().unwrap().len(), 2);

    assert_eq!(integration.restore().unwrap()["state"], "restoring");
    let status = wait_for(&mut integration, |status| {
        status["lastChange"]["state"] == "restored"
    });
    assert_eq!(status["lastChange"]["state"], "restored", "{status}");
    assert_eq!(status["lastChange"]["restorable"], false);
    assert_eq!(input_named(&status, "Mic/Aux")["deviceId"], MIC);
    assert_eq!(input_named(&status, "Desktop Audio")["muted"], false);
    assert!(integration.restore().is_err(), "nothing left to restore");
}

#[test]
fn a_setting_changed_in_obs_after_the_preview_is_not_overwritten() {
    let obs = fake_obs(PASSWORD);
    let mut integration = connected(&obs, Some(PASSWORD));
    let preview = integration
        .preview(
            &[json!({ "inputName": "Mic/Aux", "deviceId": CABLE_A_OUTPUT })],
            None,
        )
        .unwrap();
    // The user picks another device in OBS before confirming.
    obs.inputs.lock().unwrap().get_mut("Mic/Aux").unwrap().1 = json!({ "device_id": "default" });
    integration
        .apply(preview["previewId"].as_str().unwrap())
        .unwrap();
    let status = wait_for(&mut integration, |status| {
        status["lastChange"]["state"] == "failed"
    });
    assert_eq!(status["lastChange"]["state"], "failed", "{status}");
    assert_eq!(status["lastChange"]["error"], "changedInObs");
    assert_eq!(status["lastChange"]["restorable"], false);
    assert!(obs.writes.lock().unwrap().is_empty());
    assert_eq!(input_named(&status, "Mic/Aux")["deviceId"], "default");
}

#[test]
fn apply_refuses_a_different_collection_or_a_recreated_same_name_source() {
    for replacement in ["collection", "uuid"] {
        let obs = fake_obs(PASSWORD);
        let mut integration = connected(&obs, Some(PASSWORD));
        let preview = integration
            .preview(
                &[json!({ "inputName": "Mic/Aux", "deviceId": CABLE_A_OUTPUT })],
                None,
            )
            .unwrap();
        {
            let mut context = obs.context.lock().unwrap();
            if replacement == "collection" {
                // Even a copied collection retaining the UUID must be refused.
                context.collection = "Other collection".into();
            } else {
                context
                    .uuids
                    .insert("Mic/Aux".into(), "replacement-uuid".into());
            }
        }
        integration
            .apply(preview["previewId"].as_str().unwrap())
            .unwrap();
        let status = wait_for(&mut integration, |status| {
            status["lastChange"]["state"] == "failed"
        });
        assert_eq!(
            status["lastChange"]["error"], "changedInObs",
            "{replacement}: {status}"
        );
        assert!(obs.writes.lock().unwrap().is_empty(), "{replacement}");
        assert_eq!(obs.inputs.lock().unwrap()["Mic/Aux"].1["device_id"], MIC);
    }
}

#[test]
fn restore_refuses_a_different_collection_or_a_recreated_same_name_source() {
    for replacement in ["collection", "uuid"] {
        let obs = fake_obs(PASSWORD);
        let mut integration = connected(&obs, Some(PASSWORD));
        let preview = integration
            .preview(
                &[json!({ "inputName": "Mic/Aux", "deviceId": CABLE_A_OUTPUT })],
                None,
            )
            .unwrap();
        integration
            .apply(preview["previewId"].as_str().unwrap())
            .unwrap();
        let applied = wait_for(&mut integration, |status| {
            status["lastChange"]["state"] == "applied"
        });
        assert_eq!(applied["lastChange"]["state"], "applied", "{applied}");
        assert!(obs.writes.lock().unwrap()[0]["requestData"]["inputName"].is_null());
        assert_eq!(
            obs.writes.lock().unwrap()[0]["requestData"]["inputUuid"],
            "uuid-Mic/Aux"
        );
        {
            let mut context = obs.context.lock().unwrap();
            if replacement == "collection" {
                context.collection = "Other collection".into();
            } else {
                context
                    .uuids
                    .insert("Mic/Aux".into(), "replacement-uuid".into());
            }
        }
        integration.restore().unwrap();
        let status = wait_for(&mut integration, |status| {
            status["lastChange"]["state"] == "failed"
        });
        assert_eq!(
            status["lastChange"]["error"], "changedInObs",
            "{replacement}: {status}"
        );
        assert_eq!(obs.writes.lock().unwrap().len(), 1, "{replacement}");
        assert_eq!(
            obs.inputs.lock().unwrap()["Mic/Aux"].1["device_id"],
            CABLE_A_OUTPUT
        );
        assert_eq!(status["lastChange"]["restorable"], true);
    }
}

#[test]
fn a_source_without_a_public_uuid_is_read_only() {
    let obs = fake_obs(PASSWORD);
    obs.context.lock().unwrap().uuids.remove("Desktop Audio");
    let mut integration = connected(&obs, Some(PASSWORD));
    assert!(input_named(&integration.status(), "Desktop Audio")["uuid"].is_null());
    assert!(integration
        .preview(
            &[json!({ "inputName": "Desktop Audio", "muted": true })],
            None
        )
        .is_err());
    assert!(obs.writes.lock().unwrap().is_empty());
}

#[test]
fn track_flags_are_bounded_and_incomplete_flags_stay_unknown() {
    assert_eq!(
        parse_audio_tracks(
            &json!({ "1": true, "2": false, "3": true, "4": false, "5": false, "6": true })
        ),
        Some(vec![1, 3, 6])
    );
    for value in [
        Value::Null,
        json!([]),
        json!({ "1": true }),
        json!({ "1": true, "2": false, "3": true, "4": false, "5": false, "6": "true" }),
    ] {
        assert_eq!(parse_audio_tracks(&value), None);
    }
}

#[test]
fn live_inspection_retains_inactive_sources_and_unknown_audio_facts() {
    let obs = fake_obs(PASSWORD);
    {
        let mut context = obs.context.lock().unwrap();
        context.active = Some(false);
        context.tracks = Value::Null;
        context.monitor_type = None;
    }
    let mut integration = connected(&obs, Some(PASSWORD));
    let status = integration.status();
    let mic = input_named(&status, "Mic/Aux");
    assert_eq!(mic["active"], false);
    assert!(mic["audioTracks"].is_null());
    assert!(mic["monitorOnly"].is_null());
}

#[test]
fn a_wrong_or_missing_password_is_reported_and_a_pasted_one_connects() {
    let obs = fake_obs(PASSWORD);
    let mut wrong = ObsIntegration::new(ObsPaths {
        settings: Some(settings_folder(Some("not-the-password"))),
        port: Some(obs.port),
    });
    wrong.connect(None);
    let status = wait_for(&mut wrong, |status| status["connection"] == "wrongPassword");
    assert_eq!(status["connection"], "wrongPassword", "{status}");

    let mut missing = ObsIntegration::new(ObsPaths {
        settings: Some(settings_folder(None)),
        port: Some(obs.port),
    });
    missing.connect(None);
    let status = wait_for(&mut missing, |status| {
        status["connection"] == "needsPassword"
    });
    assert_eq!(status["connection"], "needsPassword", "{status}");

    missing.connect(Some(PASSWORD.into()));
    let status = wait_for(&mut missing, |status| status["connection"] == "connected");
    assert_eq!(status["connection"], "connected", "{status}");
    assert_eq!(status["passwordSource"], "pasted");
    assert!(!status.to_string().contains(PASSWORD));

    // Disconnecting forgets the connection and any restore.
    missing.disconnect();
    let status = wait_for(&mut missing, |status| status["connection"] == "off");
    assert_eq!(status["connection"], "off");
    assert!(status["passwordSource"].is_null());
    assert!(missing
        .preview(&[json!({ "inputName": "Mic/Aux", "muted": true })], None)
        .is_err());
}

#[test]
fn a_closed_obs_shows_the_saved_scene_collection() {
    // A port nobody listens on.
    let port = {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.local_addr().unwrap().port()
    };
    let folder = settings_folder(Some(PASSWORD));
    std::fs::write(
        folder.join("user.ini"),
        "[Basic]\nSceneCollection=Streaming\nSceneCollectionFile=Streaming\n",
    )
    .unwrap();
    let scenes = folder.join("basic").join("scenes");
    std::fs::create_dir_all(&scenes).unwrap();
    std::fs::write(
        scenes.join("Streaming.json"),
        json!({
            "name": "Streaming",
            "AuxAudioDevice1": { "id": "wasapi_input_capture", "name": "Mic/Aux",
                                 "settings": { "device_id": MIC }, "muted": false },
            "sources": []
        })
        .to_string(),
    )
    .unwrap();
    let mut integration = ObsIntegration::new(ObsPaths {
        settings: Some(folder),
        port: Some(port),
    });
    // Before any connection, the saved setup is shown.
    let status = wait_for(&mut integration, |status| status["source"] == "savedFile");
    assert_eq!(status["source"], "savedFile", "{status}");
    assert_eq!(status["connection"], "off");
    assert_eq!(input_named(&status, "Mic/Aux")["deviceId"], MIC);
    assert!(input_named(&status, "Mic/Aux")["active"].is_null());
    assert!(input_named(&status, "Mic/Aux")["audioTracks"].is_null());
    integration.connect(None);
    let status = wait_for(&mut integration, |status| {
        status["connection"] == "serverOff"
    });
    assert_eq!(status["connection"], "serverOff", "{status}");
    assert_eq!(status["source"], "savedFile");
    assert!(
        integration
            .preview(&[json!({ "inputName": "Mic/Aux", "muted": true })], None)
            .is_err(),
        "a saved file is never changed"
    );
}

#[test]
fn a_server_without_authentication_needs_no_password() {
    let obs = fake_obs("");
    let mut integration = connected(&obs, None);
    assert!(integration.status()["passwordSource"].is_null());
}

#[test]
fn a_partial_refused_apply_keeps_the_successful_change_restorable() {
    let obs = fake_obs(PASSWORD);
    let mut integration = connected(&obs, Some(PASSWORD));
    obs.context.lock().unwrap().refuse_write = Some("Mic/Aux".into());
    let preview = integration
        .preview(
            &[
                json!({ "inputName": "Desktop Audio", "muted": true }),
                json!({ "inputName": "Mic/Aux", "deviceId": CABLE_A_OUTPUT }),
            ],
            None,
        )
        .unwrap();
    integration
        .apply(preview["previewId"].as_str().unwrap())
        .unwrap();
    let status = wait_for(&mut integration, |status| {
        status["lastChange"]["state"] == "failed"
    });
    assert_eq!(status["lastChange"]["error"], "refusedByObs", "{status}");
    assert_eq!(status["lastChange"]["changed"], 1);
    assert_eq!(status["lastChange"]["restorable"], true);
    assert!(obs.inputs.lock().unwrap()["Desktop Audio"].2);
    assert_eq!(obs.inputs.lock().unwrap()["Mic/Aux"].1["device_id"], MIC);
    integration.restore().unwrap();
    let status = wait_for(&mut integration, |status| {
        status["lastChange"]["state"] == "restored"
    });
    assert_eq!(status["lastChange"]["state"], "restored", "{status}");
    assert!(!obs.inputs.lock().unwrap()["Desktop Audio"].2);
}

#[test]
fn a_partial_restore_can_resume_without_repeating_completed_changes() {
    let obs = fake_obs(PASSWORD);
    let mut integration = connected(&obs, Some(PASSWORD));
    let preview = integration
        .preview(
            &[
                json!({ "inputName": "Mic/Aux", "deviceId": CABLE_A_OUTPUT }),
                json!({ "inputName": "Desktop Audio", "muted": true }),
            ],
            None,
        )
        .unwrap();
    integration
        .apply(preview["previewId"].as_str().unwrap())
        .unwrap();
    let applied = wait_for(&mut integration, |status| {
        status["lastChange"]["state"] == "applied"
    });
    assert_eq!(applied["lastChange"]["state"], "applied", "{applied}");
    obs.context.lock().unwrap().refuse_write = Some("Mic/Aux".into());
    integration.restore().unwrap();
    let failed = wait_for(&mut integration, |status| {
        status["lastChange"]["state"] == "failed"
    });
    assert_eq!(failed["lastChange"]["changed"], 1, "{failed}");
    assert_eq!(failed["lastChange"]["restorable"], true);
    assert!(!obs.inputs.lock().unwrap()["Desktop Audio"].2);
    assert_eq!(
        obs.inputs.lock().unwrap()["Mic/Aux"].1["device_id"],
        CABLE_A_OUTPUT
    );
    obs.context.lock().unwrap().refuse_write = None;
    integration.restore().unwrap();
    let restored = wait_for(&mut integration, |status| {
        status["lastChange"]["state"] == "restored"
    });
    assert_eq!(restored["lastChange"]["state"], "restored", "{restored}");
    assert_eq!(restored["lastChange"]["changed"], 1);
    assert_eq!(restored["lastChange"]["restorable"], false);
    assert_eq!(obs.writes.lock().unwrap().len(), 4);
}

#[test]
fn connection_loss_preserves_confirmed_changes_for_restore_after_reconnect() {
    let obs = fake_obs(PASSWORD);
    let mut integration = connected(&obs, Some(PASSWORD));
    obs.context.lock().unwrap().drop_write_uuid = Some("uuid-Mic/Aux".into());
    let preview = integration
        .preview(
            &[
                json!({ "inputName": "Desktop Audio", "muted": true }),
                json!({ "inputName": "Mic/Aux", "deviceId": CABLE_A_OUTPUT }),
            ],
            None,
        )
        .unwrap();
    integration
        .apply(preview["previewId"].as_str().unwrap())
        .unwrap();
    let failed = wait_for(&mut integration, |status| {
        status["lastChange"]["state"] == "failed"
    });
    assert_eq!(failed["lastChange"]["error"], "connectionLost", "{failed}");
    assert_eq!(failed["lastChange"]["changed"], 1);
    assert_eq!(failed["lastChange"]["restorable"], true);
    obs.context.lock().unwrap().drop_write_uuid = None;
    integration.connect(None);
    let reconnected = wait_for(&mut integration, |status| {
        status["connection"] == "connected"
    });
    assert_eq!(reconnected["connection"], "connected", "{reconnected}");
    integration.restore().unwrap();
    let restored = wait_for(&mut integration, |status| {
        status["lastChange"]["state"] == "restored"
    });
    assert_eq!(restored["lastChange"]["state"], "restored", "{restored}");
    assert!(!obs.inputs.lock().unwrap()["Desktop Audio"].2);
}

#[test]
fn previews_expire_and_parallel_changes_are_refused_before_queueing() {
    let expiry = Duration::from_secs(OBS_PREVIEW_SECONDS);
    for age in [
        expiry - Duration::from_nanos(1),
        expiry,
        expiry + Duration::from_nanos(1),
    ] {
        let mut integration = ObsIntegration::new(ObsPaths {
            settings: None,
            port: None,
        });
        {
            let mut snapshot = integration.shared.snapshot();
            snapshot.connection = Connection::Connected;
            snapshot.source = Some(Source::Live);
            snapshot.scene_collection = Some("Streaming".into());
            let mut source = input("Mic/Aux", AudioKind::Input, Some(MIC), Some(false));
            source.uuid = Some("uuid-mic".into());
            snapshot.inputs.push(source);
        }
        let changes = [json!({ "inputName": "Mic/Aux", "deviceId": CABLE_A_OUTPUT })];
        let preview = integration.preview(&changes, None).unwrap();
        let created = integration.preview.as_ref().unwrap().created;
        let result = integration.apply_at(preview["previewId"].as_str().unwrap(), created + age);
        if age > expiry {
            assert!(result.unwrap_err().contains("expired"));
            assert!(integration.shared.commands.lock().unwrap().is_empty());
        } else {
            assert_eq!(
                result.unwrap(),
                json!({ "state": "applying", "changes": 1 })
            );
            let mut commands = integration.shared.commands.lock().unwrap();
            assert_eq!(commands.len(), 1);
            assert!(matches!(commands.pop_front(), Some(Command::Apply(_))));
        }
        integration.shared.snapshot().change = ChangeState::Applying;
        integration.shared.snapshot().restorable = true;
        assert!(integration.preview(&changes, None).is_err());
        assert!(integration.restore().is_err());
        assert!(integration.shared.commands.lock().unwrap().is_empty());
    }
}

#[test]
fn dropping_the_owner_releases_the_obs_thread_and_connection() {
    let obs = fake_obs(PASSWORD);
    let integration = connected(&obs, Some(PASSWORD));
    let worker = Arc::downgrade(&integration.shared);
    drop(integration);
    let deadline = Instant::now() + Duration::from_secs(5);
    while worker.upgrade().is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        worker.upgrade().is_none(),
        "OBS worker retained a dropped backend"
    );
    // The single-client fake server accepts a replacement only after the
    // old connection is released as well.
    let _replacement = connected(&obs, Some(PASSWORD));
}

#[test]
fn pending_connection_choices_are_bounded_and_disconnect_forgets_passwords() {
    let integration = ObsIntegration::new(ObsPaths {
        settings: None,
        port: None,
    });
    for index in 0..100 {
        integration.shared.push(Command::Connect {
            password: Some(format!("test-secret-{index}")),
        });
        integration.shared.push(Command::Refresh);
    }
    assert_eq!(integration.shared.commands.lock().unwrap().len(), 2);
    integration.shared.push(Command::Disconnect);
    let commands = integration.shared.commands.lock().unwrap();
    assert!(!commands
        .iter()
        .any(|item| matches!(item, Command::Connect { .. })));
    assert_eq!(commands.len(), 2);
}

#[test]
fn bounded_files_reject_oversize_invalid_utf8_and_directories() {
    let folder = settings_folder(None);
    let file = folder.join("bounded.json");
    std::fs::write(&file, b"1234").unwrap();
    assert_eq!(read_bounded(&file, 4).as_deref(), Some("1234"));
    assert!(read_bounded(&file, 3).is_none());
    std::fs::write(&file, [0xff]).unwrap();
    assert!(read_bounded(&file, 4).is_none());
    assert!(read_bounded(&folder, 4).is_none());
}
