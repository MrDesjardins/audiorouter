//! Tests for `virtual_devices.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn virtual_devices_list_exposes_empty_managed_inventory_without_activation() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "virtualDevices.list".into(),
        params: None,
    });
    let result = response
        .result
        .unwrap_or_else(|| panic!("unexpected response error: {:?}", response.error));
    assert_eq!(result, json!([]));

    let paged = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "virtualDevices.list".into(),
        params: Some(json!({ "limit": 1 })),
    });
    let paged_result = paged
        .result
        .unwrap_or_else(|| panic!("unexpected paged response error: {:?}", paged.error));
    assert_eq!(paged_result, json!({ "items": [], "nextCursor": null }));
}

#[cfg(windows)]
#[test]
fn managed_virtual_device_operations_validate_before_native_access() {
    let mut plane = ControlPlane::default();
    let missing = EntityId::new("missing");
    assert!(matches!(
        plane.provision_virtual_bus_device(&missing, "bus"),
        Err(ControlError::InvalidRequest(message)) if message == "virtual bus not found"
    ));

    let id = EntityId::new("bus");
    plane.create_virtual_bus(id.clone(), "Bus").unwrap();
    plane.set_virtual_bus_enabled(&id, false).unwrap();
    assert!(matches!(
        plane.provision_virtual_bus_device(&id, "bus"),
        Err(ControlError::InvalidRequest(message)) if message == "virtual bus must be enabled before device provisioning"
    ));
    assert!(matches!(
        plane.remove_virtual_bus_device(&id),
        Err(ControlError::InvalidRequest(message)) if message.contains("not tracked")
    ));
}

#[test]
fn virtual_routes_list_exposes_only_explicit_durable_routes() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(8)),
        method: "virtualRoutes.list".into(),
        params: None,
    });
    assert_eq!(
        response.result.unwrap(),
        json!({ "revision": 0, "routes": [] })
    );
    assert!(plane.describe()["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|method| method["name"] == "virtualRoutes.list"));
}

#[test]
fn virtual_routes_replace_is_revision_checked_and_idempotent() {
    let mut plane = ControlPlane::default();
    let request = |key: &str, base_revision| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "virtualRoutes.replace".into(),
        params: Some(json!({
            "baseRevision": base_revision,
            "routes": [],
            "idempotencyKey": key
        })),
    };
    let first = plane.dispatch(request("route-1", 0));
    assert_eq!(
        first.result,
        Some(json!({ "state": "applied", "revision": 1, "routes": [] }))
    );
    assert_eq!(plane.dispatch(request("route-1", 0)).result, first.result);
    assert!(plane.dispatch(request("route-2", 0)).error.is_some());
    let listed = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "virtualRoutes.list".into(),
            params: None,
        })
        .result
        .unwrap();
    assert_eq!(listed["revision"], 1);
}

#[test]
fn virtual_devices_plan_apply_is_revisionless_and_idempotent() {
    let mut plane = ControlPlane::default();
    let planned = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "virtualDevices.plan".into(),
            params: Some(json!({
                "operation": {
                    "action": "create",
                    "id": "bus-1",
                    "name": "Desktop In"
                }
            })),
        })
        .result
        .unwrap();
    assert_eq!(planned["availability"]["status"], "unavailable");
    let plan_id = planned["planId"].as_str().unwrap().to_owned();
    let request = |id, plan_id: &str| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(id)),
        method: "virtualDevices.apply".into(),
        params: Some(json!({ "planId": plan_id, "idempotencyKey": "create-bus-1" })),
    };
    let applied = plane.dispatch(request(4, &plan_id)).result.unwrap();
    assert_eq!(applied["state"], "applied");
    let events = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(40)),
            method: "events.subscribe".into(),
            params: Some(json!({
                "afterSequence": 0,
                "sessionId": "unrelated-session"
            })),
        })
        .result
        .unwrap();
    assert_eq!(events["events"][0]["category"], "virtualDevice.changed");
    let operation = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "operations.get".into(),
            params: Some(json!({ "operationId": "create-bus-1" })),
        })
        .result
        .unwrap();
    assert_eq!(operation["operation"], "virtualDevices.apply");
    let replay = plane.dispatch(request(5, &plan_id)).result.unwrap();
    assert_eq!(replay, applied);
    let second_plan = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "virtualDevices.plan".into(),
            params: Some(json!({
                "operation": {
                    "action": "create",
                    "id": "bus-2",
                    "name": "Desktop Out"
                }
            })),
        })
        .result
        .unwrap()["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let conflict = plane.dispatch(request(6, &second_plan));
    assert_eq!(
        conflict.error.as_ref().unwrap().data.as_ref().unwrap()["code"],
        "idempotencyConflict"
    );
    let inventory = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(7)),
            method: "virtualDevices.list".into(),
            params: None,
        })
        .result
        .unwrap();
    assert_eq!(inventory[0]["name"], "Desktop In");
    assert_eq!(inventory[0]["endpointIds"]["render"], Value::Null);
    assert_eq!(
        inventory[0]["capabilities"],
        json!({ "render": false, "capture": false, "channels": 2 })
    );
    assert_eq!(inventory[0]["privilege"], "deviceAdministration");
    assert_eq!(inventory[0]["restartRequired"], false);
    assert_eq!(inventory[0]["clientImpacts"], json!([]));
}

