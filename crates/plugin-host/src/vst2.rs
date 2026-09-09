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

/// The VST2 `AEffect` magic (`'VstP'`) in little-endian form.
pub const VST2_EFFECT_MAGIC: i32 = 0x5673_7450;
/// `effFlagsCanReplacing`: the effect accepts the replacing process callback.
pub const VST2_FLAG_CAN_REPLACING: i32 = 1 << 4;
/// `effFlagsProgramChunks`: the plugin exposes opaque chunk state.
pub const VST2_FLAG_PROGRAM_CHUNKS: i32 = 1 << 5;
/// `effFlagsHasEditor`: the plugin exposes a native editor.
pub const VST2_FLAG_HAS_EDITOR: i32 = 1 << 0;
pub const VST2_MAX_AUDIO_CHANNELS: i32 = 2;
pub const VST2_MAX_INPUT_CHANNELS: i32 = 4;
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
    InvalidChannels { inputs: i32, outputs: i32 },
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
        if !(1..=VST2_MAX_INPUT_CHANNELS).contains(&self.num_inputs)
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
            Err(Vst2HeaderError::InvalidChannels {
                inputs: 2,
                outputs: 3,
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
const EFF_GET_CHUNK: i32 = 23;
#[cfg(windows)]
const EFF_SET_CHUNK: i32 = 24;
#[cfg(windows)]
const MAX_VST2_STATE_BYTES: usize = 512 * 1024;
#[cfg(windows)]
const AUDIO_MASTER_VERSION: i32 = 1;
#[cfg(windows)]
const AUDIO_MASTER_GET_SAMPLE_RATE: i32 = 10;
#[cfg(windows)]
const AUDIO_MASTER_GET_BLOCK_SIZE: i32 = 11;

#[cfg(all(test, windows))]
mod opcode_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn uses_vst2_dispatcher_opcodes_for_lifecycle_and_state() {
        assert_eq!(VST2_FLAG_HAS_EDITOR, 1);
        assert_eq!(VST2_FLAG_CAN_REPLACING, 1 << 4);
        assert_eq!(VST2_FLAG_PROGRAM_CHUNKS, 1 << 5);
        assert_eq!(EFF_SET_SAMPLE_RATE, 10);
        assert_eq!(EFF_SET_BLOCK_SIZE, 11);
        assert_eq!(EFF_MAINS_CHANGED, 12);
        assert_eq!(EFF_EDIT_GET_RECT, 13);
        assert_eq!(EFF_EDIT_OPEN, 14);
        assert_eq!(EFF_EDIT_CLOSE, 15);
        assert_eq!(EFF_EDIT_IDLE, 19);
        assert_eq!(EFF_GET_CHUNK, 23);
        assert_eq!(EFF_SET_CHUNK, 24);
    }

    static DISPATCH_COUNT: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn counting_dispatcher(
        _: *mut Vst2Effect,
        _: i32,
        _: i32,
        _: isize,
        _: *mut c_void,
        _: f32,
    ) -> isize {
        DISPATCH_COUNT.fetch_add(1, Ordering::Relaxed);
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
        };
        DISPATCH_COUNT.store(0, Ordering::Relaxed);

        library.set_processing_format(48_000.0, 128).unwrap();
        library.set_processing_format(48_000.0, 128).unwrap();
        library.set_processing_format(48_000.0, 256).unwrap();

        assert!(matches!(
            library.open_editor(1),
            Err(Vst2LibraryError::InvalidEditor)
        ));
        assert_eq!(DISPATCH_COUNT.load(Ordering::Relaxed), 7);
        assert!(matches!(
            library.close_editor(),
            Err(Vst2LibraryError::InvalidEditor)
        ));
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

#[cfg(windows)]
pub struct Vst2Library {
    module: *mut c_void,
    effect: *mut Vst2Effect,
    processing_format: Option<(u32, i32)>,
    editor_open: bool,
}

#[cfg(windows)]
impl Vst2Library {
    /// Load one already-verified native x64 VST2 DLL. This function executes
    /// the DLL's loader and VST2 entry point; callers must invoke it only from
    /// the disposable worker after identity and job-sandbox setup.
    pub fn load(path: &Path) -> Result<Self, Vst2LibraryError> {
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
        let validation = unsafe { (*effect).validate_audio_effect() };
        if let Err(error) = validation {
            // SAFETY: A valid dispatcher is required by the ABI, but invalid
            // headers cannot be trusted. Unload without invoking plugin code.
            unsafe { free_library(module) };
            return Err(Vst2LibraryError::InvalidEffect(error));
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
        Ok(Self {
            module,
            effect,
            processing_format: None,
            editor_open: false,
        })
    }

    pub fn set_processing_format(
        &mut self,
        sample_rate_hz: f32,
        block_size: i32,
    ) -> Result<(), Vst2LibraryError> {
        if !sample_rate_hz.is_finite()
            || !(1.0..=192_000.0).contains(&sample_rate_hz)
            || !(1..=2048).contains(&block_size)
        {
            return Err(Vst2LibraryError::InvalidPath);
        }
        let sample_rate_bits = sample_rate_hz.to_bits();
        if self.processing_format == Some((sample_rate_bits, block_size)) {
            return Ok(());
        }
        // SAFETY: The effect was validated at load and remains owned by this
        // handle. These setup opcodes run on the worker control thread.
        unsafe {
            let dispatcher = (*self.effect).dispatcher.expect("validated dispatcher");
            if self.processing_format.is_some() {
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
        let count = self.parameter_count();
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

    pub fn save_state(&mut self) -> Result<Vec<u8>, Vst2LibraryError> {
        if !self.supports_state() {
            return Err(Vst2LibraryError::StateUnsupported);
        }
        let mut data: *mut c_void = ptr::null_mut();
        // SAFETY: The dispatcher was validated at load. VST2 writes a pointer
        // to plugin-owned state and returns its byte count; we copy it before
        // returning, so no plugin allocation crosses the worker boundary.
        let size = unsafe {
            (*self.effect).dispatcher.expect("validated dispatcher")(
                self.effect,
                EFF_GET_CHUNK,
                0,
                1,
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
            return Ok(Vec::new());
        }
        // SAFETY: The plugin reported a positive bounded byte count and a
        // non-null pointer valid for the returned chunk during this copy.
        Ok(unsafe { slice::from_raw_parts(data.cast::<u8>(), size as usize) }.to_vec())
    }

    pub fn restore_state(&mut self, bytes: &[u8]) -> Result<(), Vst2LibraryError> {
        if !self.supports_state() {
            return Err(Vst2LibraryError::StateUnsupported);
        }
        if bytes.len() > MAX_VST2_STATE_BYTES {
            return Err(Vst2LibraryError::StateTooLarge);
        }
        // SAFETY: The dispatcher was validated at load. The byte slice remains
        // alive for the synchronous call and VST2 consumes it during the call.
        let result = unsafe {
            (*self.effect).dispatcher.expect("validated dispatcher")(
                self.effect,
                EFF_SET_CHUNK,
                0,
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
    pub fn open_editor(&mut self, parent_window: usize) -> Result<(), Vst2LibraryError> {
        if !self.has_editor()
            || parent_window == 0
            || !is_window_handle(parent_window)
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

    /// Process one bounded block on the worker thread. The slices are fixed
    /// caller-owned buffers; this method does not allocate or touch the audio
    /// engine's realtime callback.
    pub fn process_replacing(
        &mut self,
        inputs: &[&[f32]],
        outputs: &mut [&mut [f32]],
    ) -> Result<(), Vst2LibraryError> {
        if !(1..=VST2_MAX_INPUT_CHANNELS as usize).contains(&inputs.len())
            || !(1..=VST2_MAX_AUDIO_CHANNELS as usize).contains(&outputs.len())
            || inputs.iter().any(|channel| channel.is_empty())
            || inputs
                .iter()
                .any(|channel| channel.len() != inputs[0].len())
            || outputs
                .iter()
                .any(|channel| channel.len() != inputs[0].len())
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
            process(
                self.effect,
                input_ptrs.as_ptr(),
                output_ptrs.as_mut_ptr(),
                inputs[0].len() as i32,
            );
        }
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
    _: *mut Vst2Effect,
    opcode: i32,
    _: i32,
    _: isize,
    _: *mut c_void,
    _: f32,
) -> isize {
    match opcode {
        AUDIO_MASTER_VERSION => 2400,
        AUDIO_MASTER_GET_SAMPLE_RATE => 48_000,
        AUDIO_MASTER_GET_BLOCK_SIZE => 128,
        _ => 0,
    }
}

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn IsWindow(window: *mut c_void) -> i32;
    fn PeekMessageW(
        message: *mut NativeMessage,
        window: *mut c_void,
        minimum: u32,
        maximum: u32,
        remove: u32,
    ) -> i32;
    fn TranslateMessage(message: *const NativeMessage) -> i32;
    fn DispatchMessageW(message: *const NativeMessage) -> isize;
}

#[cfg(windows)]
fn is_window_handle(window: usize) -> bool {
    // SAFETY: IsWindow accepts an arbitrary HWND value and only queries
    // whether it currently identifies a window; it does not retain it.
    unsafe { IsWindow(window as *mut c_void) != 0 }
}

#[cfg(windows)]
enum EditorThreadCommand {
    Open(usize, Sender<Result<(), String>>),
    Close(Sender<Result<(), String>>),
    Shutdown,
}

/// Disposable native-editor owner. The editor instance is loaded and used
/// exclusively by its Windows UI thread, separate from the worker's audio
/// processing instance. Dropping this handle requests shutdown and detaches
/// the thread; the enclosing disposable worker remains the hard containment
/// boundary if a third-party editor does not return.
#[cfg(windows)]
pub struct Vst2EditorThread {
    commands: Sender<EditorThreadCommand>,
    _thread: JoinHandle<()>,
}

#[cfg(windows)]
impl Vst2EditorThread {
    pub fn spawn(path: &Path) -> Result<Self, Vst2LibraryError> {
        if !path.is_absolute() || !path.is_file() {
            return Err(Vst2LibraryError::InvalidPath);
        }
        let path = path.to_path_buf();
        let (commands, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("audiorouter-vst2-editor".into())
            .spawn(move || editor_thread_main(path, receiver))
            .map_err(|_| Vst2LibraryError::InvalidPath)?;
        Ok(Self {
            commands,
            _thread: thread,
        })
    }

    pub fn open(&self, parent_window: usize) -> Result<(), String> {
        let (response, receiver) = mpsc::channel();
        self.commands
            .send(EditorThreadCommand::Open(parent_window, response))
            .map_err(|_| "editor UI thread stopped".to_string())?;
        receiver
            .recv_timeout(crate::WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "editor UI thread timed out".to_string())?
    }

    pub fn close(&self) -> Result<(), String> {
        let (response, receiver) = mpsc::channel();
        self.commands
            .send(EditorThreadCommand::Close(response))
            .map_err(|_| "editor UI thread stopped".to_string())?;
        receiver
            .recv_timeout(crate::WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "editor UI thread timed out".to_string())?
    }
}

#[cfg(windows)]
impl Drop for Vst2EditorThread {
    fn drop(&mut self) {
        let _ = self.commands.send(EditorThreadCommand::Shutdown);
    }
}

#[cfg(windows)]
fn editor_thread_main(path: std::path::PathBuf, receiver: Receiver<EditorThreadCommand>) {
    let mut plugin: Option<Vst2Library> = None;
    loop {
        pump_editor_messages();
        match receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(EditorThreadCommand::Open(parent, response)) => {
                let result = if !is_window_handle(parent) {
                    Err("parent window is not valid".to_string())
                } else {
                    let plugin = match plugin.as_mut() {
                        Some(plugin) => Ok(plugin),
                        None => Vst2Library::load(&path)
                            .map(|loaded| plugin.insert(loaded))
                            .map_err(|error| format!("VST2 editor load failed: {error:?}")),
                    };
                    plugin.and_then(|plugin| {
                        plugin
                            .open_editor(parent)
                            .map_err(|error| format!("VST2 editor open failed: {error:?}"))
                    })
                };
                let _ = response.send(result);
            }
            Ok(EditorThreadCommand::Close(response)) => {
                let result = plugin
                    .as_mut()
                    .ok_or_else(|| "editor is not open".to_string())
                    .and_then(|plugin| {
                        plugin
                            .close_editor()
                            .map_err(|error| format!("VST2 editor close failed: {error:?}"))
                    });
                let _ = response.send(result);
            }
            Ok(EditorThreadCommand::Shutdown) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if let Some(plugin) = plugin.as_mut() {
            let _ = plugin.idle_editor();
        }
    }
}

#[cfg(windows)]
fn pump_editor_messages() {
    let mut message = NativeMessage::default();
    while unsafe { PeekMessageW(&mut message, ptr::null_mut(), 0, 0, 1) } != 0 {
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
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const i8) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
    fn GetLastError() -> u32;
}

#[cfg(windows)]
unsafe fn load_library_w(name: *const u16) -> *mut c_void {
    // SAFETY: Caller supplies a valid NUL-terminated UTF-16 path.
    unsafe { LoadLibraryW(name) }
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
