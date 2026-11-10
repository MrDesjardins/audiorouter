//! Goals are exercised through JSON-RPC dispatch, the path REST and the UI use.

use super::*;

fn call(plane: &mut ControlPlane, method: &str, params: Value) -> Value {
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    });
    assert!(response.error.is_none(), "{method}: {:?}", response.error);
    response.result.expect("a result")
}

fn create(plane: &mut ControlPlane, goal: &str, key: &str, application: Option<Value>) -> Value {
    let mut params = json!({ "goal": goal, "idempotencyKey": key });
    if let Some(application) = application {
        params["application"] = application;
    }
    call(plane, "sessions.createFromGoal", params)
}

fn game() -> Option<Value> {
    Some(json!({ "executable": "game.exe" }))
}

fn sources_reaching(session: &Session, target: &str) -> Vec<String> {
    let mut seen = vec![target.to_owned()];
    let mut index = 0;
    while index < seen.len() {
        let current = seen[index].clone();
        for edge in &session.edges {
            let source = edge.source_node.as_str().to_owned();
            if edge.destination_node.as_str() == current && !seen.contains(&source) {
                seen.push(source);
            }
        }
        index += 1;
    }
    seen
}

#[test]
fn every_listed_goal_makes_a_valid_stopped_session_with_or_without_an_app() {
    let mut plane = ControlPlane::new("test");
    let goals = call(&mut plane, "sessions.goals", json!({}));
    let goals = goals["goals"].as_array().expect("goals");
    assert!(goals.len() >= 6, "{goals:?}");
    for goal in goals {
        let id = goal["id"].as_str().unwrap();
        for (variant, application) in [("none", None), ("app", game())] {
            if application.is_some() && goal["applicationPrompt"].is_null() {
                continue;
            }
            let result = create(&mut plane, id, &format!("{id}-{variant}"), application);
            assert_eq!(result["state"], "stopped");
            let session: Session = serde_json::from_value(result["session"].clone()).unwrap();
            assert_eq!(session.revision, 0);
            assert_eq!(session.name, goal["title"].as_str().unwrap());
            for edge in &session.edges {
                assert!(session.nodes.iter().any(|node| node.id == edge.source_node));
            }
        }
    }
}

/// A person's devices are theirs to pick, and nothing may record or play.
#[test]
fn no_goal_binds_a_device_or_arms_a_recorder() {
    let mut plane = ControlPlane::new("test");
    for goal in [
        "clean-mic",
        "game-discord",
        "podcast",
        "record-app",
        "mix-minus",
    ] {
        let result = create(
            &mut plane,
            goal,
            goal,
            game().filter(|_| goal != "clean-mic"),
        );
        let session: Session = serde_json::from_value(result["session"].clone()).unwrap();
        for node in &session.nodes {
            assert!(
                !node.parameters.contains_key("endpointId"),
                "{goal}: {}",
                node.name
            );
            assert_ne!(
                node.parameters.get("autoRecord"),
                Some(&json!(true)),
                "{goal}"
            );
        }
        assert!(plane.runtimes.is_empty(), "{goal} must not start audio");
    }
}

/// UI promise: "Your mic stays out of your headphones."
#[test]
fn the_microphone_never_reaches_headphones() {
    let mut plane = ControlPlane::new("test");
    for goal in ["game-discord", "mix-minus"] {
        let result = create(&mut plane, goal, goal, game());
        let session: Session = serde_json::from_value(result["session"].clone()).unwrap();
        let sources = sources_reaching(&session, "headphones");
        assert!(sources.contains(&APP.to_owned()), "{goal}: {sources:?}");
        assert!(
            !sources.contains(&"microphone".to_owned()),
            "{goal}: {sources:?}"
        );
    }
}

/// "Your guest hears you but not themselves."
#[test]
fn mix_minus_keeps_the_guest_out_of_their_own_feed() {
    let mut plane = ControlPlane::new("test");
    let result = create(&mut plane, "mix-minus", "m", game());
    let session: Session = serde_json::from_value(result["session"].clone()).unwrap();
    let sources = sources_reaching(&session, "to-guest");
    assert!(sources.contains(&"microphone".to_owned()));
    assert!(!sources.contains(&APP.to_owned()));
}

#[test]
fn an_unchosen_app_is_left_out_and_named() {
    let mut plane = ControlPlane::new("test");
    let result = create(&mut plane, "game-discord", "k", None);
    let session: Session = serde_json::from_value(result["session"].clone()).unwrap();
    assert!(session
        .nodes
        .iter()
        .all(|node| node.kind != NodeKind::ApplicationCapture));
    let still = result["stillToChoose"].to_string();
    assert!(still.contains("the app for \\\"Game\\\""), "{still}");
    assert!(still.contains("your microphone"), "{still}");
}

#[test]
fn a_chosen_app_follows_the_executable_rather_than_one_process() {
    let mut plane = ControlPlane::new("test");
    let result = create(&mut plane, "podcast", "k", game());
    let session: Session = serde_json::from_value(result["session"].clone()).unwrap();
    let app = session
        .nodes
        .iter()
        .find(|node| node.id.as_str() == APP)
        .unwrap();
    assert_eq!(app.parameters["processPolicy"], "allVerifiedInstances");
    assert_eq!(app.parameters["executable"], "game.exe");
}

