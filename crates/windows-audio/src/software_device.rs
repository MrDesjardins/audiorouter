//! Explicit Windows Software Device API ownership for managed virtual buses.
//!
//! This adapter is control-plane code. It may block for the bounded PnP
//! callback and must never be called from an audio callback. The caller owns
//! the returned handle for as long as the software device should remain
//! enumerated; dropping it uses the default handle lifetime and removes the
//! temporary device.

use std::ffi::c_void;
use std::ptr::null;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use windows_core::HRESULT;

const MAX_INSTANCE_ID_CHARS: usize = 64;
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(5);
const SW_DEVICE_CAPABILITIES_DRIVER_REQUIRED: u32 = 0x8;

type HswDevice = *mut c_void;

#[repr(C)]
struct SwDeviceCreateInfo {
    cb_size: u32,
    instance_id: *const u16,
    hardware_ids: *const u16,
    compatible_ids: *const u16,
    container_id: *const windows_core::GUID,
    capability_flags: u32,
    device_description: *const u16,
    device_location: *const u16,
    security_descriptor: *const c_void,
}

struct Completion {
    handle: HswDevice,
    result: HRESULT,
    instance_id: String,
}

unsafe extern "system" fn created_callback(
    handle: HswDevice,
    result: HRESULT,
    context: *mut c_void,
    instance_id: *const u16,
) {
    // The context is an mpsc Sender allocated by create(). It remains alive
    // until the bounded callback result is received. The callback is allowed
    // to run before SwDeviceCreate returns, so no stack pointer is passed.
    let sender = &*(context as *const mpsc::Sender<Completion>);
    let id = if instance_id.is_null() {
        String::new()
    } else {
        let mut length = 0;
        while *instance_id.add(length) != 0 && length <= MAX_INSTANCE_ID_CHARS {
            length += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(instance_id, length))
    };
    let _ = sender.send(Completion {
        handle,
        result,
        instance_id: id,
    });
}

#[link(name = "Swdevice")]
unsafe extern "system" {
    fn SwDeviceCreate(
        enumerator_name: *const u16,
        parent_device_instance: *const u16,
        create_info: *const SwDeviceCreateInfo,
        property_count: u32,
        properties: *const c_void,
        callback: unsafe extern "system" fn(HswDevice, HRESULT, *mut c_void, *const u16),
        context: *mut c_void,
        device: *mut HswDevice,
    ) -> HRESULT;
    fn SwDeviceClose(device: HswDevice);
}

#[derive(Debug)]
pub enum SoftwareDeviceError {
    InvalidInstanceId,
    Windows(HRESULT),
    CallbackTimeout,
    Callback(HRESULT),
    MissingHandle,
}

impl std::fmt::Display for SoftwareDeviceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInstanceId => formatter.write_str(
                "software-device instance ID must contain 1..64 printable characters without slash",
            ),
            Self::Windows(result) => {
                write!(formatter, "Windows Software Device API returned {result:?}")
            }
            Self::CallbackTimeout => {
                formatter.write_str("Windows Software Device API callback timed out")
            }
            Self::Callback(result) => write!(
                formatter,
                "Windows Software Device API callback returned {result:?}"
            ),
            Self::MissingHandle => {
                formatter.write_str("Windows Software Device API returned no device handle")
            }
        }
    }
}

impl std::error::Error for SoftwareDeviceError {}

/// Control-plane owner for the AudioRouter software-device enumerator.
#[derive(Debug, Default, Clone, Copy)]
pub struct SoftwareDeviceProvisioner;

