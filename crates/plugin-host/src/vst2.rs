//! VST2 ABI declarations and the contained Windows worker adapter.
//!
//! Portable declarations remain non-executing. The Windows implementation
//! loads only an already-verified x64 DLL inside the disposable worker and
//! owns all `AEffect` access for its lifetime.

use std::ffi::c_void;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::path::Path;
#[cfg(windows)]
use std::ptr;
#[cfg(windows)]
use std::slice;
#[cfg(windows)]
use std::sync::mpsc::{self, Receiver, Sender};
#[cfg(windows)]
use std::thread::{self, JoinHandle};
#[cfg(windows)]
use std::time::Duration;

thread_local! {
    /// The VST2 shell sub-plugin ID the current load selects. A shell asks
    /// for it through `audioMasterCurrentId` while `VSTPluginMain` runs,
    /// before `AEffect::user` exists, so it cannot travel through the host
    /// context.
    static SHELL_CURRENT_ID: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// The VST2 `AEffect` magic (`'VstP'`) in little-endian form.
pub const VST2_EFFECT_MAGIC: i32 = 0x5673_7450;
/// `effFlagsCanReplacing`: the effect accepts the replacing process callback.
pub const VST2_FLAG_CAN_REPLACING: i32 = 1 << 4;
/// `effFlagsProgramChunks`: the plugin exposes opaque chunk state.
pub const VST2_FLAG_PROGRAM_CHUNKS: i32 = 1 << 5;
/// `effFlagsHasEditor`: the plugin exposes a native editor.
pub const VST2_FLAG_HAS_EDITOR: i32 = 1 << 0;
/// `effFlagsIsSynth`: the plugin is an instrument (needs MIDI input). Value
/// from the public `aeffect.h` (`effFlagsIsSynth = 1 << 8`; also vestige/FST).
pub const VST2_FLAG_IS_SYNTH: i32 = 1 << 8;
pub const VST2_MAX_AUDIO_CHANNELS: i32 = 64;
pub const VST2_MAX_INPUT_CHANNELS: i32 = 64;
pub const VST2_MAX_PARAMETERS: i32 = 65536;

#[repr(C)]
pub struct Vst2Effect {
    pub magic: i32,
    pub dispatcher: Option<Vst2Dispatcher>,
    pub process: Option<Vst2Process>,
    pub set_parameter: Option<Vst2SetParameter>,
    pub get_parameter: Option<Vst2GetParameter>,
    pub num_programs: i32,
    pub num_parameters: i32,
    pub num_inputs: i32,
    pub num_outputs: i32,
    pub flags: i32,
    pub reserved_1: isize,
    pub reserved_2: isize,
    pub initial_delay: i32,
    pub real_quality: i32,
    pub off_quality: i32,
    pub io_ratio: f32,
    pub object: *mut c_void,
    pub user: *mut c_void,
    pub unique_id: i32,
    pub version: i32,
    pub process_replacing: Option<Vst2Process>,
    pub future: [u8; 56],
}

pub type Vst2Dispatcher = unsafe extern "C" fn(
    effect: *mut Vst2Effect,
    opcode: i32,
    index: i32,
    value: isize,
    ptr: *mut c_void,
    opt: f32,
) -> isize;
pub type Vst2Process = unsafe extern "C" fn(
    effect: *mut Vst2Effect,
    inputs: *const *const f32,
    outputs: *mut *mut f32,
    sample_frames: i32,
);
pub type Vst2SetParameter = unsafe extern "C" fn(effect: *mut Vst2Effect, index: i32, value: f32);
pub type Vst2GetParameter = unsafe extern "C" fn(effect: *mut Vst2Effect, index: i32) -> f32;
pub type Vst2PluginMain = unsafe extern "C" fn(host_callback: Vst2HostCallback) -> *mut Vst2Effect;
pub type Vst2HostCallback = unsafe extern "C" fn(
    effect: *mut Vst2Effect,
    opcode: i32,
    index: i32,
    value: isize,
    ptr: *mut c_void,
    opt: f32,
) -> isize;

#[cfg(windows)]
struct Vst2HostContext {
    sample_rate_bits: std::sync::atomic::AtomicU32,
    block_size: std::sync::atomic::AtomicI32,
    sample_position: std::sync::atomic::AtomicU64,
    /// The shell window the editor is shown in (0 while closed), so the
    /// plugin can ask for a new size (`audioMasterSizeWindow`).
    editor_parent: std::sync::atomic::AtomicUsize,
    /// Owner PID authorized by the shell for this editor parent. The worker is
    /// a different process, so its own PID must not be used for size requests.
    editor_parent_process_id: std::sync::atomic::AtomicU32,
    /// Set when the plugin reported `audioMasterIOChanged` (its channel
    /// layout or latency changed); the worker re-reads both.
    io_changed: std::sync::atomic::AtomicBool,
}

#[cfg(windows)]
impl Default for Vst2HostContext {
    fn default() -> Self {
        Self {
            sample_rate_bits: std::sync::atomic::AtomicU32::new(48_000_f32.to_bits()),
            block_size: std::sync::atomic::AtomicI32::new(128),
            sample_position: std::sync::atomic::AtomicU64::new(0),
            editor_parent: std::sync::atomic::AtomicUsize::new(0),
            editor_parent_process_id: std::sync::atomic::AtomicU32::new(0),
            io_changed: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vst2HeaderError {
    InvalidMagic,
    MissingDispatcher,
    MissingReplacingProcessor,
    MissingParameterSetter,
    MissingParameterGetter,
    InvalidChannels {
        inputs: i32,
        outputs: i32,
    },
    TooManyParameters,
    ReplacingUnsupported,
    /// The plugin declares `effFlagsIsSynth` but was loaded as an effect:
    /// only an Instrument tool plays instruments.
    Instrument,
    /// The plugin does not declare `effFlagsIsSynth` but was loaded as an
    /// instrument.
    NotInstrument,
    /// The instrument answered "no" to both MIDI questions of `effCanDo`, so
    /// notes could never reach it.
    RefusesMidi,
}

/// A plugin's answer to `effCanDo`. Many plugins answer "don't know" (0) for
/// features they do support, so only an explicit "no" is treated as a refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vst2CanDo {
    Yes,
    No,
    Unknown,
}

impl Vst2CanDo {
    fn from_answer(answer: isize) -> Self {
        match answer {
            1.. => Self::Yes,
            0 => Self::Unknown,
            _ => Self::No,
        }
    }
}

/// How the worker uses a VST2 plugin: an effect processes the route's audio;
/// an instrument makes audio from MIDI and needs no audio input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vst2Role {
    Effect,
    Instrument,
}

impl Vst2HeaderError {
    /// Plain wording for the user; the worker prints it so the supervisor
    /// can show it ("plugin is an instrument ...").
    pub fn user_message(self) -> &'static str {
        match self {
            Self::Instrument => {
                "VST2 plugin is an instrument; add it with the Instrument tool to play it with MIDI"
            }
            Self::NotInstrument => "VST2 plugin is an effect, not an instrument",
            Self::RefusesMidi => "VST2 instrument reports that it does not accept MIDI notes",
            Self::InvalidChannels { .. } => {
                "VST2 plugin has an audio channel layout AudioRouter cannot use"
            }
            // Decision (2026-11-14): the pre-2.4 accumulating `process` call is
            // not used. No plugin in the census needed it; the refusal says why.
            Self::ReplacingUnsupported | Self::MissingReplacingProcessor => {
                "VST2 plugin only offers the old accumulating process call, which AudioRouter does not use"
            }
            _ => "VST2 plugin header is not a usable audio effect",
        }
    }
}

impl Vst2Effect {
    /// Whether the header declares an instrument. Readable without calling
    /// plugin code.
    pub fn is_instrument(&self) -> bool {
        self.flags & VST2_FLAG_IS_SYNTH != 0
    }

    /// Validate only fields that can be checked without invoking plugin code.
    /// The returned header is still untrusted until the worker owns its
    /// lifetime and applies the callback/audio-buffer policy.
    pub fn validate_audio_effect(&self) -> Result<(), Vst2HeaderError> {
        self.validate(Vst2Role::Effect)
    }

    /// Header checks for the chosen role. An instrument may have no audio
    /// inputs; an effect needs at least one and must not be a synth.
    pub fn validate(&self, role: Vst2Role) -> Result<(), Vst2HeaderError> {
        if self.magic != VST2_EFFECT_MAGIC {
            return Err(Vst2HeaderError::InvalidMagic);
        }
        if self.dispatcher.is_none() {
            return Err(Vst2HeaderError::MissingDispatcher);
        }
        if self.process_replacing.is_none() {
            return Err(Vst2HeaderError::MissingReplacingProcessor);
        }
        if self.set_parameter.is_none() {
            return Err(Vst2HeaderError::MissingParameterSetter);
        }
        if self.get_parameter.is_none() {
            return Err(Vst2HeaderError::MissingParameterGetter);
        }
        // Before the channel rules: an instrument often has no inputs, and
        // "is an instrument" is the useful reason.
        match (role, self.is_instrument()) {
            (Vst2Role::Effect, true) => return Err(Vst2HeaderError::Instrument),
            (Vst2Role::Instrument, false) => return Err(Vst2HeaderError::NotInstrument),
            _ => {}
        }
        let minimum_inputs = i32::from(role == Vst2Role::Effect);
        if !(minimum_inputs..=VST2_MAX_INPUT_CHANNELS).contains(&self.num_inputs)
            || !(1..=VST2_MAX_AUDIO_CHANNELS).contains(&self.num_outputs)
        {
            return Err(Vst2HeaderError::InvalidChannels {
                inputs: self.num_inputs,
                outputs: self.num_outputs,
            });
        }
        if !(0..=VST2_MAX_PARAMETERS).contains(&self.num_parameters) {
            return Err(Vst2HeaderError::TooManyParameters);
        }
        if self.flags & VST2_FLAG_CAN_REPLACING == 0 {
            return Err(Vst2HeaderError::ReplacingUnsupported);
        }
        Ok(())
    }
}

/// Fixed-width VST2 pin ABI (128 bytes on both x86 and x64). Labels are
/// bounded byte arrays, not pointers owned by the plugin.
#[repr(C)]
#[derive(Clone)]
#[cfg(any(windows, test))]
struct Vst2PinProperties {
    label: [u8; 64],
    flags: i32,
    arrangement_type: i32,
    short_label: [u8; 8],
    future: [u8; 48],
}

#[cfg(any(windows, test))]
impl Default for Vst2PinProperties {
    fn default() -> Self {
        Self {
            label: [0; 64],
            flags: 0,
            arrangement_type: 0,
            short_label: [0; 8],
            future: [0; 48],
        }
    }
}

#[cfg(any(windows, test))]
const EFF_GET_INPUT_PROPERTIES: i32 = 33;
const EFF_GET_PLUG_CATEGORY: i32 = 35;
const EFF_SHELL_GET_NEXT_PLUGIN: i32 = 70;
/// `kPlugCategShell`: one DLL that presents several effects.
const PLUG_CATEGORY_SHELL: isize = 10;
/// Bound on sub-plugins enumerated from one shell DLL.
const MAX_SHELL_SUBPLUGINS: usize = 64;

#[cfg(any(windows, test))]
fn pin_text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    // The ABI uses vendor-encoded bytes. Keep the wire name bounded even for
    // invalid UTF-8, without interpreting control bytes as display text.
    bytes[..end]
        .iter()
        .map(|byte| {
            if (32..=126).contains(byte) {
                char::from(*byte)
            } else {
                '?'
            }
        })
        .collect()
}

#[cfg(any(windows, test))]
fn explicitly_key_pin(pin: &Vst2PinProperties) -> bool {
    [pin_text(&pin.label), pin_text(&pin.short_label)]
        .iter()
        .any(|label| {
            let lower = label.to_ascii_lowercase();
            let words: Vec<_> = lower
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|s| !s.is_empty())
                .collect();
            words
                .iter()
                .any(|word| matches!(*word, "sidechain" | "key"))
                || words.windows(2).any(|pair| pair == ["side", "chain"])
        })
}

