//! Soundboard API: trigger, stop and status of a running Soundboard tool.
//!
//! These calls only push a command onto the tool's lock-free queue or read
//! its atomics, so they stay short and never touch the audio callback's
//! locks (see the Soundboard tool plan).

use super::*;
use audiorouter_domain::SOUNDBOARD_SLOTS;

impl ControlPlane {
    /// The prepared Soundboard of a running session.
    fn running_soundboard(
        &self,
        params: &Value,
    ) -> Result<
        (
            EntityId,
            EntityId,
            std::sync::Arc<audiorouter_engine::SoundboardSource>,
        ),
        ControlError,
    > {
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let node_id = params
            .get("nodeId")
            .and_then(Value::as_str)
            .map(EntityId::new)
            .ok_or_else(|| ControlError::InvalidRequest("nodeId is required".into()))?;
        if !self
            .runtimes
            .get(&session_id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
        {
            return Err(ControlError::InvalidRequest(
                "start the prepared session before using its Soundboard".into(),
            ));
        }
        let source = self
            .soundboard_sources
            .get(&(session_id.clone(), node_id.clone()))
            .cloned()
            .ok_or_else(|| {
                ControlError::InvalidRequest(
                    "this Soundboard is not prepared for the running session".into(),
                )
            })?;
        Ok((session_id, node_id, source))
    }

    fn soundboard_slot(params: &Value) -> Result<Option<usize>, ControlError> {
        match params.get("slot") {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value
                .as_u64()
                .filter(|slot| (1..=SOUNDBOARD_SLOTS as u64).contains(slot))
                .map(|slot| Some(slot as usize - 1))
                .ok_or_else(|| {
                    ControlError::InvalidRequest(format!(
                        "slot must be a whole number from 1 to {SOUNDBOARD_SLOTS}"
                    ))
                }),
        }
    }

    pub(crate) fn dispatch_soundboard_trigger(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId, nodeId and slot are required".into())
        })?;
        let slot = Self::soundboard_slot(&params)?
            .ok_or_else(|| ControlError::InvalidRequest("slot is required".into()))?;
        let press = match params.get("action").and_then(Value::as_str) {
            None | Some("press") => true,
            Some("release") => false,
            Some(_) => {
                return Err(ControlError::InvalidRequest(
                    "action must be press or release".into(),
                ))
            }
        };
        let (session_id, node_id, source) = self.running_soundboard(&params)?;
        let accepted = if press {
            source.trigger(slot)
        } else {
            source.release(slot)
        };
        Ok(json!({ "sessionId": session_id, "nodeId": node_id,
            "slot": slot + 1, "accepted": accepted }))
    }

    pub(crate) fn dispatch_soundboard_stop(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and nodeId are required".into())
        })?;
        let slot = Self::soundboard_slot(&params)?;
        let (session_id, node_id, source) = self.running_soundboard(&params)?;
        let stopped = match slot {
            Some(slot) => source.stop(slot),
            None => source.stop_all(),
        };
        Ok(json!({ "sessionId": session_id, "nodeId": node_id, "stopped": stopped }))
    }

    pub(crate) fn dispatch_soundboard_status(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and nodeId are required".into())
        })?;
        let (session_id, node_id, source) = self.running_soundboard(&params)?;
        let playing = source.playing_mask();
        let slots = (0..SOUNDBOARD_SLOTS)
            .map(|slot| {
                json!({
                    "slot": slot + 1,
                    "loaded": source.slot_loaded(slot),
                    "playing": playing & (1 << slot) != 0,
                    "progress": f64::from(source.progress_per_mille(slot)) / 1000.0,
                })
            })
            .collect::<Vec<_>>();
        Ok(
            json!({ "sessionId": session_id, "nodeId": node_id, "slots": slots,
            "ignoredTriggers": source.ignored_triggers(),
            "droppedCommands": source.dropped_commands() }),
        )
    }
}
