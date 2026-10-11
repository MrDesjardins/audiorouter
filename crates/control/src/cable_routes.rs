//! Cable-only routes (WP-09 slice 3, spec 17 §7.3): one AudioRouter cable's
//! render source (`VirtualRenderSource`, what apps play into "Cable A
//! Input") through built-in tools into another cable's capture sink
//! (`VirtualCaptureSink`, what apps record from "Cable B Output").
//!
//! Both cables follow the driver's clock, so the route runs at the bridge
//! rate with no drift correction or resampling. `nativePaths.prepare`
//! only validates and records the route; `session.start` opens both
//! bridge leases and starts a [`audiorouter_windows_audio::CableRouteWorker`];
//! the backend service pass heartbeats the leases; session stop retires and
//! closes them. A failed lease or worker stops the route: the capture lease
//! closes and the cable outputs silence, never another source (DEC-10).

use super::*;
use audiorouter_domain::Node;

/// The driver's bridge device; one device serves every cable.
pub(crate) const CABLE_BRIDGE_DEVICE: &str = r"\\.\AudioRouterVirtualBridge";
pub(crate) const CABLE_ROUTE_RATE_HZ: u32 = 48_000;
pub(crate) const CABLE_ROUTE_FRAMES: u16 = 480;
pub(crate) const CABLE_ROUTE_CHANNELS: u16 = 2;
pub(crate) const CABLE_ROUTE_LEASE_MS: u32 = 2_000;