#[cfg(any(windows, test))]
fn vst2_discovered_buses(
    pins: &[Vst2PinProperties],
    output_channels: usize,
) -> Result<crate::bus_discovery::DiscoveredAudioBuses, crate::bus_discovery::BusDiscoveryError> {
    use crate::bus_discovery::{
        AudioBusRole, BusDiscoveryError, DiscoveredAudioBus, DiscoveredAudioBuses,
    };
    if pins.is_empty() || pins.len() > VST2_MAX_INPUT_CHANNELS as usize {
        return Err(BusDiscoveryError::InvalidBusCount);
    }
    let main_count = pins
        .iter()
        .position(explicitly_key_pin)
        .unwrap_or(pins.len());
    let key_count = pins.len() - main_count;
    if !(1..=2).contains(&main_count)
        || key_count > 2
        || !pins[main_count..].iter().all(explicitly_key_pin)
        || !(1..=2).contains(&output_channels)
    {
        return Err(BusDiscoveryError::UnsupportedLayout);
    }
    let mut inputs = vec![DiscoveredAudioBus {
        native_index: 0,
        name: "Main input".into(),
        channels: main_count as u16,
        role: AudioBusRole::Main,
    }];
    if key_count > 0 {
        inputs.push(DiscoveredAudioBus {
            native_index: main_count as u16,
            name: pin_text(&pins[main_count].label),
            channels: key_count as u16,
            role: AudioBusRole::Auxiliary,
        });
    }
    let buses = DiscoveredAudioBuses {
        inputs,
        outputs: vec![DiscoveredAudioBus {
            native_index: 0,
            name: "Main output".into(),
            channels: output_channels as u16,
            role: AudioBusRole::Main,
        }],
    };
    buses.processing_layout()?;
    Ok(buses)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labeled_pin(label: &str) -> Vst2PinProperties {
        let mut pin = Vst2PinProperties::default();
        pin.label[..label.len()].copy_from_slice(label.as_bytes());
        pin
    }

    #[test]
    fn pin_discovery_abi_and_opcode_are_fixed_width() {
        assert_eq!(EFF_GET_INPUT_PROPERTIES, 33);
        assert_eq!(std::mem::size_of::<Vst2PinProperties>(), 128);
        assert_eq!(std::mem::offset_of!(Vst2PinProperties, flags), 64);
        assert_eq!(std::mem::offset_of!(Vst2PinProperties, short_label), 72);
        assert_eq!(std::mem::offset_of!(Vst2PinProperties, future), 80);
    }

    #[test]
    fn only_explicit_key_labels_create_auxiliary_inputs() {
        for label in ["Sidechain L", "Side-chain", "Key input", "SIDE CHAIN R"] {
            assert!(explicitly_key_pin(&labeled_pin(label)), "{label}");
        }
        for label in ["Keyboard", "Monkey", "Surround", "Rear left", "Aux", ""] {
            assert!(!explicitly_key_pin(&labeled_pin(label)), "{label}");
        }
        let pins = [
            labeled_pin("Left"),
            labeled_pin("Right"),
            labeled_pin("Key L"),
            labeled_pin("Key R"),
        ];
        let buses = vst2_discovered_buses(&pins, 2).unwrap();
        assert_eq!(buses.processing_layout().unwrap().input_buses(), &[2, 2]);
        assert_eq!(buses.inputs[1].native_index, 2);
        assert!(vst2_discovered_buses(&[], 2).is_err());
    }

    #[test]
    fn surround_and_interleaved_key_pins_are_refused() {
        let surround = vec![Vst2PinProperties::default(); 6];
        assert!(vst2_discovered_buses(&surround, 2).is_err());
        let interleaved = [labeled_pin("Main"), labeled_pin("Key"), labeled_pin("Main")];
        assert!(vst2_discovered_buses(&interleaved, 2).is_err());
        let stereo = vec![Vst2PinProperties::default(); 2];
        assert!(!vst2_discovered_buses(&stereo, 2)
            .unwrap()
            .processing_layout()
            .unwrap()
            .has_sidechain());
    }

    unsafe extern "C" fn dispatcher(
        _: *mut Vst2Effect,
        _: i32,
        _: i32,
        _: isize,
        _: *mut c_void,
        _: f32,
    ) -> isize {
        0
    }

    unsafe extern "C" fn process(
        _: *mut Vst2Effect,
        _: *const *const f32,
        _: *mut *mut f32,
        _: i32,
    ) {
    }

    unsafe extern "C" fn set_parameter(_: *mut Vst2Effect, _: i32, _: f32) {}

    unsafe extern "C" fn get_parameter(_: *mut Vst2Effect, _: i32) -> f32 {
        0.0
    }

    fn valid_effect() -> Vst2Effect {
        Vst2Effect {
            magic: VST2_EFFECT_MAGIC,
            dispatcher: Some(dispatcher),
            process: Some(process),
            set_parameter: Some(set_parameter),
            get_parameter: Some(get_parameter),
            num_programs: 1,
            num_parameters: 8,
            num_inputs: 2,
            num_outputs: 2,
            flags: VST2_FLAG_CAN_REPLACING,
            reserved_1: 0,
            reserved_2: 0,
            initial_delay: 0,
            real_quality: 0,
            off_quality: 0,
            io_ratio: 1.0,
            object: std::ptr::null_mut(),
            user: std::ptr::null_mut(),
            unique_id: 1,
            version: 1,
            process_replacing: Some(process),
            future: [0; 56],
        }
    }

    #[test]
    fn validates_a_bounded_stereo_replacing_effect_header() {
        assert_eq!(valid_effect().validate_audio_effect(), Ok(()));
        let mut large = valid_effect();
        large.num_parameters = 1024;
        large.num_inputs = 8;
        large.num_outputs = 8;
        assert_eq!(large.validate_audio_effect(), Ok(()));
    }

    #[test]
    fn an_instrument_header_is_refused_with_its_own_reason() {
        // Regression (2026-11-13): BC PatchWork Synth loaded as an effect
        // because only its (empty) scan metadata was checked.
        let mut effect = valid_effect();
        effect.flags |= VST2_FLAG_IS_SYNTH;
        assert_eq!(
            effect.validate_audio_effect(),
            Err(Vst2HeaderError::Instrument)
        );
        // The reason wins over a missing-input layout too.
        effect.num_inputs = 0;
        assert_eq!(
            effect.validate_audio_effect(),
            Err(Vst2HeaderError::Instrument)
        );
        assert!(Vst2HeaderError::Instrument
            .user_message()
            .contains("instrument"));
    }

    #[test]
    fn an_instrument_role_needs_the_synth_flag_and_allows_no_audio_inputs() {
        let mut synth = valid_effect();
        synth.flags |= VST2_FLAG_IS_SYNTH;
        synth.num_inputs = 0;
        assert_eq!(synth.validate(Vst2Role::Instrument), Ok(()));
        // Not an effect: the other role still refuses it.
        assert_eq!(
            synth.validate(Vst2Role::Effect),
            Err(Vst2HeaderError::Instrument)
        );
        // An effect is not an instrument, whatever its channels.
        assert_eq!(
            valid_effect().validate(Vst2Role::Instrument),
            Err(Vst2HeaderError::NotInstrument)
        );
        // Outputs are still required and bounded.
        synth.num_outputs = 0;
        assert!(matches!(
            synth.validate(Vst2Role::Instrument),
            Err(Vst2HeaderError::InvalidChannels { .. })
        ));
    }

    /// A plugin that only has the pre-2.4 accumulating call is refused with
    /// that reason, not the generic header message.
    #[test]
    fn a_plugin_without_process_replacing_is_refused_with_its_own_reason() {
        let mut old = valid_effect();
        old.flags &= !VST2_FLAG_CAN_REPLACING;
        let error = old.validate_audio_effect().unwrap_err();
        assert_eq!(error, Vst2HeaderError::ReplacingUnsupported);
        assert!(error
            .user_message()
            .contains("old accumulating process call"));
    }

    #[test]
    fn the_events_argument_has_the_public_vstevents_layout() {
        assert_eq!(std::mem::size_of::<Vst2MidiEvent>(), 32);
        let events = [
            crate::MidiEvent::new(10, 0x90, 60, 100).unwrap(),
            crate::MidiEvent::new(70, 0x80, 60, 0).unwrap(),
            crate::MidiEvent::new(200, 0x90, 62, 90).unwrap(),
        ];
        // Only the events of the segment 64..128 go, relative to its start.
        let mut block = Vst2EventBlock::from_range(&events, 64, 128).unwrap();
        let word = std::mem::size_of::<isize>();
        let base = block.as_pointer() as *const u8;
        // SAFETY: `base` is the start of the block's word buffer, which holds
        // the numEvents word, the reserved word and one pointer word.
        let (count, reserved, first) = unsafe {
            (
                std::ptr::read(base.cast::<i32>()),
                std::ptr::read(base.add(word).cast::<isize>()),
                std::ptr::read(base.add(2 * word).cast::<*const Vst2MidiEvent>()),
            )
        };
        assert_eq!((count, reserved), (1, 0));
        // SAFETY: the pointer was just read from the block, which owns the event.
        let event = unsafe { *first };
        assert_eq!(event.event_type, 1);
        assert_eq!(event.byte_size, 32);
        assert_eq!(event.delta_frames, 6);
        assert_eq!(event.midi_data, [0x80, 60, 0, 0]);
        assert!(Vst2EventBlock::from_range(&events, 128, 192).is_none());
    }

    #[test]
    fn rejects_headers_that_cannot_be_used_as_bounded_audio_effects() {
        let mut effect = valid_effect();
        effect.magic = 0;
        assert_eq!(
            effect.validate_audio_effect(),
            Err(Vst2HeaderError::InvalidMagic)
        );

        let mut effect = valid_effect();
        effect.num_outputs = VST2_MAX_AUDIO_CHANNELS + 1;
        assert_eq!(
            effect.validate_audio_effect(),
            Err(Vst2HeaderError::InvalidChannels {
                inputs: 2,
                outputs: VST2_MAX_AUDIO_CHANNELS + 1,
            })
        );
        let mut effect = valid_effect();
        effect.process_replacing = None;
        assert_eq!(
            effect.validate_audio_effect(),
            Err(Vst2HeaderError::MissingReplacingProcessor)
        );
    }
}

#[cfg(windows)]
const EFF_OPEN: i32 = 0;
#[cfg(windows)]
const EFF_CLOSE: i32 = 1;
const EFF_SET_PROGRAM: i32 = 2;
const EFF_GET_PROGRAM: i32 = 3;
const EFF_GET_PROGRAM_NAME_INDEXED: i32 = 29;
const EFF_GET_EFFECT_NAME: i32 = 45;
#[cfg(windows)]
const EFF_SET_SAMPLE_RATE: i32 = 10;
#[cfg(windows)]
const EFF_SET_BLOCK_SIZE: i32 = 11;
#[cfg(windows)]
const EFF_MAINS_CHANGED: i32 = 12;
#[cfg(windows)]
const EFF_GET_PARAM_NAME: i32 = 8;
#[cfg(windows)]
const EFF_EDIT_GET_RECT: i32 = 13;
#[cfg(windows)]
const EFF_EDIT_OPEN: i32 = 14;
#[cfg(windows)]
const EFF_EDIT_CLOSE: i32 = 15;
#[cfg(windows)]
const EFF_EDIT_IDLE: i32 = 19;
#[cfg(windows)]
const EFF_PROCESS_EVENTS: i32 = 25;
/// `effStartProcess` and `effStopProcess` of the public aeffectx.h: a host
/// tells the plugin that a run of process calls begins or ends, after the mains
/// switch is on and before it goes off. Many plugins ignore them; some (and
/// their licence or DSP set-up) wait for them.
#[cfg(windows)]
/// `effCanDo`: asks the plugin whether it supports a named feature.
const EFF_CAN_DO: i32 = 51;
const EFF_START_PROCESS: i32 = 71;
#[cfg(windows)]
const EFF_STOP_PROCESS: i32 = 72;
const EFF_GET_CHUNK: i32 = 23;
#[cfg(windows)]
const EFF_SET_CHUNK: i32 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Vst2ChunkScope {
    Program,
    Bank,
}

impl Vst2ChunkScope {
    /// VST2 `effSetChunk` uses index 1 for a program/preset and index 0 for a
    /// bank. `effGetChunk` uses the same convention.
    const fn dispatcher_index(self) -> i32 {
        match self {
            Self::Program => 1,
            Self::Bank => 0,
        }
    }
}

#[cfg(test)]
mod chunk_scope_tests {
    use super::Vst2ChunkScope;

    #[test]
    fn program_and_bank_chunks_use_the_vst2_dispatcher_indices() {
        assert_eq!(Vst2ChunkScope::Program.dispatcher_index(), 1);
        assert_eq!(Vst2ChunkScope::Bank.dispatcher_index(), 0);
    }
}
#[cfg(windows)]
/// `kVstMidiType` and `kVstMidiEventIsRealtime` of the public aeffectx.h.
const VST_MIDI_EVENT_TYPE: i32 = 1;
const VST_MIDI_EVENT_IS_REALTIME: i32 = 1;

/// `VstMidiEvent`: 32 bytes with the same layout on 32- and 64-bit.
#[repr(C)]
#[derive(Clone, Copy)]
struct Vst2MidiEvent {
    event_type: i32,
    byte_size: i32,
    delta_frames: i32,
    flags: i32,
    note_length: i32,
    note_offset: i32,
    midi_data: [u8; 4],
    detune: i8,
    note_off_velocity: u8,
    reserved_1: u8,
    reserved_2: u8,
}
const _: () = assert!(std::mem::size_of::<Vst2MidiEvent>() == 32);

/// The memory behind one `effProcessEvents` argument: a `VstEvents` header
/// (`int32 numEvents`, pointer-sized `reserved`) followed by one pointer per
/// event. It is held as pointer-sized words, which gives the right layout for
/// both pointer widths: `numEvents` in the low four bytes of word 0 (the
/// 64-bit padding stays zero), `reserved` in word 1, pointers from word 2.
/// The event buffer is never resized after the pointers are taken, so they
/// stay valid for as long as this value lives.
struct Vst2EventBlock {
    _events: Vec<Vst2MidiEvent>,
    words: Vec<isize>,
}

impl Vst2EventBlock {
    /// Events whose offset lies in `start..end`, with offsets made relative to
    /// `start` (`deltaFrames`). `None` when there are none to send.
    fn from_range(events: &[crate::MidiEvent], start: usize, end: usize) -> Option<Self> {
        let converted: Vec<Vst2MidiEvent> = events
            .iter()
            .filter(|event| (start..end).contains(&(event.sample_offset as usize)))
            .map(|event| Vst2MidiEvent {
                event_type: VST_MIDI_EVENT_TYPE,
                byte_size: std::mem::size_of::<Vst2MidiEvent>() as i32,
                delta_frames: (event.sample_offset as usize - start) as i32,
                flags: VST_MIDI_EVENT_IS_REALTIME,
                note_length: 0,
                note_offset: 0,
                midi_data: [event.status, event.data1, event.data2, 0],
                detune: 0,
                note_off_velocity: 0,
                reserved_1: 0,
                reserved_2: 0,
            })
            .collect();
        if converted.is_empty() {
            return None;
        }
        let mut words = vec![0_isize; 2 + converted.len()];
        words[0] = converted.len() as isize;
        for (index, event) in converted.iter().enumerate() {
            words[2 + index] = event as *const Vst2MidiEvent as isize;
        }
        Some(Self {
            _events: converted,
            words,
        })
    }

    fn as_pointer(&mut self) -> *mut c_void {
        self.words.as_mut_ptr().cast()
    }
}

