//! A tiny VST2 instrument for AudioRouter's tests.
//!
//! It plays one sine voice: a note-on (`effProcessEvents`, 0x9n with a
//! velocity above zero) starts the note at the sample offset the host gave
//! (`deltaFrames`), and the matching note-off (0x8n, or 0x9n with velocity
//! zero) stops it. Output is silent otherwise, so a test can tell exactly
//! whether MIDI reached the plugin and at which sample. It declares no audio
//! inputs and `effFlagsIsSynth`, like a real synth. Parameter 0 is volume.
//!
//! It is written against the public `aeffectx.h` layout (a 32-bit-safe
//! `AEffect`, `VstEvents` and `VstMidiEvent`), with no SDK files.

use std::ffi::c_void;

const MAGIC: i32 = 0x5673_7450; // 'VstP'
const FLAG_CAN_REPLACING: i32 = 1 << 4;
const FLAG_IS_SYNTH: i32 = 1 << 8;

const EFF_OPEN: i32 = 0;
const EFF_CLOSE: i32 = 1;
const EFF_SET_SAMPLE_RATE: i32 = 10;
const EFF_PROCESS_EVENTS: i32 = 25;
const EFF_GET_PLUG_CATEGORY: i32 = 35;
const EFF_SHELL_GET_NEXT_PLUGIN: i32 = 70;
const PLUG_CATEGORY_SHELL: isize = 10;
const EFF_GET_EFFECT_NAME: i32 = 45;
const EFF_GET_VENDOR_STRING: i32 = 47;
const EFF_GET_PRODUCT_STRING: i32 = 48;
const EFF_CAN_DO: i32 = 51;
const EFF_GET_VST_VERSION: i32 = 58;
const PLUG_CATEGORY_SYNTH: isize = 2;
const MIDI_EVENT_TYPE: i32 = 1;

type Dispatcher = unsafe extern "C" fn(*mut AEffect, i32, i32, isize, *mut c_void, f32) -> isize;
type Process = unsafe extern "C" fn(*mut AEffect, *const *const f32, *mut *mut f32, i32);
type SetParameter = unsafe extern "C" fn(*mut AEffect, i32, f32);
type GetParameter = unsafe extern "C" fn(*mut AEffect, i32) -> f32;
type HostCallback = unsafe extern "C" fn(*mut AEffect, i32, i32, isize, *mut c_void, f32) -> isize;

#[repr(C)]
pub struct AEffect {
    magic: i32,
    dispatcher: Option<Dispatcher>,
    process: Option<Process>,
    set_parameter: Option<SetParameter>,
    get_parameter: Option<GetParameter>,
    num_programs: i32,
    num_params: i32,
    num_inputs: i32,
    num_outputs: i32,
    flags: i32,
    reserved_1: isize,
    reserved_2: isize,
    initial_delay: i32,
    real_quality: i32,
    off_quality: i32,
    io_ratio: f32,
    object: *mut c_void,
    user: *mut c_void,
    unique_id: i32,
    version: i32,
    process_replacing: Option<Process>,
    future: [u8; 56],
}

/// One MIDI message waiting for the next block: sample offset and its bytes.
#[derive(Clone, Copy)]
struct Pending {
    delta: i32,
    status: u8,
    note: u8,
    velocity: u8,
}

struct State {
    sample_rate: f32,
    volume: f32,
    pending: Vec<Pending>,
    note: Option<u8>,
    velocity: u8,
    phase: f64,
}

#[repr(C)]
struct MidiEvent {
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

/// # Safety
/// `effect` must be the effect this crate returned from `VSTPluginMain` and
/// its `object` must still point at the boxed `State` (not yet closed).
unsafe fn state_of<'a>(effect: *mut AEffect) -> &'a mut State {
    // SAFETY: per the contract above; the host calls one instance from one
    // thread at a time, so no other reference to the state exists.
    unsafe { &mut *((*effect).object as *mut State) }
}

/// # Safety
/// `ptr` is null or points at a writable buffer of at least 64 bytes (the
/// VST2 string size used by the host for these opcodes).
unsafe fn write_string(ptr: *mut c_void, text: &str) {
    if ptr.is_null() {
        return;
    }
    let bytes = text.as_bytes();
    let length = bytes.len().min(63);
    // SAFETY: the buffer is at least 64 bytes; `length` leaves room for NUL.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast::<u8>(), length);
        *ptr.cast::<u8>().add(length) = 0;
    }
}

