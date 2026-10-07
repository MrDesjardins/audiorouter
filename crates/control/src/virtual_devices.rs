//! Virtual buses, virtual devices, virtual routes and their bridges.

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum VirtualBusOperation {
    Create { id: EntityId, name: String },
    Rename { id: EntityId, name: String },
    SetEnabled { id: EntityId, enabled: bool },
    Delete { id: EntityId },
}

#[derive(Clone, Debug)]
pub(crate) struct VirtualBusPlan {
    pub(crate) operation: VirtualBusOperation,
    pub(crate) expires_at: Instant,
}

pub(crate) fn session_virtual_capture_bus_ids(session: &Session) -> Vec<EntityId> {
    session
        .nodes
        .iter()
        .filter(|node| node.enabled && node.kind == NodeKind::VirtualCaptureSink)
        .filter_map(|node| {
            node.parameters
                .get("busId")
                .and_then(serde_json::Value::as_str)
                .map(EntityId::new)
        })
        .collect()
}

pub(crate) fn virtual_bus_operation_from_value(
    value: &Value,
) -> Result<VirtualBusOperation, ControlError> {
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| ControlError::InvalidRequest("operation.action is required".into()))?;
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(EntityId::new)
        .ok_or_else(|| ControlError::InvalidRequest("operation.id is required".into()))?;
    match action {
        "create" => Ok(VirtualBusOperation::Create {
            id,
            name: value
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| ControlError::InvalidRequest("operation.name is required".into()))?
                .to_owned(),
        }),
        "rename" => Ok(VirtualBusOperation::Rename {
            id,
            name: value
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| ControlError::InvalidRequest("operation.name is required".into()))?
                .to_owned(),
        }),
        "setEnabled" => Ok(VirtualBusOperation::SetEnabled {
            id,
            enabled: value
                .get("enabled")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    ControlError::InvalidRequest("operation.enabled is required".into())
                })?,
        }),
        "delete" => Ok(VirtualBusOperation::Delete { id }),
        _ => Err(ControlError::InvalidRequest(
            "operation.action must be create, rename, setEnabled, or delete".into(),
        )),
    }
}

pub(crate) fn apply_virtual_bus_operation(
    registry: &mut VirtualBusRegistry,
    operation: &VirtualBusOperation,
) -> Result<(), ControlError> {
    let result = match operation {
        VirtualBusOperation::Create { id, name } => registry.create(id.clone(), name),
        VirtualBusOperation::Rename { id, name } => registry.rename(id, name),
        VirtualBusOperation::SetEnabled { id, enabled } => registry.set_enabled(id, *enabled),
        VirtualBusOperation::Delete { id } => registry.delete(id),
    };
    result.map_err(virtual_bus_control_error)
}

pub(crate) fn virtual_bus_operation_value(operation: &VirtualBusOperation) -> Value {
    match operation {
        VirtualBusOperation::Create { id, name } => {
            json!({ "action": "create", "id": id, "name": name })
        }
        VirtualBusOperation::Rename { id, name } => {
            json!({ "action": "rename", "id": id, "name": name })
        }
        VirtualBusOperation::SetEnabled { id, enabled } => {
            json!({ "action": "setEnabled", "id": id, "enabled": enabled })
        }
        VirtualBusOperation::Delete { id } => json!({ "action": "delete", "id": id }),
    }
}

pub(crate) fn virtual_bus_control_error(
    error: audiorouter_domain::VirtualBusError,
) -> ControlError {
    ControlError::InvalidRequest(format!("virtual bus operation rejected: {error:?}"))
}

#[cfg(windows)]
pub(crate) fn software_device_control_error(
    error: audiorouter_windows_audio::SoftwareDeviceError,
) -> ControlError {
    ControlError::InvalidRequest(format!("software device operation rejected: {error}"))
}

pub(crate) fn virtual_bridge_control_error(error: VirtualBusBridgeSetError) -> ControlError {
    let message = match error {
        VirtualBusBridgeSetError::InvalidCapacity => "invalid virtual bridge capacity",
        VirtualBusBridgeSetError::MissingBus => "virtual bridge is not registered",
        VirtualBusBridgeSetError::CapacityReached => "virtual bridge capacity reached",
        VirtualBusBridgeSetError::Queue(_) => "virtual bridge queue shape is invalid",
    };
    ControlError::InvalidRequest(message.into())
}

