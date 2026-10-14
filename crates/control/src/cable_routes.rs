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
/// Route rate when Windows reports neither cable endpoint.
pub(crate) const CABLE_ROUTE_RATE_HZ: u32 = 48_000;
/// Bridge block when the driver reports no preferred period (0) or one
/// outside `DefaultPeriodFrames`' range (spec 17 §5.5).
pub(crate) const CABLE_ROUTE_FALLBACK_FRAMES: u16 = 480;

/// The route's bridge block: the driver's `DefaultPeriodFrames`, which
/// AudioRouter uses as its preferred period when it opens a cable (user
/// decision 2026-10-11: follow the cable setting; a low-latency switch
/// lowers that one value). Range 128–960, else the 480-frame fallback.
pub(crate) fn cable_route_block_frames(default_period_frames: u32) -> u16 {
    match u16::try_from(default_period_frames) {
        Ok(frames @ 128..=960) => frames,
        _ => CABLE_ROUTE_FALLBACK_FRAMES,
    }
}
pub(crate) const CABLE_ROUTE_CHANNELS: u16 = 2;
pub(crate) const CABLE_ROUTE_LEASE_MS: u32 = 2_000;
/// Rates a cable offers (VCAB-11); a route runs at its cables' rate.
const CABLE_ROUTE_RATES: [u32; 3] = [44_100, 48_000, 96_000];

/// A cable endpoint's current Windows shared-mode format (its mix format).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CableEndpointFormat {
    pub sample_rate_hz: u32,
    pub channels: u16,
}

/// Whether `reference`, a lowercase KS filter reference string, is the
/// filter behind `bus`'s Input (render) or Output (capture) endpoint. The
/// driver's filter names are fixed, so a rename in Sound settings cannot
/// change the match (17 §5.5).
pub(crate) fn is_cable_filter(reference: &str, bus: &str, render: bool) -> bool {
    let side = format!(
        "cable{}{}",
        &bus[6..],
        if render { "render" } else { "capture" }
    );
    reference.strip_prefix("wave") == Some(side.as_str())
        || reference.strip_prefix("topology") == Some(side.as_str())
}

fn rate_label(rate: u32) -> String {
    if rate % 1_000 == 0 {
        format!("{} kHz", rate / 1_000)
    } else {
        format!("{:.1} kHz", f64::from(rate) / 1_000.0)
    }
}

/// The rate a cable route runs at: its cables' common Windows rate. The
/// driver refuses a lease at another rate or channel count (wrong speed is
/// worse than silence, 17 §5.2), and a cable-only route never resamples
/// (17 §7.3), so a mismatch is refused with the setting to change. An
/// endpoint Windows does not report leaves the other side's rate, else
/// 48 kHz; the running route still stops on a driver format mismatch.
pub(crate) fn cable_route_rate(
    shape: &CableRouteShape,
    render: Option<CableEndpointFormat>,
    capture: Option<CableEndpointFormat>,
) -> Result<u32, ControlError> {
    let refuse = |reason: String| Err(ControlError::InvalidRequest(reason));
    let sides = [
        (render, &shape.render_bus, "Input", shape.render_channels),
        (
            capture,
            &shape.capture_bus,
            "Output",
            shape.capture_channels,
        ),
    ];
    for (format, bus, side, channels) in sides {
        if let Some(format) = format {
            if format.channels != channels {
                let count = |channels: u16| match channels {
                    1 => "1 channel".to_owned(),
                    many => format!("{many} channels"),
                };
                return refuse(format!(
                    "{} {side} is set to {} in Windows but the route uses {}; set the cable to {} in Sound settings, or change the route",
                    cable_label(bus),
                    count(format.channels),
                    count(channels),
                    count(channels)
                ));
            }
        }
    }
    let rate = match (render, capture) {
        (Some(render), Some(capture)) if render.sample_rate_hz != capture.sample_rate_hz => {
            return refuse(format!(
                "{} Input runs at {} and {} Output at {} in Windows; set both cables to the same rate in Sound settings (a cable route does not resample)",
                cable_label(&shape.render_bus),
                rate_label(render.sample_rate_hz),
                cable_label(&shape.capture_bus),
                rate_label(capture.sample_rate_hz)
            ));
        }
        (Some(format), _) | (None, Some(format)) => format.sample_rate_hz,
        (None, None) => CABLE_ROUTE_RATE_HZ,
    };
    if !CABLE_ROUTE_RATES.contains(&rate) {
        return refuse(format!(
            "the cables run at {} Hz; set them to 44.1, 48 or 96 kHz in Sound settings",
            rate
        ));
    }
    Ok(rate)
}