#[test]
fn virtual_devices_dispatch_enforces_eight_bus_capacity() {
    let mut plane = ControlPlane::default();
    for index in 0..audiorouter_domain::MAX_VIRTUAL_BUSES {
        let planned = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(index as u64)),
            method: "virtualDevices.plan".into(),
            params: Some(json!({
                "operation": {
                    "action": "create",
                    "id": format!("bus-{index}"),
                    "name": format!("Bus {index}")
                }
            })),
        });
        let plan_id = planned.result.unwrap()["planId"]
            .as_str()
            .unwrap()
            .to_owned();
        let applied = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(100 + index as u64)),
            method: "virtualDevices.apply".into(),
            params: Some(json!({
                "planId": plan_id,
                "idempotencyKey": format!("create-bus-{index}")
            })),
        });
        assert_eq!(applied.result.unwrap()["state"], "applied");
    }

    let inventory = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(200)),
            method: "virtualDevices.list".into(),
            params: None,
        })
        .result
        .unwrap();
    assert_eq!(
        inventory.as_array().unwrap().len(),
        audiorouter_domain::MAX_VIRTUAL_BUSES
    );

    let overflow = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(201)),
        method: "virtualDevices.plan".into(),
        params: Some(json!({
            "operation": { "action": "create", "id": "bus-overflow", "name": "Overflow" }
        })),
    });
    assert!(overflow.error.is_some());
    assert!(overflow.error.unwrap().message.contains("LimitReached"));
}

#[test]
fn storage_backed_virtual_device_plan_survives_control_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-virtual-plan-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let plan_id = {
        let mut plane = ControlPlane::with_storage("plan-first", Storage::open(&path).unwrap());
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(7)),
            method: "virtualDevices.plan".into(),
            params: Some(json!({
                "operation": {
                    "action": "create",
                    "id": "bus-1",
                    "name": "Desktop In"
                }
            })),
        });
        response.result.unwrap()["planId"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let mut restarted = ControlPlane::with_storage("plan-second", Storage::open(&path).unwrap());
    let applied = restarted.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(8)),
        method: "virtualDevices.apply".into(),
        params: Some(json!({ "planId": plan_id, "idempotencyKey": "restart-apply" })),
    });
    assert_eq!(applied.result.unwrap()["state"], "applied");
    let operation = restarted.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(8)),
        method: "operations.get".into(),
        params: Some(json!({ "operationId": "restart-apply" })),
    });
    assert_eq!(
        operation.result.unwrap()["operation"],
        "virtualDevices.apply"
    );
    let mut replayed = ControlPlane::with_storage("plan-third", Storage::open(&path).unwrap());
    let replay = replayed.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(9)),
        method: "virtualDevices.apply".into(),
        params: Some(json!({ "planId": plan_id, "idempotencyKey": "restart-apply" })),
    });
    assert_eq!(replay.result.unwrap()["state"], "applied");
    let conflict = replayed.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(10)),
        method: "virtualDevices.apply".into(),
        params: Some(json!({
            "planId": "different-plan",
            "idempotencyKey": "restart-apply"
        })),
    });
    assert_eq!(
        conflict.error.as_ref().unwrap().data.as_ref().unwrap()["code"],
        "idempotencyConflict"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn storage_backed_virtual_bus_inventory_survives_control_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-virtual-bus-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    {
        let mut plane =
            ControlPlane::with_storage("virtual-bus-first", Storage::open(&path).unwrap());
        plane
            .create_virtual_bus(EntityId::new("bus-1"), "Desktop In")
            .unwrap();
        plane
            .set_virtual_bus_enabled(&EntityId::new("bus-1"), false)
            .unwrap();
    }
    let mut restarted =
        ControlPlane::with_storage("virtual-bus-second", Storage::open(&path).unwrap());
    let result = restarted
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "virtualDevices.list".into(),
            params: None,
        })
        .result
        .unwrap();
    assert_eq!(result[0]["id"], "bus-1");
    assert_eq!(result[0]["enabled"], false);
    assert_eq!(result[0]["leaseOwner"], Value::Null);
    let _ = std::fs::remove_file(path);
}

