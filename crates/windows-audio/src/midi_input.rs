//! Windows MIDI input (winmm) for instrument tools (phase P8b).
//!
//! `list_midi_input_devices` reads the device list on request. A
//! `MidiInputConnection` opens one device by name and calls a sink with every
//! short message (note, controller, pitch bend ...). The sink runs on a winmm
//! thread, never on an audio callback, and must return quickly; the control
//! plane's sink only pushes into a bounded queue.
//!
//! A device that is unplugged stops delivering messages. The connection does
//! not look for another device: the control plane reports the loss by name
//! (`connected_names`) and the user chooses again, the same rule as for
//! microphones.

use windows::Win32::Media::Audio::{
    midiInClose, midiInGetDevCapsW, midiInGetNumDevs, midiInOpen, midiInReset, midiInStart,
    midiInStop, CALLBACK_FUNCTION, HMIDIIN, MIDIINCAPSW,
};

/// `MIM_DATA`: one short message arrived.
const MIM_DATA: u32 = 0x3C3;
/// `MMSYSERR_NOERROR`.
const NO_ERROR: u32 = 0;
/// Most devices read; a PC has a handful, this only bounds a hostile driver.
const MAX_DEVICES: u32 = 64;

/// One MIDI input device as winmm names it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiInputDevice {
    /// Position in winmm's list at the moment of listing; it can change when a
    /// device is plugged or unplugged, so choose devices by name.
    pub index: u32,
    pub name: String,
}

fn unique_device_by_name<'a>(
    devices: &'a [MidiInputDevice],
    name: &str,
) -> Option<&'a MidiInputDevice> {
    let mut matches = devices.iter().filter(|device| device.name == name);
    let first = matches.next()?;
    if matches.next().is_some() {
        None
    } else {
        Some(first)
    }
}

/// Why a MIDI input could not be opened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MidiInputError {
    /// No connected device has this name.
    NotConnected,
    /// winmm refused (device busy in another program, driver error).
    Open(u32),
}

impl std::fmt::Display for MidiInputError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConnected => formatter.write_str("that MIDI device is not connected"),
            Self::Open(code) => write!(
                formatter,
                "Windows could not open the MIDI device (error {code}); another program may be using it"
            ),
        }
    }
}

impl std::error::Error for MidiInputError {}

/// A device name from winmm's fixed 32-unit buffer.
fn device_name(caps: &MIDIINCAPSW) -> String {
    // The struct is packed: copy the array out by value, never borrow a field.
    let name = caps.szPname;
    let length = name
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(name.len());
    String::from_utf16_lossy(&name[..length]).trim().to_owned()
}

/// The MIDI input devices connected right now.
pub fn list_midi_input_devices() -> Vec<MidiInputDevice> {
    // SAFETY: `midiInGetNumDevs` takes no arguments. `midiInGetDevCapsW`
    // writes at most `size_of::<MIDIINCAPSW>()` bytes into `caps`, a local
    // value that lives for the whole call.
    let count = unsafe { midiInGetNumDevs() }.min(MAX_DEVICES);
    (0..count)
        .filter_map(|index| {
            let mut caps = MIDIINCAPSW::default();
            // SAFETY: see above; `caps` is valid and correctly sized.
            let result = unsafe {
                midiInGetDevCapsW(
                    index as usize,
                    &mut caps,
                    std::mem::size_of::<MIDIINCAPSW>() as u32,
                )
            };
            (result == NO_ERROR).then(|| MidiInputDevice {
                index,
                name: device_name(&caps),
            })
        })
        .collect()
}

/// Splits winmm's packed short message (`status | data1 << 8 | data2 << 16`).
pub fn unpack_short_message(packed: usize) -> (u8, u8, u8) {
    (
        (packed & 0xFF) as u8,
        ((packed >> 8) & 0xFF) as u8,
        ((packed >> 16) & 0xFF) as u8,
    )
}

type Sink = dyn Fn(u8, u8, u8) + Send + Sync;

/// The state winmm's callback reads. It lives in a `Box` owned by the
/// connection so its address stays fixed until after `midiInClose`.
struct CallbackState {
    sink: Box<Sink>,
}

