//! Named tool presets through JSON-RPC dispatch.

use super::*;

fn call(plane: &mut ControlPlane, method: &str, params: Value) -> Result<Value, String> {
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    });
    match response.error {
        Some(error) => Err(error.message),
        None => Ok(response.result.expect("a result")),
    }
}

fn save(
    plane: &mut ControlPlane,
    kind: &str,
    name: &str,
    parameters: Value,
    key: &str,
) -> Result<Value, String> {
    call(
        plane,
        "toolPresets.save",
        json!({ "kind": kind, "name": name, "parameters": parameters, "idempotencyKey": key }),
    )
}

#[test]
fn a_preset_is_saved_listed_by_kind_replaced_by_name_and_deleted() {
    let mut plane = ControlPlane::new("test");
    save(&mut plane, "gain", "Quiet", json!({ "gainDb": -6.0 }), "a").unwrap();
    save(&mut plane, "gate", "Room", json!({}), "b").unwrap();
    let list = call(&mut plane, "toolPresets.list", json!({ "kind": "gain" })).unwrap();
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
    assert_eq!(list["items"][0]["parameters"]["gainDb"], -6.0);

    save(&mut plane, "gain", "quiet", json!({ "gainDb": -9.0 }), "c").unwrap();
    let list = call(&mut plane, "toolPresets.list", json!({ "kind": "gain" })).unwrap();
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
    assert_eq!(list["items"][0]["parameters"]["gainDb"], -9.0);

    let id = list["items"][0]["id"].clone();
    let deleted = call(
        &mut plane,
        "toolPresets.delete",
        json!({ "id": id, "idempotencyKey": "d" }),
    )
    .unwrap();
    assert_eq!(deleted["deleted"], true);
    let list = call(&mut plane, "toolPresets.list", json!({})).unwrap();
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
}

#[test]
fn settings_are_checked_with_the_same_rules_as_a_saved_route() {
    let mut plane = ControlPlane::new("test");
    let message = save(&mut plane, "gain", "Bad", json!({ "gainDb": "loud" }), "a").unwrap_err();
    assert!(message.contains("not valid for this tool"), "{message}");
}

#[test]
fn tools_that_identify_a_device_or_file_take_no_presets() {
    let mut plane = ControlPlane::new("test");
    for kind in [
        "physicalInput",
        "physicalOutput",
        "audioFile",
        "plugin",
        "networkSend",
    ] {
        let message = save(&mut plane, kind, "Mine", json!({}), kind).unwrap_err();
        assert!(message.contains("does not take presets"), "{message}");
    }
}

#[test]
fn names_are_bounded_and_required() {
    let mut plane = ControlPlane::new("test");
    assert!(save(&mut plane, "gain", "  ", json!({}), "a").is_err());
    let long = "x".repeat(65);
    assert!(save(&mut plane, "gain", &long, json!({}), "b").is_err());
}
