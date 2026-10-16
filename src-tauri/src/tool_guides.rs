//! Allowlisted documentation destinations; no caller-supplied URL or path.
pub fn slug(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "gain" => "gain",
        "volume" => "volume",
        "bassTreble" => "bass-treble",
        "graphicEq" => "graphic-eq",
        "parametricEq" => "parametric-eq",
        "compressor" => "compressor",
        "gate" => "gate",
        "limiter" => "limiter",
        "delay" => "delay",
        "pitch" => "pitch",
        "dehum" => "dehum",
        "declick" => "declick",
        "denoise" => "denoise",
        "speechDenoise" => "speech-denoise",
        "spectralGate" => "spectral-gate",
        "firFilter" => "fir-filter",
        "mute" => "mute",
        "inputSwitch" => "input-switch",
        "duck" => "duck",
        "timeShift" => "time-shift",
        "mixer" => "mixer",
        "meter" => "meter",
        "physicalInput" => "physical-input",
        "physicalOutput" => "physical-output",
        "applicationCapture" => "application-capture",
        "endpointLoopback" => "endpoint-loopback",
        "virtualRenderSource" => "virtual-render-source",
        "virtualCaptureSink" => "virtual-capture-sink",
        "testSignal" => "test-signal",
        "audioFile" => "audio-file",
        "recorder" => "recorder",
        "networkSend" => "network-send",
        "networkReceive" => "network-receive",
        "plugin" => "plugin",
        "group" => "group",
        _ => return None,
    })
}

pub fn destination(kind: &str, executable: &std::path::Path) -> Option<String> {
    let slug = slug(kind)?;
    // Repository builds open their generated local guide so unpublished edits
    // are reviewable. Installed builds use the same guide on the public site.
    let root = executable.ancestors().nth(4);
    if let Some(root) = root.filter(|root| root.join("src-tauri/Cargo.toml").is_file()) {
        let page = root.join("site/tools").join(format!("{slug}.html"));
        if page.is_file() {
            return Some(page.to_string_lossy().into_owned());
        }
    }
    Some(format!("https://audiorouter.org/tools/{slug}.html"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_known_tool_kinds_can_open_guides() {
        assert_eq!(slug("parametricEq"), Some("parametric-eq"));
        assert_eq!(slug("virtualCaptureSink"), Some("virtual-capture-sink"));
        assert_eq!(slug("group"), Some("group"));
        for invalid in ["", "../gain", "https://example.com", "gain.html", "Gain"] {
            assert_eq!(slug(invalid), None);
        }
    }
    #[test]
    fn installed_shell_uses_the_canonical_site() {
        assert_eq!(
            destination(
                "gain",
                std::path::Path::new("C:/Program Files/AudioRouter/audiorouter-shell.exe")
            ),
            Some("https://audiorouter.org/tools/gain.html".into())
        );
    }
}
