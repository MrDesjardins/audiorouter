//! Portable JSON-RPC framing and message contracts for the local API.

use serde::{Deserialize, Serialize};

pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_METHOD_NAME_BYTES: usize = 128;
pub const MAX_REQUEST_ID_BYTES: usize = 128;

/// Version and size limits for the future native audio bridge. These values
/// are deliberately independent of the JSON-RPC frame limits: audio transport
/// must reject malformed headers before a driver or shared-memory view trusts
/// any caller-provided length.
pub const AUDIO_BRIDGE_PROTOCOL_MAJOR: u16 = 1;
pub const AUDIO_BRIDGE_PROTOCOL_MINOR: u16 = 0;
pub const MAX_AUDIO_BRIDGE_BUS_ID_BYTES: usize = 128;
pub const MAX_AUDIO_BRIDGE_CHANNELS: u16 = 2;
pub const MAX_AUDIO_BRIDGE_FRAMES: u16 = 4_096;
pub const MAX_AUDIO_BRIDGE_LEASE_MS: u32 = 60_000;
pub const MAX_AUDIO_BRIDGE_PAYLOAD_BYTES: usize =
    MAX_AUDIO_BRIDGE_CHANNELS as usize * MAX_AUDIO_BRIDGE_FRAMES as usize * 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AudioBridgeContractError {
    UnsupportedMajor(u16),
    EmptyBusId,
    BusIdTooLong,
    ZeroGeneration,
    InvalidSampleRate,
    InvalidChannels,
    InvalidFrames,
    InvalidLease,
    InvalidPayloadLength,
    PayloadTooLarge,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AudioBridgeHello {
    pub protocol_major: u16,
    pub protocol_minor: u16,
    pub bus_id: String,
    pub generation: u64,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub frames_per_quantum: u16,
    pub lease_ms: u32,
}

impl AudioBridgeHello {
    pub fn validate(&self) -> Result<(), AudioBridgeContractError> {
        if self.protocol_major != AUDIO_BRIDGE_PROTOCOL_MAJOR {
            return Err(AudioBridgeContractError::UnsupportedMajor(
                self.protocol_major,
            ));
        }
        if self.bus_id.is_empty() {
            return Err(AudioBridgeContractError::EmptyBusId);
        }
        if self.bus_id.len() > MAX_AUDIO_BRIDGE_BUS_ID_BYTES {
            return Err(AudioBridgeContractError::BusIdTooLong);
        }
        if self.generation == 0 {
            return Err(AudioBridgeContractError::ZeroGeneration);
        }
        if !(8_000..=192_000).contains(&self.sample_rate_hz) {
            return Err(AudioBridgeContractError::InvalidSampleRate);
        }
        if !(1..=MAX_AUDIO_BRIDGE_CHANNELS).contains(&self.channels) {
            return Err(AudioBridgeContractError::InvalidChannels);
        }
        if !(1..=MAX_AUDIO_BRIDGE_FRAMES).contains(&self.frames_per_quantum) {
            return Err(AudioBridgeContractError::InvalidFrames);
        }
        if !(1..=MAX_AUDIO_BRIDGE_LEASE_MS).contains(&self.lease_ms) {
            return Err(AudioBridgeContractError::InvalidLease);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AudioBridgeBlockHeader {
    pub generation: u64,
    pub sequence: u64,
    pub frames: u16,
    pub channels: u16,
    pub payload_bytes: u32,
}

impl AudioBridgeBlockHeader {
    pub fn validate(&self) -> Result<(), AudioBridgeContractError> {
        if self.generation == 0 {
            return Err(AudioBridgeContractError::ZeroGeneration);
        }
        if !(1..=MAX_AUDIO_BRIDGE_CHANNELS).contains(&self.channels) {
            return Err(AudioBridgeContractError::InvalidChannels);
        }
        if self.frames == 0 || self.frames > MAX_AUDIO_BRIDGE_FRAMES {
            return Err(AudioBridgeContractError::InvalidFrames);
        }
        let expected = usize::from(self.frames)
            .saturating_mul(usize::from(self.channels))
            .saturating_mul(std::mem::size_of::<f32>());
        if expected > MAX_AUDIO_BRIDGE_PAYLOAD_BYTES {
            return Err(AudioBridgeContractError::PayloadTooLarge);
        }
        if usize::try_from(self.payload_bytes).ok() != Some(expected) {
            return Err(AudioBridgeContractError::InvalidPayloadLength);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameError {
    TooShort,
    TooLarge { length: usize, maximum: usize },
    LengthMismatch { declared: usize, actual: usize },
    Json(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageError {
    InvalidJson(String),
    InvalidRequest,
    EmptyBatch,
    BatchTooLarge { length: usize, maximum: usize },
}

pub const MAX_BATCH_REQUESTS: usize = 32;

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub fn encode_frame(value: &impl Serialize) -> Result<Vec<u8>, FrameError> {
    let payload = serde_json::to_vec(value).map_err(|error| FrameError::Json(error.to_string()))?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge {
            length: payload.len(),
            maximum: MAX_FRAME_BYTES,
        });
    }
    let length = u32::try_from(payload.len()).map_err(|_| FrameError::TooLarge {
        length: payload.len(),
        maximum: MAX_FRAME_BYTES,
    })?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_frame<T: for<'de> Deserialize<'de>>(frame: &[u8]) -> Result<T, FrameError> {
    if frame.len() < 4 {
        return Err(FrameError::TooShort);
    }
    let declared = u32::from_le_bytes(frame[..4].try_into().unwrap()) as usize;
    if declared > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge {
            length: declared,
            maximum: MAX_FRAME_BYTES,
        });
    }
    let actual = frame.len() - 4;
    if declared != actual {
        return Err(FrameError::LengthMismatch { declared, actual });
    }
    serde_json::from_slice(&frame[4..]).map_err(|error| FrameError::Json(error.to_string()))
}

pub fn decode_rpc_frame(frame: &[u8]) -> Result<RpcMessage, FrameError> {
    if frame.len() < 4 {
        return Err(FrameError::TooShort);
    }
    let declared = u32::from_le_bytes(frame[..4].try_into().unwrap()) as usize;
    if declared > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge {
            length: declared,
            maximum: MAX_FRAME_BYTES,
        });
    }
    let actual = frame.len() - 4;
    if declared != actual {
        return Err(FrameError::LengthMismatch { declared, actual });
    }
    parse_rpc_message(&frame[4..]).map_err(|error| FrameError::Json(format!("{error:?}")))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    pub fn validate(&self) -> Result<(), MessageError> {
        let id_is_bounded = self
            .id
            .as_ref()
            .map(|id| {
                matches!(
                    id,
                    serde_json::Value::String(_)
                        | serde_json::Value::Number(_)
                        | serde_json::Value::Null
                ) && serde_json::to_vec(id)
                    .is_ok_and(|encoded| encoded.len() <= MAX_REQUEST_ID_BYTES)
            })
            .unwrap_or(true);
        if self.jsonrpc != "2.0"
            || self.method.is_empty()
            || self.method.len() > MAX_METHOD_NAME_BYTES
            || !id_is_bounded
        {
            Err(MessageError::InvalidRequest)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RpcMessage {
    Single(JsonRpcRequest),
    Batch(Vec<JsonRpcRequest>),
}

pub fn parse_rpc_message(payload: &[u8]) -> Result<RpcMessage, MessageError> {
    let value: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|error| MessageError::InvalidJson(error.to_string()))?;
    if value.is_array() {
        let values = value.as_array().unwrap();
        if values.is_empty() {
            return Err(MessageError::EmptyBatch);
        }
        if values.len() > MAX_BATCH_REQUESTS {
            return Err(MessageError::BatchTooLarge {
                length: values.len(),
                maximum: MAX_BATCH_REQUESTS,
            });
        }
        let mut requests = Vec::with_capacity(values.len());
        for value in values {
            let request: JsonRpcRequest =
                serde_json::from_value(value.clone()).map_err(|_| MessageError::InvalidRequest)?;
            request.validate()?;
            requests.push(request);
        }
        Ok(RpcMessage::Batch(requests))
    } else {
        let request: JsonRpcRequest =
            serde_json::from_value(value).map_err(|_| MessageError::InvalidRequest)?;
        request.validate()?;
        Ok(RpcMessage::Single(request))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(id: Option<serde_json::Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn frame_round_trip_is_little_endian_and_json_safe() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(7)),
            method: "system.describe".into(),
            params: None,
        };
        let frame = encode_frame(&request).unwrap();
        assert_eq!(
            u32::from_le_bytes(frame[..4].try_into().unwrap()) as usize,
            frame.len() - 4
        );
        assert_eq!(decode_frame::<JsonRpcRequest>(&frame).unwrap(), request);
    }

    #[test]
    fn rejects_truncated_oversized_and_mismatched_frames() {
        assert_eq!(decode_frame::<Value>(&[]), Err(FrameError::TooShort));
        assert_eq!(
            decode_frame::<Value>(&[5, 0, 0, 0, b'{', b'}']),
            Err(FrameError::LengthMismatch {
                declared: 5,
                actual: 2
            })
        );
        let mut oversized = Vec::new();
        oversized.extend_from_slice(&((MAX_FRAME_BYTES as u32) + 1).to_le_bytes());
        assert!(matches!(
            decode_frame::<Value>(&oversized),
            Err(FrameError::TooLarge { .. })
        ));
    }

    #[test]
    fn response_helpers_preserve_json_rpc_shape() {
        let success = JsonRpcResponse::success(Some(json!(1)), json!({ "ok": true }));
        assert_eq!(success.jsonrpc, "2.0");
        assert!(success.error.is_none());
        let failure = JsonRpcResponse::failure(None, -32600, "invalid request");
        assert_eq!(failure.error.unwrap().code, -32600);
    }

    #[test]
    fn parses_batches_with_the_protocol_limit() {
        let request = json!({ "jsonrpc": "2.0", "id": 1, "method": "status.get" });
        let batch = serde_json::to_vec(&vec![request.clone(), request]).unwrap();
        assert!(
            matches!(parse_rpc_message(&batch), Ok(RpcMessage::Batch(requests)) if requests.len() == 2)
        );
        assert_eq!(parse_rpc_message(b"[]"), Err(MessageError::EmptyBatch));
        let too_many = vec![
            serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "status.get" });
            MAX_BATCH_REQUESTS + 1
        ];
        let payload = serde_json::to_vec(&too_many).unwrap();
        assert_eq!(
            parse_rpc_message(&payload),
            Err(MessageError::BatchTooLarge {
                length: 33,
                maximum: 32
            })
        );
    }

    #[test]
    fn distinguishes_notifications_and_rejects_invalid_requests() {
        let notification =
            parse_rpc_message(br#"{"jsonrpc":"2.0","method":"status.get"}"#).unwrap();
        assert!(matches!(notification, RpcMessage::Single(request) if request.is_notification()));
        assert_eq!(
            parse_rpc_message(br#"{"jsonrpc":"1.0","id":1,"method":"status.get"}"#),
            Err(MessageError::InvalidRequest)
        );
        assert_eq!(
            parse_rpc_message(br#"{"jsonrpc":"2.0","id":1,"method":""}"#),
            Err(MessageError::InvalidRequest)
        );
        assert_eq!(
            parse_rpc_message(
                &serde_json::to_vec(&json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "x".repeat(MAX_METHOD_NAME_BYTES + 1)
                }))
                .unwrap()
            ),
            Err(MessageError::InvalidRequest)
        );
        for id in [json!({ "nested": 1 }), json!([1]), json!(true)] {
            assert_eq!(
                parse_rpc_message(
                    &serde_json::to_vec(&json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "method": "status.get"
                    }))
                    .unwrap()
                ),
                Err(MessageError::InvalidRequest)
            );
        }
        assert_eq!(
            parse_rpc_message(
                &serde_json::to_vec(&json!({
                    "jsonrpc": "2.0",
                    "id": { "nested": "x".repeat(MAX_REQUEST_ID_BYTES) },
                    "method": "status.get"
                }))
                .unwrap()
            ),
            Err(MessageError::InvalidRequest)
        );
    }

    #[test]
    fn rejects_hostile_json_rpc_shapes_without_accepting_partial_requests() {
        for payload in [
            b"null".as_slice(),
            b"true".as_slice(),
            b"\"request\"".as_slice(),
            br#"{}"#.as_slice(),
            br#"{"jsonrpc":"2.0"}"#.as_slice(),
            br#"{"jsonrpc":2,"id":1,"method":"status.get"}"#.as_slice(),
            br#"{"jsonrpc":"2.0","id":1,"method":42}"#.as_slice(),
            br#"[null]"#.as_slice(),
            br#"[{"jsonrpc":"2.0","id":1,"method":"status.get"},null]"#.as_slice(),
        ] {
            assert_eq!(
                parse_rpc_message(payload),
                Err(MessageError::InvalidRequest),
                "payload should be rejected: {}",
                String::from_utf8_lossy(payload)
            );
        }
    }

    #[test]
    fn audio_bridge_contract_bounds_identity_generation_and_format() {
        let hello = AudioBridgeHello {
            protocol_major: AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: "bus-1".into(),
            generation: 7,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 128,
            lease_ms: 1_000,
        };
        assert_eq!(hello.validate(), Ok(()));
        let mut invalid = hello.clone();
        invalid.generation = 0;
        assert_eq!(
            invalid.validate(),
            Err(AudioBridgeContractError::ZeroGeneration)
        );
        invalid = hello.clone();
        invalid.protocol_major += 1;
        assert_eq!(
            invalid.validate(),
            Err(AudioBridgeContractError::UnsupportedMajor(
                AUDIO_BRIDGE_PROTOCOL_MAJOR + 1
            ))
        );
        invalid = hello;
        invalid.bus_id = "x".repeat(MAX_AUDIO_BRIDGE_BUS_ID_BYTES + 1);
        assert_eq!(
            invalid.validate(),
            Err(AudioBridgeContractError::BusIdTooLong)
        );
    }

    #[test]
    fn audio_bridge_block_header_rejects_length_and_generation_mismatch() {
        let header = AudioBridgeBlockHeader {
            generation: 3,
            sequence: 4,
            frames: 128,
            channels: 2,
            payload_bytes: 128 * 2 * 4,
        };
        assert_eq!(header.validate(), Ok(()));
        let mut invalid = header;
        invalid.payload_bytes -= 1;
        assert_eq!(
            invalid.validate(),
            Err(AudioBridgeContractError::InvalidPayloadLength)
        );
        invalid = header;
        invalid.generation = 0;
        assert_eq!(
            invalid.validate(),
            Err(AudioBridgeContractError::ZeroGeneration)
        );
        invalid = header;
        invalid.channels = MAX_AUDIO_BRIDGE_CHANNELS + 1;
        assert_eq!(
            invalid.validate(),
            Err(AudioBridgeContractError::InvalidChannels)
        );
    }
}
