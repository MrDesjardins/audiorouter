//! Tests for `sound_library.rs`.

use super::*;
use crate::test_support::*;

/// The bundled library's sound ids in `category`, read from the manifest the
/// test finds the same way the program does.
fn manifest_category(category: &str) -> Vec<String> {
    let root = library_root().expect("the repository sound library");
    category_sounds(read_manifest(&root).ok().as_ref(), category)
}

#[test]
fn category_sounds_are_sorted_ids_of_that_category_only() {
    let manifest = json!({ "sounds": [
        { "id": "lib:b", "category": "horns" },
        { "id": "lib:a", "category": "horns" },
        { "id": "lib:c", "category": "hornsx" },
        { "id": "not-a-library-id", "category": "horns" },
    ]});
    assert_eq!(
        category_sounds(Some(&manifest), "horns"),
        vec!["lib:a".to_owned(), "lib:b".to_owned()]
    );
    assert!(category_sounds(Some(&manifest), "missing").is_empty());
    assert!(category_sounds(None, "horns").is_empty());
    let many = json!({ "sounds": (0..100)
        .map(|index| json!({ "id": format!("lib:{index:03}"), "category": "big" }))
        .collect::<Vec<_>>() });
    assert_eq!(
        category_sounds(Some(&many), "big").len(),
        audiorouter_engine::MAX_SLOT_SOUNDS
    );
}

/// A slot whose set names a library category plays that category through
/// the real entry points: `nativePaths.prepare` and `session.start` through
/// `ControlPlane::dispatch`, then `soundboard.status` and
/// `soundboard.trigger`. Before the expansion the slot was reported empty.
/// The route ends in Network Send, so no audio device is needed.
#[cfg(windows)]
#[test]
fn a_category_set_slot_loads_and_plays_through_the_dispatcher() {
    let category = "horns-alerts";
    let expected = manifest_category(category);
    assert!(expected.len() > 1, "the library has a {category} category");
    let port_number = std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let node = |id: &str, kind, ports, parameters: Value| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(parameters).unwrap(),
        ports,
    };
    let session = Session {
        id: EntityId::new("category-board"),
        name: "Category board".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            node(
                "board",
                NodeKind::Soundboard,
                vec![port("out", PortDirection::Output)],
                json!({
                    "slot1Set": category,
                    "slot1Caption": "Random horn",
                    "slot2Set": "no-such-category",
                }),
            ),
            node(
                "net-send",
                NodeKind::NetworkSend,
                vec![port("in", PortDirection::Input)],
                json!({ "host": "127.0.0.1", "port": port_number }),
            ),
        ],
        edges: vec![audiorouter_domain::Edge {
            id: EntityId::new("board-net-send"),
            source_node: EntityId::new("board"),
            source_port: "out".into(),
            destination_node: EntityId::new("net-send"),
            destination_port: "in".into(),
            matrix: vec![1.0, 0.0, 0.0, 1.0],
            enabled: true,
        }],
    };
    let mut plane = ControlPlane::default();
    plane.insert_session(session).unwrap();
    let call = |plane: &mut ControlPlane, method: &str, params: Value| {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        });
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
        response.result.unwrap()
    };
    let ids = json!({ "sessionId": "category-board", "nodeId": "board" });
    call(
        &mut plane,
        "nativePaths.prepare",
        json!({ "sessionId": "category-board" }),
    );
    call(
        &mut plane,
        "session.start",
        json!({ "sessionId": "category-board", "idempotencyKey": "start" }),
    );
    let status = call(&mut plane, "soundboard.status", ids.clone());
    assert_eq!(status["slots"][0]["loaded"], true, "{status}");
    assert_eq!(
        status["slots"][1]["loaded"], false,
        "an unknown category with no sound stays empty: {status}"
    );
    let source = plane
        .soundboard_sources
        .get(&(EntityId::new("category-board"), EntityId::new("board")))
        .cloned()
        .unwrap();
    assert_eq!(source.slot_sound_count(0), expected.len());

    let trigger = call(
        &mut plane,
        "soundboard.trigger",
        json!({ "sessionId": "category-board", "nodeId": "board", "slot": 1 }),
    );
    assert_eq!(trigger["accepted"], true, "{trigger}");
    let until = std::time::Instant::now() + std::time::Duration::from_millis(300);
    let mut playing = false;
    while std::time::Instant::now() < until && !playing {
        plane.service_running_native_audio(std::time::Instant::now());
        std::thread::sleep(std::time::Duration::from_millis(1));
        playing = call(&mut plane, "soundboard.status", ids.clone())["slots"][0]["playing"] == true;
    }
    assert!(playing, "the category slot plays after a press");
    call(&mut plane, "soundboard.stop", ids);
}