/// Both cables' Windows formats, found by filter reference; `None` for an
/// endpoint Windows does not report (disabled, or the query failed).
#[cfg(windows)]
fn cable_endpoint_formats(
    shape: &CableRouteShape,
) -> (Option<CableEndpointFormat>, Option<CableEndpointFormat>) {
    use audiorouter_windows_audio::{
        enumerate_active_endpoint_display_info, enumerate_active_endpoints, EndpointDirection,
    };
    let (Ok(formats), Ok(endpoints)) = (
        enumerate_active_endpoints(),
        enumerate_active_endpoint_display_info(),
    ) else {
        return (None, None);
    };
    let find = |bus: &str, direction: EndpointDirection| {
        let render = direction == EndpointDirection::Render;
        endpoints
            .iter()
            .filter(|endpoint| {
                endpoint.direction == direction
                    && is_cable_filter(&endpoint.filter_reference, bus, render)
            })
            .find_map(|endpoint| {
                formats
                    .iter()
                    .find(|format| format.id == endpoint.id && format.direction == direction)
            })
            .map(|format| CableEndpointFormat {
                sample_rate_hz: format.sample_rate_hz,
                channels: format.channels,
            })
    };
    (
        find(&shape.render_bus, EndpointDirection::Render),
        find(&shape.capture_bus, EndpointDirection::Capture),
    )
}

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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CableRouteShape {
    pub render_bus: String,
    pub capture_bus: String,
    /// Channels of the source cable (its render source) and of the
    /// destination cable (its capture sink).
    pub render_channels: u16,
    pub capture_channels: u16,
    /// A multichannel route (WP-09 slice 5): the direct edge's matrix,
    /// mapped without the engine. `None`: the stereo engine graph.
    pub direct_matrix: Option<Vec<f32>>,
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
    let channels = |node: &Node, direction| {
        node.ports
            .iter()
            .find(|port| port.direction == direction)
            .map(|port| u16::from(port.channels))
    };
    let (Some(render_channels), Some(capture_channels)) = (
        channels(source, PortDirection::Output),
        channels(sink, PortDirection::Input),
    ) else {
        return refuse("each cable node needs its audio port".into());
    };
    if render_channels == CABLE_ROUTE_CHANNELS && capture_channels == CABLE_ROUTE_CHANNELS {
        return Some(Ok(CableRouteShape {
            render_bus,
            capture_bus,
            render_channels,
            capture_channels,
            direct_matrix: None,
        }));
    }
    // Multichannel: the cables are connected directly and the edge's matrix
    // maps the channels (pass-through, up- or downmix); tools are stereo.
    if session.nodes.iter().any(|node| {
        node.enabled
            && !matches!(
                node.kind,
                NodeKind::VirtualRenderSource | NodeKind::VirtualCaptureSink
            )
    }) {
        return refuse(format!(
            "tools work on stereo cable routes only; connect the {}-channel and {}-channel cables directly, or set both to 2 channels",
            render_channels, capture_channels
        ));
    }
    let links: Vec<&audiorouter_domain::Edge> =
        session.edges.iter().filter(|edge| edge.enabled).collect();
    let [link] = links.as_slice() else {
        return refuse("connect the two cables with one direct connection".into());
    };
    if link.source_node != source.id || link.destination_node != sink.id {
        return refuse("connect the two cables with one direct connection".into());
    }
    Some(Ok(CableRouteShape {
        render_bus,
        capture_bus,
        render_channels,
        capture_channels,
        direct_matrix: Some(link.matrix.clone()),
    }))
}