impl SoftwareDeviceProvisioner {
    /// Create one software device matching the AudioRouter driver INF.
    ///
    /// The caller must already have passed the explicit device-administration
    /// authorization gate. Windows requires administrator access for this
    /// operation; the adapter does not elevate or install a package.
    pub fn create(&self, instance_id: &str) -> Result<SoftwareDeviceHandle, SoftwareDeviceError> {
        if !valid_instance_id(instance_id) {
            return Err(SoftwareDeviceError::InvalidInstanceId);
        }
        let instance = wide(instance_id);
        let enumerator = wide("AudioRouter");
        let parent = wide("HTREE\\ROOT\\0");
        let description = wide("AudioRouter virtual bus");
        let hardware_ids = wide_multi("SWD\\AudioRouterVirtual");
        let info = SwDeviceCreateInfo {
            cb_size: std::mem::size_of::<SwDeviceCreateInfo>() as u32,
            instance_id: instance.as_ptr(),
            hardware_ids: hardware_ids.as_ptr(),
            compatible_ids: null(),
            container_id: null(),
            capability_flags: SW_DEVICE_CAPABILITIES_DRIVER_REQUIRED,
            device_description: description.as_ptr(),
            device_location: null(),
            security_descriptor: null(),
        };
        let (sender, receiver) = mpsc::channel::<Completion>();
        let context = Box::into_raw(Box::new(sender));
        let mut returned_handle = std::ptr::null_mut();
        // SAFETY: all UTF-16 buffers and `info` remain pinned for the call;
        // `context` is heap-owned until the callback result is received.
        let result = unsafe {
            SwDeviceCreate(
                enumerator.as_ptr(),
                parent.as_ptr(),
                &info,
                0,
                null(),
                created_callback,
                context.cast(),
                &mut returned_handle,
            )
        };
        if result.is_err() {
            // No successful device exists on a failed API call. Reclaim the
            // callback context; Windows did not accept the callback contract.
            unsafe { drop(Box::from_raw(context)) };
            return Err(SoftwareDeviceError::Windows(result));
        }
        let completion = match receiver.recv_timeout(CALLBACK_TIMEOUT) {
            Ok(value) => value,
            Err(RecvTimeoutError::Timeout) => {
                // Do not reclaim `context`: a late callback could still run.
                // The bounded allocation is intentionally leaked on this
                // terminal timeout to preserve callback memory safety.
                if !returned_handle.is_null() {
                    unsafe { SwDeviceClose(returned_handle) };
                }
                return Err(SoftwareDeviceError::CallbackTimeout);
            }
            Err(RecvTimeoutError::Disconnected) => {
                unsafe { drop(Box::from_raw(context)) };
                return Err(SoftwareDeviceError::Callback(E_FAIL));
            }
        };
        unsafe { drop(Box::from_raw(context)) };
        if completion.result.is_err() {
            if !completion.handle.is_null() {
                unsafe { SwDeviceClose(completion.handle) };
            }
            return Err(SoftwareDeviceError::Callback(completion.result));
        }
        let handle = if !completion.handle.is_null() {
            completion.handle
        } else {
            returned_handle
        };
        if handle.is_null() {
            return Err(SoftwareDeviceError::MissingHandle);
        }
        Ok(SoftwareDeviceHandle {
            raw: handle,
            instance_id: completion.instance_id,
        })
    }
}

#[derive(Debug)]
pub struct SoftwareDeviceHandle {
    raw: HswDevice,
    instance_id: String,
}

impl SoftwareDeviceHandle {
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }
}

impl Drop for SoftwareDeviceHandle {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: `raw` is the live handle returned by SwDeviceCreate and
            // is owned exclusively by this RAII value.
            unsafe { SwDeviceClose(self.raw) };
            self.raw = std::ptr::null_mut();
        }
    }
}

fn valid_instance_id(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= MAX_INSTANCE_ID_CHARS
        && value
            .chars()
            .all(|character| !character.is_control() && character != '\\' && character != '/')
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_multi(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0, 0]).collect()
}

const E_FAIL: HRESULT = HRESULT(0x8000_4005u32 as i32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_id_validation_is_bounded_and_pnp_safe() {
        assert!(valid_instance_id("voice-bus"));
        assert!(valid_instance_id(&"x".repeat(MAX_INSTANCE_ID_CHARS)));
        assert!(!valid_instance_id(""));
        assert!(!valid_instance_id(&"x".repeat(MAX_INSTANCE_ID_CHARS + 1)));
        assert!(!valid_instance_id("voice\\bus"));
        assert!(!valid_instance_id("voice/bus"));
        assert!(!valid_instance_id("voice\n"));
    }

    #[test]
    fn hardware_id_is_double_null_terminated() {
        let ids = wide_multi("SWD\\AudioRouterVirtual");
        assert_eq!(ids[ids.len() - 1], 0);
        assert_eq!(ids[ids.len() - 2], 0);
    }
}
