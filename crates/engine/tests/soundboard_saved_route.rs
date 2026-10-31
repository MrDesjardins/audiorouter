use audiorouter_domain::{EntityId, Session};
use audiorouter_engine::*;
use std::{collections::HashMap, sync::Arc};

#[test]
#[ignore = "requires an explicit private session copy; opens no devices"]
fn saved_soundboard_reaches_cable() {
    let session: Session = serde_json::from_slice(
        &std::fs::read(std::env::var("AUDIOROUTER_COMPILE_SESSION_JSON").unwrap()).unwrap(),
    )
    .unwrap();
    let mut media = HashMap::new();
    for id in ["burp-01", "fart-02", "applause-crowd-03"] {
        let bytes = std::fs::read(format!("../../resources/sound-library/{id}.flac")).unwrap();
        media.insert(
            format!("lib:{id}"),
            Arc::new(decode_audio_bytes(bytes, "flac", 48_000).unwrap()),
        );
    }
    let set = compile_native_paths_with_plugins_and_audio(
        &session,
        RuntimeGeneration::new(1),
        &HashMap::new(),
        &media,
    )
    .unwrap();
    let graph = set
        .paths()
        .iter()
        .find(|p| p.input_node_ids().contains(&EntityId::new("soundboard-1")))
        .unwrap();
    let board = graph
        .soundboard_for_node(&EntityId::new("soundboard-1"))
        .unwrap();
    assert!(board.trigger(0));
    let inputs = graph
        .input_node_ids()
        .iter()
        .map(|id| {
            let node = session.nodes.iter().find(|n| &n.id == id).unwrap();
            AudioBlock::new(node.ports[0].channels as usize, 128).unwrap()
        })
        .collect::<Vec<_>>();
    let mut scratch = AudioBlock::new(graph.mixer_channels(), 128).unwrap();
    let mut outputs = graph
        .output_node_ids()
        .iter()
        .map(|_| AudioBlock::new(2, 128).unwrap())
        .collect::<Vec<_>>();
    let cable = graph
        .output_node_ids()
        .iter()
        .position(|id| id.as_str() == "voice-to-cable-a")
        .unwrap();
    let mut peak = 0.0f32;
    for _ in 0..100 {
        graph
            .process(
                &inputs,
                &mut scratch,
                &mut outputs.iter_mut().collect::<Vec<_>>(),
            )
            .unwrap();
        for sample in outputs[cable].channel(0).unwrap() {
            peak = peak.max(sample.abs());
        }
    }
    assert!(peak > 0.01, "Cable-A peak was {peak}");
    let meter = graph
        .meter_snapshot_for_node(&EntityId::new("soundboard-1"))
        .unwrap();
    assert!(
        meter.peak_abs > 0.01,
        "Soundboard must expose its rendered audio to the canvas"
    );
    assert_eq!(meter.observed_frames, 100 * 128);
}
