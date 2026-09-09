//! VST2 ABI declarations and non-executing effect-header validation.
//!
//! This module deliberately does not load a DLL, call `VSTPluginMain`, or
//! dereference an `AEffect`. The eventual Windows worker adapter must perform
//! those operations only after identity verification and worker containment.

use std::ffi::c_void;

/// The VST2 `AEffect` magic (`'VstP'`) in little-endian form.
pub const VST2_EFFECT_MAGIC: i32 = 0x5673_7450;
/// `effFlagsCanReplacing`: the effect accepts the replacing process callback.
pub const VST2_FLAG_CAN_REPLACING: i32 = 1 << 4;
pub const VST2_MAX_AUDIO_CHANNELS: i32 = 2;
pub const VST2_MAX_PARAMETERS: i32 = 256;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vst2HeaderError {
    InvalidMagic,
    MissingDispatcher,
    MissingReplacingProcessor,
    MissingParameterSetter,
    MissingParameterGetter,
    InvalidChannels,
    TooManyParameters,
    ReplacingUnsupported,
}

impl Vst2Effect {
    /// Validate only fields that can be checked without invoking plugin code.
    /// The returned header is still untrusted until the worker owns its
    /// lifetime and applies the callback/audio-buffer policy.
    pub fn validate_audio_effect(&self) -> Result<(), Vst2HeaderError> {
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
        if !(1..=VST2_MAX_AUDIO_CHANNELS).contains(&self.num_inputs)
            || !(1..=VST2_MAX_AUDIO_CHANNELS).contains(&self.num_outputs)
        {
            return Err(Vst2HeaderError::InvalidChannels);
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

#[cfg(test)]
mod tests {
    use super::*;

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
        effect.num_outputs = 3;
        assert_eq!(
            effect.validate_audio_effect(),
            Err(Vst2HeaderError::InvalidChannels)
        );
        let mut effect = valid_effect();
        effect.process_replacing = None;
        assert_eq!(
            effect.validate_audio_effect(),
            Err(Vst2HeaderError::MissingReplacingProcessor)
        );
    }
}