const MAX_VST2_STATE_BYTES: usize = crate::MAX_WORKER_STATE_BYTES;
/// Marks a parameter-bank state (see `save_parameter_bank`); a plugin chunk
/// never starts with it in practice.
#[cfg(windows)]
const PARAMETER_BANK_MAGIC: &[u8] = b"AudioRouter VST2 parameters v1\0";
#[cfg(windows)]
const AUDIO_MASTER_VERSION: i32 = 1;
#[cfg(windows)]
const AUDIO_MASTER_GET_SAMPLE_RATE: i32 = 16;
#[cfg(windows)]
const AUDIO_MASTER_GET_BLOCK_SIZE: i32 = 17;
// Other host opcodes (public aeffectx.h numbering; pinned in a test).
#[cfg(windows)]
const AUDIO_MASTER_AUTOMATE: i32 = 0;
#[cfg(windows)]
const AUDIO_MASTER_CURRENT_ID: i32 = 2;
#[cfg(windows)]
const AUDIO_MASTER_IDLE: i32 = 3;
#[cfg(windows)]
const AUDIO_MASTER_PIN_CONNECTED: i32 = 4;
#[cfg(windows)]
const AUDIO_MASTER_WANT_MIDI: i32 = 6;
#[cfg(windows)]
const AUDIO_MASTER_PROCESS_EVENTS: i32 = 8;
#[cfg(windows)]
const AUDIO_MASTER_IO_CHANGED: i32 = 13;
#[cfg(windows)]
const AUDIO_MASTER_NEED_IDLE: i32 = 14;
#[cfg(windows)]
const AUDIO_MASTER_GET_INPUT_LATENCY: i32 = 18;
#[cfg(windows)]
const AUDIO_MASTER_GET_OUTPUT_LATENCY: i32 = 19;
#[cfg(windows)]
const AUDIO_MASTER_WILL_REPLACE_OR_ACCUMULATE: i32 = 22;
#[cfg(windows)]
const AUDIO_MASTER_GET_AUTOMATION_STATE: i32 = 24;
#[cfg(windows)]
const AUDIO_MASTER_GET_VENDOR_STRING: i32 = 32;
#[cfg(windows)]
const AUDIO_MASTER_GET_PRODUCT_STRING: i32 = 33;
#[cfg(windows)]
const AUDIO_MASTER_CAN_DO: i32 = 37;
#[cfg(windows)]
const AUDIO_MASTER_GET_LANGUAGE: i32 = 38;
#[cfg(windows)]
const AUDIO_MASTER_UPDATE_DISPLAY: i32 = 42;
#[cfg(windows)]
const AUDIO_MASTER_BEGIN_EDIT: i32 = 43;
#[cfg(windows)]
const AUDIO_MASTER_END_EDIT: i32 = 44;
#[cfg(windows)]
const AUDIO_MASTER_SIZE_WINDOW: i32 = 15;
/// Largest editor side accepted from a plugin, in pixels.
#[cfg(windows)]
const MAX_EDITOR_SIDE: i32 = 4096;