#[test]
fn virtual_bus_lifecycle_keeps_bridge_identity_and_drains_on_disable_delete() {
    let mut plane = ControlPlane::default();
    let id = EntityId::new("bus-stable");
    plane.create_virtual_bus(id.clone(), "Stable bus").unwrap();
    let bridge = plane.virtual_bridges.get(&id).unwrap();
    bridge.activate(1).unwrap();
    plane.set_virtual_bus_enabled(&id, false).unwrap();
    assert!(!bridge.is_active());
    assert!(plane.virtual_bridges.get(&id).is_some());
    plane.delete_virtual_bus(&id).unwrap();
    assert!(plane.virtual_bridges.get(&id).is_none());
}

#[test]
fn virtual_bridge_failure_event_is_discoverable_and_bus_scoped() {
    let mut plane = ControlPlane::default();
    let bus_id = EntityId::new("failed-bus");
    plane.publish_virtual_bridge_failure(&bus_id);
    let event = plane.events.since(0, 10).unwrap().pop().unwrap();
    assert_eq!(event.category, "virtualBridge.failed");
    assert_eq!(event.operation_id.as_deref(), Some("failed-bus"));
    assert!(event.session_id.is_none());
    let description = plane.describe();
    let categories = description["events"]["stateCategories"].as_array().unwrap();
    assert!(categories
        .iter()
        .any(|value| value == "virtualBridge.failed"));
}

#[test]
fn virtual_bridge_lease_expiry_silences_and_drains_managed_route() {
    let mut plane = ControlPlane::default();
    let id = EntityId::new("expiring-bus");
    plane
        .create_virtual_bus(id.clone(), "Expiring bus")
        .unwrap();
    let bridge = plane.virtual_bridges.get(&id).unwrap();
    bridge.activate(1).unwrap();
    bridge.renew_lease(1, 100).unwrap();

    assert_eq!(plane.expire_virtual_bridge_leases(99), 0);
    assert!(bridge.is_active());
    assert_eq!(plane.expire_virtual_bridge_leases(100), 1);
    assert!(!bridge.is_active());
    assert!(bridge.try_receive_capture().is_none());
    let event = plane.events.since(0, 10).unwrap().pop().unwrap();
    assert_eq!(event.category, "virtualBridge.expired");
    assert_eq!(event.operation_id.as_deref(), Some("expiring-bus"));
}

#[test]
fn native_detach_cleanup_deactivates_the_portable_bridge() {
    let mut plane = ControlPlane::default();
    let id = EntityId::new("detach-bus");
    plane.create_virtual_bus(id.clone(), "Detach bus").unwrap();
    let bridge = plane.virtual_bridges.get(&id).unwrap();
    bridge.activate(1).unwrap();
    assert!(bridge.is_active());

    plane.deactivate_virtual_bridge(&id);

    assert!(!bridge.is_active());
    assert!(bridge.try_receive_capture().is_none());
}

