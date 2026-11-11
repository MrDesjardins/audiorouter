//! Hold-to-compare (DEC-UI-21b): hear a running route without some tools,
//! without saving anything. The comparison lives only in memory and only in
//! the running graph, ends when released, on any save, on Stop, and by itself
//! after `MAX_HOLD`, so a closed window or a crash cannot leave a voice route
//! unprocessed.

use super::simple::resolve_node;
use super::*;

/// The longest one hold lasts when the client never releases it.
const MAX_HOLD: Duration = Duration::from_secs(30);

pub(crate) struct Comparison {
    nodes: Vec<EntityId>,
    expires_at: Instant,
}

/// Tools whose absence can be heard while the route stays connected. A source,
/// output or mixing tool would cut the route instead of comparing it.
fn can_be_compared(kind: NodeKind) -> bool {
    match kind {
        NodeKind::Gain
        | NodeKind::Volume
        | NodeKind::ParametricEq
        | NodeKind::GraphicEq
        | NodeKind::BassTreble
        | NodeKind::Compressor
        | NodeKind::Gate
        | NodeKind::Limiter
        | NodeKind::Delay
        | NodeKind::Pitch
        | NodeKind::Dehum
        | NodeKind::Declick
        | NodeKind::Denoise
        | NodeKind::SpeechDenoise
        | NodeKind::SpectralGate
        | NodeKind::FirFilter
        | NodeKind::FunVoiceChanger
        | NodeKind::Duck
        | NodeKind::Plugin => true,
        NodeKind::PhysicalInput
        | NodeKind::PhysicalOutput
        | NodeKind::ApplicationCapture
        | NodeKind::EndpointLoopback
        | NodeKind::VirtualRenderSource
        | NodeKind::VirtualCaptureSink
        | NodeKind::TestSignal
        | NodeKind::Soundboard
        | NodeKind::AudioFile
        | NodeKind::NetworkSend
        | NodeKind::NetworkReceive
        | NodeKind::Mixer
        | NodeKind::InputSwitch
        | NodeKind::OutputSwitch
        | NodeKind::Mute
        | NodeKind::Meter
        | NodeKind::Recorder
        | NodeKind::TimeShift => false,
    }
}

impl ControlPlane {
    fn republish_comparison(
        &mut self,
        session: &EntityId,
        generation: u64,
    ) -> Result<Option<&'static str>, ControlError> {
        // Exercise dispatch/expiry publication independently of endpoint
        // ownership. Production always uses the real native worker below.
        #[cfg(test)]
        if let Some(mut publications) = self.comparison_test_publications.take() {
            let published = self.with_comparison(self.get_session(session)?.clone());
            publications.push(published);
            self.comparison_test_publications = Some(publications);
            return Ok(Some("test"));
        }
        self.republish_running_native_graph(session, generation, true)
    }

    /// The graph to publish: the saved one, with any held comparison applied.
    pub(crate) fn with_comparison(&self, mut session: Session) -> Session {
        if let Some(comparison) = self
            .comparison
            .get(&session.id)
            .filter(|comparison| comparison.expires_at > Instant::now())
        {
            for node in &mut session.nodes {
                if comparison.nodes.contains(&node.id) {
                    node.bypass = true;
                }
            }
        }
        session
    }

    /// Forget a comparison without republishing, because the caller is about
    /// to replace or stop the graph.
    pub(crate) fn clear_comparison(&mut self, session: &EntityId) {
        self.comparison.remove(session);
    }

    /// Restore a hold that was never released. Called from the audio service.
    pub(crate) fn expire_comparison(&mut self, now: Instant) {
        if self.comparison.is_empty() {
            return;
        }
        // An empty Vec does not allocate, so ordinary service passes stay free.
        let expired: Vec<_> = self
            .comparison
            .iter()
            .filter(|(_, comparison)| comparison.expires_at <= now)
            .map(|(session, _)| session.clone())
            .collect();
        for session in expired {
            self.clear_comparison(&session);
            if let Some(generation) = self
                .runtimes
                .get(&session)
                .filter(|runtime| runtime.state() == RuntimeState::Running)
                .map(FakeRuntime::generation)
            {
                // A failure must not stop the audio service.
                let _ = self.republish_comparison(&session, generation);
            }
        }
    }

    pub(crate) fn dispatch_nodes_compare(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|key| !key.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let session_id = match params.get("sessionId").and_then(Value::as_str) {
            Some(id) => EntityId::new(id),
            None => self.active_session_id.clone().ok_or_else(|| {
                ControlError::InvalidRequest("no session is open; pass sessionId".into())
            })?,
        };
        self.ensure_session_loaded(&session_id)?;
        let references = params
            .get("nodes")
            .and_then(Value::as_array)
            .ok_or_else(|| ControlError::InvalidRequest("nodes is required".into()))?;
        let session = self.get_session(&session_id)?.clone();
        let mut nodes = Vec::with_capacity(references.len());
        for reference in references {
            let reference = reference.as_str().ok_or_else(|| {
                ControlError::InvalidRequest("nodes must list IDs or names".into())
            })?;
            let node = resolve_node(&session, reference)?;
            if !can_be_compared(node.kind) {
                return Err(ControlError::InvalidRequest(format!(
                    "\"{}\" cannot be left out for a comparison",
                    node.name
                )));
            }
            nodes.push(node.id.clone());
        }
        let generation = self
            .runtimes
            .get(&session_id)
            .filter(|runtime| runtime.state() == RuntimeState::Running)
            .map(FakeRuntime::generation);
        let Some(generation) = generation else {
            self.clear_comparison(&session_id);
            if nodes.is_empty() {
                return Ok(json!({ "comparing": [], "expiresInMs": 0 }));
            }
            return Err(ControlError::InvalidRequest(
                "press Play first; a comparison needs the route to be playing".into(),
            ));
        };
        let releasing = nodes.is_empty();
        if releasing {
            self.clear_comparison(&session_id);
        } else {
            self.comparison.insert(
                session_id.clone(),
                Comparison {
                    nodes: nodes.clone(),
                    expires_at: Instant::now() + MAX_HOLD,
                },
            );
        }
        match self.republish_comparison(&session_id, generation) {
            Ok(Some(_)) => {}
            // Nothing is playing through a native worker, so there is nothing to change.
            Ok(None) if releasing => {}
            Ok(None) => {
                self.clear_comparison(&session_id);
                return Err(ControlError::InvalidRequest(
                    "this route cannot be compared while it plays".into(),
                ));
            }
            Err(error) => {
                self.clear_comparison(&session_id);
                let _ = self.republish_comparison(&session_id, generation);
                return Err(error);
            }
        }
        Ok(json!({
            "comparing": nodes,
            "expiresInMs": if releasing { 0 } else { MAX_HOLD.as_millis() as u64 },
        }))
    }
}

#[cfg(test)]
#[path = "compare_tests.rs"]
mod tests;
