//! Windows audio device metadata boundary for M02.
//!
//! This first adapter slice is intentionally read-only: it enumerates active
//! endpoints and reports the endpoint-owned shared-mode format and periods.
//! It does not initialize, start, or read an audio stream.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    /// Windows process creation time in 100-ns intervals since 1601 UTC.
    /// Combined with PID, this prevents rebinding a reused process ID.
    pub creation_time_100ns: Option<u64>,
}

/// Read-only audio-session facts associated with a process. These facts are
/// observed from Windows session managers; they are not a promise that a
/// future process-loopback binding will succeed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationAudioInfo {
    pub process_id: u32,
    pub active_session_count: u32,
    pub total_session_count: u32,
    pub capture_session_count: u32,
    pub display_names: Vec<String>,
}

#[derive(Debug)]
pub enum AudioError {
    Windows(windows::core::Error),
    InvalidUtf16,
    BufferTooSmall { required: usize, available: usize },
    InvalidFrameSize,
    ApplicationNotFound { process_id: u32 },
    ApplicationIdentityChanged { process_id: u32 },
    ApplicationIdentityUnavailable { process_id: u32 },
    ApplicationRestartNotFound { executable: String },
    ApplicationRestartAmbiguous { executable: String },
    ApplicationRestartIdentityUnavailable { executable: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioFailureKind {
    InvalidArgument,
    AccessDenied,
    DeviceInUse,
    ExclusiveModeOnly,
    DeviceInvalidated,
    UnsupportedFormat,
    ServiceUnavailable,
    BufferConstraint,
    Other,
}

impl AudioError {
    /// Classify errors for stable control-plane behavior while retaining the
    /// original HRESULT for diagnostics.
    pub fn kind(&self) -> AudioFailureKind {
        if matches!(self, Self::BufferTooSmall { .. }) {
            return AudioFailureKind::BufferConstraint;
        }
        let code = match self {
            Self::Windows(error) => error.code().0 as u32,
            Self::InvalidUtf16
            | Self::InvalidFrameSize
            | Self::ApplicationNotFound { .. }
            | Self::ApplicationIdentityChanged { .. }
            | Self::ApplicationRestartNotFound { .. }
            | Self::ApplicationRestartAmbiguous { .. } => 0x80070057,
            Self::ApplicationIdentityUnavailable { .. }
            | Self::ApplicationRestartIdentityUnavailable { .. } => 0x80070005,
            Self::BufferTooSmall { .. } => unreachable!(),
        };
        match code {
            0x80070057 => AudioFailureKind::InvalidArgument,
            0x80070005 => AudioFailureKind::AccessDenied,
            0x8889000A => AudioFailureKind::DeviceInUse,
            0x88890012 => AudioFailureKind::ExclusiveModeOnly,
            0x88890004 => AudioFailureKind::DeviceInvalidated,
            0x88890008 => AudioFailureKind::UnsupportedFormat,
            0x88890010 => AudioFailureKind::ServiceUnavailable,
            _ => AudioFailureKind::Other,
        }
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows(error) => write!(formatter, "Windows audio error: {error}"),
            Self::InvalidUtf16 => formatter.write_str("endpoint ID was not valid UTF-16"),
            Self::BufferTooSmall {
                required,
                available,
            } => {
                write!(
                    formatter,
                    "capture buffer too small: need {required} bytes, have {available}"
                )
            }
            Self::InvalidFrameSize => formatter.write_str("audio frame size was invalid"),
            Self::ApplicationNotFound { process_id } => {
                write!(formatter, "application process {process_id} was not found")
            }
            Self::ApplicationIdentityChanged { process_id } => {
                write!(
                    formatter,
                    "application process {process_id} identity changed"
                )
            }
            Self::ApplicationIdentityUnavailable { process_id } => {
                write!(
                    formatter,
                    "application process {process_id} identity unavailable"
                )
            }
            Self::ApplicationRestartNotFound { executable } => {
                write!(
                    formatter,
                    "no restart candidate matched executable {executable}"
                )
            }
            Self::ApplicationRestartAmbiguous { executable } => {
                write!(
                    formatter,
                    "multiple restart candidates matched executable {executable}"
                )
            }
            Self::ApplicationRestartIdentityUnavailable { executable } => write!(
                formatter,
                "restart candidate for executable {executable} has no creation identity"
            ),
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

struct EventHandle(windows::Win32::Foundation::HANDLE);

impl Drop for EventHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

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
    event: EventHandle,
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
    event: EventHandle,
    _com: ComApartment,
}

impl SharedCapture {
    /// Open an exact active capture endpoint using its opaque endpoint ID.
    /// The stream is initialized but remains stopped until `start` is called.
    /// The duration argument is retained for API compatibility; event-driven
    /// shared-mode WASAPI requires `Initialize` to receive zero here.
    pub fn open(endpoint_id: &str, _buffer_duration_100ns: i64) -> Result<Self, AudioError> {
        use windows::Win32::Media::Audio::{
            eCapture, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
            AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
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
        let event = EventHandle(unsafe {
            windows::Win32::System::Threading::CreateEventW(None, false, false, None)?
        });
        let initialized = unsafe {
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                // GetMixFormat is the endpoint's exact shared-mode format;
                // conversion is neither needed nor desirable here. Keeping
                // the request exact avoids format/flag combinations that
                // some drivers reject with E_INVALIDARG.
                AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_NOPERSIST,
                0,
                0,
                format,
                None,
            )
        };
        unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(format.cast())) };
        initialized?;
        unsafe { client.SetEventHandle(event.0)? };
        let capture: IAudioCaptureClient = unsafe { client.GetService()? };
        Ok(Self {
            client,
            capture,
            started: false,
            event,
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

    /// Copy one packet into caller-owned storage and release the WASAPI
    /// buffer before returning. `bytes_per_frame` must describe the endpoint's
    /// mix format; the destination must be sized for the largest packet the
    /// caller permits. No allocation or borrowed device memory escapes.
    pub fn next_packet_into(
        &self,
        destination: &mut [u8],
        bytes_per_frame: usize,
    ) -> Result<Option<(CapturePacket, usize)>, AudioError> {
        use windows::Win32::Media::Audio::AUDCLNT_BUFFERFLAGS_SILENT;

        if bytes_per_frame == 0 {
            return Err(AudioError::InvalidFrameSize);
        }
        let frames = unsafe { self.capture.GetNextPacketSize()? } as usize;
        if frames == 0 {
            return Ok(None);
        }
        let required = frames
            .checked_mul(bytes_per_frame)
            .ok_or(AudioError::InvalidFrameSize)?;
        if destination.len() < required {
            return Err(AudioError::BufferTooSmall {
                required,
                available: destination.len(),
            });
        }
        let mut data = std::ptr::null_mut();
        let mut packet_frames = frames as u32;
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
            let packet_bytes = (packet_frames as usize)
                .checked_mul(bytes_per_frame)
                .ok_or(AudioError::InvalidFrameSize)?;
            if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
                destination[..packet_bytes].fill(0);
            } else {
                std::ptr::copy_nonoverlapping(
                    data.cast::<u8>(),
                    destination.as_mut_ptr(),
                    packet_bytes,
                );
            }
            self.capture.ReleaseBuffer(packet_frames)?;
            Ok(Some((
                CapturePacket {
                    frames: packet_frames,
                    flags,
                    device_position,
                    qpc_position,
                },
                packet_bytes,
            )))
        }
    }

    /// Wait for the event-driven client to signal available data. Packet reads
    /// remain explicit and bounded; a timeout is reported as `false`.
    pub fn wait_for_data(&self, timeout_ms: u32) -> Result<bool, AudioError> {
        let result = unsafe {
            windows::Win32::System::Threading::WaitForSingleObject(self.event.0, timeout_ms)
        };
        if result == windows::Win32::Foundation::WAIT_OBJECT_0 {
            Ok(true)
        } else if result == windows::Win32::Foundation::WAIT_TIMEOUT {
            Ok(false)
        } else {
            Err(AudioError::Windows(windows::core::Error::from_thread()))
        }
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
    /// The duration argument is retained for API compatibility; event-driven
    /// shared-mode WASAPI requires `Initialize` to receive zero here.
    pub fn open(endpoint_id: &str, _buffer_duration_100ns: i64) -> Result<Self, AudioError> {
        use windows::Win32::Media::Audio::{
            eRender, IAudioClient, IAudioRenderClient, IMMDeviceEnumerator, MMDeviceEnumerator,
            AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
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
        let event = EventHandle(unsafe {
            windows::Win32::System::Threading::CreateEventW(None, false, false, None)?
        });
        let initialized = unsafe {
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_NOPERSIST,
                0,
                0,
                format,
                None,
            )
        };
        unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(format.cast())) };
        initialized?;
        unsafe { client.SetEventHandle(event.0)? };
        let buffer_size = unsafe { client.GetBufferSize()? };
        let render: IAudioRenderClient = unsafe { client.GetService()? };
        Ok(Self {
            client,
            render,
            buffer_size,
            started: false,
            event,
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

    /// Copy caller-owned interleaved bytes into the currently available
    /// render frames and release the WASAPI buffer. The caller must supply the
    /// endpoint's mix-format bytes per frame; partial frames are rejected.
    /// This method never allocates and never submits more than the available
    /// device capacity.
    pub fn submit_bytes(&self, source: &[u8], bytes_per_frame: usize) -> Result<u32, AudioError> {
        if bytes_per_frame == 0 || source.len() % bytes_per_frame != 0 {
            return Err(AudioError::InvalidFrameSize);
        }
        let padding = unsafe { self.client.GetCurrentPadding()? };
        let available = self.buffer_size.saturating_sub(padding);
        let frames = available.min((source.len() / bytes_per_frame) as u32);
        if frames == 0 {
            return Ok(0);
        }
        let bytes = frames as usize * bytes_per_frame;
        unsafe {
            let data = self.render.GetBuffer(frames)?;
            std::ptr::copy_nonoverlapping(source.as_ptr(), data.cast::<u8>(), bytes);
            self.render.ReleaseBuffer(frames, 0)?;
        }
        Ok(frames)
    }

    pub fn wait_for_data(&self, timeout_ms: u32) -> Result<bool, AudioError> {
        let result = unsafe {
            windows::Win32::System::Threading::WaitForSingleObject(self.event.0, timeout_ms)
        };
        if result == windows::Win32::Foundation::WAIT_OBJECT_0 {
            Ok(true)
        } else if result == windows::Win32::Foundation::WAIT_TIMEOUT {
            Ok(false)
        } else {
            Err(AudioError::Windows(windows::core::Error::from_thread()))
        }
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

/// A control-plane subscription for endpoint topology changes.
///
/// The WASAPI callback only sets an atomic dirty bit. It does not allocate,
/// enumerate devices, acquire locks, or touch an audio stream. The owner must
/// call [`EndpointNotificationSubscription::take_dirty`] and then obtain a
/// fresh read-only snapshot when convenient.
pub struct EndpointNotificationSubscription {
    enumerator: windows::Win32::Media::Audio::IMMDeviceEnumerator,
    callback: windows::Win32::Media::Audio::IMMNotificationClient,
    dirty: Arc<AtomicBool>,
    _com: ComApartment,
}

#[windows_core::implement(windows::Win32::Media::Audio::IMMNotificationClient)]
struct EndpointNotificationCallback {
    dirty: Arc<AtomicBool>,
}

impl windows::Win32::Media::Audio::IMMNotificationClient_Impl
    for EndpointNotificationCallback_Impl
{
    fn OnDeviceStateChanged(
        &self,
        _device_id: &windows::core::PCWSTR,
        _state: windows::Win32::Media::Audio::DEVICE_STATE,
    ) -> windows::core::Result<()> {
        self.dirty.store(true, Ordering::Release);
        Ok(())
    }

    fn OnDeviceAdded(&self, _device_id: &windows::core::PCWSTR) -> windows::core::Result<()> {
        self.dirty.store(true, Ordering::Release);
        Ok(())
    }

    fn OnDeviceRemoved(&self, _device_id: &windows::core::PCWSTR) -> windows::core::Result<()> {
        self.dirty.store(true, Ordering::Release);
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        _flow: windows::Win32::Media::Audio::EDataFlow,
        _role: windows::Win32::Media::Audio::ERole,
        _device_id: &windows::core::PCWSTR,
    ) -> windows::core::Result<()> {
        self.dirty.store(true, Ordering::Release);
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _device_id: &windows::core::PCWSTR,
        _key: &windows::Win32::Foundation::PROPERTYKEY,
    ) -> windows::core::Result<()> {
        self.dirty.store(true, Ordering::Release);
        Ok(())
    }
}

impl EndpointNotificationSubscription {
    /// Register a read-only endpoint notification callback on the current
    /// thread's MTA. Registration does not change defaults or open streams.
    pub fn start() -> Result<Self, AudioError> {
        use windows::Win32::Media::Audio::{
            IMMDeviceEnumerator, IMMNotificationClient, MMDeviceEnumerator,
        };
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

        let com = ComApartment::initialize()?;
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let dirty = Arc::new(AtomicBool::new(false));
        let callback: IMMNotificationClient = (EndpointNotificationCallback {
            dirty: Arc::clone(&dirty),
        })
        .into();
        unsafe { enumerator.RegisterEndpointNotificationCallback(&callback)? };
        Ok(Self {
            enumerator,
            callback,
            dirty,
            _com: com,
        })
    }

    /// Return and clear whether a notification arrived since the last call.
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::Acquire)
    }
}

impl Drop for EndpointNotificationSubscription {
    fn drop(&mut self) {
        unsafe {
            let _ = self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.callback);
        }
    }
}

/// Control-plane endpoint monitor that turns coalesced WASAPI notifications
/// into an explicit snapshot diff. It never opens an audio stream or silently
/// changes a binding.
pub struct EndpointMonitor {
    notifications: EndpointNotificationSubscription,
    snapshot: Vec<EndpointInfo>,
}

impl EndpointMonitor {
    pub fn start() -> Result<Self, AudioError> {
        let snapshot = enumerate_active_endpoints()?;
        let notifications = EndpointNotificationSubscription::start()?;
        Ok(Self {
            notifications,
            snapshot,
        })
    }

    /// Refresh only after a notification and return the explicit metadata
    /// changes. An empty result means no notification was pending or no fields
    /// changed in the refreshed snapshot.
    pub fn poll_changes(&mut self) -> Result<Vec<EndpointChange>, AudioError> {
        if !self.notifications.take_dirty() {
            return Ok(Vec::new());
        }
        let current = enumerate_active_endpoints()?;
        let changes = diff_endpoint_snapshots(&self.snapshot, &current);
        self.snapshot = current;
        Ok(changes)
    }

    pub fn snapshot(&self) -> &[EndpointInfo] {
        &self.snapshot
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
/// PID, executable name, and an optional creation timestamp are returned;
/// command lines and full paths are intentionally excluded from this surface.
pub fn enumerate_applications() -> Result<Vec<ApplicationInfo>, AudioError> {
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
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
                let creation_time_100ns = OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION,
                    false,
                    entry.th32ProcessID,
                )
                .ok()
                .and_then(|handle| {
                    let mut creation = FILETIME::default();
                    let mut exit = FILETIME::default();
                    let mut kernel = FILETIME::default();
                    let mut user = FILETIME::default();
                    let result =
                        GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user);
                    let _ = CloseHandle(handle);
                    result.ok().map(|_| {
                        (u64::from(creation.dwHighDateTime) << 32)
                            | u64::from(creation.dwLowDateTime)
                    })
                });
                applications.push(ApplicationInfo {
                    process_id: entry.th32ProcessID,
                    executable,
                    creation_time_100ns,
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

/// Enumerate Windows audio sessions without opening or starting an audio
/// client. Render and capture endpoint session managers are both inspected;
/// a process with a capture session is an observed capture, not a guarantee
/// that every protected or future capture can be looped back.
pub fn enumerate_application_audio() -> Result<Vec<ApplicationAudioInfo>, AudioError> {
    use std::collections::BTreeMap;
    use windows::Win32::Media::Audio::{
        eCapture, eRender, AudioSessionStateActive, IAudioSessionControl2, IAudioSessionManager2,
        IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
        COINIT_MULTITHREADED,
    };
    use windows_core::Interface;

    unsafe {
        let initialized = CoInitializeEx(None, COINIT_MULTITHREADED);
        initialized.ok()?;
        let result = (|| {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let mut by_process = BTreeMap::<u32, ApplicationAudioInfo>::new();
            for (flow, is_capture) in [(eRender, false), (eCapture, true)] {
                let devices = enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)?;
                for index in 0..devices.GetCount()? {
                    let device = devices.Item(index)?;
                    let Ok(manager) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
                    else {
                        continue;
                    };
                    let Ok(sessions) = manager.GetSessionEnumerator() else {
                        continue;
                    };
                    let Ok(count) = sessions.GetCount() else {
                        continue;
                    };
                    let count = count.max(0);
                    for session_index in 0..count {
                        let Ok(session) = sessions.GetSession(session_index) else {
                            continue;
                        };
                        let Ok(control) = session.cast::<IAudioSessionControl2>() else {
                            continue;
                        };
                        let Ok(process_id) = control.GetProcessId() else {
                            continue;
                        };
                        if process_id == 0 {
                            continue;
                        }
                        let state = session
                            .GetState()
                            .unwrap_or(windows::Win32::Media::Audio::AudioSessionStateInactive);
                        let entry =
                            by_process
                                .entry(process_id)
                                .or_insert_with(|| ApplicationAudioInfo {
                                    process_id,
                                    active_session_count: 0,
                                    total_session_count: 0,
                                    capture_session_count: 0,
                                    display_names: Vec::new(),
                                });
                        entry.total_session_count += 1;
                        if state == AudioSessionStateActive {
                            entry.active_session_count += 1;
                        }
                        if is_capture {
                            entry.capture_session_count += 1;
                        }
                        let Ok(display_name) = session.GetDisplayName() else {
                            continue;
                        };
                        if !display_name.is_null() {
                            if let Ok(name) = display_name.to_string() {
                                if !name.is_empty() && !entry.display_names.contains(&name) {
                                    entry.display_names.push(name);
                                }
                            }
                            CoTaskMemFree(Some(display_name.0 as *const core::ffi::c_void));
                        }
                    }
                }
            }
            Ok(by_process.into_values().collect())
        })();
        CoUninitialize();
        result
    }
}

/// Resolve an application only when its PID, executable name, and creation
/// timestamp still match the previously observed identity. This prevents a
/// restarted process from inheriting a prior process-loopback binding.
pub fn bind_application(
    process_id: u32,
    expected_executable: &str,
    expected_creation_time_100ns: Option<u64>,
) -> Result<ApplicationInfo, AudioError> {
    if expected_creation_time_100ns.is_none() {
        return Err(AudioError::ApplicationIdentityUnavailable { process_id });
    }
    let application = enumerate_applications()?
        .into_iter()
        .find(|application| application.process_id == process_id)
        .ok_or(AudioError::ApplicationNotFound { process_id })?;
    if !application
        .executable
        .eq_ignore_ascii_case(expected_executable)
        || application.creation_time_100ns != expected_creation_time_100ns
    {
        return Err(AudioError::ApplicationIdentityChanged { process_id });
    }
    Ok(application)
}

/// Resolve a persisted executable selector after a backend restart. A PID is
/// deliberately not used here: exactly one case-insensitive executable match
/// with a creation timestamp is required, otherwise rebinding remains silent.
pub fn resolve_application_restart(
    applications: &[ApplicationInfo],
    expected_executable: &str,
) -> Result<ApplicationInfo, AudioError> {
    let matches = applications
        .iter()
        .filter(|application| {
            application
                .executable
                .eq_ignore_ascii_case(expected_executable)
        })
        .collect::<Vec<_>>();
    let Some(application) = matches.first() else {
        return Err(AudioError::ApplicationRestartNotFound {
            executable: expected_executable.to_owned(),
        });
    };
    if matches.len() != 1 {
        return Err(AudioError::ApplicationRestartAmbiguous {
            executable: expected_executable.to_owned(),
        });
    }
    if application.creation_time_100ns.is_none() {
        return Err(AudioError::ApplicationRestartIdentityUnavailable {
            executable: expected_executable.to_owned(),
        });
    }
    Ok((*application).clone())
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
        let changes = diff_endpoint_snapshots(
            std::slice::from_ref(&before),
            &[after.clone(), added.clone()],
        );
        assert_eq!(changes.len(), 2);
        assert!(changes.contains(&EndpointChange::Changed { before, after }));
        assert!(changes.contains(&EndpointChange::Added(added)));
    }

    #[test]
    fn audio_failures_have_stable_categories() {
        let error = AudioError::Windows(windows::core::Error::new(
            windows::core::HRESULT(0x8889000A_u32 as i32),
            "busy",
        ));
        assert_eq!(error.kind(), AudioFailureKind::DeviceInUse);
        let error = AudioError::Windows(windows::core::Error::new(
            windows::core::HRESULT(0x88890012_u32 as i32),
            "exclusive",
        ));
        assert_eq!(error.kind(), AudioFailureKind::ExclusiveModeOnly);
        assert_eq!(
            AudioError::InvalidFrameSize.kind(),
            AudioFailureKind::InvalidArgument
        );
        assert_eq!(
            AudioError::BufferTooSmall {
                required: 8,
                available: 4
            }
            .kind(),
            AudioFailureKind::BufferConstraint
        );
    }

    #[test]
    fn restart_binding_requires_one_verified_executable_identity() {
        let candidate = ApplicationInfo {
            process_id: 7,
            executable: "Game.EXE".into(),
            creation_time_100ns: Some(42),
        };
        assert_eq!(
            resolve_application_restart(std::slice::from_ref(&candidate), "game.exe").unwrap(),
            candidate
        );
        assert!(matches!(
            resolve_application_restart(&[], "game.exe"),
            Err(AudioError::ApplicationRestartNotFound { .. })
        ));
        assert!(matches!(
            resolve_application_restart(
                &[
                    candidate.clone(),
                    ApplicationInfo {
                        process_id: 8,
                        ..candidate.clone()
                    }
                ],
                "game.exe"
            ),
            Err(AudioError::ApplicationRestartAmbiguous { .. })
        ));
        assert!(matches!(
            resolve_application_restart(
                &[ApplicationInfo {
                    creation_time_100ns: None,
                    ..candidate
                }],
                "game.exe"
            ),
            Err(AudioError::ApplicationRestartIdentityUnavailable { .. })
        ));
    }

    #[cfg(windows)]
    #[test]
    fn application_binding_requires_the_observed_identity() {
        let process_id = std::process::id();
        let application = enumerate_applications()
            .unwrap()
            .into_iter()
            .find(|application| application.process_id == process_id)
            .unwrap();
        let bound = bind_application(
            process_id,
            &application.executable.to_ascii_lowercase(),
            application.creation_time_100ns,
        )
        .unwrap();
        assert_eq!(bound, application);
        assert!(matches!(
            bind_application(process_id, "different.exe", application.creation_time_100ns),
            Err(AudioError::ApplicationIdentityChanged { .. })
        ));
        assert!(matches!(
            bind_application(process_id, &application.executable, None),
            Err(AudioError::ApplicationIdentityUnavailable { .. })
        ));
    }

    #[cfg(windows)]
    #[test]
    fn application_audio_inventory_is_read_only() {
        let inventory = enumerate_application_audio().unwrap();
        assert!(inventory.iter().all(|item| {
            item.process_id != 0
                && item.active_session_count <= item.total_session_count
                && item.capture_session_count <= item.total_session_count
        }));
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

    #[cfg(windows)]
    #[test]
    fn endpoint_monitor_starts_with_snapshot_and_no_pending_changes() {
        let mut monitor = EndpointMonitor::start().unwrap();
        assert!(!monitor.snapshot().is_empty());
        assert!(monitor.poll_changes().unwrap().is_empty());
    }
}