/// winmm calls this on its own thread for every event of an open device.
///
/// SAFETY contract: `instance` is the address of the `CallbackState` boxed by
/// `MidiInputConnection::open`. `midiInClose` (in `Drop`) waits for callbacks
/// in progress and no callback follows it, and the state is freed only after
/// that call returns, so the reference is valid for every call.
unsafe extern "system" fn on_midi_message(
    _handle: HMIDIIN,
    message: u32,
    instance: usize,
    first: usize,
    _second: usize,
) {
    if message != MIM_DATA || instance == 0 {
        return;
    }
    // SAFETY: per the contract above.
    let state = unsafe { &*(instance as *const CallbackState) };
    let (status, data1, data2) = unpack_short_message(first);
    // A panic must not unwind into winmm's thread.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (state.sink)(status, data1, data2)
    }));
}

/// An open MIDI input device delivering short messages to a sink until dropped.
pub struct MidiInputConnection {
    handle: HMIDIIN,
    name: String,
    // Declared after `handle` only for readability: `Drop` closes the handle
    // first, then lets this box go.
    _state: Box<CallbackState>,
}

// SAFETY: the handle is an opaque winmm token used only to stop and close the
// device from whichever thread drops the connection; winmm allows that.
unsafe impl Send for MidiInputConnection {}

impl MidiInputConnection {
    /// Opens the connected device called `name` and starts it.
    pub fn open(
        name: &str,
        sink: impl Fn(u8, u8, u8) + Send + Sync + 'static,
    ) -> Result<Self, MidiInputError> {
        let devices = list_midi_input_devices();
        let device = unique_device_by_name(&devices, name).ok_or(MidiInputError::NotConnected)?;
        let state = Box::new(CallbackState {
            sink: Box::new(sink),
        });
        // A pointer-sized integer is what `midiInOpen` takes for the callback.
        let callback: unsafe extern "system" fn(HMIDIIN, u32, usize, usize, usize) =
            on_midi_message;
        let mut handle = HMIDIIN::default();
        // SAFETY: `handle` is a valid out-pointer; the callback address is a
        // function with winmm's `MIDIINPROC` signature; `instance` is the
        // address of `state`, kept alive until the device is closed in `Drop`.
        let opened = unsafe {
            midiInOpen(
                &mut handle,
                device.index,
                Some(callback as usize),
                Some(&*state as *const CallbackState as usize),
                CALLBACK_FUNCTION,
            )
        };
        if opened != NO_ERROR {
            return Err(MidiInputError::Open(opened));
        }
        // SAFETY: `handle` was just opened successfully.
        let started = unsafe { midiInStart(handle) };
        if started != NO_ERROR {
            // SAFETY: the handle is open; closing it ends the callbacks.
            unsafe {
                midiInClose(handle);
            }
            return Err(MidiInputError::Open(started));
        }
        Ok(Self {
            handle,
            name: device.name.clone(),
            _state: state,
        })
    }

    /// The device name this connection opened.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for MidiInputConnection {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful `midiInOpen` and is closed
        // exactly once, here. `midiInClose` returns after any running callback
        // finishes, so `_state` is not read after it is freed.
        unsafe {
            midiInStop(self.handle);
            midiInReset(self.handle);
            midiInClose(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_device_names_are_not_resolved_to_the_first_keyboard() {
        let devices = vec![
            MidiInputDevice {
                index: 0,
                name: "Keyboard".into(),
            },
            MidiInputDevice {
                index: 1,
                name: "Keyboard".into(),
            },
        ];
        assert!(unique_device_by_name(&devices, "Keyboard").is_none());
        assert_eq!(
            unique_device_by_name(&devices[..1], "Keyboard"),
            Some(&devices[0])
        );
    }

    #[test]
    fn short_messages_unpack_into_status_and_data_bytes() {
        assert_eq!(unpack_short_message(0x7F_45_90), (0x90, 0x45, 0x7F));
        assert_eq!(unpack_short_message(0), (0, 0, 0));
        // Bits above the third byte are the driver's, not MIDI data.
        assert_eq!(unpack_short_message(0xAB_7F_45_90), (0x90, 0x45, 0x7F));
    }

    #[test]
    fn a_missing_device_is_reported_by_name_not_replaced() {
        let error = MidiInputConnection::open("No such MIDI keyboard 7f3a", |_, _, _| {})
            .err()
            .expect("an unknown device never opens");
        assert_eq!(error, MidiInputError::NotConnected);
        assert!(error.to_string().contains("not connected"));
    }

    #[test]
    fn listing_devices_never_fails_and_names_are_trimmed() {
        for device in list_midi_input_devices() {
            assert_eq!(device.name, device.name.trim());
        }
    }
}
