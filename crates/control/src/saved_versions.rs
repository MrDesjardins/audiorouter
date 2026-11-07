//! Saved versions of a session: when each was saved, by which client (UI-07:
//! the authenticated client's identity, never a guessed person), and going
//! back to one through the ordinary plan and commit.

use super::*;

const NOTES_PER_SESSION: usize = 200;
const PAGE: usize = 100;
const THIS_PC: &str = "this PC";

fn notes_key(session: &EntityId) -> String {
    format!("revisionNotes:{}", session.as_str())
}

fn required_session(params: &Value) -> Result<EntityId, ControlError> {
    params
        .get("sessionId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(EntityId::new)
        .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))
}

impl ControlPlane {
    fn revision_notes(&self, session: &EntityId) -> Vec<Value> {
        if let Some(notes) = self.revision_notes.get(session) {
            return notes.clone();
        }
        self.storage
            .as_ref()
            .and_then(|storage| {
                storage
                    .load_control_value(&notes_key(session))
                    .ok()
                    .flatten()
            })
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Remember who saved a revision and when. A failed note must never undo
    /// a save that already succeeded, so it only leaves the version unlabelled.
    pub(crate) fn note_saved_revision(&mut self, session: &EntityId, revision: u64) {
        let mut notes = self.revision_notes(session);
        let saved_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64);
        // An enrolled client is named; anything else on this PC, such as the
        // window, connects as the Windows user, whose ID is not for display.
        let saved_by = self.active_idempotency_scope.as_ref().map(|client| {
            if self.enrollments.contains_key(client) {
                client.clone()
            } else {
                THIS_PC.to_owned()
            }
        });
        notes.push(json!({
            "revision": revision,
            "savedAtMs": saved_at_ms,
            "savedBy": saved_by,
        }));
        if notes.len() > NOTES_PER_SESSION {
            notes.drain(..notes.len() - NOTES_PER_SESSION);
        }
        if let (Some(storage), Ok(text)) = (&self.storage, serde_json::to_string(&notes)) {
            let _ = storage.save_control_value(&notes_key(session), &text);
        }
        self.revision_notes.insert(session.clone(), notes);
    }

    pub(crate) fn forget_revision_notes(&mut self, session: &EntityId) {
        self.revision_notes.remove(session);
        if let Some(storage) = &self.storage {
            let _ = storage.delete_control_value(&notes_key(session));
        }
    }

    pub(crate) fn dispatch_graph_versions(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let session_id = required_session(&params)?;
        self.ensure_session_loaded(&session_id)?;
        let page = self.graph_history_page(&session_id, None, PAGE)?;
        let notes = self.revision_notes(&session_id);
        let items = page["items"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|entry| {
                let note = notes
                    .iter()
                    .rev()
                    .find(|note| note["revision"] == entry["revision"]);
                json!({
                    "revision": entry["revision"],
                    "name": entry["name"],
                    "toolCount": entry["nodes"].as_array().map_or(0, Vec::len),
                    "connectionCount": entry["edges"].as_array().map_or(0, Vec::len),
                    "savedAtMs": note.map_or(Value::Null, |note| note["savedAtMs"].clone()),
                    "savedBy": note.map_or(Value::Null, |note| note["savedBy"].clone()),
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({ "items": items }))
    }

    pub(crate) fn dispatch_graph_restore_plan(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let session_id = required_session(&params)?;
        let base_revision = params
            .get("baseRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("baseRevision is required".into()))?;
        let revision = params
            .get("revision")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("revision is required".into()))?;
        if revision == base_revision {
            return Err(ControlError::InvalidRequest(
                "this session is already at that version".into(),
            ));
        }
        self.ensure_session_loaded(&session_id)?;
        let page = self.graph_history_page(&session_id, Some(revision.saturating_add(1)), 1)?;
        let candidate: Session = page["items"]
            .get(0)
            .filter(|entry| entry["revision"] == revision)
            .cloned()
            .ok_or_else(|| {
                ControlError::InvalidRequest(format!(
                    "version {revision} is no longer kept for this session"
                ))
            })
            .and_then(|entry| {
                serde_json::from_value(entry).map_err(|error| ControlError::Json(error.to_string()))
            })?;
        let plan_id = self.plan_graph(&session_id, base_revision, candidate)?;
        Ok(json!({ "planId": plan_id, "baseRevision": base_revision, "expiresInMs": 300000 }))
    }
}

#[cfg(test)]
#[path = "saved_versions_tests.rs"]
mod tests;
