//! Setup check (UI-27): saved sessions compared with the active devices.
//!
//! A device is judged only by how a session uses it, never alone: a 7.1
//! cable is right for a Surround input and wrong for an output. Findings
//! that say Play will be refused use the same predicates as native
//! preparation (`output_format_supported`, `surround_capture_refusal`,
//! `reject_endpoint_feedback`), so the check cannot drift from the refusal.

use super::*;

/// Most findings one check returns; the rest are counted in `truncated`.
pub(crate) const MAX_SETUP_FINDINGS: usize = 100;

/// The rate virtual cables should run at (the engine's internal rate).
const CABLE_RATE_HZ: u32 = 48_000;

/// What the check reads about one active device. Built from the Windows
/// inventory in the dispatcher and by hand in tests.
#[derive(Clone, Debug)]
pub(crate) struct DeviceFacts {
    pub id: String,
    /// A playback (render) device; otherwise a recording (capture) device.
    pub render: bool,
    pub name: String,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub is_float32: bool,
    pub channel_mask: u32,
    /// The virtual cable the device belongs to (both ends share it).
    pub cable_key: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Severity {
    /// Play is refused, or audio loops.
    Problem,
    /// Likely wrong.
    Warning,
    /// Optional improvement.
    Tip,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetupFinding {
    pub code: &'static str,
    pub severity: Severity,
    pub session_id: String,
    pub session_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    /// The other session of a loop across sessions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_session_id: Option<String>,
    pub message: String,
    pub fix: String,
}

/// How a node kind uses a Windows device. Exhaustive on purpose (no `_`
/// arm): a new kind must say whether it binds a device.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeviceUse {
    /// Records a recording device, or loopback-records a playback device
    /// when set to Surround.
    Input,
    /// Plays to a playback device.
    Output,
    /// Loopback-records a playback device.
    Loopback,
}

fn device_use(kind: NodeKind) -> Option<DeviceUse> {
    match kind {
        NodeKind::PhysicalInput => Some(DeviceUse::Input),
        NodeKind::PhysicalOutput => Some(DeviceUse::Output),
        NodeKind::EndpointLoopback => Some(DeviceUse::Loopback),
        NodeKind::ApplicationCapture
        | NodeKind::VirtualRenderSource
        | NodeKind::VirtualCaptureSink
        | NodeKind::TestSignal
        | NodeKind::Mixer
        | NodeKind::Gain
        | NodeKind::Mute
        | NodeKind::Meter
        | NodeKind::ParametricEq
        | NodeKind::Compressor
        | NodeKind::Gate
        | NodeKind::Limiter
        | NodeKind::Delay
        | NodeKind::GraphicEq
        | NodeKind::Pitch
        | NodeKind::Recorder
        | NodeKind::Plugin
        | NodeKind::AudioFile
        | NodeKind::Volume
        | NodeKind::BassTreble
        | NodeKind::Dehum
        | NodeKind::Declick
        | NodeKind::InputSwitch
        | NodeKind::OutputSwitch
        | NodeKind::Denoise
        | NodeKind::SpeechDenoise
        | NodeKind::FirFilter
        | NodeKind::SpectralGate
        | NodeKind::TimeShift
        | NodeKind::NetworkSend
        | NodeKind::NetworkReceive
        | NodeKind::Duck
        | NodeKind::FunVoiceChanger
        | NodeKind::Soundboard => None,
    }
}

/// Render → capture pairs of the same virtual cable: what plays into the
/// render end comes out of the capture end.
pub(crate) fn cable_returns(devices: &[DeviceFacts]) -> Vec<(String, String)> {
    let mut returns = Vec::new();
    for render in devices.iter().filter(|device| device.render) {
        let Some(key) = &render.cable_key else {
            continue;
        };
        for capture in devices.iter().filter(|device| !device.render) {
            if capture.cable_key.as_ref() == Some(key) {
                returns.push((render.id.clone(), capture.id.clone()));
            }
        }
    }
    returns
}

/// A node Play would bind: enabled and connected by an enabled edge.
fn in_use(session: &Session, node: &audiorouter_domain::Node) -> bool {
    node.enabled
        && session.edges.iter().any(|edge| {
            edge.enabled && (edge.source_node == node.id || edge.destination_node == node.id)
        })
}

fn format_rate(hz: u32) -> String {
    if hz % 1000 == 0 {
        format!("{} kHz", hz / 1000)
    } else {
        format!("{:.1} kHz", f64::from(hz) / 1000.0)
    }
}

/// Check `sessions` (each with whether it is running) against `devices`.
pub(crate) fn check_setup(
    sessions: &[(&Session, bool)],
    devices: &[DeviceFacts],
) -> Vec<SetupFinding> {
    let returns = cable_returns(devices);
    let mut findings = Vec::new();
    for &(session, _) in sessions {
        check_session_devices(session, devices, &mut findings);
        if let Err(ControlError::InvalidRequest(message)) =
            reject_endpoint_feedback(session, &returns)
        {
            findings.push(SetupFinding {
                code: "feedbackLoop",
                severity: Severity::Problem,
                session_id: session.id.as_str().to_owned(),
                session_name: session.name.clone(),
                node_id: None,
                node_name: None,
                device_id: None,
                device_name: None,
                other_session_id: None,
                message,
                fix: "Send the output to a different cable, or remove the connection that returns to the input.".into(),
            });
        }
    }
    cross_session_loops(sessions, devices, &returns, &mut findings);
    findings
}

fn check_session_devices(
    session: &Session,
    devices: &[DeviceFacts],
    findings: &mut Vec<SetupFinding>,
) {
    let mut rate_tipped = Vec::<&str>::new();
    for node in session.nodes.iter().filter(|node| in_use(session, node)) {
        let Some(usage) = device_use(node.kind) else {
            continue;
        };
        let Some(endpoint_id) = node.parameters.get("endpointId").and_then(Value::as_str) else {
            // A loopback that follows a Windows default role has no fixed
            // device; an unchosen device is already shown on the node.
            continue;
        };
        let surround = node_spatial_options(node).is_some();
        let device = devices.iter().find(|device| {
            device.id == endpoint_id
                && match usage {
                    DeviceUse::Output | DeviceUse::Loopback => device.render,
                    DeviceUse::Input => !device.render || surround,
                }
        });
        let finding =
            |code, severity, device: Option<&DeviceFacts>, message: String, fix: String| {
                SetupFinding {
                    code,
                    severity,
                    session_id: session.id.as_str().to_owned(),
                    session_name: session.name.clone(),
                    node_id: Some(node.id.as_str().to_owned()),
                    node_name: Some(node.name.clone()),
                    device_id: Some(endpoint_id.to_owned()),
                    device_name: device.map(|device| device.name.clone()),
                    other_session_id: None,
                    message,
                    fix,
                }
            };
        let Some(device) = device else {
            findings.push(finding(
                "deviceMissing",
                Severity::Problem,
                None,
                format!("The device chosen for \"{}\" is not connected or is turned off in Windows.", node.name),
                "Plug it in or enable it in Windows Sound settings. After a driver update Windows gives devices new IDs: choose the device again in the node's Properties.".into(),
            ));
            continue;
        };
        match usage {
            DeviceUse::Output
                if !output_format_supported(
                    device.is_float32,
                    device.channels,
                    device.sample_rate_hz,
                ) =>
            {
                findings.push(finding(
                    "outputFormat",
                    Severity::Problem,
                    Some(device),
                    format!(
                        "\"{}\" plays to {}, which is set to {} channel(s) at {}. AudioRouter plays only to stereo devices, so Play is refused.",
                        node.name, device.name, device.channels, format_rate(device.sample_rate_hz)
                    ),
                    format!(
                        "In Windows Sound settings, set {} to Stereo (Configure) at 48 kHz, or choose another output. If a game sends surround to this device, keep it and pick a different device for this output.",
                        device.name
                    ),
                ));
            }
            DeviceUse::Input if surround => {
                if let Some(refusal) = surround_capture_refusal(
                    device.is_float32,
                    device.channels,
                    device.channel_mask,
                    device.sample_rate_hz,
                ) {
                    let detail = match refusal {
                        SurroundRefusal::Layout => {
                            format!("is set to {} channel(s)", device.channels)
                        }
                        SurroundRefusal::Format => format!(
                            "has a format AudioRouter cannot read at {}",
                            format_rate(device.sample_rate_hz)
                        ),
                    };
                    findings.push(finding(
                        "surroundDevice",
                        Severity::Problem,
                        Some(device),
                        format!(
                            "\"{}\" uses Surround, which needs a 5.1 or 7.1 device, but {} {}. Play is refused.",
                            node.name, device.name, detail
                        ),
                        format!(
                            "In Windows Sound settings, set {} to 7.1 Surround (Configure) at 48 kHz and restart the game, or turn Surround off for this input.",
                            device.name
                        ),
                    ));
                }
            }
            _ => {}
        }
        if device.cable_key.is_some()
            && device.sample_rate_hz != CABLE_RATE_HZ
            && !rate_tipped.contains(&endpoint_id)
        {
            rate_tipped.push(endpoint_id);
            findings.push(finding(
                "cableRate",
                Severity::Tip,
                Some(device),
                format!(
                    "{} runs at {}. AudioRouter works at 48 kHz, so Windows converts this cable's audio.",
                    device.name, format_rate(device.sample_rate_hz)
                ),
                format!("In Windows Sound settings, set {} to 48000 Hz (Properties → Advanced).", device.name),
            ));
        }
    }
}

/// Loops that need two sessions: one plays into a device whose audio
/// another records, and back. Reported once per pair, when at least one of
/// the two is running. A loop inside one session is the Play refusal above.
fn cross_session_loops(
    sessions: &[(&Session, bool)],
    devices: &[DeviceFacts],
    returns: &[(String, String)],
    findings: &mut Vec<SetupFinding>,
) {
    let render_ids = devices
        .iter()
        .filter(|device| device.render)
        .map(|device| device.id.as_str())
        .collect::<Vec<_>>();
    // Per session: the device each source records → the devices it plays to.
    let flows = sessions
        .iter()
        .map(|&(session, _)| {
            let mut flow = Vec::<(String, String)>::new();
            for source in session.nodes.iter().filter(|node| {
                node.enabled
                    && matches!(
                        device_use(node.kind),
                        Some(DeviceUse::Input | DeviceUse::Loopback)
                    )
            }) {
                let Some(capture) = source.parameters.get("endpointId").and_then(Value::as_str)
                else {
                    continue;
                };
                for output in reachable_outputs(session, &source.id) {
                    if let Some(render) =
                        output.parameters.get("endpointId").and_then(Value::as_str)
                    {
                        flow.push((capture.to_owned(), render.to_owned()));
                    }
                }
            }
            flow
        })
        .collect::<Vec<_>>();
    // What a device's audio becomes: a playback device is recorded as itself
    // (loopback) and at its cable's other end.
    let heard_as = |render: &str| {
        let mut ids = vec![render.to_owned()];
        if render_ids.contains(&render) {
            ids.extend(
                returns
                    .iter()
                    .filter(|(r, _)| r == render)
                    .map(|(_, capture)| capture.clone()),
            );
        }
        ids
    };
    let mut reported = Vec::<(usize, usize)>::new();
    for (a, &(session_a, running_a)) in sessions.iter().enumerate() {
        for (b, &(session_b, running_b)) in sessions.iter().enumerate() {
            if a == b || !(running_a || running_b) || reported.contains(&(a.min(b), a.max(b))) {
                continue;
            }
            // A path of A plays into a device a path of B records, and that
            // path of B plays back into the device A's path records. Two
            // unrelated paths that merely share cables are not a loop.
            let looped = flows[a].iter().any(|(capture_a, render_a)| {
                let heard_by_b = heard_as(render_a);
                flows[b].iter().any(|(capture_b, render_b)| {
                    heard_by_b.contains(capture_b) && heard_as(render_b).contains(capture_a)
                })
            });
            if !looped {
                continue;
            }
            reported.push((a.min(b), a.max(b)));
            let (message, fix) = if running_a && running_b {
                (
                    format!(
                        "\"{}\" and \"{}\" are both playing and send audio into each other through your devices: a feedback loop.",
                        session_a.name, session_b.name
                    ),
                    "Stop one of them, or send one of them to a different cable.".to_owned(),
                )
            } else {
                let (idle, playing) = if running_a {
                    (session_b, session_a)
                } else {
                    (session_a, session_b)
                };
                (
                    format!(
                        "Playing \"{}\" while \"{}\" plays makes a feedback loop: each sends audio into a device the other records.",
                        idle.name, playing.name
                    ),
                    format!(
                        "Stop \"{}\" first, or send \"{}\" to a different cable.",
                        playing.name, idle.name
                    ),
                )
            };
            findings.push(SetupFinding {
                code: "crossSessionLoop",
                severity: Severity::Warning,
                session_id: session_a.id.as_str().to_owned(),
                session_name: session_a.name.clone(),
                node_id: None,
                node_name: None,
                device_id: None,
                device_name: None,
                other_session_id: Some(session_b.id.as_str().to_owned()),
                message,
                fix,
            });
        }
    }
}

impl ControlPlane {
    /// `system.checkSetup`: judge every saved session against the active
    /// devices. A read: nothing is changed.
    pub(crate) fn dispatch_setup_check(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let filter = params
            .as_ref()
            .and_then(|params| params.get("sessionId"))
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .map(EntityId::new)
                    .ok_or_else(|| {
                        ControlError::InvalidRequest("sessionId must be a string".into())
                    })
            })
            .transpose()?;
        let mut by_id = BTreeMap::<String, Session>::new();
        if let Some(storage) = &self.storage {
            for session in storage
                .list_sessions_after(None, MAX_SESSION_LIST_ITEMS)
                .map_err(storage_error)?
            {
                by_id.insert(session.id.as_str().to_owned(), session);
            }
        }
        for session in self.store.sessions_after(None, MAX_SESSION_LIST_ITEMS) {
            by_id.insert(session.id.as_str().to_owned(), session);
        }
        if let Some(filter) = &filter {
            if !by_id.contains_key(filter.as_str()) {
                return Err(ControlError::InvalidRequest("session not found".into()));
            }
        }
        let sessions = by_id
            .values()
            .map(|session| {
                let running = self
                    .runtimes
                    .get(&session.id)
                    .is_some_and(|runtime| runtime.state() == RuntimeState::Running);
                (session, running)
            })
            .collect::<Vec<_>>();
        let mut not_checked = vec![
            "Settings inside other apps, such as a game's speaker setup or the devices Discord uses (see the Apps tab).",
        ];
        // Without a device list nothing can be judged: every bound device
        // would look missing. Say so instead of failing the whole check.
        #[cfg(windows)]
        let devices = setup_devices().ok();
        #[cfg(not(windows))]
        let devices: Option<Vec<DeviceFacts>> = None;
        let mut findings = match &devices {
            Some(devices) => check_setup(&sessions, devices),
            None => {
                not_checked.push("Audio devices: the Windows device list could not be read.");
                Vec::new()
            }
        };
        if let Some(filter) = &filter {
            findings.retain(|finding| {
                finding.session_id == filter.as_str()
                    || finding.other_session_id.as_deref() == Some(filter.as_str())
            });
        }
        let truncated = findings.len().saturating_sub(MAX_SETUP_FINDINGS);
        findings.truncate(MAX_SETUP_FINDINGS);
        Ok(json!({
            "findings": findings,
            "truncated": truncated,
            "sessionsChecked": if filter.is_some() { 1 } else { sessions.len() },
            "devicesChecked": devices.as_ref().map_or(0, Vec::len),
            "notChecked": not_checked,
        }))
    }
}

/// The active Windows devices, with the cable each belongs to.
#[cfg(windows)]
fn setup_devices() -> Result<Vec<DeviceFacts>, ControlError> {
    let endpoints =
        audiorouter_windows_audio::enumerate_active_endpoints().map_err(audio_control_error)?;
    let display = audiorouter_windows_audio::enumerate_active_endpoint_display_info()
        .map_err(audio_control_error)?;
    Ok(endpoints
        .into_iter()
        .map(|endpoint| {
            let info = display
                .iter()
                .find(|info| info.id == endpoint.id && info.direction == endpoint.direction);
            DeviceFacts {
                render: endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render,
                name: info.map_or_else(|| "Unknown audio device".into(), |info| info.name.clone()),
                sample_rate_hz: endpoint.sample_rate_hz,
                channels: endpoint.channels,
                is_float32: endpoint.is_ieee_float32(),
                channel_mask: endpoint.channel_mask,
                cable_key: info.and_then(|info| {
                    audiorouter_windows_audio::known_virtual_cable_key(
                        &info.device_description,
                        &info.driver_inf_section,
                    )
                }),
                id: endpoint.id,
            }
        })
        .collect())
}

#[cfg(test)]
#[path = "setup_check_tests.rs"]
mod tests;
