//! Fun Voice Changer parameters and presets (DSP-20).
//!
//! This table is the single source for the node's parameter names, ranges,
//! defaults and presets: validation, the API catalog, the engine and the UI
//! (through `describe()`) all read it.

use serde_json::{Map, Value};

/// What a parameter accepts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VoiceParameterKind {
    Number {
        minimum: f64,
        maximum: f64,
        integer: bool,
        unit: &'static str,
    },
    Boolean,
    Choice(&'static [&'static str]),
}

/// A typed parameter value from the static tables.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VoiceParameterValue {
    Number(f64),
    Boolean(bool),
    Text(&'static str),
}

impl VoiceParameterValue {
    pub fn to_json(self) -> Value {
        match self {
            Self::Number(number) => {
                if number.fract() == 0.0 && number.abs() < 1.0e15 {
                    Value::from(number as i64)
                } else {
                    Value::from(number)
                }
            }
            Self::Boolean(flag) => Value::Bool(flag),
            Self::Text(text) => Value::String(text.to_owned()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoiceParameterSpec {
    pub name: &'static str,
    pub kind: VoiceParameterKind,
    /// Neutral value: with every parameter at its default, the node passes
    /// its input through unchanged (apart from the fixed delay).
    pub default: VoiceParameterValue,
}

pub const VOICE_KEYS: &[&str] = &[
    "c", "cs", "d", "ds", "e", "f", "fs", "g", "gs", "a", "as", "b",
];
pub const VOICE_SCALES: &[&str] = &[
    "chromatic",
    "major",
    "minor",
    "majorPentatonic",
    "minorPentatonic",
    "blues",
    "monotone",
];
pub const VOICE_MODULATION_TYPES: &[&str] = &["chorus", "flanger"];

const fn number(
    name: &'static str,
    minimum: f64,
    maximum: f64,
    unit: &'static str,
    default: f64,
) -> VoiceParameterSpec {
    VoiceParameterSpec {
        name,
        kind: VoiceParameterKind::Number {
            minimum,
            maximum,
            integer: false,
            unit,
        },
        default: VoiceParameterValue::Number(default),
    }
}

const fn integer(
    name: &'static str,
    minimum: f64,
    maximum: f64,
    default: f64,
) -> VoiceParameterSpec {
    VoiceParameterSpec {
        name,
        kind: VoiceParameterKind::Number {
            minimum,
            maximum,
            integer: true,
            unit: "",
        },
        default: VoiceParameterValue::Number(default),
    }
}

const fn boolean(name: &'static str) -> VoiceParameterSpec {
    VoiceParameterSpec {
        name,
        kind: VoiceParameterKind::Boolean,
        default: VoiceParameterValue::Boolean(false),
    }
}

const fn choice(
    name: &'static str,
    choices: &'static [&'static str],
    default: &'static str,
) -> VoiceParameterSpec {
    VoiceParameterSpec {
        name,
        kind: VoiceParameterKind::Choice(choices),
        default: VoiceParameterValue::Text(default),
    }
}

/// Every Fun Voice Changer parameter except `preset`, in signal-chain order.
pub const VOICE_CHANGER_PARAMETERS: [VoiceParameterSpec; 35] = [
    number("gateDb", -90.0, -20.0, "dBFS", -90.0),
    boolean("correctEnabled"),
    choice("key", VOICE_KEYS, "c"),
    choice("scale", VOICE_SCALES, "chromatic"),
    number("retuneMs", 0.0, 400.0, "ms", 50.0),
    number("humanizePercent", 0.0, 100.0, "%", 0.0),
    boolean("shiftEnabled"),
    number("pitchSemitones", -12.0, 12.0, "st", 0.0),
    number("pitchCents", -100.0, 100.0, "cents", 0.0),
    number("formantSemitones", -12.0, 12.0, "st", 0.0),
    boolean("ringEnabled"),
    number("ringHz", 20.0, 2_000.0, "Hz", 50.0),
    number("ringDepthPercent", 0.0, 100.0, "%", 100.0),
    boolean("crushEnabled"),
    integer("crushBits", 4.0, 16.0, 16.0),
    integer("crushDownsample", 1.0, 12.0, 1.0),
    boolean("driveEnabled"),
    number("drivePercent", 0.0, 100.0, "%", 0.0),
    number("whisperPercent", 0.0, 100.0, "%", 0.0),
    boolean("toneEnabled"),
    number("lowCutHz", 20.0, 2_000.0, "Hz", 20.0),
    number("highCutHz", 1_000.0, 20_000.0, "Hz", 20_000.0),
    number("peakHz", 200.0, 8_000.0, "Hz", 1_000.0),
    number("peakDb", -12.0, 12.0, "dB", 0.0),
    boolean("modEnabled"),
    choice("modType", VOICE_MODULATION_TYPES, "chorus"),
    number("modRateHz", 0.05, 5.0, "Hz", 0.5),
    number("modDepthPercent", 0.0, 100.0, "%", 50.0),
    number("modFeedbackPercent", -90.0, 90.0, "%", 0.0),
    boolean("spaceEnabled"),
    number("spaceSizePercent", 0.0, 100.0, "%", 50.0),
    number("spaceDecayMs", 200.0, 4_000.0, "ms", 1_000.0),
    number("spaceLevelPercent", 0.0, 100.0, "%", 30.0),
    number("mixPercent", 0.0, 100.0, "%", 100.0),
    number("outputDb", -24.0, 12.0, "dB", 0.0),
];

/// One preset: a starting point that writes every parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoicePreset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// Bumped when the recipe changes; saved nodes keep their own values.
    pub version: u32,
    /// Parameters shown in the inspector's basic view, in order.
    pub basic: &'static [&'static str],
    /// Values that differ from neutral.
    pub values: &'static [(&'static str, VoiceParameterValue)],
}

