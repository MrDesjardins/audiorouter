//! `virtualCable.status` (WP-08, spec 17 §7.1): the AudioRouter cable driver's
//! state, version and cables, for a standard user. The detection is the
//! elevated helper's own read-only logic (`audiorouter_driver_helper::
//! status_report`): device and driver version from SetupAPI/CfgMgr, endpoints
//! by device instance, and the bridge QUERY. Nothing here installs or
//! elevates; install/update/repair/remove stay shell commands (§7.2).
//! Cached for 2 s, so a UI polling it costs one enumeration every 2 s.

use super::*;

pub(crate) const VIRTUAL_CABLE_STATUS_CACHE: Duration = Duration::from_secs(2);

/// Bus ID of cable `index` (0 = `cable-a`).
fn bus_id(index: u8) -> String {
    format!("cable-{}", char::from(b'a' + index))
}

/// The API shape of a status report: per enabled cable its bus ID, both
/// endpoint IDs and whether both endpoints are present.
pub(crate) fn virtual_cable_status_json(report: &audiorouter_driver_helper::StatusReport) -> Value {
    use audiorouter_driver_helper::Direction;
    let endpoint = |cable: u8, direction: Direction| {
        report
            .endpoints
            .iter()
            .find(|endpoint| endpoint.cable == cable && endpoint.direction == direction)
            .map(|endpoint| endpoint.endpoint_id.clone())
    };
    let cables: Vec<Value> = (0..report.cable_count)
        .map(|cable| {
            // A cable's "Input" is its render endpoint, "Output" its capture.
            let render_endpoint = report
                .endpoints
                .iter()
                .find(|endpoint| endpoint.cable == cable && endpoint.direction == Direction::Input);
            let render = render_endpoint.map(|endpoint| endpoint.endpoint_id.clone());
            let capture = endpoint(cable, Direction::Output);
            let name = report
                .cable_names
                .get(&cable)
                .cloned()
                .or_else(|| {
                    render_endpoint
                        .and_then(|endpoint| {
                            endpoint
                                .friendly_name
                                .strip_suffix(" (AudioRouter Virtual Cable)")
                        })
                        .and_then(|name| name.strip_prefix("AudioRouter "))
                        .and_then(|name| name.strip_suffix(" Input"))
                        .filter(|name| !name.is_empty())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| format!("Cable {}", char::from(b'A' + cable)));
            json!({
                "busId": bus_id(cable),
                "name": name,
                "renderEndpointId": render,
                "captureEndpointId": capture,
                "present": render.is_some() && capture.is_some(),
            })
        })
        .collect();
    json!({
        "state": report.state,
        "driverVersion": report.installed_driver_version,
        "protocol": report.protocol,
        "defaultPeriodFrames": report.default_period_frames,
        "cableCount": report.cable_count,
        "cables": cables,
    })
}

#[cfg(windows)]
fn read_virtual_cable_status() -> Result<Value, ControlError> {
    use audiorouter_driver_helper::{status_report, windows_platform::WindowsPlatform, StateStore};
    let program_data = std::env::var_os("ProgramData")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\ProgramData"));
    // The helper's install record only distinguishes "never installed" from
    // "needs repair" when the device is gone; a standard user who cannot
    // read it is treated as having none.
    let state = StateStore::new(program_data.join("AudioRouter").join("driver"))
        .load()
        .ok()
        .flatten();
    let mut platform = WindowsPlatform::new();
    let report = status_report(&mut platform, state.as_ref()).map_err(|failure| {
        ControlError::InvalidRequest(format!(
            "the cable driver status is unavailable: {}",
            failure.message
        ))
    })?;
    Ok(virtual_cable_status_json(&report))
}

#[cfg(not(windows))]
fn read_virtual_cable_status() -> Result<Value, ControlError> {
    Ok(json!({
        "state": "notInstalled",
        "driverVersion": null,
        "protocol": null,
        "cableCount": 0,
        "cables": [],
    }))
}

impl ControlPlane {
    pub(crate) fn dispatch_virtual_cable_status(&mut self) -> Result<Value, ControlError> {
        if let Some((at, value)) = &self.virtual_cable_status_cache {
            if at.elapsed() < VIRTUAL_CABLE_STATUS_CACHE {
                return Ok(value.clone());
            }
        }
        let value = read_virtual_cable_status()?;
        self.virtual_cable_status_cache = Some((Instant::now(), value.clone()));
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_driver_helper::{CableEndpoint, Direction, StatusReport};

    fn endpoint(cable: u8, direction: Direction) -> CableEndpoint {
        CableEndpoint {
            cable,
            direction,
            endpoint_id: format!(
                "{{0.0.{}.00000000}}.{cable}",
                u8::from(direction == Direction::Output)
            ),
            friendly_name: audiorouter_driver_helper::interface_name(cable, direction),
        }
    }

    fn report(state: &'static str, count: u8, endpoints: Vec<CableEndpoint>) -> StatusReport {
        StatusReport {
            state,
            package_version: Some("0.1.4".into()),
            installed_driver_version: Some("0.1.4.0".into()),
            device_instance_id: Some(r"ROOT\MEDIA\0000".into()),
            cable_count: count,
            default_period_frames: Some(480),
            cable_names: Default::default(),
            protocol: Some("1.1".into()),
            capabilities: Some(0x7f),
            endpoints,
        }
    }

    #[test]
    fn status_lists_each_enabled_cable_with_both_endpoints() {
        let value = virtual_cable_status_json(&report(
            "installed",
            2,
            vec![
                endpoint(0, Direction::Input),
                endpoint(0, Direction::Output),
                endpoint(1, Direction::Input),
                endpoint(1, Direction::Output),
            ],
        ));
        assert_eq!(value["state"], "installed");
        assert_eq!(value["driverVersion"], "0.1.4.0");
        assert_eq!(value["protocol"], "1.1");
        assert_eq!(value["defaultPeriodFrames"], 480);
        assert_eq!(value["cableCount"], 2);
        let cables = value["cables"].as_array().unwrap();
        assert_eq!(cables.len(), 2);
        assert_eq!(cables[0]["busId"], "cable-a");
        assert_eq!(cables[1]["busId"], "cable-b");
        assert_eq!(cables[0]["name"], "Cable A");
        assert_eq!(cables[0]["renderEndpointId"], "{0.0.0.00000000}.0");
        assert_eq!(cables[0]["captureEndpointId"], "{0.0.1.00000000}.0");
        assert!(cables.iter().all(|cable| cable["present"] == true));
    }

    #[test]
    fn status_uses_the_user_facing_name_from_the_cable_endpoint() {
        let mut cable = endpoint(0, Direction::Input);
        cable.friendly_name = "AudioRouter Discord Input (AudioRouter Virtual Cable)".into();
        let value = virtual_cable_status_json(&report("installed", 1, vec![cable]));
        assert_eq!(value["cables"][0]["name"], "Discord");
    }

    #[test]
    fn status_keeps_the_configured_name_while_an_endpoint_is_missing() {
        let mut report = report("needsRepair", 2, Vec::new());
        report.cable_names.insert(1, "Discord".into());
        let value = virtual_cable_status_json(&report);
        assert_eq!(value["cables"][0]["name"], "Cable A");
        assert_eq!(value["cables"][1]["name"], "Discord");
    }

    #[test]
    fn status_marks_a_cable_with_a_missing_endpoint_absent() {
        let value = virtual_cable_status_json(&report(
            "needsRepair",
            2,
            vec![
                endpoint(0, Direction::Input),
                endpoint(0, Direction::Output),
                endpoint(1, Direction::Input),
            ],
        ));
        assert_eq!(value["state"], "needsRepair");
        assert_eq!(value["cables"][0]["present"], true);
        assert_eq!(value["cables"][1]["present"], false);
        assert_eq!(value["cables"][1]["captureEndpointId"], Value::Null);
        let none = virtual_cable_status_json(&StatusReport {
            installed_driver_version: None,
            protocol: None,
            capabilities: None,
            default_period_frames: None,
            device_instance_id: None,
            package_version: None,
            ..report("notInstalled", 2, Vec::new())
        });
        assert_eq!(none["state"], "notInstalled");
        assert_eq!(none["driverVersion"], Value::Null);
        assert!(none["cables"]
            .as_array()
            .unwrap()
            .iter()
            .all(|cable| cable["present"] == false));
    }

    #[test]
    fn status_is_cached_for_two_seconds() {
        let mut plane = ControlPlane::default();
        let cached = json!({ "state": "installed", "cached": true });
        plane.virtual_cable_status_cache = Some((Instant::now(), cached.clone()));
        assert_eq!(plane.dispatch_virtual_cable_status().unwrap(), cached);
    }
}
