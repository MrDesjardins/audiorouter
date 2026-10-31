//! Export public discovery metadata without storage, listeners or audio devices.
fn main() {
    let document = audiorouter_control::ControlPlane::new("tool-guide-catalog").describe();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "nodeTypes": document["nodeTypes"],
            "voicePresets": document["presets"]["voiceChanger"]
        }))
        .unwrap()
    );
}