pub(crate) fn virtual_device_request_hash(plan_id: &str) -> String {
    let fingerprint = format!("virtualDevices.apply:{plan_id}");
    format!("{:x}", Sha256::digest(fingerprint.as_bytes()))
}

pub(crate) fn virtual_device_external_request_hash(
    method: &str,
    bus_id: &str,
    instance_id: Option<&str>,
) -> String {
    let fingerprint = format!("{method}:{}:{}", bus_id, instance_id.unwrap_or_default());
    format!("{:x}", Sha256::digest(fingerprint.as_bytes()))
}

impl ControlPlane {
    pub fn create_virtual_bus(
        &mut self,
        id: EntityId,
        name: impl Into<String>,
    ) -> Result<(), ControlError> {
        let bridge_id = id.clone();
        let checkpoint = self.virtual_buses.clone();
        self.virtual_buses
            .create(id, name)
            .map_err(virtual_bus_control_error)?;
        if let Err(error) = self.virtual_bridges.ensure(bridge_id.clone()) {
            self.virtual_buses = checkpoint;
            return Err(virtual_bridge_control_error(error));
        }
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses(&self.virtual_buses) {
                self.virtual_buses = checkpoint;
                let _ = self.virtual_bridges.remove(&bridge_id);
                return Err(storage_error(error));
            }
        }
        Ok(())
    }

    /// Expire managed virtual bridge leases on the control/recovery thread.
    /// `now_tick` must use the same monotonic domain supplied to bridge lease
    /// renewal; expiry silences and drains stale buffers without touching a
    /// Windows endpoint or changing durable desired state.
    pub fn expire_virtual_bridge_leases(&mut self, now_tick: u64) -> usize {
        let expired = self.virtual_bridges.expire_stale_lease_ids(now_tick);
        for bus_id in &expired {
            self.events.append(
                0,
                Some(bus_id.as_str().to_owned()),
                "virtualBridge.expired",
                None,
            );
        }
        expired.len()
    }

    pub(crate) fn publish_virtual_bridge_failure(&mut self, bus_id: &EntityId) {
        self.events.append(
            0,
            Some(bus_id.as_str().to_owned()),
            "virtualBridge.failed",
            None,
        );
    }

    pub fn rename_virtual_bus(
        &mut self,
        id: &EntityId,
        name: impl Into<String>,
    ) -> Result<(), ControlError> {
        let checkpoint = self.virtual_buses.clone();
        self.virtual_buses
            .rename(id, name)
            .map_err(virtual_bus_control_error)?;
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses(&self.virtual_buses) {
                self.virtual_buses = checkpoint;
                return Err(storage_error(error));
            }
        }
        Ok(())
    }

    pub fn set_virtual_bus_enabled(
        &mut self,
        id: &EntityId,
        enabled: bool,
    ) -> Result<(), ControlError> {
        let checkpoint = self.virtual_buses.clone();
        self.virtual_buses
            .set_enabled(id, enabled)
            .map_err(virtual_bus_control_error)?;
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses(&self.virtual_buses) {
                self.virtual_buses = checkpoint;
                return Err(storage_error(error));
            }
        }
        if !enabled {
            if let Some(bridge) = self.virtual_bridges.get(id) {
                bridge.deactivate();
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    pub(crate) fn provision_virtual_bus_device_unpersisted(
        &mut self,
        id: &EntityId,
        instance_id: &str,
    ) -> Result<String, ControlError> {
        let bus = self
            .virtual_buses
            .list()
            .iter()
            .find(|bus| bus.id() == id)
            .ok_or_else(|| ControlError::InvalidRequest("virtual bus not found".into()))?;
        if !bus.enabled() {
            return Err(ControlError::InvalidRequest(
                "virtual bus must be enabled before device provisioning".into(),
            ));
        }
        if bus.driver_instance_id().is_some() {
            return Err(ControlError::InvalidRequest(
                "virtual bus already has a driver instance identity".into(),
            ));
        }

        let provisioner = audiorouter_windows_audio::SoftwareDeviceProvisioner;
        let returned_instance_id = self
            .managed_software_devices
            .create(&provisioner, id.as_str(), instance_id)
            .map_err(software_device_control_error)?
            .instance_id()
            .to_owned();
        if let Err(error) = self
            .virtual_buses
            .set_driver_instance_id(id, returned_instance_id.clone())
        {
            let _ = self.managed_software_devices.remove(id.as_str());
            return Err(virtual_bus_control_error(error));
        }
        Ok(returned_instance_id)
    }

    /// Provision one explicitly selected managed bus and retain its native
    /// Software Device API handle. This is a Windows-only control-plane
    /// operation; authorization and stopped-route policy belong to its caller.
    #[cfg(windows)]
    pub fn provision_virtual_bus_device(
        &mut self,
        id: &EntityId,
        instance_id: &str,
    ) -> Result<String, ControlError> {
        let checkpoint = self.virtual_buses.clone();
        let returned_instance_id =
            self.provision_virtual_bus_device_unpersisted(id, instance_id)?;
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses(&self.virtual_buses) {
                self.virtual_buses = checkpoint;
                let _ = self.managed_software_devices.remove(id.as_str());
                return Err(storage_error(error));
            }
        }
        Ok(returned_instance_id)
    }

    #[cfg(windows)]
    pub(crate) fn remove_virtual_bus_device_unpersisted(
        &mut self,
        id: &EntityId,
    ) -> Result<(), ControlError> {
        if !self.managed_software_devices.contains(id.as_str()) {
            return Err(software_device_control_error(
                audiorouter_windows_audio::SoftwareDeviceError::NotTracked,
            ));
        }
        self.virtual_buses
            .clear_driver_instance_id(id)
            .map_err(virtual_bus_control_error)?;
        Ok(())
    }

    /// Remove one explicitly owned native software device after clearing its
    /// persisted identity. The handle is dropped only after durable state is
    /// updated, so a storage failure leaves the native owner recoverable.
    #[cfg(windows)]
    pub fn remove_virtual_bus_device(&mut self, id: &EntityId) -> Result<(), ControlError> {
        let checkpoint = self.virtual_buses.clone();
        self.remove_virtual_bus_device_unpersisted(id)?;
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses(&self.virtual_buses) {
                self.virtual_buses = checkpoint;
                return Err(storage_error(error));
            }
        }
        if let Err(error) = self.managed_software_devices.remove(id.as_str()) {
            self.virtual_buses = checkpoint;
            if let Some(storage) = &self.storage {
                let _ = storage.save_virtual_buses(&self.virtual_buses);
            }
            return Err(software_device_control_error(error));
        }
        Ok(())
    }

    /// Replace the explicit cross-session virtual-bus route set. Validation
    /// covers bus references and the global session graph before durable state
    /// is changed; no endpoint or bridge is activated by this method.
    pub fn replace_virtual_bus_routes(
        &mut self,
        routes: VirtualBusRouteRegistry,
    ) -> Result<(), ControlError> {
        let revision = self
            .virtual_bus_route_revision
            .checked_add(1)
            .ok_or_else(|| ControlError::InvalidRequest("route revision exhausted".into()))?;
        self.replace_virtual_bus_routes_at_revision(routes, revision)
    }

    pub(crate) fn replace_virtual_bus_routes_at_revision(
        &mut self,
        routes: VirtualBusRouteRegistry,
        revision: u64,
    ) -> Result<(), ControlError> {
        self.validate_virtual_bus_routes(&routes)?;
        if let Some(storage) = &self.storage {
            storage
                .save_virtual_bus_route_state(&routes, revision)
                .map_err(storage_error)?;
        }
        self.virtual_bus_routes = routes;
        self.virtual_bus_route_revision = revision;
        Ok(())
    }

    pub(crate) fn validate_virtual_bus_routes(
        &self,
        routes: &VirtualBusRouteRegistry,
    ) -> Result<(), ControlError> {
        for route in routes.list() {
            if !self
                .virtual_buses
                .list()
                .iter()
                .any(|bus| bus.id() == &route.bus_id)
            {
                return Err(ControlError::InvalidRequest(
                    "virtual-bus route references an unknown bus".into(),
                ));
            }
        }
        self.store
            .validate_global_graph(routes.list())
            .map_err(|errors| {
                ControlError::InvalidRequest(format!("invalid virtual-bus routes: {errors:?}"))
            })?;
        Ok(())
    }

    pub fn virtual_bus_routes(&self) -> &VirtualBusRouteRegistry {
        &self.virtual_bus_routes
    }

    pub fn delete_virtual_bus(&mut self, id: &EntityId) -> Result<(), ControlError> {
        if self
            .virtual_bus_routes
            .list()
            .iter()
            .any(|route| &route.bus_id == id)
        {
            return Err(ControlError::InvalidRequest(
                "virtual bus is referenced by an active route".into(),
            ));
        }
        let checkpoint = self.virtual_buses.clone();
        self.virtual_buses
            .delete(id)
            .map_err(virtual_bus_control_error)?;
        if self.virtual_bridges.get(id).is_some() {
            self.virtual_bridges
                .remove(id)
                .map_err(virtual_bridge_control_error)?;
        }
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses(&self.virtual_buses) {
                self.virtual_buses = checkpoint;
                let _ = self.virtual_bridges.ensure(id.clone());
                return Err(storage_error(error));
            }
        }
        Ok(())
    }

    pub(crate) fn sync_virtual_bridge_operation(
        &mut self,
        operation: &VirtualBusOperation,
    ) -> Result<(), ControlError> {
        match operation {
            VirtualBusOperation::Create { id, .. } => self
                .virtual_bridges
                .ensure(id.clone())
                .map(|_| ())
                .map_err(virtual_bridge_control_error),
            VirtualBusOperation::SetEnabled { .. } => Ok(()),
            VirtualBusOperation::Delete { id } => {
                if self.virtual_bridges.get(id).is_some() {
                    self.virtual_bridges
                        .remove(id)
                        .map_err(virtual_bridge_control_error)?;
                }
                Ok(())
            }
            VirtualBusOperation::Rename { .. } => Ok(()),
        }
    }

    pub(crate) fn rollback_virtual_bridge_operation(
        &mut self,
        operation: &VirtualBusOperation,
    ) -> Result<(), ControlError> {
        match operation {
            VirtualBusOperation::Create { id, .. } => {
                if self.virtual_bridges.get(id).is_some() {
                    self.virtual_bridges
                        .remove(id)
                        .map_err(virtual_bridge_control_error)?;
                }
            }
            VirtualBusOperation::Delete { id } => {
                self.virtual_bridges
                    .ensure(id.clone())
                    .map(|_| ())
                    .map_err(virtual_bridge_control_error)?;
            }
            VirtualBusOperation::Rename { .. } | VirtualBusOperation::SetEnabled { .. } => {}
        }
        Ok(())
    }

    pub(crate) fn dispatch_virtual_devices_plan(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("operation is required".into()))?;
        let operation = virtual_bus_operation_from_value(
            params
                .get("operation")
                .ok_or_else(|| ControlError::InvalidRequest("operation is required".into()))?,
        )?;
        let mut candidate = self.virtual_buses.clone();
        apply_virtual_bus_operation(&mut candidate, &operation)?;
        let now = Instant::now();
        self.virtual_bus_plans
            .retain(|_, plan| plan.expires_at > now);
        if self.virtual_bus_plans.len() >= MAX_PENDING_PLAN_RECORDS {
            return Err(ControlError::InvalidRequest(
                "too many pending virtual-device plans".into(),
            ));
        }
        let timestamp_millis = unix_epoch_millis();
        let plan_id = allocate_timestamped_plan_id(
            "virtual-plan",
            timestamp_millis,
            &mut self.next_virtual_bus_plan,
            |id| self.virtual_bus_plans.contains_key(id),
            "virtual-device plan ID space is exhausted",
        )?;
        let expires_at = unix_epoch_seconds() + VIRTUAL_DEVICE_PLAN_TTL.as_secs() as i64;
        self.virtual_bus_plans.insert(
            plan_id.clone(),
            VirtualBusPlan {
                operation: operation.clone(),
                expires_at: Instant::now() + VIRTUAL_DEVICE_PLAN_TTL,
            },
        );
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_device_plan(
                &plan_id,
                &virtual_bus_operation_value(&operation),
                expires_at,
            ) {
                self.virtual_bus_plans.remove(&plan_id);
                return Err(storage_error(error));
            }
        }
        Ok(json!({
            "planId": plan_id,
            "expiresInMs": VIRTUAL_DEVICE_PLAN_TTL.as_millis(),
            "operation": virtual_bus_operation_value(&operation),
            "availability": {
                "status": "unavailable",
                "reason": "requires M03 managed virtual driver"
            },
            "requiredScopes": ["deviceAdministration"],
            "warnings": ["desired state can be stored, but Windows endpoints remain unavailable until the managed driver is installed"]
        }))
    }

    pub(crate) fn dispatch_virtual_devices_apply(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?;
        let plan_id = params
            .get("planId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let storage_key = self.scoped_idempotency_key("virtualDevices.apply", idempotency_key);
        let request_hash = virtual_device_request_hash(plan_id);
        if let Some(previous) = self.operation_outcomes.get(&storage_key) {
            if self
                .idempotency_hashes
                .get(&storage_key)
                .is_some_and(|hash| hash == &request_hash)
            {
                return Ok(previous.clone());
            }
            return Err(ControlError::IdempotencyConflict);
        }
        if let Some(storage) = &self.storage {
            if let Some(previous) = storage
                .journal_result_checked(&storage_key, &request_hash)
                .map_err(storage_error)?
            {
                let previous: Value = serde_json::from_str(&previous)
                    .map_err(|error| ControlError::Json(error.to_string()))?;
                self.remember_operation_outcome(
                    &storage_key,
                    previous.clone(),
                    "virtualDevices.apply",
                    Some(&request_hash),
                );
                return Ok(previous);
            }
        }
        let plan = self
            .virtual_bus_plans
            .get(&EntityId::new(plan_id))
            .cloned()
            .ok_or_else(|| ControlError::InvalidRequest("virtual device plan not found".into()))?;
        if plan.expires_at <= Instant::now() {
            self.virtual_bus_plans.remove(&EntityId::new(plan_id));
            return Err(ControlError::InvalidRequest(
                "virtual device plan expired".into(),
            ));
        }
        let checkpoint = self.virtual_buses.clone();
        apply_virtual_bus_operation(&mut self.virtual_buses, &plan.operation)?;
        if let Err(error) = self.sync_virtual_bridge_operation(&plan.operation) {
            self.virtual_buses = checkpoint;
            return Err(error);
        }
        let result = json!({
            "planId": plan_id,
            "state": "applied",
            "availability": {
                "status": "unavailable",
                "reason": "requires M03 managed virtual driver"
            },
            "operation": virtual_bus_operation_value(&plan.operation)
        });
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses_and_journal(
                &self.virtual_buses,
                &EntityId::new(plan_id),
                &storage_key,
                &request_hash,
                &result,
            ) {
                self.virtual_buses = checkpoint;
                let _ = self.rollback_virtual_bridge_operation(&plan.operation);
                return Err(storage_error(error));
            }
        }
        if let VirtualBusOperation::SetEnabled { id, enabled: false } = &plan.operation {
            if let Some(bridge) = self.virtual_bridges.get(id) {
                bridge.deactivate();
            }
        }
        self.virtual_bus_plans.remove(&EntityId::new(plan_id));
        self.remember_operation_outcome(
            &storage_key,
            result.clone(),
            "virtualDevices.apply",
            Some(&request_hash),
        );
        self.events
            .append(0, Some(plan_id.to_owned()), "virtualDevice.changed", None);
        Ok(result)
    }

    pub(crate) fn dispatch_virtual_devices_provision(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("busId is required".into()))?;
        let bus_id = params
            .get("busId")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("busId is required".into()))?;
        let instance_id = params
            .get("instanceId")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("instanceId is required".into()))?;
        let key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        self.dispatch_virtual_device_external(
            "virtualDevices.provision",
            bus_id,
            Some(instance_id),
            key,
        )
    }

    pub(crate) fn dispatch_virtual_devices_remove(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("busId is required".into()))?;
        let bus_id = params
            .get("busId")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("busId is required".into()))?;
        let key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        self.dispatch_virtual_device_external("virtualDevices.remove", bus_id, None, key)
    }

    pub(crate) fn dispatch_virtual_device_external(
        &mut self,
        method: &str,
        bus_id: &str,
        instance_id: Option<&str>,
        idempotency_key: &str,
    ) -> Result<Value, ControlError> {
        let storage_key = self.scoped_idempotency_key(method, idempotency_key);
        let request_hash = virtual_device_external_request_hash(method, bus_id, instance_id);
        if let Some(previous) = self.operation_outcomes.get(&storage_key) {
            if self
                .idempotency_hashes
                .get(&storage_key)
                .is_some_and(|hash| hash == &request_hash)
            {
                return Ok(previous.clone());
            }
            return Err(ControlError::IdempotencyConflict);
        }
        if let Some(storage) = &self.storage {
            if let Some(previous) = storage
                .journal_result_checked(&storage_key, &request_hash)
                .map_err(storage_error)?
            {
                let previous: Value = serde_json::from_str(&previous)
                    .map_err(|error| ControlError::Json(error.to_string()))?;
                self.remember_operation_outcome(
                    &storage_key,
                    previous.clone(),
                    method,
                    Some(&request_hash),
                );
                return Ok(previous);
            }
        }
        let operation_id = EntityId::new(format!("operation-{}", request_hash));
        let checkpoint = self.virtual_buses.clone();
        let result = match method {
            "virtualDevices.provision" => {
                #[cfg(windows)]
                let driver_instance_id = self.provision_virtual_bus_device_unpersisted(
                    &EntityId::new(bus_id),
                    instance_id.expect("provision instance id"),
                )?;
                #[cfg(not(windows))]
                let driver_instance_id = return Err(ControlError::InvalidRequest(
                    "managed virtual device provisioning is unavailable on this platform".into(),
                ));
                json!({"operationId": operation_id, "state": "completed", "busId": bus_id, "driverInstanceId": driver_instance_id, "availability": {"status": "unavailable", "reason": "requires M03 managed virtual driver"}})
            }
            "virtualDevices.remove" => {
                #[cfg(windows)]
                self.remove_virtual_bus_device_unpersisted(&EntityId::new(bus_id))?;
                #[cfg(not(windows))]
                return Err(ControlError::InvalidRequest(
                    "managed virtual device removal is unavailable on this platform".into(),
                ));
                json!({"operationId": operation_id, "state": "completed", "busId": bus_id, "driverInstanceId": null, "availability": {"status": "unavailable", "reason": "requires M03 managed virtual driver"}})
            }
            _ => {
                return Err(ControlError::InvalidRequest(
                    "unsupported virtual-device operation".into(),
                ))
            }
        };
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses_and_external_journal(
                &self.virtual_buses,
                method,
                &storage_key,
                &request_hash,
                &result,
            ) {
                #[cfg(windows)]
                if method == "virtualDevices.provision" {
                    let _ = self.managed_software_devices.remove(bus_id);
                }
                self.virtual_buses = checkpoint;
                return Err(storage_error(error));
            }
        }
        #[cfg(windows)]
        if method == "virtualDevices.remove" {
            if let Err(error) = self.managed_software_devices.remove(bus_id) {
                self.virtual_buses = checkpoint;
                if let Some(storage) = &self.storage {
                    if let Err(rollback_error) = storage.rollback_virtual_buses_external_journal(
                        &self.virtual_buses,
                        &storage_key,
                        &request_hash,
                    ) {
                        return Err(storage_error(rollback_error));
                    }
                }
                return Err(software_device_control_error(error));
            }
        }
        self.remember_operation_outcome(&storage_key, result.clone(), method, Some(&request_hash));
        Ok(result)
    }

    pub(crate) fn dispatch_virtual_devices_list(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let paged = params.get("cursor").is_some() || params.get("limit").is_some();
        let cursor = params
            .get("cursor")
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| ControlError::InvalidRequest("cursor must be a string".into()))
            })
            .transpose()?;
        let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(100);
        if !(1..=MAX_VIRTUAL_DEVICE_LIST_ITEMS as u64).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        let mut buses = self
            .virtual_buses
            .list()
            .iter()
            .map(|bus| {
                json!({
                    "id": bus.id(),
                    "name": bus.name(),
                    "driverInstanceId": bus.driver_instance_id(),
                    "direction": "bidirectional",
                    "channels": bus.channels(),
                    "enabled": bus.enabled(),
                    "availability": {
                        "status": "unavailable",
                        "reason": "requires M03 managed virtual driver"
                    },
                    "endpointIds": { "render": null, "capture": null },
                    "capabilities": { "render": false, "capture": false, "channels": 2 },
                    "privilege": "deviceAdministration",
                    "restartRequired": false,
                    "clientImpacts": [],
                    "leaseOwner": bus.lease().owner()
                })
            })
            .collect::<Vec<_>>();
        if let Some(cursor) = cursor {
            let Some(index) = buses.iter().position(|bus| bus["id"] == cursor) else {
                return Err(ControlError::InvalidRequest(
                    "invalid virtual device cursor".into(),
                ));
            };
            buses.drain(..=index);
        }
        if !paged {
            return Ok(json!(buses));
        }
        let has_more = buses.len() > limit as usize;
        buses.truncate(limit as usize);
        let next_cursor = has_more
            .then(|| buses.last().and_then(|bus| bus["id"].as_str()))
            .flatten();
        Ok(json!({ "items": buses, "nextCursor": next_cursor }))
    }

    pub(crate) fn dispatch_virtual_routes_list(&self) -> Result<Value, ControlError> {
        serde_json::to_value(json!({
            "revision": self.virtual_bus_route_revision,
            "routes": self.virtual_bus_routes.list()
        }))
        .map_err(|error| ControlError::Json(error.to_string()))
    }

    pub(crate) fn dispatch_virtual_routes_replace(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("params are required".into()))?;
        let base_revision = params
            .get("baseRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("baseRevision is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let routes_value = params
            .get("routes")
            .cloned()
            .ok_or_else(|| ControlError::InvalidRequest("routes are required".into()))?;
        let decoded: VirtualBusRouteRegistry = serde_json::from_value(routes_value)
            .map_err(|error| ControlError::InvalidRequest(format!("invalid routes: {error}")))?;
        let routes = VirtualBusRouteRegistry::new(decoded.list().to_vec())
            .map_err(|error| ControlError::InvalidRequest(error.into()))?;
        let request_hash = Self::request_hash(&json!({
            "baseRevision": base_revision,
            "routes": routes.list()
        }));
        let storage_key = self.scoped_idempotency_key("virtualRoutes.replace", idempotency_key);
        if let Some(previous) = self.lookup_idempotent_result(&storage_key, &request_hash)? {
            self.remember_operation_outcome(
                &storage_key,
                previous.clone(),
                "virtualRoutes.replace",
                Some(&request_hash),
            );
            return Ok(previous);
        }
        if base_revision != self.virtual_bus_route_revision {
            return Err(ControlError::InvalidRequest(format!(
                "baseRevision {base_revision} does not match current revision {}",
                self.virtual_bus_route_revision
            )));
        }
        let revision = self
            .virtual_bus_route_revision
            .checked_add(1)
            .ok_or_else(|| ControlError::InvalidRequest("route revision exhausted".into()))?;
        self.validate_virtual_bus_routes(&routes)?;
        let result = json!({ "state": "applied", "revision": revision, "routes": routes.list() });
        if let Some(storage) = &self.storage {
            storage
                .save_virtual_bus_route_state_and_journal(
                    &routes,
                    revision,
                    &storage_key,
                    &request_hash,
                    &result,
                )
                .map_err(storage_error)?;
        }
        self.virtual_bus_routes = routes;
        self.virtual_bus_route_revision = revision;
        self.remember_operation_outcome(
            &storage_key,
            result.clone(),
            "virtualRoutes.replace",
            Some(&request_hash),
        );
        Ok(result)
    }
}

#[cfg(test)]
#[path = "virtual_devices_tests.rs"]
mod tests;