#[test]
fn virtual_route_taps_require_an_explicit_sink_and_select_only_matching_enabled_buses() {
    let mut plane = ControlPlane::default();
    let first_bus = EntityId::new("route-bus-1");
    let second_bus = EntityId::new("route-bus-2");
    let third_bus = EntityId::new("route-bus-3");
    plane
        .create_virtual_bus(first_bus.clone(), "First")
        .unwrap();
    plane
        .create_virtual_bus(second_bus.clone(), "Second")
        .unwrap();
    plane
        .create_virtual_bus(third_bus.clone(), "Third")
        .unwrap();
    let routes = VirtualBusRouteRegistry::new(vec![
        audiorouter_domain::VirtualBusRoute {
            bus_id: first_bus.clone(),
            producer_session_id: EntityId::new("producer"),
            consumer_session_id: EntityId::new("consumer"),
        },
        audiorouter_domain::VirtualBusRoute {
            bus_id: second_bus.clone(),
            producer_session_id: EntityId::new("other-producer"),
            consumer_session_id: EntityId::new("consumer"),
        },
        audiorouter_domain::VirtualBusRoute {
            bus_id: third_bus.clone(),
            producer_session_id: EntityId::new("producer"),
            consumer_session_id: EntityId::new("consumer"),
        },
    ])
    .unwrap();
    plane.virtual_bus_routes = routes;
    plane.virtual_bus_route_revision = 1;
    assert!(plane
        .virtual_route_tap_set(&EntityId::new("producer"), &[], 1)
        .unwrap()
        .is_empty());
    plane
        .virtual_bridges
        .get(&first_bus)
        .unwrap()
        .activate(1)
        .unwrap();
    plane
        .virtual_bridges
        .get(&third_bus)
        .unwrap()
        .activate(1)
        .unwrap();
    assert_eq!(
        plane
            .virtual_route_tap_set(
                &EntityId::new("producer"),
                &[first_bus.clone(), third_bus.clone()],
                1,
            )
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        plane
            .virtual_route_tap_set(
                &EntityId::new("producer"),
                std::slice::from_ref(&first_bus),
                1,
            )
            .unwrap()
            .len(),
        1
    );
    assert!(plane
        .virtual_route_tap_set(
            &EntityId::new("producer"),
            std::slice::from_ref(&second_bus),
            1,
        )
        .unwrap()
        .is_empty());
}

#[test]
fn virtual_route_bridge_activation_follows_sink_generation_and_stop_clears_it() {
    let mut plane = ControlPlane::default();
    let bus_id = EntityId::new("generation-bus");
    let producer = EntityId::new("generation-producer");
    let mut producer_session = session();
    producer_session.id = producer.clone();
    let mut consumer_session = session();
    consumer_session.id = EntityId::new("generation-consumer");
    plane.insert_session(producer_session).unwrap();
    plane.insert_session(consumer_session).unwrap();
    plane
        .create_virtual_bus(bus_id.clone(), "Generation")
        .unwrap();
    plane
        .replace_virtual_bus_routes(
            VirtualBusRouteRegistry::new(vec![audiorouter_domain::VirtualBusRoute {
                bus_id: bus_id.clone(),
                producer_session_id: producer.clone(),
                consumer_session_id: EntityId::new("generation-consumer"),
            }])
            .unwrap(),
        )
        .unwrap();

    let bridge = plane.virtual_bridges.get(&bus_id).unwrap();
    assert!(!bridge.is_active());
    plane
        .prepare_virtual_route_bridges(&producer, 7, std::slice::from_ref(&bus_id))
        .unwrap();
    assert!(bridge.is_active());
    assert_eq!(bridge.generation(), 7);

    plane
        .prepare_virtual_route_bridges(&producer, 8, &[])
        .unwrap();
    assert!(!bridge.is_active());
    assert_eq!(bridge.generation(), 7);

    plane
        .prepare_virtual_route_bridges(&producer, 8, std::slice::from_ref(&bus_id))
        .unwrap();
    assert!(bridge.is_active());
    plane.delete_session(&producer).unwrap();
    assert!(!bridge.is_active());
}