unsafe extern "C" fn dispatch(
    effect: *mut AEffect,
    opcode: i32,
    _index: i32,
    _value: isize,
    ptr: *mut c_void,
    opt: f32,
) -> isize {
    match opcode {
        EFF_OPEN => 0,
        EFF_CLOSE => {
            // SAFETY: the effect and its state were created together by
            // `VSTPluginMain` with `Box::into_raw`; after effClose the host
            // does not use either again.
            unsafe {
                drop(Box::from_raw((*effect).object as *mut State));
                drop(Box::from_raw(effect));
            }
            0
        }
        EFF_SET_SAMPLE_RATE => {
            // SAFETY: the effect is open (see `state_of`).
            unsafe { state_of(effect) }.sample_rate = opt;
            0
        }
        EFF_PROCESS_EVENTS => {
            if ptr.is_null() {
                return 0;
            }
            // SAFETY: a VstEvents argument is `int32 numEvents`, a
            // pointer-sized `reserved`, then `numEvents` pointers to events.
            // The host keeps all of it alive for this call.
            unsafe {
                let base = ptr as *const u8;
                let word = std::mem::size_of::<isize>();
                let count = std::ptr::read(base.cast::<i32>()).max(0) as usize;
                let list = base.add(2 * word).cast::<*const MidiEvent>();
                let state = state_of(effect);
                for index in 0..count {
                    let event = &**list.add(index);
                    if event.event_type == MIDI_EVENT_TYPE {
                        state.pending.push(Pending {
                            delta: event.delta_frames,
                            status: event.midi_data[0],
                            note: event.midi_data[1],
                            velocity: event.midi_data[2],
                        });
                    }
                }
            }
            1
        }
        #[cfg(feature = "bus-fixture")]
        33 => {
            if ptr.is_null() || !(0..4).contains(&_index) {
                return 0;
            }
            // SAFETY: opcode 33 supplies a writable 128-byte pin-properties
            // buffer; this call initializes its complete fixed-width layout.
            // The effect remains open and is accessed serially by the worker.
            unsafe {
                std::ptr::write_bytes(ptr.cast::<u8>(), 0, 128);
                let key = state_of(effect).volume >= 0.5;
                let label = match (_index, key) {
                    (0, _) => "Main L",
                    (1, _) => "Main R",
                    (2, true) => "Key L",
                    (3, true) => "Key R",
                    (2, false) => "Rear L",
                    _ => "Rear R",
                };
                write_string(ptr, label);
            }
            1
        }
        EFF_GET_PLUG_CATEGORY => {
            if cfg!(feature = "shell-fixture") {
                PLUG_CATEGORY_SHELL
            } else if cfg!(feature = "bus-fixture") {
                1
            } else {
                PLUG_CATEGORY_SYNTH
            }
        }
        EFF_SHELL_GET_NEXT_PLUGIN if cfg!(feature = "shell-fixture") => {
            const SHELL: [(&str, i32); 3] = [
                ("Shell Clean", i32::from_be_bytes(*b"SHC1")),
                ("Shell Drive", i32::from_be_bytes(*b"SHD2")),
                ("Shell Echo", i32::from_be_bytes(*b"SHE3")),
            ];
            let Some((name, id)) = SHELL.get(_index as usize) else {
                return 0;
            };
            // SAFETY: the host passes a 64-byte string buffer for a shell
            // sub-plugin name.
            unsafe { write_string(ptr, name) };
            *id as isize
        }
        EFF_GET_EFFECT_NAME | EFF_GET_PRODUCT_STRING => {
            // SAFETY: the host passes a 64-byte string buffer for these.
            unsafe { write_string(ptr, "AR Test Instrument") };
            1
        }
        EFF_GET_VENDOR_STRING => {
            // SAFETY: as above.
            unsafe { write_string(ptr, "AudioRouter tests") };
            1
        }
        EFF_CAN_DO => {
            if ptr.is_null() {
                return 0;
            }
            // SAFETY: the host passes a NUL-terminated capability name.
            let name = unsafe { std::ffi::CStr::from_ptr(ptr as *const _) }.to_string_lossy();
            isize::from(matches!(
                name.as_ref(),
                "receiveVstEvents" | "receiveVstMidiEvent"
            ))
        }
        EFF_GET_VST_VERSION => 2400,
        _ => 0,
    }
}

