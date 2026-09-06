//! Windows audio device metadata boundary for M02.
//!
//! This first adapter slice is intentionally read-only: it enumerates active
//! endpoints and reports the endpoint-owned shared-mode format and periods.
//! It does not initialize, start, or read an audio stream.

use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointDirection {
    Capture,
    Render,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointInfo {
    pub id: String,
    pub direction: EndpointDirection,
    pub default_period_100ns: i64,
    pub minimum_period_100ns: i64,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub format_tag: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EndpointChange {
    Added(EndpointInfo),
    Removed(EndpointInfo),
    Changed {
        before: EndpointInfo,
        after: EndpointInfo,
    },
}

/// Diff two metadata snapshots by opaque endpoint ID. This is a control-plane
/// polling helper; it never rebinding a missing endpoint or opens a stream.
pub fn diff_endpoint_snapshots(
    previous: &[EndpointInfo],
    current: &[EndpointInfo],
) -> Vec<EndpointChange> {
    let mut changes = Vec::new();
    for before in previous {
        match current.iter().find(|after| after.id == before.id) {
            Some(after) if after != before => changes.push(EndpointChange::Changed {
                before: before.clone(),
                after: after.clone(),
            }),
            Some(_) => {}
            None => changes.push(EndpointChange::Removed(before.clone())),
        }
    }
    for after in current {
        if !previous.iter().any(|before| before.id == after.id) {
            changes.push(EndpointChange::Added(after.clone()));
        }
    }
    changes
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapturePacket {
    pub frames: u32,
    pub flags: u32,
    pub device_position: u64,
    pub qpc_position: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationInfo {
    pub process_id: u32,
    pub executable: String,
}

#[derive(Debug)]
pub enum AudioError {
    Windows(windows::core::Error),
    InvalidUtf16,
}

impl fmt::Display for AudioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows(error) => write!(formatter, "Windows audio error: {error}"),
            Self::InvalidUtf16 => formatter.write_str("endpoint ID was not valid UTF-16"),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<windows::core::Error> for AudioError {
    fn from(error: windows::core::Error) -> Self {
        Self::Windows(error)
    }
}

struct ComApartment;

impl ComApartment {
    fn initialize() -> Result<Self, AudioError> {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
            .ok()?;
        }
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { windows::Win32::System::Com::CoUninitialize() }
    }
}

/// A shared-mode capture client whose packet API never exposes a borrowed
/// device buffer. Callers must copy/process data inside their own bounded
/// realtime design; this adapter only returns packet metadata in M02's first
/// lifecycle slice.
pub struct SharedCapture {
    client: windows::Win32::Media::Audio::IAudioClient,
    capture: windows::Win32::Media::Audio::IAudioCaptureClient,
    started: bool,
    _com: ComApartment,
}

/// A shared-mode render client. `submit_silence` is the safe baseline API for
/// exercising the render queue without requiring a caller to provide audio
/// samples or accidentally emit uninitialized memory.
pub struct SharedRender {
    client: windows::Win32::Media::Audio::IAudioClient,
    render: windows::Win32::Media::Audio::IAudioRenderClient,
    buffer_size: u32,
    started: bool,
    _com: ComApartment,
}

impl SharedCapture {
    /// Open an exact active capture endpoint using its opaque endpoint ID.
    /// The stream is initialized but remains stopped until `start` is called.
    pub fn open(endpoint_id: &str, buffer_duration_100ns: i64) -> Result<Self, AudioError> {
        use windows::Win32::Media::Audio::{
            eCapture, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
            AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
            AUDCLNT_STREAMFLAGS_NOPERSIST, DEVICE_STATE_ACTIVE,
        };
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

        let com = ComApartment::initialize()?;
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let devices = unsafe { enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)? };
        let count = unsafe { devices.GetCount()? };
        let mut selected = None;
        for index in 0..count {
            let device = unsafe { devices.Item(index)? };
            let id = unsafe {
                device
                    .GetId()?
                    .to_string()
                    .map_err(|_| AudioError::InvalidUtf16)?
            };
            if id == endpoint_id {
                selected = Some(device);
                break;
            }
        }
        let device = selected.ok_or_else(|| {
            AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80070490u32 as i32),
                "capture endpoint not found",
            ))
        })?;
        let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None)? };
        let format = unsafe { client.GetMixFormat()? };
        let initialized = unsafe {
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_NOPERSIST,
                buffer_duration_100ns,
                0,
                format,
                None,
            )
        };
        unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(format.cast())) };
        initialized?;
        let capture: IAudioCaptureClient = unsafe { client.GetService()? };
        Ok(Self {
            client,
            capture,
            started: false,
            _com: com,
        })
    }

    pub fn start(&mut self) -> Result<(), AudioError> {
        unsafe { self.client.Start()? };
        self.started = true;
        Ok(())
    }

    pub fn next_packet(&self) -> Result<Option<CapturePacket>, AudioError> {
        let frames = unsafe { self.capture.GetNextPacketSize()? };
        if frames == 0 {
            return Ok(None);
        }
        let mut data = std::ptr::null_mut();
        let mut packet_frames = frames;
        let mut flags = 0;
        let mut device_position = 0;
        let mut qpc_position = 0;
        unsafe {
            self.capture.GetBuffer(
                &mut data,
                &mut packet_frames,
                &mut flags,
                Some(&mut device_position),
                Some(&mut qpc_position),
            )?;
            self.capture.ReleaseBuffer(packet_frames)?;
        }
        Ok(Some(CapturePacket {
            frames: packet_frames,
            flags,
            device_position,
            qpc_position,
        }))
    }

    pub fn stop(&mut self) -> Result<(), AudioError> {
        if self.started {
            unsafe { self.client.Stop()? };
            self.started = false;
        }
        unsafe { self.client.Reset()? };
        Ok(())
    }
}

