//! Control-plane metadata returned by isolated plugin discovery.
//!
//! Native indices are identities, not positions in the filtered list. Only
//! explicitly declared auxiliary inputs may become graph sidechain ports.

use crate::{WorkerAudioBusLayout, MAX_WORKER_AUDIO_BUSES};
use serde::{Deserialize, Serialize};

pub const MAX_BUS_NAME_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioBusRole {
    Main,
    Auxiliary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveredAudioBus {
    pub native_index: u16,
    pub name: String,
    pub channels: u16,
    pub role: AudioBusRole,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveredAudioBuses {
    pub inputs: Vec<DiscoveredAudioBus>,
    pub outputs: Vec<DiscoveredAudioBus>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BusDiscoveryError {
    InvalidBusCount,
    InvalidName,
    InvalidChannels,
    DuplicateNativeIndex,
    InvalidMainBus,
    UnsupportedOutputBus,
    UnsupportedLayout,
}

/// A non-serializable capability obtained from a live, identity-checked worker.
/// Saved ports and imported JSON cannot manufacture this binding. Revalidate
/// it for each worker replacement; it is not a reusable authorization grant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedSidechainBinding {
    pub(crate) fingerprint: String,
    pub(crate) class_id: Option<String>,
    pub(crate) native_index: u16,
    pub(crate) layout: WorkerAudioBusLayout,
}

impl VerifiedSidechainBinding {
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn class_id(&self) -> Option<&str> {
        self.class_id.as_deref()
    }
    pub fn native_index(&self) -> u16 {
        self.native_index
    }
    pub fn layout(&self) -> &WorkerAudioBusLayout {
        &self.layout
    }
}

impl DiscoveredAudioBuses {
    /// Validate untrusted worker metadata before it is used to create ports.
    /// This describes effect buses; zero-input instruments use their separate
    /// instrument path until multi-bus instrument processing is implemented.
    pub fn processing_layout(&self) -> Result<WorkerAudioBusLayout, BusDiscoveryError> {
        validate_side(&self.inputs, false)?;
        validate_side(&self.outputs, true)?;
        let inputs: Vec<_> = self.inputs.iter().map(|bus| bus.channels).collect();
        let outputs: Vec<_> = self.outputs.iter().map(|bus| bus.channels).collect();
        WorkerAudioBusLayout::new(&inputs, &outputs)
            .map_err(|_| BusDiscoveryError::UnsupportedLayout)
    }
}

fn validate_side(buses: &[DiscoveredAudioBus], output: bool) -> Result<(), BusDiscoveryError> {
    if buses.is_empty() || buses.len() > MAX_WORKER_AUDIO_BUSES {
        return Err(BusDiscoveryError::InvalidBusCount);
    }
    for (position, bus) in buses.iter().enumerate() {
        if bus.name.len() > MAX_BUS_NAME_BYTES || bus.name.chars().any(char::is_control) {
            return Err(BusDiscoveryError::InvalidName);
        }
        if !(1..=2).contains(&bus.channels) {
            return Err(BusDiscoveryError::InvalidChannels);
        }
        if buses[..position]
            .iter()
            .any(|previous| previous.native_index == bus.native_index)
        {
            return Err(BusDiscoveryError::DuplicateNativeIndex);
        }
        if position == 0 {
            if bus.native_index != 0 || bus.role != AudioBusRole::Main {
                return Err(BusDiscoveryError::InvalidMainBus);
            }
        } else if output {
            return Err(BusDiscoveryError::UnsupportedOutputBus);
        } else if bus.role != AudioBusRole::Auxiliary {
            return Err(BusDiscoveryError::InvalidMainBus);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bus(native_index: u16, channels: u16, role: AudioBusRole) -> DiscoveredAudioBus {
        DiscoveredAudioBus {
            native_index,
            channels,
            role,
            name: format!("Bus {native_index}"),
        }
    }

    fn effect() -> DiscoveredAudioBuses {
        DiscoveredAudioBuses {
            inputs: vec![
                bus(0, 2, AudioBusRole::Main),
                bus(2, 1, AudioBusRole::Auxiliary),
            ],
            outputs: vec![bus(0, 2, AudioBusRole::Main)],
        }
    }

    #[test]
    fn explicit_auxiliary_keeps_native_identity_through_serialization() {
        let metadata = effect();
        let decoded: DiscoveredAudioBuses =
            serde_json::from_slice(&serde_json::to_vec(&metadata).unwrap()).unwrap();
        assert_eq!(decoded, metadata);
        assert_eq!(decoded.inputs[1].native_index, 2);
        let layout = decoded.processing_layout().unwrap();
        assert_eq!(layout.input_buses(), &[2, 1]);
        assert_eq!(layout.output_buses(), &[2]);
    }

    #[test]
    fn extra_channels_and_extra_main_buses_never_imply_sidechain() {
        let mut metadata = effect();
        metadata.inputs[1].role = AudioBusRole::Main;
        assert_eq!(
            metadata.processing_layout(),
            Err(BusDiscoveryError::InvalidMainBus)
        );
        metadata.inputs.truncate(1);
        metadata.inputs[0].channels = 4;
        assert_eq!(
            metadata.processing_layout(),
            Err(BusDiscoveryError::InvalidChannels)
        );
    }

    #[test]
    fn duplicate_native_indices_and_auxiliary_outputs_are_refused() {
        let mut metadata = effect();
        metadata.inputs[1].native_index = 0;
        assert_eq!(
            metadata.processing_layout(),
            Err(BusDiscoveryError::DuplicateNativeIndex)
        );
        metadata = effect();
        metadata.outputs.push(bus(1, 2, AudioBusRole::Auxiliary));
        assert_eq!(
            metadata.processing_layout(),
            Err(BusDiscoveryError::UnsupportedOutputBus)
        );
    }

    #[test]
    fn untrusted_metadata_is_bounded_before_layout_construction() {
        let mut metadata = effect();
        metadata.inputs[0].name = "x".repeat(MAX_BUS_NAME_BYTES + 1);
        assert_eq!(
            metadata.processing_layout(),
            Err(BusDiscoveryError::InvalidName)
        );
        metadata.inputs.clear();
        assert_eq!(
            metadata.processing_layout(),
            Err(BusDiscoveryError::InvalidBusCount)
        );
        metadata.inputs = vec![bus(0, 1, AudioBusRole::Main); MAX_WORKER_AUDIO_BUSES + 1];
        assert_eq!(
            metadata.processing_layout(),
            Err(BusDiscoveryError::InvalidBusCount)
        );
    }

    #[test]
    fn ordinary_effect_has_no_auxiliary_bus() {
        let mut metadata = effect();
        metadata.inputs.truncate(1);
        assert!(!metadata.processing_layout().unwrap().has_sidechain());
        metadata.inputs[0].native_index = 1;
        assert_eq!(
            metadata.processing_layout(),
            Err(BusDiscoveryError::InvalidMainBus)
        );
    }

    #[test]
    fn wire_messages_validate_metadata_on_both_boundaries() {
        use crate::{decode_worker_message, encode_worker_message, WorkerMessage};
        let request = WorkerMessage::DescribeBuses;
        assert_eq!(
            decode_worker_message(&encode_worker_message(&request).unwrap()).unwrap(),
            request
        );
        let response = WorkerMessage::Buses(effect());
        assert_eq!(
            decode_worker_message(&encode_worker_message(&response).unwrap()).unwrap(),
            response
        );
        let mut invalid = effect();
        invalid.inputs[1].native_index = 0;
        assert!(encode_worker_message(&WorkerMessage::Buses(invalid.clone())).is_err());
        let json = serde_json::to_vec(&WorkerMessage::Buses(invalid)).unwrap();
        let mut wire = (json.len() as u32).to_le_bytes().to_vec();
        wire.extend(json);
        assert!(decode_worker_message(&wire).is_err());
    }
}
