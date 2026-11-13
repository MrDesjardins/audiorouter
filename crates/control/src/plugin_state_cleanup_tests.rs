//! Session deletion owns automatic plugin captures, not reusable library states.
use super::*;
use crate::test_support::session;

#[test]
fn deleting_sessions_reclaims_only_unreferenced_automatic_plugin_captures() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-state-cleanup-{}-{}",
        std::process::id(),
        unix_epoch_millis()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let database = root.join("state.sqlite");
    let storage = Storage::open(&database).unwrap();
    let original = session();
    storage.save_session(&original).unwrap();
    let mut other = original.clone();
    other.id = EntityId::new("other");
    storage.save_session(&other).unwrap();
    let assets = storage.plugin_state_directory().unwrap();
    for id in [
        "plugin-autostate-solo",
        "plugin-autostate-shared",
        "plugin-autostate-history",
        "plugin-state-manual",
    ] {
        let path = assets.join(format!("{id}.bin"));
        std::fs::write(&path, b"state").unwrap();
        std::fs::write(plugin_slider_snapshot_path(&assets, id), b"{}").unwrap();
        storage
            .save_plugin_state(&audiorouter_storage::PluginStateRecord {
                id: id.into(),
                plugin_id: "plugin-fixture".into(),
                plugin_sha256: "a".repeat(64),
                version: 1,
                path: path.to_string_lossy().into_owned(),
                state_sha256: "b".repeat(64),
                size_bytes: 5,
            })
            .unwrap();
        storage
            .set_plugin_node_state(original.id.as_str(), id, id)
            .unwrap();
    }
    storage
        .set_plugin_node_state(other.id.as_str(), "fx", "plugin-autostate-shared")
        .unwrap();
    other.nodes.push(audiorouter_domain::Node {
        id: EntityId::new("history-plugin"), kind: NodeKind::Plugin, type_version: 1, name: "Saved plugin".into(), enabled: false, bypass: false,
        parameters: serde_json::from_value(json!({"path": root.join("fixture.vst3"), "format": "vst3", "fingerprint": "a".repeat(64), "classId": "default", "stateId": "plugin-autostate-history"})).unwrap(),
        ports: vec![],
    });
    storage.save_session(&other).unwrap();
    let mut plane = ControlPlane::with_storage("state-cleanup", storage);
    let mut edited = other.clone();
    edited.nodes.retain(|node| node.kind != NodeKind::Plugin);
    let plan = plane.plan_graph(&other.id, 0, edited).unwrap();
    plane.commit_graph(&plan, 0, "remove-placeholder").unwrap();
    for id in [&original.id, &other.id] {
        let result = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "sessions.delete".into(),
                params: Some(
                    json!({"sessionId": id, "idempotencyKey": format!("delete-{}", id.as_str())}),
                ),
            })
            .result
            .unwrap();
        assert_eq!(result["deleted"], true);
        assert!(plane
            .storage
            .as_ref()
            .unwrap()
            .plugin_node_states(id.as_str())
            .unwrap()
            .is_empty());
        assert!(!assets.join("plugin-autostate-solo.bin").exists());
        assert!(!plugin_slider_snapshot_path(&assets, "plugin-autostate-solo").exists());
        assert_eq!(
            assets.join("plugin-autostate-shared.bin").exists(),
            id == &original.id
        );
        assert_eq!(
            assets.join("plugin-autostate-history.bin").exists(),
            id == &original.id,
            "saved versions retain their referenced state"
        );
        assert!(
            assets.join("plugin-state-manual.bin").exists(),
            "explicit library states are reusable"
        );
    }
    assert_eq!(
        plane
            .storage
            .as_ref()
            .unwrap()
            .list_plugin_states(None)
            .unwrap()
            .len(),
        1
    );
    drop(plane);
    std::fs::remove_file(assets.join("plugin-state-manual.bin")).unwrap();
    std::fs::remove_file(plugin_slider_snapshot_path(&assets, "plugin-state-manual")).unwrap();
    std::fs::remove_dir(assets).unwrap();
    std::fs::remove_file(database).unwrap();
    std::fs::remove_dir(root).unwrap();
}