/// One-in/one-out built-in tools a cable route may contain (the single-graph
/// runtime; `crates/engine/tests/cable_route.rs` compiles every chain).
const CABLE_ROUTE_TOOLS: &[NodeKind] = &[
    NodeKind::Meter,
    NodeKind::Gain,
    NodeKind::Volume,
    NodeKind::BassTreble,
    NodeKind::Dehum,
    NodeKind::Declick,
    NodeKind::Denoise,
    NodeKind::SpeechDenoise,
    NodeKind::SpectralGate,
    NodeKind::FirFilter,
    NodeKind::TimeShift,
    NodeKind::Mute,
    NodeKind::ParametricEq,
    NodeKind::Compressor,
    NodeKind::Gate,
    NodeKind::Limiter,
    NodeKind::Delay,
    NodeKind::GraphicEq,
    NodeKind::Pitch,
    NodeKind::Duck,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CableRouteShape {
    pub render_bus: String,
    pub capture_bus: String,
}

fn cable_bus(node: &Node) -> Option<String> {
    let bus = node.parameters.get("busId").and_then(Value::as_str)?;
    let bytes = bus.as_bytes();
    (bytes.len() == 7 && bus.starts_with("cable-") && (b'a'..=b'h').contains(&bytes[6]))
        .then(|| bus.to_owned())
}

fn cable_label(bus: &str) -> String {
    format!("Cable {}", bus[6..].to_ascii_uppercase())
}

/// `None` when the session has no enabled cable source (not a cable route).
/// Otherwise the route's cables, or why this shape is not supported yet.
pub(crate) fn cable_route_shape(
    session: &Session,
) -> Option<Result<CableRouteShape, ControlError>> {
    let enabled = |kind: NodeKind| {
        session
            .nodes
            .iter()
            .filter(move |node| node.enabled && node.kind == kind)
    };
    let sources: Vec<&Node> = enabled(NodeKind::VirtualRenderSource).collect();
    if sources.is_empty() {
        return None;
    }
    let refuse = |reason: String| Some(Err(ControlError::InvalidRequest(reason)));
    let sinks: Vec<&Node> = enabled(NodeKind::VirtualCaptureSink).collect();
    if sources.len() != 1 || sinks.len() != 1 {
        return refuse(
            "a cable route needs exactly one Cable input and one Cable output; mixing several cables is not supported yet".into(),
        );
    }
    if let Some(other) = session.nodes.iter().find(|node| {
        node.enabled
            && !matches!(
                node.kind,
                NodeKind::VirtualRenderSource | NodeKind::VirtualCaptureSink
            )
            && !CABLE_ROUTE_TOOLS.contains(&node.kind)
    }) {
        return refuse(format!(
            "{} cannot be used in a cable-to-cable route yet; use built-in tools only",
            other.name
        ));
    }
    let (source, sink) = (sources[0], sinks[0]);
    let (Some(render_bus), Some(capture_bus)) = (cable_bus(source), cable_bus(sink)) else {
        return refuse(
            "choose the cable (Cable A to Cable H) for each cable node in its Properties".into(),
        );
    };
    if render_bus == capture_bus {
        return refuse(format!(
            "{} cannot feed itself; choose a different cable for the output",
            cable_label(&render_bus)
        ));
    }
    let stereo = |node: &Node, direction| {
        node.ports
            .iter()
            .filter(|port| port.direction == direction)
            .all(|port| port.channels == CABLE_ROUTE_CHANNELS as u8)
    };
    if !stereo(source, PortDirection::Output) || !stereo(sink, PortDirection::Input) {
        return refuse("cable routes are stereo; set both cable nodes to 2 channels".into());
    }
    Some(Ok(CableRouteShape {
        render_bus,
        capture_bus,
    }))
}

#[cfg(windows)]
pub(crate) struct ActiveCableRoute {
    pub session_id: EntityId,
    pub generation: u64,
    pub shape: CableRouteShape,
    worker: Option<audiorouter_windows_audio::CableRouteWorker>,
    render: Option<audiorouter_windows_audio::NativeBridgeController>,
    capture: Option<audiorouter_windows_audio::NativeBridgeController>,
    last_heartbeat: std::time::Instant,
    /// Last counters, kept after the route stops for diagnostics.
    counters: audiorouter_windows_audio::CableRouteCounters,
    failure: Option<String>,
}

#[cfg(windows)]
impl ActiveCableRoute {
    pub fn is_running(&self) -> bool {
        self.worker.is_some()
    }

    /// Retire and close both leases, then join the worker. Every step runs
    /// even when an earlier one fails; the first error is returned.
    fn shutdown(&mut self) -> Option<String> {
        let mut first = None;
        let mut note = |error: String| {
            first.get_or_insert(error);
        };
        if let Some(worker) = &self.worker {
            worker.begin_retire();
        }
        for controller in [self.capture.as_mut(), self.render.as_mut()]
            .into_iter()
            .flatten()
        {
            if let Err(error) = controller.deactivate() {
                note(format!("cable lease release failed: {error:?}"));
            }
        }
        if let Some(worker) = self.worker.take() {
            let (counters, failure) = worker.stop_with_counters();
            self.counters = counters;
            if let Some(error) = failure {
                note(format!("{error:?}"));
            }
        }
        for controller in [self.capture.take(), self.render.take()]
            .into_iter()
            .flatten()
        {
            if let Err(error) = controller.close() {
                note(format!("cable mapping cleanup failed: {error:?}"));
            }
        }
        first
    }

    fn to_json(&self) -> Value {
        let counters = self.worker.as_ref().map_or(
            self.counters,
            audiorouter_windows_audio::CableRouteWorker::counters,
        );
        json!({
            "sessionId": self.session_id,
            "generation": self.generation,
            "renderBus": self.shape.render_bus,
            "captureBus": self.shape.capture_bus,
            "state": if self.is_running() { "running" } else if self.failure.is_some() { "failed" } else { "stopped" },
            "failure": self.failure,
            "renderBlocks": counters.render_blocks,
            "renderSequenceGaps": counters.render_sequence_gaps,
            "captureBlocks": counters.capture_blocks,
            "forwarded": counters.forwarded,
            "silence": counters.silence,
            "dropped": counters.dropped,
            "trimmed": counters.trimmed,
            "engineSilentQuanta": counters.engine_silent,
        })
    }
}

#[cfg(windows)]
impl Drop for ActiveCableRoute {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

impl ControlPlane {
    /// `nativePaths.prepare` for a cable route: validate and record it. No
    /// lease or thread exists until `session.start`.
    #[cfg(windows)]
    pub(crate) fn prepare_cable_route(
        &mut self,
        session_id: &EntityId,
        generation: u64,
        shape: &CableRouteShape,
    ) -> Result<Value, ControlError> {
        if self
            .cable_route
            .as_ref()
            .is_some_and(|route| route.is_running() && route.session_id != *session_id)
        {
            return Err(ControlError::InvalidRequest(
                "another cable route is playing; stop it first".into(),
            ));
        }
        if self.any_native_worker_attached() {
            return Err(ControlError::InvalidRequest(
                "native worker is already attached".into(),
            ));
        }
        self.cable_route_prepared = Some((session_id.clone(), generation));
        Ok(json!({
            "sessionId": session_id,
            "generation": generation,
            "adapter": "cable-route",
            "renderBus": shape.render_bus,
            "captureBus": shape.capture_bus,
        }))
    }

    #[cfg(windows)]
    pub(crate) fn cable_route_prepared_for(&self, session_id: &EntityId) -> bool {
        self.cable_route_prepared
            .as_ref()
            .is_some_and(|(prepared, _)| prepared == session_id)
    }

    /// Open both leases and start the route for a running runtime
    /// generation. On error nothing stays open.
    #[cfg(windows)]
    pub(crate) fn start_cable_route(
        &mut self,
        session_id: &EntityId,
        session: &Session,
        generation: u64,
    ) -> Result<(), ControlError> {
        let started = self.open_cable_route(session_id, session, generation);
        match &started {
            // Privacy mute is a latch; a new route starts in its state.
            Ok(()) => {
                if let Some(worker) = self
                    .cable_route
                    .as_ref()
                    .and_then(|route| route.worker.as_ref())
                {
                    worker.set_privacy_muted(self.privacy_muted);
                }
            }
            // A failed start keeps nothing prepared: the next Play prepares
            // again from the session as it is then.
            Err(_) => self.cable_route_prepared = None,
        }
        started
    }

    #[cfg(windows)]
    fn open_cable_route(
        &mut self,
        session_id: &EntityId,
        session: &Session,
        generation: u64,
    ) -> Result<(), ControlError> {
        use audiorouter_protocol::{AudioBridgeDirection, AudioBridgeHello};
        use audiorouter_windows_audio::{
            CableRouteProcessor, CableRouteWorker, CableRouteWorkerConfig,
            NativeBridgeControlClient, NativeBridgeController, NativeBridgeRegion,
            NativeBridgeSession,
        };
        let session = audiorouter_engine::prune_inactive_upstream(session).into_owned();
        let shape = cable_route_shape(&session).unwrap_or_else(|| {
            Err(ControlError::InvalidRequest(
                "the session has no Cable input".into(),
            ))
        })?;
        if self
            .cable_route
            .as_ref()
            .is_some_and(|route| route.is_running() && route.session_id != *session_id)
        {
            return Err(ControlError::InvalidRequest(
                "another cable route is playing; stop it first".into(),
            ));
        }
        if let Some(mut previous) = self.cable_route.take() {
            let _ = previous.shutdown();
        }
        let bridge_error = |what: &str, error: &dyn std::fmt::Debug| {
            ControlError::InvalidRequest(format!("{what}: {error:?}"))
        };
        // The driver must be installed, compatible, and have both cables on.
        let info = NativeBridgeControlClient::open(CABLE_BRIDGE_DEVICE)
            .and_then(|client| client.query())
            .map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "the AudioRouter cable driver is not available: {error}"
                ))
            })?;
        info.check_compatible().map_err(|error| {
            bridge_error("the AudioRouter cable driver is incompatible", &error)
        })?;
        for bus in [&shape.render_bus, &shape.capture_bus] {
            let index = u32::from(bus.as_bytes()[6] - b'a') + 1;
            if index > info.cable_count {
                return Err(ControlError::InvalidRequest(format!(
                    "{} is not enabled ({} cables are installed)",
                    cable_label(bus),
                    info.cable_count
                )));
            }
        }
        let graph = audiorouter_engine::compile_session_at_sample_rate(
            &session,
            audiorouter_engine::RuntimeGeneration::new(generation),
            CABLE_ROUTE_RATE_HZ,
        )
        .map_err(|error| bridge_error("cable route compile", &error))?;
        let processor = CableRouteProcessor::new(
            usize::from(CABLE_ROUTE_CHANNELS),
            usize::from(CABLE_ROUTE_FRAMES),
            graph,
        )
        .map_err(|error| bridge_error("cable route", &error))?;
        // A fresh lease generation per start: a stale mapping of an earlier
        // run can never be mistaken for this one.
        let lease_generation = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(generation)
            .max(1);
        let hello = |bus: &str, direction| AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: bus.to_owned(),
            direction,
            generation: lease_generation,
            sample_rate_hz: CABLE_ROUTE_RATE_HZ,
            channels: CABLE_ROUTE_CHANNELS,
            frames_per_quantum: CABLE_ROUTE_FRAMES,
            lease_ms: CABLE_ROUTE_LEASE_MS,
        };
        let mapping = |bus: &str, side: &str| {
            std::env::temp_dir().join(format!(
                "audiorouter-cable-{}-{bus}-{lease_generation}-{side}.slot",
                std::process::id()
            ))
        };
        let render_path = mapping(&shape.render_bus, "render");
        let capture_path = mapping(&shape.capture_bus, "capture");
        let block_samples = usize::from(CABLE_ROUTE_FRAMES) * usize::from(CABLE_ROUTE_CHANNELS);
        let render_session = NativeBridgeSession::create(
            &render_path,
            hello(&shape.render_bus, AudioBridgeDirection::RenderSource),
        )
        .map_err(|error| bridge_error("cable input preparation failed", &error))?;
        // From here every early return drops what exists; sessions and
        // controllers remove their own mapping files.
        let capture_session = match NativeBridgeSession::create_primed(
            &capture_path,
            hello(&shape.capture_bus, AudioBridgeDirection::CaptureSink),
            &vec![0.0; block_samples],
        ) {
            Ok(session) => session,
            Err(error) => {
                let _ = render_session.remove_owned_mapping();
                return Err(bridge_error("cable output preparation failed", &error));
            }
        };
        let views =
            NativeBridgeRegion::open(&render_path, CABLE_ROUTE_CHANNELS, CABLE_ROUTE_FRAMES)
                .and_then(|render| {
                    NativeBridgeRegion::open(
                        &capture_path,
                        CABLE_ROUTE_CHANNELS,
                        CABLE_ROUTE_FRAMES,
                    )
                    .map(|capture| (render, capture))
                });
        let (render_view, capture_view) = match views {
            Ok(views) => views,
            Err(error) => {
                let _ = render_session.remove_owned_mapping();
                let _ = capture_session.remove_owned_mapping();
                return Err(bridge_error("cable mapping", &error));
            }
        };
        let mut route = ActiveCableRoute {
            session_id: session_id.clone(),
            generation,
            shape: shape.clone(),
            worker: None,
            render: None,
            capture: None,
            last_heartbeat: std::time::Instant::now(),
            counters: Default::default(),
            failure: None,
        };
        let worker = CableRouteWorker::start(
            render_view,
            capture_view,
            processor,
            CableRouteWorkerConfig {
                render_generation: lease_generation,
                capture_generation: lease_generation,
                block_samples,
            },
        );
        let worker = match worker {
            Ok(worker) => worker,
            Err(error) => {
                let _ = render_session.remove_owned_mapping();
                let _ = capture_session.remove_owned_mapping();
                return Err(bridge_error("cable route worker", &error));
            }
        };
        // The render thread must already poll when Windows starts
        // publishing into the render source.
        if !worker.wait_armed(std::time::Duration::from_secs(2)) {
            drop(worker); // joins before the mapped files are removed
            let _ = render_session.remove_owned_mapping();
            let _ = capture_session.remove_owned_mapping();
            return Err(ControlError::InvalidRequest(
                "the cable route worker did not start".into(),
            ));
        }
        route.worker = Some(worker);
        match NativeBridgeController::activate_prepared_section(CABLE_BRIDGE_DEVICE, render_session)
        {
            Ok(controller) => route.render = Some(controller),
            Err(error) => {
                // Join the worker first: Windows refuses to delete a file
                // that is still mapped.
                drop(route);
                let _ = capture_session.remove_owned_mapping();
                let _ = std::fs::remove_file(&render_path);
                return Err(bridge_error(
                    &format!(
                        "{} Input could not be opened",
                        cable_label(&shape.render_bus)
                    ),
                    &error,
                ));
            }
        }
        match NativeBridgeController::activate_prepared_section(
            CABLE_BRIDGE_DEVICE,
            capture_session,
        ) {
            Ok(controller) => route.capture = Some(controller),
            Err(error) => {
                drop(route);
                let _ = std::fs::remove_file(&capture_path);
                return Err(bridge_error(
                    &format!(
                        "{} Output could not be opened",
                        cable_label(&shape.capture_bus)
                    ),
                    &error,
                ));
            }
        }
        self.cable_route = Some(route);
        Ok(())
    }

    /// One backend service pass: heartbeat both leases every quarter lease
    /// and stop a failed route (its cable then outputs silence). Returns
    /// whether a route is running.
    #[cfg(windows)]
    pub(crate) fn service_cable_route(&mut self, now: std::time::Instant) -> bool {
        let Some(route) = self.cable_route.as_mut().filter(|route| route.is_running()) else {
            return false;
        };
        let mut failure = route
            .worker
            .as_ref()
            .and_then(|worker| worker.failure().map(str::to_owned));
        if failure.is_none()
            && now.saturating_duration_since(route.last_heartbeat)
                >= std::time::Duration::from_millis(u64::from(CABLE_ROUTE_LEASE_MS / 4))
        {
            route.last_heartbeat = now;
            for controller in [route.render.as_mut(), route.capture.as_mut()]
                .into_iter()
                .flatten()
            {
                if let Err(error) = controller.heartbeat() {
                    failure = Some(format!("cable lease heartbeat failed: {error:?}"));
                    break;
                }
            }
        }
        if let Some(failure) = failure {
            let _ = route.shutdown();
            route.failure = Some(failure);
            // The session is no longer playing; Play starts it again.
            let session_id = route.session_id.clone();
            if let Some(runtime) = self.runtimes.get_mut(&session_id) {
                runtime.stop();
            }
            let revision = self
                .get_session(&session_id)
                .map_or(0, |session| session.revision);
            self.events
                .append(revision, None, "runtime.stopped", Some(session_id));
            return false;
        }
        true
    }

    /// Stop the session's cable route, if any. Returns the first teardown
    /// error; the route is released either way.
    #[cfg(windows)]
    pub(crate) fn stop_cable_route(&mut self, session_id: &EntityId) -> Option<ControlError> {
        if self
            .cable_route_prepared
            .as_ref()
            .is_some_and(|(prepared, _)| prepared == session_id)
        {
            self.cable_route_prepared = None;
        }
        let route = self
            .cable_route
            .as_mut()
            .filter(|route| route.session_id == *session_id)?;
        route.shutdown().map(ControlError::InvalidRequest)
    }

    #[cfg(windows)]
    pub(crate) fn set_cable_route_privacy_muted(&self, muted: bool) {
        if let Some(worker) = self
            .cable_route
            .as_ref()
            .and_then(|route| route.worker.as_ref())
        {
            worker.set_privacy_muted(muted);
        }
    }

    #[cfg(windows)]
    pub(crate) fn cable_route_running(&self) -> bool {
        self.cable_route
            .as_ref()
            .is_some_and(ActiveCableRoute::is_running)
    }

    #[cfg(windows)]
    pub(crate) fn cable_route_session(&self) -> Option<&EntityId> {
        self.cable_route
            .as_ref()
            .filter(|route| route.is_running())
            .map(|route| &route.session_id)
            .or_else(|| self.cable_route_prepared.as_ref().map(|(id, _)| id))
    }

    /// The `cableRoute` diagnostics object (null when none was started).
    pub(crate) fn cable_route_diagnostics(&self) -> Value {
        #[cfg(windows)]
        if let Some(route) = &self.cable_route {
            return route.to_json();
        }
        Value::Null
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_domain::Port;

    fn cable(id: &str, kind: NodeKind, bus: &str, channels: u8) -> Node {
        let direction = if kind == NodeKind::VirtualRenderSource {
            PortDirection::Output
        } else {
            PortDirection::Input
        };
        Node {
            id: EntityId::new(id),
            kind,
            type_version: 1,
            name: id.into(),
            enabled: true,
            bypass: false,
            parameters: json!({ "busId": bus }).as_object().unwrap().clone(),
            ports: vec![Port {
                name: if direction == PortDirection::Output {
                    "out"
                } else {
                    "in"
                }
                .into(),
                direction,
                channels,
            }],
        }
    }

    fn session(nodes: Vec<Node>) -> Session {
        Session {
            id: EntityId::new("cable"),
            name: "Cable".into(),
            schema_version: 1,
            revision: 1,
            nodes,
            edges: Vec::new(),
        }
    }

    fn tool(kind: NodeKind) -> Node {
        Node {
            id: EntityId::new("tool"),
            kind,
            type_version: 1,
            name: "Tool".into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: Vec::new(),
        }
    }

    fn reason(session: &Session) -> String {
        match cable_route_shape(session) {
            Some(Err(ControlError::InvalidRequest(reason))) => reason,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn cable_route_shape_accepts_one_cable_in_and_one_out_with_tools() {
        let mut nodes = vec![
            cable("in", NodeKind::VirtualRenderSource, "cable-a", 2),
            cable("out", NodeKind::VirtualCaptureSink, "cable-h", 2),
            tool(NodeKind::Compressor),
        ];
        assert_eq!(
            cable_route_shape(&session(nodes.clone())).unwrap().unwrap(),
            CableRouteShape {
                render_bus: "cable-a".into(),
                capture_bus: "cable-h".into()
            }
        );
        // A disabled extra source is ignored.
        let mut extra = cable("in-2", NodeKind::VirtualRenderSource, "cable-c", 2);
        extra.enabled = false;
        nodes.push(extra);
        assert!(cable_route_shape(&session(nodes)).unwrap().is_ok());
    }

    #[test]
    fn cable_route_shape_ignores_sessions_without_a_cable_source() {
        assert!(cable_route_shape(&session(vec![cable(
            "out",
            NodeKind::VirtualCaptureSink,
            "cable-b",
            2
        )]))
        .is_none());
    }

    #[test]
    fn cable_route_shape_refuses_unsupported_routes_with_a_reason() {
        let a = || cable("in", NodeKind::VirtualRenderSource, "cable-a", 2);
        let b = || cable("out", NodeKind::VirtualCaptureSink, "cable-b", 2);
        assert!(reason(&session(vec![a()])).contains("exactly one"));
        let mut second = a();
        second.id = EntityId::new("in-2");
        assert!(reason(&session(vec![a(), second, b()])).contains("exactly one"));
        assert!(
            reason(&session(vec![a(), b(), tool(NodeKind::PhysicalOutput)]))
                .contains("cannot be used")
        );
        assert!(reason(&session(vec![a(), b(), tool(NodeKind::Plugin)])).contains("cannot be used"));
        assert!(reason(&session(vec![a(), b(), tool(NodeKind::Mixer)])).contains("cannot be used"));
        assert!(reason(&session(vec![
            a(),
            cable("out", NodeKind::VirtualCaptureSink, "cable-a", 2)
        ]))
        .contains("Cable A cannot feed itself"));
        assert!(reason(&session(vec![
            a(),
            cable("out", NodeKind::VirtualCaptureSink, "cable-i", 2)
        ]))
        .contains("choose the cable"));
        assert!(reason(&session(vec![
            a(),
            cable("out", NodeKind::VirtualCaptureSink, "cable-b", 1)
        ]))
        .contains("stereo"));
    }
    /// The control path on a PC without the AudioRouter driver: preparation
    /// records the route, Start fails with a clear message, and nothing is
    /// left running or on disk.
    #[cfg(windows)]
    #[test]
    fn cable_route_start_without_the_driver_fails_cleanly() {
        if audiorouter_windows_audio::NativeBridgeControlClient::open(CABLE_BRIDGE_DEVICE).is_ok() {
            eprintln!("AudioRouter driver present; skipping the no-driver path");
            return;
        }
        let mut plane = ControlPlane::default();
        let mut route = session(vec![
            cable("in", NodeKind::VirtualRenderSource, "cable-a", 2),
            cable("out", NodeKind::VirtualCaptureSink, "cable-b", 2),
        ]);
        route.id = EntityId::new("cable-no-driver");
        route.edges.push(audiorouter_domain::Edge {
            id: EntityId::new("in-out"),
            source_node: EntityId::new("in"),
            source_port: "out".into(),
            destination_node: EntityId::new("out"),
            destination_port: "in".into(),
            matrix: vec![1.0, 0.0, 0.0, 1.0],
            enabled: true,
        });
        plane.insert_session(route.clone()).unwrap();
        let prepared = plane
            .dispatch_native_paths_prepare(Some(json!({ "sessionId": route.id })))
            .unwrap();
        assert_eq!(prepared["adapter"], "cable-route");
        assert_eq!(prepared["renderBus"], "cable-a");
        assert_eq!(plane.native_adapter_kind(), Some("cable-route"));
        assert_eq!(plane.native_adapter_state(), "configured-stopped");
        let error = plane.session_start(&route.id).unwrap_err();
        assert!(
            matches!(&error, ControlError::InvalidRequest(message) if message.contains("cable driver is not available")),
            "{error:?}"
        );
        assert!(plane
            .runtimes
            .get(&route.id)
            .is_none_or(|runtime| runtime.state() != RuntimeState::Running));
        assert!(plane.cable_route.is_none());
        // Nothing stays prepared: a later Play prepares from the session as
        // it is then, and no stale route can start beside another worker.
        assert!(plane.cable_route_prepared.is_none());
        assert_eq!(plane.native_adapter_kind(), None);
        assert_eq!(plane.cable_route_diagnostics(), Value::Null);
        let prefix = format!("audiorouter-cable-{}-", std::process::id());
        assert!(!std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().starts_with(&prefix)));
        // A fresh preparation followed by Stop releases it too.
        plane
            .dispatch_native_paths_prepare(Some(json!({ "sessionId": route.id })))
            .unwrap();
        plane.session_stop(&route.id).unwrap();
        assert_eq!(plane.native_adapter_kind(), None);
    }
}
