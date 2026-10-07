//! Test helpers shared by the control-plane test modules.

use super::*;
pub(crate) use audiorouter_domain::{Edge, Node, NodeKind, Port, PortDirection};
pub(crate) use audiorouter_engine::{RuntimeGraph, RuntimeProcessor};

pub(crate) fn feedback_fixture() -> Session {
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let node = |id: &str, kind, endpoint: &str| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: if matches!(kind, NodeKind::PhysicalInput | NodeKind::PhysicalOutput) {
            serde_json::from_value(json!({"endpointId": endpoint})).unwrap()
        } else {
            Default::default()
        },
        ports: match kind {
            NodeKind::PhysicalInput => vec![port("out", PortDirection::Output)],
            NodeKind::PhysicalOutput => vec![port("in", PortDirection::Input)],
            _ => vec![
                port("in", PortDirection::Input),
                port("out", PortDirection::Output),
            ],
        },
    };
    Session {
        id: EntityId::new("feedback"),
        name: "feedback".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            node("game", NodeKind::PhysicalInput, "cable-b-output"),
            node("eq", NodeKind::Gain, ""),
            node("mix", NodeKind::Mixer, ""),
            node("recording", NodeKind::PhysicalOutput, "cable-b-input"),
        ],
        edges: [("game", "eq"), ("eq", "mix"), ("mix", "recording")]
            .into_iter()
            .enumerate()
            .map(|(index, (source, destination))| Edge {
                id: EntityId::new(format!("e{index}")),
                source_node: EntityId::new(source),
                source_port: "out".into(),
                destination_node: EntityId::new(destination),
                destination_port: "in".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            })
            .collect(),
    }
}

pub(crate) fn encode_test_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        output.push(TABLE[(a >> 2) as usize] as char);
        output.push(TABLE[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

pub(crate) fn tiny_test_wav() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&38u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8_000u32.to_le_bytes());
    bytes.extend_from_slice(&16_000u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&4096i16.to_le_bytes());
    bytes
}

#[derive(Default)]
pub(crate) struct CountingTap(pub(crate) std::sync::atomic::AtomicUsize);

impl AudioTap for CountingTap {
    fn on_processed_block(&self, _start_frame: u64, _block: &AudioBlock) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// A WAV a player accepts: RIFF size matches the file, and a `data`
/// chunk with audio ends inside the file. Returns the data byte count.
pub(crate) fn assert_playable_wav(path: &std::path::Path) -> u64 {
    let bytes = std::fs::read(path).unwrap();
    assert!(
        bytes.len() > 44 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "{path:?} is not a WAV"
    );
    let riff = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    assert_eq!(riff, bytes.len() - 8, "{path:?}: RIFF size not finalized");
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        if id == b"data" {
            assert!(size > 0, "{path:?}: empty data chunk");
            assert!(
                offset + 8 + size <= bytes.len(),
                "{path:?}: data chunk overruns the file"
            );
            return size as u64;
        }
        offset += 8 + size + (size & 1);
    }
    panic!("{path:?}: no data chunk");
}

pub(crate) struct TestRecorderWorker;

impl RecorderWorker for TestRecorderWorker {
    fn finalize(&mut self, _frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        Ok(RecorderFinalizationOutcome {
            state: "completed".into(),
            file_finalized: true,
            recoverable: false,
        })
    }
}

pub(crate) struct HookRecorderWorker {
    pub(crate) hooks: Arc<std::sync::Mutex<Vec<String>>>,
}

pub(crate) struct FailingTapRecorderWorker {
    pub(crate) tap: Arc<dyn AudioTap>,
}

pub(crate) struct SuccessfulTapRecorderWorker {
    pub(crate) tap: Arc<dyn AudioTap>,
}

impl RecorderWorker for FailingTapRecorderWorker {
    fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
        Some(self.tap.clone())
    }

    fn finalize(&mut self, _frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        Err("synthetic encoder failure".into())
    }
}

impl RecorderWorker for SuccessfulTapRecorderWorker {
    fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
        Some(self.tap.clone())
    }

    fn finalize(&mut self, _frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        Ok(RecorderFinalizationOutcome {
            state: "completed".into(),
            file_finalized: true,
            recoverable: false,
        })
    }
}

impl RecorderWorker for HookRecorderWorker {
    fn arm(&mut self) -> Result<(), String> {
        self.hooks.lock().unwrap().push("arm".into());
        Ok(())
    }

    fn start(&mut self, frame: u64) -> Result<(), String> {
        self.hooks.lock().unwrap().push(format!("start:{frame}"));
        Ok(())
    }

    fn split(&mut self, frame: u64) -> Result<(), String> {
        self.hooks.lock().unwrap().push(format!("split:{frame}"));
        Ok(())
    }

    fn finalize(&mut self, _frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        Ok(RecorderFinalizationOutcome {
            state: "completed".into(),
            file_finalized: true,
            recoverable: false,
        })
    }
}

pub(crate) fn session() -> Session {
    Session {
        id: EntityId::new("session"),
        name: "test".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            Node {
                id: EntityId::new("in"),
                kind: NodeKind::PhysicalInput,
                type_version: 1,
                name: "Input".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![Port {
                    name: "main".into(),
                    direction: PortDirection::Output,
                    channels: 1,
                }],
            },
            Node {
                id: EntityId::new("out"),
                kind: NodeKind::PhysicalOutput,
                type_version: 1,
                name: "Output".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![Port {
                    name: "main".into(),
                    direction: PortDirection::Input,
                    channels: 1,
                }],
            },
        ],
        edges: vec![Edge {
            id: EntityId::new("edge"),
            source_node: EntityId::new("in"),
            source_port: "main".into(),
            destination_node: EntityId::new("out"),
            destination_port: "main".into(),
            matrix: vec![1.0],
            enabled: true,
        }],
    }
}
