//! Portable JSON-RPC framing and message contracts for the local API.

use serde::{Deserialize, Serialize};

pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

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
        if self.jsonrpc != "2.0" || self.method.is_empty() {
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
    }
}