#[test]
fn virtual_route_bridge_rejects_stale_reactivation_generation() {
    let mut plane = ControlPlane::default();
    let bus_id = EntityId::new("stale-generation-bus");
    let producer = EntityId::new("stale-generation-producer");
    let mut producer_session = session();
    producer_session.id = producer.clone();
    let mut consumer_session = session();
    consumer_session.id = EntityId::new("stale-generation-consumer");
    plane.insert_session(producer_session).unwrap();
    plane.insert_session(consumer_session).unwrap();
    plane.create_virtual_bus(bus_id.clone(), "Stale").unwrap();
    plane
        .replace_virtual_bus_routes(
            VirtualBusRouteRegistry::new(vec![audiorouter_domain::VirtualBusRoute {
                bus_id: bus_id.clone(),
                producer_session_id: producer.clone(),
                consumer_session_id: EntityId::new("stale-generation-consumer"),
            }])
            .unwrap(),
        )
        .unwrap();
    plane
        .prepare_virtual_route_bridges(&producer, 4, std::slice::from_ref(&bus_id))
        .unwrap();
    plane
        .prepare_virtual_route_bridges(&producer, 4, &[])
        .unwrap();
    let error = plane.prepare_virtual_route_bridges(&producer, 3, &[bus_id]);
    assert!(
        matches!(error, Err(ControlError::InvalidRequest(message)) if message.contains("stale"))
    );
}

#[test]
fn virtual_bus_routes_require_known_buses_and_protect_referenced_deletion() {
    let mut plane = ControlPlane::default();
    plane.insert_session(session()).unwrap();
    let mut consumer = session();
    consumer.id = EntityId::new("consumer");
    plane.insert_session(consumer).unwrap();
    let bus_id = EntityId::new("bus-route");
    plane
        .create_virtual_bus(bus_id.clone(), "Route bus")
        .unwrap();
    let route = audiorouter_domain::VirtualBusRoute {
        bus_id: bus_id.clone(),
        producer_session_id: EntityId::new("session"),
        consumer_session_id: EntityId::new("consumer"),
    };
    plane
        .replace_virtual_bus_routes(
            audiorouter_domain::VirtualBusRouteRegistry::new(vec![route]).unwrap(),
        )
        .unwrap();
    assert_eq!(plane.virtual_bus_routes().list().len(), 1);
    assert!(plane.delete_virtual_bus(&bus_id).is_err());
    assert!(plane
        .replace_virtual_bus_routes(
            audiorouter_domain::VirtualBusRouteRegistry::new(vec![
                audiorouter_domain::VirtualBusRoute {
                    bus_id: EntityId::new("missing"),
                    producer_session_id: EntityId::new("session"),
                    consumer_session_id: EntityId::new("consumer"),
                },
            ])
            .unwrap(),
        )
        .is_err());
}

#[test]
fn virtual_device_lifecycle_requires_explicit_device_administration_scope() {
    let request = |method: &str, params: Option<Value>| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(40)),
        method: method.into(),
        params,
    };
    for grant in [
        ClientGrant::read_only(),
        ClientGrant::for_role(ClientRole::Editor),
        ClientGrant::for_role(ClientRole::Operator),
    ] {
        let mut plane = ControlPlane::default();
        let plan = plane.dispatch_authorized(
            request(
                "virtualDevices.plan",
                Some(json!({ "operation": { "action": "create", "id": "bus-1", "name": "Desktop" } })),
            ),
            &grant,
        );
        assert_eq!(plan.error.unwrap().code, -32001);
        let apply = plane.dispatch_authorized(
            request(
                "virtualDevices.apply",
                Some(json!({ "planId": "plan-1", "idempotencyKey": "key-1" })),
            ),
            &grant,
        );
        assert_eq!(apply.error.unwrap().code, -32001);
        let provision = plane.dispatch_authorized(
            request(
                "virtualDevices.provision",
                Some(json!({ "busId": "bus-1", "instanceId": "instance-1", "idempotencyKey": "key-2" })),
            ),
            &grant,
        );
        assert_eq!(provision.error.unwrap().code, -32001);
        let remove = plane.dispatch_authorized(
            request(
                "virtualDevices.remove",
                Some(json!({ "busId": "bus-1", "idempotencyKey": "key-3" })),
            ),
            &grant,
        );
        assert_eq!(remove.error.unwrap().code, -32001);
    }
    let mut plane = ControlPlane::default();
    let grant = ClientGrant::with_scopes([PermissionScope::DeviceAdministration]);
    let response = plane.dispatch_authorized(
        request(
            "virtualDevices.plan",
            Some(json!({ "operation": { "action": "create", "id": "bus-1", "name": "Desktop" } })),
        ),
        &grant,
    );
    assert!(response.result.is_some());
}
