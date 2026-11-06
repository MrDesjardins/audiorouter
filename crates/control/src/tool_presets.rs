//! Named settings a person keeps for one kind of tool (UI-05). A preset holds
//! settings only, never a device, application or file identity, and applies
//! through the ordinary `nodes.set` or a draft edit.

use super::simple::{default_parameters, node_template};
use super::*;
use audiorouter_domain::Node;

const SETTINGS_KEY: &str = "toolPresets";
const MAX_PRESETS: usize = 200;
const MAX_NAME_CHARS: usize = 64;
const MAX_PARAMETERS_BYTES: usize = 64 * 1024;

/// Tools with settings worth naming. Kinds whose settings identify a device,
/// file, computer or plugin state are left out on purpose.
fn takes_presets(kind: NodeKind) -> bool {
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
        | NodeKind::FunVoiceChanger => true,
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
        | NodeKind::TimeShift
        | NodeKind::Duck
        | NodeKind::FirFilter
        | NodeKind::Plugin => false,
    }
}

fn required_key(params: &Value) -> Result<&str, ControlError> {
    params
        .get("idempotencyKey")
        .and_then(Value::as_str)
        .filter(|key| !key.is_empty())
        .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))
}

fn parse_kind(text: &str) -> Result<NodeKind, ControlError> {
    serde_json::from_value(json!(text))
        .map_err(|_| ControlError::InvalidRequest(format!("unknown tool kind \"{text}\"")))
}

impl ControlPlane {
    fn load_tool_presets(&self) -> Result<Vec<Value>, ControlError> {
        if let Some(storage) = &self.storage {
            let stored = storage
                .load_control_value(SETTINGS_KEY)
                .map_err(storage_error)?;
            return Ok(stored
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_default());
        }
        Ok(self.tool_presets.clone())
    }

    fn store_tool_presets(&mut self, presets: Vec<Value>) -> Result<(), ControlError> {
        if let Some(storage) = &self.storage {
            let text = serde_json::to_string(&presets)
                .map_err(|error| ControlError::Json(error.to_string()))?;
            storage
                .save_control_value(SETTINGS_KEY, &text)
                .map_err(storage_error)?;
        }
        self.tool_presets = presets;
        Ok(())
    }

    pub(crate) fn dispatch_tool_presets_list(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let kind = params
            .as_ref()
            .and_then(|params| params.get("kind"))
            .and_then(Value::as_str);
        let items = self
            .load_tool_presets()?
            .into_iter()
            .filter(|preset| kind.is_none_or(|kind| preset["kind"] == kind))
            .collect::<Vec<_>>();
        Ok(json!({ "items": items }))
    }

    pub(crate) fn dispatch_tool_presets_save(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let scoped = self.scoped_idempotency_key("toolPresets.save", required_key(&params)?);
        let hash = Self::request_hash(&json!({
            "kind": params.get("kind"),
            "name": params.get("name"),
            "parameters": params.get("parameters"),
        }));
        if let Some(previous) = self.lookup_idempotent_result(&scoped, &hash)? {
            return Ok(previous);
        }
        let kind_text = params
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("kind is required".into()))?;
        let kind = parse_kind(kind_text)?;
        if !takes_presets(kind) {
            return Err(ControlError::InvalidRequest(format!(
                "\"{kind_text}\" does not take presets"
            )));
        }
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("name is required".into()))?;
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(ControlError::InvalidRequest(format!(
                "a preset name can be at most {MAX_NAME_CHARS} characters"
            )));
        }
        let parameters = params
            .get("parameters")
            .and_then(Value::as_object)
            .ok_or_else(|| ControlError::InvalidRequest("parameters must be an object".into()))?;
        if serde_json::to_vec(parameters)
            .map_err(|error| ControlError::Json(error.to_string()))?
            .len()
            > MAX_PARAMETERS_BYTES
        {
            return Err(ControlError::InvalidRequest(
                "these settings are too large for a preset".into(),
            ));
        }
        // The same validator a saved route goes through, on a one-tool route.
        let (_, ports, _) = node_template(kind).ok_or_else(|| {
            ControlError::InvalidRequest(format!("\"{kind_text}\" does not take presets"))
        })?;
        let mut merged = default_parameters(kind);
        merged.extend(parameters.clone());
        let probe = Session {
            id: EntityId::new("preset-check"),
            name: "Preset check".into(),
            schema_version: 1,
            revision: 0,
            nodes: vec![Node {
                id: EntityId::new("tool"),
                kind,
                type_version: 1,
                name: "Tool".into(),
                enabled: true,
                bypass: false,
                parameters: merged,
                ports,
            }],
            edges: Vec::new(),
        };
        audiorouter_domain::validate_session(&probe).map_err(|errors| {
            ControlError::InvalidRequest(format!(
                "these settings are not valid for this tool: {}",
                errors.first().map_or_else(String::new, ToString::to_string)
            ))
        })?;
        let mut presets = self.load_tool_presets()?;
        // Saving a name again replaces that preset, as a person expects from "Save as".
        presets.retain(|preset| {
            !(preset["kind"] == kind_text
                && preset["name"]
                    .as_str()
                    .is_some_and(|known| known.eq_ignore_ascii_case(name)))
        });
        if presets.len() >= MAX_PRESETS {
            return Err(ControlError::InvalidRequest(format!(
                "you can keep {MAX_PRESETS} presets; delete one first"
            )));
        }
        let id = format!(
            "preset-{}",
            &audiorouter_storage::lower_hex(&Sha256::digest(
                format!("{kind_text}:{}", name.to_lowercase()).as_bytes()
            ))[..16]
        );
        let preset = json!({ "id": id, "kind": kind_text, "name": name, "parameters": parameters });
        presets.push(preset.clone());
        self.store_tool_presets(presets)?;
        let result = json!({ "preset": preset });
        self.journal_idempotent_result(&scoped, "toolPresets.save", &hash, &result)?;
        Ok(result)
    }

    pub(crate) fn dispatch_tool_presets_delete(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        required_key(&params)?;
        let id = params
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("id is required".into()))?;
        let mut presets = self.load_tool_presets()?;
        let before = presets.len();
        presets.retain(|preset| preset["id"] != id);
        let deleted = presets.len() != before;
        if deleted {
            self.store_tool_presets(presets)?;
        }
        Ok(json!({ "deleted": deleted }))
    }
}

#[cfg(test)]
#[path = "tool_presets_tests.rs"]
mod tests;
