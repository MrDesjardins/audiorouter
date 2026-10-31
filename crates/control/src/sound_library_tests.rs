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