impl Drop for SharedCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl SharedRender {
    /// Open an exact active render endpoint using its opaque endpoint ID.
    /// The stream is initialized but remains stopped until `start` is called.
    pub fn open(endpoint_id: &str, buffer_duration_100ns: i64) -> Result<Self, AudioError> {
        use windows::Win32::Media::Audio::{
            eRender, IAudioClient, IAudioRenderClient, IMMDeviceEnumerator, MMDeviceEnumerator,
            AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
            AUDCLNT_STREAMFLAGS_NOPERSIST, DEVICE_STATE_ACTIVE,
        };
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

        let com = ComApartment::initialize()?;
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let devices = unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)? };
        let count = unsafe { devices.GetCount()? };
        let mut selected = None;
        for index in 0..count {
            let device = unsafe { devices.Item(index)? };
            let id = unsafe {
                device
                    .GetId()?
                    .to_string()
                    .map_err(|_| AudioError::InvalidUtf16)?
            };
            if id == endpoint_id {
                selected = Some(device);
                break;
            }
        }
        let device = selected.ok_or_else(|| {
            AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80070490u32 as i32),
                "render endpoint not found",
            ))
        })?;
        let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None)? };
        let format = unsafe { client.GetMixFormat()? };
        let initialized = unsafe {
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_NOPERSIST,
                buffer_duration_100ns,
                0,
                format,
                None,
            )
        };
        unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(format.cast())) };
        initialized?;
        let buffer_size = unsafe { client.GetBufferSize()? };
        let render: IAudioRenderClient = unsafe { client.GetService()? };
        Ok(Self {
            client,
            render,
            buffer_size,
            started: false,
            _com: com,
        })
    }

    pub fn start(&mut self) -> Result<(), AudioError> {
        unsafe { self.client.Start()? };
        self.started = true;
        Ok(())
    }

    /// Submit all currently available frames as silence and return that count.
    pub fn submit_silence(&self) -> Result<u32, AudioError> {
        let padding = unsafe { self.client.GetCurrentPadding()? };
        let available = self.buffer_size.saturating_sub(padding);
        if available == 0 {
            return Ok(0);
        }
        unsafe {
            let _data = self.render.GetBuffer(available)?;
            self.render.ReleaseBuffer(
                available,
                windows::Win32::Media::Audio::AUDCLNT_BUFFERFLAGS_SILENT.0 as u32,
            )?;
        }
        Ok(available)
    }

    pub fn stop(&mut self) -> Result<(), AudioError> {
        if self.started {
            unsafe { self.client.Stop()? };
            self.started = false;
        }
        unsafe { self.client.Reset()? };
        Ok(())
    }
}

