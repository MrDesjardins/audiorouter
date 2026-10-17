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

/// The published guide page. Every build, repository builds included, opens
/// the public site so a help link never lands on a local file.
pub fn destination(kind: &str) -> Option<String> {
    Some(format!(
        "https://audiorouter.org/tools/{}.html",
        slug(kind)?
    ))
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
    fn every_build_uses_the_canonical_site() {
        assert_eq!(
            destination("physicalInput"),
            Some("https://audiorouter.org/tools/physical-input.html".into())
        );
        assert_eq!(destination("../gain"), None);
    }
}