#[cfg(all(test, windows))]
mod opcode_tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn uses_vst2_dispatcher_opcodes_for_lifecycle_and_state() {
        assert_eq!(VST2_FLAG_HAS_EDITOR, 1);
        assert_eq!(VST2_FLAG_CAN_REPLACING, 1 << 4);
        assert_eq!(VST2_FLAG_PROGRAM_CHUNKS, 1 << 5);
        assert_eq!(VST2_FLAG_IS_SYNTH, 256);
        assert_eq!(EFF_SET_SAMPLE_RATE, 10);
        assert_eq!(EFF_SET_BLOCK_SIZE, 11);
        assert_eq!(EFF_MAINS_CHANGED, 12);
        assert_eq!(EFF_EDIT_GET_RECT, 13);
        assert_eq!(EFF_EDIT_OPEN, 14);
        assert_eq!(EFF_EDIT_CLOSE, 15);
        assert_eq!(EFF_EDIT_IDLE, 19);
        assert_eq!(EFF_GET_CHUNK, 23);
        assert_eq!(EFF_SET_CHUNK, 24);
        assert_eq!(Vst2ChunkScope::Program.dispatcher_index(), 1);
        assert_eq!(Vst2ChunkScope::Bank.dispatcher_index(), 0);
        assert_eq!(EFF_START_PROCESS, 71);
        assert_eq!(EFF_STOP_PROCESS, 72);
    }

    thread_local! {
        static OPCODES: std::cell::RefCell<Vec<i32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    unsafe extern "C" fn recording_dispatcher(
        _: *mut Vst2Effect,
        opcode: i32,
        _: i32,
        _: isize,
        _: *mut c_void,
        _: f32,
    ) -> isize {
        OPCODES.with(|codes| codes.borrow_mut().push(opcode));
        0
    }

    thread_local! {
        static CAN_DO: std::cell::RefCell<Vec<(String, isize)>> = const { std::cell::RefCell::new(Vec::new()) };
        static CAN_DO_ASKED: std::cell::RefCell<Vec<(i32, String)>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    /// Answers `effCanDo` from the table in `CAN_DO` and records each question.
    unsafe extern "C" fn can_do_dispatcher(
        _: *mut Vst2Effect,
        opcode: i32,
        _: i32,
        _: isize,
        pointer: *mut c_void,
        _: f32,
    ) -> isize {
        if pointer.is_null() {
            return 0;
        }
        let feature = std::ffi::CStr::from_ptr(pointer as *const std::ffi::c_char)
            .to_string_lossy()
            .into_owned();
        CAN_DO_ASKED.with(|asked| asked.borrow_mut().push((opcode, feature.clone())));
        CAN_DO.with(|table| {
            table
                .borrow()
                .iter()
                .find(|(name, _)| *name == feature)
                .map_or(0, |(_, answer)| *answer)
        })
    }

    /// `effCanDo` is opcode 51 in the public `aeffectx.h`; an instrument is
    /// refused only when it says an explicit "no" to both MIDI questions.
    #[test]
    fn an_instrument_is_refused_only_when_it_says_no_to_both_midi_questions() {
        assert_eq!(EFF_CAN_DO, 51);
        let mut effect = Vst2Effect {
            magic: VST2_EFFECT_MAGIC,
            dispatcher: Some(can_do_dispatcher),
            process: None,
            set_parameter: None,
            get_parameter: None,
            num_programs: 0,
            num_parameters: 0,
            num_inputs: 0,
            num_outputs: 2,
            flags: VST2_FLAG_CAN_REPLACING | VST2_FLAG_IS_SYNTH,
            reserved_1: 0,
            reserved_2: 0,
            initial_delay: 0,
            real_quality: 0,
            off_quality: 0,
            io_ratio: 1.0,
            object: ptr::null_mut(),
            user: ptr::null_mut(),
            unique_id: 0,
            version: 0,
            process_replacing: None,
            future: [0; 56],
        };
        let mut accepts = |answers: &[(&str, isize)]| {
            CAN_DO.with(|table| {
                *table.borrow_mut() = answers
                    .iter()
                    .map(|(name, answer)| ((*name).to_owned(), *answer))
                    .collect();
            });
            // SAFETY: `effect` is a live local with a valid dispatcher.
            unsafe { instrument_accepts_midi(&mut effect) }
        };
        // Most plugins answer "don't know" (0) to everything.
        assert!(accepts(&[]));
        assert!(accepts(&[
            ("receiveVstEvents", 1),
            ("receiveVstMidiEvent", 1)
        ]));
        // One "no" is not enough: plugins disagree on which name they answer.
        assert!(accepts(&[("receiveVstEvents", -1)]));
        assert!(accepts(&[
            ("receiveVstEvents", -1),
            ("receiveVstMidiEvent", 1)
        ]));
        CAN_DO_ASKED.with(|asked| asked.borrow_mut().clear());
        assert!(!accepts(&[
            ("receiveVstEvents", -1),
            ("receiveVstMidiEvent", -1)
        ]));
        assert_eq!(
            CAN_DO_ASKED.with(|asked| asked.borrow().clone()),
            vec![
                (51, "receiveVstEvents".to_owned()),
                (51, "receiveVstMidiEvent".to_owned())
            ]
        );
        assert_eq!(Vst2CanDo::from_answer(1), Vst2CanDo::Yes);
        assert_eq!(Vst2CanDo::from_answer(0), Vst2CanDo::Unknown);
        assert_eq!(Vst2CanDo::from_answer(-1), Vst2CanDo::No);
        assert_eq!(
            Vst2HeaderError::RefusesMidi.user_message(),
            "VST2 instrument reports that it does not accept MIDI notes"
        );
    }

    /// The host brackets processing the way a conforming host does: start after
    /// the mains switch is on, stop before it goes off, on a format change and
    /// at close.
    #[test]
    fn start_and_stop_process_bracket_the_mains_switch() {
        let effect = Box::new(Vst2Effect {
            magic: VST2_EFFECT_MAGIC,
            dispatcher: Some(recording_dispatcher),
            process: None,
            set_parameter: None,
            get_parameter: None,
            num_programs: 0,
            num_parameters: 0,
            num_inputs: 1,
            num_outputs: 1,
            flags: VST2_FLAG_CAN_REPLACING,
            reserved_1: 0,
            reserved_2: 0,
            initial_delay: 0,
            real_quality: 0,
            off_quality: 0,
            io_ratio: 1.0,
            object: ptr::null_mut(),
            user: ptr::null_mut(),
            unique_id: 0,
            version: 0,
            process_replacing: None,
            future: [0; 56],
        });
        let mut library = Vst2Library {
            module: ptr::null_mut(),
            effect: Box::into_raw(effect),
            processing_format: None,
            editor_open: false,
            role: Vst2Role::Effect,
            host_context: Box::new(Vst2HostContext::default()),
        };
        OPCODES.with(|codes| codes.borrow_mut().clear());
        library.set_processing_format(48_000.0, 128).unwrap();
        library.set_processing_format(44_100.0, 128).unwrap();
        // A plugin that asks for an I/O change is suspended and resumed once.
        assert!(!library.apply_io_change());
        library
            .host_context
            .io_changed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(library.apply_io_change());
        assert!(!library.apply_io_change());
        drop(library);
        let seen = OPCODES.with(|codes| codes.borrow().clone());
        let (rate, block, mains, start, stop, close) = (
            EFF_SET_SAMPLE_RATE,
            EFF_SET_BLOCK_SIZE,
            EFF_MAINS_CHANGED,
            EFF_START_PROCESS,
            EFF_STOP_PROCESS,
            EFF_CLOSE,
        );
        assert_eq!(
            seen,
            vec![
                // First format: nothing to stop yet.
                rate, block, mains, start,
                // New format: stop, mains off, set up again, mains on, start.
                stop, mains, rate, block, mains, start,
                // ioChanged: stop, mains off, mains on, start.
                stop, mains, mains, start, // Close: stop, mains off, close.
                stop, mains, close,
            ]
        );
    }

    /// Regression (2026-11-13): the host answered sample rate and block size
    /// on opcodes 10 and 11 (tempoAt, getNumAutomatableParameters) instead of
    /// 16 and 17, so a plugin asking through the callback got 0. Numbers from
    /// the public aeffectx.h (checked against LMMS's copy), pinned literally.
    #[test]
    fn host_opcodes_match_the_public_aeffectx_numbering() {
        assert_eq!(AUDIO_MASTER_GET_SAMPLE_RATE, 16);
        assert_eq!(AUDIO_MASTER_GET_BLOCK_SIZE, 17);
        assert_eq!(AUDIO_MASTER_SIZE_WINDOW, 15);
        for (value, expected) in [
            (AUDIO_MASTER_AUTOMATE, 0),
            (AUDIO_MASTER_VERSION, 1),
            (AUDIO_MASTER_CURRENT_ID, 2),
            (AUDIO_MASTER_IDLE, 3),
            (AUDIO_MASTER_PIN_CONNECTED, 4),
            (AUDIO_MASTER_WANT_MIDI, 6),
            (AUDIO_MASTER_PROCESS_EVENTS, 8),
            (AUDIO_MASTER_IO_CHANGED, 13),
            (AUDIO_MASTER_NEED_IDLE, 14),
            (AUDIO_MASTER_GET_INPUT_LATENCY, 18),
            (AUDIO_MASTER_GET_OUTPUT_LATENCY, 19),
            (AUDIO_MASTER_WILL_REPLACE_OR_ACCUMULATE, 22),
            (AUDIO_MASTER_GET_AUTOMATION_STATE, 24),
            (AUDIO_MASTER_GET_VENDOR_STRING, 32),
            (AUDIO_MASTER_GET_PRODUCT_STRING, 33),
            (AUDIO_MASTER_CAN_DO, 37),
            (AUDIO_MASTER_GET_LANGUAGE, 38),
            (AUDIO_MASTER_UPDATE_DISPLAY, 42),
            (AUDIO_MASTER_BEGIN_EDIT, 43),
            (AUDIO_MASTER_END_EDIT, 44),
        ] {
            assert_eq!(value, expected);
        }
    }

    fn blank_effect(user: *mut c_void) -> Vst2Effect {
        Vst2Effect {
            magic: VST2_EFFECT_MAGIC,
            dispatcher: None,
            process: None,
            set_parameter: None,
            get_parameter: None,
            num_programs: 0,
            num_parameters: 0,
            num_inputs: 1,
            num_outputs: 1,
            flags: VST2_FLAG_CAN_REPLACING,
            reserved_1: 0,
            reserved_2: 0,
            initial_delay: 0,
            real_quality: 0,
            off_quality: 0,
            io_ratio: 1.0,
            object: ptr::null_mut(),
            user,
            unique_id: 0,
            version: 0,
            process_replacing: None,
            future: [0; 56],
        }
    }

    #[test]
    fn host_callback_answers_strings_capabilities_and_io_changes() {
        let context = Box::new(Vst2HostContext::default());
        let mut effect = blank_effect((&*context as *const Vst2HostContext) as *mut c_void);
        let effect = &mut effect as *mut Vst2Effect;
        let call = |opcode: i32, pointer: *mut c_void| {
            // SAFETY: `effect` is the live local AEffect whose host field is the
            // boxed context; each pointer argument is a live local buffer.
            unsafe { host_callback(effect, opcode, 0, 0, pointer, 0.0) }
        };
        let mut buffer = [0xffu8; 64];
        assert_eq!(
            call(AUDIO_MASTER_GET_VENDOR_STRING, buffer.as_mut_ptr().cast()),
            1
        );
        assert_eq!(&buffer[..12], b"AudioRouter\0");
        let mut product = [0xffu8; 64];
        assert_eq!(
            call(AUDIO_MASTER_GET_PRODUCT_STRING, product.as_mut_ptr().cast()),
            1
        );
        assert_eq!(&product[..12], b"AudioRouter\0");
        assert_eq!(call(AUDIO_MASTER_GET_VENDOR_STRING, ptr::null_mut()), 0);
        let ask = |text: &[u8]| {
            let mut c = text.to_vec();
            c.push(0);
            call(AUDIO_MASTER_CAN_DO, c.as_mut_ptr().cast())
        };
        assert_eq!(ask(b"sizeWindow"), 1);
        assert_eq!(ask(b"acceptIOChanges"), 1);
        assert_eq!(ask(b"receiveVstMidiEvent"), -1);
        assert_eq!(ask(b"somethingNew"), 0);
        assert_eq!(
            call(AUDIO_MASTER_WILL_REPLACE_OR_ACCUMULATE, ptr::null_mut()),
            1
        );
        assert_eq!(call(AUDIO_MASTER_GET_LANGUAGE, ptr::null_mut()), 1);
        assert!(!context
            .io_changed
            .load(std::sync::atomic::Ordering::Acquire));
        assert_eq!(call(AUDIO_MASTER_IO_CHANGED, ptr::null_mut()), 1);
        assert!(context
            .io_changed
            .load(std::sync::atomic::Ordering::Acquire));
    }

    #[test]
    fn host_callback_reports_the_negotiated_processing_format() {
        let context = Box::new(Vst2HostContext {
            sample_rate_bits: std::sync::atomic::AtomicU32::new(44_100_f32.to_bits()),
            block_size: std::sync::atomic::AtomicI32::new(512),
            ..Default::default()
        });
        let mut effect = Vst2Effect {
            magic: VST2_EFFECT_MAGIC,
            dispatcher: None,
            process: None,
            set_parameter: None,
            get_parameter: None,
            num_programs: 0,
            num_parameters: 0,
            num_inputs: 1,
            num_outputs: 1,
            flags: VST2_FLAG_CAN_REPLACING,
            reserved_1: 0,
            reserved_2: 0,
            initial_delay: 0,
            real_quality: 0,
            off_quality: 0,
            io_ratio: 1.0,
            object: ptr::null_mut(),
            user: (&*context as *const Vst2HostContext) as *mut c_void,
            unique_id: 0,
            version: 0,
            process_replacing: None,
            future: [0; 56],
        };
        let effect = &mut effect as *mut Vst2Effect;

        assert_eq!(
            // SAFETY: `effect` points to the live local AEffect above, whose host
            // field points to the boxed `context` that outlives both calls; the opcode
            // ignores the remaining arguments, including the null pointer.
            unsafe {
                host_callback(
                    effect,
                    AUDIO_MASTER_GET_SAMPLE_RATE,
                    0,
                    0,
                    ptr::null_mut(),
                    0.0,
                )
            },
            44_100
        );
        assert_eq!(
            // SAFETY: same live effect and context as the call above.
            unsafe {
                host_callback(
                    effect,
                    AUDIO_MASTER_GET_BLOCK_SIZE,
                    0,
                    0,
                    ptr::null_mut(),
                    0.0,
                )
            },
            512
        );
    }

    thread_local! {
        static DISPATCH_COUNT: Cell<usize> = const { Cell::new(0) };
    }

    unsafe extern "C" fn counting_dispatcher(
        _: *mut Vst2Effect,
        _: i32,
        _: i32,
        _: isize,
        _: *mut c_void,
        _: f32,
    ) -> isize {
        DISPATCH_COUNT.with(|count| count.set(count.get() + 1));
        0
    }

    unsafe extern "C" fn non_finite_process(
        _: *mut Vst2Effect,
        _: *const *const f32,
        outputs: *mut *mut f32,
        frames: i32,
    ) {
        // SAFETY: The test supplies one output channel with the declared
        // number of frames to the ABI callback.
        unsafe {
            for frame in 0..frames as usize {
                *(*outputs).add(frame) = f32::NAN;
            }
        }
    }

    #[test]
    fn processing_format_setup_is_cached_until_the_format_changes() {
        let effect = Box::new(Vst2Effect {
            magic: VST2_EFFECT_MAGIC,
            dispatcher: Some(counting_dispatcher),
            process: None,
            set_parameter: None,
            get_parameter: None,
            num_programs: 0,
            num_parameters: 0,
            num_inputs: 1,
            num_outputs: 1,
            flags: VST2_FLAG_HAS_EDITOR,
            reserved_1: 0,
            reserved_2: 0,
            initial_delay: 0,
            real_quality: 0,
            off_quality: 0,
            io_ratio: 1.0,
            object: ptr::null_mut(),
            user: ptr::null_mut(),
            unique_id: 0,
            version: 0,
            process_replacing: None,
            future: [0; 56],
        });
        let effect = Box::into_raw(effect);
        let mut library = Vst2Library {
            module: ptr::null_mut(),
            effect,
            processing_format: None,
            editor_open: false,
            role: Vst2Role::Effect,
            host_context: Box::new(Vst2HostContext::default()),
        };
        DISPATCH_COUNT.with(|count| count.set(0));

        assert!(matches!(
            library.set_processing_format(7_999.0, 128),
            Err(Vst2LibraryError::InvalidPath)
        ));
        assert!(matches!(
            library.set_processing_format(8_000.5, 128),
            Err(Vst2LibraryError::InvalidPath)
        ));
        assert!(matches!(
            library.set_processing_format(192_001.0, 128),
            Err(Vst2LibraryError::InvalidPath)
        ));
        assert!(matches!(
            library.set_processing_format(48_000.0, 2_049),
            Err(Vst2LibraryError::InvalidPath)
        ));
        assert_eq!(DISPATCH_COUNT.with(Cell::get), 0);
        library.set_processing_format(48_000.0, 128).unwrap();
        library.set_processing_format(48_000.0, 128).unwrap();
        library.set_processing_format(48_000.0, 256).unwrap();

        assert!(matches!(
            library.open_editor(1, std::process::id()),
            Err(Vst2LibraryError::InvalidEditor)
        ));
        assert_eq!(DISPATCH_COUNT.with(Cell::get), 10);
        assert!(matches!(
            library.close_editor(),
            Err(Vst2LibraryError::InvalidEditor)
        ));
        // ReaJS advertises chunks but returns zero before a script is chosen.
        // A parameter-bank envelope must remain a valid, restorable asset.
        // SAFETY: this test owns the live fake effect until library is dropped.
        unsafe {
            (*library.effect).flags |= VST2_FLAG_PROGRAM_CHUNKS;
        }
        let saved = library.save_state().unwrap();
        assert!(saved.starts_with(PARAMETER_BANK_MAGIC));
        library.restore_state(&saved).unwrap();
        drop(library);
    }

    #[test]
    fn process_replacing_rejects_non_finite_native_output() {
        let effect = Box::new(Vst2Effect {
            magic: VST2_EFFECT_MAGIC,
            dispatcher: Some(counting_dispatcher),
            process: None,
            set_parameter: None,
            get_parameter: None,
            num_programs: 0,
            num_parameters: 0,
            num_inputs: 1,
            num_outputs: 1,
            flags: VST2_FLAG_CAN_REPLACING,
            reserved_1: 0,
            reserved_2: 0,
            initial_delay: 0,
            real_quality: 0,
            off_quality: 0,
            io_ratio: 1.0,
            object: ptr::null_mut(),
            user: ptr::null_mut(),
            unique_id: 0,
            version: 0,
            process_replacing: Some(non_finite_process),
            future: [0; 56],
        });
        let effect = Box::into_raw(effect);
        let mut library = Vst2Library {
            module: ptr::null_mut(),
            effect,
            processing_format: None,
            editor_open: false,
            role: Vst2Role::Effect,
            host_context: Box::new(Vst2HostContext::default()),
        };
        let input = [0.0; 4];
        let mut output = [0.0; 4];
        let inputs = [&input[..]];

        let mut outputs = [&mut output[..]];
        assert!(matches!(
            library.process_replacing(&inputs, &mut outputs),
            Err(Vst2LibraryError::NonFiniteOutput)
        ));

        let mut mismatched_output = [0.0; 3];
        let mut malformed_outputs = [&mut mismatched_output[..]];
        assert!(matches!(
            library.process_replacing(&inputs, &mut malformed_outputs),
            Err(Vst2LibraryError::InvalidPath)
        ));
        let extra_input = [0.0; 4];
        let mut extra_output = [0.0; 4];
        let mismatched_inputs = [&input[..], &extra_input[..]];
        let mut mismatched_outputs = [&mut output[..], &mut extra_output[..]];
        assert!(matches!(
            library.process_replacing(&mismatched_inputs, &mut mismatched_outputs),
            Err(Vst2LibraryError::InvalidPath)
        ));
        let oversized_input = vec![0.0; crate::MAX_WORKER_FRAMES + 1];
        let mut oversized_output = vec![0.0; crate::MAX_WORKER_FRAMES + 1];
        let oversized_inputs = [&oversized_input[..]];
        let mut oversized_outputs = [&mut oversized_output[..]];
        assert!(matches!(
            library.process_replacing(&oversized_inputs, &mut oversized_outputs),
            Err(Vst2LibraryError::InvalidPath)
        ));
        drop(library);
    }
}

#[cfg(windows)]
#[derive(Debug)]
pub enum Vst2LibraryError {
    InvalidPath,
    LoadLibrary(u32),
    MissingEntryPoint,
    NullEffect,
    InvalidEffect(Vst2HeaderError),
    InvalidParameter,
    InvalidState,
    StateUnsupported,
    StateTooLarge,
    InvalidEditor,
    NonFiniteOutput,
}

/// Sends `effCanDo` for one feature name.
///
/// # Safety
/// `effect` must point to a validated, opened effect that stays alive for the
/// call. `feature` is NUL-terminated and outlives the call; the plugin only
/// reads it.
unsafe fn can_do(effect: *mut Vst2Effect, feature: &std::ffi::CStr) -> Vst2CanDo {
    let dispatcher = (*effect).dispatcher.expect("validated dispatcher");
    Vst2CanDo::from_answer(dispatcher(
        effect,
        EFF_CAN_DO,
        0,
        0,
        feature.as_ptr() as *mut c_void,
        0.0,
    ))
}

/// Whether an instrument can receive notes: false only when it answers an
/// explicit "no" to both MIDI questions.
///
/// # Safety
/// Same contract as [`can_do`].
unsafe fn instrument_accepts_midi(effect: *mut Vst2Effect) -> bool {
    can_do(effect, c"receiveVstEvents") != Vst2CanDo::No
        || can_do(effect, c"receiveVstMidiEvent") != Vst2CanDo::No
}

#[cfg(windows)]
pub struct Vst2Library {
    module: *mut c_void,
    effect: *mut Vst2Effect,
    processing_format: Option<(u32, i32)>,
    editor_open: bool,
    role: Vst2Role,
    // Declared last so the context outlives the effect/module cleanup. VST2
    // reserves AEffect::user for host-owned instance data; resvd1/resvd2
    // remain reserved by the ABI and are not repurposed.
    host_context: Box<Vst2HostContext>,
}

#[cfg(windows)]
impl Vst2Library {
    /// Load one already-verified native x64 VST2 DLL. This function executes
    /// the DLL's loader and VST2 entry point; callers must invoke it only from
    /// the disposable worker after identity and job-sandbox setup.
    pub fn load(path: &Path) -> Result<Self, Vst2LibraryError> {
        Self::load_as(path, Vst2Role::Effect)
    }

    /// Like `load`, for an effect or an instrument.
    pub fn load_as(path: &Path, role: Vst2Role) -> Result<Self, Vst2LibraryError> {
        if !path.is_absolute() || !path.is_file() {
            return Err(Vst2LibraryError::InvalidPath);
        }
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: `wide` is NUL-terminated and remains alive for the call;
        // Windows owns the returned module handle until `FreeLibrary`.
        let module = unsafe { load_library_w(wide.as_ptr()) };
        if module.is_null() {
            return Err(Vst2LibraryError::LoadLibrary(last_error()));
        }
        // SAFETY: Both names are established VST2 entry-point spellings and
        // are converted only after GetProcAddress returns a non-null address.
        // The VST2 ABI defines the same C calling convention for each.
        let entry_address = unsafe { get_proc_address(module, c"VSTPluginMain".as_ptr()) }
            .or_else(|| unsafe { get_proc_address(module, c"main".as_ptr()) });
        let Some(entry) = entry_address
            // SAFETY: same contract as the first lookup: `module` is the live handle and
            // the name is a static NUL-terminated C string.
            .map(|address| unsafe { std::mem::transmute::<*mut c_void, Vst2PluginMain>(address) })
        else {
            // SAFETY: `module` is the valid handle returned above and has not
            // been transferred anywhere else.
            unsafe { free_library(module) };
            return Err(Vst2LibraryError::MissingEntryPoint);
        };
        // SAFETY: The entry point is from the validated module. The callback
        // performs no blocking work and returns no borrowed data.
        let effect = unsafe { entry(host_callback) };
        if effect.is_null() {
            // SAFETY: `module` remains owned locally after a null entry result.
            unsafe { free_library(module) };
            return Err(Vst2LibraryError::NullEffect);
        }
        // SAFETY: The plugin owns this pointer, but the entry-point contract
        // makes the returned AEffect readable until effClose.
        let validation = unsafe { (*effect).validate(role) };
        if let Err(error) = validation {
            // SAFETY: A valid dispatcher is required by the ABI, but invalid
            // headers cannot be trusted. Unload without invoking plugin code.
            unsafe { free_library(module) };
            return Err(Vst2LibraryError::InvalidEffect(error));
        }
        let host_context = Box::new(Vst2HostContext::default());
        // SAFETY: `host_context` is boxed before the effect is opened and is
        // retained by this handle until after effClose and module unload.
        unsafe {
            (*effect).user = (&*host_context as *const Vst2HostContext) as *mut c_void;
        }
        // SAFETY: The validated dispatcher is called only during worker setup,
        // never from the realtime engine callback.
        unsafe {
            ((*effect).dispatcher.expect("validated dispatcher"))(
                effect,
                EFF_OPEN,
                0,
                0,
                ptr::null_mut(),
                0.0,
            )
        };
        // SAFETY: the effect was opened just above and is still owned here.
        if role == Vst2Role::Instrument && !unsafe { instrument_accepts_midi(effect) } {
            // SAFETY: close the opened effect before unloading its module; neither
            // is used again.
            unsafe {
                ((*effect).dispatcher.expect("validated dispatcher"))(
                    effect,
                    EFF_CLOSE,
                    0,
                    0,
                    ptr::null_mut(),
                    0.0,
                );
                free_library(module);
            }
            return Err(Vst2LibraryError::InvalidEffect(
                Vst2HeaderError::RefusesMidi,
            ));
        }
        Ok(Self {
            module,
            effect,
            processing_format: None,
            editor_open: false,
            role,
            host_context,
        })
    }

    /// Load one sub-effect of a shell DLL by the ID that
    /// `effShellGetNextPlugin` reported, answering `audioMasterCurrentId`
    /// during load. Every other part of the load path is unchanged.
    pub fn load_shell_sub_plugin_as(
        path: &Path,
        role: Vst2Role,
        sub_plugin_id: u32,
    ) -> Result<Self, Vst2LibraryError> {
        SHELL_CURRENT_ID.with(|cell| cell.set(sub_plugin_id));
        let result = Self::load_as(path, role);
        SHELL_CURRENT_ID.with(|cell| cell.set(0));
        result
    }

    /// The AEffect unique ID (the four-character plugin code).
    pub fn unique_id(&self) -> u32 {
        // SAFETY: this handle was validated at load, so the field is readable
        // for its lifetime.
        unsafe { (*self.effect).unique_id as u32 }
    }

    /// The plugin's display name from `effGetEffectName`.
    pub fn effect_name(&mut self) -> String {
        let mut name = [0_u8; 64];
        // SAFETY: the dispatcher was validated at load; the buffer is
        // caller-owned and bounded to the VST2 name contract.
        unsafe {
            ((*self.effect).dispatcher.expect("validated dispatcher"))(
                self.effect,
                EFF_GET_EFFECT_NAME,
                0,
                0,
                name.as_mut_ptr().cast(),
                0.0,
            );
        }
        let end = name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(name.len());
        String::from_utf8_lossy(&name[..end]).trim().to_owned()
    }

    /// Asks the plugin whether it supports a named feature (`effCanDo`).
    pub fn can_do(&mut self, feature: &std::ffi::CStr) -> Vst2CanDo {
        // SAFETY: the effect is owned by this handle; see `can_do`.
        unsafe { can_do(self.effect, feature) }
    }

    pub fn set_processing_format(
        &mut self,
        sample_rate_hz: f32,
        block_size: i32,
    ) -> Result<(), Vst2LibraryError> {
        if !sample_rate_hz.is_finite()
            || sample_rate_hz.fract() != 0.0
            || !(8_000.0..=192_000.0).contains(&sample_rate_hz)
            || !(1..=2048).contains(&block_size)
        {
            return Err(Vst2LibraryError::InvalidPath);
        }
        let sample_rate_bits = sample_rate_hz.to_bits();
        if self.processing_format == Some((sample_rate_bits, block_size)) {
            return Ok(());
        }
        self.host_context.sample_rate_bits.store(
            sample_rate_hz.to_bits(),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.host_context
            .block_size
            .store(block_size, std::sync::atomic::Ordering::Relaxed);
        // SAFETY: The effect was validated at load and remains owned by this
        // handle. These setup opcodes run on the worker control thread.
        unsafe {
            let dispatcher = (*self.effect).dispatcher.expect("validated dispatcher");
            if self.processing_format.is_some() {
                dispatcher(self.effect, EFF_STOP_PROCESS, 0, 0, ptr::null_mut(), 0.0);
                dispatcher(self.effect, EFF_MAINS_CHANGED, 0, 0, ptr::null_mut(), 0.0);
            }
            dispatcher(
                self.effect,
                EFF_SET_SAMPLE_RATE,
                0,
                0,
                ptr::null_mut(),
                sample_rate_hz,
            );
            dispatcher(
                self.effect,
                EFF_SET_BLOCK_SIZE,
                0,
                block_size as isize,
                ptr::null_mut(),
                0.0,
            );
            dispatcher(self.effect, EFF_MAINS_CHANGED, 0, 1, ptr::null_mut(), 0.0);
            dispatcher(self.effect, EFF_START_PROCESS, 0, 0, ptr::null_mut(), 0.0);
        }
        self.processing_format = Some((sample_rate_bits, block_size));
        Ok(())
    }

    pub fn input_channels(&self) -> usize {
        // SAFETY: This method is only available for a successfully validated
        // handle, whose channel count is bounded by validate_audio_effect.
        unsafe { (*self.effect).num_inputs as usize }
    }

    pub fn output_channels(&self) -> usize {
        // SAFETY: This method is only available for a successfully validated
        // handle, whose channel count is bounded by validate_audio_effect.
        unsafe { (*self.effect).num_outputs as usize }
    }

    pub fn parameter_count(&self) -> usize {
        // SAFETY: This method is only available for a successfully validated
        // handle, whose parameter count is bounded by validate_audio_effect.
        unsafe { (*self.effect).num_parameters as usize }
    }

    /// Query pins only in the isolated worker, between processing quanta.
    pub fn describe_buses(
        &mut self,
    ) -> Result<crate::bus_discovery::DiscoveredAudioBuses, crate::bus_discovery::BusDiscoveryError>
    {
        let mut pins = vec![Vst2PinProperties::default(); self.input_channels()];
        for (index, pin) in pins.iter_mut().enumerate() {
            // SAFETY: the live library owns a validated effect/dispatcher;
            // `pin` is a zeroed, correctly aligned 128-byte caller-owned ABI
            // buffer, alive through this synchronous worker-thread call.
            let supported = unsafe {
                (*self.effect).dispatcher.expect("validated dispatcher")(
                    self.effect,
                    EFF_GET_INPUT_PROPERTIES,
                    index as i32,
                    0,
                    (pin as *mut Vst2PinProperties).cast(),
                    0.0,
                )
            };
            if supported != 1 {
                *pin = Vst2PinProperties::default();
            }
        }
        vst2_discovered_buses(&pins, self.output_channels())
    }

    pub fn set_parameter(&mut self, parameter_id: u32, value: f32) -> Result<(), Vst2LibraryError> {
        if !value.is_finite()
            || !(0.0..=1.0).contains(&value)
            || usize::try_from(parameter_id)
                .ok()
                .filter(|id| *id < self.parameter_count())
                .is_none()
        {
            return Err(Vst2LibraryError::InvalidParameter);
        }
        // SAFETY: The setter and effect pointer were validated at load; this
        // control operation runs on the worker thread before processing.
        unsafe {
            (*self.effect)
                .set_parameter
                .expect("validated parameter setter")(
                self.effect, parameter_id as i32, value
            );
        }
        Ok(())
    }

    pub fn parameter_descriptors(
        &mut self,
    ) -> Result<Vec<crate::ParameterDescriptor>, Vst2LibraryError> {
        let count = self.parameter_count().min(crate::MAX_PARAMETER_DESCRIPTORS);
        let mut descriptors = Vec::with_capacity(count);
        for parameter_id in 0..count {
            let mut title = [0_u8; 64];
            // SAFETY: The dispatcher was validated at load; the buffer is
            // caller-owned and bounded to the VST2 parameter-name contract.
            unsafe {
                (*self.effect).dispatcher.expect("validated dispatcher")(
                    self.effect,
                    EFF_GET_PARAM_NAME,
                    parameter_id as i32,
                    0,
                    title.as_mut_ptr().cast(),
                    0.0,
                );
            }
            let title_end = title
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(title.len());
            let title = String::from_utf8_lossy(&title[..title_end]);
            let title = if title.trim().is_empty() {
                format!("Parameter {parameter_id}")
            } else {
                title.into_owned()
            };
            // SAFETY: The getter was validated at load and receives a bounded
            // parameter index from the plugin's own count.
            let default = unsafe {
                ((*self.effect)
                    .get_parameter
                    .expect("validated parameter getter"))(
                    self.effect, parameter_id as i32
                )
            };
            let default = if default.is_finite() {
                default.clamp(0.0, 1.0)
            } else {
                0.0
            };
            descriptors.push(
                crate::ParameterDescriptor::new(parameter_id as u32, title, default, 0.0, 1.0)
                    .map_err(|_| Vst2LibraryError::InvalidParameter)?,
            );
        }
        Ok(descriptors)
    }

    /// Whether this DLL presents itself as a shell of several effects.
    pub fn is_shell(&mut self) -> bool {
        // SAFETY: the dispatcher was validated at load; opcode 35 has no
        // pointer arguments.
        unsafe {
            ((*self.effect).dispatcher.expect("validated dispatcher"))(
                self.effect,
                EFF_GET_PLUG_CATEGORY,
                0,
                0,
                ptr::null_mut(),
                0.0,
            ) == PLUG_CATEGORY_SHELL
        }
    }

    /// Enumerate the sub-plugins of a shell DLL without loading one. Each
    /// entry carries the plugin ID the shell expects back through
    /// `audioMasterCurrentId` when that sub-plugin is loaded.
    pub fn shell_sub_plugins(
        &mut self,
    ) -> Result<Vec<crate::ShellPluginDescriptor>, Vst2LibraryError> {
        if !self.is_shell() {
            return Ok(Vec::new());
        }
        // Hosts disagree on this opcode: some advance an internal cursor and
        // want 0/0 on every call, others read the supplied index. Try the
        // indexed form first, then the cursor form, accepting whichever
        // returns a non-empty bounded list.
        let indexed = self.walk_shell_sub_plugins(true);
        if !indexed.is_empty() {
            return Ok(indexed);
        }
        Ok(self.walk_shell_sub_plugins(false))
    }

    fn walk_shell_sub_plugins(&mut self, indexed: bool) -> Vec<crate::ShellPluginDescriptor> {
        let mut plugins = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for step in 0..MAX_SHELL_SUBPLUGINS {
            let mut name = [0_u8; 64];
            // SAFETY: the dispatcher was validated at load; the buffer is
            // caller-owned and bounded to the VST2 name contract.
            let id = unsafe {
                ((*self.effect).dispatcher.expect("validated dispatcher"))(
                    self.effect,
                    EFF_SHELL_GET_NEXT_PLUGIN,
                    if indexed { step as i32 } else { 0 },
                    0,
                    name.as_mut_ptr().cast(),
                    0.0,
                )
            };
            if id == 0 {
                break;
            }
            // A VST2 unique ID is a four-character code sign-extended into a
            // pointer-sized integer; keep its low 32 bits.
            let id = id as i32 as u32;
            if !seen.insert(id) {
                break;
            }
            let end = name
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(name.len());
            let text = String::from_utf8_lossy(&name[..end]).trim().to_owned();
            let text = if text.is_empty() {
                format!("Shell plugin {}", plugins.len() + 1)
            } else {
                text
            };
            if let Ok(descriptor) = crate::ShellPluginDescriptor::new(id, text) {
                plugins.push(descriptor);
            }
        }
        plugins
    }

    /// The number of programs this plugin reports, bounded for display.
    pub fn program_count(&self) -> usize {
        // SAFETY: `self.effect` is a validated, non-null AEffect for this
        // library's lifetime. Reading a plain count field does not call plugin
        // code.
        let count = unsafe { (*self.effect).num_programs };
        usize::try_from(count.max(0))
            .unwrap_or(0)
            .min(crate::MAX_PROGRAMS)
    }

    /// The plugin's currently selected program index.
    pub fn current_program(&mut self) -> Result<u32, Vst2LibraryError> {
        // SAFETY: the dispatcher was validated at load; opcode 3 has no
        // pointer arguments.
        let current = unsafe {
            ((*self.effect).dispatcher.expect("validated dispatcher"))(
                self.effect,
                EFF_GET_PROGRAM,
                0,
                0,
                ptr::null_mut(),
                0.0,
            )
        };
        Ok(u32::try_from(current.max(0)).unwrap_or(0))
    }

    /// Bounded program names, in plugin order. A plugin that answers with an
    /// empty or invalid name gets a positional placeholder rather than failing
    /// the whole list.
    pub fn programs(&mut self) -> Result<Vec<crate::ProgramDescriptor>, Vst2LibraryError> {
        let count = self.program_count();
        let mut programs = Vec::with_capacity(count);
        for index in 0..count {
            let mut name = [0_u8; 64];
            // SAFETY: the dispatcher was validated at load; the buffer is
            // caller-owned and bounded to the VST2 program-name contract.
            let returned = unsafe {
                ((*self.effect).dispatcher.expect("validated dispatcher"))(
                    self.effect,
                    EFF_GET_PROGRAM_NAME_INDEXED,
                    index as i32,
                    0,
                    name.as_mut_ptr().cast(),
                    0.0,
                )
            };
            let text = if returned != 0 {
                let end = name
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(name.len());
                String::from_utf8_lossy(&name[..end]).trim().to_owned()
            } else {
                String::new()
            };
            let text = if text.is_empty() {
                format!("Program {}", index + 1)
            } else {
                text
            };
            programs.push(
                crate::ProgramDescriptor::new(index as u32, text)
                    .map_err(|_| Vst2LibraryError::InvalidParameter)?,
            );
        }
        Ok(programs)
    }

    /// Select one program by its bounded index. Out-of-range values are
    /// refused without calling plugin code.
    pub fn set_program(&mut self, index: u32) -> Result<(), Vst2LibraryError> {
        if index as usize >= self.program_count() {
            return Err(Vst2LibraryError::InvalidParameter);
        }
        // SAFETY: the dispatcher was validated at load; opcode 2 has no
        // pointer arguments.
        unsafe {
            ((*self.effect).dispatcher.expect("validated dispatcher"))(
                self.effect,
                EFF_SET_PROGRAM,
                index as i32,
                0,
                ptr::null_mut(),
                0.0,
            );
        }
        Ok(())
    }

    /// A plugin without chunk support (for example ReaComp, ReaGate) is
    /// described completely by its parameters, so its state is a parameter
    /// bank: `PARAMETER_BANK_MAGIC`, a little-endian `u32` count, then that
    /// many little-endian `f32` normalized values.
    fn save_parameter_bank(&mut self) -> Vec<u8> {
        let count = self.parameter_count();
        let mut bytes = Vec::with_capacity(PARAMETER_BANK_MAGIC.len() + 4 + count * 4);
        bytes.extend_from_slice(PARAMETER_BANK_MAGIC);
        bytes.extend_from_slice(&(count as u32).to_le_bytes());
        for parameter_id in 0..count {
            // SAFETY: The getter was validated at load and receives a bounded
            // parameter index from the plugin's own count.
            let value = unsafe {
                ((*self.effect)
                    .get_parameter
                    .expect("validated parameter getter"))(
                    self.effect, parameter_id as i32
                )
            };
            let value = if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.0
            };
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn restore_parameter_bank(&mut self, bytes: &[u8]) -> Result<(), Vst2LibraryError> {
        let body = &bytes[PARAMETER_BANK_MAGIC.len()..];
        let count = body
            .get(..4)
            .map(|raw| u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize)
            .ok_or(Vst2LibraryError::InvalidState)?;
        let values = &body[4..];
        if values.len() != count.checked_mul(4).ok_or(Vst2LibraryError::InvalidState)? {
            return Err(Vst2LibraryError::InvalidState);
        }
        // A plugin update may add or remove parameters; restore the overlap.
        for (parameter_id, raw) in values
            .chunks_exact(4)
            .take(self.parameter_count())
            .enumerate()
        {
            let value = f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
            if value.is_finite() {
                self.set_parameter(parameter_id as u32, value.clamp(0.0, 1.0))?;
            }
        }
        Ok(())
    }

    pub fn save_state(&mut self) -> Result<Vec<u8>, Vst2LibraryError> {
        if !self.supports_state() {
            return Ok(self.save_parameter_bank());
        }
        let mut data: *mut c_void = ptr::null_mut();
        // SAFETY: The dispatcher was validated at load. VST2 writes a pointer
        // to plugin-owned state and returns its byte count; we copy it before
        // returning, so no plugin allocation crosses the worker boundary.
        let size = unsafe {
            (*self.effect).dispatcher.expect("validated dispatcher")(
                self.effect,
                EFF_GET_CHUNK,
                Vst2ChunkScope::Program.dispatcher_index(),
                0,
                (&mut data as *mut *mut c_void).cast(),
                0.0,
            )
        };
        if !(0..=MAX_VST2_STATE_BYTES as isize).contains(&size) || (size > 0 && data.is_null()) {
            return Err(if size > MAX_VST2_STATE_BYTES as isize {
                Vst2LibraryError::StateTooLarge
            } else {
                Vst2LibraryError::InvalidState
            });
        }
        if size == 0 {
            // Some chunk-capable plugins (ReaJS before a script is selected)
            // have no chunk yet. Preserve their current parameter bank instead
            // of sending an invalid empty asset or terminating the worker.
            return Ok(self.save_parameter_bank());
        }
        // SAFETY: The plugin reported a positive bounded byte count and a
        // non-null pointer valid for the returned chunk during this copy.
        Ok(unsafe { slice::from_raw_parts(data.cast::<u8>(), size as usize) }.to_vec())
    }

    pub fn restore_state(&mut self, bytes: &[u8]) -> Result<(), Vst2LibraryError> {
        if bytes.len() > MAX_VST2_STATE_BYTES {
            return Err(Vst2LibraryError::StateTooLarge);
        }
        if bytes.starts_with(PARAMETER_BANK_MAGIC) {
            return self.restore_parameter_bank(bytes);
        }
        self.restore_chunk(bytes, Vst2ChunkScope::Program)
    }

    /// Apply an opaque program (`index=1`) or bank (`index=0`) chunk through
    /// the VST2 ABI. The caller keeps the checked byte slice alive for the
    /// synchronous dispatcher call.
    fn restore_chunk(
        &mut self,
        bytes: &[u8],
        scope: Vst2ChunkScope,
    ) -> Result<(), Vst2LibraryError> {
        if bytes.len() > MAX_VST2_STATE_BYTES {
            return Err(Vst2LibraryError::StateTooLarge);
        }
        if !self.supports_state() {
            return Err(Vst2LibraryError::StateUnsupported);
        }
        // SAFETY: The dispatcher was validated at load. The byte slice remains
        // alive for the synchronous call and VST2 consumes it during the call.
        let result = unsafe {
            (*self.effect).dispatcher.expect("validated dispatcher")(
                self.effect,
                EFF_SET_CHUNK,
                scope.dispatcher_index(),
                bytes.len() as isize,
                bytes.as_ptr().cast_mut().cast(),
                0.0,
            )
        };
        if result < 0 {
            return Err(Vst2LibraryError::InvalidState);
        }
        Ok(())
    }

    /// Apply a parsed `.fxp`/`.fxb` body to the open plugin. Parameter programs
    /// set the overlapping normalized values; chunk programs use the plugin's own
    /// chunk interface; banks apply each program onto an existing program slot,
    /// bounded by the plugin's own count.
    pub fn apply_preset(
        &mut self,
        preset: &crate::vst2_preset::Vst2Preset,
    ) -> Result<(), Vst2LibraryError> {
        // SAFETY: this handle was validated at load, so its identity is
        // readable for its lifetime.
        let unique_id = unsafe { (*self.effect).unique_id };
        if preset.fx_id != 0 && preset.fx_id != u32::from_ne_bytes(unique_id.to_ne_bytes()) {
            return Err(Vst2LibraryError::InvalidState);
        }
        // Preset application is a live mutation. Capture the previous state
        // first and roll it back if any program/chunk operation fails, so a
        // malformed plugin response cannot leave a half-applied bank.
        let previous_program = self.current_program().ok();
        let before = self.save_state()?;
        let result = self.apply_preset_body(&preset.body);
        if let Err(error) = result {
            let program_restored =
                previous_program.is_none_or(|index| self.set_program(index).is_ok());
            if !program_restored || self.restore_state(&before).is_err() {
                return Err(Vst2LibraryError::InvalidState);
            }
            return Err(error);
        }
        Ok(())
    }

    fn apply_preset_body(
        &mut self,
        body: &crate::vst2_preset::Vst2PresetBody,
    ) -> Result<(), Vst2LibraryError> {
        match body {
            crate::vst2_preset::Vst2PresetBody::ProgramParameters { parameters, .. } => {
                self.apply_parameter_values(parameters)
            }
            crate::vst2_preset::Vst2PresetBody::ProgramChunk { chunk, .. } => {
                self.restore_chunk(chunk, Vst2ChunkScope::Program)
            }
            crate::vst2_preset::Vst2PresetBody::Bank {
                current_program,
                programs,
            } => {
                if self.program_count() == 0 || programs.len() > self.program_count() {
                    return Err(Vst2LibraryError::InvalidState);
                }
                let count = self.program_count().min(programs.len());
                for (index, program) in programs.iter().take(count).enumerate() {
                    self.set_program(index as u32)?;
                    match program {
                        crate::vst2_preset::Vst2BankProgram::Parameters { parameters, .. } => {
                            self.apply_parameter_values(parameters)?;
                        }
                        crate::vst2_preset::Vst2BankProgram::Chunk { chunk, .. } => {
                            self.restore_chunk(chunk, Vst2ChunkScope::Program)?
                        }
                    }
                }
                self.set_program(*current_program)?;
                Ok(())
            }
            crate::vst2_preset::Vst2PresetBody::BankChunk { chunk, .. } => {
                self.restore_chunk(chunk, Vst2ChunkScope::Bank)
            }
        }
    }

    fn apply_parameter_values(&mut self, values: &[f32]) -> Result<(), Vst2LibraryError> {
        for (index, value) in values.iter().take(self.parameter_count()).enumerate() {
            // A preset may come from another revision of the plugin; values
            // are already bounded and normalization is the VST2 contract.
            self.set_parameter(index as u32, value.clamp(0.0, 1.0))?;
        }
        Ok(())
    }

    pub fn supports_state(&self) -> bool {
        // SAFETY: This method is only available for a successfully validated
        // handle, so the flags field is readable for its lifetime.
        unsafe { (*self.effect).flags & VST2_FLAG_PROGRAM_CHUNKS != 0 }
    }

    pub fn has_editor(&self) -> bool {
        // SAFETY: This method is only available for a successfully validated
        // handle, so the flags field is readable for its lifetime.
        unsafe { (*self.effect).flags & VST2_FLAG_HAS_EDITOR != 0 }
    }

    pub fn editor_descriptor(&mut self) -> Result<crate::EditorDescriptor, Vst2LibraryError> {
        if !self.has_editor() {
            return crate::EditorDescriptor::new(false, 0, 0)
                .map_err(|_| Vst2LibraryError::InvalidEditor);
        }
        let mut rect: *mut Vst2EditorRect = ptr::null_mut();
        // SAFETY: The dispatcher was validated at load. The plugin writes an
        // optional pointer to its own rectangle, which is copied immediately.
        let result = unsafe {
            (*self.effect).dispatcher.expect("validated dispatcher")(
                self.effect,
                EFF_EDIT_GET_RECT,
                0,
                0,
                (&mut rect as *mut *mut Vst2EditorRect).cast(),
                0.0,
            )
        };
        if result == 0 || rect.is_null() {
            return crate::EditorDescriptor::new(true, 0, 0)
                .map_err(|_| Vst2LibraryError::InvalidEditor);
        }
        // SAFETY: A successful effEditGetRect result points to the plugin's
        // live ERect for the duration of the synchronous call.
        let rect = unsafe { *rect };
        let width = i32::from(rect.right) - i32::from(rect.left);
        let height = i32::from(rect.bottom) - i32::from(rect.top);
        if !(1..=4096).contains(&width) || !(1..=4096).contains(&height) {
            return Err(Vst2LibraryError::InvalidEditor);
        }
        crate::EditorDescriptor::new(true, width as u16, height as u16)
            .map_err(|_| Vst2LibraryError::InvalidEditor)
    }

    /// Open the native editor with a caller-owned parent window. This must be
    /// invoked only on the worker's dedicated Windows UI thread; this method
    /// does not create a window, validate cross-process authorization, or run a
    /// message pump.
    pub fn open_editor(
        &mut self,
        parent_window: usize,
        owner_process_id: u32,
    ) -> Result<(), Vst2LibraryError> {
        if !self.has_editor()
            || parent_window == 0
            || owner_process_id == 0
            || !is_window_handle(parent_window)
            || window_owner_process_id(parent_window) != Some(owner_process_id)
            || self.editor_open
        {
            return Err(Vst2LibraryError::InvalidEditor);
        }
        // SAFETY: The effect is valid for this library lifetime, and the
        // parent handle is supplied by the future authorized UI host. The
        // caller owns the UI-thread/message-pump invariant.
        let result = unsafe {
            (*self.effect).dispatcher.expect("validated dispatcher")(
                self.effect,
                EFF_EDIT_OPEN,
                0,
                0,
                parent_window as *mut c_void,
                0.0,
            )
        };
        if result < 0 {
            return Err(Vst2LibraryError::InvalidEditor);
        }
        self.editor_open = true;
        Ok(())
    }

    /// Close an editor previously opened on the owning UI thread.
    pub fn close_editor(&mut self) -> Result<(), Vst2LibraryError> {
        if !self.editor_open {
            return Err(Vst2LibraryError::InvalidEditor);
        }
        // SAFETY: The effect and editor state are valid, and this call is
        // restricted to the owning UI thread by the caller.
        unsafe {
            (*self.effect).dispatcher.expect("validated dispatcher")(
                self.effect,
                EFF_EDIT_CLOSE,
                0,
                0,
                ptr::null_mut(),
                0.0,
            );
        }
        self.editor_open = false;
        Ok(())
    }

    /// Run one bounded native-editor idle tick on the owning UI thread.
    pub fn idle_editor(&mut self) -> Result<(), Vst2LibraryError> {
        if !self.editor_open {
            return Err(Vst2LibraryError::InvalidEditor);
        }
        // SAFETY: The effect and editor state are valid, and this call is
        // restricted to the owning UI thread by the caller.
        unsafe {
            (*self.effect).dispatcher.expect("validated dispatcher")(
                self.effect,
                EFF_EDIT_IDLE,
                0,
                0,
                ptr::null_mut(),
                0.0,
            );
        }
        Ok(())
    }

    pub fn latency(&self, sample_rate_hz: u32) -> Result<crate::WorkerLatency, Vst2LibraryError> {
        // SAFETY: This method is only available for a successfully validated
        // handle, so the initial-delay field is readable for its lifetime.
        let samples = unsafe { (*self.effect).initial_delay };
        if samples < 0 {
            return Err(Vst2LibraryError::InvalidState);
        }
        crate::WorkerLatency::new(samples as u32, sample_rate_hz)
            .map_err(|_| Vst2LibraryError::InvalidState)
    }

    /// Acts on `audioMasterIOChanged`: a plugin that changed its latency or
    /// pins asks the host to suspend and resume it. Cycles the mains switch
    /// (with stop/start process around it) once per request and reports
    /// whether it did, so the caller can re-read the latency. Pin counts are
    /// not renegotiated: a changed layout is caught by the channel checks.
    pub fn apply_io_change(&mut self) -> bool {
        let requested = self
            .host_context
            .io_changed
            .swap(false, std::sync::atomic::Ordering::Relaxed);
        if !requested || self.processing_format.is_none() {
            return false;
        }
        // SAFETY: the effect is owned by this handle and the call is made on
        // the worker control thread, like the other setup opcodes.
        unsafe {
            let dispatcher = (*self.effect).dispatcher.expect("validated dispatcher");
            dispatcher(self.effect, EFF_STOP_PROCESS, 0, 0, ptr::null_mut(), 0.0);
            dispatcher(self.effect, EFF_MAINS_CHANGED, 0, 0, ptr::null_mut(), 0.0);
            dispatcher(self.effect, EFF_MAINS_CHANGED, 0, 1, ptr::null_mut(), 0.0);
            dispatcher(self.effect, EFF_START_PROCESS, 0, 0, ptr::null_mut(), 0.0);
        }
        true
    }

    /// Delivers the MIDI events whose offsets lie in `start..end` to the
    /// plugin (`effProcessEvents`), offsets relative to `start`, just before
    /// the segment of the block that begins there is processed. Nothing is
    /// sent when there are no such events.
    pub fn send_midi(
        &mut self,
        events: &[crate::MidiEvent],
        start: usize,
        end: usize,
    ) -> Result<(), Vst2LibraryError> {
        let Some(mut block) = Vst2EventBlock::from_range(events, start, end) else {
            return Ok(());
        };
        // SAFETY: the validated dispatcher takes a VstEvents pointer for
        // effProcessEvents; `block` owns the header words and every event and
        // outlives the call, and the plugin must not keep the pointer.
        unsafe {
            let dispatcher = (*self.effect).dispatcher.expect("validated dispatcher");
            dispatcher(
                self.effect,
                EFF_PROCESS_EVENTS,
                0,
                0,
                block.as_pointer(),
                0.0,
            );
        }
        Ok(())
    }

    /// Process one bounded block on the worker thread. The slices are fixed
    /// caller-owned buffers; this method does not allocate or touch the audio
    /// engine's realtime callback.
    pub fn process_replacing(
        &mut self,
        inputs: &[&[f32]],
        outputs: &mut [&mut [f32]],
    ) -> Result<(), Vst2LibraryError> {
        // An instrument may have no audio inputs; the block length is then
        // the output length.
        let frames = outputs.first().map_or(0, |channel| channel.len());
        if inputs.len() != self.input_channels()
            || outputs.len() != self.output_channels()
            || inputs.len() > VST2_MAX_INPUT_CHANNELS as usize
            || (inputs.is_empty() && self.role == Vst2Role::Effect)
            || !(1..=VST2_MAX_AUDIO_CHANNELS as usize).contains(&outputs.len())
            || frames == 0
            || frames > crate::MAX_WORKER_FRAMES
            || inputs.iter().any(|channel| channel.len() != frames)
            || outputs.iter().any(|channel| channel.len() != frames)
        {
            return Err(Vst2LibraryError::InvalidPath);
        }
        let mut input_ptrs: [*const f32; VST2_MAX_INPUT_CHANNELS as usize] =
            [ptr::null(); VST2_MAX_INPUT_CHANNELS as usize];
        let mut output_ptrs: [*mut f32; VST2_MAX_AUDIO_CHANNELS as usize] =
            [ptr::null_mut(); VST2_MAX_AUDIO_CHANNELS as usize];
        for (index, channel) in inputs.iter().enumerate() {
            input_ptrs[index] = channel.as_ptr();
        }
        for (index, channel) in outputs.iter_mut().enumerate() {
            output_ptrs[index] = channel.as_mut_ptr();
        }
        // SAFETY: All pointers refer to caller-owned slices with equal,
        // non-empty lengths; channel count is bounded by the validated effect.
        unsafe {
            let process = (*self.effect)
                .process_replacing
                .expect("validated replacing processor");
            IN_PROCESS_CALL.store(true, std::sync::atomic::Ordering::Relaxed);
            process(
                self.effect,
                input_ptrs.as_ptr(),
                output_ptrs.as_mut_ptr(),
                frames as i32,
            );
            IN_PROCESS_CALL.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        self.host_context
            .sample_position
            .fetch_add(frames as u64, std::sync::atomic::Ordering::Relaxed);
        if outputs
            .iter()
            .flat_map(|channel| channel.iter())
            .any(|sample| !sample.is_finite())
        {
            return Err(Vst2LibraryError::NonFiniteOutput);
        }
        Ok(())
    }
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct Vst2EditorRect {
    top: i16,
    left: i16,
    bottom: i16,
    right: i16,
}

#[cfg(windows)]
impl Drop for Vst2Library {
    fn drop(&mut self) {
        // SAFETY: Drop runs on the owning worker thread; the effect and module
        // were created together and no caller retains their raw pointers.
        unsafe {
            if !self.effect.is_null() {
                if let Some(dispatcher) = (*self.effect).dispatcher {
                    if self.editor_open {
                        dispatcher(self.effect, EFF_EDIT_CLOSE, 0, 0, ptr::null_mut(), 0.0);
                    }
                    if self.processing_format.is_some() {
                        dispatcher(self.effect, EFF_STOP_PROCESS, 0, 0, ptr::null_mut(), 0.0);
                    }
                    dispatcher(self.effect, EFF_MAINS_CHANGED, 0, 0, ptr::null_mut(), 0.0);
                    dispatcher(self.effect, EFF_CLOSE, 0, 0, ptr::null_mut(), 0.0);
                }
            }
            if !self.module.is_null() {
                free_library(self.module);
            }
        }
    }
}

#[cfg(windows)]
unsafe extern "C" fn host_callback(
    effect: *mut Vst2Effect,
    opcode: i32,
    index: i32,
    value: isize,
    pointer: *mut c_void,
    _: f32,
) -> isize {
    // SAFETY: a non-null effect reaching this callback is the plugin's live
    // AEffect; see `host_context_of` for its host field.
    let context = unsafe { host_context_of(effect) };
    match opcode {
        AUDIO_MASTER_VERSION => 2400,
        AUDIO_MASTER_SIZE_WINDOW => {
            // The plugin's editor changed size (index: width, value: height).
            let parent = context.map_or(0, |context| {
                context
                    .editor_parent
                    .load(std::sync::atomic::Ordering::Acquire)
            });
            let owner = context.map_or(0, |context| {
                context
                    .editor_parent_process_id
                    .load(std::sync::atomic::Ordering::Acquire)
            });
            match (i32::try_from(value), parent, owner) {
                (Ok(height), parent, owner) if parent != 0 && owner != 0 => {
                    isize::from(fit_parent_window(parent, owner, index, height))
                }
                _ => 0,
            }
        }
        7 => {
            // audioMasterGetTime: ABI-owned per-thread snapshot, valid until
            // this thread's next call. The position is shared atomically with
            // the editor; no mutable timing storage is shared between threads.
            HOST_TIME
                .try_with(|time| {
                    // SAFETY: this thread exclusively owns its TLS snapshot. The
                    // returned pointer remains stable for the VST2 time-info
                    // borrow.
                    unsafe {
                        let info = &mut *time.get();
                        info.sample_rate = context.map_or(48_000.0, |c| {
                            f64::from(f32::from_bits(
                                c.sample_rate_bits
                                    .load(std::sync::atomic::Ordering::Relaxed),
                            ))
                        });
                        info.sample_pos = context.map_or(0.0, |c| {
                            c.sample_position.load(std::sync::atomic::Ordering::Relaxed) as f64
                        });
                        info.ppq_pos = info.sample_pos / info.sample_rate * 2.0;
                        info.bar_start_pos = (info.ppq_pos / 4.0).floor() * 4.0;
                        time.get() as isize
                    }
                })
                .unwrap_or(0)
        }
        // audioMasterGetCurrentProcessLevel: 2 (realtime) only inside a process
        // call, 1 (user thread) otherwise. Setup, state and editor calls share
        // the worker thread, and a plugin told "realtime" there may defer or
        // skip its interface work.
        23 => {
            if IN_PROCESS_CALL.load(std::sync::atomic::Ordering::Relaxed) {
                2
            } else {
                1
            }
        }
        34 => 10000, // audioMasterGetVendorVersion.
        // Notifications and questions a conforming host answers. Unanswered
        // (0) is "not supported", which several plugins treat as a reason to
        // skip features or to assume 44.1 kHz.
        AUDIO_MASTER_AUTOMATE | AUDIO_MASTER_IDLE | AUDIO_MASTER_NEED_IDLE => 0,
        AUDIO_MASTER_CURRENT_ID => SHELL_CURRENT_ID.with(std::cell::Cell::get) as isize,
        // 0 means "connected" for the deprecated pin query.
        AUDIO_MASTER_PIN_CONNECTED => 0,
        AUDIO_MASTER_WANT_MIDI => 1,
        // Effects send no events back to the host.
        AUDIO_MASTER_PROCESS_EVENTS => 0,
        AUDIO_MASTER_IO_CHANGED => {
            if let Some(context) = context {
                context
                    .io_changed
                    .store(true, std::sync::atomic::Ordering::Release);
            }
            1
        }
        AUDIO_MASTER_GET_INPUT_LATENCY | AUDIO_MASTER_GET_OUTPUT_LATENCY => 0,
        // 1: the host replaces the output buffer (processReplacing).
        AUDIO_MASTER_WILL_REPLACE_OR_ACCUMULATE => 1,
        // 0: automation state is unsupported (not "off").
        AUDIO_MASTER_GET_AUTOMATION_STATE => 0,
        AUDIO_MASTER_GET_VENDOR_STRING | AUDIO_MASTER_GET_PRODUCT_STRING => {
            // SAFETY: the VST2 ABI gives a writable buffer of at least 64
            // bytes (kVstMaxVendorStrLen/ProductStrLen) when it is non-null.
            unsafe { write_host_string(pointer, "AudioRouter") }
        }
        AUDIO_MASTER_CAN_DO => {
            // SAFETY: a canDo query passes a NUL-terminated ASCII string.
            let text = unsafe { read_host_string(pointer) };
            host_can_do(&text)
        }
        // English; this host shows no plugin UI text of its own.
        AUDIO_MASTER_GET_LANGUAGE => 1,
        AUDIO_MASTER_UPDATE_DISPLAY | AUDIO_MASTER_BEGIN_EDIT | AUDIO_MASTER_END_EDIT => 0,
        AUDIO_MASTER_GET_SAMPLE_RATE => context.map_or(48_000, |context| {
            f32::from_bits(
                context
                    .sample_rate_bits
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .round() as isize
        }),
        AUDIO_MASTER_GET_BLOCK_SIZE => context.map_or(128, |context| {
            context
                .block_size
                .load(std::sync::atomic::Ordering::Relaxed) as isize
        }),
        _ => 0,
    }
}

/// Capability answer for `audioMasterCanDo`: 1 yes, -1 no, 0 unknown.
#[cfg(windows)]
fn host_can_do(capability: &str) -> isize {
    match capability {
        "sendVstTimeInfo" | "sizeWindow" | "acceptIOChanges" | "startStopProcess"
        | "supplyIdle" | "sendVstEvents" | "sendVstMidiEvent" => 1,
        // This host never sends these.
        "receiveVstEvents"
        | "receiveVstMidiEvent"
        | "reportConnectionChanges"
        | "offline"
        | "openFileSelector"
        | "closeFileSelector"
        | "editFile"
        | "shellCategory"
        | "asyncProcessing" => -1,
        _ => 0,
    }
}

/// Copy `text` and a NUL into a plugin buffer of at least 64 bytes.
///
/// # Safety
/// `pointer` is null or points to at least 64 writable bytes.
#[cfg(windows)]
unsafe fn write_host_string(pointer: *mut c_void, text: &str) -> isize {
    if pointer.is_null() {
        return 0;
    }
    let bytes = text.as_bytes();
    let length = bytes.len().min(63);
    // SAFETY: the caller guarantees 64 writable bytes; length + 1 <= 64.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), length);
        *pointer.cast::<u8>().add(length) = 0;
    }
    1
}

/// Read a short NUL-terminated string a plugin passed, bounded to 64 bytes.
///
/// # Safety
/// `pointer` is null or points to a NUL-terminated string.
#[cfg(windows)]
unsafe fn read_host_string(pointer: *mut c_void) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut bytes = Vec::new();
    for index in 0..64 {
        // SAFETY: reading up to the NUL terminator the caller guarantees.
        let byte = unsafe { *pointer.cast::<u8>().add(index) };
        if byte == 0 {
            break;
        }
        bytes.push(byte);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The host context of an effect, when it has one yet.
///
/// # Safety
/// `effect` is null or points to a readable `AEffect`. Its `user` field is
/// null (during the plugin's entry point, before `Vst2Library::load` sets it)
/// or points to the `Vst2HostContext` that the owning `Vst2Library` keeps
/// alive until after effClose and module unload.
#[cfg(windows)]
unsafe fn host_context_of<'a>(effect: *mut Vst2Effect) -> Option<&'a Vst2HostContext> {
    // SAFETY: see the function contract above.
    unsafe {
        if effect.is_null() || (*effect).user.is_null() {
            None
        } else {
            Some(&*((*effect).user as *const Vst2HostContext))
        }
    }
}

/// Resize the shell's editor window so its client area is `width` × `height`
/// pixels, the size of the plugin's editor. The request is posted
/// (`SWP_ASYNCWINDOWPOS`): the window belongs to another process whose
/// thread may be busy. Returns whether the request was made.
#[cfg(windows)]
fn fit_parent_window(parent: usize, owner_process_id: u32, width: i32, height: i32) -> bool {
    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_ASYNCWINDOWPOS: u32 = 0x4000;
    if !(1..=MAX_EDITOR_SIDE).contains(&width)
        || !(1..=MAX_EDITOR_SIDE).contains(&height)
        || !is_window_handle(parent)
        || owner_process_id == 0
        || window_owner_process_id(parent) != Some(owner_process_id)
    {
        return false;
    }
    let window = parent as *mut c_void;
    let mut frame = NativeRect {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    };
    // SAFETY: the handle was just checked and is only queried and asked to
    // resize; `frame` is a live local RECT for the synchronous call.
    unsafe {
        let style = window_long(window, GWL_STYLE) as u32;
        let extended = window_long(window, GWL_EXSTYLE) as u32;
        let dpi = match GetDpiForWindow(window) {
            0 => 96,
            dpi => dpi,
        };
        if AdjustWindowRectExForDpi(&mut frame, style, 0, extended, dpi) == 0 {
            return false;
        }
        SetWindowPos(
            window,
            ptr::null_mut(),
            0,
            0,
            frame.right - frame.left,
            frame.bottom - frame.top,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
        ) != 0
    }
}

/// Make this worker process draw in real pixels, like the desktop shell whose
/// window hosts the editor. Without it Windows treats the plugin's window as
/// 96 DPI inside a per-monitor window, and editors come out the wrong size
/// on a scaled display. Call once, before any plugin is loaded.
#[cfg(windows)]
pub fn enable_per_monitor_dpi_awareness() {
    // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2.
    const PER_MONITOR_AWARE_V2: isize = -4;
    // SAFETY: a process-wide setting with no pointers; failure (already set,
    // or an older Windows) leaves the previous mode and is not an error.
    unsafe {
        SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2);
    }
}

/// Set while a plugin's process call runs (see opcode 23 above).
#[cfg(windows)]
static IN_PROCESS_CALL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
#[repr(C)]
struct Vst2TimeInfo {
    sample_pos: f64,
    sample_rate: f64,
    nano_seconds: f64,
    ppq_pos: f64,
    tempo: f64,
    bar_start_pos: f64,
    cycle_start_pos: f64,
    cycle_end_pos: f64,
    time_sig_numerator: i32,
    time_sig_denominator: i32,
    smpte_offset: i32,
    smpte_frame_rate: i32,
    samples_to_next_clock: i32,
    flags: i32,
}

#[cfg(windows)]
thread_local! {
    static HOST_TIME: std::cell::UnsafeCell<Vst2TimeInfo> = const { std::cell::UnsafeCell::new(Vst2TimeInfo {
        sample_pos: 0.0, sample_rate: 48_000.0, nano_seconds: 0.0, ppq_pos: 0.0,
        tempo: 120.0, bar_start_pos: 0.0, cycle_start_pos: 0.0, cycle_end_pos: 0.0,
        time_sig_numerator: 4, time_sig_denominator: 4, smpte_offset: 0, smpte_frame_rate: 0,
        samples_to_next_clock: 0, flags: 2 | 512 | 1024 | 2048 | 8192,
    }) };
}

/// Service windows created by a plugin while it was loaded on the worker's
/// owner thread, including licensing windows, even while the host is idle.
#[cfg(windows)]
pub fn pump_owner_messages() {
    // SAFETY: Win32 fills a correctly sized local MSG; dispatch is on the
    // window's owning thread, with no borrowed plugin buffers outstanding.
    unsafe {
        let mut message: NativeMessage = std::mem::zeroed();
        while PeekMessageW(&mut message, ptr::null_mut(), 0, 0, 1) != 0 {
            if message.message == 0x0012 {
                continue;
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

#[cfg(windows)]
/// # Safety
/// `window` must be a handle that may be queried (a null or stale handle
/// makes the call return 0).
#[cfg(windows)]
unsafe fn window_long(window: *mut c_void, index: i32) -> isize {
    #[cfg(target_pointer_width = "64")]
    // SAFETY: forwarded caller contract; a plain query.
    unsafe {
        GetWindowLongPtrW(window, index)
    }
    #[cfg(target_pointer_width = "32")]
    // SAFETY: forwarded caller contract; a plain query.
    unsafe {
        GetWindowLongW(window, index) as isize
    }
}

#[link(name = "user32")]
unsafe extern "system" {
    fn IsWindow(window: *mut c_void) -> i32;
    fn GetWindowThreadProcessId(window: *mut c_void, process_id: *mut u32) -> u32;
    fn PeekMessageW(
        message: *mut NativeMessage,
        window: *mut c_void,
        minimum: u32,
        maximum: u32,
        remove: u32,
    ) -> i32;
    fn TranslateMessage(message: *const NativeMessage) -> i32;
    fn DispatchMessageW(message: *const NativeMessage) -> isize;
    #[cfg(target_pointer_width = "64")]
    fn GetWindowLongPtrW(window: *mut c_void, index: i32) -> isize;
    // 32-bit user32 has no GetWindowLongPtrW export (it is a macro there).
    #[cfg(target_pointer_width = "32")]
    fn GetWindowLongW(window: *mut c_void, index: i32) -> i32;
    fn GetDpiForWindow(window: *mut c_void) -> u32;
    fn AdjustWindowRectExForDpi(
        rect: *mut NativeRect,
        style: u32,
        menu: i32,
        extended_style: u32,
        dpi: u32,
    ) -> i32;
    fn SetWindowPos(
        window: *mut c_void,
        insert_after: *mut c_void,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn SetProcessDpiAwarenessContext(context: isize) -> i32;
}

/// Win32 `RECT`.
#[cfg(windows)]
#[repr(C)]
struct NativeRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(windows)]
fn is_window_handle(window: usize) -> bool {
    // SAFETY: IsWindow accepts an arbitrary HWND value and only queries
    // whether it currently identifies a window; it does not retain it.
    unsafe { IsWindow(window as *mut c_void) != 0 }
}

#[cfg(windows)]
fn window_owner_process_id(window: usize) -> Option<u32> {
    let mut process_id = 0;
    // SAFETY: The HWND is only queried; the output points to a local scalar
    // that remains valid for the synchronous Win32 call.
    let thread_id = unsafe { GetWindowThreadProcessId(window as *mut c_void, &mut process_id) };
    (thread_id != 0 && process_id != 0).then_some(process_id)
}

#[cfg(windows)]
enum EditorThreadCommand {
    Open(usize, u32),
    Close,
    Shutdown,
}

/// A handle to the editor entry points of the processing instance, for its
/// UI thread. VST2 hosts run the editor (open/idle/close) on a UI thread
/// while the audio thread processes the same instance; this is the plugin
/// ABI's normal threading model. Showing the processing instance is what
/// lets an editor meter or analyse live audio (for example ReaFIR building
/// a noise profile) and makes every edit audible immediately.
///
/// Invariant: the `Vst2Library` this came from outlives it. The worker owns
/// both, and `Vst2EditorThread` joins its thread on drop, closing the editor
/// first, before the library can be unloaded.
#[cfg(windows)]
pub struct Vst2EditorAccess {
    effect: *mut Vst2Effect,
    has_editor: bool,
}

// SAFETY: the pointer is only dereferenced on the editor thread for the
// editor opcodes, which the VST2 ABI allows concurrently with processing on
// the audio thread. The owning library outlives the access (see above).
#[cfg(windows)]
unsafe impl Send for Vst2EditorAccess {}

#[cfg(windows)]
impl Vst2Library {
    /// Editor entry points of this instance for its UI thread.
    pub fn editor_access(&self) -> Vst2EditorAccess {
        Vst2EditorAccess {
            effect: self.effect,
            has_editor: self.has_editor(),
        }
    }
}

#[cfg(windows)]
impl Vst2EditorAccess {
    fn dispatch(&self, opcode: i32, pointer: *mut c_void) -> isize {
        // SAFETY: the effect is live for the access lifetime (invariant
        // above) and this runs on the editor thread only.
        unsafe {
            (*self.effect).dispatcher.expect("validated dispatcher")(
                self.effect,
                opcode,
                0,
                0,
                pointer,
                0.0,
            )
        }
    }

    /// The editor's size in pixels, when the plugin reports a usable one.
    /// Some plugins only know it once their editor is open.
    fn editor_size(&self) -> Option<(i32, i32)> {
        let mut rect: *mut Vst2EditorRect = ptr::null_mut();
        let result = self.dispatch(
            EFF_EDIT_GET_RECT,
            (&mut rect as *mut *mut Vst2EditorRect).cast(),
        );
        if result == 0 || rect.is_null() {
            return None;
        }
        // SAFETY: a successful effEditGetRect leaves a pointer to the
        // plugin's live ERect, copied here before any other plugin call.
        let rect = unsafe { *rect };
        let width = i32::from(rect.right) - i32::from(rect.left);
        let height = i32::from(rect.bottom) - i32::from(rect.top);
        ((1..=MAX_EDITOR_SIDE).contains(&width) && (1..=MAX_EDITOR_SIDE).contains(&height))
            .then_some((width, height))
    }

    /// Tell the host callback which window the editor is in (0: none).
    fn set_editor_parent(&self, parent: usize, owner_process_id: u32) {
        // SAFETY: the effect is live for the access lifetime; see
        // `host_context_of` for its host field.
        if let Some(context) = unsafe { host_context_of(self.effect) } {
            context
                .editor_parent
                .store(parent, std::sync::atomic::Ordering::Release);
            context
                .editor_parent_process_id
                .store(owner_process_id, std::sync::atomic::Ordering::Release);
        }
    }

    /// Open the editor in `parent` and size that window to it.
    fn open_in(&self, parent: usize, owner_process_id: u32) -> bool {
        // Size the window before the editor appears where the plugin already
        // knows its size, and again after: the size can change on opening.
        let before = self.editor_size();
        if let Some((width, height)) = before {
            fit_parent_window(parent, owner_process_id, width, height);
        }
        self.set_editor_parent(parent, owner_process_id);
        if self.dispatch(EFF_EDIT_OPEN, parent as *mut c_void) < 0 {
            self.set_editor_parent(0, 0);
            return false;
        }
        let after = self.editor_size();
        if let Some((width, height)) = after.filter(|size| Some(*size) != before) {
            fit_parent_window(parent, owner_process_id, width, height);
        }
        true
    }

    fn close(&self) {
        self.dispatch(EFF_EDIT_CLOSE, ptr::null_mut());
        self.set_editor_parent(0, 0);
    }
}

/// Native-editor owner: one Windows UI thread that opens, idles and closes
/// the editor of the processing instance. Dropping it closes an open editor
/// and joins the thread, so the instance is never used after unload. The
/// disposable worker process remains the containment boundary if a
/// third-party editor never returns.
#[cfg(windows)]
pub struct Vst2EditorThread {
    commands: Sender<EditorThreadCommand>,
    thread: Option<JoinHandle<()>>,
    exited: Receiver<()>,
}

#[cfg(windows)]
impl Vst2EditorThread {
    pub fn spawn(access: Vst2EditorAccess) -> Result<Self, Vst2LibraryError> {
        let (commands, receiver) = mpsc::channel();
        let (exit_signal, exited) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("audiorouter-vst2-editor".into())
            .spawn(move || {
                editor_thread_main(access, receiver);
                let _ = exit_signal.send(());
            })
            .map_err(|_| Vst2LibraryError::InvalidEditor)?;
        Ok(Self {
            commands,
            thread: Some(thread),
            exited,
        })
    }

    /// Load the plugin on a new UI thread and keep that thread for its
    /// editor, then hand the loaded instance back for processing.
    ///
    /// Many VST2 plugins (JUCE-based ones among them) set up their window
    /// system on the thread that loads them and crash with an access
    /// violation when another thread opens their editor. Pitchproof did as
    /// soon as its window was opened (owner report 2026-11-12). Hosts load
    /// and show editors on one UI thread and process on another, which the
    /// VST2 ABI allows; this does the same.
    pub fn spawn_loading(path: &Path) -> Result<(Vst2Library, Self), Vst2LibraryError> {
        Self::spawn_loading_as(path, Vst2Role::Effect)
    }

    /// Like `spawn_loading`, for an effect or an instrument.
    pub fn spawn_loading_as(
        path: &Path,
        role: Vst2Role,
    ) -> Result<(Vst2Library, Self), Vst2LibraryError> {
        let path = path.to_path_buf();
        let (commands, receiver) = mpsc::channel();
        let (exit_signal, exited) = mpsc::channel();
        let (loaded_sender, loaded) = mpsc::channel::<Result<LoadedOnUiThread, Vst2LibraryError>>();
        let thread = thread::Builder::new()
            .name("audiorouter-vst2-editor".into())
            .spawn(move || {
                let library = match Vst2Library::load_as(&path, role) {
                    Ok(library) => library,
                    Err(error) => {
                        let _ = loaded_sender.send(Err(error));
                        let _ = exit_signal.send(());
                        return;
                    }
                };
                let access = library.editor_access();
                if loaded_sender.send(Ok(LoadedOnUiThread(library))).is_ok() {
                    editor_thread_main(access, receiver);
                }
                let _ = exit_signal.send(());
            })
            .map_err(|_| Vst2LibraryError::InvalidEditor)?;
        let editor = Self {
            commands,
            thread: Some(thread),
            exited,
        };
        match loaded.recv() {
            Ok(Ok(LoadedOnUiThread(library))) => Ok((library, editor)),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(Vst2LibraryError::InvalidEditor),
        }
    }

    /// Validate the request and ask the UI thread to open the editor. It
    /// does not wait for the plugin to build its window: the worker keeps
    /// processing audio meanwhile (waiting here dropped audio blocks).
    pub fn open(
        &self,
        parent_window: usize,
        owner_process_id: u32,
        authorization_token: &str,
    ) -> Result<(), String> {
        if authorization_token.is_empty() {
            return Err("editor authorization token is missing".into());
        }
        if !is_window_handle(parent_window)
            || window_owner_process_id(parent_window) != Some(owner_process_id)
        {
            return Err("parent window is not valid".into());
        }
        self.commands
            .send(EditorThreadCommand::Open(parent_window, owner_process_id))
            .map_err(|_| "editor UI thread stopped".to_string())
    }

    /// Ask the UI thread to close the editor. It does not wait: the worker
    /// keeps processing audio while the plugin takes its editor down. The
    /// desktop shell keeps its window alive until the editor has left it
    /// (`plugin_editor_windows.rs`); destroying that window under a plugin
    /// that was still closing crashed plugins (issue #136). Edits already
    /// live in the processing instance, so nothing is copied back.
    pub fn close(&self) -> Result<(), String> {
        self.commands
            .send(EditorThreadCommand::Close)
            .map_err(|_| "editor UI thread stopped".to_string())
    }
}

/// A plugin loaded on its UI thread, on its way to the processing thread.
#[cfg(windows)]
struct LoadedOnUiThread(Vst2Library);

// SAFETY: ownership moves once, right after loading, from the UI thread to
// the worker thread that processes it; the UI thread keeps only the editor
// access, whose editor opcodes the VST2 ABI allows concurrently with
// processing (see `Vst2EditorAccess`). No other state is shared.
#[cfg(windows)]
unsafe impl Send for LoadedOnUiThread {}

/// How long dropping an editor thread waits for the editor to close.
#[cfg(windows)]
pub const EDITOR_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

#[cfg(windows)]
impl Drop for Vst2EditorThread {
    fn drop(&mut self) {
        let _ = self.commands.send(EditorThreadCommand::Shutdown);
        // Join once the editor has closed. An editor stuck inside the plugin
        // (for example a parent window whose thread never pumps messages) is
        // left behind after a bounded wait: this only happens as the
        // disposable worker exits, and the process ends with it.
        if self.exited.recv_timeout(EDITOR_SHUTDOWN_TIMEOUT).is_ok() {
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

#[cfg(windows)]
fn editor_thread_main(access: Vst2EditorAccess, receiver: Receiver<EditorThreadCommand>) {
    // The shell window the editor is open in.
    let mut open_in: Option<usize> = None;
    loop {
        pump_editor_messages();
        match receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(EditorThreadCommand::Open(parent, owner_pid)) => {
                if access.has_editor
                    && open_in.is_none()
                    && is_window_handle(parent)
                    && window_owner_process_id(parent) == Some(owner_pid)
                    && access.open_in(parent, owner_pid)
                {
                    open_in = Some(parent);
                }
            }
            Ok(EditorThreadCommand::Close) => {
                if open_in.take().is_some() {
                    access.close();
                }
            }
            Ok(EditorThreadCommand::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                if open_in.is_some() {
                    access.close();
                }
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        match open_in {
            // The shell's window went away without a close request (the
            // shell ended). Let the plugin drop its editor instead of idling
            // on a window that no longer exists.
            Some(parent) if !is_window_handle(parent) => {
                access.close();
                open_in = None;
            }
            Some(_) => {
                access.dispatch(EFF_EDIT_IDLE, ptr::null_mut());
            }
            None => {}
        }
    }
}

#[cfg(windows)]
fn pump_editor_messages() {
    let mut message = NativeMessage::default();
    // SAFETY: `message` is a live, writable `#[repr(C)]` MSG; a null window
    // filter reads messages for this (the editor) thread, and PM_REMOVE (1)
    // takes each one off the queue.
    while unsafe { PeekMessageW(&mut message, ptr::null_mut(), 0, 0, 1) } != 0 {
        // SAFETY: `message` was just filled by PeekMessageW; both calls only read it
        // and dispatch to window procedures owned by this thread.
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct NativeMessage {
    window: *mut c_void,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    point_x: i32,
    point_y: i32,
    private: u32,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryExW(name: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const i8) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
    fn GetLastError() -> u32;
}

#[cfg(windows)]
unsafe fn load_library_w(name: *const u16) -> *mut c_void {
    // SAFETY: Caller supplies a valid NUL-terminated UTF-16 path.
    // Search the verified plugin's directory and Windows default safe DLL
    // locations, never the backend's working directory.
    unsafe { LoadLibraryExW(name, ptr::null_mut(), 0x100 | 0x1000) }
}

#[cfg(windows)]
unsafe fn get_proc_address(module: *mut c_void, name: *const i8) -> Option<*mut c_void> {
    // SAFETY: Caller supplies a valid module handle and NUL-terminated name.
    let address = unsafe { GetProcAddress(module, name) };
    (!address.is_null()).then_some(address)
}

#[cfg(windows)]
unsafe fn free_library(module: *mut c_void) {
    // SAFETY: Caller owns a live module handle and calls this exactly once.
    unsafe { FreeLibrary(module) };
}

#[cfg(windows)]
fn last_error() -> u32 {
    // SAFETY: GetLastError has no preconditions and reads the current thread's
    // Win32 error value immediately after the failed loader call.
    unsafe { GetLastError() }
}