#[cfg(windows)]
pub(crate) struct ActiveCableRoute {
    pub session_id: EntityId,
    pub generation: u64,
    pub shape: CableRouteShape,
    pub block_frames: u16,
    pub sample_rate_hz: u32,
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
            "blockFrames": self.block_frames,
            "sampleRateHz": self.sample_rate_hz,
            "renderChannels": self.shape.render_channels,
            "captureChannels": self.shape.capture_channels,
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
            CableMatrixRoute, CableRouteProcessor, CableRouteStage, CableRouteWorker,
            CableRouteWorkerConfig, NativeBridgeControlClient, NativeBridgeController,
            NativeBridgeRegion, NativeBridgeSession,
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
        let frames = cable_route_block_frames(info.default_period_frames);
        let (render_format, capture_format) = cable_endpoint_formats(&shape);
        let rate = cable_route_rate(&shape, render_format, capture_format)?;
        let stage = match &shape.direct_matrix {
            Some(matrix) => CableRouteStage::Matrix(
                CableMatrixRoute::new(
                    usize::from(shape.render_channels),
                    usize::from(shape.capture_channels),
                    usize::from(frames),
                    matrix,
                )
                .map_err(|error| bridge_error("cable channel mapping", &error))?,
            ),
            None => {
                let graph = audiorouter_engine::compile_session_at_sample_rate(
                    &session,
                    audiorouter_engine::RuntimeGeneration::new(generation),
                    rate,
                )
                .map_err(|error| bridge_error("cable route compile", &error))?;
                CableRouteStage::Engine(Box::new(
                    CableRouteProcessor::new(
                        usize::from(CABLE_ROUTE_CHANNELS),
                        usize::from(frames),
                        graph,
                    )
                    .map_err(|error| bridge_error("cable route", &error))?,
                ))
            }
        };
        // A fresh lease generation per start: a stale mapping of an earlier
        // run can never be mistaken for this one.
        let lease_generation = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(generation)
            .max(1);
        let hello = |bus: &str, direction, channels: u16| AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: bus.to_owned(),
            direction,
            generation: lease_generation,
            sample_rate_hz: rate,
            channels,
            frames_per_quantum: frames,
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
        let render_block_samples = usize::from(frames) * usize::from(shape.render_channels);
        let capture_block_samples = usize::from(frames) * usize::from(shape.capture_channels);
        let render_session = NativeBridgeSession::create(
            &render_path,
            hello(
                &shape.render_bus,
                AudioBridgeDirection::RenderSource,
                shape.render_channels,
            ),
        )
        .map_err(|error| bridge_error("cable input preparation failed", &error))?;
        // From here every early return drops what exists; sessions and
        // controllers remove their own mapping files.
        let capture_session = match NativeBridgeSession::create_primed(
            &capture_path,
            hello(
                &shape.capture_bus,
                AudioBridgeDirection::CaptureSink,
                shape.capture_channels,
            ),
            &vec![0.0; capture_block_samples],
        ) {
            Ok(session) => session,
            Err(error) => {
                let _ = render_session.remove_owned_mapping();
                return Err(bridge_error("cable output preparation failed", &error));
            }
        };
        let views = NativeBridgeRegion::open(&render_path, shape.render_channels, frames).and_then(
            |render| {
                NativeBridgeRegion::open(&capture_path, shape.capture_channels, frames)
                    .map(|capture| (render, capture))
            },
        );
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
            block_frames: frames,
            sample_rate_hz: rate,
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
            stage,
            CableRouteWorkerConfig {
                render_generation: lease_generation,
                capture_generation: lease_generation,
                render_block_samples,
                capture_block_samples,
                // Half a block period (5 ms at 480 frames and 48 kHz).
                silence_holdoff_us: u64::from(frames) * 1_000_000 / u64::from(rate) / 2,
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
            // The driver plays silence for a lease whose rate or channels no
            // longer match the cable (format changed in Sound settings while
            // playing); say so instead of running silent.
            if failure.is_none() {
                failure = [
                    (route.render.as_ref(), &route.shape.render_bus, "Input"),
                    (route.capture.as_ref(), &route.shape.capture_bus, "Output"),
                ]
                .into_iter()
                .find(|(controller, _, _)| {
                    controller.is_some_and(|controller| controller.counters().format_mismatches > 0)
                })
                .map(|(_, bus, side)| {
                    format!(
                        "{} {side} changed format in Windows (sample rate or channels); press Play to start the route at the new format",
                        cable_label(bus)
                    )
                });
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
                capture_bus: "cable-h".into(),
                render_channels: 2,
                capture_channels: 2,
                direct_matrix: None,
            }
        );
        // A disabled extra source is ignored.
        let mut extra = cable("in-2", NodeKind::VirtualRenderSource, "cable-c", 2);
        extra.enabled = false;
        nodes.push(extra);
        assert!(cable_route_shape(&session(nodes)).unwrap().is_ok());
    }

    fn stereo_shape() -> CableRouteShape {
        CableRouteShape {
            render_bus: "cable-a".into(),
            capture_bus: "cable-b".into(),
            render_channels: 2,
            capture_channels: 2,
            direct_matrix: None,
        }
    }

    fn format(sample_rate_hz: u32, channels: u16) -> Option<CableEndpointFormat> {
        Some(CableEndpointFormat {
            sample_rate_hz,
            channels,
        })
    }

    fn rate_refusal(
        shape: &CableRouteShape,
        render: Option<CableEndpointFormat>,
        capture: Option<CableEndpointFormat>,
    ) -> String {
        match cable_route_rate(shape, render, capture) {
            Err(ControlError::InvalidRequest(reason)) => reason,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn cable_filters_match_by_letter_and_side_only() {
        assert!(is_cable_filter("topologycablearender", "cable-a", true));
        assert!(is_cable_filter("wavecablearender", "cable-a", true));
        assert!(is_cable_filter("topologycablebcapture", "cable-b", false));
        assert!(!is_cable_filter("topologycablebcapture", "cable-b", true));
        assert!(!is_cable_filter("topologycablearender", "cable-b", true));
        assert!(!is_cable_filter("topologycablehrender", "cable-a", true));
        assert!(!is_cable_filter("xtopologycablearender", "cable-a", true));
        assert!(!is_cable_filter("", "cable-a", true));
    }

    #[test]
    fn cable_route_runs_at_the_cables_common_rate() {
        let shape = stereo_shape();
        // The r7 Stage A defect: 44.1 and 96 kHz cables got a 48 kHz lease.
        for rate in [44_100, 48_000, 96_000] {
            assert_eq!(
                cable_route_rate(&shape, format(rate, 2), format(rate, 2)).unwrap(),
                rate
            );
        }
        assert_eq!(
            cable_route_rate(&shape, None, format(96_000, 2)).unwrap(),
            96_000
        );
        assert_eq!(
            cable_route_rate(&shape, format(44_100, 2), None).unwrap(),
            44_100
        );
        assert_eq!(
            cable_route_rate(&shape, None, None).unwrap(),
            CABLE_ROUTE_RATE_HZ
        );
        let mut eight = stereo_shape();
        eight.render_channels = 8;
        eight.capture_channels = 8;
        assert_eq!(
            cable_route_rate(&eight, format(96_000, 8), format(96_000, 8)).unwrap(),
            96_000
        );
    }

    #[test]
    fn cable_route_refuses_mismatched_cable_formats_with_the_setting_to_change() {
        let shape = stereo_shape();
        assert_eq!(
            rate_refusal(&shape, format(44_100, 2), format(48_000, 2)),
            "Cable A Input runs at 44.1 kHz and Cable B Output at 48 kHz in Windows; set both cables to the same rate in Sound settings (a cable route does not resample)"
        );
        assert_eq!(
            rate_refusal(&shape, format(48_000, 8), format(48_000, 2)),
            "Cable A Input is set to 8 channels in Windows but the route uses 2 channels; set the cable to 2 channels in Sound settings, or change the route"
        );
        assert!(rate_refusal(&shape, None, format(48_000, 1))
            .starts_with("Cable B Output is set to 1 channel in"));
        assert!(rate_refusal(&shape, format(32_000, 2), None).contains("32000 Hz"));
    }

    #[test]
    fn cable_route_block_follows_the_driver_default_period() {
        assert_eq!(cable_route_block_frames(480), 480);
        assert_eq!(cable_route_block_frames(128), 128);
        assert_eq!(cable_route_block_frames(960), 960);
        // Not reported (no LOW_LATENCY_PERIODS) or outside the registry range.
        for value in [0, 64, 127, 961, 4_096, u32::MAX] {
            assert_eq!(
                cable_route_block_frames(value),
                CABLE_ROUTE_FALLBACK_FRAMES,
                "{value}"
            );
        }
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
        // Not stereo: needs one direct connection and no tools.
        assert!(reason(&session(vec![
            a(),
            cable("out", NodeKind::VirtualCaptureSink, "cable-b", 1)
        ]))
        .contains("one direct connection"));
        assert!(reason(&session(vec![
            cable("in", NodeKind::VirtualRenderSource, "cable-a", 8),
            b(),
            tool(NodeKind::Compressor)
        ]))
        .contains("tools work on stereo cable routes only"));
    }

    fn link(matrix: Vec<f32>) -> audiorouter_domain::Edge {
        audiorouter_domain::Edge {
            id: EntityId::new("in-out"),
            source_node: EntityId::new("in"),
            source_port: "out".into(),
            destination_node: EntityId::new("out"),
            destination_port: "in".into(),
            matrix,
            enabled: true,
        }
    }

    #[test]
    fn cable_route_shape_maps_multichannel_cables_directly() {
        let identity: Vec<f32> = (0..64).map(|i| f32::from(u8::from(i % 9 == 0))).collect();
        let mut route = session(vec![
            cable("in", NodeKind::VirtualRenderSource, "cable-a", 8),
            cable("out", NodeKind::VirtualCaptureSink, "cable-b", 8),
        ]);
        route.edges.push(link(identity.clone()));
        let shape = cable_route_shape(&route).unwrap().unwrap();
        assert_eq!((shape.render_channels, shape.capture_channels), (8, 8));
        assert_eq!(shape.direct_matrix, Some(identity));
        // 7.1 down to stereo keeps the edge's matrix.
        let mut down = session(vec![
            cable("in", NodeKind::VirtualRenderSource, "cable-a", 8),
            cable("out", NodeKind::VirtualCaptureSink, "cable-b", 2),
        ]);
        down.edges.push(link(vec![0.5; 16]));
        let shape = cable_route_shape(&down).unwrap().unwrap();
        assert_eq!((shape.render_channels, shape.capture_channels), (8, 2));
        assert_eq!(shape.direct_matrix.unwrap().len(), 16);
        // A stereo route keeps the engine path, even when connected directly.
        let mut stereo = session(vec![
            cable("in", NodeKind::VirtualRenderSource, "cable-a", 2),
            cable("out", NodeKind::VirtualCaptureSink, "cable-b", 2),
        ]);
        stereo.edges.push(link(vec![1.0, 0.0, 0.0, 1.0]));
        assert_eq!(
            cable_route_shape(&stereo).unwrap().unwrap().direct_matrix,
            None
        );
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