unsafe extern "C" fn process_legacy(
    effect: *mut AEffect,
    inputs: *const *const f32,
    outputs: *mut *mut f32,
    frames: i32,
) {
    // SAFETY: same contract as `process_replacing`.
    unsafe { process_replacing(effect, inputs, outputs, frames) }
}

unsafe extern "C" fn process_replacing(
    effect: *mut AEffect,
    _inputs: *const *const f32,
    outputs: *mut *mut f32,
    frames: i32,
) {
    // SAFETY: the host passes two writable channel buffers of `frames`
    // samples (this plugin declares two outputs) and an open effect.
    unsafe {
        let state = state_of(effect);
        let left = std::slice::from_raw_parts_mut(*outputs, frames.max(0) as usize);
        let right = std::slice::from_raw_parts_mut(*outputs.add(1), frames.max(0) as usize);
        let mut pending = std::mem::take(&mut state.pending);
        pending.sort_by_key(|message| message.delta);
        let mut next = 0;
        for (index, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            while next < pending.len() && pending[next].delta <= index as i32 {
                let message = pending[next];
                next += 1;
                let kind = message.status & 0xF0;
                if kind == 0x90 && message.velocity > 0 {
                    state.note = Some(message.note);
                    state.velocity = message.velocity;
                    state.phase = 0.0;
                } else if (kind == 0x80 || kind == 0x90) && state.note == Some(message.note) {
                    state.note = None;
                }
            }
            let sample = match state.note {
                Some(note) => {
                    let frequency = 440.0 * 2f64.powf((f64::from(note) - 69.0) / 12.0);
                    state.phase += frequency / f64::from(state.sample_rate);
                    state.phase -= state.phase.floor();
                    let level = f64::from(state.velocity) / 127.0 * f64::from(state.volume) * 0.5;
                    ((state.phase * std::f64::consts::TAU).sin() * level) as f32
                }
                None => 0.0,
            };
            *l = sample;
            *r = sample;
        }
        // Events that arrived with offsets past this block are not kept.
        pending.clear();
        state.pending = pending;
    }
}

unsafe extern "C" fn set_parameter(effect: *mut AEffect, index: i32, value: f32) {
    if index == 0 {
        // SAFETY: the effect is open (see `state_of`).
        unsafe { state_of(effect) }.volume = value.clamp(0.0, 1.0);
    }
}

unsafe extern "C" fn get_parameter(effect: *mut AEffect, index: i32) -> f32 {
    if index == 0 {
        // SAFETY: the effect is open (see `state_of`).
        unsafe { state_of(effect) }.volume
    } else {
        0.0
    }
}

/// The VST2 entry point.
///
/// # Safety
/// Called by a VST2 host once per instance, with a valid host callback.
#[no_mangle]
pub unsafe extern "C" fn VSTPluginMain(_host: HostCallback) -> *mut AEffect {
    let state = Box::new(State {
        sample_rate: 48_000.0,
        volume: 1.0,
        pending: Vec::with_capacity(64),
        note: None,
        velocity: 0,
        phase: 0.0,
    });
    Box::into_raw(Box::new(AEffect {
        magic: MAGIC,
        dispatcher: Some(dispatch),
        process: Some(process_legacy),
        set_parameter: Some(set_parameter),
        get_parameter: Some(get_parameter),
        num_programs: 1,
        num_params: 1,
        num_inputs: if cfg!(feature = "bus-fixture") {
            4
        } else if cfg!(feature = "shell-fixture") {
            2
        } else {
            0
        },
        num_outputs: 2,
        flags: FLAG_CAN_REPLACING
            | if cfg!(feature = "bus-fixture") || cfg!(feature = "shell-fixture") {
                0
            } else {
                FLAG_IS_SYNTH
            },
        reserved_1: 0,
        reserved_2: 0,
        initial_delay: 0,
        real_quality: 0,
        off_quality: 0,
        io_ratio: 1.0,
        object: Box::into_raw(state).cast(),
        user: std::ptr::null_mut(),
        unique_id: i32::from_be_bytes(*b"ARTI"),
        version: 1,
        process_replacing: Some(process_replacing),
        future: [0; 56],
    }))
}