/// An assistant (MCP) or script addresses a Soundboard by its name and
/// leaves out `sessionId`: the backend resolves the active session and the
/// name (`resolve_node_target`). Before, every such call failed with
/// "sessionId is required". IDs keep working, so Stream Deck is unchanged.
#[cfg(windows)]
#[test]
fn soundboard_calls_resolve_the_active_session_and_a_node_name() {
    let port_number = std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let node = |id: &str, name: &str, kind, ports, parameters: Value| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: name.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(parameters).unwrap(),
        ports,
    };
    let session = Session {
        id: EntityId::new("named-board"),
        name: "Named board".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            node(
                "board",
                "Party Board",
                NodeKind::Soundboard,
                vec![port("out", PortDirection::Output)],
                json!({ "slot1Set": "horns-alerts" }),
            ),
            node(
                "net-send",
                "Network send",
                NodeKind::NetworkSend,
                vec![port("in", PortDirection::Input)],
                json!({ "host": "127.0.0.1", "port": port_number }),
            ),
        ],
        edges: vec![audiorouter_domain::Edge {
            id: EntityId::new("board-net-send"),
            source_node: EntityId::new("board"),
            source_port: "out".into(),
            destination_node: EntityId::new("net-send"),
            destination_port: "in".into(),
            matrix: vec![1.0, 0.0, 0.0, 1.0],
            enabled: true,
        }],
    };
    let mut plane = ControlPlane::default();
    plane.insert_session(session).unwrap();
    let request = |plane: &mut ControlPlane, method: &str, params: Value| {
        plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        })
    };
    let call = |plane: &mut ControlPlane, method: &str, params: Value| {
        let response = request(plane, method, params);
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
        response.result.unwrap()
    };
    call(
        &mut plane,
        "sessions.active.set",
        json!({ "sessionId": "named-board", "idempotencyKey": "open" }),
    );
    call(
        &mut plane,
        "nativePaths.prepare",
        json!({ "sessionId": "named-board" }),
    );
    call(
        &mut plane,
        "session.start",
        json!({ "sessionId": "named-board", "idempotencyKey": "start" }),
    );

    let by_name = json!({ "nodeId": "party board" });
    let status = call(&mut plane, "soundboard.status", by_name.clone());
    assert_eq!(status["sessionId"], "named-board", "{status}");
    assert_eq!(status["nodeId"], "board", "the name resolves to the ID");
    assert_eq!(status["slots"][0]["loaded"], true, "{status}");
    let trigger = call(
        &mut plane,
        "soundboard.trigger",
        json!({ "nodeId": "Party Board", "slot": 1 }),
    );
    assert_eq!(trigger["accepted"], true, "{trigger}");
    call(&mut plane, "soundboard.stop", by_name);
    // The ID form Stream Deck sends still works.
    call(
        &mut plane,
        "soundboard.status",
        json!({ "sessionId": "named-board", "nodeId": "board" }),
    );

    let unknown = request(
        &mut plane,
        "soundboard.status",
        json!({ "nodeId": "No such board" }),
    );
    let message = unknown.error.expect("an unknown name is refused").message;
    assert!(
        message.contains("no node named") && message.contains("Party Board"),
        "the refusal lists the session's nodes: {message}"
    );
}

/// Two boards share a category and a sound and the second adds one more.
/// The manifest is read once for the whole load (it was read again for every
/// library sound), each sound is decoded once, and the session budget counts
/// across boards: a budget that fits the first board refuses the second
/// (review 2026-11-01, P2-8 and P3).
#[test]
fn soundboard_load_reads_the_manifest_once_and_bounds_the_session() {
    let horns = manifest_category("horns-alerts");
    assert!(horns.len() > 1, "the library has a horns-alerts category");
    let board = |id: &str, parameters: Value| Node {
        id: EntityId::new(id),
        kind: NodeKind::Soundboard,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(parameters).unwrap(),
        ports: vec![Port {
            name: "out".into(),
            direction: PortDirection::Output,
            channels: 2,
        }],
    };
    let session = Session {
        id: EntityId::new("two-boards"),
        name: "Two boards".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            board(
                "a",
                json!({ "slot1Set": "horns-alerts", "slot2MediaId": "lib:applause-crowd-01" }),
            ),
            board(
                "b",
                json!({
                    "slot1Set": "horns-alerts",
                    "slot2MediaId": "lib:applause-crowd-01",
                    "slot3MediaId": "lib:whoosh",
                    "slot4MediaId": "lib:no-such-sound",
                }),
            ),
        ],
        edges: Vec::new(),
    };
    let extra = manifest_category("whooshes");
    assert_eq!(extra, vec!["lib:whoosh".to_owned()], "the library's whoosh");

    let mut plane = ControlPlane::default();
    let mut media = HashMap::new();
    let stats = plane
        .load_soundboard_media_with(&session, 48_000, &mut media, SoundboardBudgets::default())
        .unwrap();
    assert_eq!(stats.manifest_reads, 1);
    assert_eq!(
        stats.decoded,
        horns.len() + 2,
        "each sound is decoded once; the missing one is skipped"
    );
    assert!(media.contains_key("lib:whoosh"));
    assert!(!media.contains_key("lib:no-such-sound"));

    let bytes = |id: &str| media[id].samples.len() * std::mem::size_of::<f32>();
    let first_board =
        horns.iter().map(|id| bytes(id)).sum::<usize>() + bytes("lib:applause-crowd-01");
    let refused = |budgets| {
        ControlPlane::default()
            .load_soundboard_media_with(&session, 48_000, &mut HashMap::new(), budgets)
            .unwrap_err()
    };
    match refused(SoundboardBudgets {
        board_bytes: usize::MAX,
        session_bytes: first_board,
    }) {
        ControlError::NodeNotReady { node_id, message } => {
            assert_eq!(node_id, "b", "the first board fits, the second does not");
            assert!(message.contains("together"), "{message}");
        }
        other => panic!("unexpected error {other:?}"),
    }
    match refused(SoundboardBudgets {
        board_bytes: first_board - 1,
        session_bytes: usize::MAX,
    }) {
        ControlError::NodeNotReady { node_id, message } => {
            assert_eq!(node_id, "a");
            assert!(message.contains("\"a\" holds more sound"), "{message}");
        }
        other => panic!("unexpected error {other:?}"),
    }
    // A route with no library sound never reads the manifest.
    let own = Session {
        nodes: vec![board("c", json!({ "slot1MediaId": "user-sound" }))],
        ..session.clone()
    };
    let stats = ControlPlane::default()
        .load_soundboard_media_with(
            &own,
            48_000,
            &mut HashMap::new(),
            SoundboardBudgets::default(),
        )
        .unwrap();
    assert_eq!(stats, SoundboardLoadStats::default());
}