#[test]
fn a_repeated_key_returns_the_same_session_and_new_keys_never_overwrite() {
    let mut plane = ControlPlane::new("test");
    let first = create(&mut plane, "clean-mic", "same", None);
    let again = create(&mut plane, "clean-mic", "same", None);
    let other = create(&mut plane, "clean-mic", "other", None);
    assert_eq!(first["session"]["id"], again["session"]["id"]);
    assert_ne!(first["session"]["id"], other["session"]["id"]);
}

#[test]
fn unknown_goals_and_misplaced_apps_are_refused_in_plain_words() {
    let mut plane = ControlPlane::new("test");
    let refused = |plane: &mut ControlPlane, params: Value| {
        plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "sessions.createFromGoal".into(),
                params: Some(params),
            })
            .error
            .expect("refused")
            .message
    };
    let message = refused(&mut plane, json!({ "goal": "nope", "idempotencyKey": "a" }));
    assert!(message.contains("clean-mic"), "{message}");
    let message = refused(
        &mut plane,
        json!({ "goal": "clean-mic", "application": { "executable": "x.exe" }, "idempotencyKey": "b" }),
    );
    assert!(message.contains("no application source"), "{message}");
}

/// A starting route that cannot play is worse than none: once its devices are
/// chosen, every goal must pass the multi-path compiler Play uses.
#[test]
fn every_goal_compiles_for_play_once_its_devices_are_chosen() {
    let mut plane = ControlPlane::new("test");
    for goal in [
        "clean-mic",
        "game-discord",
        "podcast",
        "record-app",
        "mix-minus",
    ] {
        let application = game().filter(|_| goal != "clean-mic");
        let result = create(&mut plane, goal, goal, application);
        let mut session: Session = serde_json::from_value(result["session"].clone()).unwrap();
        for node in &mut session.nodes {
            if matches!(
                node.kind,
                NodeKind::PhysicalInput | NodeKind::PhysicalOutput
            ) {
                node.parameters.insert(
                    "endpointId".into(),
                    json!(format!("device-{}", node.id.as_str())),
                );
            }
        }
        audiorouter_engine::compile_native_paths_with_plugins_and_audio(
            &session,
            audiorouter_engine::RuntimeGeneration::new(1),
            &Default::default(),
            &Default::default(),
        )
        .unwrap_or_else(|error| panic!("{goal}: {error:?}"));
    }
}

/// First-time-user walk-through (2026-11-08): a goal's app source names an
/// executable, not a process, and Play rejected it with "select a running
/// application" even while the app was playing. Play now looks the app up.
#[cfg(windows)]
#[test]
fn play_looks_up_a_goal_app_and_says_plainly_when_it_is_not_running() {
    let mut plane = ControlPlane::new("test");
    let result = create(
        &mut plane,
        "record-app",
        "k",
        Some(json!({
            "executable": "audiorouter-no-such-app.exe",
            "executablePath": r"C:\Nowhere\audiorouter-no-such-app.exe"
        })),
    );
    let session_id = result["session"]["id"].as_str().unwrap().to_owned();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "nativePaths.prepare".into(),
        params: Some(json!({ "sessionId": session_id })),
    });
    let message = response.error.expect("not running").message;
    assert!(
        message.contains(
            "audiorouter-no-such-app.exe is not running. Start it, then press Play again"
        ),
        "{message}"
    );
}

/// The process choice behind it: the one root of a multi-process app (as a
/// browser's helpers are) is chosen, and two independent roots are refused.
#[cfg(windows)]
#[test]
fn a_goal_app_binds_to_its_one_running_root_process() {
    use audiorouter_windows_audio::ApplicationInfo;
    let path = r"C:\Program Files\Mozilla Firefox\firefox.exe";
    let process = |process_id, parent_process_id, creation| ApplicationInfo {
        process_id,
        parent_process_id,
        executable: "firefox.exe".into(),
        executable_path: Some(path.into()),
        creation_time_100ns: Some(creation),
    };
    let root_and_helpers = [
        process(10, 1, 100),
        process(11, 10, 101),
        process(12, 10, 102),
    ];
    assert_eq!(
        crate::native_dispatch::choose_running_application(
            &root_and_helpers,
            "Game",
            "firefox.exe",
            Some(path)
        )
        .unwrap(),
        (Some(10), Some(100))
    );
    let two_roots = [process(10, 1, 100), process(20, 1, 200)];
    let message = crate::native_dispatch::choose_running_application(
        &two_roots,
        "Game",
        "firefox.exe",
        Some(path),
    )
    .unwrap_err();
    let ControlError::InvalidRequest(message) = message else {
        panic!("expected an invalid-request message, got {message:?}");
    };
    assert!(
        message.contains("More than one copy of firefox.exe is running"),
        "{message}"
    );
}