impl Drop for SharedRender {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Enumerate active capture and render endpoints without opening streams.
pub fn enumerate_active_endpoints() -> Result<Vec<EndpointInfo>, AudioError> {
    unsafe {
        let initialized = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
        initialized.ok()?;
        let result = enumerate_after_com_init();
        windows::Win32::System::Com::CoUninitialize();
        result
    }
}

/// Enumerate process identities suitable for a later process-loopback binding.
/// Only PID and executable name are returned; command lines and full paths are
/// intentionally excluded from this discovery surface.
pub fn enumerate_applications() -> Result<Vec<ApplicationInfo>, AudioError> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut applications = Vec::new();
        let first = Process32FirstW(snapshot, &mut entry);
        if first.is_ok() {
            loop {
                let length = entry
                    .szExeFile
                    .iter()
                    .position(|character| *character == 0)
                    .unwrap_or(entry.szExeFile.len());
                let executable = String::from_utf16(&entry.szExeFile[..length])
                    .map_err(|_| AudioError::InvalidUtf16)?;
                applications.push(ApplicationInfo {
                    process_id: entry.th32ProcessID,
                    executable,
                });
                if applications.len() >= 4096 || Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        first.map(|_| applications).map_err(AudioError::Windows)
    }
}

unsafe fn enumerate_after_com_init() -> Result<Vec<EndpointInfo>, AudioError> {
    use windows::Win32::Media::Audio::{
        eCapture, eRender, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
        DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let mut endpoints = Vec::new();
    for (direction, flow) in [
        (EndpointDirection::Capture, eCapture),
        (EndpointDirection::Render, eRender),
    ] {
        let devices = enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)?;
        let count = devices.GetCount()?;
        for index in 0..count {
            let device = devices.Item(index)?;
            let id = device
                .GetId()
                .map_err(AudioError::Windows)?
                .to_string()
                .map_err(|_| AudioError::InvalidUtf16)?;
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let mut default_period = 0;
            let mut minimum_period = 0;
            client.GetDevicePeriod(Some(&mut default_period), Some(&mut minimum_period))?;
            let format = client.GetMixFormat()?;
            let format_value = *format;
            endpoints.push(EndpointInfo {
                id,
                direction,
                default_period_100ns: default_period,
                minimum_period_100ns: minimum_period,
                sample_rate_hz: format_value.nSamplesPerSec,
                channels: format_value.nChannels,
                bits_per_sample: format_value.wBitsPerSample,
                format_tag: format_value.wFormatTag,
            });
            windows::Win32::System::Com::CoTaskMemFree(Some(format.cast()));
        }
    }
    Ok(endpoints)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_metadata_shape_is_stable() {
        let info = EndpointInfo {
            id: "endpoint".into(),
            direction: EndpointDirection::Capture,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 0xfffe,
        };
        assert_eq!(info.direction, EndpointDirection::Capture);
        assert!(info.minimum_period_100ns <= info.default_period_100ns);
    }

    #[test]
    fn endpoint_snapshot_diff_preserves_identity_and_detects_changes() {
        let before = EndpointInfo {
            id: "same".into(),
            direction: EndpointDirection::Capture,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 1,
            bits_per_sample: 32,
            format_tag: 3,
        };
        let mut after = before.clone();
        after.channels = 2;
        let added = EndpointInfo {
            id: "new".into(),
            ..before.clone()
        };
        let changes = diff_endpoint_snapshots(&[before.clone()], &[after.clone(), added.clone()]);
        assert_eq!(changes.len(), 2);
        assert!(changes.contains(&EndpointChange::Changed { before, after }));
        assert!(changes.contains(&EndpointChange::Added(added)));
    }

    #[cfg(windows)]
    #[test]
    fn opening_an_unknown_capture_endpoint_fails_without_starting_audio() {
        let error = SharedCapture::open("audiorouter-missing-endpoint", 1_000_000);
        assert!(matches!(error, Err(AudioError::Windows(_))));
    }

    #[cfg(windows)]
    #[test]
    fn opening_an_unknown_render_endpoint_fails_without_starting_audio() {
        let error = SharedRender::open("audiorouter-missing-endpoint", 1_000_000);
        assert!(matches!(error, Err(AudioError::Windows(_))));
    }

    #[cfg(windows)]
    #[test]
    fn active_endpoint_enumeration_is_read_only() {
        let endpoints = enumerate_active_endpoints().unwrap();
        assert!(!endpoints.is_empty());
        assert!(endpoints.iter().all(|endpoint| !endpoint.id.is_empty()));
        assert!(endpoints.iter().all(|endpoint| endpoint.sample_rate_hz > 0));
    }
}