use VoiceParameterValue::{Boolean as B, Number as N, Text as T};

pub const VOICE_PRESETS: [VoicePreset; 14] = [
    VoicePreset {
        id: "pitchCorrectSubtle",
        name: "Pitch Correct (subtle)",
        description: "Gently pulls sung notes to the scale; keeps natural movement.",
        version: 1,
        basic: &["key", "scale", "retuneMs", "humanizePercent"],
        values: &[
            ("correctEnabled", B(true)),
            ("retuneMs", N(80.0)),
            ("humanizePercent", N(40.0)),
            ("gateDb", N(-55.0)),
        ],
    },
    VoicePreset {
        id: "pitchCorrectHard",
        name: "Pitch Correct (hard)",
        description: "Snaps every note instantly: the robotic tuned-vocal sound.",
        version: 1,
        basic: &["key", "scale", "retuneMs", "mixPercent"],
        values: &[
            ("correctEnabled", B(true)),
            ("retuneMs", N(0.0)),
            ("gateDb", N(-55.0)),
        ],
    },
    VoicePreset {
        id: "deep",
        name: "Deep",
        description: "A lower, bigger voice.",
        version: 1,
        basic: &["pitchSemitones", "formantSemitones", "mixPercent"],
        values: &[
            ("shiftEnabled", B(true)),
            ("pitchSemitones", N(-5.0)),
            ("formantSemitones", N(-3.0)),
        ],
    },
    VoicePreset {
        id: "chipmunk",
        name: "Chipmunk",
        description: "High and squeaky.",
        version: 1,
        basic: &["pitchSemitones", "formantSemitones", "mixPercent"],
        values: &[
            ("shiftEnabled", B(true)),
            ("pitchSemitones", N(7.0)),
            ("formantSemitones", N(5.0)),
        ],
    },
    VoicePreset {
        id: "monster",
        name: "Monster",
        description: "Low, growling and roomy.",
        version: 1,
        basic: &[
            "pitchSemitones",
            "formantSemitones",
            "drivePercent",
            "spaceLevelPercent",
        ],
        values: &[
            ("shiftEnabled", B(true)),
            ("pitchSemitones", N(-8.0)),
            ("formantSemitones", N(-5.0)),
            ("driveEnabled", B(true)),
            ("drivePercent", N(40.0)),
            ("spaceEnabled", B(true)),
            ("spaceSizePercent", N(30.0)),
            ("spaceDecayMs", N(800.0)),
            ("spaceLevelPercent", N(20.0)),
        ],
    },
    VoicePreset {
        id: "child",
        name: "Child",
        description: "A smaller, younger-sounding voice.",
        version: 1,
        basic: &["pitchSemitones", "formantSemitones"],
        values: &[
            ("shiftEnabled", B(true)),
            ("pitchSemitones", N(4.0)),
            ("formantSemitones", N(3.0)),
            ("toneEnabled", B(true)),
            ("lowCutHz", N(150.0)),
        ],
    },
    VoicePreset {
        id: "robot",
        name: "Robot",
        description: "Flat, buzzing machine voice.",
        version: 1,
        basic: &["ringHz", "ringDepthPercent", "crushBits"],
        values: &[
            ("correctEnabled", B(true)),
            ("scale", T("monotone")),
            ("retuneMs", N(0.0)),
            ("ringEnabled", B(true)),
            ("ringHz", N(50.0)),
            ("ringDepthPercent", N(70.0)),
            ("crushEnabled", B(true)),
            ("crushBits", N(10.0)),
            ("modEnabled", B(true)),
            ("modType", T("flanger")),
            ("modRateHz", N(0.2)),
            ("modDepthPercent", N(40.0)),
            ("modFeedbackPercent", N(50.0)),
            ("outputDb", N(7.0)),
        ],
    },
    VoicePreset {
        id: "alien",
        name: "Alien",
        description: "Strange, shimmering and a little higher.",
        version: 1,
        basic: &["pitchSemitones", "ringHz", "modDepthPercent"],
        values: &[
            ("shiftEnabled", B(true)),
            ("pitchSemitones", N(3.0)),
            ("ringEnabled", B(true)),
            ("ringHz", N(300.0)),
            ("ringDepthPercent", N(40.0)),
            ("modEnabled", B(true)),
            ("modRateHz", N(1.5)),
            ("modDepthPercent", N(60.0)),
            ("outputDb", N(5.0)),
        ],
    },
    VoicePreset {
        id: "radio",
        name: "Radio",
        description: "Thin and slightly gritty, like an old radio.",
        version: 1,
        basic: &["lowCutHz", "highCutHz", "drivePercent"],
        values: &[
            ("toneEnabled", B(true)),
            ("lowCutHz", N(300.0)),
            ("highCutHz", N(3_000.0)),
            ("peakHz", N(1_500.0)),
            ("peakDb", N(4.0)),
            ("driveEnabled", B(true)),
            ("drivePercent", N(30.0)),
            ("outputDb", N(-3.0)),
        ],
    },
    VoicePreset {
        id: "megaphone",
        name: "Megaphone",
        description: "Loud, honky and distorted.",
        version: 1,
        basic: &["drivePercent", "peakDb"],
        values: &[
            ("toneEnabled", B(true)),
            ("lowCutHz", N(500.0)),
            ("highCutHz", N(4_000.0)),
            ("peakHz", N(2_000.0)),
            ("peakDb", N(6.0)),
            ("driveEnabled", B(true)),
            ("drivePercent", N(70.0)),
            ("outputDb", N(-3.0)),
        ],
    },
    VoicePreset {
        id: "telephone",
        name: "Telephone",
        description: "Narrow, low-fidelity phone line.",
        version: 1,
        basic: &["highCutHz", "crushBits"],
        values: &[
            ("toneEnabled", B(true)),
            ("lowCutHz", N(300.0)),
            ("highCutHz", N(3_400.0)),
            ("crushEnabled", B(true)),
            ("crushBits", N(12.0)),
            ("crushDownsample", N(6.0)),
        ],
    },
    VoicePreset {
        id: "whisper",
        name: "Whisper",
        description: "Breathy whisper with no voiced tone.",
        version: 1,
        basic: &["whisperPercent", "highCutHz"],
        values: &[
            ("whisperPercent", N(100.0)),
            ("toneEnabled", B(true)),
            ("lowCutHz", N(200.0)),
            ("highCutHz", N(8_000.0)),
            ("gateDb", N(-50.0)),
            ("outputDb", N(8.0)),
        ],
    },
    VoicePreset {
        id: "cave",
        name: "Cave",
        description: "Big, echoing space.",
        version: 1,
        basic: &["spaceSizePercent", "spaceDecayMs", "spaceLevelPercent"],
        values: &[
            ("spaceEnabled", B(true)),
            ("spaceSizePercent", N(90.0)),
            ("spaceDecayMs", N(2_500.0)),
            ("spaceLevelPercent", N(50.0)),
        ],
    },
    VoicePreset {
        id: "custom",
        name: "Custom",
        description: "Everything off: build your own voice.",
        version: 1,
        basic: &[],
        values: &[],
    },
];

pub fn voice_parameter(name: &str) -> Option<&'static VoiceParameterSpec> {
    VOICE_CHANGER_PARAMETERS
        .iter()
        .find(|spec| spec.name == name)
}

pub fn voice_preset(id: &str) -> Option<&'static VoicePreset> {
    VOICE_PRESETS.iter().find(|preset| preset.id == id)
}

/// Whether `value` is valid for the named Fun Voice Changer parameter.
pub fn valid_voice_parameter(name: &str, value: &Value) -> bool {
    if name == "preset" {
        return value.as_str().is_some_and(|id| voice_preset(id).is_some());
    }
    let Some(spec) = voice_parameter(name) else {
        return false;
    };
    match spec.kind {
        VoiceParameterKind::Number {
            minimum,
            maximum,
            integer,
            ..
        } => value.as_f64().is_some_and(|number| {
            number.is_finite()
                && (minimum..=maximum).contains(&number)
                && (!integer || number.fract() == 0.0)
        }),
        VoiceParameterKind::Boolean => value.is_boolean(),
        VoiceParameterKind::Choice(choices) => {
            value.as_str().is_some_and(|text| choices.contains(&text))
        }
    }
}

/// Every parameter a preset writes, `preset` included (one graph edit).
pub fn voice_preset_parameters(preset: &VoicePreset) -> Map<String, Value> {
    let mut parameters = Map::new();
    parameters.insert("preset".to_owned(), Value::String(preset.id.to_owned()));
    for spec in &VOICE_CHANGER_PARAMETERS {
        let value = preset
            .values
            .iter()
            .find(|(name, _)| *name == spec.name)
            .map_or(spec.default, |(_, value)| *value);
        parameters.insert(spec.name.to_owned(), value.to_json());
    }
    parameters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_writes_valid_values_for_every_parameter() {
        let mut ids = std::collections::HashSet::new();
        for preset in &VOICE_PRESETS {
            assert!(ids.insert(preset.id), "duplicate preset {}", preset.id);
            for (name, _) in preset.values {
                assert!(
                    voice_parameter(name).is_some(),
                    "{}: unknown {name}",
                    preset.id
                );
            }
            for name in preset.basic {
                assert!(
                    voice_parameter(name).is_some(),
                    "{}: unknown {name}",
                    preset.id
                );
            }
            let parameters = voice_preset_parameters(preset);
            assert_eq!(parameters.len(), VOICE_CHANGER_PARAMETERS.len() + 1);
            for (name, value) in &parameters {
                assert!(
                    valid_voice_parameter(name, value),
                    "{}: {name} = {value}",
                    preset.id
                );
            }
        }
        assert_eq!(
            voice_preset_parameters(voice_preset("custom").unwrap()).len(),
            36
        );
    }

    #[test]
    fn defaults_are_valid_and_validation_rejects_out_of_range_and_wrong_types() {
        for spec in &VOICE_CHANGER_PARAMETERS {
            assert!(
                valid_voice_parameter(spec.name, &spec.default.to_json()),
                "{}",
                spec.name
            );
        }
        assert!(!valid_voice_parameter("pitchSemitones", &Value::from(13)));
        assert!(!valid_voice_parameter("pitchSemitones", &Value::from("3")));
        assert!(!valid_voice_parameter("crushBits", &Value::from(8.5)));
        assert!(valid_voice_parameter("crushBits", &Value::from(8)));
        assert!(!valid_voice_parameter("scale", &Value::from("lydian")));
        assert!(!valid_voice_parameter("ringEnabled", &Value::from(1)));
        assert!(!valid_voice_parameter("preset", &Value::from("spooky")));
        assert!(valid_voice_parameter("preset", &Value::from("robot")));
        assert!(!valid_voice_parameter("unknownKnob", &Value::from(1)));
    }
}
